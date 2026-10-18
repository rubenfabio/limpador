//! The cleanable item: the common language between collectors, engine, UI, quarantine
//! and history (T-01.01).

use crate::message::Message;
use crate::risk::Signal;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Category {
    Docker,
    DependenciesAndBuilds,
    Git,
    AiAndEditors,
    /// Not development related (Downloads, Temp…); shown apart from the others.
    GeneralFiles,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Nature {
    /// Can be rebuilt or downloaded again (caches, dependencies, builds).
    Recreatable,
    /// May hold data that exists nowhere else (branches, volumes, personal files).
    UserContent,
}

/// Risk levels, ordered by severity: `Safe < Moderate < Caution < Blocked`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Risk {
    /// 🟢 Recreated automatically, without user action and without loss.
    Safe,
    /// 🟡 Recreatable, but costs the user an action or time.
    Moderate,
    /// 🔴 May hold data that only exists there.
    Caution,
    /// ⛔ In use, unsaved work, or state that prevents a decision.
    Blocked,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CleanAction {
    /// Delete or quarantine: the destination is derived from risk and nature (T-01.02).
    Remove,
    /// Give space back without deleting content (virtual disks).
    Compact,
    /// Run the tool's own cleaning command (e.g. a package manager cache prune).
    ManagerCommand { program: String, args: Vec<String> },
}

/// What an item is: a path on disk or a named resource (image, branch, volume…).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Resource {
    Path(PathBuf),
    Name(String),
}

impl Resource {
    /// Spelling used in identifiers: paths with `/` and, on Windows (case-insensitive),
    /// in lower case, so the same folder always yields the same id.
    fn canonical(&self) -> String {
        match self {
            Self::Path(path) => canonical_path(path),
            Self::Name(name) => name.clone(),
        }
    }
}

fn canonical_path(path: &Path) -> String {
    let text = path.to_string_lossy().replace('\\', "/");
    let text = text.trim_end_matches('/').to_owned();
    if cfg!(windows) {
        text.to_lowercase()
    } else {
        text
    }
}

/// Stable identifier: the same resource gives the same id in every analysis.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ItemId(String);

