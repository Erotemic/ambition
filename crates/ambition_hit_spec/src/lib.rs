//! Contact policy shared by authored moves, projectiles, and body reactions.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VictimPosture {
    Grounded,
    Airborne,
    Prone,
}

/// Prone bodies are a separate target class, even when they touch the floor.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct HitTargets {
    pub grounded: bool,
    pub airborne: bool,
    pub prone: bool,
}

impl Default for HitTargets {
    fn default() -> Self {
        Self {
            grounded: true,
            airborne: true,
            prone: true,
        }
    }
}

impl HitTargets {
    pub fn permits(self, posture: VictimPosture) -> bool {
        match posture {
            VictimPosture::Grounded => self.grounded,
            VictimPosture::Airborne => self.airborne,
            VictimPosture::Prone => self.prone,
        }
    }
}

/// Transcendent attacks pass through other attacks. They can still hit bodies.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClashPolicy {
    #[default]
    Normal,
    Transcendent,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub enum LaunchWeight {
    #[default]
    Victim,
    Fixed(f32),
}

impl LaunchWeight {
    pub fn resolve(self, victim_weight: f32) -> f32 {
        match self {
            Self::Victim => victim_weight,
            Self::Fixed(weight) => weight,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum TechPolicy {
    #[default]
    Normal,
    Deny,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArmorInteraction {
    #[default]
    Respect,
    Bypass,
}

/// Each volume has its own policy. Missing fields preserve an ordinary hit.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct HitProperties {
    pub targets: HitTargets,
    pub clash: ClashPolicy,
    pub hitlag_scale: f32,
    pub sdi_scale: f32,
    pub tech: TechPolicy,
    pub weight: LaunchWeight,
    pub armor: ArmorInteraction,
}

impl Default for HitProperties {
    fn default() -> Self {
        Self::ORDINARY
    }
}

impl HitProperties {
    pub const ORDINARY: Self = Self {
        targets: HitTargets {
            grounded: true,
            airborne: true,
            prone: true,
        },
        clash: ClashPolicy::Normal,
        hitlag_scale: 1.0,
        sdi_scale: 1.0,
        tech: TechPolicy::Normal,
        weight: LaunchWeight::Victim,
        armor: ArmorInteraction::Respect,
    };
    pub fn is_ordinary(&self) -> bool {
        *self == Self::default()
    }

    pub fn is_valid(self) -> bool {
        self.hitlag_scale.is_finite()
            && self.hitlag_scale >= 0.0
            && self.sdi_scale.is_finite()
            && self.sdi_scale >= 0.0
            && match self.weight {
                LaunchWeight::Victim => true,
                LaunchWeight::Fixed(weight) => weight.is_finite() && weight > 0.0,
            }
    }
}

/// A guard blocks a strike at its damage plus the shield bonus.
/// Unblockable strikes pass through guards and parries.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum HitGuard {
    Blockable { extra_shield_damage: u16 },
    Unblockable,
}
impl Default for HitGuard {
    fn default() -> Self {
        Self::Blockable {
            extra_shield_damage: 0,
        }
    }
}
impl HitGuard {
    pub fn is_ordinary(&self) -> bool {
        *self == Self::default()
    }
}
