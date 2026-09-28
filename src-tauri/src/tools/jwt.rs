//! Inspección de JSON Web Tokens.
//!
//! Decodificar y verificar son dos cosas distintas y la interfaz tiene que
//! dejarlo claro: **cualquiera** puede leer el contenido de un JWT, porque va
//! en Base64 y no cifrado. Que el contenido se lea no significa que el token
//! sea legítimo; eso solo lo dice la firma.
//!
//! La verificación cubre la familia HMAC (HS256/384/512), que es la que se
//! puede comprobar con un secreto compartido. RS*, PS* y ES* necesitan la clave
//! pública del emisor y la maquinaria de RSA y curvas elípticas; se informa de
//! que no se verifican en lugar de dar por bueno lo que no se ha comprobado.

use base64::Engine;
use serde::Serialize;
use serde_json::Value;

use crate::error::{AppError, AppResult};
use crate::tools::hashing::{constant_time_eq, hmac_bytes, MacAlgorithm};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SignatureStatus {
    /// Firma comprobada y correcta.
    Valid,
    /// Firma comprobada y **no** correcta: el token no es de quien dice ser.
    Invalid,
    /// Algoritmo que esta herramienta no puede comprobar.
    Unsupported,
    /// No se ha dado secreto, así que no se ha comprobado nada.
    NotChecked,
    /// El token declara `alg: none`, que es un vector de ataque conocido.
    Unsecured,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenInfo {
    pub header: Value,
    pub payload: Value,
    pub signature_base64: String,
    pub algorithm: Option<String>,
    pub signature: SignatureStatus,
    pub signature_detail: String,
    /// Avisos sobre el contenido: expirado, aún no válido, sin caducidad.
    pub warnings: Vec<String>,
    pub issued_at: Option<String>,
    pub expires_at: Option<String>,
}

pub fn inspect(token: &str, secret: Option<&str>) -> AppResult<TokenInfo> {
    let token = token.trim();
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return Err(AppError::msg(format!(
            "un JWT tiene tres partes separadas por puntos y este tiene {}. \
             ¿Has copiado el token entero, sin el prefijo «Bearer»?",
            parts.len()
        )));
    }

    let header = decode_json(parts[0], "cabecera")?;
    let payload = decode_json(parts[1], "contenido")?;

    let algorithm = header
        .get("alg")
        .and_then(Value::as_str)
        .map(str::to_string);

    let (signature, signature_detail) = check_signature(
        algorithm.as_deref(),
        &format!("{}.{}", parts[0], parts[1]),
        parts[2],
        secret,
    );

    let (warnings, issued_at, expires_at) = inspect_claims(&payload);

    Ok(TokenInfo {
        header,
        payload,
        signature_base64: parts[2].to_string(),
        algorithm,
        signature,
        signature_detail,
        warnings,
        issued_at,
        expires_at,
    })
}

fn decode_json(part: &str, what: &str) -> AppResult<Value> {
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(part)
        .map_err(|e| AppError::msg(format!("la {what} no es Base64URL válido: {e}")))?;
    serde_json::from_slice(&bytes)
        .map_err(|e| AppError::msg(format!("la {what} no contiene JSON válido: {e}")))
}

fn check_signature(
    algorithm: Option<&str>,
    signed_part: &str,
    signature: &str,
    secret: Option<&str>,
) -> (SignatureStatus, String) {
    let Some(algorithm) = algorithm else {
        return (
            SignatureStatus::Unsupported,
            "La cabecera no declara algoritmo.".to_string(),
        );
    };

    if algorithm.eq_ignore_ascii_case("none") {
        return (
            SignatureStatus::Unsecured,
            "El token declara «alg: none», es decir, sin firma. Un servidor que \
             acepte esto da por válido cualquier token que le envíen."
                .to_string(),
        );
    }

    let mac = match algorithm {
        "HS256" => MacAlgorithm::Sha256,
        "HS384" => MacAlgorithm::Sha384,
        "HS512" => MacAlgorithm::Sha512,
        other => {
            return (
                SignatureStatus::Unsupported,
                format!(
                    "{other} se firma con clave asimétrica y hace falta la clave \
                     pública del emisor para comprobarlo. Aquí solo se verifican \
                     HS256, HS384 y HS512."
                ),
            )
        }
    };

    let Some(secret) = secret.filter(|s| !s.is_empty()) else {
        return (
            SignatureStatus::NotChecked,
            "Introduce el secreto para comprobar la firma. Sin ella, el contenido \
             de arriba es solo lo que el token afirma, no algo demostrado."
                .to_string(),
        );
    };

    let expected = match hmac_bytes(mac, secret.as_bytes(), signed_part.as_bytes()) {
        Ok(bytes) => bytes,
        Err(error) => return (SignatureStatus::Invalid, error.to_string()),
    };

    let actual = match base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(signature) {
        Ok(bytes) => bytes,
        Err(e) => {
            return (
                SignatureStatus::Invalid,
                format!("la firma no es Base64URL válido: {e}"),
            )
        }
    };

    if constant_time_eq(&expected, &actual) {
        (
            SignatureStatus::Valid,
            format!("Firma {algorithm} correcta para el secreto indicado."),
        )
    } else {
        (
            SignatureStatus::Invalid,
            format!(
                "La firma no corresponde a este secreto. O el secreto es otro, \
                 o el token se ha modificado después de firmarse."
            ),
        )
    }
}

