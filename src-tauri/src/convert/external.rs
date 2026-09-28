//! Conversiones que delegan en un motor externo.
//!
//! A diferencia del motor de datos, que trabaja en memoria, aquí se pasa por
//! archivos: es lo que esperan Pandoc, FFmpeg y LibreOffice, y evita tener que
//! sostener un vídeo de un gigabyte en RAM.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::engines::{self, manifest::EngineId};
use crate::error::{AppError, AppResult};

// ── Documentos ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DocFormat {
    Markdown,
    Html,
    Docx,
    Odt,
    Rst,
    Latex,
    Epub,
    Text,
    Pdf,
}

impl DocFormat {
    pub const ALL: &'static [DocFormat] = &[
        DocFormat::Markdown,
        DocFormat::Html,
        DocFormat::Docx,
        DocFormat::Odt,
        DocFormat::Rst,
        DocFormat::Latex,
        DocFormat::Epub,
        DocFormat::Text,
        DocFormat::Pdf,
    ];

    pub fn id(self) -> &'static str {
        match self {
            DocFormat::Markdown => "markdown",
            DocFormat::Html => "html",
            DocFormat::Docx => "docx",
            DocFormat::Odt => "odt",
            DocFormat::Rst => "rst",
            DocFormat::Latex => "latex",
            DocFormat::Epub => "epub",
            DocFormat::Text => "text",
            DocFormat::Pdf => "pdf",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            DocFormat::Markdown => "Markdown",
            DocFormat::Html => "HTML",
            DocFormat::Docx => "Word (DOCX)",
            DocFormat::Odt => "OpenDocument (ODT)",
            DocFormat::Rst => "reStructuredText",
            DocFormat::Latex => "LaTeX",
            DocFormat::Epub => "EPUB",
            DocFormat::Text => "Texto plano",
            DocFormat::Pdf => "PDF",
        }
    }

    pub fn extensions(self) -> &'static [&'static str] {
        match self {
            DocFormat::Markdown => &["md", "markdown"],
            DocFormat::Html => &["html", "htm"],
            DocFormat::Docx => &["docx"],
            DocFormat::Odt => &["odt"],
            DocFormat::Rst => &["rst"],
            DocFormat::Latex => &["tex", "latex"],
            DocFormat::Epub => &["epub"],
            DocFormat::Text => &["txt"],
            DocFormat::Pdf => &["pdf"],
        }
    }

    pub fn default_extension(self) -> &'static str {
        self.extensions()[0]
    }

    /// Nombre del formato tal como lo llama Pandoc.
    fn pandoc_name(self) -> Option<&'static str> {
        match self {
            DocFormat::Markdown => Some("markdown"),
            DocFormat::Html => Some("html"),
            DocFormat::Docx => Some("docx"),
            DocFormat::Odt => Some("odt"),
            DocFormat::Rst => Some("rst"),
            DocFormat::Latex => Some("latex"),
            DocFormat::Epub => Some("epub"),
            DocFormat::Text => Some("plain"),
            // Pandoc necesita un motor de LaTeX instalado aparte para escribir
            // PDF. Esa vía se deja a LibreOffice, que lo hace por su cuenta.
            DocFormat::Pdf => None,
        }
    }

    pub fn can_read(self) -> bool {
        // Leer un PDF es extraer texto de un formato de presentación: sale
        // desordenado y sin estructura. Es mejor no ofrecerlo que ofrecerlo mal.
        self != DocFormat::Pdf
    }

    pub fn can_write(self) -> bool {
        true
    }

    /// Qué motor hace falta para escribir este formato.
    pub fn engine_for_output(self) -> EngineId {
        match self {
            DocFormat::Pdf => EngineId::LibreOffice,
            _ => EngineId::Pandoc,
        }
    }

    pub fn from_id(id: &str) -> Option<DocFormat> {
        DocFormat::ALL
            .iter()
            .copied()
            .find(|format| format.id().eq_ignore_ascii_case(id))
    }

    pub fn from_path(path: &Path) -> Option<DocFormat> {
        let extension = path.extension()?.to_str()?.to_ascii_lowercase();
        DocFormat::ALL
            .iter()
            .copied()
            .find(|format| format.extensions().contains(&extension.as_str()))
    }

    /// Nota sobre lo que no se puede hacer con este formato.
    pub fn limitation(self) -> Option<&'static str> {
        match self {
            DocFormat::Pdf => Some(
                "Solo salida, y a través de LibreOffice. Leer un PDF es extraer \
                 texto de un formato pensado para imprimir: el resultado sale sin \
                 estructura y casi nunca sirve.",
            ),
            _ => None,
        }
    }
}

