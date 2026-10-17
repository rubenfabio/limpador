use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

/// A real git repository driven through the `git` CLI, isolated from the user's
/// global and system configuration (no hooks, no signing, no templates).
#[derive(Debug, Clone)]
pub struct GitRepo {
    path: PathBuf,
    config_home: PathBuf,
}

impl GitRepo {
    /// Creates a repository whose default branch is `main`.
    pub fn init(path: &Path, config_home: &Path) -> io::Result<Self> {
        fs::create_dir_all(path)?;
        let repo = Self {
            path: path.to_owned(),
            config_home: config_home.to_owned(),
        };
        repo.git(&["init", "--quiet", "--initial-branch=main"])?;
        Ok(repo)
    }

    /// Handle to an existing repository.
    pub fn open(path: &Path, config_home: &Path) -> Self {
        Self {
            path: path.to_owned(),
            config_home: config_home.to_owned(),
        }
    }

    /// Creates a bare repository, used as a local stand-in for a remote such as GitHub.
    pub fn init_bare(path: &Path, config_home: &Path) -> io::Result<Self> {
        fs::create_dir_all(path)?;
        let repo = Self {
            path: path.to_owned(),
            config_home: config_home.to_owned(),
        };
        repo.git(&["init", "--quiet", "--bare", "--initial-branch=main"])?;
        Ok(repo)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Writes `file` and commits it with author and committer dates set to `at`.
    pub fn commit_file(
        &self,
        file: &str,
        contents: &str,
        message: &str,
        at: SystemTime,
    ) -> io::Result<()> {
        let target = self.path.join(file);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(target, contents)?;
        self.git(&["add", "--", file])?;
        self.git_at(&["commit", "--quiet", "-m", message], at)?;
        Ok(())
    }

    /// Commits whatever is staged, dated `at`.
    pub fn commit_staged(&self, message: &str, at: SystemTime) -> io::Result<()> {
        self.git_at(&["commit", "--quiet", "-m", message], at)?;
        Ok(())
    }

    /// Runs git in this repository and returns trimmed stdout; fails on non-zero exit.
    pub fn git(&self, args: &[&str]) -> io::Result<String> {
        self.git_at(args, SystemTime::now())
    }

    fn git_at(&self, args: &[&str], at: SystemTime) -> io::Result<String> {
        let seconds = at.duration_since(UNIX_EPOCH).map_err(io::Error::other)?;
        let date = format!("@{} +0000", seconds.as_secs());
        // Identity, hooks and signing come from the isolated config written by FakeMachine.
        let mut command = Command::new("git");
        // Git exports repository locations to hooks; tests run from hooks must not use them.
        for var in limpador_core::ports::GIT_LOCATION_VARS {
            command.env_remove(var);
        }
        let output = command
            .args(args)
            .current_dir(&self.path)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", self.config_home.join("gitconfig"))
            .env("GIT_AUTHOR_DATE", &date)
            .env("GIT_COMMITTER_DATE", &date)
            .env("GIT_TERMINAL_PROMPT", "0")
            .output()?;
        if !output.status.success() {
            return Err(io::Error::other(format!(
                "git {} failed: {}",
                args.join(" "),
                String::from_utf8_lossy(&output.stderr)
            )));
        }
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
    }
}

/// Repository where `branch` was integrated into `main` by a squash merge, pushed,
/// and then deleted on the remote — the common GitHub PR flow.
#[derive(Debug, Clone)]
pub struct SquashMergedBranch {
    pub repo: GitRepo,
    pub remote: GitRepo,
    pub branch: String,
}
