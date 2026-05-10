use thiserror::Error;

/// Domain-specific errors for the traz system.
#[derive(Debug, Error)]
pub enum TrazError {
    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Event not found: {0}")]
    NotFound(String),

    #[error("Database error: {0}")]
    Database(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Integration error: {0}")]
    Integration(String),
}

impl TrazError {
    pub fn validation(msg: impl Into<String>) -> Self {
        Self::Validation(msg.into())
    }
}
