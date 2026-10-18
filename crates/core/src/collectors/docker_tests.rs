use super::*;
use crate::collector::{StopSignal, tests::context};
use crate::collectors::scripted::Scripted;
use crate::risk::{Destination, assess};
use std::time::UNIX_EPOCH;

/// 2026-10-17 12:00 UTC, the "now" of these scenarios.
const NOW: u64 = 1_792_238_400;
const VERSION_UP: &str = r#"{"Client":{"Version":"29.5.2"},"Server":{"Platform":{"Name":"Docker Desktop 4.76.0 (228118)"},"Version":"29.5.2"}}"#;
const VERSION_DOWN: &str = r#"{"Client":{"Version":"29.5.2"},"Server":null}"#;
const DF: &str = "docker system df -v --format json";
const NETWORKS: &str = "docker network ls --no-trunc --format json";

/// Same shape as Docker Desktop 4.76's output, with neutral names.
fn usage_json() -> String {
    r#"{"Images":[
      {"Containers":"0","CreatedAt":"2026-07-19 09:00:00 -0300 -03","ID":"sha256:aaa111aaa111aaa111","Repository":"node","SharedSize":"0B","Size":"1.1GB","Tag":"18","UniqueSize":"1.1GB"},
      {"Containers":"0","CreatedAt":"2026-10-10 09:00:00 -0300 -03","ID":"sha256:bbb222","Repository":"node","SharedSize":"0B","Size":"1.2GB","Tag":"20","UniqueSize":"1.2GB"},
      {"Containers":"0","CreatedAt":"2026-08-01 09:00:00 -0300 -03","ID":"sha256:ccc333ccc333ccc333","Repository":"<none>","SharedSize":"500MB","Size":"800MB","Tag":"<none>","UniqueSize":"300MB"},
      {"Containers":"1","CreatedAt":"2026-01-01 09:00:00 -0300 -03","ID":"sha256:ddd444","Repository":"postgres","SharedSize":"0B","Size":"395MB","Tag":"16-alpine","UniqueSize":"394.8MB"},
      {"Containers":"1","CreatedAt":"2026-01-01 09:00:00 -0300 -03","ID":"sha256:eee555","Repository":"redis","SharedSize":"0B","Size":"57.3MB","Tag":"7-alpine","UniqueSize":"57.33MB"}
    ],"Containers":[
      {"CreatedAt":"2026-09-01 10:00:00 -0300 -03","ID":"c1","Image":"postgres:16-alpine","Labels":"com.docker.compose.depends_on=cache:service_started:false,com.docker.compose.project.working_dir=C:\\Projetos\\exemplo\\infra","Names":"exemplo-db","Networks":"infra_default","Size":"2MB (virtual 397MB)","State":"exited","Status":"Exited (0) 6 weeks ago"},
      {"CreatedAt":"2026-10-16 10:00:00 -0300 -03","ID":"c2","Image":"redis:7-alpine","Labels":"","Names":"cache-agora","Networks":"bridge","Size":"0B (virtual 57.3MB)","State":"running","Status":"Up 2 hours"},
      {"CreatedAt":"2026-10-15 10:00:00 -0300 -03","ID":"c3","Image":"alpine","Labels":"","Names":"recente","Networks":"bridge","Size":"0B (virtual 7MB)","State":"exited","Status":"Exited (0) 2 days ago"},
      {"CreatedAt":"2026-05-01 10:00:00 -0300 -03","ID":"c4","Image":"alpine","Labels":"","Names":"com-dados","Networks":"bridge","Size":"500MB (virtual 507MB)","State":"exited","Status":"Exited (0) 5 months ago"}
    ],"Volumes":[],"BuildCache":[
      {"ID":"b1","Size":"6GB","LastUsedAt":"2026-09-01 10:00:00 -0300 -03","InUse":false},
      {"ID":"b2","Size":"2GB","LastUsedAt":"2026-10-16 10:00:00 -0300 -03","InUse":false},
      {"ID":"b3","Size":"1GB","LastUsedAt":"2026-08-01 10:00:00 -0300 -03","InUse":true}
    ]}"#
    .to_owned()
}

