//! The room binding sweep, against a room authored the way content gets it wrong.

use ambition_engine_core as ae;
use ambition_entity_catalog::placements::CharacterBrain;

use super::binding::RoomBindings;
use crate::rooms::{Authored, GroundItemSpec, KinematicPathSpec, RoomSpec};

fn aabb(x: f32, y: f32) -> ae::Aabb {
    ae::Aabb::new(ae::Vec2::new(x, y), ae::Vec2::new(8.0, 8.0))
}

fn empty_world() -> ae::World {
    ae::World::new(
        "binding_tests",
        ae::Vec2::new(640.0, 360.0),
        ae::Vec2::new(32.0, 32.0),
        Vec::new(),
    )
}

/// A room with one real patrol path and one real pickup, plus an enemy and an
/// item that each name something else.
fn room_with_two_typos() -> RoomSpec {
    let mut room = RoomSpec::new("level_1_2", empty_world());

    room.kinematic_paths.push(KinematicPathSpec::new(
        "ledge_patrol",
        "Ledge Patrol",
        aabb(0.0, 0.0),
        ae::KinematicPath::line(ae::Vec2::ZERO, ae::Vec2::new(64.0, 0.0), 30.0),
    ));

    // Points at a path this room does not have.
    room.enemy_spawns.push(Authored::new(
        "goomba_a",
        "Goomba A",
        aabb(32.0, 0.0),
        CharacterBrain::Patrol {
            path_id: Some("ledge_patrl".to_owned()),
        },
    ));
    // Points at the path by its display name, which the runtime accepts.
    room.enemy_spawns.push(Authored::new(
        "goomba_b",
        "Goomba B",
        aabb(64.0, 0.0),
        CharacterBrain::Patrol {
            path_id: Some("Ledge Patrol".to_owned()),
        },
    ));
    // Names an archetype no catalog carries.
    room.enemy_spawns.push(Authored::new(
        "koopa_a",
        "Koopa A",
        aabb(96.0, 0.0),
        CharacterBrain::Custom("snake_kooopa".to_owned()),
    ));

    room.ground_items.push(GroundItemSpec {
        id: "pickup_a".to_owned(),
        name: "Milk".to_owned(),
        held_item: "gun_sword".to_owned(),
        pos: ae::Vec2::new(128.0, 0.0),
        half_extent: ae::Vec2::new(8.0, 8.0),
    });
    room.ground_items.push(GroundItemSpec {
        id: "pickup_b".to_owned(),
        name: "Axe".to_owned(),
        held_item: "battleaxe".to_owned(),
        pos: ae::Vec2::new(160.0, 0.0),
        half_extent: ae::Vec2::new(8.0, 8.0),
    });

    room
}

/// One pass, one report, every namespace this sweep OWNS: the bad patrol path
/// and the bad archetype come back together, each naming what declared it. The
/// good references — including the one that addresses a path by display name
/// rather than id — do not appear.
///
/// Before this, each was a separate silent fallback: the patrol brain went
/// passive, the archetype defaulted. The bad pickup id is deliberately not here;
/// it fails construction outright, and one defect gets one authority.
#[test]
fn construction_reports_every_unresolved_ref() {
    let room = room_with_two_typos();
    let bindings = RoomBindings::default().with_characters(["goomba", "snake_koopa"]);

    let report = bindings.sweep(&room);
    assert_eq!(report.len(), 2, "one pass finds both:\n{report}");

    let by_namespace: Vec<_> = report
        .unresolved()
        .iter()
        .map(|u| (u.namespace, u.id.as_str(), u.declared_by.as_str()))
        .collect();
    assert_eq!(
        by_namespace,
        vec![
            ("character", "snake_kooopa", "enemy spawn `koopa_a`"),
            (
                "kinematic path",
                "ledge_patrl",
                "patrol brain of `goomba_a`"
            ),
        ],
    );

    // The path report offers both spellings the real path answers to, because
    // "the id you used is not one of these" is what a fixer needs to see.
    let path = &report.unresolved()[1];
    assert_eq!(path.did_you_mean.as_deref(), Some("ledge_patrol"));
    assert_eq!(
        path.available,
        vec!["Ledge Patrol".to_owned(), "ledge_patrol".to_owned()],
    );
}

/// Two paths answering to one spelling is not a resolution failure — the first
/// one wins and everything draws — so it used to pass in total silence, with the
/// author's second declaration simply unreachable. It is reported now, as a
/// warning rather than a binding failure, because the room is still publishable.
#[test]
fn a_room_that_declares_one_path_twice_says_so() {
    let mut room = room_with_two_typos();
    room.kinematic_paths.push(KinematicPathSpec::new(
        "ledge_patrol",
        "Ledge Patrol (copy)",
        aabb(0.0, 0.0),
        ae::KinematicPath::line(ae::Vec2::ZERO, ae::Vec2::new(64.0, 0.0), 30.0),
    ));

    let report = RoomBindings::default().sweep(&room);
    let ambiguous: Vec<_> = report
        .ambiguous()
        .iter()
        .map(|a| (a.namespace, a.id.as_str()))
        .collect();
    assert_eq!(ambiguous, vec![("kinematic path", "ledge_patrol")]);
}

/// The validator must accept every spelling the runtime accepts, including the
/// normalized display-name slug used by LDtk-authored patrol references.
#[test]
fn normalized_path_name_is_not_a_false_binding_error() {
    let mut room = RoomSpec::new("slug_room", empty_world());
    room.kinematic_paths.push(KinematicPathSpec::new(
        "enemy_patrol_a",
        "enemy patrol path A",
        aabb(0.0, 0.0),
        ae::KinematicPath::line(ae::Vec2::ZERO, ae::Vec2::new(64.0, 0.0), 30.0),
    ));
    room.enemy_spawns.push(Authored::new(
        "goomba_a",
        "Goomba A",
        aabb(32.0, 0.0),
        CharacterBrain::Patrol {
            path_id: Some("enemy_patrol_path_a".to_owned()),
        },
    ));

    let report = RoomBindings::default().sweep(&room);
    assert!(
        report.is_empty(),
        "runtime-valid alias was rejected:\n{report}"
    );
}

/// An id and display name that happen to be identical are two spellings of one
/// declaration, not two competing declarations.
#[test]
fn repeated_alias_for_one_path_is_not_ambiguous() {
    let mut room = RoomSpec::new("same_alias_room", empty_world());
    room.kinematic_paths.push(KinematicPathSpec::new(
        "patrol",
        "patrol",
        aabb(0.0, 0.0),
        ae::KinematicPath::line(ae::Vec2::ZERO, ae::Vec2::new(64.0, 0.0), 30.0),
    ));

    let report = RoomBindings::default().sweep(&room);
    assert!(report.ambiguous().is_empty(), "{report}");
}

/// A sweep with no catalogs still checks the room against ITSELF, and says so.
///
/// This is the honest half: an absent resolver means "not checked", never
/// "checked and clean". A caller that needs full coverage reads `checked()`
/// rather than trusting an empty report.
#[test]
fn a_sweep_without_catalogs_says_which_namespaces_it_decided() {
    let room = room_with_two_typos();
    let bindings = RoomBindings::default();

    assert_eq!(bindings.checked(), vec!["kinematic path"]);
    let report = bindings.sweep(&room);
    assert_eq!(
        report.len(),
        1,
        "only the room-internal namespace is decidable here:\n{report}"
    );
}
