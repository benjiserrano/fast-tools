//! Motor de conversión entre formatos de datos estructurados.
//!
//! Todos los formatos se traducen a través de un pivote común,
//! [`serde_json::Value`]. Convertir A a B es decodificar A al pivote y
//! codificar el pivote como B, así que añadir un formato nuevo cuesta un codec
//! (dos funciones) y no un conversor por cada pareja existente.
//!
//! El pivote es JSON y no un tipo propio porque su modelo de datos —objetos,
//! listas, cadenas, números, booleanos, nulo— es el denominador común de todos
//! los formatos que se soportan, y porque evita una capa de traducción extra en
//! el formato que más se usa.

pub mod codec;
pub mod external;
pub mod format;
pub mod table;

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub use format::Format;

#[derive(Debug, thiserror::Error)]
pub enum ConvertError {
    #[error("El contenido no es texto UTF-8 válido")]
    NotUtf8,

    #[error("{format} inválido: {message}")]
    Parse {
        format: &'static str,
        message: String,
    },

    #[error("No se puede escribir como {format}: {message}")]
    Encode {
        format: &'static str,
        message: String,
    },

    #[error("Error de E/S: {0}")]
    Io(#[from] std::io::Error),
}

impl ConvertError {
    pub fn parse(format: Format, message: impl std::fmt::Display) -> Self {
        ConvertError::Parse {
            format: format.label(),
            message: message.to_string(),
        }
    }

    pub fn encode(format: Format, message: impl std::fmt::Display) -> Self {
        ConvertError::Encode {
            format: format.label(),
            message: message.to_string(),
        }
    }
}

pub type ConvertResult<T> = Result<T, ConvertError>;

/// Ajustes de conversión. Todos tienen un valor por defecto sensato para que la
/// UI pueda convertir sin pedir nada al usuario.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Opts {
    /// Salida indentada y legible en vez de compacta.
    pub pretty: bool,
    /// Espacios de indentación cuando `pretty`.
    pub indent: u8,
    /// Separador de las claves aplanadas al ir a un formato tabular.
    pub separator: String,
    /// Convertir celdas de texto al tipo que aparentan al leer tablas.
    pub infer_types: bool,
    /// Nombre del elemento raíz al escribir XML sin uno evidente.
    pub xml_root: String,
    /// Hoja concreta del libro de Excel; si falta, la primera.
    pub sheet: Option<String>,
}

impl Default for Opts {
    fn default() -> Self {
        Opts {
            pretty: true,
            indent: 2,
            separator: ".".to_string(),
            infer_types: true,
            xml_root: "root".to_string(),
            sheet: None,
        }
    }
}

impl Opts {
    /// Indentación efectiva; 0 cuando la salida es compacta.
    pub fn effective_indent(&self) -> usize {
        if self.pretty {
            self.indent as usize
        } else {
            0
        }
    }

    pub fn separator_or_default(&self) -> &str {
        if self.separator.is_empty() {
            "."
        } else {
            &self.separator
        }
    }
}

/// Decodifica cualquier formato al pivote.
pub fn decode(format: Format, bytes: &[u8], opts: &Opts) -> ConvertResult<Value> {
    codec::for_format(format).decode(bytes, opts)
}

/// Codifica el pivote a cualquier formato.
pub fn encode(format: Format, value: &Value, opts: &Opts) -> ConvertResult<Vec<u8>> {
    codec::for_format(format).encode(value, opts)
}

/// Conversión completa de un formato a otro.
///
/// Convertir un formato a sí mismo no es un caso trivial que haya que atajar:
/// es la forma de reformatear o normalizar un archivo, así que pasa por el
/// pivote igual que el resto.
pub fn convert(from: Format, to: Format, bytes: &[u8], opts: &Opts) -> ConvertResult<Vec<u8>> {
    let value = decode(from, bytes, opts)?;
    encode(to, &value, opts)
}

