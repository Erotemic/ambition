//! Combat-kit message/event vocabulary + small shared value types.
//!
//! Holds `FeatureCombatTuning`, the hit model
//! (`HitMode`, `HitKnockback`, `ActorStimulus`), the typed gameplay-effect
//! messages consumed in [`bus`](super::bus) (`SetFlagRequested`,
//! `QuestAdvanceRequested`, `SwitchActivated`, `GameplaySfxRequested`), the
//! room-reset signals (`RoomResetReason`, `ResetRoomFeaturesEvent`), and the
//! `GameplayBanner` HUD resource. Pure data/messages — no systems.

use super::*;

// The feature-visual taxonomy (`FeatureVisualKind`, `BoundFeatureKind`) moved
// to `ambition_platformer2d_shared_tangle::feature_kind` and the `FeatureView`
// read-model row to `ambition_sim_view` (recon C2): the taxonomy is foundation
// vocabulary and the row is the read-model's own — neither is combat model.

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

// Typed cross-system gameplay effects emitted by feature code.
//
// Each is its own Bevy [`Message`] with a focused consumer in `bus.rs`.
// This deliberately replaces the former single `GameplayEffect` enum bus:
// unrelated domain events (save flags, quest advances, switch activations,
// audio) no longer share one channel, so a consumer only declares a reader
// for the message it actually handles.
//
// Do not reintroduce a generic effect enum or side-channel `Vec`s for
// progression/save/audio routing. Add a typed message + a focused consumer
// system instead.

/// Set a save/quest flag. The consumer mirrors `on == true` into a
/// `QuestAdvanceEvent::FlagSet` so flag-driven quest steps advance in the
/// same frame as the save write.
#[derive(Message, Clone, Debug, PartialEq)]
pub struct SetFlagRequested {
    pub id: String,
    pub on: bool,
}

// `QuestAdvanceRequested` moved to `crate::quest` (E2): quest owns its
// advance vocabulary; combat must not name it.

// `SwitchActivated` moved to `crate::encounter::switches` (E2): it names
// encounter vocabulary; combat must not.

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

/// Reset request for ECS-owned room features.
///
/// Same-room resets and full sandbox resets emit this once, and
/// `reset_ecs_room_features` consumes it through Bevy's message stream.
#[derive(Message, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ResetRoomFeaturesEvent {
    pub reason: RoomResetReason,
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

/// Message form for systems that cannot cheaply acquire `ResMut<GameplayBanner>`
/// without bloating an already-large system signature.
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

