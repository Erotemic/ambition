//! Ambition input binding for the portal gun.
//!
//! Translates the Ambition [`ControlFrame`] (and the nearest-interactable HUD
//! state) into the reusable portal intent messages
//! ([`FirePortalGun`] / [`TogglePortalGun`] / [`DropPortalGun`] /
//! [`PickUpPortalGun`]). Portal core consumes only those intents and never
//! reads `ControlFrame` for the gun's gestures, so a replay or another input
//! layer can drive the gun by emitting the same messages.
//!
//! Gesture ownership lives here:
//! - `Attack` (no Shield) → fire, with the aim resolved from right-stick / move
//!   axis / facing;
//! - `Shield + Attack` → drop;
//! - `Attack` while not holding the gun → pickup attempt;
//! - `Interact` (when no door / NPC claims it) → color toggle.

use bevy::prelude::*;

use ambition_characters::control::{DrivingParticipant, PlayerSlot, SlotControls};
use ambition_platformer2d_core::{BodyKinematics, ControlFrame};
use ambition_platformer2d_shared_tangle::markers::ControlledSubject;
use ambition_portal2d::{
    DropPortalGun, FirePortalGun, PickUpPortalGun, PortalGun, TogglePortalGun,
};
#[cfg(feature = "portal_render")]
use ambition_portal2d_presentation::PortalAimHint;
use ambition_sim_view::affordances::{InteractVariant, NearestInteractable};

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

