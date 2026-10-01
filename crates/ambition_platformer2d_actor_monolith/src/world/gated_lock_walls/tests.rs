//! the App-level test below builds its own world, which is the shape that
//! can pass with production wiring absent. It is here because the invariant it
//! pins is a *cache invalidation* one, and provoking a hot-reload against a live
//! host is a much worse test than provoking it against three resources.

use super::*;

use ambition_platformer2d_ldtk::{
    ActiveLdtkProject, LdtkEntityInstance, LdtkFieldInstance, LdtkLayerInstance, LdtkLevel,
    LdtkProject,
};
use ambition_platformer2d_shared_tangle::authored_logic::PublishCondition;
use ambition_platformer2d_shared_tangle::feature_overlay::FeatureEcsWorldOverlay;
use serde_json::Value;

const FLAG: &str = "bob_field_survey_received";

fn field(identifier: &str, value: &str) -> LdtkFieldInstance {
    LdtkFieldInstance {
        identifier: identifier.into(),
        value: Value::String(value.into()),
        real_editor_values: vec![Value::Null],
    }
}

/// One level whose `activeArea` is `alice_relay`, holding one `LockWall`.
///
/// `gated_by` present by default: the interesting negative case is a wall
/// WITHOUT it, and a fixture whose default is the boring case makes the negative
/// test a one-line edit.
fn project_with_one_wall(gated_by: Option<&str>) -> LdtkProject {
    let mut fields = vec![
        field("id", "alice_private_return_lock"),
        field("name", "wall"),
    ];
    if let Some(gated_by) = gated_by {
        fields.push(field("gated_by", gated_by));
    }
    LdtkProject {
        json_version: "1.5.3".into(),
        levels: vec![LdtkLevel {
            identifier: "alice_relay".into(),
            iid: "level-iid".into(),
            world_x: 0,
            world_y: 0,
            px_wid: 1024,
            px_hei: 768,
            field_instances: vec![field("activeArea", "alice_relay")],
            layer_instances: vec![LdtkLayerInstance {
                identifier: "Ambition".into(),
                layer_type: "Entities".into(),
                c_wid: 64,
                c_hei: 48,
                grid_size: 16,
                entity_instances: vec![
                    // a real level has one and the converter REFUSES an area without it ("no
                    // PlayerStart").
                    LdtkEntityInstance {
                        iid: "PlayerStart-test-alice".into(),
                        identifier: "PlayerStart".into(),
                        pivot: vec![0.0, 0.0],
                        px: [96, 96],
                        width: 16,
                        height: 16,
                        field_instances: Vec::new(),
                    },
                    LdtkEntityInstance {
                        iid: "LockWall-test-alice".into(),
                        identifier: "LockWall".into(),
                        pivot: vec![0.0, 0.0],
                        px: [800, 624],
                        width: 96,
                        height: 112,
                        field_instances: fields,
                    },
                ],
                int_grid_csv: Vec::new(),
                grid_tiles: Vec::new(),
            }],
        }],
    }
}

/// The fixture project, CONVERTED — the road production takes.
///
/// Converting here means a converter that stops emitting either field fails in these tests.
fn room_with_one_wall(
    gated_by: Option<&str>,
    room_id: &str,
) -> ambition_platformer2d_world::rooms::RoomSpec {
    project_with_one_wall(gated_by)
        .to_room_set_with_entry(
            "alice_relay",
            &ambition_platformer2d_ldtk::LdtkVocabulary::engine(),
        )
        .unwrap_or_else(|errors| panic!("fixture converts to rooms: {errors:?}"))
        .rooms
        .into_iter()
        .find(|room| room.id == room_id)
        .unwrap_or_else(|| {
            ambition_platformer2d_world::rooms::RoomSpec::new(
                room_id,
                ambition_platformer2d_core::World::new(
                    room_id,
                    ambition_platformer2d_core::Vec2::new(1024.0, 768.0),
                    ambition_platformer2d_core::Vec2::new(96.0, 96.0),
                    Vec::new(),
                ),
            )
        })
}

/// The walk finds an authored gated wall, with its footprint.
#[test]
fn an_authored_gated_wall_is_found_with_its_footprint() {
    let walls = authored_gated_lock_walls(&room_with_one_wall(Some(FLAG), "alice_relay"));
    assert_eq!(walls.len(), 1);
    assert_eq!(walls[0].id, "alice_private_return_lock");
    assert_eq!(walls[0].gated_by, FLAG);
    assert_eq!(
        walls[0].min,
        ambition_platformer2d_core::Vec2::new(800.0, 624.0)
    );
    assert_eq!(
        walls[0].size,
        ambition_platformer2d_core::Vec2::new(96.0, 112.0)
    );
}

