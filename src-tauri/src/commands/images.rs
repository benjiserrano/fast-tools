//! Comandos de las herramientas de imagen.
//!
//! Las vistas previas viajan a la interfaz como URI de datos en Base64. Es la
//! única forma de pintar en el WebView una imagen que está en memoria sin
//! escribirla antes en disco ni abrir un servidor local, y la política de
//! seguridad de la ventana ya permite `data:` en `img-src`.

use std::path::{Path, PathBuf};

use base64::Engine;
use image::DynamicImage;
use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};
use crate::tools::exif;
use crate::tools::images::{self, ImageKind, ResizeOptions, SaveOptions};
use crate::tools::qr;

/// Lado máximo de una vista previa. Más allá, la URI en Base64 ocupa varios
/// megabytes de texto y el WebView tarda más en decodificarla que en pintarla.
const PREVIEW_MAX_SIDE: u32 = 1400;

/// Tope al abrir un archivo de imagen, para no tragarse un archivo enorme
/// entero en memoria antes siquiera de saber qué es.
const MAX_FILE_BYTES: u64 = 256 * 1024 * 1024;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageFormatInfo {
    pub id: &'static str,
    pub label: &'static str,
    pub extensions: &'static [&'static str],
    pub default_extension: &'static str,
    pub can_read: bool,
    pub can_write: bool,
    pub lossy: bool,
    pub supports_alpha: bool,
    pub limitation: Option<&'static str>,
}

#[tauri::command]
pub fn list_image_formats() -> Vec<ImageFormatInfo> {
    ImageKind::ALL
        .iter()
        .map(|kind| ImageFormatInfo {
            id: kind.id(),
            label: kind.label(),
            extensions: kind.extensions(),
            default_extension: kind.default_extension(),
            can_read: kind.can_read(),
            can_write: kind.can_write(),
            lossy: kind.is_lossy(),
            supports_alpha: kind.supports_alpha(),
            limitation: kind.limitation(),
        })
        .collect()
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadedImage {
    pub path: String,
    pub kind: &'static str,
    pub label: &'static str,
    pub width: u32,
    pub height: u32,
    pub bytes: usize,
    pub has_alpha: bool,
    /// Vista previa reducida, lista para un `<img src>`.
    pub preview: String,
}

fn read_file(path: &Path) -> AppResult<Vec<u8>> {
    let size = std::fs::metadata(path)
        .map_err(|e| AppError::msg(format!("no se pudo abrir «{}»: {e}", path.display())))?
        .len();
    if size > MAX_FILE_BYTES {
        return Err(AppError::msg(format!(
            "«{}» ocupa {:.0} MB y el máximo es {} MB",
            path.display(),
            size as f64 / 1_048_576.0,
            MAX_FILE_BYTES / 1_048_576
        )));
    }
    Ok(std::fs::read(path)?)
}

/// Codifica una imagen como PNG dentro de una URI de datos, reduciéndola si
/// hace falta para que no pese de más.
fn preview(image: &DynamicImage) -> AppResult<String> {
    let scaled = if image.width() > PREVIEW_MAX_SIDE || image.height() > PREVIEW_MAX_SIDE {
        image.resize(
            PREVIEW_MAX_SIDE,
            PREVIEW_MAX_SIDE,
            image::imageops::FilterType::Triangle,
        )
    } else {
        image.clone()
    };

    let png = images::save(&scaled, ImageKind::Png, &SaveOptions::default())?;
    Ok(format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(png)
    ))
}