const NETWORK_LINES: &str = concat!(
    r#"{"ID":"n1","Name":"bridge","Driver":"bridge"}"#,
    "\n",
    r#"{"ID":"n2","Name":"infra_default","Driver":"bridge"}"#,
    "\n",
    r#"{"ID":"n3","Name":"antigo_default","Driver":"bridge"}"#,
    "\n"
);

fn running_docker() -> Arc<Scripted> {
    Arc::new(
        Scripted::default()
            .ok("docker version --format json", VERSION_UP)
            .ok(DF, &usage_json())
            .ok(NETWORKS, NETWORK_LINES),
    )
}

fn collector(commands: Arc<Scripted>) -> DockerCollector {
    DockerCollector::new(commands, DockerSettings::default())
}

fn analyze(commands: Arc<Scripted>) -> Vec<Item> {
    let docker = collector(commands);
    let mut ctx = context(StopSignal::new(None));
    ctx.clock = Arc::new(Frozen);
    let mut items = vec![];
    docker.analyze(&ctx, &mut |i| items.push(i)).unwrap();
    items
}

struct Frozen;
impl crate::ports::Clock for Frozen {
    fn now(&self) -> SystemTime {
        UNIX_EPOCH + Duration::from_secs(NOW)
    }
}

fn find<'a>(items: &'a [Item], name: &str) -> Option<&'a Item> {
    items.iter().find(|i| i.name == name)
}

#[test]
fn t03_01_ca1_stopped_docker_is_unavailable_with_a_start_action() {
    let commands = Arc::new(Scripted::default().answer(
        "docker version --format json",
        1,
        VERSION_DOWN,
        "failed to connect to the docker API at npipe:////./pipe/dockerDesktopLinuxEngine",
    ));
    let detection = collector(commands).detect(&context(StopSignal::new(None)));
    assert_eq!(
        detection,
        Detection::Unavailable {
            reason: Message::new("docker.stopped"),
            action: Some(Message::new("docker.start"))
        }
    );
}

#[test]
fn t03_01_ca2_running_docker_is_analyzed() {
    let commands = running_docker();
    assert_eq!(
        collector(Arc::clone(&commands)).detect(&context(StopSignal::new(None))),
        Detection::Available
    );
    let items = analyze(commands);
    let kinds: std::collections::BTreeSet<&str> = items.iter().map(|i| i.kind.as_str()).collect();
    assert_eq!(
        kinds.into_iter().collect::<Vec<_>>(),
        vec!["build_cache", "container", "image", "network"]
    );
}

#[test]
fn t03_01_ca3_the_analysis_never_starts_or_changes_docker() {
    let commands = running_docker();
    let docker = collector(Arc::clone(&commands));
    docker.detect(&context(StopSignal::new(None)));
    analyze(Arc::clone(&commands));
    for call in commands.calls() {
        assert!(
            call == "docker version --format json" || call == DF || call == NETWORKS,
            "unexpected command during analysis: {call}"
        );
    }
}

#[test]
fn t03_01_ca4_missing_docker_is_not_installed() {
    let detection =
        collector(Arc::new(Scripted::default())).detect(&context(StopSignal::new(None)));
    assert_eq!(detection, Detection::NotInstalled);
}

#[test]
fn other_engines_are_not_supported_yet() {
    let commands = Arc::new(Scripted::default().ok(
        "docker version --format json",
        r#"{"Server":{"Platform":{"Name":"Rancher Desktop"},"Version":"27"}}"#,
    ));
    assert_eq!(
        collector(commands).detect(&context(StopSignal::new(None))),
        Detection::Unavailable {
            reason: Message::new("docker.unsupported_engine").with("engine", "Rancher Desktop"),
            action: None
        }
    );
}

#[test]
fn t03_02_ca1_unused_dangling_image_is_safe() {
    let items = analyze(running_docker());
    let dangling = find(&items, "<none> (ccc333ccc333)").unwrap();
    assert_eq!(dangling.risk, Risk::Safe);
    assert_eq!(dangling.reason.key, "reason.image_dangling");
}

