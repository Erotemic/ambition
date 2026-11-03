//! The table of every published semantic body rig, `(target, ron_text)`
//! sorted by target, from the full-resolution `assets/sprites` tree. Embedded
//! by `build.rs` where there is no source tree (`static_content`, wasm,
//! Android); read from the tree in a development build. The table is empty on
//! a checkout that never published a rig. It is a part of the current sheet
//! generation ([`crate::baked_sheet_rons::current`]), so a development build
//! can publish an edited rig with its sheets.



/// Every published body rig, as `(target, ron_text)` sorted by target.
pub fn body_rigs(content: &crate::SpriteContent) -> &[(&'static str, std::sync::Arc<str>)] {
    crate::baked_sheet_rons::current(&content).body_rigs()
}

/// The published body rig of target `target`, as RON text.
pub fn baked_body_rig<'a>(content: &'a crate::SpriteContent, target: &str) -> Option<&'a str> {
    crate::baked_sheet_rons::current(content).body_rig(target)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    /// Measure the loop-policy disagreement for LANDMARK-CLIP-TIME.
    /// A changed set is diagnostic evidence, not a required content shape.
    #[test]
    fn probe_rig_and_animator_loop_differences() {
    let content = &crate::test_support::shared_sprites();
        if body_rigs(&content).is_empty() {
            eprintln!("no published body rig on this checkout: nothing to compare");
            return;
        }
        let mut rows = 0;
        let mut differ = BTreeSet::new();
        for (target, text) in body_rigs(&content) {
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
    }
}
