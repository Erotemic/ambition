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

use crate::input::ControlFrame;
use crate::player::affordances::{InteractVariant, NearestInteractable};
use crate::player::{PlayerEntity, PlayerKinematics, PrimaryPlayer};
use crate::portal::{DropPortalGun, FirePortalGun, PickUpPortalGun, PortalGun, TogglePortalGun};

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

/// Translate this frame's `ControlFrame` into portal-gun intents for the primary
/// player. Runs in the portal weapon set so the intents are visible to the core
/// fire/toggle/pickup/drop systems the same frame.
#[allow(clippy::too_many_arguments)]
pub fn portal_input_adapter_system(
    control: Res<ControlFrame>,
    nearest: Option<Res<NearestInteractable>>,
    players: Query<
        (&PlayerKinematics, Option<&PortalGun>),
        (With<PlayerEntity>, With<PrimaryPlayer>),
    >,
    mut fire: MessageWriter<FirePortalGun>,
    mut toggle: MessageWriter<TogglePortalGun>,
    mut drop: MessageWriter<DropPortalGun>,
    mut pickup: MessageWriter<PickUpPortalGun>,
) {
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

    let Ok((kin, gun)) = players.single() else {
        return;
    };
    let holding_gun = gun.is_some();

    if control.attack_pressed {
        if control.shield_held {
            // Shield+Attack is the drop gesture (held-gun only; core/inventory
            // adapter no-ops if not holding).
            drop.write(DropPortalGun);
        } else if holding_gun {
            // Plain Attack while holding the gun fires it.
            fire.write(FirePortalGun {
                aim: pick_aim(&control, kin.facing),
            });
        } else {
            // Plain Attack while NOT holding the gun is a pickup attempt
            // (consumed only if overlapping an armed pickup).
            pickup.write(PickUpPortalGun);
        }
    }
}
