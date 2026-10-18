//! User configuration (preferences, roots, protections), stored as TOML (ADR-0018).

mod preferences;

pub use preferences::{Frequency, OutOfRange, Preferences, Schedule};

use serde::{Deserialize, Serialize};
use std::io;
use std::path::PathBuf;

/// Folders analyzed for projects and paths/patterns left out (T-02.02).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Roots {
    pub paths: Vec<PathBuf>,
    /// Absolute paths or glob patterns (e.g. `**/vendor`).
    pub exclusions: Vec<String>,
}

/// Rules that win over everything else (T-02.04, I10).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Protections {
    /// Always active, never "parado".
    pub pinned_projects: Vec<PathBuf>,
    /// Nothing inside is ever cleanable.
    pub protected_paths: Vec<PathBuf>,
    pub protected_branches: Vec<String>,
    /// Image and volume name patterns.
    pub protected_docker: Vec<String>,
}

impl Default for Protections {
    fn default() -> Self {
        Self {
            pinned_projects: Vec::new(),
            protected_paths: Vec::new(),
            protected_branches: ["main", "master", "develop", "dev", "release/*", "hotfix/*"]
                .map(String::from)
                .to_vec(),
            protected_docker: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub onboarding_done: bool,
    pub preferences: Preferences,
    pub roots: Roots,
    pub protections: Protections,
}

impl Config {
    /// "Restaurar padrões": preferences only; roots and protections stay (T-02.05).
    pub fn reset_preferences(&mut self) {
        self.preferences = Preferences::default();
    }
}

#[derive(Debug)]
pub enum ConfigError {
    Invalid(Vec<OutOfRange>),
    Io(io::Error),
}

#[cfg(test)]
// Tests tweak one field of a default value: clearer than struct-update syntax here.
#[allow(clippy::field_reassign_with_default)]
mod tests {
    use super::*;
    use crate::adapters::TomlConfigRepository;
    use crate::message::Message;
    use crate::ports::ConfigRepository;
    use std::fs;
    use std::time::Duration;

    fn store() -> (tempfile::TempDir, TomlConfigRepository) {
        let dir = tempfile::tempdir().unwrap();
        let store = TomlConfigRepository::new(dir.path());
        (dir, store)
    }

    #[test]
    fn t02_05_ca1_new_install_uses_every_default() {
        let (_dir, store) = store();
        let (config, warnings) = store.load();
        let p = &config.preferences;

        assert!(warnings.is_empty());
        assert_eq!(p.project_inactivity_days, 30);
        assert_eq!(p.quarantine_days, 14);
        assert_eq!(
            p.schedule,
            Schedule {
                frequency: Frequency::Daily,
                hour: 13,
                minute: 0
            }
        );
        assert_eq!(p.alert_recoverable_gb, 10);
        assert_eq!(p.alert_free_disk_percent, 15);
        assert_eq!(p.temp_min_age_days, 7);
        assert_eq!(p.stash_age_days, 30);
        assert_eq!(p.downloads_age_days, 30);
        assert_eq!(p.agent_session_age_days, 90);
        assert_eq!(p.container_data_threshold_mb, 100);
        assert!(p.github_lookup && p.defer_on_battery);
        assert!(!p.fetch_before_analysis && !p.auto_clean_safe && !p.start_with_system);
        assert_eq!(p.history_retention_days, 365);
        assert_eq!(p.collector_timeout_minutes, 5);
        assert_eq!(p.scan_depth, 6);
        assert!(p.validate().is_empty());
        assert!(!config.onboarding_done);
    }

    #[test]
    fn t02_05_ca2_zero_quarantine_days_is_refused() {
        let (_dir, store) = store();
        let mut config = Config::default();
        config.preferences.quarantine_days = 0;

        match store.save(&config) {
            Err(ConfigError::Invalid(problems)) => assert_eq!(
                problems,
                vec![OutOfRange {
                    field: "quarantine_days",
                    value: 0,
                    min: 1,
                    max: 90
                }]
            ),
            other => panic!("expected refusal, got {other:?}"),
        }
        assert!(!store.path().exists());
    }

    #[test]
    fn every_range_limit_is_inclusive() {
        let mut p = Preferences::default();
        p.project_inactivity_days = 7;
        p.scan_depth = 15;
        p.schedule.hour = 23;
        assert!(p.validate().is_empty());
        p.project_inactivity_days = 6;
        p.scan_depth = 16;
        p.schedule.hour = 24;
        let fields: Vec<_> = p.validate().into_iter().map(|o| o.field).collect();
        assert_eq!(
            fields,
            vec!["project_inactivity_days", "schedule.hour", "scan_depth"]
        );
    }

    #[test]
    fn t02_05_ca3_changed_values_drive_the_next_analysis() {
        let mut p = Preferences::default();
        p.project_inactivity_days = 60;
        p.collector_timeout_minutes = 2;
        assert_eq!(p.project_inactivity(), Duration::from_secs(60 * 24 * 3600));
        assert_eq!(p.collector_timeout(), Duration::from_secs(120));
    }

    #[test]
    fn t02_05_ca4_restoring_defaults_keeps_roots_and_protections() {
        let mut config = Config::default();
        config.preferences.quarantine_days = 30;
        config.roots.paths.push("C:\\Projetos".into());
        config
            .protections
            .protected_paths
            .push("C:\\Projetos\\tcc".into());

        config.reset_preferences();

        assert_eq!(config.preferences, Preferences::default());
        assert_eq!(config.roots.paths, vec![PathBuf::from("C:\\Projetos")]);
        assert_eq!(
            config.protections.protected_paths,
            vec![PathBuf::from("C:\\Projetos\\tcc")]
        );
    }

    #[test]
    fn t02_05_ca5_saved_values_survive_a_restart() {
        let (_dir, store) = store();
        let mut config = Config::default();
        config.onboarding_done = true;
        config.preferences.project_inactivity_days = 45;
        config.preferences.schedule.frequency = Frequency::Weekly;
        config.roots.exclusions.push("**/vendor".into());
        store.save(&config).unwrap();

        let (loaded, warnings) = TomlConfigRepository::new(store.path().parent().unwrap()).load();
        assert!(warnings.is_empty());
        assert_eq!(loaded, config);
    }

    #[test]
    fn missing_fields_take_defaults_and_bad_values_are_reset_with_a_warning() {
        let (_dir, store) = store();
        fs::write(
            store.path(),
            "[preferences]\nquarantine_days = 500\nscan_depth = 8\n",
        )
        .unwrap();

        let (config, warnings) = store.load();
        assert_eq!(config.preferences.quarantine_days, 14);
        assert_eq!(config.preferences.scan_depth, 8);
        assert_eq!(config.preferences.project_inactivity_days, 30);
        assert_eq!(
            warnings,
            vec![Message::new("config.value_reset").with("field", "quarantine_days")]
        );
    }

    #[test]
    fn unreadable_file_falls_back_to_defaults_with_a_warning() {
        let (_dir, store) = store();
        fs::create_dir(store.path()).unwrap(); // a folder where the file should be
        let (config, warnings) = store.load();
        assert_eq!(config, Config::default());
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].key, "config.unreadable");
    }

    #[test]
    fn broken_file_falls_back_to_defaults_with_a_warning() {
        let (_dir, store) = store();
        fs::write(store.path(), "isto não é toml = = =").unwrap();
        let (config, warnings) = store.load();
        assert_eq!(config, Config::default());
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].key, "config.invalid_file");
    }

    #[test]
    fn saving_creates_the_folder_and_leaves_no_temporary_file() {
        let dir = tempfile::tempdir().unwrap();
        let store = TomlConfigRepository::new(&dir.path().join("novo"));
        store.save(&Config::default()).unwrap();
        assert!(store.path().exists());
        assert!(!store.path().with_extension("toml.tmp").exists());
    }

    #[test]
    fn default_protected_branches_follow_the_task() {
        assert_eq!(
            Protections::default().protected_branches,
            vec!["main", "master", "develop", "dev", "release/*", "hotfix/*"]
        );
    }
}
