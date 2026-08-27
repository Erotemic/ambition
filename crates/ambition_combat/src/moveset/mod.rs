//! Data-driven move playback — the runtime half of the Smash model.
//!
//! An actor plays a [`MoveSpec`](ambition_entity_catalog::MoveSpec) by
//! carrying a [`MovePlayback`] component; [`advance_move_playback`] is the
//! ONE system that turns the authored timeline into simulation:
//!
//! - Proper time. The playback clock advances by
//!   `WorldTime::entity_dt(ProperTimeScale)` (ADR 0011) — the owning actor's
//!   own clock. A dilated actor's windows, volumes, events, and picture all
//!   slow together because they are one timeline (`MovePlayback::phase` is
//!   what presentation samples the bound clip by).
//! - Windows → hitbox entities. Each `Active` window's volumes become
//!   `(Hitbox, HitboxHits)` entities (`FollowOwner`, facing-mirrored,
//!   entity-local offsets) on window entry and despawn on window exit —
//!   window-scoped by the move's own clock, so no wall-time lifetime can
//!   drift from a dilated owner. Damage resolution is the existing
//!   [`apply_hitbox_damage`](super::hitbox::apply_hitbox_damage) path:
//!   moves need NO parallel hit plumbing.
//! - Events → messages. Timed events emit [`MoveEventMessage`]s;
//!   consumers (audio bridge, techniques/effects) subscribe downstream.
//!
//! Re-binding a move onto a different actor is inserting the same
//! `MovePlayback` on a different entity — zero per-actor Rust. That is the
//! decomposability contract, pinned by the tests below.

use bevy::prelude::{
    Commands, Component, Entity, Message, MessageReader, MessageWriter, Query, Res, With,
};

use ambition_entity_catalog::{
    AttackDir, ImpulseMode, MoveEventKind, MoveSpec, MoveWindow, MovesetContract, WindowTag,
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
use ambition_characters::brain::action_set::{ActionRequest, RangedCommitment, SpecialActionSpec};
use ambition_characters::brain::ActorActionMessage;
use ambition_characters::control::ActorControl;
use ambition_entity_catalog::placements::DamageKind;
use ambition_sfx::{PresentationSourceId, SfxId, SfxMessage, SfxWriter};
use ambition_time::WorldTime;

// The four moveset verb ids now live beside the contract they key into
// (`ambition_entity_catalog`), because a verb name is authoring vocabulary
// rather than runtime behaviour — and because a character DEFINITION must be
// able to name the verb its moveset binds without reaching up into this crate.
// Re-exported so every `moveset::ATTACK_VERB`-style path is unchanged.
pub use ambition_entity_catalog::{ATTACK_VERB, RANGED_VERB, SMASH_VERB, SPECIAL_VERB, TAUNT_VERB};
// The capture verbs, on the same road for the same reason.
pub use ambition_entity_catalog::{
    CAPTURE_PUMMEL_VERB, CAPTURE_THROW_BACK_VERB, CAPTURE_THROW_DOWN_VERB,
    CAPTURE_THROW_FORWARD_VERB, CAPTURE_THROW_UP_VERB, GRAB_VERB,
};

// These three ids are what the builders themselves author into every moveset, and `prefabs.rs`
// had to become lowerable to `ambition_characters` so character preparation could call
// `build_actor_moveset` from below. Plain `&str`s travel; what could NOT travel is the
// compile-time assertions under them, which need `ambition_sfx`'s id table — so the low crate
// owns the text and this one keeps the pin.
pub use ambition_characters::moveset_prefabs::{SLASH_ARC_VFX, SLASH_POKE_VFX, SWING_SFX_CUE};

// AND THE THREE `PLAYER_ROBOT_*` CUES CAME BACK UP. They went down with the builders because
// they were adjacent in the file, not because preparation needed them: the overlay that reads them
// has exactly one production caller, the protagonist road, and `prepare_character` never reaches
// it. Text in the low crate and its compile-time proof in this one was the shape that move created;
// both are here now. See `player_robot_slash`'s own doc.
mod player_robot_slash;
pub use player_robot_slash::{
    apply_player_robot_slash_sfx, PLAYER_ROBOT_IMPACT_SFX_CUE, PLAYER_ROBOT_POGO_SFX_CUE,
    PLAYER_ROBOT_SWING_SFX_CUE,
};

const _: () = assert!(
    ambition_sfx::SfxId::from_static(PLAYER_ROBOT_SWING_SFX_CUE).hash()
        == ambition_sfx::ids::PLAYER_ROBOT_SLASH_AIR.hash()
);
const _: () = assert!(
    ambition_sfx::SfxId::from_static(PLAYER_ROBOT_IMPACT_SFX_CUE).hash()
        == ambition_sfx::ids::PLAYER_ROBOT_SLASH_IMPACT.hash()
);
const _: () = assert!(
    ambition_sfx::SfxId::from_static(PLAYER_ROBOT_POGO_SFX_CUE).hash()
        == ambition_sfx::ids::PLAYER_ROBOT_SLASH_IMPACT_POGO.hash()
);

// D-B split: the MoveSpec builders and actor-moveset construction live in
// `prefabs.rs`. Re-exported so `moveset::<builder>` paths (and `tests.rs`'s
// `use super::*`) are unchanged by the relocation.
// THE REGISTRY IS ITS OWN MODULE (P1.7): `prefabs.rs` is the build-time
// half of the Smash model and character preparation calls it, so it has to be
// able to sit at or below `ambition_characters`. Expanding an authored prefab
// KEY is a different job from building a move from a spec, and it is the one
// that validates presentation ids through `ambition_vfx` — a crate the
// character domain must not reach. See `prefab_registry`'s own doc.
mod prefab_registry;

pub use ambition_characters::moveset_prefabs::*;
pub use prefab_registry::*;

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

/// Keep the routing markers agreeing with the moveset they route into.
///
/// `MovesetMelee` and [`MovesetRanged`](ambition_characters::brain::MovesetRanged)
/// are not independent state — they are a projection of "does this moveset author
/// an `attack` / `ranged` verb". They were nonetheless written by hand at three
/// unrelated places (the actor cluster seed, the prepared-character projection)
/// and by NOBODY on the catalog persona path, which replaces `ActorMoveset`
/// wholesale on a kit swap and never touched them. The consequences are all
/// silent:
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
    /// WHERE this event happens, relative to the owner, in WORLD units.
    ///
    /// authored body-local and resolved HERE, because this is where the
    /// facing the move committed to and the owner's gravity frame are both in
    /// hand — the consumer has neither. Zero for every event kind that has no
    /// place of its own, which is all of them but `Vfx`.
    pub world_offset: ae::Vec2,
    /// HOW this event is oriented — the sibling of [`Self::world_offset`],
    /// resolved from the SAME two authorities in the same expression.
    pub world_pose: ambition_vfx::FxPose,
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
    /// Was this body grounded when this move last looked? Owned by the
    /// playback because the LANDING EDGE is a fact about this move's history,
    /// not about the body.
    ///
    /// seeded `true`, which reads backwards and is the point. It means
    /// *no airborne observation yet*, so a move begun ON THE GROUND can never
    /// cross the edge on its first tick and be charged an aerial's landing lag
    /// — which is exactly what a `false` seed did, and what
    /// `a_grounded_move_never_pays_landing_lag` caught. The construction site
    /// cannot supply the real answer (`MovePlayback::new` sees no body), so the
    /// safe direction is to assume grounded until a tick observes otherwise.
    ///
    /// the price, and it is nil in practice: a move that starts airborne and
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
    pub hit_targets: Vec<String>,
    /// The DIRECTION the gesture that started this move asked for.
    ///
    /// ⭐⭐ CAPTURED WHERE IT IS KNOWN, which is the whole point. The read-model
    /// swing used to RECONSTRUCT this by matching the move's id against a
    /// seven-entry canonical vocabulary (`attack_up`, `attack_air_back`, …), and
    /// no shipped fighter spells its moves that way — Pointed authors
    /// `polygon_tilt_up`, Pugnacious `polygon_brawler_air_back` — so every one
    /// of them synthesised `Forward`. Animation, the HUD and the gizmos read
    /// this, so all of them were told the same wrong thing.
    ///
    /// ⛔ NOT AN `Option`. A move no directional gesture started — a chain
    /// successor, a held weapon's action, a special — reports `Forward`, which
    /// is exactly what the flat swing published for one. Saying so here is
    /// what stops a fallback appearing in every consumer.
    pub attack_intent: AttackIntent,
    /// Was this move SELECTED as a ground move?
    ///
    /// ⭐⭐ THE STANCE IS A FACT ABOUT THE MOVE, NOT ABOUT EACH RECTANGLE. It is
    /// the same `grounded` that chose which variant this press resolves to
    /// (`move_for_verb_in_stance`), so a consumer asking "was this an aerial"
    /// gets the answer the SELECTOR used rather than a re-observation.
    ///
    /// ⛔⛔ AND IT IS NOT [`Self::was_grounded`], which is a per-TICK observation
    /// for the landing edge and is seeded `true` on purpose. Clank eligibility
    /// asked `BodyGroundState::on_ground` at COLLISION time, so a ground attack
    /// stopped clanking the moment its owner walked off a ledge mid-swing and an
    /// aerial started when its owner landed. "Grounded attack" is a
    /// CLASSIFICATION and it is settled when the swing comes out.
    ///
    /// ⛔ `true` for a move nobody selected in a stance — the same default the
    /// selector takes for a body with no ground state.
    pub started_grounded: bool,
    /// The ranged intent that STARTED this move, if one did.
    ///
    /// a move's fire event usually arrives after its own request is gone.
    /// `ActorControl.fire` is an EDGE — `clear_edges()` nulls it every tick — and
    /// a ranged move has startup, so by the time its authored `Ranged` frame
    /// fires, the intent that triggered it has been cleared. The handler fell
    /// back to the body's horizontal facing, which is right for a forward shot
    /// and flattens every aim that was UP, DOWN or DIAGONAL: a move triggered
    /// with an upward aim fired sideways.
    ///
    /// Captured here at move start and consumed at the fire frame, so the shot
    /// carries the aim the player actually gave it. `None` for a move nobody
    /// aimed, which is what keeps the facing fallback meaningful.
    ///
    /// the POLICY travels with the direction. A body-local `(1,0)` and a world
    /// `(1,0)` are different shots under non-default gravity, so storing the
    /// vector alone would re-introduce the frame confusion `dir_to_world` exists
    /// to prevent.
    pub aim: Option<(ae::Vec2, ae::GameplayFramePolicy)>,
    /// The CHARGE this use is playing, or `None` for a use that never entered
    /// charge mode.
    ///
    /// Per-USE and not per-move: only a press that resolved to the gesture the
    /// move charges on ([`ambition_entity_catalog::MoveSpec::charge_gesture`] —
    /// Smash for every smash attack, Special for a held neutral-B) charges, so
    /// the same `MoveSpec` reached through another verb plays its plain
    /// timeline. Rollback state, carried with the rest of this component.
    pub charge: Option<MoveCharge>,
    /// Seconds of this use's proper time spent going ROUND the authored loop
    /// ([`MoveSpec::repeat`]) — `0.0` for a move that plays once.
    ///
    /// Counted rather than derived from the clock, because the clock rewinds:
    /// `t` is back where it was and only this says how long the flurry has been
    /// running. It is what the loop's own maximum is measured against.
    pub looped_s: f32,
}

/// One chargeable use's charge clock.
///
/// The move's own timeline freezes at the authored hold point while Attack is
/// held; `held_s` accumulates in the owner's proper time; and the fraction is
/// FROZEN at release so every hit the use generates lands with the same
/// payoff — a multi-hit smash cannot pay more for its later pulses.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MoveCharge {
    /// The policy this use resolved at its start. Held rather than re-read so a
    /// content reload mid-move cannot move the hold point under a frozen clock.
    pub policy: ambition_entity_catalog::SmashChargeSpec,
    /// Seconds of the owner's proper time spent holding.
    pub held_s: f32,
    /// The fraction the release froze, `0..=1`. `None` = still charging (or not
    /// yet at the hold point).
    pub released_fraction: Option<f32>,
}

impl MoveCharge {
    fn new(policy: ambition_entity_catalog::SmashChargeSpec) -> Self {
        Self {
            policy,
            held_s: 0.0,
            released_fraction: None,
        }
    }

    /// The fraction in force: the frozen one once released, the live one while
    /// still holding.
    pub fn fraction(&self) -> f32 {
        self.released_fraction
            .unwrap_or_else(|| self.policy.fraction_for(self.held_s))
    }

    /// Still holding — the timeline is frozen and the charge is growing.
    pub fn charging(&self) -> bool {
        self.released_fraction.is_none()
    }

    /// Freeze the payoff. Idempotent: a second release cannot raise it.
    fn release(&mut self) {
        if self.released_fraction.is_none() {
            self.released_fraction = Some(self.policy.fraction_for(self.held_s));
        }
    }
}

impl bevy::ecs::entity::MapEntities for MovePlayback {
    fn map_entities<M: bevy::ecs::entity::EntityMapper>(&mut self, entity_mapper: &mut M) {
        for (_, entity) in &mut self.live_boxes {
            *entity = entity_mapper.get_mapped(*entity);
        }
    }
}

/// A body-local authored volume, placed into the world around its owner.
///
/// `world_offset` is relative to the body's position; `half_extent` is the
/// axis-aligned bound after the frame rotation. `shape` is `Some` only when the
/// volume is not a plain rectangle.
#[derive(Debug, Clone, PartialEq)]
pub struct PlacedBodyVolume {
    pub world_offset: ae::Vec2,
    pub half_extent: ae::Vec2,
    pub shape: Option<ae::VolumeShape>,
}

/// Place a body-local authored volume: mirror by facing, then rotate into the
/// body's gravity frame.
///
/// The `+x = committed facing, +y = gravity-down` contract every authored
/// `HitVolume` states, applied once. Strike volumes and CAPTURE volumes both go
/// through it.
pub fn place_body_local_volume(
    shape: &ambition_entity_catalog::VolumeShape,
    facing: f32,
    body_frame: &ae::AccelerationFrame,
) -> PlacedBodyVolume {
    let (local, half_extent, shape) = match shape {
        ambition_entity_catalog::VolumeShape::Rect {
            offset,
            half_extents,
        } => (
            ae::Vec2::new(offset.0 * facing, offset.1),
            ae::Vec2::new(half_extents.0, half_extents.1),
            None,
        ),
        ambition_entity_catalog::VolumeShape::Circle { offset, radius } => (
            ae::Vec2::new(offset.0 * facing, offset.1),
            ae::Vec2::splat(*radius),
            Some(ae::VolumeShape::circle(*radius)),
        ),
    };
    PlacedBodyVolume {
        world_offset: body_frame.to_world(local),
        // Axis-aligned extents rotate with the frame too (a circle's splat is
        // rotation-invariant, so this is uniform).
        half_extent: body_frame.to_world_half(half_extent),
        shape,
    }
}

/// Despawn a playback's live strike volumes. The half of teardown shared by
/// ending a move and by REPLACING one — a move-into-move cancel drops the old
/// volumes and then overwrites the playback, so it wants this half alone.
pub fn despawn_live_boxes(commands: &mut bevy::prelude::Commands, playback: &mut MovePlayback) {
    for (_, entity) in playback.live_boxes.drain(..) {
        commands.entity(entity).despawn();
    }
}

