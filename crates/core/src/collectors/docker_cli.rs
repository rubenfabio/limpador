//! Reading the Docker CLI's JSON output (verified against Docker Desktop 4.76 / engine 29.5).
//! Sizes come as human text with SI units ("170.7MB" = 170.7 × 10⁶ bytes).

use serde::Deserialize;
use std::collections::BTreeMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// `docker system df -v --format json`: one object with every image, container, volume
/// and build cache record, including each image's exclusive size.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "PascalCase", default)]
pub struct DiskUsage {
    pub images: Vec<ImageUsage>,
    pub containers: Vec<ContainerUsage>,
    pub volumes: Vec<VolumeUsage>,
    pub build_cache: Vec<BuildCacheUsage>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "PascalCase", default)]
pub struct ImageUsage {
    #[serde(rename = "ID")]
    pub id: String,
    pub repository: String,
    pub tag: String,
    pub created_at: String,
    pub size: String,
    pub unique_size: String,
    /// Number of containers (running or stopped) using the image, as text.
    pub containers: String,
}

impl ImageUsage {
    pub fn is_dangling(&self) -> bool {
        self.repository == "<none>" || self.repository.is_empty()
    }

    pub fn reference(&self) -> String {
        format!("{}:{}", self.repository, self.tag)
    }

    pub fn container_count(&self) -> u64 {
        self.containers.trim().parse().unwrap_or(0)
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "PascalCase", default)]
pub struct ContainerUsage {
    #[serde(rename = "ID")]
    pub id: String,
    pub names: String,
    pub image: String,
    pub created_at: String,
    pub labels: String,
    /// "8.19kB (virtual 1.11GB)": writable layer, then the total with the image.
    pub size: String,
    /// "running", "exited", "created"…
    pub state: String,
    pub status: String,
    pub networks: String,
}

impl ContainerUsage {
    pub fn is_running(&self) -> bool {
        matches!(self.state.as_str(), "running" | "restarting" | "paused")
    }

    /// Bytes of the container's own writable layer.
    pub fn writable_layer(&self) -> u64 {
        let first = self.size.split_whitespace().next().unwrap_or("");
        parse_size(first).unwrap_or(0)
    }

    pub fn labels(&self) -> BTreeMap<String, String> {
        parse_labels(&self.labels)
    }

    /// Folder of the Compose project that created the container, if any.
    pub fn compose_working_dir(&self) -> Option<String> {
        self.labels()
            .remove("com.docker.compose.project.working_dir")
            .filter(|dir| !dir.is_empty())
    }

    /// Whether `image` (as listed by `docker image ls`) is the one this container runs.
    pub fn uses(&self, image: &ImageUsage) -> bool {
        self.image == image.id
            || self.image == image.reference()
            || (image.tag == "latest" && self.image == image.repository)
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "PascalCase", default)]
pub struct VolumeUsage {
    pub name: String,
    pub labels: String,
    pub size: String,
    /// Containers using the volume, as text.
    pub links: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "PascalCase", default)]
pub struct BuildCacheUsage {
    #[serde(rename = "ID")]
    pub id: String,
    pub size: String,
    pub last_used_at: String,
    pub in_use: bool,
}

/// One line of `docker network ls --format json`.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "PascalCase", default)]
pub struct NetworkInfo {
    #[serde(rename = "ID")]
    pub id: String,
    pub name: String,
    pub driver: String,
}

/// `docker version --format json`, only what decides support.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "PascalCase", default)]
pub struct VersionInfo {
    pub server: Option<ServerInfo>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "PascalCase", default)]
pub struct ServerInfo {
    pub platform: PlatformInfo,
    pub version: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "PascalCase", default)]
pub struct PlatformInfo {
    pub name: String,
}

/// Parses JSON Lines (one object per line), skipping blank lines.
pub fn parse_lines<T: for<'de> Deserialize<'de>>(text: &str) -> Result<Vec<T>, serde_json::Error> {
    text.lines()
        .filter(|l| !l.trim().is_empty())
        .map(serde_json::from_str)
        .collect()
}

