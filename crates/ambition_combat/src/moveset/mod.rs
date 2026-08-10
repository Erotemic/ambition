//! Data-driven move playback — the runtime half of the Smash model.
//!
//! An actor plays a [`MoveSpec`](ambition_entity_catalog::MoveSpec) by
//! carrying a [`MovePlayback`] component; [`advance_move_playback`] is the
//! ONE system that turns the authored timeline into simulation:
//!
//! - **Proper time.** The playback clock advances by
//!   `WorldTime::entity_dt(ProperTimeScale)` (ADR 0011) — the owning actor's
//!   own clock. A dilated actor's windows, volumes, events, and picture all
//!   slow together because they are one timeline (`MovePlayback::phase` is
//!   what presentation samples the bound clip by).
//! - **Windows → hitbox entities.** Each `Active` window's volumes become
//!   `(Hitbox, HitboxHits)` entities (`FollowOwner`, facing-mirrored,
//!   entity-local offsets) on window entry and despawn on window exit —
//!   window-scoped by the move's own clock, so no wall-time lifetime can
//!   drift from a dilated owner. Damage resolution is the existing
//!   [`apply_hitbox_damage`](super::hitbox::apply_hitbox_damage) path:
//!   moves need NO parallel hit plumbing.
//! - **Events → messages.** Timed events emit [`MoveEventMessage`]s;
//!   consumers (audio bridge, techniques/effects) subscribe downstream.
//!
//! Re-binding a move onto a different actor is inserting the same
//! `MovePlayback` on a different entity — zero per-actor Rust. That is the
//! decomposability contract, pinned by the tests below.

use bevy::prelude::{
    Commands, Component, Entity, Message, MessageReader, MessageWriter, Query, Res, With,
};

use ambition_entity_catalog::{
    AttackDir, ClipBinding, EffectRef, HitVolume, MoveEvent, MoveEventKind, MoveSpec, MoveWindow,
    MovesetContract, VolumeShape, WindowTag,
};
use ambition_platformer2d_core as ae;
use ambition_platformer2d_core::AabbExt;
use ambition_time::ProperTimeScale;

use super::components::{ActorFaction, BodyMelee, MeleeSwing};
use super::hitbox::{Hitbox, HitboxAnchor, HitboxHits};
use crate::{hit_side_from_actor_faction, AttackIntent, AttackSpec};
use ambition_characters::actor::attack_gesture::{
    resolve_attack_gesture, AttackGestureState, AttackGestureTuning, AttackPosture, AttackStrength,
    ResolvedAttackGesture,
};
use ambition_characters::brain::action_set::{
    ActionRequest, MeleeActionSpec, RangedActionSpec, SpecialActionSpec,
};
use ambition_characters::brain::{ActorActionMessage, ActorControl};
use ambition_entity_catalog::placements::DamageKind;
use ambition_sfx::{PresentationSourceId, SfxId, SfxMessage, SfxWriter};
use ambition_time::WorldTime;

// **The four moveset verb ids now live beside the contract they key into**
// (`ambition_entity_catalog`), because a verb name is authoring vocabulary
// rather than runtime behaviour — and because a character DEFINITION must be
// able to name the verb its moveset binds without reaching up into this crate.
// Re-exported so every `moveset::ATTACK_VERB`-style path is unchanged.
pub use ambition_entity_catalog::{ATTACK_VERB, RANGED_VERB, SMASH_VERB, SPECIAL_VERB};

/// [`HitVolume::vfx`] tags the move runtime knows (§7.2): the sweeping slash
/// arc and the grounded down-tilt's horizontal poke. Unknown tags draw the arc
/// (never a silent drop — a tagged volume asked for presentation).
pub const SLASH_ARC_VFX: &str = "slash_arc";
pub const SLASH_POKE_VFX: &str = "slash_poke";

/// The SFX cue a plain swing fires. Names the engine's procedural `slash` cue
/// (`ambition_sfx::ids::PLAYER_SLASH` = `"player.slash"`) so the audio runtime
/// resolves it to the guaranteed procedural sound — the old bespoke melee path
/// used `SfxMessage::Slash`, and the moveset must stay audible. (The prior
/// `"melee_swing"` string matched no bank sample and no procedural cue, so it
/// silently no-op-ed — the "no attack SFX" bug.)
pub const SWING_SFX_CUE: &str = "player.slash";
/// Dry blade-through-air cue reserved for the canonical robot protagonist.
pub const PLAYER_ROBOT_SWING_SFX_CUE: &str = "player.robot.slash.air";
/// Material selector carried by the canonical robot protagonist's slash volume.
/// The victim-side resolver ([`crate::util::resolve_strike_sfx`]) recognises it
/// by [`ambition_sfx::ids::PLAYER_ROBOT_SLASH_IMPACT`], so a cue authored here
/// that no longer hashes to that id would silently stop resolving to a material
/// variant and play the selector itself. Both spellings are therefore pinned to
/// the id table at compile time rather than trusted to stay in sync.
pub const PLAYER_ROBOT_IMPACT_SFX_CUE: &str = "player.robot.slash.impact";
const _: () = assert!(
    ambition_sfx::SfxId::from_static(PLAYER_ROBOT_SWING_SFX_CUE).hash()
        == ambition_sfx::ids::PLAYER_ROBOT_SLASH_AIR.hash()
);
const _: () = assert!(
    ambition_sfx::SfxId::from_static(PLAYER_ROBOT_IMPACT_SFX_CUE).hash()
        == ambition_sfx::ids::PLAYER_ROBOT_SLASH_IMPACT.hash()
);
/// Rebound cue the canonical robot protagonist's down-air pogo authors onto its
/// `pogo_bounce` effect. Every other body leaves it unauthored and keeps the
/// engine's generic pogo cue.
pub const PLAYER_ROBOT_POGO_SFX_CUE: &str = "player.robot.slash.impact.pogo";
const _: () = assert!(
    ambition_sfx::SfxId::from_static(PLAYER_ROBOT_POGO_SFX_CUE).hash()
        == ambition_sfx::ids::PLAYER_ROBOT_SLASH_IMPACT_POGO.hash()
);

// D-B split: the MoveSpec builders and actor-moveset construction live in
// `prefabs.rs`. Re-exported so `moveset::<builder>` paths (and `tests.rs`'s
// `use super::*`) are unchanged by the relocation.
mod prefabs;

pub use prefabs::*;

/// Marker: this body has a melee swing as a data-driven moveset `"attack"` move
/// (the ONLY melee path — the flat `BodyMelee` driver is gone). The swing is
/// triggered by [`trigger_moveset_moves`], run by [`advance_move_playback`], and
/// its `BodyMelee` read-model is projected from the live [`MovePlayback`] by
/// [`project_moveset_melee_to_body_melee`] so every consumer (actor anim index,
/// view/telegraph index, HUD) keeps reading the same shape. Every body whose
/// `ActionSet.melee` is `Some` carries this marker; it gates the projection
/// query so a body with no attack move publishes no phantom swing.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct MovesetMelee;

/// **Keep the routing markers agreeing with the moveset they route into.**
///
/// `MovesetMelee` and [`MovesetRanged`](ambition_characters::brain::MovesetRanged)
/// are not independent state — they are a projection of "does this moveset author
/// an `attack` / `ranged` verb". They were nonetheless written by hand at three
/// unrelated places (the actor cluster seed, the prepared-character projection)
/// and by NOBODY on the catalog persona path, which replaces `ActorMoveset`
/// wholesale on a kit swap and never touched them. The consequences are all
/// silent (GPT 5.6, 2026-07-27):
///
/// * a stale `MovesetMelee` diverts an attack into a timeline the new moveset
///   does not contain — the input is consumed and nothing happens;
/// * a missing `MovesetRanged` on a form that DOES author a ranged move routes it
///   back to the flat emitter, so the move's aim sampling never runs;
/// * a swap to a form with no routed moves keeps both.
///
/// So they are derived here, from the one authority, whenever it changes. Deriving
/// beats synchronizing: a third writer of `ActorMoveset` added tomorrow gets the
/// markers right without knowing they exist.
///
/// Spawn is still seeded by `ActorClusterSeed::into_components`, which is correct
/// and one tick earlier than this system could be — this reconciles CHANGES.
pub fn reconcile_moveset_routing_markers(
    mut commands: Commands,
    bodies: Query<
        (
            Entity,
            &ActorMoveset,
            bevy::prelude::Has<MovesetMelee>,
            bevy::prelude::Has<ambition_characters::brain::MovesetRanged>,
        ),
        bevy::prelude::Changed<ActorMoveset>,
    >,
) {
    for (entity, moveset, has_melee_marker, has_ranged_marker) in &bodies {
        let routes_melee = moveset.0.verbs.keys().any(|verb| is_melee_verb(verb));
        let routes_ranged = moveset.0.verbs.contains_key(RANGED_VERB);
        if routes_melee != has_melee_marker {
            if routes_melee {
                commands.entity(entity).insert(MovesetMelee);
            } else {
                commands.entity(entity).remove::<MovesetMelee>();
            }
        }
        if routes_ranged != has_ranged_marker {
            if routes_ranged {
                commands
                    .entity(entity)
                    .insert(ambition_characters::brain::MovesetRanged);
            } else {
                commands
                    .entity(entity)
                    .remove::<ambition_characters::brain::MovesetRanged>();
            }
        }
    }
}

