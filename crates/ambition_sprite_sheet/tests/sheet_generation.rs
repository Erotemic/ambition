//! A published sheet table is read whole: the text, the record index, the
//! file-root registry and the mechanics digest all answer from the new
//! generation together, and a table that does not parse is refused.
//!
//! Its own test binary: a publication changes what every reader of the
//! process reads, so it must not race the crate's other tests.

#![cfg(not(any(feature = "static_content", target_arch = "wasm32", target_os = "android")))]

use ambition_sprite_sheet::baked_sheet_rons::{current, publish_sheet_generation, sheet_rons};
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
    let id = publish_sheet_generation(edited).unwrap_or_else(|refused| panic!("{refused}"));
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
    let refused = publish_sheet_generation(broken).expect_err("a sheet table that does not parse was published");
    assert!(refused.0.iter().any(|(file, _)| file == SHEET), "the refusal names the file: {refused}");
    assert_eq!(current().id(), id, "a refused table changed the generation");
    assert_eq!(collision_scale_in_index(), 2.5, "a refused table changed the records");
}
