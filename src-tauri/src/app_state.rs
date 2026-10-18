//! What the commands work on: configuration, the analyzer and the folders the app uses.
//! Kept free of Tauri types so it can be tested directly.

use crate::view::{AnalysisView, DriveView, ItemsEvent, analysis_view, items_event};
use limpador_core::analysis::Analyzer;
use limpador_core::collector::Registry;
use limpador_core::collectors::docker::{DockerCollector, DockerSettings, start_desktop};
use limpador_core::collectors::wsl_disks::{WslDisksCollector, docker_data_disk};
use limpador_core::config::{Config, ConfigStore};
use limpador_core::disks::drives;
use limpador_core::engine::{Engine, EngineConfig, EventFn};
use limpador_core::message::Message;
use limpador_core::onboarding::{self, OnboardingChoices, RootSuggestion};
use limpador_core::ports::{CommandRunner, SystemClock};
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
    /// `%LOCALAPPDATA%`, where Docker Desktop and WSL keep their virtual disks.
    pub local_app_data: PathBuf,
}

impl Paths {
    pub fn real(config_dir: PathBuf, home: PathBuf, local_app_data: PathBuf) -> Self {
        Self {
            config_dir,
            home,
            projetos: PathBuf::from("C:\\Projetos"),
            local_app_data,
        }
    }