/// A timed move event fired by [`advance_move_playback`]. The move runtime
/// stays content-free: it names the event; downstream consumers (the audio
/// bridge, content techniques via the `Effect` vocabulary) resolve keys.
#[derive(Message, Debug, Clone)]
pub struct MoveEventMessage {
    pub owner: Entity,
    pub move_id: String,
    /// Stable authored package that owns this move's presentation cues.
    ///
    /// Real playback derives it from the body's catalog character owner. The
    /// unscoped sentinel exists only for narrow fixtures and legacy synthetic
    /// bodies; dispatch then uses the active context's primary source.
    pub presentation_source: PresentationSourceId,
    pub kind: MoveEventKind,
}

/// This actor is playing a move. Insert to start; the system removes it when
/// the timeline completes. Facing locks at move start (the Smash convention —
/// a swing doesn't re-aim mid-animation).
#[derive(bevy::prelude::Component, Debug, Clone)]
#[component(map_entities)]
pub struct MovePlayback {
    pub spec: MoveSpec,
    /// `+1.0` faces right, `-1.0` left; mirrors every volume's x offset.
    pub facing: f32,
    /// **Was this body grounded when this move last looked?** Owned by the
    /// playback because the LANDING EDGE is a fact about this move's history,
    /// not about the body.
    ///
    /// ⛔ **seeded `true`, which reads backwards and is the point.** It means
    /// *no airborne observation yet*, so a move begun ON THE GROUND can never
    /// cross the edge on its first tick and be charged an aerial's landing lag
    /// — which is exactly what a `false` seed did, and what
    /// `a_grounded_move_never_pays_landing_lag` caught. The construction site
    /// cannot supply the real answer (`MovePlayback::new` sees no body), so the
    /// safe direction is to assume grounded until a tick observes otherwise.
    ///
    /// ⚠ the price, and it is nil in practice: a move that starts airborne and
    /// touches down within the SAME tick pays nothing. A body already on the
    /// floor when its move started is a grounded move.
    pub was_grounded: bool,
    /// Seconds of the OWNER'S proper time since move start.
    pub t: f32,
    /// CM4: this move CONNECTED with a victim. Set by the hit-resolution side
    /// (`mark_move_playback_landed_hits` + the volume resolver) and read by the
    /// cancel conditions (`OnHit`/`OnWhiff`) — the combo-confirm fact.
    pub landed_hit: bool,
    /// Live hitbox entity per entered-but-not-exited Active window index.
    ///
    /// A CACHE, not the authority. Its authority is `(t, window)`: the box exists
    /// exactly while the owner's clock is inside the window, and
    /// [`retire_orphaned_strike_volumes`] enforces that against the world every
    /// frame.
    ///
    /// Under GGRS (ADR 0027) this component is CLONED and entity-remapped across
    /// a rollback rather than rebuilt empty, so a restored cache can name a dead
    /// entity. That is fine BECAUSE the cache is not the authority: every slot is
    /// validated against the live world in `advance_move_playback` before it is
    /// believed, and an unbacked slot is dropped and respawned from `(t, window)`.
    /// Do not "optimize" that liveness check away — without it a strike silently
    /// whiffs for the rest of its window during resimulation.
    live_boxes: Vec<(usize, Entity)>,
    /// Which timed events already fired (parallel to `spec.events`).
    fired: Vec<bool>,
    /// One-hit-per-target dedup for a Player-faction Volume strike (the player's
    /// slash / pogo). The downstream Volume resolver (`apply_feature_hit_events`)
    /// folds each landed target key in, and the strike's per-tick emit ignores
    /// them — so a multi-tick Active window hits each target ONCE. It lives HERE,
    /// on the persistent per-strike move, NOT on the projected `BodyMelee.swing`:
    /// that swing is a read-model rebuilt every frame by
    /// `project_moveset_melee_to_body_melee`, which wiped the accumulator and made
    /// every active tick re-hit + re-fire the hit SFX (the old flat-swing dedup
    /// didn't survive the moveset projection). Starts empty per move.
    ///
    /// ⛔ **this used to say it "is not yet carried across a rollback resume".
    /// That was false and stale.** The registration is a CLONE snapshot, so the
    /// whole component including this list is restored; what was missing was the
    /// CHECKSUM projection, which is why two peers could disagree about who had
    /// already been struck and still agree on the hash. Both are right now — see
    /// `SnapshotResolve for MovePlayback`.
    pub hit_targets: Vec<String>,
    /// **The ranged intent that STARTED this move**, if one did.
    ///
    /// ⛔ **a move's fire event usually arrives after its own request is gone.**
    /// `ActorControl.fire` is an EDGE — `clear_edges()` nulls it every tick — and
    /// a ranged move has startup, so by the time its authored `Ranged` frame
    /// fires, the intent that triggered it has been cleared. The handler fell
    /// back to the body's horizontal facing, which is right for a forward shot
    /// and flattens every aim that was UP, DOWN or DIAGONAL: a move triggered
    /// with an upward aim fired sideways (GPT 5.6 review, 2026-08-04).
    ///
    /// Captured here at move start and consumed at the fire frame, so the shot
    /// carries the aim the player actually gave it. `None` for a move nobody
    /// aimed, which is what keeps the facing fallback meaningful.
    ///
    /// ⚠ the POLICY travels with the direction. A body-local `(1,0)` and a world
    /// `(1,0)` are different shots under non-default gravity, so storing the
    /// vector alone would re-introduce the frame confusion `dir_to_world` exists
    /// to prevent.
    pub aim: Option<(ae::Vec2, ae::GameplayFramePolicy)>,
}

impl bevy::ecs::entity::MapEntities for MovePlayback {
    fn map_entities<M: bevy::ecs::entity::EntityMapper>(&mut self, entity_mapper: &mut M) {
        for (_, entity) in &mut self.live_boxes {
            *entity = entity_mapper.get_mapped(*entity);
        }
    }
}

impl MovePlayback {
    pub fn new(spec: MoveSpec, facing: f32) -> Self {
        Self::new_at(spec, facing, 0.0)
    }

    /// Start a move with its clock pre-advanced to `t0` seconds (owner proper
    /// time). Used to SKIP a leading window: a boss strike commanded without a
    /// telegraph (possession, or a bare `Strike` step) starts at `t0 = telegraph
    /// window`, so its Active window is live immediately and the projected
    /// `active_elapsed` still folds in the telegraph offset (E53). Events with
    /// `at_s <= t0` are pre-marked fired so seeking past them doesn't retro-fire.
    /// **Resume a move mid-flight.** Historical: built for the deleted
    /// `ambition_platformer2d_runtime::snapshot` engine's
    /// `SnapshotResolve`.
    ///
    /// The blob carries the CHOICE — which move, how far in, did it land — and the
    /// `MoveSpec` is resolved back out of the owner's authored `ActorMoveset`. The
    /// `live_boxes` cache comes back empty, which is exactly right: a blob cannot
    /// carry an `Entity` (N3.1 decision 2), and it does not have to.
    /// [`retire_orphaned_strike_volumes`] despawns the boxes the rewound tick left
    /// standing, and the window's own `(inside, not-live)` arm re-spawns whatever the
    /// restored clock says should exist. The box's existence is DERIVED from
    /// `(t, window)`, so restoring `t` restores the box.
    pub fn resumed(spec: MoveSpec, facing: f32, t: f32, landed_hit: bool) -> Self {
        let mut pb = Self::new_at(spec, facing, t);
        pb.landed_hit = landed_hit;
        pb
    }

    pub fn new_at(spec: MoveSpec, facing: f32, t0: f32) -> Self {
        let t0 = t0.clamp(0.0, spec.duration_s);
        // ⛔ STRICTLY before `t0`, not `<=`. An event authored AT the start is the
        // common case, not an edge one: the player's swipe is `windup_s: 0.0`
        // precisely so "the arc and the swing cue all land on the frame of the
        // press", which puts its SFX event at `at_s == 0.0`. With `<=` that event
        // was pre-marked fired before the move began and could never sound — the
        // player's swing was silent from 2026-07-26 until this was found.
        //
        // The pre-marking exists so SEEKING past events does not retro-fire them,
        // and `<` still does that: seek to 0.5 and everything before 0.5 stays
        // quiet. An event exactly AT the seek target is one you seeked TO, so it
        // should still fire.
        let fired: Vec<bool> = spec.events.iter().map(|ev| ev.at_s < t0).collect();
        Self {
            spec,
            facing,
            // "No airborne observation yet" — see the field's doc for why the
            // seed is this way round.
            was_grounded: true,
            t: t0,
            landed_hit: false,
            live_boxes: Vec::new(),
            fired,
            hit_targets: Vec::new(),
            // Nobody aimed unless a caller says so — `with_aim` is the seam, and
            // the facing fallback at the fire frame is what an unaimed move gets.
            aim: None,
        }
    }

    /// Remember the ranged intent that started this move. See [`Self::aim`].
    pub fn with_aim(mut self, aim: Option<(ae::Vec2, ae::GameplayFramePolicy)>) -> Self {
        self.aim = aim;
        self
    }

    /// Normalized move progress — what presentation samples the bound clip
    /// by (the clip is SLAVED to the move; it never runs its own clock).
    pub fn phase(&self) -> f32 {
        self.spec.phase_at(self.t)
    }

    pub fn finished(&self) -> bool {
        self.t >= self.spec.duration_s
    }
}

