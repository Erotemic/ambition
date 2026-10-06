//! Compile-time table of every published semantic body rig,
//! `(target, ron_text)` sorted by target. `build.rs` fills it with
//! `include_str!` from the full-resolution `assets/sprites` tree, so every
//! platform carries the same rigs and a headless build reads them with no file
//! access. The table is empty on a checkout that never published a rig.

include!(concat!(env!("OUT_DIR"), "/baked_body_rigs.rs"));

/// The published body rig of sheet target `target`, as RON text.
pub fn baked_body_rig(target: &str) -> Option<&'static str> {
    BAKED_BODY_RIGS
        .binary_search_by(|(name, _)| (*name).cmp(target))
        .ok()
        .map(|index| BAKED_BODY_RIGS[index].1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    /// A published rig states `looping` for each clip, and the animator has
    /// its own rule for each row ([`crate::character::row_loops`]). They are
    /// two authorities for one fact, and they do not agree.
    ///
    /// This holds the rows on which they disagree, by name, so that the set
    /// cannot grow in silence and so that the repair is seen. The landmark
    /// table of a package follows the animator, because the animator is what
    /// a player sees (queue row LANDMARK-CLIP-TIME).
    #[test]
    fn the_rows_a_rig_and_the_animator_time_differently_are_these() {
        if BAKED_BODY_RIGS.is_empty() {
            eprintln!("no published body rig on this checkout: nothing to compare");
            return;
        }
        let mut rows = 0;
        let mut differ = BTreeSet::new();
        for (target, text) in BAKED_BODY_RIGS {
            let rig = ambition_characters::actor::BodyRigDefinition::from_published_ron(text)
                .unwrap_or_else(|error| panic!("{target}: {error}"))
                .prepare()
                .unwrap_or_else(|error| panic!("{target}: {error:?}"));
            for name in rig.clip_names() {
                rows += 1;
                if rig.clip(name).expect("a named clip").looping != crate::character::row_loops(name) {
                    differ.insert(name.to_string());
                }
            }
        }
        eprintln!("rig and animator: {rows} rig clips, differ on {differ:?}");
        assert!(rows >= 20, "only {rows} rig clips were compared");
        let expected: BTreeSet<String> = EXPECTED.iter().map(|name| name.to_string()).collect();
        assert_eq!(differ, expected);
    }

    /// The rig holds each of these rows on its last frame and the animator
    /// loops it (measured 2026-10-05, eight rigs). The repair is one loop
    /// statement for each row that both read (queue row LANDMARK-CLIP-TIME).
    ///
    /// `stunned` is the other way round (2026-10-06, the T-rex): his rig loops
    /// it, and the animator has no name rule for it, so it calls it a
    /// one-shot. Neither times it: only the T-rex boss shows it, and his
    /// sprite and his rig both follow his module's PIN (`PinnedRow::looping`).
    const EXPECTED: [&str; 6] = ["crouch", "crouch_jump", "jump", "skid", "stunned", "taunt"];
}
