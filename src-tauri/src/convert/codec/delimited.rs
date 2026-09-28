//! CSV y TSV: el mismo codec con distinto separador.

use serde_json::Value;

use super::{as_text, Codec};
use crate::convert::table::{cell_to_string, infer_scalar, Table};
use crate::convert::{ConvertError, ConvertResult, Format, Opts};

pub struct Delimited(Format);

pub static CSV: Delimited = Delimited(Format::Csv);
pub static TSV: Delimited = Delimited(Format::Tsv);

impl Delimited {
    fn delimiter(&self) -> u8 {
        self.0.delimiter().unwrap_or(b',')
    }

    fn input_delimiter(&self, text: &str) -> u8 {
        match self.0 {
            // Muchos archivos llamados CSV usan punto y coma (Excel con
            // configuración regional europea), tabulador o barra vertical.
            // Para escribir conservamos la coma estándar; al leer aceptamos
            // esas variantes sin pedir al usuario que conozca el delimitador.
            Format::Csv => detect_delimiter(text).unwrap_or_else(|| self.delimiter()),
            _ => self.delimiter(),
        }
    }
}

const DELIMITER_CANDIDATES: [u8; 4] = [b',', b';', b'\t', b'|'];

/// Deduce el delimitador usando varios registros lógicos, no líneas físicas.
/// Así ignora delimitadores y saltos de línea encerrados entre comillas.
fn detect_delimiter(text: &str) -> Option<u8> {
    let records = delimiter_counts(text, 32);
    let first = records.first()?;

    DELIMITER_CANDIDATES
        .iter()
        .enumerate()
        .filter(|(index, _)| first[*index] > 0)
        .max_by_key(|(index, _)| {
            let expected = first[*index];
            let matching_records = records
                .iter()
                .filter(|counts| counts[*index] == expected)
                .count();
            let records_with_delimiter = records
                .iter()
                .filter(|counts| counts[*index] > 0)
                .count();

            (
                matching_records,
                records_with_delimiter,
                expected,
                DELIMITER_CANDIDATES.len() - *index,
            )
        })
        .map(|(_, delimiter)| *delimiter)
}

fn delimiter_counts(text: &str, limit: usize) -> Vec<[usize; DELIMITER_CANDIDATES.len()]> {
    let bytes = text.as_bytes();
    let mut records = Vec::new();
    let mut counts = [0; DELIMITER_CANDIDATES.len()];
    let mut in_quotes = false;
    let mut has_content = false;
    let mut index = 0;

    while index < bytes.len() && records.len() < limit {
        let byte = bytes[index];
        if byte == b'"' {
            if in_quotes && bytes.get(index + 1) == Some(&b'"') {
                index += 1;
            } else {
                in_quotes = !in_quotes;
            }
            has_content = true;
        } else if !in_quotes && (byte == b'\n' || byte == b'\r') {
            if has_content {
                records.push(counts);
                counts = [0; DELIMITER_CANDIDATES.len()];
                has_content = false;
            }
            if byte == b'\r' && bytes.get(index + 1) == Some(&b'\n') {
                index += 1;
            }
        } else {
            has_content |= !byte.is_ascii_whitespace();
            if !in_quotes {
                if let Some(position) = DELIMITER_CANDIDATES
                    .iter()
                    .position(|candidate| *candidate == byte)
                {
                    counts[position] += 1;
                }
            }
        }
        index += 1;
    }

    if has_content && records.len() < limit {
        records.push(counts);
    }
    records
}

impl Codec for Delimited {
    fn decode(&self, bytes: &[u8], opts: &Opts) -> ConvertResult<Value> {
        let text = as_text(bytes)?;

        let mut reader = csv::ReaderBuilder::new()
            .delimiter(self.input_delimiter(text))
            .has_headers(true)
            // Las exportaciones reales traen filas con columnas de más o de
            // menos. Abortar por eso obligaría a arreglar el archivo a mano
            // antes de poder mirarlo, que es justo lo que se quiere evitar.
            .flexible(true)
            .from_reader(text.as_bytes());

        let headers: Vec<String> = reader
            .headers()
            .map_err(|e| ConvertError::parse(self.0, e))?
            .iter()
            .map(str::to_string)
            .collect();

        let mut rows = Vec::new();
        for record in reader.records() {
            let record = record.map_err(|e| ConvertError::parse(self.0, e))?;
            let mut cells: Vec<Value> = record
                .iter()
                .map(|cell| {
                    if opts.infer_types {
                        infer_scalar(cell)
                    } else {
                        Value::String(cell.to_string())
                    }
                })
                .collect();
            cells.resize(headers.len(), Value::Null);
            rows.push(cells);
        }

        Ok(Table { headers, rows }.into_value(opts.separator_or_default()))
    }

