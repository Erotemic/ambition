//! Possession redirects the primary participant's seat to a nearby actor.
//!
//! The target then uses the ordinary `DrivingParticipant` → [`SlotControls`] →
//! `ActorControl`/`ActionSet` path; the home avatar is inert until release returns
//! the seat. Downstream presentation follows [`ControlledSubject`]. Bosses are
//! valid targets and consume the same driven-seat input through their boss path.

use bevy::prelude::*;

use ambition_characters::brain::Brain;
use ambition_characters::control::ActorControl;
use ambition_characters::control::{DrivingParticipant, PlayerSlot};

use ambition_platformer2d_shared_tangle::markers::ControlledSubject;
use ambition_platformer2d_shared_tangle::sim_id::SimId;
use ambition_platformer2d_shared_tangle::temporary_control::TemporaryControl;

use crate::actor::PlayerEntity;
use crate::features::{CenteredAabb, FeatureSimEntity};

/// Internal seat bookkeeping for possession.
///
/// Gameplay and presentation should derive the live driver from
/// [`ControlledSubject`], not branch on this resource.
#[derive(Resource, Clone, Default)]
pub struct PossessionState {
    /// The actor currently possessed (the primary seat is redirected here), or
    /// `None` while driving the home avatar.
    pub possessed: Option<Entity>,
    /// Home avatar to receive the seat on release. The driving-seat projection
    /// consumes and clears this value when it returns the seat.
    pub home: Option<Entity>,
    /// How long Down+Interact has been held toward the possess threshold.
    ///
    /// Lives HERE rather than in a `Local<f32>` on the trigger system because
    /// this resource is registered rollback state and a `Local` is not: GGRS
    /// cannot save or restore per-system state, so a rewind would rewind the
    /// possession decision while leaving the charge that produced it at its
    /// predicted value.
    pub hold_timer: f32,
    /// Previous frame's Down+Interact, for rising-edge release detection. Same
    /// reasoning as `hold_timer`: edge state must rewind with the decision.
    pub prev_down_interact: bool,
}

/// Publish the entity currently holding the primary participant's seat.
///
/// Normal play has exactly one holder; load/transition frames may briefly have
/// none. Command application may delay a possess/release change by one frame.
pub fn resolve_controlled_subject(
    drivers: Query<(Entity, &DrivingParticipant)>,
    mut subject: ResMut<ControlledSubject>,
) {
    // Resolve through the shared seat-holder policy so duplicate holders are
    // handled consistently across gameplay and presentation.
    let chosen = crate::control::body_driving_seat(&drivers, PlayerSlot::PRIMARY);
    // Write only on an actual change of subject: an unconditional store marks
    // the resource changed every frame, which defeats change detection for
    // every downstream consumer (the control-prompt rebuild gates on it).
    if subject.0 != chosen {
        subject.0 = chosen;
    }
}

/// Possession reach (px): Down+Interact possesses the nearest candidate within this.
const POSSESS_RADIUS: f32 = 150.0;

/// Seconds the player must hold Down+Interact (with a candidate in range) to
/// commit a possession. A deliberate gesture so you don't possess by brushing
/// the button mid-fight; releasing fully is instant (a single press).
const POSSESS_HOLD_S: f32 = 2.0;

/// Stick deflection (gravity-resolved "down") past which the player counts as
/// holding Down for the possession gesture — the same threshold drop-through
/// uses.
pub const POSSESS_DOWN_THRESHOLD: f32 = 0.35;

