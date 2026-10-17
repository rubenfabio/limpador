//! Core of Limpador: domain model, analysis engine, collectors, quarantine and history.
//!
//! This crate must not depend on Tauri or on the UI (ADR-0003): the desktop app,
//! the CLI and the MCP server are thin shells around it.

pub mod ports;

/// Version of the core library.
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn t00_01_ca1_core_exposes_semver_version() {
        let parts: Vec<&str> = version().split('.').collect();
        assert_eq!(parts.len(), 3, "version must be MAJOR.MINOR.PATCH");
        assert!(parts.iter().all(|p| p.parse::<u32>().is_ok()));
    }
}
