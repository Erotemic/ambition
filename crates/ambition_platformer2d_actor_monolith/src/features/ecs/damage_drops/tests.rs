//! Guard the room ownership and source identity of every death drop.
//!
//! One test checks the spawned components in a real `World`; another scans the
//! full damage path to ensure every collectible-spawning function is included in
//! that invariant table. Both are required: component checks need coverage, and
//! coverage alone does not validate the spawned state.

use super::*;
use bevy::prelude::{App, Commands, Entity, Name, Or, Update, With};

/// Every collectible-spawning function covered by the death-drop invariant.
const DEATH_DROPS_UNDER_GUARD: &[&str] = &[
    "drop_currency_coin",
    "drop_health_pickup",
    "drop_ability_pickup",
    "drop_held_weapon",
];

/// Damage-path sources scanned at compile time for coverage of collectible
/// spawns. Drops outside this path belong to a different owner.
const DAMAGE_PATH_SRC: &[(&str, &str)] = &[
    ("damage_drops.rs", include_str!("../damage_drops.rs")),
    ("damage/mod.rs", include_str!("../damage/mod.rs")),
    ("damage/actor_hit.rs", include_str!("../damage/actor_hit.rs")),
    ("damage/boss_hit.rs", include_str!("../damage/boss_hit.rs")),
];

/// Fire every drop in [`DEATH_DROPS_UNDER_GUARD`] once, through the same
/// `Commands` path the damage systems use.
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
        With<crate::items::pickup::GroundItem>,
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
        DEATH_DROPS_UNDER_GUARD.len(),
        "expected one drop per function in DEATH_DROPS_UNDER_GUARD, found {dropped:?}"
    );

    // ASK THE ROSTER, do not restate it. The question is not "does this
    // carry `RoomScopedEntity`" — it is *would a room change retire this?*, and
    // `RoomResident` is the production type the transition's own query is built
    // from. Naming the marker here would have been a second spelling of the rule
    // that stops agreeing with the first the day the roster gains a term.
    let mut residents = world.query_filtered::<Entity, ambition_platformer2d_shared_tangle::lifecycle::RoomResident>();
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

/// THE COVERAGE HALF: the table names every function in the damage path that
/// drops a collectible.
///
/// Red the moment a further drop function is written, whatever it is called,
/// wherever in the damage path it lives, and whichever collectible it spawns. It
/// says nothing about components — it only refuses to let the invariant above go
/// stale.
///
/// it reports the enclosing TOP-LEVEL function, so an inline drop written
/// back into a damage system surfaces as `apply_actor_hit` rather than as a name
/// that could be added to the table. That is intended: the fix for one is to
/// move it beside its siblings, not to widen the table.
#[test]
fn the_death_drop_table_is_complete() {
    let mut defined: Vec<String> = DAMAGE_PATH_SRC
        .iter()
        .flat_map(|(_, src)| collectible_drop_fns(src))
        .collect();
    defined.sort();
    defined.dedup();
    let mut guarded: Vec<String> = DEATH_DROPS_UNDER_GUARD
        .iter()
        .map(|name| (*name).to_string())
        .collect();
    guarded.sort();

    assert_eq!(
        defined, guarded,
        "the damage path defines a different set of collectible-dropping functions than \
         DEATH_DROPS_UNDER_GUARD names. Add the new one to the table AND to \
         spawn_every_death_drop — that is what puts it under the guard. If the extra name \
         is a damage SYSTEM, it is spawning a drop inline: move it into damage_drops.rs, \
         which is the one place the class rule is spelled."
    );
}

/// Every top-level function in `src` whose body constructs a collectible.
///
/// Line-oriented on purpose: top-level items start at column zero, so a `fn` at
/// column zero opens a new body and everything indented under it belongs to that
/// body. Cheap and total over these modules' shape; if one ever grows nested
/// helpers that drop, this scan reports the enclosing top-level function, which
/// is still the right unit for the table.
///
/// two spellings, because a collectible has two shapes. A `PickupFeature`
/// is walked over and granted; a `GroundItem` is picked up and wielded. Watching
/// only the first is what let two drops sit outside this guard.
fn collectible_drop_fns(src: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut current: Option<String> = None;
    let mut body_drops_a_collectible = false;

    let close = |current: &mut Option<String>, drops: &mut bool, found: &mut Vec<String>| {
        if *drops {
            if let Some(name) = current.take() {
                found.push(name);
            }
        }
        *current = None;
        *drops = false;
    };

    for line in src.lines() {
        let signature = ["pub fn ", "pub(crate) fn ", "pub(super) fn ", "fn "]
            .iter()
            .find_map(|prefix| line.strip_prefix(prefix));
        if let Some(rest) = signature {
            close(&mut current, &mut body_drops_a_collectible, &mut found);
            let name = rest.split(['(', '<']).next().unwrap_or(rest);
            current = Some(name.trim().to_string());
        } else if line.contains("PickupFeature::new(") || line.contains("GroundItem {") {
            body_drops_a_collectible = true;
        }
    }
    close(&mut current, &mut body_drops_a_collectible, &mut found);
    found
}
