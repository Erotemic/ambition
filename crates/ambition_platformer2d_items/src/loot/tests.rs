//! Death drops must retire with their room and state their source identity.

use super::*;
use bevy::prelude::{App, Commands, Entity, Name, Or, Update, With, Without};

/// Spawn each supported death drop through the gameplay command path.
fn spawn_every_death_drop(mut commands: Commands) {
    let parent = SimId::placement("guard_body");
    let pos = ae::Vec2::new(32.0, 48.0);
    drop_currency_coin(
        &mut commands,
        SessionSpawnScope::UNSCOPED,
        &parent,
        "guard",
        pos,
        1,
    );
    drop_health_pickup(
        &mut commands,
        SessionSpawnScope::UNSCOPED,
        &parent,
        "guard",
        pos,
        1,
    );
    drop_ability_pickup(
        &mut commands,
        SessionSpawnScope::UNSCOPED,
        &parent,
        "guard",
        pos,
        "dash",
        "Dash",
    );
    drop_held_weapon(
        &mut commands,
        SessionSpawnScope::UNSCOPED,
        &parent,
        pos,
        ambition_characters::brain::HeldItemSpec {
            id: "guard_weapon".into(),
            melee: None,
            ranged: None,
            use_behavior: ambition_characters::brain::HeldUseBehavior::ThrowOnUse,
        },
        ae::Vec2::splat(16.0),
        "Guard weapon",
    );
}

/// THE INVARIANT: a death drop is room-scoped and names its parent.
///
/// Neither failure is tidiness, and they are different failures:
///
/// ```text
/// no RoomScopedEntity the roster a room CHANGE retires is `(With<RoomScopedEntity>, Without<InCustodyOf>)`, so a session-scoped drop is not in it and FOLLOWS YOU into the next room. For a pickup whose picture is a `RoomVisual` it is worse still: the sim keeps publishing a view nothing is drawing, so `draw_unclaimed_feature_views` mints a magenta stand-in every transition, forever — and those stand-ins are
/// what the room-transition cover waits on.
/// no SpawnOrigin `rebuild_dynamic_feature_views` discovers runtime-minted loot by construction PROVENANCE. A drop that states no parent was not in the room spec and cannot say so.
/// ```
#[test]
fn every_death_drop_is_room_scoped_and_states_its_parent() {
    let mut app = App::new();
    app.add_systems(Update, spawn_every_death_drop);
    app.update();

    let world = app.world_mut();
    // the population is BOTH collectible spellings, because the class is
    // "what a death drops" and not "what carries a `PickupFeature`" — the two
    // `GroundItem` drops are precisely the ones that were never checked.
    let mut drops = world.query_filtered::<(Entity, Option<&FeatureId>, Option<&Name>), Or<(
        With<PickupFeature>,
        With<ambition_held_items::GroundItem>,
    )>>();
    let dropped: Vec<(Entity, String)> = drops
        .iter(world)
        .map(|(entity, id, name)| {
            let label = id
                .map(|id| id.0.clone())
                .or_else(|| name.map(|name| name.to_string()))
                .unwrap_or_else(|| format!("{entity}"));
            (entity, label)
        })
        .collect();

    // the denominator, asserted rather than assumed: a drop function that
    // stopped spawning anything would otherwise make the checks below pass over
    // an empty population.
    assert_eq!(
        dropped.len(),
        4,
        "expected one drop from each grant operation, found {dropped:?}"
    );

    // ASK THE ROSTER, do not restate it. The question is not "does this
    // carry `RoomScopedEntity`" — it is *would a room change retire this?*, and
    // `RoomResident` is the production type the transition's own query is built
    // from. Naming the marker here would have been a second spelling of the rule
    // that stops agreeing with the first the day the roster gains a term.
    let mut residents = world
        .query_filtered::<Entity, ambition_platformer2d_shared_tangle::lifecycle::RoomResident>();
    let resident: std::collections::HashSet<Entity> = residents.iter(world).collect();
    let escapes_the_room: Vec<&str> = dropped
        .iter()
        .filter(|(entity, _)| !resident.contains(entity))
        .map(|(_, id)| id.as_str())
        .collect();
    assert!(
        escapes_the_room.is_empty(),
        "a drop outlives the room it fell in: {escapes_the_room:?} is not in the \
         `RoomResident` roster a room change retires, so it follows the player into the \
         next room at its old coordinates — see drop_currency_coin."
    );

    let anonymous: Vec<&str> = dropped
        .iter()
        .filter(|(entity, _)| {
            !matches!(
                world.get::<SpawnOrigin>(*entity),
                Some(SpawnOrigin::Dynamic { .. })
            )
        })
        .map(|(_, id)| id.as_str())
        .collect();
    assert!(
        anonymous.is_empty(),
        "a drop cannot state the body it fell out of: {anonymous:?} spawned without \
         SpawnOrigin::Dynamic, so the view rebuild that discovers runtime-minted loot \
         by provenance cannot see it — see dynamic_drop_origin."
    );
}

/// A DEATH DROP THAT BECOMES A CARRIABLE OBJECT NEEDS AN IDENTITY; ONE THAT
/// GRANTS A QUANTITY DOES NOT.
///
/// The three `PickupFeature` drops — coin, heart, ability — hand the player a
/// COUNT and vanish. `OwnedItems` is their durable record and a checkpoint
/// restores it wholesale (`OwnedItemsBaseline`), so an identity would be a
/// second authority over a fact the bag already settles.
///
/// The weapon is the one drop that becomes an object the player carries, and
/// every durable road that could give it back is keyed by `SimId`:
///
/// ```text
/// capture_minted_item_baseline   (&SimId, &SpawnOrigin, &GroundItem, &ItemCustody)
/// capture_custody_baseline       (&SimId, &InCustodyOf)
/// TransactionBaseline::capture   (Entity, &SimId, Option<&SpawnOrigin>)
/// ```
///
/// ⛔ So an anonymous weapon drop is invisible to all three: a checkpoint taken
/// while the player holds it has no description of it, and the death that
/// retires `SpawnedThisAttempt` destroys it with nothing able to rebuild it.
/// Every boss's signature gauntlet reaches the world down this road.
#[test]
fn only_the_death_drop_that_becomes_an_object_carries_an_identity() {
    let mut app = App::new();
    app.add_systems(Update, spawn_every_death_drop);
    app.update();

    let world = app.world_mut();
    let mut carried =
        world.query_filtered::<Option<&SimId>, With<ambition_held_items::GroundItem>>();
    let objects: Vec<Option<SimId>> = carried.iter(world).map(|id| id.cloned()).collect();
    assert_eq!(
        objects.len(),
        1,
        "one death drop becomes a carriable object; without it this measures nothing"
    );
    assert_eq!(
        objects[0].as_ref(),
        Some(&SimId::death_drop(
            &SimId::placement("guard_body"),
            "weapon"
        )),
        "the weapon a defeated body drops must carry the identity every durable \
         road that can give it back is keyed by — derived from the body it fell \
         out of, so a rewind of the death re-mints the same one"
    );

    let mut granted = world.query_filtered::<Option<&SimId>, (
        With<PickupFeature>,
        Without<ambition_held_items::GroundItem>,
    )>();
    let quantities: Vec<Option<SimId>> = granted.iter(world).map(|id| id.cloned()).collect();
    assert_eq!(
        quantities.len(),
        3,
        "coin, heart and ability grant quantities"
    );
    assert!(
        quantities.iter().all(Option::is_none),
        "a drop that grants a QUANTITY must stay anonymous: `OwnedItems` is its \
         durable record and an identity here would be a second authority over it"
    );
}
