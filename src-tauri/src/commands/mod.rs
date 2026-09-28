//! Comandos expuestos al WebView.
//!
//! Capa fina: valida, delega en el módulo correspondiente y traduce el error a
//! [`AppError`]. Nada de lógica de negocio aquí.

pub mod convert;
pub mod engines;
pub mod images;
pub mod net;
pub mod tools;

use serde::Serialize;

use crate::error::{AppError, AppResult};
use crate::portable::{self, StorageMode};
use crate::registry::{self, Tool};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageInfo {
    pub root: String,
    pub mode: StorageMode,
    pub degraded_reason: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub name: &'static str,
    pub version: &'static str,
    pub storage: StorageInfo,
}

/// Catálogo completo, incluidas las herramientas todavía no implementadas.
#[tauri::command]
pub fn list_tools() -> &'static [Tool] {
    registry::TOOLS
}

#[tauri::command]
pub fn get_tool(id: String) -> AppResult<&'static Tool> {
    registry::find(&id).ok_or_else(|| AppError::msg(format!("Herramienta desconocida: «{id}»")))
}

#[tauri::command]
pub fn app_info() -> AppInfo {
    let storage = portable::storage();
    AppInfo {
        name: "Fast tools",
        version: env!("CARGO_PKG_VERSION"),
        storage: StorageInfo {
            root: storage.root.display().to_string(),
            mode: storage.mode,
            degraded_reason: storage.degraded_reason.clone(),
        },
    }
}

/// Abre el directorio de datos en el explorador de archivos.
///
/// Se invoca con la ruta resuelta por [`portable`], nunca con una ruta que
/// venga de la UI: así el comando no puede usarse para abrir rutas arbitrarias.
#[tauri::command]
pub fn reveal_data_dir() -> AppResult<()> {
    let dir = portable::data_dir();
    std::fs::create_dir_all(dir)?;

    // Sin shell: los argumentos van como vector, no concatenados.
    std::process::Command::new("explorer.exe")
        .arg(dir)
        .spawn()
        .map_err(|e| AppError::msg(format!("No se pudo abrir el explorador: {e}")))?;
    Ok(())
}
