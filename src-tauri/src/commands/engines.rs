//! Comandos de gestión de motores externos y de las conversiones que los usan.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

use crate::convert::external::{self, DocFormat, MediaFormat, MediaOptions};
use crate::engines::{self, download, manifest::EngineId, EngineStatus};
use crate::error::{AppError, AppResult};

/// Evento por el que la interfaz sigue la descarga.
pub const PROGRESS_EVENT: &str = "engine://progress";

/// Cada cuánto se informa del avance. Emitir por cada bloque recibido
/// saturaría el canal con miles de mensajes por segundo para nada.
const PROGRESS_INTERVAL: Duration = Duration::from_millis(150);

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressEvent {
    pub engine: &'static str,
    pub downloaded: u64,
    pub total: u64,
}

#[tauri::command]
pub fn list_engines() -> Vec<EngineStatus> {
    engines::all_status()
}

/// Descarga e instala un motor.
///
/// **El consentimiento se pide en la interfaz antes de llamar aquí**: antes de
/// este punto el usuario ha visto nombre, versión, tamaño, origen y licencia, y
/// ha aceptado. Este comando no descarga nada por su cuenta ni se invoca al
/// arrancar: solo desde el botón de instalar.
#[tauri::command]
pub async fn install_engine(app: AppHandle, id: String) -> AppResult<EngineStatus> {
    let engine = EngineId::from_id(&id)
        .ok_or_else(|| AppError::msg(format!("motor desconocido: «{id}»")))?;
    let spec = engines::manifest::spec(engine);
    let directory = engines::engine_dir(engine)?;

    let mut last_emit = Instant::now()
        .checked_sub(PROGRESS_INTERVAL)
        .unwrap_or_else(Instant::now);

    download::install(spec, &directory, |progress| {
        let now = Instant::now();
        let finished = progress.downloaded >= progress.total && progress.total > 0;
        if now.duration_since(last_emit) < PROGRESS_INTERVAL && !finished {
            return;
        }
        last_emit = now;

        let _ = app.emit(
            PROGRESS_EVENT,
            ProgressEvent {
                engine: spec.id.id(),
                downloaded: progress.downloaded,
                total: progress.total,
            },
        );
    })
    .await?;

    Ok(engines::status(engine))
}

#[tauri::command]
pub fn remove_engine(id: String) -> AppResult<EngineStatus> {
    let engine = EngineId::from_id(&id)
        .ok_or_else(|| AppError::msg(format!("motor desconocido: «{id}»")))?;
    engines::remove(engine)?;
    Ok(engines::status(engine))
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifyEngineResult {
    pub ok: bool,
    pub message: String,
}

/// Vuelve a calcular el hash del ejecutable y lo compara con el registrado.
///
/// Es la comprobación cara que no se hace en cada ejecución. Existe para poder
/// pedirla a mano cuando algo huele raro.
#[tauri::command]
pub fn verify_engine(id: String) -> AppResult<VerifyEngineResult> {
    let engine = EngineId::from_id(&id)
        .ok_or_else(|| AppError::msg(format!("motor desconocido: «{id}»")))?;

    let status = engines::status(engine);
    let Some(path) = status.path.as_ref() else {
        return Ok(VerifyEngineResult {
            ok: false,
            message: "El motor no está instalado.".to_string(),
        });
    };

    if status.source != Some(engines::Source::Managed) {
        return Ok(VerifyEngineResult {
            ok: true,
            message: format!(
                "«{path}» no lo ha instalado fast-tools, así que no hay hash de \
                 referencia con el que compararlo."
            ),
        });
    }

    let directory = engines::engine_dir(engine)?;
    let record: download::InstallRecord = serde_json::from_slice(
        &std::fs::read(directory.join(download::RECORD_FILE))?,
    )
    .map_err(|e| AppError::msg(format!("no se pudo leer el registro: {e}")))?;

    let actual = download::hash_file(Path::new(path))?;
    let ok = actual == record.executable_sha256;

    Ok(VerifyEngineResult {
        ok,
        message: if ok {
            format!("Íntegro. SHA-256 {actual}")
        } else {
            format!(
                "El ejecutable NO coincide con el que se instaló.\n\
                 Registrado: {}\nActual: {actual}\nBórralo y vuelve a instalarlo.",
                record.executable_sha256
            )
        },
    })
}

// ── Catálogos de formato ──────────────────────────────────────────────────

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocFormatInfo {
    pub id: &'static str,
    pub label: &'static str,
    pub extensions: &'static [&'static str],
    pub default_extension: &'static str,
    pub can_read: bool,
    pub can_write: bool,
    pub engine: &'static str,
    pub limitation: Option<&'static str>,
}