/// A `LockWall` with no `gated_by` is not this system's business.
///
/// Encounter walls — the other consumer of `LockWall` — are exactly the walls that carry no
/// `gated_by`, and they must keep working.
#[test]
fn a_wall_with_no_authored_gate_is_left_to_its_other_consumer() {
    assert!(authored_gated_lock_walls(&room_with_one_wall(None, "alice_relay")).is_empty());
}

/// Only the active room's walls.
#[test]
fn walls_in_another_room_are_not_found() {
    assert!(authored_gated_lock_walls(&room_with_one_wall(Some(FLAG), "drain_alley")).is_empty());
}

/// A world with the system, its inputs, and the one condition it asks.
fn world_with_one_gated_wall() -> App {
    world_with_one_wall_gated_by(FLAG)
}

/// The same world, with the authored `gated_by` string as the parameter.
///
/// ⭐ THE STRING IS THE SUBJECT NOW. `gated_by` is an authored condition LINE,
/// so "what may an author write here" is the thing under test and it cannot be
/// a constant in the fixture.
fn world_with_one_wall_gated_by(gated_by: &str) -> App {
    let mut app = App::new();
    app.insert_resource(ActiveLdtkProject(project_with_one_wall(Some(gated_by))));
    app.insert_resource(ambition_persistence::save::AmbitionGameSave::default());
    ambition_platformer2d_shared_tangle::lifecycle::insert_live_room_component(app.world_mut(), FeatureEcsWorldOverlay::default());
    // the world-fact domain's own condition, published exactly as its plugin
    // publishes it. The system under test never names a flag.
    app.publish_condition(
        crate::world_facts::flag_set_descriptor(),
        crate::world_facts::flag_set,
    );
    ambition_platformer2d_world::rooms::insert_room_set(
        app.world_mut(),
        ambition_platformer2d_world::rooms::RoomSet::from_parts_or_panic(
            "alice_relay",
            vec![room_with_one_wall(Some(gated_by), "alice_relay")],
            Vec::new(),
        ),
    );
    // Mirror the production ordering contract: the overlay rebuild clears
    // `gate_solids` each frame BEFORE this system re-contributes.
    app.add_systems(
        Update,
        (
            |mut overlays: ambition_platformer2d_shared_tangle::feature_overlay::RoomOverlays| {
                for mut overlay in overlays.each() {
                    overlay.gate_solids.clear();
                    overlay.gate_passes.clear();
                }
            },
            sync_authored_gated_lock_walls,
        )
            .chain(),
    );
    app
}

fn standing(app: &App) -> usize {
    ambition_platformer2d_shared_tangle::lifecycle::sole_live_room_component::<FeatureEcsWorldOverlay>(app.world()).expect("the live room has a collision overlay")
        .gate_solids
        .len()
}

/// The bodies each gate is open for, by block name (Q54).
fn open_for(app: &App) -> Vec<(String, Vec<Entity>)> {
    ambition_platformer2d_shared_tangle::lifecycle::sole_live_room_component::<FeatureEcsWorldOverlay>(app.world())
        .expect("the live room has a collision overlay")
        .gate_passes
        .iter()
        .map(|pass| (pass.block.clone(), pass.bodies.clone()))
        .collect()
}

/// THE WALL STANDS UNTIL ITS CONDITION IS SATISFIED, AND THEN IT IS GONE.
///
/// nothing here reads a flag. The system asks `world.flag_set`, the world-fact
/// domain answers, and the wall follows — which is the whole reason this stopped
/// being a const table.
#[test]
fn the_wall_stands_until_its_authored_condition_is_satisfied() {
    let mut app = world_with_one_gated_wall();
    app.update();
    assert_eq!(standing(&app), 1, "the flag is clear, so the wall is up");

    app.world_mut()
        .resource_mut::<ambition_persistence::save::AmbitionGameSave>()
        .data_mut()
        .set_flag(FLAG, true);
    app.update();
    assert_eq!(
        standing(&app),
        0,
        "the condition is satisfied; the wall opens"
    );
}

