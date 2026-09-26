//! Mary-O's platform-fighter repertoire.
//!
//! Her home character catalog grants `RunJump`, which excludes attack; the
//! moveset therefore does not grant combat capability by itself. A host that
//! grants attack can use this light, quick kit with strong down-air pressure.
//!
//! The table is content: `assets/data/movesets/mary_o.ron`, loaded through
//! [`crate::pack::PACK`]. The tests here read it there.

#[cfg(test)]
mod tests {

    // The move schema refuses a verb bound to a move the table does not define.
    // That every press is answered, in every posture it is asked in, is checked
    // by the host ratchet
    // `smash_roster_movesets::report_the_smash_kit_every_selectable_fighter_has`.

    /// THE STOMP IS HER HARDEST HIT, which is the identity claim the module
    /// doc makes and the one a retune would quietly lose.
    #[test]
    fn the_down_air_hits_harder_than_anything_else_she_has() {
        let moveset = crate::pack::PACK.moveset(crate::provider::MARY_O_CHARACTER_ID);
        let damage = |id: &str| {
            moveset
                .move_by_id(id)
                .unwrap_or_else(|| panic!("{id} exists"))
                .windows
                .iter()
                .flat_map(|w| w.volumes.iter())
                .map(|v| v.damage)
                .max()
                .unwrap_or(0)
        };
        let stomp = damage("stomp");
        for other in ["shell_kick", "block_punch", "ground_pound", "fireball"] {
            assert!(
                stomp >= damage(other),
                "`{other}` hits harder than the stomp, so the platformer \
                 protagonist's best move is no longer falling on somebody"
            );
        }
    }
}
