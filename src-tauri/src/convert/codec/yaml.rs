use serde_json::Value;

use super::{as_text, Codec};
use crate::convert::{ConvertError, ConvertResult, Format, Opts};

pub struct Yaml;

impl Codec for Yaml {
    fn decode(&self, bytes: &[u8], _opts: &Opts) -> ConvertResult<Value> {
        // Se pasa primero por el modelo de YAML y no directo al pivote porque
        // las claves de mezcla (`<<: *base`) no son parte de la deserialización:
        // hay que expandirlas a mano con `apply_merge`. Ir directo a JSON
        // dejaría un campo `<<` literal y perdería los valores heredados.
        let mut document: serde_norway::Value =
            serde_norway::from_str(as_text(bytes)?).map_err(|e| ConvertError::parse(Format::Yaml, e))?;
        document
            .apply_merge()
            .map_err(|e| ConvertError::parse(Format::Yaml, e))?;

        serde_json::to_value(document).map_err(|e| {
            ConvertError::parse(
                Format::Yaml,
                format!(
                    "no tiene equivalente en el resto de formatos ({e}). \
                     Suele deberse a claves que no son texto."
                ),
            )
        })
    }

    fn encode(&self, value: &Value, _opts: &Opts) -> ConvertResult<Vec<u8>> {
        // La indentación de YAML no es configurable en el serializador y
        // tampoco tiene modo compacto: el formato ya es su propia forma legible.
        serde_norway::to_string(value)
            .map(String::into_bytes)
            .map_err(|e| ConvertError::encode(Format::Yaml, e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn expande_anclas_y_alias() {
        let entrada = "
base: &base
  region: eu
pro:
  <<: *base
  replicas: 3
";
        let value = Yaml.decode(entrada.as_bytes(), &Opts::default()).unwrap();
        assert_eq!(value["pro"]["region"], json!("eu"));
        assert_eq!(value["pro"]["replicas"], json!(3));
    }

    #[test]
    fn ida_y_vuelta_conserva_tipos() {
        let opts = Opts::default();
        let original = json!({"activo": true, "reintentos": 3, "ratio": 0.5, "nombre": "api"});

        let yaml = Yaml.encode(&original, &opts).unwrap();
        let recuperado = Yaml.decode(&yaml, &opts).unwrap();

        assert_eq!(recuperado, original);
    }

    #[test]
    fn un_yaml_roto_nombra_el_formato() {
        let error = Yaml
            .decode(b"a:\n  - 1\n - 2", &Opts::default())
            .unwrap_err();
        assert!(error.to_string().contains("YAML"));
    }
}
