//! Yarn command + function + markup registrations — the "vocabulary"
//! that authored `.yarn` content can invoke at runtime.
//!
//! Phase 1 (this commit): empty plugin scaffold. Subsequent phases
//! fill it in:
//!
//! - **Phase 2** registers custom commands: `set_flag`, `clear_flag`,
//!   `give_item`, `spawn_chest`, `play_sfx`, `camera_zoom`. Each
//!   becomes a Bevy system that consumes the matching
//!   `ExecuteCommand` payload and writes to `SandboxSave` / audio
//!   buses / cinematic state.
//!
//! - **Phase 3** registers custom functions: `boss_cleared(id)`,
//!   `flag(name)`, `visit_count(npc_id)`, `quest_active(id)`,
//!   `inventory_has(item)`. These read straight from `SandboxSave`
//!   and surface to Yarn `<<if>>` predicates.
//!
//! - **Phase 4** registers markup-attribute consumers for `[shout]`
//!   and `[whisper]` — span-aware text rendering that triggers
//!   camera shake / audio pitch hooks.
//!
//! Keeping all registrations in one module (vs. scattering them
//! across `audio`, `persistence`, `presentation`, etc.) keeps the
//! "what verbs can dialogue invoke" list single-source-of-truth.
//! The bridge has the integration concerns — coupling the bridge
//! to several sandbox subsystems here is the right tradeoff.

use bevy::prelude::*;

/// Plugin that registers Yarn commands + functions + markup
/// consumers. Mounted from `YarnBridgePlugin` alongside the runner
/// lifecycle.
pub struct YarnBindingsPlugin;

impl Plugin for YarnBindingsPlugin {
    fn build(&self, _app: &mut App) {
        // Phase 2 fills this in with command registrations.
        // Phase 3 fills this in with function registrations.
        // Phase 4 fills this in with markup-attribute observers.
    }
}
