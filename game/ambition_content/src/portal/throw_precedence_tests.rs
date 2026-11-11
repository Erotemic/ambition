//! `Shield + Attack` on a body holding the portal gun, with the portal
//! adapters installed: the gun is an ordinary held item, so the held-item
//! throw puts it down, and the portal adapters take no part in that press.

use bevy::prelude::*;

use ambition_characters::control::ActorControl;
use ambition_characters::control::{DrivingParticipant, PlayerSlot, SlotControls};
use ambition_combat::held_items::HeldItem;
use ambition_held_items::{throw_held_item_system, GroundItem};
use ambition_platformer2d_core::BodyKinematics;
use ambition_platformer2d_shared_tangle::markers::ControlledSubject;
use ambition_platformer2d_shared_tangle::markers::{PlayerEntity, PrimaryPlayer};
use ambition_portal2d::{FirePortalGenerator, PortalGenerator, TogglePortalGenerator};

use super::input_adapter::portal_input_adapter_system;
use super::inventory_adapter::reconcile_portal_generators;

/// `Shield + Attack` puts the gun down where any kept item goes: the body
/// stops being a generator, the gun lies in the world as one (so its pair's
/// portals stay open), and no shot is fired.
#[test]
fn shield_attack_puts_the_portal_gun_down_like_any_item() {
    let mut app = App::new();
    app.add_message::<ambition_sfx::OwnedSfxMessage>();
    app.add_message::<FirePortalGenerator>();
    app.add_message::<TogglePortalGenerator>();
    app.init_resource::<ambition_platformer2d_shared_tangle::gravity::GravityField>();
    app.init_resource::<ambition_platformer2d_shared_tangle::gravity::GravityZones>();

    // The device gesture, on the slot the body's brain reads...
    let mut slots = SlotControls::default();
    let mut frame = ambition_platformer2d_core::ControlFrame::default();
    frame.attack_pressed = true;
    frame.shield_held = true;
    slots.set(PlayerSlot::PRIMARY, frame);
    app.insert_resource(slots);
    // ...and the same press on the brain-resolved frame the body acts on.
    let mut control = ActorControl::default();
    control.0.melee_pressed = true;
    control.0.shield_held = true;

    let gun = ambition_characters::brain::held_item_by_id("portal_gun").expect("the portal gun is a held item");
    let body = app
        .world_mut()
        .spawn((
            PlayerEntity,
            PrimaryPlayer,
            DrivingParticipant(PlayerSlot::PRIMARY),
            BodyKinematics { material: Default::default(),
                pos: Vec2::new(100.0, 100.0),
                vel: Vec2::ZERO,
                canonical_size: Vec2::new(24.0, 40.0),
                facing: 1.0,
            },
            ambition_platformer2d_core::BodyBaseSize {
                base_size: Vec2::new(24.0, 40.0),
            },
            ambition_characters::brain::ActionSet::default(),
            ambition_characters::brain::action_set::IdentityKit::default(),
            ambition_combat::moveset::ActorMoveset::default(),
            control,
            HeldItem::new(gun),
            PortalGenerator::default(),
        ))
        .id();
    app.insert_resource(ControlledSubject(Some(body)));
    app.add_systems(
        Update,
        (portal_input_adapter_system, throw_held_item_system, reconcile_portal_generators).chain(),
    );
    app.update();

    let entity = app.world().entity(body);
    assert!(!entity.contains::<HeldItem>(), "Shield + Attack did not put the gun down");
    assert!(!entity.contains::<PortalGenerator>(), "the emptied hand still opens portals");
    let lying: Vec<bool> = app
        .world_mut()
        .query::<(&GroundItem, Has<PortalGenerator>)>()
        .iter(app.world())
        .map(|(item, generator)| {
            assert_eq!(item.spec.id, "portal_gun");
            generator
        })
        .collect();
    assert_eq!(lying, vec![true], "the gun lies in the world, and is a generator there");
    let fired = app.world().resource::<Messages<FirePortalGenerator>>().iter_current_update_messages().count();
    assert_eq!(fired, 0, "Shield + Attack is the put-down, not a shot");
}

