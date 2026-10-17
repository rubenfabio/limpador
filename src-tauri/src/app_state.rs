//! What the commands work on: configuration, the analyzer and the folders the app uses.
//! Kept free of Tauri types so it can be tested directly.

use limpador_core::analysis::Analyzer;
use limpador_core::collector::Registry;
use limpador_core::config::{Config, ConfigStore};
use limpador_core::engine::{CollectorState, Engine, EngineConfig};
use limpador_core::message::Message;
use limpador_core::onboarding::{self, OnboardingChoices, RootSuggestion};
use limpador_core::ports::{SystemClock, SystemCommandRunner};
use limpador_core::roots::{RootError, add_root};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Folders the app reads and writes. A sandboxed run puts all of them inside the fake
/// machine (ADR-0012).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Paths {
    pub config_dir: PathBuf,
    pub home: PathBuf,
    /// Usually `C:\Projetos`, the first root suggested.
    pub projetos: PathBuf,
}

impl Paths {
    pub fn real(config_dir: PathBuf, home: PathBuf) -> Self {
        Self {
            config_dir,
            home,
            projetos: PathBuf::from("C:\\Projetos"),
        }
    }

    pub fn sandbox(root: &Path) -> Self {
        Self {
            config_dir: root.join("app-data").join("config"),
            home: root.join("home"),
            projetos: root.join("Projetos"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub onboarding_done: bool,
    pub roots: Vec<String>,
    pub auto_clean_safe: bool,
    pub start_with_system: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Choices {
    pub roots: Vec<String>,
    pub auto_clean_safe: bool,
    pub start_with_system: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CollectorSummary {
    pub id: String,
    pub state: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisSummary {
    pub recoverable: u64,
    pub recoverable_safe: u64,
    pub space_items: usize,
    pub organization_items: usize,
    pub protected_items: usize,
    pub projects: usize,
    pub warnings: Vec<Message>,
    pub collectors: Vec<CollectorSummary>,
}

pub struct AppState {
    paths: Paths,
    store: ConfigStore,
    config: Mutex<Config>,
    /// Problems found while loading the configuration (shown once in the UI).
    pub load_warnings: Vec<Message>,
    analyzer: Analyzer,
}

impl AppState {
    pub fn load(paths: Paths, registry: Registry) -> Self {
        let store = ConfigStore::new(&paths.config_dir);
        let (config, load_warnings) = store.load();
        let engine = Engine::new(registry, engine_config(&config));
        Self {
            paths,
            store,
            config: Mutex::new(config),
            load_warnings,
            analyzer: Analyzer {
                engine,
                clock: Arc::new(SystemClock),
                commands: Arc::new(SystemCommandRunner),
            },
        }
    }

    pub fn settings(&self) -> Settings {
        let config = self.config.lock().unwrap();
        Settings {
            onboarding_done: config.onboarding_done,
            roots: config
                .roots
                .paths
                .iter()
                .map(|p| p.display().to_string())
                .collect(),
            auto_clean_safe: config.preferences.auto_clean_safe,
            start_with_system: config.preferences.start_with_system,
        }
    }

    /// Root suggestions for the onboarding (≤ 10 s).
    pub fn suggest_roots(&self) -> Vec<RootSuggestion> {
        onboarding::suggest_roots(
            &onboarding::default_candidates(&self.paths.projetos, &self.paths.home),
            Duration::from_secs(10),
        )
    }

    /// Checks a folder the user picked by hand; the error is a translation key.
    pub fn validate_root(&self, path: &str) -> Result<(), String> {
        let mut scratch = limpador_core::config::Roots::default();
        add_root(&mut scratch, Path::new(path))
            .map(|_| ())
            .map_err(|e| match e {
                RootError::NotFound => "roots.not_found".to_owned(),
                RootError::NotReadable => "roots.not_readable".to_owned(),
            })
    }

    pub fn complete_onboarding(&self, choices: Choices) -> Result<Settings, String> {
        self.update(|config| {
            onboarding::complete(
                config,
                OnboardingChoices {
                    roots: choices.roots.iter().map(PathBuf::from).collect(),
                    auto_clean_safe: choices.auto_clean_safe,
                    start_with_system: choices.start_with_system,
                },
            );
        })
    }

    pub fn skip_onboarding(&self) -> Result<Settings, String> {
        self.update(onboarding::skip)
    }

    /// Runs a full analysis with the current configuration.
    pub fn run_analysis(&self) -> AnalysisSummary {
        let config = self.config.lock().unwrap().clone();
        let report = self.analyzer.analyze(&config, Arc::new(|_, _| {}));
        let totals = &report.ranking.totals;
        AnalysisSummary {
            recoverable: totals.recoverable,
            recoverable_safe: totals.recoverable_safe,
            space_items: report.ranking.space.len(),
            organization_items: totals.organization_items,
            protected_items: report.ranking.protected.len(),
            projects: report.projects.len(),
            warnings: report.warnings.clone(),
            collectors: report
                .result
                .collectors
                .iter()
                .map(|c| CollectorSummary {
                    id: c.id.clone(),
                    state: state_name(&c.state),
                })
                .collect(),
        }
    }

    /// Applies `change`, saves, and only then keeps the new configuration.
    fn update(&self, change: impl FnOnce(&mut Config)) -> Result<Settings, String> {
        {
            let mut config = self.config.lock().unwrap();
            let mut next = config.clone();
            change(&mut next);
            self.store
                .save(&next)
                .map_err(|e| format!("config.save_failed: {e:?}"))?;
            *config = next;
        }
        Ok(self.settings())
    }
}

/// Engine settings that come from the user's preferences (T-02.05).
pub fn engine_config(config: &Config) -> EngineConfig {
    EngineConfig {
        collector_timeout: config.preferences.collector_timeout(),
        ..EngineConfig::default()
    }
}

fn state_name(state: &CollectorState) -> &'static str {
    match state {
        CollectorState::Waiting => "waiting",
        CollectorState::Analyzing(_) => "analyzing",
        CollectorState::Done => "done",
        CollectorState::Error(_) => "error",
        CollectorState::Unavailable { .. } => "unavailable",
        CollectorState::NotInstalled => "not_installed",
        CollectorState::Cancelled => "cancelled",
        CollectorState::TimedOut => "timed_out",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn sandbox() -> (tempfile::TempDir, AppState) {
        let dir = tempfile::tempdir().unwrap();
        for project in ["api", "web"] {
            let folder = dir.path().join("Projetos").join(project);
            fs::create_dir_all(&folder).unwrap();
            fs::write(folder.join("package.json"), "{}").unwrap();
        }
        let state = AppState::load(Paths::sandbox(dir.path()), Registry::default());
        (dir, state)
    }

    #[test]
    fn t02_01_ca1_sandbox_suggests_its_projetos_folder_with_the_count() {
        let (dir, state) = sandbox();
        assert_eq!(
            state.suggest_roots(),
            vec![RootSuggestion {
                path: dir.path().join("Projetos"),
                projects: 2
            }]
        );
    }

    #[test]
    fn new_install_starts_with_the_onboarding() {
        let (_dir, state) = sandbox();
        let settings = state.settings();
        assert!(!settings.onboarding_done);
        assert!(settings.roots.is_empty());
        assert!(state.load_warnings.is_empty());
    }

    #[test]
    fn t02_01_ca3_manual_folder_is_validated_and_saved() {
        let (dir, state) = sandbox();
        let extra = dir.path().join("extra");
        fs::create_dir_all(&extra).unwrap();
        let extra = extra.display().to_string();

        assert_eq!(state.validate_root(&extra), Ok(()));
        assert_eq!(
            state.validate_root(&format!("{extra}-nao-existe")),
            Err("roots.not_found".to_owned())
        );

        state
            .complete_onboarding(Choices {
                roots: vec![extra.clone()],
                auto_clean_safe: false,
                start_with_system: false,
            })
            .unwrap();
        let reloaded = AppState::load(Paths::sandbox(dir.path()), Registry::default());
        assert_eq!(reloaded.settings().roots, vec![extra]);
    }

    #[test]
    fn t02_01_ca5_choices_are_saved_and_survive_a_restart() {
        let (dir, state) = sandbox();
        let root = dir.path().join("Projetos").display().to_string();
        let settings = state
            .complete_onboarding(Choices {
                roots: vec![root.clone()],
                auto_clean_safe: true,
                start_with_system: true,
            })
            .unwrap();
        let expected = Settings {
            onboarding_done: true,
            roots: vec![root],
            auto_clean_safe: true,
            start_with_system: true,
        };
        assert_eq!(settings, expected);
        let reloaded = AppState::load(Paths::sandbox(dir.path()), Registry::default());
        assert_eq!(reloaded.settings(), expected);
    }

    #[test]
    fn t02_01_ca4_skipping_saves_no_roots() {
        let (_dir, state) = sandbox();
        let settings = state.skip_onboarding().unwrap();
        assert!(settings.onboarding_done);
        assert!(settings.roots.is_empty());
    }

    #[test]
    fn analysis_with_the_saved_roots_counts_their_projects() {
        let (dir, state) = sandbox();
        state
            .complete_onboarding(Choices {
                roots: vec![dir.path().join("Projetos").display().to_string()],
                auto_clean_safe: false,
                start_with_system: false,
            })
            .unwrap();
        let summary = state.run_analysis();
        assert_eq!(summary.projects, 2);
        assert_eq!(summary.recoverable, 0);
        assert!(summary.collectors.is_empty() && summary.warnings.is_empty());
    }

    #[test]
    fn real_paths_suggest_c_projetos_first() {
        let paths = Paths::real("C:\\cfg".into(), "C:\\Users\\dev".into());
        assert_eq!(paths.projetos, PathBuf::from("C:\\Projetos"));
    }

    #[test]
    fn t02_05_ca3_collector_time_limit_comes_from_the_preferences() {
        let mut config = Config::default();
        config.preferences.collector_timeout_minutes = 2;
        assert_eq!(
            engine_config(&config).collector_timeout,
            Duration::from_secs(120)
        );
    }

    #[test]
    fn collector_states_have_stable_names() {
        assert_eq!(state_name(&CollectorState::Done), "done");
        assert_eq!(state_name(&CollectorState::NotInstalled), "not_installed");
        assert_eq!(state_name(&CollectorState::TimedOut), "timed_out");
    }
}
