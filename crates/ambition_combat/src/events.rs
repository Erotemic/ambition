//! Combat message/event vocabulary and small shared value types.

use super::*;


#[derive(Clone, Copy, Debug)]
pub struct FeatureCombatTuning {
    pub enemy_attack_windup: f32,
    pub enemy_attack_active: f32,
    pub boss_attack_windup: f32,
    pub boss_attack_active: f32,
}

/// Default attack-phase timings (seconds). Single source of truth, shared by
/// [`FeatureCombatTuning::default`] and `Platformer2dFeelTuningMonolith::default` (which
/// projects them back out via `Platformer2dFeelTuningMonolith::feature_combat_tuning`).
pub const DEFAULT_ENEMY_ATTACK_WINDUP: f32 = 0.36;
pub const DEFAULT_ENEMY_ATTACK_ACTIVE: f32 = 0.20;
pub const DEFAULT_BOSS_ATTACK_WINDUP: f32 = 0.52;
pub const DEFAULT_BOSS_ATTACK_ACTIVE: f32 = 0.32;

impl Default for FeatureCombatTuning {
    fn default() -> Self {
        Self {
            enemy_attack_windup: DEFAULT_ENEMY_ATTACK_WINDUP,
            enemy_attack_active: DEFAULT_ENEMY_ATTACK_ACTIVE,
            boss_attack_windup: DEFAULT_BOSS_ATTACK_WINDUP,
            boss_attack_active: DEFAULT_BOSS_ATTACK_ACTIVE,
        }
    }
}

/// Victim reaction mode for `HitEvent`s landing on a player. Ignored
/// for non-player targets.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum HitMode {
    /// Normal combat damage: preserve the room and apply knockback
    /// plus hitstun.
    #[default]
    Knockback,
    /// Lava / spike-pit style recovery: put the player back on the
    /// last safe platform.
    SafeRespawn,
}

// Combat remains the event vocabulary's front door; producers keep naming them from here.
pub use ambition_platformer2d_core::hit_response::{HitKnockback, HitKnockbackMagnitude};

/// Relationship/AI stimuli observed by actors.
///
/// Damage systems should emit facts such as "this actor was damaged by that
/// entity". Aggression/relationship systems decide whether that means fight,
/// flee, ignore, call for help, or future faction-specific behavior.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActorStimulus {
    DamagedBy {
        actor: Entity,
        source: Option<Entity>,
        damage: i32,
    },
    /// The actor was explicitly challenged to a fight (e.g. the player picked
    /// the "challenge" dialogue option). Provokes the actor into combat
    /// unconditionally — bypassing the strike-threshold gate that `DamagedBy`
    /// respects — because the challenge IS the deliberate consent to fight.
    /// `challenger` is who threw down the gauntlet (the player), used as the
    /// initial chase target.
    Challenged {
        actor: Entity,
        challenger: Option<Entity>,
    },
}

// Cross-system gameplay effects use typed messages with focused consumers.
// Do not reintroduce a generic effect enum or side-channel vectors.

/// Set a save/quest flag. The consumer mirrors `on == true` into a
/// `QuestAdvanceEvent::FlagSet` so flag-driven quest steps advance in the
/// same frame as the save write.
#[derive(Message, Clone, Debug, PartialEq)]
pub struct SetFlagRequested {
    pub id: String,
    pub on: bool,
}



/// Standalone audio-only gameplay effect. Use typed presentation vectors for
/// sounds that also imply VFX/progression, and this message for bare audio.
#[derive(Message, Clone, Debug, PartialEq)]
pub struct GameplaySfxRequested {
    pub id: ambition_sfx::SfxId,
    pub pos: ae::Vec2,
}

/// Why a room reset fired. Lets a consumer treat a player DEATH differently from
/// a deliberate MANUAL reset — e.g. the portal adapter preserves the player's gun
/// portals across a death but clears them on a manual reset.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RoomResetReason {
    /// The player died / fell out of the world (engine-raised reset). Portals are
    /// PRESERVED so a death doesn't wipe the player's gun-portal setup.
    PlayerDeath,
    /// A deliberate reset: the manual delete-key reset or a scripted room replay.
    /// The gun's portals are cleared (authored level portals are always spared by
    /// `clear_portals_on_reset`). Default so any plain construction clears.
    #[default]
    Manual,
}

