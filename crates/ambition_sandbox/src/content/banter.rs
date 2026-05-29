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
//!   `apply_feature_hit_events` instead of relying on whatever
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

/// Install pirate-cove combat barks for every pirate variant
/// (Admiral / Raider / Quartermaster / Lookout / Navigator and the
/// heavy bruisers Broadside Bess / Iron Mary / Salt Annet). The
/// peaceful crew use these lines when they go hostile via
/// `hostile_from_npc`; the heavies (already `EnemyRuntime`) use
/// them when the player strikes them. Keep flavor consistent with
/// the matching arms in `npc_hit_barks` / `npc_hostile_bark` — the
/// two paths fire on different actor types but should sound like
/// the same character.
pub fn install_pirate_banter(registry: &mut CombatBanterRegistry) {
    registry.set_hit_barks(
        "Pirate Admiral",
        vec![
            "Belay that, ye barnacle!",
            "Mind the epaulettes, scallywag!",
            "Avast — that be admiralty property!",
            "I'll keelhaul yer cooldowns!",
        ],
    );
    registry.set_hit_barks(
        "Pirate Raider",
        vec![
            "Yarrrgh!",
            "Quit pokin' me loot hand!",
            "I'll swab the floor with ye!",
            "Yo-ho-NO, ye landlubber!",
        ],
    );
    registry.set_hit_barks(
        "Pirate Quartermaster",
        vec![
            "Inventory says NO, ye dock-rat!",
            "Yarr! Every coin's a-counted!",
            "Tally that on yer hide, swabbie!",
        ],
    );
    registry.set_hit_barks(
        "Pirate Lookout",
        vec![
            "Land ho — an' I see YE comin'!",
            "Spyglass to me eye, boots to yer head!",
            "Crow's nest don't sit empty, savvy?",
        ],
    );
    registry.set_hit_barks(
        "Pirate Navigator",
        vec![
            "Wrong heading, ye chartless dog!",
            "I'll plot ye a course straight to Davy Jones!",
            "Compass says: punch back!",
        ],
    );
    registry.set_hit_barks(
        "Broadside Bess",
        vec![
            "Mind me cleaver, wee skipper!",
            "Aye, that smarts — but ye're worse off!",
            "Broadside Bess don't bend easy!",
            "Yarrrr! Take that an' a barrel more!",
        ],
    );
    registry.set_hit_barks(
        "Iron Mary",
        vec![
            "Iron don't flinch, ye gull!",
            "Pry harder, swab — I'll rust ye flat!",
            "Yo-ho, an' a clout to the noggin!",
            "Try me on a calmer sea, landlubber!",
        ],
    );
    registry.set_hit_barks(
        "Salt Annet",
        vec![
            "Salt in the eye, blood in the bilge!",
            "Yargh! Watch yer manners on me deck!",
            "Wee skipper thinks he's bold, does he?",
            "Annet bites back, every time!",
        ],
    );
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
