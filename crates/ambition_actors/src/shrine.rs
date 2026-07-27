//! Healing / save-point shrine.
//!
//! An interactable shrine that, on a single `Interact`, **heals the player to
//! full** (health + mana) and acts as a **save point** (decided: one Interact
//! does both).
//!
//! The save point is two systems, and it was neither of them until 2026-07-27.
//! [`heal_save_shrine_system`] records a [`PersistedCheckpoint`] — room id plus
//! position — into `SandboxSave`, which the value-comparing autosave then commits
//! to disk. [`restore_checkpoint_on_session_start`] puts the body back there when
//! a session opens in that room.
//!
//! What was here before was `save.set_changed()` on a value the shrine never
//! modified, plus a log line claiming it had saved. The autosave compares values,
//! so the marker wrote nothing; and there was no checkpoint field to write into
//! even if it had (GPT 5.6 review, 2026-07-27). Both halves matter: a checkpoint
//! nothing records is a lie, and a checkpoint nothing restores is a number in a
//! file.
//!
//! [`PersistedCheckpoint`]: ambition_persistence::save_data::PersistedCheckpoint
//!
//! Handoff / not-yet-built:
//! - placement is LDtk-authored (`ShrineSpawn`); routing the heal/save through
//!   the affordance/prompt system via an `Interactable` is the follow-up (see
//!   TODO "Healing / save-point shrine").

use bevy::prelude::*;

use crate::actor::BodyKinematics;
use crate::actor::BodyMana;
use ambition_characters::actor::BodyHealth;
use ambition_characters::brain::ActorControl;
use ambition_engine_core::{self as ae, AabbExt};
use ambition_platformer_primitives::markers::ControlledSubject;

/// A healing / save-point shrine the player can `Interact` with.
#[derive(Component, Clone, Copy, Debug)]
pub struct HealShrine {
    pub pos: Vec2,
    pub half_extent: Vec2,
}

// The heal/save shrine is now an LDtk-authored `ShrineSpawn` entity (spawned at
// room load through the installed placement-lowering registry); the old debug
// spawner is retired.

/// `Interact` while overlapping a [`HealShrine`] heals the body to full
/// (health + mana) and writes a save checkpoint. `interact_pressed` is an edge,
/// so one press = one heal.
///
/// Acts on the **controlled subject** — the body the player is driving — reading
/// its body-generic [`ActorControl`] interact intent (populated for any body
/// carrying `Brain::Player`) and healing THAT body. So a possessed actor resting
/// at a shrine heals itself, not the vacated home avatar. The intent belongs to
/// the body at the shrine, not to one machine-wide input frame (relativity
/// principle / §4 of the restructuring blueprint). Falls back to the primary
/// player for the startup frame before the subject resolver has run.
pub fn heal_save_shrine_system(
    controlled: Option<Res<ControlledSubject>>,
    mut bodies: Query<(
        &ActorControl,
        &BodyKinematics,
        &mut BodyHealth,
        &mut BodyMana,
    )>,
    // SLOT-0 BY DESIGN: a shrine heals the body that touched it (via
    // `ControlledSubject`, above) but ALSO writes a CHECKPOINT to the save. The
    // checkpoint is a session fact owned by the local player, not by whatever body
    // slot 0 happens to be driving — hence the second, primary-scoped query.
    primary: Query<Entity, crate::actor::PrimaryPlayerOnly>,
    shrines: Query<&HealShrine>,
    // WHICH room the checkpoint is in. A position with no room is not a
    // checkpoint — it is a pair of numbers that will one day be applied in the
    // wrong place. Optional so narrow fixtures without a room set still heal.
    room_set: Option<
        ambition_platformer_primitives::lifecycle::SessionWorldRef<crate::rooms::RoomSet>,
    >,
    mut save: ResMut<ambition_persistence::save::SandboxSave>,
    mut activation: ResMut<ShrineActivationPulse>,
    mut sfx: ambition_sfx::SfxWriter,
) {
    let Some(subject) = controlled
        .and_then(|subject| subject.0)
        .or_else(|| primary.single().ok())
    else {
        return;
    };
    let Ok((control, kin, mut health, mut mana)) = bodies.get_mut(subject) else {
        return;
    };
    if !control.0.interact_pressed {
        return;
    }
    let player_aabb = ae::Aabb::new(kin.pos, kin.size * 0.5);
    let touching = shrines
        .iter()
        .any(|s| player_aabb.strict_intersects(ae::Aabb::new(s.pos, s.half_extent)));
    if !touching {
        return;
    }
    health.reset(); // health to full
    mana.meter.refill_full(); // mana to full

    // THE CHECKPOINT. This used to be `save.set_changed()` and nothing else — a
    // marker on a value the shrine never modified, which the value-comparing
    // autosave correctly ignores, so resting at a "save point" wrote nothing to
    // disk while the log line said it had (GPT 5.6, 2026-07-27).
    //
    // Written for the PRIMARY player's session, not the possessed subject's body:
    // the checkpoint is where this player resumes, and a possessed actor's
    // position is not where the player will be standing next session. The heal
    // above is the subject's; the checkpoint is the session's.
    if let Some(room_set) = room_set.as_deref() {
        let checkpoint = ambition_persistence::save_data::PersistedCheckpoint::new(
            room_set.active_spec().id.clone(),
            kin.pos.x.round() as i32,
            kin.pos.y.round() as i32,
        );
        // Assign only on a real change, so resting twice at the same shrine does
        // not churn the file.
        if save.data().checkpoint.as_ref() != Some(&checkpoint) {
            save.data_mut().checkpoint = Some(checkpoint);
        }
    }
    activation.remaining = 0.78;
    sfx.write(ambition_sfx::SfxMessage::Play {
        id: ambition_sfx::ids::WORLD_HEALTH_COLLECT,
        pos: kin.pos,
    });
    bevy::log::info!(
        target: "ambition::shrine",
        "shrine: healed to full + checkpoint recorded"
    );
}