/// End a move: its volumes stop existing and the body stops playing it.
///
/// there were FOUR hand-copies of this, and one of them carried the
/// comment *"Tear down exactly as natural completion does (the ONE teardown
/// path)"* — a claim the code made true by duplication, which is the same thing
/// as not being true. Capture needs a fifth caller (a body that gets grabbed
/// mid-swing must not keep a live hitbox), and a fifth copy is where a divergence
/// becomes likely rather than possible.
///
/// this is a consolidation, not an action-state framework. It does what
/// the four copies did and nothing else; a body's move ending has no other
/// meaning today, and inventing one here would be building the abstraction
/// nobody has asked for.
pub fn cancel_move_playback(
    commands: &mut bevy::prelude::Commands,
    owner: bevy::prelude::Entity,
    playback: &mut MovePlayback,
) {
    despawn_live_boxes(commands, playback);
    commands.entity(owner).remove::<MovePlayback>();
}

impl MovePlayback {
    pub fn new(spec: MoveSpec, facing: f32) -> Self {
        Self::new_at(spec, facing, 0.0)
    }

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
        // STRICTLY before `t0`, not `<=`. An event authored AT the start is the common case, not an
        // edge one: the player's swipe is `windup_s: 0.0` precisely so "the arc and the swing cue
        // all land on the frame of the press", which puts its SFX event at `at_s == 0.0`.
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
            // What the flat swing published for a move no direction asked for.
            // `with_attack_intent` is the seam that says otherwise.
            attack_intent: AttackIntent::Forward,
            // `started_in_stance` is the seam that says otherwise.
            started_grounded: true,
            // Nobody aimed unless a caller says so — `with_aim` is the seam, and
            // the facing fallback at the fire frame is what an unaimed move gets.
            aim: None,
            // A use charges only when the press that started it asked to —
            // `charged_by_gesture` is that seam.
            charge: None,
            looped_s: 0.0,
        }
    }

    /// Remember the ranged intent that started this move. See [`Self::aim`].
    pub fn with_aim(mut self, aim: Option<(ae::Vec2, ae::GameplayFramePolicy)>) -> Self {
        self.aim = aim;
        self
    }

    /// Remember the STANCE this move was selected in. See
    /// [`Self::started_grounded`].
    pub fn started_in_stance(mut self, grounded: bool) -> Self {
        self.started_grounded = grounded;
        self
    }

    /// Remember the DIRECTION the gesture asked for. See
    /// [`Self::attack_intent`].
    pub fn with_attack_intent(mut self, intent: AttackIntent) -> Self {
        self.attack_intent = intent;
        self
    }

    /// Enter charge mode iff the gesture that started this use is the one the
    /// move charges on AND the move authors (or derives) a charge policy.
    ///
    /// Both halves are required, and neither is redundant: the gesture is what
    /// makes THIS use the chargeable one, and the policy is what makes the MOVE
    /// chargeable. A move borrowed by another verb never freezes its timeline,
    /// and a slot with no payoff never freezes it either.
    ///
    /// ⭐ THE GESTURE IS A MATCH, NOT A BOOLEAN. This took `is_smash` while the
    /// mechanic had exactly one binding, and the genre has two: a smash freezes
    /// on the Attack hold and a chargeable neutral special on the Special hold.
    /// `started_by` is what the press RESOLVED to (`None` for every verb that
    /// charges nothing), and [`MoveSpec::charge_gesture`] is what the move
    /// ASKED for; charge mode is where they agree.
    #[must_use]
    pub fn charged_by_gesture(
        mut self,
        started_by: Option<ambition_entity_catalog::ChargeGesture>,
    ) -> Self {
        self.charge = started_by
            .filter(|gesture| *gesture == self.spec.charge_gesture)
            .and_then(|_| self.spec.charge_policy())
            .map(MoveCharge::new);
        self
    }

    /// The resolved charge fraction to PRESENT, `None` when this use is not
    /// currently charging.
    ///
    /// The one fact presentation reads for the charge pose / pulse / cue: it
    /// appears when the hold latches, rises to `1.0` at maximum, and goes back
    /// to `None` the instant the move releases. ⛔ presentation must not
    /// re-derive this from move names or Startup progress — a tapped smash and
    /// a held one share both.
    /// Is this body ROOTED by a charge right now — the timeline frozen at the
    /// hold point with the button still down?
    ///
    /// ⛔ A CHARGING FIGHTER DOES NOT WALK. Jon, 2026-08-23: *"when the
    /// character is charging their smash attack, they should not be able to
    /// walk or move."* That is the genre's rule and it is a fact about
    /// CHARGING, not about any one move's authoring — a smash's Startup window
    /// carries the default `motion_scale: 1.0` like every other window, so
    /// before this a charging body kept full steering while its clock stood
    /// still, and could walk the whole stage in its windup pose.
    ///
    /// Distinct from `charge.charging()`, which is true from the move's first
    /// tick: a body on its way TO the hold point is still swinging, and only the
    /// freeze roots it.
    pub fn rooted_by_charge(&self) -> bool {
        self.charge.is_some_and(|charge| {
            charge.charging() && self.t >= charge.policy.hold_at_s.min(self.spec.duration_s)
        })
    }

    /// The steering authority this body has RIGHT NOW: zero if the move's stance
    /// roots it or a charge holds it, otherwise the live window's authored
    /// motion lock.
    ///
    /// One place, so the two integration call sites cannot disagree about
    /// whether a moving body may steer.
    pub fn motion_scale_now(&self) -> f32 {
        // ⭐ THE STANCE RULE OUTRANKS THE WINDOW. `roots_steering` is a fact
        // about the posture the move answers from — in this genre you cannot
        // walk out of a grounded attack — and a window's `motion_scale` is a
        // per-move refinement WITHIN whatever the stance allows. A move that
        // roots therefore roots for its whole duration, including the recovery
        // windows an author left at the default 1.0.
        if self.spec.gates.roots_steering || self.rooted_by_charge() {
            return 0.0;
        }
        self.spec.motion_scale_at(self.t)
    }

    /// Is this move's sprite drawn MIRRORED right now?
    ///
    /// ⭐ A PURE FUNCTION OF THE MOVE CLOCK, which is what makes it rollback-safe
    /// without being rollback state: `t` already rewinds, so a resimulated frame
    /// draws the same way the abandoned one did. A latch that toggled every
    /// `1/hz` seconds would be state nobody registered.
    ///
    /// ⛔ PRESENTATION ONLY. Nothing about the body's own facing changes — see
    /// `MoveSpec::sprite_spin_hz`.
    pub fn sprite_mirrored_now(&self) -> bool {
        let Some(hz) = self.spec.sprite_spin_hz.filter(|hz| *hz > 0.0) else {
            return false;
        };
        // Half-periods: one full mirror cycle per hertz means the sprite spends
        // half of each period flipped.
        ((self.t * hz * 2.0).floor() as i64).rem_euclid(2) == 1
    }

    /// The charge fraction this use PAYS OUT: live while still holding, frozen
    /// once released. `None` for a use that never entered charge mode.
    ///
    /// ⛔ NOT [`Self::smash_charge_fraction`], which is the PRESENTATION
    /// question and goes back to `None` the instant the move releases. A
    /// payoff is read at the moment of release or later — a charged shot's
    /// fire event lands after it — so the two cannot be the same accessor.
    pub fn charge_fraction(&self) -> Option<f32> {
        self.charge.map(|charge| charge.fraction())
    }

    pub fn smash_charge_fraction(&self) -> Option<f32> {
        self.charge
            .filter(MoveCharge::charging)
            .map(|c| c.policy.fraction_for(c.held_s))
    }

    /// This move is holding its owner UNTOUCHABLE right now — an authored
    /// [`WindowTag::Invuln`] window covering the clock.
    ///
    /// Projected onto `Invulnerability::MOVE` rather than read at each damage
    /// site: the body's vulnerability has one authority
    /// (`ambition_combat::util::body_vulnerable`) and a move grant that any
    /// other rule had to learn about separately would be a second one.
    pub fn intangible_now(&self) -> bool {
        self.spec
            .tagged_window_covers(self.t, |tag| matches!(tag, WindowTag::Invuln))
    }

    /// This move is ARMORING its owner right now — an authored
    /// [`WindowTag::Armor`] window covering the clock.
    ///
    /// Armor is not invulnerability and deliberately does not travel with it: an
    /// armoured body is HIT, takes the damage, and simply does not answer for it
    /// — no launch and no hitstun. Two different questions, so two different
    /// facts.
    pub fn armored_now(&self) -> bool {
        self.spec
            .tagged_window_covers(self.t, |tag| matches!(tag, WindowTag::Armor))
    }

    /// The damage/knockback scale every hit of this use lands with.
    ///
    /// ⛔⛔ ONE AUTHORITY, AND THE OTHER ONE PAID OUT UNCONDITIONALLY. A use
    /// that never entered charge mode scales by `1.0`, full stop. It used to
    /// fall through to a TIMELINE reading — `smash_charge_mult` interpolated by
    /// how far the clock had run through the leading Startup window — which
    /// sounds like a partial payoff and is not one: a strike volume only ever
    /// spawns INSIDE an Active window, Active begins where that Startup window
    /// ends, and the fraction is clamped, so every non-charging use of a move
    /// with a multiplier landed at the FULL multiplier on every hit. George's
    /// `bivalence` is `Feel::Special` and never charges, so its authored 7/13
    /// damage was really 7×1.6 and 13×1.6.
    ///
    /// ⇒ `MoveCharge` is now the only thing that pays. A mechanic that wants
    /// power derived from a timeline may have one, under its own name; it may
    /// not have this one by inheritance.
    pub fn charge_scale(&self) -> f32 {
        match self.charge {
            Some(charge) => 1.0 + charge.fraction() * (self.spec.smash_charge_mult - 1.0),
            None => 1.0,
        }
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

#[derive(Component, Debug, Clone)]
#[require(crate::stale::BodyStaleMoves)]
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

/// Where this volume sits in its move's AUTHORED order — the sweetspot rule.
///
/// A move that wants a good and a bad way to land it authors two volumes: the
/// tip that kills and the base that does not. Both are live at once and both
/// reach a body standing between them, and the runtime spawns one hitbox per
/// volume with its own dedup — so a single swing landed BOTH. Measured before
/// this existed: one press, one Active window, two overlapping volumes, and a
/// victim that took 15 and then 4, with two knockbacks. The vocabulary the
/// parity inventory asks for was not merely unimplemented; authoring it
/// produced a double hit.
///
/// ⭐ THE PRIORITY IS THE AUTHORING ORDER, and that is the genre's rule rather
/// than a preference: Smash resolves overlapping hitboxes of one attack by
/// their id, lowest first, so an author orders the tip before the base and the
/// tip wins. Nothing new had to be invented to say it — a move already lists
/// its volumes in an order, and this is the runtime finally honouring it.
///
/// ⛔ NOT a per-move exception and not a `priority` field: adding one would let
/// two volumes claim the same rank and make the answer depend on query order,
/// which is the determinism trap this repository keeps rediscovering.
#[derive(bevy::prelude::Component, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct StrikeRank {
    /// The window this volume was authored in, then its index within that
    /// window — the move's own reading order, flattened so one comparison
    /// answers "which of these two did the author write first".
    pub window: u16,
    pub volume: u16,
}

impl bevy::ecs::entity::MapEntities for StrikeVolume {
    fn map_entities<M: bevy::ecs::entity::EntityMapper>(&mut self, mapper: &mut M) {
        self.owner = mapper.get_mapped(self.owner);
    }
}

