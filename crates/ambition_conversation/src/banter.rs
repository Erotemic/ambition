//! Combat-banter registry (generic half).
//!
//! ⭐ IT LIVES HERE BECAUSE A BARK IS THE SHORTEST CONVERSATION THERE IS.
//! The registry is a name → lines table with no combat semantics at all; what
//! made it look like actor machinery was only that the hit path reads it. Moved
//! out of the actor monolith 2026-08-28 (D33): it was 63 lines depending on
//! nothing but `std` and bevy, and it was one of the four concepts standing
//! between `features/ecs/damage` and its own crate.
//!
//! The hit path reads it; the authored line sets (pirate barks, intro raiders,
//! boss banter) are content and populate it from `ambition_content`.

use std::collections::HashMap;

use bevy::prelude::*;

#[derive(Resource, Default, Debug, Clone)]
pub struct CombatBanterRegistry {
    /// Lines an enemy yells when hit. Indexed by enemy display
    /// name. The line picked rotates with strike count to avoid repetition.
    pub on_hit: HashMap<String, Vec<&'static str>>,
    /// Lines an actor mutters periodically during a fight even when
    /// not being hit. Used by the boss idle-bark ticker so the giant
    /// has personality between strikes.
    pub idle: HashMap<String, Vec<&'static str>>,
}

impl CombatBanterRegistry {
    /// Pick a hit-bark line for the named enemy based on a rotation
    /// counter (typically derived from the enemy's hit count). Returns
    /// `None` if the enemy has no registered lines — the combat
    /// system silently skips the bubble in that case.
    pub fn pick_hit_bark(&self, enemy_name: &str, rotation: u32) -> Option<&'static str> {
        pick_line(&self.on_hit, enemy_name, rotation)
    }

    /// Bulk-register a set of hit-bark lines for one enemy name.
    /// Overwrites any existing entry for that name.
    pub fn set_hit_barks(&mut self, enemy_name: impl Into<String>, lines: Vec<&'static str>) {
        self.on_hit.insert(enemy_name.into(), lines);
    }

    /// Pick an idle-bark line by name + rotation counter. Same shape
    /// as `pick_hit_bark` so the caller can use a simple per-actor
    /// tick counter (e.g. number of idle barks fired so far).
    pub fn pick_idle_bark(&self, name: &str, rotation: u32) -> Option<&'static str> {
        pick_line(&self.idle, name, rotation)
    }

    /// Bulk-register idle barks for one actor. Overwrites any
    /// existing entry.
    pub fn set_idle_barks(&mut self, name: impl Into<String>, lines: Vec<&'static str>) {
        self.idle.insert(name.into(), lines);
    }
}

fn pick_line(
    table: &HashMap<String, Vec<&'static str>>,
    name: &str,
    rotation: u32,
) -> Option<&'static str> {
    let lines = table.get(name)?;
    if lines.is_empty() {
        return None;
    }
    Some(lines[(rotation as usize) % lines.len()])
}
