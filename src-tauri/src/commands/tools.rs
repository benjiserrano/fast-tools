//! Comandos de las herramientas de texto, criptografía e identificadores.

use serde::{Deserialize, Serialize};

use crate::convert::{self, Format, Opts};
use crate::error::{AppError, AppResult};
use crate::tools::{cert, hashing, ids, jwt, password, text};

// ── Texto ─────────────────────────────────────────────────────────────────

/// Reformatea un documento sin cambiarle el formato: validar y normalizar.
///
/// Reutiliza el motor de conversión porque formatear es convertir un formato a
/// sí mismo: se analiza y se vuelve a escribir, lo que de paso valida la
/// entrada y da el mismo mensaje de error que el conversor.
#[tauri::command]
pub fn format_document(format: String, input: String, opts: Opts) -> AppResult<String> {
    let format = Format::from_id(&format)
        .ok_or_else(|| AppError::msg(format!("formato desconocido: «{format}»")))?;

    let bytes = convert::convert(format, format, input.as_bytes(), &opts)?;
    String::from_utf8(bytes)
        .map_err(|_| AppError::msg("el resultado no es texto"))
}

#[tauri::command]
pub fn format_sql(input: String, options: text::SqlOptions) -> String {
    text::format_sql(&input, &options)
}

#[tauri::command]
pub fn diff_text(
    left: String,
    right: String,
    options: text::DiffOptions,
) -> text::DiffResult {
    text::diff(&left, &right, &options)
}

// ── Resúmenes ─────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AlgorithmInfo {
    pub id: &'static str,
    pub label: &'static str,
    /// Motivo por el que no debe usarse en contextos de seguridad, si lo hay.
    pub insecure_note: Option<&'static str>,
}

