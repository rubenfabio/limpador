//! Protection rules: they win over every other rule (T-02.04, I10).
//! Critical code: mutation target ≥ 90% (mutation/politica.json).

use crate::config::Protections;
use crate::glob;
use crate::item::{Category, Item, Resource};
use crate::message::Message;
use crate::roots::{contains, path_key};
use std::path::{Path, PathBuf};

/// The rule that protects `item`, if any. Protected items leave the ranking and show up
/// in "Protegidos" with this rule.
pub fn protection_for(item: &Item, rules: &Protections) -> Option<Message> {
    let paths = [resource_path(item), item.project_hint.as_deref()];
    for path in paths.into_iter().flatten() {
        if let Some(rule) = rules.protected_paths.iter().find(|r| contains(r, path)) {
            return Some(Message::new("protection.path").with("path", rule.display()));
        }
    }
    let Resource::Name(name) = &item.resource else {
        return None;
    };
    let (patterns, key) = match (item.category, item.kind.as_str()) {
        (Category::Git, "branch") => (&rules.protected_branches, "protection.branch"),
        (Category::Docker, "image" | "volume") => (&rules.protected_docker, "protection.docker"),
        _ => return None,
    };
    patterns
        .iter()
        .find(|pattern| glob::matches(pattern, name))
        .map(|pattern| Message::new(key).with("pattern", pattern))
}

fn resource_path(item: &Item) -> Option<&Path> {
    match &item.resource {
        Resource::Path(path) => Some(path),
        Resource::Name(_) => None,
    }
}

/// "Proteger isto": adds the rule that protects exactly this item and returns it.
pub fn protect(item: &Item, rules: &mut Protections) -> Message {
    match (&item.resource, item.category) {
        (Resource::Path(path), _) => add_path(&mut rules.protected_paths, path),
        (Resource::Name(name), Category::Git) => add_name(&mut rules.protected_branches, name),
        (Resource::Name(name), Category::Docker) => add_name(&mut rules.protected_docker, name),
        (Resource::Name(_), _) => match &item.project_hint {
            Some(path) => add_path(&mut rules.protected_paths, path),
            None => add_name(&mut rules.protected_docker, &item.name),
        },
    };
    protection_for(item, rules).expect("the new rule protects the item")
}

fn add_path(list: &mut Vec<PathBuf>, path: &Path) {
    if !list.iter().any(|p| path_key(p) == path_key(path)) {
        list.push(path.to_owned());
    }
}

fn add_name(list: &mut Vec<String>, name: &str) {
    if !list.iter().any(|n| n == name) {
        list.push(name.to_owned());
    }
}

pub fn is_pinned(project_root: &Path, rules: &Protections) -> bool {
    rules
        .pinned_projects
        .iter()
        .any(|p| path_key(p) == path_key(project_root))
}

