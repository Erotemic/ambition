//! Aggregated integration-test binary for `ambition_content`.
//!
//! Rust links one integration-test binary per top-level `tests/*.rs`. Collapsing
//! these 6 targets into one removes that many link steps against this crate's
//! engine graph from every `cargo test` of it. Which tests run is unchanged;
//! filter a former target with `--test content_it -- <module_name>`.
//!
//! ⚠ `autotests = false` makes a forgotten `mod` line silently skip a whole
//! file — `content_it_sync` is the guard that turns that into a failure.

mod content_it_sync;

mod aerial_authoring;
mod boss_fight_validator;
mod boss_presentation;
mod boss_seeds;
mod content_pack_registry;
mod dialogue_lint;
mod fighter_brain_ladder;
mod intro_sprite_catalog;
mod yarn_compile;