/// Despawn every strike volume whose owner's clock says it should not exist.
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
        // The owner's DRIVER, so a POSSESSED body's strike carries its EFFECTIVE
        // faction (a driven body fights as `Player`): `effective_faction`'s
        // contract is that every hitbox stamp resolves through it, and this move
        // strike is one of them. `None` (nobody drives it)  the authored faction
        // (identity for every ordinary actor).
        Option<&ambition_characters::control::DrivingParticipant>,
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
        // MUTABLE since the timeline learned to move its own owner: a
        // `MoveEventKind::Impulse` crossing is authored SELF-MOTION and this is
        // the system that owns the move clock, so it is the one place that can
        // apply it at the authored instant. `trigger_moveset_moves` does the
        // same write for `start_impulse` at the press; this is that seam, later
        // on the same timeline.
        &mut ae::BodyKinematics,
        Option<&ProperTimeScale>,
        // I4: the owner's rollback-stable identity, so the transient strike volume
        // it opens can derive one. Without it every anonymous hitbox folded to the
        // same constant in the entity-reference probes, and swapped owners were
        // invisible. `None` (a bare test body) simply mints nothing.
        Option<&ambition_platformer2d_shared_tangle::sim_id::SimId>,
    )>,
    // IS ATTACK STILL DOWN? The body-generic resolved gesture, produced by
    // `resolve_attack_gestures` earlier in this same tick — never a device and
    // never a participant. `held` is `Some` for exactly as long as the press
    // that started the move is sustained, so a tap resolves as released the
    // moment the hold point is reached.
    held_attack: Query<&ResolvedAttackGesture>,
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
        driver,
        config,
        worn,
        body_source,
        mut kin,
        scale,
        owner_sim_id,
    ) in &mut players
    {
        let strike_faction = crate::targeting::effective_faction(*faction, driver);
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
        // THE CHARGE HOLD. A chargeable use walks its ordinary timeline to the
        // authored hold point and stands there while Attack is held. The proper
        // time it would have spent advancing is spent CHARGING instead — no
        // more, no less — so hitlag and a global pause slow the charge exactly
        // as they slow the swing, and no tick's worth of time is lost or
        // double-spent at the boundary.
        //
        // "Still held" is the body-generic resolved gesture, produced earlier in
        // this same tick, never a device and never a participant: a CPU that
        // taps Smash reaches the hold point already released and continues with
        // the minimum payoff, and a policy that holds charges like a person.
        let dt = match playback.charge {
            Some(charge) if charge.charging() => {
                let duration = playback.spec.duration_s;
                let hold_at = charge.policy.hold_at_s.clamp(0.0, duration);
                if (t_prev + dt).min(duration) < hold_at {
                    dt
                } else {
                    let to_hold = (hold_at - t_prev).max(0.0);
                    let spare = (dt - to_hold).max(0.0);
                    // WHICH BUTTON, asked of the move. A smash holds on
                    // Attack; a chargeable neutral special holds on Special.
                    // Reading the attack hold for both is what made the second
                    // one impossible: the finger on Special was on the wrong
                    // field, so the charge released on the tick it latched.
                    let still_held =
                        held_attack
                            .get(owner)
                            .is_ok_and(|g| match playback.spec.charge_gesture {
                                ambition_entity_catalog::ChargeGesture::Smash => g.held.is_some(),
                                ambition_entity_catalog::ChargeGesture::Special => g.special_held,
                            });
                    let charge = playback.charge.as_mut().expect("matched above");
                    if still_held {
                        let accrued = spare.min(charge.policy.max_hold_s - charge.held_s);
                        charge.held_s += accrued;
                        if charge.held_s >= charge.policy.max_hold_s {
                            // Maximum fires the move whether or not the button
                            // is down: a full charge is LOADED, not stored.
                            charge.release();
                        }
                        // Whatever the hold could not absorb — only non-zero on
                        // the tick the maximum is reached — carries the move
                        // forward, so the release is not a frame late.
                        to_hold
                            + if charge.charging() {
                                0.0
                            } else {
                                spare - accrued
                            }
                    } else {
                        charge.release();
                        dt
                    }
                }
            }
            _ => dt,
        };
        playback.t = (t_prev + dt).min(playback.spec.duration_s);

        // THE FLURRY. An authored loop sends the clock back to its own start
        // for as long as the button is down, and the move's remaining timeline
        // is the finisher it exits into — so a rapid jab that ends in a
        // launcher is ONE move rather than a chain of bespoke ones.
        //
        // ⛔ the loop lives HERE, in playback, and knows no fighter: a move
        // says which of its own windows repeat and the runtime does the same
        // thing to every move that says so.
        //
        // Two exits, and both are the button's: RELEASE, and the authored
        // maximum, which is what stops a held press being a stall. A body that
        // has stopped holding leaves on the very next crossing.
        let looped = match playback.spec.repeat {
            Some(spec) if spec.is_live() && playback.t >= spec.to_s => {
                let still_held = held_attack.get(owner).is_ok_and(|g| g.held.is_some());
                let spent = playback.looped_s + (spec.to_s - spec.from_s);
                if still_held && spent <= spec.max_s {
                    playback.looped_s = spent;
                    // The overshoot rides round with the clock, so no proper
                    // time is lost at the wrap.
                    playback.t = spec.from_s + (playback.t - spec.to_s);
                    true
                } else {
                    false
                }
            }
            _ => false,
        };
        let t = playback.t;
        if looped {
            // The pulse is a NEW strike: drop the volumes the previous lap
            // opened so the window re-spawns and can hit again, and re-arm the
            // events inside the loop so each lap sounds like one.
            despawn_live_boxes(&mut commands, &mut playback);
            let pb = &mut *playback;
            let repeat = pb.spec.repeat.expect("matched above");
            for (idx, ev) in pb.spec.events.iter().enumerate() {
                if ev.at_s >= repeat.from_s && ev.at_s < repeat.to_s {
                    pb.fired[idx] = false;
                }
            }
        }

        // Timed events crossing (t_prev, t] fire exactly once, in order.
        // Split-borrow locals keep the fired flags and the spec readable
        // side by side.
        let pb = &mut *playback;
        for (idx, ev) in pb.spec.events.iter().enumerate() {
            // no lower bound: `fired[idx]` already guarantees once-only, and a
            // `ev.at_s > t_prev` bound is unsatisfiable for an event at 0.0 on the
            // first advance, where `t_prev` is also 0.0. It added nothing except
            // that hole.
            if !pb.fired[idx] && ev.at_s <= t {
                pb.fired[idx] = true;
                // AUTHORED SELF-MOTION IS SIMULATION, so it lands HERE and
                // is not announced as a message.
                //
                // Every other `MoveEventKind` names something for a CONSUMER to
                // resolve — a cue, a cosmetic burst, a content technique, a
                // shot — and `dispatch_move_events` is where those are resolved.
                // An impulse names no consumer: it is a velocity write on the
                // owner, exactly like `start_impulse` at the press, and this is
                // the system that holds both the move clock and the body. One
                // writer, one site — publishing it as a message and applying it
                // somewhere else is the follow-up-call shape this tree keeps
                // paying for.
                if let MoveEventKind::Impulse { local, mode } = &ev.kind {
                    let body_frame = owner_frames
                        .get(owner)
                        .map(|frame| frame.basis())
                        .unwrap_or(ae::AccelerationFrame::new(ae::DEFAULT_GRAVITY_DIR));
                    // Body-local `(+x = facing, +y = gravity-down)` mirrored by
                    // the facing the MOVE started with, then rotated into the
                    // owner's frame — the same two steps `start_impulse` takes,
                    // so a rise stays a rise under any gravity.
                    //
                    // `pb.facing`, not the body's live facing: a move whose
                    // burst pointed wherever the body happened to be looking
                    // three windows later would not be the move that was
                    // committed to.
                    let world = body_frame.to_world(ae::Vec2::new(local.0 * pb.facing, local.1));
                    kin.vel = match mode {
                        ImpulseMode::Add => kin.vel + world,
                        ImpulseMode::Set => world,
                    };
                    continue;
                }
                // A `Vfx` says where it happens in the same body-local terms a
                // hit volume does; mirror by the move's committed facing and
                // rotate into the owner's frame — the two steps `Impulse` takes
                // directly above, so a burst authored at the end of a swing
                // stays there under any gravity and either facing.
                let (world_offset, world_pose) = match &ev.kind {
                    MoveEventKind::Vfx { at, .. } => {
                        let body_frame = owner_frames
                            .get(owner)
                            .map(|frame| frame.basis())
                            .unwrap_or(ae::AccelerationFrame::new(ae::DEFAULT_GRAVITY_DIR));
                        let offset = if *at != (0.0, 0.0) {
                            body_frame.to_world(ae::Vec2::new(at.0 * pb.facing, at.1))
                        } else {
                            ae::Vec2::ZERO
                        };
                        // an effect at the body's CENTRE still has a facing —
                        // the old `at != (0,0)` guard was about not paying for a
                        // transform that yields zero, never about orientation.
                        (
                            offset,
                            ambition_vfx::FxPose::of(
                                pb.facing,
                                ambition_platformer2d_shared_tangle::gravity::gravity_upright_angle(
                                    body_frame.down,
                                ),
                            ),
                        )
                    }
                    _ => (ae::Vec2::ZERO, ambition_vfx::FxPose::UPRIGHT),
                };
                events.write(MoveEventMessage {
                    owner,
                    move_id: pb.spec.id.clone(),
                    presentation_source: presentation_source.clone(),
                    kind: ev.kind.clone(),
                    world_offset,
                    world_pose,
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
                        world_offset: ae::Vec2::ZERO,
                        // A sustained `Effect` is routed to a keyed TECHNIQUE,
                        // not drawn as art, so it has no pose of its own.
                        world_pose: ambition_vfx::FxPose::UPRIGHT,
                        owner,
                        move_id: pb.spec.id.clone(),
                        presentation_source: presentation_source.clone(),
                        kind: MoveEventKind::Effect(effect.clone()),
                    });
                }
            }
        }

        // HITBOX TRACKS. An attack whose shape moves through its swing is
        // authored as several Active windows laid end to end — the platform-
        // fighter "hitbox track", one strike sampled at keyframes. Each window
        // spawns its own box, so without this the arc would hit the same victim
        // once PER SEGMENT: a four-keyframe sword swing dealing quadruple damage.
        //
        // So a window that ends exactly where the next begins hands its hit set
        // forward. contiguity is the whole rule, and it is not a guess about
        // intent — it is the literal continuity of the volume in time. The box
        // never left, so the strike never ended, so the victim is still struck.
        // A GAP means the box went away and came back, which is precisely what a
        // genuine multi-hit move (a drill, a rapid jab) is, and it rehits.
        //
        // The carry only has to survive within one tick: contiguous windows hand
        // off on the single tick where the clock crosses their shared edge.
        // Nothing about it is rollback state, which is why this costs no wire
        // format. it does assume `windows` is authored in time order, which
        // every spec is and `MoveFrameData` already relies on.
        // ⭐⭐ ONE SET FOR THE WHOLE PULSE, not one per volume index.
        //
        // ⛔⛔ THE PREDECESSOR HANDED OFF BY VOLUME INDEX — "volume `v` hands to
        // volume `v`" — which attached a swing's hit memory to an ORDINAL. A
        // keyframe that changed how many volumes it authors, or their order,
        // silently gave one volume's memory to a different volume; and it could
        // not express the thing it was for, because a victim struck by the
        // SOURSPOT on one tick was recorded only in the sourspot's ledger, so
        // stepping into the SWEETSPOT on the next tick landed a second hit off
        // the same swing.
        //
        // A pulse is ONE continuous stretch of Active time, and it owns ONE
        // per-victim ledger shared by every sibling volume in it. A GAP in
        // Active time starts a new pulse and earns a second hit — which is
        // exactly what a genuine multi-hit (a drill, a rapid jab) is, and what
        // separates one from a sweet/sour pair.
        let mut handoff: Vec<(f32, std::collections::HashSet<Entity>)> = Vec::new();

        // Active windows: spawn volumes on entry, despawn on exit. The box
        // lives exactly while the OWNER'S clock is inside the window, so
        // dilation stretches the box's world-time life automatically.
        for (w_idx, window) in pb.spec.windows.iter().enumerate() {
            if !matches!(window.tag, WindowTag::Active) || window.volumes.is_empty() {
                continue;
            }
            let inside = window.start_s <= t && t < window.end_s;
            // A cached slot naming an entity that no longer exists is treated as absent, so the
            // window re-spawns its volume.
            //
            // This is what makes the "existence is DERIVED from `(t, window)`" contract
            // actually hold under GGRS (ADR 0027): bevy_ggrs restores `MovePlayback` by CLONING
            // it and remapping entities, so after a LoadWorld a slot can name a
            // despawned/unmappable entity for a window the restored clock says is active.
            // Believing the stale slot meant the strike silently whiffed for the rest of the
            // window during resimulation.
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
                    let charge_scale = pb.charge_scale();
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
                            // The window's OWN start, not the move's clock: a
                            // hitbox track lays several Active windows end to
                            // end, and each box wants the shape drawn at the
                            // moment it appears. Inert for a sheet that
                            // publishes one shape per row (every character
                            // sheet today) — it resolves the coarse shape.
                            authored_volumes.resolve(
                                &character_catalog,
                                sprite_cid,
                                clip,
                                kin.size,
                                Some(window.start_s),
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
                            // the plain authored shape goes through the SHARED
                            // placement (see `place_body_local_volume`), which is
                            // the same call a capture attempt makes — so a grab
                            // box and an attack box cannot disagree about gravity.
                            None => {
                                let placed =
                                    place_body_local_volume(&volume.shape, pb.facing, &body_frame);
                                (placed.world_offset, placed.half_extent, placed.shape)
                            }
                        };
                        // The two authored-manifest arms above are still placed
                        // here: a convex hull anchors at the body and a resolved
                        // bbox has already been mirrored, so neither takes the
                        // body-local road.
                        let (local_offset, half_extent) = match &manifest {
                            Some(_) => (
                                body_frame.to_world(local),
                                body_frame.to_world_half(half_extent),
                            ),
                            None => (local, half_extent),
                        };
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
                                growth: volume.knockback_growth,
                            },
                            // CM1: the authored launch direction rides the
                            // volume through to the victim-side resolver.
                            launch_dir: volume.launch_dir.map(|(x, y)| ae::Vec2::new(x, y)),
                            // The authored HOLD rides the same volume the launch
                            // does — an intermediate multi-hit pulse authors it,
                            // the final one does not.
                            reaction: volume.reaction,
                            frame_down,
                        };
                        // §7.2: the slash VFX rides the SAME resolved volume the
                        // damage does (one box drives both) — emitted once at the
                        // Active edge.
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
                        // The PULSE handoff: a window opening exactly where an
                        // Active window closed this tick continues the same
                        // pulse, so every volume it spawns starts from what the
                        // pulse has already hit. ⛔ not per volume index — see
                        // the note on `handoff`.
                        let carried = handoff
                            .iter()
                            .find(|(end_s, _)| *end_s == window.start_s)
                            .map(|(_, hit)| hit.clone())
                            .unwrap_or_default();
                        let mut ec = commands.spawn((
                            hb,
                            HitboxHits { hit: carried },
                            StrikeVolume {
                                owner,
                                window: w_idx,
                            },
                            // The authored order travels with the volume, so
                            // the arbitration at the strike seam needs nothing
                            // but the boxes it already has in hand.
                            StrikeRank {
                                window: w_idx as u16,
                                volume: v_idx as u16,
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
                    // Carry this window's ledger forward before the boxes go —
                    // the UNION over its volumes, because they share one pulse
                    // and a victim any of them reached is a victim the pulse
                    // reached. The despawn is a deferred command, but reading
                    // now is simpler than reasoning about when it lands.
                    handoff.push((
                        window.end_s,
                        pb.live_boxes
                            .iter()
                            .filter(|(idx, _)| *idx == w_idx)
                            .filter_map(|(_, entity)| live_strike_volumes.get(*entity).ok())
                            .flat_map(|hits| hits.hit.iter().copied())
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
            cancel_move_playback(&mut commands, owner, pb);
        }
    }
}

/// THE EDGE CANCEL — recovery ends when the ground it was owed to goes away.
///
/// The other half of [`resolve_aerial_landings`]: that one charges the lag for
/// touching down mid-move, and this one stops charging it once the body is no
/// longer touched down. Sliding off a platform lip during recovery is the
/// genre's reward for landing at the edge on purpose.
///
/// ⛔⛔ IT CANNOT LIVE IN `resolve_aerial_landings`, and that is not a style
/// choice: the lag OUTLIVES the playback. Charging it cancels the move, so a
/// body paying recovery has no `MovePlayback` at all and that query cannot see
/// it. This one asks for the two components every body has.
///
/// ⛔ Gated on a declared rule, so a world that never asked keeps its lag
/// running wherever the body is — see
/// [`crate::rules::DeclaredCombatRules::edge_cancel_recovery`].
pub fn edge_cancel_landing_recovery(
    rules: Option<bevy::prelude::Res<crate::rules::ResolvedCombatTuning>>,
    mut bodies: bevy::prelude::Query<(
        &ambition_platformer2d_core::BodyGroundState,
        &mut ambition_characters::actor::BodyCombat,
    )>,
) {
    // No resolved rules at all is a world outside a match, which declares
    // nothing and changes nothing.
    if !rules.is_some_and(|r| r.edge_cancel_recovery) {
        return;
    }
    for (ground, mut combat) in &mut bodies {
        if !ground.on_ground && combat.landing_lag_timer > 0.0 {
            combat.landing_lag_timer = 0.0;
        }
    }
}

/// An aerial move that touches down before it ended owes its authored landing
/// lag — unless it auto-cancelled.
///
/// The platform-fighter commitment rule, and the reason spacing an aerial is a
/// decision: you throw it knowing that landing mid-move costs you. A move that
/// authors neither field lands the way it always did, so this is inert for
/// every move that has not opted in.
///
/// body-generic by construction. It reads `MovePlayback` and
/// `BodyGroundState`, which every body carries — a CPU fighter, a possessed
/// boss and a human all pay the same lag for the same move. There is no
/// controller in the query.
///
/// the landing EDGE, not the grounded state. A move begun on the ground
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
        cancel_move_playback(&mut commands, owner, &mut playback);
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
        // THE TURNAROUND, because a move thrown out of one points the NEW way —
        // see the pivot note below.
        Option<&ambition_platformer2d_core::BodyMotionFacts>,
    )>,
) {
    for (control, kin, ground, mut state, tuning, mut resolved, motion) in &mut bodies {
        let frame = &control.0;
        // ⭐⭐ THE PIVOT: A MOVE THROWN OUT OF A TURNAROUND COMES OUT THE NEW
        // WAY. The body is mid-turn and has already committed to facing the
        // other direction, so what it throws goes with it — which is exactly
        // what a PIVOT GRAB is, and it needs no move of its own: the existing
        // forward move simply points the other way.
        //
        // ⭐ THE SAME RULE THE REVERSE AERIAL RUSH USES, and saying it once is
        // the point: a turnaround is FINISHED by whatever you commit to out of
        // it. Jumping resolves it in the movement kernel; acting resolves its
        // DIRECTION here.
        //
        // ⛔ RESOLVED HERE, at the ONE place facing is folded into an aim.
        // Doing it at the move selector instead looked right and changed
        // nothing — the direction is already decided by the time that code
        // runs, which a test of the pure `attack_dir_from_axis` would never
        // have caught.
        //
        // ⛔ THE DIRECTION ONLY: the kernel still owns the phase and its clock.
        let facing = if motion.is_some_and(|m| m.turning_around) {
            -kin.facing
        } else {
            kin.facing
        };
        *resolved = resolve_attack_gesture(
            &mut state,
            *tuning,
            frame.attack_axis,
            facing,
            ground.map(|g| g.on_ground).unwrap_or(true),
            frame.melee_pressed,
            frame.melee_held,
            frame.melee_released,
            frame.melee_strong_hint,
        );
    }
}

