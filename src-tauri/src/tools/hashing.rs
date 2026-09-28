//! Funciones resumen y códigos de autenticación.
//!
//! Los archivos se procesan por bloques y no cargándolos en memoria: una ISO de
//! 8 GB tiene que poder comprobarse en una máquina con 8 GB de RAM.

use std::io::Read;

use md5::Digest;
use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};

/// Tamaño de bloque al leer archivos. 64 KiB es el punto donde deja de mejorar
/// el rendimiento y solo se gasta memoria.
const CHUNK: usize = 64 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Algorithm {
    Md5,
    Sha1,
    Sha256,
    Sha512,
    Blake3,
    Crc32,
}

impl Algorithm {
    pub const ALL: &'static [Algorithm] = &[
        Algorithm::Md5,
        Algorithm::Sha1,
        Algorithm::Sha256,
        Algorithm::Sha512,
        Algorithm::Blake3,
        Algorithm::Crc32,
    ];

    pub fn id(self) -> &'static str {
        match self {
            Algorithm::Md5 => "md5",
            Algorithm::Sha1 => "sha1",
            Algorithm::Sha256 => "sha256",
            Algorithm::Sha512 => "sha512",
            Algorithm::Blake3 => "blake3",
            Algorithm::Crc32 => "crc32",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Algorithm::Md5 => "MD5",
            Algorithm::Sha1 => "SHA-1",
            Algorithm::Sha256 => "SHA-256",
            Algorithm::Sha512 => "SHA-512",
            Algorithm::Blake3 => "BLAKE3",
            Algorithm::Crc32 => "CRC32",
        }
    }

    /// Por qué no debe usarse para nada que dependa de la seguridad.
    ///
    /// La interfaz muestra este aviso junto al resultado. Estos algoritmos
    /// siguen estando porque hacen falta para comprobar descargas antiguas y
    /// para comparar con sistemas heredados, no porque sean recomendables.
    pub fn insecure_note(self) -> Option<&'static str> {
        match self {
            Algorithm::Md5 => Some(
                "Roto: se pueden fabricar dos archivos distintos con el mismo MD5. \
                 Solo para comprobar descargas, nunca para firmas ni contraseñas.",
            ),
            Algorithm::Sha1 => Some(
                "Roto desde 2017: hay colisiones demostradas. \
                 Solo por compatibilidad con sistemas antiguos.",
            ),
            Algorithm::Crc32 => Some(
                "No es criptográfico: detecta errores de transmisión, \
                 no manipulación deliberada.",
            ),
            _ => None,
        }
    }

    pub fn from_id(id: &str) -> Option<Algorithm> {
        Algorithm::ALL
            .iter()
            .copied()
            .find(|a| a.id().eq_ignore_ascii_case(id))
    }
}

/// Estado de resumen en curso, para poder alimentarlo por bloques.
enum Digester {
    Md5(md5::Md5),
    Sha1(sha1::Sha1),
    Sha256(sha2::Sha256),
    Sha512(sha2::Sha512),
    Blake3(Box<blake3::Hasher>),
    Crc32(crc32fast::Hasher),
}

impl Digester {
    fn new(algorithm: Algorithm) -> Digester {
        match algorithm {
            Algorithm::Md5 => Digester::Md5(md5::Md5::new()),
            Algorithm::Sha1 => Digester::Sha1(sha1::Sha1::new()),
            Algorithm::Sha256 => Digester::Sha256(sha2::Sha256::new()),
            Algorithm::Sha512 => Digester::Sha512(sha2::Sha512::new()),
            Algorithm::Blake3 => Digester::Blake3(Box::new(blake3::Hasher::new())),
            Algorithm::Crc32 => Digester::Crc32(crc32fast::Hasher::new()),
        }
    }

    fn update(&mut self, bytes: &[u8]) {
        match self {
            Digester::Md5(state) => state.update(bytes),
            Digester::Sha1(state) => state.update(bytes),
            Digester::Sha256(state) => state.update(bytes),
            Digester::Sha512(state) => state.update(bytes),
            Digester::Blake3(state) => {
                state.update(bytes);
            }
            Digester::Crc32(state) => state.update(bytes),
        }
    }

    fn finish(self) -> String {
        match self {
            Digester::Md5(state) => hex(&state.finalize()),
            Digester::Sha1(state) => hex(&state.finalize()),
            Digester::Sha256(state) => hex(&state.finalize()),
            Digester::Sha512(state) => hex(&state.finalize()),
            Digester::Blake3(state) => state.finalize().to_hex().to_string(),
            Digester::Crc32(state) => format!("{:08x}", state.finalize()),
        }
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().fold(String::new(), |mut out, byte| {
        use std::fmt::Write;
        let _ = write!(out, "{byte:02x}");
        out
    })
}

pub fn hash_bytes(algorithm: Algorithm, bytes: &[u8]) -> String {
    let mut digester = Digester::new(algorithm);
    digester.update(bytes);
    digester.finish()
}

/// Calcula varios resúmenes en una sola pasada por el archivo.
pub fn hash_reader(
    algorithms: &[Algorithm],
    mut reader: impl Read,
) -> AppResult<Vec<(Algorithm, String)>> {
    let mut digesters: Vec<(Algorithm, Digester)> = algorithms
        .iter()
        .map(|&algorithm| (algorithm, Digester::new(algorithm)))
        .collect();

    let mut buffer = vec![0u8; CHUNK];
    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        for (_, digester) in &mut digesters {
            digester.update(&buffer[..read]);
        }
    }

