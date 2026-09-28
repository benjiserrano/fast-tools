//! Libros de Excel.
//!
//! Solo se lee y escribe una hoja: la conversión trata el libro como una tabla,
//! que es lo que se espera al pasarlo a CSV o JSON. La hoja se elige con
//! `opts.sheet`; sin ella, la primera.

use calamine::{Data, Reader};
use rust_xlsxwriter::{Format as CellFormat, Workbook};
use serde_json::{Number, Value};

use super::Codec;
use crate::convert::table::{infer_scalar, Table};
use crate::convert::{ConvertError, ConvertResult, Format, Opts};

pub struct Xlsx;

/// Más allá de esto un entero no sobrevive al `f64` en que Excel guarda todo,
/// así que se deja como decimal en vez de fingir precisión que no hay.
const MAX_EXACT_INTEGER: f64 = 9_007_199_254_740_992.0;

impl Codec for Xlsx {
    fn decode(&self, bytes: &[u8], opts: &Opts) -> ConvertResult<Value> {
        let cursor = std::io::Cursor::new(bytes.to_vec());
        let mut workbook = calamine::open_workbook_auto_from_rs(cursor)
            .map_err(|e| ConvertError::parse(Format::Xlsx, e))?;

        let sheet = match &opts.sheet {
            Some(name) => name.clone(),
            None => workbook
                .sheet_names()
                .first()
                .cloned()
                .ok_or_else(|| ConvertError::parse(Format::Xlsx, "el libro no tiene hojas"))?,
        };

        let range = workbook.worksheet_range(&sheet).map_err(|e| {
            ConvertError::parse(Format::Xlsx, format!("no se pudo leer la hoja «{sheet}»: {e}"))
        })?;

        let mut rows = range.rows();
        let Some(header_row) = rows.next() else {
            return Ok(Value::Array(Vec::new()));
        };

        let headers: Vec<String> = header_row
            .iter()
            .enumerate()
            .map(|(index, cell)| match cell_to_value(cell, false) {
                Value::String(name) if !name.trim().is_empty() => name.trim().to_string(),
                // Una cabecera vacía dejaría una columna sin nombre y todas sus
                // celdas se perderían al construir el objeto.
                _ => format!("columna{}", index + 1),
            })
            .collect();

        let rows = rows
            .map(|row| {
                let mut cells: Vec<Value> = row
                    .iter()
                    .map(|cell| cell_to_value(cell, opts.infer_types))
                    .collect();
                cells.resize(headers.len(), Value::Null);
                cells.truncate(headers.len());
                cells
            })
            .collect();

        Ok(Table { headers, rows }.into_value(opts.separator_or_default()))
    }

    fn encode(&self, value: &Value, opts: &Opts) -> ConvertResult<Vec<u8>> {
        let table = Table::from_value(value, opts.separator_or_default());

        let mut workbook = Workbook::new();
        let worksheet = workbook.add_worksheet();
        if let Some(name) = &opts.sheet {
            worksheet
                .set_name(name)
                .map_err(|e| ConvertError::encode(Format::Xlsx, e))?;
        }

        let header_format = CellFormat::new().set_bold();
        for (column, header) in table.headers.iter().enumerate() {
            let column = column_index(column)?;
            worksheet
                .write_string_with_format(0, column, header, &header_format)
                .map_err(|e| ConvertError::encode(Format::Xlsx, e))?;
        }

        for (row_index, row) in table.rows.iter().enumerate() {
            // +1 por la fila de cabeceras.
            let row_number = u32::try_from(row_index + 1)
                .map_err(|_| ConvertError::encode(Format::Xlsx, "demasiadas filas para una hoja"))?;
            for (column_index_usize, cell) in row.iter().enumerate() {
                let column = column_index(column_index_usize)?;
                write_cell(worksheet, row_number, column, cell)?;
            }
        }

        worksheet.autofit();
        workbook
            .save_to_buffer()
            .map_err(|e| ConvertError::encode(Format::Xlsx, e))
    }
}

fn column_index(index: usize) -> ConvertResult<u16> {
    u16::try_from(index).map_err(|_| {
        ConvertError::encode(
            Format::Xlsx,
            "demasiadas columnas: una hoja de Excel admite 16 384",
        )
    })
}

fn write_cell(
    worksheet: &mut rust_xlsxwriter::Worksheet,
    row: u32,
    column: u16,
    cell: &Value,
) -> ConvertResult<()> {
    let fail = |e: rust_xlsxwriter::XlsxError| ConvertError::encode(Format::Xlsx, e);

    match cell {
        // Una celda nula se deja sin escribir: en Excel «vacío» y «cadena
        // vacía» son cosas distintas, y el nulo es lo primero.
        Value::Null => Ok(()),
        Value::Bool(flag) => worksheet.write_boolean(row, column, *flag).map(drop).map_err(fail),
        Value::Number(number) => match number.as_f64() {
            Some(float) => worksheet.write_number(row, column, float).map(drop).map_err(fail),
            None => worksheet
                .write_string(row, column, number.to_string())
                .map(drop)
                .map_err(fail),
        },
        Value::String(text) => worksheet.write_string(row, column, text).map(drop).map_err(fail),
        nested => worksheet
            .write_string(row, column, nested.to_string())
            .map(drop)
            .map_err(fail),
    }
}