/// A REPLACED ROOM SET INVALIDATES THE CACHE.
///
/// this is the regression the original cache shipped WITHOUT, and it is
/// carried across deliberately: a hot reload that swaps the authored source
/// under an unchanged room id and save state kept serving walls computed from
/// data that is no longer loaded.
#[test]
fn swapping_the_room_set_alone_invalidates_the_cached_walls() {
    let mut app = world_with_one_gated_wall();
    app.update();
    assert_eq!(standing(&app), 1);

    // A quiet frame keeps serving the cached wall — this is what makes the next
    // assertion about invalidation rather than about recomputation.
    app.update();
    assert_eq!(standing(&app), 1);

    // Same room id, same save, different authored content.
    {
        let mut rooms = app.world_mut().query_filtered::<
            &mut ambition_platformer2d_world::rooms::RoomSet,
            bevy::prelude::With<ambition_platformer2d_shared_tangle::lifecycle::SessionRoot>,
        >();
        let mut set = rooms
            .iter_mut(app.world_mut())
            .next()
            .expect("the fixture installs a room set");
        for room in &mut set.rooms {
            room.lock_walls.clear();
        }
    }
    app.update();
    assert_eq!(
        standing(&app),
        0,
        "the wall set must track the replaced room set"
    );
}

/// A QUESTION THAT CANNOT BE PREPARED YET LEAVES THE WALL STANDING — AND IS
/// RETRIED.
///
/// ⭐⭐ THE CACHE HOLDS A PREPARED QUESTION NOW, and preparation happens when the
/// room is cached. That buys a wall a `PreparedCondition` instead of a freshly
/// minted argument every frame — and it introduces an ORDER the old code could
/// not have: a provider that registers AFTER the first room is cached would have
/// left its walls holding `None` forever, which is a gate that never opens
/// because of startup sequence rather than because of the world.
///
/// ⛔ BOTH ARMS, because one cannot show the rule: the wall must STAND while the
/// question is unpreparable (the same safe direction an unanswerable question
/// takes) and must OPEN once the provider arrives and the flag is set. A test
/// that only checked the first would pass over a permanent `None`.
#[test]
fn a_wall_whose_question_cannot_be_prepared_yet_stands_until_the_catalog_moves() {
    use ambition_platformer2d_shared_tangle::authored_logic::ConditionCatalog;

    let mut app = App::new();
    app.insert_resource(ActiveLdtkProject(project_with_one_wall(Some(FLAG))));
    app.insert_resource(ambition_persistence::save::AmbitionGameSave::default());
    ambition_platformer2d_shared_tangle::lifecycle::insert_live_room_component(app.world_mut(), FeatureEcsWorldOverlay::default());
    // A catalog that exists and does NOT publish `world.flag_set`. The system
    // returns early when there is no catalog at all, so an empty one is what puts
    // the fixture in the state under test rather than past it.
    app.init_resource::<ConditionCatalog>();
    ambition_platformer2d_world::rooms::insert_room_set(
        app.world_mut(),
        ambition_platformer2d_world::rooms::RoomSet::from_parts_or_panic(
            "alice_relay",
            vec![room_with_one_wall(Some(FLAG), "alice_relay")],
            Vec::new(),
        ),
    );
    app.add_systems(
        Update,
        (
            |mut overlays: ambition_platformer2d_shared_tangle::feature_overlay::RoomOverlays| {
                for mut overlay in overlays.each() {
                    overlay.gate_solids.clear();
                    overlay.gate_passes.clear();
                }
            },
            sync_authored_gated_lock_walls,
        )
            .chain(),
    );

    app.update();
    assert_eq!(
        standing(&app),
        1,
        "nobody can answer this wall's question yet, so it must stay up — a gate \
         that opened because its provider had not registered would open in \
         exactly the situations where the world is least well understood"
    );

    // ⭐ THE PROVIDER ARRIVES, AND THAT EDGE IS WHAT RE-PREPARES. `publish_condition`
    // mutates `ConditionCatalog`, the cache is keyed on that resource's change
    // tick, so the question is prepared once here rather than re-parsed on every
    // tick for every wall — which is what this system used to do to cover exactly
    // this case. If the cache ever stops watching the catalog, this goes red.
    app.publish_condition(
        crate::world_facts::flag_set_descriptor(),
        crate::world_facts::flag_set,
    );
    app.world_mut()
        .resource_mut::<ambition_persistence::save::AmbitionGameSave>()
        .data_mut()
        .set_flag(FLAG, true);
    app.update();
    assert_eq!(
        standing(&app),
        0,
        "the provider registered after the room was cached and the wall never \
         retried its preparation, so it stands forever on a satisfied condition"
    );
}

