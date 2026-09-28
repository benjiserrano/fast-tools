//! Generación de identificadores.

use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};

/// Tope de generación por llamada. Más que esto no cabe en pantalla ni en el
/// portapapeles de forma útil, y evita que un cero de más bloquee la ventana.
const MAX_BATCH: usize = 10_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum IdKind {
    /// Aleatorio. Sin orden: como clave primaria fragmenta los índices.
    Uuidv4,
    /// Con marca de tiempo al principio: ordenable y amigable para índices.
    Uuidv7,
    /// Como UUIDv7 pero en Base32, más corto y sin guiones.
    Ulid,
    /// UUIDv4 sin guiones, para identificadores en URL.
    Uuidv4Compact,
}

impl IdKind {
    pub const ALL: &'static [IdKind] = &[
        IdKind::Uuidv7,
        IdKind::Uuidv4,
        IdKind::Ulid,
        IdKind::Uuidv4Compact,
    ];

    pub fn id(self) -> &'static str {
        match self {
            IdKind::Uuidv4 => "uuidv4",
            IdKind::Uuidv7 => "uuidv7",
            IdKind::Ulid => "ulid",
            IdKind::Uuidv4Compact => "uuidv4-compact",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            IdKind::Uuidv4 => "UUID v4 (aleatorio)",
            IdKind::Uuidv7 => "UUID v7 (ordenable)",
            IdKind::Ulid => "ULID",
            IdKind::Uuidv4Compact => "UUID v4 sin guiones",
        }
    }

    pub fn note(self) -> &'static str {
        match self {
            IdKind::Uuidv4 => {
                "Totalmente aleatorio. Como clave primaria fragmenta el índice \
                 de la base de datos al insertar."
            }
            IdKind::Uuidv7 => {
                "Lleva la marca de tiempo delante, así que se ordena por fecha \
                 de creación. Es la opción recomendada para claves nuevas."
            }
            IdKind::Ulid => {
                "Ordenable como el v7 pero en Base32: 26 caracteres, sin guiones \
                 y sin ambigüedad entre letras y dígitos."
            }
            IdKind::Uuidv4Compact => "Un UUID v4 sin guiones, para usar en rutas de URL.",
        }
    }

    pub fn from_id(id: &str) -> Option<IdKind> {
        IdKind::ALL
            .iter()
            .copied()
            .find(|kind| kind.id().eq_ignore_ascii_case(id))
    }

    fn generate_one(self, ulids: &mut ulid::Generator) -> String {
        match self {
            IdKind::Uuidv4 => uuid::Uuid::new_v4().to_string(),
            IdKind::Uuidv7 => uuid::Uuid::now_v7().to_string(),
            IdKind::Uuidv4Compact => uuid::Uuid::new_v4().simple().to_string(),
            // El generador conserva el orden entre ULIDs creados dentro del
            // mismo milisegundo. Sin él, una tanda de mil saldría desordenada
            // dentro de cada milisegundo y se perdería justo la propiedad por
            // la que se elige un ULID.
            IdKind::Ulid => {
                let generated = ulids.generate().ok();
                generated.unwrap_or_else(ulid::Ulid::generate).to_string()
            }
        }
    }
}

pub fn generate(kind: IdKind, count: usize, uppercase: bool) -> AppResult<Vec<String>> {
    if count == 0 {
        return Ok(Vec::new());
    }
    if count > MAX_BATCH {
        return Err(AppError::msg(format!(
            "el máximo por tanda es {MAX_BATCH}; has pedido {count}"
        )));
    }

    let mut ulids = ulid::Generator::new();
    Ok((0..count)
        .map(|_| {
            let id = kind.generate_one(&mut ulids);
            if uppercase {
                id.to_uppercase()
            } else {
                id
            }
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn genera_la_cantidad_pedida_y_sin_repetir() {
        let ids = generate(IdKind::Uuidv4, 500, false).unwrap();
        assert_eq!(ids.len(), 500);
        assert_eq!(ids.iter().collect::<HashSet<_>>().len(), 500);
    }

    #[test]
    fn el_uuid_v4_tiene_la_forma_canonica() {
        let ids = generate(IdKind::Uuidv4, 1, false).unwrap();
        let id = &ids[0];
        assert_eq!(id.len(), 36);
        assert_eq!(id.matches('-').count(), 4);
        // El nibble de versión.
        assert_eq!(id.chars().nth(14), Some('4'));
    }

    #[test]
    fn el_uuid_v7_es_ordenable_por_tiempo() {
        // Generados en orden, deben salir ordenados como texto.
        let primero = generate(IdKind::Uuidv7, 1, false).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(2));
        let segundo = generate(IdKind::Uuidv7, 1, false).unwrap();

        assert!(
            primero[0] < segundo[0],
            "{} debería ir antes que {}",
            primero[0],
            segundo[0]
        );
        assert_eq!(primero[0].chars().nth(14), Some('7'));
    }

    #[test]
    fn el_ulid_mide_26_caracteres_sin_guiones() {
        let ids = generate(IdKind::Ulid, 1, false).unwrap();
        assert_eq!(ids[0].len(), 26);
        assert!(!ids[0].contains('-'));
    }

    #[test]
    fn la_version_compacta_quita_los_guiones() {
        let ids = generate(IdKind::Uuidv4Compact, 1, false).unwrap();
        assert_eq!(ids[0].len(), 32);
        assert!(!ids[0].contains('-'));
    }

    #[test]
    fn respeta_las_mayusculas() {
        let ids = generate(IdKind::Uuidv4, 1, true).unwrap();
        assert_eq!(ids[0], ids[0].to_uppercase());
    }

    #[test]
    fn rechaza_tandas_desmedidas_en_vez_de_colgarse() {
        let error = generate(IdKind::Uuidv4, MAX_BATCH + 1, false)
            .unwrap_err()
            .to_string();
        assert!(error.contains("máximo"), "{error}");

        assert!(generate(IdKind::Uuidv4, 0, false).unwrap().is_empty());
    }

    #[test]
    fn los_identificadores_redondean_y_todos_tienen_nota() {
        for kind in IdKind::ALL {
            assert_eq!(IdKind::from_id(kind.id()), Some(*kind));
            assert!(!kind.note().is_empty());
        }
    }
}
