//! The full analysis: roots → projects → collectors → project, protection and risk of
//! each item → ranking (F01 + F02 together). Read-only from start to end (I1).

use crate::activity::{ActivityRules, ProjectState, associate, last_activity, state};
use crate::config::Config;
use crate::config::Protections;
use crate::engine::{AnalysisInputs, AnalysisResult, Engine, EventFn};
use crate::item::{Item, Resource};
use crate::message::Message;
use crate::ports::{Clock, CommandRunner};
use crate::projects::{Discovery, DiscoveryOptions, Project, discover};
use crate::protection::protection_for;
use crate::ranking::{RankedItem, Ranking, rank};
use crate::risk::assess;
use crate::roots::{available, is_excluded, path_key};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;

#[derive(Debug, Clone, PartialEq)]
pub struct ProjectSummary {
    pub project: Project,
    pub last_activity: Option<SystemTime>,
    pub state: ProjectState,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AnalysisReport {
    pub result: Arc<AnalysisResult>,
    pub ranking: Ranking,
    pub projects: Vec<ProjectSummary>,
    /// E.g. unavailable roots: the analysis still completes.
    pub warnings: Vec<Message>,
}

pub struct Analyzer {
    pub engine: Engine,
    pub clock: Arc<dyn Clock>,
    pub commands: Arc<dyn CommandRunner>,
}

/// Ranked items of one collector that just finished (T-08.06), before the others.
pub type PartialFn = Arc<dyn Fn(&str, Vec<RankedItem>) + Send + Sync>;

impl Analyzer {
    pub fn analyze(&self, config: &Config, on_event: EventFn) -> AnalysisReport {
        self.analyze_streaming(config, on_event, Arc::new(|_, _| {}))
    }