/// **What KIND of thing hit this body.** A cause, and nothing else.
///
/// * a "player's" slash meant *a slash filed under the player-side spelling*,
///   so a possessed enemy's swing was the player's and an empowered ally's was
///   not, and the outgoing damage slider reached the wrong strikes both ways;
/// * `BossAttack` selected a heavier launch, so a heavy anything that was not a
///   boss was unrepresentable;
/// * which consumer owned an event was decided by the source's direction rather
///   than by which population the named victim was in.
///
/// ⇒ **identity questions are asked of the ATTACKER**, whose entity
/// [`HitEvent::attacker`] carries, and the vocabulary is left with the job it is
/// actually good at: telling the HUD, the trace and the victim's reaction what
/// kind of event this was. A hit that names its victim reaches its consumer on
/// that name; only an unresolved broadcast still asks a question of the cause,
/// and [`Self::seeks_victims`] is the whole of it.
///
/// New attack sources should add a variant here rather than building a parallel
/// `apply_*_attack` path. The canonical channel is [`HitEvent`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HitSource {
    /// **A body-owned strike** — a swing, a slash, a lunge corridor. Whose swing
    /// it is comes from [`HitEvent::attacker`]; how hard it lands comes from the
    /// attacker's own weight and the strike's authored [`HitKnockback`].
    Melee,
    /// **A fired shot.** Kept distinct from [`Self::Melee`] because a victim
    /// genuinely wants to know whether it took a contact swing or a ranged shot
    /// — that is a real difference in the world, unlike who fired it.
    Projectile,
    /// **A body's own footprint harmed what it touched** — walking into an
    /// enemy, a star-powered runner flattening what it passes through, a charger
    /// that rams a wall and bursts. Contact harm runs in both directions and
    /// always has; the striker is whoever's footprint it was.
    Contact,
    /// **Environmental hazard** (spike, lava, falling debris). Victim reaction
    /// depends on [`HitEvent::mode`] — `SafeRespawn` returns the body to the
    /// last safe platform; `Knockback` applies hitstun + knockback.
    Hazard,
    /// **The body left the world** past the stage's blast margin — the pit, the
    /// void, the blast zone. Distinct from [`Self::Hazard`] because nothing
    /// touched this body: the stage simply ended, and whoever knocked it out
    /// there is credited by [`HitEvent::attacker`], not by geometry. A platform
    /// fighter scores on exactly this source, so collapsing it into `Hazard` (as
    /// the kernel's reset gate once did) makes the genre unbuildable.
    LeftTheWorld,
    /// **A pogo rebound**, resolved by orb-exact match rather than broadcast
    /// overlap: the carrying [`HitEvent::volume`] is the orb's authoritative
    /// AABB and the consumer matches it with `approximately_same_aabb`. Bodies
    /// are skipped under this source — body pogo consumes a resolved
    /// [`crate::hitbox::LandedBodyHit`] instead.
    Pogo,
}

impl HitSource {
    /// **Is this an unresolved strike still hunting for whom it hit?**
    ///
    /// **only ask this of an event whose target is NOT resolved.** Every
    /// victim-side producer in the tree stamps [`HitTarget::Body`] now, and a
    /// named victim is the whole answer — the three sites that consult this all
    /// reach it only on a `Volume` / `UnresolvedFeatures` / `OrbMatch` event,
    /// where "who is this broadcast FOR" is a real question.
    ///
    /// so it is NOT "attacker-side", which is what it was called, and that
    /// name is why it read as a fact about the player-versus-world direction of
    /// combat. It is a fact about the event's RESOLUTION state.
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
    /// **One pre-resolved body victim, named by entity.** A producer that
    /// already did the work — overlap, relationship, self-exclusion, dedup —
    /// stamps who it picked, and every consumer applies the hit to exactly that
    /// body. Explicit victim identity outranks
    /// [`HitSource::seeks_victims`]'s legacy broadcast direction.
    ///
    /// Its real job was telling two consumers which one owned the event, which is a question
    /// each consumer can answer by asking whether the victim is in ITS population. A controller
    /// kind is not a damage route.
    Body(bevy::prelude::Entity),
    /// Orb-AABB match (pogo). Only the breakable whose AABB
    /// approximately equals `volume` is hit; actors / bosses are
    /// skipped.
    OrbMatch,
    /// **The part of a strike the body resolver could not resolve.**
    ///
    /// A body-owned melee strike resolves every real combat body itself, by
    /// identity, in [`crate::hitbox::apply_hitbox_damage`] — and publishes one
    /// [`crate::hitbox::LandedBodyHit`] per contact. But a strike also reaches
    /// things that are not bodies: a breakable crate, and a boss whose HP and
    /// phase live on an encounter rather than on a combat body. Those have no
    /// entity the resolver can name, so they stay UNRESOLVED and the geometry
    /// has to be broadcast for them.
    ///
    /// **This is not [`Self::Volume`], and the difference is load-bearing.**
    /// `Volume` means "nothing here is resolved — scan everything", and the
    /// wielded world-AOE primitive still means exactly that. This variant means
    /// "the bodies are ALREADY resolved; scan only what a body resolver cannot
    /// see". A consumer that treats the two alike damages every combat body a
    /// second time, on top of the identified hit it already took.
    ///
    /// so it exists to keep an unresolved broadcast from masquerading as a
    /// body hit, which is the shape the combat campaign is removing. When
    /// bosses and breakables become resolvable victims in their own right, this
    /// variant goes away with them; it does not become the general answer.
    UnresolvedFeatures,
}

