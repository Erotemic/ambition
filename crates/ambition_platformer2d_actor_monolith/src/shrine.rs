//! Healing / save-point shrine.
//!
//! An interactable shrine that, on a single `Interact`, heals the player to
//! full (health + mana) and acts as a save point (decided: one Interact
//! does both).
//!
//! The autosave compares values, so the marker wrote nothing; and there was no checkpoint field
//! to write into even if it had. Both halves matter: a checkpoint nothing records is a lie, and
//! a checkpoint nothing restores is a number in a file.
//!
//! ⭐ THIS FILE OWNS THE FIRST HALF ONLY, since A1b. Coming back to a checkpoint
//! — the startup resume and the death/retry reset — is session lifecycle and
//! lives in [`crate::session::checkpoint`]. What a shrine owns is the
//! interaction: heal the body that touched me, and ask for a checkpoint to be
//! committed.
//!
//! [`PersistedCheckpoint`]: ambition_persistence::save_data::PersistedCheckpoint
//!
//! Handoff / not-yet-built:
//! - placement is LDtk-authored (`ShrineSpawn`); routing the heal/save through
//!   the affordance/prompt system via an `Interactable` is the follow-up (see
//!   TODO "Healing / save-point shrine").

use ambition_platformer2d_shared_tangle::shrine::ShrineActivationPulse;
use bevy::prelude::*;

