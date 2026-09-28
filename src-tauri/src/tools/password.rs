//! Generación de contraseñas y resúmenes de contraseña.
//!
//! Dos cosas distintas que comparten pantalla en la mayoría de herramientas:
//!
//! - **Generar**: crear una contraseña aleatoria con entropía conocida.
//! - **Resumir**: guardar una contraseña de forma que no se pueda recuperar,
//!   con bcrypt o Argon2.
//!
//! La aleatoriedad viene del generador del sistema operativo. `rand::rng()` es
//! ChaCha12 sembrado por el sistema y re-sembrado periódicamente: apto para
//! material secreto, a diferencia de un generador pseudoaleatorio corriente.

use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};

const MAX_LENGTH: usize = 256;
const MAX_COUNT: usize = 500;

/// Caracteres que se confunden al leerlos o dictarlos por teléfono.
const AMBIGUOUS: &str = "Il1O0o|`'\"{}[]()/\\";

const LOWER: &str = "abcdefghijklmnopqrstuvwxyz";
const UPPER: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
const DIGITS: &str = "0123456789";
const SYMBOLS: &str = "!@#$%^&*-_=+:;,.?";

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PasswordOptions {
    pub length: usize,
    pub count: usize,
    pub lowercase: bool,
    pub uppercase: bool,
    pub digits: bool,
    pub symbols: bool,
    /// Excluye los caracteres que se confunden al leer: I, l, 1, O, 0…
    pub avoid_ambiguous: bool,
}