/// "170.7MB" → 170_700_000. Docker uses SI units (kB = 1000 B).
pub fn parse_size(text: &str) -> Option<u64> {
    let text = text.trim();
    let split = text
        .find(|c: char| !(c.is_ascii_digit() || c == '.'))
        .unwrap_or(text.len());
    let (number, unit) = text.split_at(split);
    let number: f64 = number.parse().ok()?;
    let factor: f64 = match unit.trim().to_ascii_lowercase().as_str() {
        "b" | "" => 1.0,
        "kb" => 1e3,
        "mb" => 1e6,
        "gb" => 1e9,
        "tb" => 1e12,
        _ => return None,
    };
    // Sizes are non-negative and far below u64::MAX, so the cast cannot truncate badly.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    Some((number * factor).round() as u64)
}

/// "2026-05-08 18:16:10 -0300 -03" → the instant it names (time-zone aware).
pub fn parse_time(text: &str) -> Option<SystemTime> {
    let mut parts = text.split_whitespace();
    let (date, time, offset) = (parts.next()?, parts.next()?, parts.next()?);

    let mut date = date.split('-').map(str::parse::<i64>);
    let (year, month, day) = (date.next()?.ok()?, date.next()?.ok()?, date.next()?.ok()?);
    let mut time = time.split(':').map(str::parse::<i64>);
    let (hour, minute, second) = (time.next()?.ok()?, time.next()?.ok()?, time.next()?.ok()?);

    let sign = match offset.as_bytes().first()? {
        b'+' => 1,
        b'-' => -1,
        _ => return None,
    };
    let offset_hours: i64 = offset.get(1..3)?.parse().ok()?;
    let offset_minutes: i64 = offset.get(3..5)?.parse().ok()?;

    let seconds = days_from_civil(year, month, day) * 86_400 + hour * 3600 + minute * 60 + second
        - sign * (offset_hours * 3600 + offset_minutes * 60);
    u64::try_from(seconds)
        .ok()
        .map(|s| UNIX_EPOCH + Duration::from_secs(s))
}

