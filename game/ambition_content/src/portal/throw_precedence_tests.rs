//! Shield+Attack, when a portal adapter and a held item both want it.
//!
//! So a body holding a laser sword lost the press to an action that was then refused, and the axe
//! could not be thrown at all.
//!
//!  the fix is the rule, not an ordering: an action spends the input edge where it COMMITS,
//! never in a producer whose consumer may reject it.

use bevy::prelude::*;

use ambition_characters::brain::HeldItemSpec;
use ambition_characters::control::ActorControl;
use ambition_characters::control::{DrivingParticipant, PlayerSlot, SlotControls};
use ambition_combat::held_items::HeldItem;
use ambition_held_items::{throw_held_item_system, GroundItem};
use ambition_platformer2d_core::BodyKinematics;
use ambition_platformer2d_shared_tangle::markers::ControlledSubject;
use ambition_platformer2d_shared_tangle::markers::{PlayerEntity, PrimaryPlayer};
use ambition_portal2d::{
    DropPortalGun, FirePortalGun, PickUpPortalGun, PortalGun, TogglePortalGun,
};

use super::input_adapter::portal_input_adapter_system;
use super::inventory_adapter::drop_portal_gun_system;

/// What the body was carrying when it pressed Shield+Attack.
#[derive(Clone, Copy)]
enum Carrying {
    Item,
    Gun,
    /// The pathological case: both. The item's throw wins, by the drop's own
    /// stated precedence.
    Both,
}

struct Outcome {
    holds_item: bool,
    holds_gun: bool,
    ground_items: usize,
    press_survived: bool,
}

/// Press Shield+Attack once, with the portal adapters AND the held-item throw
/// composed exactly as the app composes them.
fn press_shield_attack(carrying: Carrying) -> Outcome {
    let mut app = App::new();
    app.add_message::<ambition_sfx::OwnedSfxMessage>();
    app.add_message::<FirePortalGun>();
    app.add_message::<TogglePortalGun>();
    app.add_message::<DropPortalGun>();
    app.add_message::<PickUpPortalGun>();
    app.init_resource::<ambition_platformer2d_shared_tangle::gravity::GravityField>();
    app.init_resource::<ambition_platformer2d_shared_tangle::gravity::GravityZones>();

    // The device gesture, on the slot the body's brain reads.
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

    let mut body = app.world_mut().spawn((
        PlayerEntity,
        PrimaryPlayer,
        DrivingParticipant(PlayerSlot::PRIMARY),
        BodyKinematics {
            pos: Vec2::new(100.0, 100.0),
            vel: Vec2::ZERO,
            size: Vec2::new(24.0, 40.0),
            facing: 1.0,
        },
        ambition_platformer2d_core::BodyBaseSize {
            base_size: Vec2::new(24.0, 40.0),
        },
        ambition_characters::brain::ActionSet::default(),
        control,
    ));
    if matches!(carrying, Carrying::Item | Carrying::Both) {
        body.insert(HeldItem::new(HeldItemSpec {
            id: "laser_sword".into(),
            ..Default::default()
        }));
    }
    if matches!(carrying, Carrying::Gun | Carrying::Both) {
        body.insert(PortalGun {
            active: true,
            ..PortalGun::default()
        });
    }
    let body = body.id();
    app.insert_resource(ControlledSubject(Some(body)));

    // The production composition: the adapter produces intents, the drop and the
    // throw each answer the ones they own.
    app.add_systems(
        Update,
        (
            portal_input_adapter_system,
            drop_portal_gun_system,
            throw_held_item_system,
        )
            .chain(),
    );
    app.update();

    let ground_items = {
        let mut q = app.world_mut().query::<&GroundItem>();
        q.iter(app.world()).count()
    };
    let entity = app.world().entity(body);
    Outcome {
        holds_item: entity.contains::<HeldItem>(),
        holds_gun: entity.contains::<PortalGun>(),
        ground_items,
        press_survived: entity.get::<ActorControl>().unwrap().0.melee_pressed,
    }
}

#[test]
fn shield_attack_throws_a_held_item_with_the_portal_adapter_installed() {
    let outcome = press_shield_attack(Carrying::Item);
    assert!(
        !outcome.holds_item,
        "⛔ Shield+Attack did not throw the held item. The portal input adapter \
         emitted a drop intent for a gun this body does not have and spent the \
         Attack press doing it, so the throw saw melee_pressed == false"
    );
    assert_eq!(
        outcome.ground_items, 1,
        "the thrown item must land in the world as a ground item"
    );
    assert!(
        !outcome.press_survived,
        "the throw is this press's action, so it must spend the edge exactly once"
    );
}

/// The portal gun still drops, and still spends the press. Moving the
/// consumption to the commit site must not have taken the behaviour with it.
#[test]
fn shield_attack_drops_the_portal_gun_when_that_is_all_the_body_holds() {
    let outcome = press_shield_attack(Carrying::Gun);
    assert!(!outcome.holds_gun, "Shield+Attack drops the portal gun");
    assert!(
        !outcome.press_survived,
        "the drop answered the press, so the wearer's jab must not answer it too"
    );
}

/// Both in hand: the ITEM's throw wins, which is the precedence
/// `drop_portal_gun_system`'s `Without<HeldItem>` filter already declared and
/// its comment already stated. Pinned here because nothing enforced it across
/// the two systems.
#[test]
fn a_body_holding_both_throws_the_item_and_keeps_the_gun() {
    let outcome = press_shield_attack(Carrying::Both);
    assert!(!outcome.holds_item, "the throwable takes precedence");
    assert_eq!(outcome.ground_items, 1, "the item reaches the ground");
    assert!(
        outcome.holds_gun,
        "the gun is NOT dropped in the same press — the drop is refused while a \
         throwable is in hand, and refusing must not also consume the gesture"
    );
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
        app.add_message::<FirePortalGun>();
        app.add_message::<TogglePortalGun>();
        app.add_message::<DropPortalGun>();
        app.add_message::<PickUpPortalGun>();
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
                    BodyKinematics {
                        pos: Vec2::new(x, 100.0),
                        vel: Vec2::ZERO,
                        size: Vec2::new(24.0, 40.0),
                        facing: 1.0,
                    },
                    ambition_platformer2d_core::BodyBaseSize {
                        base_size: Vec2::new(24.0, 40.0),
                    },
                    ambition_characters::brain::ActionSet::default(),
                    ActorControl::default(),
                    PortalGun {
                        active: true,
                        ..PortalGun::default()
                    },
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
        let messages = world.resource::<Messages<TogglePortalGun>>();
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
