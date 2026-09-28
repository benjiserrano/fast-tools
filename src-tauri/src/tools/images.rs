//! Conversión y manipulación de imágenes.
//!
//! Todo en memoria y sin procesos externos. El único formato que no se lee es
//! AVIF: decodificarlo necesita libdav1d, una biblioteca en C que rompería la
//! promesa de un único `.exe` sin dependencias. Escribir AVIF sí se puede,
//! porque el codificador es Rust puro.

use std::io::Cursor;

use image::{DynamicImage, ImageFormat, ImageReader};
use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};

/// Tope de píxeles a decodificar. Un PNG de 50 KB puede declarar 50 000 × 50 000
/// píxeles y reventar la memoria al expandirse: es la «bomba de descompresión»
/// clásica. 100 megapíxeles deja pasar cualquier foto real.
const MAX_PIXELS: u64 = 100_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ImageKind {
    Png,
    Jpeg,
    Webp,
    Avif,
    Gif,
    Bmp,
    Tiff,
    Ico,
    Svg,
}

impl ImageKind {
    pub const ALL: &'static [ImageKind] = &[
        ImageKind::Png,
        ImageKind::Jpeg,
        ImageKind::Webp,
        ImageKind::Avif,
        ImageKind::Gif,
        ImageKind::Bmp,
        ImageKind::Tiff,
        ImageKind::Ico,
        ImageKind::Svg,
    ];

    pub fn id(self) -> &'static str {
        match self {
            ImageKind::Png => "png",
            ImageKind::Jpeg => "jpeg",
            ImageKind::Webp => "webp",
            ImageKind::Avif => "avif",
            ImageKind::Gif => "gif",
            ImageKind::Bmp => "bmp",
            ImageKind::Tiff => "tiff",
            ImageKind::Ico => "ico",
            ImageKind::Svg => "svg",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            ImageKind::Png => "PNG",
            ImageKind::Jpeg => "JPEG",
            ImageKind::Webp => "WebP",
            ImageKind::Avif => "AVIF",
            ImageKind::Gif => "GIF",
            ImageKind::Bmp => "BMP",
            ImageKind::Tiff => "TIFF",
            ImageKind::Ico => "ICO",
            ImageKind::Svg => "SVG",
        }
    }

    pub fn extensions(self) -> &'static [&'static str] {
        match self {
            ImageKind::Png => &["png"],
            ImageKind::Jpeg => &["jpg", "jpeg"],
            ImageKind::Webp => &["webp"],
            ImageKind::Avif => &["avif"],
            ImageKind::Gif => &["gif"],
            ImageKind::Bmp => &["bmp"],
            ImageKind::Tiff => &["tif", "tiff"],
            ImageKind::Ico => &["ico"],
            ImageKind::Svg => &["svg"],
        }
    }

    pub fn default_extension(self) -> &'static str {
        self.extensions()[0]
    }

    /// SVG es vectorial: entra rasterizándose, pero no se puede generar desde
    /// una imagen de píxeles sin vectorizarla, que es otro problema.
    pub fn can_read(self) -> bool {
        self != ImageKind::Avif
    }

    pub fn can_write(self) -> bool {
        self != ImageKind::Svg
    }

    /// Por qué un formato no admite lectura o escritura. Se muestra en la UI
    /// para que la opción desactivada no parezca un fallo.
    pub fn limitation(self) -> Option<&'static str> {
        match self {
            ImageKind::Avif => Some(
                "Solo escritura: decodificar AVIF necesita una biblioteca en C \
                 que rompería la portabilidad del ejecutable.",
            ),
            ImageKind::Svg => Some(
                "Solo lectura: se rasteriza a píxeles. Convertir píxeles a \
                 vectores es vectorizar, que es otra cosa.",
            ),
            _ => None,
        }
    }

    /// Si el formato pierde calidad al guardar y admite ajustarla.
    pub fn is_lossy(self) -> bool {
        matches!(self, ImageKind::Jpeg | ImageKind::Webp | ImageKind::Avif)
    }

    pub fn supports_alpha(self) -> bool {
        !matches!(self, ImageKind::Jpeg)
    }

    pub fn from_id(id: &str) -> Option<ImageKind> {
        ImageKind::ALL
            .iter()
            .copied()
            .find(|kind| kind.id().eq_ignore_ascii_case(id))
    }

    fn to_image_format(self) -> Option<ImageFormat> {
        match self {
            ImageKind::Png => Some(ImageFormat::Png),
            ImageKind::Jpeg => Some(ImageFormat::Jpeg),
            ImageKind::Webp => Some(ImageFormat::WebP),
            ImageKind::Avif => Some(ImageFormat::Avif),
            ImageKind::Gif => Some(ImageFormat::Gif),
            ImageKind::Bmp => Some(ImageFormat::Bmp),
            ImageKind::Tiff => Some(ImageFormat::Tiff),
            ImageKind::Ico => Some(ImageFormat::Ico),
            ImageKind::Svg => None,
        }
    }

    /// Detección por contenido. Las firmas mandan sobre la extensión porque un
    /// archivo llamado `.png` que en realidad es JPEG es muy común.
    pub fn detect(bytes: &[u8]) -> Option<ImageKind> {
        if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
            return Some(ImageKind::Png);
        }
        if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
            return Some(ImageKind::Jpeg);
        }
        if bytes.starts_with(b"GIF8") {
            return Some(ImageKind::Gif);
        }
        if bytes.starts_with(b"BM") {
            return Some(ImageKind::Bmp);
        }
        if bytes.starts_with(b"II\x2a\x00") || bytes.starts_with(b"MM\x00\x2a") {
            return Some(ImageKind::Tiff);
        }
        if bytes.starts_with(&[0x00, 0x00, 0x01, 0x00]) {
            return Some(ImageKind::Ico);
        }
        if bytes.len() > 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
            return Some(ImageKind::Webp);
        }
        if bytes.len() > 12 && &bytes[4..8] == b"ftyp" && &bytes[8..12] == b"avif" {
            return Some(ImageKind::Avif);
        }
        // El SVG es texto: puede empezar por la declaración XML o por la etiqueta.
        let head = String::from_utf8_lossy(&bytes[..bytes.len().min(512)]);
        if head.contains("<svg") {
            return Some(ImageKind::Svg);
        }
        None
    }
}

