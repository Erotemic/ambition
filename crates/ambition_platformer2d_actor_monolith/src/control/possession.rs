//! Possession redirects each participant's seat to a nearby body.
//!
//! The target then uses the ordinary `DrivingParticipant` → [`SlotControls`](ambition_characters::control::SlotControls) →
//! `ActorControl`/`ActionSet` path; the home avatar is inert until release returns
//! the seat. Downstream presentation follows [`ControlledSubject`]. Bosses are
//! valid targets and consume the same driven-seat input through their boss path.

use bevy::prelude::*;

use ambition_characters::brain::Brain;
use ambition_characters::control::ActorControl;
use ambition_characters::control::{DrivingParticipant, PlayerSlot};

use ambition_platformer2d_shared_tangle::sim_id::SimId;

use ambition_combat::components::CenteredAabb;
use ambition_platformer2d_shared_tangle::lifecycle::FeatureSimEntity;
use ambition_platformer2d_shared_tangle::markers::ControlledSubject;
use ambition_platformer2d_shared_tangle::markers::PlayerEntity;

/// Registers possession's seat bookkeeping and the identities it publishes.
pub struct PossessionPlugin;

impl Plugin for PossessionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PossessionState>();
        app.init_resource::<ControlledSubject>();
    }
}

/// One participant's possession and gesture state.
/// Timers and input edges rewind with the possession decision.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Possession {
    pub possessed: Option<Entity>,
    /// Retained until the driving projection returns the seat.
    pub home: Option<Entity>,
    pub hold_timer: f32,
    pub prev_down_interact: bool,
}

impl Possession {
    const IDLE: Self = Self {
        possessed: None,
        home: None,
        hold_timer: 0.0,
        prev_down_interact: false,
    };
}

impl Default for Possession {
    fn default() -> Self {
        Self::IDLE
    }
}

/// Possession records for the active session's participant seats.
#[derive(Resource, Clone, Default)]
pub struct PossessionState {
    seats: std::collections::BTreeMap<PlayerSlot, Possession>,
}

impl PossessionState {
    /// An unwritten seat is idle.
    pub fn get(&self, slot: PlayerSlot) -> &Possession {
        self.seats.get(&slot).unwrap_or(&Possession::IDLE)
    }