/// Feed semantic combat press edges into the body-owned action buffer, and
/// re-propose a buffered press on every tick of its window.
///
/// The buffer PROPOSES; [`trigger_moveset_moves`] still decides. A slot lives
/// until the action authority accepts the action — which spends it — or until
/// the authored window ([`AttackGestureTuning::action_buffer_s`]) runs out. So
/// an attack pressed a frame before endlag ends starts on the frame endlag
/// ends, and a press that was genuinely too early still costs the player the
/// press.
///
/// ⛔ NO per-move grace timers, and no device input: what is buffered is the
/// resolved control state every controller already produces, so leniency is one
/// mechanic for a human, a CPU, a replay and a policy alike.
///
/// The attack slot's clock lives on [`ae::BodyActionBuffer`] and its MEANING on
/// [`AttackGestureState::buffered_press`]; this is the one system that writes
/// either, which is what keeps them from disagreeing.
pub fn buffer_combat_action_presses(
    world_time: Res<WorldTime>,
    mut bodies: Query<(
        &ActorControl,
        &AttackGestureTuning,
        &mut AttackGestureState,
        &mut ResolvedAttackGesture,
        &mut ae::BodyActionBuffer,
        Option<&ProperTimeScale>,
        // WHERE THE BODY WAS WHEN IT ASKED. A special's direction is
        // facing-relative and its posture decides which of `special_down` and
        // `special_air_down` the player meant, and both have to be read at the
        // PRESS — see `SpecialGestureIntent`. `Option` for bare test bodies,
        // which are treated as standing and right-facing.
        Option<&ae::BodyKinematics>,
        Option<&ae::BodyGroundState>,
    )>,
) {
    for (control, tuning, mut state, mut resolved, mut buffer, scale, kin, ground) in &mut bodies {
        // ADR 0011: the owner's own clock, so a dilated body's leniency
        // dilates with everything else it does.
        let dt = world_time.entity_dt(scale.copied().unwrap_or_default());
        if !buffer.is_empty() {
            buffer.tick(dt);
        }
        // An expired window is an expired press, whichever half notices first.
        if buffer.attack <= 0.0 && state.buffered_press.is_some() {
            state.buffered_press = None;
        }
        if buffer.special <= 0.0 && state.buffered_special.is_some() {
            state.buffered_special = None;
        }
        let frame = &control.0;
        let window = tuning.action_buffer_s.max(0.0);
        if let Some(intent) = resolved.pressed {
            buffer.attack = window;
            state.buffered_press = Some(intent);
        } else if let Some(intent) = state.buffered_press {
            // ONE press field downstream: a buffered press and a live one are
            // the same event to every consumer, so nothing has to learn that
            // buffering exists.
            resolved.pressed = Some(intent);
        }
        if frame.grab_pressed {
            buffer.grab = window;
        }
        if frame.pogo_pressed {
            buffer.pogo = window;
        }
        // THE SPECIAL SLOT, resolved at the press and replayed verbatim.
        //
        // ⛔⛔ IT USED TO BE A BARE TIMER, and the replay re-read the direction
        // off the LIVE stick: press Up+Special during endlag, let go, and the
        // buffered press came out as a NEUTRAL special. Out of shield it did not
        // even qualify, because the out-of-shield rule asks whether the press
        // RISES. The buffer's own doc already said why that is wrong — "a
        // buffered press must be replayed verbatim rather than reinterpreted
        // from the live stick later" — and the attack slot beside it had obeyed
        // it since M1.
        if frame.special_pressed {
            buffer.special = window;
            state.buffered_special = Some(
                ambition_characters::actor::attack_gesture::SpecialGestureIntent {
                    direction: attack_dir_from_axis(
                        frame.attack_axis,
                        kin.map_or(1.0, |kin| kin.facing),
                    ),
                    // DECIDED HERE, not read off live ECS state at replay.
                    posture: if ground.is_none_or(|ground| ground.on_ground) {
                        ambition_characters::actor::attack_gesture::AttackPosture::Grounded
                    } else {
                        ambition_characters::actor::attack_gesture::AttackPosture::Airborne
                    },
                },
            );
        }
        // ONE field downstream, live or buffered, exactly as `pressed` is.
        resolved.special = state.buffered_special;
        // The SUSTAIN is the live button and nothing else. It is deliberately
        // not buffered: a buffer answers "did a press happen recently", and a
        // charge asks "is the finger still down", which only the current frame
        // can say.
        resolved.special_held = frame.special_held;
    }
}

/// Which buffered verb a resolution consumed, so acceptance spends exactly the
/// slot that proposed the move.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ProposedVerb {
    /// Nothing buffered proposed this (a taunt, a ranged intent, no move).
    Unbuffered,
    Attack,
    Grab,
    Special,
}

impl ProposedVerb {
    fn spend(self, buffer: &mut ae::BodyActionBuffer) {
        match self {
            ProposedVerb::Unbuffered => {}
            ProposedVerb::Attack => buffer.spend_attack(),
            ProposedVerb::Grab => buffer.grab = 0.0,
            ProposedVerb::Special => buffer.special = 0.0,
        }
    }
}

/// Name the writer of a move's self-motion impulse.
///
/// Two call sites author the same impulse — the plain trigger and the CANCEL path — and they
/// are byte-identical expressions.
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

/// Resolve the held weapon's directional Attack move, or `None` when the item
/// handles that press through another runtime path (for example a projectile or
/// throw system).
///
/// A weapon with melee vocabulary uses the normal moveset builder and therefore
/// inherits the directional attack family. The wearer's own attack timelines
/// remain intact for unequip/rewind; only press resolution changes. Keeping the
/// Attack slot present also preserves the touch-control prompt for non-melee
/// held items.
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

/// WHICH DIRECTIONS RISE out of a raised guard — the platform-fighter half of
/// the out-of-shield rule, and the only half that belongs to combat.
///
/// The permission itself is [`ae::OutOfShieldGate`], owned by the movement
/// kernel and read identically here. ⛔ It was written a second time in this
/// file: two implementations of one policy, which is how "up-smash may,
/// forward-smash may not" grows back as an exception list. What is genuinely
/// local is the DIRECTION reading below — up attack and up special RISE, which
/// is why this genre lets those two out of a crouched guard and makes
/// everything else wait for it to come down. `AttackDir` is combat's
/// vocabulary and does not travel down to the kernel to make the gate bigger.
fn rises_out_of_shield(
    gate: &ae::OutOfShieldGate,
    dir: AttackDir,
    action: ae::OutOfShieldAction,
) -> bool {
    gate.permits(action) && (gate.unrestricted() || dir == AttackDir::Up)
}

/// What a CAPTOR's press means: pummel, a directional throw, or nothing.
///
/// ⭐⭐ EXTRACTED FROM `trigger_moveset_moves`, which had grown into one
/// contextual resolver holding free attacks, smashes, specials, shield,
/// out-of-shield, grab, pummel, throws, items, buffers, cancels and running
/// variants in a single `if/else if` chain. A GPT review named the cost and the
/// evidence for it: the held-direction throw bug was another contextual
/// interpretation inside that chain getting hard to reason about.
///
/// ⛔ CAPTURE REPLACES THE ORDINARY ACTION CONTEXT rather than adding to it —
/// that is why this is a whole resolver and not a special case inside one. A
/// captor has exactly this vocabulary and nothing else.
///
/// `pressed` is the direction an attack press carried, `aimed` is what the
/// stick says right now, and `throw_armed` is whether that stick has been back
/// to neutral since the grab (see `SmashHoldState::throw_armed`).
fn resolve_capture_action<'a>(
    moveset: &ambition_entity_catalog::MovesetContract,
    pressed: Option<AttackDir>,
    aimed: AttackDir,
    throw_armed: bool,
    grounded: bool,
) -> (Option<MoveSpec>, &'a [&'a str], ProposedVerb) {
    // ⭐ A DIRECTION ALONE THROWS. Jon, 2026-08-23: *"Throw should not
    // require you to press attack and a direction. just pressing the
    // direction after you grab should trigger the throw."* That is the
    // genre's rule — a held opponent is thrown by tilting the stick, and
    // Attack is what PUMMELS — and this branch required an attack press
    // carrying a direction for both.
    //
    // ⛔ THE ATTACK PRESS STILL WORKS, deliberately. Attack+direction
    // was the only way to throw until now, so removing it would break
    // the input every existing test and every CPU brain already uses —
    // `capture_context_frame` presses Attack with an aimed stick and
    // knows nothing about this. The press is consulted FIRST so a
    // deliberate aimed press keeps its exact meaning; the stick is the
    // fallback that makes the direction sufficient on its own.
    //
    // ⛔ NEUTRAL IS NOT A THROW in either road: with no direction there
    // is nothing to throw toward, and a captor standing still holding a
    // captive must stay held rather than resolve some default throw.
    // ⛔⛔ THE STICK ROAD NEEDS AN EDGE, NOT A LEVEL. You walk into a
    // grab, so the stick that reached it is usually already pointing
    // somewhere — and reading the live axis on the first captive tick
    // threw the victim instantly, before the captor could pummel or
    // choose. `CapturedBy::throw_armed` says the captor's stick has
    // been back to neutral since this capture began, which is what
    // turns "still holding forward" into "pressed forward".
    //
    // ⛔ THE ATTACK PRESS DOES NOT WAIT FOR IT. A press is already an
    // edge — it is the input event, not the stick's position — so
    // Attack+direction throws exactly as it always did, and every
    // existing fixture and CPU that presses it keeps working.
    // ⛔ THE RULESET'S LATCH, read off the ruleset's own component. A
    // hold with no `SmashHoldState` is one this ruleset has no throw
    // vocabulary for, and it arms nothing — the same reading
    // `tick_capture_holds` takes of an absent hold state.
    let throw_dir = match pressed {
        // An aimed attack press: its own direction, unchanged.
        Some(dir) => Some(dir),
        // No press. The stick decides, and only once it has been
        // released since the grab.
        None if throw_armed && aimed != AttackDir::Neutral => Some(aimed),
        None => None,
    };
    match throw_dir {
        Some(AttackDir::Neutral) => (
            moveset
                .move_for_verb_in_stance(CAPTURE_PUMMEL_VERB, grounded)
                .cloned(),
            &[CAPTURE_PUMMEL_VERB][..],
            ProposedVerb::Attack,
        ),
        Some(AttackDir::Forward) => (
            moveset
                .move_for_verb_in_stance(CAPTURE_THROW_FORWARD_VERB, grounded)
                .cloned(),
            &[CAPTURE_THROW_FORWARD_VERB][..],
            ProposedVerb::Attack,
        ),
        Some(AttackDir::Back) => (
            moveset
                .move_for_verb_in_stance(CAPTURE_THROW_BACK_VERB, grounded)
                .cloned(),
            &[CAPTURE_THROW_BACK_VERB][..],
            ProposedVerb::Attack,
        ),
        Some(AttackDir::Up) => (
            moveset
                .move_for_verb_in_stance(CAPTURE_THROW_UP_VERB, grounded)
                .cloned(),
            &[CAPTURE_THROW_UP_VERB][..],
            ProposedVerb::Attack,
        ),
        Some(AttackDir::Down) => (
            moveset
                .move_for_verb_in_stance(CAPTURE_THROW_DOWN_VERB, grounded)
                .cloned(),
            &[CAPTURE_THROW_DOWN_VERB][..],
            ProposedVerb::Attack,
        ),
        None => (None, &[][..], ProposedVerb::Unbuffered),
    }
}

/// Trigger a body's authored move from control-frame verb edges. Directional
/// attack resolution follows the authored verb chain; ranged and special use
/// their corresponding verbs. A live move rejects replacement unless its
/// cancel window authorizes the requested move. Jump/dash cancel the move and
/// are executed by the locomotion path from the same control frame. Facing is
/// captured at trigger time.
///
/// This is the single trigger seam for every body. When a held weapon owns the
/// Attack press, [`held_weapon_attack_move`] resolves the weapon action instead
/// of the wearer's normal attack without deleting the wearer's authored moves.

