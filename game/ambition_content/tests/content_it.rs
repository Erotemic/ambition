//! Aggregated integration-test binary for `ambition_content`.
//!
//! Rust links one integration-test binary per top-level `tests/*.rs`. Collapsing
//! these 6 targets into one removes that many link steps against this crate's
//! engine graph from every `cargo test` of it. Which tests run is unchanged;
//! filter a former target with `--test content_it -- <module_name>`.
//!
//! `autotests = false` makes a forgotten `mod` line silently skip a whole
//! file — `content_it_sync` is the guard that turns that into a failure.

mod content_it_sync;

mod aerial_authoring;
// The countdown it kept (`the_striker_row_lives_exactly_as_long_as_the_placement_that_needs_it`)
// said so itself — *"if the skitter was CAST … delete the row, its note, its SURVIVORS entry, and
// this test"* — and with both sides false it had become two absences agreeing.
//
// the surviving claim is STRONGER, and it is no longer a count at all: the test
// it named, `only_the_uncast_placements_still_ride_the_display_name_fallback`,
// was DELETED by `78f22965e` ("A placement names its creature, or it is not a
// placement") -- the invariant became structural rather than something a test
// counts down. `worlds::tests::every_shipped_enemy_placement_can_be_built` is
// what stands there now, asked of every world the provider ships.
mod boss_fight_validator;
mod boss_presentation;
mod boss_seeds;
mod content_pack_registry;
mod dialogue_lint;
mod fighter_brain_ladder;
mod intro_sprite_catalog;
mod puppy_slug_forced_seat;
mod summoned_minions_resolve;
mod yarn_compile;
mod yarn_condition_aliases;
