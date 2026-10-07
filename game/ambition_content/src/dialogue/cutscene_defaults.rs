//! Ambition's cutscene library, read from its content pack.

use ambition_cutscene::CutsceneLibrary;

/// Every cutscene `pack` ships, in library order: the `cutscene_library` it
/// lowered from `assets/data/cutscenes/*.ron`. A pack that declares no cutscene
/// file ships none (it compiles; the startup validator then names each room
/// binding that has no script).
pub fn cutscene_scripts_of(
    pack: &ambition_content_pack::PreparedContentPack,
) -> Vec<ambition_cutscene::CutsceneScript> {
    ambition_cutscene::content_schema::lowered_cutscenes(pack)
        .cloned()
        .unwrap_or_default()
}

/// Every cutscene `pack` ships, as a library. Installed at composition, and
/// revised by a content reload (`reload::publish_cutscene_library`).
pub fn cutscene_library_of(pack: &ambition_content_pack::PreparedContentPack) -> CutsceneLibrary {
    let mut library = CutsceneLibrary::default();
    for script in cutscene_scripts_of(pack) {
        library.insert(script);
    }
    library
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The cutscenes are content: a script appended to a cutscene file
    /// reaches the lowered library through Ambition's own pack compile, and
    /// one id in both files refuses the pack. The control is the unedited
    /// compile, whose library is the one the game installs.
    #[test]
    fn a_cutscene_authored_in_the_pack_is_in_the_library() {
        use ambition_cutscene::content_schema::lowered_cutscenes;
        let ids = |pack: &ambition_content_pack::PreparedContentPack| -> Vec<String> {
            lowered_cutscenes(pack)
                .expect("the pack lowers a library")
                .iter()
                .map(|script| script.id.clone())
                .collect()
        };
        let shipped = crate::pack::compile_pack().expect("the shipped pack compiles");
        let mut shipped_ids = ids(&shipped);
        shipped_ids.sort();
        let installed: Vec<String> = cutscene_library_of(crate::pack::shipped()).scripts.into_keys().collect();
        assert_eq!(shipped_ids, installed);

        let append = |file: &'static str, script: &'static str| {
            move |path: &str, text: String| {
                if path != file {
                    return text;
                }
                let end = text.rfind(']').expect("the file is a list");
                format!("{}, {script}\n]", text[..end].trim_end().trim_end_matches(','))
            }
        };
        let probe = r#"(id: "pack_probe", beats: [Wait(seconds: 0.1)])"#;
        let edited = crate::pack::compile_pack_with(append("data/cutscenes/intro.ron", probe))
            .expect("one more script compiles");
        assert!(ids(&edited).contains(&"pack_probe".to_string()));

        let again = r#"(id: "test_intro", beats: [Wait(seconds: 0.1)])"#;
        let doubled = crate::pack::compile_pack_with(append("data/cutscenes/intro.ron", again));
        let failure = format!("{:?}", doubled.expect_err("an id in both files refuses the pack"));
        assert!(failure.contains("DuplicateIdentity"), "{failure}");
    }

    #[test]
    fn default_cutscene_library_includes_test_intro() {
        let lib = cutscene_library_of(crate::pack::shipped());
        assert!(lib.get("test_intro").is_some());
    }

    #[test]
    fn default_cutscene_library_includes_boss_intro() {
        let lib = cutscene_library_of(crate::pack::shipped());
        assert!(lib.get("boss_intro_gradient_sentinel").is_some());
    }
}

