//! Herramientas de texto que necesitan Rust: formateo de SQL y comparación.

use serde::{Deserialize, Serialize};
use similar::{ChangeTag, TextDiff};

// ── SQL ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SqlOptions {
    pub indent: u8,
    /// Palabras reservadas en mayúsculas.
    pub uppercase: bool,
    /// Cada cláusula en su propia línea en lugar de comprimir lo que quepa.
    pub lines_between_queries: u8,
}

impl Default for SqlOptions {
    fn default() -> Self {
        SqlOptions {
            indent: 2,
            uppercase: true,
            lines_between_queries: 1,
        }
    }
}

pub fn format_sql(input: &str, options: &SqlOptions) -> String {
    let formatting = sqlformat::FormatOptions {
        indent: sqlformat::Indent::Spaces(options.indent),
        uppercase: Some(options.uppercase),
        lines_between_queries: options.lines_between_queries,
        ignore_case_convert: None,
    };
    // Sin sustitución de parámetros: los `?` y `$1` de la consulta son parte de
    // lo que el usuario quiere ver formateado, no valores que rellenar.
    sqlformat::format(input, &sqlformat::QueryParams::None, &formatting)
}

// ── Comparación ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DiffGranularity {
    Line,
    Word,
    Char,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ChangeKind {
    Equal,
    Insert,
    Delete,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffPiece {
    pub kind: ChangeKind,
    pub text: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffLine {
    pub kind: ChangeKind,
    /// Número de línea en el texto de la izquierda, si existe allí.
    pub left_number: Option<usize>,
    /// Número de línea en el texto de la derecha, si existe allí.
    pub right_number: Option<usize>,
    /// La línea partida en trozos, para resaltar qué palabras cambiaron.
    pub pieces: Vec<DiffPiece>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffResult {
    pub lines: Vec<DiffLine>,
    pub added: usize,
    pub removed: usize,
    pub identical: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct DiffOptions {
    pub granularity: DiffGranularity,
    /// Trata como iguales las líneas que solo difieren en mayúsculas.
    pub ignore_case: bool,
    /// Trata como iguales las líneas que solo difieren en espacios.
    pub ignore_whitespace: bool,
}

impl Default for DiffOptions {
    fn default() -> Self {
        DiffOptions {
            granularity: DiffGranularity::Line,
            ignore_case: false,
            ignore_whitespace: false,
        }
    }
}

pub fn diff(left: &str, right: &str, options: &DiffOptions) -> DiffResult {
    // La comparación se hace sobre una versión normalizada, pero lo que se
    // muestra es siempre el texto original: si se enseñara el normalizado, el
    // usuario vería un texto que no es el suyo.
    let left_normalized = normalize(left, options);
    let right_normalized = normalize(right, options);

    let diff = TextDiff::from_lines(&left_normalized, &right_normalized);

    let left_lines: Vec<&str> = left.lines().collect();
    let right_lines: Vec<&str> = right.lines().collect();

    let mut lines = Vec::new();
    let mut added = 0;
    let mut removed = 0;

    for change in diff.iter_all_changes() {
        let left_number = change.old_index().map(|index| index + 1);
        let right_number = change.new_index().map(|index| index + 1);

        let original = match (change.old_index(), change.new_index()) {
            (Some(index), _) => left_lines.get(index).copied().unwrap_or_default(),
            (None, Some(index)) => right_lines.get(index).copied().unwrap_or_default(),
            _ => "",
        };

        let kind = match change.tag() {
            ChangeTag::Equal => ChangeKind::Equal,
            ChangeTag::Insert => {
                added += 1;
                ChangeKind::Insert
            }
            ChangeTag::Delete => {
                removed += 1;
                ChangeKind::Delete
            }
        };

        lines.push(DiffLine {
            kind,
            left_number,
            right_number,
            pieces: vec![DiffPiece {
                kind,
                text: original.to_string(),
            }],
        });
    }

    // Resaltado fino: para cada pareja de líneas sustituidas, se marca qué
    // cambió dentro. Sin esto, cambiar una palabra pinta la línea entera en
    // rojo y verde y hay que buscar la diferencia a ojo.
    if options.granularity != DiffGranularity::Line {
        refine_pairs(&mut lines, options.granularity);
    }

    DiffResult {
        identical: added == 0 && removed == 0,
        lines,
        added,
        removed,
    }
}

fn normalize(text: &str, options: &DiffOptions) -> String {
    let mut normalized = text.to_string();
    if options.ignore_case {
        normalized = normalized.to_lowercase();
    }
    if options.ignore_whitespace {
        normalized = normalized
            .lines()
            .map(|line| line.split_whitespace().collect::<Vec<_>>().join(" "))
            .collect::<Vec<_>>()
            .join("\n");
    }
    normalized
}

/// Busca borrados seguidos de inserciones y afina su contenido.
fn refine_pairs(lines: &mut [DiffLine], granularity: DiffGranularity) {
    let mut index = 0;
    while index < lines.len() {
        if lines[index].kind != ChangeKind::Delete {
            index += 1;
            continue;
        }

        let deletes = count_run(lines, index, ChangeKind::Delete);
        let inserts = count_run(lines, index + deletes, ChangeKind::Insert);

        // Solo se afina cuando hay tantas líneas borradas como insertadas: es
        // el caso de una sustitución, donde emparejarlas tiene sentido.
        if deletes > 0 && deletes == inserts {
            for offset in 0..deletes {
                let old = text_of(&lines[index + offset]);
                let new = text_of(&lines[index + deletes + offset]);
                let (left_pieces, right_pieces) = split_changes(&old, &new, granularity);
                lines[index + offset].pieces = left_pieces;
                lines[index + deletes + offset].pieces = right_pieces;
            }
        }

        index += deletes + inserts;
    }
}

fn count_run(lines: &[DiffLine], start: usize, kind: ChangeKind) -> usize {
    lines[start.min(lines.len())..]
        .iter()
        .take_while(|line| line.kind == kind)
        .count()
}

fn text_of(line: &DiffLine) -> String {
    line.pieces
        .iter()
        .map(|piece| piece.text.as_str())
        .collect()
}

fn split_changes(
    old: &str,
    new: &str,
    granularity: DiffGranularity,
) -> (Vec<DiffPiece>, Vec<DiffPiece>) {
    let diff = match granularity {
        DiffGranularity::Word => TextDiff::from_words(old, new),
        _ => TextDiff::from_chars(old, new),
    };

    let mut left = Vec::new();
    let mut right = Vec::new();

    for change in diff.iter_all_changes() {
        let text = change.value().to_string();
        match change.tag() {
            ChangeTag::Equal => {
                push_piece(&mut left, ChangeKind::Equal, &text);
                push_piece(&mut right, ChangeKind::Equal, &text);
            }
            ChangeTag::Delete => push_piece(&mut left, ChangeKind::Delete, &text),
            ChangeTag::Insert => push_piece(&mut right, ChangeKind::Insert, &text),
        }
    }

    (left, right)
}

/// Une trozos consecutivos del mismo tipo para no devolver un elemento por
/// carácter, que multiplicaría por diez el tamaño de la respuesta.
fn push_piece(pieces: &mut Vec<DiffPiece>, kind: ChangeKind, text: &str) {
    match pieces.last_mut() {
        Some(last) if last.kind == kind => last.text.push_str(text),
        _ => pieces.push(DiffPiece {
            kind,
            text: text.to_string(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formatea_sql_en_mayusculas_e_indentado() {
        let salida = format_sql("select a,b from t where x=1", &SqlOptions::default());

        assert!(salida.contains("SELECT"), "{salida}");
        assert!(salida.contains("FROM"), "{salida}");
        assert!(salida.contains('\n'), "debe partir en líneas: {salida}");
    }

    #[test]
    fn respeta_la_opcion_de_no_poner_mayusculas() {
        let opciones = SqlOptions {
            uppercase: false,
            ..SqlOptions::default()
        };
        let salida = format_sql("select a from t", &opciones);
        assert!(salida.contains("select"), "{salida}");
    }

    #[test]
    fn conserva_los_marcadores_de_parametro() {
        // Un `?` es parte de la consulta, no un hueco que rellenar.
        let salida = format_sql("select * from t where id = ?", &SqlOptions::default());
        assert!(salida.contains('?'), "{salida}");

        let salida = format_sql("select * from t where id = $1", &SqlOptions::default());
        assert!(salida.contains("$1"), "{salida}");
    }

    #[test]
    fn dos_textos_iguales_no_tienen_diferencias() {
        let resultado = diff("a\nb\n", "a\nb\n", &DiffOptions::default());

        assert!(resultado.identical);
        assert_eq!(resultado.added, 0);
        assert_eq!(resultado.removed, 0);
    }

    #[test]
    fn cuenta_altas_y_bajas() {
        let resultado = diff("a\nb\nc\n", "a\nx\nc\nd\n", &DiffOptions::default());

        assert!(!resultado.identical);
        assert_eq!(resultado.removed, 1, "se quitó «b»");
        assert_eq!(resultado.added, 2, "se pusieron «x» y «d»");
    }

    #[test]
    fn numera_las_lineas_de_cada_lado() {
        let resultado = diff("a\nb\n", "a\nc\n", &DiffOptions::default());

        let iguales: Vec<_> = resultado
            .lines
            .iter()
            .filter(|line| line.kind == ChangeKind::Equal)
            .collect();
        assert_eq!(iguales[0].left_number, Some(1));
        assert_eq!(iguales[0].right_number, Some(1));

        let borrada = resultado
            .lines
            .iter()
            .find(|line| line.kind == ChangeKind::Delete)
            .unwrap();
        assert_eq!(borrada.left_number, Some(2));
        assert_eq!(borrada.right_number, None);
    }

    #[test]
    fn muestra_el_texto_original_aunque_se_ignoren_mayusculas() {
        let opciones = DiffOptions {
            ignore_case: true,
            ..DiffOptions::default()
        };
        let resultado = diff("Hola\n", "HOLA\n", &opciones);

        assert!(resultado.identical, "ignorando mayúsculas son iguales");
        // Y lo que se enseña sigue siendo lo que escribió el usuario.
        assert_eq!(resultado.lines[0].pieces[0].text, "Hola");
    }

    #[test]
    fn ignora_los_espacios_cuando_se_pide() {
        let opciones = DiffOptions {
            ignore_whitespace: true,
            ..DiffOptions::default()
        };
        assert!(diff("a   b\n", "a b\n", &opciones).identical);

        // Y sin la opción, no.
        assert!(!diff("a   b\n", "a b\n", &DiffOptions::default()).identical);
    }

    #[test]
    fn el_resaltado_por_palabra_senala_solo_lo_que_cambio() {
        let opciones = DiffOptions {
            granularity: DiffGranularity::Word,
            ..DiffOptions::default()
        };
        let resultado = diff("el gato duerme\n", "el perro duerme\n", &opciones);

        let borrada = resultado
            .lines
            .iter()
            .find(|line| line.kind == ChangeKind::Delete)
            .expect("debe haber una línea borrada");

        // «el » y « duerme» quedan como iguales; solo «gato» se marca.
        let cambiado: String = borrada
            .pieces
            .iter()
            .filter(|piece| piece.kind == ChangeKind::Delete)
            .map(|piece| piece.text.as_str())
            .collect();
        assert_eq!(cambiado.trim(), "gato");

        assert!(
            borrada.pieces.iter().any(|p| p.kind == ChangeKind::Equal),
            "parte de la línea debe quedar sin marcar"
        );
    }

    #[test]
    fn agrupa_los_trozos_contiguos_del_mismo_tipo() {
        let (left, _) = split_changes("abc", "xbc", DiffGranularity::Char);

        // «bc» debe salir en un solo trozo, no en dos de un carácter.
        assert!(
            left.len() <= 2,
            "se esperaban pocos trozos agrupados, salieron {left:?}"
        );
    }

    #[test]
    fn un_texto_vacio_frente_a_otro_lo_cuenta_entero() {
        let resultado = diff("", "a\nb\n", &DiffOptions::default());
        assert_eq!(resultado.added, 2);
        assert_eq!(resultado.removed, 0);
    }
}
