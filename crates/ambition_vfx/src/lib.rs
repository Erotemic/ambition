//! Shared visual-effect vocabulary.
//! Combat owns damage-box requests and their materialization.

// The kernel, not the platformer. Gameplay geometry stays on `ambition_geometry`; the only direct
// core edge now is the backend-neutral rollback declaration vocabulary in `rollback_registration`.

pub mod fx;
pub mod lightning;
pub mod vfx;
pub use fx::FxId;
pub use lightning::{LightningEnd, LightningKey, LightningLook};
pub use vfx::{
    FireworksRequest, FxPose, FxRequest, HitBurst, HurtFeedback, ImpactBlow, ImpactMaterial, ParticleKind, VfxForRoom, VfxInRoom, VfxMessage,
    VfxWriter,
};

// ===================================================================
// The side an effect is emitted BY.
// ===================================================================
//
/// Presentation-neutral side tag carried by effect messages and hitboxes.
///
/// This intentionally mirrors the combat-facing actor faction vocabulary
/// without depending on the character crate: effect producers map their richer
/// game-side faction into this small fact at the emit site, and combat
/// resolvers map it back when they need faction relations.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum HitSide {
    /// Local or remote player-controlled side.
    #[default]
    Player,
    /// Encounter-spawned hostile side.
    Enemy,
    /// Authored NPC side. Peaceful NPCs do not emit combat effects, but
    /// provoked NPCs keep this side so personal-grudge combat stays expressible.
    Npc,
    /// Boss-tier hostile side.
    Boss,
    /// Inert/non-combatant side.
    Neutral,
    /// A hazard: it belongs to no side and hurts every fighter, including the
    /// one who placed it.
    ///
    /// This is the presentation-side name for `DamageTeam::Environment`, not a
    /// new idea. `placements.rs` asserts `Environment.can_damage(Player)` and
    /// `Environment.can_damage(Enemy)`. It is a separate variant because the
    /// `Hitbox` road carries `source: HitSide` and no `DamageTeam`.
    ///
    /// It is not `Neutral` with damage: `Neutral` takes no part, and the resolver
    /// never spawns a damaging hitbox for it.
    Environment,
}

impl HitSide {
    pub fn is_player_side(self) -> bool {
        matches!(self, Self::Player)
    }

    pub fn is_hostile_side(self) -> bool {
        matches!(self, Self::Enemy | Self::Boss)
    }
}

// ===================================================================
// Effect vocabulary + the message seam + the executor.
// ===================================================================

// Domain-owned rollback declaration; the host supplies the backend registrar.
mod rollback_registration;
pub use rollback_registration::register_rollback_state;