/// Days since 1970-01-01 for a proleptic Gregorian date (H. Hinnant's algorithm).
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let year_of_era = year - era * 400;
    let month_index = (month + 9) % 12;
    let day_of_year = (153 * month_index + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

/// "a=1,b=x,y,c=" → {a: 1, b: "x,y", c: ""}. Values may contain commas (Compose writes
/// `depends_on=db:service_started:false,cache:service_started:false`), so a piece without
/// `=` belongs to the previous label.
pub fn parse_labels(text: &str) -> BTreeMap<String, String> {
    let mut labels = BTreeMap::new();
    let mut last: Option<String> = None;
    for piece in text.split(',').filter(|p| !p.is_empty()) {
        match piece.split_once('=') {
            Some((key, value)) if !key.contains(':') => {
                labels.insert(key.to_owned(), value.to_owned());
                last = Some(key.to_owned());
            }
            _ => {
                if let Some(value) = last.as_ref().and_then(|k| labels.get_mut(k)) {
                    value.push(',');
                    value.push_str(piece);
                }
            }
        }
    }
    labels
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes_use_si_units() {
        assert_eq!(parse_size("170.7MB"), Some(170_700_000));
        assert_eq!(parse_size("8.19kB"), Some(8_190));
        assert_eq!(parse_size("1.11GB"), Some(1_110_000_000));
        assert_eq!(parse_size("378B"), Some(378));
        assert_eq!(parse_size("0B"), Some(0));
        assert_eq!(parse_size("2TB"), Some(2_000_000_000_000));
        assert_eq!(parse_size("N/A"), None);
        assert_eq!(parse_size("12XB"), None);
    }

    #[test]
    fn times_take_the_offset_into_account() {
        let at = parse_time("2026-05-08 18:16:10 -0300 -03").unwrap();
        // 2026-05-08 21:16:10 UTC
        assert_eq!(at, UNIX_EPOCH + Duration::from_secs(1_778_274_970));
        let utc = parse_time("1970-01-02 00:00:00 +0000 UTC").unwrap();
        assert_eq!(utc, UNIX_EPOCH + Duration::from_secs(86_400));
        let east = parse_time("1970-01-02 01:30:00 +0130 X").unwrap();
        assert_eq!(east, UNIX_EPOCH + Duration::from_secs(86_400));
        assert_eq!(parse_time("2026-05-08"), None);
        assert_eq!(parse_time("2026-05-08 18:16:10 0300"), None);
    }

    #[test]
    fn leap_years_and_month_boundaries_are_right() {
        assert_eq!(days_from_civil(1970, 1, 1), 0);
        assert_eq!(days_from_civil(2000, 3, 1), 11_017);
        assert_eq!(
            days_from_civil(2024, 2, 29) + 1,
            days_from_civil(2024, 3, 1)
        );
        assert_eq!(days_from_civil(1969, 12, 31), -1);
    }

    #[test]
    fn labels_keep_values_that_contain_commas() {
        let labels = parse_labels(
            "com.docker.compose.depends_on=db:service_started:false,cache:service_started:false,\
             com.docker.compose.project.working_dir=C:\\Projetos\\exemplo\\infra,vazio=",
        );
        assert_eq!(
            labels["com.docker.compose.depends_on"],
            "db:service_started:false,cache:service_started:false"
        );
        assert_eq!(
            labels["com.docker.compose.project.working_dir"],
            "C:\\Projetos\\exemplo\\infra"
        );
        assert_eq!(labels["vazio"], "");
        assert!(parse_labels("").is_empty());
        let continued = parse_labels("depends_on=db:service_healthy:true,cache:mode=x,a=1");
        assert_eq!(
            continued["depends_on"], "db:service_healthy:true,cache:mode=x",
            "a piece whose key has ':' continues the previous value"
        );
        assert_eq!(continued.len(), 2);
    }

    #[test]
    fn containers_expose_layer_size_state_and_compose_folder() {
        let container = ContainerUsage {
            size: "8.19kB (virtual 1.11GB)".into(),
            state: "exited".into(),
            labels: "com.docker.compose.project.working_dir=C:\\p\\infra".into(),
            image: "redis:7-alpine".into(),
            ..ContainerUsage::default()
        };
        assert_eq!(container.writable_layer(), 8_190);
        assert!(!container.is_running());
        assert_eq!(
            container.compose_working_dir().as_deref(),
            Some("C:\\p\\infra")
        );
        for state in ["running", "restarting", "paused"] {
            let c = ContainerUsage {
                state: state.into(),
                ..ContainerUsage::default()
            };
            assert!(c.is_running(), "{state}");
        }
        let image = ImageUsage {
            id: "sha256:abc".into(),
            repository: "redis".into(),
            tag: "7-alpine".into(),
            ..ImageUsage::default()
        };
        assert!(container.uses(&image));
        let by_id = ContainerUsage {
            image: "sha256:abc".into(),
            ..ContainerUsage::default()
        };
        assert!(by_id.uses(&image));
        let latest = ImageUsage {
            repository: "nginx".into(),
            tag: "latest".into(),
            ..image.clone()
        };
        let short = ContainerUsage {
            image: "nginx".into(),
            ..ContainerUsage::default()
        };
        assert!(short.uses(&latest));
        assert!(!short.uses(&image));
        let short_redis = ContainerUsage {
            image: "redis".into(),
            ..ContainerUsage::default()
        };
        assert!(
            !short_redis.uses(&image),
            "redis means redis:latest, not 7-alpine"
        );
    }

    #[test]
    fn images_know_when_they_are_dangling_and_used() {
        let dangling = ImageUsage {
            repository: "<none>".into(),
            containers: "0".into(),
            ..ImageUsage::default()
        };
        assert!(dangling.is_dangling());
        assert_eq!(dangling.container_count(), 0);
        let used = ImageUsage {
            repository: "redis".into(),
            containers: "2".into(),
            ..ImageUsage::default()
        };
        assert!(!used.is_dangling());
        assert_eq!(used.container_count(), 2);
    }

    #[test]
    fn json_lines_skip_blank_lines() {
        let networks: Vec<NetworkInfo> =
            parse_lines("{\"ID\":\"1\",\"Name\":\"bridge\",\"Driver\":\"bridge\"}\n\n").unwrap();
        assert_eq!(networks.len(), 1);
        assert_eq!(networks[0].name, "bridge");
    }
}
