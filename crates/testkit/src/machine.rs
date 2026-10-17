use crate::clock::FixedClock;
use crate::git::{GitRepo, SquashMergedBranch};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use tempfile::TempDir;

/// Git settings for every repository of the fake machine: fixed identity, no hooks,
/// no signing, no templates, no line-ending conversion.
const GIT_CONFIG: &str = "\
[user]
\tname = Limpador Test
\temail = test@limpador.invalid
[init]
\tdefaultBranch = main
\ttemplateDir =
[core]
\tautocrlf = false
\thooksPath = no-hooks-in-fake-machine
[commit]
\tgpgsign = false
[tag]
\tgpgsign = false
";

/// Ecosystem of a generated project.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectKind {
    Node,
    Python,
    Rust,
}

impl ProjectKind {
    fn files(self, name: &str) -> Vec<(&'static str, String)> {
        match self {
            Self::Node => vec![
                ("package.json", format!("{{\n  \"name\": \"{name}\"\n}}\n")),
                ("src/index.js", "console.log('ok');\n".to_owned()),
            ],
            Self::Python => vec![
                ("pyproject.toml", format!("[project]\nname = \"{name}\"\n")),
                ("main.py", "print('ok')\n".to_owned()),
            ],
            Self::Rust => vec![
                (
                    "Cargo.toml",
                    "[package]\nname = \"exemplo\"\nversion = \"0.1.0\"\n".to_owned(),
                ),
                ("src/main.rs", "fn main() {}\n".to_owned()),
            ],
        }
    }
}

/// Declarative description of a project, e.g. "Node project idle for 45 days, with git".
#[derive(Debug, Clone)]
pub struct ProjectSpec {
    name: String,
    kind: ProjectKind,
    inactive_days: u64,
    git: bool,
}

impl ProjectSpec {
    pub fn new(name: &str, kind: ProjectKind) -> Self {
        Self {
            name: name.to_owned(),
            kind,
            inactive_days: 0,
            git: false,
        }
    }

    pub fn node(name: &str) -> Self {
        Self::new(name, ProjectKind::Node)
    }

    pub fn python(name: &str) -> Self {
        Self::new(name, ProjectKind::Python)
    }

    pub fn rust(name: &str) -> Self {
        Self::new(name, ProjectKind::Rust)
    }

    /// Last commit and every file modification happen `days` before the clock's now.
    pub fn inactive_days(mut self, days: u64) -> Self {
        self.inactive_days = days;
        self
    }

    pub fn with_git(mut self) -> Self {
        self.git = true;
        self
    }
}

/// A disposable machine in a temporary directory. Dropping it deletes everything,
/// even when the test panics; links inside it are removed, never their targets.
#[derive(Debug)]
pub struct FakeMachine {
    dir: TempDir,
    clock: FixedClock,
}

impl FakeMachine {
    pub fn new() -> io::Result<Self> {
        let dir = tempfile::Builder::new()
            .prefix("limpador-maquina-falsa-")
            .tempdir()?;
        let machine = Self {
            dir,
            clock: FixedClock::reference(),
        };
        fs::create_dir_all(machine.roots())?;
        fs::create_dir_all(machine.git_config_home())?;
        fs::write(machine.git_config_home().join("gitconfig"), GIT_CONFIG)?;
        Ok(machine)
    }

    pub fn root(&self) -> &Path {
        self.dir.path()
    }

    pub fn clock(&self) -> &FixedClock {
        &self.clock
    }

    /// Folder that plays the role of the user's project roots (e.g. `C:\Projetos`).
    pub fn roots(&self) -> PathBuf {
        self.path("roots")
    }

    pub fn path(&self, relative: &str) -> PathBuf {
        self.root().join(relative)
    }

    /// Writes a file (creating parent folders) and sets its modification time.
    pub fn write_file(
        &self,
        relative: &str,
        contents: &str,
        modified: SystemTime,
    ) -> io::Result<PathBuf> {
        let path = self.path(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, contents)?;
        set_modified(&path, modified)?;
        Ok(path)
    }

    /// Generates a project under [`Self::roots`] and returns its folder.
    pub fn project(&self, spec: &ProjectSpec) -> io::Result<PathBuf> {
        let dir = self.roots().join(&spec.name);
        let when = self.clock.days_ago(spec.inactive_days);
        let files = spec.kind.files(&spec.name);

        if spec.git {
            let repo = GitRepo::init(&dir, &self.git_config_home())?;
            for (file, contents) in &files {
                repo.commit_file(file, contents, &format!("add {file}"), when)?;
            }
        } else {
            fs::create_dir_all(&dir)?;
            for (file, contents) in &files {
                let path = dir.join(file);
                if let Some(parent) = path.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::write(path, contents)?;
            }
        }
        for (file, _) in &files {
            set_modified(&dir.join(file), when)?;
        }
        Ok(dir)
    }

