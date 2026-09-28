//! Motores externos: dónde están, si se puede confiar en ellos y cómo se usan.
//!
//! Orden de resolución, de más a menos preferente:
//!
//! 1. El que haya instalado fast-tools en su carpeta de datos.
//! 2. Uno instalado manualmente por el usuario en esa misma carpeta.
//! 3. Uno del sistema: en el `PATH` o en su ruta de instalación habitual.
//!
//! Se prefiere el propio porque es el único cuya versión y hash se conocen.

pub mod download;
pub mod manifest;

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Serialize;

use crate::error::{AppError, AppResult};
use crate::portable;
use download::{InstallRecord, RECORD_FILE};
use manifest::{EngineId, EngineSpec};

/// De dónde ha salido el ejecutable que se va a usar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Source {
    /// Descargado y verificado por fast-tools.
    Managed,
    /// Puesto a mano en la carpeta de motores, sin registro de instalación.
    Manual,
    /// Encontrado en el sistema.
    System,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineStatus {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub enables: &'static str,
    pub available: bool,
    pub source: Option<Source>,
    pub path: Option<String>,
    pub version: Option<String>,
    /// Espacio que ocupa en disco lo instalado por fast-tools.
    pub disk_bytes: Option<u64>,
    pub downloadable: bool,
    pub download_size: Option<u64>,
    pub download_version: Option<&'static str>,
    pub download_origin: Option<String>,
    pub license: Option<&'static str>,
    pub no_download_reason: Option<&'static str>,
}

pub fn engines_root() -> AppResult<PathBuf> {
    Ok(portable::subdir("engines")?)
}