/// A body's data-driven move repertoire — the Bevy-side carrier of a headless
/// [`MovesetContract`]. A body that exposes this contract triggers its moves
/// through [`trigger_moveset_moves`]: a control-frame verb edge inserts the
/// matching [`MovePlayback`]. This + [`dispatch_move_events`] are the production
/// seam the moveset system was missing (nothing ever created a `MovePlayback` in
/// the live game) — the first real consumer is the PCA's data-driven signature
/// move (fable review 2026-07-02 §A1, Path B: prove the moveset on a real actor
/// before folding the boss onto it). The boss fold reuses the SAME trigger +
/// dispatch — a boss is an actor whose repertoire happens to be large.
#[derive(Component, Debug, Clone)]
pub struct ActorMoveset(pub MovesetContract);

/// Which move window a spawned strike volume belongs to.
///
/// The volume's existence is DERIVED from `(owner's playback t, window)`. This marker
/// is what lets [`retire_orphaned_strike_volumes`] check that derivation against the
/// world without reading `MovePlayback`'s private cache — and without which a rollback
/// that rebuilds `MovePlayback` from a blob would strand every live box forever.
#[derive(bevy::prelude::Component, Clone, Copy, Debug)]
pub struct StrikeVolume {
    pub owner: Entity,
    pub window: usize,
}

impl bevy::ecs::entity::MapEntities for StrikeVolume {
    fn map_entities<M: bevy::ecs::entity::EntityMapper>(&mut self, mapper: &mut M) {
        self.owner = mapper.get_mapped(self.owner);
    }
}

/// **Despawn every strike volume whose owner's clock says it should not exist.**
///
/// Runs every frame, before [`advance_move_playback`], and is a no-op in the ordinary
/// case: the window's exit edge already despawned the box and dropped it from
/// `live_boxes`. It earns its keep when the two disagree, which happens when
/// `MovePlayback` is REPLACED rather than advanced:
///
/// - (historical) the deleted `ambition_platformer2d_runtime::snapshot::restore` rebuilt it from a
///   blob (`MovePlayback::resumed`) with an empty `live_boxes`; this system despawned
///   the boxes the rewound-from tick left standing. Under GGRS (ADR 0027) the playback
///   is instead CLONED + entity-remapped, so the failure mode INVERTS: a cloned slot
///   can name a dead `Entity` for a window that should be live. This system does not
///   repair that direction — `advance_move_playback` does, by validating each cached
///   slot against the live world before believing it.
/// - Any future code that swaps a playback mid-move.
///
/// N3.1's rule, honoured: *"if restoring something requires a rebuild pass, the rebuild
/// must be the SAME system that maintains it per-frame (no restore-only code paths)."*
/// This is that system, and it runs whether or not anyone ever rolls back.
pub fn retire_orphaned_strike_volumes(
    mut commands: Commands,
    volumes: Query<(Entity, &StrikeVolume)>,
    owners: Query<&MovePlayback>,
) {
    // Sorted by entity-independent key: this despawns, and despawn order is not
    // observable, but the ITERATION must not depend on archetype layout for any
    // future side effect. `(owner index, window)` is stable within a tick.
    for (volume, mark) in &volumes {
        let alive = owners.get(mark.owner).is_ok_and(|pb| {
            pb.spec
                .windows
                .get(mark.window)
                .is_some_and(|w| w.start_s <= pb.t && pb.t < w.end_s)
        }) && owners
            .get(mark.owner)
            .is_ok_and(|pb| pb.live_boxes.iter().any(|(_, e)| *e == volume));
        if !alive {
            commands.entity(volume).despawn();
        }
    }
}

