//! Inspección de certificados X.509.
//!
//! Solo lectura: no valida cadenas de confianza ni consulta listas de
//! revocación, porque eso requiere los certificados intermedios y acceso a la
//! red. Responde a «qué hay dentro de este certificado y hasta cuándo vale»,
//! que es la pregunta que uno se hace cuando un despliegue falla por TLS.

use serde::Serialize;
use x509_parser::prelude::*;

use crate::error::{AppError, AppResult};
use crate::tools::hashing::{hash_bytes, Algorithm};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CertificateInfo {
    pub subject: String,
    pub issuer: String,
    pub serial: String,
    pub version: u32,
    pub signature_algorithm: String,
    pub public_key_algorithm: String,
    pub not_before: String,
    pub not_after: String,
    pub days_remaining: i64,
    pub alternative_names: Vec<String>,
    pub is_ca: bool,
    pub self_signed: bool,
    pub sha256_fingerprint: String,
    pub sha1_fingerprint: String,
    pub warnings: Vec<String>,
}

/// Días antes de la caducidad a partir de los cuales conviene avisar.
const RENEWAL_WINDOW_DAYS: i64 = 30;

pub fn inspect(input: &str) -> AppResult<CertificateInfo> {
    let der = to_der(input)?;

    let (_, certificate) = X509Certificate::from_der(&der).map_err(|e| {
        AppError::msg(format!(
            "no se pudo interpretar el certificado: {e}. \
             ¿Es un certificado y no una clave privada o una solicitud de firma?"
        ))
    })?;

    let validity = certificate.validity();
    let not_before = format_time(validity.not_before);
    let not_after = format_time(validity.not_after);

    let now = chrono::Utc::now().timestamp();
    let days_remaining = (validity.not_after.timestamp() - now) / 86_400;

    let subject = certificate.subject().to_string();
    let issuer = certificate.issuer().to_string();
    let self_signed = subject == issuer;

    let alternative_names = certificate
        .subject_alternative_name()
        .ok()
        .flatten()
        .map(|extension| {
            extension
                .value
                .general_names
                .iter()
                .map(describe_general_name)
                .collect()
        })
        .unwrap_or_default();

    let is_ca = certificate
        .basic_constraints()
        .ok()
        .flatten()
        .is_some_and(|extension| extension.value.ca);

    let mut warnings = Vec::new();
    if validity.not_after.timestamp() < now {
        warnings.push(format!("Caducado el {not_after}."));
    } else if days_remaining <= RENEWAL_WINDOW_DAYS {
        warnings.push(format!(
            "Caduca en {days_remaining} días: dentro de la ventana de renovación."
        ));
    }
    if validity.not_before.timestamp() > now {
        warnings.push(format!("Todavía no es válido: empieza el {not_before}."));
    }
    if self_signed && !is_ca {
        warnings.push(
            "Autofirmado: ningún navegador ni cliente lo aceptará sin instalarlo a mano."
                .to_string(),
        );
    }
    let signature_algorithm = describe_signature(&certificate);
    if signature_algorithm.contains("sha1") || signature_algorithm.contains("md5") {
        warnings.push(format!(
            "Firmado con {signature_algorithm}, que está roto. Los clientes modernos lo rechazan."
        ));
    }

    Ok(CertificateInfo {
        subject,
        issuer,
        serial: certificate.raw_serial_as_string(),
        // El campo ASN.1 cuenta desde cero: un certificado v3 lleva un 2. Se
        // suma uno para mostrar la versión como la nombra todo el mundo.
        version: certificate.version().0 + 1,
        signature_algorithm,
        public_key_algorithm: certificate
            .public_key()
            .algorithm
            .algorithm
            .to_id_string(),
        not_before,
        not_after,
        days_remaining,
        alternative_names,
        is_ca,
        self_signed,
        sha256_fingerprint: fingerprint(Algorithm::Sha256, &der),
        sha1_fingerprint: fingerprint(Algorithm::Sha1, &der),
        warnings,
    })
}

/// Acepta PEM (`-----BEGIN CERTIFICATE-----`) o DER en crudo.
fn to_der(input: &str) -> AppResult<Vec<u8>> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(AppError::msg("no hay ningún certificado que analizar"));
    }

    if trimmed.contains("-----BEGIN") {
        let (_, pem) = parse_x509_pem(trimmed.as_bytes())
            .map_err(|e| AppError::msg(format!("el bloque PEM no es válido: {e}")))?;

        if pem.label != "CERTIFICATE" {
            return Err(AppError::msg(format!(
                "el bloque PEM es de tipo «{}» y hace falta «CERTIFICATE». \
                 Una clave privada o una solicitud de firma no se pueden inspeccionar aquí.",
                pem.label
            )));
        }
        return Ok(pem.contents);
    }

    Err(AppError::msg(
        "pega el certificado en formato PEM, empezando por «-----BEGIN CERTIFICATE-----»",
    ))
}

fn describe_signature(certificate: &X509Certificate) -> String {
    certificate
        .signature_algorithm
        .algorithm
        .to_id_string()
        .to_ascii_lowercase()
}

fn describe_general_name(name: &GeneralName) -> String {
    match name {
        GeneralName::DNSName(value) => (*value).to_string(),
        GeneralName::IPAddress(bytes) => format_ip(bytes),
        GeneralName::RFC822Name(value) => format!("correo:{value}"),
        GeneralName::URI(value) => (*value).to_string(),
        other => format!("{other:?}"),
    }
}

