//! Reusable, content-free actor vocabulary: identity + the control contract.
//!
//! Data-first shared vocabulary for enemies, bosses, NPCs, moving hazards,
//! and other authored entities. Owns [`ActorKind`]/[`DamageTeam`] identity,
//! the [`control`] `ActorControl`/`ActorControlFrame` contract that brains
//! write and simulation consumes, the [`ai`] intent layer
//! (`CharacterAiIntent`), [`pose`] (`ActorPose`/`ActorFaction`), and the
//! [`character_catalog`] cast data.

pub mod pose;
pub use pose::{ActorFaction, ActorPose};
pub mod population_cap;
pub use population_cap::{ActorAdmission, AuthoredPopulationCap};
pub mod ai;
pub mod body;
pub use body::{BodyAnimFacts, BodyCombat, BodyHealth, BodyWallet, BodyWalletShield, DeathPolicy};
pub mod body_step;
pub use body_step::step_body;
pub mod attack_gesture;
pub mod character_catalog;
pub mod control;
pub mod death_traits;
/// The authored character — see the module doc for why it lives here now.
pub mod definition;
pub use death_traits::CharacterDeathTraits;
/// The pool an undescribed body gets — surfaced flat because its two consumers
/// are the character blueprint and the NPC spawn seed, in another crate.
pub use definition::DEFAULT_UNAUTHORED_BODY_HEALTH;
pub mod limb;
pub use limb::{fan_out_limb_intents, Limb, LimbIntents, LimbRig, LimbRouteState, LimbSlot};
pub mod intrinsics;
pub use intrinsics::{CharacterLocomotion, CharacterMount, ContactDamage};
pub mod worn;
pub use worn::{RecharacterizeBody, WornCharacter};

use ambition_entity_catalog::placements::DamageTeam;

/// Coarse category for room entities that have identity or behavior.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ActorKind {
    Player,
    Enemy,
    Boss,
    Npc,
    MovingPlatform,
    Hazard,
    Projectile,
    Pickup,
    Breakable,
    Debug,
}

/// Lightweight identity/name payload for authored actors.
#[derive(Clone, Debug, PartialEq)]
pub struct Actor {
    pub id: String,
    pub name: String,
    pub kind: ActorKind,
    pub faction: DamageTeam,
}

impl Actor {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        kind: ActorKind,
        faction: DamageTeam,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            kind,
            faction,
        }
    }
}

/// Why a body cannot be hurt right now — a SET, not a flag.
///
/// More than one thing can be true at once (a transformation is playing AND a star is burning), and
/// each owner has to be able to stop being true without deciding for the others. A bool cannot
/// express that: the second writer to finish decides, so the loser is either left invincible
/// forever or stripped early.
///
/// Reasons are bits so the whole set is one `Copy` word, which keeps
/// [`Health`] snapshot-encodable as it was.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Invulnerability(u32);

