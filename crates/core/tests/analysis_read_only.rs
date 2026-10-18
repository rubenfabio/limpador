//! I1 "analysis never changes anything", enforced in tests with the fake machine.

use limpador_core::adapters::SystemCommandRunner;
use limpador_core::collector::{
    AnalysisContext, CleanOutcome, Collector, CollectorDescriptor, CollectorError, Detection,
    KindSpec, Revalidation, StopSignal,
};
use limpador_core::item::{Category, Item, Nature};
use limpador_core::risk::Destination;
use limpador_testkit::{FakeMachine, ProjectSpec};
use std::fs;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::Path;
use std::sync::Arc;

/// Collector that lists the roots and, when `writes` is set, also leaves a file behind.
struct ScanCollector {
    writes: bool,
}

impl Collector for ScanCollector {
    fn descriptor(&self) -> CollectorDescriptor {
        CollectorDescriptor {
            id: "scan",
            kinds: vec![KindSpec {
                kind: "cache",
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
        _emit: &mut dyn FnMut(Item),
    ) -> Result<(), CollectorError> {
        for root in &ctx.roots {
            let entries = fs::read_dir(root).map(|d| d.count()).unwrap_or(0);
            if self.writes {
                fs::write(root.join("rastro.tmp"), entries.to_string()).unwrap();
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

fn analyze(collector: &dyn Collector, roots: &Path) {
    let ctx = AnalysisContext::new(
        vec![roots.to_owned()],
        Arc::new(limpador_core::adapters::SystemClock),
        Arc::new(SystemCommandRunner),
        StopSignal::new(None),
        Arc::new(|_| {}),
    );
    collector.analyze(&ctx, &mut |_| {}).unwrap();
}

#[test]
fn t01_04_ca4_collector_that_writes_during_analysis_fails_the_test() {
    let machine = FakeMachine::new().unwrap();
    machine
        .project(&ProjectSpec::node("api").inactive_days(40))
        .unwrap();
    let before = machine.snapshot();

    analyze(&ScanCollector { writes: true }, &machine.roots());

    let guard = catch_unwind(AssertUnwindSafe(|| machine.assert_unchanged_since(&before)));
    let message = guard.unwrap_err();
    let message = message.downcast_ref::<String>().unwrap();
    assert!(message.contains("rastro.tmp"), "{message}");
}

#[test]
fn read_only_collector_passes_the_guard() {
    let machine = FakeMachine::new().unwrap();
    machine
        .project(&ProjectSpec::node("api").inactive_days(40))
        .unwrap();
    let before = machine.snapshot();

    analyze(&ScanCollector { writes: false }, &machine.roots());

    machine.assert_unchanged_since(&before);
}

#[test]
fn node_modules_analysis_and_revalidation_change_nothing() {
    use limpador_core::adapters::native_platform;
    use limpador_core::collectors::node_modules::NodeModulesCollector;
    use limpador_core::projects::{DiscoveryOptions, discover};

    let machine = FakeMachine::new().unwrap();
    let api = machine
        .project(&ProjectSpec::node("api").inactive_days(40))
        .unwrap();
    fs::create_dir_all(api.join("node_modules").join("react")).unwrap();
    fs::write(api.join("node_modules").join("react").join("index.js"), "x").unwrap();
    let before = machine.snapshot();

    let collector = NodeModulesCollector::new(native_platform());
    let discovery = discover(&[machine.roots()], &DiscoveryOptions::default());
    let ctx = AnalysisContext::new(
        vec![machine.roots()],
        Arc::new(limpador_core::adapters::SystemClock),
        Arc::new(SystemCommandRunner),
        StopSignal::new(None),
        Arc::new(|_| {}),
    )
    .with_discovery(Arc::new(discovery));
    let mut items = vec![];
    collector.analyze(&ctx, &mut |i| items.push(i)).unwrap();
    assert_eq!(items.len(), 1);
    collector.revalidate(&ctx, &items[0]);

    machine.assert_unchanged_since(&before);
}