/// One hit event in world space — the single canonical channel for
/// damage in either direction (attacker → feature, or anything →
/// player). Producers emit these as Bevy messages; the feature- and
/// player-damage systems filter by source-direction and apply.
///
/// Source-specific resolution:
/// - `PlayerSlash` / `PlayerProjectile`: broadcast match — every
///   feature whose AABB strict-intersects `volume` takes a hit.
/// - `PogoBounce`: orb-exact match — only the breakable whose AABB
///   approximately equals `volume` is hit; actors / bosses are skipped.
/// - `Hazard` / `Enemy*` / `Boss*` with `target = Player(e)`: the
///   pre-resolved player victim takes the hit (mode + knockback
///   applied). `target = Volume` falls back to the primary player.
/// The in-flight victim-side hits, staged at the end of the Combat phase for
/// the player-victim resolver that runs in the NEXT frame's PlayerSimulation
/// phase.
///
/// A message buffer cannot carry sim state across a frame boundary under GGRS:
/// the buffer is cleared on `LoadWorld` (so a rewind between the strike's
/// write and the victim's read silently un-hits the player), and reader
/// cursors are `Local`s no snapshot can see. Cross-frame combat truth
/// therefore lives in this rollback-registered FIFO — the
/// `SwitchActivationQueue` pattern, found by the
/// Phase-5 exit oracle when an enemy hit on the player failed to survive
/// resimulation.
#[derive(bevy::prelude::Resource, Clone, Debug, Default)]
pub struct PendingPlayerHitEvents(pub Vec<HitEvent>);

impl bevy::ecs::entity::MapEntities for PendingPlayerHitEvents {
    fn map_entities<M: bevy::ecs::entity::EntityMapper>(&mut self, mapper: &mut M) {
        for event in &mut self.0 {
            if let Some(attacker) = event.attacker.as_mut() {
                *attacker = mapper.get_mapped(*attacker);
            }
            if let HitTarget::Body(entity) = &mut event.target {
                *entity = mapper.get_mapped(*entity);
            }
        }
    }
}

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
    /// Knockback impulse to apply to the victim, and **the only channel that
    /// carries one**. `None` means this hit genuinely does not push its target
    /// (pogo, player projectile). a source-specific impulse field is not an
    /// alternative spelling — see [`HitSource::Melee`] for the one that
    /// existed and what it cost.
    pub knockback: Option<HitKnockback>,
    /// Target keys that have already been hit by this one-hit-per-
    /// target source. Empty for ordinary one-frame projectiles /
    /// hazards / pogos.
    pub ignored_targets: Vec<String>,
    /// Authored STRIKE SOUND identity (CM8): the sound the attack behind this
    /// hit makes on contact, carried to the ONE victim-side reaction so the
    /// payload is never chosen by an `is_player` branch. `None` = the victim's
    /// own [`ambition_vfx::HurtFeedback`] default sound. A `Copy` `u64` id —
    /// cheap weight on the snapshotted `PendingPlayerHitEvents` FIFO, unlike a
    /// `String` — and excluded from the checksum like the other id fields.
    pub strike_sfx: Option<ambition_sfx::SfxId>,
}

#[cfg(test)]
mod resolution_direction_tests {
    use super::HitSource;

    /// The predicate answers one question — is this unresolved strike still
    /// hunting for whom it hit — and the split is between a cause that goes
    /// LOOKING for a body and one that happens TO whoever is standing there.
    ///
    /// That mattered only because the player FIFO staged on the COMPLEMENT of this predicate; it
    /// stages on whether the named victim is in its own population now, so the answer gates nothing
    /// for a resolved hit — and every contact producer in the tree resolves its victim. A body
    /// whose footprint harms what it touches genuinely is out looking.
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