/// Pins or unpins a project; returns whether anything changed.
pub fn set_pinned(project_root: &Path, pinned: bool, rules: &mut Protections) -> bool {
    let was = is_pinned(project_root, rules);
    if pinned && !was {
        rules.pinned_projects.push(project_root.to_owned());
    } else if !pinned && was {
        rules
            .pinned_projects
            .retain(|p| path_key(p) != path_key(project_root));
    }
    was != pinned
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::activity::{ActivityRules, ProjectState, state};
    use crate::adapters::TomlConfigRepository;
    use crate::config::Config;
    use crate::item::{ItemDraft, Nature, Risk};
    use crate::ports::ConfigRepository;
    use crate::projects::{Project, ProjectGit};
    use crate::ranking::{RankedItem, rank};
    use crate::risk::assess;
    use std::time::{Duration, SystemTime};

    fn item(category: Category, kind: &str, resource: Resource) -> Item {
        let name = match &resource {
            Resource::Path(p) => p.display().to_string(),
            Resource::Name(n) => n.clone(),
        };
        ItemDraft::new(
            "t",
            kind,
            category,
            resource,
            name,
            Nature::UserContent,
            Risk::Safe,
        )
        .sizes(10, 10)
        .reason(Message::new("reason"))
        .build()
        .unwrap()
    }

    fn path_item(path: &str) -> Item {
        item(
            Category::DependenciesAndBuilds,
            "node_modules",
            Resource::Path(path.into()),
        )
    }

    fn branch(name: &str) -> Item {
        item(Category::Git, "branch", Resource::Name(name.into()))
    }

    fn volume(name: &str) -> Item {
        item(Category::Docker, "volume", Resource::Name(name.into()))
    }

    fn protections() -> Protections {
        Protections {
            protected_paths: vec!["C:\\Projetos\\tcc".into()],
            protected_docker: vec!["postgres-dados-*".into()],
            ..Protections::default()
        }
    }

    #[test]
    fn t02_04_ca1_items_under_a_protected_path_leave_the_ranking() {
        let rules = protections();
        let inside = path_item("C:\\Projetos\\tcc\\node_modules");
        let outside = path_item("C:\\Projetos\\loja\\node_modules");

        let entries = [inside, outside]
            .into_iter()
            .map(|i| {
                let a = assess(&i, protection_for(&i, &rules).as_ref());
                RankedItem::new(i, a, None, None)
            })
            .collect();
        let ranking = rank(entries);

        assert_eq!(ranking.space.len(), 1);
        assert_eq!(ranking.protected.len(), 1);
        assert_eq!(
            ranking.protected[0].assessment.protected_by,
            Some(Message::new("protection.path").with("path", "C:\\Projetos\\tcc"))
        );
    }

    #[test]
    fn items_of_a_protected_repository_are_protected_through_their_project() {
        let rules = protections();
        let mut feature = branch("feature/x");
        feature.project_hint = Some("C:\\Projetos\\tcc".into());
        assert!(protection_for(&feature, &rules).is_some());
    }

    #[test]
    fn t02_04_ca2_release_branches_are_protected_by_default() {
        let rules = Protections::default();
        assert_eq!(
            protection_for(&branch("release/2.0"), &rules),
            Some(Message::new("protection.branch").with("pattern", "release/*"))
        );
        for name in ["main", "master", "develop", "dev", "hotfix/urgente"] {
            assert!(protection_for(&branch(name), &rules).is_some(), "{name}");
        }
        assert!(protection_for(&branch("feature/login"), &rules).is_none());
    }

    #[test]
    fn t02_04_ca3_docker_volume_matching_a_pattern_is_protected() {
        let rules = protections();
        assert_eq!(
            protection_for(&volume("postgres-dados-pessoais"), &rules),
            Some(Message::new("protection.docker").with("pattern", "postgres-dados-*"))
        );
        assert!(protection_for(&volume("redis-cache"), &rules).is_none());
    }

    #[test]
    fn branch_patterns_do_not_protect_docker_names_and_vice_versa() {
        let rules = Protections {
            protected_docker: vec!["main".into()],
            ..Protections::default()
        };
        assert!(protection_for(&volume("main"), &rules).is_some());
        let other = item(
            Category::AiAndEditors,
            "model",
            Resource::Name("main".into()),
        );
        assert!(protection_for(&other, &rules).is_none());
    }

    #[test]
    fn t02_04_ca4_protect_this_takes_effect_now_and_is_saved() {
        let dir = tempfile::tempdir().unwrap();
        let store = TomlConfigRepository::new(dir.path());
        let mut config = Config::default();
        let target = volume("meu-banco");
        assert!(protection_for(&target, &config.protections).is_none());

        let rule = protect(&target, &mut config.protections);
        assert_eq!(
            rule,
            Message::new("protection.docker").with("pattern", "meu-banco")
        );
        assert!(protection_for(&target, &config.protections).is_some());

        store.save(&config).unwrap();
        let (loaded, _) = store.load();
        assert!(protection_for(&target, &loaded.protections).is_some());
    }

    #[test]
    fn protecting_paths_and_branches_adds_the_right_rule_once() {
        let mut rules = Protections::default();
        let folder = path_item("C:\\Projetos\\x\\dist");
        protect(&folder, &mut rules);
        protect(&folder, &mut rules);
        assert_eq!(
            rules.protected_paths,
            vec![PathBuf::from("C:\\Projetos\\x\\dist")]
        );

        let feature = branch("feature/y");
        protect(&feature, &mut rules);
        assert!(rules.protected_branches.contains(&"feature/y".to_owned()));

        let mut model = item(
            Category::AiAndEditors,
            "model",
            Resource::Name("llama3".into()),
        );
        model.project_hint = Some("C:\\Projetos\\ia".into());
        protect(&model, &mut rules);
        assert!(protection_for(&model, &rules).is_some());
    }

    #[test]
    fn t02_04_ca5_pinned_project_idle_for_200_days_is_active() {
        let mut rules = Protections::default();
        let root = PathBuf::from("C:\\Projetos\\tcc");
        assert!(set_pinned(&root, true, &mut rules));
        assert!(!set_pinned(&root, true, &mut rules));
        assert!(is_pinned(&root, &rules));
        assert_eq!(
            rules.pinned_projects.len(),
            1,
            "pinning twice keeps one entry"
        );

        let project = Project {
            root: root.clone(),
            name: "tcc".into(),
            git: ProjectGit::Repository,
        };
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(400 * 86_400);
        let last = Some(now - Duration::from_secs(200 * 86_400));
        let activity = ActivityRules {
            inactivity: Duration::from_secs(30 * 86_400),
            pinned: rules.pinned_projects.clone(),
            running_containers: vec![],
        };
        assert_eq!(state(&project, last, now, &activity), ProjectState::Pinned);

        assert!(set_pinned(&root, false, &mut rules));
        assert!(!is_pinned(&root, &rules));
    }
}
