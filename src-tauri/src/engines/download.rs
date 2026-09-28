//! Descarga, verificación y extracción de motores externos.
//!
//! Este es el único punto del programa que trae código ejecutable de internet,
//! así que concentra las comprobaciones:
//!
//! 1. La URL tiene que ser HTTPS y estar en la lista de anfitriones permitidos.
//! 2. El archivo se descarga a un temporal mientras se calcula su SHA-256.
//! 3. Si el hash no coincide con el fijado en el manifiesto, se borra la
//!    descarga y se aborta. No hay reintento automático ni forma de saltárselo.
//! 4. La extracción rechaza cualquier entrada que escape del directorio de
//!    destino, que es como un archivo comprimido preparado a mala fe
//!    sobrescribiría archivos del sistema.
//! 5. Tras extraer, el ejecutable esperado tiene que estar donde dice el
//!    manifiesto; si no, la instalación se deshace.

use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};

use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use sha2::Digest;

use super::manifest::{is_allowed_url, EngineSpec};
use crate::error::{AppError, AppResult};
use crate::tools::hashing::constant_time_eq;

/// Nombre del registro que queda junto al motor instalado.
pub const RECORD_FILE: &str = "instalado.json";

/// Lo que se guarda al instalar, para poder detectar después si alguien ha
/// cambiado el ejecutable por otro.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallRecord {
    pub version: String,
    /// Ruta del ejecutable, relativa a la carpeta del motor.
    pub executable: String,
    /// SHA-256 del archivo descargado, tal como se verificó.
    pub archive_sha256: String,
    /// SHA-256 del ejecutable ya extraído.
    pub executable_sha256: String,
    pub executable_size: u64,
    pub installed_at: String,
}

pub struct Progress {
    pub downloaded: u64,
    pub total: u64,
}

/// Descarga, verifica, extrae y deja el motor listo en `engine_dir`.
///
/// `on_progress` se llama durante la descarga; el resto de fases no informan
/// porque duran segundos frente a los minutos que tarda bajar 190 MB.
pub async fn install<F>(
    spec: &EngineSpec,
    engine_dir: &Path,
    mut on_progress: F,
) -> AppResult<InstallRecord>
where
    F: FnMut(Progress),
{
    let download = spec.download.as_ref().ok_or_else(|| {
        AppError::msg(format!(
            "{} no se puede descargar: {}",
            spec.name,
            spec.no_download_reason.unwrap_or("no hay origen definido")
        ))
    })?;

    // Regla 1: nada de orígenes no permitidos, nada sin cifrar.
    if !is_allowed_url(download.url) {
        return Err(AppError::msg(format!(
            "la dirección de descarga de {} no está permitida: {}",
            spec.name, download.url
        )));
    }

    std::fs::create_dir_all(engine_dir)?;
    let archive_path = engine_dir.join("descarga.parcial");

    // Regla 2: se calcula el hash sobre la marcha, sin cargar 190 MB en memoria.
    let actual_hash = match fetch_to_file(download.url, &archive_path, download.size_bytes, &mut on_progress).await {
        Ok(hash) => hash,
        Err(error) => {
            let _ = std::fs::remove_file(&archive_path);
            return Err(error);
        }
    };

    // Regla 3: si no cuadra, se borra y se aborta. Sin excepciones.
    if !constant_time_eq(actual_hash.as_bytes(), download.sha256.as_bytes()) {
        let _ = std::fs::remove_file(&archive_path);
        return Err(AppError::msg(format!(
            "El archivo descargado de {} no es el esperado.\n\
             Esperado: {}\n\
             Obtenido: {}\n\
             La descarga se ha borrado. No se ha ejecutado nada.",
            spec.name, download.sha256, actual_hash
        )));
    }

    let extracted = engine_dir.join("contenido");
    // Una instalación anterior a medias dejaría archivos mezclados.
    if extracted.exists() {
        std::fs::remove_dir_all(&extracted)?;
    }
    let extract_result = extract_zip(&archive_path, &extracted);
    let _ = std::fs::remove_file(&archive_path);
    extract_result?;

    // Regla 5: el ejecutable tiene que estar donde dice el manifiesto.
    let executable = extracted.join(download.executable_in_archive.replace('/', std::path::MAIN_SEPARATOR_STR));
    if !executable.is_file() {
        let _ = std::fs::remove_dir_all(&extracted);
        return Err(AppError::msg(format!(
            "el archivo de {} no contiene «{}» donde se esperaba; \
             la instalación se ha deshecho",
            spec.name, download.executable_in_archive
        )));
    }

    let metadata = std::fs::metadata(&executable)?;
    let record = InstallRecord {
        version: download.version.to_string(),
        executable: download.executable_in_archive.to_string(),
        archive_sha256: download.sha256.to_string(),
        executable_sha256: hash_file(&executable)?,
        executable_size: metadata.len(),
        installed_at: chrono::Utc::now().to_rfc3339(),
    };

    std::fs::write(
        engine_dir.join(RECORD_FILE),
        serde_json::to_vec_pretty(&record)
            .map_err(|e| AppError::msg(format!("no se pudo escribir el registro: {e}")))?,
    )?;

    Ok(record)
}

