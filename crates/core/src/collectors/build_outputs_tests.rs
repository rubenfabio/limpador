use super::*;
use crate::adapters::{SystemCommandRunner, native_platform};
use crate::collector::{StopSignal, tests::context};
use crate::collectors::scripted::Scripted;
use crate::config::DEFAULT_BUILD_FOLDERS;
use crate::projects::{DiscoveryOptions, discover};
use crate::risk::{Signal, assess};
use limpador_testkit::{FakeMachine, LockedFile, ProjectSpec};
use std::fs;

fn file(path: &Path, bytes: usize) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, vec![b'x'; bytes]).unwrap();
}

fn defaults() -> Vec<String> {
    DEFAULT_BUILD_FOLDERS
        .iter()
        .map(|f| (*f).to_owned())
        .collect()
}

fn collector_with(folders: Vec<String>) -> BuildOutputsCollector {
    BuildOutputsCollector::new(Arc::new(SystemCommandRunner), native_platform(), folders)
}

fn collector() -> BuildOutputsCollector {
    collector_with(defaults())
}

fn ctx(machine: &FakeMachine) -> AnalysisContext {
    let discovery = discover(&[machine.roots()], &DiscoveryOptions::default());
    context(StopSignal::new(None)).with_discovery(Arc::new(discovery))
}

fn analyze_with(collector: &BuildOutputsCollector, machine: &FakeMachine) -> Vec<Item> {
    let mut items = vec![];
    collector
        .analyze(&ctx(machine), &mut |i| items.push(i))
        .unwrap();
    items
}

fn analyze(machine: &FakeMachine) -> Vec<Item> {
    analyze_with(&collector(), machine)
}

/// Node project with git whose `.gitignore` lists `ignored`.
fn git_project(machine: &FakeMachine, name: &str, ignored: &str) -> PathBuf {
    let root = machine
        .project(&ProjectSpec::node(name).with_git())
        .unwrap();
    machine
        .repo(&root)
        .commit_file(".gitignore", ignored, "ignore", machine.clock().days_ago(1))
        .unwrap();
    root
}

#[test]
fn t04_02_ca1_a_versioned_dist_is_never_a_candidate() {
    let machine = FakeMachine::new().unwrap();
    let root = machine
        .project(&ProjectSpec::node("lib").with_git())
        .unwrap();
    let repo = machine.repo(&root);
    // Published package: dist is committed, and only later added to .gitignore.
    repo.commit_file("dist/index.js", "x", "build", machine.clock().days_ago(2))
        .unwrap();
    repo.commit_file(
        ".gitignore",
        "dist/\n",
        "ignore",
        machine.clock().days_ago(1),
    )
    .unwrap();
    file(&root.join("dist/novo.js"), 10);
    // Neither versioned nor ignored: not a candidate either.
    file(&root.join("out/x.js"), 10);

    assert!(analyze(&machine).is_empty());
}

#[test]
fn t04_02_ca2_an_ignored_next_folder_is_a_moderate_candidate() {
    let machine = FakeMachine::new().unwrap();
    let root = git_project(&machine, "site", ".next/\nnode_modules/\n");
    file(&root.join(".next/cache/a.bin"), 300);
    file(&root.join(".next/server/page.js"), 50);

    let items = analyze(&machine);
    assert_eq!(items.len(), 1);
    let item = &items[0];
    assert_eq!(item.parts, vec![root.join(".next")]);
    assert_eq!((item.apparent_size, item.freeable_size), (350, 350));
    assert_eq!(item.risk, Risk::Moderate);
    assert_eq!(item.name, "site · build");
    assert_eq!(item.resource, Resource::Path(root.clone()));
    assert_eq!(
        item.reason,
        Message::new("reason.build_outputs").with("count", 1)
    );
    assert_eq!(assess(item, None).destination, Some(Destination::Direct));
}

#[test]
fn t04_02_ca3_without_git_build_is_not_a_candidate() {
    let machine = FakeMachine::new().unwrap();
    let root = machine.project(&ProjectSpec::node("solto")).unwrap();
    for folder in [
        "build",
        "dist",
        "out",
        "coverage",
        ".output",
        "storybook-static",
    ] {
        file(&root.join(folder).join("x.js"), 1);
    }
    assert!(analyze(&machine).is_empty());
}