/// Everything that happens when a press becomes a move.
///
/// ⭐ ONE FUNCTION, because the two start sites — the cancel path and the plain
/// trigger — did the same three things in two copies, and a fourth thing had to
/// join them. The recovery spend is that fourth thing; adding it beside the
/// duplicate would have made it a carry list of four, which is how this
/// repository loses a rule down one road.
struct StartingMove<'a, 'cw, 'cs> {
    commands: &'a mut Commands<'cw, 'cs>,
    entity: Entity,
    spec: ambition_entity_catalog::MoveSpec,
    facing: f32,
    /// Captured at START, because it is gone by the fire frame. See
    /// [`MovePlayback::aim`].
    aim: Option<(ae::Vec2, ae::GameplayFramePolicy)>,
    /// Which gesture the press RESOLVED to, when it is one that charges.
    started_by: Option<ambition_entity_catalog::ChargeGesture>,
    /// The DIRECTION the gesture asked for — see [`MovePlayback::attack_intent`].
    attack_intent: AttackIntent,
    /// The STANCE this move was SELECTED in — see
    /// [`MovePlayback::started_grounded`].
    started_grounded: bool,
    proposer: ProposedVerb,
    action_buffer: &'a mut ae::BodyActionBuffer,
    // ⭐ THE COMPONENTS, NOT THE QUERIES. A helper that took the queries would
    // tie the system's `Commands` and its two `Query` borrows to one
    // world/state pair, which they do not share; resolving both at the call site
    // says what this actually needs — the guard it spends and the budget it
    // charges.
    shield: Option<bevy::prelude::Mut<'a, ae::BodyShieldState>>,
    oos_policy: Option<ae::OutOfShield>,
    jump: Option<bevy::prelude::Mut<'a, ae::BodyJumpState>>,
    /// The B-REVERSE WINDOW this accepted move opens, the gesture history it
    /// opens on, and THE LATERAL SIGN THAT BOUGHT THE PRESS. `None` when the
    /// move is not a special, or the match declares no special turn — see
    /// `AttackGestureState::special_turn_ticks`.
    ///
    /// ⛔⛔ THE SIGN IS NOT OPTIONAL BOOKKEEPING. Without it the same tick's
    /// stick is read a second time in `CombatSet::Playback` as a fresh
    /// post-press flick, and a plain fresh Back+Special comes out a wavebounce.
    gesture_window: Option<(&'a mut AttackGestureState, u8, f32)>,
    /// The body's weapon and the recharge its ranged action authors — spent
    /// here when the accepted move is one that fires. `None` for a body with no
    /// melee cluster or no ranged action, which is every move that fires
    /// nothing.
    weapon: Option<(&'a mut BodyMelee, f32)>,
}

fn start_move(m: StartingMove<'_, '_, '_>) {
    // ⭐ THE RECOVERY IS SPENT AT THE START, not at the impulse and not on
    // landing, and NOT conditioned on the stance it started from. A move whose
    // `RecoveryUse` spends costs one use the moment it begins, so a fighter
    // cannot buy a second rise by cancelling out of the first — and
    // `afford_recovery` has already refused this move if there was none left.
    //
    // ⛔ SO THE RULE IS PER USE, NOT PER AIRTIME, and the difference is only
    // visible from the ground: a grounded recovery that gets stuffed before it
    // leaves the floor has still spent the charge. That is deliberate. The
    // player COMMITTED the recovery, and a version that waited for the body to
    // actually leave the ground would hand a fighter a free retry for every
    // grounded up-B somebody jabbed them out of. `started_grounded` is right
    // here in `StartingMove` and is deliberately not consulted.
    let StartingMove {
        commands,
        entity,
        spec,
        facing,
        aim,
        started_by,
        attack_intent,
        started_grounded,
        proposer,
        action_buffer,
        mut shield,
        oos_policy,
        jump,
        weapon,
        gesture_window,
    } = m;
    // Asked before `spec` is handed to the playback below.
    let fires_ranged = move_fires_ranged(&spec);
    if spec.gates.recovery.spends() {
        if let Some(mut jump) = jump {
            jump.recovery_charges = jump.recovery_charges.saturating_sub(1);
            // ⭐ THE EPISODE OPENS WITH THE LAST CHARGE, and it is armed here
            // rather than derived later because only the SPEND knows this was
            // the last one. While the recovery move plays, `body_is_helpless`
            // suppresses it — the move is the answer the fighter is giving.
            //
            // ⭐ …UNLESS THE MOVE DECLINES IT. A recovery that hands its owner a
            // vehicle has already given the height and the control together, so
            // the budget is the whole price — see
            // `RecoveryUse::SpendWithoutFreefall`. The CHARGE is spent either
            // way, one line up: this is the punishment, not the cost.
            jump.post_recovery_helpless =
                jump.recovery_charges == 0 && spec.gates.recovery.arms_freefall();
        }
    }
    // ⭐⭐ THE ONE LINE THAT SAYS A MOVE HAPPENED, and it is here because this is
    // the ACCEPTANCE authority: every attack, every special, from the trigger
    // road and the cancel road alike, passes through this function and nothing
    // else does. A log at the input edge would report presses that were refused;
    // a log at an effect would miss the moves that author none.
    //
    // ⛔ `info!` UNDER ITS OWN TARGET, not `debug!`. Jon asked for a log he can
    // hand back when something misbehaves — *"a log for when a player input a
    // major move… effects between those would be also easier to understand"* —
    // and a line nobody can see by default does not do that job. The target is
    // what keeps it filterable.
    //
    // ⚠ IT WILL REPEAT UNDER ROLLBACK. A resimulated frame re-runs this and says
    // the move started again. That is honest — the simulation really did start
    // it again — and the alternative, an authoritative-pass guard, hides the
    // resim from exactly the reader who needs to see it. Read a burst of
    // identical lines as a rewind, not as a repeated press.
    bevy::log::info!(
        target: "ambition::moves",
        "move accepted: entity={entity:?} move=`{}` grounded={started_grounded}",
        spec.id,
    );
    commands.entity(entity).insert(
        MovePlayback::new(spec, facing)
            .with_aim(aim)
            .with_attack_intent(attack_intent)
            .started_in_stance(started_grounded)
            .charged_by_gesture(started_by),
    );
    // ACCEPTED — the proposal is over. A buffered press that starts a move must
    // not start the next one behind it.
    proposer.spend(action_buffer);
    // ... and the guard leaves with the action it launched.
    if let Some(shield) = shield.as_mut() {
        ae::movement::spend_out_of_shield(shield, oos_policy);
    }
    // ⭐⭐ AND THE WEAPON IS SPENT AT ACCEPTANCE, not at the fire beat. That is
    // what lets `dispatch_move_events` promise the shot: by the time the
    // timeline reaches `MoveEventKind::Ranged` there is nothing left to refuse
    // it with, and no second firing move can have slipped in during the windup.
    //
    // ⛔ `max`, NOT ASSIGN. A shorter weapon must never shorten a longer
    // recharge that is already running (a worn modifier, a future rule) — the
    // only thing a fire may do to this clock is push it out.
    // ⭐ THE B-REVERSE WINDOW OPENS WHERE THE MOVE IS ACCEPTED, never where the
    // press is resolved: a press that starts nothing turns nobody.
    if let Some((gesture, ticks, press_sign)) = gesture_window {
        gesture.special_turn_ticks = ticks;
        // ⛔⛔ AND THE PRESS OWNS ITS OWN EDGE. `apply_special_turn_flicks` runs
        // LATER THIS TICK (`CombatSet::Playback`, ordered after
        // `CombatSet::Trigger`) and calls a lateral sign a flick when it differs
        // from the remembered one. Leaving the memory at whatever it held before
        // the press made the press itself qualify: the fresh Back that chose a
        // turnaround-B was counted again as the post-press flick, flipping the
        // facing twice and reversing the drift. Seeding it here is what makes
        // the Playback comment — *"a flick on the same tick as the press IS the
        // press, not a B-reverse"* — true of the code as well as of the intent.
        gesture.prev_lateral_sign = press_sign;
    }
    if let Some((melee, refire_s)) = weapon {
        if fires_ranged {
            melee.ranged_cooldown = melee.ranged_cooldown.max(refire_s.max(0.0));
        }
    }
}

/// May this body START this move, as far as its recovery budget is concerned?
///
/// ⛔⛔ WITHOUT THIS A PLATFORM FIGHTER HAS NO BOTTOM BLASTZONE. Measured at the
/// source 2026-08-24: `MoveSpec` carries no cooldown, no cost and no per-airtime
/// rule, and `MoveGates` knew only `grounded` — which cannot tell the second use
/// in one airtime from the first. A fighter authoring a rising special could
/// press it forever and could only be killed by a launch that outran its own
/// recovery.
///
/// ⚠ read-only, and asked BEFORE a cancel tears the current move down: refusing
/// after the teardown would leave the body with neither move.
fn afford_recovery(spec: &ambition_entity_catalog::MoveSpec, charges_left: Option<u8>) -> bool {
    if !spec.gates.recovery.spends() {
        return true;
    }
    // A body with no jump cluster is a bare fixture, not a fighter with an
    // exhausted budget — it has no recovery to spend and none to run out of.
    charges_left.is_none_or(|left| left > 0)
}

/// May this move start on a body whose pose somebody else owns?
///
/// ⭐ THE SIBLING OF [`afford_recovery`], and shaped like it on purpose:
/// read-only, asked BEFORE any teardown, returning a plain bool. Both answer the
/// same kind of question — *is this move allowed to begin* — and both must be
/// asked in the same breath, because a move that passes one and fails the other
/// after acceptance has already spent what it cost.
fn permitted_while_held(spec: &ambition_entity_catalog::MoveSpec, held: bool) -> bool {
    !(held && spec.gates.forbidden_while_held)
}

/// Does this move author a SHOT? A move whose timeline carries
/// [`MoveEventKind::Ranged`] fires the owner's ranged action when it reaches
/// that beat.
fn move_fires_ranged(spec: &ambition_entity_catalog::MoveSpec) -> bool {
    spec.events
        .iter()
        .any(|ev| matches!(ev.kind, ambition_entity_catalog::MoveEventKind::Ranged))
}

/// May this body START a move that fires its weapon?
///
/// ⭐⭐ THE WEAPON IS ASKED HERE, WHERE THE MOVE IS ACCEPTED — the one change
/// that makes an accepted firing move honest. The recharge used to be asked at
/// the projectile spawner instead, a quarter of a second after the move had
/// begun: the fighter committed, the windup played, the muzzle flashed, and the
/// shot was silently dropped. Measured 2026-08-23, that was happening to 22 of
/// 28 authored ranged events in the duel arena.
///
/// ⛔ AND REFUSING HERE IS NOT THE SAME AS SWALLOWING THE PRESS. This runs
/// BEFORE `proposer.spend`, so a buffered press keeps re-proposing for the rest
/// of its window and starts the move the moment the weapon comes back — the
/// ordinary buffering every other move already gets, rather than a queue of its
/// own.
///
/// ⛔ A MOVE THAT FIRES NOTHING IS NEVER REFUSED, and a body with no melee
/// cluster (a bare fixture) has no weapon to be recharging.
fn weapon_ready(spec: &ambition_entity_catalog::MoveSpec, melee: Option<&BodyMelee>) -> bool {
    !move_fires_ranged(spec) || melee.is_none_or(|m| m.ranged_cooldown <= 0.0)
}

/// A fighter that spent its recovery, is still in the air, and whose RECOVERY
/// move has ended.
///
/// ⭐⭐ THE ONE RULE, and it lives here because this is the lowest layer that can
/// see all three terms: `BodyJumpState` and `BodyGroundState` are the kernel's,
/// and `MovePlayback` is this crate's. The movement gates and the move-start
/// authority both call it, so a body cannot be helpless to one and not the other
/// — which is exactly what it was: the movement kernel knew, and
/// `trigger_moveset_moves` (which reads `ActorControl`, never an `InputState`)
/// did not, so a spent fighter could still start aerials and specials.
///
/// ⛔⛔ "STILL RECOVERING" MEANS THE RECOVERY, not any move. An earlier version
/// took ANY `MovePlayback` to postpone helplessness, so a fighter that threw its
/// recovery and then started anything at all stopped being helpless — which is
/// the state the rule exists to produce.
///
/// ⭐ AND IT CANNOT REACH A GAME THAT DOES NOT WANT IT. Charges only fall to zero
/// when a move whose `MoveGates::recovery` spends one does, so a cast that
/// authors no recovery never satisfies this — by construction, with no flag.
pub fn body_is_helpless(
    jump: &ae::BodyJumpState,
    grounded: bool,
    playing: Option<&MovePlayback>,
) -> bool {
    let still_recovering = playing.is_some_and(|pb| pb.spec.gates.recovery.spends());
    // ⛔⛔ THE EPISODE, NOT THE COUNT. `recovery_charges == 0` is a resource
    // reading, and a resource reading cannot be ENDED by anything short of a
    // refresh — refreshes are landing-shaped, which being hit deliberately is
    // not. So an accepted hit that hands the air dodge back gave a fighter a
    // dodge it was still forbidden to use. `post_recovery_helpless` is cleared
    // by the hit, AND SO IS THE CHARGE: freefall is the punishment for having
    // spent the recovery, and the genre lifts both together (see
    // `apply_body_hit_reaction`). The dodge half of the original reasoning
    // stands unchanged.
    jump.post_recovery_helpless && !grounded && !still_recovering
}

pub fn trigger_moveset_moves(
    mut commands: Commands,
    // THE MATCH'S DECLARED RULES, for the special-start turn (B-reverse /
    // wavebounce). `Option` because a world outside a match declares none and
    // turns nobody around.
    combat_rules: Option<bevy::prelude::Res<crate::rules::ResolvedCombatTuning>>,
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
        // What this body is holding. A weapon in hand OWNS the Attack press
        // ([`held_weapon_attack_move`]); every other verb is untouched.
        Option<&crate::held_items::HeldItem>,
        // Is this body RUNNING? Read for the dash attack, off the PUBLISHED
        // fact — ADR 0024's read surface, which is what a consumer outside the
        // movement kernel is owed.
        //
        // The tests passed because they told the selector the body was dashing; none of them could
        // ask whether a fighter ever is. `running` is ordinary grounded locomotion, which is where
        // the genre's running attack lives.
        //
        // `Option` for bare test bodies, which are treated as standing.
        Option<&ae::BodyMotionFacts>,
        // The body's buffered combat presses. This system is the SPEND site:
        // `buffer_combat_action_presses` re-proposes a press every tick of its
        // window, and accepting one is what ends the proposal — so a buffered
        // press starts exactly one move.
        &mut ae::BodyActionBuffer,
        // THE WEAPON, and the recharge its ranged action authors. Taken here
        // rather than by a lookup query because starting a firing move is a
        // SPEND — the same shape the guard and the recovery budget already use,
        // and they are looked up only because they are read by other systems in
        // the same set. `Option` on both: a body with no melee cluster and a
        // body with no ranged action both fire nothing.
        Option<&mut BodyMelee>,
        Option<&ambition_characters::brain::action_set::ActionSet>,
        // THE GESTURE HISTORY, so an accepted special can open its B-reverse
        // window. Taken here rather than in the gesture system because opening
        // it is an ACCEPTANCE, and this is the acceptance authority.
        // ⚠ NESTED, AND ONLY BECAUSE OF ARITY. Bevy's `QueryData` tuple runs out
        // at sixteen and this query reached it; the gesture pair travels with
        // the held-fact purely so the outer tuple still fits. No meaning is
        // implied by the grouping.
        (
            Option<&mut AttackGestureState>,
            Option<&AttackGestureTuning>,
            // IS SOMETHING ELSE HOLDING THIS BODY — a saddle, a lift, a grab.
            // Read as a bare fact rather than by asking WHO, because the domains
            // that hold a body are ones this crate does not depend on and should
            // not. See `MoveGates::forbidden_while_held`.
            bevy::prelude::Has<ambition_platformer2d_core::PoseOwnedExternally>,
        ),
    )>,
    // WHO IS HOLDING SOMEBODY. The inverse of `CapturedBy`, and the reason
    // there is no mirrored `Capturing` component to read instead: one authority,
    // scanned. At most one captive per captor and a handful per stage.
    captives: Query<(Entity, &crate::capture::CapturedBy)>,
    // The platform-fighter half of a hold, for the throw edge. Separate from
    // `captives` because the RELATION is generic and this is not: a ruleset
    // without a throw vocabulary carries no row here and arms nothing.
    hold_states: Query<&ambition_characters::smash_capture::SmashHoldState>,
    // THE GUARD, and the body's own shield policy. Starting an action out of a
    // raised shield is a SPEND — see `OutOfShieldGate` — so this is taken
    // mutably and looked up by entity, the same shape the strike seam uses.
    // `BodyShieldState` is deliberately absent from the body query above so
    // this is the system's only access to it.
    mut guards: Query<&mut ae::BodyShieldState>,
    // THE RECOVERY BUDGET. Taken mutably and looked up by entity like the guard
    // above, and for the same reason: starting a recovery is a SPEND.
    mut jumps: Query<&mut ae::BodyJumpState>,
    // Where the out-of-shield rule is authored: the body's own movement policy
    // carries its shield tuning. `None` for a bare test body, which then has no
    // rule and behaves exactly as it did.
    shield_policies: Query<&ae::MotionModel>,
    // This is the second writer to name itself.
    //
    // `Option` on both: the FEATURE and the PLUGIN are two switches, and a
    // body without an identity is still a body.
    #[cfg(feature = "causal")] log: Option<bevy::prelude::ResMut<ambition_causal::CausalRecording>>,
    #[cfg(feature = "causal")] identities: Query<&crate::components::ActorIdentity>,
    #[cfg(feature = "causal")] tick: Option<bevy::prelude::Res<ambition_time::SimTick>>,
) {
    #[cfg(feature = "causal")]
    let mut log = log;
    for (
        entity,
        moveset,
        control,
        gesture,
        resolved_frame,
        mut kin,
        ground,
        playback,
        held,
        motion_facts,
        mut action_buffer,
        mut melee,
        action_set,
        // ⚠ `body_is_held` is NOT `held` above, which is the item in this body's
        // HAND. This one is whether the BODY is held.
        (mut gesture_state, gesture_tuning, body_is_held),
    ) in &mut bodies
    {
        // The weapon this body would spend if the move it starts fires one.
        let refire_s = action_set
            .and_then(|set| set.ranged.as_ref())
            .map(|ranged| ranged.refire_s);
        let body_frame = resolved_frame
            .map(|frame| frame.basis())
            .unwrap_or(ae::AccelerationFrame::new(ae::DEFAULT_GRAVITY_DIR));
        let frame = &control.0;
        let grounded = ground.map(|g| g.on_ground).unwrap_or(true);
        // The stick sign an accepted special will seed its flick memory with —
        // read through the SAME function the post-press recognizer uses, so the
        // seed and the comparison cannot drift.
        let press_lateral_sign = gesture_tuning
            .map(|tuning| {
                ambition_characters::actor::attack_gesture::special_turn_stick_sign(control, tuning)
            })
            .unwrap_or(0.0);
        // ⛔⛔ A HELPLESS FIGHTER STARTS NOTHING, and this is the authority that
        // decides it — not the movement kernel's `InputState`, which this system
        // never reads. The gate lived only there, so a fighter that had spent its
        // recovery could not jump or air-dodge and could still throw an aerial
        // or another special, which is the whole of what helplessness forbids.
        //
        // ⭐ BEFORE any resolution: refusing at the START is what makes it one
        // rule, rather than a list of verbs each remembering to check.
        if jumps
            .get(entity)
            .ok()
            .is_some_and(|jump| body_is_helpless(jump, grounded, playback.as_deref()))
        {
            continue;
        }
        // ⭐⭐ A ROLL'S RECOVERY REFUSES MOVES, AND THE PRECONDITION THIS COMMENT
        // ASKED FOR IS NOW MET. It used to read: *"before this becomes an action
        // gate, the roll needs a timer that is ITS OWN"* — because
        // `dodge_roll_timer` is shared with the SPOT DODGE (`facts.spot_dodging`
        // is a refinement of `dodge_rolling`, not a sibling), so an endlag hung
        // off its expiry silenced fighters that had only spot-dodged, and gating
        // on it broke `an_up_tilt_launches_much_further_at_a_high_percent`.
        //
        // ⇒ THE KERNEL ARMS `dodge_roll_endlag_timer` ONLY FOR A ROLL now
        // (`if state.dodge_roll_timer <= 0.0 && !state.spot_dodging`), so the
        // fact reaches exactly the maneuver that owes recovery. The punish
        // window a roll buys is real: it was canonical state that nothing
        // consulted, which is a mechanic recorded and not implemented.
        //
        // ⛔ A SPOT DODGE STILL OWES NOTHING — it never receives this timer.
        if motion_facts.is_some_and(|facts| facts.dodge_roll_endlag) {
            continue;
        }
        // ⭐⭐ AN EVADE IS A COMMITMENT UNTIL ITS TAIL. Today an attack cancels a
        // spot dodge on frame one, which makes the dodge strictly better than
        // the genre's: invulnerable AND instantly actionable. The tail is what
        // stays cancellable — the genre's spot-dodge-into-attack.
        //
        // ⭐ THE FACT, not a timer. `evade_committed` is resolved in the kernel,
        // which is the only place holding both the evade's remaining time and
        // this body's tuning; a gate here that subtracted a timer from a
        // constant would re-derive the rule and drift from it.
        //
        // ⛔ AND IT IS FALSE FOR EVERY BODY IN A GAME THAT DECLARES NO TAIL, so
        // this refuses nothing that was previously allowed unless a match asked.
        // That distinction is what an earlier attempt to gate on
        // `dodge_roll_endlag` got wrong: that timer is SHARED with the spot
        // dodge, so it silenced fighters that had only spot-dodged.
        if motion_facts.is_some_and(|facts| facts.evade_committed) {
            continue;
        }
        let running = motion_facts.is_some_and(|facts| facts.running);
        // Capture replaces the ordinary action context. Resolve only pummel or
        // directional throw verbs while holding a captive; throws ignore strike
        // charge strength, and an unauthored throw resolves to no move.
        let holding_captive = crate::capture::captive_of(entity, &captives).is_some();
        // A BUFFERED EDGE IS AN EDGE. `buffer_combat_action_presses` holds a
        // press open for its authored window and this reads the two the same
        // way, so the resolution chain below does not know which one arrived.
        // (`gesture.pressed` is already unified upstream — the buffer republishes
        // into it, intent and all.)
        // THE OUT-OF-SHIELD RULE, read once for this body's press. Whether a
        // raised guard is a launching platform or a wall comes from the body's
        // own authored policy — never from a list of moves that get exceptions.
        let oos_policy = shield_policies
            .get(entity)
            .ok()
            .and_then(|model| model.shield_tuning().out_of_shield);
        let gate = ae::OutOfShieldGate::read(
            guards.get(entity).is_ok_and(|shield| shield.active),
            oos_policy,
        );
        let grab_pressed = frame.grab_pressed || action_buffer.grab > 0.0;
        // ⭐ ATTACK ON A RAISED GUARD IS A GRAB, and it is the genre's rule:
        // shield + A grabs, and it is how most players grab at all. Jon,
        // 2026-08-23: *"if you are shielding and press a, that should trigger a
        // grab."*
        //
        // Before this, an attack press from behind a guard reached the ATTACK
        // arm, which `rises_out_of_shield` lets through only when it is aimed
        // UP — so shield + A did nothing whatsoever unless you were holding up,
        // and the grab was reachable only from a dedicated grab button.
        //
        // ⛔ It still asks the body's own out-of-shield POLICY below, exactly as
        // the dedicated button does: this adds a road to the grab, never an
        // exemption from the rule that says whether a guard may spend itself on
        // one. A game that declares no policy is unrestricted and always was.
        //
        // ⛔ Neutral or aimed makes no difference: a grab has no directions, so
        // reading one here would invent a vocabulary the capture kit does not
        // author.
        // ⛔⛔ THE BUTTON, NOT THE BODY STATE, and the difference is a whole
        // tick. `guard_up` reads `BodyShieldState::active`, which OUTLIVES the
        // press that raised it — a guard comes down through drop lag, not on the
        // frame the button is released — so gating on it turns an attack thrown
        // just after letting go into a surprise grab.
        //
        // ⭐ MEASURED, and it is not a small effect: gated on `guard_up`, the
        // CPUs' offence became grabbing and `every_live_fighter_stays_inside_the_frame`
        // went red on its PREMISE - "no fighter was ever outside the room's own
        // bounds in this match (0 body-frames)". Nobody was knocked off the
        // stage at all, because a grab launches nobody. The fighter brain never
        // holds shield on a frame it attacks; the body's guard was simply still
        // standing when the press arrived.
        //
        // Holding the shield is also what the genre asks of a player: you grab
        // OUT OF a guard you are holding, not out of one you just dropped.
        let shield_grab = frame.shield_held && gesture.pressed.is_some();
        // THE SPECIAL PRESS, live or replayed, WITH ITS MEANING. ⛔ the bare
        // `action_buffer.special > 0.0` this replaces said only THAT a press was
        // waiting, never what it was — see `SpecialGestureIntent`.
        let special_intent = gesture.special;
        let pogo_pressed = frame.pogo_pressed || action_buffer.pogo > 0.0;
        // Set by the arm that resolves a CHARGEABLE gesture — the attack arm on
        // a smash, the special arm on a special. Every other verb leaves it
        // `None` and its move never freezes.
        let mut started_by: Option<ambition_entity_catalog::ChargeGesture> = None;
        // THE PROPOSED SPECIAL-TURN EFFECT, applied only where the move is
        // actually accepted. See the special arm below for why it is not applied
        // where it is decided.
        let mut special_turn = false;

        // ⭐⭐ THE PIVOT'S OTHER HALF. `resolve_attack_gestures` already resolves
        // the attack DIRECTION against `-kin.facing` while the body is turning —
        // that is what makes a pivot grab need no move of its own. The body
        // still HOLDS the old facing, and `start_move` snapshots it into the
        // playback, which is the value every hit volume is mirrored by and every
        // `start_impulse` multiplied by. So the correct move came out pointing
        // backwards: the right name, the wrong geometry.
        // Ticks of B-reverse window this press would open. `0` = none — not a
        // special, or a match that declares no special turn.
        let mut special_turn_ticks = 0u8;
        let mut pivot_turn = false;
        // The direction the resolved gesture asked for, carried onto the
        // accepted playback. `Forward` for a move no directional gesture
        // started — see `MovePlayback::attack_intent`.
        let mut attack_intent = AttackIntent::Forward;
        // ⭐⭐ PROPOSED HERE, COMMITTED WHERE THE MOVE STARTS. The double-jump
        // cancel used to write `kin.vel` in the middle of resolution, before
        // `cancel_permits` had been asked — so a buffered aerial thrown during a
        // non-cancelable move killed the fighter's rise and then started
        // nothing. A rejected attack must not change physics. Same correction
        // the special-turn above already got, and this is its second customer.
        let mut cancel_air_jump_rise = 0.0f32;
        //  every branch below asks for a move IN THIS STANCE. The capture kit
        // declares its whole vocabulary grounded-only, and a captor carried into
        // the air was still able to pummel and throw because the exact-verb
        // lookup did not read the gate its own repertoire had authored.
        type Resolution<'a> = (Option<MoveSpec>, &'a [&'a str], ProposedVerb);
        let (spec, verb_names, proposer): Resolution = if holding_captive {
            // ⛔ THE RULESET'S LATCH, read off the ruleset's own component. A
            // hold with no `SmashHoldState` is one this ruleset has no throw
            // vocabulary for, and it arms nothing.
            let throw_armed = crate::capture::captive_of(entity, &captives)
                .and_then(|victim| hold_states.get(victim).ok())
                .is_some_and(|state| state.throw_armed);
            resolve_capture_action(
                &moveset.0,
                gesture.pressed.map(|intent| intent.direction),
                attack_dir_from_axis(frame.attack_axis, kin.facing),
                throw_armed,
                grounded,
            )
        } else if (grab_pressed || shield_grab) && gate.permits(ae::OutOfShieldAction::Grab) {
            // A free body's grab. The move's own Active window carries the
            // capture attempt; this only starts the move.
            //
            // ⭐ **the RUNNING grab**, the capture kit's half of the dash
            // attack: a body already running reaches out with `grab_dash`, which
            // every fighter has because `SmashCaptureRepertoire::bound` derives
            // it from that fighter's own standing grab. Same gait fact the dash
            // attack reads — ⛔ `running`, NEVER `BodyMotionFacts::dashing`,
            // which `SMASH_FIGHTER_KIT` leaves permanently false and which made
            // the dash attack unreachable in the game it was built for.
            (
                moveset
                    .0
                    .move_for_flat_verb(GRAB_VERB, grounded, running)
                    .cloned(),
                &[GRAB_VERB][..],
                ProposedVerb::Grab,
            )
        } else if special_intent.is_some_and(|intent| {
            rises_out_of_shield(&gate, intent.direction, ae::OutOfShieldAction::UpSpecial)
        }) {
            // ⛔⛔ THE PRESS'S OWN DIRECTION AND POSTURE, not the live stick's.
            // `special_intent` is `ResolvedAttackGesture::special`, which
            // `buffer_combat_action_presses` resolves at the press and republishes
            // every tick of its window — so a buffered Up+Special replayed after
            // the stick has centred is still an up-special, still qualifies as one
            // out of shield, and still picks the AIR variant if that is where the
            // player asked from.
            let intent = special_intent.expect("the arm above matched on it");
            let special = (
                moveset
                    .0
                    .move_for_directional_verb(
                        SPECIAL_VERB,
                        intent.direction,
                        matches!(
                            intent.posture,
                            ambition_characters::actor::attack_gesture::AttackPosture::Grounded
                        ),
                    )
                    .cloned(),
                &[SPECIAL_VERB][..],
                ProposedVerb::Special,
            );
            // ⭐⭐ B-REVERSE AND WAVEBOUNCE, AS TWO SETTINGS OF ONE RULE. A
            // special started with a BACK press turns the fighter around, and —
            // if the ruleset asks for the stronger version — reverses its drift
            // with it. The genre has both; making them one special-start policy
            // is what keeps either from becoming a fighter-specific velocity
            // hack, which the parity row rules out by name.
            //
            // ⛔ THE PRESS'S OWN DIRECTION, like everything else in this arm:
            // `AttackDir::Back` already means "away from facing" and is
            // republished for the whole buffered window, so a press read after
            // the stick centres still means what it meant.
            //
            // ⛔ IT DOES NOT TOUCH MOVE SELECTION. The move was already chosen
            // above from the same direction; this is where the BODY answers,
            // and the two stay separate exactly as the row requires.
            // ⛔⛔ PROPOSED HERE, APPLIED AT ACCEPTANCE. This arm is still
            // RESOLVING which move the press would start, and that resolution
            // can come back `None` — a fighter with no authored special for this
            // direction. Turning the body here meant a press that threw nothing
            // still turned it, and a BUFFERED press turned it again every tick.
            //
            // ⭐ THE GENERAL RULE: proposing may compute, only accepting may
            // mutate. `facing`, `vel` and resource counters belong to the body,
            // and a press that starts no move has spent nothing.
            let pressed_back = matches!(intent.direction, AttackDir::Back);
            special_turn = pressed_back && combat_rules.as_ref().is_some_and(|r| r.special_turn);
            // ⭐⭐ AND THE OTHER TOGGLE OPENS ITS WINDOW. A flick DURING it flips
            // the facing again and reverses the drift — so back-then-flick is a
            // B-reverse, and back-before-AND-flick-after flips twice (which is
            // no flip) and reverses the drift, which is a WAVEBOUNCE. The fourth
            // outcome needs no recognition of its own.
            //
            // ⛔ THE RULESET'S OWN READING OF "one gesture", not a new knob:
            // `flick_window_ticks` is already how long a flick and a press count
            // as the same intent.
            if combat_rules.as_ref().is_some_and(|r| r.special_turn) {
                // ⛔ THE AUTHORED TICKS, SPENT AS TICKS. This used to divide by
                // a hardcoded 60.0 into seconds that were then aged on the
                // SCALED clock, so the same authored number bought a different
                // number of input opportunities at every time scale.
                // ⭐ ITS OWN WINDOW, AND `+ 1` BECAUSE THE KNOB COUNTS
                // SUBSEQUENT TICKS. The recognizer runs later THIS tick (it is
                // in `CombatSet::Playback`) and spends one there; without the
                // extra, an authored `4` bought three chances while the ordinary
                // attack flick's `4` buys four (`age_ticks <= 4`). One word in
                // two mechanics' mouths meaning two different counts is what
                // this whole split is about.
                special_turn_ticks = gesture_tuning
                    .map(|tuning| tuning.special_turn_window_ticks.saturating_add(1))
                    .unwrap_or(0);
            }
            // ⛔⛔ NOT GATED ON THE TURN. The two halves are INDEPENDENT
            // outcomes, and gating this one behind `special_turn` made the
            // fourth combination undeclarable:
            //
            //   turn  drift   technique
            //   ────  ─────   ─────────────────────────────────────────────
            //   no    no      an ordinary special
            //   yes   no      turnaround-B — you come out facing the other way
            //   yes   yes     B-reverse — facing AND momentum turn
            //   no    yes     a WAVEBOUNCE — momentum turns, facing does not
            //
            // ⚠ THE RECOGNISER IS STILL MISSING, and this does not pretend
            // otherwise: the genre distinguishes these by the ORDER of stick and
            // button, which this arm cannot see — it is handed one resolved
            // direction. So a game DECLARES which technique its Back+Special
            // performs rather than a player choosing per press. Shipping the
            // knob is worth more than withholding it until the recogniser
            // exists; see the ledger row for the input-order half.
            // THIS USE IS A SPECIAL, recorded for the same reason the smash arm
            // records its own: the resolution that chose the verb is what makes
            // a chargeable neutral-B chargeable, and a move reached through
            // another verb must not freeze.
            started_by = Some(ambition_entity_catalog::ChargeGesture::Special);
            special
        } else if (gesture.pressed.is_some() || pogo_pressed)
            && rises_out_of_shield(
                &gate,
                gesture
                    .pressed
                    .map_or(AttackDir::Down, |intent| intent.direction),
                ae::OutOfShieldAction::UpAttack,
            )
        {
            // A dedicated pogo press IS a down-air (the move carrying the pogo
            // on-hit technique); a plain melee press resolves by aim. When only
            // pogo is pressed, force Down so an aerial body reaches `attack_air_down`.
            let (base_verb, dir, gesture_grounded) = if pogo_pressed && gesture.pressed.is_none() {
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
            // A WEAPON IN HAND OWNS THIS PRESS. With something held, the
            // wearer's own repertoire is not consulted at all — the weapon
            // answers with its swing, or it answers elsewhere and nothing runs
            // here (see [`held_weapon_attack_move`] for why this arbitrates
            // instead of revoking the wearer's verbs).
            // A RUN PRE-EMPTS THE SMASH GESTURE, and it has to, because
            // the two inputs are the same one. `resolve_attack_gesture` calls a
            // press a SMASH when a direction FLICK preceded it inside the
            // window — and flicking a direction is exactly how a player enters
            // a run, so the canonical dash-attack input (tap forward, press
            // Attack) was answered with a forward smash and the running attack
            // was unreachable by the way it is actually performed.
            //
            // this is the genre's answer, not a preference. Ultimate,
            // Smash 4, Brawl and Melee all resolve Attack-out-of-a-run as the
            // running attack; none of them lets a forward smash come straight
            // out of one without a cancel first. So there is no knob here —
            // where the games AGREE, we ship what they do.
            //
            // conditioned on the move being AUTHORED, which is what keeps
            // this from stealing a smash. A fighter with no running attack
            // resolves its press exactly as before, rather than falling through
            // to a tilt.
            let running_attack = running
                && gesture_grounded
                && moveset
                    .0
                    .move_for_verb_in_stance(
                        &ambition_entity_catalog::dash_stance_verb(ATTACK_VERB),
                        gesture_grounded,
                    )
                    .is_some();
            let spec = if let Some(held) = held {
                held_weapon_attack_move(&held.spec, dir, gesture_grounded)
            } else {
                moveset
                    .0
                    // the DASH ATTACK. A running body's press is its own
                    // move in this genre, and it was resolving as whatever the
                    // stick happened to name — the forward tilt, or the jab.
                    // The base is forced to ATTACK when the run owns the press,
                    // so a `smash_dash` nobody authors is never asked for and
                    // the runtime's verb vocabulary stays the one list.
                    .move_for_attack(
                        if running_attack {
                            ATTACK_VERB
                        } else {
                            base_verb
                        },
                        dir,
                        gesture_grounded,
                        running_attack,
                    )
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
            // a running attack answers to the ATTACK family whatever gesture
            // asked for it — the cancel namespace follows the move that ran.
            let verb_names: &[&str] = if base_verb == SMASH_VERB && !running_attack {
                &[SMASH_VERB, ATTACK_VERB, "any_attack"]
            } else {
                &[ATTACK_VERB, "any_attack"]
            };
            // THIS USE IS A SMASH. Recorded here and nowhere else: the same
            // resolution that chose the smash verb is what makes the use
            // chargeable, so a move borrowed by another verb — or a running
            // attack that pre-empted the gesture — never freezes its timeline.
            // ⭐⭐ THE DOUBLE-JUMP CANCEL: throwing an AERIAL out of a jump you
            // spent in the air kills the rest of that jump's rise. It is what
            // turns a double jump into an approach rather than a commitment —
            // rise, throw, and land where you chose instead of at the top of an
            // arc.
            //
            // ⛔ THE BOUND IS IN THE FACT, NOT HERE — and it is an AMOUNT now.
            // `air_jump_rise_owned` is what this body's own air jump put in and
            // still has, a quantity that only ever shrinks. The bool it replaced
            // said "an air jump was spent, and I am rising no faster than one
            // could push me", which a weak opponent launch also satisfies for
            // the rest of the airtime — so an aerial deleted knockback.
            if combat_rules.as_ref().is_some_and(|r| r.double_jump_cancel) && !gesture_grounded {
                cancel_air_jump_rise = motion_facts
                    .map(|facts| facts.air_jump_rise_owned)
                    .unwrap_or(0.0);
            }
            started_by = (base_verb == SMASH_VERB && !running_attack)
                .then_some(ambition_entity_catalog::ChargeGesture::Smash);
            // ⛔ PROPOSED, like everything else on this road. A press the
            // playing move refuses must not turn the fighter around.
            pivot_turn = motion_facts.is_some_and(|facts| facts.turning_around);
            // ⭐ AND THE DIRECTION TRAVELS WITH THE MOVE. Resolved from the same
            // three facts the move was selected from, so the read-model swing
            // never has to guess it back out of a move id.
            attack_intent = attack_intent_of(
                dir,
                if gesture_grounded {
                    AttackPosture::Grounded
                } else {
                    AttackPosture::Airborne
                },
                running_attack,
            );
            (spec, verb_names, ProposedVerb::Attack)
        } else if frame.taunt_pressed {
            // LAST in the chain on purpose: a taunt loses to every verb that
            // does something, so a press that overlaps a real action is that
            // action rather than a mood.
            (
                moveset
                    .0
                    .move_for_directional_verb(
                        TAUNT_VERB,
                        attack_dir_from_axis(frame.attack_axis, kin.facing),
                        grounded,
                    )
                    .cloned(),
                &[TAUNT_VERB][..],
                ProposedVerb::Unbuffered,
            )
        } else if frame.fire.is_some() {
            // A ranged intent (`frame.fire = Some(dir)`) starts the body's `"ranged"`
            // move; its fire event spawns the projectile, sampling live aim. The move
            // plays to completion before another starts (its duration is a cadence
            // gate; the body-side refire cooldown remains the hard rate floor).
            (
                moveset
                    .0
                    .move_for_verb_in_stance(RANGED_VERB, grounded)
                    .cloned(),
                &[RANGED_VERB],
                ProposedVerb::Unbuffered,
            )
        } else {
            (None, &[], ProposedVerb::Unbuffered)
        };

        if let Some(mut pb) = playback {
            // CM4, locomotion escapes: a `jump`/`dash` edge inside a permitting
            // cancel window ENDS the move (early recovery-cancel); the verb
            // itself runs through the normal locomotion path this same tick.
            //
            // the edge is now the BURST press, but the AUTHORED cancel class
            // stays `"dash"`: it is content vocabulary (`CANCEL_CLASS_NAMES`),
            // not the channel's name. Renaming it is a CONTENT migration.
            //
            // Zero content sites spell `"dash"` today; the word survives only here, in
            // `CANCEL_CLASS_NAMES` and in `ambition_entity_catalog`'s own tests.
            let loco = if frame.jump_pressed {
                Some("jump")
            } else if frame.burst_pressed {
                Some("dash")
            } else {
                None
            };
            if let Some(name) = loco {
                if pb.spec.cancel_permits(pb.t, pb.landed_hit, &[name]) {
                    cancel_move_playback(&mut commands, entity, &mut pb);
                    continue;
                }
            }
            // CM4, move-into-move: the requested move starts same-frame iff a
            // cancel window covering `t` permits it under the hit-state
            // condition. Otherwise: today's reject, byte-identically.
            // ⭐ THE CHAIN. A follow-up on the attack family inside a cancel
            // window takes the successor that window NAMES instead of
            // restarting the move that is playing — jab into jab2 into jab3,
            // authored as a cancel table and nothing else.
            //
            // Only an UNDIRECTED follow-up takes it, which is what keeps this a
            // chain rather than a second cancel rule: in this genre a tilt out
            // of jab 1 is a tilt, and a directed press is a genuine
            // move-into-move that already had its answer. ⛔ no fighter id and
            // no move-name special case — the vocabulary is the `into` list a
            // window already authors.
            //
            // TWO FOLLOW-UPS CONTINUE A STRING, AND HOLDING IS THE ONE THIS
            // GAME'S BODIES ACTUALLY PRODUCE. `MovePlayback`'s other two
            // sustained mechanics both read `ResolvedAttackGesture::held` — the
            // smash charge waits on it and the flurry loop repeats on it — and
            // this one read the press EDGE alone. Measured over a 90-second
            // George mirror: the two brains hold Attack for 960 body-ticks in
            // thirteen runs (median 66 ticks each) and produce exactly ONE
            // fresh neutral jab press in the whole match. The chain's only
            // entrance was the input nobody gives.
            //
            // ⛔ A HOLD MAY ONLY CONTINUE A STRING, NEVER START A MOVE. It is
            // read only when no press resolved a move of its own, and it can
            // reach nothing but a successor the playing window already NAMES —
            // so a held button repeats exactly what was authored as a chain and
            // nothing else.
            //
            // ⛔⛔ A HELD SMASH IS A CHARGE, and the two must never be the same
            // gesture: excluded by strength here, not by asking which move is
            // playing.
            let neutral_repress = proposer == ProposedVerb::Attack
                && gesture.pressed.map(|intent| intent.direction) == Some(AttackDir::Neutral);
            let held_string = spec.is_none()
                && gesture.held.is_some_and(|intent| {
                    intent.direction == AttackDir::Neutral
                        && intent.strength != AttackStrength::Smash
                });
            let successor = (neutral_repress || held_string)
                .then(|| {
                    pb.spec
                        .cancel_successors(pb.t, pb.landed_hit)
                        .find(|id| *id != pb.spec.id.as_str())
                        .and_then(|id| moveset.0.move_by_id(id))
                        .cloned()
                })
                .flatten();
            let Some(spec) = successor.or(spec) else {
                continue;
            };
            // ⛔ ASKED BEFORE THE TEARDOWN BELOW. Refusing a move the body cannot
            // afford AFTER cancelling the one it was playing would leave it with
            // neither, which is worse than the free recovery this prevents.
            if !afford_recovery(&spec, jumps.get(entity).ok().map(|j| j.recovery_charges)) {
                continue;
            }
            // ⛔ AND ASKED BEFORE THE TEARDOWN FOR THE SAME REASON: a fighter
            // refused for a recharging weapon keeps the move it was playing.
            if !weapon_ready(&spec, melee.as_deref()) {
                continue;
            }
            // ⛔ THE CANCEL ROAD OWES THE SAME REFUSAL AS THE TRIGGER ROAD. A
            // move forbidden while the body is held is forbidden however it was
            // reached; a gate enforced on only one of two entry paths is a gate
            // somebody will walk around without meaning to.
            if !permitted_while_held(&spec, body_is_held) {
                continue;
            }
            let mut names: Vec<&str> = verb_names.to_vec();
            names.push(spec.id.as_str());
            if !pb.spec.cancel_permits(pb.t, pb.landed_hit, &names) {
                continue;
            }
            // Tear down exactly as natural completion does — LITERALLY the same
            // function now, rather than the same lines retyped — then replace the
            // playback, since insert overwrites. Only the volume half: removing
            // the component here and re-inserting it below would be two writes
            // where one will do.
            despawn_live_boxes(&mut commands, &mut pb);
            // ⭐⭐ THE ACCEPTED SPECIAL-TURN, and this is its ONE commit point on
            // this road. Proposed where the special was resolved; applied here,
            // where the move is certain to start. ⛔ BEFORE the start impulse:
            // the turn reverses the drift the fighter ARRIVED with, and running
            // it afterwards would reverse the move's own impulse too.
            // ⭐ TWO ROADS ASK FOR ONE FLIP — the special arm's B-reverse and
            // the attack arm's PIVOT — and they are exclusive by construction,
            // being different verbs. `||`, so a reader does not have to wonder
            // whether two flips could cancel.
            if special_turn || pivot_turn {
                kin.facing = -kin.facing;
            }
            // ⭐⭐ THE ACCEPTED DOUBLE-JUMP CANCEL, and this is its commit point
            // for the same reason the turn above is: the move is certain to
            // start here and was only proposed where it was resolved.
            //
            // ⛔⛔ AND THE AXIS IS THE BODY'S OWN RISE, NOT WORLD Y. Under
            // rotated gravity a fighter's up IS world X, and `vel.y` would shed
            // its lateral drift instead. ⛔ It sheds at most what the jump put
            // in and at most what is actually there, so a launch the fighter is
            // riding survives whatever it throws.
            if cancel_air_jump_rise > 0.0 {
                let down = body_frame.down;
                let rise = -kin.vel.dot(down);
                let shed = rise.min(cancel_air_jump_rise).max(0.0);
                kin.vel += shed * down;
            }
            // ⛔⛔ THE DRIFT HALF IS NOT HERE ANY MORE, and that is the
            // recogniser. It used to reverse on EVERY back-special, which is the
            // B-reverse final state applied unconditionally — so one gesture
            // could not choose between turnaround-B, B-reverse and wavebounce.
            // A flick during the window this move just opened is what buys it:
            // see `apply_special_turn_flicks`.
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
            start_move(StartingMove {
                commands: &mut commands,
                entity,
                spec,
                facing: kin.facing,
                aim: control.0.fire.map(|req| (req.dir, req.dir_policy)),
                started_by,
                attack_intent,
                // The SAME stance the selector used — see
                // `MovePlayback::started_grounded`.
                started_grounded: grounded,
                proposer,
                action_buffer: &mut action_buffer,
                shield: guards.get_mut(entity).ok(),
                oos_policy,
                jump: jumps.get_mut(entity).ok(),
                weapon: melee.as_deref_mut().zip(refire_s),
                gesture_window: (special_turn_ticks > 0)
                    .then(|| {
                        gesture_state
                            .as_deref_mut()
                            .map(|g| (g, special_turn_ticks, press_lateral_sign))
                    })
                    .flatten(),
            });
            continue;
        }

        let charges_left = jumps.get(entity).ok().map(|jump| jump.recovery_charges);
        if let Some(spec) = spec
            .filter(|spec| afford_recovery(spec, charges_left))
            .filter(|spec| permitted_while_held(spec, body_is_held))
            .filter(|spec| weapon_ready(spec, melee.as_deref()))
        {
            // ⭐⭐ THE ACCEPTED SPECIAL-TURN, and this is its ONE commit point on
            // this road. Proposed where the special was resolved; applied here,
            // where the move is certain to start. ⛔ BEFORE the start impulse:
            // the turn reverses the drift the fighter ARRIVED with, and running
            // it afterwards would reverse the move's own impulse too.
            // ⭐ TWO ROADS ASK FOR ONE FLIP — the special arm's B-reverse and
            // the attack arm's PIVOT — and they are exclusive by construction,
            // being different verbs. `||`, so a reader does not have to wonder
            // whether two flips could cancel.
            if special_turn || pivot_turn {
                kin.facing = -kin.facing;
            }
            // ⭐⭐ THE ACCEPTED DOUBLE-JUMP CANCEL, and this is its commit point
            // for the same reason the turn above is: the move is certain to
            // start here and was only proposed where it was resolved.
            //
            // ⛔⛔ AND THE AXIS IS THE BODY'S OWN RISE, NOT WORLD Y. Under
            // rotated gravity a fighter's up IS world X, and `vel.y` would shed
            // its lateral drift instead. ⛔ It sheds at most what the jump put
            // in and at most what is actually there, so a launch the fighter is
            // riding survives whatever it throws.
            if cancel_air_jump_rise > 0.0 {
                let down = body_frame.down;
                let rise = -kin.vel.dot(down);
                let shed = rise.min(cancel_air_jump_rise).max(0.0);
                kin.vel += shed * down;
            }
            // ⛔⛔ THE DRIFT HALF IS NOT HERE ANY MORE, and that is the
            // recogniser. It used to reverse on EVERY back-special, which is the
            // B-reverse final state applied unconditionally — so one gesture
            // could not choose between turnaround-B, B-reverse and wavebounce.
            // A flick during the window this move just opened is what buys it:
            // see `apply_special_turn_flicks`.
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
            start_move(StartingMove {
                commands: &mut commands,
                entity,
                spec,
                facing: kin.facing,
                aim: control.0.fire.map(|req| (req.dir, req.dir_policy)),
                started_by,
                attack_intent,
                // The SAME stance the selector used — see
                // `MovePlayback::started_grounded`.
                started_grounded: grounded,
                proposer,
                action_buffer: &mut action_buffer,
                shield: guards.get_mut(entity).ok(),
                oos_policy,
                jump: jumps.get_mut(entity).ok(),
                weapon: melee.as_deref_mut().zip(refire_s),
                gesture_window: (special_turn_ticks > 0)
                    .then(|| {
                        gesture_state
                            .as_deref_mut()
                            .map(|g| (g, special_turn_ticks, press_lateral_sign))
                    })
                    .flatten(),
            });
        }
    }
}

/// THE OTHER HALF OF THE SPECIAL TURN: a lateral FLICK inside the window an
/// accepted special opened.
///
/// ⭐⭐ TWO TOGGLES, NOT THREE TECHNIQUES. Each qualifying input flips the
/// facing; a flick AFTER the press also reverses the lateral drift.
///
/// ```text
/// back BEFORE the press       flip                   → turnaround-B
/// back flick AFTER the press  flip + reverse drift   → B-reverse
/// both                        flip twice (= no flip)
///                             + reverse drift        → WAVEBOUNCE
/// ```
///
/// ⛔ SO THE FOURTH OUTCOME FALLS OUT OF THE OTHER TWO. Before this, the drift
/// reversal was applied unconditionally to every back-special — the B-reverse
/// final state, with no way for one gesture to ask for a different one.
///
/// ⛔ THE DRIFT, NOT THE WHOLE VELOCITY: reversing `vel` outright would flip a
/// launch the fighter is riding, which is the mistake three other maneuvers in
/// this kernel have already made. ⛔⛔ AND THE AXIS IS the body's own SIDE — under
/// rotated gravity a fighter's left/right IS world Y, and world X would reverse
/// its RISE instead.
///
/// ⛔ ONCE. The window closes on the flick that spends it, so a stick waggled
/// through a long special turns the fighter one time.
pub fn apply_special_turn_flicks(
    combat_rules: Option<bevy::prelude::Res<crate::rules::ResolvedCombatTuning>>,
    mut bodies: bevy::prelude::Query<(
        &mut AttackGestureState,
        &ActorControl,
        &AttackGestureTuning,
        &mut ae::BodyKinematics,
        Option<&ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame>,
    )>,
) {
    let reverses_drift = combat_rules
        .as_ref()
        .is_some_and(|rules| rules.special_turn_reverses_drift);
    for (mut gesture, control, tuning, mut kin, frame) in &mut bodies {
        let sign =
            ambition_characters::actor::attack_gesture::special_turn_stick_sign(control, tuning);
        let flicked = sign != 0.0 && sign != gesture.prev_lateral_sign;
        gesture.prev_lateral_sign = sign;
        if gesture.special_turn_ticks == 0 {
            continue;
        }
        // ⛔ ONE TICK PER TICK. This window is authored in
        // `AttackGestureTuning::flick_window_ticks` and is now spent in the same
        // unit; it used to age on `WorldTime::sim_dt()`, which hitstop, pause
        // and bullet time all scale, so the player got more chances at the
        // B-reverse the slower the match ran.
        gesture.special_turn_ticks -= 1;
        if !flicked {
            continue;
        }
        gesture.special_turn_ticks = 0;
        kin.facing = -kin.facing;
        if reverses_drift {
            let body_frame = frame
                .map(|frame| frame.basis())
                .unwrap_or_else(|| ae::AccelerationFrame::new(ae::DEFAULT_GRAVITY_DIR));
            let side = body_frame.side;
            let along = kin.vel.dot(side);
            kin.vel -= 2.0 * along * side;
        }
    }
}

/// Republish the DEFENSIVE windows of every body's live move onto the two facts
/// the rest of combat already reads.
///
/// `WindowTag::Invuln` and `WindowTag::Armor` are authoring vocabulary that the
/// runtime consumed nowhere: a move could declare either and the declaration
/// changed nothing. They are answered here, and deliberately not by teaching the
/// damage sites to look at move timelines:
///
/// * intangibility becomes `Invulnerability::MOVE`, one more reason in the
///   bitset [`crate::util::body_vulnerable`] already reads — so the damage
///   resolver honours it and the `unhittable` fact presentation blinks on shows
///   it, with nothing new taught to either;
/// * armor becomes `BodyCombat::armored`, read by `apply_body_hit_reaction`,
///   which already holds that component on both damage roads.
///
/// ⭐ Both are written EVERY tick for every combat body, present move or not.
/// That is what makes the grant retract when the window closes: a projection
/// that only wrote while a move was playing would leave the last move's
/// intangibility latched on a body that is no longer doing anything.
pub fn project_move_defense_windows(
    mut bodies: Query<(
        &mut ambition_characters::actor::BodyCombat,
        Option<&mut ambition_characters::actor::BodyHealth>,
        Option<&MovePlayback>,
    )>,
) {
    for (mut combat, health, playback) in &mut bodies {
        let intangible = playback.is_some_and(MovePlayback::intangible_now);
        let armored = playback.is_some_and(MovePlayback::armored_now);
        // Compared before writing: these run for every combat body every tick,
        // and an unconditional write would mark both components changed forever.
        if combat.armored != armored {
            combat.armored = armored;
        }
        if let Some(mut health) = health {
            if health
                .health
                .invulnerable
                .holds(ambition_characters::actor::Invulnerability::MOVE)
                != intangible
            {
                health.health.invulnerable.set(
                    ambition_characters::actor::Invulnerability::MOVE,
                    intangible,
                );
            }
        }
    }
}

/// CM4: mark the attacker's playing move as CONNECTED from the same resolved
/// body-hit fact on-hit techniques consume.
///
/// Move cancels do not infer it from `HitTarget`, and on-hit effects do not perform their own
/// overlap pass; both consume [`crate::hitbox::LandedBodyHit`]. World/broadcast effects that do not
/// resolve a body victim continue to mark their own outcome at their own resolution seam.
pub fn mark_move_playback_landed_hits(
    mut landed_hits: MessageReader<crate::hitbox::LandedBodyHit>,
    mut playbacks: Query<(&mut MovePlayback, Option<&mut crate::stale::BodyStaleMoves>)>,
) {
    for landed in landed_hits.read() {
        let Ok((mut pb, queue)) = playbacks.get_mut(landed.attacker) else {
            continue;
        };
        // THE FALSE→TRUE EDGE IS ONE MOVE USE CONNECTING, and staling is
        // counted on it rather than on the message.
        //
        // A move that connects with three opponents has been used once.
        //
        // one authority for "this use connected" and one consumer of it: the
        // playback already had to carry the fact for OnHit/OnWhiff cancels, and
        // `MovePlayback` is rollback state, so the edge survives a rewind.
        if !pb.landed_hit {
            pb.landed_hit = true;
            if let Some(mut queue) = queue {
                queue.record(crate::stale::stale_move_hash(&pb.spec.id));
            }
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
/// - `Ranged` → BRIDGE to the shared projectile request seam by writing the SAME
///   `ActorActionMessage::Ranged` the flat `frame.fire` resolver emits, so the
///   mature `spawn_projectiles_from_brain_actions` consumer (body-side
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
        // The weapon THIS MOVE drew, if it drew one, and what is actually in
        // the hand. Two reads because they answer different questions: the
        // brandish says WHOSE the item is, `HeldItem` says WHAT it is — and
        // duplicating the id onto the brandish would be a second authority for
        // a fact one component already owns.
        Option<&crate::held_items::MoveBrandishedItem>,
        Option<&crate::held_items::HeldItem>,
    )>,
    // The move that is playing, for the aim it was STARTED with. See
    // `MovePlayback::aim`.
    playbacks: Query<&MovePlayback>,
    mut sfx: SfxWriter,
    // Presentation's `process_fx_requests` fans one request into the effect + the cue its own
    // name addresses, so a move's author states the picture and gets the sound.
    mut fx_requests: MessageWriter<ambition_vfx::FxRequest>,
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
            MoveEventKind::Vfx {
                effect, scale, sfx, ..
            } => {
                // CM5 per-move cosmetic effect. there is no table here any
                // more: the authored NAME goes on the wire as its hash, and
                // presentation resolves it against the rows the shipped FX
                // sheets actually carry. An id no sheet has is a counted miss
                // at draw time (SFX's policy), never a panic on the RL-hot path.
                let pos = positions
                    .get(ev.owner)
                    .map(|k| k.pos)
                    .unwrap_or(ae::Vec2::ZERO)
                    + ev.world_offset;
                // `FxRequest`'s own doc states the property: the bank ships one
                // `vfx.<family>.<row>` cue per authored row, so *"an emitter that says which
                // effect has already said which sound"*.
                //
                // A fighter's author had to remember a backend detail, and several characters grew
                // a test whose whole job was checking they had.
                //
                // the `.loop` cues are why the OVERRIDE arm is not
                // speculative: a sustained effect wants a looping variant of its
                // own row's sound, which is a real thing to say. An authored
                // `sfx: None` means *say what the art says* — and that is what
                // 74 of those 145 calls were laboriously spelling out.
                let mut request =
                    ambition_vfx::FxRequest::new(pos, ambition_vfx::FxId::new(effect))
                        .with_scale(*scale)
                        .from_source(ev.presentation_source.clone())
                        // the pose derived beside the OFFSET, so the artwork and the place it
                        // lands cannot disagree about facing.
                        .with_pose(ev.world_pose);
                if let Some(cue) = sfx {
                    request = request.with_sfx(SfxId::new(cue));
                }
                fx_requests.write(request);
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
                let Ok((actions_set, control, worn, brandished, held)) =
                    ranged_owners.get(ev.owner)
                else {
                    continue;
                };
                // ⭐⭐ A MOVE THAT DREW A WEAPON FIRES THAT WEAPON. `MoveSpec::equips`
                // put it in the hand at the start of this move, and the shot a
                // player sees leave the barrel has to be the barrel's shot — the
                // admiral's gun-sword is not his pistol, and firing the pistol's
                // numbers out of a drawn gun-sword is the kind of disagreement
                // nobody can see and everybody feels.
                //
                // ⛔ SCOPED TO THE BRANDISH, not to "is anything held". A pirate
                // raider carries a gun-sword all match and its ranged verb is
                // reached the way it always was; only an item a MOVE drew for
                // itself displaces the body's own weapon, and only while that
                // move plays.
                let brandished_ranged = brandished
                    .filter(|brandish| {
                        playbacks
                            .get(ev.owner)
                            .is_ok_and(|pb| pb.spec.id == brandish.move_id)
                    })
                    .and(held)
                    .and_then(|item| item.spec.ranged.clone());
                let Some(spec) = brandished_ranged.or_else(|| actions_set.ranged.clone()) else {
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
                // THE CHARGE BECOMES THE SHOT, at the one place a shot is
                // built. A move that froze its timeline on a held Special
                // released with a fraction, and `at_charge` is what turns that
                // fraction into damage, speed, size and the look a player reads
                // it by. A shot that authors no ladder — every ranged action
                // that existed before charging did — comes back unchanged.
                let spec = match playbacks
                    .get(ev.owner)
                    .ok()
                    .and_then(|pb| pb.charge_fraction())
                {
                    Some(fraction) => spec.at_charge(fraction),
                    None => spec,
                };
                let kin = positions.get(ev.owner).ok();
                let origin = kin.map(|k| k.pos).unwrap_or(ae::Vec2::ZERO);
                // Aim priority: a live fire edge, then the aim captured when the
                // move started, then body-facing in the controlled-body frame.
                // Startup usually clears the initiating edge before the fire frame.
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
                        // ⭐ THE MOVE WAS ACCEPTED, so this shot is owed. The
                        // weapon's recharge was spent at `start_move`; the
                        // consumer must not ask again.
                        commitment: RangedCommitment::CommittedMove,
                        spec,
                        origin,
                        dir,
                        dir_policy,
                    },
                });
            }
            // UNREACHABLE BY CONSTRUCTION, and named rather than swept into
            // a wildcard. An `Impulse` is a velocity write on the owner, so
            // `advance_move_playback` applies it at the authored instant and
            // never publishes it — there is nothing here to resolve. The arm
            // exists so that the NEXT variant somebody adds still has to come
            // past this match and say what it means, which a `_ => {}` would
            // quietly excuse it from.
            MoveEventKind::Impulse { .. } => {}
        }
    }
}

