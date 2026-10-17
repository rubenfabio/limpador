//! Boundaries between the core and the outside world (ADR-0004).
//!
//! Collectors never touch the clock or spawn processes directly: they receive these
//! traits, so tests can swap in the fakes from `limpador-testkit`.

use std::io;
use std::process::Command;
use std::time::SystemTime;

/// Source of the current time.
pub trait Clock: Send + Sync {
    fn now(&self) -> SystemTime;
}

/// The real system clock.
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> SystemTime {
        SystemTime::now()
    }
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

/// Runs real processes.
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemCommandRunner;

impl CommandRunner for SystemCommandRunner {
    fn run(&self, program: &str, args: &[&str]) -> io::Result<CommandOutput> {
        let output = Command::new(program).args(args).output()?;
        Ok(CommandOutput {
            // A missing code means the process was killed by a signal.
            status: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn system_clock_follows_real_time() {
        let before = SystemTime::now();
        let now = SystemClock.now();
        assert!(now >= before);
        assert!(now.duration_since(before).unwrap() < Duration::from_secs(5));
    }

    #[test]
    fn system_runner_captures_status_and_output() {
        let out = SystemCommandRunner.run("git", &["--version"]).unwrap();
        assert!(out.success());
        assert!(out.stdout.starts_with("git version"));
    }

    #[test]
    fn system_runner_reports_failure_status() {
        let out = SystemCommandRunner
            .run("git", &["comando-que-nao-existe"])
            .unwrap();
        assert!(!out.success());
        assert_ne!(out.status, 0);
        assert!(!out.stderr.is_empty());
    }

    #[test]
    fn system_runner_errors_when_program_is_missing() {
        assert!(
            SystemCommandRunner
                .run("programa-inexistente-xyz", &[])
                .is_err()
        );
    }
}
