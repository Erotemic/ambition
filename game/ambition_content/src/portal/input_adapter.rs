//! Ambition input binding for the portal gun.
//!
//! Translates the Ambition [`ControlFrame`] (and the nearest-interactable HUD
//! state) into the reusable portal intent messages
//! ([`FirePortalGun`] / [`TogglePortalGun`] / [`DropPortalGun`] /
//! [`PickUpPortalGun`]). Portal core consumes only those intents, so it never
//! reads `ControlFrame` for the gun's gestures — a replay or a different input
//! layer can drive the gun by emitting the same messages.
//!
//! Gesture ownership lives here:
//! - `Attack` (no Shield) → fire, with the aim resolved from right-stick / move
//!   axis / facing;
//! - `Shield + Attack` → drop;
//! - `Attack` while not holding the gun → pickup attempt;
//! - `Interact` (when no door / NPC claims it) → color toggle.

use bevy::prelude::*;

use ambition_actors::actor::{BodyKinematics, PlayerEntity, PrimaryPlayer};
use ambition_actors::player::affordances::{InteractVariant, NearestInteractable};
use ambition_characters::brain::{Brain, PlayerSlot, SlotControls};
use ambition_input::ControlFrame;
use ambition_platformer_primitives::markers::ControlledSubject;
use ambition_portal::{DropPortalGun, FirePortalGun, PickUpPortalGun, PortalGun, TogglePortalGun};
#[cfg(feature = "portal_render")]
use ambition_portal_presentation::PortalAimHint;

/// Aim direction for a fired portal: right-stick aim, else movement axis, else
/// straight ahead along facing. (Moved out of portal core so the core fire
/// system consumes a resolved `FirePortalGun { aim }` instead of reading the
/// control frame.)
pub fn pick_aim(control: &ControlFrame, facing: f32) -> Vec2 {
    let aim = Vec2::new(control.aim_x, control.aim_y);
    if aim.length() > 0.2 {
        return aim;
    }
    let mv = Vec2::new(control.axis_x, control.axis_y);
    if mv.length() > 0.2 {
        return mv;
    }
    Vec2::new(if facing >= 0.0 { 1.0 } else { -1.0 }, 0.0)
}

/// Translate this frame's controller input into portal-gun intents for the body
/// the local player is DRIVING (the controlled subject — home avatar or possessed
/// actor). The gun is a `PortalGun` held BY that body, so gestures come from the
/// controlled body's slot and the aim/holder from its own kinematics: possess an
/// actor holding the gun and it fires from that actor, not the vacated home avatar.
/// Runs in the portal weapon set so the intents are visible to the core
/// fire/toggle/pickup/drop systems the same frame.
#[allow(clippy::too_many_arguments)]
pub fn portal_input_adapter_system(
    nearest: Option<Res<NearestInteractable>>,
    controlled: Option<Res<ControlledSubject>>,
    // The controller's slot frame (the sanctioned per-slot input source).
    slots: Res<SlotControls>,
    // The controlled body: its brain (→ slot), position, and held gun (if any).
    holders: Query<(&Brain, &BodyKinematics, Option<&PortalGun>)>,
    primary_fallback: Query<Entity, (With<PlayerEntity>, With<PrimaryPlayer>)>,
    #[cfg(feature = "portal_render")] mut aim_hint: Option<ResMut<PortalAimHint>>,
    mut fire: MessageWriter<FirePortalGun>,
    mut toggle: MessageWriter<TogglePortalGun>,
    mut drop: MessageWriter<DropPortalGun>,
    mut pickup: MessageWriter<PickUpPortalGun>,
) {
    let Some(subject) = controlled
        .and_then(|subject| subject.0)
        .or_else(|| primary_fallback.single().ok())
    else {
        return;
    };
    let Ok((brain, kin, gun)) = holders.get(subject) else {
        return;
    };
    let slot = brain.player_slot().unwrap_or(PlayerSlot::PRIMARY);
    let control = slots.get(slot);
    let control = &control;
    // Color toggle: Interact, but only when no genuine interactable (door / NPC /
    // switch) claims the press — matching the HUD label.
    if control.interact_pressed {
        let claimed = nearest
            .as_deref()
            .is_some_and(|n| !matches!(n.0, InteractVariant::None));
        if !claimed {
            toggle.write(TogglePortalGun);
        }
    }
    // Publish the resolved aim for the visible-build held-gun presentation
    // (`sync_portal_mode_indicator`), so portal presentation reads this hint
    // instead of `ControlFrame`. Render-only: the `PortalAimHint` resource exists
    // exclusively behind `portal_render`.
    #[cfg(feature = "portal_render")]
    if let Some(aim_hint) = aim_hint.as_deref_mut() {
        aim_hint.aim = pick_aim(control, kin.facing);
    }
    let holding_gun = gun.is_some();

    if control.attack_pressed {
        if control.shield_held {
            // Shield+Attack is the drop gesture (held-gun only; core/inventory
            // adapter no-ops if not holding).
            drop.write(DropPortalGun);
        } else if holding_gun {
            // Plain Attack while holding the gun fires it.
            fire.write(FirePortalGun {
                aim: pick_aim(control, kin.facing),
            });
        } else {
            // Plain Attack while NOT holding the gun is a pickup attempt
            // (consumed only if overlapping an armed pickup).
            pickup.write(PickUpPortalGun);
        }
    }
}
