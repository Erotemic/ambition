//! Possession — Down + Interact transfers the player's controller brain onto a
//! nearby actor. Bosses are valid targets: the boss tick consumes
//! `Brain::Player`, driving boss movement AND its authored specials via a
//! deterministic input mapping over `BossCapability`.
//!
//! Possession is NOT input-copying. It is **brain transfer**. On possess we move
//! [`Brain::Player`]`(PlayerSlot::PRIMARY)` off the home avatar and onto the
//! target actor. The target then reads slot-0 input through the SAME
//! universal-brain path every player-controlled body uses:
//! `Brain::Player` → [`SlotControls`] → its own `ActorControl` → its own
//! `ActionSet`. It moves, attacks, and fires through its own body path — no
//! `Possessed` marker, no input mirror, no possession-specific override in the
//! actor tick.
//!
//! The home avatar, now without a player brain, is inert (a neutral
//! `ActorControl`, no local attack authority) until release restores its brain.
//!
//! Everything downstream — camera, portal viewer, nameplates, the melee
//! lifecycle — derives from [`ControlledSubject`], i.e. "who carries
//! `Brain::Player(PRIMARY)` this frame", never from a possession flag. That is
//! the whole point: possession is proof that control is actor-generic.
//!
//! Bosses are in scope: the boss tick (`crate::features::ecs::bosses::tick`)
//! handles a `Brain::Player` boss — reading slot input for movement and mapping
//! attack/special input onto its authored `BossCapability`. Restricting WHICH
//! boss is possessable (progression / design) is a targeting-policy gate to add
//! above this trigger, not a "bosses can never be controlled" barrier.

use bevy::prelude::*;

use ambition_characters::brain::{ActorControl, Brain, PlayerSlot};

use crate::features::TemporaryControl;
use ambition_platformer_primitives::markers::ControlledSubject;
use ambition_platformer_primitives::sim_id::SimId;

use crate::actor::PlayerEntity;
use crate::features::{CenteredAabb, FeatureSimEntity};

/// Brain-transfer bookkeeping for possession.
///
/// `controlled == None` means the local player drives the home avatar;
/// `Some(actor)` means slot-0's brain has been transferred to `actor`. The
/// remaining fields remember what to restore on release. This resource is
/// possession-INTERNAL: no gameplay/presentation system branches on it. Ask
/// [`ControlledSubject`] instead.
#[derive(Resource, Clone, Default)]
pub struct PossessionState {
    /// The actor currently possessed (its `Brain::Player(PRIMARY)` was
    /// transferred here), or `None` while driving the home avatar.
    pub possessed: Option<Entity>,
    /// The home avatar whose player brain was vacated, restored on release.
    pub home: Option<Entity>,
    /// The possessed actor's brain before transfer, restored on release.
    pub restore_brain: Option<Brain>,
    /// How long Down+Interact has been held toward the possess threshold.
    ///
    /// Lives HERE rather than in a `Local<f32>` on the trigger system because
    /// this resource is registered rollback state and a `Local` is not: GGRS
    /// cannot save or restore per-system state, so a rewind would rewind the
    /// possession decision while leaving the charge that produced it at its
    /// predicted value (deep review 2026-07-19 §2.4).
    pub hold_timer: f32,
    /// Previous frame's Down+Interact, for rising-edge release detection. Same
    /// reasoning as `hold_timer`: edge state must rewind with the decision.
    pub prev_down_interact: bool,
}

