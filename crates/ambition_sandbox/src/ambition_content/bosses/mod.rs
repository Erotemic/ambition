//! Named Ambition boss content registration.
//!
//! Owns the install of the default [`BossEncounterRegistry`] so the named
//! boss roster is constructed in one content-owned place. The general boss
//! machinery (profiles, specs, encounter registry/system, patterns) still
//! lives in `crate::boss_encounter`; this module owns the bespoke per-boss
//! *behavior* and *bark content* that names individual bosses:
//!
//! - [`gnu_ton`] — GNU-ton's bespoke arena gating (retreat-ladder reveal +
//!   floor-gate) and head-hurtbox regression coverage.
//! - [`banter`] — boss combat-banter lines + the idle-bark ticker
//!   ([`banter::install_boss_banter`] / [`banter::tick_boss_idle_barks`]),
//!   installed next to its dialogue registration.

use bevy::prelude::*;

pub mod banter;
pub mod cut_rope;
pub mod gnu_ton;
#[cfg(feature = "ui")]
pub mod yarn;

pub use banter::{install_boss_banter, tick_boss_idle_barks};
pub use cut_rope::{
    emit_cut_rope_room_replay_after_dialogue_closes, is_cut_rope_boss,
    reset_cut_rope_boss_arena_on_room_reset, reset_cut_rope_boss_attempt,
    spawn_cut_rope_victory_npc, steer_cut_rope_boss_under_anvil,
    sync_cut_rope_boss_arena_prop_visuals, tick_cut_rope_boss_arena, CutRopeBossArenaState,
    CutRopeHeavyObjectCycle, CutRopeRoomReplayRequested, PendingCutRopeRoomReplay,
    SmirkingBehemothVictoryNpc, CUT_ROPE_BOSS_ID, CUT_ROPE_VICTORY_NPC_DIALOGUE_ID,
    CUT_ROPE_VICTORY_NPC_ID,
};
pub use gnu_ton::gate_gnu_ton_arena_ladder;

/// Installs the default Ambition boss encounter registry resource and
/// the cut-rope Yarn vocabulary + mirror feed.
pub struct AmbitionBossContentPlugin;

impl Plugin for AmbitionBossContentPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(crate::boss_encounter::BossEncounterRegistry::default());

        // Cut-rope Yarn vocabulary: installed on the DialogueRunner via the
        // dialog runtime's content-bindings seam, plus the per-frame extras
        // feed (after the generic mirror refresh so the snapshot Yarn reads
        // is consistent within the tick).
        #[cfg(feature = "ui")]
        {
            app.init_resource::<crate::dialog::yarn_bindings::YarnContentBindings>();
            app.world_mut()
                .resource_mut::<crate::dialog::yarn_bindings::YarnContentBindings>()
                .installers
                .push(yarn::install_cut_rope_yarn_bindings);
            app.add_systems(
                Update,
                yarn::mirror_cut_rope_heavy_object
                    .after(crate::dialog::yarn_bindings::refresh_yarn_state_mirror),
            );
        }
    }
}