#[test]
fn t03_02_ca2_unused_image_older_than_the_threshold_is_moderate() {
    let items = analyze(running_docker());
    let node18 = find(&items, "node:18").unwrap();
    assert_eq!(node18.risk, Risk::Moderate);
    assert_eq!(
        node18.recreate,
        Some(Message::new("recreate.image_pull").with("image", "node:18"))
    );
    assert_eq!(node18.reason.params["days"], "90");
    assert!(find(&items, "node:20").is_none(), "pulled a week ago");
}

#[test]
fn t03_02_ca3_image_of_a_running_container_is_blocked() {
    let items = analyze(running_docker());
    let redis = find(&items, "redis:7-alpine").unwrap();
    assert_eq!(redis.risk, Risk::Blocked);
    assert_eq!(
        redis.block_reason,
        Some(Message::new("block.image_used_by_running").with("container", "cache-agora"))
    );
}

#[test]
fn image_used_only_by_stopped_containers_is_not_offered() {
    assert!(find(&analyze(running_docker()), "postgres:16-alpine").is_none());
}

#[test]
fn t03_02_ca4_freeable_size_excludes_shared_layers() {
    let items = analyze(running_docker());
    let dangling = find(&items, "<none> (ccc333ccc333)").unwrap();
    assert_eq!(dangling.apparent_size, 800_000_000);
    assert_eq!(dangling.freeable_size, 300_000_000);
}

#[test]
fn t03_02_ca5_failed_removal_is_reported_with_the_reason() {
    let commands = Arc::new(Scripted::default().answer(
        "docker image rm sha256:aaa111aaa111aaa111",
        1,
        "",
        "Error response from daemon: conflict: unable to remove repository reference",
    ));
    let item = find(&analyze(running_docker()), "node:18").unwrap().clone();
    let error = collector(commands)
        .clean(&item, Destination::Direct)
        .unwrap_err();
    assert_eq!(
        error,
        CollectorError::Failed(
            Message::new("docker.command_failed")
                .with("command", "docker image rm sha256:aaa111aaa111aaa111")
                .with(
                    "detail",
                    "Error response from daemon: conflict: unable to remove repository reference"
                )
        )
    );
}

#[test]
fn removed_image_reports_the_freed_space() {
    let item = find(&analyze(running_docker()), "node:18").unwrap().clone();
    let commands =
        Arc::new(Scripted::default().ok("docker image rm sha256:aaa111aaa111aaa111", "Deleted"));
    assert_eq!(
        collector(commands).clean(&item, Destination::Direct),
        Ok(CleanOutcome::Cleaned {
            freed_bytes: 1_100_000_000
        })
    );
}

#[test]
fn t03_03_ca1_old_stopped_compose_container_with_small_layer_is_moderate_and_direct() {
    let items = analyze(running_docker());
    let db = find(&items, "exemplo-db").unwrap();
    assert_eq!((db.risk, db.nature), (Risk::Moderate, Nature::Recreatable));
    assert_eq!(db.freeable_size, 2_000_000);
    assert_eq!(
        db.project_hint.as_deref(),
        Some(std::path::Path::new("C:\\Projetos\\exemplo\\infra"))
    );
    assert_eq!(assess(db, None).destination, Some(Destination::Direct));
}

#[test]
fn recent_container_without_project_is_not_offered() {
    assert!(find(&analyze(running_docker()), "recente").is_none());
}

#[test]
fn container_holding_data_waits_for_the_quarantine() {
    // T-03.03 CA2/CA4 arrive with F09; until then it is never offered for direct deletion.
    assert!(find(&analyze(running_docker()), "com-dados").is_none());
}

#[test]
fn t03_03_ca3_running_container_is_not_listed() {
    assert!(find(&analyze(running_docker()), "cache-agora").is_none());
}

#[test]
fn t03_03_ca5_container_removal_keeps_named_volumes() {
    let item = find(&analyze(running_docker()), "exemplo-db")
        .unwrap()
        .clone();
    let commands = Arc::new(Scripted::default().ok("docker container rm c1", "c1"));
    collector(Arc::clone(&commands))
        .clean(&item, Destination::Direct)
        .unwrap();
    assert_eq!(
        commands.calls(),
        vec!["docker container rm c1"],
        "no -v: volumes stay"
    );
}

