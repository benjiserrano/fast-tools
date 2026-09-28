use serde_json::Value;

use super::{as_text, Codec};
use crate::convert::{ConvertError, ConvertResult, Format, Opts};

pub struct Toml;

impl Codec for Toml {
    fn decode(&self, bytes: &[u8], _opts: &Opts) -> ConvertResult<Value> {
        toml::from_str(as_text(bytes)?).map_err(|e| ConvertError::parse(Format::Toml, e))
    }

    fn encode(&self, value: &Value, opts: &Opts) -> ConvertResult<Vec<u8>> {
        // TOML solo admite una tabla en la raíz. Convertir una lista implicaría
        // inventar un nombre de tabla que envolviera los datos, y entonces el
        // archivo resultante ya no tendría la forma que el usuario espera.
        // Mejor decirlo que hacerlo a sus espaldas.
        if !value.is_object() {
            return Err(ConvertError::encode(
                Format::Toml,
                format!(
                    "TOML necesita un objeto en la raíz y esto es {}. \
                     Envuelve los datos en un objeto o elige otro formato.",
                    describe(value)
                ),
            ));
        }

        let text = if opts.pretty {
            toml::to_string_pretty(value)
        } else {
            toml::to_string(value)
        };
        text.map(String::into_bytes)
            .map_err(|e| ConvertError::encode(Format::Toml, e))
    }
}

fn describe(value: &Value) -> &'static str {
    match value {
        Value::Array(_) => "una lista",
        Value::String(_) => "un texto",
        Value::Number(_) => "un número",
        Value::Bool(_) => "un booleano",
        Value::Null => "un nulo",
        Value::Object(_) => "un objeto",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn ida_y_vuelta_de_tabla_anidada() {
        let opts = Opts::default();
        let original = json!({
            "servidor": {"host": "localhost", "puerto": 8080},
            "activo": true
        });

        let toml_text = Toml.encode(&original, &opts).unwrap();
        let recuperado = Toml.decode(&toml_text, &opts).unwrap();

        assert_eq!(recuperado, original);
    }

    #[test]
    fn una_lista_en_la_raiz_da_un_error_que_explica_que_hacer() {
        let error = Toml
            .encode(&json!([1, 2, 3]), &Opts::default())
            .unwrap_err()
            .to_string();

        assert!(error.contains("una lista"), "{error}");
        assert!(error.contains("Envuelve"), "{error}");
    }
}