/// Advance every playing move by its owner's proper time; manage
/// window-scoped hitboxes; fire timed events; retire finished moves.
pub fn advance_move_playback(
    mut commands: Commands,
    world_time: Res<WorldTime>,
    // The owner's per-tick resolved frame (ADR 0024), for rotating authored
    // body-local volumes into world space. Looked up by owner entity; a bare
    // test body without one uses the engine default down.
    owner_frames: Query<&ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame>,
    character_catalog: Res<ambition_characters::actor::character_catalog::CharacterCatalog>,
    character_owners: Option<
        Res<ambition_characters::actor::character_catalog::CharacterCatalogOwners>,
    >,
    authored_volumes: Res<super::authored_volumes::AuthoredAttackVolumeResolver>,
    mut events: MessageWriter<MoveEventMessage>,
    // §7.2: a vfx-tagged volume draws its slash FROM the spawned hitbox
    // geometry — one box drives damage AND presentation, so they can never
    // point different ways (the one-box-drives-damage-and-slash invariant; this
    // is the sole melee strike path).
    mut vfx: MessageWriter<ambition_vfx::vfx::VfxMessage>,
    mut players: Query<(
        Entity,
        &mut MovePlayback,
        &ActorFaction,
        // The owner's brain, so a POSSESSED body's strike carries its EFFECTIVE
        // faction (a controlled body fights as `Player`): `effective_faction`'s
        // contract is that every hitbox stamp resolves through it, and this move
        // strike is one of them. `None`/non-player-brain ⇒ the authored faction
        // (identity for every ordinary actor + the player's own body).
        Option<&ambition_characters::brain::Brain>,
        // §7.1: actors project their sprite catalog id onto combat tuning;
        // controllable bodies carry the same identity as WornCharacter. Both
        // resolve authored per-animation blade geometry from the App-local catalog.
        Option<&super::components::CombatTuning>,
        Option<&ambition_characters::actor::WornCharacter>,
        // A13's published attribution. THE authority on who this body sounds
        // like: it is derived from the prepared registry FIRST and the assembled
        // catalog second, so a character declared only through
        // `register_character` — which has no `CharacterCatalogOwners` entry at
        // all — still names its own provider here.
        Option<&ambition_sfx::BodyPresentationSource>,
        &ae::BodyKinematics,
        Option<&ProperTimeScale>,
        // I4: the owner's rollback-stable identity, so the transient strike volume
        // it opens can derive one. Without it every anonymous hitbox folded to the
        // same constant in the entity-reference probes, and swapped owners were
        // invisible. `None` (a bare test body) simply mints nothing.
        Option<&ambition_platformer2d_shared_tangle::sim_id::SimId>,
    )>,
    // Liveness oracle for the `live_boxes` cache. The cache is NOT the
    // authority — `(t, window)` is — so every cached slot is validated against
    // the world before it is believed. See the `(inside, Some(slot))` arm.
    //
    // It reads `HitboxHits` as well as answering liveness, because a strike that
    // hands off between contiguous windows must hand off WHO IT ALREADY HIT —
    // see the handoff carry below.
    live_strike_volumes: Query<&HitboxHits, With<StrikeVolume>>,
) {
    for (
        owner,
        mut playback,
        faction,
        brain,
        config,
        worn,
        body_source,
        kin,
        scale,
        owner_sim_id,
    ) in &mut players
    {
        let strike_faction = crate::targeting::effective_faction(*faction, brain);
        let character_id = worn
            .map(ambition_characters::actor::WornCharacter::id)
            .or_else(|| config.and_then(|tuning| tuning.sprite_character_id.as_deref()));
        // Read the published attribution; do NOT re-derive it. This function is
        // the ORIGINAL caller of `write_from`, and it kept its own owners-map
        // lookup after A13 hoisted the derivation onto the body — so the one
        // emitter the whole mechanism was built for was the one attributing
        // registered-only characters to nobody. `unscoped` then sent the cue to
        // the session's global emission context, where it was either credited to
        // the session owner's bank or denied outright.
        //
        // The fallback stays for bodies the publisher has not reached (an entity
        // spawned and striking inside the same tick, or a composition without
        // the character runtime): it is exactly the old behaviour, so this is
        // never worse than before and is right whenever the component exists.
        let presentation_source = body_source
            .map(|source| source.id().clone())
            .or_else(|| {
                character_id
                    .and_then(|id| {
                        character_owners
                            .as_deref()
                            .and_then(|owners| owners.provider_for(id))
                    })
                    .map(PresentationSourceId::new)
            })
            .unwrap_or_else(PresentationSourceId::unscoped);
        // ADR 0011: entity dt collapses to sim dt when the actor carries no
        // ProperTimeScale — undilated actors are the identity case.
        let dt = world_time.entity_dt(scale.copied().unwrap_or_default());
        let t_prev = playback.t;
        playback.t = (t_prev + dt).min(playback.spec.duration_s);
        let t = playback.t;

        // Timed events crossing (t_prev, t] fire exactly once, in order.
        // Split-borrow locals keep the fired flags and the spec readable
        // side by side.
        let pb = &mut *playback;
        for (idx, ev) in pb.spec.events.iter().enumerate() {
            // ⚠ no lower bound: `fired[idx]` already guarantees once-only, and a
            // `ev.at_s > t_prev` bound is unsatisfiable for an event at 0.0 on the
            // first advance, where `t_prev` is also 0.0. It added nothing except
            // that hole.
            if !pb.fired[idx] && ev.at_s <= t {
                pb.fired[idx] = true;
                events.write(MoveEventMessage {
                    owner,
                    move_id: pb.spec.id.clone(),
                    presentation_source: presentation_source.clone(),
                    kind: ev.kind.clone(),
                });
            }
        }

        // Sustained (held) effects: while `t` is inside a window carrying a
        // `sustain_effect`, emit its `Effect` EVERY frame — the consuming technique
        // times its own cadence off this per-frame "active this tick" signal. This
        // is how a move expresses a HELD special (a lingering beam, a continuous
        // rain), the shape the boss `apple_rain`-style specials need. Dilation
        // stretches the sustain the same way (fewer proper-time frames of it).
        for window in &pb.spec.windows {
            if let Some(effect) = &window.sustain_effect {
                if window.start_s <= t && t < window.end_s {
                    events.write(MoveEventMessage {
                        owner,
                        move_id: pb.spec.id.clone(),
                        presentation_source: presentation_source.clone(),
                        kind: MoveEventKind::Effect(effect.clone()),
                    });
                }
            }
        }

        // ⭐ **HITBOX TRACKS.** An attack whose shape moves through its swing is
        // authored as several Active windows laid end to end — the platform-
        // fighter "hitbox track", one strike sampled at keyframes. Each window
        // spawns its own box, so without this the arc would hit the same victim
        // once PER SEGMENT: a four-keyframe sword swing dealing quadruple damage.
        //
        // So a window that ends exactly where the next begins hands its hit set
        // forward. ⛔ **contiguity is the whole rule, and it is not a guess about
        // intent** — it is the literal continuity of the volume in time. The box
        // never left, so the strike never ended, so the victim is still struck.
        // A GAP means the box went away and came back, which is precisely what a
        // genuine multi-hit move (a drill, a rapid jab) is, and it rehits.
        //
        // The carry only has to survive within one tick: contiguous windows hand
        // off on the single tick where the clock crosses their shared edge.
        // Nothing about it is rollback state, which is why this costs no wire
        // format. ⚠ it does assume `windows` is authored in time order, which
        // every spec is and `MoveFrameData` already relies on.
        let mut handoff: Vec<(f32, Vec<std::collections::HashSet<Entity>>)> = Vec::new();

        // Active windows: spawn volumes on entry, despawn on exit. The box
        // lives exactly while the OWNER'S clock is inside the window, so
        // dilation stretches the box's world-time life automatically.
        for (w_idx, window) in pb.spec.windows.iter().enumerate() {
            if !matches!(window.tag, WindowTag::Active) || window.volumes.is_empty() {
                continue;
            }
            let inside = window.start_s <= t && t < window.end_s;
            // Validate the cache against the WORLD before trusting it. A cached
            // slot naming an entity that no longer exists is treated as absent,
            // so the window re-spawns its volume.
            //
            // This is what makes the "existence is DERIVED from `(t, window)`"
            // contract actually hold under GGRS (ADR 0027): bevy_ggrs restores
            // `MovePlayback` by CLONING it and remapping entities, so after a
            // LoadWorld a slot can name a despawned/unmappable entity for a
            // window the restored clock says is active. Believing the stale slot
            // meant the strike silently whiffed for the rest of the window
            // during resimulation. `retire_orphaned_strike_volumes` only covers
            // the mirror case (a live box whose owner forgot it), so this arm
            // owns the other direction — and, being mechanism-agnostic, it also
            // covers any future path that despawns a volume out from under a
            // playback.
            let live_slot = pb
                .live_boxes
                .iter()
                .position(|(idx, entity)| *idx == w_idx && live_strike_volumes.contains(*entity));
            if !inside || live_slot.is_none() {
                // Drop stale slots for this window so the cache cannot grow a
                // dead entry that blocks the despawn arm later.
                pb.live_boxes
                    .retain(|(idx, entity)| *idx != w_idx || live_strike_volumes.contains(*entity));
            }
            match (inside, live_slot) {
                (true, None) => {
                    // Authored volume offsets are BODY-LOCAL (side, down); rotate
                    // them through the owner's gravity frame at spawn — so an
                    // authored above-the-head volume stays above the head under any
                    // gravity (fable review 2026-07-02 §B1: the unrotated form
                    // spawned it screen-up, into a sideways body's ceiling).
                    let body_frame = owner_frames
                        .get(owner)
                        .map(|frame| frame.basis())
                        .unwrap_or(ae::AccelerationFrame::new(ae::DEFAULT_GRAVITY_DIR));
                    let frame_down = body_frame.down;
                    // CM3: the smash-charge payoff. The scale interpolates
                    // `1.0 → smash_charge_mult` by the charge fraction reached at
                    // this release instant (`t`, the owner's clock), so a held
                    // smash lands harder than a tap. `1.0` (every non-charge move)
                    // leaves damage/knockback byte-identical — parity.
                    let charge_scale = pb.spec.charge_scale_at(t);
                    for (v_idx, volume) in window.volumes.iter().enumerate() {
                        // §7.1: a vfx-tagged (bladed) volume prefers the owner's
                        // AUTHORED manifest hit polygon for this move's clip —
                        // the box you author and see in `debug-hitboxes` IS the
                        // gameplay damage box, restored onto the moveset path.
                        // Directional variants rebind `clip`, so `attack_up` /
                        // `attack_down` resolve their own rows the day they're
                        // authored. Resolved body-LOCAL (origin, facing +1,
                        // screen-down); the hitbox's own facing/frame_down
                        // mirror + rotate it at query time (`place_at`), the
                        // same math the bespoke path applied. `None` (no
                        // authored row / silent volume) falls back to the
                        // synthetic authored shape.
                        let manifest = volume.vfx.as_ref().and_then(|_| {
                            let clip = pb.spec.clip.clip.as_str();
                            let sprite_cid = config
                                .and_then(|c| c.sprite_character_id.as_deref())
                                .or_else(|| {
                                    worn.map(ambition_characters::actor::WornCharacter::id)
                                });
                            authored_volumes.resolve(
                                &character_catalog,
                                sprite_cid,
                                clip,
                                ae::Vec2::ZERO,
                                kin.size,
                                1.0,
                                ae::Vec2::new(0.0, 1.0),
                            )
                        });
                        let (local, half_extent, shape) = match &manifest {
                            // The authored convex blade: body-local points; the
                            // hitbox anchors at the body and `place_at` mirrors
                            // + gravity-rotates the hull each query.
                            Some(ae::CombatVolume::Convex { points, bounds }) => (
                                ae::Vec2::ZERO,
                                bounds.half_size(),
                                Some(ae::VolumeShape::Convex {
                                    points: points.clone(),
                                }),
                            ),
                            // The authored bbox fallback: same spawn-time
                            // resolution as a synthetic Rect.
                            Some(vol) => {
                                let b = vol.bounds();
                                let c = b.center();
                                (ae::Vec2::new(c.x * pb.facing, c.y), b.half_size(), None)
                            }
                            None => match volume.shape {
                                VolumeShape::Rect {
                                    offset,
                                    half_extents,
                                } => (
                                    ae::Vec2::new(offset.0 * pb.facing, offset.1),
                                    ae::Vec2::new(half_extents.0, half_extents.1),
                                    None,
                                ),
                                VolumeShape::Circle { offset, radius } => (
                                    ae::Vec2::new(offset.0 * pb.facing, offset.1),
                                    ae::Vec2::splat(radius),
                                    Some(ae::VolumeShape::circle(radius)),
                                ),
                            },
                        };
                        let local_offset = body_frame.to_world(local);
                        // Axis-aligned extents rotate with the frame too (a
                        // circle's splat is rotation-invariant, so this is
                        // uniform).
                        let half_extent = body_frame.to_world_half(half_extent);
                        let hb = Hitbox {
                            // CM8: the authored strike sound rides the volume onto
                            // the spawned box, so it reaches the victim-side
                            // reaction (a sword and a claw are heard apart).
                            strike_sfx: volume.hit_sfx.as_deref().map(ambition_sfx::SfxId::new),
                            owner,
                            source: hit_side_from_actor_faction(strike_faction),
                            anchor: HitboxAnchor::FollowOwner { local_offset },
                            half_extent,
                            shape,
                            facing: pb.facing,
                            // CM3: charge scaling folds onto the authored base —
                            // damage rounds, knockback scales linearly. Both are
                            // identity at `charge_scale == 1.0` (parity).
                            damage: ((volume.damage as f32) * charge_scale).round() as i32,
                            // Authored moveset knockback is an ABSOLUTE launch
                            // speed in engine units (px/s), never a feel
                            // multiplier — see 2c465cc77 (a FeelScale here
                            // launched victims at ~100x intended speed).
                            knockback: crate::strike::HitboxKnockback::LaunchSpeed {
                                base: volume.knockback * charge_scale,
                                // CM1: the smash-percent growth term rides the
                                // volume through to victim-side scaling at overlap.
                                growth: volume.kb_growth,
                            },
                            // CM1: the authored launch direction rides the
                            // volume through to the victim-side resolver.
                            launch_dir: volume.launch_dir.map(|(x, y)| ae::Vec2::new(x, y)),
                            frame_down,
                        };
                        // §7.2: the slash VFX rides the SAME resolved volume the
                        // damage does (one box drives both) — emitted once at the
                        // Active edge.
                        //
                        // The volume goes across WHOLE. Until 2026-08-01 this
                        // site took `.bounds()` first and handed presentation a
                        // box, which is the step that made the drawn art
                        // unfittable to the hull that hurts.
                        if let Some(tag) = &volume.vfx {
                            let kind = if tag == SLASH_POKE_VFX {
                                ambition_vfx::vfx::SlashKind::Poke
                            } else {
                                ambition_vfx::vfx::SlashKind::Arc
                            };
                            // The AERIAL clips are named `air_*`, not
                            // `attack_air*` — `directional_attack_variants`
                            // rebinds them that way on purpose, because that is
                            // what the sprite manifests author their hit polys
                            // under. This match never learned the second set of
                            // names, so the up-air and the down-air fell to
                            // `Side` and drew a horizontal crescent turned to
                            // point up or down. Rotation hid it: the art aimed
                            // the right way, and was the wrong art.
                            let pose = match pb.spec.clip.clip.as_str() {
                                "attack_up" | "air_up" => ambition_vfx::vfx::SlashPose::Up,
                                "attack_down" | "air_down" => ambition_vfx::vfx::SlashPose::Down,
                                _ => ambition_vfx::vfx::SlashPose::Side,
                            };
                            crate::util::emit_melee_slash(
                                &mut vfx,
                                &hb.world_volume(kin.pos),
                                kin.pos,
                                owner,
                                kind,
                                pose,
                            );
                        }
                        // NO HitboxLifetime on purpose: the window's exit
                        // edge (owner proper time) is the despawn authority,
                        // not a wall-clock countdown.
                        // The track handoff: a window opening exactly where an
                        // Active window closed this tick inherits who that box
                        // already hit, so one swing is one hit per victim.
                        let carried = handoff
                            .iter()
                            .find(|(end_s, _)| *end_s == window.start_s)
                            .and_then(|(_, sets)| sets.get(v_idx).cloned())
                            .unwrap_or_default();
                        let mut ec = commands.spawn((
                            hb,
                            HitboxHits { hit: carried },
                            StrikeVolume {
                                owner,
                                window: w_idx,
                            },
                        ));
                        // I4: a stable identity for the transient box, derived from
                        // rollback state only (owner id, move, window, volume) so
                        // the resim mints the same one. It is what the
                        // entity-reference probes fold the CARRIER through; an
                        // anonymous carrier makes a permutation among carriers
                        // invisible, which is precisely the case those probes exist
                        // for.
                        if let Some(owner_sim_id) = owner_sim_id {
                            ec.insert(
                                ambition_platformer2d_shared_tangle::sim_id::SimId::strike_volume(
                                    owner_sim_id,
                                    pb.spec.id.as_str(),
                                    w_idx,
                                    v_idx,
                                ),
                            );
                        }
                        // Conditional on-hit technique (pogo, lifesteal, …): a
                        // volume authoring `on_hit` gets the sidecar the
                        // resolved-hit on-hit projection reads (fable AJ1).
                        if let Some(effect) = &volume.on_hit {
                            ec.insert(super::on_hit::HitboxOnHit::new(effect.clone()));
                        }
                        let hitbox = ec.id();
                        pb.live_boxes.push((w_idx, hitbox));
                    }
                }
                (false, Some(_)) => {
                    // Carry this window's hit sets forward before the boxes go,
                    // in spawn order so volume `v` hands to volume `v`. The
                    // despawn is a deferred command, but reading now is simpler
                    // than reasoning about when it lands.
                    handoff.push((
                        window.end_s,
                        pb.live_boxes
                            .iter()
                            .filter(|(idx, _)| *idx == w_idx)
                            .map(|(_, entity)| {
                                live_strike_volumes
                                    .get(*entity)
                                    .map(|hits| hits.hit.clone())
                                    .unwrap_or_default()
                            })
                            .collect(),
                    ));
                    pb.live_boxes.retain(|(idx, entity)| {
                        if *idx == w_idx {
                            commands.entity(*entity).despawn();
                            false
                        } else {
                            true
                        }
                    });
                }
                _ => {}
            }
        }

        if pb.finished() {
            for (_, entity) in pb.live_boxes.drain(..) {
                commands.entity(entity).despawn();
            }
            commands.entity(owner).remove::<MovePlayback>();
        }
    }
}