/// Formatos a los que se puede convertir desde `from`.
///
/// Hoy son todos, porque cada formato tiene codec en ambos sentidos. La función
/// existe igualmente para que la UI no dé por supuesta esa propiedad: la Fase 4
/// añade formatos que dependen de motores externos y no siempre estarán
/// disponibles.
pub fn available_targets(_from: Format) -> Vec<Format> {
    Format::ALL.to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"[{"nombre":"Ana","edad":33},{"nombre":"Luis","edad":41}]"#;

    fn text(bytes: Vec<u8>) -> String {
        String::from_utf8(bytes).expect("salida de texto")
    }

    /// La única conversión que puede fallar por el formato en sí es escribir
    /// TOML desde datos que no tienen un objeto en la raíz. Cualquier otro
    /// error en esta matriz es una regresión.
    fn es_la_limitacion_conocida_de_toml(to: Format, error: &ConvertError) -> bool {
        to == Format::Toml && error.to_string().contains("TOML necesita un objeto")
    }

    #[test]
    fn convierte_entre_todas_las_parejas_sin_reventar() {
        let opts = Opts::default();
        let value = decode(Format::Json, SAMPLE.as_bytes(), &opts).unwrap();
        let envuelto = serde_json::json!({ "registros": value });

        for from in Format::ALL {
            let encoded = encode(*from, &envuelto, &opts)
                .unwrap_or_else(|e| panic!("no se pudo escribir {}: {e}", from.label()));

            for to in Format::ALL {
                if let Err(e) = convert(*from, *to, &encoded, &opts) {
                    assert!(
                        es_la_limitacion_conocida_de_toml(*to, &e),
                        "{} -> {}: {e}",
                        from.label(),
                        to.label()
                    );
                }
            }
        }
    }

    #[test]
    fn una_tabla_a_toml_explica_por_que_no_puede() {
        // CSV siempre produce una lista de filas, y TOML no admite una lista en
        // la raíz. Es una limitación del formato de destino, no un fallo, y el
        // mensaje tiene que decir qué hacer.
        let error = convert(
            Format::Csv,
            Format::Toml,
            b"nombre,edad\nAna,33\n",
            &Opts::default(),
        )
        .unwrap_err()
        .to_string();

        assert!(error.contains("una lista"), "{error}");
        assert!(error.contains("Envuelve"), "{error}");
    }

    #[test]
    fn ida_y_vuelta_json_yaml_conserva_los_datos() {
        let opts = Opts::default();
        let original = decode(Format::Json, SAMPLE.as_bytes(), &opts).unwrap();

        let yaml = encode(Format::Yaml, &original, &opts).unwrap();
        let recovered = decode(Format::Yaml, &yaml, &opts).unwrap();

        assert_eq!(recovered, original);
    }

    #[test]
    fn ida_y_vuelta_json_csv_conserva_los_datos() {
        let opts = Opts::default();
        let original = decode(Format::Json, SAMPLE.as_bytes(), &opts).unwrap();

        let csv = convert(Format::Json, Format::Csv, SAMPLE.as_bytes(), &opts).unwrap();
        let recovered = decode(Format::Csv, &csv, &opts).unwrap();

        assert_eq!(recovered, original);
        assert!(text(csv).starts_with("nombre,edad"));
    }

    #[test]
    fn la_indentacion_se_respeta() {
        let compact = Opts {
            pretty: false,
            ..Opts::default()
        };
        let salida = text(convert(Format::Json, Format::Json, SAMPLE.as_bytes(), &compact).unwrap());
        assert!(!salida.contains('\n'), "compacto no debe llevar saltos");

        let pretty = Opts::default();
        let salida = text(convert(Format::Json, Format::Json, SAMPLE.as_bytes(), &pretty).unwrap());
        assert!(salida.contains("\n  "), "debe indentar con dos espacios");
    }

    #[test]
    fn un_error_de_sintaxis_explica_el_formato() {
        let error = decode(Format::Json, b"{no es json}", &Opts::default()).unwrap_err();
        let message = error.to_string();
        assert!(message.contains("JSON"), "el mensaje debe nombrar el formato: {message}");
    }
}