/// Decodifica cualquier formato de entrada a píxeles.
///
/// `svg_width` fija el ancho al que se rasteriza un SVG; el alto sale de su
/// proporción. Para el resto de formatos no se usa.
pub fn load(bytes: &[u8], svg_width: Option<u32>) -> AppResult<DynamicImage> {
    let kind = ImageKind::detect(bytes)
        .ok_or_else(|| AppError::msg("no se reconoce el formato de la imagen"))?;

    if kind == ImageKind::Svg {
        return rasterize_svg(bytes, svg_width.unwrap_or(1024));
    }
    if !kind.can_read() {
        return Err(AppError::msg(format!(
            "{}: {}",
            kind.label(),
            kind.limitation().unwrap_or("formato no admitido")
        )));
    }

    let reader = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|e| AppError::msg(format!("no se pudo leer la imagen: {e}")))?;

    let (width, height) = reader
        .into_dimensions()
        .map_err(|e| AppError::msg(format!("no se pudieron leer las dimensiones: {e}")))?;
    guard_pixels(width, height)?;

    ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|e| AppError::msg(format!("no se pudo leer la imagen: {e}")))?
        .decode()
        .map_err(|e| AppError::msg(format!("no se pudo decodificar la imagen: {e}")))
}

fn guard_pixels(width: u32, height: u32) -> AppResult<()> {
    let pixels = u64::from(width) * u64::from(height);
    if pixels > MAX_PIXELS {
        return Err(AppError::msg(format!(
            "la imagen declara {width}×{height} ({} megapíxeles) y el máximo es {}. \
             Un archivo pequeño que declara un tamaño enorme suele ser un intento \
             de agotar la memoria.",
            pixels / 1_000_000,
            MAX_PIXELS / 1_000_000
        )));
    }
    Ok(())
}

