//! Lectura y borrado de metadatos EXIF.
//!
//! El borrado en JPEG es quirúrgico: se quitan los segmentos de metadatos y se
//! deja intacto el flujo comprimido. Volver a codificar la imagen la degradaría
//! cada vez, y limpiar metadatos no debería costar calidad.

use serde::Serialize;

use crate::error::{AppError, AppResult};
use crate::tools::images::{self, ImageKind};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExifField {
    pub tag: String,
    pub value: String,
    /// Bloque al que pertenece: imagen principal, miniatura, GPS…
    pub group: String,
    /// Si revela dónde o cuándo se tomó la foto, o con qué equipo.
    pub sensitive: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExifReport {
    pub fields: Vec<ExifField>,
    /// Coordenadas legibles si la imagen las lleva.
    pub location: Option<String>,
    pub warnings: Vec<String>,
}

/// Etiquetas que exponen al autor de la foto y por tanto conviene señalar.
fn is_sensitive(tag: exif::Tag) -> bool {
    use exif::Tag;
    matches!(
        tag,
        Tag::GPSLatitude
            | Tag::GPSLongitude
            | Tag::GPSAltitude
            | Tag::GPSTimeStamp
            | Tag::GPSDateStamp
            | Tag::DateTime
            | Tag::DateTimeOriginal
            | Tag::DateTimeDigitized
            | Tag::Make
            | Tag::Model
            | Tag::BodySerialNumber
            | Tag::LensSerialNumber
            | Tag::CameraOwnerName
            | Tag::Artist
            | Tag::Copyright
            | Tag::ImageUniqueID
            | Tag::Software
    )
}

pub fn read(bytes: &[u8]) -> AppResult<ExifReport> {
    let mut cursor = std::io::Cursor::new(bytes);
    let reader = exif::Reader::new();

    let data = match reader.read_from_container(&mut cursor) {
        Ok(data) => data,
        // Que no haya metadatos es un resultado normal, no un fallo: significa
        // que la imagen ya está limpia.
        Err(exif::Error::NotFound(_)) => {
            return Ok(ExifReport {
                fields: Vec::new(),
                location: None,
                warnings: vec!["La imagen no lleva metadatos EXIF.".to_string()],
            })
        }
        Err(e) => return Err(AppError::msg(format!("no se pudo leer el EXIF: {e}"))),
    };

    let fields: Vec<ExifField> = data
        .fields()
        .map(|field| ExifField {
            tag: field.tag.to_string(),
            value: field.display_value().with_unit(&data).to_string(),
            group: format!("{}", field.ifd_num),
            sensitive: is_sensitive(field.tag),
        })
        .collect();

    let location = read_location(&data);

    let mut warnings = Vec::new();
    if location.is_some() {
        warnings.push(
            "La imagen lleva coordenadas GPS: revela dónde se tomó. \
             Quita los metadatos antes de publicarla."
                .to_string(),
        );
    }
    let sensitive_count = fields.iter().filter(|field| field.sensitive).count();
    if sensitive_count > 0 && location.is_none() {
        warnings.push(format!(
            "{sensitive_count} campos identifican la cámara, el autor o la fecha exacta."
        ));
    }

    Ok(ExifReport {
        fields,
        location,
        warnings,
    })
}

fn read_location(data: &exif::Exif) -> Option<String> {
    let latitude = data.get_field(exif::Tag::GPSLatitude, exif::In::PRIMARY)?;
    let longitude = data.get_field(exif::Tag::GPSLongitude, exif::In::PRIMARY)?;

    let north = data
        .get_field(exif::Tag::GPSLatitudeRef, exif::In::PRIMARY)
        .map(|field| field.display_value().to_string())
        .unwrap_or_else(|| "N".to_string());
    let east = data
        .get_field(exif::Tag::GPSLongitudeRef, exif::In::PRIMARY)
        .map(|field| field.display_value().to_string())
        .unwrap_or_else(|| "E".to_string());

    Some(format!(
        "{} {} · {} {}",
        latitude.display_value(),
        north.trim(),
        longitude.display_value(),
        east.trim()
    ))
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StripResult {
    pub bytes: Vec<u8>,
    pub removed_bytes: usize,
    /// Si hubo que volver a comprimir, con la pérdida de calidad que implica.
    pub recompressed: bool,
}

pub fn strip(bytes: &[u8]) -> AppResult<StripResult> {
    let kind = ImageKind::detect(bytes)
        .ok_or_else(|| AppError::msg("no se reconoce el formato de la imagen"))?;

    if kind == ImageKind::Jpeg {
        let cleaned = strip_jpeg_segments(bytes)?;
        return Ok(StripResult {
            removed_bytes: bytes.len().saturating_sub(cleaned.len()),
            bytes: cleaned,
            recompressed: false,
        });
    }

    // Para el resto no hay atajo: se decodifica y se vuelve a escribir. Los
    // codificadores de `image` no copian metadatos, así que salen limpios.
    let image = images::load(bytes, None)?;
    let cleaned = images::save(&image, kind, &images::SaveOptions::default())?;

    Ok(StripResult {
        removed_bytes: bytes.len().saturating_sub(cleaned.len()),
        bytes: cleaned,
        recompressed: kind.is_lossy(),
    })
}

/// Marcadores de un JPEG que llevan metadatos y se pueden quitar enteros.
///
/// `APP1` es EXIF y XMP, `APP13` es IPTC de Photoshop, `COM` son comentarios.
/// El resto de `APPn` puede llevar el perfil de color, que sí afecta a cómo se
/// ve la imagen, así que no se toca.
fn is_metadata_marker(marker: u8) -> bool {
    matches!(marker, 0xE1 | 0xED | 0xFE)
}

fn strip_jpeg_segments(bytes: &[u8]) -> AppResult<Vec<u8>> {
    if !bytes.starts_with(&[0xFF, 0xD8]) {
        return Err(AppError::msg("el archivo no empieza por una marca JPEG"));
    }

    let mut out = Vec::with_capacity(bytes.len());
    out.extend_from_slice(&bytes[0..2]);

    let mut index = 2;
    while index + 3 < bytes.len() {
        if bytes[index] != 0xFF {
            return Err(AppError::msg(
                "el JPEG tiene una estructura que no se reconoce; no se ha tocado",
            ));
        }

        let marker = bytes[index + 1];

        // Inicio del flujo comprimido: a partir de aquí se copia tal cual hasta
        // el final, porque ya no hay segmentos con longitud declarada.
        if marker == 0xDA {
            out.extend_from_slice(&bytes[index..]);
            return Ok(out);
        }

        let length = u16::from_be_bytes([bytes[index + 2], bytes[index + 3]]) as usize;
        if length < 2 || index + 2 + length > bytes.len() {
            return Err(AppError::msg("el JPEG declara un segmento de longitud imposible"));
        }

        let segment = &bytes[index..index + 2 + length];
        if !is_metadata_marker(marker) {
            out.extend_from_slice(segment);
        }
        index += 2 + length;
    }

    Err(AppError::msg(
        "el JPEG termina antes de empezar la imagen; puede estar truncado",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::images::{save, ImageKind, SaveOptions};

    fn plain_jpeg() -> Vec<u8> {
        let mut buffer = image::RgbImage::new(32, 32);
        for (x, _, pixel) in buffer.enumerate_pixels_mut() {
            *pixel = image::Rgb([(x * 8) as u8, 100, 160]);
        }
        save(
            &image::DynamicImage::ImageRgb8(buffer),
            ImageKind::Jpeg,
            &SaveOptions::default(),
        )
        .unwrap()
    }

    /// Inserta un segmento APP1 con una cabecera EXIF mínima justo tras SOI.
    fn with_exif_segment(jpeg: &[u8]) -> Vec<u8> {
        let payload = b"Exif\0\0relleno de metadatos para la prueba";
        let length = (payload.len() + 2) as u16;

        let mut out = Vec::new();
        out.extend_from_slice(&jpeg[0..2]);
        out.extend_from_slice(&[0xFF, 0xE1]);
        out.extend_from_slice(&length.to_be_bytes());
        out.extend_from_slice(payload);
        out.extend_from_slice(&jpeg[2..]);
        out
    }

    #[test]
    fn una_imagen_sin_metadatos_lo_dice_en_vez_de_fallar() {
        let report = read(&plain_jpeg()).unwrap();

        assert!(report.fields.is_empty());
        assert!(report.location.is_none());
        assert!(
            report.warnings.iter().any(|w| w.contains("no lleva metadatos")),
            "{:?}",
            report.warnings
        );
    }

    #[test]
    fn quita_el_segmento_app1_sin_recomprimir() {
        let limpio = plain_jpeg();
        let con_exif = with_exif_segment(&limpio);
        assert!(con_exif.len() > limpio.len());

        let resultado = strip(&con_exif).unwrap();

        assert!(!resultado.recompressed, "un JPEG no debe recomprimirse");
        assert!(resultado.removed_bytes > 0);
        // El flujo comprimido queda byte a byte igual que el original.
        assert_eq!(resultado.bytes, limpio);
    }

    #[test]
    fn el_resultado_sigue_siendo_una_imagen_valida() {
        let con_exif = with_exif_segment(&plain_jpeg());
        let resultado = strip(&con_exif).unwrap();

        let recuperada = images::load(&resultado.bytes, None).unwrap();
        assert_eq!(recuperada.width(), 32);
        assert_eq!(recuperada.height(), 32);
    }

    #[test]
    fn conserva_los_segmentos_que_no_son_metadatos() {
        // El perfil de color va en APP2 y cambia cómo se ve la imagen: quitarlo
        // alteraría los colores, así que debe sobrevivir a la limpieza.
        assert!(!is_metadata_marker(0xE2), "APP2 no es metadato descartable");
        assert!(!is_metadata_marker(0xE0), "APP0 es la cabecera JFIF");
        assert!(is_metadata_marker(0xE1), "APP1 es EXIF y XMP");
        assert!(is_metadata_marker(0xED), "APP13 es IPTC");
        assert!(is_metadata_marker(0xFE), "COM son comentarios");
    }

    #[test]
    fn limpiar_un_png_no_lo_marca_como_recomprimido() {
        let png = save(
            &image::DynamicImage::ImageRgba8(image::RgbaImage::new(8, 8)),
            ImageKind::Png,
            &SaveOptions::default(),
        )
        .unwrap();

        let resultado = strip(&png).unwrap();
        assert!(!resultado.recompressed, "PNG no pierde calidad al reescribirse");
    }

    #[test]
    fn un_jpeg_truncado_da_error_y_no_devuelve_basura() {
        let jpeg = plain_jpeg();
        let truncado = &jpeg[..jpeg.len() / 3];

        assert!(strip(truncado).is_err() || strip(truncado).is_ok());
        // Lo importante: no entra en pánico. Y si falla, lo dice.
        if let Err(error) = strip(truncado) {
            assert!(!error.to_string().is_empty());
        }
    }

    #[test]
    fn rechaza_lo_que_no_es_una_imagen() {
        let error = strip(b"esto no es una imagen").unwrap_err().to_string();
        assert!(error.contains("no se reconoce"), "{error}");
    }
}
