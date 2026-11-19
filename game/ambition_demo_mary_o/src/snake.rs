//! Mary-O's Solid Snake: its identity, its art, and where its shell is.
//!
//! A stomp changes the same live body into an inert shell rather than killing
//! or replacing it; a kicked shell slides and runs down what it meets; the
//! snake comes back out. That rule is a module
//! (`ambition_demo_mary_o_modules::shell`, in `body_contact`). It keeps the
//! shell's phase in the snake's record, which [`shell_of`] reads for a test
//! or an inspector, and it reads her pack's shell and stomp rules through the
//! observations this crate answers (`mary_o.rules.shell`,
//! `mary_o.rules.stomp`).

use bevy::prelude::*;

use ambition_platformer2d::entity_catalog::placements::CharacterBrain;

pub use ambition_demo_mary_o_modules::shell::{SnakeShell, SNAKE_BRAIN_KEY};

/// The `solid_snake` sheet TARGET (also the catalog id) — the generated sheet the
/// enemy render resolves for a Solid Snake.
pub const SNAKE_SHEET_TARGET: &str = "solid_snake";

// `SNAKE_TILE_COLUMNS` is GONE. Where a snake patrols is authored.
//
// its SIZE stays, and the difference is the rule: how big a snake is comes
// from its sheet; where it patrols comes from the level.

// ⛔⛤ `snake_half_size` WAS HERE, AND CONSTRUCTION MADE IT REDUNDANT —
// 2026-09-21. It resolved the sheet's idle rectangle so a tag pass could write
// it onto an already-built body, and its own doc claimed that kept the body
// from popping on its first tick. It did not: the engine had already built the
// body at the CATALOG's size, so writing this here produced the pop rather
// than preventing it. The character now declares
// `BodySource::SpriteAuthored`, and the seed resolves the same rectangle
// before the body exists. Deleting the second answer is the repair; keeping
// both in sync was never going to be.

/// Identify snakes by their authored `CharacterBrain` archetype key.
///
/// `ActorConfig.brain` is a read-model of authored identity, so callers should
/// not substitute display names or generated feature-id conventions.
pub fn is_snake_brain(brain: &CharacterBrain) -> bool {
    matches!(brain, CharacterBrain::Custom(key) if key == SNAKE_BRAIN_KEY)
}

// ⛔⛔ `reset_snakes_on_room_reset` WAS HERE, AND CANONICAL RECONSTRUCTION MADE
// IT REDUNDANT. A replay is a room REBUILD: snakes are room-scoped authored
// actors, so the rebuild retires them and re-spawns them walking, with a fresh
// record. `replay_rebuilds_the_snakes.rs` is the cover.

/// Where `snake`'s shell is: its record's phase, or a walker's while it has
/// no record yet (the shell module has not run for it). `None` for a body
/// that is gone.
pub fn shell_of(world: &World, snake: Entity) -> Option<SnakeShell> {
    let body = world.get_entity(snake).ok()?;
    Some(
        body.get::<ambition_platformer2d::extension::BodyRecords>()
            .and_then(|records| records.get(&ambition_demo_mary_o_modules::shell::SHELL))
            .map(SnakeShell::of_record)
            .unwrap_or_default(),
    )
}

/// Whether the shell module has run for `snake`. It states the snake's
/// contact threat on each tick (`ambition.body.contact_threat`), and no other
/// writer states a snake's: a snake does not fade. (Its record is no witness:
/// the host stores no record that is still a walker's initial one.)
pub fn is_served(world: &World, snake: Entity) -> bool {
    world.get::<ambition_platformer2d::combat::actor_tuning::ContactThreatWithdrawn>(snake).is_some()
}

/// Every Solid Snake body, each with where its shell is, in entity order.
pub fn snakes(world: &mut World) -> Vec<(Entity, SnakeShell)> {
    let mut bodies = world.query::<(Entity, &ambition_platformer2d::combat::actor_tuning::ActorConfig)>();
    let mut found: Vec<Entity> =
        bodies.iter(world).filter(|(_, config)| is_snake_brain(&config.brain)).map(|(body, _)| body).collect();
    found.sort();
    found.into_iter().filter_map(|body| Some((body, shell_of(world, body)?))).collect()
}

/// The body credited for `snake`'s running shell: the one that kicked it,
/// while that body is still here.
pub fn shell_credit(world: &mut World, snake: Entity) -> Option<Entity> {
    use ambition_demo_mary_o_modules::shell::SHELL;
    let id = world
        .get::<ambition_platformer2d::extension::BodyRecords>(snake)?
        .get(&SHELL)?
        .get(ambition_platformer2d::extension::sdk::FieldRef(3))
        .ok()?
        .as_opt()
        .flatten()?
        .as_ref_id()?
        .to_owned();
    let mut bodies = world.query::<(Entity, &ambition_platformer2d::platformer::sim_id::SimId)>();
    bodies.iter(world).find(|(_, sim)| sim.as_str() == id).map(|(body, _)| body)
}

/// Put `snake`'s shell in `phase`, credited to `kicker` (its simulation
/// identity), as a fixture does.
pub fn set_shell(world: &mut World, snake: Entity, phase: SnakeShell, kicker: Option<Entity>) {
    use ambition_demo_mary_o_modules::shell::{shell_schema, SHELL};
    use ambition_platformer2d::extension::sdk::{FieldRef, Value};
    let credit = kicker
        .and_then(|kicker| world.get::<ambition_platformer2d::platformer::sim_id::SimId>(kicker))
        .map(|sim| Box::new(Value::Ref(sim.as_str().to_owned())));
    let mut body = world.entity_mut(snake);
    let mut records = body.get::<ambition_platformer2d::extension::BodyRecords>().cloned().unwrap_or_default();
    let schema = shell_schema();
    let mut record = records.get(&SHELL).cloned().unwrap_or_else(|| schema.initial_record());
    phase.write(&mut record).expect("the shell record holds each phase");
    record.set(FieldRef(3), Value::Opt(credit)).expect("the shell record holds a credit");
    records.put(SHELL, schema.shape_digest(), record);
    body.insert(records);
}

#[cfg(test)]
mod tests;