fn rasterize_svg(bytes: &[u8], width: u32) -> AppResult<DynamicImage> {
    // Se usan las reexportaciones de `resvg` en vez de depender de `usvg` y
    // `tiny-skia` por separado: si las versiones no coinciden exactamente, sus
    // tipos son incompatibles aunque se llamen igual.
    use resvg::{tiny_skia, usvg};

    let options = usvg::Options::default();
    let tree = usvg::Tree::from_data(bytes, &options)
        .map_err(|e| AppError::msg(format!("el SVG no es válido: {e}")))?;

    let size = tree.size();
    if size.width() <= 0.0 || size.height() <= 0.0 {
        return Err(AppError::msg("el SVG no declara un tamaño utilizable"));
    }

    let scale = width as f32 / size.width();
    let height = (size.height() * scale).round().max(1.0) as u32;
    guard_pixels(width, height)?;

    let mut pixmap = tiny_skia::Pixmap::new(width, height)
        .ok_or_else(|| AppError::msg("no se pudo reservar el lienzo para el SVG"))?;

    let mut canvas = pixmap.as_mut();
    resvg::render(
        &tree,
        usvg::Transform::from_scale(scale, scale),
        &mut canvas,
    );

    image::RgbaImage::from_raw(width, height, pixmap.take())
        .map(DynamicImage::ImageRgba8)
        .ok_or_else(|| AppError::msg("el lienzo rasterizado tiene un tamaño inesperado"))
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SaveOptions {
    /// Calidad de 1 a 100 en los formatos con pérdida.
    pub quality: u8,
    /// Color de fondo al aplanar transparencia sobre un formato sin alfa.
    pub background: String,
}

impl Default for SaveOptions {
    fn default() -> Self {
        SaveOptions {
            quality: 85,
            background: "#ffffff".to_string(),
        }
    }
}

pub fn save(image: &DynamicImage, kind: ImageKind, options: &SaveOptions) -> AppResult<Vec<u8>> {
    if !kind.can_write() {
        return Err(AppError::msg(format!(
            "{}: {}",
            kind.label(),
            kind.limitation().unwrap_or("no se puede escribir")
        )));
    }

    // Guardar una imagen con transparencia como JPEG dejaría los píxeles
    // transparentes en negro. Se aplanan sobre un fondo elegido.
    let prepared = if !kind.supports_alpha() {
        flatten(image, parse_color(&options.background)?)
    } else {
        image.clone()
    };

    let mut out = Vec::new();

    match kind {
        ImageKind::Jpeg => {
            let quality = options.quality.clamp(1, 100);
            let mut encoder =
                image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, quality);
            encoder
                .encode_image(&prepared)
                .map_err(|e| AppError::msg(format!("no se pudo escribir el JPEG: {e}")))?;
        }
        ImageKind::Webp => {
            // El codificador WebP de `image` solo hace modo sin pérdida, así
            // que la calidad no se aplica aquí; se dice para que nadie espere
            // que el control tenga efecto.
            prepared
                .write_to(&mut Cursor::new(&mut out), ImageFormat::WebP)
                .map_err(|e| AppError::msg(format!("no se pudo escribir el WebP: {e}")))?;
        }
        other => {
            let format = other
                .to_image_format()
                .ok_or_else(|| AppError::msg("formato sin codificador"))?;
            prepared
                .write_to(&mut Cursor::new(&mut out), format)
                .map_err(|e| {
                    AppError::msg(format!("no se pudo escribir el {}: {e}", other.label()))
                })?;
        }
    }

    Ok(out)
}

/// Compone la imagen sobre un color opaco, respetando el canal alfa.
fn flatten(image: &DynamicImage, background: [u8; 3]) -> DynamicImage {
    let source = image.to_rgba8();
    let mut output = image::RgbImage::new(source.width(), source.height());

    for (x, y, pixel) in source.enumerate_pixels() {
        let alpha = f32::from(pixel[3]) / 255.0;
        let blend = |index: usize| {
            (f32::from(pixel[index]) * alpha + f32::from(background[index]) * (1.0 - alpha))
                .round() as u8
        };
        output.put_pixel(x, y, image::Rgb([blend(0), blend(1), blend(2)]));
    }

    DynamicImage::ImageRgb8(output)
}

pub fn parse_color(text: &str) -> AppResult<[u8; 3]> {
    let hex = text.trim().trim_start_matches('#');
    let expanded = match hex.len() {
        // Forma corta: #abc equivale a #aabbcc.
        3 => hex.chars().flat_map(|c| [c, c]).collect::<String>(),
        6 => hex.to_string(),
        _ => {
            return Err(AppError::msg(format!(
                "«{text}» no es un color hexadecimal; se esperaba #rgb o #rrggbb"
            )))
        }
    };

    let component = |range: std::ops::Range<usize>| {
        u8::from_str_radix(&expanded[range], 16)
            .map_err(|_| AppError::msg(format!("«{text}» tiene dígitos no hexadecimales")))
    };

    Ok([component(0..2)?, component(2..4)?, component(4..6)?])
}

