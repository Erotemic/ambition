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

use ambition_characters::brain::{Brain, PlayerSlot, SlotControls};
use ambition_input::ControlFrame;
use ambition_platformer2d_actor_monolith::actor::{BodyKinematics, PlayerEntity, PrimaryPlayer};
use ambition_platformer2d_actor_monolith::affordances::{InteractVariant, NearestInteractable};
use ambition_platformer2d_shared_tangle::markers::ControlledSubject;
use ambition_portal2d::{
    DropPortalGun, FirePortalGun, PickUpPortalGun, PortalGun, TogglePortalGun,
};
#[cfg(feature = "portal_render")]
use ambition_portal2d_presentation::PortalAimHint;

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
    // The controlled body: its brain (→ slot), position, held gun (if any), and
    // its brain-resolved control frame — which this system CONSUMES the Attack
    // press from when the gun answers it. See the note at the fire branch.
    mut holders: Query<(
        &Brain,
        &BodyKinematics,
        Option<&PortalGun>,
        &mut ambition_characters::brain::ActorControl,
    )>,
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
    let Ok((brain, kin, gun, mut actor_control)) = holders.get_mut(subject) else {
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
            actor_control.0.melee_pressed = false;
        } else if holding_gun {
            // Plain Attack while holding the gun fires it.
            fire.write(FirePortalGun {
                aim: pick_aim(control, kin.facing),
            });
            // ⭐⭐ **AND THE PRESS IS SPENT HERE** (queue D60). A weapon in hand
            // owns the Attack press, and `trigger_moveset_moves` arbitrates that
            // from `HeldItem` — which the portal gun is not, and must not become
            // (187 references; its own component is the right shape). So the
            // arbiter cannot see this gun, and the wearer's jab answered the very
            // same press: two mechanisms, one button, exactly D51's bug.
            //
            // ⛔ **consumed here rather than special-cased in the arbiter.** A
            // third branch in `trigger_moveset_moves` naming `PortalGun` would
            // add a path to the one place whose entire job is having a single
            // one. Marking the press spent where it is spent is the mechanism
            // the pickup and the throw already use, and it crosses the phase
            // boundary for free: this runs in `PlayerSimulation`, the trigger
            // looks in `Combat`.
            //
            // ⚠ **no verb is revoked, and that is deliberate.** D51's lesson is
            // that taking the wearer's `attack` away also takes the on-screen
            // Attack button, because `touch_action_available` draws it only
            // while the scheme carries an Attack label. The gun stays tappable
            // on a phone because the slot is untouched.
            actor_control.0.melee_pressed = false;
        } else {
            // Plain Attack while NOT holding the gun is a pickup attempt
            // (consumed only if overlapping an armed pickup).
            //
            // ⚠ **NOT consumed here.** The grant path clears the press itself
            // when it actually picks something up (`items::pickup`), and a press
            // that grabs nothing must still reach the wearer's jab — swinging at
            // empty air is the correct answer to "Attack while holding nothing".
            pickup.write(PickUpPortalGun);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_characters::brain::ActorControl;

    /// A body driving the gun, with the Attack press already on its slot AND on
    /// its brain-resolved frame — which is how a real press arrives: the slot
    /// carries the device gesture and `ActorControl` carries what the body will
    /// act on.
    fn app_with_holder(holding: bool) -> (App, Entity) {
        let mut app = App::new();
        app.add_message::<FirePortalGun>();
        app.add_message::<TogglePortalGun>();
        app.add_message::<DropPortalGun>();
        app.add_message::<PickUpPortalGun>();

        let mut slots = SlotControls::default();
        let mut frame = ambition_platformer2d_core::ControlFrame::default();
        frame.attack_pressed = true;
        slots.set(PlayerSlot::PRIMARY, frame);
        app.insert_resource(slots);

        let mut control = ActorControl::default();
        control.0.melee_pressed = true;
        let mut body = app.world_mut().spawn((
            Brain::Player(PlayerSlot::PRIMARY),
            BodyKinematics::default(),
            control,
        ));
        if holding {
            body.insert(PortalGun::default());
        }
        let body = body.id();
        app.insert_resource(ControlledSubject(Some(body)));
        app.add_systems(Update, portal_input_adapter_system);
        (app, body)
    }

    fn melee_still_pressed(app: &App, body: Entity) -> bool {
        app.world()
            .get::<ActorControl>(body)
            .unwrap()
            .0
            .melee_pressed
    }

    /// ⭐⭐ **the gun answers the press, so the jab must not** (queue D60).
    ///
    /// `trigger_moveset_moves` arbitrates the Attack press from `HeldItem`, and
    /// the portal gun is its own component — so the arbiter cannot see it and
    /// the wearer's jab answered the same press. Two mechanisms, one button:
    /// D51's bug in a second place.
    #[test]
    fn firing_the_gun_spends_the_attack_press() {
        let (mut app, body) = app_with_holder(true);
        app.update();
        assert_eq!(
            app.world()
                .resource::<Messages<FirePortalGun>>()
                .iter_current_update_messages()
                .count(),
            1,
            "holding the gun, Attack fires it"
        );
        assert!(
            !melee_still_pressed(&app, body),
            "⛔ the press survived the shot, so the wearer's jab answers it too"
        );
    }

    /// The poison, and it is the case that must NOT change. Attack while holding
    /// nothing is a pickup attempt; if it grabs nothing, the press belongs to the
    /// wearer's jab. Swinging at empty air is the correct answer, and consuming
    /// the press here would silently delete the unarmed attack.
    #[test]
    fn attacking_with_no_gun_leaves_the_press_for_the_body() {
        let (mut app, body) = app_with_holder(false);
        app.update();
        assert_eq!(
            app.world()
                .resource::<Messages<PickUpPortalGun>>()
                .iter_current_update_messages()
                .count(),
            1,
            "with no gun, Attack attempts a pickup"
        );
        assert!(
            melee_still_pressed(&app, body),
            "⛔ a pickup attempt that grabs nothing must leave the jab its press"
        );
    }
}