#[tauri::command]
pub fn load_image(path: String, svg_width: Option<u32>) -> AppResult<LoadedImage> {
    let source = PathBuf::from(&path);
    let bytes = read_file(&source)?;

    let kind = ImageKind::detect(&bytes)
        .ok_or_else(|| AppError::msg(format!("«{path}» no parece una imagen")))?;
    let image = images::load(&bytes, svg_width)?;

    Ok(LoadedImage {
        path,
        kind: kind.id(),
        label: kind.label(),
        width: image.width(),
        height: image.height(),
        bytes: bytes.len(),
        has_alpha: image.color().has_alpha(),
        preview: preview(&image)?,
    })
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessRequest {
    pub path: String,
    pub to: String,
    #[serde(default)]
    pub resize: ResizeOptions,
    #[serde(default)]
    pub save: SaveOptions,
    pub svg_width: Option<u32>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessResult {
    pub width: u32,
    pub height: u32,
    pub bytes: usize,
    /// Tamaño del archivo original, para poder comparar.
    pub original_bytes: usize,
    pub preview: String,
}

/// Aplica las transformaciones y devuelve el resultado **sin guardarlo**.
///
/// Separar el cálculo del guardado permite que la interfaz enseñe el peso y el
/// aspecto resultantes mientras se mueve el control de calidad, y que solo se
/// escriba en disco cuando el usuario esté conforme.
#[tauri::command]
pub fn process_image(request: ProcessRequest) -> AppResult<ProcessResult> {
    let (processed, encoded, original_bytes) = run_pipeline(&request)?;

    Ok(ProcessResult {
        width: processed.width(),
        height: processed.height(),
        bytes: encoded.len(),
        original_bytes,
        preview: preview(&processed)?,
    })
}

#[tauri::command]
pub fn save_processed_image(
    request: ProcessRequest,
    destination: String,
    overwrite: bool,
) -> AppResult<String> {
    let target = PathBuf::from(&destination);
    if target.exists() && !overwrite {
        return Err(AppError::msg(format!("«{destination}» ya existe")));
    }

    let (_, encoded, _) = run_pipeline(&request)?;
    write_atomically(&target, &encoded)?;
    Ok(destination)
}

fn run_pipeline(request: &ProcessRequest) -> AppResult<(DynamicImage, Vec<u8>, usize)> {
    let to = ImageKind::from_id(&request.to)
        .ok_or_else(|| AppError::msg(format!("formato desconocido: «{}»", request.to)))?;

    let bytes = read_file(Path::new(&request.path))?;
    let original = images::load(&bytes, request.svg_width)?;
    let processed = images::transform(&original, &request.resize)?;
    let encoded = images::save(&processed, to, &request.save)?;

    Ok((processed, encoded, bytes.len()))
}

/// Escribe en un temporal y renombra, para no dejar un archivo a medias si el
/// proceso muere mientras guarda.
fn write_atomically(destination: &Path, bytes: &[u8]) -> AppResult<()> {
    if let Some(parent) = destination.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let staging = destination.with_extension("parcial");
    std::fs::write(&staging, bytes)?;
    std::fs::rename(&staging, destination).inspect_err(|_| {
        let _ = std::fs::remove_file(&staging);
    })?;
    Ok(())
}

// ── Favicon ───────────────────────────────────────────────────────────────

#[tauri::command]
pub fn favicon_sizes() -> &'static [u32] {
    images::FAVICON_SIZES
}

#[tauri::command]
pub fn generate_favicon(
    path: String,
    sizes: Vec<u32>,
    destination: String,
    overwrite: bool,
) -> AppResult<String> {
    let target = PathBuf::from(&destination);
    if target.exists() && !overwrite {
        return Err(AppError::msg(format!("«{destination}» ya existe")));
    }

    let bytes = read_file(Path::new(&path))?;
    // Un favicon se hace a menudo desde un SVG: se rasteriza al mayor de los
    // tamaños pedidos para que ninguna resolución salga de una ampliación.
    let svg_width = sizes.iter().copied().max().unwrap_or(256).max(256);
    let image = images::load(&bytes, Some(svg_width))?;

    let ico = images::favicon(&image, &sizes)?;
    write_atomically(&target, &ico)?;
    Ok(destination)
}

// ── Paleta ────────────────────────────────────────────────────────────────

#[tauri::command]
pub fn extract_palette(path: String, count: usize) -> AppResult<Vec<images::PaletteColor>> {
    let bytes = read_file(Path::new(&path))?;
    let image = images::load(&bytes, None)?;
    images::palette(&image, count)
}

// ── EXIF ──────────────────────────────────────────────────────────────────

#[tauri::command]
pub fn read_exif(path: String) -> AppResult<exif::ExifReport> {
    let bytes = read_file(Path::new(&path))?;
    exif::read(&bytes)
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StripExifResult {
    pub destination: String,
    pub removed_bytes: usize,
    pub recompressed: bool,
}

#[tauri::command]
pub fn strip_exif(
    path: String,
    destination: String,
    overwrite: bool,
) -> AppResult<StripExifResult> {
    let target = PathBuf::from(&destination);
    if target.exists() && !overwrite {
        return Err(AppError::msg(format!("«{destination}» ya existe")));
    }

    let bytes = read_file(Path::new(&path))?;
    let result = exif::strip(&bytes)?;
    write_atomically(&target, &result.bytes)?;

    Ok(StripExifResult {
        destination,
        removed_bytes: result.removed_bytes,
        recompressed: result.recompressed,
    })
}

// ── Códigos QR ────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CorrectionInfo {
    pub id: &'static str,
    pub label: &'static str,
    pub note: &'static str,
}

#[tauri::command]
pub fn qr_correction_levels() -> Vec<CorrectionInfo> {
    qr::Correction::ALL
        .iter()
        .map(|level| CorrectionInfo {
            id: level.id(),
            label: level.label(),
            note: level.note(),
        })
        .collect()
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QrResult {
    /// PNG en URI de datos, para la vista previa.
    pub preview: String,
    pub svg: String,
    pub bytes: usize,
}

#[tauri::command]
pub fn generate_qr(content: String, options: qr::QrOptions) -> AppResult<QrResult> {
    let png = qr::to_png(&content, &options)?;
    let svg = qr::to_svg(&content, &options)?;

    Ok(QrResult {
        preview: format!(
            "data:image/png;base64,{}",
            base64::engine::general_purpose::STANDARD.encode(&png)
        ),
        svg,
        bytes: png.len(),
    })
}

#[tauri::command]
pub fn save_qr(
    content: String,
    options: qr::QrOptions,
    destination: String,
    overwrite: bool,
) -> AppResult<String> {
    let target = PathBuf::from(&destination);
    if target.exists() && !overwrite {
        return Err(AppError::msg(format!("«{destination}» ya existe")));
    }

    let as_svg = target
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("svg"));

    let bytes = if as_svg {
        qr::to_svg(&content, &options)?.into_bytes()
    } else {
        qr::to_png(&content, &options)?
    };

    write_atomically(&target, &bytes)?;
    Ok(destination)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("ft-{name}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write_sample(dir: &Path, name: &str, kind: ImageKind) -> PathBuf {
        let mut buffer = image::RgbaImage::new(64, 32);
        for (x, y, pixel) in buffer.enumerate_pixels_mut() {
            *pixel = image::Rgba([(x * 4) as u8, (y * 8) as u8, 200, 255]);
        }
        let bytes = images::save(
            &DynamicImage::ImageRgba8(buffer),
            kind,
            &SaveOptions::default(),
        )
        .unwrap();

        let path = dir.join(name);
        std::fs::write(&path, bytes).unwrap();
        path
    }

    #[test]
    fn carga_una_imagen_con_su_vista_previa() {
        let dir = temp_dir("load");
        let path = write_sample(&dir, "muestra.png", ImageKind::Png);

        let loaded = load_image(path.display().to_string(), None).unwrap();

        assert_eq!(loaded.kind, "png");
        assert_eq!((loaded.width, loaded.height), (64, 32));
        assert!(loaded.preview.starts_with("data:image/png;base64,"));
        assert!(loaded.has_alpha);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn procesar_no_escribe_nada_en_disco() {
        let dir = temp_dir("process");
        let path = write_sample(&dir, "muestra.png", ImageKind::Png);
        let antes = std::fs::read_dir(&dir).unwrap().count();

        let result = process_image(ProcessRequest {
            path: path.display().to_string(),
            to: "jpeg".into(),
            resize: ResizeOptions {
                width: Some(32),
                no_upscale: false,
                ..ResizeOptions::default()
            },
            save: SaveOptions::default(),
            svg_width: None,
        })
        .unwrap();

        assert_eq!(result.width, 32);
        assert_eq!(result.height, 16, "debe conservar la proporción 2:1");
        assert!(result.bytes > 0);
        assert!(result.original_bytes > 0);

        let despues = std::fs::read_dir(&dir).unwrap().count();
        assert_eq!(antes, despues, "process_image no debe crear archivos");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn guardar_respeta_el_archivo_existente() {
        let dir = temp_dir("save");
        let path = write_sample(&dir, "muestra.png", ImageKind::Png);
        let destino = dir.join("salida.jpg");

        let request = || ProcessRequest {
            path: path.display().to_string(),
            to: "jpeg".into(),
            resize: ResizeOptions::default(),
            save: SaveOptions::default(),
            svg_width: None,
        };

        save_processed_image(request(), destino.display().to_string(), false).unwrap();
        assert!(destino.exists());

        let error = save_processed_image(request(), destino.display().to_string(), false)
            .unwrap_err()
            .to_string();
        assert!(error.contains("ya existe"), "{error}");

        // Con permiso explícito sí sobrescribe.
        save_processed_image(request(), destino.display().to_string(), true).unwrap();

        // Y no quedan restos del guardado atómico.
        let parciales = std::fs::read_dir(&dir)
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| entry.file_name().to_string_lossy().contains("parcial"))
            .count();
        assert_eq!(parciales, 0);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn genera_un_ico_con_varias_resoluciones() {
        let dir = temp_dir("favicon");
        let path = write_sample(&dir, "logo.png", ImageKind::Png);
        let destino = dir.join("favicon.ico");

        generate_favicon(
            path.display().to_string(),
            vec![16, 32, 48],
            destino.display().to_string(),
            false,
        )
        .unwrap();

        let ico = std::fs::read(&destino).unwrap();
        assert_eq!(u16::from_le_bytes([ico[4], ico[5]]), 3);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn el_qr_sale_en_png_y_svg_a_la_vez() {
        let result = generate_qr(
            "https://fast-tools.test".into(),
            qr::QrOptions::default(),
        )
        .unwrap();

        assert!(result.preview.starts_with("data:image/png;base64,"));
        assert!(result.svg.contains("<svg"));
        assert!(result.bytes > 0);
    }

    #[test]
    fn guardar_el_qr_elige_formato_por_la_extension() {
        let dir = temp_dir("qr");

        let svg_path = dir.join("codigo.svg");
        save_qr(
            "hola".into(),
            qr::QrOptions::default(),
            svg_path.display().to_string(),
            false,
        )
        .unwrap();
        assert!(std::fs::read_to_string(&svg_path).unwrap().contains("<svg"));

        let png_path = dir.join("codigo.png");
        save_qr(
            "hola".into(),
            qr::QrOptions::default(),
            png_path.display().to_string(),
            false,
        )
        .unwrap();
        assert!(std::fs::read(&png_path).unwrap().starts_with(b"\x89PNG"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn la_paleta_sale_ordenada_de_mas_a_menos() {
        let dir = temp_dir("palette");
        let path = write_sample(&dir, "muestra.png", ImageKind::Png);

        let colors = extract_palette(path.display().to_string(), 5).unwrap();
        assert!(!colors.is_empty());
        for pair in colors.windows(2) {
            assert!(pair[0].share >= pair[1].share, "{colors:?}");
        }

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn un_formato_de_destino_inventado_da_error_con_su_nombre() {
        let dir = temp_dir("badformat");
        let path = write_sample(&dir, "muestra.png", ImageKind::Png);

        let error = process_image(ProcessRequest {
            path: path.display().to_string(),
            to: "heic".into(),
            resize: ResizeOptions::default(),
            save: SaveOptions::default(),
            svg_width: None,
        })
        .unwrap_err()
        .to_string();

        assert!(error.contains("heic"), "{error}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn un_archivo_que_no_es_imagen_lo_dice() {
        let dir = temp_dir("notimage");
        let path = dir.join("texto.png");
        std::fs::write(&path, b"esto no es una imagen").unwrap();

        let error = load_image(path.display().to_string(), None)
            .unwrap_err()
            .to_string();
        assert!(error.contains("no parece una imagen"), "{error}");

        let _ = std::fs::remove_dir_all(&dir);
    }
}
