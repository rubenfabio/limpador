//! When a project was last worked on, whether it is "parado", and which project an
//! item belongs to (T-01.06).

use crate::ports::CommandRunner;
use crate::projects::{Discovery, NOT_SEARCHED, Project, ProjectGit};
use crate::roots::{contains, path_key};
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectState {
    Active,
    /// No activity for longer than the threshold.
    Inactive {
        idle: Duration,
    },
    /// Pinned by the user: always counts as active.
    Pinned,
}

impl ProjectState {
    pub fn is_active(self) -> bool {
        !matches!(self, Self::Inactive { .. })
    }
}

/// What decides a project's state besides its files and commits.
#[derive(Debug, Clone, Default)]
pub struct ActivityRules {
    pub inactivity: Duration,
    pub pinned: Vec<PathBuf>,
    /// Projects with a running container (from the Docker collector).
    pub running_containers: Vec<PathBuf>,
}

/// Most recent of: last commit on any local branch, and last change to a file that is
/// not ignored by git and not inside a dependency/build folder.
pub fn last_activity(project: &Project, commands: &dyn CommandRunner) -> Option<SystemTime> {
    let commits = if project.git == ProjectGit::None {
        None
    } else {
        last_commit(&project.root, commands)
    };
    commits.max(last_file_change(project, commands))
}

pub fn state(
    project: &Project,
    last: Option<SystemTime>,
    now: SystemTime,
    rules: &ActivityRules,
) -> ProjectState {
    let is = |list: &[PathBuf]| list.iter().any(|p| path_key(p) == path_key(&project.root));
    if is(&rules.pinned) {
        return ProjectState::Pinned;
    }
    if is(&rules.running_containers) {
        return ProjectState::Active;
    }
    // Without any file or commit there is no sign of work: as idle as the threshold.
    let idle = last.map_or(rules.inactivity, |at| {
        now.duration_since(at).unwrap_or_default()
    });
    if idle > rules.inactivity {
        ProjectState::Inactive { idle }
    } else {
        ProjectState::Active
    }
}

/// The project an item belongs to: the most specific project containing `hint`.
/// Paths inside a worktree belong to the worktree's origin repository.
pub fn associate<'a>(hint: &Path, discovery: &'a Discovery) -> Option<&'a Project> {
    let origin = discovery
        .worktrees
        .iter()
        .filter(|w| contains(&w.root, hint))
        .max_by_key(|w| path_key(&w.root).len())
        .and_then(|w| match &w.git {
            ProjectGit::Worktree { origin } => Some(origin.clone()),
            _ => None,
        });
    let target = origin.as_deref().unwrap_or(hint);
    discovery
        .projects
        .iter()
        .filter(|p| contains(&p.root, target))
        .max_by_key(|p| path_key(&p.root).len())
}

fn last_commit(root: &Path, commands: &dyn CommandRunner) -> Option<SystemTime> {
    let root = root.to_string_lossy();
    let out = commands
        .run(
            "git",
            &[
                "-C",
                &root,
                "for-each-ref",
                "--format=%(committerdate:unix)",
                "refs/heads",
            ],
        )
        .ok()?;
    out.stdout
        .lines()
        .filter_map(|line| line.trim().parse::<u64>().ok())
        .max()
        .map(|secs| UNIX_EPOCH + Duration::from_secs(secs))
}

fn last_file_change(project: &Project, commands: &dyn CommandRunner) -> Option<SystemTime> {
    // Outside a repository `git ls-files` fails and the folder is walked instead.
    let files = git_files(&project.root, commands).unwrap_or_else(|| walk(&project.root));
    files
        .iter()
        .filter(|f| !in_dependency_folder(f.strip_prefix(&project.root).unwrap_or(f)))
        .filter_map(|f| fs::metadata(f).and_then(|m| m.modified()).ok())
        .max()
}

/// Tracked plus untracked-but-not-ignored files, as git sees the working tree.
fn git_files(root: &Path, commands: &dyn CommandRunner) -> Option<Vec<PathBuf>> {
    let text = root.to_string_lossy();
    let out = commands
        .run(
            "git",
            &[
                "-C",
                &text,
                "ls-files",
                "-z",
                "--cached",
                "--others",
                "--exclude-standard",
            ],
        )
        .ok()
        .filter(|o| o.success())?;
    Some(
        out.stdout
            .split('\0')
            .filter(|f| !f.is_empty())
            .map(|f| root.join(f))
            .collect(),
    )
}