/// Project a `MovesetMelee` body's live [`MovePlayback`] into its [`BodyMelee`] read-model so every
/// existing consumer — the actor anim index, the view/telegraph index, the HUD, the melee
/// integration tests — keeps working unchanged after melee moved onto the moveset. In particular,
/// damage resolution must never consult this projection as an authority gate: the live strike
/// volume is the authority. A body with no live move has its projected swing cleared (its cooldown
/// floors still tick in `tick_body_melee_cooldowns`).
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
/// They stop being the same question the moment a moveset is hand-authored, and
/// a fighting game's move list is named after its MOVES (`jab`, `smash_forward`,
/// `tilt_up`), not after the buttons. Misclassifying one no longer suppresses
/// gameplay — live strike volumes are authoritative — but it still publishes the
/// wrong animation/HUD/telegraph state and can change movement policy that reads
/// "mid-attack" from the projection. Both attack and smash verb families are
/// therefore classified here by their bindings, not by move-id spelling.
fn is_melee_swing_move(moveset: Option<&MovesetContract>, id: &str) -> bool {
    if let Some(verb) = moveset.and_then(|m| verb_for_move(m, id)) {
        return is_melee_verb(verb);
    }
    // the id fallback is deliberately NARROW, and "is there a playback" is the wrong question —
    // a patch that widened `BodyCombat:attacking` to `playback.is_some` was proposed and
    // reverted: it reported a body as attacking while it fired a bolt.
    id == ATTACK_VERB || id.starts_with("attack_") || id == SMASH_VERB || id.starts_with("smash_")
}

