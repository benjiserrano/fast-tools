//! Un codec por formato: del formato al pivote y del pivote al formato.

mod delimited;
mod json;
mod toml_codec;
mod xlsx;
mod xml;
mod yaml;

use serde_json::Value;

use super::{ConvertError, ConvertResult, Format, Opts};

pub trait Codec: Sync {
    fn decode(&self, bytes: &[u8], opts: &Opts) -> ConvertResult<Value>;
    fn encode(&self, value: &Value, opts: &Opts) -> ConvertResult<Vec<u8>>;
}

pub fn for_format(format: Format) -> &'static dyn Codec {
    match format {
        Format::Json => &json::Json,
        Format::Yaml => &yaml::Yaml,
        Format::Toml => &toml_codec::Toml,
        Format::Xml => &xml::Xml,
        Format::Csv => &delimited::CSV,
        Format::Tsv => &delimited::TSV,
        Format::Xlsx => &xlsx::Xlsx,
    }
}

/// Bytes a texto, quitando la marca de orden.
///
/// Los archivos exportados desde Excel y desde PowerShell llevan BOM UTF-8 muy
/// a menudo. Dejarla pasar convierte la primera cabecera en `\u{feff}nombre` y
/// la conversión produce una columna fantasma que nadie entiende.
pub(crate) fn as_text(bytes: &[u8]) -> ConvertResult<&str> {
    let text = std::str::from_utf8(bytes).map_err(|_| ConvertError::NotUtf8)?;
    Ok(text.strip_prefix('\u{feff}').unwrap_or(text))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quita_la_marca_de_orden_de_bytes() {
        let con_bom = "\u{feff}nombre,edad";
        assert_eq!(as_text(con_bom.as_bytes()).unwrap(), "nombre,edad");
        assert_eq!(as_text(b"nombre,edad").unwrap(), "nombre,edad");
    }

    #[test]
    fn rechaza_lo_que_no_es_utf8() {
        assert!(matches!(
            as_text(&[0xff, 0xfe, 0x00]),
            Err(ConvertError::NotUtf8)
        ));
    }
}
