//! User preferences and thresholds (T-02.05): defaults, allowed ranges and validation.

use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Frequency {
    Daily,
    Weekly,
    Off,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Schedule {
    pub frequency: Frequency,
    pub hour: u32,
    pub minute: u32,
}

impl Default for Schedule {
    fn default() -> Self {
        Self {
            frequency: Frequency::Daily,
            hour: 13,
            minute: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Preferences {
    pub project_inactivity_days: u32,
    pub quarantine_days: u32,
    /// Chosen in onboarding (T-02.01); off until then.
    pub auto_clean_safe: bool,
    pub schedule: Schedule,
    /// Chosen in onboarding (T-02.01); off until then.
    pub start_with_system: bool,
    pub alert_recoverable_gb: u32,
    pub alert_free_disk_percent: u32,
    pub temp_min_age_days: u32,
    pub stash_age_days: u32,
    pub downloads_age_days: u32,
    pub agent_session_age_days: u32,
    pub container_data_threshold_mb: u32,
    /// Only used when GitHub credentials already exist on the machine (T-05.03).
    pub github_lookup: bool,
    pub fetch_before_analysis: bool,
    pub defer_on_battery: bool,
    pub history_retention_days: u32,
    pub collector_timeout_minutes: u32,
    pub scan_depth: u32,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            project_inactivity_days: 30,
            quarantine_days: 14,
            auto_clean_safe: false,
            schedule: Schedule::default(),
            start_with_system: false,
            alert_recoverable_gb: 10,
            alert_free_disk_percent: 15,
            temp_min_age_days: 7,
            stash_age_days: 30,
            downloads_age_days: 30,
            agent_session_age_days: 90,
            container_data_threshold_mb: 100,
            github_lookup: true,
            fetch_before_analysis: false,
            defer_on_battery: true,
            history_retention_days: 365,
            collector_timeout_minutes: 5,
            scan_depth: 6,
        }
    }
}

/// A numeric preference outside its allowed range.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutOfRange {
    pub field: &'static str,
    pub value: u32,
    pub min: u32,
    pub max: u32,
}

impl Preferences {
    /// Every numeric preference with its allowed range (T-02.05 table).
    fn numeric(&mut self) -> [(&'static str, &mut u32, u32, u32); 14] {
        [
            (
                "project_inactivity_days",
                &mut self.project_inactivity_days,
                7,
                365,
            ),
            ("quarantine_days", &mut self.quarantine_days, 1, 90),
            ("schedule.hour", &mut self.schedule.hour, 0, 23),
            ("schedule.minute", &mut self.schedule.minute, 0, 59),
            (
                "alert_recoverable_gb",
                &mut self.alert_recoverable_gb,
                1,
                500,
            ),
            (
                "alert_free_disk_percent",
                &mut self.alert_free_disk_percent,
                5,
                50,
            ),
            ("temp_min_age_days", &mut self.temp_min_age_days, 1, 90),
            ("stash_age_days", &mut self.stash_age_days, 7, 365),
            ("downloads_age_days", &mut self.downloads_age_days, 7, 365),
            (
                "agent_session_age_days",
                &mut self.agent_session_age_days,
                14,
                730,
            ),
            (
                "container_data_threshold_mb",
                &mut self.container_data_threshold_mb,
                1,
                10_240,
            ),
            (
                "history_retention_days",
                &mut self.history_retention_days,
                30,
                3650,
            ),
            (
                "collector_timeout_minutes",
                &mut self.collector_timeout_minutes,
                1,
                60,
            ),
            ("scan_depth", &mut self.scan_depth, 2, 15),
        ]
    }

    /// Every value outside its range; empty when all are valid.
    pub fn validate(&self) -> Vec<OutOfRange> {
        let mut copy = self.clone();
        copy.numeric()
            .into_iter()
            .filter(|(_, value, min, max)| !(*min..=*max).contains(*value))
            .map(|(field, value, min, max)| OutOfRange {
                field,
                value: *value,
                min,
                max,
            })
            .collect()
    }

    /// Replaces out-of-range values with their defaults; returns the fields it fixed.
    pub fn sanitize(&mut self) -> Vec<&'static str> {
        let mut defaults = Self::default();
        let fallback: Vec<u32> = defaults.numeric().iter().map(|(_, v, _, _)| **v).collect();
        self.numeric()
            .into_iter()
            .zip(fallback)
            .filter_map(|((field, value, min, max), default)| {
                (!(min..=max).contains(value)).then(|| {
                    *value = default;
                    field
                })
            })
            .collect()
    }

    pub fn collector_timeout(&self) -> Duration {
        Duration::from_secs(u64::from(self.collector_timeout_minutes) * 60)
    }

    pub fn project_inactivity(&self) -> Duration {
        Duration::from_secs(u64::from(self.project_inactivity_days) * 24 * 60 * 60)
    }
}
