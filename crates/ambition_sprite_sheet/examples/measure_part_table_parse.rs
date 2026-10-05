//! How long does the game take to read each published part-flipbook draw
//! table? `RiggedSpriteAsset::baked` reads a table on the main thread the
//! first time a sheet realizes it. Times the embedded bincode the game decodes
//! beside a parse of the RON it was built from.
//!
//!     cargo run --profile profiling -p ambition_sprite_sheet --example measure_part_table_parse
//!
//! Prints the slowest tables and the totals.

use std::time::Instant;

use ambition_sprite_sheet::baked_part_flipbooks::{published_ron_on_build_host, BAKED_PART_FLIPBOOKS};
use ambition_sprite_sheet::character::rigged::RiggedSpriteAsset;

fn main() {
    let mut rows = Vec::new();
    for (key, table, _) in BAKED_PART_FLIPBOOKS {
        if key.contains('.') {
            continue;
        }
        let text = published_ron_on_build_host(key).expect("the published file");
        let start = Instant::now();
        RiggedSpriteAsset::from_published_ron(&text).unwrap_or_else(|e| panic!("{key}: {e}"));
        let parse = start.elapsed().as_secs_f64() * 1e3;
        let start = Instant::now();
        table.decode().unwrap_or_else(|e| panic!("{key}: {e}"));
        let decode = start.elapsed().as_secs_f64() * 1e3;
        rows.push((*key, text.len(), parse, decode));
    }
    assert!(!rows.is_empty(), "no embedded part flipbooks");
    rows.sort_by(|a, b| b.2.total_cmp(&a.2));
    for (key, bytes, parse, decode) in rows.iter().take(12) {
        println!("{key:36} {:6.2} MB RON  parse {parse:7.1} ms  decode {decode:6.2} ms", *bytes as f64 / 1e6);
    }
    let parse: f64 = rows.iter().map(|r| r.2).sum();
    let decode: f64 = rows.iter().map(|r| r.3).sum();
    let bytes: usize = rows.iter().map(|r| r.1).sum();
    println!("{} tables, {:.1} MB of RON: parse {parse:.0} ms, decode {decode:.0} ms", rows.len(), bytes as f64 / 1e6);
}