#[tauri::command]
pub fn list_hash_algorithms() -> Vec<AlgorithmInfo> {
    hashing::Algorithm::ALL
        .iter()
        .map(|algorithm| AlgorithmInfo {
            id: algorithm.id(),
            label: algorithm.label(),
            insecure_note: algorithm.insecure_note(),
        })
        .collect()
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HashResult {
    pub algorithm: &'static str,
    pub label: &'static str,
    pub digest: String,
}

fn parse_algorithms(ids: &[String]) -> AppResult<Vec<hashing::Algorithm>> {
    if ids.is_empty() {
        return Err(AppError::msg("elige al menos un algoritmo"));
    }
    ids.iter()
        .map(|id| {
            hashing::Algorithm::from_id(id)
                .ok_or_else(|| AppError::msg(format!("algoritmo desconocido: «{id}»")))
        })
        .collect()
}

fn to_results(pairs: Vec<(hashing::Algorithm, String)>) -> Vec<HashResult> {
    pairs
        .into_iter()
        .map(|(algorithm, digest)| HashResult {
            algorithm: algorithm.id(),
            label: algorithm.label(),
            digest,
        })
        .collect()
}

#[tauri::command]
pub fn hash_text(input: String, algorithms: Vec<String>) -> AppResult<Vec<HashResult>> {
    let algorithms = parse_algorithms(&algorithms)?;
    Ok(to_results(hashing::hash_reader(
        &algorithms,
        input.as_bytes(),
    )?))
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileHashResult {
    pub path: String,
    pub bytes: u64,
    pub hashes: Vec<HashResult>,
}

#[tauri::command]
pub fn hash_file(path: String, algorithms: Vec<String>) -> AppResult<FileHashResult> {
    let algorithms = parse_algorithms(&algorithms)?;

    let file = std::fs::File::open(&path)
        .map_err(|e| AppError::msg(format!("no se pudo abrir «{path}»: {e}")))?;
    let bytes = file.metadata()?.len();

    // Envuelto en un lector con búfer: sin él, cada bloque sería una llamada al
    // sistema y una ISO grande tardaría minutos de más.
    let reader = std::io::BufReader::new(file);
    let hashes = to_results(hashing::hash_reader(&algorithms, reader)?);

    Ok(FileHashResult {
        path,
        bytes,
        hashes,
    })
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChecksumCheck {
    pub matches: bool,
    pub computed: String,
    pub expected: String,
    pub algorithm: &'static str,
}

/// Compara un archivo con una suma publicada.
///
/// El algoritmo se deduce de la longitud de la suma esperada, que es lo que el
/// usuario tiene a mano al copiarla de una página de descargas.
#[tauri::command]
pub fn verify_checksum(path: String, expected: String) -> AppResult<ChecksumCheck> {
    let expected = expected.trim().to_ascii_lowercase();
    let algorithm = match expected.len() {
        8 => hashing::Algorithm::Crc32,
        32 => hashing::Algorithm::Md5,
        40 => hashing::Algorithm::Sha1,
        64 => hashing::Algorithm::Sha256,
        128 => hashing::Algorithm::Sha512,
        other => {
            return Err(AppError::msg(format!(
                "una suma de {other} caracteres no corresponde a ningún algoritmo conocido"
            )))
        }
    };

    if !expected.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(AppError::msg(
            "la suma esperada debe estar en hexadecimal",
        ));
    }

    let file = std::fs::File::open(&path)
        .map_err(|e| AppError::msg(format!("no se pudo abrir «{path}»: {e}")))?;
    let computed = hashing::hash_reader(&[algorithm], std::io::BufReader::new(file))?
        .into_iter()
        .next()
        .map(|(_, digest)| digest)
        .unwrap_or_default();

    Ok(ChecksumCheck {
        // Comparación en tiempo constante por costumbre: aquí el contenido no
        // es secreto, pero la regla «nunca compares resúmenes con ==» se aplica
        // sin excepciones para que no se cuele donde sí importa.
        matches: hashing::constant_time_eq(computed.as_bytes(), expected.as_bytes()),
        computed,
        expected,
        algorithm: algorithm.id(),
    })
}

#[tauri::command]
pub fn hmac_text(input: String, key: String, algorithm: String) -> AppResult<String> {
    let algorithm = hashing::MacAlgorithm::from_id(&algorithm)
        .ok_or_else(|| AppError::msg(format!("algoritmo HMAC desconocido: «{algorithm}»")))?;
    hashing::hmac_hex(algorithm, key.as_bytes(), input.as_bytes())
}

// ── Identificadores ───────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IdKindInfo {
    pub id: &'static str,
    pub label: &'static str,
    pub note: &'static str,
}

#[tauri::command]
pub fn list_id_kinds() -> Vec<IdKindInfo> {
    ids::IdKind::ALL
        .iter()
        .map(|kind| IdKindInfo {
            id: kind.id(),
            label: kind.label(),
            note: kind.note(),
        })
        .collect()
}

#[tauri::command]
pub fn generate_ids(kind: String, count: usize, uppercase: bool) -> AppResult<Vec<String>> {
    let kind = ids::IdKind::from_id(&kind)
        .ok_or_else(|| AppError::msg(format!("tipo de identificador desconocido: «{kind}»")))?;
    ids::generate(kind, count, uppercase)
}

// ── JWT ───────────────────────────────────────────────────────────────────

#[tauri::command]
pub fn inspect_jwt(token: String, secret: Option<String>) -> AppResult<jwt::TokenInfo> {
    jwt::inspect(&token, secret.as_deref())
}

// ── Contraseñas ───────────────────────────────────────────────────────────

#[tauri::command]
pub fn generate_passwords(
    options: password::PasswordOptions,
) -> AppResult<password::GeneratedPasswords> {
    password::generate(&options)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HashPasswordRequest {
    pub password: String,
    pub scheme: String,
    pub cost: Option<u32>,
}

#[tauri::command]
pub fn hash_password(request: HashPasswordRequest) -> AppResult<String> {
    let scheme = password::HashScheme::from_id(&request.scheme)
        .ok_or_else(|| AppError::msg(format!("esquema desconocido: «{}»", request.scheme)))?;
    password::hash(&request.password, scheme, request.cost)
}

#[tauri::command]
pub fn verify_password(password_text: String, encoded: String) -> AppResult<password::VerifyResult> {
    password::verify(&password_text, &encoded)
}

// ── Certificados ──────────────────────────────────────────────────────────

#[tauri::command]
pub fn inspect_certificate(input: String) -> AppResult<cert::CertificateInfo> {
    cert::inspect(&input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formatear_json_lo_normaliza_y_valida() {
        let salida = format_document(
            "json".into(),
            r#"{"b":1,"a":[1,2]}"#.into(),
            Opts::default(),
        )
        .unwrap();

        assert!(salida.contains("\n  "), "debe indentar: {salida}");
        // Y conserva el orden original de las claves.
        assert!(salida.find("\"b\"").unwrap() < salida.find("\"a\"").unwrap());
    }

    #[test]
    fn formatear_un_documento_roto_da_el_error_del_motor() {
        let error = format_document("json".into(), "{roto".into(), Opts::default())
            .unwrap_err()
            .to_string();
        assert!(error.contains("JSON"), "{error}");
    }

    #[test]
    fn el_hash_de_texto_acepta_varios_algoritmos() {
        let resultados = hash_text(
            "abc".into(),
            vec!["md5".into(), "sha256".into()],
        )
        .unwrap();

        assert_eq!(resultados.len(), 2);
        assert_eq!(resultados[0].digest, "900150983cd24fb0d6963f7d28e17f72");
        assert_eq!(resultados[1].label, "SHA-256");
    }

    #[test]
    fn sin_algoritmos_lo_dice() {
        let error = hash_text("abc".into(), vec![]).unwrap_err().to_string();
        assert!(error.contains("al menos un algoritmo"), "{error}");
    }

    #[test]
    fn un_algoritmo_inventado_da_error_con_su_nombre() {
        let error = hash_text("abc".into(), vec!["sha3-512".into()])
            .unwrap_err()
            .to_string();
        assert!(error.contains("sha3-512"), "{error}");
    }

    #[test]
    fn la_verificacion_de_suma_deduce_el_algoritmo_por_la_longitud() {
        let dir = std::env::temp_dir().join(format!("ft-sum-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let archivo = dir.join("datos.txt");
        std::fs::write(&archivo, b"abc").unwrap();

        let esperado = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
        let resultado =
            verify_checksum(archivo.display().to_string(), esperado.into()).unwrap();

        assert!(resultado.matches);
        assert_eq!(resultado.algorithm, "sha256");

        // Y detecta una suma que no cuadra.
        let mala = "0".repeat(64);
        let resultado = verify_checksum(archivo.display().to_string(), mala).unwrap();
        assert!(!resultado.matches);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn una_suma_de_longitud_rara_se_rechaza() {
        let error = verify_checksum("x".into(), "abc".into())
            .unwrap_err()
            .to_string();
        assert!(error.contains("3 caracteres"), "{error}");
    }

    #[test]
    fn la_suma_debe_ser_hexadecimal() {
        let error = verify_checksum("x".into(), "z".repeat(64))
            .unwrap_err()
            .to_string();
        assert!(error.contains("hexadecimal"), "{error}");
    }

    #[test]
    fn el_hash_de_archivo_informa_del_tamano() {
        let dir = std::env::temp_dir().join(format!("ft-hashfile-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let archivo = dir.join("datos.bin");
        std::fs::write(&archivo, vec![0u8; 1234]).unwrap();

        let resultado =
            hash_file(archivo.display().to_string(), vec!["sha256".into()]).unwrap();

        assert_eq!(resultado.bytes, 1234);
        assert_eq!(resultado.hashes.len(), 1);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn generar_identificadores_valida_el_tipo() {
        assert_eq!(generate_ids("uuidv4".into(), 3, false).unwrap().len(), 3);

        let error = generate_ids("uuidv9".into(), 1, false)
            .unwrap_err()
            .to_string();
        assert!(error.contains("uuidv9"), "{error}");
    }

    #[test]
    fn el_catalogo_de_algoritmos_marca_los_rotos() {
        let algoritmos = list_hash_algorithms();
        let md5 = algoritmos.iter().find(|a| a.id == "md5").unwrap();
        let sha256 = algoritmos.iter().find(|a| a.id == "sha256").unwrap();

        assert!(md5.insecure_note.is_some());
        assert!(sha256.insecure_note.is_none());
    }

    #[test]
    fn verificar_contrasena_funciona_de_punta_a_punta() {
        let encoded = hash_password(HashPasswordRequest {
            password: "secreta".into(),
            scheme: "bcrypt".into(),
            cost: Some(10),
        })
        .unwrap();

        assert!(verify_password("secreta".into(), encoded.clone())
            .unwrap()
            .matches);
        assert!(!verify_password("otra".into(), encoded).unwrap().matches);
    }
}
