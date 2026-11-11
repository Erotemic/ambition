//! Ambition input binding for portal generators.
//!
//! Translates the Ambition [`ControlFrame`] (and the nearest-interactable HUD
//! state) into the reusable portal intent messages ([`FirePortalGenerator`] /
//! [`TogglePortalGenerator`]). Portal core consumes only those intents and
//! never reads `ControlFrame`, so a replay or another input layer can drive a
//! generator by emitting the same messages.
//!
//! The gestures of a body that is a [`PortalGenerator`] (it holds the portal
//! gun):
//! - `Attack` (no Shield) → fire, with the aim resolved from right-stick / move
//!   axis / facing;
//! - `Interact` (when no door / NPC claims it) → color toggle.
//!
//! Taking the gun up and putting it down are the ordinary held-item road's
//! (`Attack` over it, `Shield + Attack`), not this adapter's.

use bevy::prelude::*;

use ambition_characters::control::{DrivingParticipant, PlayerSlot, SlotControls};
use ambition_platformer2d_core::{BodyKinematics, ControlFrame};
use ambition_platformer2d_shared_tangle::markers::ControlledSubject;
use ambition_portal2d::{FirePortalGenerator, PortalGenerator, TogglePortalGenerator};
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

/// Translate this frame's controller input into portal intents for each
/// driven body that is a generator. Gestures come from the body's slot, and
/// aim and origin from its own kinematics: possess an actor holding the gun
/// and it fires from that actor, not the vacated home avatar. Runs in the
/// portal input set so the core fire and toggle see the intents in the same
/// frame.
pub fn portal_input_adapter_system(
    nearest: Option<Res<NearestInteractable>>,
    controlled: Res<ControlledSubject>,
    // The controller's slot frame (the sanctioned per-slot input source).
    slots: Res<SlotControls>,
    // A producer cannot know whether its action will be accepted (the fire is
    // refused for an inactive generator), so it does not spend the press.
    holders: Query<(Option<&DrivingParticipant>, &BodyKinematics), With<PortalGenerator>>,
    // The seated half of the driven population: the union `DrivenBodies`
    // names, written out here because that `SystemParam` is the actor crate's.
    driven_seats: Query<
        (
            Entity,
            Option<&ambition_platformer2d_shared_tangle::sim_id::SimId>,
        ),
        With<DrivingParticipant>,
    >,
    mut fire: MessageWriter<FirePortalGenerator>,
    mut toggle: MessageWriter<TogglePortalGenerator>,
) {
    // Every driven body makes its own gestures, and each gesture names its
    // body, so a couch's second seat can fire and toggle its own gun.
    let mut subjects: Vec<Entity> = Vec::new();
    if let Some(subject) = controlled.0 {
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
    for subject in subjects {
        let Ok((driver, kin)) = holders.get(subject) else {
            continue;
        };
        let slot = driver.map_or(PlayerSlot::PRIMARY, |driver| driver.0);
        let control = slots.get(slot);
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
                toggle.write(TogglePortalGenerator { body: subject });
            }
        }
        // Plain Attack fires. `Shield + Attack` is the held-item road's: it
        // puts the gun down.
        //
        // The press is spent for a fire at the seam that accepts it,
        // `resolve_portal_fire_intent`, after the generator answers (it refuses
        // one that is not `active`). The gun is a held item with no verb, so
        // the body has no swing for the press to reach either way.
        if control.attack_pressed && !control.shield_held {
            fire.write(FirePortalGenerator {
                aim: pick_aim(&control, kin.facing),
                body: subject,
            });
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
        app.add_message::<FirePortalGenerator>();
        app.add_message::<TogglePortalGenerator>();

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
            body.insert(PortalGenerator::default());
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
    /// This runs the adapter and the resolver, because the press is spent
    /// where the fire is accepted: a refused action cannot eat the press.
    #[test]
    fn firing_the_gun_spends_the_attack_press() {
        let (mut app, body) = app_with_holder(true);
        app.update();
        assert_eq!(
            app.world()
                .resource::<Messages<FirePortalGenerator>>()
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

    /// The negative control. Attack while holding no gun fires nothing, and
    /// the press belongs to the body (its jab, or a pickup).
    #[test]
    fn attacking_with_no_gun_leaves_the_press_for_the_body() {
        let (mut app, body) = app_with_holder(false);
        app.update();
        assert_eq!(
            app.world()
                .resource::<Messages<FirePortalGenerator>>()
                .iter_current_update_messages()
                .count(),
            0,
            "with no gun, Attack fires nothing"
        );
        assert!(
            melee_still_pressed(&app, body),
            "⛔ a body that is no generator must keep its press"
        );
    }

    /// Two seats, two guns, two portals, from one tick's presses.
    ///
    /// Every gun gesture names its body. Without that, the resolver would have to
    /// guess whose press it was, or fire one shot per body for one press.
    #[test]
    fn two_driven_bodies_each_fire_their_own_portal_gun() {
        let mut app = App::new();
        app.add_message::<FirePortalGenerator>();
        app.add_message::<TogglePortalGenerator>();
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
                    PortalGenerator::default(),
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