/// A STANDING WALL EXPLAINS ITSELF AS STRUCTURE (M5): the term, the flag it
/// names and the flag's state are readable off `GatedLockWallVerdicts` without
/// parsing a log — and the explanation follows the wall down.
#[test]
fn a_standing_wall_says_which_flag_is_unset_and_stops_saying_it_when_it_opens() {
    let mut app = world_with_one_gated_wall();
    app.update();
    assert_eq!(standing(&app), 1, "premise: the wall is up");
    let verdicts = app.world().resource::<GatedLockWallVerdicts>();
    let (wall_id, why) = verdicts
        .by_wall
        .iter()
        .find_map(|(id, verdict)| verdict.why_not().map(|why| (id.clone(), why.clone())))
        .expect("a standing wall publishes a structured why-not");
    assert_eq!(why.term, "world.flag_set");
    assert_eq!(
        why.subject, FLAG,
        "the why-not names the flag the wall waits on"
    );
    assert!(
        why.observed.contains("no such flag"),
        "and the flag's state in the domain's words: {}",
        why.observed
    );

    app.world_mut()
        .resource_mut::<ambition_persistence::save::AmbitionGameSave>()
        .data_mut()
        .set_flag(FLAG, true);
    app.update();
    assert_eq!(standing(&app), 0, "premise: the wall opened");
    let verdicts = app.world().resource::<GatedLockWallVerdicts>();
    assert!(
        verdicts.why_standing(&wall_id).is_none(),
        "an open wall has nothing to explain: {:?}",
        verdicts.by_wall.get(&wall_id)
    );
}

/// A WALL MAY ASK ANY PUBLISHED CONDITION, NOT ONLY `world.flag_set`.
///
/// ⛔ THIS IS THE GATE FAMILY THAT WAS PUBLISHED AND UNREACHABLE.
/// `inventory.holds` has shipped as a production condition for as long as the
/// bag has, and `prepare_question` hardcoded the condition ID to
/// `world.flag_set` while passing the authored string as its ARGUMENT — so no
/// route could ask it, however well it was written. The fix is in the FIELD's
/// data shape, not in a new condition.
///
/// The wall is gated on carrying an axe, so this also pins the direction: an
/// empty bag leaves the wall UP, and acquiring the item takes it down. A test
/// that only asserted the open half would pass against a wall that never stood.
#[test]
fn a_wall_may_be_gated_on_an_item_the_player_carries() {
    use ambition_platformer2d_shared_tangle::authored_logic::PublishCondition;

    let mut app = world_with_one_wall_gated_by("inventory.holds axe");
    // An EMPTY bag, not the starter bag: the starter carries health cells and
    // this wall must be able to be up at all.
    app.insert_resource(ambition_items::OwnedItems::default());
    app.publish_condition(
        crate::items::conditions::holds_descriptor(),
        crate::items::conditions::holds,
    );

    app.update();
    assert_eq!(
        standing(&app),
        1,
        "the bag is empty, so the wall the author gated on carrying an axe stands"
    );

    app.world_mut()
        .resource_mut::<ambition_items::OwnedItems>()
        .grant(ambition_items::Item::Axe, 1);
    app.update();
    assert_eq!(
        standing(&app),
        0,
        "the player is carrying the axe the wall asked about; it opens"
    );
}

/// ⭐ A ROUTE GATED ON WHAT THE BODY CAN DO — the fifth gate family, walked end
/// to end for the first time.
///
/// `body.can` was published and unit-tested against a hand-built world, and
/// `gated_by` was widened to name a condition. Neither fact says the ROAD
/// works: a condition nothing authored can reach is a condition that lands
/// dead, and a unit test that calls `can()` directly cannot witness the wall
/// never asking. This walks the whole chain — an authored `gated_by` line, the
/// syntactic discriminator, `ConditionCatalog::prepare_line`, the descriptor's
/// `Name` parameter, `AbilitySet` — and the only thing it changes between the
/// two assertions is a bool on the body.
///
/// ⛔ THE BODY ARRIVES WITHOUT THE VERB, which is the half that can fail
/// silently. A wall that was never up cannot be observed opening, so the closed
/// arm is asserted first and from a body that EXISTS: "nobody to ask" and "the
/// body cannot do it" are different answers and only the second one is this
/// test's subject.
#[test]
fn a_wall_may_be_gated_on_what_the_body_can_do() {
    use ambition_platformer2d_shared_tangle::authored_logic::PublishCondition;

    let mut app = world_with_one_wall_gated_by("body.can wall_climb");
    app.publish_subject_condition(
        crate::body_conditions::can_descriptor(),
        crate::body_conditions::can,
        crate::body_conditions::can_for,
    );
    let body = app
        .world_mut()
        .spawn((
            ambition_platformer2d_core::body_clusters::BodyAbilities::new(
                ambition_platformer2d_core::abilities::AbilitySet::default(),
            ),
            // ⚠ A SEAT, NOT JUST A MARKER. `body.can` asks the body the
            // participant is DRIVING, because possession moves the seat off the
            // home avatar; a fixture holding `PlayerEntity` alone describes a
            // body nobody is driving and the wall would correctly refuse to ask
            // it. Same reason the dormancy fixtures spawn a seat.
            ambition_characters::control::DrivingParticipant(
                ambition_characters::control::PlayerSlot::PRIMARY,
            ),
        ))
        .id();

    app.update();
    assert_eq!(
        (standing(&app), open_for(&app)),
        (1, Vec::new()),
        "the body is here and cannot climb, so the wall the author gated on \
         climbing stands for it"
    );

    app.world_mut()
        .entity_mut(body)
        .get_mut::<ambition_platformer2d_core::body_clusters::BodyAbilities>()
        .expect("the body was just spawned with one")
        .abilities
        .wall_climb = true;
    app.update();
    assert_eq!(
        (standing(&app), open_for(&app)),
        (1, vec![(format!("{GATED_LOCK_BLOCK_PREFIX}alice_private_return_lock"), vec![body])]),
        "the same body can climb now; the route opens for it, and the wall \
         stands for every other body (Q54)"
    );
}

