//! Expresiones regulares y expresiones cron.

use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};

// ── Expresiones regulares ─────────────────────────────────────────────────

/// Tope de coincidencias devueltas. Un patrón como `.*` sobre un texto largo
/// produce una coincidencia por posición y la respuesta se dispara.
const MAX_MATCHES: usize = 500;

/// Tamaño máximo del programa compilado del patrón.
///
/// Sin este límite, un patrón anidado como `(a|b|c){20}{20}` hace que el motor
/// reserve gigabytes al compilarlo. El crate `regex` no tiene retroceso, así
/// que no sufre explosión exponencial al ejecutar, pero sí al construir.
const MAX_PATTERN_SIZE: usize = 1 << 20;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct RegexOptions {
    pub case_insensitive: bool,
    /// `^` y `$` encajan en cada línea en vez de solo al principio y al final.
    pub multi_line: bool,
    /// El punto también encaja con el salto de línea.
    pub dot_matches_newline: bool,
    /// Ignora espacios y admite comentarios con `#` dentro del patrón.
    pub extended: bool,
}

impl Default for RegexOptions {
    fn default() -> Self {
        RegexOptions {
            case_insensitive: false,
            multi_line: true,
            dot_matches_newline: false,
            extended: false,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureGroup {
    pub index: usize,
    pub name: Option<String>,
    pub value: Option<String>,
    pub start: Option<usize>,
    pub end: Option<usize>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegexMatch {
    pub start: usize,
    pub end: usize,
    pub value: String,
    pub groups: Vec<CaptureGroup>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegexReport {
    pub matches: Vec<RegexMatch>,
    pub total: usize,
    /// Verdadero si se alcanzó el tope y hay más coincidencias sin mostrar.
    pub truncated: bool,
    /// Nombres de los grupos con nombre, en orden.
    pub group_names: Vec<Option<String>>,
    /// Aviso sobre el patrón, si hay algo que convenga saber.
    pub note: Option<String>,
}

pub fn test_regex(
    pattern: &str,
    haystack: &str,
    options: &RegexOptions,
) -> AppResult<RegexReport> {
    if pattern.is_empty() {
        return Err(AppError::msg("escribe un patrón"));
    }

    let regex = regex::RegexBuilder::new(pattern)
        .case_insensitive(options.case_insensitive)
        .multi_line(options.multi_line)
        .dot_matches_new_line(options.dot_matches_newline)
        .ignore_whitespace(options.extended)
        .size_limit(MAX_PATTERN_SIZE)
        .build()
        .map_err(|e| AppError::msg(format!("El patrón no es válido.\n{e}")))?;

    let group_names: Vec<Option<String>> = regex
        .capture_names()
        .map(|name| name.map(str::to_string))
        .collect();

    let mut matches = Vec::new();
    let mut total = 0usize;

    for capture in regex.captures_iter(haystack) {
        total += 1;
        if matches.len() >= MAX_MATCHES {
            continue;
        }

        let whole = capture.get(0).expect("el grupo 0 siempre existe");
        let groups = (1..capture.len())
            .map(|index| {
                let found = capture.get(index);
                CaptureGroup {
                    index,
                    name: group_names.get(index).cloned().flatten(),
                    value: found.map(|m| m.as_str().to_string()),
                    start: found.map(|m| m.start()),
                    end: found.map(|m| m.end()),
                }
            })
            .collect();

        matches.push(RegexMatch {
            start: whole.start(),
            end: whole.end(),
            value: whole.as_str().to_string(),
            groups,
        });
    }

    // Un patrón que encaja con la cadena vacía produce una coincidencia por
    // cada posición del texto, y quien lo escribe casi nunca lo pretende.
    let note = if regex.is_match("") && total > 0 {
        Some(
            "Este patrón encaja también con la cadena vacía, así que produce una \
             coincidencia en cada posición del texto. Si no es lo que buscabas, \
             revisa los cuantificadores: «*» y «?» admiten cero repeticiones."
                .to_string(),
        )
    } else {
        None
    };

    Ok(RegexReport {
        truncated: total > matches.len(),
        matches,
        total,
        group_names,
        note,
    })
}

/// Sustituye las coincidencias, admitiendo `$1` y `$nombre` en el reemplazo.
pub fn replace_regex(
    pattern: &str,
    haystack: &str,
    replacement: &str,
    options: &RegexOptions,
) -> AppResult<String> {
    let regex = regex::RegexBuilder::new(pattern)
        .case_insensitive(options.case_insensitive)
        .multi_line(options.multi_line)
        .dot_matches_new_line(options.dot_matches_newline)
        .ignore_whitespace(options.extended)
        .size_limit(MAX_PATTERN_SIZE)
        .build()
        .map_err(|e| AppError::msg(format!("El patrón no es válido.\n{e}")))?;

    Ok(regex.replace_all(haystack, replacement).into_owned())
}

// ── Expresiones cron ──────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CronReport {
    /// Las próximas ejecuciones, en hora local.
    pub next_runs: Vec<String>,
    /// Cuánto falta para la primera, en palabras.
    pub time_to_next: Option<String>,
    /// Descripción de cada campo del patrón.
    pub fields: Vec<CronField>,
    pub note: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CronField {
    pub name: &'static str,
    pub value: String,
}

/// Nombres de los campos de un cron de cinco y de seis posiciones.
const FIELDS_5: [&str; 5] = ["Minuto", "Hora", "Día del mes", "Mes", "Día de la semana"];
const FIELDS_6: [&str; 6] = [
    "Segundo",
    "Minuto",
    "Hora",
    "Día del mes",
    "Mes",
    "Día de la semana",
];

pub fn parse_cron(expression: &str, count: usize) -> AppResult<CronReport> {
    use croner::Cron;

    let expression = expression.trim();
    if expression.is_empty() {
        return Err(AppError::msg("escribe una expresión cron"));
    }

    let cron: Cron = expression
        .parse()
        .map_err(|e| AppError::msg(format!("La expresión no es válida.\n{e}")))?;

    let parts: Vec<&str> = expression.split_whitespace().collect();
    let names: &[&str] = match parts.len() {
        6 => &FIELDS_6,
        _ => &FIELDS_5,
    };
    let fields = parts
        .iter()
        .zip(names)
        .map(|(value, name)| CronField {
            name,
            value: (*value).to_string(),
        })
        .collect();

    let now = chrono::Local::now();
    let count = count.clamp(1, 50);

    let mut next_runs = Vec::new();
    let mut cursor = now;
    for _ in 0..count {
        match cron.find_next_occurrence(&cursor, false) {
            Ok(next) => {
                next_runs.push(next.format("%Y-%m-%d %H:%M:%S").to_string());
                cursor = next;
            }
            // Un patrón como «0 0 30 2 *» —30 de febrero— es válido pero no
            // ocurre nunca. Vale la pena decirlo en vez de dar una lista vacía.
            Err(_) => break,
        }
    }

    let time_to_next = cron
        .find_next_occurrence(&now, false)
        .ok()
        .map(|next| humanize(next.signed_duration_since(now)));

    let note = if next_runs.is_empty() {
        Some(
            "La expresión es válida pero no se cumple nunca. Suele pasar al \
             combinar un día del mes con un mes que no lo tiene, como el 30 de \
             febrero."
                .to_string(),
        )
    } else {
        None
    };

    Ok(CronReport {
        next_runs,
        time_to_next,
        fields,
        note,
    })
}

fn humanize(duration: chrono::TimeDelta) -> String {
    let seconds = duration.num_seconds().max(0);
    match seconds {
        s if s < 60 => format!("{s} s"),
        s if s < 3600 => format!("{} min", s / 60),
        s if s < 86_400 => format!("{} h {} min", s / 3600, (s % 3600) / 60),
        s => format!("{} días", s / 86_400),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encuentra_coincidencias_con_sus_posiciones() {
        let report = test_regex(r"\d+", "a1b22c333", &RegexOptions::default()).unwrap();

        assert_eq!(report.total, 3);
        assert_eq!(report.matches[0].value, "1");
        assert_eq!(report.matches[0].start, 1);
        assert_eq!(report.matches[2].value, "333");
    }

    #[test]
    fn devuelve_los_grupos_de_captura() {
        let report = test_regex(
            r"(\w+)@(\w+\.\w+)",
            "escribe a ana@ejemplo.com hoy",
            &RegexOptions::default(),
        )
        .unwrap();

        let groups = &report.matches[0].groups;
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].value.as_deref(), Some("ana"));
        assert_eq!(groups[1].value.as_deref(), Some("ejemplo.com"));
    }

    #[test]
    fn devuelve_los_nombres_de_los_grupos() {
        let report = test_regex(
            r"(?<usuario>\w+)@(?<dominio>[\w.]+)",
            "ana@ejemplo.com",
            &RegexOptions::default(),
        )
        .unwrap();

        assert_eq!(report.matches[0].groups[0].name.as_deref(), Some("usuario"));
        assert_eq!(report.matches[0].groups[1].name.as_deref(), Some("dominio"));
    }

    #[test]
    fn un_grupo_opcional_que_no_encaja_sale_nulo() {
        let report = test_regex(r"(a)(b)?", "a", &RegexOptions::default()).unwrap();

        assert_eq!(report.matches[0].groups[0].value.as_deref(), Some("a"));
        assert_eq!(report.matches[0].groups[1].value, None);
    }

    #[test]
    fn respeta_las_banderas() {
        let sin_bandera = test_regex(r"hola", "HOLA", &RegexOptions::default()).unwrap();
        assert_eq!(sin_bandera.total, 0);

        let con_bandera = test_regex(
            r"hola",
            "HOLA",
            &RegexOptions {
                case_insensitive: true,
                ..RegexOptions::default()
            },
        )
        .unwrap();
        assert_eq!(con_bandera.total, 1);
    }

    #[test]
    fn multilinea_cambia_el_significado_de_los_anclajes() {
        let texto = "uno\ndos";

        let multi = test_regex(r"^dos$", texto, &RegexOptions::default()).unwrap();
        assert_eq!(multi.total, 1, "con multiLine, ^ y $ van por línea");

        let simple = test_regex(
            r"^dos$",
            texto,
            &RegexOptions {
                multi_line: false,
                ..RegexOptions::default()
            },
        )
        .unwrap();
        assert_eq!(simple.total, 0);
    }

    #[test]
    fn avisa_del_patron_que_encaja_con_la_cadena_vacia() {
        let report = test_regex(r"\d*", "abc", &RegexOptions::default()).unwrap();
        assert!(report.note.is_some(), "{:?}", report.note);
        assert!(report.note.unwrap().contains("cuantificadores"));
    }

    #[test]
    fn recorta_cuando_hay_demasiadas_coincidencias() {
        let texto = "a".repeat(MAX_MATCHES + 50);
        let report = test_regex("a", &texto, &RegexOptions::default()).unwrap();

        assert_eq!(report.total, MAX_MATCHES + 50);
        assert_eq!(report.matches.len(), MAX_MATCHES);
        assert!(report.truncated);
    }

    #[test]
    fn un_patron_invalido_explica_el_fallo() {
        let error = test_regex("(sin cerrar", "x", &RegexOptions::default())
            .unwrap_err()
            .to_string();
        assert!(error.contains("no es válido"), "{error}");
    }

    #[test]
    fn un_patron_vacio_se_rechaza() {
        assert!(test_regex("", "x", &RegexOptions::default()).is_err());
    }

    #[test]
    fn sustituye_usando_los_grupos() {
        let resultado = replace_regex(
            r"(\w+)@(\w+)",
            "ana@ejemplo",
            "$2 tiene a $1",
            &RegexOptions::default(),
        )
        .unwrap();
        assert_eq!(resultado, "ejemplo tiene a ana");
    }

    #[test]
    fn el_cron_lista_las_proximas_ejecuciones() {
        let report = parse_cron("0 9 * * 1", 5).unwrap();

        assert_eq!(report.next_runs.len(), 5);
        assert!(report.time_to_next.is_some());
        // Todas a las nueve en punto.
        for run in &report.next_runs {
            assert!(run.contains("09:00:00"), "{run}");
        }
    }

    #[test]
    fn el_cron_nombra_cada_campo() {
        let report = parse_cron("30 8 * * *", 1).unwrap();

        assert_eq!(report.fields.len(), 5);
        assert_eq!(report.fields[0].name, "Minuto");
        assert_eq!(report.fields[0].value, "30");
        assert_eq!(report.fields[1].name, "Hora");
        assert_eq!(report.fields[2].name, "Día del mes");
        assert_eq!(report.fields[4].name, "Día de la semana");
    }

    #[test]
    fn el_cron_de_seis_campos_empieza_por_los_segundos() {
        let report = parse_cron("15 30 8 * * *", 1).unwrap();
        assert_eq!(report.fields.len(), 6);
        assert_eq!(report.fields[0].name, "Segundo");
    }

    #[test]
    fn las_ejecuciones_van_en_orden_y_no_se_repiten() {
        let report = parse_cron("*/5 * * * *", 6).unwrap();

        for pair in report.next_runs.windows(2) {
            assert!(pair[0] < pair[1], "{:?}", report.next_runs);
        }
    }

    #[test]
    fn una_expresion_cron_invalida_explica_el_fallo() {
        let error = parse_cron("esto no es cron", 5).unwrap_err().to_string();
        assert!(error.contains("no es válida"), "{error}");
    }

    #[test]
    fn una_expresion_vacia_se_rechaza() {
        assert!(parse_cron("   ", 5).is_err());
    }

    #[test]
    fn humaniza_las_esperas() {
        use chrono::TimeDelta;
        assert_eq!(humanize(TimeDelta::seconds(30)), "30 s");
        assert_eq!(humanize(TimeDelta::seconds(600)), "10 min");
        assert_eq!(humanize(TimeDelta::seconds(3700)), "1 h 1 min");
        assert_eq!(humanize(TimeDelta::seconds(180_000)), "2 días");
    }
}