#[tauri::command]
pub fn list_doc_formats() -> Vec<DocFormatInfo> {
    DocFormat::ALL
        .iter()
        .map(|format| DocFormatInfo {
            id: format.id(),
            label: format.label(),
            extensions: format.extensions(),
            default_extension: format.default_extension(),
            can_read: format.can_read(),
            can_write: format.can_write(),
            engine: format.engine_for_output().id(),
            limitation: format.limitation(),
        })
        .collect()
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaFormatInfo {
    pub id: &'static str,
    pub label: &'static str,
    pub audio_only: bool,
}

#[tauri::command]
pub fn list_media_formats() -> Vec<MediaFormatInfo> {
    MediaFormat::ALL
        .iter()
        .map(|format| MediaFormatInfo {
            id: format.id(),
            label: format.label(),
            audio_only: format.is_audio_only(),
        })
        .collect()
}

// ── Conversión por lotes ──────────────────────────────────────────────────

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileOutcome {
    pub source: String,
    pub output: Option<String>,
    pub error: Option<String>,
}

fn destination_for(source: &Path, extension: &str, output_dir: Option<&Path>) -> AppResult<PathBuf> {
    let stem = source
        .file_stem()
        .ok_or_else(|| AppError::msg(format!("«{}» no tiene nombre", source.display())))?;

    let directory = match output_dir {
        Some(dir) => dir.to_path_buf(),
        None => source
            .parent()
            .ok_or_else(|| AppError::msg("el archivo no está en ninguna carpeta"))?
            .to_path_buf(),
    };

    Ok(directory.join(format!("{}.{extension}", stem.to_string_lossy())))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocRequest {
    pub paths: Vec<String>,
    pub to: String,
    pub output_dir: Option<String>,
    #[serde(default)]
    pub overwrite: bool,
}

#[tauri::command]
pub fn convert_documents(request: DocRequest) -> AppResult<Vec<FileOutcome>> {
    let to = DocFormat::from_id(&request.to)
        .ok_or_else(|| AppError::msg(format!("formato desconocido: «{}»", request.to)))?;

    // Se comprueba el motor una sola vez, antes del lote: si falta, no tiene
    // sentido intentar cien archivos para dar cien veces el mismo error.
    engines::resolve(to.engine_for_output())?;

    let output_dir = request.output_dir.as_ref().map(PathBuf::from);

    Ok(request
        .paths
        .iter()
        .map(|source| {
            let path = PathBuf::from(source);
            let outcome = (|| -> AppResult<PathBuf> {
                let from = DocFormat::from_path(&path).ok_or_else(|| {
                    AppError::msg(format!("no se reconoce el formato de «{source}»"))
                })?;
                let destination =
                    destination_for(&path, to.default_extension(), output_dir.as_deref())?;

                if destination == path {
                    return Err(AppError::msg("el destino coincide con el origen"));
                }
                if destination.exists() && !request.overwrite {
                    return Err(AppError::msg(format!(
                        "«{}» ya existe",
                        destination.display()
                    )));
                }

                external::convert_document(from, to, &path, &destination)?;
                Ok(destination)
            })();

            match outcome {
                Ok(destination) => FileOutcome {
                    source: source.clone(),
                    output: Some(destination.display().to_string()),
                    error: None,
                },
                Err(error) => FileOutcome {
                    source: source.clone(),
                    output: None,
                    error: Some(error.to_string()),
                },
            }
        })
        .collect())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaRequest {
    pub paths: Vec<String>,
    pub to: String,
    pub output_dir: Option<String>,
    #[serde(default)]
    pub overwrite: bool,
    #[serde(default)]
    pub options: MediaOptions,
}

#[tauri::command]
pub fn convert_media_files(request: MediaRequest) -> AppResult<Vec<FileOutcome>> {
    let to = MediaFormat::from_id(&request.to)
        .ok_or_else(|| AppError::msg(format!("formato desconocido: «{}»", request.to)))?;

    engines::resolve(EngineId::Ffmpeg)?;

    let output_dir = request.output_dir.as_ref().map(PathBuf::from);

    Ok(request
        .paths
        .iter()
        .map(|source| {
            let path = PathBuf::from(source);
            let outcome = (|| -> AppResult<PathBuf> {
                let destination = destination_for(&path, to.id(), output_dir.as_deref())?;

                if destination == path {
                    return Err(AppError::msg("el destino coincide con el origen"));
                }
                if destination.exists() && !request.overwrite {
                    return Err(AppError::msg(format!(
                        "«{}» ya existe",
                        destination.display()
                    )));
                }

                external::convert_media(to, &path, &destination, &request.options)?;
                Ok(destination)
            })();

            match outcome {
                Ok(destination) => FileOutcome {
                    source: source.clone(),
                    output: Some(destination.display().to_string()),
                    error: None,
                },
                Err(error) => FileOutcome {
                    source: source.clone(),
                    output: None,
                    error: Some(error.to_string()),
                },
            }
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn el_catalogo_de_documentos_marca_el_motor_de_cada_uno() {
        let formats = list_doc_formats();

        let pdf = formats.iter().find(|f| f.id == "pdf").unwrap();
        assert_eq!(pdf.engine, "libreoffice");
        assert!(!pdf.can_read);
        assert!(pdf.limitation.is_some());

        let docx = formats.iter().find(|f| f.id == "docx").unwrap();
        assert_eq!(docx.engine, "pandoc");
        assert!(docx.can_read);
    }

    #[test]
    fn el_catalogo_de_multimedia_distingue_audio_de_video() {
        let formats = list_media_formats();
        assert!(formats.iter().find(|f| f.id == "mp3").unwrap().audio_only);
        assert!(!formats.iter().find(|f| f.id == "mp4").unwrap().audio_only);
    }

    #[test]
    fn el_destino_cambia_la_extension_y_respeta_la_carpeta() {
        let source = Path::new("C:/docs/informe.md");
        assert_eq!(
            destination_for(source, "pdf", None).unwrap(),
            PathBuf::from("C:/docs/informe.pdf")
        );
        assert_eq!(
            destination_for(source, "html", Some(Path::new("D:/salida"))).unwrap(),
            PathBuf::from("D:/salida/informe.html")
        );
    }

    #[test]
    fn el_destino_conserva_los_puntos_intermedios_del_nombre() {
        let source = Path::new("C:/docs/acta.2026.v2.md");
        assert_eq!(
            destination_for(source, "pdf", None).unwrap(),
            PathBuf::from("C:/docs/acta.2026.v2.pdf")
        );
    }

    #[test]
    fn un_motor_desconocido_da_error_con_su_nombre() {
        let error = remove_engine("imagemagick".into()).unwrap_err().to_string();
        assert!(error.contains("imagemagick"), "{error}");
    }

    #[test]
    fn el_lote_falla_pronto_si_falta_el_motor() {
        // Sin Pandoc instalado, no se intenta archivo por archivo.
        if engines::status(EngineId::Pandoc).available {
            return;
        }
        let error = convert_documents(DocRequest {
            paths: vec!["a.md".into(), "b.md".into()],
            to: "html".into(),
            output_dir: None,
            overwrite: false,
        })
        .unwrap_err()
        .to_string();

        assert!(error.contains("Pandoc"), "{error}");
    }

    #[test]
    fn verificar_un_motor_ausente_lo_dice_sin_fallar() {
        if engines::status(EngineId::Ffmpeg).available {
            return;
        }
        let result = verify_engine("ffmpeg".into()).unwrap();
        assert!(!result.ok);
        assert!(result.message.contains("no está instalado"));
    }
}
