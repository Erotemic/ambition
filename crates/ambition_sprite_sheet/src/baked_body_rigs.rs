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
