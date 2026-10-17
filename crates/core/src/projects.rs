//! Finding software projects inside the roots (T-02.03).

use crate::roots::is_excluded;
use std::fs;
use std::path::{Path, PathBuf};

/// Files and folders that make a folder a project. Entries starting with `*` match by
/// extension (`*.sln`).
pub const DEFAULT_MARKERS: &[&str] = &[
    ".git",
    "package.json",
    "pyproject.toml",
    "requirements.txt",
    "Cargo.toml",
    "go.mod",
    "pom.xml",
    "build.gradle",
    "build.gradle.kts",
    "*.sln",
    "*.csproj",
    "composer.json",
    "Gemfile",
    "docker-compose.yml",
    "compose.yaml",
];

/// Dependency and build folders: never searched (and never projects themselves).
pub(crate) const NOT_SEARCHED: &[&str] = &[
    ".git",
    "node_modules",
    ".venv",
    "venv",
    "target",
    "vendor",
    "__pycache__",
    ".next",
    ".nuxt",
    ".svelte-kit",
    ".gradle",
    ".turbo",
    "dist",
    "build",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectGit {
    None,
    Repository,
    /// A git worktree; `origin` is the main repository it belongs to.
    Worktree {
        origin: PathBuf,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Project {
    pub root: PathBuf,
    pub name: String,
    pub git: ProjectGit,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Discovery {
    pub projects: Vec<Project>,
    /// Worktrees belong to their origin repository; they are not projects of their own.
    pub worktrees: Vec<Project>,
}

#[derive(Debug, Clone)]
pub struct DiscoveryOptions {
    pub exclusions: Vec<String>,
    /// Levels below each root that are searched (T-02.05 `scan_depth`).
    pub max_depth: u32,
    pub markers: Vec<String>,
}

impl Default for DiscoveryOptions {
    fn default() -> Self {
        Self {
            exclusions: Vec::new(),
            max_depth: 6,
            markers: DEFAULT_MARKERS.iter().map(|m| (*m).to_owned()).collect(),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Enclosing {
    Nothing,
    /// Inside a folder project without git: nested markers belong to it (no git = outermost wins).
    PlainProject,
    /// Inside a git repository: everything belongs to it, except nested repositories.
    Repository,
}

pub fn discover(roots: &[PathBuf], options: &DiscoveryOptions) -> Discovery {
    let mut found = Discovery::default();
    for root in roots {
        visit(root, 0, Enclosing::Nothing, options, &mut found);
    }
    found
}

fn visit(
    dir: &Path,
    depth: u32,
    enclosing: Enclosing,
    options: &DiscoveryOptions,
    found: &mut Discovery,
) {
    if is_excluded(dir, &options.exclusions) {
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    let entries: Vec<fs::DirEntry> = entries.filter_map(Result::ok).collect();
    let names: Vec<String> = entries
        .iter()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();

    let mut inside = enclosing;
    if let Some(git) = git_kind(dir) {
        let project = new_project(dir, git);
        if matches!(project.git, ProjectGit::Worktree { .. }) {
            found.worktrees.push(project);
        } else {
            found.projects.push(project);
        }
        inside = Enclosing::Repository;
    } else if enclosing == Enclosing::Nothing && has_marker(&names, &options.markers) {
        found.projects.push(new_project(dir, ProjectGit::None));
        inside = Enclosing::PlainProject;
    }

    if depth >= options.max_depth {
        return;
    }
    for entry in entries {
        let name = entry.file_name().to_string_lossy().into_owned();
        // file_type() does not follow links, so junctions and symlinks are never reported as
        // folders and never entered (I3).
        let is_real_dir = entry.file_type().is_ok_and(|t| t.is_dir());
        if is_real_dir && !NOT_SEARCHED.contains(&name.as_str()) {
            visit(&entry.path(), depth + 1, inside, options, found);
        }
    }
}

fn new_project(dir: &Path, git: ProjectGit) -> Project {
    Project {
        root: dir.to_owned(),
        name: dir.file_name().map_or_else(
            || dir.display().to_string(),
            |n| n.to_string_lossy().into_owned(),
        ),
        git,
    }
}

fn has_marker(names: &[String], markers: &[String]) -> bool {
    markers.iter().any(|marker| match marker.strip_prefix('*') {
        Some(suffix) => names.iter().any(|n| n.ends_with(suffix)),
        None => names.iter().any(|n| n == marker),
    })
}

/// `.git` folder → repository; `.git` file → worktree or submodule (read from `gitdir:`).
fn git_kind(dir: &Path) -> Option<ProjectGit> {
    let dot_git = dir.join(".git");
    let meta = fs::symlink_metadata(&dot_git).ok()?;
    if meta.is_dir() {
        return Some(ProjectGit::Repository);
    }
    let content = fs::read_to_string(&dot_git).ok()?;
    let gitdir = content
        .lines()
        .find_map(|l| l.strip_prefix("gitdir:"))?
        .trim();
    let gitdir = dir.join(gitdir); // relative gitdirs are relative to the worktree
    let text = gitdir.to_string_lossy().replace('\\', "/");
    match text.find("/.git/worktrees/") {
        Some(at) => Some(ProjectGit::Worktree {
            origin: PathBuf::from(&text[..at]),
        }),
        None => Some(ProjectGit::Repository), // submodule: its own repository
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use limpador_testkit::{FakeMachine, ProjectSpec, create_junction};

    fn names(projects: &[Project]) -> Vec<String> {
        let mut names: Vec<String> = projects.iter().map(|p| p.name.clone()).collect();
        names.sort();
        names
    }

    fn discover_in(machine: &FakeMachine, options: &DiscoveryOptions) -> Discovery {
        discover(&[machine.roots()], options)
    }

    #[test]
    fn t02_03_ca1_monorepo_is_a_single_project() {
        let machine = FakeMachine::new().unwrap();
        machine
            .project(&ProjectSpec::node("mono").with_git())
            .unwrap();
        let now = machine.clock().days_ago(0);
        machine
            .write_file("roots/mono/apps/web/package.json", "{}", now)
            .unwrap();
        machine
            .write_file("roots/mono/apps/api/package.json", "{}", now)
            .unwrap();

        let found = discover_in(&machine, &DiscoveryOptions::default());
        assert_eq!(names(&found.projects), vec!["mono"]);
        assert_eq!(found.projects[0].git, ProjectGit::Repository);
    }

    #[test]
    fn t02_03_ca2_folder_without_git_with_package_json_is_a_project() {
        let machine = FakeMachine::new().unwrap();
        machine.project(&ProjectSpec::node("prototipo")).unwrap();

        let found = discover_in(&machine, &DiscoveryOptions::default());
        assert_eq!(found.projects.len(), 1);
        assert_eq!(found.projects[0].name, "prototipo");
        assert_eq!(found.projects[0].git, ProjectGit::None);
    }

    #[test]
    fn folder_project_without_git_is_the_outermost_marker() {
        let machine = FakeMachine::new().unwrap();
        machine.project(&ProjectSpec::node("site")).unwrap();
        let now = machine.clock().days_ago(0);
        machine
            .write_file("roots/site/tools/requirements.txt", "x", now)
            .unwrap();

        assert_eq!(
            names(&discover_in(&machine, &DiscoveryOptions::default()).projects),
            vec!["site"]
        );
    }

    #[test]
    fn t02_03_ca3_repository_cloned_inside_another_is_a_separate_project() {
        let machine = FakeMachine::new().unwrap();
        machine
            .project(&ProjectSpec::node("externo").with_git())
            .unwrap();
        machine
            .project(&ProjectSpec::rust("externo/libs/interno").with_git())
            .unwrap();

        let found = discover_in(&machine, &DiscoveryOptions::default());
        assert_eq!(names(&found.projects), vec!["externo", "interno"]);
    }

    #[test]
    fn t02_03_ca4_worktree_belongs_to_its_origin_repository() {
        let machine = FakeMachine::new().unwrap();
        let repo = machine
            .project(&ProjectSpec::node("loja").with_git())
            .unwrap();
        let worktree = machine.roots().join("loja-feature");
        machine
            .repo(&repo)
            .git(&[
                "worktree",
                "add",
                "--quiet",
                "-b",
                "feature",
                &worktree.to_string_lossy(),
            ])
            .unwrap();

        let found = discover_in(&machine, &DiscoveryOptions::default());
        assert_eq!(names(&found.projects), vec!["loja"]);
        assert_eq!(found.worktrees.len(), 1);
        assert_eq!(found.worktrees[0].name, "loja-feature");
        let ProjectGit::Worktree { origin } = &found.worktrees[0].git else {
            panic!("expected a worktree");
        };
        assert_eq!(
            crate::roots::path_key(origin),
            crate::roots::path_key(&repo)
        );
    }

    #[test]
    fn t02_03_ca5_package_json_inside_node_modules_is_not_a_project() {
        let machine = FakeMachine::new().unwrap();
        let now = machine.clock().days_ago(0);
        machine
            .write_file("roots/solto/node_modules/lib/package.json", "{}", now)
            .unwrap();

        assert!(
            discover_in(&machine, &DiscoveryOptions::default())
                .projects
                .is_empty()
        );
    }

    #[test]
    fn t02_03_ca6_projects_deeper_than_the_limit_are_not_found() {
        let machine = FakeMachine::new().unwrap();
        let now = machine.clock().days_ago(0);
        machine
            .write_file("roots/1/2/3/4/5/6/7/fundo/package.json", "{}", now)
            .unwrap();

        let shallow = DiscoveryOptions::default(); // depth 6
        assert!(discover_in(&machine, &shallow).projects.is_empty());
        let deep = DiscoveryOptions {
            max_depth: 8,
            ..DiscoveryOptions::default()
        };
        assert_eq!(names(&discover_in(&machine, &deep).projects), vec!["fundo"]);
    }

    #[test]
    fn excluded_folders_are_not_searched() {
        let machine = FakeMachine::new().unwrap();
        machine.project(&ProjectSpec::node("cliente-x")).unwrap();
        machine.project(&ProjectSpec::node("meu")).unwrap();
        let options = DiscoveryOptions {
            exclusions: vec![
                machine
                    .roots()
                    .join("cliente-x")
                    .to_string_lossy()
                    .into_owned(),
            ],
            ..DiscoveryOptions::default()
        };

        assert_eq!(
            names(&discover_in(&machine, &options).projects),
            vec!["meu"]
        );
    }

    #[test]
    fn junctions_are_never_followed() {
        let machine = FakeMachine::new().unwrap();
        let outside = tempfile::tempdir().unwrap();
        fs::write(outside.path().join("package.json"), "{}").unwrap();
        create_junction(&machine.roots().join("atalho"), outside.path()).unwrap();

        assert!(
            discover_in(&machine, &DiscoveryOptions::default())
                .projects
                .is_empty()
        );
    }

    #[test]
    fn extension_markers_match_any_file_name() {
        let machine = FakeMachine::new().unwrap();
        let now = machine.clock().days_ago(0);
        machine.write_file("roots/dotnet/App.sln", "", now).unwrap();
        assert_eq!(
            names(&discover_in(&machine, &DiscoveryOptions::default()).projects),
            vec!["dotnet"]
        );
    }
}
