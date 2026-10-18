//! What the interface receives after an analysis (F08): plain, serializable data built
//! from the core's report. No rules are decided here (ADR-0003); this only reshapes.

use limpador_core::activity::ProjectState;
use limpador_core::analysis::AnalysisReport;
use limpador_core::disks::DriveSpace;
use limpador_core::engine::CollectorState;
use limpador_core::item::{Category, CleanAction, Nature, Resource, Risk};
use limpador_core::message::Message;
use limpador_core::ranking::RankedItem;
use limpador_core::risk::Destination;
use serde::Serialize;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisView {
    /// Milliseconds since the Unix epoch.
    pub analyzed_at: u64,
    pub duration_ms: u64,
    pub cancelled: bool,
    pub totals: TotalsView,
    pub space: Vec<ItemView>,
    pub organization: Vec<ItemView>,
    pub protected: Vec<ItemView>,
    pub collectors: Vec<CollectorView>,
    pub projects: Vec<ProjectView>,
    pub warnings: Vec<Message>,
    /// Drives holding the analyzed folders (filled by the app, T-08.01).
    pub drives: Vec<DriveView>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DriveView {
    pub root: String,
    pub total: u64,
    pub available: u64,
}

impl From<&DriveSpace> for DriveView {
    fn from(drive: &DriveSpace) -> Self {
        Self {
            root: drive.root.display().to_string(),
            total: drive.total,
            available: drive.available,
        }
    }
}

/// One collector's state change during an analysis (T-08.06), sent as an event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressEvent {
    pub collector: String,
    pub state: &'static str,
    pub done: Option<u64>,
    pub total: Option<u64>,
}

pub fn progress_event(collector: &str, state: &CollectorState) -> ProgressEvent {
    let (done, total) = match state {
        CollectorState::Analyzing(p) => (Some(p.done), p.total),
        _ => (None, None),
    };
    ProgressEvent {
        collector: collector.to_owned(),
        state: collector_view_state(state),
        done,
        total,
    }
}

/// Items of one collector that just finished, before the analysis ends (T-08.06).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemsEvent {
    pub collector: String,
    pub items: Vec<ItemView>,
}

