//! Compile-time table of `(filename_root, ron_text)` pairs for every
//! `*_spritesheet.ron` under `assets/sprites/`. Populated by `build.rs`
//! via `include_str!`, which means Android / wasm builds carry the same
//! data desktop does — they no longer try to read the missing
//! `CARGO_MANIFEST_DIR/assets/sprites` path at runtime.
//!
//! Mirrors the runtime scan: root `assets/sprites/` plus one level of
//! subdirs (the bosses publish into per-character subdirs).

include!(concat!(env!("OUT_DIR"), "/baked_sheet_rons.rs"));
