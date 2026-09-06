//! The room binding sweep, against a room authored the way content gets it wrong.

use ambition_entity_catalog::placements::{
    CharacterBrain, DamageKind, DamageTeam, HazardRespawn, HazardSpec, InteractableSpec,
    InteractionKindSpec, PlacementSchema,
};
use ambition_platformer2d_core as ae;

use super::binding::RoomBindings;
use ambition_platformer2d_world::rooms::{Authored, EnemySpawnSpec, GroundItemSpec, KinematicPathSpec, RoomSpec};
use ambition_platformer2d_world::placements::PlacementRecord;

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
        EnemySpawnSpec::new(
            CharacterBrain::Patrol {
                path_id: Some("ledge_patrl".to_owned()),
            },
            // the placement must name a creature now; these rows
            // are about PATH binding, so any registered id serves.
            "fixture_walker",
        ),
    ));
    // Points at the path by its display name, which the runtime accepts.
    room.enemy_spawns.push(Authored::new(
        "goomba_b",
        "Goomba B",
        aabb(64.0, 0.0),
        EnemySpawnSpec::new(
            CharacterBrain::Patrol {
                path_id: Some("Ledge Patrol".to_owned()),
            },
            // the placement must name a creature now; these rows
            // are about PATH binding, so any registered id serves.
            "fixture_walker",
        ),
    ));
    // Names an archetype no catalog carries.
    room.enemy_spawns.push(Authored::new(
        "koopa_a",
        "Koopa A",
        aabb(96.0, 0.0),
        EnemySpawnSpec::new(
            CharacterBrain::Custom("snake_kooopa".to_owned()),
            // the placement must name a creature now; these rows
            // are about PATH binding, so any registered id serves.
            "fixture_walker",
        ),
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
/// comes back naming what declared it, and the good references — including the
/// one that addresses a path by display name rather than id — do not appear.
///
/// the CHARACTER namespace left this sweep with the archetype roster (AC6).
#[test]
fn construction_reports_every_unresolved_ref() {
    let room = room_with_two_typos();
    let bindings = RoomBindings::default();

    let report = bindings.sweep(&room);
    assert_eq!(report.len(), 1, "one pass finds the patrol path:\n{report}");

    let by_namespace: Vec<_> = report
        .unresolved()
        .iter()
        .map(|u| (u.namespace, u.id.as_str(), u.declared_by.as_str()))
        .collect();
    assert_eq!(
        by_namespace,
        vec![(
            "kinematic path",
            "ledge_patrl",
            "patrol brain of `goomba_a`",
        )],
    );
}

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
        EnemySpawnSpec::new(
            CharacterBrain::Patrol {
                path_id: Some("enemy_patrol_path_a".to_owned()),
            },
            // the placement must name a creature now; these rows
            // are about PATH binding, so any registered id serves.
            "fixture_walker",
        ),
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

fn npc_placement(id: &str, patrol_path_id: Option<&str>) -> PlacementRecord {
    PlacementRecord::new(
        id,
        PlacementSchema::Interactable(InteractableSpec::new(
            "Talk",
            InteractionKindSpec::Npc {
                character_id: Some("fixture_walker".to_owned()),
                dialogue_id: None,
                patrol_radius: 32.0,
                patrol_path_id: patrol_path_id.map(str::to_owned),
                brain_override: None,
            },
        )),
        aabb(0.0, 0.0),
    )
}

fn hazard_placement(id: &str, path_id: Option<&str>) -> PlacementRecord {
    PlacementRecord::new(
        id,
        PlacementSchema::Hazard(HazardSpec {
            damage: 1,
            knockback: [0.0, 0.0],
            kind: DamageKind::Hazard,
            team: DamageTeam::Environment,
            hitstop_seconds: 0.0,
            respawn: HazardRespawn::Never,
            path_id: path_id.map(str::to_owned),
        }),
        aabb(0.0, 0.0),
    )
}

/// THE INVARIANT: every reference that SHRUGS is named, not just the enemy
/// brain. Three roads resolve a path id by string equality and all three fall
/// through to `None` in silence; the sweep decided one of them, so an NPC or a
/// moving hazard pointing at nothing was a clean report and a dead feature.
///
/// The NPC case is the quietest: it still patrols its home±radius lane, so it
/// MOVES — just not along the waypoints somebody drew — which is why a report is
/// the only thing that can catch it.
#[test]
fn an_npc_and_a_hazard_pointing_at_no_path_are_both_reported() {
    let mut room = RoomSpec::new("placement_refs", empty_world());
    room.kinematic_paths.push(KinematicPathSpec::new(
        "ledge_patrol",
        "Ledge Patrol",
        aabb(0.0, 0.0),
        ae::KinematicPath::line(ae::Vec2::ZERO, ae::Vec2::new(64.0, 0.0), 30.0),
    ));
    room.placements
        .push(npc_placement("shopkeep", Some("ledge_patrl")));
    room.placements
        .push(hazard_placement("saw_a", Some("saw_track")));

    let report = RoomBindings::default().sweep(&room);
    let named: Vec<_> = report
        .unresolved()
        .iter()
        .map(|u| (u.namespace, u.id.as_str(), u.declared_by.as_str()))
        .collect();
    // Report order is `(namespace, declared_by, id)` — deterministic by
    // construction (ADR 0023), so `motion path of…` precedes `npc patrol of…`.
    assert_eq!(
        named,
        vec![
            (
                "kinematic path",
                "saw_track",
                "motion path of hazard `saw_a`"
            ),
            ("kinematic path", "ledge_patrl", "npc patrol of `shopkeep`"),
        ],
    );
}

/// The poison for the row above: an NPC and a hazard that DO resolve — and a
/// pair that authored no path at all — must stay silent. A blank or absent
/// `path_id` is "this thing does not move", not a typo, and reporting it would
/// bury the two real failures under every stationary placement in the room.
#[test]
fn a_resolvable_or_unauthored_placement_path_is_not_reported() {
    let mut room = RoomSpec::new("quiet_refs", empty_world());
    room.kinematic_paths.push(KinematicPathSpec::new(
        "ledge_patrol",
        "Ledge Patrol",
        aabb(0.0, 0.0),
        ae::KinematicPath::line(ae::Vec2::ZERO, ae::Vec2::new(64.0, 0.0), 30.0),
    ));
    room.placements
        .push(npc_placement("shopkeep", Some("ledge_patrol")));
    // Addressed by the normalized display-name slug, the spelling the runtime
    // lookup table and `matches_id` share.
    room.placements
        .push(hazard_placement("saw_a", Some("ledge_patrol")));
    room.placements.push(npc_placement("statue", None));
    room.placements.push(hazard_placement("spikes", Some("  ")));

    let report = RoomBindings::default().sweep(&room);
    assert!(report.is_empty(), "nothing here is broken:\n{report}");
}