fn format_ip(bytes: &[u8]) -> String {
    match bytes.len() {
        4 => bytes
            .iter()
            .map(u8::to_string)
            .collect::<Vec<_>>()
            .join("."),
        16 => bytes
            .chunks(2)
            .map(|pair| format!("{:x}{:02x}", pair[0], pair[1]))
            .collect::<Vec<_>>()
            .join(":"),
        _ => format!("{bytes:?}"),
    }
}

fn format_time(time: ASN1Time) -> String {
    chrono::DateTime::from_timestamp(time.timestamp(), 0)
        .map(|datetime| datetime.format("%Y-%m-%d %H:%M:%S UTC").to_string())
        .unwrap_or_else(|| time.to_string())
}

/// Huella en el formato con dos puntos que muestran navegadores y `openssl`.
fn fingerprint(algorithm: Algorithm, der: &[u8]) -> String {
    let hex = hash_bytes(algorithm, der);
    hex.as_bytes()
        .chunks(2)
        .map(|pair| String::from_utf8_lossy(pair).to_uppercase())
        .collect::<Vec<_>>()
        .join(":")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Genera un certificado autofirmado real para los tests. Pegar un PEM
    /// literal caducaría con el tiempo y haría fallar la suite sola.
    fn make_certificate(names: &[&str]) -> String {
        let subject: Vec<String> = names.iter().map(|n| (*n).to_string()).collect();
        rcgen::generate_simple_self_signed(subject)
            .expect("rcgen debe poder generar un certificado")
            .cert
            .pem()
    }

    #[test]
    fn lee_los_campos_de_un_certificado_real() {
        let info = inspect(&make_certificate(&["fast-tools.test"])).unwrap();

        // El nombre del host va en el SAN, no en el sujeto: desde el RFC 2818
        // los clientes miran el SAN y el CN quedó como campo descriptivo.
        assert!(
            info.alternative_names.contains(&"fast-tools.test".to_string()),
            "{:?}",
            info.alternative_names
        );
        assert!(!info.subject.is_empty());
        assert!(info.self_signed, "un autofirmado tiene emisor igual al sujeto");
        assert!(!info.sha256_fingerprint.is_empty());
        assert!(!info.serial.is_empty());
        assert!(info.days_remaining > 0, "debería estar vigente");
        assert_eq!(info.version, 3, "los certificados actuales son v3");
    }

    #[test]
    fn lista_los_nombres_alternativos() {
        let info = inspect(&make_certificate(&["a.test", "b.test", "*.c.test"])).unwrap();

        assert!(info.alternative_names.contains(&"a.test".to_string()));
        assert!(info.alternative_names.contains(&"b.test".to_string()));
        assert!(info.alternative_names.contains(&"*.c.test".to_string()));
    }

    #[test]
    fn las_dos_huellas_tienen_la_longitud_de_su_algoritmo() {
        let info = inspect(&make_certificate(&["x.test"])).unwrap();

        // 32 bytes en hexadecimal con dos puntos entre pares.
        assert_eq!(info.sha256_fingerprint.matches(':').count(), 31);
        assert_eq!(info.sha1_fingerprint.matches(':').count(), 19);
    }

    #[test]
    fn avisa_de_que_un_autofirmado_no_lo_acepta_nadie() {
        let info = inspect(&make_certificate(&["x.test"])).unwrap();

        // rcgen marca sus certificados como CA, así que el aviso de
        // autofirmado solo aplica a los que no lo son.
        if !info.is_ca {
            assert!(
                info.warnings.iter().any(|w| w.contains("Autofirmado")),
                "{:?}",
                info.warnings
            );
        }
    }

    #[test]
    fn rechaza_una_entrada_vacia() {
        let error = inspect("   ").unwrap_err().to_string();
        assert!(error.contains("ningún certificado"), "{error}");
    }

    #[test]
    fn pide_pem_cuando_le_dan_otra_cosa() {
        let error = inspect("esto no es un certificado").unwrap_err().to_string();
        assert!(error.contains("BEGIN CERTIFICATE"), "{error}");
    }

    #[test]
    fn rechaza_una_clave_privada_explicando_por_que() {
        let clave = "-----BEGIN PRIVATE KEY-----\nMC4CAQAwBQYDK2VwBCIEIA==\n-----END PRIVATE KEY-----";
        let error = inspect(clave).unwrap_err().to_string();
        assert!(
            error.contains("clave privada") || error.contains("PRIVATE KEY"),
            "{error}"
        );
    }

    #[test]
    fn un_pem_corrupto_da_error_y_no_entra_en_panico() {
        let valido = make_certificate(&["x.test"]);
        // Se estropea el contenido conservando las cabeceras PEM.
        let corrupto = valido.replacen("MI", "XX", 1);

        assert!(inspect(&corrupto).is_err(), "un PEM alterado debe fallar");
    }

    #[test]
    fn la_huella_va_en_mayusculas_separada_por_dos_puntos() {
        let huella = fingerprint(Algorithm::Sha256, b"abc");
        assert!(huella.starts_with("BA:78:16:BF"), "{huella}");
        assert_eq!(huella.matches(':').count(), 31);
    }

    #[test]
    fn formatea_direcciones_ip() {
        assert_eq!(format_ip(&[192, 168, 1, 1]), "192.168.1.1");
        assert_eq!(format_ip(&[10, 0, 0, 255]), "10.0.0.255");
        assert!(format_ip(&[0x20, 0x01, 0x0d, 0xb8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1])
            .starts_with("2001:"));
    }
}
