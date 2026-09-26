//! Ambition's dormancy rule: how near an observer must be for a hostile to
//! keep thinking. The engine decides which bodies are hostiles that may sleep
//! (see `features::ecs::dormancy::wake_radius`); this crate states the distance.

use bevy::prelude::*;

use ambition_combat::scoped_rules::{DeclareRulesExt, RulesScope};
use ambition_platformer2d_actor_monolith::features::ecs::dormancy::DormancyRule;

/// Wake radius for roaming Ambition hostiles.
///
/// This is derived from the controlled body's fastest repeated traversal
/// (blink), not ordinary run speed, with roughly 2.4 seconds of lead time. The
/// radius is observer-motion policy and should not be tuned down merely to make
/// dormancy trigger inside today's room sizes.
pub const AMBITION_WAKE_RADIUS: f32 = 2560.0;

/// State the rule for Ambition's own rooms, which carry no mode tag.
pub fn register(app: &mut App) {
    app.declare_rules(
        RulesScope::UntaggedRooms,
        DormancyRule {
            hostile_wake_radius: AMBITION_WAKE_RADIUS,
        },
    );
}