pub fn convert_document(
    from: DocFormat,
    to: DocFormat,
    input: &Path,
    output: &Path,
) -> AppResult<()> {
    if !from.can_read() {
        return Err(AppError::msg(format!(
            "{}: {}",
            from.label(),
            from.limitation().unwrap_or("no se puede leer")
        )));
    }

    if to == DocFormat::Pdf {
        return convert_to_pdf(input, output);
    }

    let (Some(from_name), Some(to_name)) = (from.pandoc_name(), to.pandoc_name()) else {
        return Err(AppError::msg(format!(
            "no hay forma de convertir {} a {}",
            from.label(),
            to.label()
        )));
    };

    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)?;
    }

    engines::run(
        EngineId::Pandoc,
        [
            "--from".as_ref(),
            from_name.as_ref(),
            "--to".as_ref(),
            to_name.as_ref(),
            // Sin esto, un HTML de entrada con un `<script>` o un `<iframe>`
            // pasaría tal cual al documento de salida.
            "--sandbox".as_ref(),
            "--standalone".as_ref(),
            "--output".as_ref(),
            output.as_os_str(),
            input.as_os_str(),
        ] as [&std::ffi::OsStr; 9],
    )?;

    Ok(())
}

/// Exporta a PDF con LibreOffice.
///
/// `soffice` no admite decir el nombre del archivo de salida: escribe en el
/// directorio que se le indique, con el mismo nombre base y extensión `.pdf`.
/// Por eso se convierte en un directorio temporal y luego se mueve al destino.
fn convert_to_pdf(input: &Path, output: &Path) -> AppResult<()> {
    let staging = output
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."))
        .join(format!(".fast-tools-pdf-{}", std::process::id()));
    std::fs::create_dir_all(&staging)?;

    let cleanup = || {
        let _ = std::fs::remove_dir_all(&staging);
    };

    let result = engines::run(
        EngineId::LibreOffice,
        [
            "--headless".as_ref(),
            "--convert-to".as_ref(),
            "pdf".as_ref(),
            "--outdir".as_ref(),
            staging.as_os_str(),
            input.as_os_str(),
        ] as [&std::ffi::OsStr; 6],
    );

    if let Err(error) = result {
        cleanup();
        return Err(error);
    }

    let stem = input
        .file_stem()
        .ok_or_else(|| AppError::msg("el archivo de origen no tiene nombre"))?;
    let produced = staging.join(format!("{}.pdf", stem.to_string_lossy()));

    if !produced.is_file() {
        cleanup();
        return Err(AppError::msg(
            "LibreOffice terminó sin error pero no dejó ningún PDF. \
             Comprueba que no haya otra instancia de LibreOffice abierta: \
             en modo headless solo puede haber una.",
        ));
    }

    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)?;
    }
    // `rename` falla entre volúmenes distintos; copiar y borrar siempre vale.
    std::fs::copy(&produced, output)?;
    cleanup();

    Ok(())
}

// ── Audio y vídeo ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MediaFormat {
    Mp4,
    Mkv,
    Webm,
    Mov,
    Gif,
    Mp3,
    Wav,
    Flac,
    Ogg,
    M4a,
}

