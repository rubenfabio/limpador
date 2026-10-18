//! Adapters: the real implementations of the ports (ADR-0023), named after the technology
//! they use. Test doubles live in `limpador-testkit` and next to the tests.

mod system;
mod toml_config;
#[cfg(not(windows))]
mod unix;
#[cfg(windows)]
mod windows;

pub use system::{GIT_LOCATION_VARS, SystemClock, SystemCommandRunner};
pub use toml_config::TomlConfigRepository;
#[cfg(not(windows))]
pub use unix::UnixPlatform;
#[cfg(windows)]
pub use windows::WindowsPlatform;

use crate::ports::Platform;
use std::sync::Arc;

/// The platform adapter of the system the app runs on.
pub fn native_platform() -> Arc<dyn Platform> {
    #[cfg(windows)]
    {
        Arc::new(WindowsPlatform)
    }
    #[cfg(not(windows))]
    {
        Arc::new(UnixPlatform)
    }
}