/// True iff the player's stick is held "down" in the GRAVITY-resolved frame past
/// [`POSSESS_DOWN_THRESHOLD`]. The possession gesture is Down + Interact;
/// exposed so the interaction system can SUPPRESS a normal interact while Down is
/// held — i.e. Down+Interact is *claimed* by possession and never opens a door /
/// NPC. Sharing it keeps both systems agreeing on what "down" means under any
/// gravity orientation.
pub fn holding_descend(
    axis_x: f32,
    axis_y: f32,
    gravity_dir: ambition_platformer2d_core::Vec2,
    movement_mode: ambition_platformer2d_core::InputFrameMode,
) -> bool {
    ambition_platformer2d_core::AccelerationFrame::new(gravity_dir)
        .resolve_input(
            movement_mode,
            ambition_platformer2d_core::ScreenAxes::new(axis_x, axis_y),
        )
        .y
        > POSSESS_DOWN_THRESHOLD
}

/// True iff the stick is held "up" in the GRAVITY-resolved frame past
/// [`POSSESS_DOWN_THRESHOLD`]. Held Up is an alternative interact (a hands-free
/// way into a door); it shares this module's threshold and resolution so up and
/// down mean opposite things under any gravity rather than merely similar ones.
pub fn holding_ascend(
    axis_x: f32,
    axis_y: f32,
    gravity_dir: ambition_platformer2d_core::Vec2,
    movement_mode: ambition_platformer2d_core::InputFrameMode,
) -> bool {
    ambition_platformer2d_core::AccelerationFrame::new(gravity_dir)
        .resolve_input(
            movement_mode,
            ambition_platformer2d_core::ScreenAxes::new(axis_x, axis_y),
        )
        .y
        < -POSSESS_DOWN_THRESHOLD
}

/// Hold gravity-relative Down + Interact to possess the nearest candidate; press
/// it again to release. The hold uses real time so bullet-time does not change its duration.
///
/// Possession is currently primary-seat gameplay policy, so it reads
/// `SlotControls[PRIMARY]`. This keeps release input available while the home avatar is vacated.
#[allow(clippy::too_many_arguments)]
pub fn possession_trigger_system(
    slots: Res<ambition_characters::control::SlotControls>,
    controlled: Option<Res<ambition_platformer2d_shared_tangle::markers::ControlledSubject>>,
    frames: Query<&ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame>,
    user_settings: Option<Res<ambition_persistence::settings::UserSettings>>,
    world_time: Res<ambition_time::WorldTime>,
    // The possession DECISION — who is driven and where the seat returns to. The
    // seat itself is written by `crate::control::project_driving_participant`,
    // which reads this.
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
            ambition_platformer2d_core::BodyClusterQueryData,
            &mut crate::features::MotionModel,
        ),
        crate::actor::PrimaryPlayerOnly,
    >,
    // Possession candidates: any brain-bearing feature body — INCLUDING bosses.
    // Bosses are valid controllable bodies (their tick reads whichever slot drives
    // the body), so there is no `Without<BossConfig>` barrier here. Restricting WHICH boss is
    // possessable (progression/design) is a targeting-policy gate to add above
    // this, not a "bosses can never be controlled" exclusion in the body model.
    candidates: Query<
        (
            Entity,
            &CenteredAabb,
            Option<&ambition_characters::actor::BodyHealth>,
        ),
        (
            With<FeatureSimEntity>,
            With<ActorControl>,
            With<Brain>,
            Without<PlayerEntity>,
        ),
    >,
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
        ambition_platformer2d_core::InputFrameMode::DEFAULT_MOVEMENT,
        |s| s.gameplay.resolved_movement_frame_mode(),
    );
    // Possession is currently primary-seat gameplay policy.
    let control = slots.get(ambition_characters::control::PlayerSlot::PRIMARY);
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
        // Structural tangibility gate: a dead body is an
        // intangible corpse — you cannot possess a corpse. Excluded BEFORE
        // distance selection so a nearer corpse never shadows a farther live body.
        .filter(|(_, _, health)| !ambition_combat::util::body_is_corpse(*health))
        .map(|(entity, aabb, _)| (entity, (aabb.center - home_pos).length()))
        .filter(|(_, dist)| *dist <= POSSESS_RADIUS)
        .min_by(|a, b| a.1.total_cmp(&b.1));
    let Some((target, _)) = nearest else {
        return;
    };

    // THE SEAT MOVES; NOTHING ELSE DOES. Record the decision — who is
    // driving and where the seat goes back to — and let
    // `crate::control::project_driving_participant` be the one system that acts
    // on it. Both bodies get a fresh neutral `ActorControl` so no stale
    // edge-triggered intent (a held jump, a pressed attack) leaks across the
    // handover.
    //
    // Its `ActorFaction` is likewise left alone — effective allegiance (a DRIVEN body fights as
    // Player) handles its player-side combat.
    state.home = Some(home_entity);
    state.possessed = Some(target);

    commands.entity(home_entity).insert(ActorControl::default());
    commands
        .entity(target)
        .insert(ActorControl::default())
        // Record the possession by stable id so a snapshot restores the control
        // MODE across a rewind (the home avatar is always the primary player).
        .insert(TemporaryControl::Player {
            controller: SimId::player_slot(0),
        });
    // Body custody is derived from rollback-authoritative `PossessionState` by
    // `project_body_custody`; do not write `InCustodyOf` at the possession site.
}