/// Seat one's portal toggle is decided by SEAT ONE's surroundings.
///
/// ⛔⛔ IT WAS DECIDED BY SEAT ZERO'S. The adapter loops over every driven body
/// and asked the singleton `NearestInteractable.0` — one answer computed from
/// one `ControlledSubject` — whether an ordinary interaction had claimed the
/// press. Found by the 2026-08-31 GPT review. Both failures are real:
/// seat zero near a chest SUPPRESSED seat one's toggle, and seat zero standing
/// clear let seat one both toggle and interact.
///
/// ⚠ THE ANSWER IS STILL A PREDICTION, and the test pins that shape rather than
/// pretending otherwise: this adapter runs in `PlayerSimulation` and the
/// interaction road spends the press later in `FeatureInteraction`, so the claim
/// cannot be read — only anticipated from the same reach test.
#[test]
fn one_seats_surroundings_do_not_decide_another_seats_toggle() {
    use ambition_sim_view::affordances::{InteractVariant, NearestInteractable};

    fn toggles_for(near_zero: bool, near_one: bool) -> Vec<Entity> {
        let mut app = App::new();
        app.add_message::<ambition_sfx::OwnedSfxMessage>();
        app.add_message::<FirePortalGenerator>();
        app.add_message::<TogglePortalGenerator>();
        app.init_resource::<ambition_platformer2d_shared_tangle::gravity::GravityField>();
        app.init_resource::<ambition_platformer2d_shared_tangle::gravity::GravityZones>();

        // BOTH seats press Interact on the same tick.
        let mut slots = SlotControls::default();
        let mut frame = ambition_platformer2d_core::ControlFrame::default();
        frame.interact_pressed = true;
        slots.set(PlayerSlot::PRIMARY, frame);
        slots.set(PlayerSlot(1), frame);
        app.insert_resource(slots);

        let spawn = |app: &mut App, slot: u8, x: f32| {
            app.world_mut()
                .spawn((
                    PlayerEntity,
                    DrivingParticipant(PlayerSlot(slot)),
                    BodyKinematics { material: Default::default(),
                        pos: Vec2::new(x, 100.0),
                        vel: Vec2::ZERO,
                        canonical_size: Vec2::new(24.0, 40.0),
                        facing: 1.0,
                    },
                    ambition_platformer2d_core::BodyBaseSize {
                        base_size: Vec2::new(24.0, 40.0),
                    },
                    ambition_characters::brain::ActionSet::default(),
                    ActorControl::default(),
                    PortalGenerator::default(),
                ))
                .id()
        };
        let zero = spawn(&mut app, 0, 100.0);
        let one = spawn(&mut app, 1, 900.0);
        app.insert_resource(ControlledSubject(Some(zero)));

        // The proximity answer, staged per body — the fact the adapter reads.
        let mut by_body = std::collections::HashMap::new();
        if near_zero {
            by_body.insert(zero, InteractVariant::Open);
        }
        if near_one {
            by_body.insert(one, InteractVariant::Open);
        }
        // `.0` is seat zero's, exactly as the producer computes it.
        let seat_zero = if near_zero {
            InteractVariant::Open
        } else {
            InteractVariant::None
        };
        app.insert_resource(NearestInteractable(seat_zero, by_body));

        app.add_systems(Update, portal_input_adapter_system);
        app.update();

        let world = app.world_mut();
        let messages = world.resource::<Messages<TogglePortalGenerator>>();
        let mut cursor = messages.get_cursor();
        cursor.read(messages).map(|t| t.body).collect()
    }

    let (zero_near, one_clear) = (true, false);
    let toggles = toggles_for(zero_near, one_clear);
    assert!(
        toggles.len() == 1,
        "seat zero is at a chest and seat one is in open ground: exactly one \
         toggle should come out, and it is seat one's. Got {} — seat zero's \
         surroundings decided seat one's press",
        toggles.len()
    );

    // ⛔ THE OTHER DIRECTION, which the singleton got wrong the opposite way:
    // seat zero clear, seat one AT the chest. Seat one must NOT toggle.
    let toggles = toggles_for(false, true);
    assert_eq!(
        toggles.len(),
        1,
        "seat zero is clear and seat one is at a chest: seat zero toggles, seat \
         one's press belongs to the interaction"
    );

    // ⛔ AND THE PREMISE. With nobody near anything, both seats toggle — or the
    // assertions above pass because the adapter emits nothing at all.
    assert_eq!(toggles_for(false, false).len(), 2);
    // …and with both at a chest, neither does.
    assert_eq!(toggles_for(true, true).len(), 0);
}