/// A SAME-ROOM REPLAY HAS BEEN ADMITTED.
///
/// ⛔⛔ TWO DIFFERENT TRANSACTIONS TRAVEL ON ONE MESSAGE, and this doc claimed
/// only the first until 2026-08-31. Read [`Self::subject`] before believing
/// anything about the world:
///
/// * `Some(subject)` — the lifecycle operation OWNS the one pending-commit slot
///   and the room WILL be rebuilt. This is the contract every consequence below
///   was written against.
/// * `None` — **nothing was recorded and nothing will be rebuilt.**
///   `admit_room_replay` constructs its own `Admission::Admitted` on that arm,
///   logs *"clearing the attempt, not rebuilding"*, and writes this message
///   anyway. The consequences still run, so a bodyless replay retires attempt
///   residue, resets gravity and clears portals for an operation that never
///   acquired lifecycle ownership.
///
/// ⚠ THAT IS A PARTIAL TRANSACTION AND IT IS RECORDED AS A DEFECT, not a design:
/// queue row D-REPLAY-NOSUBJECT. The doc is written this way so a consumer added
/// before it is fixed knows which of the two it is handling. ⛔ do not "fix" it
/// by making the `None` arm silent — the reachable case is a headless or tooling
/// composition in the frame before `ControlledSubject` resolves, and a replay
/// that does nothing at all there is what made D-SFX-RESET-RED take five wrong
/// hypotheses to find.
///
/// ⭐ THIS IS THE SIGNAL EVERY REPLAY CONSEQUENCE HANGS OFF. Exactly one system
/// writes it — `runtime::sandbox_reset::admit_room_replay` — and only after the
/// operation has been accepted. Reset gravity here, clear a content cycle here,
/// retire the previous attempt's residue here; do NOT do any of it on
/// `RoomReplayRequested`, which is the ASK and can be refused.
///
/// ⛔⛔ IT WAS `RoomReplayAdmitted`, AND THE NAME WAS THE BUG. A message
/// called "reset the room's features" invites a listener to reset something the
/// moment it sees one, and the ask was what it saw: the avatar, gravity, hit
/// events, a boss arena's heavy-object cycle and the attempt's dropped loot were
/// all reset before anything had checked whether the replay could be described,
/// let alone admitted. When it could not, the world had been half-reset for an
/// operation that never happened.
#[derive(Message, Clone, Debug, PartialEq, Eq)]
pub struct RoomReplayAdmitted {
    /// Why the room is being replayed. Policy differs by it: a death preserves
    /// the player's placed gun portals, a deliberate retry clears them.
    pub reason: RoomResetReason,
    /// The body the replay is FOR, by stable identity, resolved once at
    /// admission.
    ///
    /// ⛔ NOT RE-DERIVED LATER, for the reason `RoomTransitionIntent` gives
    /// about its own subject: control can move, end, or the body can die during
    /// the wait. `None` only where a composition genuinely has no controlled
    /// body and the replay is a room rebuild with nobody in it.
    pub subject: Option<ambition_platformer2d_shared_tangle::sim_id::SimId>,
}

impl RoomReplayAdmitted {
    /// A replay admitted for nobody in particular — the room comes back, and no
    /// body is returned to spawn. The ordinary shape for a fixture, and the real
    /// shape for a composition with no controlled body.
    ///
    /// ⛔ There is deliberately no `Default`. A replay's subject and reason are
    /// both decisions, and a default would let a producer forget to make either.
    pub fn because(reason: RoomResetReason) -> Self {
        Self {
            reason,
            subject: None,
        }
    }

    /// A deliberate retry with no named subject.
    pub fn manual() -> Self {
        Self::because(RoomResetReason::Manual)
    }

    /// Name the body this replay is for.
    #[must_use]
    pub fn for_subject(
        mut self,
        subject: ambition_platformer2d_shared_tangle::sim_id::SimId,
    ) -> Self {
        self.subject = Some(subject);
        self
    }
}

/// Runtime HUD banner state owned directly by Bevy ECS.
///
/// Gameplay systems either mutate this resource directly or emit
/// [`GameplayBannerRequested`] when their parameter list is already large.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct GameplayBanner {
    pub text: String,
    pub timer: f32,
}

impl GameplayBanner {
    pub fn show(&mut self, text: impl Into<String>, duration: f32) {
        self.text = text.into();
        self.timer = duration.max(0.0);
    }

    pub fn clear(&mut self) {
        self.text.clear();
        self.timer = 0.0;
    }

    pub fn visible(&self) -> bool {
        self.timer > 0.0 && !self.text.is_empty()
    }

    pub fn tick(&mut self, dt: f32) {
        self.timer = (self.timer - dt).max(0.0);
        if self.timer <= 0.0 {
            self.text.clear();
        }
    }
}

