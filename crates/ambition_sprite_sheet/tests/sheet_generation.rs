//! A published sheet table is read whole: the text, the record index, the
//! file-root registry and the mechanics digest all answer from the new
//! generation together, and a table that does not parse is refused.
//!
//! Its own test binary: a publication changes what every reader of the
//! process reads, so it must not race the crate's other tests.

#![cfg(not(any(feature = "static_content", target_arch = "wasm32", target_os = "android")))]

use ambition_sprite_sheet::baked_sheet_rons::{current, publish_sheet_generation, sheet_rons, SheetTexts};
use ambition_sprite_sheet::character::sheets::{baked_sheet_mechanics_digest, record_for_sheet_key};

const SHEET: &str = "player_robot_v3";

fn collision_scale_in_index() -> f32 {
    record_for_sheet_key(SHEET).and_then(|record| record.tuning.as_ref()).map(|tuning| tuning.collision_scale).expect("the player's sheet states a tuning")
}

fn collision_scale_in_registry() -> f32 {
    ambition_sprite_sheet::shared_baked_sheet_registry()
        .get(SHEET)
        .and_then(|record| record.tuning.as_ref())
        .map(|tuning| tuning.collision_scale)
        .expect("the registry holds the player's sheet")
}

fn kept(text: String) -> &'static str {
    Box::leak(text.into_boxed_str())
}

