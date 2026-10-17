//! First run (T-02.01): suggest root folders and record the user's choices.

use crate::config::Config;
use crate::projects::{DiscoveryOptions, discover};
use crate::roots::path_key;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// Depth of the quick search behind each suggestion.
const SUGGESTION_DEPTH: u32 = 3;

/// Where developers usually keep projects on Windows. `projetos` is normally `C:\Projetos`;
/// a sandboxed app points it into the fake machine.
pub fn default_candidates(projetos: &Path, home: &Path) -> Vec<PathBuf> {
    let mut candidates = vec![projetos.to_owned()];
    for relative in [
        "source/repos",
        "Projects",
        "projetos",
        "dev",
        "code",
        "repos",
        "Documents/GitHub",
    ] {
        candidates.push(home.join(relative));
    }
    candidates
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RootSuggestion {
    pub path: PathBuf,
    pub projects: usize,
}

/// Candidates that exist and hold at least one project, with how many. Stops looking
/// once `budget` is spent, so the first screen never waits long (≤ 10 s).
pub fn suggest_roots(candidates: &[PathBuf], budget: Duration) -> Vec<RootSuggestion> {
    let deadline = Instant::now() + budget;
    let options = DiscoveryOptions {
        max_depth: SUGGESTION_DEPTH,
        ..DiscoveryOptions::default()
    };
    let mut seen = Vec::new();
    let mut suggestions = Vec::new();
    for candidate in candidates {
        if Instant::now() >= deadline {
            break;
        }
        let key = path_key(candidate);
        if !candidate.is_dir() || seen.contains(&key) {
            continue;
        }
        seen.push(key);
        let projects = discover(std::slice::from_ref(candidate), &options)
            .projects
            .len();
        if projects > 0 {
            suggestions.push(RootSuggestion {
                path: candidate.clone(),
                projects,
            });
        }
    }
    suggestions
}

/// What the user decided in the onboarding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OnboardingChoices {
    pub roots: Vec<PathBuf>,
    pub auto_clean_safe: bool,
    pub start_with_system: bool,
}

/// Saves the choices into the configuration and marks the onboarding as done.
pub fn complete(config: &mut Config, choices: OnboardingChoices) {
    config.roots.paths = choices.roots;
    config.preferences.auto_clean_safe = choices.auto_clean_safe;
    config.preferences.start_with_system = choices.start_with_system;
    config.onboarding_done = true;
}

/// Skipping keeps everything empty; the dashboard then invites to configure (T-02.01 CA4).
pub fn skip(config: &mut Config) {
    config.onboarding_done = true;
}

#[cfg(test)]
mod tests {
    use super::*;
    use limpador_testkit::{FakeMachine, ProjectSpec};

    #[test]
    fn t02_01_ca1_candidate_with_projects_is_suggested_with_its_count() {
        let machine = FakeMachine::new().unwrap();
        for i in 0..12 {
            machine
                .project(&ProjectSpec::node(&format!("projeto-{i}")))
                .unwrap();
        }

        let suggestions = suggest_roots(&[machine.roots()], Duration::from_secs(10));
        assert_eq!(
            suggestions,
            vec![RootSuggestion {
                path: machine.roots(),
                projects: 12
            }]
        );
    }

    #[test]
    fn t02_01_ca2_candidate_without_projects_is_not_suggested() {
        let machine = FakeMachine::new().unwrap();
        std::fs::create_dir_all(machine.path("vazia/docs")).unwrap();
        let missing = machine.path("nao-existe");

        let suggestions = suggest_roots(&[machine.path("vazia"), missing], Duration::from_secs(10));
        assert!(suggestions.is_empty());
    }

    #[test]
    fn duplicate_candidates_are_counted_once() {
        let machine = FakeMachine::new().unwrap();
        machine.project(&ProjectSpec::node("a")).unwrap();
        let suggestions =
            suggest_roots(&[machine.roots(), machine.roots()], Duration::from_secs(10));
        assert_eq!(suggestions.len(), 1);
    }

    #[test]
    fn the_search_stops_when_the_time_budget_is_spent() {
        let machine = FakeMachine::new().unwrap();
        machine.project(&ProjectSpec::node("a")).unwrap();
        assert!(suggest_roots(&[machine.roots()], Duration::ZERO).is_empty());
    }

    #[test]
    fn the_quick_search_is_shallow() {
        let machine = FakeMachine::new().unwrap();
        let now = machine.clock().days_ago(0);
        machine
            .write_file("roots/a/b/c/d/package.json", "{}", now)
            .unwrap();
        assert!(suggest_roots(&[machine.roots()], Duration::from_secs(10)).is_empty());
    }

    #[test]
    fn default_candidates_follow_the_task() {
        let home = Path::new("C:\\Users\\dev");
        let candidates = default_candidates(Path::new("C:\\Projetos"), home);
        assert_eq!(candidates[0], PathBuf::from("C:\\Projetos"));
        assert!(candidates.contains(&home.join("source/repos")));
        assert!(candidates.contains(&home.join("Documents/GitHub")));
        assert_eq!(candidates.len(), 8);
    }

    #[test]
    fn t02_01_ca3_and_ca5_choices_are_saved_and_the_onboarding_is_done() {
        let mut config = Config::default();
        complete(
            &mut config,
            OnboardingChoices {
                roots: vec!["C:\\Projetos".into(), "D:\\extra".into()],
                auto_clean_safe: true,
                start_with_system: true,
            },
        );
        assert!(config.onboarding_done);
        assert_eq!(
            config.roots.paths,
            vec![PathBuf::from("C:\\Projetos"), PathBuf::from("D:\\extra")]
        );
        assert!(config.preferences.auto_clean_safe && config.preferences.start_with_system);
    }

    #[test]
    fn t02_01_ca4_skipping_leaves_no_roots_configured() {
        let mut config = Config::default();
        skip(&mut config);
        assert!(config.onboarding_done);
        assert!(config.roots.paths.is_empty());
    }
}