impl ItemId {
    pub fn new(collector: &str, kind: &str, resource: &Resource) -> Self {
        Self(format!("{collector}/{kind}/{}", resource.canonical()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A validated cleanable item. Built only through [`ItemDraft::build`].
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Item {
    pub id: ItemId,
    pub collector: String,
    pub kind: String,
    pub category: Category,
    pub resource: Resource,
    pub name: String,
    pub apparent_size: u64,
    /// Bytes that really come back to the disk (e.g. without shared hardlinks or layers).
    pub freeable_size: u64,
    pub size_estimated: bool,
    pub nature: Nature,
    /// Risk proposed by the collector, before engine signals (T-01.02).
    pub risk: Risk,
    pub reason: Message,
    pub recreate: Option<Message>,
    /// Path used to associate the item with a project (T-01.06).
    pub project_hint: Option<PathBuf>,
    pub last_activity: Option<SystemTime>,
    pub action: CleanAction,
    pub side_effects: Vec<Message>,
    pub block_reason: Option<Message>,
    pub contains: Vec<ItemId>,
    /// Folders that make up the item when it groups several (e.g. every `node_modules`
    /// of a monorepo). Empty when the resource alone is the item.
    pub parts: Vec<PathBuf>,
    pub alerts: Vec<Message>,
    pub signals: Vec<Signal>,
}

impl Item {
    /// Items that free no space (branches, stashes, networks) count as organization.
    pub fn is_organization(&self) -> bool {
        self.freeable_size == 0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ItemError {
    MissingReason,
    MissingRecreate,
    MissingBlockReason,
    FreeableExceedsApparent,
}

/// Collects an item's data; [`ItemDraft::build`] enforces the model's rules.
#[derive(Debug, Clone)]
pub struct ItemDraft {
    item: Item,
    reason: Option<Message>,
}

impl ItemDraft {
    pub fn new(
        collector: &str,
        kind: &str,
        category: Category,
        resource: Resource,
        name: impl Into<String>,
        nature: Nature,
        risk: Risk,
    ) -> Self {
        Self {
            item: Item {
                id: ItemId::new(collector, kind, &resource),
                collector: collector.to_owned(),
                kind: kind.to_owned(),
                category,
                resource,
                name: name.into(),
                apparent_size: 0,
                freeable_size: 0,
                size_estimated: false,
                nature,
                risk,
                reason: Message::new(""),
                recreate: None,
                project_hint: None,
                last_activity: None,
                action: CleanAction::Remove,
                side_effects: Vec::new(),
                block_reason: None,
                contains: Vec::new(),
                parts: Vec::new(),
                alerts: Vec::new(),
                signals: Vec::new(),
            },
            reason: None,
        }
    }

    pub fn sizes(mut self, apparent: u64, freeable: u64) -> Self {
        self.item.apparent_size = apparent;
        self.item.freeable_size = freeable;
        self
    }

    pub fn size_estimated(mut self) -> Self {
        self.item.size_estimated = true;
        self
    }

    pub fn reason(mut self, reason: Message) -> Self {
        self.reason = Some(reason);
        self
    }

    pub fn recreate(mut self, how: Message) -> Self {
        self.item.recreate = Some(how);
        self
    }

    pub fn project_hint(mut self, path: impl Into<PathBuf>) -> Self {
        self.item.project_hint = Some(path.into());
        self
    }

    pub fn last_activity(mut self, at: SystemTime) -> Self {
        self.item.last_activity = Some(at);
        self
    }

    pub fn action(mut self, action: CleanAction) -> Self {
        self.item.action = action;
        self
    }

    pub fn side_effect(mut self, effect: Message) -> Self {
        self.item.side_effects.push(effect);
        self
    }

    pub fn block_reason(mut self, reason: Message) -> Self {
        self.item.block_reason = Some(reason);
        self
    }

    pub fn contains(mut self, child: ItemId) -> Self {
        self.item.contains.push(child);
        self
    }

    pub fn part(mut self, folder: impl Into<PathBuf>) -> Self {
        self.item.parts.push(folder.into());
        self
    }

    pub fn alert(mut self, alert: Message) -> Self {
        self.item.alerts.push(alert);
        self
    }

    pub fn signal(mut self, signal: Signal) -> Self {
        self.item.signals.push(signal);
        self
    }

    pub fn build(self) -> Result<Item, ItemError> {
        let mut item = self.item;
        item.reason = self.reason.ok_or(ItemError::MissingReason)?;
        if item.nature == Nature::Recreatable && item.recreate.is_none() {
            return Err(ItemError::MissingRecreate);
        }
        if item.risk == Risk::Blocked && item.block_reason.is_none() {
            return Err(ItemError::MissingBlockReason);
        }
        if item.freeable_size > item.apparent_size {
            return Err(ItemError::FreeableExceedsApparent);
        }
        Ok(item)
    }
}

/// Items not contained in another item of the same set: summing these never counts
/// the same bytes twice (a worktree and the `node_modules` inside it).
pub fn top_level(items: &[Item]) -> Vec<&Item> {
    let present: HashSet<&ItemId> = items.iter().map(|i| &i.id).collect();
    let nested: HashSet<&ItemId> = items
        .iter()
        .flat_map(|i| i.contains.iter())
        .filter(|child| present.contains(child))
        .collect();
    items.iter().filter(|i| !nested.contains(&i.id)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const GB: u64 = 1024 * 1024 * 1024;

    fn draft(kind: &str, path: &str, nature: Nature, risk: Risk) -> ItemDraft {
        ItemDraft::new(
            "teste",
            kind,
            Category::DependenciesAndBuilds,
            Resource::Path(PathBuf::from(path)),
            kind,
            nature,
            risk,
        )
        .reason(Message::new("reason.test"))
        .recreate(Message::new("recreate.test"))
    }

    #[test]
    fn t01_01_ca1_same_resource_gets_the_same_id() {
        let first = ItemId::new(
            "deps",
            "node_modules",
            &Resource::Path("C:\\Projetos\\api\\node_modules".into()),
        );
        let second = ItemId::new(
            "deps",
            "node_modules",
            &Resource::Path("C:\\Projetos\\api\\node_modules".into()),
        );
        assert_eq!(first, second);
        assert_eq!(first.as_str(), canonical_id_for_api());
    }

    fn canonical_id_for_api() -> &'static str {
        if cfg!(windows) {
            "deps/node_modules/c:/projetos/api/node_modules"
        } else {
            "deps/node_modules/C:/Projetos/api/node_modules"
        }
    }

    #[cfg(windows)]
    #[test]
    fn t01_01_ca1_path_spelling_does_not_change_the_id_on_windows() {
        let a = ItemId::new("d", "k", &Resource::Path("C:\\Projetos\\API\\".into()));
        let b = ItemId::new("d", "k", &Resource::Path("c:/projetos/api".into()));
        assert_eq!(a, b);
    }

    #[test]
    fn different_resources_or_kinds_get_different_ids() {
        let path = Resource::Path("C:\\a".into());
        assert_ne!(ItemId::new("d", "k1", &path), ItemId::new("d", "k2", &path));
        assert_ne!(
            ItemId::new("d", "k", &Resource::Name("node:18".into())),
            ItemId::new("d", "k", &Resource::Name("node:20".into()))
        );
    }

    #[test]
    fn t01_01_ca2_item_without_reason_is_rejected() {
        let result = ItemDraft::new(
            "teste",
            "cache",
            Category::DependenciesAndBuilds,
            Resource::Name("x".into()),
            "x",
            Nature::UserContent,
            Risk::Caution,
        )
        .build();
        assert_eq!(result, Err(ItemError::MissingReason));
    }

    #[test]
    fn t01_01_ca3_recreatable_item_without_how_to_recreate_is_rejected() {
        let result = ItemDraft::new(
            "teste",
            "cache",
            Category::DependenciesAndBuilds,
            Resource::Name("x".into()),
            "x",
            Nature::Recreatable,
            Risk::Safe,
        )
        .reason(Message::new("reason.test"))
        .build();
        assert_eq!(result, Err(ItemError::MissingRecreate));
    }

    #[test]
    fn user_content_does_not_need_how_to_recreate() {
        let result = ItemDraft::new(
            "git",
            "branch",
            Category::Git,
            Resource::Name("feature/x".into()),
            "feature/x",
            Nature::UserContent,
            Risk::Safe,
        )
        .reason(Message::new("reason.merged"))
        .build();
        assert!(result.is_ok());
    }

    #[test]
    fn t01_01_ca4_blocked_item_without_block_reason_is_rejected() {
        let result = draft("worktree", "C:\\w", Nature::UserContent, Risk::Blocked).build();
        assert_eq!(result, Err(ItemError::MissingBlockReason));

        let ok = draft("worktree", "C:\\w", Nature::UserContent, Risk::Blocked)
            .block_reason(Message::new("block.unsaved_work"))
            .build();
        assert!(ok.is_ok());
    }

    #[test]
    fn freeable_size_cannot_exceed_apparent_size() {
        let result = draft("x", "C:\\x", Nature::Recreatable, Risk::Safe)
            .sizes(10, 11)
            .build();
        assert_eq!(result, Err(ItemError::FreeableExceedsApparent));
    }

    #[test]
    fn t01_01_ca5_nested_items_are_not_counted_twice() {
        let modules = draft(
            "node_modules",
            "C:\\w\\node_modules",
            Nature::Recreatable,
            Risk::Moderate,
        )
        .sizes(GB, GB)
        .build()
        .unwrap();
        let worktree = draft("worktree", "C:\\w", Nature::UserContent, Risk::Caution)
            .sizes(3 * GB, 3 * GB)
            .contains(modules.id.clone())
            .build()
            .unwrap();
        let items = vec![modules, worktree];

        let total: u64 = top_level(&items).iter().map(|i| i.freeable_size).sum();
        assert_eq!(total, 3 * GB);
    }

    #[test]
    fn contained_item_alone_still_counts() {
        let modules = draft(
            "node_modules",
            "C:\\w\\nm",
            Nature::Recreatable,
            Risk::Moderate,
        )
        .sizes(GB, GB)
        .build()
        .unwrap();
        let worktree = draft("worktree", "C:\\w", Nature::UserContent, Risk::Caution)
            .sizes(3 * GB, 3 * GB)
            .contains(ItemId::new(
                "teste",
                "outro",
                &Resource::Name("ausente".into()),
            ))
            .build()
            .unwrap();
        let items = vec![modules, worktree];
        assert_eq!(top_level(&items).len(), 2);
    }

    #[test]
    fn t01_01_ca6_zero_size_items_are_organization_items() {
        let branch = ItemDraft::new(
            "git",
            "branch",
            Category::Git,
            Resource::Name("feature/x".into()),
            "feature/x",
            Nature::UserContent,
            Risk::Safe,
        )
        .reason(Message::new("reason.merged"))
        .build()
        .unwrap();
        assert!(branch.is_organization());

        let cache = draft("cache", "C:\\c", Nature::Recreatable, Risk::Safe)
            .sizes(5, 5)
            .build()
            .unwrap();
        assert!(!cache.is_organization());
    }

    #[test]
    fn risks_are_ordered_by_severity() {
        assert!(Risk::Safe < Risk::Moderate);
        assert!(Risk::Moderate < Risk::Caution);
        assert!(Risk::Caution < Risk::Blocked);
    }

    #[test]
    fn draft_keeps_every_field() {
        let at = SystemTime::UNIX_EPOCH;
        let item = draft("x", "C:\\x", Nature::Recreatable, Risk::Safe)
            .sizes(10, 4)
            .size_estimated()
            .project_hint("C:\\p")
            .last_activity(at)
            .action(CleanAction::Compact)
            .side_effect(Message::new("effect.stops_docker"))
            .alert(Message::new("alert.env_file"))
            .signal(Signal::InUse(Message::new("in_use")))
            .build()
            .unwrap();
        assert_eq!((item.apparent_size, item.freeable_size), (10, 4));
        assert!(item.size_estimated);
        assert_eq!(item.project_hint, Some(PathBuf::from("C:\\p")));
        assert_eq!(item.last_activity, Some(at));
        assert_eq!(item.action, CleanAction::Compact);
        assert_eq!(item.side_effects.len(), 1);
        assert_eq!(item.alerts.len(), 1);
        assert_eq!(item.signals.len(), 1);
        assert_eq!(item.recreate, Some(Message::new("recreate.test")));
    }
}
