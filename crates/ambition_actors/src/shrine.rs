//! Healing / save-point shrine.
//!
//! An interactable shrine that, on a single `Interact`, **heals the player to
//! full** (health + mana) and acts as a **save point** (decided: one Interact
//! does both). The save is a checkpoint write: touching `Res<SandboxSave>` marks
//! it changed, and the existing `autosave_sandbox_save` persists it (desktop;
//! no-op on wasm).
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
// room load via `spawn_room_feature_entities`); the old debug spawner is retired.

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
                              // Save checkpoint: mark the live save changed so `autosave_sandbox_save`
                              // persists the current state to disk.
    save.set_changed();
    activation.remaining = 0.78;
    sfx.write(ambition_sfx::SfxMessage::Play {
        id: ambition_sfx::ids::WORLD_HEALTH_COLLECT,
        pos: kin.pos,
    });
    bevy::log::info!(target: "ambition::shrine", "shrine: healed to full + saved");
}

pub use ambition_platformer_primitives::shrine::ShrineActivationPulse;

#[cfg(test)]
mod tests;
