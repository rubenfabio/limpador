//! Ports: the boundaries between the core and the outside world (ADR-0004, ADR-0023).
//!
//! The core only depends on these traits. Their implementations live in
//! [`crate::adapters`] (real ones, named after their technology) and in
//! `limpador-testkit` (test doubles).

mod config;
mod platform;

pub use config::ConfigRepository;
pub use platform::{FileIdentity, Platform};

use std::io;
use std::time::SystemTime;

/// Source of the current time.
pub trait Clock: Send + Sync {
    fn now(&self) -> SystemTime;
}

/// Result of running an external command such as `git` or `docker`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandOutput {
    pub status: i32,
    pub stdout: String,
    pub stderr: String,
}

impl CommandOutput {
    pub fn success(&self) -> bool {
        self.status == 0
    }
}

/// Runs external programs. Implementations must not use a shell.
pub trait CommandRunner: Send + Sync {
    fn run(&self, program: &str, args: &[&str]) -> io::Result<CommandOutput>;
}
