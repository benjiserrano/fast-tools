//! Error único que cruza la frontera IPC hacia la UI.
//!
//! Se serializa como string plano: la UI muestra el mensaje tal cual, así que
//! los mensajes se escriben pensando en el usuario, no en el desarrollador.

use serde::{Serialize, Serializer};

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Error de E/S: {0}")]
    Io(#[from] std::io::Error),

    #[error("{0}")]
    Convert(#[from] crate::convert::ConvertError),

    #[error("{0}")]
    Message(String),
}

impl AppError {
    pub fn msg(text: impl Into<String>) -> Self {
        Self::Message(text.into())
    }
}

impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

pub type AppResult<T> = Result<T, AppError>;
