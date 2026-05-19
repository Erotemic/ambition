//! Boss combat-banter lines.
//!
//! Scholar-on-shoulder quotes for GNU-ton: pedantic, recursive, self-referential.
//! Lines should sound like someone who has read every GNU manual and missed the point.

use crate::content::banter::CombatBanterRegistry;

/// Register hit-bark lines for all boss encounters.
pub fn install_boss_banter(registry: &mut CombatBanterRegistry) {
    // GNU-ton — the scholar atop the giant wildebeest.
    // Hit barks: pompous, recursive, free-software militant.
    // Name key must match the LDtk BossSpawn entity name exactly.
    registry.set_hit_barks(
        "GNU-ton",
        vec![
            "That's GPL v3, not v2! Read the license!",
            "I wrote a 47-page rebuttal to that attack!",
            "I can see further than everyone else — standing on the shoulders of giants!",
            "It's not Linux, it's GNU slash Linux!",
            "My wildebeest is free as in freedom — free as in freedom!",
            "I was going to finish the Hurd kernel, but then you hit me!",
            "Violence is closed source!",
            "I have a recursive name. You have a recursive kick.",
            "Have you considered contributing a patch instead?",
        ],
    );
}
