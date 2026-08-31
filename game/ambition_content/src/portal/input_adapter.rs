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

use ambition_characters::control::{DrivingParticipant, PlayerSlot, SlotControls};
use ambition_input::ControlFrame;
use ambition_platformer2d_core::BodyKinematics;
use ambition_platformer2d_shared_tangle::markers::ControlledSubject;
use ambition_platformer2d_shared_tangle::markers::{PlayerEntity, PrimaryPlayer};
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
    // The controlled body: its brain (→ slot), position and held gun (if any).
    //
    // A producer cannot know whether the action it names will be ACCEPTED — the drop is refused for
    // a body holding a throwable, the fire is refused for an inactive gun — so spending the press
    // here spent it for actions that never happened. See the drop branch below.
    holders: Query<(
        Option<&DrivingParticipant>,
        &BodyKinematics,
        Option<&PortalGun>,
    )>,
    // The seated half of the driven population — the union `DrivenBodies`
    // names, spelled here because that `SystemParam` is the actor crate's and
    // this adapter is content's.
    driven_seats: Query<
        (
            Entity,
            Option<&ambition_platformer2d_shared_tangle::sim_id::SimId>,
        ),
        With<DrivingParticipant>,
    >,
    primary_fallback: Query<Entity, (With<PlayerEntity>, With<PrimaryPlayer>)>,
    #[cfg(feature = "portal_render")] mut aim_hint: Option<ResMut<PortalAimHint>>,
    mut fire: MessageWriter<FirePortalGun>,
    mut toggle: MessageWriter<TogglePortalGun>,
    mut drop: MessageWriter<DropPortalGun>,
    mut pickup: MessageWriter<PickUpPortalGun>,
) {
    // ⭐ EVERY DRIVEN BODY MAKES ITS OWN GESTURES, and each gesture names the
    // body that made it — so a couch's second seat can fire, toggle, drop and
    // pick up its own gun. This resolved ONE `ControlledSubject`; the second
    // seat's presses reached nothing.
    //
    // ⚠ The fallback is the STARTUP frame and nothing else.
    let mut subjects: Vec<Entity> = Vec::new();
    if let Some(subject) = controlled.as_deref().and_then(|held| held.0) {
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
    if subjects.is_empty() {
        subjects.extend(primary_fallback.single().ok());
    }
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
            // ⛔⛔ THIS BODY'S REACH, NOT SEAT ZERO'S. It asked the singleton
            // `NearestInteractable.0` — one answer computed from ONE controlled
            // subject — inside a loop over every driven body. With two people
            // playing, seat zero standing near a chest suppressed seat one's
            // toggle, and seat zero standing clear let seat one both toggle AND
            // interact.
            //
            // ⚠ IT IS STILL A PREDICTION, and that is a property of the phase
            // rather than a shortcut: this adapter runs in `PlayerSimulation`
            // and the interaction road SPENDS the press later, in
            // `FeatureInteraction`, so the claim cannot be read — only
            // anticipated. Both use the same `strict_intersects` reach, which is
            // what keeps the anticipation right.
            let claimed = nearest
                .as_deref()
                .is_some_and(|n| !matches!(n.for_body(subject), InteractVariant::None));
            if !claimed {
                toggle.write(TogglePortalGun { body: subject });
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
                // Shield+Attack is the drop gesture — an INTENT, not a claim on the
                // press.
                //
                // So a body holding a laser sword pressed Shield+Attack, this spent the press, the drop
                // refused the intent, and `throw_held_item_system` — whose Shield+Attack throw is the
                // correct answer — found `melee_pressed == false` and did nothing. The item could not
                // be thrown at all.
                //
                //  the press is spent where the action COMMITS. That also removes
                // an ordering question rather than answering it: the drop and the
                // throw are mutually exclusive by `Without<HeldItem>`, so whichever
                // runs first, only the one that actually acts consumes the edge.
                drop.write(DropPortalGun { body: subject });
            } else if holding_gun {
                // Plain Attack while holding the gun fires it.
                fire.write(FirePortalGun {
                    aim: pick_aim(control, kin.facing),
                    body: subject,
                });
                // the press IS spent for a fire — but at the seam that accepts it. A weapon in
                // hand owns the Attack press, and `trigger_moveset_moves` arbitrates that from
                // `HeldItem` — which the portal gun is not, and must not become (its own component
                // is the right shape).
                //
                // but not HERE. `resolve_portal_fire_intent` refuses a gun
                // that is not `active`, so spending the press in this branch spent it
                // for fires that never happened, exactly as the drop branch above
                // did. It is consumed there, after the gun has actually answered.
                //
                // still not special-cased in the arbiter: a third branch in
                // `trigger_moveset_moves` naming `PortalGun` would add a path to the
                // one place whose entire job is having a single one. Marking the
                // press spent where it is spent is the mechanism the pickup and the
                // throw already use, and it crosses the phase boundary for free —
                // both run in `PlayerSimulation`, the trigger looks in `Combat`.
                //
                // The gun stays tappable on a phone because the slot is untouched.
            } else {
                // Plain Attack while NOT holding the gun is a pickup attempt
                // (consumed only if overlapping an armed pickup).
                //
                // NOT consumed here. The grant path clears the press itself
                // when it actually picks something up (`items::pickup`), and a press
                // that grabs nothing must still reach the wearer's jab — swinging at
                // empty air is the correct answer to "Attack while holding nothing".
                pickup.write(PickUpPortalGun { body: subject });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_characters::control::ActorControl;

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
        // the COMPOSED path, not the adapter alone. The adapter is a read-only intent producer
        // now; the press is spent by whichever system ACCEPTS the action.
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

    /// the gun answers the press, so the jab must not.
    ///
    /// `trigger_moveset_moves` arbitrates the Attack press from `HeldItem`, and the portal gun
    /// is its own component — so the arbiter cannot see it and the wearer's jab answered the
    /// same press.
    ///
    /// this now runs the adapter AND the resolver, because the press is
    /// spent where the fire is accepted rather than where it is requested. The
    /// outcome is identical for a real fire; what changed is that a REFUSED
    /// action can no longer eat the press.
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

    /// ⭐⭐ TWO SEATS, TWO GUNS, TWO PORTALS — from one tick's presses.
    ///
    /// ⛔⛔ THE GESTURE CARRIED AN AIM AND NOTHING ELSE, so this adapter
    /// resolved one `ControlledSubject` and the resolver re-derived the firer
    /// the same way. A second seat holding a portal gun made a press that
    /// reached nothing, and a resolver that had simply looped driven bodies
    /// would have had to GUESS whose press it was and fired one shot per body
    /// for one press. Every gun gesture names its body now.
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

        let mut seated = |app: &mut App, slot: u8, sim: &str, x: f32| -> Entity {
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
        // ⛔ AND EACH SHOT LEAVES ITS OWN BODY. Two intents could both come from
        // one body if the resolver still re-derived the firer.
        assert!(
            origins.contains(&100.0) && origins.contains(&900.0),
            "each shot must originate at its own firer: {origins:?}"
        );
    }
}