/// ⭐ A ROUTE GATED ON BODY SIZE — the sixth gate family, and the one the plan's
/// goal names first: *"gate routes through body size."*
///
/// The crawlspace. Nothing changes between the two assertions but the body's
/// height, and the route it may take changes with it — which is the whole claim
/// of the capability-progression program in one test: exploration follows from
/// what the body physically IS, not from a story flag someone remembered to set.
#[test]
fn a_wall_may_be_gated_on_the_body_being_small_enough_to_pass() {
    use ambition_platformer2d_shared_tangle::authored_logic::PublishCondition;
    use ambition_platformer2d_core::body_clusters::BodyKinematics;

    let mut app = world_with_one_wall_gated_by("body.fits 32");
    app.publish_subject_condition(
        crate::body_conditions::fits_descriptor(),
        crate::body_conditions::fits,
        crate::body_conditions::fits_for,
    );
    let mut standing_body = BodyKinematics::default();
    standing_body.size.y = 64.0;
    let body = app
        .world_mut()
        .spawn((
            standing_body,
            // ⚠ A SEAT, NOT JUST A MARKER. `body.fits` asks the body the
            // participant is DRIVING, because possession moves the seat off the
            // home avatar; a fixture holding `PlayerEntity` alone describes a
            // body nobody is driving and the wall would correctly refuse to ask
            // it. Same reason the dormancy fixtures spawn a seat.
            ambition_characters::control::DrivingParticipant(
                ambition_characters::control::PlayerSlot::PRIMARY,
            ),
        ))
        .id();

    app.update();
    assert_eq!(
        (standing(&app), open_for(&app)),
        (1, Vec::new()),
        "a body twice the opening's height does not get through it"
    );

    app.world_mut()
        .entity_mut(body)
        .get_mut::<BodyKinematics>()
        .expect("the body was just spawned with one")
        .size
        .y = 30.0;
    app.update();
    assert_eq!(
        (standing(&app), open_for(&app)),
        (1, vec![(format!("{GATED_LOCK_BLOCK_PREFIX}alice_private_return_lock"), vec![body])]),
        "the same body, low enough now, and the crawlspace is open for it"
    );
}

/// GATE-PER-ACTOR (Q54): in one live room, the wall is open for the body that
/// satisfies it and solid for the body beside it that does not, in the same
/// tick. Two seats, one that climbs and one that does not; the population
/// question ("does a driven body climb?") says yes, and that is not what opens
/// the wall for the second body. The control: when both climb, both pass.
#[test]
fn a_body_gate_is_open_only_for_the_bodies_that_satisfy_it() {
    use ambition_platformer2d_core::body_clusters::BodyAbilities;
    use ambition_characters::control::{DrivingParticipant, PlayerSlot};

    let mut app = world_with_one_wall_gated_by("body.can wall_climb");
    app.publish_subject_condition(
        crate::body_conditions::can_descriptor(),
        crate::body_conditions::can,
        crate::body_conditions::can_for,
    );
    let mut climbs = ambition_platformer2d_core::abilities::AbilitySet::default();
    climbs.wall_climb = true;
    let alice = app
        .world_mut()
        .spawn((BodyAbilities::new(climbs), DrivingParticipant(PlayerSlot::PRIMARY)))
        .id();
    let bob = app
        .world_mut()
        .spawn((
            BodyAbilities::new(ambition_platformer2d_core::abilities::AbilitySet::default()),
            DrivingParticipant(PlayerSlot(1)),
        ))
        .id();
    let wall = format!("{GATED_LOCK_BLOCK_PREFIX}alice_private_return_lock");

    app.update();
    assert_eq!(
        (standing(&app), open_for(&app)),
        (1, vec![(wall.clone(), vec![alice])]),
        "the wall is open for Alice, who climbs, and solid for Bob, who does not"
    );

    app.world_mut()
        .get_mut::<BodyAbilities>(bob)
        .expect("Bob's body")
        .abilities
        .wall_climb = true;
    app.update();
    let mut both = vec![alice, bob];
    both.sort();
    assert_eq!(
        (standing(&app), open_for(&app)),
        (1, vec![(wall, both)]),
        "control: both climb, and the wall is open for both"
    );
}