/// **Resume where the player last rested.**
///
/// A checkpoint you cannot return to is a number in a file. This is the other
/// half: once per constructed session, if the save names a checkpoint in the room
/// that was just built, the primary body starts THERE instead of at the room's
/// authored spawn.
///
/// Deliberately a separate system rather than a branch inside session setup.
/// Construction has one job and already has more parameters than it should; a
/// post-construction placement is additive, is testable on its own, and cannot
/// make the authored-spawn path behave differently for every existing test.
///
/// Movement goes through [`ae::movement::transit_body`] — the ONE transit
/// authority (ADR 0024) — so arrival is at rest with contacts and attachment
/// reconciled, not a raw position write that leaves the body believing it is
/// still standing on the floor it left.
///
/// Runs once per session: `applied_for` remembers which session generation it has
/// already placed, so a later room transition does not yank the player back to the
/// shrine they woke up at.
pub fn restore_checkpoint_on_session_start(
    save: Res<ambition_persistence::save::SandboxSave>,
    room_set: Option<
        ambition_platformer_primitives::lifecycle::SessionWorldRef<crate::rooms::RoomSet>,
    >,
    scope: Option<Res<ambition_platformer_primitives::lifecycle::ActiveSessionScope>>,
    mut transitions: MessageWriter<ambition_world::rooms::RoomTransitionRequested>,
    mut bodies: Query<
        (ae::BodyClusterQueryData, &mut crate::features::MotionModel),
        crate::actor::PrimaryPlayerOnly,
    >,
    mut applied_for: Local<Option<Option<u64>>>,
    mut routed_for: Local<Option<Option<u64>>>,
) {
    let Some(room_set) = room_set.as_deref() else {
        return;
    };
    let generation = scope.and_then(|scope| scope.current()).map(|id| id.0);
    if *applied_for == Some(generation) {
        return;
    }
    let Some(checkpoint) = save.data().checkpoint.as_ref() else {
        // Nothing to resume. Mark the session handled so this stops looking.
        *applied_for = Some(generation);
        return;
    };

    // ROUTE FIRST. The checkpoint's room is where the player rested, so it is
    // where the session belongs — and until 2026-07-27 nothing acted on that.
    // The room id was only COMPARED against whatever room the session happened
    // to open, and a mismatch returned: rest in B, quit, start a session that
    // opens in A, and the checkpoint was silently ignored. Worse, the handled
    // latch was set BEFORE the comparison, so walking into B later in the same
    // session did not apply it either (GPT 5.6, 2026-07-27).
    //
    // Requesting an ordinary transition rather than repointing the room set:
    // staging a room is a transaction with content, geometry and authorization
    // in it, and "the one place rooms are staged" is worth more than saving a
    // message.
    if checkpoint.room_id != room_set.active_spec().id {
        // Once per session. A transition takes several frames to commit, and
        // re-requesting every frame would restart it forever.
        if *routed_for == Some(generation) {
            return;
        }
        let Some(target_room) = room_set
            .rooms
            .iter()
            .position(|room| room.id == checkpoint.room_id)
        else {
            // The checkpoint names a room this world does not have — a save from
            // another game, or a room that was removed. Not fatal and not
            // silent: the session keeps its own starting room.
            bevy::log::warn!(
                target: "ambition::shrine",
                "checkpoint names room `{}`, which this world does not contain; \
                 starting at the session's own room instead",
                checkpoint.room_id
            );
            *applied_for = Some(generation);
            return;
        };
        *routed_for = Some(generation);
        transitions.write(ambition_world::rooms::RoomTransitionRequested::new(
            ambition_world::rooms::RoomTransition {
                // A synthetic zone: the resume is not a door anybody walked
                // through, and `Door` is the activation that never fires on its
                // own, so this cannot be re-triggered by proximity.
                zone: ambition_world::rooms::LoadingZone {
                    id: "checkpoint_resume".to_string(),
                    name: "Checkpoint".to_string(),
                    activation: ambition_world::rooms::LoadingZoneActivation::Door,
                    aabb: ae::Aabb::new(
                        ae::Vec2::new(checkpoint.x as f32, checkpoint.y as f32),
                        ae::Vec2::ONE,
                    ),
                },
                target_room,
                arrival: ae::Vec2::new(checkpoint.x as f32, checkpoint.y as f32),
            },
            None,
        ));
        return;
    }

    let Ok((clusters, mut model)) = bodies.single_mut() else {
        // No body yet — construction has not finished. Leave `applied_for`
        // untouched so the next tick tries again, rather than marking a session
        // handled that was never placed.
        return;
    };
    *applied_for = Some(generation);
    let mut item = clusters;
    let mut clusters = item.as_clusters_mut();
    ae::movement::transit_body(
        &mut model,
        &mut clusters,
        ae::Vec2::new(checkpoint.x as f32, checkpoint.y as f32),
        ae::movement::TransitVelocity::Zero,
    );
    bevy::log::info!(
        target: "ambition::shrine",
        "resumed at the checkpoint in `{}` ({}, {})",
        checkpoint.room_id, checkpoint.x, checkpoint.y
    );
}

pub use ambition_platformer_primitives::shrine::ShrineActivationPulse;

#[cfg(test)]
mod tests;
