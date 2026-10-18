use crate::config::{Config, ConfigError};
use crate::message::Message;

/// Where the user's configuration is kept (ADR-0018).
pub trait ConfigRepository: Send + Sync {
    /// Loads the configuration. A missing store means defaults; unreadable data or
    /// out-of-range values fall back to defaults, with a warning for each problem.
    fn load(&self) -> (Config, Vec<Message>);

    /// Saves atomically. Out-of-range values are refused.
    fn save(&self, config: &Config) -> Result<(), ConfigError>;
}