/// ⭐ A ROUTE GATED ON A WORLD MECHANISM — the seventh gate family, in its
/// boolean form.
///
/// The door past the arena. `world.switch_on` is not `world.flag_set` with a
/// different name: a flag is a story fact something recorded about the player, a
/// switch is a mechanism's own state, and the input here is already flowing —
/// clearing a wave encounter latches every switch linked to it.
///
/// ⛔ THE WALL MUST BE UP FIRST, from a save that EXISTS. An empty save and no
/// save at all are different answers (`NotSatisfied` versus `Unanswerable`) and
/// only the first is this test's subject; the fixture installs a default save,
/// so the closed arm means "the mechanism is off" and not "nobody could answer".
#[test]
fn a_wall_may_be_gated_on_a_world_mechanism_being_latched_on() {
    use ambition_platformer2d_shared_tangle::authored_logic::PublishCondition;

    let mut app = world_with_one_wall_gated_by("world.switch_on arena_switch_north");
    app.publish_condition(
        crate::world_facts::switch_on_descriptor(),
        crate::world_facts::switch_on,
    );

    app.update();
    assert_eq!(
        standing(&app),
        1,
        "the arena switch has never been latched, so the door past it stands"
    );

    app.world_mut()
        .resource_mut::<ambition_persistence::save::AmbitionGameSave>()
        .data_mut()
        .set_switch("arena_switch_north", true);
    app.update();
    assert_eq!(
        standing(&app),
        0,
        "the mechanism is on now; the route it holds shut opens"
    );
}

/// A ROUTE GATED ON AN ENCOUNTER OUTCOME — and the arm that separates it from
/// the switch.
///
/// A wave encounter's completion latches every switch linked to it, so
/// `world.switch_on` and `encounter.cleared` usually agree. ⛔ THEY MUST NOT BE
/// THE SAME QUESTION: the switch is the mechanism, resettable by a reset switch
/// the player can walk up to; this is the outcome the save recorded. A door
/// gated on having cleared the arena must not reopen because somebody reset the
/// lights, so the middle assertion here sets the SWITCH and requires the wall to
/// stay standing.
#[test]
fn a_wall_gated_on_an_encounter_outcome_does_not_answer_to_its_switch() {
    use ambition_platformer2d_shared_tangle::authored_logic::PublishCondition;

    let mut app = world_with_one_wall_gated_by("encounter.cleared goblin_encounter");
    app.publish_condition(
        ambition_encounter_features::conditions::cleared_descriptor(),
        ambition_encounter_features::conditions::cleared,
    );

    app.update();
    assert_eq!(standing(&app), 1, "nothing has been fought yet");

    app.world_mut()
        .resource_mut::<ambition_persistence::save::AmbitionGameSave>()
        .data_mut()
        .set_switch("goblin_encounter_reset_switch", true);
    app.update();
    assert_eq!(
        standing(&app),
        1,
        "the mechanism's lights are green and the encounter is still unfinished; \
         these are two facts and this wall asked for the second"
    );

    app.world_mut()
        .resource_mut::<ambition_persistence::save::AmbitionGameSave>()
        .data_mut()
        .set_encounter(
            "goblin_encounter",
            ambition_persistence::save_data::PersistedEncounterState::Cleared,
        );
    app.update();
    assert_eq!(standing(&app), 0, "cleared; the route past it opens");
}

/// AN AUTHORED CONDITION THAT DOES NOT EXIST LEAVES THE WALL STANDING.
///
/// ⛔ AND IT IS NOT DEMOTED TO A FLAG LOOKUP. `names_its_own_condition` is
/// syntactic on purpose: a value shaped like a condition id IS a condition
/// reference, so a misspelt one reaches the catalog's diagnostic instead of
/// silently becoming `world.flag_set("inventry.holds axe")` — a question that
/// would also never be satisfied, with nothing said about why.
#[test]
fn a_condition_the_catalog_does_not_publish_leaves_the_wall_up() {
    let mut app = world_with_one_wall_gated_by("inventry.holds axe");
    app.update();
    assert_eq!(
        standing(&app),
        1,
        "the condition does not exist, so the question cannot be prepared and \
         the wall must not open"
    );
}

