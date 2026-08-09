//! **A drop belongs to the room it fell in** — the class guard for the parent
//! module's collectible drops.
//!
//! ⛔ **this deliberately does NOT assert "the three drop functions insert
//! `RoomScopedEntity`".** That check goes green the day a fourth drop function
//! is added without the marker, which is exactly how `drop_ability_pickup` was
//! shipped session-scoped for months while its two siblings carried the fix and
//! a 19-line comment stating the rule. The guard is therefore two halves that
//! only work together:
//!
//! 1. [`every_dropped_pickup_is_room_scoped`] — the INVARIANT. It spawns the
//!    drops into a real `World` and asks the World, not the source, whether
//!    each one is room-scoped. ⚠ asking the World is the point: `RoomVisual`
//!    reaches `RoomScopedEntity` through `#[require]`, so a component can be
//!    present without its name appearing at the spawn site, and a grep-shaped
//!    guard would false-accuse the day a drop acquires it that way.
//! 2. [`the_pickup_drop_table_is_complete`] — the COVERAGE. It scans the parent
//!    module's own source for every function that spawns a `PickupFeature` and
//!    fails if one is missing from the table half 1 drives.
//!
//! ⇒ **a fourth drop function cannot be added quietly.** Adding one turns half 2
//! red on the day it is written; the only way back to green is to put it in the
//! table, at which point half 1 starts checking its lifetime scope. Neither half
//! survives on its own — half 1 alone is blind to new functions, half 2 alone
//! never looks at a component.

use super::*;
use bevy::prelude::{App, Commands, Entity, Update, With};

/// **Every drop function in the parent module that spawns a collectible.**
///
/// Kept in sync with the module by [`the_pickup_drop_table_is_complete`] rather
/// than by discipline — see the module comment for why a hand-maintained list is
/// safe here and would not be on its own.
const PICKUP_DROPS_UNDER_GUARD: &[&str] = &[
    "drop_currency_coin",
    "drop_health_pickup",
    "drop_ability_pickup",
];

/// The parent module's own source, read at COMPILE time so the coverage scan
/// cannot drift from the code that actually compiled.
const DAMAGE_DROPS_SRC: &str = include_str!("../damage_drops.rs");

/// Fire every drop in [`PICKUP_DROPS_UNDER_GUARD`] once, through the same
/// `Commands` path the damage systems use.
fn spawn_every_pickup_drop(mut commands: Commands) {
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
}

/// **THE INVARIANT: a sim entity that publishes a pickup view is room-scoped.**
///
/// The failure this pins is not tidiness. A session-scoped drop whose picture is
/// a `RoomVisual` (and therefore room-scoped) leaves the sim publishing a Pickup
/// view for an entity nothing is drawing, so `draw_unclaimed_feature_views`
/// mints a magenta stand-in for it in the NEXT room, every transition, forever —
/// and those stand-ins are what the room-transition cover waits on. Jon's
/// 2026-08-05 log is an 8-second black screen from exactly this.
#[test]
fn every_dropped_pickup_is_room_scoped() {
    let mut app = App::new();
    app.add_systems(Update, spawn_every_pickup_drop);
    app.update();

    let world = app.world_mut();
    let mut dropped_pickups = world
        .query_filtered::<(Entity, &FeatureId), (With<FeatureSimEntity>, With<PickupFeature>)>();
    let dropped: Vec<(Entity, String)> = dropped_pickups
        .iter(world)
        .map(|(entity, id)| (entity, id.0.clone()))
        .collect();

    // ⚠ the denominator, asserted rather than assumed: a drop function that
    // stopped spawning anything would otherwise make the marker check below
    // pass over an empty population.
    assert_eq!(
        dropped.len(),
        PICKUP_DROPS_UNDER_GUARD.len(),
        "expected one dropped pickup per function in PICKUP_DROPS_UNDER_GUARD, \
         found {dropped:?}"
    );

    let session_scoped_only: Vec<&str> = dropped
        .iter()
        .filter(|(entity, _)| world.get::<RoomScopedEntity>(*entity).is_none())
        .map(|(_, id)| id.as_str())
        .collect();
    assert!(
        session_scoped_only.is_empty(),
        "a drop outlives its own picture: {session_scoped_only:?} spawned without \
         RoomScopedEntity, but every feature visual is a RoomVisual and therefore \
         room-scoped. Two lifetimes for one thing is the bug — see drop_currency_coin."
    );
}

/// **THE COVERAGE HALF: the table names every function that drops a pickup.**
///
/// Red the moment a fourth drop function is written, whatever it is called and
/// whatever it does or does not insert. It says nothing about components — it
/// only refuses to let the invariant above go stale.
#[test]
fn the_pickup_drop_table_is_complete() {
    let mut defined = pickup_drop_fns(DAMAGE_DROPS_SRC);
    defined.sort();
    let mut guarded: Vec<String> = PICKUP_DROPS_UNDER_GUARD
        .iter()
        .map(|name| (*name).to_string())
        .collect();
    guarded.sort();

    assert_eq!(
        defined, guarded,
        "the parent module defines a different set of pickup-dropping functions than \
         PICKUP_DROPS_UNDER_GUARD names. Add the new one to the table AND to \
         spawn_every_pickup_drop — that is what puts its lifetime scope under the guard."
    );
}

/// Every top-level function in `src` whose body constructs a `PickupFeature`.
///
/// Line-oriented on purpose: top-level items start at column zero, so a `fn` at
/// column zero opens a new body and everything indented under it belongs to that
/// body. Cheap and total over this module's shape; if the module ever grows
/// nested helpers that drop pickups, this scan reports the enclosing top-level
/// function, which is still the right unit for the table.
fn pickup_drop_fns(src: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut current: Option<String> = None;
    let mut body_drops_a_pickup = false;

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
            close(&mut current, &mut body_drops_a_pickup, &mut found);
            let name = rest.split(['(', '<']).next().unwrap_or(rest);
            current = Some(name.trim().to_string());
        } else if line.contains("PickupFeature::new(") {
            body_drops_a_pickup = true;
        }
    }
    close(&mut current, &mut body_drops_a_pickup, &mut found);
    found
}
