//! Docker collector (F03): images, stopped containers, build cache and networks.
//! Volumes and containers holding data need the quarantine (F09) and are not offered yet.

use super::docker_cli::{
    DiskUsage, ImageUsage, NetworkInfo, VersionInfo, parse_lines, parse_size, parse_time,
};
use crate::collector::{
    AnalysisContext, CleanOutcome, Collector, CollectorDescriptor, CollectorError, Detection,
    KindSpec, Revalidation,
};
use crate::item::{Category, CleanAction, Item, ItemDraft, Nature, Resource, Risk};
use crate::message::Message;
use crate::ports::CommandRunner;
use crate::risk::Destination;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

pub const ID: &str = "docker";
const DAY: Duration = Duration::from_secs(86_400);

/// Thresholds from the preferences (T-02.05).
#[derive(Debug, Clone)]
pub struct DockerSettings {
    /// Unused tagged images and stopped containers older than this are candidates.
    pub inactivity: Duration,
    /// Containers whose writable layer reaches this hold data (🔴, need quarantine).
    pub container_data_threshold: u64,
    /// Build cache used more recently than this is kept.
    pub build_cache_age: Duration,
}

impl Default for DockerSettings {
    fn default() -> Self {
        Self {
            inactivity: 30 * DAY,
            container_data_threshold: 100_000_000,
            build_cache_age: 7 * DAY,
        }
    }
}

pub struct DockerCollector {
    commands: Arc<dyn CommandRunner>,
    settings: DockerSettings,
}

impl DockerCollector {
    pub fn new(commands: Arc<dyn CommandRunner>, settings: DockerSettings) -> Self {
        Self { commands, settings }
    }

    fn docker(&self, args: &[&str]) -> Result<String, CollectorError> {
        let out = self
            .commands
            .run("docker", args)
            .map_err(|e| failed(args, &e.to_string()))?;
        if out.success() {
            Ok(out.stdout)
        } else {
            Err(failed(args, out.stderr.lines().next().unwrap_or("")))
        }
    }

    fn usage(&self) -> Result<DiskUsage, CollectorError> {
        let text = self.docker(&["system", "df", "-v", "--format", "json"])?;
        serde_json::from_str(&text).map_err(|e| invalid_output("system df", &e))
    }

    fn networks(&self) -> Result<Vec<NetworkInfo>, CollectorError> {
        let text = self.docker(&["network", "ls", "--no-trunc", "--format", "json"])?;
        parse_lines(&text).map_err(|e| invalid_output("network ls", &e))
    }

    /// Every candidate item for the current Docker state.
    fn items(&self, usage: &DiskUsage, networks: &[NetworkInfo], now: SystemTime) -> Vec<Item> {
        let mut items = Vec::new();
        items.extend(
            usage
                .images
                .iter()
                .filter_map(|i| self.image_item(i, usage, now)),
        );
        items.extend(
            usage
                .containers
                .iter()
                .filter_map(|c| self.container_item(c, now)),
        );
        items.extend(self.build_cache_item(usage, now));
        items.extend(unused_networks(networks, usage).map(network_item));
        items
    }

    fn image_item(&self, image: &ImageUsage, usage: &DiskUsage, now: SystemTime) -> Option<Item> {
        let users: Vec<_> = usage.containers.iter().filter(|c| c.uses(image)).collect();
        let created = parse_time(&image.created_at);
        let apparent = parse_size(&image.size).unwrap_or(0);
        let unique = parse_size(&image.unique_size)
            .unwrap_or(apparent)
            .min(apparent);
        let name = if image.is_dangling() {
            format!("<none> ({})", short_id(&image.id))
        } else {
            image.reference()
        };
        let draft = |risk| {
            ItemDraft::new(
                ID,
                "image",
                Category::Docker,
                Resource::Name(image.id.clone()),
                &name,
                Nature::Recreatable,
                risk,
            )
            .sizes(apparent, unique)
        };

        if let Some(running) = users.iter().find(|c| c.is_running()) {
            return draft(Risk::Blocked)
                .reason(Message::new("reason.image_in_use").with("image", &name))
                .block_reason(
                    Message::new("block.image_used_by_running").with("container", &running.names),
                )
                .recreate(Message::new("recreate.image_pull").with("image", &name))
                .build()
                .ok();
        }
        if !users.is_empty() || image.container_count() > 0 {
            return None; // freed when its stopped containers are removed
        }
        let (risk, reason, recreate) = if image.is_dangling() {
            (
                Risk::Safe,
                Message::new("reason.image_dangling"),
                Message::new("recreate.image_rebuild"),
            )
        } else {
            let age = created.and_then(|c| now.duration_since(c).ok())?;
            if age <= self.settings.inactivity {
                return None;
            }
            (
                Risk::Moderate,
                Message::new("reason.image_unused")
                    .with("image", &name)
                    .with("days", age.as_secs() / 86_400),
                Message::new("recreate.image_pull").with("image", &name),
            )
        };
        let mut draft = draft(risk).reason(reason).recreate(recreate);
        if let Some(at) = created {
            draft = draft.last_activity(at);
        }
        draft.build().ok()
    }

