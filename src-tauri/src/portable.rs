//! Resolución de rutas de datos.
//!
//! Este es el **único** punto del programa que decide dónde se escribe en disco.
//! El resto del código llama a [`data_dir`] o [`subdir`] y nunca construye rutas
//! de almacenamiento por su cuenta.
//!
//! Orden de preferencia:
//!   1. `<dir_del_exe>/fast-tools-data` → modo portable (USB, carpeta suelta)
//!   2. `%LOCALAPPDATA%/fast-tools`     → si (1) no admite escritura
//!   3. `%TEMP%/fast-tools`             → último recurso, se pierde al limpiar temporales
//!
//! Nunca se escribe en el registro de Windows.

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use serde::Serialize;

/// Nombre de la carpeta de datos que se crea junto al ejecutable.
pub const DATA_DIR_NAME: &str = "fast-tools-data";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum StorageMode {
    /// Datos junto al ejecutable. Modo deseado: la app es realmente portable.
    Portable,
    /// El directorio del ejecutable es de solo lectura (USB protegido, Program Files).
    LocalAppData,
    /// Ni el directorio del ejecutable ni LOCALAPPDATA admiten escritura.
    Temporary,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Storage {
    /// Raíz del área de datos, siempre absoluta.
    pub root: PathBuf,
    pub mode: StorageMode,
    /// Por qué no se pudo usar el modo portable. La UI lo muestra como aviso.
    pub degraded_reason: Option<String>,
}

static STORAGE: OnceLock<Storage> = OnceLock::new();

/// Resuelve el almacenamiento una sola vez por proceso.
pub fn storage() -> &'static Storage {
    STORAGE.get_or_init(resolve)
}

pub fn data_dir() -> &'static Path {
    &storage().root
}

/// Crea (si hace falta) y devuelve un subdirectorio del área de datos.
// Sin usar hasta la Fase 1, que guarda aquí el historial de conversiones.
#[allow(dead_code)]
pub fn subdir(name: &str) -> io::Result<PathBuf> {
    let path = data_dir().join(name);
    fs::create_dir_all(&path)?;
    Ok(path)
}

fn resolve() -> Storage {
    // Camino feliz: se sale por aquí y no hay motivo de degradación que contar.
    // Si no, la razón del fallo se arrastra a los modos de reserva para que la
    // barra de estado pueda explicar por qué la app no es portable.
    let reason = match exe_dir() {
        Ok(dir) => {
            let candidate = dir.join(DATA_DIR_NAME);
            match probe(&candidate) {
                Ok(()) => {
                    return Storage {
                        root: candidate,
                        mode: StorageMode::Portable,
                        degraded_reason: None,
                    }
                }
                Err(e) => format!("«{}» no admite escritura: {e}", candidate.display()),
            }
        }
        Err(e) => format!("No se pudo localizar el ejecutable: {e}"),
    };
    let degraded_reason = Some(reason);

    if let Some(local) = local_app_data() {
        let candidate = local.join("fast-tools");
        if probe(&candidate).is_ok() {
            return Storage {
                root: candidate,
                mode: StorageMode::LocalAppData,
                degraded_reason,
            };
        }
    }

    // Si el temporal también falla, el resto del código recibirá errores de E/S
    // al usar la ruta. Eso es preferible a abortar el arranque: la mayoría de
    // herramientas funcionan solo en memoria y siguen siendo útiles.
    let candidate = std::env::temp_dir().join("fast-tools");
    let _ = probe(&candidate);
    Storage {
        root: candidate,
        mode: StorageMode::Temporary,
        degraded_reason,
    }
}

fn exe_dir() -> io::Result<PathBuf> {
    let exe = std::env::current_exe()?;
    exe.parent().map(Path::to_path_buf).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "el ejecutable no tiene directorio padre",
        )
    })
}

fn local_app_data() -> Option<PathBuf> {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .filter(|p| !p.as_os_str().is_empty())
}

/// Comprueba que se puede crear el directorio **y escribir dentro**.
///
/// Crear el directorio no basta: una unidad puede montarse de solo lectura
/// después de que la carpeta ya exista. El centinela lleva el PID para no
/// chocar con otra instancia corriendo en paralelo desde el mismo USB.
fn probe(dir: &Path) -> io::Result<()> {
    fs::create_dir_all(dir)?;
    let sentinel = dir.join(format!(".write-probe-{}", std::process::id()));
    {
        let mut file = fs::File::create(&sentinel)?;
        file.write_all(b"ok")?;
        file.sync_all()?;
    }
    fs::remove_file(&sentinel)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe_escribe_y_no_deja_rastro() {
        let dir = std::env::temp_dir().join(format!("ft-probe-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);

        probe(&dir).expect("el temporal del sistema debería admitir escritura");

        let restos: Vec<_> = fs::read_dir(&dir)
            .expect("el directorio debe existir tras el probe")
            .filter_map(Result::ok)
            .map(|e| e.file_name())
            .collect();
        assert!(restos.is_empty(), "probe dejó ficheros: {restos:?}");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn probe_falla_si_la_ruta_es_un_fichero() {
        let file = std::env::temp_dir().join(format!("ft-probe-file-{}", std::process::id()));
        fs::write(&file, b"no soy un directorio").unwrap();

        assert!(probe(&file).is_err(), "un fichero no puede ser data_dir");

        let _ = fs::remove_file(&file);
    }

    #[test]
    fn storage_es_absoluto_y_utilizable() {
        let s = storage();
        assert!(s.root.is_absolute(), "root debe ser absoluta: {:?}", s.root);

        let sub = subdir("ft-test-subdir").expect("subdir debe poder crearse");
        assert!(sub.is_dir());
        let _ = fs::remove_dir_all(sub);
    }

    #[test]
    fn storage_es_estable_entre_llamadas() {
        assert_eq!(storage().root, storage().root);
        assert_eq!(data_dir(), storage().root.as_path());
    }
}
