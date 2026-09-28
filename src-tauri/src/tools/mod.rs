//! Implementación de las herramientas que se ejecutan en Rust.
//!
//! Aquí vive la lógica; `commands/` solo la expone al WebView. Cada módulo es
//! independiente y no sabe nada de Tauri, así que se puede probar sin levantar
//! una ventana.

pub mod cert;
pub mod exif;
pub mod hashing;
pub mod http;
pub mod ids;
pub mod images;
pub mod jwt;
pub mod net;
pub mod password;
pub mod qr;
pub mod text;