/// **An aerial move that touches down before it ended owes its authored landing
/// lag — unless it auto-cancelled.**
///
/// The platform-fighter commitment rule, and the reason spacing an aerial is a
/// decision: you throw it knowing that landing mid-move costs you. A move that
/// authors neither field lands the way it always did, so this is inert for
/// every move that has not opted in.
///
/// ⭐ **body-generic by construction.** It reads `MovePlayback` and
/// `BodyGroundState`, which every body carries — a CPU fighter, a possessed
/// boss and a human all pay the same lag for the same move. There is no
/// controller in the query.
///
/// ⚠ **the landing EDGE, not the grounded state.** A move begun on the ground
/// (a jab, a down-tilt) is never mid-air, so it can never cross the edge and
/// never pays. That is why the previous grounded-ness is remembered on the
/// playback rather than re-derived: "is grounded now" would charge every
/// ground move its landing lag on the frame it started.
pub fn resolve_aerial_landings(
    mut commands: Commands,
    mut bodies: Query<(
        Entity,
        &mut MovePlayback,
        &ambition_platformer2d_core::BodyGroundState,
        &mut ambition_characters::actor::BodyCombat,
    )>,
) {
    for (owner, mut playback, ground, mut combat) in &mut bodies {
        let was_airborne = !playback.was_grounded;
        playback.was_grounded = ground.on_ground;
        if !was_airborne || !ground.on_ground {
            continue;
        }
        // Landed this frame, out of a move that was still running.
        let Some(lag) = playback.spec.landing_lag_s else {
            continue;
        };
        // Auto-cancel: the dangerous part is over, so the landing is clean.
        if playback
            .spec
            .autocancel_after_s
            .is_some_and(|after| playback.t >= after)
        {
            continue;
        }
        combat.landing_lag_timer = combat.landing_lag_timer.max(lag.max(0.0));
        // The move is OVER — its remaining windows do not survive the landing,
        // which is what makes the lag a cost rather than a delay.
        for (_, entity) in playback.live_boxes.drain(..) {
            commands.entity(entity).despawn();
        }
        commands.entity(owner).remove::<MovePlayback>();
    }
}

/// Reduce an attack aim axis to a discrete [`AttackDir`] for directional move
/// selection, relative to the body's current `facing` (±1).
///
/// The `axis` arrives gravity/screen-local (`+x` = screen-right, `+y` =
/// gravity-down), the same value as locomotion — it is NOT pre-mirrored by
/// facing. But [`AttackDir`] is FACING-relative (`+x` = facing, `Back` = opposite
/// facing), so the horizontal arm folds facing in: `axis.x * facing` is the
/// forward/back projection. Without this, pressing toward the way you just turned
/// (move left → `axis.x < 0` while `facing = -1`) misreads as `Back` and fires
/// the aerial back-attack in the wrong direction. This is the SAME transform
/// `resolve_attack_intent_from_view` and the aim helper (`compute_aim`) apply. The vertical arm is gravity-local (Up = toward
/// the head under ANY gravity), so `y < 0` is Up with no facing term. Vertical
/// wins ties so a clear up/down aim beats slight horizontal drift.
pub fn attack_dir_from_axis(axis: ae::LocalAxes, facing: f32) -> AttackDir {
    ambition_characters::actor::attack_gesture::attack_dir_from_axis(axis, facing, 0.5)
}

/// Interpret raw actor-control edges into deterministic semantic attack edges.
/// This runs once for every body immediately before move triggering. The
/// multi-tick state is rollback-authoritative; the resolved frame is derived.
pub fn resolve_attack_gestures(
    mut bodies: Query<(
        &ActorControl,
        &ae::BodyKinematics,
        Option<&ambition_platformer2d_core::BodyGroundState>,
        &mut AttackGestureState,
        &AttackGestureTuning,
        &mut ResolvedAttackGesture,
    )>,
) {
    for (control, kin, ground, mut state, tuning, mut resolved) in &mut bodies {
        let frame = &control.0;
        *resolved = resolve_attack_gesture(
            &mut state,
            *tuning,
            frame.attack_axis,
            kin.facing,
            ground.map(|g| g.on_ground).unwrap_or(true),
            frame.melee_pressed,
            frame.melee_held,
            frame.melee_released,
            frame.melee_strong_hint,
        );
    }
}

/// TRIGGER a body's data-driven move from its control-frame verb edges: a
/// `special_pressed` → the DIRECTIONAL `"special"` verb, a `melee_pressed` →
/// the DIRECTIONAL `"attack"` verb (resolved by aim + grounded state through the
/// authored verb chain — `attack_air_down` → `attack_down` → `attack`), a ranged
/// intent → the `"ranged"` verb. A body already playing a move refuses a new one — the move's
/// own duration IS the fire-rate gate — UNLESS the playing move authors a
/// `Cancelable` window covering this instant whose condition holds and whose
/// `into` names the request (CM4): then the live boxes tear down exactly as
/// natural completion does and the new move starts same-frame. `jump`/`dash`
/// entries END the move early on those edges — the normal locomotion path
/// (reading the SAME control frame this tick) performs the jump/dash itself;
/// no second dispatcher. An empty cancel timeline is byte-identical to the
/// pre-CM4 reject (the parity pin). Facing locks at trigger from the body's
/// kinematics (the Smash convention — a committed swing doesn't re-aim).
///
/// ONE trigger seam for every body (guardrail #1): the same system drives an
/// actor's melee, the PCA's signature move, a folded boss's pattern, and the
/// player's directional repertoire (R2.5). A body authoring only `"attack"`
/// resolves every direction to it — byte-identical to the pre-directional path.
///
/// ⭐ **A WEAPON IN HAND OWNS THE ATTACK PRESS** — see
/// [`held_weapon_attack_move`]. The wearer's own `attack` verbs keep existing;
/// they simply stop being what that press reaches.
/// **Name the writer of a move's self-motion impulse.**
///
/// Two call sites author the same impulse — the plain trigger and the CANCEL
/// path — and they are byte-identical expressions. Naming them apart is the
/// point: "a move moved this body" is one answer, "a CANCEL moved this body"
/// is a different bug.
///
/// The fact itself comes from `ambition_causal::velocity_authored`, so this
/// writer's story has the same shape as every other one. The move id rides
/// along as an extra field rather than being folded into the label — a label is
/// a SITE, and there is one site per arm however many moves flow through it.
#[cfg(feature = "causal")]
#[allow(clippy::too_many_arguments)]
fn record_impulse_authorship(
    log: Option<&mut ambition_causal::CausalRecording>,
    identities: &Query<&crate::components::ActorIdentity>,
    tick: Option<&ambition_time::SimTick>,
    body: Entity,
    writer: &'static str,
    move_id: &str,
    before: ae::Vec2,
    after: ae::Vec2,
    _impulse: ae::Vec2,
) {
    let (Some(log), Ok(identity)) = (log, identities.get(body)) else {
        return;
    };
    if !log.is_recording() {
        return;
    }
    log.record(
        ambition_causal::velocity_authored(
            tick.map_or(0, |t| t.get()),
            ambition_causal::SubjectKey::Sim(identity.id.clone()),
            writer,
            before.x,
            after.x,
        )
        .field("move_id", move_id.to_string()),
    );
}

