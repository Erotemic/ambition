//! Sanic's repertoire, for the stage he visits rather than the one he lives
//! on.
//!
//! A move table is what the attack is; the ability is whether this body may
//! attack at all. At home the answer is no and these sixteen moves are
//! unreachable. On a stage that grants the verb (`MatchAbilities::levelled`)
//! they are what he swings.
//!
//! This is not his spin dash. `declare_sanic_techniques` puts spin dash and
//! the transform on his body as techniques, and they stay there. The side
//! special in his move file is a separate move that looks like one, so a crossover stage
//! gets a signature move without two authorities owning it.
//!
//! ## The character
//!
//! Speed, and the cost of it. The fastest startups on the grid after the Shadow
//! Oni's, the least commitment of anybody — almost nothing here has a locked
//! tail — and the weakest single hit in the game. He does not win an exchange;
//! he has three before you finish one.
//!
//! The table is content: `assets/data/movesets/sanic.ron`, loaded through
//! [`crate::pack::PACK`]. The tests here read it there.

#[cfg(test)]
mod tests {
    use ambition_entity_catalog::WindowTag;

    // The move schema refuses a verb bound to a move the table does not define.
    // That every press is answered, in every posture it is asked in, is checked
    // by the host ratchet
    // `smash_roster_movesets::report_the_smash_kit_every_selectable_fighter_has`.

    /// Speed is the whole character, and it is checkable: every grounded
    /// normal starts in under seven frames at 60Hz.
    #[test]
    fn his_normals_come_out_faster_than_a_tenth_of_a_second() {
        let moveset = crate::pack::PACK.moveset(crate::SANIC_CHARACTER_ID);
        for id in ["quick_jab", "run_up_kick", "heel_flick", "skid"] {
            let startup = moveset
                .move_by_id(id)
                .unwrap_or_else(|| panic!("{id} exists"))
                .windows
                .iter()
                .find(|w| matches!(w.tag, WindowTag::Active))
                .expect("a strike has an active window")
                .start_s;
            assert!(
                startup <= 0.06,
                "`{id}` starts in {startup}s, which is not a fast character"
            );
        }
    }
}
