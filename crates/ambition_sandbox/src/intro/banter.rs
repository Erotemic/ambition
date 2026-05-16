//! Intro combat-banter lines.
//!
//! Story content for the [`crate::banter::CombatBanterRegistry`]:
//! one-liners that the two intro raid antagonists yell when the
//! player strikes them. Faction-flavored so the wrong-list narrative
//! beat keeps landing during combat (it doesn't only live in the
//! intro_raid cutscene, which the player may have skipped).
//!
//! Installed by [`crate::intro::plugin::install_intro_banter_system`]
//! at startup — the sandbox-side registry stays empty by default so a
//! plain sandbox build doesn't carry intro content.

use crate::banter::CombatBanterRegistry;

/// Bulk-install every intro-NPC's combat barks. Idempotent at the
/// per-name level — re-running replaces the line list for each
/// matching name.
pub fn install_intro_banter(registry: &mut CombatBanterRegistry) {
    // Framebreaker (anti-machine hardliner). Lines hammer the
    // anti-AI angle. `Clanker` is the deliberate slur the design
    // doc reserves for hostile hardliners.
    registry.set_hit_barks(
        "Framebreaker",
        vec![
            "Kill the Clanker before it learns our names!",
            "No more replacement gods!",
            "It does not have to be alive to ruin lives!",
            "Stay down, scrap!",
        ],
    );
    // Nazi salvage guard. Lines tilt order-following + the
    // wrong-list realization the cutscene plants — the player
    // hears them out loud during combat in case the cutscene was
    // skipped.
    registry.set_hit_barks(
        "Nazi Salvage Guard",
        vec![
            "Hold your line!",
            "This one isn't on the manifest — then manifest it!",
            "Burn the notebooks!",
            "Take anything that boots!",
        ],
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn install_intro_banter_registers_both_raiders() {
        let mut reg = CombatBanterRegistry::default();
        install_intro_banter(&mut reg);
        assert!(reg.pick_hit_bark("Framebreaker", 0).is_some());
        assert!(reg.pick_hit_bark("Nazi Salvage Guard", 0).is_some());
    }

    #[test]
    fn intro_banter_uses_clanker_slur_for_framebreaker_only() {
        // Design doc constraint: `Clanker` only from hostile
        // anti-machine hardliners, never neutral NPCs.
        let mut reg = CombatBanterRegistry::default();
        install_intro_banter(&mut reg);
        let framebreaker_lines = reg
            .on_hit
            .get("Framebreaker")
            .expect("framebreaker hit barks missing");
        assert!(
            framebreaker_lines.iter().any(|l| l.contains("Clanker")),
            "Framebreaker barks should include the Clanker slur"
        );
        let nazi_lines = reg
            .on_hit
            .get("Nazi Salvage Guard")
            .expect("nazi guard hit barks missing");
        assert!(
            !nazi_lines.iter().any(|l| l.contains("Clanker")),
            "Nazi Salvage Guard barks must NOT use the Clanker slur"
        );
    }
}
