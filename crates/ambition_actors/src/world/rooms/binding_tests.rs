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

/// One pass, one report, every namespace: the bad patrol path, the bad
/// archetype, and the bad pickup id all come back together, each naming what it
/// was declared by. The good references — including the one that addresses a
/// path by display name rather than id — do not appear.
///
/// Before this, each of these three was a separate silent fallback: the patrol
/// brain went passive, the archetype defaulted, and the ground item was (in its
/// own doc's words) "skipped at spawn rather than erroring".
#[test]
fn construction_reports_every_unresolved_ref() {
    let room = room_with_two_typos();
    let bindings = RoomBindings::default()
        .with_characters(["goomba", "snake_koopa"])
        .with_held_items(["gun_sword", "axe"]);

    let report = bindings.sweep(&room);
    assert_eq!(report.len(), 3, "one pass finds all three:\n{report}");

    let by_namespace: Vec<_> = report
        .unresolved()
        .iter()
        .map(|u| (u.namespace, u.id.as_str(), u.declared_by.as_str()))
        .collect();
    assert_eq!(
        by_namespace,
        vec![
            ("character", "snake_kooopa", "enemy spawn `koopa_a`"),
            ("held item", "battleaxe", "ground item `pickup_b`"),
            (
                "kinematic path",
                "ledge_patrl",
                "patrol brain of `goomba_a`"
            ),
        ],
    );

    // The path report offers both spellings the real path answers to, because
    // "the id you used is not one of these" is what a fixer needs to see.
    let path = &report.unresolved()[2];
    assert_eq!(path.did_you_mean.as_deref(), Some("ledge_patrol"));
    assert_eq!(
        path.available,
        vec!["Ledge Patrol".to_owned(), "ledge_patrol".to_owned()],
    );
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
