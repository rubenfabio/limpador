//! Configuration kept as TOML in the app's configuration folder (ADR-0018).

use crate::config::{Config, ConfigError};
use crate::message::Message;
use crate::ports::ConfigRepository;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Reads and writes `config.toml` in the app's configuration folder.
#[derive(Debug, Clone)]
pub struct TomlConfigRepository {
    path: PathBuf,
}

impl TomlConfigRepository {
    pub fn new(folder: &Path) -> Self {
        Self {
            path: folder.join("config.toml"),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl ConfigRepository for TomlConfigRepository {
    /// Loads the configuration. A missing file means defaults; an unreadable file or
    /// out-of-range values fall back to defaults, with a warning for each problem.
    fn load(&self) -> (Config, Vec<Message>) {
        let text = match fs::read_to_string(&self.path) {
            Ok(text) => text,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return (Config::default(), vec![]),
            Err(e) => {
                return (
                    Config::default(),
                    vec![Message::new("config.unreadable").with("error", e)],
                );
            }
        };
        let mut config: Config = match toml::from_str(&text) {
            Ok(config) => config,
            Err(e) => {
                return (
                    Config::default(),
                    vec![Message::new("config.invalid_file").with("error", e.message())],
                );
            }
        };
        let warnings = config
            .preferences
            .sanitize()
            .into_iter()
            .map(|field| Message::new("config.value_reset").with("field", field))
            .collect();
        (config, warnings)
    }

    /// Saves atomically (temporary file + rename). Out-of-range values are refused.
    fn save(&self, config: &Config) -> Result<(), ConfigError> {
        let problems = config.preferences.validate();
        if !problems.is_empty() {
            return Err(ConfigError::Invalid(problems));
        }
        let text =
            toml::to_string_pretty(config).map_err(|e| ConfigError::Io(io::Error::other(e)))?;
        if let Some(folder) = self.path.parent() {
            fs::create_dir_all(folder).map_err(ConfigError::Io)?;
        }
        let temporary = self.path.with_extension("toml.tmp");
        fs::write(&temporary, text).map_err(ConfigError::Io)?;
        fs::rename(&temporary, &self.path).map_err(ConfigError::Io)
    }
}