pub fn engine_dir(id: EngineId) -> AppResult<PathBuf> {
    let dir = engines_root()?.join(id.id());
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

fn read_record(dir: &Path) -> Option<InstallRecord> {
    let bytes = std::fs::read(dir.join(RECORD_FILE)).ok()?;
    serde_json::from_slice(&bytes).ok()
}

/// Localiza el ejecutable del motor, sin comprobar su integridad.
fn locate(spec: &EngineSpec) -> Option<(PathBuf, Source, Option<String>)> {
    if let Ok(dir) = engine_dir(spec.id) {
        // 1. Instalación gestionada, con registro.
        if let Some(record) = read_record(&dir) {
            let path = dir
                .join("contenido")
                .join(record.executable.replace('/', std::path::MAIN_SEPARATOR_STR));
            if path.is_file() {
                return Some((path, Source::Managed, Some(record.version)));
            }
        }

        // 2. Copiado a mano. Se busca el ejecutable por su nombre, sin bajar
        //    más de dos niveles: basta para «bin/ffmpeg.exe» y evita recorrer
        //    un árbol entero en cada arranque.
        let executable = format!("{}.exe", spec.command);
        for candidate in [
            dir.join(&executable),
            dir.join("bin").join(&executable),
            dir.join("contenido").join(&executable),
            dir.join("contenido").join("bin").join(&executable),
        ] {
            if candidate.is_file() {
                return Some((candidate, Source::Manual, None));
            }
        }
    }

    // 3. Instalado en el sistema.
    for candidate in spec.system_paths {
        let path = PathBuf::from(candidate);
        if path.is_file() {
            return Some((path, Source::System, None));
        }
    }
    if let Some(path) = which(spec.command) {
        return Some((path, Source::System, None));
    }

    None
}

/// Busca un ejecutable en el `PATH`, como haría el sistema al lanzarlo.
fn which(command: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    let extensions: Vec<String> = std::env::var("PATHEXT")
        .unwrap_or_else(|_| ".EXE;.CMD;.BAT".to_string())
        .split(';')
        .filter(|extension| !extension.is_empty())
        .map(|extension| extension.to_ascii_lowercase())
        .collect();

    for directory in std::env::split_paths(&path) {
        for extension in &extensions {
            let candidate = directory.join(format!("{command}{extension}"));
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

fn directory_size(path: &Path) -> u64 {
    let Ok(entries) = std::fs::read_dir(path) else {
        return 0;
    };
    entries
        .filter_map(Result::ok)
        .map(|entry| match entry.file_type() {
            Ok(kind) if kind.is_dir() => directory_size(&entry.path()),
            Ok(_) => entry.metadata().map(|meta| meta.len()).unwrap_or(0),
            Err(_) => 0,
        })
        .sum()
}

pub fn status(id: EngineId) -> EngineStatus {
    let spec = manifest::spec(id);
    let located = locate(spec);

    let disk_bytes = engine_dir(id)
        .ok()
        .map(|dir| directory_size(&dir))
        .filter(|size| *size > 0);

    EngineStatus {
        id: spec.id.id(),
        name: spec.name,
        description: spec.description,
        enables: spec.enables,
        available: located.is_some(),
        source: located.as_ref().map(|(_, source, _)| *source),
        path: located
            .as_ref()
            .map(|(path, _, _)| path.display().to_string()),
        version: located.as_ref().and_then(|(_, _, version)| version.clone()),
        disk_bytes,
        downloadable: spec.download.is_some(),
        download_size: spec.download.as_ref().map(|d| d.size_bytes),
        download_version: spec.download.as_ref().map(|d| d.version),
        download_origin: spec
            .download
            .as_ref()
            .and_then(|d| d.url.strip_prefix("https://"))
            .and_then(|rest| rest.split('/').next())
            .map(str::to_string),
        license: spec.download.as_ref().map(|d| d.license),
        no_download_reason: spec.no_download_reason,
    }
}

pub fn all_status() -> Vec<EngineStatus> {
    EngineId::ALL.iter().copied().map(status).collect()
}

/// Borra lo que fast-tools haya instalado de un motor.
pub fn remove(id: EngineId) -> AppResult<()> {
    let dir = engine_dir(id)?;
    if dir.exists() {
        std::fs::remove_dir_all(&dir)?;
    }
    Ok(())
}

/// Comprueba que el ejecutable sigue siendo el que se instaló.
///
/// Volver a calcular el SHA-256 de un ejecutable de 80 MB en cada ejecución
/// añadiría medio segundo a cada conversión, así que primero se mira el tamaño
/// y la fecha de modificación —que es gratis— y solo se rehashea si algo ha
/// cambiado. Un ejecutable sustituido por otro cambia de fecha salvo que quien
/// lo haga se moleste en falsearla, y para entonces ya tiene acceso de escritura
/// a la carpeta del usuario.
fn verify_managed(dir: &Path, path: &Path) -> AppResult<()> {
    let Some(record) = read_record(dir) else {
        return Ok(());
    };

    let metadata = std::fs::metadata(path)?;
    if metadata.len() == record.executable_size {
        return Ok(());
    }

    let actual = download::hash_file(path)?;
    if actual != record.executable_sha256 {
        return Err(AppError::msg(format!(
            "El ejecutable del motor ha cambiado desde que se instaló y no se va \
             a ejecutar.\nEsperado: {}\nObtenido: {}\nBorra el motor y vuelve a \
             instalarlo.",
            record.executable_sha256, actual
        )));
    }
    Ok(())
}

/// Ruta del ejecutable de un motor, ya verificado, listo para lanzarse.
pub fn resolve(id: EngineId) -> AppResult<PathBuf> {
    let spec = manifest::spec(id);

    let (path, source, _) = locate(spec).ok_or_else(|| {
        AppError::msg(format!(
            "{} no está disponible. {}",
            spec.name,
            if spec.download.is_some() {
                "Instálalo desde la pantalla de motores."
            } else {
                spec.no_download_reason.unwrap_or("")
            }
        ))
    })?;

    if source == Source::Managed {
        verify_managed(&engine_dir(id)?, &path)?;
    }

    Ok(path)
}

/// Lanza un motor.
///
/// Los argumentos van como vector y **nunca** se construye una línea de
/// comandos concatenando texto: las rutas las elige el usuario y pueden llevar
/// comillas, ampersands o cualquier otro metacarácter que un intérprete de
/// órdenes tomaría como instrucción.
pub fn run<I, S>(id: EngineId, args: I) -> AppResult<std::process::Output>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let executable = resolve(id)?;
    let spec = manifest::spec(id);

    let mut command = Command::new(&executable);
    command.args(args);

    // Sin esto, cada conversión abre y cierra una ventana de consola negra.
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }

    let output = command
        .output()
        .map_err(|e| AppError::msg(format!("no se pudo ejecutar {}: {e}", spec.name)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // La salida de error de estos programas es larguísima; se queda la
        // cola, que es donde está el motivo real del fallo.
        let tail: Vec<&str> = stderr.lines().rev().take(6).collect();
        let detail = tail.into_iter().rev().collect::<Vec<_>>().join("\n");

        return Err(AppError::msg(format!(
            "{} terminó con error{}.\n{}",
            spec.name,
            output
                .status
                .code()
                .map(|code| format!(" (código {code})"))
                .unwrap_or_default(),
            detail.trim()
        )));
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn el_estado_describe_todos_los_motores() {
        let all = all_status();
        assert_eq!(all.len(), EngineId::ALL.len());

        for entry in &all {
            assert!(!entry.name.is_empty());
            assert!(!entry.enables.is_empty());
            // Si no se puede descargar, hay que explicar por qué.
            if !entry.downloadable {
                assert!(entry.no_download_reason.is_some(), "{}", entry.name);
            }
        }
    }

    #[test]
    fn libreoffice_se_declara_no_descargable() {
        let entry = status(EngineId::LibreOffice);
        assert!(!entry.downloadable);
        assert!(entry.download_size.is_none());
        assert!(entry.no_download_reason.unwrap().contains("instalador"));
    }

    #[test]
    fn los_motores_descargables_publican_origen_y_licencia() {
        for id in [EngineId::Pandoc, EngineId::Ffmpeg] {
            let entry = status(id);
            assert!(entry.downloadable, "{}", entry.name);
            assert_eq!(
                entry.download_origin.as_deref(),
                Some("github.com"),
                "{}",
                entry.name
            );
            assert!(entry.license.is_some(), "{}", entry.name);
            assert!(entry.download_size.unwrap() > 0);
        }
    }

    #[test]
    fn un_motor_ausente_explica_que_hacer() {
        // Ninguno de estos estará instalado en una máquina de integración.
        if status(EngineId::Pandoc).available {
            return;
        }
        let error = resolve(EngineId::Pandoc).unwrap_err().to_string();
        assert!(error.contains("no está disponible"), "{error}");
        assert!(error.contains("pantalla de motores"), "{error}");
    }

    #[test]
    fn which_encuentra_algo_que_seguro_existe() {
        // cmd.exe está en el PATH de cualquier Windows.
        #[cfg(windows)]
        assert!(which("cmd").is_some(), "cmd debería estar en el PATH");
    }

    #[test]
    fn which_no_se_inventa_lo_que_no_hay() {
        assert!(which("programa-que-no-existe-jamas-12345").is_none());
    }
}