/// **The swing a held weapon answers a directional Attack with** — `None` when
/// that weapon answers the press somewhere other than the move runtime.
///
/// Jon, on the gun-sword: *"When I have the laser sword and I use it, I
/// incorrectly still use my normal jab attack. Holding an item should reroute
/// normal attack actions to the item action, which might be like throw for
/// bombs or fire for the gun sword … the default for the gun sword is they all
/// route to the one action the item has: shoot."*
///
/// The defect was **two claimants on one press**, the same shape
/// `revoke_host_owned_ranged` fixed for ranged one slot over: equipping already
/// cleared the item's melee out of the wearer's `ActionSet`, but the wearer's
/// MOVESET still bound `attack`, so the jab ran beside the bolt. The `ActionSet`
/// and the moveset are a UNION for the Attack slot
/// (`ambition_characters::action_scheme::combat_actions`), and only one half was
/// ever displaced.
///
/// So the press is arbitrated HERE, by identity, from the one authority on what
/// a body is holding:
///
/// - the weapon authors a melee verb (an axe) → **its** swing answers, built
///   through the same [`build_actor_moveset`] a spawned body's would be, so the
///   whole directional family (tilts, aerials, the pogo down-air) comes with it;
/// - the weapon authors no melee verb (the gun-sword's bolt, a bomb's throw, a
///   gauntlet's bespoke system) → **nothing** answers here, and the item's own
///   subject-generic system consumes the press it already reads.
///
/// ⛔ **the wearer's `attack` MOVES are not pruned, and must not be.** A
/// timeline nothing presses is inert; deleting it on a reachability argument
/// throws away authored content and makes unequipping a restore problem. Only
/// the RESOLUTION moves, and it moves back the instant the hand is empty — so
/// there is nothing to stash, and a rewind past an equip is correct for free
/// (`HeldItem` is already rollback state).
///
/// ⛔ **and the slot must survive, which is why this is not a verb revoke.**
/// `touch_action_available` draws — and admits touches for — an on-screen Attack
/// button only while `ControlPrompt` carries a label for the Attack slot, and
/// that label comes from the scheme's union of moveset verb and `ActionSet`
/// melee. A guard that took the wearer's `attack` verbs away would leave the
/// gun-sword with no Attack slot at all: still fireable on a desktop (the
/// persona gate's `holds_item` exception keeps `melee_pressed` alive) and
/// **untappable on a phone**. Resolving the press instead of deleting the verb
/// keeps the button drawn.
fn held_weapon_attack_move(
    spec: &ambition_characters::brain::HeldItemSpec,
    dir: AttackDir,
    grounded: bool,
) -> Option<MoveSpec> {
    let melee = spec.melee.as_ref()?;
    // Built per press rather than cached: the alternative is a second copy of
    // the wearer's contract to keep coherent across equip / unequip / rewind,
    // and this runs once on a press edge for the one body holding the weapon.
    build_actor_moveset(None, Some(melee), None, None)?
        .move_for_directional_verb(ATTACK_VERB, dir, grounded)
        .cloned()
}

pub fn trigger_moveset_moves(
    mut commands: Commands,
    mut bodies: Query<(
        Entity,
        &ActorMoveset,
        &ActorControl,
        &ResolvedAttackGesture,
        // The body's per-tick resolved frame (ADR 0024): a move's authored
        // body-local start impulse rotates through the SAME frame the body's
        // movement integrated under. `Option` for bare test bodies.
        Option<&ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame>,
        // Mutable so a move's authored `start_impulse` (self-motion) lands at
        // trigger — the move-start seam.
        &mut ae::BodyKinematics,
        // Grounded state selects tilt-vs-air variants. Absent on bare test
        // bodies → treated as grounded (immaterial: such bodies author only
        // the base `attack`, which every direction resolves to).
        Option<&ambition_platformer2d_core::BodyGroundState>,
        // The playing move, if any — the CM4 cancel seam. `None` = the plain
        // trigger path.
        Option<&mut MovePlayback>,
        // **What this body is holding.** A weapon in hand OWNS the Attack press
        // ([`held_weapon_attack_move`]); every other verb is untouched.
        Option<&crate::held_items::HeldItem>,
    )>,
    // ⛔ **WHO WROTE THIS BODY'S VELOCITY.** A move's `start_impulse` is a
    // velocity write outside the integrator, and the causal log reported what
    // the velocity WAS and never who set it — which cost six rebuild-and-print
    // cycles on one 12-tick window before the first authored fact existed
    // (queue S51). This is the second writer to name itself.
    //
    // ⚠ `Option` on both: the FEATURE and the PLUGIN are two switches, and a
    // body without an identity is still a body.
    #[cfg(feature = "causal")] log: Option<bevy::prelude::ResMut<ambition_causal::CausalRecording>>,
    #[cfg(feature = "causal")] identities: Query<&crate::components::ActorIdentity>,
    #[cfg(feature = "causal")] tick: Option<bevy::prelude::Res<ambition_time::SimTick>>,
) {
    #[cfg(feature = "causal")]
    let mut log = log;
    for (entity, moveset, control, gesture, resolved_frame, mut kin, ground, playback, held) in
        &mut bodies
    {
        let body_frame = resolved_frame
            .map(|frame| frame.basis())
            .unwrap_or(ae::AccelerationFrame::new(ae::DEFAULT_GRAVITY_DIR));
        let frame = &control.0;
        let grounded = ground.map(|g| g.on_ground).unwrap_or(true);
        // Resolve the requested verb + the names the candidate answers to
        // (verb, class, resolved move id — the ONE cancel namespace).
        //
        // OWNED rather than borrowed from the contract, because a held weapon's
        // swing is not IN the wearer's contract — it belongs to the thing in the
        // hand, and both answers have to have the same type to be arbitrated.
        let (spec, verb_names): (Option<MoveSpec>, &[&str]) = if frame.special_pressed {
            let dir = attack_dir_from_axis(frame.attack_axis, kin.facing);
            (
                moveset
                    .0
                    .move_for_directional_verb(SPECIAL_VERB, dir, grounded)
                    .cloned(),
                &[SPECIAL_VERB],
            )
        } else if gesture.pressed.is_some() || frame.pogo_pressed {
            // A dedicated pogo press IS a down-air (the move carrying the pogo
            // on-hit technique); a plain melee press resolves by aim. When only
            // pogo is pressed, force Down so an aerial body reaches `attack_air_down`.
            let (base_verb, dir, gesture_grounded) =
                if frame.pogo_pressed && gesture.pressed.is_none() {
                    (ATTACK_VERB, AttackDir::Down, grounded)
                } else if let Some(intent) = gesture.pressed {
                    (
                        if intent.strength == AttackStrength::Smash {
                            SMASH_VERB
                        } else {
                            ATTACK_VERB
                        },
                        intent.direction,
                        intent.posture == AttackPosture::Grounded,
                    )
                } else {
                    (ATTACK_VERB, AttackDir::Down, grounded)
                };
            // ⭐ **A WEAPON IN HAND OWNS THIS PRESS.** With something held, the
            // wearer's own repertoire is not consulted at all — the weapon
            // answers with its swing, or it answers elsewhere and nothing runs
            // here (see [`held_weapon_attack_move`] for why this arbitrates
            // instead of revoking the wearer's verbs).
            let spec = if let Some(held) = held {
                held_weapon_attack_move(&held.spec, dir, gesture_grounded)
            } else {
                moveset
                    .0
                    .move_for_directional_verb(base_verb, dir, gesture_grounded)
                    .or_else(|| {
                        if base_verb == SMASH_VERB {
                            moveset
                                .0
                                .move_for_directional_verb(ATTACK_VERB, dir, gesture_grounded)
                        } else {
                            None
                        }
                    })
                    .cloned()
            };
            let verb_names: &[&str] = if base_verb == SMASH_VERB {
                &[SMASH_VERB, ATTACK_VERB, "any_attack"]
            } else {
                &[ATTACK_VERB, "any_attack"]
            };
            (spec, verb_names)
        } else if frame.fire.is_some() {
            // A ranged intent (`frame.fire = Some(dir)`) starts the body's `"ranged"`
            // move; its fire event spawns the projectile, sampling live aim. The move
            // plays to completion before another starts (its duration is a cadence
            // gate; the body-side refire cooldown remains the hard rate floor).
            (
                moveset.0.move_for_verb(RANGED_VERB).cloned(),
                &[RANGED_VERB],
            )
        } else {
            (None, &[])
        };

        if let Some(mut pb) = playback {
            // CM4, locomotion escapes: a `jump`/`dash` edge inside a permitting
            // cancel window ENDS the move (early recovery-cancel); the verb
            // itself runs through the normal locomotion path this same tick.
            let loco = if frame.jump_pressed {
                Some("jump")
            } else if frame.dash_pressed {
                Some("dash")
            } else {
                None
            };
            if let Some(name) = loco {
                if pb.spec.cancel_permits(pb.t, pb.landed_hit, &[name]) {
                    for (_, e) in pb.live_boxes.drain(..) {
                        commands.entity(e).despawn();
                    }
                    commands.entity(entity).remove::<MovePlayback>();
                    continue;
                }
            }
            // CM4, move-into-move: the requested move starts same-frame iff a
            // cancel window covering `t` permits it under the hit-state
            // condition. Otherwise: today's reject, byte-identically.
            let Some(spec) = spec else { continue };
            let mut names: Vec<&str> = verb_names.to_vec();
            names.push(spec.id.as_str());
            if !pb.spec.cancel_permits(pb.t, pb.landed_hit, &names) {
                continue;
            }
            // Tear down exactly as natural completion does (the ONE teardown
            // path), then replace the playback — insert overwrites.
            for (_, e) in pb.live_boxes.drain(..) {
                commands.entity(e).despawn();
            }
            if let Some((ix, iy)) = spec.start_impulse {
                let local = ae::Vec2::new(ix * kin.facing, iy);
                let world_impulse = body_frame.to_world(local);
                let before = kin.vel;
                kin.vel += world_impulse;
                #[cfg(feature = "causal")]
                record_impulse_authorship(
                    log.as_deref_mut(),
                    &identities,
                    tick.as_deref(),
                    entity,
                    "move_start_impulse_cancel",
                    &spec.id,
                    before,
                    kin.vel,
                    world_impulse,
                );
                let _ = before;
            }
            commands
                .entity(entity)
                // ⭐ **capture the aim at START, because it will be gone by the
                // fire frame.** See `MovePlayback::aim`.
                .insert(
                    MovePlayback::new(spec, kin.facing)
                        .with_aim(control.0.fire.map(|req| (req.dir, req.dir_policy))),
                );
            continue;
        }

        if let Some(spec) = spec {
            // Self-motion: a body-local impulse mirrored by facing and rotated
            // into the owner's gravity frame (a jab's lunge stays "forward"
            // under any gravity). Identity when the move authors none.
            if let Some((ix, iy)) = spec.start_impulse {
                let local = ae::Vec2::new(ix * kin.facing, iy);
                let world_impulse = body_frame.to_world(local);
                let before = kin.vel;
                kin.vel += world_impulse;
                #[cfg(feature = "causal")]
                record_impulse_authorship(
                    log.as_deref_mut(),
                    &identities,
                    tick.as_deref(),
                    entity,
                    "move_start_impulse",
                    &spec.id,
                    before,
                    kin.vel,
                    world_impulse,
                );
                let _ = before;
            }
            commands.entity(entity).insert(
                // ⚠ **the plain trigger path needs the aim just as much as the
                // cancel path above.** Capturing on one of the two start sites
                // would have fixed aimed shots only for moves that interrupted
                // another one, which is the rarer half.
                MovePlayback::new(spec, kin.facing)
                    .with_aim(control.0.fire.map(|req| (req.dir, req.dir_policy))),
            );
        }
    }
}