/// Message form for indirect banner requests.
#[derive(Message, Clone, Debug, PartialEq)]
pub struct GameplayBannerRequested {
    pub text: String,
    pub duration: f32,
}

impl GameplayBannerRequested {
    pub fn new(text: impl Into<String>, duration: f32) -> Self {
        Self {
            text: text.into(),
            duration,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NpcDialogueRequest {
    pub npc_id: String,
    pub npc_name: String,
    pub dialogue_id: String,
}

/// Semantic cause of a [`HitEvent`]. Attacker identity comes from
/// [`HitEvent::attacker`], victim routing from the named target, and launch
/// strength from authored hit data. Add new attack causes here rather than
/// creating parallel hit channels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HitSource {
    /// A body-owned strike — a swing, a slash, a lunge corridor. Whose swing
    /// it is comes from [`HitEvent::attacker`]; how hard it lands comes from the
    /// attacker's own weight and the strike's authored [`HitKnockback`].
    Melee,
    /// A fired shot. Kept distinct from [`Self::Melee`] because a victim
    /// genuinely wants to know whether it took a contact swing or a ranged shot
    /// — that is a real difference in the world, unlike who fired it.
    Projectile,
    /// A body's own footprint harmed what it touched — walking into an
    /// enemy, a star-powered runner flattening what it passes through, a charger
    /// that rams a wall and bursts. Contact harm runs in both directions and
    /// always has; the striker is whoever's footprint it was.
    Contact,
    /// Environmental hazard (spike, lava, falling debris). Victim reaction
    /// depends on [`HitEvent::mode`] — `SafeRespawn` returns the body to the
    /// last safe platform; `Knockback` applies hitstun + knockback.
    Hazard,
    /// The body left the world past the stage's blast margin — the pit, the
    /// void, the blast zone. Distinct from [`Self::Hazard`] because nothing
    /// touched this body: the stage simply ended, and whoever knocked it out
    /// there is credited by [`HitEvent::attacker`], not by geometry. A platform
    /// fighter scores on exactly this source, so collapsing it into `Hazard` (as
    /// the kernel's reset gate once did) makes the genre unbuildable.
    LeftTheWorld,
    /// A pogo rebound, resolved by orb-exact match rather than broadcast
    /// overlap: the carrying [`HitEvent::volume`] is the orb's authoritative
    /// AABB and the consumer matches it with `approximately_same_aabb`. Bodies
    /// are skipped under this source — body pogo consumes a resolved
    /// [`crate::hitbox::LandedBodyHit`] instead.
    Pogo,
}

impl HitSource {
    /// Whether this unresolved strike still needs victim resolution.
    /// Resolved `HitTarget::Body` events already name their victim.
    pub fn seeks_victims(&self) -> bool {
        matches!(
            self,
            HitSource::Melee
                | HitSource::Projectile
                | HitSource::Pogo
                | HitSource::Contact
        )
    }

}

/// How a hit event resolves its victim.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum HitTarget {
    /// Broadcast: any feature / actor / boss whose AABB intersects
    /// `volume` takes the hit. Default for attacker-side broadcast
    /// hits (player slash, player projectile, hazard areas in
    /// authoring zones).
    #[default]
    Volume,
    /// One pre-resolved body victim. Explicit victim identity is the complete routing answer.
    Body(bevy::prelude::Entity),
    /// Orb-AABB match (pogo). Only the breakable whose AABB
    /// approximately equals `volume` is hit; actors / bosses are
    /// skipped.
    OrbMatch,
    /// Remainder of a strike after body victims were resolved by identity.
    /// Consumers must scan only non-body targets here or bodies receive duplicate damage.
    /// TODO(compat-remove): remove once bosses and breakables are directly resolvable victims.
    UnresolvedFeatures,
}

/// One hit waiting in the cross-frame FIFO, with the STABLE IDENTITIES of the
/// two bodies it connects.
///
/// ⛔⛔ THE IDS EXIST BECAUSE THE CHECKSUM CANNOT ASK THE WORLD. A registered
/// checksum is a bare `fn(&T) -> u64` with no `World`, so the only identity it
/// can see is whatever the queue itself carries — and the queue carried
/// `Entity`, which is allocator identity and must never be hashed. So the
/// projection reduced the attacker to `is_some()` and the victim to the tag
/// `Body`, and "A hits X" fingerprinted the same as "B hits Y" with every other
/// field equal. The desync oracle could not see WHO a staged hit connects,
/// which is most of what a staged hit is.
///
/// ⭐ RESOLVED AT STAGING, WHERE THE WORLD IS IN HAND. Both are written once, by
/// the one system that fills this queue, and then rewound with it — so they
/// cannot drift from the `Entity` handles beside them the way a lazily
/// recomputed copy would.
///
/// ⭐ THE ENTITIES ARE STILL THE RUNTIME ANSWER. `MapEntities` fixes them across
/// a rollback and the consumer routes on them exactly as before; these ids are
/// the FINGERPRINT's, not a second routing authority. One thing decides where a
/// hit lands.
///
/// ⚠ `None` MEANS UNIDENTIFIED, AND TWO UNIDENTIFIED BODIES STILL COLLIDE. Every
/// body in a composed match carries a `SimId`; a hand-built fixture can spawn one
/// that does not, and for those the oracle is no blinder than it was. Stated
/// rather than papered over with an allocator-derived fallback, which would trade
/// blindness for false desync reports.
#[derive(Clone, Debug)]
pub struct StagedPlayerHit {
    pub event: HitEvent,
    /// Who dealt it. `None` for a hit with no attacker entity, or one whose
    /// attacker carries no stable identity.
    pub attacker_id: Option<ambition_platformer2d_shared_tangle::sim_id::SimId>,
    /// Who it was routed to, for a `HitTarget::Body` hit. `None` for every
    /// target that names no body.
    pub victim_id: Option<ambition_platformer2d_shared_tangle::sim_id::SimId>,
}

/// Rollback-registered FIFO for victim-side hits that intentionally cross a frame boundary.
/// Bevy message buffers and reader cursors are not rollback state.
#[derive(bevy::prelude::Resource, Clone, Debug, Default)]
pub struct PendingPlayerHitEvents(pub Vec<StagedPlayerHit>);

impl bevy::ecs::entity::MapEntities for PendingPlayerHitEvents {
    fn map_entities<M: bevy::ecs::entity::EntityMapper>(&mut self, mapper: &mut M) {
        for staged in &mut self.0 {
            let event = &mut staged.event;
            if let Some(attacker) = event.attacker.as_mut() {
                *attacker = mapper.get_mapped(*attacker);
            }
            if let HitTarget::Body(entity) = &mut event.target {
                *entity = mapper.get_mapped(*entity);
            }
        }
    }
}

/// Canonical world-space hit message. `target` states the victim-resolution mode.
#[derive(Message, Clone, Debug)]
pub struct HitEvent {
    /// World-space volume the hit covers. For broadcast / orb-match
    /// hits this is the broadcast / orb volume; for resolved single-
    /// victim hits this is the volume at the impact location. A
    /// [`CombatVolume`] so an attack can carry an effect-shaped (rotated
    /// or convex) hitbox; the common case is still an axis-aligned box
    /// (`Aabb` converts via `.into()`).
    pub volume: ae::CombatVolume,
    /// Damage to apply.
    pub damage: i32,
    /// Who or what dealt the hit.
    pub source: HitSource,
    pub attacker: Option<bevy::prelude::Entity>,
    /// Hint for how the consumer resolves the victim. See
    /// [`HitTarget`].
    pub target: HitTarget,
    /// Reaction mode for player victims (`Knockback` / `SafeRespawn`).
    /// Ignored for non-player targets.
    pub mode: HitMode,
    /// Knockback carried by this hit. `None` means the hit does not push its victim.
    pub knockback: Option<HitKnockback>,
    /// Target keys that have already been hit by this one-hit-per-
    /// target source. Empty for ordinary one-frame projectiles /
    /// hazards / pogos.
    pub ignored_targets: Vec<String>,
    /// Authored contact-sound id. `None` uses the victim feedback default.
    pub strike_sfx: Option<ambition_sfx::SfxId>,
}

#[cfg(test)]
mod resolution_direction_tests {
    use crate::events::HitSource;

    /// Only unresolved strike causes seek victims; hazards and world exit arrive at a victim.
    #[test]
    fn a_strike_seeks_victims_and_the_world_does_not() {
        for hunting in [
            HitSource::Melee,
            HitSource::Projectile,
            HitSource::Pogo,
            HitSource::Contact,
        ] {
            assert!(hunting.seeks_victims(), "{hunting:?} is a strike looking for a victim");
        }
        for arriving in [HitSource::Hazard, HitSource::LeftTheWorld] {
            assert!(
                !arriving.seeks_victims(),
                "{arriving:?} happens TO a body; it is not out looking for one"
            );
        }
    }
}