/// Revisa las fechas estándar del contenido.
fn inspect_claims(payload: &Value) -> (Vec<String>, Option<String>, Option<String>) {
    let now = chrono::Utc::now().timestamp();
    let mut warnings = Vec::new();

    let read = |claim: &str| payload.get(claim).and_then(Value::as_i64);

    let expires_at = read("exp").map(|exp| {
        if exp < now {
            warnings.push(format!(
                "Caducado desde hace {}.",
                humanize(now.saturating_sub(exp))
            ));
        }
        format_timestamp(exp)
    });

    if expires_at.is_none() {
        warnings.push(
            "Sin fecha de caducidad («exp»): este token vale para siempre si no se revoca."
                .to_string(),
        );
    }

    if let Some(not_before) = read("nbf") {
        if not_before > now {
            warnings.push(format!(
                "Todavía no es válido: empieza dentro de {}.",
                humanize(not_before - now)
            ));
        }
    }

    let issued_at = read("iat").map(format_timestamp);

    (warnings, issued_at, expires_at)
}

fn format_timestamp(seconds: i64) -> String {
    chrono::DateTime::from_timestamp(seconds, 0)
        .map(|datetime| datetime.format("%Y-%m-%d %H:%M:%S UTC").to_string())
        .unwrap_or_else(|| format!("marca de tiempo fuera de rango ({seconds})"))
}