/// Derive [`ControlledSubject`] from the ECS: the entity carrying
/// `Brain::Player(PRIMARY)`. Runs early each frame; there is exactly one such
/// entity during normal play (the home avatar, or the possessed actor while
/// possessing). A one-frame lag across a possess/release transition (commands
/// apply at a later sync point) is benign — no consumer double-acts, because
/// each body only emits actions for itself and only when it carries the brain.
pub fn resolve_controlled_subject(
    brains: Query<(Entity, &Brain)>,
    mut subject: ResMut<ControlledSubject>,
) {
    // HARD INVARIANT: exactly one entity carries `Brain::Player(PRIMARY)` during
    // normal play (zero only during a load/transition frame). Two is a bug the
    // whole architecture rests on NOT happening — a stale home brain that wasn't
    // vacated, or a double-assigned slot. Surface it loudly instead of silently
    // picking one and diverging.
    let mut chosen = None;
    let mut count = 0u32;
    for (entity, brain) in &brains {
        if brain.player_slot() == Some(PlayerSlot::PRIMARY) {
            count += 1;
            if chosen.is_none() {
                chosen = Some(entity);
            }
        }
    }
    debug_assert!(
        count <= 1,
        "control invariant violated: {count} entities carry Brain::Player(PRIMARY) \
         (expected exactly one); possession/vacate left a stale player brain"
    );
    if count > 1 {
        bevy::log::error!(
            "control invariant: {count} entities carry Brain::Player(PRIMARY); \
             using the first as the controlled subject"
        );
    }
    subject.0 = chosen;
}

/// Possession reach (px): Down+Interact possesses the nearest candidate within this.
const POSSESS_RADIUS: f32 = 150.0;

/// Seconds the player must **hold** Down+Interact (with a candidate in range) to
/// commit a possession. A deliberate gesture so you don't possess by brushing
/// the button mid-fight; releasing fully is instant (a single press).
const POSSESS_HOLD_S: f32 = 2.0;

/// Stick deflection (gravity-resolved "down") past which the player counts as
/// holding **Down** for the possession gesture — the same threshold drop-through
/// uses.
pub const POSSESS_DOWN_THRESHOLD: f32 = 0.35;

/// True iff the player's stick is held "down" in the GRAVITY-resolved frame past
/// [`POSSESS_DOWN_THRESHOLD`]. The possession gesture is **Down + Interact**;
/// exposed so the interaction system can SUPPRESS a normal interact while Down is
/// held — i.e. Down+Interact is *claimed* by possession and never opens a door /
/// NPC. Sharing it keeps both systems agreeing on what "down" means under any
/// gravity orientation.
pub fn holding_descend(
    axis_x: f32,
    axis_y: f32,
    gravity_dir: ambition_engine_core::Vec2,
    movement_mode: ambition_engine_core::InputFrameMode,
) -> bool {
    ambition_engine_core::AccelerationFrame::new(gravity_dir)
        .resolve_input(
            movement_mode,
            ambition_engine_core::ScreenAxes::new(axis_x, axis_y),
        )
        .y
        > POSSESS_DOWN_THRESHOLD
}

