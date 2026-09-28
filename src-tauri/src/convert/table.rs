//! Puente entre datos anidados y formatos tabulares.
//!
//! CSV, TSV y XLSX son rejillas planas; JSON, YAML, TOML y XML son árboles.
//! Aquí vive la traducción en ambos sentidos: aplanar con claves separadas por
//! puntos (`cliente.direccion.ciudad`, `items.0.sku`) y reconstruir el árbol a
//! partir de esas claves.
//!
//! La conversión a tabla es **con pérdida** en un caso: un objeto o lista vacía
//! se escribe como celda vacía y al volver no se distingue de un nulo. No hay
//! forma de representarlo en una rejilla sin inventar una convención que luego
//! rompería los archivos al abrirlos en Excel.

use serde_json::{Map, Number, Value};

/// Clave de la columna cuando las filas no son objetos (una lista de escalares).
const SCALAR_COLUMN: &str = "value";

#[derive(Debug, Clone, PartialEq)]
pub struct Table {
    pub headers: Vec<String>,
    pub rows: Vec<Vec<Value>>,
}

impl Table {
    /// Aplana un valor a rejilla.
    ///
    /// Una lista produce una fila por elemento; cualquier otra cosa produce una
    /// sola fila. Las cabeceras son la unión de las claves en el orden en que
    /// aparecen, no ordenadas alfabéticamente: el orden del archivo original es
    /// información que el usuario espera conservar.
    pub fn from_value(value: &Value, separator: &str) -> Table {
        let records: Vec<&Value> = match value {
            Value::Array(items) => items.iter().collect(),
            other => vec![other],
        };

        let mut headers: Vec<String> = Vec::new();
        let mut flattened: Vec<Vec<(String, Value)>> = Vec::with_capacity(records.len());

        for record in records {
            let mut pairs = Vec::new();
            flatten_into("", record, separator, &mut pairs);
            for (key, _) in &pairs {
                if !headers.iter().any(|h| h == key) {
                    headers.push(key.clone());
                }
            }
            flattened.push(pairs);
        }

        let rows = flattened
            .into_iter()
            .map(|pairs| {
                headers
                    .iter()
                    .map(|header| {
                        pairs
                            .iter()
                            .find(|(key, _)| key == header)
                            .map(|(_, value)| value.clone())
                            .unwrap_or(Value::Null)
                    })
                    .collect()
            })
            .collect();

        Table { headers, rows }
    }

    /// Reconstruye el árbol: siempre una lista de objetos, una por fila.
    pub fn into_value(self, separator: &str) -> Value {
        let Table { headers, rows } = self;

        let records = rows
            .into_iter()
            .map(|cells| {
                let mut record = Value::Object(Map::new());
                for (header, cell) in headers.iter().zip(cells) {
                    let path: Vec<&str> = header.split(separator).collect();
                    insert_path(&mut record, &path, cell);
                }
                record
            })
            .collect();

        Value::Array(records)
    }
}

fn flatten_into(prefix: &str, value: &Value, separator: &str, out: &mut Vec<(String, Value)>) {
    let key = |segment: &str| {
        if prefix.is_empty() {
            segment.to_string()
        } else {
            format!("{prefix}{separator}{segment}")
        }
    };

    match value {
        Value::Object(map) if !map.is_empty() => {
            for (name, child) in map {
                flatten_into(&key(name), child, separator, out);
            }
        }
        Value::Array(items) if !items.is_empty() => {
            for (index, child) in items.iter().enumerate() {
                flatten_into(&key(&index.to_string()), child, separator, out);
            }
        }
        // Escalares y contenedores vacíos. Sin prefijo significa que el valor
        // entero era un escalar, así que necesita un nombre de columna.
        scalar => {
            let name = if prefix.is_empty() {
                SCALAR_COLUMN.to_string()
            } else {
                prefix.to_string()
            };
            let cell = match scalar {
                Value::Object(_) | Value::Array(_) => Value::Null,
                other => other.clone(),
            };
            out.push((name, cell));
        }
    }
}