pub fn items_event(collector: &str, items: &[RankedItem]) -> ItemsEvent {
    ItemsEvent {
        collector: collector.to_owned(),
        items: items.iter().map(item_view).collect(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TotalsView {
    pub recoverable: u64,
    pub recoverable_safe: u64,
    pub recoverable_moderate: u64,
    pub recoverable_caution: u64,
    /// Category → bytes, in a fixed order.
    pub by_category: Vec<CategoryTotal>,
    pub organization_items: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryTotal {
    pub category: &'static str,
    pub bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemView {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub collector: String,
    pub category: &'static str,
    /// Path or name, as shown in the detail.
    pub resource: String,
    pub is_path: bool,
    pub apparent_size: u64,
    pub freeable_size: u64,
    pub size_estimated: bool,
    pub risk: &'static str,
    pub nature: &'static str,
    pub reasons: Vec<Message>,
    pub recreate: Option<Message>,
    pub destination: Option<&'static str>,
    pub automation_eligible: bool,
    pub protected_by: Option<Message>,
    pub block_reason: Option<Message>,
    pub side_effects: Vec<Message>,
    pub alerts: Vec<Message>,
    pub action: &'static str,
    pub project: Option<String>,
    pub project_state: Option<&'static str>,
    /// Milliseconds since the Unix epoch.
    pub last_activity: Option<u64>,
    pub contains: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CollectorView {
    pub id: String,
    pub state: &'static str,
    pub message: Option<Message>,
    pub action: Option<Message>,
    pub duration_ms: u64,
    pub items: usize,
    pub partial: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectView {
    pub root: String,
    pub name: String,
    pub state: &'static str,
    pub idle_days: Option<u64>,
    pub recoverable: u64,
}

pub fn category_name(category: Category) -> &'static str {
    match category {
        Category::Docker => "docker",
        Category::DependenciesAndBuilds => "dependencies",
        Category::Git => "git",
        Category::AiAndEditors => "ai_editors",
        Category::GeneralFiles => "general_files",
    }
}

pub fn risk_name(risk: Risk) -> &'static str {
    match risk {
        Risk::Safe => "safe",
        Risk::Moderate => "moderate",
        Risk::Caution => "caution",
        Risk::Blocked => "blocked",
    }
}

fn project_state_name(state: ProjectState) -> &'static str {
    match state {
        ProjectState::Active => "active",
        ProjectState::Inactive { .. } => "inactive",
        ProjectState::Pinned => "pinned",
    }
}

fn millis(at: SystemTime) -> u64 {
    at.duration_since(UNIX_EPOCH)
        .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
}

pub fn item_view(entry: &RankedItem) -> ItemView {
    let item = &entry.item;
    let a = &entry.assessment;
    let (resource, is_path) = match &item.resource {
        Resource::Path(path) => (path.display().to_string(), true),
        Resource::Name(name) => (name.clone(), false),
    };
    ItemView {
        id: item.id.as_str().to_owned(),
        name: item.name.clone(),
        kind: item.kind.clone(),
        collector: item.collector.clone(),
        category: category_name(item.category),
        resource,
        is_path,
        apparent_size: item.apparent_size,
        freeable_size: item.freeable_size,
        size_estimated: item.size_estimated,
        risk: risk_name(a.risk),
        nature: match item.nature {
            Nature::Recreatable => "recreatable",
            Nature::UserContent => "user_content",
        },
        reasons: a.reasons.clone(),
        recreate: item.recreate.clone(),
        destination: a.destination.map(|d| match d {
            Destination::Direct => "direct",
            Destination::Quarantine => "quarantine",
        }),
        automation_eligible: a.automation_eligible,
        protected_by: a.protected_by.clone(),
        block_reason: item.block_reason.clone(),
        side_effects: item.side_effects.clone(),
        alerts: item.alerts.clone(),
        action: match item.action {
            CleanAction::Remove => "remove",
            CleanAction::Compact => "compact",
            CleanAction::ManagerCommand { .. } => "manager_command",
        },
        project: entry.project.as_ref().map(|p| p.display().to_string()),
        project_state: entry.project_state.map(project_state_name),
        last_activity: item.last_activity.map(millis),
        contains: item
            .contains
            .iter()
            .map(|c| c.as_str().to_owned())
            .collect(),
    }
}

/// Stable name of a collector state for the interface.
pub fn collector_view_state(state: &CollectorState) -> &'static str {
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

fn collector_view(report: &limpador_core::engine::CollectorReport) -> CollectorView {
    let (message, action) = match &report.state {
        CollectorState::Error(m) => (Some(m.clone()), None),
        CollectorState::Unavailable { reason, action } => (Some(reason.clone()), action.clone()),
        _ => (None, None),
    };
    CollectorView {
        id: report.id.clone(),
        state: collector_view_state(&report.state),
        message,
        action,
        duration_ms: u64::try_from(report.duration.as_millis()).unwrap_or(u64::MAX),
        items: report.items,
        partial: report.partial,
    }
}

const CATEGORIES: [Category; 5] = [
    Category::Docker,
    Category::DependenciesAndBuilds,
    Category::Git,
    Category::AiAndEditors,
    Category::GeneralFiles,
];

pub fn analysis_view(report: &AnalysisReport) -> AnalysisView {
    let totals = &report.ranking.totals;
    let by_risk = |risk| totals.by_risk.get(&risk).copied().unwrap_or(0);
    AnalysisView {
        analyzed_at: millis(report.result.started_at),
        duration_ms: u64::try_from(report.result.duration.as_millis()).unwrap_or(u64::MAX),
        cancelled: report.result.cancelled,
        totals: TotalsView {
            recoverable: totals.recoverable,
            recoverable_safe: totals.recoverable_safe,
            recoverable_moderate: by_risk(Risk::Moderate),
            recoverable_caution: by_risk(Risk::Caution),
            by_category: CATEGORIES
                .iter()
                .map(|c| CategoryTotal {
                    category: category_name(*c),
                    bytes: totals.by_category.get(c).copied().unwrap_or(0),
                })
                .collect(),
            organization_items: totals.organization_items,
        },
        space: report.ranking.space.iter().map(item_view).collect(),
        organization: report.ranking.organization.iter().map(item_view).collect(),
        protected: report.ranking.protected.iter().map(item_view).collect(),
        collectors: report
            .result
            .collectors
            .iter()
            .map(collector_view)
            .collect(),
        projects: report
            .projects
            .iter()
            .map(|p| {
                let root = p.project.root.clone();
                ProjectView {
                    root: root.display().to_string(),
                    name: p.project.name.clone(),
                    state: project_state_name(p.state),
                    idle_days: match p.state {
                        ProjectState::Inactive { idle } => Some(idle.as_secs() / 86_400),
                        _ => None,
                    },
                    recoverable: totals.by_project.get(&Some(root)).copied().unwrap_or(0),
                }
            })
            .collect(),
        warnings: report.warnings.clone(),
        drives: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use limpador_core::analysis::ProjectSummary;
    use limpador_core::engine::{AnalysisResult, CollectorReport};
    use limpador_core::item::{ItemDraft, ItemId};
    use limpador_core::projects::{Project, ProjectGit};
    use limpador_core::ranking::rank;
    use limpador_core::risk::assess;
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::time::Duration;

    const GB: u64 = 1_000_000_000;

    fn entry(
        name: &str,
        gb: u64,
        risk: Risk,
        category: Category,
        project: Option<(&str, ProjectState)>,
    ) -> RankedItem {
        let item = ItemDraft::new(
            "t",
            "x",
            category,
            Resource::Path(format!("C:\\p\\{name}").into()),
            name,
            Nature::Recreatable,
            risk,
        )
        .sizes(gb * GB, gb * GB)
        .reason(Message::new("reason.x"))
        .recreate(Message::new("recreate.x"))
        .block_reason(Message::new("block.x"))
        .last_activity(UNIX_EPOCH + Duration::from_secs(10))
        .build()
        .unwrap();
        let assessment = assess(&item, None);
        RankedItem::new(
            item,
            assessment,
            project.map(|(p, _)| PathBuf::from(p)),
            project.map(|(_, s)| s),
        )
    }

    fn report() -> AnalysisReport {
        let idle = ProjectState::Inactive {
            idle: Duration::from_secs(45 * 86_400),
        };
        let ranking = rank(vec![
            entry("a", 3, Risk::Safe, Category::Docker, None),
            entry(
                "b",
                5,
                Risk::Moderate,
                Category::DependenciesAndBuilds,
                Some(("C:\\p\\api", idle)),
            ),
            entry("c", 2, Risk::Caution, Category::GeneralFiles, None),
            entry("d", 9, Risk::Blocked, Category::Docker, None),
        ]);
        AnalysisReport {
            result: Arc::new(AnalysisResult {
                started_at: UNIX_EPOCH + Duration::from_secs(1_000),
                duration: Duration::from_millis(1_500),
                cancelled: false,
                collectors: vec![CollectorReport {
                    id: "docker".into(),
                    state: CollectorState::Unavailable {
                        reason: Message::new("docker.stopped"),
                        action: Some(Message::new("docker.start")),
                    },
                    duration: Duration::from_millis(20),
                    items: 0,
                    partial: false,
                    rejected: 0,
                }],
                items: vec![],
            }),
            ranking,
            projects: vec![ProjectSummary {
                project: Project {
                    root: "C:\\p\\api".into(),
                    name: "api".into(),
                    git: ProjectGit::Repository,
                },
                last_activity: None,
                state: idle,
            }],
            warnings: vec![Message::new("roots.unavailable").with("path", "E:\\")],
        }
    }

    #[test]
    fn totals_are_split_by_risk_and_category_in_a_fixed_order() {
        let view = analysis_view(&report());
        assert_eq!(view.totals.recoverable, 10 * GB);
        assert_eq!(
            (
                view.totals.recoverable_safe,
                view.totals.recoverable_moderate,
                view.totals.recoverable_caution
            ),
            (3 * GB, 5 * GB, 2 * GB)
        );
        let categories: Vec<_> = view
            .totals
            .by_category
            .iter()
            .map(|c| (c.category, c.bytes / GB))
            .collect();
        assert_eq!(
            categories,
            vec![
                ("docker", 3),
                ("dependencies", 5),
                ("git", 0),
                ("ai_editors", 0),
                ("general_files", 2)
            ]
        );
    }

    #[test]
    fn items_keep_ranking_order_and_every_detail_the_interface_shows() {
        let view = analysis_view(&report());
        let names: Vec<_> = view.space.iter().map(|i| i.name.as_str()).collect();
        assert_eq!(names, vec!["b", "a", "c", "d"]);

        let b = &view.space[0];
        assert_eq!(
            (b.risk, b.destination, b.category),
            ("moderate", Some("direct"), "dependencies")
        );
        assert_eq!(b.project.as_deref(), Some("C:\\p\\api"));
        assert_eq!(b.project_state, Some("inactive"));
        assert_eq!(b.resource, "C:\\p\\b");
        assert!(b.is_path);
        assert_eq!(b.last_activity, Some(10_000));
        assert_eq!(b.action, "remove");
        assert_eq!(b.nature, "recreatable");
        assert_eq!(b.recreate, Some(Message::new("recreate.x")));

        let d = &view.space[3];
        assert_eq!((d.risk, d.destination), ("blocked", None));
        assert_eq!(d.block_reason, Some(Message::new("block.x")));
        assert!(view.space[1].automation_eligible);
    }

    #[test]
    fn collectors_projects_and_warnings_are_carried_over() {
        let view = analysis_view(&report());
        assert_eq!(view.analyzed_at, 1_000_000);
        assert_eq!(view.duration_ms, 1_500);
        assert_eq!(
            view.collectors,
            vec![CollectorView {
                id: "docker".into(),
                state: "unavailable",
                message: Some(Message::new("docker.stopped")),
                action: Some(Message::new("docker.start")),
                duration_ms: 20,
                items: 0,
                partial: false,
            }]
        );
        assert_eq!(
            view.projects,
            vec![ProjectView {
                root: "C:\\p\\api".into(),
                name: "api".into(),
                state: "inactive",
                idle_days: Some(45),
                recoverable: 5 * GB
            }]
        );
        assert_eq!(view.warnings.len(), 1);
    }

    #[test]
    fn names_are_stable_for_the_interface() {
        assert_eq!(risk_name(Risk::Safe), "safe");
        assert_eq!(risk_name(Risk::Caution), "caution");
        assert_eq!(category_name(Category::AiAndEditors), "ai_editors");
        assert_eq!(project_state_name(ProjectState::Pinned), "pinned");
        assert_eq!(project_state_name(ProjectState::Active), "active");
        let mut nested = entry("w", 1, Risk::Safe, Category::Git, None);
        nested
            .item
            .contains
            .push(ItemId::new("t", "x", &Resource::Name("n".into())));
        nested.item.action = CleanAction::Compact;
        let view = item_view(&nested);
        assert_eq!((view.contains.len(), view.action), (1, "compact"));
    }

    #[test]
    fn progress_events_carry_the_count_only_while_analyzing() {
        let analyzing = progress_event(
            "docker",
            &CollectorState::Analyzing(limpador_core::collector::Progress {
                done: 3,
                total: Some(8),
            }),
        );
        assert_eq!(
            (analyzing.state, analyzing.done, analyzing.total),
            ("analyzing", Some(3), Some(8))
        );
        let done = progress_event("docker", &CollectorState::Done);
        assert_eq!((done.done, done.total), (None, None));
    }

    #[test]
    fn a_failed_collector_shows_its_message() {
        let failure = Message::new("docker.command_failed");
        let view = collector_view(&CollectorReport {
            id: "docker".into(),
            state: CollectorState::Error(failure.clone()),
            duration: Duration::from_millis(5),
            items: 0,
            partial: false,
            rejected: 0,
        });
        assert_eq!(
            (view.state, view.message, view.action),
            ("error", Some(failure), None)
        );
    }

    #[test]
    fn items_events_carry_the_collectors_items_as_the_interface_shows_them() {
        let entry = entry(
            "cache",
            2,
            Risk::Safe,
            Category::DependenciesAndBuilds,
            None,
        );
        let event = items_event("deps", std::slice::from_ref(&entry));
        assert_eq!(event.collector, "deps");
        assert_eq!(event.items, vec![item_view(&entry)]);
    }
}