impl Invulnerability {
    /// A transformation beat is holding the body untouchable
    /// (`TransformBeatPolicy::untouchable`).
    pub const TRANSFORMING: u32 = 1 << 0;
    /// The body's own power state makes it untouchable — a timed pickup, a
    /// super form, whatever a game calls it. Named for what is TRUE of the body
    /// rather than for the object that caused it: "star" is one game's word for
    /// one of its pickups, and the engine has no business knowing it.
    pub const EMPOWERED: u32 = 1 << 1;
    pub const SCRIPTED: u32 = 1 << 2;
    /// An authored `WindowTag::Invuln` window on the move this body is playing.
    ///
    /// A reason like every other, which is the whole point: hit eligibility has
    /// ONE authority (`ambition_platformer2d::combat::util::body_vulnerable`), so a move that
    /// grants intangibility is answered by every rule that already asks it —
    /// the damage resolver and the presentation read-model — with nothing new
    /// to learn. Presentation policy then decides whether this reason opts into
    /// a shared cue. Republished every tick from the live move by
    /// `ambition_platformer2d::combat::moveset::project_move_defense_windows`, so it retracts
    /// when the window closes rather than waiting to be cleared.
    pub const MOVE: u32 = 1 << 3;
    /// A ruleset is protecting a body that has just RETURNED — the beat after a
    /// stock is spent.
    ///
    /// ⛔⛔ ITS OWN REASON, not [`Self::EMPOWERED`], and the difference is
    /// ownership. `Empowered` is ONE component: a body cannot hold a power-up's
    /// grant and a respawn's grant as two independent things, so a respawn that
    /// borrowed it OVERWROTE whatever the body was already carrying, and ending
    /// the respawn beat removed the whole component and every semantic in it. A
    /// reason bit is the type that already solves this — "take or release ONE
    /// reason, leaving every other reason alone" is what it says of itself.
    pub const RESPAWN: u32 = 1 << 4;
    /// The body is not in the world at all — see
    /// [`BodyMode::Submerged`](ambition_platformer2d_core::player_state::BodyMode::Submerged).
    ///
    /// ⛔⛔ THE MODE'S OWN REASON, NOT THE MOVE'S. A trapdoor authors an
    /// `Invuln` window over the same beats, and that would have been enough for
    /// the one move that exists today — but then the MODE's contract ("nothing
    /// can hit it") would be true only for callers who remembered to author the
    /// window, and the second user of it would be struck under the stage with
    /// nothing in the code saying why it was allowed. A body that is absent is
    /// untouchable because it is absent.
    pub const SUBMERGED: u32 = 1 << 5;

    /// Nothing is holding it.
    pub const fn none() -> Self {
        Self(0)
    }

    /// Take or release ONE reason, leaving every other reason alone. This is
    /// the whole point of the type.
    pub fn set(&mut self, reason: u32, held: bool) {
        if held {
            self.0 |= reason;
        } else {
            self.0 &= !reason;
        }
    }

    /// Whether this specific reason is held.
    pub fn holds(&self, reason: u32) -> bool {
        self.0 & reason != 0
    }

    /// Whether ANY reason is held — the damage gate's question.
    pub fn any(&self) -> bool {
        self.0 != 0
    }

    /// Drop every reason (a reset / respawn).
    pub fn clear(&mut self) {
        self.0 = 0;
    }

    /// The raw reason bits, for snapshot encoding.
    pub fn bits(&self) -> u32 {
        self.0
    }

    /// Rebuild from snapshot bits.
    pub fn from_bits(bits: u32) -> Self {
        Self(bits)
    }
}

/// Generic hit-point component for enemies, bosses, breakables, and the player.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Health {
    pub current: i32,
    pub max: i32,
    pub invulnerable: Invulnerability,
}

impl Health {
    pub fn new(max: i32) -> Self {
        let max = max.max(1);
        Self {
            current: max,
            max,
            invulnerable: Invulnerability::none(),
        }
    }

    pub fn alive(self) -> bool {
        self.current > 0
    }

    pub fn ratio(self) -> f32 {
        if self.max <= 0 {
            0.0
        } else {
            (self.current.max(0) as f32 / self.max as f32).clamp(0.0, 1.0)
        }
    }

    /// Apply positive damage and return whether this call killed the entity.
    pub fn damage(&mut self, amount: i32) -> bool {
        if self.invulnerable.any() || amount <= 0 || !self.alive() {
            return false;
        }
        self.current = (self.current - amount).max(0);
        self.current == 0
    }

    pub fn heal(&mut self, amount: i32) {
        if amount > 0 {
            self.current = (self.current + amount).min(self.max);
        }
    }

