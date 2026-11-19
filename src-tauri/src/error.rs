use serde::{Serialize, Serializer};

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("{0}")]
    Config(String),
    #[error("Keychain error: {0}")]
    Keychain(String),
    #[error("{0}")]
    Provider(String),
    #[error("Network error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("HTTP {status} for {url}: {message}")]
    HttpStatus {
        status: u16,
        url: String,
        message: String,
    },
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Tauri(#[from] tauri::Error),
}

pub type AppResult<T> = Result<T, AppError>;

impl AppError {
    /// The message plus the causes kept in `source()`. `reqwest` stores DNS, TLS and
    /// connection details there, which is exactly what a transient network failure needs.
    pub fn detail(&self) -> String {
        let mut message = self.to_string();
        let mut source = std::error::Error::source(self);
        while let Some(error) = source {
            let text = error.to_string();
            if !text.is_empty() && !message.contains(&text) {
                message.push_str(": ");
                message.push_str(&text);
            }
            source = error.source();
        }
        message
    }
}

/// Tauri commands need a serialisable error; the frontend shows the message with its causes.
impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.detail())
    }
}
