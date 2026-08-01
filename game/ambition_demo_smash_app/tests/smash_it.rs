//! Aggregated integration-test binary for `ambition_demo_smash_app`.
//!
//! Rust links one integration-test binary per top-level `tests/*.rs`. Collapsing
//! these 2 targets into one removes that many link steps against this crate's
//! engine graph from every `cargo test` of it. Which tests run is unchanged;
//! filter a former target with `--test smash_it -- <module_name>`.
//!
//! ⚠ `autotests = false` makes a forgotten `mod` line silently skip a whole
//! file — `smash_it_sync` is the guard that turns that into a failure.

mod smash_it_sync;

mod the_screen_decides;
mod the_stage_kills;