impl Default for PasswordOptions {
    fn default() -> Self {
        PasswordOptions {
            length: 20,
            count: 5,
            lowercase: true,
            uppercase: true,
            digits: true,
            symbols: true,
            avoid_ambiguous: false,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneratedPasswords {
    pub passwords: Vec<String>,
    /// Bits de entropía por contraseña.
    pub entropy_bits: f64,
    pub alphabet_size: usize,
    pub strength: &'static str,
    pub note: String,
}

pub fn generate(options: &PasswordOptions) -> AppResult<GeneratedPasswords> {
    let alphabet = build_alphabet(options)?;

    if options.length == 0 || options.length > MAX_LENGTH {
        return Err(AppError::msg(format!(
            "la longitud debe estar entre 1 y {MAX_LENGTH}"
        )));
    }
    if options.count == 0 || options.count > MAX_COUNT {
        return Err(AppError::msg(format!(
            "la cantidad debe estar entre 1 y {MAX_COUNT}"
        )));
    }

    let mut rng = rand::rng();
    let passwords = (0..options.count)
        .map(|_| {
            (0..options.length)
                .map(|_| alphabet[rng.random_range(0..alphabet.len())])
                .collect::<String>()
        })
        .collect();

    // Entropía real de un muestreo uniforme e independiente: log2(N) por
    // carácter. Solo vale porque cada carácter se sortea del alfabeto completo;
    // forzar «al menos un símbolo» la reduciría y la haría difícil de calcular.
    let entropy_bits = (alphabet.len() as f64).log2() * options.length as f64;

    Ok(GeneratedPasswords {
        passwords,
        entropy_bits,
        alphabet_size: alphabet.len(),
        strength: strength_for(entropy_bits),
        note: note_for(entropy_bits),
    })
}

fn build_alphabet(options: &PasswordOptions) -> AppResult<Vec<char>> {
    let mut alphabet = String::new();
    if options.lowercase {
        alphabet.push_str(LOWER);
    }
    if options.uppercase {
        alphabet.push_str(UPPER);
    }
    if options.digits {
        alphabet.push_str(DIGITS);
    }
    if options.symbols {
        alphabet.push_str(SYMBOLS);
    }

    if alphabet.is_empty() {
        return Err(AppError::msg(
            "elige al menos un conjunto de caracteres: minúsculas, mayúsculas, dígitos o símbolos",
        ));
    }

    let chars: Vec<char> = if options.avoid_ambiguous {
        alphabet.chars().filter(|c| !AMBIGUOUS.contains(*c)).collect()
    } else {
        alphabet.chars().collect()
    };

    if chars.is_empty() {
        return Err(AppError::msg(
            "excluir los caracteres ambiguos ha dejado el alfabeto vacío",
        ));
    }
    Ok(chars)
}

/// Umbrales orientativos frente a un atacante con hardware dedicado.
fn strength_for(bits: f64) -> &'static str {
    match bits {
        b if b < 50.0 => "débil",
        b if b < 75.0 => "aceptable",
        b if b < 110.0 => "fuerte",
        _ => "excesiva",
    }
}

fn note_for(bits: f64) -> String {
    match bits {
        b if b < 50.0 => {
            "Por debajo de 50 bits está al alcance de un ataque por fuerza bruta \
             con hardware dedicado. Alarga la contraseña."
                .to_string()
        }
        b if b < 75.0 => {
            "Suficiente para cuentas corrientes protegidas con límite de intentos, \
             corta para cifrar archivos."
                .to_string()
        }
        b if b < 110.0 => {
            "Fuera del alcance de la fuerza bruta con la tecnología actual.".to_string()
        }
        _ => "Muy por encima de lo necesario: el eslabón débil estará en otro sitio.".to_string(),
    }
}

// ── Resumen de contraseñas ────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum HashScheme {
    Bcrypt,
    Argon2id,
}

impl HashScheme {
    pub fn from_id(id: &str) -> Option<HashScheme> {
        match id.to_ascii_lowercase().as_str() {
            "bcrypt" => Some(HashScheme::Bcrypt),
            "argon2id" | "argon2" => Some(HashScheme::Argon2id),
            _ => None,
        }
    }
}

/// Coste de bcrypt. Por debajo de 10 es demasiado rápido de atacar hoy; por
/// encima de 15 la propia comprobación tarda segundos en cada inicio de sesión.
const MIN_COST: u32 = 10;
const MAX_COST: u32 = 15;

pub fn hash(password: &str, scheme: HashScheme, cost: Option<u32>) -> AppResult<String> {
    if password.is_empty() {
        return Err(AppError::msg("la contraseña está vacía"));
    }

    match scheme {
        HashScheme::Bcrypt => {
            let cost = cost.unwrap_or(bcrypt::DEFAULT_COST);
            if !(MIN_COST..=MAX_COST).contains(&cost) {
                return Err(AppError::msg(format!(
                    "el coste de bcrypt debe estar entre {MIN_COST} y {MAX_COST}"
                )));
            }
            // bcrypt trunca en 72 bytes en silencio: lo que sobra no cuenta,
            // así que una frase de paso larga sería más débil de lo que parece.
            if password.len() > 72 {
                return Err(AppError::msg(
                    "bcrypt solo tiene en cuenta los primeros 72 bytes y esta contraseña \
                     los supera. Usa Argon2id, que no tiene ese límite.",
                ));
            }
            bcrypt::hash(password, cost)
                .map_err(|e| AppError::msg(format!("bcrypt falló: {e}")))
        }
        HashScheme::Argon2id => {
            use argon2::password_hash::{PasswordHasher, SaltString};

            // La sal se saca del mismo generador que las contraseñas en vez de
            // activar el `OsRng` de `password_hash`, que arrastraría otra
            // versión de `rand_core` al binario solo para esto.
            let mut salt_bytes = [0u8; 16];
            rand::rng().fill(&mut salt_bytes[..]);
            let salt = SaltString::encode_b64(&salt_bytes)
                .map_err(|e| AppError::msg(format!("no se pudo preparar la sal: {e}")))?;

            argon2::Argon2::default()
                .hash_password(password.as_bytes(), &salt)
                .map(|hash| hash.to_string())
                .map_err(|e| AppError::msg(format!("Argon2 falló: {e}")))
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifyResult {
    pub matches: bool,
    pub scheme: &'static str,
}

/// Comprueba una contraseña contra un resumen, deduciendo el esquema del
/// prefijo del propio resumen.
pub fn verify(password: &str, encoded: &str) -> AppResult<VerifyResult> {
    let encoded = encoded.trim();

    if encoded.starts_with("$argon2") {
        use argon2::password_hash::{PasswordHash, PasswordVerifier};

        let parsed = PasswordHash::new(encoded)
            .map_err(|e| AppError::msg(format!("el resumen Argon2 no es válido: {e}")))?;
        let matches = argon2::Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .is_ok();
        return Ok(VerifyResult {
            matches,
            scheme: "Argon2",
        });
    }

    if encoded.starts_with("$2") {
        let matches = bcrypt::verify(password, encoded)
            .map_err(|e| AppError::msg(format!("el resumen bcrypt no es válido: {e}")))?;
        return Ok(VerifyResult {
            matches,
            scheme: "bcrypt",
        });
    }

    Err(AppError::msg(
        "no se reconoce el formato del resumen: se esperaba uno de bcrypt («$2…») \
         o de Argon2 («$argon2…»)",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn genera_la_longitud_y_cantidad_pedidas() {
        let result = generate(&PasswordOptions {
            length: 32,
            count: 10,
            ..PasswordOptions::default()
        })
        .unwrap();

        assert_eq!(result.passwords.len(), 10);
        assert!(result.passwords.iter().all(|p| p.chars().count() == 32));
    }

    #[test]
    fn no_repite_contrasenas() {
        let result = generate(&PasswordOptions {
            length: 24,
            count: 200,
            ..PasswordOptions::default()
        })
        .unwrap();

        assert_eq!(
            result.passwords.iter().collect::<HashSet<_>>().len(),
            200,
            "dos contraseñas iguales con 24 caracteres indican un generador roto"
        );
    }

    #[test]
    fn respeta_los_conjuntos_elegidos() {
        let result = generate(&PasswordOptions {
            length: 200,
            count: 1,
            lowercase: true,
            uppercase: false,
            digits: false,
            symbols: false,
            avoid_ambiguous: false,
        })
        .unwrap();

        assert!(result.passwords[0].chars().all(|c| LOWER.contains(c)));
        assert_eq!(result.alphabet_size, 26);
    }

    #[test]
    fn excluye_los_caracteres_ambiguos() {
        let result = generate(&PasswordOptions {
            length: 200,
            count: 3,
            avoid_ambiguous: true,
            ..PasswordOptions::default()
        })
        .unwrap();

        for password in &result.passwords {
            assert!(
                !password.chars().any(|c| AMBIGUOUS.contains(c)),
                "salió un carácter ambiguo en {password}"
            );
        }
    }

    #[test]
    fn la_entropia_es_log2_del_alfabeto_por_la_longitud() {
        let result = generate(&PasswordOptions {
            length: 10,
            count: 1,
            lowercase: true,
            uppercase: false,
            digits: false,
            symbols: false,
            avoid_ambiguous: false,
        })
        .unwrap();

        // 26 caracteres ≈ 4,7 bits cada uno.
        assert!((result.entropy_bits - 47.0).abs() < 0.1, "{}", result.entropy_bits);
        assert_eq!(result.strength, "débil");
    }

    #[test]
    fn clasifica_la_fuerza_por_tramos() {
        assert_eq!(strength_for(40.0), "débil");
        assert_eq!(strength_for(60.0), "aceptable");
        assert_eq!(strength_for(90.0), "fuerte");
        assert_eq!(strength_for(200.0), "excesiva");
    }

    #[test]
    fn sin_ningun_conjunto_lo_dice_en_vez_de_generar_basura() {
        let error = generate(&PasswordOptions {
            lowercase: false,
            uppercase: false,
            digits: false,
            symbols: false,
            ..PasswordOptions::default()
        })
        .unwrap_err()
        .to_string();

        assert!(error.contains("al menos un conjunto"), "{error}");
    }

    #[test]
    fn rechaza_longitudes_y_cantidades_imposibles() {
        let demasiado_larga = PasswordOptions {
            length: MAX_LENGTH + 1,
            ..PasswordOptions::default()
        };
        assert!(generate(&demasiado_larga).is_err());

        let demasiadas = PasswordOptions {
            count: MAX_COUNT + 1,
            ..PasswordOptions::default()
        };
        assert!(generate(&demasiadas).is_err());
    }

    #[test]
    fn bcrypt_va_y_vuelve() {
        // Coste mínimo: los tests no deben tardar segundos.
        let encoded = hash("contraseña-secreta", HashScheme::Bcrypt, Some(MIN_COST)).unwrap();
        assert!(encoded.starts_with("$2"));

        assert!(verify("contraseña-secreta", &encoded).unwrap().matches);
        assert!(!verify("otra", &encoded).unwrap().matches);
    }

    #[test]
    fn argon2_va_y_vuelve() {
        let encoded = hash("contraseña-secreta", HashScheme::Argon2id, None).unwrap();
        assert!(encoded.starts_with("$argon2"));

        let resultado = verify("contraseña-secreta", &encoded).unwrap();
        assert!(resultado.matches);
        assert_eq!(resultado.scheme, "Argon2");
        assert!(!verify("otra", &encoded).unwrap().matches);
    }

    #[test]
    fn el_mismo_resumen_no_sale_dos_veces() {
        // La sal aleatoria hace que dos resúmenes de la misma contraseña
        // difieran; si salieran iguales, una tabla precalculada las rompería.
        let uno = hash("misma", HashScheme::Bcrypt, Some(MIN_COST)).unwrap();
        let dos = hash("misma", HashScheme::Bcrypt, Some(MIN_COST)).unwrap();
        assert_ne!(uno, dos);
    }

    #[test]
    fn bcrypt_avisa_del_limite_de_72_bytes() {
        let larga = "a".repeat(80);
        let error = hash(&larga, HashScheme::Bcrypt, Some(MIN_COST))
            .unwrap_err()
            .to_string();

        assert!(error.contains("72 bytes"), "{error}");
        assert!(error.contains("Argon2id"), "{error}");

        // Argon2 sí la admite.
        assert!(hash(&larga, HashScheme::Argon2id, None).is_ok());
    }

    #[test]
    fn rechaza_costes_de_bcrypt_fuera_de_rango() {
        assert!(hash("x", HashScheme::Bcrypt, Some(4)).is_err());
        assert!(hash("x", HashScheme::Bcrypt, Some(31)).is_err());
    }

    #[test]
    fn un_resumen_irreconocible_lo_dice() {
        let error = verify("x", "5f4dcc3b5aa765d61d8327deb882cf99")
            .unwrap_err()
            .to_string();
        assert!(error.contains("no se reconoce"), "{error}");
    }

    #[test]
    fn una_contrasena_vacia_se_rechaza() {
        assert!(hash("", HashScheme::Bcrypt, None).is_err());
    }
}