/// `Down + Interact` controls possession: **hold ~2s** (with a candidate in
/// range) to transfer your controller brain onto the nearest non-boss actor;
/// press it again to release. `Down` is the gravity-resolved descend axis past
/// [`POSSESS_DOWN_THRESHOLD`]. The hold runs on real time (`raw_dt`) so
/// bullet-time doesn't change the feel.
///
/// The gesture belongs to slot 0, so it reads the local device frame
/// (`Res<ControlFrame>`) directly rather than any body's input — the home avatar
/// is inert (neutral input) while vacated, but the local device still drives the
/// release.
///
/// **This is the ONE sim system that holds the global `ControlFrame`, which makes
/// possession local-player-only: a second player could never possess anything.**
/// It is enumerated as the sole `Bridge::Slot0Gesture` in
/// `ambition_runtime/tests/control_frame_lint.rs`, whose allowlist doubles as the
/// N1 multiplayer checklist. The fix is to read the acting slot's
/// `SlotInteractionState` / `SlotControls`, exactly as `interaction_input_system`
/// already does for the interact buffer — a behavior change, not a refactor, so
/// it is deferred rather than hidden.
#[allow(clippy::too_many_arguments)]
pub fn possession_trigger_system(
    control: Res<ambition_input::ControlFrame>,
    controlled: Option<Res<ambition_platformer_primitives::markers::ControlledSubject>>,
    frames: Query<&crate::physics::ResolvedMotionFrame>,
    user_settings: Option<Res<ambition_persistence::settings::UserSettings>>,
    world_time: Res<ambition_time::WorldTime>,
    mut state: ResMut<PossessionState>,
    mut commands: Commands,
    // Home avatar kinematics: its position seeds the candidate search, and on
    // release it steps out to the vacated actor's spot (camera continuity).
    // SLOT-0 BY DESIGN: the HOME AVATAR is a real concept — the body slot 0 owns and
    // returns to on release. It is precisely the body that is NOT the controlled
    // subject while possession is active, so it cannot be found any other way.
    mut home_q: Query<
        (
            Entity,
            ambition_engine_core::BodyClusterQueryData,
            &mut crate::features::MotionModel,
        ),
        crate::actor::PrimaryPlayerOnly,
    >,
    // Possession candidates: any brain-driven feature body — INCLUDING bosses.
    // Bosses are valid controllable bodies (their tick consumes `Brain::Player`),
    // so there is no `Without<BossConfig>` barrier here. Restricting WHICH boss is
    // possessable (progression/design) is a targeting-policy gate to add above
    // this, not a "bosses can never be controlled" exclusion in the body model.
    candidates: Query<
        (Entity, &CenteredAabb),
        (
            With<FeatureSimEntity>,
            With<ActorControl>,
            With<Brain>,
            Without<PlayerEntity>,
        ),
    >,
    // The target's authored brain, snapshotted for restore on release. Its
    // faction is NOT touched — effective allegiance (`Brain::Player` ⇒ combat
    // treats it as Player) makes the possessed body fight its former allies
    // without mutating `ActorFaction`.
    target_data: Query<&Brain>,
    // Read-only AABB lookup for the vacate exit on release.
    actor_aabbs: Query<&CenteredAabb>,
) {
    // The CONTROLLED body's resolved frame decides what "down" means for the
    // gesture — while possessing, that is the possessed body's frame.
    let gravity_dir = crate::control::controlled_frame_down(
        controlled.as_deref(),
        home_q.single().map(|(entity, _, _)| entity).ok(),
        &frames,
    );
    let movement_mode = user_settings.as_deref().map_or(
        ambition_engine_core::InputFrameMode::DEFAULT_MOVEMENT,
        |s| s.gameplay.movement_frame_mode,
    );
    let down = holding_descend(control.axis_x, control.axis_y, gravity_dir, movement_mode);
    // The gesture is a HOLD, so it accumulates on the interact button being
    // HELD — not the single-frame `interact_pressed` edge (which doors / the
    // heal-shrine also consume, resetting the hold every frame). The release is
    // the rising edge of (down + held), tracked via `prev_down_interact`.
    let down_interact = down && control.interact_held;
    let release_edge = down_interact && !state.prev_down_interact;
    state.prev_down_interact = down_interact;

    // Already possessing → a fresh Down+Interact press releases (no hold).
    if let Some(target) = state.possessed {
        state.hold_timer = 0.0;
        if release_edge {
            release_possession(&mut commands, &mut state, target, &actor_aabbs, &mut home_q);
        }
        return;
    }

    // Not possessing → accumulate the hold; commit at the threshold.
    if !down_interact {
        state.hold_timer = 0.0;
        return;
    }
    state.hold_timer += world_time.raw_dt;
    if state.hold_timer < POSSESS_HOLD_S {
        return;
    }
    state.hold_timer = 0.0;

    let Ok((home_entity, home_clusters, _)) = home_q.single() else {
        return;
    };
    let home_pos = home_clusters.kinematics.pos;
    let nearest = candidates
        .iter()
        .map(|(entity, aabb)| (entity, (aabb.center - home_pos).length()))
        .filter(|(_, dist)| *dist <= POSSESS_RADIUS)
        .min_by(|a, b| a.1.total_cmp(&b.1));
    let Some((target, _)) = nearest else {
        return;
    };
    let Ok(target_brain) = target_data.get(target) else {
        return;
    };

    // BRAIN TRANSFER. Remember the target's brain to restore, then move the
    // player brain from the home avatar to the target. Both bodies get a fresh
    // neutral `ActorControl` so no stale edge-triggered intent (a held jump, a
    // pressed attack) leaks across the handover. The target's `ActorFaction` is
    // left untouched — effective allegiance handles its player-side combat.
    state.home = Some(home_entity);
    state.restore_brain = Some(target_brain.clone());
    state.possessed = Some(target);

    commands
        .entity(home_entity)
        .remove::<Brain>()
        .insert(ActorControl::default());
    commands
        .entity(target)
        .insert(Brain::Player(PlayerSlot::PRIMARY))
        .insert(ActorControl::default())
        // Record the possession by stable id so a snapshot restores the control
        // MODE across a rewind (the home avatar is always the primary player).
        .insert(TemporaryControl::Player {
            controller: SimId::player_slot(0),
        });
}