    /// Like `analyze`, and hands each collector's ranked items to `on_partial` as soon as
    /// that collector finishes. Projects are read first, so partial items already carry
    /// their project's activity.
    pub fn analyze_streaming(
        &self,
        config: &Config,
        on_event: EventFn,
        on_partial: PartialFn,
    ) -> AnalysisReport {
        let (roots, warnings) = available(&config.roots);
        let discovery = discover(
            &roots,
            &DiscoveryOptions {
                exclusions: config.roots.exclusions.clone(),
                max_depth: config.preferences.scan_depth,
                ..DiscoveryOptions::default()
            },
        );

        let now = self.clock.now();
        let rules = ActivityRules {
            inactivity: config.preferences.project_inactivity(),
            pinned: config.protections.pinned_projects.clone(),
            running_containers: Vec::new(),
        };
        let projects: Vec<ProjectSummary> = discovery
            .projects
            .iter()
            .map(|project| {
                let last = last_activity(project, self.commands.as_ref());
                ProjectSummary {
                    project: project.clone(),
                    last_activity: last,
                    state: state(project, last, now, &rules),
                }
            })
            .collect();
        let ranker = Arc::new(Ranker {
            states: projects
                .iter()
                .map(|s| (path_key(&s.project.root), s.state))
                .collect(),
            discovery,
            exclusions: config.roots.exclusions.clone(),
            protections: config.protections.clone(),
        });

        let partial_ranker = Arc::clone(&ranker);
        let result = self.engine.analyze(
            &AnalysisInputs {
                roots,
                clock: Arc::clone(&self.clock),
                commands: Arc::clone(&self.commands),
                on_items: Some(Arc::new(move |id, items| {
                    on_partial(
                        id,
                        items
                            .iter()
                            .filter_map(|i| partial_ranker.entry(i))
                            .collect(),
                    );
                })),
            },
            on_event,
        );

        AnalysisReport {
            ranking: rank(
                result
                    .items
                    .iter()
                    .filter_map(|i| ranker.entry(i))
                    .collect(),
            ),
            result,
            projects,
            warnings,
        }
    }
}

/// What it takes to place one item: its project, that project's activity, its protection.
struct Ranker {
    discovery: Discovery,
    states: HashMap<String, ProjectState>,
    exclusions: Vec<String>,
    protections: Protections,
}

impl Ranker {
    /// `None` for items under an excluded folder.
    fn entry(&self, item: &Item) -> Option<RankedItem> {
        if excluded(item, &self.exclusions) {
            return None;
        }
        let project = item_path(item)
            .and_then(|path| associate(path, &self.discovery))
            .map(|p| p.root.clone());
        let project_state = project
            .as_ref()
            .and_then(|root| self.states.get(&path_key(root)).copied());
        let protection = protection_for(item, &self.protections);
        let assessment = assess(item, protection.as_ref());
        Some(RankedItem::new(
            item.clone(),
            assessment,
            project,
            project_state,
        ))
    }
}

/// Where the item lives, for project association: its own path, or the collector's hint
/// (e.g. the Compose working folder of a Docker volume).
fn item_path(item: &Item) -> Option<&Path> {
    item.project_hint.as_deref().or(match &item.resource {
        Resource::Path(path) => Some(path.as_path()),
        Resource::Name(_) => None,
    })
}

fn excluded(item: &Item, exclusions: &[String]) -> bool {
    let paths: [Option<&PathBuf>; 2] = [
        match &item.resource {
            Resource::Path(path) => Some(path),
            Resource::Name(_) => None,
        },
        item.project_hint.as_ref(),
    ];
    paths
        .into_iter()
        .flatten()
        .any(|p| is_excluded(p, exclusions))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::SystemCommandRunner;
    use crate::collector::{
        AnalysisContext, CleanOutcome, Collector, CollectorDescriptor, CollectorError, Detection,
        KindSpec, Registry, Revalidation,
    };
    use crate::config::Protections;
    use crate::engine::EngineConfig;
    use crate::item::{Category, ItemDraft, Nature, Risk};
    use crate::risk::Destination;
    use limpador_testkit::{FakeMachine, ProjectSpec};
    use std::sync::Mutex;

    /// Reports a `node_modules` item for every project folder found under the roots.
    struct NodeModulesCollector;

    impl Collector for NodeModulesCollector {
        fn descriptor(&self) -> CollectorDescriptor {
            CollectorDescriptor {
                id: "deps",
                kinds: vec![KindSpec {
                    kind: "node_modules",
                    category: Category::DependenciesAndBuilds,
                    nature: Nature::Recreatable,
                    restorable: false,
                }],
                catalog_locations: vec![],
            }
        }
        fn detect(&self, _ctx: &AnalysisContext) -> Detection {
            Detection::Available
        }
        fn analyze(
            &self,
            ctx: &AnalysisContext,
            emit: &mut dyn FnMut(Item),
        ) -> Result<(), CollectorError> {
            for root in &ctx.roots {
                for project in std::fs::read_dir(root).unwrap().filter_map(Result::ok) {
                    let path = project.path().join("node_modules");
                    emit(
                        ItemDraft::new(
                            "deps",
                            "node_modules",
                            Category::DependenciesAndBuilds,
                            Resource::Path(path),
                            project.file_name().to_string_lossy(),
                            Nature::Recreatable,
                            Risk::Moderate,
                        )
                        .sizes(1_000, 1_000)
                        .reason(Message::new("reason.node_modules"))
                        .recreate(Message::new("recreate.npm_install"))
                        .build()
                        .unwrap(),
                    );
                }
            }
            Ok(())
        }
        fn revalidate(&self, _ctx: &AnalysisContext, _item: &Item) -> Revalidation {
            Revalidation::Gone
        }
        fn clean(&self, _item: &Item, _d: Destination) -> Result<CleanOutcome, CollectorError> {
            Err(CollectorError::Unsupported)
        }
    }

    /// The fake machine's clock implements the trait of the core build it links against,
    /// not this test build's, so unit tests here use their own frozen clock.
    struct Frozen(SystemTime);
    impl Clock for Frozen {
        fn now(&self) -> SystemTime {
            self.0
        }
    }

    fn analyzer(machine: &FakeMachine) -> Analyzer {
        let mut registry = Registry::default();
        registry.register(Arc::new(NodeModulesCollector)).unwrap();
        Analyzer {
            engine: Engine::new(registry, EngineConfig::default()),
            clock: Arc::new(Frozen(machine.clock().days_ago(0))),
            commands: Arc::new(SystemCommandRunner),
        }
    }

    fn config(machine: &FakeMachine) -> Config {
        let mut config = Config::default();
        config.roots.paths = vec![machine.roots()];
        config
    }

    #[test]
    fn ranks_items_with_their_projects_activity() {
        let machine = FakeMachine::new().unwrap();
        machine
            .project(&ProjectSpec::node("parado").inactive_days(90).with_git())
            .unwrap();
        machine
            .project(&ProjectSpec::node("ativo").inactive_days(1).with_git())
            .unwrap();

        let report = analyzer(&machine).analyze(&config(&machine), Arc::new(|_, _| {}));

        let order: Vec<&str> = report
            .ranking
            .space
            .iter()
            .map(|e| e.item.name.as_str())
            .collect();
        assert_eq!(order, vec!["parado", "ativo"]);
        assert!(matches!(
            report.ranking.space[0].project_state,
            Some(ProjectState::Inactive { .. })
        ));
        assert_eq!(report.ranking.totals.recoverable, 2_000);
        assert_eq!(report.projects.len(), 2);
    }

    #[test]
    fn partial_items_arrive_ranked_with_their_projects_activity() {
        let machine = FakeMachine::new().unwrap();
        machine
            .project(&ProjectSpec::node("parado").inactive_days(90).with_git())
            .unwrap();
        machine
            .project(&ProjectSpec::node("excluido").inactive_days(90).with_git())
            .unwrap();
        let mut config = config(&machine);
        config.roots.exclusions = vec!["**/excluido".into()];

        let partial = Arc::new(Mutex::new(vec![]));
        let sink = Arc::clone(&partial);
        let report = analyzer(&machine).analyze_streaming(
            &config,
            Arc::new(|_, _| {}),
            Arc::new(move |id, items| sink.lock().unwrap().push((id.to_owned(), items))),
        );

        let partial = partial.lock().unwrap();
        assert_eq!(partial.len(), 1);
        let (id, items) = &partial[0];
        assert_eq!(id, "deps");
        assert_eq!(
            items
                .iter()
                .map(|e| e.item.name.as_str())
                .collect::<Vec<_>>(),
            vec!["parado"],
            "excluded folders stay out of partial results too"
        );
        assert!(matches!(
            items[0].project_state,
            Some(ProjectState::Inactive { .. })
        ));
        assert_eq!(items[0], report.ranking.space[0]);
    }

    #[test]
    fn protections_and_exclusions_apply_to_collected_items() {
        let machine = FakeMachine::new().unwrap();
        machine.project(&ProjectSpec::node("tcc")).unwrap();
        machine.project(&ProjectSpec::node("cliente-x")).unwrap();
        machine.project(&ProjectSpec::node("livre")).unwrap();
        let mut config = config(&machine);
        config.protections = Protections {
            protected_paths: vec![machine.roots().join("tcc")],
            ..Protections::default()
        };
        config.roots.exclusions.push(
            machine
                .roots()
                .join("cliente-x")
                .to_string_lossy()
                .into_owned(),
        );

        let report = analyzer(&machine).analyze(&config, Arc::new(|_, _| {}));

        let names = |entries: &[RankedItem]| -> Vec<String> {
            entries.iter().map(|e| e.item.name.clone()).collect()
        };
        assert_eq!(names(&report.ranking.space), vec!["livre"]);
        assert_eq!(names(&report.ranking.protected), vec!["tcc"]);
        let mut projects: Vec<&str> = report
            .projects
            .iter()
            .map(|p| p.project.name.as_str())
            .collect();
        projects.sort_unstable();
        assert_eq!(
            projects,
            vec!["livre", "tcc"],
            "excluded folders are not projects"
        );
    }

    #[test]
    fn project_search_respects_the_scan_depth_preference() {
        let machine = FakeMachine::new().unwrap();
        let now = machine.clock().days_ago(0);
        machine
            .write_file("roots/a/b/c/fundo/package.json", "{}", now)
            .unwrap();
        let mut config = config(&machine);

        config.preferences.scan_depth = 3;
        assert!(
            analyzer(&machine)
                .analyze(&config, Arc::new(|_, _| {}))
                .projects
                .is_empty()
        );
        config.preferences.scan_depth = 4;
        assert_eq!(
            analyzer(&machine)
                .analyze(&config, Arc::new(|_, _| {}))
                .projects
                .len(),
            1
        );
    }

    #[test]
    fn unavailable_roots_become_warnings_and_the_analysis_completes() {
        let machine = FakeMachine::new().unwrap();
        machine.project(&ProjectSpec::node("api")).unwrap();
        let mut config = config(&machine);
        config.roots.paths.push(machine.path("disco-externo"));

        let report = analyzer(&machine).analyze(&config, Arc::new(|_, _| {}));
        assert_eq!(report.warnings.len(), 1);
        assert_eq!(report.warnings[0].key, "roots.unavailable");
        assert_eq!(report.ranking.space.len(), 1);
    }

    #[test]
    fn the_analysis_never_changes_the_machine() {
        let machine = FakeMachine::new().unwrap();
        machine
            .project(&ProjectSpec::node("api").inactive_days(40).with_git())
            .unwrap();
        let before = machine.snapshot();
        analyzer(&machine).analyze(&config(&machine), Arc::new(|_, _| {}));
        machine.assert_unchanged_since(&before);
    }
}
