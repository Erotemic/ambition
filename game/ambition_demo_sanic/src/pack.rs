//! The Sanic demo's content pack.
//!
//! The demo states its cast (`assets/data/character_catalog.ron`) and its
//! fighters' moves (`assets/data/movesets/`) as data, and registers them
//! through [`PACK`].

use ambition_platformer2d::content::EmbeddedPack;

/// `assets/pack.ron` and every source it declares: the manifest is the one
/// list, so a source line added there is played with no Rust build.
pub static PACK: EmbeddedPack = ambition_platformer2d::content_pack! {
    root: "assets",
    // The demo's tuning is its own policy.
    schemas: [crate::rules::sanic_rules_schema],
};

/// A build that reads the pack off disk states the room rules in `scope`
/// again when an edit to them is published between sessions.
pub fn room_rules_reload(
    scope: ambition_platformer2d::combat::scoped_rules::RulesScope,
) -> ambition_platformer2d::content::live_reload::EmbeddedPackReloadPlugin {
    ambition_platformer2d::content::live_reload::EmbeddedPackReloadPlugin::room_rules(&PACK, scope)
}

/// The rules of the demo's rooms, as its pack states them
/// (`assets/data/room_rules.ron`). An edit to the file is played at the next
/// start, with no Rust build.
pub fn room_rules() -> ambition_platformer2d::combat::room_rules::RoomRules {
    *ambition_platformer2d::combat::room_rules::lowered_room_rules(PACK.prepared())
        .expect("the demo's pack states the rules of its rooms (`data/room_rules.ron`)")
}