    /// Repository where `branch` was squash-merged into `main` on the remote and
    /// then deleted there; the local branch still exists with a gone upstream.
    pub fn squash_merged_branch(
        &self,
        repo_name: &str,
        branch: &str,
    ) -> io::Result<SquashMergedBranch> {
        let config = self.git_config_home();
        let remote = GitRepo::init_bare(&self.path(&format!("remotes/{repo_name}.git")), &config)?;
        let repo = GitRepo::init(&self.roots().join(repo_name), &config)?;
        let remote_url = remote.path().to_string_lossy().into_owned();

        repo.commit_file(
            "README.md",
            "# repo\n",
            "initial commit",
            self.clock.days_ago(10),
        )?;
        repo.git(&["remote", "add", "origin", &remote_url])?;
        repo.git(&["push", "--quiet", "-u", "origin", "main"])?;

        repo.git(&["switch", "--quiet", "-c", branch])?;
        repo.commit_file(
            "feature.txt",
            "one\n",
            "feature: part 1",
            self.clock.days_ago(9),
        )?;
        repo.commit_file(
            "feature.txt",
            "one\ntwo\n",
            "feature: part 2",
            self.clock.days_ago(8),
        )?;
        repo.git(&["push", "--quiet", "-u", "origin", branch])?;

        repo.git(&["switch", "--quiet", "main"])?;
        repo.git(&["merge", "--squash", "--quiet", branch])?;
        repo.commit_staged(&format!("{branch} (#1)"), self.clock.days_ago(7))?;
        repo.git(&["push", "--quiet", "origin", "main"])?;
        repo.git(&["push", "--quiet", "origin", "--delete", branch])?;
        repo.git(&["fetch", "--quiet", "--prune"])?;

        Ok(SquashMergedBranch {
            repo,
            remote,
            branch: branch.to_owned(),
        })
    }

    /// Creates a file whose full path is longer than the classic 260-character limit.
    pub fn long_path_file(&self) -> io::Result<PathBuf> {
        let mut dir = self.path("caminho-longo");
        while dir.as_os_str().len() < 300 {
            dir.push("pasta-com-nome-bem-comprido-para-estourar-o-limite");
        }
        fs::create_dir_all(&dir)?;
        let file = dir.join("arquivo.txt");
        fs::write(&file, "conteúdo\n")?;
        Ok(file)
    }

    /// Handle to a repository of this machine, using its isolated git config.
    pub fn repo(&self, dir: &Path) -> GitRepo {
        GitRepo::open(dir, &self.git_config_home())
    }

    fn git_config_home(&self) -> PathBuf {
        self.path(".git-config")
    }

    /// Every file and folder of the machine, with size and modification time.
    pub fn snapshot(&self) -> MachineSnapshot {
        let mut entries = BTreeMap::new();
        let mut pending = vec![self.root().to_owned()];
        while let Some(dir) = pending.pop() {
            for entry in fs::read_dir(&dir).expect("readable fake machine") {
                let entry = entry.expect("readable entry");
                let meta = fs::symlink_metadata(entry.path()).expect("metadata");
                let relative = entry.path().strip_prefix(self.root()).unwrap().to_owned();
                if meta.is_dir() {
                    pending.push(entry.path());
                    entries.insert(relative, None);
                } else {
                    entries.insert(relative, Some((meta.len(), meta.modified().ok())));
                }
            }
        }
        MachineSnapshot(entries)
    }

    /// Guard for "analysis never changes anything" (I1): panics listing every path that
    /// was created, removed or modified since `before`.
    pub fn assert_unchanged_since(&self, before: &MachineSnapshot) {
        let after = self.snapshot();
        let changed: Vec<String> = before
            .0
            .keys()
            .chain(after.0.keys())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .filter(|path| before.0.get(*path) != after.0.get(*path))
            .map(|path| path.display().to_string())
            .collect();
        assert!(
            changed.is_empty(),
            "a máquina falsa foi alterada: {}",
            changed.join(", ")
        );
    }
}

/// Picture of the fake machine: relative path → (size, mtime) for files, `None` for folders.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MachineSnapshot(BTreeMap<PathBuf, Option<(u64, Option<SystemTime>)>>);

fn set_modified(path: &Path, modified: SystemTime) -> io::Result<()> {
    File::options()
        .write(true)
        .open(path)?
        .set_modified(modified)
}