async fn fetch_to_file<F>(
    url: &str,
    destination: &Path,
    expected_size: u64,
    on_progress: &mut F,
) -> AppResult<String>
where
    F: FnMut(Progress),
{
    let client = reqwest::Client::builder()
        .user_agent(concat!("fast-tools/", env!("CARGO_PKG_VERSION")))
        // Sin esto, un servidor que responda «301 → http://» degradaría la
        // conexión a texto plano sin que nadie se entere.
        .https_only(true)
        .build()
        .map_err(|e| AppError::msg(format!("no se pudo preparar la descarga: {e}")))?;

    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| AppError::msg(format!("no se pudo conectar: {e}")))?;

    if !response.status().is_success() {
        return Err(AppError::msg(format!(
            "el servidor respondió {} al pedir el archivo",
            response.status()
        )));
    }

    let total = response.content_length().unwrap_or(expected_size);
    let mut file = std::fs::File::create(destination)?;
    let mut hasher = sha2::Sha256::new();
    let mut downloaded: u64 = 0;
    let mut stream = response.bytes_stream();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| AppError::msg(format!("se cortó la descarga: {e}")))?;

        downloaded += chunk.len() as u64;
        // Un servidor que enviase mucho más de lo anunciado llenaría el disco.
        if downloaded > expected_size.saturating_mul(2) {
            return Err(AppError::msg(
                "el servidor está enviando mucho más de lo anunciado; descarga abortada",
            ));
        }

        hasher.update(&chunk);
        file.write_all(&chunk)?;
        on_progress(Progress { downloaded, total });
    }

    file.sync_all()?;
    Ok(hex(&hasher.finalize()))
}

pub fn hash_file(path: &Path) -> AppResult<String> {
    let mut file = std::fs::File::open(path)?;
    let mut hasher = sha2::Sha256::new();
    let mut buffer = vec![0u8; 64 * 1024];

    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hex(&hasher.finalize()))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().fold(String::new(), |mut out, byte| {
        use std::fmt::Write;
        let _ = write!(out, "{byte:02x}");
        out
    })
}

/// Extrae un ZIP negándose a escribir fuera del directorio de destino.
///
/// Un archivo comprimido preparado a mala fe puede declarar rutas como
/// `../../Windows/System32/x.dll` o rutas absolutas. Escribirlas tal cual es el
/// fallo conocido como «zip slip», y permite sobrescribir cualquier archivo al
/// que llegue el proceso.
pub fn extract_zip(archive_path: &Path, destination: &Path) -> AppResult<usize> {
    let file = std::fs::File::open(archive_path)?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| AppError::msg(format!("el archivo comprimido no es válido: {e}")))?;

    std::fs::create_dir_all(destination)?;
    let root = destination
        .canonicalize()
        .map_err(|e| AppError::msg(format!("no se pudo resolver el destino: {e}")))?;

    let mut written = 0usize;

    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .map_err(|e| AppError::msg(format!("no se pudo leer una entrada: {e}")))?;

        let raw_name = entry.name().to_string();
        let relative = safe_relative_path(&raw_name).ok_or_else(|| {
            AppError::msg(format!(
                "el archivo comprimido contiene una ruta peligrosa y se ha rechazado entero: «{raw_name}»"
            ))
        })?;

        let target = root.join(&relative);

        if entry.is_dir() {
            std::fs::create_dir_all(&target)?;
            continue;
        }

        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let mut out = std::fs::File::create(&target)?;
        std::io::copy(&mut entry, &mut out)?;
        written += 1;
    }

    Ok(written)
}