    fn container_item(
        &self,
        container: &super::docker_cli::ContainerUsage,
        now: SystemTime,
    ) -> Option<Item> {
        if container.is_running() {
            return None;
        }
        let layer = container.writable_layer();
        if layer >= self.settings.container_data_threshold {
            return None; // holds data: needs the quarantine (T-03.03 with F09)
        }
        let created = parse_time(&container.created_at);
        let hint = container.compose_working_dir();
        let old_enough = created
            .and_then(|c| now.duration_since(c).ok())
            .is_some_and(|age| age > self.settings.inactivity);
        // A Compose container follows its project's activity; others need age alone.
        if hint.is_none() && !old_enough {
            return None;
        }
        let recreate = if hint.is_some() {
            Message::new("recreate.container_compose")
        } else {
            Message::new("recreate.container_run").with("image", &container.image)
        };
        let mut draft = ItemDraft::new(
            ID,
            "container",
            Category::Docker,
            Resource::Name(container.id.clone()),
            &container.names,
            Nature::Recreatable,
            Risk::Moderate,
        )
        .sizes(layer, layer)
        .reason(
            Message::new("reason.container_stopped")
                .with("name", &container.names)
                .with("image", &container.image)
                .with("status", &container.status),
        )
        .recreate(recreate);
        if let Some(dir) = hint {
            draft = draft.project_hint(dir);
        }
        if let Some(at) = created {
            draft = draft.last_activity(at);
        }
        draft.build().ok()
    }

    fn build_cache_item(&self, usage: &DiskUsage, now: SystemTime) -> Option<Item> {
        let (mut old, mut recent) = (0u64, 0u64);
        for record in &usage.build_cache {
            let size = parse_size(&record.size).unwrap_or(0);
            let age = parse_time(&record.last_used_at).and_then(|t| now.duration_since(t).ok());
            if !record.in_use && age.is_some_and(|a| a > self.settings.build_cache_age) {
                old += size;
            } else {
                recent += size;
            }
        }
        if old == 0 {
            return None;
        }
        ItemDraft::new(
            ID,
            "build_cache",
            Category::Docker,
            Resource::Name("build-cache".into()),
            "build cache",
            Nature::Recreatable,
            Risk::Safe,
        )
        .sizes(old, old)
        .action(CleanAction::ManagerCommand {
            program: "docker".into(),
            args: prune_cache_args(self.settings.build_cache_age),
        })
        .reason(Message::new("reason.build_cache").with("recent", recent))
        .recreate(Message::new("recreate.build_cache"))
        .build()
        .ok()
    }

    fn rerun_until(&self, item: &Item, now: SystemTime) -> Result<Option<Item>, CollectorError> {
        let usage = self.usage()?;
        let networks = if item.kind == "network" {
            self.networks()?
        } else {
            vec![]
        };
        Ok(self
            .items(&usage, &networks, now)
            .into_iter()
            .find(|candidate| candidate.id == item.id))
    }
}

fn prune_cache_args(age: Duration) -> Vec<String> {
    ["builder", "prune", "-f", "--filter"]
        .map(String::from)
        .into_iter()
        .chain([format!("until={}h", age.as_secs() / 3600)])
        .collect()
}

const DEFAULT_NETWORKS: [&str; 3] = ["bridge", "host", "none"];

fn unused_networks<'a>(
    networks: &'a [NetworkInfo],
    usage: &'a DiskUsage,
) -> impl Iterator<Item = &'a NetworkInfo> {
    networks.iter().filter(move |network| {
        !DEFAULT_NETWORKS.contains(&network.name.as_str())
            && !usage
                .containers
                .iter()
                .any(|c| c.networks.split(',').any(|n| n == network.name))
    })
}

fn network_item(network: &NetworkInfo) -> Item {
    ItemDraft::new(
        ID,
        "network",
        Category::Docker,
        Resource::Name(network.id.clone()),
        &network.name,
        Nature::Recreatable,
        Risk::Safe,
    )
    .reason(Message::new("reason.network_unused").with("name", &network.name))
    .recreate(Message::new("recreate.network_compose"))
    .build()
    .expect("network item has every required field")
}

