//! Safe gain, ranking order and totals (T-01.03, ADR-0007).
//! Critical code: mutation target ≥ 90% (mutation/politica.json).

use crate::activity::ProjectState;
use crate::item::{Category, Item, ItemId, Risk};
use crate::risk::Assessment;
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::time::SystemTime;

/// An item with everything needed to place it in the ranking.
#[derive(Debug, Clone, PartialEq)]
pub struct RankedItem {
    pub item: Item,
    pub assessment: Assessment,
    /// Root of the item's project, if any.
    pub project: Option<PathBuf>,
    pub project_state: Option<ProjectState>,
    /// Bytes × risk weight × activity weight, kept as bytes × 100 to stay in integers.
    pub safe_gain: u128,
}

/// Risk weight in tenths: 🟢 1.0 · 🟡 0.6 · 🔴 0.2 · ⛔ 0.
fn risk_weight(risk: Risk) -> u128 {
    match risk {
        Risk::Safe => 10,
        Risk::Moderate => 6,
        Risk::Caution => 2,
        Risk::Blocked => 0,
    }
}

/// Activity weight in tenths: no project or inactive project 1.0 · active (or pinned) 0.3.
fn activity_weight(state: Option<ProjectState>) -> u128 {
    match state {
        Some(ProjectState::Active | ProjectState::Pinned) => 3,
        Some(ProjectState::Inactive { .. }) | None => 10,
    }
}

/// `freeable × risk weight × activity weight`, in bytes × 100.
pub fn safe_gain(freeable: u64, risk: Risk, state: Option<ProjectState>) -> u128 {
    u128::from(freeable) * risk_weight(risk) * activity_weight(state)
}

impl RankedItem {
    pub fn new(
        item: Item,
        assessment: Assessment,
        project: Option<PathBuf>,
        project_state: Option<ProjectState>,
    ) -> Self {
        let safe_gain = safe_gain(item.freeable_size, assessment.risk, project_state);
        Self {
            item,
            assessment,
            project,
            project_state,
            safe_gain,
        }
    }

