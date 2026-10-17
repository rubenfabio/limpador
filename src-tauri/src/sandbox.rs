//! Sandbox mode for the real-app e2e tests (ADR-0012).
//!
//! Only builds compiled with the `sandbox` feature accept a sandbox. A production
//! build that receives one refuses to start, so the catalog and data folders can
//! never be redirected on a user's machine.

use std::ffi::OsString;
use std::path::PathBuf;

/// Environment variable that points the app at a fake machine.
pub const SANDBOX_ENV: &str = "LIMPADOR_SANDBOX";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SandboxDecision {
    /// No sandbox requested: normal operation.
    Disabled,
    /// Test build with a sandbox: use this fake machine root.
    Enabled(PathBuf),
    /// Production build asked to use a sandbox: must not start.
    Refused,
}

pub fn decide(requested: Option<OsString>) -> SandboxDecision {
    decide_for_build(requested, cfg!(feature = "sandbox"))
}

fn decide_for_build(requested: Option<OsString>, sandbox_build: bool) -> SandboxDecision {
    match requested {
        None => SandboxDecision::Disabled,
        Some(root) if sandbox_build && !root.is_empty() => SandboxDecision::Enabled(root.into()),
        Some(_) => SandboxDecision::Refused,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn t00_02_ca5_production_build_refuses_sandbox() {
        assert_eq!(
            decide_for_build(Some("C:\\maquina-falsa".into()), false),
            SandboxDecision::Refused
        );
    }

    #[test]
    fn production_build_without_sandbox_runs_normally() {
        assert_eq!(decide_for_build(None, false), SandboxDecision::Disabled);
    }

    #[test]
    fn test_build_accepts_sandbox_root() {
        assert_eq!(
            decide_for_build(Some("C:\\maquina-falsa".into()), true),
            SandboxDecision::Enabled(PathBuf::from("C:\\maquina-falsa"))
        );
    }

    #[test]
    fn test_build_refuses_empty_sandbox_root() {
        assert_eq!(
            decide_for_build(Some(OsString::new()), true),
            SandboxDecision::Refused
        );
    }

    #[test]
    fn test_build_without_sandbox_runs_normally() {
        assert_eq!(decide_for_build(None, true), SandboxDecision::Disabled);
    }

    #[cfg(not(feature = "sandbox"))]
    #[test]
    fn t00_02_ca5_this_build_is_a_production_build() {
        assert_eq!(decide(Some("x".into())), SandboxDecision::Refused);
    }
}