/// The read-model direction a resolved gesture asked for.
///
/// ⭐ ONE TABLE, and it is the whole of what the string parser was trying to
/// recover. Posture chooses the aerial family; `running` names the dash attack,
/// which no move id ever spelled.
pub fn attack_intent_of(
    dir: AttackDir,
    posture: ambition_characters::actor::attack_gesture::AttackPosture,
    running: bool,
) -> AttackIntent {
    use ambition_characters::actor::attack_gesture::AttackPosture;
    let airborne = matches!(posture, AttackPosture::Airborne);
    match (dir, airborne) {
        (AttackDir::Up, false) => AttackIntent::Up,
        (AttackDir::Up, true) => AttackIntent::AirUp,
        (AttackDir::Down, false) => AttackIntent::Down,
        (AttackDir::Down, true) => AttackIntent::AirDown,
        (AttackDir::Back, false) => AttackIntent::Back,
        (AttackDir::Back, true) => AttackIntent::AirBack,
        (AttackDir::Neutral, true) | (AttackDir::Forward, true) => AttackIntent::AirForward,
        (AttackDir::Forward, false) if running => AttackIntent::DashForward,
        (AttackDir::Forward, false) => AttackIntent::Forward,
        (AttackDir::Neutral, false) if running => AttackIntent::DashForward,
        (AttackDir::Neutral, false) => AttackIntent::Neutral,
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
        // ⭐⭐ THE DIRECTION THE GESTURE ASKED FOR, carried on the playback since
        // the move started. This used to MATCH THE MOVE ID against a seven-entry
        // canonical vocabulary, and the comment here claimed *"the move's
        // directional variant id carries the swing direction"* — which is simply
        // untrue for shipped content: Pointed authors `polygon_tilt_up`,
        // Pugnacious `polygon_brawler_air_back`, and every borrowed fighter adds
        // another prefix, so all of them fell through to `Forward`. Animation,
        // the HUD and the gizmos read this.
        intent: pb.attack_intent,
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
    swing.hit_targets = pb.hit_targets.clone();
    swing
}

#[cfg(test)]
mod tests;
