use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Global configuration for a traz instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrazConfig {
    /// Path to the SQLite database file.
    pub db_path: PathBuf,
    /// Default API port.
    pub api_port: u16,
    /// Whether semantic search embeddings are enabled.
    #[serde(default)]
    pub embeddings_enabled: bool,
    /// Path to a custom embedding model if provided.
    #[serde(default)]
    pub embeddings_model_path: Option<PathBuf>,
}

impl TrazConfig {
    /// Resolve configuration from environment, local file, and defaults.
    ///
    /// Priority:
    /// 1. `$TRAZ_DB` environment variable
    /// 2. Project-local `.traz/traz.db` (searches parent directories)
    /// 3. Global `~/.local/share/traz/traz.db` (XDG data dir)
    pub fn resolve() -> Self {
        // 1. Check for environment override
        if let Ok(custom) = std::env::var("TRAZ_DB") {
            return Self::load_or_default(PathBuf::from(custom));
        }

        // 2. Search upwards for .traz/traz.db (Project Local)
        if let Ok(cwd) = std::env::current_dir() {
            let mut current = cwd.as_path();
            loop {
                let local_traz = current.join(".traz");
                if local_traz.is_dir() {
                    return Self::load_or_default(local_traz.join("traz.db"));
                }
                match current.parent() {
                    Some(parent) => current = parent,
                    None => break,
                }
            }
        }

        // 3. Fallback to Global
        let mut path = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push("traz");
        path.push("traz.db");
        Self::load_or_default(path)
    }

    fn load_or_default(db_path: PathBuf) -> Self {
        let config_path = db_path
            .parent()
            .unwrap_or(Path::new("."))
            .join("config.toml");

        let mut config = if config_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&config_path) {
                if let Ok(c) = toml::from_str::<TrazConfig>(&content) {
                    c
                } else {
                    Self::default_with_paths(db_path.clone())
                }
            } else {
                Self::default_with_paths(db_path.clone())
            }
        } else {
            Self::default_with_paths(db_path.clone())
        };

        // Final enforcement of the resolved db_path
        config.db_path = db_path;

        // Environment port override
        if let Ok(port_str) = std::env::var("TRAZ_PORT")
            && let Ok(p) = port_str.parse()
        {
            config.api_port = p;
        }

        config
    }

    fn default_with_paths(db_path: PathBuf) -> Self {
        Self {
            db_path,
            api_port: 4000,
            embeddings_enabled: false,
            embeddings_model_path: None,
        }
    }

    /// Persist the current configuration to config.toml in the database directory.
    pub fn save(&self) -> anyhow::Result<()> {
        let parent = self.db_path.parent().unwrap_or(Path::new("."));
        let config_path = parent.join("config.toml");
        std::fs::create_dir_all(parent)?;

        let content = toml::to_string_pretty(self)?;
        let temp_path = parent.join(format!(".config.toml.tmp.{}", std::process::id()));

        std::fs::write(&temp_path, content)?;

        // Ensure data is synced to physical storage before rename
        if let Ok(file) = std::fs::File::open(&temp_path) {
            let _ = file.sync_all();
        }

        // Atomic rename
        std::fs::rename(&temp_path, &config_path)?;
        Ok(())
    }

    /// Return the directory containing the database (for init, etc.)
    pub fn data_dir(&self) -> &Path {
        self.db_path.parent().unwrap_or(Path::new("."))
    }
}