#[test]
fn t03_05_ca1_only_build_cache_unused_for_a_week_counts() {
    let items = analyze(running_docker());
    let cache = find(&items, "build cache").unwrap();
    assert_eq!(cache.freeable_size, 6_000_000_000);
    assert_eq!(
        cache.reason,
        Message::new("reason.build_cache").with("recent", 3_000_000_000u64)
    );
    assert_eq!(
        cache.action,
        CleanAction::ManagerCommand {
            program: "docker".into(),
            args: ["builder", "prune", "-f", "--filter", "until=168h"]
                .map(String::from)
                .to_vec()
        }
    );
}

#[test]
fn t03_05_ca2_compose_network_without_containers_is_a_safe_organization_item() {
    let items = analyze(running_docker());
    let network = find(&items, "antigo_default").unwrap();
    assert_eq!(network.risk, Risk::Safe);
    assert!(network.is_organization());
    assert!(
        find(&items, "infra_default").is_none(),
        "still used by a container"
    );
}

#[test]
fn t03_05_ca3_default_networks_are_never_listed() {
    let items = analyze(running_docker());
    for name in ["bridge", "host", "none"] {
        assert!(find(&items, name).is_none(), "{name}");
    }
}

#[test]
fn t03_05_ca4_old_build_cache_is_eligible_for_automation() {
    let items = analyze(running_docker());
    assert!(assess(find(&items, "build cache").unwrap(), None).automation_eligible);
}

#[test]
fn cleaning_cache_and_networks_runs_their_commands() {
    let items = analyze(running_docker());
    let commands = Arc::new(
        Scripted::default()
            .ok("docker builder prune -f --filter until=168h", "")
            .ok("docker network rm n3", "n3"),
    );
    let docker = collector(Arc::clone(&commands));
    docker
        .clean(find(&items, "build cache").unwrap(), Destination::Direct)
        .unwrap();
    docker
        .clean(find(&items, "antigo_default").unwrap(), Destination::Direct)
        .unwrap();
    assert_eq!(
        commands.calls(),
        vec![
            "docker builder prune -f --filter until=168h",
            "docker network rm n3"
        ]
    );
}

#[test]
fn revalidation_sees_items_that_changed_or_disappeared() {
    let commands = running_docker();
    let items = analyze(Arc::clone(&commands));
    let docker = collector(Arc::clone(&commands));
    let mut ctx = context(StopSignal::new(None));
    ctx.clock = Arc::new(Frozen);
    let node18 = find(&items, "node:18").unwrap();
    assert!(matches!(
        docker.revalidate(&ctx, node18),
        Revalidation::StillValid(_)
    ));
    assert!(matches!(
        docker.revalidate(&ctx, find(&items, "antigo_default").unwrap()),
        Revalidation::StillValid(_)
    ));

    // Someone started a container from node:18, and the network got removed.
    let changed = usage_json()
        .replace(
            r#""ID":"c3","Image":"alpine""#,
            r#""ID":"c3","Image":"node:18""#,
        )
        .replace(
            r#""State":"exited","Status":"Exited (0) 2 days ago""#,
            r#""State":"running","Status":"Up""#,
        );
    commands.set(DF, &changed);
    commands.set(
        NETWORKS,
        &NETWORK_LINES.replace(
            r#"{"ID":"n3","Name":"antigo_default","Driver":"bridge"}"#,
            "",
        ),
    );

    assert_eq!(
        docker.revalidate(&ctx, node18),
        Revalidation::Changed(
            Message::new("block.image_used_by_running").with("container", "recente")
        )
    );
    assert_eq!(
        docker.revalidate(&ctx, find(&items, "antigo_default").unwrap()),
        Revalidation::Gone
    );
}

#[test]
fn broken_output_is_an_isolated_collector_error() {
    let commands = Arc::new(Scripted::default().ok(DF, "isto não é json"));
    let docker = collector(commands);
    let result = docker.analyze(&context(StopSignal::new(None)), &mut |_| {});
    assert!(matches!(result, Err(CollectorError::Failed(m)) if m.key == "docker.invalid_output"));
}

#[test]
fn every_item_kind_is_declared() {
    let docker = collector(running_docker());
    let mut registry = crate::collector::Registry::default();
    let items = analyze(running_docker());
    registry.register(Arc::new(docker)).unwrap();
    assert!(items.iter().all(|i| registry.declares(i)));
}

