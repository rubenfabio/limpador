//! Final risk, destination and automation eligibility of an item (T-01.02, ADR-0006).
//! Critical code: mutation target ≥ 90% (mutation/politica.json).

use crate::item::{Item, Nature, Risk};
use crate::message::Message;

/// Extra evidence about an item, from the collector or the engine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Signal {
    /// Open file, running container, editor holding the cache… → ⛔.
    InUse(Message),
    /// Uncommitted changes, unpushed commits… → ⛔.
    UnsavedWork(Message),
    /// Any other finding that raises the risk to at least `risk`.
    Raise { risk: Risk, reason: Message },
}

impl Signal {
    fn risk(&self) -> Risk {
        match self {
            Self::InUse(_) | Self::UnsavedWork(_) => Risk::Blocked,
            Self::Raise { risk, .. } => *risk,
        }
    }

    fn reason(&self) -> &Message {
        match self {
            Self::InUse(reason) | Self::UnsavedWork(reason) | Self::Raise { reason, .. } => reason,
        }
    }
}

/// Where a cleaned item goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Destination {
    /// Deleted right away (recreatable content).
    Direct,
    /// Kept restorable for the quarantine period (user content or 🔴).
    Quarantine,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Assessment {
    /// The most severe of the base risk and every signal.
    pub risk: Risk,
    /// Rule that protects the item; protected items leave the ranking.
    pub protected_by: Option<Message>,
    /// Every reason that contributed, base reason first.
    pub reasons: Vec<Message>,
    /// `None` when nothing may be done (⛔ or protected).
    pub destination: Option<Destination>,
    pub automation_eligible: bool,
}

/// Combines the collector's proposal, the signals and an optional protection rule.
pub fn assess(item: &Item, protection: Option<&Message>) -> Assessment {
    let risk = item
        .signals
        .iter()
        .map(Signal::risk)
        .fold(item.risk, Risk::max);

    let mut reasons = vec![item.reason.clone()];
    reasons.extend(item.block_reason.iter().cloned());
    reasons.extend(item.signals.iter().map(|s| s.reason().clone()));

    let protected = protection.is_some();
    let destination = if protected || risk == Risk::Blocked {
        None
    } else if item.nature == Nature::UserContent || risk == Risk::Caution {
        Some(Destination::Quarantine)
    } else {
        Some(Destination::Direct)
    };

    let automation_eligible = !protected
        && risk == Risk::Safe
        && item.nature == Nature::Recreatable
        && item.side_effects.is_empty();

    Assessment {
        risk,
        protected_by: protection.cloned(),
        reasons,
        destination,
        automation_eligible,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::item::{Category, ItemDraft, Resource};

    fn item(nature: Nature, risk: Risk) -> ItemDraft {
        ItemDraft::new(
            "teste",
            "x",
            Category::DependenciesAndBuilds,
            Resource::Name("x".into()),
            "x",
            nature,
            risk,
        )
        .reason(Message::new("reason.base"))
        .recreate(Message::new("recreate.x"))
    }

    #[test]
    fn t01_02_ca1_safe_item_in_use_becomes_blocked_citing_the_use() {
        let in_use = Message::new("signal.in_use").with("process", "code.exe");
        let item = item(Nature::Recreatable, Risk::Safe)
            .signal(Signal::InUse(in_use.clone()))
            .build()
            .unwrap();

        let a = assess(&item, None);
        assert_eq!(a.risk, Risk::Blocked);
        assert!(a.reasons.contains(&in_use));
        assert_eq!(a.destination, None);
        assert!(!a.automation_eligible);
    }

    #[test]
    fn unsaved_work_blocks_too() {
        let item = item(Nature::UserContent, Risk::Caution)
            .signal(Signal::UnsavedWork(Message::new("signal.uncommitted")))
            .build()
            .unwrap();
        assert_eq!(assess(&item, None).risk, Risk::Blocked);
    }

    #[test]
    fn t01_02_ca2_moderate_item_with_caution_signal_becomes_caution() {
        let raise = Message::new("signal.no_lockfile");
        let item = item(Nature::Recreatable, Risk::Moderate)
            .signal(Signal::Raise {
                risk: Risk::Caution,
                reason: raise.clone(),
            })
            .build()
            .unwrap();

        let a = assess(&item, None);
        assert_eq!(a.risk, Risk::Caution);
        assert_eq!(a.reasons, vec![Message::new("reason.base"), raise]);
    }

    #[test]
    fn signals_never_lower_the_risk() {
        let item = item(Nature::Recreatable, Risk::Caution)
            .signal(Signal::Raise {
                risk: Risk::Safe,
                reason: Message::new("signal.minor"),
            })
            .build()
            .unwrap();
        assert_eq!(assess(&item, None).risk, Risk::Caution);
    }

    #[test]
    fn t01_02_ca3_protected_item_has_no_destination_and_keeps_the_rule() {
        let rule = Message::new("protection.path").with("path", "C:\\Projetos\\tcc");
        let item = item(Nature::Recreatable, Risk::Safe).build().unwrap();

        let a = assess(&item, Some(&rule));
        assert_eq!(a.protected_by, Some(rule));
        assert_eq!(a.destination, None);
        assert!(!a.automation_eligible);
    }

    #[test]
    fn t01_02_ca4_user_content_goes_to_quarantine_even_when_safe() {
        let branch = item(Nature::UserContent, Risk::Safe).build().unwrap();
        assert_eq!(
            assess(&branch, None).destination,
            Some(Destination::Quarantine)
        );
    }

    #[test]
    fn t01_02_ca5_recreatable_moderate_item_is_deleted_directly() {
        let node_modules = item(Nature::Recreatable, Risk::Moderate).build().unwrap();
        assert_eq!(
            assess(&node_modules, None).destination,
            Some(Destination::Direct)
        );
    }

    #[test]
    fn caution_items_go_to_quarantine_even_when_recreatable() {
        let item = item(Nature::Recreatable, Risk::Caution).build().unwrap();
        assert_eq!(
            assess(&item, None).destination,
            Some(Destination::Quarantine)
        );
    }

    #[test]
    fn blocked_base_risk_keeps_its_block_reason() {
        let block = Message::new("block.locked_worktree");
        let item = item(Nature::UserContent, Risk::Blocked)
            .block_reason(block.clone())
            .build()
            .unwrap();
        let a = assess(&item, None);
        assert_eq!(a.destination, None);
        assert!(a.reasons.contains(&block));
    }

    #[test]
    fn t01_02_ca6_compaction_with_side_effects_is_not_eligible_for_automation() {
        let compaction = item(Nature::Recreatable, Risk::Safe)
            .side_effect(Message::new("effect.stops_docker"))
            .build()
            .unwrap();
        assert!(!assess(&compaction, None).automation_eligible);
    }

    #[test]
    fn t01_02_ca7_safe_recreatable_cache_without_side_effects_is_eligible() {
        let cache = item(Nature::Recreatable, Risk::Safe).build().unwrap();
        assert!(assess(&cache, None).automation_eligible);
    }

    #[test]
    fn only_safe_items_are_eligible_for_automation() {
        let moderate = item(Nature::Recreatable, Risk::Moderate).build().unwrap();
        assert!(!assess(&moderate, None).automation_eligible);
        let safe_user_content = item(Nature::UserContent, Risk::Safe).build().unwrap();
        assert!(!assess(&safe_user_content, None).automation_eligible);
    }
}
