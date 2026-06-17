//! Ambition's authored combat-banter lines.
//!
//! The registry type moved machinery-side
//! ([`ambition_gameplay_core::features::banter::CombatBanterRegistry`]) so
//! the combat hit path can read it; this module keeps WHAT the named
//! enemies say. Re-exported here so `crate::banter::CombatBanterRegistry`
//! paths keep working.

pub use ambition_gameplay_core::features::banter::CombatBanterRegistry;

/// Install pirate-cove combat barks for every pirate variant
/// (Admiral / Raider / Quartermaster / Lookout / Navigator and the
/// heavy bruisers Broadside Bess / Iron Mary / Salt Annet). The
/// peaceful crew use these lines when they go hostile via
/// `enemy_runtime_for_npc_combat`; the heavies (already `EnemyRuntime`) use
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
