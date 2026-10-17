//! Sandbox mode for the real-app e2e tests (ADR-0012).
//!
//! Only builds compiled with the `sandbox` feature accept a sandbox. A production
//! build that receives one refuses to start, so the catalog and data folders can
//! never be redirected on a user's machine.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

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

/// Message explaining why the app must not start, if it must not.
pub fn refusal_message(decision: &SandboxDecision) -> Option<String> {
    (*decision == SandboxDecision::Refused).then(|| {
        format!(
            "{SANDBOX_ENV} só é aceito em builds de teste (feature `sandbox`). Recusando iniciar."
        )
    })
}

/// Where the webview keeps its profile (cache, storage) inside the fake machine, so a
/// sandboxed run never writes to the user's real app-data folders.
pub fn webview_data_dir(root: &Path) -> PathBuf {
    root.join("app-data").join("webview")
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
    fn t00_02_ca5_refused_sandbox_stops_startup_with_explanation() {
        let message = refusal_message(&SandboxDecision::Refused).unwrap();
        assert!(message.contains(SANDBOX_ENV));
        assert!(message.contains("builds de teste"));
    }

    #[test]
    fn allowed_decisions_do_not_stop_startup() {
        assert_eq!(refusal_message(&SandboxDecision::Disabled), None);
        assert_eq!(
            refusal_message(&SandboxDecision::Enabled("C:\\maquina-falsa".into())),
            None
        );
    }

    #[test]
    fn t00_04_ca4_webview_data_stays_inside_the_fake_machine() {
        let root = Path::new("C:\\maquina-falsa");
        let dir = webview_data_dir(root);
        assert!(dir.starts_with(root));
        assert_ne!(dir, root);
    }

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