    pub fn reset(&mut self) {
        self.current = self.max;
        self.invulnerable.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn health_reports_kill_once() {
        let mut health = Health::new(3);
        assert!(!health.damage(2));
        assert_eq!(health.current, 1);
        assert!(health.damage(1));
        assert!(!health.damage(1));
    }

    #[test]
    fn health_invulnerable_drops_damage() {
        let mut health = Health::new(5);
        health.invulnerable.set(Invulnerability::SCRIPTED, true);
        assert!(!health.damage(3));
        assert_eq!(health.current, 5);
        // Disabling invuln re-enables damage.
        health.invulnerable.set(Invulnerability::SCRIPTED, false);
        assert!(!health.damage(3));
        assert_eq!(health.current, 2);
    }

    #[test]
    fn health_damage_zero_or_negative_is_no_op() {
        let mut health = Health::new(5);
        assert!(!health.damage(0));
        assert!(!health.damage(-3));
        assert_eq!(health.current, 5);
    }

    /// Percent is not health, and the meter can exceed the pool. (S4)
    ///
    /// Lives here rather than beside the damage systems because `BodyHealth` is
    /// here: this is the authority's own contract, and a consumer crate asserting
    /// it would be asserting something it does not own.
    #[test]
    fn damage_percent_is_unclamped_so_a_hud_can_print_188() {
        let mut body = BodyHealth::new(Health::new(50)).with_policy(DeathPolicy::Unbounded);
        body.damage(94);
        assert!(
            (body.damage_percent() - 1.88).abs() < 1e-6,
            "damage_percent() = {} — 94 damage over a 50 pool is 188%",
            body.damage_percent()
        );
        assert_eq!(
            body.health.ratio(),
            1.0,
            "the POOL is untouched under Unbounded: its death is the world's, and \
             a drained pool is what used to make it stop taking hits"
        );
    }

    /// `Health::damage` returns early on `!alive()`, so a hit landing on an empty pool was DROPPED
    /// rather than clamped, and knockback growth (which scales off this meter) flatlined at 100%.
    #[test]
    fn damage_percent_keeps_climbing_past_a_full_pool() {
        let mut body = BodyHealth::new(Health::new(20));
        body.damage(7);
        assert_eq!(body.damage_taken(), 7);
        body.damage(100);
        assert_eq!(
            body.damage_taken(),
            107,
            "the meter saturated at the pool max, so a body cannot be MORE hurt \
             than its pool is deep"
        );
        assert!(body.damage_percent() > 5.0);
    }

    /// An `Unbounded` body keeps taking damage forever, which is the whole reason
    /// the variant exists and what it could not do before S4.
    #[test]
    fn damage_percent_grows_on_a_body_the_meter_may_not_kill() {
        let mut body = BodyHealth::new(Health::new(10)).with_policy(DeathPolicy::Unbounded);
        for _ in 0..20 {
            assert!(
                !body.damage(10),
                "the meter killed a body whose death is the world's"
            );
            assert!(body.alive());
        }
        assert_eq!(body.damage_taken(), 200);
        assert_eq!(
            body.current(),
            body.max(),
            "the pool drained under Unbounded"
        );
    }

    #[test]
    fn health_heal_clamps_to_max() {
        let mut health = Health::new(10);
        health.damage(7);
        assert_eq!(health.current, 3);
        health.heal(50); // tries to over-heal
        assert_eq!(health.current, 10);
    }

    #[test]
    fn health_heal_zero_or_negative_is_no_op() {
        let mut health = Health::new(10);
        health.damage(4);
        let before = health.current;
        health.heal(0);
        assert_eq!(health.current, before);
        health.heal(-5);
        assert_eq!(health.current, before);
    }

    #[test]
    fn health_ratio_within_envelope() {
        let mut health = Health::new(10);
        assert_eq!(health.ratio(), 1.0);
        health.damage(5);
        assert!((health.ratio() - 0.5).abs() < 1e-6);
        health.damage(50); // overkill
        assert_eq!(health.ratio(), 0.0);
    }

    #[test]
    fn health_reset_restores_max_and_clears_invuln() {
        let mut health = Health::new(8);
        health.damage(5);
        health.invulnerable.set(Invulnerability::SCRIPTED, true);
        health.reset();
        assert_eq!(health.current, 8);
        assert!(!health.invulnerable.any());
    }

    #[test]
    fn health_alive_tracks_current() {
        let mut health = Health::new(2);
        assert!(health.alive());
        health.damage(1);
        assert!(health.alive());
        health.damage(1);
        assert!(!health.alive());
    }

    #[test]
    fn health_new_clamps_max_to_minimum_of_one() {
        // Negative or zero max becomes 1 so health.alive() is always
        // meaningful (a 0-max entity is degenerate).
        let h = Health::new(0);
        assert_eq!(h.max, 1);
        assert!(h.alive());

        let h = Health::new(-5);
        assert_eq!(h.max, 1);
    }
}
