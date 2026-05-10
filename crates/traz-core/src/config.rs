use std::path::{Path, PathBuf};

/// Global configuration for a traz instance.
#[derive(Debug, Clone)]
pub struct TrazConfig {
    /// Path to the SQLite database file.
    pub db_path: PathBuf,
    /// Default API port.
    pub api_port: u16,
}

impl TrazConfig {
    /// Resolve configuration from environment and defaults.
    ///
    /// Priority:
    /// 1. `$TRAZ_DB` environment variable
    /// 2. `~/.local/share/traz/traz.db` (XDG data dir)
    pub fn resolve() -> Self {
        let db_path = if let Ok(custom) = std::env::var("TRAZ_DB") {
            PathBuf::from(custom)
        } else {
            let mut path = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
            path.push("traz");
            path.push("traz.db");
            path
        };

        let api_port = std::env::var("TRAZ_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(4000);

        Self { db_path, api_port }
    }

    /// Return the directory containing the database (for init, etc.)
    pub fn data_dir(&self) -> &Path {
        self.db_path.parent().unwrap_or(Path::new("."))
    }
}