    pub fn sandbox(root: &Path) -> Self {
        Self {
            config_dir: root.join("app-data").join("config"),
            home: root.join("home"),
            projetos: root.join("Projetos"),
            local_app_data: root.join("local-app-data"),
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

/// Receives the items of each collector that finishes during an analysis.
pub type ItemsFn = Arc<dyn Fn(ItemsEvent) + Send + Sync>;

pub struct AppState {
    paths: Paths,
    store: ConfigStore,
    config: Mutex<Config>,
    /// Problems found while loading the configuration (shown once in the UI).
    pub load_warnings: Vec<Message>,
    analyzer: Analyzer,
    /// Last complete analysis, for screens opened later (detail, views).
    last_view: Mutex<Option<AnalysisView>>,
}

impl AppState {
    /// `commands` runs git, docker and wsl: the real ones, or the fake machine's in sandbox.
    pub fn load(paths: Paths, commands: Arc<dyn CommandRunner>) -> Self {
        let store = ConfigStore::new(&paths.config_dir);
        let (config, load_warnings) = store.load();
        let engine = Engine::new(registry(&config, &paths, &commands), engine_config(&config));
        Self {
            paths,
            store,
            config: Mutex::new(config),
            load_warnings,
            analyzer: Analyzer {
                engine,
                clock: Arc::new(SystemClock),
                commands,
            },
            last_view: Mutex::new(None),
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

    /// Full analysis for the interface. `on_event` receives each collector's progress and
    /// `on_items` its items as soon as it finishes.
    pub fn analyze(&self, on_event: EventFn, on_items: ItemsFn) -> AnalysisView {
        let config = self.config.lock().unwrap().clone();
        let report = self.analyzer.analyze_streaming(
            &config,
            on_event,
            Arc::new(move |id, items| on_items(items_event(id, &items))),
        );
        let mut view = analysis_view(&report);
        let mut folders = config.roots.paths.clone();
        folders.push(self.paths.local_app_data.clone());
        view.drives = drives(&folders).iter().map(DriveView::from).collect();
        if !view.cancelled {
            *self.last_view.lock().unwrap() = Some(view.clone());
        }
        view
    }

    pub fn last_analysis(&self) -> Option<AnalysisView> {
        self.last_view.lock().unwrap().clone()
    }

    /// Starts Docker Desktop when the analysis found it stopped (T-08.01).
    pub fn start_docker(&self) -> Result<(), Message> {
        start_desktop(self.analyzer.commands.as_ref())
    }

    /// Stops the running analysis, if any (T-01.05: within 2 s).
    // motivo: só repassa ao Engine::cancel, coberto pelos testes do motor (T-01.05).
    #[cfg_attr(test, mutants::skip)]
    pub fn cancel_analysis(&self) {
        self.analyzer.engine.cancel();
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

/// Every collector the app runs, configured from the preferences and the folders.
pub fn registry(config: &Config, paths: &Paths, commands: &Arc<dyn CommandRunner>) -> Registry {
    let docker = DockerCollector::new(Arc::clone(commands), docker_settings(config));
    let disks = WslDisksCollector::new(
        Arc::clone(commands),
        docker_data_disk(&paths.local_app_data),
    );
    let mut registry = Registry::default();
    registry
        .register(Arc::new(docker))
        .expect("docker collector registers");
    registry
        .register(Arc::new(disks))
        .expect("wsl-disks collector registers");
    registry
}

/// Docker thresholds that come from the user's preferences.
pub fn docker_settings(config: &Config) -> DockerSettings {
    let prefs = &config.preferences;
    DockerSettings {
        inactivity: prefs.project_inactivity(),
        container_data_threshold: u64::from(prefs.container_data_threshold_mb) * 1_000_000,
        ..DockerSettings::default()
    }
}

/// Engine settings that come from the user's preferences (T-02.05).
pub fn engine_config(config: &Config) -> EngineConfig {
    EngineConfig {
        collector_timeout: config.preferences.collector_timeout(),
        ..EngineConfig::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sandbox_commands::SandboxCommands;
    use std::fs;

    fn sandbox() -> (tempfile::TempDir, AppState) {
        let dir = tempfile::tempdir().unwrap();
        for project in ["api", "web"] {
            let folder = dir.path().join("Projetos").join(project);
            fs::create_dir_all(&folder).unwrap();
            fs::write(folder.join("package.json"), "{}").unwrap();
        }
        let state = AppState::load(
            Paths::sandbox(dir.path()),
            Arc::new(SandboxCommands::new(dir.path())),
        );
        (dir, state)
    }

    #[test]
    fn docker_thresholds_follow_the_preferences() {
        let mut config = Config::default();
        config.preferences.project_inactivity_days = 45;
        config.preferences.container_data_threshold_mb = 250;
        let settings = docker_settings(&config);
        assert_eq!(settings.inactivity, Duration::from_secs(45 * 24 * 60 * 60));
        assert_eq!(settings.container_data_threshold, 250_000_000);
        assert_eq!(
            settings.build_cache_age,
            DockerSettings::default().build_cache_age
        );
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
        let reloaded = AppState::load(
            Paths::sandbox(dir.path()),
            Arc::new(SandboxCommands::new(dir.path())),
        );
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
        let reloaded = AppState::load(
            Paths::sandbox(dir.path()),
            Arc::new(SandboxCommands::new(dir.path())),
        );
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
        let view = state.analyze(Arc::new(|_, _| {}), Arc::new(|_| {}));
        assert_eq!(view.projects.len(), 2);
        assert_eq!(view.totals.recoverable, 0);
        assert!(view.warnings.is_empty());
        assert_eq!(
            view.collectors
                .iter()
                .map(|c| (c.id.as_str(), c.state))
                .collect::<Vec<_>>(),
            vec![("docker", "not_installed"), ("wsl-disks", "not_installed")],
            "the fake machine has no Docker or WSL unless a scenario scripts them"
        );
    }

    #[test]
    fn starting_docker_runs_docker_desktop_start() {
        let (dir, state) = sandbox();
        assert_eq!(
            state.start_docker().unwrap_err().key,
            "docker.start_failed",
            "no Docker on the fake machine"
        );
        let commands = dir.path().join("commands");
        fs::create_dir_all(&commands).unwrap();
        let answer = SandboxCommands::file_name("docker", &["desktop", "start"]);
        fs::write(commands.join(format!("{answer}.out")), "").unwrap();
        assert_eq!(state.start_docker(), Ok(()));
    }

    #[test]
    fn analyze_reports_progress_drives_and_keeps_the_last_result() {
        let (dir, state) = sandbox();
        state
            .complete_onboarding(Choices {
                roots: vec![dir.path().join("Projetos").display().to_string()],
                auto_clean_safe: false,
                start_with_system: false,
            })
            .unwrap();
        assert_eq!(state.last_analysis(), None);

        let events = Arc::new(Mutex::new(Vec::new()));
        let sink = Arc::clone(&events);
        let batches = Arc::new(Mutex::new(Vec::new()));
        let batch_sink = Arc::clone(&batches);
        let view = state.analyze(
            Arc::new(move |id, s| {
                sink.lock()
                    .unwrap()
                    .push((id.to_owned(), crate::view::progress_event(id, s)));
            }),
            Arc::new(move |event| batch_sink.lock().unwrap().push(event.collector)),
        );

        assert_eq!(view.projects.len(), 2);
        assert_eq!(view.drives.len(), 1, "the temp folder's drive");
        assert!(view.drives[0].total > 0);
        let events = events.lock().unwrap();
        assert!(
            events
                .iter()
                .any(|(id, e)| id == "docker" && e.state == "not_installed")
        );
        let mut batches = batches.lock().unwrap().clone();
        batches.sort();
        assert_eq!(
            batches,
            vec!["docker", "wsl-disks"],
            "one batch per finished collector"
        );
        assert_eq!(state.last_analysis(), Some(view));
        state.cancel_analysis(); // nothing running: no effect
    }

    #[test]
    fn real_paths_suggest_c_projetos_first() {
        let paths = Paths::real(
            "C:\\cfg".into(),
            "C:\\Users\\dev".into(),
            "C:\\local".into(),
        );
        assert_eq!(paths.projetos, PathBuf::from("C:\\Projetos"));
        assert_eq!(paths.local_app_data, PathBuf::from("C:\\local"));
    }

    #[test]
    fn the_app_runs_the_docker_and_virtual_disk_collectors() {
        let dir = tempfile::tempdir().unwrap();
        let commands: Arc<dyn CommandRunner> = Arc::new(SandboxCommands::new(dir.path()));
        let registry = registry(&Config::default(), &Paths::sandbox(dir.path()), &commands);
        let ids: Vec<&str> = registry
            .collectors()
            .iter()
            .map(|c| c.descriptor().id)
            .collect();
        assert_eq!(ids, vec!["docker", "wsl-disks"]);
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
}
