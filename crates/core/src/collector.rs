//! Contract every collector follows, and the registry that validates them (T-01.04, ADR-0004).
//! Adding a collector never changes the engine: it only implements [`Collector`].

use crate::item::{Category, Item, Nature};
use crate::message::Message;
use crate::ports::{Clock, CommandRunner};
use crate::risk::Destination;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

/// One kind of item a collector produces (e.g. "volume").
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KindSpec {
    pub kind: &'static str,
    pub category: Category,
    pub nature: Nature,
    /// Whether the collector can restore this kind from quarantine.
    pub restorable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollectorDescriptor {
    pub id: &'static str,
    pub kinds: Vec<KindSpec>,
    /// Catalog entries (ADR-0005) the collector reads its locations from.
    pub catalog_locations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Detection {
    Available,
    /// Installed but cannot be analyzed now (e.g. "Docker parado"), with a suggested action.
    Unavailable {
        reason: Message,
        action: Option<Message>,
    },
    NotInstalled,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Revalidation {
    /// Still there; size and risk may have been refreshed.
    StillValid(Box<Item>),
    Changed(Message),
    Gone,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CleanOutcome {
    Cleaned {
        freed_bytes: u64,
    },
    Quarantined {
        location: PathBuf,
    },
    PartiallyCleaned {
        freed_bytes: u64,
        remaining: Message,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CollectorError {
    Unsupported,
    Failed(Message),
}

/// Cooperative stop: cancellation by the user or the collector's time limit.
#[derive(Debug, Clone)]
pub struct StopSignal {
    cancelled: Arc<AtomicBool>,
    deadline: Option<Instant>,
}

impl StopSignal {
    pub fn new(deadline: Option<Instant>) -> Self {
        Self {
            cancelled: Arc::new(AtomicBool::new(false)),
            deadline,
        }
    }

    /// Same cancellation flag, different deadline (one per collector).
    pub fn with_deadline(&self, deadline: Option<Instant>) -> Self {
        Self {
            cancelled: Arc::clone(&self.cancelled),
            deadline,
        }
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }

    pub fn timed_out(&self) -> bool {
        self.deadline.is_some_and(|d| Instant::now() >= d)
    }

    /// Collectors check this between units of work and return early when true.
    pub fn should_stop(&self) -> bool {
        self.is_cancelled() || self.timed_out()
    }
}

/// Progress of one collector: `done` units out of `total` (when known).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Progress {
    pub done: u64,
    pub total: Option<u64>,
}

pub type ProgressFn = Arc<dyn Fn(Progress) + Send + Sync>;

/// Everything a collector may use while analyzing. It offers no way to change the
/// system: analysis is read-only (I1).
#[derive(Clone)]
pub struct AnalysisContext {
    pub roots: Vec<PathBuf>,
    pub clock: Arc<dyn Clock>,
    pub commands: Arc<dyn CommandRunner>,
    pub stop: StopSignal,
    progress: ProgressFn,
}

impl AnalysisContext {
    pub fn new(
        roots: Vec<PathBuf>,
        clock: Arc<dyn Clock>,
        commands: Arc<dyn CommandRunner>,
        stop: StopSignal,
        progress: ProgressFn,
    ) -> Self {
        Self {
            roots,
            clock,
            commands,
            stop,
            progress,
        }
    }

    pub fn report(&self, done: u64, total: Option<u64>) {
        (self.progress)(Progress { done, total });
    }

    pub fn should_stop(&self) -> bool {
        self.stop.should_stop()
    }
}

pub trait Collector: Send + Sync {
    fn descriptor(&self) -> CollectorDescriptor;

    fn detect(&self, ctx: &AnalysisContext) -> Detection;

    /// Finds items and hands each one to `emit`. Must not change the system (I1),
    /// must report progress and must return early when `ctx.should_stop()`.
    fn analyze(
        &self,
        ctx: &AnalysisContext,
        emit: &mut dyn FnMut(Item),
    ) -> Result<(), CollectorError>;

    /// Checks an item again right before acting on it (I6).
    fn revalidate(&self, ctx: &AnalysisContext, item: &Item) -> Revalidation;

    fn clean(&self, item: &Item, destination: Destination) -> Result<CleanOutcome, CollectorError>;

    /// Brings a quarantined item back. Required for every user-content kind (I9).
    fn restore(&self, _item: &Item) -> Result<(), CollectorError> {
        Err(CollectorError::Unsupported)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegistrationError {
    /// A user-content kind must be restorable from quarantine (I9).
    UserContentNotRestorable {
        collector: String,
        kind: String,
    },
    DuplicateId(String),
}

#[derive(Default, Clone)]
pub struct Registry {
    collectors: Vec<Arc<dyn Collector>>,
}

impl Registry {
    pub fn register(&mut self, collector: Arc<dyn Collector>) -> Result<(), RegistrationError> {
        let descriptor = collector.descriptor();
        if self
            .collectors
            .iter()
            .any(|c| c.descriptor().id == descriptor.id)
        {
            return Err(RegistrationError::DuplicateId(descriptor.id.to_owned()));
        }
        if let Some(kind) = descriptor
            .kinds
            .iter()
            .find(|k| k.nature == Nature::UserContent && !k.restorable)
        {
            return Err(RegistrationError::UserContentNotRestorable {
                collector: descriptor.id.to_owned(),
                kind: kind.kind.to_owned(),
            });
        }
        self.collectors.push(collector);
        Ok(())
    }

    pub fn collectors(&self) -> &[Arc<dyn Collector>] {
        &self.collectors
    }

    /// Whether `item` matches a kind its collector declared, with the same category
    /// and nature. Items that do not are discarded by the engine.
    pub fn declares(&self, item: &Item) -> bool {
        self.collectors.iter().any(|c| {
            let d = c.descriptor();
            d.id == item.collector
                && d.kinds.iter().any(|k| {
                    k.kind == item.kind && k.category == item.category && k.nature == item.nature
                })
        })
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::adapters::SystemClock;
    use crate::item::{ItemDraft, Resource, Risk};
    use crate::ports::CommandOutput;
    use std::io;
    use std::sync::Mutex;
    use std::time::Duration;

    /// Command runner that refuses everything: collectors under test must not need one.
    pub(crate) struct NoCommands;
    impl CommandRunner for NoCommands {
        fn run(&self, program: &str, _args: &[&str]) -> io::Result<CommandOutput> {
            Err(io::Error::other(format!("unexpected command: {program}")))
        }
    }

    pub(crate) fn context(stop: StopSignal) -> AnalysisContext {
        AnalysisContext::new(
            vec![],
            Arc::new(SystemClock),
            Arc::new(NoCommands),
            stop,
            Arc::new(|_| {}),
        )
    }

    /// Configurable collector for tests.
    pub(crate) struct FakeCollector {
        pub id: &'static str,
        pub detection: Detection,
        pub items: Vec<&'static str>,
        pub kinds: Vec<KindSpec>,
        pub gone: Mutex<Vec<String>>,
    }

    impl FakeCollector {
        pub(crate) fn new(id: &'static str, items: Vec<&'static str>) -> Self {
            Self {
                id,
                detection: Detection::Available,
                items,
                kinds: vec![KindSpec {
                    kind: "cache",
                    category: Category::DependenciesAndBuilds,
                    nature: Nature::Recreatable,
                    restorable: false,
                }],
                gone: Mutex::new(vec![]),
            }
        }

        pub(crate) fn item(&self, name: &str) -> Item {
            ItemDraft::new(
                self.id,
                "cache",
                Category::DependenciesAndBuilds,
                Resource::Name(name.to_owned()),
                name,
                Nature::Recreatable,
                Risk::Safe,
            )
            .sizes(10, 10)
            .reason(Message::new("reason.fake"))
            .recreate(Message::new("recreate.fake"))
            .build()
            .unwrap()
        }
    }

    impl Collector for FakeCollector {
        fn descriptor(&self) -> CollectorDescriptor {
            CollectorDescriptor {
                id: self.id,
                kinds: self.kinds.clone(),
                catalog_locations: vec![],
            }
        }
        fn detect(&self, _ctx: &AnalysisContext) -> Detection {
            self.detection.clone()
        }
        fn analyze(
            &self,
            ctx: &AnalysisContext,
            emit: &mut dyn FnMut(Item),
        ) -> Result<(), CollectorError> {
            for (done, name) in self.items.iter().enumerate() {
                emit(self.item(name));
                ctx.report(done as u64 + 1, Some(self.items.len() as u64));
            }
            Ok(())
        }
        fn revalidate(&self, _ctx: &AnalysisContext, item: &Item) -> Revalidation {
            if self.gone.lock().unwrap().contains(&item.name) {
                Revalidation::Gone
            } else {
                Revalidation::StillValid(Box::new(item.clone()))
            }
        }
        fn clean(&self, item: &Item, _d: Destination) -> Result<CleanOutcome, CollectorError> {
            Ok(CleanOutcome::Cleaned {
                freed_bytes: item.freeable_size,
            })
        }
    }

    fn user_content_kind(restorable: bool) -> KindSpec {
        KindSpec {
            kind: "volume",
            category: Category::Docker,
            nature: Nature::UserContent,
            restorable,
        }
    }

    #[test]
    fn t01_04_ca1_new_collector_takes_part_through_the_contract_alone() {
        let mut registry = Registry::default();
        registry
            .register(Arc::new(FakeCollector::new("novo", vec!["a", "b"])))
            .unwrap();

        // What the engine does for every registered collector, with no collector-specific code.
        let ctx = context(StopSignal::new(None));
        let mut items = vec![];
        for collector in registry.collectors() {
            assert_eq!(collector.detect(&ctx), Detection::Available);
            collector.analyze(&ctx, &mut |i| items.push(i)).unwrap();
        }
        assert_eq!(items.len(), 2);
        assert!(items.iter().all(|i| registry.declares(i)));
    }

    #[test]
    fn t01_04_ca2_user_content_without_restore_is_rejected() {
        let mut collector = FakeCollector::new("docker", vec![]);
        collector.kinds = vec![user_content_kind(false)];

        let error = Registry::default()
            .register(Arc::new(collector))
            .unwrap_err();
        assert_eq!(
            error,
            RegistrationError::UserContentNotRestorable {
                collector: "docker".into(),
                kind: "volume".into()
            }
        );
    }

    #[test]
    fn user_content_with_restore_is_accepted() {
        let mut collector = FakeCollector::new("docker", vec![]);
        collector.kinds = vec![user_content_kind(true)];
        assert!(Registry::default().register(Arc::new(collector)).is_ok());
    }

    #[test]
    fn duplicate_collector_ids_are_rejected() {
        let mut registry = Registry::default();
        registry
            .register(Arc::new(FakeCollector::new("x", vec![])))
            .unwrap();
        assert_eq!(
            registry.register(Arc::new(FakeCollector::new("x", vec![]))),
            Err(RegistrationError::DuplicateId("x".into()))
        );
    }

    #[test]
    fn t01_04_ca3_missing_ecosystem_is_reported_as_not_installed() {
        let mut collector = FakeCollector::new("docker", vec!["nunca"]);
        collector.detection = Detection::NotInstalled;
        let ctx = context(StopSignal::new(None));
        assert_eq!(collector.detect(&ctx), Detection::NotInstalled);
    }

    #[test]
    fn t01_04_ca5_vanished_item_revalidates_as_gone() {
        let collector = FakeCollector::new("c", vec!["a"]);
        let item = collector.item("a");
        let ctx = context(StopSignal::new(None));
        assert_eq!(
            collector.revalidate(&ctx, &item),
            Revalidation::StillValid(Box::new(item.clone()))
        );

        collector.gone.lock().unwrap().push("a".into());
        assert_eq!(collector.revalidate(&ctx, &item), Revalidation::Gone);
    }

    #[test]
    fn items_from_undeclared_kinds_are_not_recognized() {
        let mut registry = Registry::default();
        let collector = FakeCollector::new("c", vec![]);
        let foreign = FakeCollector::new("outro", vec![]).item("x");
        let own = collector.item("y");
        registry.register(Arc::new(collector)).unwrap();
        assert!(registry.declares(&own));
        assert!(!registry.declares(&foreign));
    }

    #[test]
    fn same_kind_with_another_category_or_nature_is_not_recognized() {
        let mut registry = Registry::default();
        registry
            .register(Arc::new(FakeCollector::new("c", vec![])))
            .unwrap();
        let declared = FakeCollector::new("c", vec![]).item("x");

        let mut other_category = declared.clone();
        other_category.category = Category::Docker;
        let mut other_nature = declared.clone();
        other_nature.nature = Nature::UserContent;

        assert!(registry.declares(&declared));
        assert!(!registry.declares(&other_category));
        assert!(!registry.declares(&other_nature));
    }

    #[test]
    fn context_reports_when_the_collector_must_stop() {
        let stop = StopSignal::new(None);
        let ctx = context(stop.clone());
        assert!(!ctx.should_stop());
        stop.cancel();
        assert!(ctx.should_stop());
    }

    #[test]
    fn restore_is_unsupported_unless_implemented() {
        let collector = FakeCollector::new("c", vec![]);
        assert_eq!(
            collector.restore(&collector.item("a")),
            Err(CollectorError::Unsupported)
        );
    }

    #[test]
    fn stop_signal_covers_cancellation_and_deadline() {
        let stop = StopSignal::new(None);
        assert!(!stop.should_stop());
        let late = stop.with_deadline(Some(Instant::now() - Duration::from_millis(1)));
        assert!(late.timed_out() && late.should_stop());
        assert!(!stop.timed_out());

        stop.cancel();
        assert!(stop.is_cancelled() && stop.should_stop());
        assert!(late.is_cancelled(), "deadline copies share cancellation");
    }

    #[test]
    fn progress_reaches_the_engine() {
        let seen = Arc::new(Mutex::new(vec![]));
        let sink = Arc::clone(&seen);
        let ctx = AnalysisContext::new(
            vec![],
            Arc::new(SystemClock),
            Arc::new(NoCommands),
            StopSignal::new(None),
            Arc::new(move |p| sink.lock().unwrap().push(p)),
        );
        FakeCollector::new("c", vec!["a", "b"])
            .analyze(&ctx, &mut |_| {})
            .unwrap();
        assert_eq!(
            *seen.lock().unwrap(),
            vec![
                Progress {
                    done: 1,
                    total: Some(2)
                },
                Progress {
                    done: 2,
                    total: Some(2)
                }
            ]
        );
    }
}