// ── Transformaciones ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FitMode {
    /// Encaja dentro del recuadro conservando la proporción.
    Contain,
    /// Llena el recuadro conservando la proporción y recortando lo que sobra.
    Cover,
    /// Estira hasta el tamaño exacto, deformando la imagen.
    Stretch,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ResizeOptions {
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub fit: FitMode,
    /// Grados en sentido horario: 0, 90, 180 o 270.
    pub rotate: u16,
    pub flip_horizontal: bool,
    pub flip_vertical: bool,
    /// Evita agrandar una imagen más allá de su tamaño original.
    pub no_upscale: bool,
}

impl Default for ResizeOptions {
    fn default() -> Self {
        ResizeOptions {
            width: None,
            height: None,
            fit: FitMode::Contain,
            rotate: 0,
            flip_horizontal: false,
            flip_vertical: false,
            no_upscale: true,
        }
    }
}

pub fn transform(image: &DynamicImage, options: &ResizeOptions) -> AppResult<DynamicImage> {
    let mut result = image.clone();

    if options.width.is_some() || options.height.is_some() {
        let (source_width, source_height) = (result.width(), result.height());
        let (target_width, target_height) =
            target_size(source_width, source_height, options)?;

        let grows = target_width > source_width || target_height > source_height;
        if !(options.no_upscale && grows) {
            // Lanczos3 es el filtro que mejor conserva el detalle al reducir,
            // que es el caso habitual. Cuesta más, pero una imagen se
            // redimensiona una vez.
            result = match options.fit {
                FitMode::Cover => result.resize_to_fill(
                    target_width,
                    target_height,
                    image::imageops::FilterType::Lanczos3,
                ),
                FitMode::Contain => result.resize(
                    target_width,
                    target_height,
                    image::imageops::FilterType::Lanczos3,
                ),
                FitMode::Stretch => result.resize_exact(
                    target_width,
                    target_height,
                    image::imageops::FilterType::Lanczos3,
                ),
            };
        }
    }

    result = match options.rotate {
        0 => result,
        90 => result.rotate90(),
        180 => result.rotate180(),
        270 => result.rotate270(),
        other => {
            return Err(AppError::msg(format!(
                "solo se puede rotar 0, 90, 180 o 270 grados, no {other}"
            )))
        }
    };

    if options.flip_horizontal {
        result = result.fliph();
    }
    if options.flip_vertical {
        result = result.flipv();
    }

    Ok(result)
}

/// Resuelve el tamaño de destino cuando solo se da una de las dos medidas.
fn target_size(width: u32, height: u32, options: &ResizeOptions) -> AppResult<(u32, u32)> {
    if width == 0 || height == 0 {
        return Err(AppError::msg("la imagen de origen no tiene tamaño"));
    }

    let ratio = f64::from(height) / f64::from(width);
    let (target_width, target_height) = match (options.width, options.height) {
        (Some(w), Some(h)) => (w, h),
        (Some(w), None) => (w, (f64::from(w) * ratio).round() as u32),
        (None, Some(h)) => ((f64::from(h) / ratio).round() as u32, h),
        (None, None) => (width, height),
    };

    if target_width == 0 || target_height == 0 {
        return Err(AppError::msg("el tamaño de destino no puede ser cero"));
    }
    guard_pixels(target_width, target_height)?;
    Ok((target_width.max(1), target_height.max(1)))
}

// ── Favicon ───────────────────────────────────────────────────────────────

/// Tamaños que espera Windows y que usan los navegadores en pestaña, favoritos
/// y accesos directos del escritorio.
pub const FAVICON_SIZES: &[u32] = &[16, 32, 48, 64, 128, 256];