/// Convierte el nombre de una entrada en una ruta relativa segura, o `None`.
///
/// Se rechaza todo lo que no sea una secuencia de nombres normales: rutas
/// absolutas, letras de unidad, rutas UNC y cualquier `..`.
pub fn safe_relative_path(name: &str) -> Option<PathBuf> {
    if name.is_empty() {
        return None;
    }
    // Windows acepta «/» y «\» como separador; el ZIP usa «/», pero una entrada
    // maliciosa puede traer «\» para colarse por otra ruta.
    if name.contains('\\') || name.contains(':') {
        return None;
    }

    let candidate = Path::new(name);
    let mut safe = PathBuf::new();

    for component in candidate.components() {
        match component {
            Component::Normal(part) => safe.push(part),
            // Un componente vacío o «.» no aporta nada y se ignora.
            Component::CurDir => {}
            // Cualquier otra cosa sale del destino.
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => return None,
        }
    }

    if safe.as_os_str().is_empty() {
        None
    } else {
        Some(safe)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("ft-eng-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn make_zip(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let mut buffer = Cursor::new(Vec::new());
        {
            let mut writer = zip::ZipWriter::new(&mut buffer);
            let options: zip::write::FileOptions<'_, ()> = zip::write::FileOptions::default();
            for (name, contents) in entries {
                writer.start_file(*name, options).unwrap();
                writer.write_all(contents).unwrap();
            }
            writer.finish().unwrap();
        }
        buffer.into_inner()
    }

    #[test]
    fn extrae_un_archivo_normal() {
        let dir = temp_dir("extract");
        let archive = dir.join("a.zip");
        std::fs::write(
            &archive,
            make_zip(&[("carpeta/archivo.txt", b"hola"), ("raiz.txt", b"mundo")]),
        )
        .unwrap();

        let destination = dir.join("salida");
        let written = extract_zip(&archive, &destination).unwrap();

        assert_eq!(written, 2);
        assert_eq!(
            std::fs::read_to_string(destination.join("carpeta/archivo.txt")).unwrap(),
            "hola"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn rechaza_un_archivo_que_intenta_salir_del_destino() {
        let dir = temp_dir("slip");
        let archive = dir.join("malo.zip");
        std::fs::write(
            &archive,
            make_zip(&[("../../evil.exe", b"carga maliciosa")]),
        )
        .unwrap();

        let destination = dir.join("salida");
        let error = extract_zip(&archive, &destination).unwrap_err().to_string();

        assert!(error.contains("ruta peligrosa"), "{error}");
        // Y no ha escrito nada fuera.
        assert!(!dir.join("evil.exe").exists());
        assert!(!dir.parent().unwrap().join("evil.exe").exists());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn la_comprobacion_de_rutas_cubre_todas_las_formas_de_escapar() {
        // Formas válidas.
        assert!(safe_relative_path("a.txt").is_some());
        assert!(safe_relative_path("dir/sub/a.txt").is_some());
        assert!(safe_relative_path("./a.txt").is_some());

        // Escapes por ruta relativa.
        assert!(safe_relative_path("../a.txt").is_none());
        assert!(safe_relative_path("dir/../../a.txt").is_none());
        assert!(safe_relative_path("..").is_none());

        // Rutas absolutas de Unix y de Windows.
        assert!(safe_relative_path("/etc/passwd").is_none());
        assert!(safe_relative_path("C:/Windows/x.dll").is_none());
        assert!(safe_relative_path(r"C:\Windows\x.dll").is_none());

        // Separador de Windows usado para esquivar la comprobación.
        assert!(safe_relative_path(r"..\..\evil.exe").is_none());
        assert!(safe_relative_path(r"dir\sub\a.txt").is_none());

        // Rutas UNC.
        assert!(safe_relative_path(r"\\servidor\recurso\x").is_none());

        // Vacías.
        assert!(safe_relative_path("").is_none());
        assert!(safe_relative_path(".").is_none());
    }

    #[test]
    fn el_hash_de_archivo_coincide_con_el_vector_conocido() {
        let dir = temp_dir("hash");
        let path = dir.join("abc.txt");
        std::fs::write(&path, b"abc").unwrap();

        assert_eq!(
            hash_file(&path).unwrap(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Comprueba de punta a punta que el manifiesto es correcto: que la URL
    /// sigue viva, que el archivo que llega tiene el hash fijado, que la ruta
    /// del ejecutable dentro del comprimido es la que dice y que el programa
    /// arranca.
    ///
    /// Necesita red y descarga 42 MB, así que no entra en la suite normal:
    /// `cargo test -- --ignored instala_pandoc_de_verdad`
    #[tokio::test]
    #[ignore = "descarga 42 MB de la red"]
    async fn instala_pandoc_de_verdad() {
        use crate::engines::manifest::{spec, EngineId};

        let dir = temp_dir("pandoc-real");
        let engine = spec(EngineId::Pandoc);

        let record = install(engine, &dir, |_| {})
            .await
            .expect("la instalación de Pandoc debe funcionar");

        assert_eq!(record.version, "3.11");
        assert!(!record.executable_sha256.is_empty());

        let executable = dir
            .join("contenido")
            .join(record.executable.replace('/', std::path::MAIN_SEPARATOR_STR));
        assert!(executable.is_file(), "falta {}", executable.display());

        // Y arranca.
        let output = std::process::Command::new(&executable)
            .arg("--version")
            .output()
            .expect("pandoc debe poder ejecutarse");
        let version = String::from_utf8_lossy(&output.stdout);
        assert!(version.contains("pandoc"), "salida inesperada: {version}");
        assert!(version.contains("3.11"), "versión inesperada: {version}");

        // No quedan restos de la descarga.
        assert!(!dir.join("descarga.parcial").exists());

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Lo mismo para FFmpeg. Va aparte porque su archivo comprimido anida el
    /// ejecutable en `bin/`, que es justo la clase de detalle que se escribe
    /// mal en el manifiesto y no se descubre hasta que un usuario lo intenta.
    ///
    /// `cargo test -- --ignored instala_ffmpeg_de_verdad`
    #[tokio::test]
    #[ignore = "descarga 192 MB de la red"]
    async fn instala_ffmpeg_de_verdad() {
        use crate::engines::manifest::{spec, EngineId};

        let dir = temp_dir("ffmpeg-real");
        let engine = spec(EngineId::Ffmpeg);

        let record = install(engine, &dir, |_| {})
            .await
            .expect("la instalación de FFmpeg debe funcionar");

        let executable = dir
            .join("contenido")
            .join(record.executable.replace('/', std::path::MAIN_SEPARATOR_STR));
        assert!(executable.is_file(), "falta {}", executable.display());

        let output = std::process::Command::new(&executable)
            .arg("-version")
            .output()
            .expect("ffmpeg debe poder ejecutarse");
        let version = String::from_utf8_lossy(&output.stdout);
        assert!(version.contains("ffmpeg version"), "salida: {version:.200}");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn instalar_rechaza_un_motor_que_no_se_descarga() {
        use crate::engines::manifest::{spec, EngineId};

        let dir = temp_dir("nodownload");
        let error = install(spec(EngineId::LibreOffice), &dir, |_| {})
            .await
            .unwrap_err()
            .to_string();

        assert!(error.contains("no se puede descargar"), "{error}");
        assert!(error.contains("instalador"), "{error}");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn instalar_rechaza_una_url_fuera_de_la_lista() {
        use crate::engines::manifest::{Download, EngineId, EngineSpec};

        let dir = temp_dir("badurl");
        let falso = EngineSpec {
            id: EngineId::Pandoc,
            name: "Falso",
            command: "falso",
            description: "",
            enables: "",
            download: Some(Download {
                version: "1.0",
                url: "https://evil.test/carga.zip",
                sha256: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                size_bytes: 10,
                executable_in_archive: "x.exe",
                license: "",
            }),
            no_download_reason: None,
            system_paths: &[],
        };

        let error = install(&falso, &dir, |_| {}).await.unwrap_err().to_string();
        assert!(error.contains("no está permitida"), "{error}");

        let _ = std::fs::remove_dir_all(&dir);
    }
}