/// Every file under `root`, skipping dependency/build folders and never following links.
fn walk(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let mut pending = vec![root.to_owned()];
    while let Some(dir) = pending.pop() {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.filter_map(Result::ok) {
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            if kind.is_dir() && !kind.is_symlink() {
                if !NOT_SEARCHED.contains(&entry.file_name().to_string_lossy().as_ref()) {
                    pending.push(entry.path());
                }
            } else if kind.is_file() {
                files.push(entry.path());
            }
        }
    }
    files
}

fn in_dependency_folder(relative: &Path) -> bool {
    relative.components().any(|c| match c {
        Component::Normal(name) => NOT_SEARCHED.contains(&name.to_string_lossy().as_ref()),
        _ => false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::SystemCommandRunner;
    use crate::projects::{DiscoveryOptions, discover};
    use limpador_testkit::{FakeMachine, ProjectSpec};

    const DAY: Duration = Duration::from_secs(24 * 3600);

    fn rules() -> ActivityRules {
        ActivityRules {
            inactivity: 30 * DAY,
            ..ActivityRules::default()
        }
    }

    fn project_of(machine: &FakeMachine, name: &str) -> Project {
        discover(&[machine.roots()], &DiscoveryOptions::default())
            .projects
            .into_iter()
            .find(|p| p.name == name)
            .unwrap()
    }

    fn now(machine: &FakeMachine) -> SystemTime {
        machine.clock().days_ago(0)
    }

    fn state_of(machine: &FakeMachine, name: &str, rules: &ActivityRules) -> ProjectState {
        let project = project_of(machine, name);
        let last = last_activity(&project, &SystemCommandRunner);
        state(&project, last, now(machine), rules)
    }

    #[test]
    fn a_recent_commit_counts_even_when_every_file_is_old() {
        let machine = FakeMachine::new().unwrap();
        let root = machine
            .project(&ProjectSpec::node("api").inactive_days(45).with_git())
            .unwrap();
        let old = machine.clock().days_ago(45);
        machine
            .repo(&root)
            .commit_file(
                "CHANGELOG.md",
                "v2\n",
                "release",
                machine.clock().days_ago(2),
            )
            .unwrap();
        machine
            .write_file("roots/api/CHANGELOG.md", "v2\n", old)
            .unwrap();

        assert_eq!(state_of(&machine, "api", &rules()), ProjectState::Active);
        let last = last_activity(&project_of(&machine, "api"), &SystemCommandRunner).unwrap();
        assert_eq!(last, machine.clock().days_ago(2));
    }

    #[test]
    fn files_ignored_by_git_do_not_count() {
        let machine = FakeMachine::new().unwrap();
        let root = machine
            .project(&ProjectSpec::node("api").inactive_days(45).with_git())
            .unwrap();
        let old = machine.clock().days_ago(45);
        machine
            .repo(&root)
            .commit_file(".gitignore", "logs/\n", "ignore logs", old)
            .unwrap();
        machine
            .write_file("roots/api/.gitignore", "logs/\n", old)
            .unwrap();
        machine
            .write_file("roots/api/logs/hoje.log", "x", now(&machine))
            .unwrap();

        assert!(!state_of(&machine, "api", &rules()).is_active());
    }

    #[test]
    fn nested_changes_count_in_projects_without_git() {
        let machine = FakeMachine::new().unwrap();
        machine
            .project(&ProjectSpec::python("script").inactive_days(90))
            .unwrap();
        machine
            .write_file(
                "roots/script/src/util/novo.py",
                "x",
                machine.clock().days_ago(3),
            )
            .unwrap();

        assert_eq!(state_of(&machine, "script", &rules()), ProjectState::Active);
    }

    #[test]
    fn t01_06_ca1_project_idle_for_45_days_is_inactive_for_45_days() {
        let machine = FakeMachine::new().unwrap();
        machine
            .project(&ProjectSpec::node("api").inactive_days(45).with_git())
            .unwrap();

        assert_eq!(
            state_of(&machine, "api", &rules()),
            ProjectState::Inactive { idle: 45 * DAY }
        );
    }

    #[test]
    fn t01_06_ca2_code_changed_yesterday_makes_it_active() {
        let machine = FakeMachine::new().unwrap();
        machine
            .project(&ProjectSpec::node("api").inactive_days(45).with_git())
            .unwrap();
        machine
            .write_file("roots/api/src/novo.js", "x", machine.clock().days_ago(1))
            .unwrap();

        assert_eq!(state_of(&machine, "api", &rules()), ProjectState::Active);
    }

    #[test]
    fn t01_06_ca3_recent_files_in_node_modules_do_not_count() {
        let machine = FakeMachine::new().unwrap();
        machine
            .project(&ProjectSpec::node("api").inactive_days(45).with_git())
            .unwrap();
        machine
            .write_file("roots/api/node_modules/lib/index.js", "x", now(&machine))
            .unwrap();

        assert!(!state_of(&machine, "api", &rules()).is_active());
    }

    #[test]
    fn projects_without_git_use_their_files() {
        let machine = FakeMachine::new().unwrap();
        machine
            .project(&ProjectSpec::python("script").inactive_days(90))
            .unwrap();
        machine
            .write_file("roots/script/.venv/lib/x.py", "x", now(&machine))
            .unwrap();

        assert_eq!(
            state_of(&machine, "script", &rules()),
            ProjectState::Inactive { idle: 90 * DAY }
        );
    }

    #[test]
    fn t01_06_ca4_running_container_makes_an_idle_project_active() {
        let machine = FakeMachine::new().unwrap();
        let root = machine
            .project(&ProjectSpec::node("api").inactive_days(45).with_git())
            .unwrap();
        let rules = ActivityRules {
            running_containers: vec![root],
            ..rules()
        };
        assert_eq!(state_of(&machine, "api", &rules), ProjectState::Active);
    }

    #[test]
    fn t01_06_ca5_pinned_project_is_always_active() {
        let machine = FakeMachine::new().unwrap();
        let root = machine
            .project(&ProjectSpec::node("tcc").inactive_days(200).with_git())
            .unwrap();
        let rules = ActivityRules {
            pinned: vec![root],
            ..rules()
        };
        let state = state_of(&machine, "tcc", &rules);
        assert_eq!(state, ProjectState::Pinned);
        assert!(state.is_active());
    }

    #[test]
    fn exactly_at_the_threshold_is_still_active() {
        let machine = FakeMachine::new().unwrap();
        machine
            .project(&ProjectSpec::node("api").inactive_days(30).with_git())
            .unwrap();
        assert_eq!(state_of(&machine, "api", &rules()), ProjectState::Active);
    }

    #[test]
    fn t01_06_ca6_monorepo_dependency_folder_belongs_to_the_monorepo() {
        let machine = FakeMachine::new().unwrap();
        machine
            .project(&ProjectSpec::node("mono").with_git())
            .unwrap();
        machine
            .write_file("roots/mono/apps/web/package.json", "{}", now(&machine))
            .unwrap();
        let discovery = discover(&[machine.roots()], &DiscoveryOptions::default());

        let hint = machine.roots().join("mono/apps/web/node_modules");
        assert_eq!(associate(&hint, &discovery).unwrap().name, "mono");
    }

    #[test]
    fn t01_06_ca7_compose_working_dir_associates_a_volume_with_its_project() {
        let machine = FakeMachine::new().unwrap();
        machine.project(&ProjectSpec::node("api")).unwrap();
        machine.project(&ProjectSpec::node("web")).unwrap();
        let discovery = discover(&[machine.roots()], &DiscoveryOptions::default());

        let compose_working_dir = machine.roots().join("api");
        assert_eq!(
            associate(&compose_working_dir, &discovery).unwrap().name,
            "api"
        );
    }

    #[test]
    fn worktree_items_belong_to_the_origin_repository() {
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
                "f",
                &worktree.to_string_lossy(),
            ])
            .unwrap();
        let discovery = discover(&[machine.roots()], &DiscoveryOptions::default());

        let hint = worktree.join("node_modules");
        assert_eq!(associate(&hint, &discovery).unwrap().name, "loja");
    }

    #[test]
    fn items_outside_every_project_have_no_project() {
        let machine = FakeMachine::new().unwrap();
        machine.project(&ProjectSpec::node("api")).unwrap();
        let discovery = discover(&[machine.roots()], &DiscoveryOptions::default());
        assert!(associate(Path::new("C:\\Users\\x\\AppData\\npm-cache"), &discovery).is_none());
    }
}