/// ⛔ THE VERDICTS DESCRIBE THE ACTIVE ROOM, SO A ROOM WITH NO WALLS RETRACTS
/// THEM.
///
/// The publication sits after a fast return taken whenever the wall cache is
/// empty — the common case — so a room with gated walls followed by a room with
/// none left the first room's map standing. Every consumer of
/// `GatedLockWallVerdicts` reads it as "why is the route in front of you
/// blocked", and in the second room the honest answer is "nothing is". A
/// consumer that showed the previous room's reason would be naming a flag the
/// player cannot act on in a room where no wall waits on it.
///
/// ⭐ THIS IS THE SIBLING OF `swapping_the_room_set_alone_invalidates_the_cached_walls`,
/// which pins the OVERLAY half of the same edge. Both halves are needed: the
/// wall came down there while its verdict stayed up.
#[test]
fn a_room_with_no_gated_walls_retracts_the_previous_rooms_verdicts() {
    let mut app = world_with_one_gated_wall();
    app.update();
    assert_eq!(standing(&app), 1, "premise: the wall is up");
    assert!(
        !app.world().resource::<GatedLockWallVerdicts>().by_wall.is_empty(),
        "premise: a standing wall published its reason"
    );

    // The same swap the cache-invalidation test makes: same room id, same save,
    // authored content with no walls left in it.
    {
        let mut rooms = app.world_mut().query_filtered::<
            &mut ambition_platformer2d_world::rooms::RoomSet,
            bevy::prelude::With<ambition_platformer2d_shared_tangle::lifecycle::SessionRoot>,
        >();
        let mut set = rooms
            .iter_mut(app.world_mut())
            .next()
            .expect("the fixture installs a room set");
        for room in &mut set.rooms {
            room.lock_walls.clear();
        }
    }
    app.update();

    assert_eq!(standing(&app), 0, "premise: no wall stands here");
    assert!(
        app.world().resource::<GatedLockWallVerdicts>().by_wall.is_empty(),
        "a room with no gated walls says so: {:?}",
        app.world().resource::<GatedLockWallVerdicts>().by_wall
    );
}

/// ⛔ A ROUTE GATED ON THE BODY ASKS THE BODY THE PARTICIPANT IS DRIVING.
///
/// `body.can` is a published condition an author may write into `gated_by`, so
/// possession is a PRODUCTION property of the route, not a unit-test property of
/// the reader: while the participant drives a vessel that cannot climb, a wall
/// gated on climbing must stand even though the home avatar left behind can.
/// The condition's own tests pin the answer; this pins that the WALL follows it.
///
/// ⭐ BOTH DIRECTIONS in one fixture pair, because a reader that answered
/// `NotSatisfied` unconditionally would pass the first half alone.
#[test]
fn a_wall_gated_on_the_body_follows_the_vessel_the_participant_drives() {
    fn app_with_possession(home_climbs: bool, vessel_climbs: bool) -> App {
        let mut app = world_with_one_wall_gated_by("body.can wall_climb");
        app.publish_condition(
            crate::body_conditions::can_descriptor(),
            crate::body_conditions::can,
        );
        let abilities = |climbs: bool| {
            let mut set = ambition_platformer2d_core::abilities::AbilitySet::default();
            set.wall_climb = climbs;
            ambition_platformer2d_core::body_clusters::BodyAbilities::new(set)
        };
        // The home avatar keeps `PlayerEntity`; possession moved
        // `DrivingParticipant` onto the vessel. See `control::authority`.
        app.world_mut().spawn((
            abilities(home_climbs),
            ambition_platformer2d_shared_tangle::markers::PlayerEntity,
        ));
        let driven = app
            .world_mut()
            .spawn((
                abilities(vessel_climbs),
                ambition_characters::control::DrivingParticipant(
                    ambition_characters::control::PlayerSlot::PRIMARY,
                ),
            ))
            .id();
        app.insert_resource(
            ambition_platformer2d_shared_tangle::markers::ControlledSubject(Some(driven)),
        );
        app
    }

    let mut climbing_home = app_with_possession(true, false);
    climbing_home.update();
    assert_eq!(
        standing(&climbing_home),
        1,
        "the participant is driving a vessel that cannot climb, so the climbing \
         body it left behind must not open this route"
    );

    let mut climbing_vessel = app_with_possession(false, true);
    climbing_vessel.update();
    assert_eq!(
        standing(&climbing_vessel),
        0,
        "the driven vessel climbs, so the wall opens for it"
    );
}