/// Adds images, containers and cache records to the standard scenario.
fn with_extra(images: &str, containers: &str, cache: &str) -> Arc<Scripted> {
    let mut usage = usage_json();
    if !images.is_empty() {
        usage = usage.replacen(r#""Images":["#, &format!(r#""Images":[{images},"#), 1);
    }
    if !containers.is_empty() {
        usage = usage.replacen(
            r#""Containers":["#,
            &format!(r#""Containers":[{containers},"#),
            1,
        );
    }
    if !cache.is_empty() {
        usage = usage.replacen(
            r#""BuildCache":["#,
            &format!(r#""BuildCache":[{cache},"#),
            1,
        );
    }
    Arc::new(
        Scripted::default()
            .ok("docker version --format json", VERSION_UP)
            .ok(DF, &usage)
            .ok(NETWORKS, NETWORK_LINES),
    )
}

#[test]
fn image_counted_as_used_is_not_offered_even_if_its_container_is_not_listed() {
    let image = r#"{"Containers":"1","CreatedAt":"2026-01-01 09:00:00 -0300 -03","ID":"sha256:fff666","Repository":"mysql","SharedSize":"0B","Size":"600MB","Tag":"8","UniqueSize":"600MB"}"#;
    assert!(find(&analyze(with_extra(image, "", "")), "mysql:8").is_none());
}

#[test]
fn container_without_project_is_offered_only_after_the_threshold() {
    // NOW is 2026-10-17 09:00 -03; exactly 30 days earlier is not "older than 30 days".
    let exact = r#"{"CreatedAt":"2026-09-17 09:00:00 -0300 -03","ID":"c8","Image":"alpine","Labels":"","Names":"limite","Networks":"bridge","Size":"0B (virtual 7MB)","State":"exited","Status":"Exited (0)"}"#;
    let older = r#"{"CreatedAt":"2026-09-17 08:59:00 -0300 -03","ID":"c9","Image":"alpine","Labels":"","Names":"passou","Networks":"bridge","Size":"0B (virtual 7MB)","State":"exited","Status":"Exited (0)"}"#;
    let items = analyze(with_extra("", &format!("{exact},{older}"), ""));
    assert!(find(&items, "limite").is_none());
    assert!(find(&items, "passou").is_some());
}

#[test]
fn recent_compose_container_follows_its_project() {
    let recent = r#"{"CreatedAt":"2026-10-16 10:00:00 -0300 -03","ID":"c7","Image":"postgres:16-alpine","Labels":"com.docker.compose.project.working_dir=C:\\Projetos\\novo","Names":"novo-db","Networks":"novo_default","Size":"1MB (virtual 396MB)","State":"exited","Status":"Exited (0) 1 day ago"}"#;
    let items = analyze(with_extra("", recent, ""));
    let db = find(&items, "novo-db").unwrap();
    assert_eq!(
        db.project_hint.as_deref(),
        Some(std::path::Path::new(r"C:\Projetos\novo"))
    );
}

#[test]
fn build_cache_used_exactly_a_week_ago_is_kept() {
    let exact =
        r#"{"ID":"b9","Size":"4GB","LastUsedAt":"2026-10-10 09:00:00 -0300 -03","InUse":false}"#;
    let cache = find(&analyze(with_extra("", "", exact)), "build cache")
        .unwrap()
        .clone();
    assert_eq!(
        cache.freeable_size, 6_000_000_000,
        "the 4 GB used exactly 7 days ago stay"
    );
    assert_eq!(cache.reason.params["recent"], "7000000000");
}

#[test]
fn starting_docker_desktop_reports_failures() {
    let started = Scripted::default().ok("docker desktop start", "");
    assert_eq!(start_desktop(&started), Ok(()));
    assert_eq!(started.calls(), vec!["docker desktop start"]);

    let refused = Scripted::default().answer("docker desktop start", 1, "", "  sem licença\n");
    assert_eq!(
        start_desktop(&refused),
        Err(Message::new("docker.start_failed").with("detail", "sem licença"))
    );
    let missing = Scripted::default();
    let error = start_desktop(&missing).unwrap_err();
    assert_eq!(error.key, "docker.start_failed");
    assert!(!error.params["detail"].is_empty());
}
