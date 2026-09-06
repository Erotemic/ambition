//! Intro combat-banter lines.
//!
//! Story content for the [`crate::banter::CombatBanterRegistry`]:
//! one-liners that the two intro raid antagonists yell when the
//! player strikes them. Raid-flavored so the wrong-list narrative
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
    // Generic intro raiders: short, readable barks that keep the
    // wrong-list pressure without front-loading later factions.
    registry.set_hit_barks(
        "Lab Raider",
        vec![
            "Contain the prototype!",
            "Keep it away from the consoles!",
            "No one leaves with the core!",
            "Hold the west line!",
        ],
    );
    registry.set_hit_barks(
        "Salvage Guard",
        vec![
            "Hold your line!",
            "This one isn't on the manifest. Tag it and move!",
            "Secure the notebooks!",
            "Take only marked equipment!",
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
        assert!(reg.pick_hit_bark("Lab Raider", 0).is_some());
        assert!(reg.pick_hit_bark("Salvage Guard", 0).is_some());
    }

    #[test]
    fn intro_banter_preserves_wrong_list_pressure() {
        let mut reg = CombatBanterRegistry::default();
        install_intro_banter(&mut reg);
        let joined = reg
            .on_hit
            .values()
            .flatten()
            .copied()
            .collect::<Vec<_>>()
            .join("\n");
        assert!(joined.contains("manifest"));
        assert!(joined.contains("prototype"));
    }
}