    fn encode(&self, value: &Value, opts: &Opts) -> ConvertResult<Vec<u8>> {
        let table = Table::from_value(value, opts.separator_or_default());

        let mut writer = csv::WriterBuilder::new()
            .delimiter(self.delimiter())
            .from_writer(Vec::new());

        writer
            .write_record(&table.headers)
            .map_err(|e| ConvertError::encode(self.0, e))?;
        for row in &table.rows {
            writer
                .write_record(row.iter().map(cell_to_string))
                .map_err(|e| ConvertError::encode(self.0, e))?;
        }

        writer
            .into_inner()
            .map_err(|e| ConvertError::encode(self.0, e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn text(bytes: Vec<u8>) -> String {
        String::from_utf8(bytes).unwrap()
    }

    #[test]
    fn lee_cabeceras_y_tipa_las_celdas() {
        let entrada = "nombre,edad,activo\nAna,33,true\n";
        let value = CSV.decode(entrada.as_bytes(), &Opts::default()).unwrap();

        assert_eq!(value[0]["nombre"], json!("Ana"));
        assert_eq!(value[0]["edad"], json!(33));
        assert_eq!(value[0]["activo"], json!(true));
    }

    #[test]
    fn sin_inferencia_todo_es_texto() {
        let opts = Opts {
            infer_types: false,
            ..Opts::default()
        };
        let value = CSV.decode(b"edad\n33\n", &opts).unwrap();
        assert_eq!(value[0]["edad"], json!("33"));
    }

    #[test]
    fn respeta_comas_y_comillas_dentro_de_los_campos() {
        let entrada = "nombre,nota\n\"Pérez, Ana\",\"dijo \"\"hola\"\"\"\n";
        let value = CSV.decode(entrada.as_bytes(), &Opts::default()).unwrap();

        assert_eq!(value[0]["nombre"], json!("Pérez, Ana"));
        assert_eq!(value[0]["nota"], json!("dijo \"hola\""));

        // Y al escribir vuelve a citarlos igual.
        let salida = text(CSV.encode(&value, &Opts::default()).unwrap());
        assert!(salida.contains("\"Pérez, Ana\""), "{salida}");
    }

    #[test]
    fn acepta_filas_con_columnas_de_menos() {
        let entrada = "a,b,c\n1,2\n";
        let value = CSV.decode(entrada.as_bytes(), &Opts::default()).unwrap();

        assert_eq!(value[0]["a"], json!(1));
        assert_eq!(value[0]["c"], Value::Null);
    }

    #[test]
    fn tolera_saltos_de_linea_de_windows() {
        let value = CSV
            .decode(b"a,b\r\n1,2\r\n", &Opts::default())
            .unwrap();
        assert_eq!(value[0]["b"], json!(2));
    }

    #[test]
    fn tsv_usa_tabulador() {
        let value = TSV.decode(b"a\tb\n1\t2\n", &Opts::default()).unwrap();
        assert_eq!(value[0]["b"], json!(2));

        let salida = text(TSV.encode(&value, &Opts::default()).unwrap());
        assert!(salida.starts_with("a\tb"), "{salida:?}");
    }

    #[test]
    fn csv_detecta_delimitadores_habituales() {
        for (entrada, columna) in [
            ("a,b\n1,2\n", "b"),
            ("a;b\n1;2\n", "b"),
            ("a\tb\n1\t2\n", "b"),
            ("a|b\n1|2\n", "b"),
        ] {
            let value = CSV.decode(entrada.as_bytes(), &Opts::default()).unwrap();
            assert_eq!(value[0][columna], json!(2), "entrada: {entrada:?}");
        }
    }

    #[test]
    fn deteccion_ignora_delimitadores_y_saltos_dentro_de_comillas() {
        let entrada = "nombre;nota\nAna;\"uno,dos\n| tres\"\n";
        let value = CSV.decode(entrada.as_bytes(), &Opts::default()).unwrap();

        assert_eq!(value[0]["nombre"], json!("Ana"));
        assert_eq!(value[0]["nota"], json!("uno,dos\n| tres"));
    }

    #[test]
    fn aplana_lo_anidado_al_escribir() {
        let value = json!([{"cliente": {"nombre": "Ana"}}]);
        let salida = text(CSV.encode(&value, &Opts::default()).unwrap());
        assert!(salida.starts_with("cliente.nombre"), "{salida}");
    }
}