pub fn favicon(image: &DynamicImage, sizes: &[u32]) -> AppResult<Vec<u8>> {
    if sizes.is_empty() {
        return Err(AppError::msg("elige al menos un tamaño"));
    }
    if let Some(bad) = sizes.iter().find(|size| **size == 0 || **size > 256) {
        return Err(AppError::msg(format!(
            "un icono ICO admite entre 1 y 256 píxeles por lado, no {bad}"
        )));
    }

    // Se construye el ICO a mano: `image` escribe ICO de una sola resolución y
    // un favicon útil lleva varias dentro del mismo archivo.
    let entries: Vec<(u32, Vec<u8>)> = sizes
        .iter()
        .map(|size| {
            let scaled =
                image.resize_exact(*size, *size, image::imageops::FilterType::Lanczos3);
            let mut png = Vec::new();
            scaled
                .to_rgba8()
                .write_to(&mut Cursor::new(&mut png), ImageFormat::Png)
                .map_err(|e| AppError::msg(format!("no se pudo preparar el tamaño {size}: {e}")))?;
            Ok((*size, png))
        })
        .collect::<AppResult<_>>()?;

    let mut out = Vec::new();
    // Cabecera ICONDIR: reservado, tipo 1 (icono), número de imágenes.
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&(entries.len() as u16).to_le_bytes());

    let header_size = 6 + 16 * entries.len();
    let mut offset = header_size as u32;

    for (size, png) in &entries {
        // 256 se codifica como 0 en el campo de un byte.
        let dimension = if *size == 256 { 0u8 } else { *size as u8 };
        out.push(dimension);
        out.push(dimension);
        out.push(0); // colores de la paleta: 0 = sin paleta
        out.push(0); // reservado
        out.extend_from_slice(&1u16.to_le_bytes()); // planos de color
        out.extend_from_slice(&32u16.to_le_bytes()); // bits por píxel
        out.extend_from_slice(&(png.len() as u32).to_le_bytes());
        out.extend_from_slice(&offset.to_le_bytes());
        offset += png.len() as u32;
    }

    for (_, png) in &entries {
        out.extend_from_slice(png);
    }

    Ok(out)
}

// ── Paleta de colores ─────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaletteColor {
    pub hex: String,
    pub rgb: [u8; 3],
    /// Porcentaje de píxeles más cercanos a este color.
    pub share: f32,
}