fn humanize(seconds: i64) -> String {
    const MINUTE: i64 = 60;
    const HOUR: i64 = 60 * MINUTE;
    const DAY: i64 = 24 * HOUR;

    match seconds {
        s if s < MINUTE => format!("{s} s"),
        s if s < HOUR => format!("{} min", s / MINUTE),
        s if s < DAY => format!("{} h", s / HOUR),
        s => format!("{} días", s / DAY),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::engine::general_purpose::URL_SAFE_NO_PAD as B64;

    /// Construye un token firmado de verdad, para no depender de constantes
    /// pegadas que caducarían.
    fn make_token(payload: Value, secret: &str, algorithm: &str) -> String {
        let header = serde_json::json!({"alg": algorithm, "typ": "JWT"});
        let head = B64.encode(serde_json::to_vec(&header).unwrap());
        let body = B64.encode(serde_json::to_vec(&payload).unwrap());
        let signed = format!("{head}.{body}");

        let mac = match algorithm {
            "HS384" => MacAlgorithm::Sha384,
            "HS512" => MacAlgorithm::Sha512,
            _ => MacAlgorithm::Sha256,
        };
        let signature = hmac_bytes(mac, secret.as_bytes(), signed.as_bytes()).unwrap();
        format!("{signed}.{}", B64.encode(signature))
    }

    fn future() -> i64 {
        chrono::Utc::now().timestamp() + 3600
    }

    #[test]
    fn decodifica_cabecera_y_contenido() {
        let token = make_token(
            serde_json::json!({"sub": "1234", "name": "Ana", "exp": future()}),
            "secreto",
            "HS256",
        );

        let info = inspect(&token, None).unwrap();
        assert_eq!(info.payload["name"], serde_json::json!("Ana"));
        assert_eq!(info.algorithm.as_deref(), Some("HS256"));
    }

    #[test]
    fn sin_secreto_no_afirma_que_la_firma_sea_buena() {
        let token = make_token(serde_json::json!({"exp": future()}), "secreto", "HS256");

        let info = inspect(&token, None).unwrap();
        assert_eq!(info.signature, SignatureStatus::NotChecked);
        assert!(info.signature_detail.contains("secreto"));
    }

    #[test]
    fn valida_la_firma_con_el_secreto_correcto() {
        let token = make_token(serde_json::json!({"exp": future()}), "secreto", "HS256");

        let info = inspect(&token, Some("secreto")).unwrap();
        assert_eq!(info.signature, SignatureStatus::Valid);
    }

    #[test]
    fn detecta_el_secreto_equivocado() {
        let token = make_token(serde_json::json!({"exp": future()}), "secreto", "HS256");

        let info = inspect(&token, Some("otro")).unwrap();
        assert_eq!(info.signature, SignatureStatus::Invalid);
    }

    #[test]
    fn detecta_un_contenido_manipulado() {
        let token = make_token(
            serde_json::json!({"admin": false, "exp": future()}),
            "secreto",
            "HS256",
        );
        let mut parts: Vec<String> = token.split('.').map(str::to_string).collect();
        parts[1] = B64.encode(
            serde_json::to_vec(&serde_json::json!({"admin": true, "exp": future()})).unwrap(),
        );
        let manipulado = parts.join(".");

        let info = inspect(&manipulado, Some("secreto")).unwrap();
        assert_eq!(info.signature, SignatureStatus::Invalid);
        assert!(info.signature_detail.contains("modificado"), "{}", info.signature_detail);
    }

    #[test]
    fn verifica_las_tres_variantes_de_hmac() {
        for algorithm in ["HS256", "HS384", "HS512"] {
            let token = make_token(serde_json::json!({"exp": future()}), "s3creto", algorithm);
            let info = inspect(&token, Some("s3creto")).unwrap();
            assert_eq!(info.signature, SignatureStatus::Valid, "falló {algorithm}");
        }
    }

    #[test]
    fn marca_alg_none_como_inseguro() {
        let header = B64.encode(br#"{"alg":"none","typ":"JWT"}"#);
        let body = B64.encode(br#"{"admin":true}"#);
        let token = format!("{header}.{body}.");

        let info = inspect(&token, Some("da igual")).unwrap();
        assert_eq!(info.signature, SignatureStatus::Unsecured);
        assert!(info.signature_detail.contains("cualquier token"));
    }

    #[test]
    fn dice_que_no_puede_con_los_asimetricos() {
        let header = B64.encode(br#"{"alg":"RS256"}"#);
        let body = B64.encode(br#"{"sub":"1"}"#);
        let token = format!("{header}.{body}.firma");

        let info = inspect(&token, Some("secreto")).unwrap();
        assert_eq!(info.signature, SignatureStatus::Unsupported);
        assert!(info.signature_detail.contains("clave pública"));
    }

    #[test]
    fn avisa_de_los_tokens_caducados() {
        let pasado = chrono::Utc::now().timestamp() - 7200;
        let token = make_token(serde_json::json!({"exp": pasado}), "s", "HS256");

        let info = inspect(&token, None).unwrap();
        assert!(
            info.warnings.iter().any(|w| w.contains("Caducado")),
            "{:?}",
            info.warnings
        );
    }

    #[test]
    fn avisa_de_los_tokens_sin_caducidad() {
        let token = make_token(serde_json::json!({"sub": "1"}), "s", "HS256");

        let info = inspect(&token, None).unwrap();
        assert!(
            info.warnings.iter().any(|w| w.contains("para siempre")),
            "{:?}",
            info.warnings
        );
    }

    #[test]
    fn avisa_de_los_tokens_que_aun_no_valen() {
        let token = make_token(
            serde_json::json!({"nbf": future(), "exp": future() + 60}),
            "s",
            "HS256",
        );

        let info = inspect(&token, None).unwrap();
        assert!(
            info.warnings.iter().any(|w| w.contains("Todavía no es válido")),
            "{:?}",
            info.warnings
        );
    }

    #[test]
    fn un_token_mal_formado_explica_el_problema() {
        let error = inspect("solo.dos", None).unwrap_err().to_string();
        assert!(error.contains("tres partes"), "{error}");
        assert!(error.contains("Bearer"), "{error}");
    }

    #[test]
    fn humaniza_las_duraciones() {
        assert_eq!(humanize(30), "30 s");
        assert_eq!(humanize(600), "10 min");
        assert_eq!(humanize(7200), "2 h");
        assert_eq!(humanize(172_800), "2 días");
    }
}