/// End the possession and step the home body out to the vacated actor's
/// position so the camera does not snap back.
///
/// The target never lost its policy and the home avatar never lost anything but its seat, so all
/// that is left is: clear the decision (the seat goes home in
/// `crate::control::project_driving_participant`), clear the stale control edges, and put the body
/// where the player expects it.
///
///  `state.home` is deliberately NOT cleared here. The seat still has to go
/// somewhere, and the writer that hands it back is the one that consumes the
/// record — see [`PossessionState::home`].
fn release_possession(
    commands: &mut Commands,
    state: &mut PossessionState,
    target: Entity,
    actor_aabbs: &Query<&CenteredAabb>,
    // SLOT-0 BY DESIGN: the home avatar (see `possession_trigger_system`).
    home_q: &mut Query<
        (
            Entity,
            ambition_platformer2d_core::BodyClusterQueryData,
            &mut crate::features::MotionModel,
        ),
        crate::actor::PrimaryPlayerOnly,
    >,
) {
    state.possessed = None;

    // Clear the stale edges on the body being let go and hand its temporary-control record back to
    // `Autonomous`.
    //
    //  RELEASE TOUCHES NO SCOPE, because possession touched none. Residency resumes in
    // whatever room is active NOW — `RoomScopedEntity` carries no room id, so a body released
    // two rooms later is resident THERE and the next transition out retires it correctly.
    if let Ok(mut ec) = commands.get_entity(target) {
        ec.insert(ActorControl::default())
            .insert(TemporaryControl::Autonomous);
    }

    // The home avatar sheds its stale edges and vacate-exits to the actor's spot.
    if let Some(home) = state.home {
        if let Ok(mut ec) = commands.get_entity(home) {
            ec.insert(ActorControl::default());
        }
        if let (Ok(aabb), Ok((_, mut cluster_item, mut motion_model))) =
            (actor_aabbs.get(target), home_q.get_mut(home))
        {
            // THE discrete-transit authority: the vacate-exit is a scripted
            // teleport arriving at rest (ADR 0024 authority model).
            let mut clusters = cluster_item.as_clusters_mut();
            ambition_platformer2d_core::movement::transit_body(
                &mut motion_model,
                &mut clusters,
                aabb.center,
                ambition_platformer2d_core::movement::TransitVelocity::Zero,
            );
        }
    }
}

/// If the possessed actor is gone (despawned / removed), end the possession so
/// the player isn't stranded driving nothing.
pub fn release_possession_if_target_lost(
    mut state: ResMut<PossessionState>,
    still_present: Query<()>,
) {
    let Some(target) = state.possessed else {
        return;
    };
    if still_present.get(target).is_ok() {
        return;
    }
    // Target vanished mid-possession.
    state.possessed = None;
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