pub fn palette(image: &DynamicImage, count: usize) -> AppResult<Vec<PaletteColor>> {
    let count = count.clamp(1, 32);

    // Se reduce antes de cuantizar: el color dominante de una foto de 20 MP es
    // el mismo que el de su miniatura, y así el cálculo tarda milisegundos.
    let sample = image.resize(200, 200, image::imageops::FilterType::Triangle);
    let pixels = sample.to_rgba8();

    // Los píxeles casi transparentes no aportan color y falsearían la paleta.
    let opaque: Vec<u8> = pixels
        .pixels()
        .filter(|pixel| pixel[3] > 128)
        .flat_map(|pixel| pixel.0)
        .collect();

    if opaque.is_empty() {
        return Err(AppError::msg("la imagen es completamente transparente"));
    }

    let quantizer = color_quant::NeuQuant::new(10, count, &opaque);
    let colors = quantizer.color_map_rgba();

    let mut tally = vec![0usize; count];
    for pixel in opaque.chunks_exact(4) {
        let index = quantizer.index_of(pixel);
        if let Some(slot) = tally.get_mut(index) {
            *slot += 1;
        }
    }

    let total: usize = tally.iter().sum::<usize>().max(1);
    let mut result: Vec<PaletteColor> = tally
        .iter()
        .enumerate()
        .filter(|(_, hits)| **hits > 0)
        .filter_map(|(index, hits)| {
            let base = index * 4;
            let rgb = [
                *colors.get(base)?,
                *colors.get(base + 1)?,
                *colors.get(base + 2)?,
            ];
            Some(PaletteColor {
                hex: format!("#{:02x}{:02x}{:02x}", rgb[0], rgb[1], rgb[2]),
                rgb,
                share: (*hits as f32 / total as f32) * 100.0,
            })
        })
        .collect();

    result.sort_by(|a, b| b.share.total_cmp(&a.share));
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(width: u32, height: u32) -> DynamicImage {
        let mut buffer = image::RgbaImage::new(width, height);
        for (x, y, pixel) in buffer.enumerate_pixels_mut() {
            *pixel = image::Rgba([(x % 256) as u8, (y % 256) as u8, 128, 255]);
        }
        DynamicImage::ImageRgba8(buffer)
    }

    fn encode(image: &DynamicImage, kind: ImageKind) -> Vec<u8> {
        save(image, kind, &SaveOptions::default()).unwrap()
    }

    #[test]
    fn va_y_vuelve_por_los_formatos_sin_perdida() {
        let original = sample(32, 24);

        for kind in [ImageKind::Png, ImageKind::Bmp, ImageKind::Tiff] {
            let bytes = encode(&original, kind);
            let recovered = load(&bytes, None)
                .unwrap_or_else(|e| panic!("{}: {e}", kind.label()));

            assert_eq!(recovered.width(), 32, "{}", kind.label());
            assert_eq!(recovered.height(), 24, "{}", kind.label());
        }
    }

    #[test]
    fn detecta_los_formatos_por_su_firma() {
        for kind in [
            ImageKind::Png,
            ImageKind::Jpeg,
            ImageKind::Gif,
            ImageKind::Bmp,
            ImageKind::Tiff,
            ImageKind::Webp,
        ] {
            let bytes = encode(&sample(8, 8), kind);
            assert_eq!(
                ImageKind::detect(&bytes),
                Some(kind),
                "no se detectó {}",
                kind.label()
            );
        }
    }

    #[test]
    fn detecta_svg_por_su_etiqueta() {
        let svg = br#"<?xml version="1.0"?><svg xmlns="http://www.w3.org/2000/svg"/>"#;
        assert_eq!(ImageKind::detect(svg), Some(ImageKind::Svg));
    }

    #[test]
    fn rasteriza_svg_al_ancho_pedido_conservando_la_proporcion() {
        let svg = br##"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="50">
            <rect width="100" height="50" fill="#3366cc"/></svg>"##;

        let rendered = load(svg, Some(200)).unwrap();
        assert_eq!(rendered.width(), 200);
        assert_eq!(rendered.height(), 100, "debe respetar la proporción 2:1");
    }

    #[test]
    fn un_svg_roto_da_error_en_vez_de_entrar_en_panico() {
        let error = load(b"<svg esto no cierra", None).unwrap_err().to_string();
        assert!(!error.is_empty());
    }

    #[test]
    fn guardar_como_jpeg_aplana_la_transparencia() {
        let mut transparent = image::RgbaImage::new(4, 4);
        for pixel in transparent.pixels_mut() {
            *pixel = image::Rgba([255, 0, 0, 0]); // rojo totalmente transparente
        }
        let source = DynamicImage::ImageRgba8(transparent);

        let options = SaveOptions {
            background: "#00ff00".to_string(),
            ..SaveOptions::default()
        };
        let bytes = save(&source, ImageKind::Jpeg, &options).unwrap();
        let recovered = load(&bytes, None).unwrap().to_rgb8();

        // Transparente al 100 % sobre verde da verde, no negro.
        let pixel = recovered.get_pixel(1, 1);
        assert!(pixel[1] > 200, "esperaba verde, salió {pixel:?}");
        assert!(pixel[0] < 60, "no debería quedar rojo: {pixel:?}");
    }

    #[test]
    fn la_calidad_de_jpeg_cambia_el_tamano() {
        let original = sample(128, 128);

        let alta = save(
            &original,
            ImageKind::Jpeg,
            &SaveOptions {
                quality: 95,
                ..SaveOptions::default()
            },
        )
        .unwrap();
        let baja = save(
            &original,
            ImageKind::Jpeg,
            &SaveOptions {
                quality: 20,
                ..SaveOptions::default()
            },
        )
        .unwrap();

        assert!(baja.len() < alta.len(), "{} vs {}", baja.len(), alta.len());
    }

    #[test]
    fn redimensiona_conservando_la_proporcion_con_una_sola_medida() {
        let original = sample(200, 100);
        let options = ResizeOptions {
            width: Some(50),
            no_upscale: false,
            ..ResizeOptions::default()
        };

        let result = transform(&original, &options).unwrap();
        assert_eq!(result.width(), 50);
        assert_eq!(result.height(), 25);
    }

    #[test]
    fn el_modo_cover_recorta_hasta_llenar() {
        let original = sample(200, 100);
        let options = ResizeOptions {
            width: Some(50),
            height: Some(50),
            fit: FitMode::Cover,
            no_upscale: false,
            ..ResizeOptions::default()
        };

        let result = transform(&original, &options).unwrap();
        assert_eq!((result.width(), result.height()), (50, 50));
    }

    #[test]
    fn no_agranda_cuando_se_le_pide_que_no() {
        let original = sample(50, 50);
        let options = ResizeOptions {
            width: Some(500),
            no_upscale: true,
            ..ResizeOptions::default()
        };

        let result = transform(&original, &options).unwrap();
        assert_eq!(result.width(), 50, "no debía crecer");
    }

    #[test]
    fn rota_intercambiando_los_lados() {
        let original = sample(40, 20);
        let options = ResizeOptions {
            rotate: 90,
            ..ResizeOptions::default()
        };

        let result = transform(&original, &options).unwrap();
        assert_eq!((result.width(), result.height()), (20, 40));
    }

    #[test]
    fn rechaza_rotaciones_que_no_son_multiplos_de_noventa() {
        let options = ResizeOptions {
            rotate: 45,
            ..ResizeOptions::default()
        };
        let error = transform(&sample(4, 4), &options).unwrap_err().to_string();
        assert!(error.contains("45"), "{error}");
    }

    #[test]
    fn rechaza_imagenes_con_demasiados_pixeles() {
        let error = guard_pixels(50_000, 50_000).unwrap_err().to_string();
        assert!(error.contains("megapíxeles"), "{error}");

        // Una foto normal pasa sin problema.
        assert!(guard_pixels(6000, 4000).is_ok());
    }

    #[test]
    fn el_favicon_lleva_todas_las_resoluciones_dentro() {
        let ico = favicon(&sample(256, 256), &[16, 32, 48]).unwrap();

        // Cabecera ICONDIR: reservado 0, tipo 1, y el número de imágenes.
        assert_eq!(&ico[0..2], &[0, 0]);
        assert_eq!(&ico[2..4], &[1, 0]);
        assert_eq!(u16::from_le_bytes([ico[4], ico[5]]), 3);

        // La primera entrada declara 16×16.
        assert_eq!(ico[6], 16);
        assert_eq!(ico[7], 16);
    }

    #[test]
    fn el_favicon_codifica_256_como_cero() {
        let ico = favicon(&sample(256, 256), &[256]).unwrap();
        assert_eq!(ico[6], 0, "256 se escribe como 0 en un campo de un byte");
    }

    #[test]
    fn el_favicon_rechaza_tamanos_imposibles() {
        assert!(favicon(&sample(8, 8), &[]).is_err());
        let error = favicon(&sample(8, 8), &[512]).unwrap_err().to_string();
        assert!(error.contains("256"), "{error}");
    }

    #[test]
    fn la_paleta_saca_el_color_dominante() {
        let mut buffer = image::RgbaImage::new(100, 100);
        for pixel in buffer.pixels_mut() {
            *pixel = image::Rgba([220, 20, 60, 255]);
        }

        let colors = palette(&DynamicImage::ImageRgba8(buffer), 4).unwrap();
        assert!(!colors.is_empty());

        let dominant = &colors[0];
        assert!(dominant.share > 90.0, "cuota: {}", dominant.share);
        assert!(
            (i16::from(dominant.rgb[0]) - 220).abs() < 25,
            "esperaba carmesí, salió {:?}",
            dominant.rgb
        );
    }

    #[test]
    fn la_paleta_ignora_los_pixeles_transparentes() {
        let mut buffer = image::RgbaImage::new(20, 20);
        for pixel in buffer.pixels_mut() {
            *pixel = image::Rgba([0, 0, 0, 0]);
        }

        let error = palette(&DynamicImage::ImageRgba8(buffer), 4)
            .unwrap_err()
            .to_string();
        assert!(error.contains("transparente"), "{error}");
    }

    #[test]
    fn interpreta_colores_hexadecimales() {
        assert_eq!(parse_color("#ff8800").unwrap(), [255, 136, 0]);
        assert_eq!(parse_color("ff8800").unwrap(), [255, 136, 0]);
        // Forma corta.
        assert_eq!(parse_color("#f80").unwrap(), [255, 136, 0]);
        assert!(parse_color("#12345").is_err());
        assert!(parse_color("#gggggg").is_err());
    }

    #[test]
    fn los_formatos_declaran_sus_limitaciones() {
        assert!(!ImageKind::Avif.can_read());
        assert!(ImageKind::Avif.can_write());
        assert!(ImageKind::Svg.can_read());
        assert!(!ImageKind::Svg.can_write());
        assert!(ImageKind::Avif.limitation().is_some());
        assert!(ImageKind::Svg.limitation().is_some());
        assert!(ImageKind::Png.limitation().is_none());
    }

    #[test]
    fn los_identificadores_redondean() {
        for kind in ImageKind::ALL {
            assert_eq!(ImageKind::from_id(kind.id()), Some(*kind));
        }
    }
}