/// CM4: mark the attacker's playing move as CONNECTED from the same resolved
/// body-hit fact on-hit techniques consume.
///
/// The contact resolver owns the meaning of "landed". Move cancels do not infer
/// it from `HitTarget`, and on-hit effects do not perform their own overlap pass;
/// both consume [`crate::hitbox::LandedBodyHit`]. World/broadcast effects that do
/// not resolve a body victim continue to mark their own outcome at their own
/// resolution seam.
pub fn mark_move_playback_landed_hits(
    mut landed_hits: MessageReader<crate::hitbox::LandedBodyHit>,
    mut playbacks: Query<&mut MovePlayback>,
) {
    for landed in landed_hits.read() {
        if let Ok(mut pb) = playbacks.get_mut(landed.attacker) {
            pb.landed_hit = true;
        }
    }
}

/// Consume [`MoveEventMessage`]s — the moveset runtime is content-free, it only
/// NAMES events; this resolves them:
/// - `Sfx { cue }` → play the cue at the owner's position.
/// - `Effect { key }` → BRIDGE to the existing content-technique seam by writing
///   the SAME `ActorActionMessage::Special { Special(key) }` the brain special path
///   emits, so every content `Technique` consumer fires unchanged. This is the
///   exact seam the boss's `Special(key)` profiles reuse once the boss folds onto
///   the moveset — a data-driven move fires a content technique with zero new
///   plumbing (fable review §A1, Path B).
/// - `Ranged` → BRIDGE to the existing enemy-projectile seam by writing the SAME
///   `ActorActionMessage::Ranged` the flat `frame.fire` resolver emits, so the
///   mature `spawn_enemy_projectiles_from_brain_actions` consumer (body-side
///   fire-rate, recoil, muzzle, visual kind) fires the shot unchanged. The shot's
///   direction is SAMPLED LIVE from the owner's current `fire` intent at THIS event
///   frame (option A — a moveset shot still tracks a strafing target, unlike a
///   facing-locked `MovePlayback`); with no live intent it falls back to forward.
pub fn dispatch_move_events(
    mut events: MessageReader<MoveEventMessage>,
    positions: Query<&ae::BodyKinematics>,
    ranged_owners: Query<(
        &ambition_characters::brain::action_set::ActionSet,
        &ActorControl,
        // A3: the owner's worn equipment, so a worn Move/Verb-scoped modifier
        // scales the shot's damage/speed at fire (trigger-resolve).
        Option<&ambition_characters::equipment::WornEquipment>,
    )>,
    // The move that is playing, for the aim it was STARTED with. See
    // `MovePlayback::aim`.
    playbacks: Query<&MovePlayback>,
    mut sfx: SfxWriter,
    mut vfx: MessageWriter<ambition_vfx::VfxMessage>,
    mut actions: MessageWriter<ActorActionMessage>,
) {
    for ev in events.read() {
        match &ev.kind {
            MoveEventKind::Sfx { cue } => {
                let pos = positions
                    .get(ev.owner)
                    .map(|k| k.pos)
                    .unwrap_or(ae::Vec2::ZERO);
                let request = SfxMessage::Play {
                    id: SfxId::new(cue),
                    pos,
                };
                if ev.presentation_source.is_unscoped() {
                    sfx.write(request);
                } else {
                    sfx.write_from(ev.presentation_source.clone(), request);
                }
            }
            MoveEventKind::Vfx { effect } => {
                // CM5 per-move cosmetic burst: resolve the id against the
                // content-registered vocabulary and spawn it at the owner. A
                // typo can't reach here — `presentation_problems` rejects an
                // unresolvable id at startup — but stay robust if it somehow
                // does (skip, never panic on the RL-hot path).
                let Some(kind) = ambition_vfx::move_vfx_kind(effect) else {
                    continue;
                };
                let pos = positions
                    .get(ev.owner)
                    .map(|k| k.pos)
                    .unwrap_or(ae::Vec2::ZERO);
                vfx.write(ambition_vfx::VfxMessage::Explosion {
                    pos,
                    kind,
                    scale: 1.0,
                });
            }
            MoveEventKind::Effect(effect) => {
                // Bridge to the content-technique seam by the effect KEY, and
                // thread the opaque `effect.params` through the `Special`
                // channel (A1 / R2.2) so the keyed technique can hydrate its own
                // typed params. A paramless effect carries the empty default, so
                // every existing content-const technique stays byte-identical.
                actions.write(ActorActionMessage {
                    actor: ev.owner,
                    request: ActionRequest::Special {
                        spec: SpecialActionSpec::Special(effect.key.clone()),
                        params: effect.params.clone(),
                    },
                });
            }
            MoveEventKind::Ranged => {
                // The owner's ranged CAPABILITY + LIVE aim supply the concrete shot;
                // the move stays content-free.
                let Ok((actions_set, control, worn)) = ranged_owners.get(ev.owner) else {
                    continue;
                };
                let Some(spec) = actions_set.ranged.clone() else {
                    continue; // owner has no ranged weapon — the move fires nothing
                };
                // A3 trigger-resolve: fold worn equipment modifiers into THIS shot's
                // speed/damage. No worn equipment (the common case) returns the spec
                // unchanged — parity by construction.
                let spec = match worn {
                    Some(worn) => ambition_characters::equipment::resolved_ranged(
                        spec,
                        worn,
                        &ev.move_id,
                        RANGED_VERB,
                    ),
                    None => spec,
                };
                let kin = positions.get(ev.owner).ok();
                let origin = kin.map(|k| k.pos).unwrap_or(ae::Vec2::ZERO);
                // Sample the owner's live aim at the fire frame, and fall back to
                // the body's FACING when there is none.
                //
                // ⛔ **the fallback used to be a bare `(1.0, 0.0)`, and its comment
                // said that was "forward (controlled-body-local +x = the body's
                // facing direction)". It is not.** `ControlledBodyLocal` +x is the
                // gravity frame's SIDE axis — `dir_to_world` applies
                // `AccelerationFrame::to_world` and nothing else — so under normal
                // gravity the fallback resolved to world +x and every such shot
                // went RIGHT, whichever way the body was looking.
                //
                // ⚠ and the fallback is the COMMON path, not the rare one.
                // `frame.fire` is an EDGE: `clear_edges()` nulls it every tick. A
                // ranged move has startup, so by the time its fire frame arrives
                // the intent that started it is already gone and this branch is
                // what runs. Jon reported it as "Maryo's fireball only shoots to
                // her right, not the way she is facing" — the demo passes
                // `kin.facing` correctly on the press, and the value never
                // survived to the shot.
                //
                // ⭐ **THREE tiers, and the middle one is the fix.** A live edge
                // this frame wins (a moveset shot still tracks a strafing target).
                // Otherwise the aim the move was STARTED with — captured into
                // `MovePlayback::aim`, because the request that triggered the move
                // has been cleared by the time its fire frame arrives, which is
                // the common case this comment already described. Only an
                // unaimed move falls through to facing.
                //
                // ⛔ **before the middle tier, an UPWARD aim fired sideways.** The
                // facing fallback repairs left-versus-right and flattens every
                // non-horizontal shot, and it was reached by essentially every
                // authored ranged move (GPT 5.6 review, 2026-08-04).
                let started_with = playbacks.get(ev.owner).ok().and_then(|pb| pb.aim);
                let (dir, dir_policy) = match (control.0.fire, started_with) {
                    (Some(req), _) => (req.dir, req.dir_policy),
                    (None, Some((dir, policy))) => (dir, policy),
                    (None, None) => (
                        ae::Vec2::new(kin.map(|k| k.facing.signum()).unwrap_or(1.0), 0.0),
                        ae::GameplayFramePolicy::ControlledBodyLocal,
                    ),
                };
                actions.write(ActorActionMessage {
                    actor: ev.owner,
                    request: ActionRequest::Ranged {
                        spec,
                        origin,
                        dir,
                        dir_policy,
                    },
                });
            }
        }
    }
}

