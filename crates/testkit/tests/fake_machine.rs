use limpador_core::ports::Clock;
use limpador_testkit::{FakeMachine, LockedFile, ProjectSpec, create_junction};
use std::fs;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::PathBuf;
use std::time::UNIX_EPOCH;

fn epoch_secs(t: std::time::SystemTime) -> u64 {
    t.duration_since(UNIX_EPOCH).unwrap().as_secs()
}

#[test]
fn t00_02_ca1_inactive_project_has_files_and_last_commit_dated_in_the_past() {
    let machine = FakeMachine::new().unwrap();
    let expected = machine.clock().days_ago(45);

    let dir = machine
        .project(&ProjectSpec::node("api").inactive_days(45).with_git())
        .unwrap();

    for file in ["package.json", "src/index.js"] {
        let modified = fs::metadata(dir.join(file)).unwrap().modified().unwrap();
        assert_eq!(epoch_secs(modified), epoch_secs(expected), "{file}");
    }
    let last_commit: u64 = machine
        .repo(&dir)
        .git(&["log", "-1", "--format=%ct"])
        .unwrap()
        .parse()
        .unwrap();
    assert_eq!(last_commit, epoch_secs(expected));
    assert!(machine.clock().now() > expected);
}

#[test]
fn t00_02_ca1_projects_without_git_also_get_controlled_dates() {
    let machine = FakeMachine::new().unwrap();
    let dir = machine
        .project(&ProjectSpec::python("script").inactive_days(90))
        .unwrap();

    let modified = fs::metadata(dir.join("main.py"))
        .unwrap()
        .modified()
        .unwrap();
    assert_eq!(
        epoch_secs(modified),
        epoch_secs(machine.clock().days_ago(90))
    );
    assert!(!dir.join(".git").exists());
}

#[test]
fn t00_02_ca2_squash_scenario_has_branch_squash_commit_and_gone_upstream() {
    let machine = FakeMachine::new().unwrap();
    let scenario = machine.squash_merged_branch("loja", "feature/x").unwrap();
    let repo = &scenario.repo;

    assert_eq!(
        repo.git(&["branch", "--list", "feature/x", "--format=%(refname:short)"])
            .unwrap(),
        "feature/x"
    );
    assert_eq!(
        repo.git(&["log", "main", "-1", "--format=%s"]).unwrap(),
        "feature/x (#1)"
    );
    // The squash commit carries the branch content but is not its ancestor.
    assert_eq!(repo.git(&["show", "main:feature.txt"]).unwrap(), "one\ntwo");
    assert!(
        repo.git(&["merge-base", "--is-ancestor", "feature/x", "main"])
            .is_err()
    );
    assert!(
        repo.git(&[
            "rev-parse",
            "--verify",
            "--quiet",
            "refs/remotes/origin/feature/x"
        ])
        .is_err()
    );
    assert_eq!(
        repo.git(&[
            "branch",
            "--list",
            "feature/x",
            "--format=%(upstream:track)"
        ])
        .unwrap(),
        "[gone]"
    );
}

#[test]
fn t00_02_ca3_temp_folder_is_removed_even_when_the_test_panics() {
    let mut root: Option<PathBuf> = None;
    let result = catch_unwind(AssertUnwindSafe(|| {
        let machine = FakeMachine::new().unwrap();
        machine
            .project(&ProjectSpec::rust("cli").with_git())
            .unwrap();
        root = Some(machine.root().to_owned());
        panic!("falha simulada no meio do teste");
    }));

    assert!(result.is_err());
    let root = root.unwrap();
    assert!(!root.exists(), "{} still exists", root.display());
}

#[test]
fn t00_02_ca6_junction_target_outside_the_machine_survives_cleanup() {
    let outside = tempfile::tempdir().unwrap();
    let precious = outside.path().join("precioso.txt");
    fs::write(&precious, "não apagar").unwrap();

    {
        let machine = FakeMachine::new().unwrap();
        let link = machine.roots().join("atalho");
        create_junction(&link, outside.path()).unwrap();
        assert_eq!(
            fs::read_to_string(link.join("precioso.txt")).unwrap(),
            "não apagar"
        );
    }

    assert_eq!(fs::read_to_string(&precious).unwrap(), "não apagar");
}

#[test]
fn names_with_spaces_and_accents_are_supported() {
    let machine = FakeMachine::new().unwrap();
    let dir = machine
        .project(&ProjectSpec::node("projeto com espaço e acentuação").with_git())
        .unwrap();
    assert!(dir.join("package.json").exists());
}

#[test]
fn paths_longer_than_260_characters_are_created_and_cleaned() {
    let root;
    {
        let machine = FakeMachine::new().unwrap();
        let file = machine.long_path_file().unwrap();
        assert!(file.as_os_str().len() > 260);
        assert_eq!(fs::read_to_string(&file).unwrap(), "conteúdo\n");
        root = machine.root().to_owned();
    }
    assert!(!root.exists());
}

#[cfg(windows)]
#[test]
fn locked_file_cannot_be_deleted_until_released() {
    let machine = FakeMachine::new().unwrap();
    let file = machine
        .write_file("em-uso.log", "x", machine.clock().now())
        .unwrap();

    let lock = LockedFile::open(&file).unwrap();
    assert!(fs::remove_file(&file).is_err());
    drop(lock);
    assert!(fs::remove_file(&file).is_ok());
}
