//! Combat-banter registry (generic half).
//!
//! The registry TYPE + pick/set mechanics live machinery-side so the
//! combat hit path (`crate::features::ecs::damage`) can read it; the
//! authored line sets (pirate barks, intro raiders, boss banter) are
//! content and populate it via plugin startup systems in the
//! `ambition_content` crate.

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