impl MediaFormat {
    pub const ALL: &'static [MediaFormat] = &[
        MediaFormat::Mp4,
        MediaFormat::Mkv,
        MediaFormat::Webm,
        MediaFormat::Mov,
        MediaFormat::Gif,
        MediaFormat::Mp3,
        MediaFormat::Wav,
        MediaFormat::Flac,
        MediaFormat::Ogg,
        MediaFormat::M4a,
    ];

    pub fn id(self) -> &'static str {
        match self {
            MediaFormat::Mp4 => "mp4",
            MediaFormat::Mkv => "mkv",
            MediaFormat::Webm => "webm",
            MediaFormat::Mov => "mov",
            MediaFormat::Gif => "gif",
            MediaFormat::Mp3 => "mp3",
            MediaFormat::Wav => "wav",
            MediaFormat::Flac => "flac",
            MediaFormat::Ogg => "ogg",
            MediaFormat::M4a => "m4a",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            MediaFormat::Mp4 => "MP4",
            MediaFormat::Mkv => "MKV",
            MediaFormat::Webm => "WebM",
            MediaFormat::Mov => "MOV",
            MediaFormat::Gif => "GIF animado",
            MediaFormat::Mp3 => "MP3",
            MediaFormat::Wav => "WAV",
            MediaFormat::Flac => "FLAC",
            MediaFormat::Ogg => "OGG",
            MediaFormat::M4a => "M4A",
        }
    }

    pub fn is_audio_only(self) -> bool {
        matches!(
            self,
            MediaFormat::Mp3 | MediaFormat::Wav | MediaFormat::Flac | MediaFormat::Ogg | MediaFormat::M4a
        )
    }

    pub fn from_id(id: &str) -> Option<MediaFormat> {
        MediaFormat::ALL
            .iter()
            .copied()
            .find(|format| format.id().eq_ignore_ascii_case(id))
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct MediaOptions {
    /// Calidad de audio en kbit/s. Se ignora en los formatos sin pérdida.
    pub audio_bitrate: u32,
    /// Factor de calidad de vídeo (CRF). Menos es mejor calidad y más peso.
    pub video_quality: u32,
    /// Quita la pista de vídeo, para extraer solo el audio.
    pub audio_only: bool,
    /// Segundo en el que empieza el recorte.
    pub start_seconds: Option<f64>,
    /// Duración del recorte en segundos.
    pub duration_seconds: Option<f64>,
}

impl Default for MediaOptions {
    fn default() -> Self {
        MediaOptions {
            audio_bitrate: 192,
            video_quality: 23,
            audio_only: false,
            start_seconds: None,
            duration_seconds: None,
        }
    }
}

pub fn convert_media(
    to: MediaFormat,
    input: &Path,
    output: &Path,
    options: &MediaOptions,
) -> AppResult<()> {
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let mut args: Vec<std::ffi::OsString> = Vec::new();

    // Sobrescribir sin preguntar: la comprobación de «ya existe» la hace la
    // capa de comandos antes de llegar aquí, y si no, ffmpeg se quedaría
    // esperando una respuesta que nadie va a teclear.
    args.push("-y".into());
    args.push("-nostdin".into());

    // El recorte va antes de `-i` para que ffmpeg salte por índice en lugar de
    // decodificar y descartar todo lo anterior: la diferencia es de minutos.
    if let Some(start) = options.start_seconds {
        args.push("-ss".into());
        args.push(format!("{start}").into());
    }

    args.push("-i".into());
    args.push(input.as_os_str().to_os_string());

    if let Some(duration) = options.duration_seconds {
        args.push("-t".into());
        args.push(format!("{duration}").into());
    }

    if to.is_audio_only() || options.audio_only {
        args.push("-vn".into());
    }

    if !to.is_audio_only() {
        args.push("-crf".into());
        args.push(options.video_quality.clamp(0, 51).to_string().into());
    }

    // Los formatos sin pérdida ignoran el bitrate; ponérselo sería mentir
    // sobre lo que hace el control.
    if !matches!(to, MediaFormat::Wav | MediaFormat::Flac) {
        args.push("-b:a".into());
        args.push(format!("{}k", options.audio_bitrate.clamp(32, 512)).into());
    }

    args.push(output.as_os_str().to_os_string());

    engines::run(EngineId::Ffmpeg, args)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn los_identificadores_de_documento_redondean() {
        for format in DocFormat::ALL {
            assert_eq!(DocFormat::from_id(format.id()), Some(*format));
            assert!(!format.extensions().is_empty());
        }
    }

    #[test]
    fn los_identificadores_de_multimedia_redondean() {
        for format in MediaFormat::ALL {
            assert_eq!(MediaFormat::from_id(format.id()), Some(*format));
        }
    }

    #[test]
    fn detecta_el_formato_de_documento_por_la_extension() {
        assert_eq!(
            DocFormat::from_path(Path::new("informe.md")),
            Some(DocFormat::Markdown)
        );
        assert_eq!(
            DocFormat::from_path(Path::new("a/b/CARTA.DOCX")),
            Some(DocFormat::Docx)
        );
        assert_eq!(DocFormat::from_path(Path::new("x.desconocido")), None);
    }

    #[test]
    fn el_pdf_solo_es_de_salida_y_lo_explica() {
        assert!(!DocFormat::Pdf.can_read());
        assert!(DocFormat::Pdf.can_write());
        assert!(DocFormat::Pdf.limitation().unwrap().contains("Solo salida"));
    }

    #[test]
    fn leer_un_pdf_se_rechaza_antes_de_llamar_a_ningun_motor() {
        let error = convert_document(
            DocFormat::Pdf,
            DocFormat::Markdown,
            Path::new("x.pdf"),
            Path::new("y.md"),
        )
        .unwrap_err()
        .to_string();

        assert!(error.contains("Solo salida"), "{error}");
    }

    #[test]
    fn cada_formato_declara_el_motor_que_necesita() {
        assert_eq!(DocFormat::Pdf.engine_for_output(), EngineId::LibreOffice);
        assert_eq!(DocFormat::Docx.engine_for_output(), EngineId::Pandoc);
        assert_eq!(DocFormat::Html.engine_for_output(), EngineId::Pandoc);
    }

    #[test]
    fn los_formatos_de_solo_audio_estan_bien_clasificados() {
        assert!(MediaFormat::Mp3.is_audio_only());
        assert!(MediaFormat::Flac.is_audio_only());
        assert!(!MediaFormat::Mp4.is_audio_only());
        assert!(!MediaFormat::Gif.is_audio_only());
    }

    #[test]
    fn una_conversion_sin_motor_instalado_dice_que_falta() {
        // En una máquina sin Pandoc ni FFmpeg, el error tiene que ser útil.
        if engines::status(EngineId::Pandoc).available {
            return;
        }
        let error = convert_document(
            DocFormat::Markdown,
            DocFormat::Html,
            Path::new("a.md"),
            Path::new("b.html"),
        )
        .unwrap_err()
        .to_string();

        assert!(error.contains("Pandoc"), "{error}");
        assert!(error.contains("no está disponible"), "{error}");
    }
}
