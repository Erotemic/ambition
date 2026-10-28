//! Embed the published sprite tree (sheet manifests, portrait manifests,
//! body rigs, part flipbooks, landmark tables and ultrapack catalogs) in a
//! build that has no source tree beside it, and nothing in any other build.
//!
//! A build embeds when it turns on this crate's `static_content` (the player
//! builds: `desktop_player`, the web build and `android` forward it), or when
//! its target has no file system to read the tree from (wasm, Android). Then it
//! sets `cfg(embedded_sprites)` and writes the tables the `baked_*` modules
//! include.
//!
//! A development build embeds nothing and watches no file of the tree: the
//! crate reads the tree at run time (`sprite_tree`), so a sprite publish
//! compiles no crate. Before 2026-10-11 every build embedded it, and touching
//! one sheet compiled 23 crates (185 s) for the Sanic demo app.
//!
//! The files are found by `src/sprite_tree.rs`, the same scan the crate runs.

use std::fs;
use std::path::{Path, PathBuf};

// The published part-flipbook schema, the same file the crate compiles
// (`src/character/rigged/published.rs`): one definition for the RON read here
// and the bincode the game decodes.
#[allow(dead_code)]
#[path = "src/character/rigged/published.rs"]
mod published;

// The embedded landmark table schema, shared with the crate the same way
// (`src/character/landmarks_published.rs`).
#[allow(dead_code)]
#[path = "src/character/landmarks_published.rs"]
mod landmarks_published;

// The scan of the published tree, shared with the crate the same way.
#[allow(dead_code)]
#[path = "src/sprite_tree.rs"]
mod sprite_tree;

/// The source files this script reads besides itself.
const SHARED_SOURCES: [&str; 3] =
    ["src/character/rigged/published.rs", "src/character/landmarks_published.rs", "src/sprite_tree.rs"];

fn main() {
    // The static-SFX-bank cfg/env is set in `ambition_app/build.rs`: a build
    // script affects only its own crate, and the consumer lives there.
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=AMBITION_ART_STORE");
    for source in SHARED_SOURCES {
        println!("cargo:rerun-if-changed={source}");
    }
    println!("cargo::rustc-check-cfg=cfg(embedded_sprites)");
    println!("cargo::rustc-check-cfg=cfg(has_baked_packs)");

    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let owner = manifest_dir.join("../ambition_platformer2d_actor_monolith");

    let target_arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let embed = std::env::var_os("CARGO_FEATURE_STATIC_CONTENT").is_some()
        || target_arch == "wasm32"
        || target_os == "android";

    // Whether this tree has an ultrapack. Pack output is gitignored, so on a
    // fresh checkout there is none, and the tests that need packs are
    // `#[ignore]`d with a reason rather than skipped in silence.
    //
    // ⚠ In a development build this is read when the crate builds and not
    // watched: watching the pack folder would compile the crate on each sprite
    // publish. After the first publish on a fresh tree the pack tests stay
    // ignored until this crate builds again (`touch` this file).
    if !sprite_tree::pack_catalogs(&owner).is_empty() {
        println!("cargo::rustc-cfg=has_baked_packs");
    }

    if !embed {
        return;
    }
    println!("cargo::rustc-cfg=embedded_sprites");
    let selected = sprite_tree::selection(&owner).unwrap_or_else(|error| panic!("{error}"));
    // Cargo watches a directory recursively.
    for dir in sprite_tree::sprite_dirs(&owner) {
        println!("cargo:rerun-if-changed={}", dir.display());
    }
    println!("cargo:rerun-if-changed={}", owner.join("assets/sprite_packs").display());

    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    write_text_table(&sprite_tree::sheet_manifests_in(&selected).expect_one_for_each_key(), &out_dir, "BAKED_SHEET_RONS", "baked_sheet_rons.rs");
    write_text_table(&sprite_tree::portraits_in(&selected), &out_dir, "BAKED_PORTRAIT_RONS", "baked_portrait_rons.rs");
    write_text_table(&sprite_tree::packs_in(&selected), &out_dir, "BAKED_PACK_CATALOGS", "baked_pack_catalogs.rs");
    write_text_table(
        &sprite_tree::companions_in(&selected, "_body_rig.ron", false).expect_one_for_each_key(),
        &out_dir,
        "BAKED_BODY_RIGS",
        "baked_body_rigs.rs",
    );
    bake_part_flipbooks(&sprite_tree::companions_in(&selected, "_parts.ron", true).expect_one_for_each_key(), &out_dir);
}