fn short_id(id: &str) -> &str {
    let id = id.strip_prefix("sha256:").unwrap_or(id);
    &id[..id.len().min(12)]
}

fn failed(args: &[&str], detail: &str) -> CollectorError {
    CollectorError::Failed(
        Message::new("docker.command_failed")
            .with("command", format!("docker {}", args.join(" ")))
            .with("detail", detail),
    )
}

fn invalid_output(command: &str, error: &serde_json::Error) -> CollectorError {
    CollectorError::Failed(
        Message::new("docker.invalid_output")
            .with("command", command)
            .with("detail", error),
    )
}

impl Collector for DockerCollector {
    fn descriptor(&self) -> CollectorDescriptor {
        let kind = |kind| KindSpec {
            kind,
            category: Category::Docker,
            nature: Nature::Recreatable,
            restorable: false,
        };
        CollectorDescriptor {
            id: ID,
            kinds: vec![
                kind("image"),
                kind("container"),
                kind("build_cache"),
                kind("network"),
            ],
            catalog_locations: vec![],
        }
    }

    fn detect(&self, _ctx: &AnalysisContext) -> Detection {
        let Ok(out) = self
            .commands
            .run("docker", &["version", "--format", "json"])
        else {
            return Detection::NotInstalled;
        };
        let version: VersionInfo = serde_json::from_str(&out.stdout).unwrap_or_default();
        match version.server {
            Some(server) if server.platform.name.contains("Docker Desktop") => Detection::Available,
            Some(server) => Detection::Unavailable {
                reason: Message::new("docker.unsupported_engine")
                    .with("engine", server.platform.name),
                action: None,
            },
            None => Detection::Unavailable {
                reason: Message::new("docker.stopped"),
                action: Some(Message::new("docker.start")),
            },
        }
    }

    fn analyze(
        &self,
        ctx: &AnalysisContext,
        emit: &mut dyn FnMut(Item),
    ) -> Result<(), CollectorError> {
        let usage = self.usage()?;
        ctx.report(1, Some(2));
        let networks = self.networks()?;
        for item in self.items(&usage, &networks, ctx.clock.now()) {
            emit(item);
        }
        ctx.report(2, Some(2));
        Ok(())
    }

    fn revalidate(&self, ctx: &AnalysisContext, item: &Item) -> Revalidation {
        match self.rerun_until(item, ctx.clock.now()) {
            Ok(Some(current)) if current.risk == Risk::Blocked => Revalidation::Changed(
                current
                    .block_reason
                    .unwrap_or_else(|| Message::new("docker.changed")),
            ),
            Ok(Some(current)) => Revalidation::StillValid(Box::new(current)),
            Ok(None) => Revalidation::Gone,
            Err(CollectorError::Failed(message)) => Revalidation::Changed(message),
            Err(CollectorError::Unsupported) => {
                Revalidation::Changed(Message::new("docker.changed"))
            }
        }
    }

    fn clean(
        &self,
        item: &Item,
        _destination: Destination,
    ) -> Result<CleanOutcome, CollectorError> {
        let Resource::Name(id) = &item.resource else {
            return Err(CollectorError::Unsupported);
        };
        match item.kind.as_str() {
            "image" => self.docker(&["image", "rm", id])?,
            // Without -v: named volumes stay (T-03.03 CA5).
            "container" => self.docker(&["container", "rm", id])?,
            "network" => self.docker(&["network", "rm", id])?,
            "build_cache" => {
                let args = prune_cache_args(self.settings.build_cache_age);
                self.docker(&args.iter().map(String::as_str).collect::<Vec<_>>())?
            }
            _ => return Err(CollectorError::Unsupported),
        };
        Ok(CleanOutcome::Cleaned {
            freed_bytes: item.freeable_size,
        })
    }
}

/// Starts Docker Desktop: the action offered when the engine is stopped (T-08.01).
/// Waits until the engine answers, so a new analysis can follow right away.
pub fn start_desktop(commands: &dyn CommandRunner) -> Result<(), Message> {
    let failed = |detail: &str| Message::new("docker.start_failed").with("detail", detail.trim());
    match commands.run("docker", &["desktop", "start"]) {
        Ok(output) if output.status == 0 => Ok(()),
        Ok(output) => Err(failed(&output.stderr)),
        Err(error) => Err(failed(&error.to_string())),
    }
}

#[cfg(test)]
#[path = "docker_tests.rs"]
mod tests;
