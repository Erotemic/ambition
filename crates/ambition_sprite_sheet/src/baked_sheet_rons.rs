//! Compile-time table of `(filename_root, ron_text)` pairs for every
//! `*_spritesheet.ron` under `assets/sprites/`. `build.rs` fills it with
//! `include_str!`, so Android and wasm builds carry the same data as desktop
//! and do not read `CARGO_MANIFEST_DIR/assets/sprites` at runtime.
//!
//! Mirrors the runtime scan: root `assets/sprites/` plus one level of
//! subdirs (bosses publish into per-character subdirs).

include!(concat!(env!("OUT_DIR"), "/baked_sheet_rons.rs"));