#[test]
fn t04_02_ca4_without_git_unambiguous_names_are_candidates() {
    let machine = FakeMachine::new().unwrap();
    let root = machine.project(&ProjectSpec::node("solto")).unwrap();
    file(&root.join(".next/a.js"), 5);
    file(&root.join(".angular/cache/b.js"), 7);
    file(&root.join(".angular/config.json"), 1);
    file(&root.join("build/c.js"), 9);

    let items = analyze(&machine);
    assert_eq!(items.len(), 1);
    assert_eq!(
        items[0].parts,
        vec![root.join(".angular").join("cache"), root.join(".next")]
    );
    assert_eq!(items[0].apparent_size, 12);
}

#[test]
fn t04_02_ca5_the_build_script_says_how_to_recreate() {
    let machine = FakeMachine::new().unwrap();
    let root = machine.project(&ProjectSpec::node("app")).unwrap();
    file(&root.join(".turbo/x"), 1);
    let recreate = |machine: &FakeMachine| analyze(machine)[0].recreate.clone().unwrap();

    assert_eq!(recreate(&machine), Message::new("recreate.next_build"));
    fs::write(
        root.join("package.json"),
        r#"{"name": "app", "scripts": {"build": "turbo build"}}"#,
    )
    .unwrap();
    assert_eq!(
        recreate(&machine),
        Message::new("recreate.run_command").with("command", "npm run build")
    );
    fs::write(root.join("pnpm-lock.yaml"), "").unwrap();
    assert_eq!(
        recreate(&machine),
        Message::new("recreate.run_command").with("command", "pnpm run build")
    );
}

#[test]
fn monorepo_packages_are_grouped_and_dependencies_skipped() {
    let machine = FakeMachine::new().unwrap();
    let root = git_project(&machine, "mono", "dist/\n.next/\nnode_modules/\n");
    file(&root.join("packages/web/.next/a.js"), 1);
    file(&root.join("packages/lib/dist/b.js"), 2);
    // Inside node_modules: belongs to the dependencies, not to the builds.
    file(&root.join("node_modules/pkg/dist/c.js"), 4);

    let items = analyze(&machine);
    assert_eq!(items.len(), 1);
    assert_eq!(
        items[0].parts,
        vec![
            root.join("packages").join("lib").join("dist"),
            root.join("packages").join("web").join(".next"),
        ]
    );
    assert_eq!(items[0].reason.params["count"], "2");
}

#[test]
fn accented_folder_names_are_checked_against_git_too() {
    let machine = FakeMachine::new().unwrap();
    let root = machine
        .project(&ProjectSpec::node("acentos").with_git())
        .unwrap();
    let repo = machine.repo(&root);
    repo.commit_file(
        "pacotes/ação/dist/index.js",
        "x",
        "versionado",
        machine.clock().days_ago(2),
    )
    .unwrap();
    repo.commit_file(
        ".gitignore",
        "dist/\n",
        "ignore",
        machine.clock().days_ago(1),
    )
    .unwrap();
    file(&root.join("pacotes/ação/dist/novo.js"), 3);
    file(&root.join("pacotes/café/dist/livre.js"), 4);

    let items = analyze(&machine);
    assert_eq!(items.len(), 1);
    assert_eq!(
        items[0].parts,
        vec![root.join("pacotes").join("café").join("dist")],
        "the versioned one under an accented name stays"
    );
}

#[test]
fn the_folder_list_comes_from_the_preferences() {
    let machine = FakeMachine::new().unwrap();
    let root = git_project(&machine, "py", "saida/\n.next/\n");
    file(&root.join("saida/a"), 1);
    file(&root.join(".next/b"), 1);

    let items = analyze_with(&collector_with(vec!["saida".into()]), &machine);
    assert_eq!(items[0].parts, vec![root.join("saida")]);
}

#[test]
fn a_git_failure_offers_nothing() {
    let machine = FakeMachine::new().unwrap();
    let root = git_project(&machine, "site", ".next/\n");
    file(&root.join(".next/a"), 1);
    let broken =
        BuildOutputsCollector::new(Arc::new(Scripted::default()), native_platform(), defaults());
    assert!(analyze_with(&broken, &machine).is_empty());
}