#[test]
fn a_published_sheet_table_is_read_whole_and_a_broken_one_is_refused() {
    let first = current();
    let shipped_scale = collision_scale_in_index();
    assert_eq!(collision_scale_in_registry(), shipped_scale, "control: the index and the registry agree");
    let shipped_digest = baked_sheet_mechanics_digest().to_owned();

    let old = format!("collision_scale: {shipped_scale},");
    let edited: Vec<(&'static str, &'static str)> = sheet_rons()
        .iter()
        .map(|(key, text)| {
            if *key == SHEET {
                assert_eq!(text.matches(&old).count(), 1, "the probe edits one value of `{SHEET}`");
                (*key, kept(text.replace(&old, "collision_scale: 2.5,")))
            } else {
                (*key, *text)
            }
        })
        .collect();
    let id = publish_sheet_generation(SheetTexts { sheets: edited, ..SheetTexts::current() }).unwrap_or_else(|refused| panic!("{refused}"));
    assert_eq!(id, first.id() + 1, "a publication is the next generation");
    assert_eq!(current().id(), id);
    assert!(sheet_rons().iter().any(|(key, text)| *key == SHEET && text.contains("collision_scale: 2.5,")), "the table is not the published one");
    assert_eq!(collision_scale_in_index(), 2.5, "the record index is not the published table's");
    assert_eq!(collision_scale_in_registry(), 2.5, "the file-root registry is not the published table's");
    assert_ne!(baked_sheet_mechanics_digest(), shipped_digest, "the mechanics digest is not the published table's");

    let broken: Vec<(&'static str, &'static str)> = sheet_rons()
        .iter()
        .map(|(key, text)| if *key == SHEET { (*key, "[ (target: ") } else { (*key, *text) })
        .collect();
    let refused = publish_sheet_generation(SheetTexts { sheets: broken, ..SheetTexts::current() }).expect_err("a sheet table that does not parse was published");
    assert!(refused.0.iter().any(|(file, _)| file == SHEET), "the refusal names the file: {refused}");
    assert_eq!(current().id(), id, "a refused table changed the generation");
    assert_eq!(collision_scale_in_index(), 2.5, "a refused table changed the records");

    // A preparation parses only the sheet files whose text changed.
    use ambition_sprite_sheet::baked_sheet_rons::prepare_sheet_table;
    let unchanged = prepare_sheet_table(SheetTexts::current()).unwrap_or_else(|refused| panic!("{refused}"));
    assert_eq!(unchanged.parsed_now(), 0, "an unchanged table was parsed again");
    let one_edited: Vec<(&'static str, &'static str)> = sheet_rons()
        .iter()
        .map(|(key, text)| if *key == SHEET { (*key, kept(format!("{text}\n// edited\n"))) } else { (*key, *text) })
        .collect();
    let one = prepare_sheet_table(SheetTexts { sheets: one_edited, ..SheetTexts::current() }).unwrap_or_else(|refused| panic!("{refused}"));
    assert_eq!(one.parsed_now(), 1, "a table with one edited sheet parsed {} of {} files", one.parsed_now(), sheet_rons().len());

    // A read of the tree keeps only the texts that changed: each other text
    // is the current generation's own (a text read is kept for the life of
    // the process). The published `SHEET` differs from its file on disk.
    let owner = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../ambition_platformer2d_actor_monolith"));
    let read = ambition_sprite_sheet::baked_sheet_rons::read_sheet_table_under(owner);
    let held: std::collections::HashMap<&str, &str> = sheet_rons().iter().copied().collect();
    let new_texts: Vec<&str> = read.sheets.iter().filter(|(key, text)| held.get(key).is_none_or(|was| !std::ptr::eq(*was, *text))).map(|(key, _)| *key).collect();
    assert!(new_texts == [SHEET], "a read of the tree kept a new text for {} files: {:?}", new_texts.len(), &new_texts[..new_texts.len().min(3)]);
    assert_eq!(read.sheets.len(), sheet_rons().len(), "control: the read is the whole table");

    // The effect index is built from the records of its generation: an
    // edited effect row is what the index says.
    use ambition_sprite_sheet::fx::{authored_effect, fx_sheets};
    let fx_target = fx_sheets().first().expect("the premise: this checkout publishes an effect sheet").target;
    let (fx_key, fx_text) = *sheet_rons().iter().find(|(key, _)| *key == fx_target).expect("the effect sheet is in the table");
    let effect = authored_effect(record_for_sheet_key(fx_target).unwrap().rows[0].animation.as_str()).expect("the effect sheet's first row is an effect").clone();
    let duration = format!("duration_secs: {:?}", effect.duration_secs);
    let mut edited_fx = None;
    let sheets: Vec<(&'static str, &'static str)> = sheet_rons()
        .iter()
        .map(|(key, text)| {
            if *key == fx_key {
                // The first row's timing; the row is found by its name.
                let at = text.find(&format!("\"{}\"", effect.name)).expect("the row is named in its sheet");
                let offset = at + text[at..].find(&duration).expect("the row states its duration");
                let edited = kept(format!("{}duration_secs: 0.25{}", &text[..offset], &text[offset + duration.len()..]));
                edited_fx = Some(edited);
                (*key, edited)
            } else {
                (*key, *text)
            }
        })
        .collect();
    assert_ne!(effect.duration_secs, 0.25, "the premise: the edit changes the row");
    assert!(edited_fx.is_some() && fx_text.len() > 0);
    publish_sheet_generation(SheetTexts { sheets, ..SheetTexts::current() }).unwrap_or_else(|refused| panic!("{refused}"));
    assert_eq!(authored_effect(effect.name).map(|effect| effect.duration_secs), Some(0.25), "the effect index is not the published generation's");

    // A body rig is a part of the generation: an edited rig is what a reader
    // reads, and a rig that does not parse refuses the table.
    let rig_of = |target: &str| ambition_sprite_sheet::baked_body_rigs::baked_body_rig(target);
    let (target, text) = *current().body_rigs().first().expect("the premise: this checkout publishes a body rig");
    assert_eq!(rig_of(target), Some(text), "control: a reader reads the current generation's rig");
    let edited_rig = kept(format!("{text}\n// edited\n"));
    let rigs = |text: &'static str| current().body_rigs().iter().map(|(name, was)| (*name, if *name == target { text } else { *was })).collect();
    let id = publish_sheet_generation(SheetTexts { body_rigs: rigs(edited_rig), ..SheetTexts::current() }).unwrap_or_else(|refused| panic!("{refused}"));
    assert_eq!(rig_of(target), Some(edited_rig), "the body rig is not the published generation's");
    let refused = publish_sheet_generation(SheetTexts { body_rigs: rigs("( schema_version: "), ..SheetTexts::current() })
        .expect_err("a body rig that does not parse was published");
    assert!(refused.0.iter().any(|(file, _)| *file == format!("{target}_body_rig.ron")), "the refusal names the rig: {refused}");
    assert_eq!((current().id(), rig_of(target)), (id, Some(edited_rig)), "a refused rig changed the generation");
}
