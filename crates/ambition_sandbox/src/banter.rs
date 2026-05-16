//! Combat-banter registry.
//!
//! Stores one-liner sets keyed by enemy name so the combat hit path
//! can fire `VfxMessage::SpeechBubble` when the player strikes a
//! named enemy. Authored content (e.g. intro raiders shouting
//! "wrong list" lines) populates the registry via a plugin startup
//! system — [`crate::intro::banter`] is the first such contributor.
//!
//! Why a resource (not a `const HashMap`):
//! - story content lives in submodules so the future game / sandbox
//!   crate split can move authoring out of sandbox without touching
//!   the combat hit path,
//! - reload-friendly: a future hot-reload story-content loader can
//!   replace the resource at runtime,
//! - testable: insert a synthetic registry in unit tests of
//!   `apply_feature_damage_events` instead of relying on whatever
//!   the default plugin registered.
//!
//! Pattern: when a named enemy is hit, the combat system calls
//! [`CombatBanterRegistry::pick_hit_bark`] which returns one of the
//! enemy's lines (rotated by strike count so the same line doesn't
//! repeat every frame). The system only fires a bubble on the
//! *first* hit per non-overlapping-hit-flash window, so the rate is
//! limited by the existing visual hit-flash cooldown — no extra
//! bookkeeping needed.

use std::collections::HashMap;

use bevy::prelude::*;

#[derive(Resource, Default, Debug, Clone)]
pub struct CombatBanterRegistry {
    /// Lines an enemy yells when hit. Indexed by enemy display
    /// name (matches `EnemyRuntime::name`). The line picked rotates
    /// with strike count to avoid repetition.
    pub on_hit: HashMap<String, Vec<&'static str>>,
}

impl CombatBanterRegistry {
    /// Pick a hit-bark line for the named enemy based on a rotation
    /// counter (typically derived from the enemy's hit count). Returns
    /// `None` if the enemy has no registered lines — the combat
    /// system silently skips the bubble in that case.
    pub fn pick_hit_bark(&self, enemy_name: &str, rotation: u32) -> Option<&'static str> {
        let lines = self.on_hit.get(enemy_name)?;
        if lines.is_empty() {
            return None;
        }
        let idx = (rotation as usize) % lines.len();
        Some(lines[idx])
    }

    /// Bulk-register a set of hit-bark lines for one enemy name.
    /// Overwrites any existing entry for that name.
    pub fn set_hit_barks(&mut self, enemy_name: impl Into<String>, lines: Vec<&'static str>) {
        self.on_hit.insert(enemy_name.into(), lines);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pick_hit_bark_returns_none_when_enemy_unknown() {
        let reg = CombatBanterRegistry::default();
        assert!(reg.pick_hit_bark("Unknown", 0).is_none());
    }

    #[test]
    fn pick_hit_bark_rotates_through_lines() {
        let mut reg = CombatBanterRegistry::default();
        reg.set_hit_barks("Test", vec!["a", "b", "c"]);
        assert_eq!(reg.pick_hit_bark("Test", 0), Some("a"));
        assert_eq!(reg.pick_hit_bark("Test", 1), Some("b"));
        assert_eq!(reg.pick_hit_bark("Test", 2), Some("c"));
        // Wraps via modulo.
        assert_eq!(reg.pick_hit_bark("Test", 3), Some("a"));
        assert_eq!(reg.pick_hit_bark("Test", 7), Some("b"));
    }

    #[test]
    fn pick_hit_bark_returns_none_for_empty_list() {
        let mut reg = CombatBanterRegistry::default();
        reg.set_hit_barks("Empty", vec![]);
        assert!(reg.pick_hit_bark("Empty", 0).is_none());
    }
}