fn cell_to_value(cell: &Data, infer: bool) -> Value {
    match cell {
        Data::Empty => Value::Null,
        Data::Bool(flag) => Value::Bool(*flag),
        Data::Int(integer) => Value::Number((*integer).into()),
        Data::Float(float) => number_from_f64(*float),
        Data::String(text) => {
            if infer {
                infer_scalar(text)
            } else {
                Value::String(text.clone())
            }
        }
        Data::DateTime(stamp) => match stamp.as_datetime() {
            // ISO 8601 para que el valor siga siendo una fecha al otro lado y
            // no un número de serie de Excel que nadie sabe interpretar.
            Some(datetime) => Value::String(datetime.format("%Y-%m-%dT%H:%M:%S").to_string()),
            None => number_from_f64(stamp.as_f64()),
        },
        Data::DateTimeIso(text) | Data::DurationIso(text) => Value::String(text.clone()),
        Data::Error(error) => Value::String(format!("{error:?}")),
    }
}

/// Excel guarda todo número como `f64`. Los que son enteros exactos vuelven a
/// entero para que una ida y vuelta desde JSON no convierta `33` en `33.0`.
fn number_from_f64(float: f64) -> Value {
    if float.fract() == 0.0 && float.abs() <= MAX_EXACT_INTEGER {
        return Value::Number((float as i64).into());
    }
    Number::from_f64(float)
        .map(Value::Number)
        .unwrap_or(Value::Null)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn ida_y_vuelta_conserva_tipos_y_orden() {
        let opts = Opts::default();
        let original = json!([
            {"nombre": "Ana", "edad": 33, "activo": true},
            {"nombre": "Luis", "edad": 41, "activo": false}
        ]);

        let libro = Xlsx.encode(&original, &opts).unwrap();
        let recuperado = Xlsx.decode(&libro, &opts).unwrap();

        assert_eq!(recuperado, original);
    }

    #[test]
    fn el_resultado_es_un_zip_de_excel() {
        let libro = Xlsx.encode(&json!([{"a": 1}]), &Opts::default()).unwrap();
        assert!(libro.starts_with(b"PK\x03\x04"), "debe ser un archivo ZIP");
    }

    #[test]
    fn los_enteros_no_se_vuelven_decimales() {
        assert_eq!(number_from_f64(33.0), json!(33));
        assert_eq!(number_from_f64(-7.0), json!(-7));
        assert_eq!(number_from_f64(0.5), json!(0.5));
    }

    #[test]
    fn los_enteros_fuera_de_rango_se_quedan_como_decimales() {
        let enorme = MAX_EXACT_INTEGER * 4.0;
        assert!(number_from_f64(enorme).is_f64());
    }

    #[test]
    fn las_celdas_nulas_quedan_vacias_y_vuelven_nulas() {
        let opts = Opts::default();
        let original = json!([{"a": 1, "b": null}]);

        let libro = Xlsx.encode(&original, &opts).unwrap();
        let recuperado = Xlsx.decode(&libro, &opts).unwrap();

        assert_eq!(recuperado[0]["b"], Value::Null);
    }

    #[test]
    fn respeta_el_nombre_de_hoja_pedido() {
        let opts = Opts {
            sheet: Some("Datos".to_string()),
            ..Opts::default()
        };
        let libro = Xlsx.encode(&json!([{"a": 1}]), &opts).unwrap();

        let cursor = std::io::Cursor::new(libro);
        let workbook = calamine::open_workbook_auto_from_rs(cursor).unwrap();
        assert_eq!(workbook.sheet_names(), vec!["Datos".to_string()]);
    }

    #[test]
    fn las_fechas_salen_en_iso_y_no_como_numero_de_serie() {
        // Excel guarda las fechas como días desde 1900. Sin traducirlas, un
        // CSV exportado traería «45658» donde debería poner la fecha.
        let mut workbook = Workbook::new();
        let worksheet = workbook.add_worksheet();
        let fecha = CellFormat::new().set_num_format("yyyy-mm-dd");
        worksheet.write_string(0, 0, "vencimiento").unwrap();
        worksheet
            .write_datetime_with_format(
                1,
                0,
                rust_xlsxwriter::ExcelDateTime::from_ymd(2026, 3, 14).unwrap(),
                &fecha,
            )
            .unwrap();
        let libro = workbook.save_to_buffer().unwrap();

        let recuperado = Xlsx.decode(&libro, &Opts::default()).unwrap();
        let celda = recuperado[0]["vencimiento"].as_str().unwrap_or_default();

        assert!(
            celda.starts_with("2026-03-14"),
            "esperaba una fecha ISO, salió {celda:?}"
        );
    }

    #[test]
    fn una_cabecera_vacia_recibe_nombre_generado() {
        // Caso real: una hoja exportada donde la segunda columna tiene datos
        // pero no título. Sin un nombre generado, esa columna se perdería.
        let mut workbook = Workbook::new();
        let worksheet = workbook.add_worksheet();
        worksheet.write_string(0, 0, "a").unwrap();
        // La celda (0, 1) se deja en blanco a propósito.
        worksheet.write_number(1, 0, 1.0).unwrap();
        worksheet.write_number(1, 1, 2.0).unwrap();
        let libro = workbook.save_to_buffer().unwrap();

        let recuperado = Xlsx.decode(&libro, &Opts::default()).unwrap();

        assert_eq!(recuperado[0]["a"], json!(1));
        assert_eq!(recuperado[0]["columna2"], json!(2));
    }
}
