
use super::*;

fn catalog() -> ambition_characters::actor::character_catalog::CharacterCatalog {
    crate::character_catalog::shipped_catalog()
}

#[test]
fn known_dialogue_ids_are_derived_from_yarn_titles() {
    let ids = known_dialogue_ids();
    assert!(ids.iter().any(|id| id == "creator_intro"));
    // A root that exists only as `__1`/`__2` jump targets cannot start, so it is
    // not an id; its variants are.
    assert!(!ids.iter().any(|id| id == "oiler_post_stabilizer"));
    assert!(ids.iter().any(|id| id == "oiler_post_stabilizer__1"));
    assert!(ids.iter().any(|id| id == "hub_guide__test_sfx"));
    assert!(ids.iter().any(|id| id == "hall_player"));
    assert_eq!(ids.windows(2).filter(|pair| pair[0] == pair[1]).count(), 0);
}

#[test]
fn every_catalog_hall_dialogue_id_has_a_yarn_node() {
    // Pure-text cross-check — no Yarn runtime — so it runs in every config and fails at `cargo
    // test`. The titles are read by the validator's own reader: the runtime loads every source
    // as one project, so a title in any file starts.
    let known = known_dialogue_ids();
    let nodes: std::collections::HashSet<&str> = known.iter().map(String::as_str).collect();

    let catalog = catalog();
    let missing: Vec<(&String, &str)> = catalog
        .data()
        .characters
        .iter()
        .filter_map(|(id, entry)| {
            entry
                .hall_dialogue_id
                .as_deref()
                .filter(|hid| !nodes.contains(hid))
                .map(|hid| (id, hid))
        })
        .collect();

    assert!(
        missing.is_empty(),
        "catalog hall_dialogue_id(s) with no matching `title:` node in \
         the game's Yarn (Inspect would start an unknown node):\n{}",
        missing
            .iter()
            .map(|(id, hid)| format!("  {id} -> {hid}"))
            .collect::<Vec<_>>()
            .join("\n"),
    );
}
/// The DEFAULT player character is the CURRENT incarnation, and the Hall has a
/// pedestal for it. So on the one interaction every playthrough is likeliest to
/// make, the speaker is the listener — and the engine SUPPRESSES a
/// self-conversation that has no `__self` branch
/// (`ambition_dialog::DialogueNodeIndex::entry_node`).
///
/// Without this node the player's own pedestal would silently become
/// un-talkable. The mirror scene is content; that it must exist is structure.
///
/// `hall_player` is a conversation key, not a character, and the pedestal binds the two
/// separately — so the branch this guards is still named for the conversation.
#[test]
fn the_player_pedestal_has_a_self_branch_because_the_default_character_is_the_player() {
    assert_eq!(
        crate::character_catalog::DEFAULT_CHARACTER,
        "player_robot_v3",
        "this guard assumes the default worn character is the current incarnation",
    );
    let known = known_dialogue_ids();
    assert!(
        known.iter().any(|id| id == "hall_player__self"),
        "hall.yarn must author `hall_player__self`: the default player wears the \
         current incarnation, and the Hall gives it a pedestal, so talking to it \
         is self-talk — which the engine suppresses unless content authored the \
         branch",
    );
}

/// Every `<root>__self` branch belongs to a real root node. A self branch for
/// a dialogue that does not exist is dead content.
#[test]
fn every_self_branch_has_a_root_node() {
    for file in yarn_files() {
        let (name, titles) = (&file.path, &file.titles);
        for title in titles {
            if let Some(root) = title.strip_suffix("__self") {
                assert!(
                    titles.iter().any(|other| other == root),
                    "{name}: `{title}` is a self branch of `{root}`, which has no \
                     `title:` node in the same file",
                );
            }
        }
    }
}

/// The Yarn files are the pack's `yarn_dialogue` sources: the game loads
/// exactly the files the manifest lists, in its order, and admission refuses
/// a title that two nodes use and a node with no end, with the file named.
#[test]
fn the_yarn_files_are_the_packs_dialogue_sources() {
    let manifest = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/pack.ron"))
        .expect("the manifest is readable");
    let listed: Vec<&str> = manifest
        .lines()
        .filter(|line| line.contains("schema: \"yarn_dialogue\""))
        .filter_map(|line| line.split('"').nth(1))
        .collect();
    assert!(listed.len() >= 7, "the manifest lists the dialogue files: {listed:?}");
    let loaded: Vec<&str> = yarn_sources().iter().map(|(name, _)| *name).collect();
    assert_eq!(loaded, listed, "the game loads the files the manifest lists");
    for (name, text) in yarn_sources() {
        let on_disk = std::fs::read_to_string(format!("{}/assets/{name}", env!("CARGO_MANIFEST_DIR")))
            .expect("a listed file is on disk");
        assert_eq!(*text, on_disk, "`{name}` is the file's text");
    }

    // A spoken line is in the pack's identity: an edit to it is a new pack.
    let line = "title: creator_intro";
    let edited = crate::pack::compile_pack_with(|path, text| {
        if path != "dialogue/sandbox/intro.yarn" {
            return text;
        }
        let at = text.find(line).expect("the probe edits the creator's first node");
        let end = at + text[at..].find("===").expect("the node ends");
        format!("{}Narrator: an added line.\n{}", &text[..end], &text[end..])
    })
    .expect("an edited line compiles");
    assert_ne!(edited.fingerprint, crate::pack::shipped().fingerprint, "a spoken line moved no identity");

    let with = |file: &'static str, edit: fn(String) -> String| {
        crate::pack::compile_pack_with(move |path, text| if path == file { edit(text) } else { text })
    };
    // `symmetry.yarn` takes a title `intro.yarn` already uses.
    let twice = format!("{:?}", with("dialogue/sandbox/symmetry.yarn", |text| {
        format!("title: creator_intro\n---\nAgain.\n===\n{text}")
    })
    .expect_err("one title on two nodes"));
    assert!(twice.contains("creator_intro"), "{twice}");
    let open = format!("{:?}", with("dialogue/sandbox/dojo.yarn", |text| format!("{text}\ntitle: unended\n---\nNo end.\n"))
        .expect_err("a node with no end"));
    assert!(open.contains("node `unended`") && open.contains("dialogue/sandbox/dojo.yarn"), "{open}");
}