#[test]
fn revalidation_blocks_outputs_in_use_and_sees_them_gone() {
    let machine = FakeMachine::new().unwrap();
    let root = machine.project(&ProjectSpec::node("dev")).unwrap();
    file(&root.join(".next/server/app.js"), 3);
    let item = analyze(&machine).remove(0);

    let Revalidation::StillValid(free) = collector().revalidate(&ctx(&machine), &item) else {
        panic!("still there");
    };
    assert!(free.signals.is_empty());
    {
        let _dev = LockedFile::open(&root.join(".next/server/app.js")).unwrap();
        let Revalidation::StillValid(held) = collector().revalidate(&ctx(&machine), &item) else {
            panic!("still there");
        };
        if cfg!(windows) {
            assert!(matches!(held.signals[..], [Signal::InUse(_)]));
            assert_eq!(assess(&held, None).risk, Risk::Blocked);
        }
    }
    // Revalidating a project no longer under the roots still works.
    let empty = context(StopSignal::new(None));
    assert!(matches!(
        collector().revalidate(&empty, &item),
        Revalidation::StillValid(_)
    ));

    fs::remove_dir_all(root.join(".next")).unwrap();
    assert_eq!(
        collector().revalidate(&ctx(&machine), &item),
        Revalidation::Gone
    );
    let mut orphan = item.clone();
    orphan.project_hint = None;
    assert_eq!(
        collector().revalidate(&ctx(&machine), &orphan),
        Revalidation::Gone
    );
}

#[test]
fn cleaning_removes_only_recognized_folders_of_the_project() {
    let machine = FakeMachine::new().unwrap();
    let root = machine.project(&ProjectSpec::node("dev")).unwrap();
    file(&root.join(".next/a.js"), 3);
    file(&root.join(".vite/b.js"), 4);
    let item = analyze(&machine).remove(0);

    assert_eq!(
        collector().clean(&item, Destination::Quarantine),
        Err(CollectorError::Unsupported)
    );
    let refused = Err(CollectorError::Failed(Message::new("clean.outside_item")));
    for parts in [
        vec![root.join("src")],
        vec![root.clone()],
        vec![machine.path("outro").join(".next")],
        vec![],
    ] {
        let mut tampered = item.clone();
        tampered.parts = parts;
        assert_eq!(collector().clean(&tampered, Destination::Direct), refused);
    }
    let mut no_project = item.clone();
    no_project.project_hint = None;
    assert_eq!(collector().clean(&no_project, Destination::Direct), refused);

    assert_eq!(
        collector().clean(&item, Destination::Direct),
        Ok(CleanOutcome::Cleaned { freed_bytes: 7 })
    );
    assert!(!root.join(".next").exists() && !root.join(".vite").exists());
    assert!(root.join("src").exists() && root.join("package.json").exists());
    let Err(CollectorError::Failed(failure)) = collector().clean(&item, Destination::Direct) else {
        panic!("already removed");
    };
    assert_eq!(failure.key, "clean.remove_failed");
}

#[test]
fn progress_is_reported_and_cancellation_stops() {
    let machine = FakeMachine::new().unwrap();
    machine.project(&ProjectSpec::node("a")).unwrap();
    machine.project(&ProjectSpec::node("b")).unwrap();
    let progress = Arc::new(std::sync::Mutex::new(vec![]));
    let sink = Arc::clone(&progress);
    let discovery = Arc::new(discover(&[machine.roots()], &DiscoveryOptions::default()));
    let ctx = AnalysisContext::new(
        vec![],
        Arc::new(crate::adapters::SystemClock),
        Arc::new(crate::collector::tests::NoCommands),
        StopSignal::new(None),
        Arc::new(move |p| sink.lock().unwrap().push((p.done, p.total))),
    )
    .with_discovery(Arc::clone(&discovery));
    collector().analyze(&ctx, &mut |_| {}).unwrap();
    assert_eq!(*progress.lock().unwrap(), vec![(1, Some(2)), (2, Some(2))]);
    assert_eq!(collector().detect(&ctx), Detection::Available);

    let root = machine.roots().join("a");
    file(&root.join(".next/x"), 1);
    let stopped = context(StopSignal::new(None)).with_discovery(discovery);
    stopped.stop.cancel();
    let mut items = vec![];
    collector()
        .analyze(&stopped, &mut |i| items.push(i))
        .unwrap();
    assert!(items.is_empty());
}
