//! Measure RON parsing and binary decoding for selected part tables.
//!
//! Set `AMBITION_CONTENT_LIBRARY` to a `library.json` path, then run:
//! `cargo run --profile profiling -p ambition_sprite_sheet --example measure_part_table_parse`.

use std::time::Instant;

use ambition_sprite_sheet::baked_part_flipbooks::{part_flipbooks, published_ron_on_build_host};
use ambition_sprite_sheet::character::rigged::RiggedSpriteAsset;

fn main() {
    let path = std::env::var("AMBITION_CONTENT_LIBRARY").expect("Set AMBITION_CONTENT_LIBRARY to a library.json path");
    let mut libraries = ambition_asset_manager::content_library::ContentLibraries::default();
    libraries.insert(ambition_asset_manager::content_library::ContentLibrary::read(path).expect("read the selected library")).unwrap();
    let snapshot = libraries.snapshot().expect("read the selected products");
    let content = ambition_sprite_sheet::SpriteContent::from_texts(
        ambition_sprite_sheet::baked_sheet_rons::SheetTexts::from_snapshot(&snapshot, None).unwrap()
    ).expect("admit the selected sprite metadata");
    let mut rows = Vec::new();
    for (key, table, _) in part_flipbooks(&content) {
        if key.contains('.') {
            continue;
        }
        let text = published_ron_on_build_host(&content, key).expect("the published file");
        let start = Instant::now();
        RiggedSpriteAsset::from_published_ron(&text).unwrap_or_else(|e| panic!("{key}: {e}"));
        let parse = start.elapsed().as_secs_f64() * 1e3;
        let start = Instant::now();
        table.decode().unwrap_or_else(|e| panic!("{key}: {e}"));
        let decode = start.elapsed().as_secs_f64() * 1e3;
        rows.push((*key, text.len(), parse, decode));
    }
    assert!(!rows.is_empty(), "no selected part flipbooks");
    rows.sort_by(|a, b| b.2.total_cmp(&a.2));
    for (key, bytes, parse, decode) in rows.iter().take(12) {
        println!("{key:36} {:6.2} MB RON  parse {parse:7.1} ms  decode {decode:6.2} ms", *bytes as f64 / 1e6);
    }
    let parse: f64 = rows.iter().map(|r| r.2).sum();
    let decode: f64 = rows.iter().map(|r| r.3).sum();
    let bytes: usize = rows.iter().map(|r| r.1).sum();
    println!("{} tables, {:.1} MB of RON: parse {parse:.0} ms, decode {decode:.0} ms", rows.len(), bytes as f64 / 1e6);
}