    Ok(digesters
        .into_iter()
        .map(|(algorithm, digester)| (algorithm, digester.finish()))
        .collect())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MacAlgorithm {
    Sha256,
    Sha384,
    Sha512,
}

impl MacAlgorithm {
    pub fn from_id(id: &str) -> Option<MacAlgorithm> {
        match id.to_ascii_lowercase().as_str() {
            "sha256" => Some(MacAlgorithm::Sha256),
            "sha384" => Some(MacAlgorithm::Sha384),
            "sha512" => Some(MacAlgorithm::Sha512),
            _ => None,
        }
    }
}

/// HMAC en crudo. La comparación de dos HMAC debe hacerse con
/// [`constant_time_eq`], nunca con `==` sobre las cadenas hexadecimales.
pub fn hmac_bytes(algorithm: MacAlgorithm, key: &[u8], message: &[u8]) -> AppResult<Vec<u8>> {
    use hmac::{Hmac, Mac};

    macro_rules! run {
        ($hash:ty) => {{
            let mut mac = Hmac::<$hash>::new_from_slice(key)
                .map_err(|e| AppError::msg(format!("clave no válida: {e}")))?;
            mac.update(message);
            mac.finalize().into_bytes().to_vec()
        }};
    }

    Ok(match algorithm {
        MacAlgorithm::Sha256 => run!(sha2::Sha256),
        MacAlgorithm::Sha384 => run!(sha2::Sha384),
        MacAlgorithm::Sha512 => run!(sha2::Sha512),
    })
}

pub fn hmac_hex(algorithm: MacAlgorithm, key: &[u8], message: &[u8]) -> AppResult<String> {
    Ok(hex(&hmac_bytes(algorithm, key, message)?))
}

/// Comparación en tiempo constante.
///
/// Comparar firmas con `==` termina en cuanto encuentra el primer byte
/// distinto, y ese tiempo revela cuántos bytes iniciales eran correctos: basta
/// para reconstruir una firma válida byte a byte.
pub fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.iter()
        .zip(right)
        .fold(0u8, |acc, (a, b)| acc | (a ^ b))
        == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    // Vectores del RFC 1321 (MD5), RFC 3174 (SHA-1) y FIPS 180-4.
    #[test]
    fn coincide_con_los_vectores_de_prueba_conocidos() {
        assert_eq!(
            hash_bytes(Algorithm::Md5, b"abc"),
            "900150983cd24fb0d6963f7d28e17f72"
        );
        assert_eq!(
            hash_bytes(Algorithm::Sha1, b"abc"),
            "a9993e364706816aba3e25717850c26c9cd0d89d"
        );
        assert_eq!(
            hash_bytes(Algorithm::Sha256, b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            hash_bytes(Algorithm::Sha256, b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            hash_bytes(Algorithm::Blake3, b"abc"),
            "6437b3ac38465133ffb63b75273a8db548c558465d79db03fd359c6cd5bd9d85"
        );
    }

    #[test]
    fn el_crc32_usa_el_polinomio_estandar() {
        // Valor canónico de «123456789» para CRC-32/ISO-HDLC.
        assert_eq!(hash_bytes(Algorithm::Crc32, b"123456789"), "cbf43926");
    }

    #[test]
    fn leer_por_bloques_da_lo_mismo_que_de_una_vez() {
        // Más de un bloque para forzar varias iteraciones.
        let datos = vec![b'x'; CHUNK * 2 + 17];

        let por_bloques = hash_reader(&[Algorithm::Sha256], datos.as_slice()).unwrap();
        assert_eq!(por_bloques[0].1, hash_bytes(Algorithm::Sha256, &datos));
    }

    #[test]
    fn una_sola_pasada_calcula_varios_algoritmos() {
        let resultados =
            hash_reader(&[Algorithm::Md5, Algorithm::Sha256], b"abc".as_slice()).unwrap();

        assert_eq!(resultados.len(), 2);
        assert_eq!(resultados[0].1, hash_bytes(Algorithm::Md5, b"abc"));
        assert_eq!(resultados[1].1, hash_bytes(Algorithm::Sha256, b"abc"));
    }

    // Vector del RFC 4231, caso 1.
    #[test]
    fn el_hmac_coincide_con_el_rfc_4231() {
        let key = [0x0b; 20];
        let resultado = hmac_hex(MacAlgorithm::Sha256, &key, b"Hi There").unwrap();
        assert_eq!(
            resultado,
            "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7"
        );
    }

    #[test]
    fn los_algoritmos_rotos_llevan_aviso_y_los_buenos_no() {
        assert!(Algorithm::Md5.insecure_note().is_some());
        assert!(Algorithm::Sha1.insecure_note().is_some());
        assert!(Algorithm::Crc32.insecure_note().is_some());
        assert!(Algorithm::Sha256.insecure_note().is_none());
        assert!(Algorithm::Blake3.insecure_note().is_none());
    }

    #[test]
    fn la_comparacion_en_tiempo_constante_distingue_bien() {
        assert!(constant_time_eq(b"abc", b"abc"));
        assert!(!constant_time_eq(b"abc", b"abd"));
        assert!(!constant_time_eq(b"abc", b"ab"));
        assert!(constant_time_eq(b"", b""));
    }

    #[test]
    fn los_identificadores_redondean() {
        for algorithm in Algorithm::ALL {
            assert_eq!(Algorithm::from_id(algorithm.id()), Some(*algorithm));
        }
        assert!(Algorithm::from_id("sha3").is_none());
    }
}
