use serde::Serialize;
use serde_json::{ser::PrettyFormatter, Serializer, Value};

use super::{as_text, Codec};
use crate::convert::{ConvertError, ConvertResult, Format, Opts};

pub struct Json;

impl Codec for Json {
    fn decode(&self, bytes: &[u8], _opts: &Opts) -> ConvertResult<Value> {
        serde_json::from_str(as_text(bytes)?).map_err(|e| ConvertError::parse(Format::Json, e))
    }

    fn encode(&self, value: &Value, opts: &Opts) -> ConvertResult<Vec<u8>> {
        let indent = opts.effective_indent();
        if indent == 0 {
            return serde_json::to_vec(value)
                .map_err(|e| ConvertError::encode(Format::Json, e));
        }

        let spaces = " ".repeat(indent);
        let mut out = Vec::new();
        let mut serializer =
            Serializer::with_formatter(&mut out, PrettyFormatter::with_indent(spaces.as_bytes()));
        value
            .serialize(&mut serializer)
            .map_err(|e| ConvertError::encode(Format::Json, e))?;
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn respeta_la_indentacion_pedida() {
        let opts = Opts {
            indent: 4,
            ..Opts::default()
        };
        let out = Json.encode(&json!({"a": 1}), &opts).unwrap();
        assert_eq!(String::from_utf8(out).unwrap(), "{\n    \"a\": 1\n}");
    }

    #[test]
    fn conserva_el_orden_de_las_claves() {
        // Con la opción `preserve_order` de serde_json el orden del archivo
        // original se mantiene; sin ella saldrían alfabéticas y el diff contra
        // el archivo de partida sería ilegible.
        let opts = Opts {
            pretty: false,
            ..Opts::default()
        };
        let value: Value = serde_json::from_str(r#"{"z":1,"a":2,"m":3}"#).unwrap();
        let out = Json.encode(&value, &opts).unwrap();
        assert_eq!(String::from_utf8(out).unwrap(), r#"{"z":1,"a":2,"m":3}"#);
    }

    #[test]
    fn conserva_enteros_grandes() {
        let opts = Opts {
            pretty: false,
            ..Opts::default()
        };
        let entrada = r#"{"id":9007199254740993}"#;
        let value = Json.decode(entrada.as_bytes(), &opts).unwrap();
        let out = Json.encode(&value, &opts).unwrap();
        assert_eq!(String::from_utf8(out).unwrap(), entrada);
    }
}