/// Restore the home avatar's player brain and the target's authored brain +
/// faction, then step the home body out to the vacated actor's position so the
/// camera (which was following the actor) doesn't snap back.
fn release_possession(
    commands: &mut Commands,
    state: &mut PossessionState,
    target: Entity,
    actor_aabbs: &Query<&CenteredAabb>,
    // SLOT-0 BY DESIGN: the home avatar (see `possession_trigger_system`).
    home_q: &mut Query<
        (
            Entity,
            ambition_engine_core::BodyClusterQueryData,
            &mut crate::features::MotionModel,
        ),
        crate::actor::PrimaryPlayerOnly,
    >,
) {
    state.possessed = None;

    // Restore the actor's authored brain, clearing stale edges. Its faction was
    // never touched (effective allegiance), so there is nothing to restore. The
    // cached `restore_brain` is kept in sync with the actor's autonomous SOURCE
    // (refreshed by `BrainCommand` if it switched during possession), so releasing
    // resumes the CURRENT selected source. Its temporary-control record returns to
    // `Autonomous`.
    if let Some(brain) = state.restore_brain.take() {
        if let Ok(mut ec) = commands.get_entity(target) {
            ec.insert(brain)
                .insert(ActorControl::default())
                .insert(TemporaryControl::Autonomous);
        }
    }

    // Restore the home avatar's player brain and vacate-exit to the actor's spot.
    if let Some(home) = state.home.take() {
        if let Ok(mut ec) = commands.get_entity(home) {
            ec.insert(Brain::Player(PlayerSlot::PRIMARY))
                .insert(ActorControl::default());
        }
        if let (Ok(aabb), Ok((_, mut cluster_item, mut motion_model))) =
            (actor_aabbs.get(target), home_q.get_mut(home))
        {
            // THE discrete-transit authority: the vacate-exit is a scripted
            // teleport arriving at rest (ADR 0024 authority model).
            let mut clusters = cluster_item.as_clusters_mut();
            ambition_engine_core::movement::transit_body(
                &mut motion_model,
                &mut clusters,
                aabb.center,
                ambition_engine_core::movement::TransitVelocity::Zero,
            );
        }
    }
}

/// If the possessed actor is gone (despawned / removed), hand control back to
/// the home avatar so the player isn't stranded driving nothing. The actor's
/// brain can't be restored (it's gone); only the home brain is re-attached.
pub fn release_possession_if_target_lost(
    mut state: ResMut<PossessionState>,
    mut commands: Commands,
    still_present: Query<(), With<Brain>>,
) {
    let Some(target) = state.possessed else {
        return;
    };
    if still_present.get(target).is_ok() {
        return;
    }
    // Target vanished mid-possession.
    if let Some(home) = state.home.take() {
        if let Ok(mut ec) = commands.get_entity(home) {
            ec.insert(Brain::Player(PlayerSlot::PRIMARY))
                .insert(ActorControl::default());
        }
    }
    state.possessed = None;
    state.restore_brain = None;
}

#[cfg(test)]
mod tests;

impl bevy::ecs::entity::MapEntities for PossessionState {
    fn map_entities<M: bevy::ecs::entity::EntityMapper>(&mut self, mapper: &mut M) {
        if let Some(entity) = self.possessed.as_mut() {
            *entity = mapper.get_mapped(*entity);
        }
        if let Some(entity) = self.home.as_mut() {
            *entity = mapper.get_mapped(*entity);
        }
    }
}