/// Project a `MovesetMelee` body's live [`MovePlayback`] into its [`BodyMelee`]
/// read-model so every existing consumer — the actor anim index, the
/// view/telegraph index, the HUD, the melee integration tests — keeps working
/// unchanged after melee moved onto the moveset. The move's Active window(s) drive
/// a synthesized `MeleeSwing` whose phase (Startup/Active/Recovery) and elapsed
/// mirror the move; the real hitboxes/damage are owned by
/// [`advance_move_playback`], so this writes NO gameplay — it is purely the
/// read-model the flat `BodyMelee` swing used to publish. In particular, damage
/// resolution must never consult this projection as an authority gate: the live
/// strike volume is the authority. A body with no live move
/// has its projected swing cleared (its cooldown floors still tick in
/// `tick_body_melee_cooldowns`).
///
/// Runs AFTER `advance_move_playback` (so `t` is current). It is the SOLE writer
/// of a `MovesetMelee` body's swing — there is no flat melee driver competing for
/// it anymore.
pub fn project_moveset_melee_to_body_melee(
    mut bodies: Query<
        (Option<&MovePlayback>, Option<&ActorMoveset>, &mut BodyMelee),
        With<MovesetMelee>,
    >,
) {
    for (playback, moveset, mut melee) in &mut bodies {
        // Only a MELEE swing move projects a swing. A body's ranged shot
        // (`"ranged"`) or a special as a moveset move is ALSO `MovesetMelee`, and
        // those are NOT swings — projecting one would publish a phantom
        // `BodyMelee.swing` the movement pipeline reads as "mid-attack", freezing
        // a firing/special-ing body.
        match playback {
            Some(pb) if is_melee_swing_move(moveset.map(|m| &m.0), &pb.spec.id) => {
                melee.swing = Some(synth_swing_from_move(pb))
            }
            _ => melee.swing = None,
        }
    }
}

/// Which input verb a move answers in this moveset, if any.
///
/// The moveset binds verb → move id, so this is just the inverse. Deterministic:
/// `verbs` is a `BTreeMap`, and a move bound to two verbs answers to the first
/// in sort order — a case no authored moveset has, and one that must not depend
/// on hash order if it ever appears.
fn verb_for_move<'a>(moveset: &'a MovesetContract, id: &str) -> Option<&'a str> {
    moveset
        .verbs
        .iter()
        .find(|(_, bound)| bound.as_str() == id)
        .map(|(verb, _)| verb.as_str())
}

/// Whether an input verb belongs to the melee family.
///
/// Kept as one predicate for both routing-marker derivation and live playback
/// projection so a directional-only or smash-only moveset cannot be routed one
/// way and presented another.
fn is_melee_verb(verb: &str) -> bool {
    verb == ATTACK_VERB
        || verb.starts_with("attack_")
        || verb == SMASH_VERB
        || verb.starts_with("smash_")
}

/// Whether a move is a melee swing versus a ranged shot or a content special.
///
/// **Asks the VERB, not the name.** The derived movesets name every swing after
/// its verb (`attack` / `attack_up` / `attack_air_down`), so for years "does the
/// id start with `attack`" and "is it bound to the attack verb" were the same
/// question — and the id was the cheaper one to ask.
///
/// They stop being the same question the moment a moveset is hand-authored, and
/// a fighting game's move list is named after its MOVES (`jab`, `smash_forward`,
/// `tilt_up`), not after the buttons. Misclassifying one no longer suppresses
/// gameplay — live strike volumes are authoritative — but it still publishes the
/// wrong animation/HUD/telegraph state and can change movement policy that reads
/// "mid-attack" from the projection. Both attack and smash verb families are
/// therefore classified here by their bindings, not by move-id spelling.
///
/// The id check remains as the fallback for a move that the owner's moveset does
/// not bind (a boss projecting moves it never registered a verb for), so this is
/// a strict SUPERSET of the old rule: nothing that swung before stops swinging.
fn is_melee_swing_move(moveset: Option<&MovesetContract>, id: &str) -> bool {
    if let Some(verb) = moveset.and_then(|m| verb_for_move(m, id)) {
        return is_melee_verb(verb);
    }
    id == ATTACK_VERB || id.starts_with("attack_") || id == SMASH_VERB || id.starts_with("smash_")
}

/// Map a moveset `"attack"` move id back to the swing direction it was derived
/// for. The directional variants (`prefabs::directional_attack_variants`) name
/// their moves after the intent, so the read-model swing (and the sprite row it
/// drives) can recover the direction the flat path used to carry on `AttackSpec`.
/// The base `"attack"` and any unknown id read as the forward swing.
fn attack_intent_from_move_id(id: &str) -> AttackIntent {
    match id {
        "attack_up" => AttackIntent::Up,
        "attack_down" => AttackIntent::Down,
        "attack_air" => AttackIntent::AirForward,
        "attack_air_up" => AttackIntent::AirUp,
        "attack_air_back" => AttackIntent::AirBack,
        "attack_air_down" => AttackIntent::AirDown,
        _ => AttackIntent::Forward,
    }
}

/// Build the read-model `MeleeSwing` for a live move: startup = first Active
/// window start, active = span from first Active start to last Active end
/// (covers multi-hit combos), recovery = remainder. Only the timing is
/// meaningful — every geometry/impulse field is inert (the real strike is the
/// moveset's own hitbox), so the derived phase answers is_active/is_winding_up
/// exactly as the flat swing did.
fn synth_swing_from_move(pb: &MovePlayback) -> MeleeSwing {
    let spec = &pb.spec;
    let actives: Vec<&MoveWindow> = spec
        .windows
        .iter()
        .filter(|w| matches!(w.tag, WindowTag::Active))
        .collect();
    let (startup, active) = match (actives.first(), actives.last()) {
        (Some(first), Some(last)) => (first.start_s, (last.end_s - first.start_s).max(0.0)),
        // A move with no Active window (a pure sustain/telegraph) reads as all
        // windup until it ends — still "swinging" for the anim tint.
        _ => (spec.duration_s, 0.0),
    };
    let recovery = (spec.duration_s - startup - active).max(0.0);
    let attack_spec = AttackSpec {
        // The move's directional variant id (from `directional_attack_variants`)
        // carries the swing direction — recover it so the read-model swing drives
        // the correct directional sprite row (up-tilt reads `AttackUp`, a down-air
        // `AirDown`, …) and any preview gizmo points the right way. The base
        // `"attack"` move stays `Forward` (byte-parity with the pre-directional
        // hardcode).
        intent: attack_intent_from_move_id(spec.id.as_str()),
        startup_seconds: startup,
        active_seconds: active,
        recovery_seconds: recovery,
        hitbox_offset: ae::Vec2::ZERO,
        hitbox_half_size: ae::Vec2::ZERO,
        self_impulse: ae::Vec2::ZERO,
        knockback: ae::Vec2::ZERO,
        damage_kind: DamageKind::Slash,
        can_pogo: false,
        damage_override: None,
    };
    let mut swing = MeleeSwing::new(attack_spec);
    swing.elapsed = pb.t;
    // Expose the move's PERSISTENT one-hit-per-target dedup on the read-model
    // swing, so `apply_hitbox_damage` emits it as the strike's `ignored_targets`.
    // The resolver folds newly-landed keys back into `MovePlayback.hit_targets`
    // (not this swing, which is rebuilt next frame) — that persistence is what
    // stops the multi-hit / hit-SFX spam.
    swing.hit_targets = pb.hit_targets.clone();
    swing
}

#[cfg(test)]
mod tests;