fn insert_path(target: &mut Value, path: &[&str], value: Value) {
    let Some((head, rest)) = path.split_first() else {
        return;
    };
    if rest.is_empty() {
        *slot(target, head) = value;
    } else {
        insert_path(slot(target, head), rest, value);
    }
}

/// Devuelve el hueco donde escribir `key` dentro de `parent`, creando el
/// contenedor adecuado. Un segmento de solo dígitos significa índice de lista;
/// cualquier otro, clave de objeto.
fn slot<'a>(parent: &'a mut Value, key: &str) -> &'a mut Value {
    match as_index(key) {
        Some(index) => {
            if !parent.is_array() {
                *parent = Value::Array(Vec::new());
            }
            let array = parent.as_array_mut().expect("acabamos de garantizar lista");
            while array.len() <= index {
                array.push(Value::Null);
            }
            &mut array[index]
        }
        None => {
            if !parent.is_object() {
                *parent = Value::Object(Map::new());
            }
            parent
                .as_object_mut()
                .expect("acabamos de garantizar objeto")
                .entry(key.to_string())
                .or_insert(Value::Null)
        }
    }
}

fn as_index(segment: &str) -> Option<usize> {
    if segment.is_empty() || !segment.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    // «007» es un código, no el índice 7.
    if segment.len() > 1 && segment.starts_with('0') {
        return None;
    }
    segment.parse().ok()
}

/// Convierte el texto de una celda al tipo que aparenta.
///
/// Conservador a propósito: los ceros a la izquierda y el `+` inicial se
/// respetan como texto porque son códigos postales, identificadores y
/// teléfonos, y convertirlos a número los destruye.
pub fn infer_scalar(raw: &str) -> Value {
    let text = raw.trim();
    if text.is_empty() {
        return Value::Null;
    }

    match text.to_ascii_lowercase().as_str() {
        "true" => return Value::Bool(true),
        "false" => return Value::Bool(false),
        "null" | "nil" | "~" => return Value::Null,
        _ => {}
    }

    if looks_like_code(text) {
        return Value::String(raw.to_string());
    }

    if let Ok(integer) = text.parse::<i64>() {
        return Value::Number(integer.into());
    }
    if let Ok(float) = text.parse::<f64>() {
        if let Some(number) = Number::from_f64(float) {
            return Value::Number(number);
        }
    }

    Value::String(raw.to_string())
}

fn looks_like_code(text: &str) -> bool {
    if text.starts_with('+') {
        return true;
    }
    let digits = text.strip_prefix('-').unwrap_or(text);
    digits.len() > 1 && digits.starts_with('0') && !digits.starts_with("0.")
}

