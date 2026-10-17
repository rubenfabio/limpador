//! Runs every registered collector and gathers one consistent picture (T-01.05).
//!
//! Collectors run in parallel (bounded), one failure never stops the others, each has a
//! cooperative time limit, the user can cancel, and only one analysis runs at a time.

use crate::collector::{
    AnalysisContext, Collector, CollectorError, Detection, Progress, Registry, StopSignal,
};
use crate::item::Item;
use crate::message::Message;
use crate::ports::{Clock, CommandRunner};
use std::collections::VecDeque;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::PathBuf;
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::{Duration, Instant, SystemTime};

#[derive(Debug, Clone)]
pub struct EngineConfig {
    /// Default 5 minutes (T-02.05).
    pub collector_timeout: Duration,
    /// Collectors analyzing at the same time, to spare the disk.
    pub max_parallel: usize,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            collector_timeout: Duration::from_secs(5 * 60),
            max_parallel: 4,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CollectorState {
    Waiting,
    Analyzing(Progress),
    Done,
    Error(Message),
    Unavailable {
        reason: Message,
        action: Option<Message>,
    },
    NotInstalled,
    Cancelled,
    TimedOut,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CollectorReport {
    pub id: String,
    pub state: CollectorState,
    pub duration: Duration,
    pub items: usize,
    /// Items found before a time limit hit: shown, but marked as partial.
    pub partial: bool,
    /// Items discarded because their kind was not declared by the collector.
    pub rejected: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AnalysisResult {
    pub started_at: SystemTime,
    pub duration: Duration,
    pub cancelled: bool,
    pub collectors: Vec<CollectorReport>,
    pub items: Vec<Item>,
}

/// Inputs shared by every collector of one analysis.
#[derive(Clone)]
pub struct AnalysisInputs {
    pub roots: Vec<PathBuf>,
    pub clock: Arc<dyn Clock>,
    pub commands: Arc<dyn CommandRunner>,
}

/// Live state changes, for the UI (T-08.06).
pub type EventFn = Arc<dyn Fn(&str, &CollectorState) + Send + Sync>;

#[derive(Default)]
struct State {
    running: Option<StopSignal>,
    /// Bumped at the end of every analysis, so waiters know theirs finished.
    generation: u64,
    finished: Option<Arc<AnalysisResult>>,
    /// Last analysis that was not cancelled.
    last: Option<Arc<AnalysisResult>>,
}

pub struct Engine {
    registry: Registry,
    config: EngineConfig,
    state: Mutex<State>,
    changed: Condvar,
}

impl Engine {
    pub fn new(registry: Registry, config: EngineConfig) -> Self {
        Self {
            registry,
            config,
            state: Mutex::new(State::default()),
            changed: Condvar::new(),
        }
    }

    /// Runs an analysis, or waits for the one already running and returns its result.
    pub fn analyze(&self, inputs: &AnalysisInputs, on_event: EventFn) -> Arc<AnalysisResult> {
        let stop = {
            let mut state = self.state.lock().unwrap();
            if state.running.is_some() {
                let generation = state.generation;
                while state.generation == generation {
                    state = self.changed.wait(state).unwrap();
                }
                return Arc::clone(state.finished.as_ref().expect("finished analysis"));
            }
            let stop = StopSignal::new(None);
            state.running = Some(stop.clone());
            stop
        };

        let result = Arc::new(self.run(inputs, &stop, &on_event));

        let mut state = self.state.lock().unwrap();
        state.running = None;
        state.generation += 1;
        state.finished = Some(Arc::clone(&result));
        if !result.cancelled {
            state.last = Some(Arc::clone(&result));
        }
        self.changed.notify_all();
        result
    }

    /// Asks the running analysis to stop; collectors return at their next check.
    pub fn cancel(&self) {
        if let Some(stop) = &self.state.lock().unwrap().running {
            stop.cancel();
        }
    }

    pub fn is_running(&self) -> bool {
        self.state.lock().unwrap().running.is_some()
    }

    /// Last complete (not cancelled) analysis; a cancelled one never replaces it.
    pub fn last_result(&self) -> Option<Arc<AnalysisResult>> {
        self.state.lock().unwrap().last.clone()
    }

    fn run(
        &self,
        inputs: &AnalysisInputs,
        stop: &StopSignal,
        on_event: &EventFn,
    ) -> AnalysisResult {
        let started_at = inputs.clock.now();
        let started = Instant::now();
        let collectors = self.registry.collectors();
        for c in collectors {
            on_event(c.descriptor().id, &CollectorState::Waiting);
        }

        let queue = Mutex::new(
            collectors
                .iter()
                .cloned()
                .enumerate()
                .collect::<VecDeque<_>>(),
        );
        let outputs = Mutex::new(Vec::new());
        let workers = self.config.max_parallel.clamp(1, collectors.len().max(1));
        thread::scope(|scope| {
            for _ in 0..workers {
                scope.spawn(|| {
                    loop {
                        let Some((index, collector)) = queue.lock().unwrap().pop_front() else {
                            break;
                        };
                        let output = self.run_one(&collector, inputs, stop, on_event);
                        outputs.lock().unwrap().push((index, output));
                    }
                });
            }
        });

        let mut outputs = outputs.into_inner().unwrap();
        outputs.sort_by_key(|(index, _)| *index);
        let mut reports = Vec::new();
        let mut items = Vec::new();
        for (_, (report, found)) in outputs {
            reports.push(report);
            items.extend(found);
        }
        AnalysisResult {
            started_at,
            duration: started.elapsed(),
            cancelled: stop.is_cancelled(),
            collectors: reports,
            items,
        }
    }

    fn run_one(
        &self,
        collector: &Arc<dyn Collector>,
        inputs: &AnalysisInputs,
        stop: &StopSignal,
        on_event: &EventFn,
    ) -> (CollectorReport, Vec<Item>) {
        let id = collector.descriptor().id.to_owned();
        let started = Instant::now();
        let own_stop = stop.with_deadline(Some(started + self.config.collector_timeout));
        let progress_id = id.clone();
        let progress_events = Arc::clone(on_event);
        let ctx = AnalysisContext::new(
            inputs.roots.clone(),
            Arc::clone(&inputs.clock),
            Arc::clone(&inputs.commands),
            own_stop.clone(),
            Arc::new(move |p| progress_events(&progress_id, &CollectorState::Analyzing(p))),
        );

        let mut found = Vec::new();
        let mut rejected = 0;
        // A panicking collector is an isolated failure, like an error.
        let outcome = catch_unwind(AssertUnwindSafe(|| match collector.detect(&ctx) {
            Detection::Available => {
                on_event(
                    &id,
                    &CollectorState::Analyzing(Progress {
                        done: 0,
                        total: None,
                    }),
                );
                collector
                    .analyze(&ctx, &mut |item| {
                        if self.registry.declares(&item) {
                            found.push(item);
                        } else {
                            rejected += 1;
                        }
                    })
                    .map(|()| None)
            }
            Detection::Unavailable { reason, action } => {
                Ok(Some(CollectorState::Unavailable { reason, action }))
            }
            Detection::NotInstalled => Ok(Some(CollectorState::NotInstalled)),
        }));

        let mut partial = false;
        let state = match outcome {
            Ok(Ok(Some(state))) => state,
            _ if stop.is_cancelled() => CollectorState::Cancelled,
            _ if own_stop.timed_out() => {
                partial = true;
                CollectorState::TimedOut
            }
            Ok(Ok(None)) => CollectorState::Done,
            Ok(Err(CollectorError::Failed(message))) => CollectorState::Error(message),
            Ok(Err(CollectorError::Unsupported)) => {
                CollectorState::Error(Message::new("engine.collector_unsupported"))
            }
            Err(_) => CollectorState::Error(Message::new("engine.collector_panicked")),
        };
        if state == CollectorState::Cancelled {
            found.clear();
        }
        on_event(&id, &state);

        let report = CollectorReport {
            id,
            state,
            duration: started.elapsed(),
            items: found.len(),
            partial,
            rejected,
        };
        (report, found)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collector::tests::{FakeCollector, NoCommands};
    use crate::collector::{CleanOutcome, CollectorDescriptor, Revalidation};
    use crate::ports::SystemClock;
    use crate::risk::Destination;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn inputs() -> AnalysisInputs {
        AnalysisInputs {
            roots: vec![],
            clock: Arc::new(SystemClock),
            commands: Arc::new(NoCommands),
        }
    }

    fn quiet() -> EventFn {
        Arc::new(|_, _| {})
    }

    fn engine(collectors: Vec<Arc<dyn Collector>>, config: EngineConfig) -> Engine {
        let mut registry = Registry::default();
        for c in collectors {
            registry.register(c).unwrap();
        }
        Engine::new(registry, config)
    }

    /// Wraps a fake collector with a custom analysis behavior.
    struct Behaving<F: Fn(&AnalysisContext, &mut dyn FnMut(Item)) -> Result<(), CollectorError>> {
        inner: FakeCollector,
        behavior: F,
        detections: AtomicUsize,
    }

    impl<F> Behaving<F>
    where
        F: Fn(&AnalysisContext, &mut dyn FnMut(Item)) -> Result<(), CollectorError> + Send + Sync,
    {
        fn new(id: &'static str, behavior: F) -> Arc<Self> {
            Arc::new(Self {
                inner: FakeCollector::new(id, vec![]),
                behavior,
                detections: AtomicUsize::new(0),
            })
        }
    }

    impl<F> Collector for Behaving<F>
    where
        F: Fn(&AnalysisContext, &mut dyn FnMut(Item)) -> Result<(), CollectorError> + Send + Sync,
    {
        fn descriptor(&self) -> CollectorDescriptor {
            self.inner.descriptor()
        }
        fn detect(&self, ctx: &AnalysisContext) -> Detection {
            self.detections.fetch_add(1, Ordering::SeqCst);
            self.inner.detect(ctx)
        }
        fn analyze(
            &self,
            ctx: &AnalysisContext,
            emit: &mut dyn FnMut(Item),
        ) -> Result<(), CollectorError> {
            (self.behavior)(ctx, emit)
        }
        fn revalidate(&self, ctx: &AnalysisContext, item: &Item) -> Revalidation {
            self.inner.revalidate(ctx, item)
        }
        fn clean(&self, item: &Item, d: Destination) -> Result<CleanOutcome, CollectorError> {
            self.inner.clean(item, d)
        }
    }

    fn state_of<'a>(result: &'a AnalysisResult, id: &str) -> &'a CollectorReport {
        result.collectors.iter().find(|c| c.id == id).unwrap()
    }

    #[test]
    fn t01_05_ca1_one_failing_collector_does_not_stop_the_others() {
        let failing = Behaving::new("falha", |_, _| {
            Err(CollectorError::Failed(Message::new("erro.disco")))
        });
        let panicking = Behaving::new("panico", |_, _| panic!("bug no coletor"));
        let engine = engine(
            vec![
                Arc::new(FakeCollector::new("a", vec!["x"])),
                failing,
                panicking,
                Arc::new(FakeCollector::new("b", vec!["y", "z"])),
            ],
            EngineConfig::default(),
        );

        let result = engine.analyze(&inputs(), quiet());
        assert_eq!(state_of(&result, "a").state, CollectorState::Done);
        assert_eq!(state_of(&result, "b").state, CollectorState::Done);
        assert_eq!(
            state_of(&result, "falha").state,
            CollectorState::Error(Message::new("erro.disco"))
        );
        assert_eq!(
            state_of(&result, "panico").state,
            CollectorState::Error(Message::new("engine.collector_panicked"))
        );
        assert_eq!(result.items.len(), 3);
    }

    #[test]
    fn t01_05_ca2_collector_over_the_time_limit_keeps_partial_items() {
        let slow = Behaving::new("lento", |ctx, emit| {
            let fake = FakeCollector::new("lento", vec![]);
            emit(fake.item("primeiro"));
            while !ctx.should_stop() {
                thread::sleep(Duration::from_millis(5));
            }
            Ok(())
        });
        let engine = engine(
            vec![slow],
            EngineConfig {
                collector_timeout: Duration::from_millis(50),
                max_parallel: 2,
            },
        );

        let result = engine.analyze(&inputs(), quiet());
        let report = state_of(&result, "lento");
        assert_eq!(report.state, CollectorState::TimedOut);
        assert!(report.partial);
        assert_eq!(report.items, 1);
        assert_eq!(result.items.len(), 1);
    }

    #[test]
    fn t01_05_ca3_cancel_stops_within_two_seconds_and_keeps_the_previous_result() {
        let engine = Arc::new(engine(
            vec![Behaving::new("longo", |ctx, _| {
                while !ctx.should_stop() {
                    thread::sleep(Duration::from_millis(5));
                }
                Ok(())
            })],
            EngineConfig::default(),
        ));
        // A previous, complete analysis would never end here, so seed one by hand.
        let previous = Arc::new(AnalysisResult {
            started_at: SystemTime::UNIX_EPOCH,
            duration: Duration::ZERO,
            cancelled: false,
            collectors: vec![],
            items: vec![],
        });
        engine.state.lock().unwrap().last = Some(Arc::clone(&previous));

        let runner = {
            let engine = Arc::clone(&engine);
            thread::spawn(move || engine.analyze(&inputs(), quiet()))
        };
        while !engine.is_running() {
            thread::sleep(Duration::from_millis(1));
        }
        let asked = Instant::now();
        engine.cancel();
        let result = runner.join().unwrap();

        assert!(asked.elapsed() < Duration::from_secs(2));
        assert!(result.cancelled);
        assert_eq!(state_of(&result, "longo").state, CollectorState::Cancelled);
        assert_eq!(engine.last_result(), Some(previous));
    }

    #[test]
    fn t01_05_ca4_a_second_request_joins_the_running_analysis() {
        let gate = Arc::new((Mutex::new(false), Condvar::new()));
        let held = Arc::clone(&gate);
        let collector = Behaving::new("espera", move |_, _| {
            let (open, cv) = &*held;
            let mut open = open.lock().unwrap();
            while !*open {
                open = cv.wait(open).unwrap();
            }
            Ok(())
        });
        let engine = Arc::new(engine(vec![collector.clone()], EngineConfig::default()));

        let first = {
            let engine = Arc::clone(&engine);
            thread::spawn(move || engine.analyze(&inputs(), quiet()))
        };
        while !engine.is_running() {
            thread::sleep(Duration::from_millis(1));
        }
        let second = {
            let engine = Arc::clone(&engine);
            thread::spawn(move || engine.analyze(&inputs(), quiet()))
        };
        thread::sleep(Duration::from_millis(50));
        *gate.0.lock().unwrap() = true;
        gate.1.notify_all();

        let (a, b) = (first.join().unwrap(), second.join().unwrap());
        assert!(Arc::ptr_eq(&a, &b), "both callers get the same analysis");
        assert_eq!(collector.detections.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn t01_05_ca5_result_has_the_state_and_duration_of_every_collector() {
        let engine = engine(
            vec![
                Arc::new(FakeCollector::new("a", vec!["x"])),
                Arc::new(FakeCollector::new("b", vec![])),
            ],
            EngineConfig::default(),
        );
        let result = engine.analyze(&inputs(), quiet());

        assert_eq!(
            result
                .collectors
                .iter()
                .map(|c| c.id.as_str())
                .collect::<Vec<_>>(),
            vec!["a", "b"]
        );
        assert!(
            result
                .collectors
                .iter()
                .all(|c| c.state == CollectorState::Done)
        );
        assert!(
            result
                .collectors
                .iter()
                .all(|c| c.duration <= result.duration)
        );
        assert_eq!(engine.last_result(), Some(result));
    }

    #[test]
    fn unavailable_and_missing_collectors_are_reported_without_items() {
        let mut stopped = FakeCollector::new("docker", vec!["nunca"]);
        stopped.detection = Detection::Unavailable {
            reason: Message::new("docker.stopped"),
            action: Some(Message::new("docker.start")),
        };
        let mut missing = FakeCollector::new("ollama", vec!["nunca"]);
        missing.detection = Detection::NotInstalled;
        let engine = engine(
            vec![Arc::new(stopped), Arc::new(missing)],
            EngineConfig::default(),
        );

        let result = engine.analyze(&inputs(), quiet());
        assert_eq!(
            state_of(&result, "docker").state,
            CollectorState::Unavailable {
                reason: Message::new("docker.stopped"),
                action: Some(Message::new("docker.start"))
            }
        );
        assert_eq!(
            state_of(&result, "ollama").state,
            CollectorState::NotInstalled
        );
        assert!(result.items.is_empty());
    }

    #[test]
    fn items_of_undeclared_kinds_are_rejected() {
        let rogue = Behaving::new("c", |_, emit| {
            emit(FakeCollector::new("outro", vec![]).item("intruso"));
            emit(FakeCollector::new("c", vec![]).item("valido"));
            Ok(())
        });
        let result = engine(vec![rogue], EngineConfig::default()).analyze(&inputs(), quiet());
        let report = state_of(&result, "c");
        assert_eq!((report.items, report.rejected), (1, 1));
    }

    #[test]
    fn events_follow_each_collector_from_waiting_to_done() {
        let events = Arc::new(Mutex::new(vec![]));
        let sink = Arc::clone(&events);
        let engine = engine(
            vec![Arc::new(FakeCollector::new("a", vec!["x"]))],
            EngineConfig::default(),
        );
        engine.analyze(
            &inputs(),
            Arc::new(move |id, state| sink.lock().unwrap().push((id.to_owned(), state.clone()))),
        );
        let events = events.lock().unwrap();
        assert_eq!(events.first().unwrap().1, CollectorState::Waiting);
        assert!(
            events
                .iter()
                .any(|(_, s)| matches!(s, CollectorState::Analyzing(_)))
        );
        assert_eq!(events.last().unwrap().1, CollectorState::Done);
    }

    #[test]
    fn collectors_run_in_parallel_up_to_the_limit() {
        let running = Arc::new(AtomicUsize::new(0));
        let peak = Arc::new(AtomicUsize::new(0));
        let make = |id: &'static str| {
            let (running, peak) = (Arc::clone(&running), Arc::clone(&peak));
            Behaving::new(id, move |_, _| {
                let now = running.fetch_add(1, Ordering::SeqCst) + 1;
                peak.fetch_max(now, Ordering::SeqCst);
                thread::sleep(Duration::from_millis(40));
                running.fetch_sub(1, Ordering::SeqCst);
                Ok(())
            })
        };
        let collectors: Vec<Arc<dyn Collector>> =
            vec![make("a"), make("b"), make("c"), make("d"), make("e")];
        let engine = engine(
            collectors,
            EngineConfig {
                collector_timeout: Duration::from_secs(5),
                max_parallel: 2,
            },
        );
        engine.analyze(&inputs(), quiet());
        assert_eq!(peak.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn empty_registry_gives_an_empty_result() {
        let result = engine(vec![], EngineConfig::default()).analyze(&inputs(), quiet());
        assert!(result.collectors.is_empty() && result.items.is_empty() && !result.cancelled);
    }
}