use ambition_characters::actor::BodyHealth;
use ambition_characters::control::ActorControl;
use ambition_platformer2d_core::BodyKinematics;
use ambition_platformer2d_core::BodyMana;
use ambition_platformer2d_core::{self as ae, AabbExt};

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
/// Acts on the controlled subject — the body the player is driving — reading
/// its body-generic [`ActorControl`] interact intent (populated for any body
/// holding the primary seat) and healing THAT body. So a possessed actor resting
/// at a shrine heals itself, not the vacated home avatar. The intent belongs to
/// the body at the shrine, not to one machine-wide input frame (relativity
/// principle / §4 of the restructuring blueprint). Falls back to the primary
/// player for the startup frame before the subject resolver has run.
pub fn heal_save_shrine_system(
    // ⭐ EVERY DRIVEN BODY HEALS. A shrine heals the body that touched it, and
    // "the body that touched it" was one entity by construction — so a couch's
    // second seat could stand in the shrine and press interact forever.
    driven: ambition_held_items::DrivenBodies,
    mut bodies: Query<(
        &ActorControl,
        &BodyKinematics,
        &mut BodyHealth,
        &mut BodyMana,
    )>,
    // ⚠ THE STARTUP-FRAME FALLBACK SUBJECT, and nothing else. Before a seat is
    // attached there is no driven body at all, and the primary avatar is the
    // subject every single-player fixture expects.
    //
    // ⛔ It is NOT the checkpoint's owner. That comment lived here and the code
    // never followed it — see the checkpoint write below for which rule is real.
    primary: Query<Entity, ambition_platformer2d_shared_tangle::markers::PrimaryPlayerOnly>,
    shrines: Query<&HealShrine>,
    // WHICH room the checkpoint is in. A position with no room is not a
    // checkpoint — it is a pair of numbers that will one day be applied in the
    // wrong place. Optional so narrow fixtures without a room set still heal.
    room_set: Option<
        ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<
            ambition_platformer2d_world::rooms::RoomSet,
        >,
    >,
    mut save: ResMut<ambition_persistence::save::AmbitionGameSave>,
    // THE INSTANT, which is the half a `PersistedCheckpoint` cannot carry.
    // That value says WHERE the body comes back; this says WHEN the rest of the
    // world was last agreed, and every domain that has reset-relevant state
    // snapshots itself off it. `Option` so a narrow fixture with no horizon
    // installed still heals and still writes its checkpoint.
    mut committed: Option<
        MessageWriter<ambition_platformer2d_shared_tangle::lifecycle::CheckpointCommitted>,
    >,
    mut activation: ResMut<ShrineActivationPulse>,
    mut sfx: ambition_sfx::SfxWriter,
) {
    // ⚠ THE FALLBACK IS THE STARTUP FRAME and nothing else.
    let mut subjects = driven.entities();
    if subjects.is_empty() {
        subjects.extend(primary.single().ok());
    }
    // ⛔⛔ THE CHECKPOINT IS ONE FACT AND THE HEAL IS N. Two seats resting on the
    // same tick heal two bodies and must not write two checkpoints — the second
    // would silently overwrite the first. It is written by the FIRST body in
    // `driven.entities()`' rewind-stable order that actually rests, so the value
    // does not depend on query order.
    //
    // ⚠ WHICH BODY SHOULD OWN IT IS AN OPEN QUESTION, and this preserves today's
    // answer rather than deciding it: the comment below has long claimed the
    // checkpoint is "the PRIMARY player's session, not the possessed subject's
    // body" while the code has always written the RESTING body's position — so a
    // checkpoint taken while possessing resumes the primary avatar somewhere it
    // never stood. See D-SHRINE-CHECKPOINT-OWNER in `docs/planning/queue.md`;
    // changing it is a save-compatibility ruling, not a side effect of a
    // multi-seat conversion.
    let mut checkpoint_written = false;
    for subject in subjects {
        let Ok((control, kin, mut health, mut mana)) = bodies.get_mut(subject) else {
            continue;
        };
        if !control.0.interact_pressed {
            continue;
        }
        let player_aabb = ae::Aabb::new(kin.pos, kin.size * 0.5);
        let touching = shrines
            .iter()
            .any(|s| player_aabb.strict_intersects(ae::Aabb::new(s.pos, s.half_extent)));
        if !touching {
            continue;
        }
        health.reset(); // health to full
        mana.meter.refill_full(); // mana to full
        if checkpoint_written {
            // Healed, and the session already has its checkpoint for this tick.
            continue;
        }
        checkpoint_written = true;

        // ⛔⛔ **THE REST REVIVES WHAT RESTED-AWAY DEATH RECORDED, and until now
        // nothing did.** An `OnRest` placement's death writes
        // `enemy_<id>_dead_until_rest`, and `AmbitionGameSave::clear_dead_until_
        // rest_flags` exists to drop those. Measured 2026-09-09 by `git grep`:
        // that function's only occurrence in the whole workspace was its own
        // definition. Nothing called it. So the flag was written and never
        // cleared, and `OnRest` behaved exactly like `DeadStaysDead` — an
        // authored policy that reads as a mechanic and was a synonym, across the
        // shipped placements that use it.
        //
        // ⭐ HERE, AND NOT ON `CheckpointCommitted`. That message is "whatever a
        // game decides a checkpoint is" — a shrine, a flag, a room entry, an
        // AUTOSAVE — and reviving a corpse because the game autosaved is not the
        // mechanic. Resting is a deliberate act, and this is the seam where the
        // deliberate act happens.
        //
        // ⚠ INSIDE THE `checkpoint_written` GUARD, so it is one fact per tick
        // like the checkpoint beside it: two seats resting on the same tick must
        // not clear the flags twice, and the count below would be a lie about the
        // second.
        let revived = save.data_mut().clear_dead_until_rest_flags();
        if revived > 0 {
            bevy::log::info!(
                target: "ambition_platformer2d::shrine",
                "shrine: rest revived {revived} body/bodies whose policy is OnRest"
            );
        }

        // THE CHECKPOINT.
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
            if save.data().checkpoint() != Some(&checkpoint) {
                save.data_mut().set_checkpoint(checkpoint);
            }
        }
        // RAISED UNCONDITIONALLY, and NOT inside the change guard above.
        // Resting twice at the same shrine writes the same position, so that guard
        // is right about the FILE and would be badly wrong about the horizon: the
        // second rest is a real checkpoint at which the player may be carrying
        // something they were not carrying at the first. Suppressing it would leave
        // the baseline describing the earlier visit, and a later death would take
        // back an object the player had legitimately banked.
        //
        // raised even when there is no room set, for the same reason the heal
        // above is: a composition with no rooms still has hands.
        if let Some(committed) = committed.as_mut() {
            committed.write(ambition_platformer2d_shared_tangle::lifecycle::CheckpointCommitted);
        }
        activation.remaining = 0.78;
        sfx.write(ambition_sfx::SfxMessage::Play {
            id: ambition_sfx::ids::WORLD_HEALTH_COLLECT,
            pos: kin.pos,
        });
        bevy::log::info!(
            target: "ambition_platformer2d::shrine",
            "shrine: healed to full + checkpoint recorded"
        );
    }
}

#[cfg(test)]
mod tests;
