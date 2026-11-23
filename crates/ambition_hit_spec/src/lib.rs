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

/// A body status an accepted hit installs as part of its reaction, with the
/// time it lasts. The receiver resolves it inside its own reception: an
/// ignored or blocked hit installs nothing, and armor that absorbs the hit's
/// launch absorbs its status too.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HitStatus {
    pub kind: HitStatusKind,
    /// Seconds, above 0 and at most [`HitStatusKind::max_s`].
    pub duration_s: f32,
}

/// The four on-hit statuses.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum HitStatusKind {
    /// Holds the body's motion and delays its stun and launch; a mash buys
    /// time back.
    Paralyze,
    /// Drives a standing body into the floor in place of its launch. A body in
    /// the air keeps its ordinary reaction.
    Bury,
    /// Encases the body: no control, while its motion (the launch) continues.
    Freeze,
    /// A standing body sags and then collapses, in place of its launch. A body
    /// in the air keeps its ordinary reaction.
    Crumple,
}

impl HitStatusKind {
    /// The longest status a hit may author. A held body cannot act, so a
    /// longer one is a stock lost with no counterplay but the mash.
    pub const fn max_s(self) -> f32 {
        match self {
            Self::Paralyze => 2.0,
            Self::Bury | Self::Freeze => 2.5,
            Self::Crumple => 1.5,
        }
    }

    /// Whether this status replaces the hit's launch, and so needs a standing
    /// body to happen at all.
    pub const fn replaces_launch(self) -> bool {
        matches!(self, Self::Bury | Self::Crumple)
    }
}

impl HitStatus {
    pub fn is_valid(self) -> bool {
        self.duration_s.is_finite() && self.duration_s > 0.0 && self.duration_s <= self.kind.max_s()
    }
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