/// ⭐⛤ **THE WALL NAMES ITSELF IN THE VERDICT RING — THE OPEN HALF OF M5,
/// CLOSED 2026-09-21.**
///
/// The ring recorded `world.flag_set("alice_can_return") -> no` and nothing
/// about WHO asked, so a reader could not tell one wall's question from a
/// dialogue node's, a shop line's, or a second wall gated on the same flag —
/// and *"why is this door shut"* meant grepping the level for everything that
/// mentions the flag.
///
/// ⚠ THIS DRIVES THE PRODUCTION SYSTEM. `sync_authored_gated_lock_walls` is
/// what builds the `AuthoredAsk`; a test that constructed one itself would be
/// asserting that a struct literal holds what it was given.
#[test]
fn the_ring_says_which_wall_asked() {
    use ambition_platformer2d_shared_tangle::authored_logic::AuthoredVerdictLog;

    let mut app = world_with_one_gated_wall();
    app.world_mut().insert_resource(AuthoredVerdictLog::default());
    app.update();

    let log = app.world().resource::<AuthoredVerdictLog>();
    let asked: Vec<String> = log
        .recent()
        .iter()
        .map(|verdict| verdict.asked_by().to_string())
        .collect();
    assert!(
        !asked.is_empty(),
        "the wall asked nothing, so this arm measures no source at all"
    );
    assert!(
        asked
            .iter()
            .any(|source| source == "lock_wall:alice_private_return_lock"),
        "the ring does not say which wall asked: {asked:?}"
    );
}

/// OW1 cut 7q: each live room's gated walls stand in that room's overlay.
/// With two rooms live, the gate once read the sole live room's definition,
/// found none, and retracted: the overlay rebuild then cleared every room's
/// `gate_solids`, so every gated wall in both rooms was gone.
///
/// #0 is `drain_alley`, which authors no gated wall; #1 is `alice_relay`,
/// which authors one. The wall stands in #1 only, and opens when its flag is
/// set.
#[test]
fn each_live_room_keeps_its_own_gated_walls() {
    use ambition_platformer2d_shared_tangle::lifecycle::{
        insert_live_room_component, spawn_live_room, LiveRoomInstance,
    };
    let mut app = App::new();
    app.insert_resource(ActiveLdtkProject(project_with_one_wall(Some(FLAG))));
    app.insert_resource(ambition_persistence::save::AmbitionGameSave::default());
    app.publish_condition(
        crate::world_facts::flag_set_descriptor(),
        crate::world_facts::flag_set,
    );
    let set = ambition_platformer2d_world::rooms::RoomSet::from_parts_or_panic(
        "drain_alley",
        vec![
            room_with_one_wall(Some(FLAG), "drain_alley"),
            room_with_one_wall(Some(FLAG), "alice_relay"),
        ],
        Vec::new(),
    );
    let relay = set.definition_by_id("alice_relay").expect("alice_relay is in the set");
    ambition_platformer2d_world::rooms::insert_room_set(app.world_mut(), set);
    let first = insert_live_room_component(app.world_mut(), FeatureEcsWorldOverlay::default());
    let second = spawn_live_room(app.world_mut(), LiveRoomInstance::ACTIVATION.next(), relay);
    app.world_mut().entity_mut(second).insert(FeatureEcsWorldOverlay::default());
    app.add_systems(
        Update,
        (
            |mut overlays: ambition_platformer2d_shared_tangle::feature_overlay::RoomOverlays| {
                for mut overlay in overlays.each() {
                    overlay.gate_solids.clear();
                    overlay.gate_passes.clear();
                }
            },
            sync_authored_gated_lock_walls,
        )
            .chain(),
    );
    let gates = |app: &App, root| {
        app.world()
            .get::<FeatureEcsWorldOverlay>(root)
            .expect("each live room has a collision overlay")
            .gate_solids
            .len()
    };

    app.update();
    assert_eq!(
        (gates(&app, first), gates(&app, second)),
        (0, 1),
        "(#0's gated walls, #1's): the flag is clear, so #1's wall is up and #0 has none"
    );

    app.world_mut()
        .resource_mut::<ambition_persistence::save::AmbitionGameSave>()
        .data_mut()
        .set_flag(FLAG, true);
    app.update();
    assert_eq!(
        (gates(&app, first), gates(&app, second)),
        (0, 0),
        "(#0's gated walls, #1's): the flag is set, so #1's wall opens"
    );
}
