//! The engine's removable capabilities, and the sets their systems run in.
//!
//! A capability is removed from a composition by not installing its plugins
//! (`ambition_platformer2d_runtime::profile`). In a host that installs several
//! games, a capability is installed for all of them, and it is dormant where
//! the active room's game does not use it: its crate puts its per-tick work in
//! [`CapabilitySet`], and the runtime gives each set the condition "the active
//! room's rules use this capability" ([`UsedCapabilities`], declared by each
//! game for its own rule scope). A room whose game declares nothing uses every
//! capability.

use bevy::ecs::schedule::SystemSet;

/// A removable unit of the engine group.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Capability {
    /// Authored cutscenes and their trigger queue.
    Cutscenes,
    /// The dialogue runtime, the conversation authority and its UI bridge.
    Dialogue,
    /// Named boss encounters (generic encounters remain).
    BossEncounters,
    /// Using a held item or a wielded ability.
    HeldUse,
    /// Touching an item to collect it, and the authored conditions that ask about the bag.
    /// ⚠ The bag itself (`OwnedItems`) is core state and is NOT removed (`Q106`).
    Inventory,
}

impl Capability {
    pub const ALL: [Capability; 5] = [
        Capability::Cutscenes,
        Capability::Dialogue,
        Capability::BossEncounters,
        Capability::HeldUse,
        Capability::Inventory,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Capability::Cutscenes => "cutscenes",
            Capability::Dialogue => "dialogue",
            Capability::BossEncounters => "boss-encounters",
            Capability::HeldUse => "held-use",
            Capability::Inventory => "inventory",
        }
    }

    /// The content-pack capability this runtime capability interprets, when
    /// authored content can need it: a pack that requires it is refused by a
    /// composition that omits this capability
    /// (`ambition_engine_schemas::engine_schemas_without`). `None` when no
    /// pack capability belongs to this one alone: the item catalog is read by
    /// collection and by held use, and dialogue is not a pack schema.
    ///
    /// The ids are the owners' constants (`ambition_cutscene::content_schema::
    /// CUTSCENE_CAPABILITY`, `ambition_boss_encounter::pattern::content_schema::
    /// BOSS_PATTERN_CAPABILITY`), spelled here because this crate does not
    /// enable their `content_pack` feature; `supported_profiles.rs` holds the
    /// two equal.
    pub const fn content_capability(self) -> Option<&'static str> {
        match self {
            Capability::Cutscenes => Some("cutscene"),
            Capability::BossEncounters => Some("boss_pattern"),
            Capability::Dialogue | Capability::HeldUse | Capability::Inventory => None,
        }
    }

    const fn bit(self) -> u8 {
        1 << self as u8
    }
}

/// The per-tick systems of one capability. They run only where the active
/// room uses it ([`UsedCapabilities`]); the runtime gives each set that
/// condition in each schedule.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CapabilitySet(pub Capability);

/// The capabilities one game's rooms use: a rule each game declares for its
/// own rule scope (`declare_rules(scope, UsedCapabilities::..)`). A room with
/// no declaration uses every capability.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UsedCapabilities(u8);

impl UsedCapabilities {
    /// Every capability.
    pub const ALL: Self = {
        let mut bits = 0;
        let mut index = 0;
        while index < Capability::ALL.len() {
            bits |= Capability::ALL[index].bit();
            index += 1;
        }
        Self(bits)
    };

    /// These, without `capability`.
    pub const fn without(self, capability: Capability) -> Self {
        Self(self.0 & !capability.bit())
    }

    pub const fn uses(self, capability: Capability) -> bool {
        self.0 & capability.bit() != 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_uses_each_capability_and_without_removes_only_one() {
        let used = UsedCapabilities::ALL.without(Capability::BossEncounters);
        for capability in Capability::ALL {
            assert_eq!(used.uses(capability), capability != Capability::BossEncounters);
            assert!(UsedCapabilities::ALL.uses(capability));
        }
    }
}
