//! The authored-placement schema vocabulary — architecture.md §4b.
//!
//! These are the closed, serde-able authoring enums that authored maps
//! declare over content: brains (who a spawned actor becomes), the
//! damage relationship/category, and the hazard/prop respawn policy.
//! They are pure data (no `Vec2`, no runtime state, no Bevy) so they live
//! in the Tier-0 catalog — below every crate that interprets them. The
//! sim/content LOWERS these records into behavior at room-load; the arrow
//! is always sim/content → catalog, never the reverse.

use serde::{Deserialize, Serialize};

/// Damage/team relationship used by hitboxes and hurtboxes — the `can_damage`
/// matrix that decides whether one side's hit may affect another.
///
/// Deliberately distinct from `ActorFaction` (`ambition_characters`), which
/// is a `#[derive(Component)]` actor-side tag (`is_player_side`/`is_hostile_side`,
/// with `Npc`/`Boss` variants). This one is the *damage* relationship; that one
/// is the *ECS actor* tag.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DamageTeam {
    Player,
    Enemy,
    Neutral,
    Environment,
}

impl DamageTeam {
    /// True when damage from `self` is allowed to affect `target` by default.
    pub fn can_damage(self, target: Self) -> bool {
        match (self, target) {
            (Self::Player, Self::Enemy) => true,
            (Self::Enemy, Self::Player) => true,
            (Self::Environment, Self::Player | Self::Enemy | Self::Neutral) => true,
            (Self::Neutral, _) => false,
            _ => false,
        }
    }
}

/// The broad gameplay category of damage. This is intentionally separate from
/// presentation so hazards, attacks, and projectiles can share damage handling.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DamageKind {
    Slash,
    Pogo,
    Contact,
    Hazard,
    Projectile,
    Environmental,
    Custom,
}

/// How temporary/destructible hazards/props return after being consumed or
/// killed. (The ADR-0022 *actor* `RespawnPolicy` is a distinct enum — this one
/// covers the authored hazard/prop lifecycle.)
#[derive(Clone, Copy, Debug, PartialEq, Default, Serialize, Deserialize)]
pub enum HazardRespawn {
    /// Never respawn inside the current run/session.
    #[default]
    Never,
    /// Respawn after a timer in simulation seconds.
    AfterSeconds(f32),
    /// Respawn when the room is re-entered.
    OnRoomReload,
    /// The object is persistent and controlled by story/save state.
    Persistent,
}

/// Authored enemy behavior tag. The sandbox maps `Custom(name)` to its
/// own `CharacterArchetype` via `CharacterArchetype::from_brain`; the engine
/// only carries this enum as a typed payload between LDtk authoring
/// and sandbox dispatch.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum CharacterBrain {
    Passive,
    Patrol { path_id: Option<String> },
    Guard { leash_radius: f32 },
    Custom(String),
}

/// Authored boss behavior tag. Same shape and contract as
/// `CharacterBrain`: the engine doesn't simulate against the variants;
/// the sandbox decides per-boss behavior from the payload.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum BossBrain {
    Dormant,
    PhaseScript { script_id: String },
    Custom(String),
}

/// The authored hazard schema — what a `DamageVolume`-style placement SAYS,
/// in plain pairs (the `HitVolume` idiom: `[f32; 2]`, never kernel types).
/// The lowering interpreter (W-queue step 3) converts to `Vec2`/components
/// once at room load; the legacy `DamageVolume` runtime type dissolves when
/// that interpreter lands ([W-a] verdict 3).
///
/// NOTE for step 3: legacy hazards may still carry an INLINE motion path
/// (`DamageVolume.motion`). The schema deliberately has `path_id` only —
/// dissolution lifts inline paths into room-level `KinematicPath` entries.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HazardSpec {
    pub damage: i32,
    pub knockback: [f32; 2],
    pub kind: DamageKind,
    pub team: DamageTeam,
    pub hitstop_seconds: f32,
    pub respawn: HazardRespawn,
    /// Reference to a room-level `KinematicPath` (moving hazards).
    pub path_id: Option<String>,
}

/// The CLOSED authored-placement schema (architecture.md §4b.3): everything an
/// authored map may declare beyond geometry, as editor-visible plain data.
/// Variants grow as W-queue step 3 converts hardcoded spawn branches into
/// registered lowering interpreters; `PlacementKind` (the fieldless mirror
/// keying the lowering registry) lands with the registry in that step.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum PlacementSchema {
    Hazard(HazardSpec),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn environment_damage_affects_player_and_enemy() {
        assert!(DamageTeam::Environment.can_damage(DamageTeam::Player));
        assert!(DamageTeam::Environment.can_damage(DamageTeam::Enemy));
        assert!(!DamageTeam::Player.can_damage(DamageTeam::Player));
    }

    #[test]
    fn can_damage_matrix_encodes_the_friendly_fire_rules() {
        use DamageTeam::{Enemy, Environment, Neutral, Player};
        // The two combat loops cross faction lines.
        assert!(Player.can_damage(Enemy), "player hits enemies");
        assert!(Enemy.can_damage(Player), "enemies hit the player");
        // No same-faction friendly fire.
        assert!(!Player.can_damage(Player));
        assert!(!Enemy.can_damage(Enemy));
        // Environment (hazards) hits everything except itself.
        assert!(Environment.can_damage(Neutral));
        // Neutral never deals damage; nothing targets it offensively except
        // the environment.
        assert!(!Neutral.can_damage(Player));
        assert!(!Neutral.can_damage(Enemy));
        assert!(!Player.can_damage(Neutral));
        assert!(!Enemy.can_damage(Neutral));
        assert!(!Player.can_damage(Environment));
    }
}