/// Representación en celda de un valor ya aplanado.
pub fn cell_to_string(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::String(text) => text.clone(),
        Value::Bool(flag) => flag.to_string(),
        Value::Number(number) => number.to_string(),
        nested => nested.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const SEP: &str = ".";

    #[test]
    fn aplana_lista_de_objetos_conservando_el_orden_de_claves() {
        let value = json!([
            {"nombre": "Ana", "edad": 33},
            {"nombre": "Luis", "edad": 41}
        ]);
        let table = Table::from_value(&value, SEP);

        assert_eq!(table.headers, vec!["nombre", "edad"]);
        assert_eq!(table.rows.len(), 2);
        assert_eq!(table.rows[0], vec![json!("Ana"), json!(33)]);
    }

    #[test]
    fn la_union_de_claves_rellena_huecos_con_nulo() {
        let value = json!([{"a": 1}, {"b": 2}]);
        let table = Table::from_value(&value, SEP);

        assert_eq!(table.headers, vec!["a", "b"]);
        assert_eq!(table.rows[0], vec![json!(1), Value::Null]);
        assert_eq!(table.rows[1], vec![Value::Null, json!(2)]);
    }

    #[test]
    fn aplana_objetos_anidados_con_puntos() {
        let value = json!([{"cliente": {"direccion": {"ciudad": "Vigo"}}}]);
        let table = Table::from_value(&value, SEP);

        assert_eq!(table.headers, vec!["cliente.direccion.ciudad"]);
        assert_eq!(table.rows[0], vec![json!("Vigo")]);
    }

    #[test]
    fn aplana_listas_anidadas_con_indices() {
        let value = json!([{"items": [{"sku": "A"}, {"sku": "B"}]}]);
        let table = Table::from_value(&value, SEP);

        assert_eq!(table.headers, vec!["items.0.sku", "items.1.sku"]);
    }

    #[test]
    fn un_escalar_suelto_recibe_nombre_de_columna() {
        let table = Table::from_value(&json!(["a", "b"]), SEP);
        assert_eq!(table.headers, vec![SCALAR_COLUMN]);
        assert_eq!(table.rows.len(), 2);
    }

    #[test]
    fn un_objeto_suelto_es_una_sola_fila() {
        let table = Table::from_value(&json!({"a": 1, "b": 2}), SEP);
        assert_eq!(table.headers, vec!["a", "b"]);
        assert_eq!(table.rows.len(), 1);
    }

    #[test]
    fn ida_y_vuelta_de_estructura_anidada() {
        let original = json!([
            {"id": 1, "cliente": {"nombre": "Ana"}, "tags": ["x", "y"]},
            {"id": 2, "cliente": {"nombre": "Luis"}, "tags": ["z"]}
        ]);

        let recovered = Table::from_value(&original, SEP).into_value(SEP);

        // La segunda fila no tiene tags.1, así que queda explícitamente nula.
        assert_eq!(recovered[0], original[0]);
        assert_eq!(recovered[1]["id"], json!(2));
        assert_eq!(recovered[1]["cliente"]["nombre"], json!("Luis"));
        assert_eq!(recovered[1]["tags"][0], json!("z"));
    }

    #[test]
    fn infiere_tipos_basicos() {
        assert_eq!(infer_scalar("42"), json!(42));
        assert_eq!(infer_scalar("-7"), json!(-7));
        assert_eq!(infer_scalar("3.5"), json!(3.5));
        assert_eq!(infer_scalar("true"), json!(true));
        assert_eq!(infer_scalar("FALSE"), json!(false));
        assert_eq!(infer_scalar("null"), Value::Null);
        assert_eq!(infer_scalar("   "), Value::Null);
        assert_eq!(infer_scalar("hola"), json!("hola"));
    }

    #[test]
    fn no_destruye_codigos_que_parecen_numeros() {
        // Códigos postales, identificadores y teléfonos.
        assert_eq!(infer_scalar("007"), json!("007"));
        assert_eq!(infer_scalar("08001"), json!("08001"));
        assert_eq!(infer_scalar("+34600112233"), json!("+34600112233"));
        // Un decimal que empieza por cero sí es un número.
        assert_eq!(infer_scalar("0.5"), json!(0.5));
        assert_eq!(infer_scalar("0"), json!(0));
    }

    #[test]
    fn los_indices_distinguen_de_las_claves_numericas() {
        assert_eq!(as_index("0"), Some(0));
        assert_eq!(as_index("12"), Some(12));
        assert_eq!(as_index("007"), None);
        assert_eq!(as_index("a"), None);
        assert_eq!(as_index(""), None);
    }

    #[test]
    fn las_celdas_nulas_se_escriben_vacias() {
        assert_eq!(cell_to_string(&Value::Null), "");
        assert_eq!(cell_to_string(&json!("texto")), "texto");
        assert_eq!(cell_to_string(&json!(12)), "12");
        assert_eq!(cell_to_string(&json!(true)), "true");
    }
}
