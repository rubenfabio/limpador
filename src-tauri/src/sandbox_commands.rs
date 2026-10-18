//! Commands of the fake machine (ADR-0012): in sandbox, `docker` and `wsl` answer from
//! files the e2e scenario writes, so a test never depends on (or touches) the real Docker.
//! `git` stays real: the fake machine's repositories are real repositories.

use limpador_core::adapters::SystemCommandRunner;
use limpador_core::ports::{CommandOutput, CommandRunner};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub struct SandboxCommands {
    folder: PathBuf,
}

impl SandboxCommands {
    pub fn new(sandbox_root: &Path) -> Self {
        Self {
            folder: sandbox_root.join("commands"),
        }
    }

    /// File holding the answer to `program args`: every character that is not a letter or
    /// digit becomes `_` (e.g. `docker version --format json` → `docker_version___format_json`).
    pub fn file_name(program: &str, args: &[&str]) -> String {
        std::iter::once(program)
            .chain(args.iter().copied())
            .collect::<Vec<_>>()
            .join(" ")
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
            .collect()
    }
}

impl CommandRunner for SandboxCommands {
    fn run(&self, program: &str, args: &[&str]) -> io::Result<CommandOutput> {
        if program == "git" {
            return SystemCommandRunner.run(program, args);
        }
        let base = self.folder.join(Self::file_name(program, args));
        // A missing answer behaves like a program that is not installed.
        let stdout = fs::read_to_string(base.with_extension("out"))?;
        let status = fs::read_to_string(base.with_extension("status"))
            .ok()
            .and_then(|s| s.trim().parse().ok())
            .unwrap_or(0);
        let stderr = fs::read_to_string(base.with_extension("err")).unwrap_or_default();
        Ok(CommandOutput {
            status,
            stdout,
            stderr,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn answers_come_from_the_fake_machine() {
        let dir = tempfile::tempdir().unwrap();
        let commands = SandboxCommands::new(dir.path());
        let base = dir
            .path()
            .join("commands")
            .join("docker_version___format_json");
        fs::create_dir_all(base.parent().unwrap()).unwrap();
        fs::write(base.with_extension("out"), "{\"Server\":null}").unwrap();
        fs::write(base.with_extension("status"), "1").unwrap();
        fs::write(base.with_extension("err"), "failed to connect").unwrap();

        let out = commands
            .run("docker", &["version", "--format", "json"])
            .unwrap();
        assert_eq!(
            out,
            CommandOutput {
                status: 1,
                stdout: "{\"Server\":null}".into(),
                stderr: "failed to connect".into()
            }
        );
    }

    #[test]
    fn unscripted_commands_look_not_installed() {
        let dir = tempfile::tempdir().unwrap();
        let error = SandboxCommands::new(dir.path())
            .run("wsl", &["-l", "-v"])
            .unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::NotFound);
    }

    #[test]
    fn answers_without_status_succeed() {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path().join("commands").join("wsl__l__v");
        fs::create_dir_all(base.parent().unwrap()).unwrap();
        fs::write(base.with_extension("out"), "lista").unwrap();
        let out = SandboxCommands::new(dir.path())
            .run("wsl", &["-l", "-v"])
            .unwrap();
        assert_eq!(
            (out.status, out.stdout.as_str(), out.stderr.as_str()),
            (0, "lista", "")
        );
    }

    #[test]
    fn git_stays_real() {
        let dir = tempfile::tempdir().unwrap();
        let out = SandboxCommands::new(dir.path())
            .run("git", &["--version"])
            .unwrap();
        assert!(out.stdout.starts_with("git version"));
    }
}