/// Translate this frame's controller input into portal-gun intents for each
/// driven body. The gun is a `PortalGun` held by that body, so gestures come
/// from the body's slot and aim and holder from its own kinematics: possess an
/// actor holding the gun and it fires from that actor, not the vacated home
/// avatar. Runs in the portal weapon set so the core fire, toggle, pickup and
/// drop systems see the intents in the same frame.
#[allow(clippy::too_many_arguments)]
pub fn portal_input_adapter_system(
    nearest: Option<Res<NearestInteractable>>,
    controlled: Option<Res<ControlledSubject>>,
    // The controller's slot frame (the sanctioned per-slot input source).
    slots: Res<SlotControls>,
    // The controlled body: its brain (→ slot), position and held gun (if any).
    //
    // A producer cannot know whether its action will be accepted (the drop is
    // refused for a body holding a throwable, the fire for an inactive gun), so
    // it does not spend the press. See the drop branch below.
    holders: Query<(
        Option<&DrivingParticipant>,
        &BodyKinematics,
        Option<&PortalGun>,
    )>,
    // The seated half of the driven population: the union `DrivenBodies`
    // names, written out here because that `SystemParam` is the actor crate's.
    driven_seats: Query<
        (
            Entity,
            Option<&ambition_platformer2d_shared_tangle::sim_id::SimId>,
        ),
        With<DrivingParticipant>,
    >,
    #[cfg(feature = "portal_render")] mut aim_hint: Option<ResMut<PortalAimHint>>,
    mut fire: MessageWriter<FirePortalGun>,
    mut toggle: MessageWriter<TogglePortalGun>,
    mut drop: MessageWriter<DropPortalGun>,
    mut pickup: MessageWriter<PickUpPortalGun>,
) {
    // Every driven body makes its own gestures, and each gesture names its
    // body, so a couch's second seat can fire, toggle, drop and pick up its own
    // gun.

    let mut subjects: Vec<Entity> = Vec::new();
    // Held separately, because the held-gun presentation is not per-body. See
    // the `PortalAimHint` write below.
    let presented_subject = controlled.as_deref().and_then(|held| held.0);
    if let Some(subject) = presented_subject {
        subjects.push(subject);
    }
    // Ordered by stable identity, never by query order: a resimulation must
    // produce these gestures in the same sequence (ADR 0023).
    let mut seated: Vec<(Option<String>, Entity)> = driven_seats
        .iter()
        .map(|(entity, sim)| (sim.map(|id| id.as_str().to_string()), entity))
        .collect();
    seated.sort();
    for (_, entity) in seated {
        if !subjects.contains(&entity) {
            subjects.push(entity);
        }
    }
    // The body whose gun is drawn: the controlled subject, else the first seat in
    // stable order. Never "whichever seat the loop visited last".
    // Gated to match its only reader, which is
    // `#[cfg(feature = "portal_render")]`. Without the gate the crate's default
    // build warns, which the workspace build never shows because feature
    // unification enables `portal_render`. An underscore name would hide the
    // warning but keep an unread computation.
    #[cfg(feature = "portal_render")]
    let presented_subject = presented_subject.or_else(|| subjects.first().copied());
    for subject in subjects {
        let Ok((driver, kin, gun)) = holders.get(subject) else {
            continue;
        };
        let slot = driver.map_or(PlayerSlot::PRIMARY, |driver| driver.0);
        let control = slots.get(slot);
        let control = &control;
        // Color toggle: Interact, but only when no genuine interactable (door / NPC /
        // switch) claims the press — matching the HUD label.
        if control.interact_pressed {
            // This body's reach, not seat zero's. The singleton
            // `NearestInteractable.0` answers for one controlled subject, so inside
            // this loop it would let seat zero's position suppress or allow seat one's
            // toggle.
            //
            // It is still a prediction: this adapter runs in `PlayerSimulation`, and
            // the interaction road spends the press later, in `FeatureInteraction`.
            // Both use the same `strict_intersects` reach, which keeps the prediction
            // right.
            let claimed = nearest
                .as_deref()
                .is_some_and(|n| !matches!(n.for_body(subject), InteractVariant::None));
            if !claimed {
                toggle.write(TogglePortalGun { body: subject });
            }
        }
        // Publish the resolved aim for the visible-build held-gun presentation
        // (`sync_portal_mode_indicator`), so presentation reads this hint, not
        // `ControlFrame`. Render-only: `PortalAimHint` exists only behind
        // `portal_render`.
        //
        // The controlled subject's aim only, inside a loop over every driven body.
        // `PortalAimHint` is a singleton, and the gun it describes is drawn for one
        // body: the `PortalAffordanceBody` from `ControlledSubject`, read back with
        // `carriers.single()`. Writing it per seat would let the last seat win.
        //
        // The gameplay above is per-body; this line is presentation, which has one
        // viewer. If every seat gets a drawn gun, make this per-body state keyed by
        // `PortalAffordanceBody`, not a singleton written N times.
        #[cfg(feature = "portal_render")]
        if Some(subject) == presented_subject {
            if let Some(aim_hint) = aim_hint.as_deref_mut() {
                aim_hint.aim = pick_aim(control, kin.facing);
            }
        }
        let holding_gun = gun.is_some();

        if control.attack_pressed {
            if control.shield_held {
                // Shield+Attack is the drop gesture: an intent, not a claim on the press.
                //
                // The press is spent where the action commits. Otherwise a body holding a
                // laser sword would lose the press here, the drop would refuse, and
                // `throw_held_item_system` (whose Shield+Attack throw is correct) would see
                // `melee_pressed == false`. The drop and the throw are mutually exclusive
                // by `Without<HeldItem>`, so whichever runs first, only the one that acts
                // consumes the edge.
                drop.write(DropPortalGun { body: subject });
            } else if holding_gun {
                // Plain Attack while holding the gun fires it.
                fire.write(FirePortalGun {
                    aim: pick_aim(control, kin.facing),
                    body: subject,
                });
                // The press is spent for a fire, at the seam that accepts it:
                // `resolve_portal_fire_intent`, after the gun answers (it refuses a gun
                // that is not `active`). A weapon in hand owns the Attack press, and
                // `trigger_moveset_moves` arbitrates from `HeldItem`, which the portal gun
                // is not and must not become.
                //
                // The arbiter has no `PortalGun` branch: its job is to be the single path.
                // Marking the press spent where it is spent is what the pickup and the
                // throw do, and it crosses the phase boundary (both run in
                // `PlayerSimulation`; the trigger reads in `Combat`).
                //
                // The gun stays tappable on a phone because the slot is untouched.
            } else {
                // Plain Attack while NOT holding the gun is a pickup attempt
                // (consumed only if overlapping an armed pickup).
                //
                // Not consumed here. The grant path clears the press when it picks
                // something up (`items::pickup`), and a press that grabs nothing must
                // still reach the wearer's jab.
                pickup.write(PickUpPortalGun { body: subject });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_characters::control::ActorControl;

    /// A body driving the gun, with the Attack press on its slot and on its
    /// brain-resolved frame, as a real press arrives: the slot carries the device
    /// gesture and `ActorControl` carries what the body will act on.
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
            DrivingParticipant(PlayerSlot::PRIMARY),
            BodyKinematics::default(),
            control,
        ));
        if holding {
            body.insert(PortalGun {
                active: true,
                ..PortalGun::default()
            });
        }
        let body = body.id();
        app.insert_resource(ControlledSubject(Some(body)));
        // The composed path, not the adapter alone: the adapter only produces
        // intents, and the system that accepts the action spends the press.
        app.add_message::<ambition_portal2d::PortalFireIntent>();
        app.add_systems(
            Update,
            (
                portal_input_adapter_system,
                super::super::fire_adapter::resolve_portal_fire_intent,
            )
                .chain(),
        );
        (app, body)
    }

    fn melee_still_pressed(app: &App, body: Entity) -> bool {
        app.world()
            .get::<ActorControl>(body)
            .unwrap()
            .0
            .melee_pressed
    }

    /// The gun answers the press, so the jab must not.
    ///
    /// `trigger_moveset_moves` arbitrates the Attack press from `HeldItem`, and
    /// the portal gun is its own component, so the arbiter cannot see it. This
    /// runs the adapter and the resolver, because the press is spent where the
    /// fire is accepted: a refused action cannot eat the press.
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

    /// The negative control. Attack while holding nothing is a pickup attempt;
    /// if it grabs nothing, the press belongs to the wearer's jab. Consuming it
    /// here would silently delete the unarmed attack.
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

    /// Two seats, two guns, two portals, from one tick's presses.
    ///
    /// Every gun gesture names its body. Without that, the resolver would have to
    /// guess whose press it was, or fire one shot per body for one press.
    #[test]
    fn two_driven_bodies_each_fire_their_own_portal_gun() {
        let mut app = App::new();
        app.add_message::<FirePortalGun>();
        app.add_message::<TogglePortalGun>();
        app.add_message::<DropPortalGun>();
        app.add_message::<PickUpPortalGun>();
        app.add_message::<ambition_portal2d::PortalFireIntent>();
        app.insert_resource(ControlledSubject(None));

        let mut slots = SlotControls::default();
        let mut frame = ambition_platformer2d_core::ControlFrame::default();
        frame.attack_pressed = true;
        slots.set(PlayerSlot::PRIMARY, frame);
        slots.set(PlayerSlot(1), frame);
        app.insert_resource(slots);

        let seated = |app: &mut App, slot: u8, sim: &str, x: f32| -> Entity {
            let mut control = ActorControl::default();
            control.0.melee_pressed = true;
            app.world_mut()
                .spawn((
                    DrivingParticipant(PlayerSlot(slot)),
                    BodyKinematics {
                        pos: Vec2::new(x, 0.0),
                        facing: 1.0,
                        ..BodyKinematics::default()
                    },
                    control,
                    PortalGun {
                        active: true,
                        ..PortalGun::default()
                    },
                    ambition_platformer2d_shared_tangle::sim_id::SimId::placement(sim),
                ))
                .id()
        };
        let _a = seated(&mut app, 0, "seat_a", 100.0);
        let _b = seated(&mut app, 1, "seat_b", 900.0);

        app.add_systems(
            Update,
            (
                portal_input_adapter_system,
                super::super::fire_adapter::resolve_portal_fire_intent,
            )
                .chain(),
        );
        app.update();

        let world = app.world_mut();
        let mut cursor = world
            .resource_mut::<bevy::prelude::Messages<ambition_portal2d::PortalFireIntent>>()
            .get_cursor();
        let world = app.world();
        let origins: Vec<f32> = cursor
            .read(world.resource::<bevy::prelude::Messages<ambition_portal2d::PortalFireIntent>>())
            .map(|intent| intent.origin.x)
            .collect();
        assert_eq!(
            origins.len(),
            2,
            "two seats each pressed Attack holding an active gun; got {origins:?}"
        );
        // Each shot leaves its own body. Both intents could come from one body if
        // the resolver re-derived the firer.
        assert!(
            origins.contains(&100.0) && origins.contains(&900.0),
            "each shot must originate at its own firer: {origins:?}"
        );
    }
}