    /// Create the seat's record when it first needs one.
    pub fn get_mut(&mut self, slot: PlayerSlot) -> &mut Possession {
        self.seats.entry(slot).or_default()
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = (PlayerSlot, &Possession)> {
        self.seats.iter().map(|(slot, record)| (*slot, record))
    }

    pub(crate) fn iter_mut(&mut self) -> impl Iterator<Item = (PlayerSlot, &mut Possession)> {
        self.seats.iter_mut().map(|(slot, record)| (*slot, record))
    }

    pub fn any_possessed(&self) -> bool {
        self.seats.values().any(|record| record.possessed.is_some())
    }

    pub(crate) fn referenced_entities(&self) -> Vec<Entity> {
        self.seats
            .values()
            .flat_map(|record| record.possessed.into_iter().chain(record.home))
            .collect()
    }

    /// Seat keys and scalar facts complement the ordered entity-reference probe.
    pub(crate) fn facts_checksum(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hash = std::collections::hash_map::DefaultHasher::new();
        for (slot, record) in self.iter() {
            slot.hash(&mut hash);
            record.possessed.is_some().hash(&mut hash);
            record.home.is_some().hash(&mut hash);
            record.hold_timer.to_bits().hash(&mut hash);
            record.prev_down_interact.hash(&mut hash);
        }
        hash.finish()
    }
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

/// Hold Down + Interact to possess a nearby body; press it again to release.
/// Each seat uses its driven body's frame and its own replayed input policy.
/// The hold uses wall time, including during slow motion.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn possession_trigger_system(
    slots: Res<ambition_characters::control::SlotControls>,
    drivers: Query<(Entity, &DrivingParticipant)>,
    frames: Query<&ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame>,
    live: ambition_platformer2d_shared_tangle::lifecycle::LiveRooms,
    world_time: Res<ambition_time::WorldTime>,
    mut states: ResMut<PossessionState>,
    mut commands: Commands,
    mut home_q: Query<
        (
            Entity,
            ambition_platformer2d_core::BodyClusterQueryData,
            &mut ambition_platformer2d_core::movement::MotionModel,
            &SimId,
        ),
        With<PlayerEntity>,
    >,
    candidates: Query<
        (
            Entity,
            &CenteredAabb,
            Option<&ambition_characters::actor::BodyHealth>,
            (
                Has<ambition_combat::death_rules::OutOfPlay>,
                Option<&ambition_platformer2d_core::DepthPlane>,
            ),
            Option<&SimId>,
        ),
        (
            With<FeatureSimEntity>,
            With<ActorControl>,
            With<Brain>,
            Without<PlayerEntity>,
        ),
    >,
    actor_aabbs: Query<&CenteredAabb>,
) {
    let occupied: std::collections::BTreeSet<Entity> = states
        .iter()
        .filter_map(|(_, record)| record.possessed)
        .collect();
    let mut proposals = Vec::new();
    for raw_slot in 0..ambition_characters::control::SlotControls::MAX_SLOTS {
        let slot = PlayerSlot(raw_slot as u8);
        let old = *states.get(slot);
        let driven = crate::control::body_driving_seat(&drivers, slot);
        let home = if old.home.is_some() {
            old.home
        } else {
            driven.filter(|body| home_q.get(*body).is_ok())
        };
        let Some(home) = home else {
            if old != Possession::default() {
                *states.get_mut(slot) = Possession::default();
            }
            continue;
        };
        let down = driven
            .and_then(|body| frames.get(body).ok())
            .map_or(ambition_platformer2d_core::DEFAULT_GRAVITY_DIR, |frame| {
                frame.down()
            });
        let control = slots.get(slot);
        let down_interact = control.interact_held
            && holding_descend(
                control.axis_x,
                control.axis_y,
                down,
                control.control_frame_modes.movement,
            );
        let record = states.get_mut(slot);
        let release_edge = down_interact && !record.prev_down_interact;
        record.prev_down_interact = down_interact;
        if let Some(target) = record.possessed {
            record.hold_timer = 0.0;
            if release_edge {
                release_possession(&mut commands, record, target, &actor_aabbs, &mut home_q);
            }
            continue;
        }
        if !down_interact {
            record.hold_timer = 0.0;
            continue;
        }
        record.hold_timer += world_time.wall_dt();
        if record.hold_timer < POSSESS_HOLD_S {
            continue;
        }
        record.hold_timer = 0.0;
        let Ok((_, clusters, _, home_id)) = home_q.get(home) else {
            continue;
        };
        let at = clusters.kinematics.pos;
        let room = live.of(home);
        let nearest = ambition_platformer2d_shared_tangle::sim_selection::winner_by(
            candidates
                .iter()
                .filter(|(body, _, health, (out, plane), _)| {
                    live.of(*body) == room
                        && !occupied.contains(body)
                        && drivers.get(*body).is_err()
                        && !ambition_combat::util::body_is_untouchable(*health, *out, *plane)
                })
                .map(|(body, bounds, _, _, id)| (body, (bounds.center - at).length(), id))
                .filter(|(_, distance, _)| *distance <= POSSESS_RADIUS),
            |(_, distance, _)| *distance,
            |(_, _, id)| *id,
        );
        if let Some((target, _, _)) = nearest {
            proposals.push((slot, home, target, home_id.clone()));
        }
    }
    // Refuse contested targets before any body or control claim is changed.
    for (slot, home, target, home_id) in &proposals {
        if proposals
            .iter()
            .filter(|(_, _, other, _)| other == target)
            .count()
            != 1
        {
            continue;
        }
        let record = states.get_mut(*slot);
        record.home = Some(*home);
        record.possessed = Some(*target);
        commands.entity(*home).insert(ActorControl::default());
        commands.entity(*target).insert(ActorControl::default());
        ambition_platformer2d_shared_tangle::temporary_control::file_claim(
            &mut commands,
            *target,
            ambition_platformer2d_shared_tangle::temporary_control::ControlClaimant::Possession,
            home_id.clone(),
        );
    }
}

/// Release one claim and place the home body at the target's position.
/// Keep the home record until the driving projection returns the seat.
fn release_possession(
    commands: &mut Commands,
    state: &mut Possession,
    target: Entity,
    actor_aabbs: &Query<&CenteredAabb>,
    home_q: &mut Query<
        (
            Entity,
            ambition_platformer2d_core::BodyClusterQueryData,
            &mut ambition_platformer2d_core::movement::MotionModel,
            &SimId,
        ),
        With<PlayerEntity>,
    >,
) {
    state.possessed = None;

    if let Ok(mut entity) = commands.get_entity(target) {
        entity.insert(ActorControl::default());
    }
    // The mount claim can remain after the possession claim is released.
    ambition_platformer2d_shared_tangle::temporary_control::drop_claim(
        commands,
        target,
        ambition_platformer2d_shared_tangle::temporary_control::ControlClaimant::Possession,
    );

    // Clear the home body's control edges before the seat returns.
    if let Some(home) = state.home {
        if let Ok(mut ec) = commands.get_entity(home) {
            ec.insert(ActorControl::default());
        }
        if let (Ok(aabb), Ok((_, mut cluster_item, mut motion_model, _))) =
            (actor_aabbs.get(target), home_q.get_mut(home))
        {
            // The release is a discrete transit that arrives at rest (ADR 0024).
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

/// Release a possession when either body is removed.
pub fn release_possession_if_target_lost(
    mut states: ResMut<PossessionState>,
    still_present: Query<()>,
    mut commands: Commands,
) {
    for (_, record) in states.iter_mut() {
        let Some(target) = record.possessed else {
            continue;
        };
        if still_present.get(target).is_ok()
            && record
                .home
                .is_some_and(|home| still_present.get(home).is_ok())
        {
            continue;
        }
        ambition_platformer2d_shared_tangle::temporary_control::drop_claim(
            &mut commands,
            target,
            ambition_platformer2d_shared_tangle::temporary_control::ControlClaimant::Possession,
        );
        if let Ok(mut entity) = commands.get_entity(target) {
            entity.insert(ActorControl::default());
        }
        record.possessed = None;
    }
}

#[cfg(test)]
mod tests;

impl bevy::ecs::entity::MapEntities for PossessionState {
    fn map_entities<M: bevy::ecs::entity::EntityMapper>(&mut self, mapper: &mut M) {
        for (_, record) in self.iter_mut() {
            if let Some(entity) = record.possessed.as_mut() {
                *entity = mapper.get_mapped(*entity);
            }
            if let Some(entity) = record.home.as_mut() {
                *entity = mapper.get_mapped(*entity);
            }
        }
    }
}