    fn blocked(&self) -> bool {
        self.assessment.risk == Risk::Blocked
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Totals {
    /// Everything that may be cleaned: excludes ⛔ and protected items.
    pub recoverable: u64,
    /// Only 🟢.
    pub recoverable_safe: u64,
    pub by_risk: HashMap<Risk, u64>,
    pub by_category: HashMap<Category, u64>,
    /// `None` groups items without a project ("Sem projeto").
    pub by_project: HashMap<Option<PathBuf>, u64>,
    pub organization_items: usize,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Ranking {
    /// Items that free space, best first; ⛔ at the end.
    pub space: Vec<RankedItem>,
    /// Items that free no space (branches, stashes…), 🟢 first, oldest first.
    pub organization: Vec<RankedItem>,
    /// Out of the ranking, shown in "Protegidos" with their rule.
    pub protected: Vec<RankedItem>,
    pub totals: Totals,
}

pub fn rank(items: Vec<RankedItem>) -> Ranking {
    let mut ranking = Ranking::default();
    for entry in items {
        if entry.assessment.protected_by.is_some() {
            ranking.protected.push(entry);
        } else if entry.item.is_organization() {
            ranking.organization.push(entry);
        } else {
            ranking.space.push(entry);
        }
    }
    ranking.space.sort_by(space_order);
    ranking.organization.sort_by(|a, b| {
        a.assessment
            .risk
            .cmp(&b.assessment.risk)
            .then_with(|| oldest_first(a.item.last_activity, b.item.last_activity))
            .then_with(|| a.item.name.cmp(&b.item.name))
    });
    ranking.totals = totals(&ranking.space, ranking.organization.len());
    ranking
}

fn space_order(a: &RankedItem, b: &RankedItem) -> Ordering {
    a.blocked()
        .cmp(&b.blocked())
        .then_with(|| b.safe_gain.cmp(&a.safe_gain))
        .then_with(|| b.item.freeable_size.cmp(&a.item.freeable_size))
        .then_with(|| oldest_first(a.item.last_activity, b.item.last_activity))
        .then_with(|| a.item.name.cmp(&b.item.name))
}

/// Longer without activity first; items without a date go last.
fn oldest_first(a: Option<SystemTime>, b: Option<SystemTime>) -> Ordering {
    match (a, b) {
        (Some(a), Some(b)) => a.cmp(&b),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

/// Sums over cleanable items only, counting nested items once (T-01.01).
fn totals(space: &[RankedItem], organization_items: usize) -> Totals {
    let cleanable: Vec<&RankedItem> = space.iter().filter(|e| !e.blocked()).collect();
    let present: HashSet<&ItemId> = cleanable.iter().map(|e| &e.item.id).collect();
    let nested: HashSet<&ItemId> = cleanable
        .iter()
        .flat_map(|e| e.item.contains.iter())
        .filter(|id| present.contains(id))
        .collect();

    let mut totals = Totals {
        organization_items,
        ..Totals::default()
    };
    for entry in cleanable
        .into_iter()
        .filter(|e| !nested.contains(&e.item.id))
    {
        let size = entry.item.freeable_size;
        totals.recoverable += size;
        if entry.assessment.risk == Risk::Safe {
            totals.recoverable_safe += size;
        }
        *totals.by_risk.entry(entry.assessment.risk).or_default() += size;
        *totals.by_category.entry(entry.item.category).or_default() += size;
        *totals.by_project.entry(entry.project.clone()).or_default() += size;
    }
    totals
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::item::{Category, ItemDraft, Nature, Resource};
    use crate::message::Message;
    use crate::risk::assess;
    use std::time::Duration;

    const GB: u64 = 1_000_000_000;

    fn item(name: &str, gb: u64, risk: Risk, category: Category) -> Item {
        ItemDraft::new(
            "teste",
            "x",
            category,
            Resource::Name(name.into()),
            name,
            Nature::Recreatable,
            risk,
        )
        .sizes(gb * GB, gb * GB)
        .reason(Message::new("reason"))
        .recreate(Message::new("recreate"))
        .block_reason(Message::new("block"))
        .build()
        .unwrap()
    }

    fn entry(name: &str, gb: u64, risk: Risk, state: Option<ProjectState>) -> RankedItem {
        let item = item(name, gb, risk, Category::DependenciesAndBuilds);
        let assessment = assess(&item, None);
        let project = state.map(|_| PathBuf::from(format!("C:/p/{name}")));
        RankedItem::new(item, assessment, project, state)
    }

    fn inactive() -> Option<ProjectState> {
        Some(ProjectState::Inactive {
            idle: Duration::from_secs(40 * 86_400),
        })
    }

    fn names(entries: &[RankedItem]) -> Vec<&str> {
        entries.iter().map(|e| e.item.name.as_str()).collect()
    }

    #[test]
    fn safe_gain_follows_the_formula() {
        // 10 GB 🟡 inactive = 6.0 × GB; 8 GB 🟢 without project = 8.0 × GB; 20 GB 🟡 active = 3.6 × GB.
        assert_eq!(
            safe_gain(10 * GB, Risk::Moderate, inactive()),
            u128::from(600 * GB)
        );
        assert_eq!(safe_gain(8 * GB, Risk::Safe, None), u128::from(800 * GB));
        assert_eq!(
            safe_gain(20 * GB, Risk::Moderate, Some(ProjectState::Active)),
            u128::from(360 * GB)
        );
        assert_eq!(safe_gain(GB, Risk::Caution, None), u128::from(20 * GB));
        assert_eq!(safe_gain(GB, Risk::Blocked, None), 0);
        assert_eq!(
            safe_gain(GB, Risk::Safe, Some(ProjectState::Pinned)),
            u128::from(30 * GB)
        );
    }

    #[test]
    fn t01_03_ca1_safe_cache_beats_bigger_moderate_item() {
        let ranking = rank(vec![
            entry("A", 10, Risk::Moderate, inactive()),
            entry("B", 8, Risk::Safe, None),
        ]);
        assert_eq!(names(&ranking.space), vec!["B", "A"]);
    }

    #[test]
    fn t01_03_ca2_active_project_item_goes_after() {
        let ranking = rank(vec![
            entry("C", 20, Risk::Moderate, Some(ProjectState::Active)),
            entry("A", 10, Risk::Moderate, inactive()),
            entry("B", 8, Risk::Safe, None),
        ]);
        assert_eq!(names(&ranking.space), vec!["B", "A", "C"]);
    }

    #[test]
    fn t01_03_ca3_ties_put_the_bigger_item_first() {
        // 10 GB 🟢 active (3.0) and 5 GB 🟡 inactive (3.0) tie on gain.
        let ranking = rank(vec![
            entry("menor", 5, Risk::Moderate, inactive()),
            entry("maior", 10, Risk::Safe, Some(ProjectState::Active)),
        ]);
        assert_eq!(names(&ranking.space), vec!["maior", "menor"]);
    }

    #[test]
    fn full_ties_go_by_idle_time_then_name() {
        let mut old = entry("b-antigo", 1, Risk::Safe, None);
        old.item.last_activity = Some(SystemTime::UNIX_EPOCH);
        let mut recent = entry("a-recente", 1, Risk::Safe, None);
        recent.item.last_activity = Some(SystemTime::UNIX_EPOCH + Duration::from_secs(100));
        let undated = entry("0-sem-data", 1, Risk::Safe, None);
        let same = entry("c", 1, Risk::Safe, None);
        let ranking = rank(vec![undated, same, recent, old]);
        assert_eq!(
            names(&ranking.space),
            vec!["b-antigo", "a-recente", "0-sem-data", "c"]
        );
    }

    #[test]
    fn t01_03_ca4_blocked_items_go_last_even_when_huge() {
        let ranking = rank(vec![
            entry("enorme", 50, Risk::Blocked, None),
            entry("pequeno", 1, Risk::Caution, inactive()),
            entry("bloqueado-menor", 2, Risk::Blocked, None),
        ]);
        assert_eq!(
            names(&ranking.space),
            vec!["pequeno", "enorme", "bloqueado-menor"]
        );
    }

    #[test]
    fn t01_03_ca5_organization_items_have_their_own_list() {
        let branch = |name: &str, risk: Risk, days: u64| {
            let mut e = entry(name, 0, risk, None);
            e.item.last_activity =
                Some(SystemTime::UNIX_EPOCH + Duration::from_secs(100_000_000 - days * 86_400));
            e
        };
        let ranking = rank(vec![
            branch("recente", Risk::Safe, 1),
            branch("cuidado", Risk::Caution, 90),
            branch("antiga", Risk::Safe, 60),
            entry("cache", 1, Risk::Safe, None),
        ]);
        assert_eq!(names(&ranking.space), vec!["cache"]);
        assert_eq!(
            names(&ranking.organization),
            vec!["antiga", "recente", "cuidado"]
        );
        assert_eq!(ranking.totals.organization_items, 3);
        assert_eq!(ranking.totals.recoverable, GB);
    }

    #[test]
    fn t01_03_ca6_category_totals_add_up_to_the_recoverable_total() {
        let mut items = vec![
            entry("a", 3, Risk::Safe, None),
            entry("b", 5, Risk::Moderate, inactive()),
            entry("c", 7, Risk::Caution, Some(ProjectState::Active)),
            entry("bloq", 11, Risk::Blocked, None),
        ];
        let docker = item("img", 2, Risk::Safe, Category::Docker);
        items.push(RankedItem::new(
            docker.clone(),
            assess(&docker, None),
            None,
            None,
        ));

        let totals = rank(items).totals;
        assert_eq!(totals.recoverable, 17 * GB);
        assert_eq!(totals.by_category.values().sum::<u64>(), totals.recoverable);
        assert_eq!(totals.by_risk.values().sum::<u64>(), totals.recoverable);
        assert_eq!(totals.by_project.values().sum::<u64>(), totals.recoverable);
        assert_eq!(totals.recoverable_safe, 5 * GB);
        assert_eq!(totals.by_category[&Category::Docker], 2 * GB);
        assert_eq!(totals.by_risk[&Risk::Caution], 7 * GB);
        assert_eq!(totals.by_project[&None], 5 * GB);
    }

    #[test]
    fn protected_items_leave_the_ranking_and_the_totals() {
        let cache = item("tcc-cache", 4, Risk::Safe, Category::DependenciesAndBuilds);
        let rule = Message::new("protection.path");
        let protected = RankedItem::new(cache.clone(), assess(&cache, Some(&rule)), None, None);

        let ranking = rank(vec![protected, entry("livre", 1, Risk::Safe, None)]);
        assert_eq!(names(&ranking.space), vec!["livre"]);
        assert_eq!(names(&ranking.protected), vec!["tcc-cache"]);
        assert_eq!(ranking.totals.recoverable, GB);
    }

    #[test]
    fn nested_items_are_counted_once_in_every_total() {
        let modules = entry("node_modules", 1, Risk::Moderate, inactive());
        let mut worktree = entry("worktree", 3, Risk::Caution, inactive());
        worktree.item.contains.push(modules.item.id.clone());

        let totals = rank(vec![modules, worktree]).totals;
        assert_eq!(totals.recoverable, 3 * GB);
        assert_eq!(totals.by_risk.values().sum::<u64>(), 3 * GB);
    }
}