/// Each published part-flipbook draw table, parsed here and embedded as
/// bincode of the crate's schema, so the game decodes it and does not parse
/// its text (about 50 ms a table, 123 ms to decode all 161, measured
/// 2026-10-11). A full-resolution table that tracks a landmark also embeds its
/// landmark table.
fn bake_part_flipbooks(entries: &[(String, PathBuf)], out_dir: &Path) {
    let encoded_dir = out_dir.join("part_flipbooks");
    fs::create_dir_all(&encoded_dir).unwrap_or_else(|error| panic!("create {}: {error}", encoded_dir.display()));
    let landmarks_dir = out_dir.join("landmarks");
    fs::create_dir_all(&landmarks_dir).unwrap_or_else(|error| panic!("create {}: {error}", landmarks_dir.display()));
    let mut landmarks: Vec<(String, PathBuf, Vec<u8>)> = Vec::new();
    let mut body = String::new();
    body.push_str("// Auto-generated by build.rs. Do not edit.\n");
    body.push_str("pub static BAKED_PART_FLIPBOOKS: &[(&str, BakedPartFlipbook, &str)] = &[\n");
    for (key, path) in entries {
        let escaped_path = escaped(path);
        // A table this build cannot read is embedded as text: the game refuses
        // it with the error the RON gives, and the build does not fail on one
        // broken publish.
        let text = fs::read_to_string(path).unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
        let parsed = sprite_tree::read_part_flipbook(&text);
        // A landmark is gameplay data and has no quality tier, so only the
        // full-resolution table publishes one.
        if let (false, Ok(published)) = (key.contains('.'), &parsed) {
            if let Some(bytes) = sprite_tree::landmarks_of(published).and_then(|table| bincode::serialize(&table).ok()) {
                let encoded = landmarks_dir.join(format!("{key}.bin"));
                fs::write(&encoded, &bytes).unwrap_or_else(|error| panic!("write {}: {error}", encoded.display()));
                landmarks.push((key.clone(), encoded, bytes));
            }
        }
        let table = match parsed.ok().and_then(|published| bincode::serialize(&published).ok()) {
            Some(bytes) => {
                let encoded = encoded_dir.join(format!("{key}.bin"));
                fs::write(&encoded, bytes).unwrap_or_else(|error| panic!("write {}: {error}", encoded.display()));
                format!("BakedPartFlipbook::Encoded(include_bytes!(\"{}\"))", escaped(&encoded))
            }
            None => format!("BakedPartFlipbook::Ron(include_str!(\"{escaped_path}\"))"),
        };
        body.push_str(&format!("    (\"{key}\", {table}, \"{escaped_path}\"),\n"));
    }
    body.push_str("];\n");
    fs::write(out_dir.join("baked_part_flipbooks.rs"), body).expect("write baked_part_flipbooks.rs");
    write_landmark_tables(&landmarks, out_dir);
}

/// Write the embedded landmark tables, sorted by target as the flipbooks are,
/// and the one digest that covers them all.
fn write_landmark_tables(landmarks: &[(String, PathBuf, Vec<u8>)], out_dir: &Path) {
    let mut body = String::new();
    body.push_str("// Auto-generated by build.rs. Do not edit.\n");
    body.push_str("pub static BAKED_LANDMARKS: &[(&str, &[u8])] = &[\n");
    for (key, encoded, _) in landmarks {
        body.push_str(&format!("    (\"{key}\", include_bytes!(\"{}\")),\n", escaped(encoded)));
    }
    body.push_str("];\n");
    let digest = landmarks_published::landmark_tables_digest(
        landmarks.iter().map(|(key, _, bytes)| (key.as_str(), bytes.as_slice())),
    );
    body.push_str(&format!("pub static BAKED_LANDMARKS_DIGEST: &str = \"{digest}\";\n"));
    fs::write(out_dir.join("baked_landmarks.rs"), body).expect("write baked_landmarks.rs");
}

/// Write `entries` as `static <table>: &[(&str, &str)]` of `include_str!`.
fn write_text_table(entries: &[(String, PathBuf)], out_dir: &Path, table: &str, file: &str) {
    let mut body = String::new();
    body.push_str("// Auto-generated by build.rs. Do not edit.\n");
    body.push_str(&format!("pub static {table}: &[(&str, &str)] = &[\n"));
    for (key, path) in entries {
        body.push_str(&format!("    (\"{key}\", include_str!(\"{}\")),\n", escaped(path)));
    }
    body.push_str("];\n");
    fs::write(out_dir.join(file), body).unwrap_or_else(|error| panic!("write {file}: {error}"));
}

/// `path` as a Rust string literal body. An absolute host path is safe for
/// `include_str!`: the build script runs on the compiling host, also for an
/// Android cross-compile.
fn escaped(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "\\\\")
}
