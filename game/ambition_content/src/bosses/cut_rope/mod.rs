//! Cut-rope boss arena rules.
//!
//! The arena is authored in LDtk as ordinary `Prop` entities named/kinded
//! `cut_rope_rope` and `cut_rope_anvil`, plus a `BossSpawn` whose behavior id
//! is `smirking_behemoth_boss`. This system keeps the one-off mechanic tied to
//! authored level data rather than hard-coded coordinates: cutting the rope prop
//! starts the anvil prop falling; the anvil impact forces the boss encounter
//! through the normal death pipeline.

#![allow(unused_imports)]
use bevy::prelude::*;
use bevy::sprite::Anchor;

use ambition_actors::boss_encounter::{
    BossEncounterRegistry, EncounterBeat, EncounterEffect, EncounterScript, EncounterTrigger,
    ReleaseOnDeath,
};
use ambition_actors::features::BossConfig;
use ambition_actors::features::{
    ActorPose, BossClusterQueryData, BossClusterRef, BossRef, CenteredAabb, DamageableVolumes,
    EnemyActorBundle, FeatureBaseBundle, FeatureId, FeatureName, FeatureSimEntity, PogoPolicy,
    PogoTargetVolumes, PostBossNpc,
};
use ambition_actors::rooms::{PropSpec, RoomSet};
use ambition_characters::brain::ActorControl;
use ambition_characters::brain::BossAttackState;
use ambition_combat::{GameplayBanner, HitEvent, HitSource, ResetRoomFeaturesEvent};
use ambition_encounter::EncounterParticipants;
use ambition_engine_core::config::world_to_bevy;
use ambition_engine_core::{self as ae, AabbExt};
use ambition_render::rendering::PropVisual;
use ambition_sfx::SfxMessage;
use ambition_sprite_sheet::character::{
    build_character_sprite, feet_anchor_for, CharacterAnimator,
};
use ambition_sprite_sheet::game_assets::GameAssets;
use ambition_vfx::vfx::{DebrisBurstMessage, PhysicsDebrisCue};
use ambition_vfx::{ExplosionKind, ExplosionRequest, FireworksRequest, ParticleKind, VfxMessage};

pub const CUT_ROPE_BOSS_ID: &str = "smirking_behemoth_boss";
pub const CUT_ROPE_VICTORY_NPC_ID: &str = "smirking_behemoth_victory_npc";
pub const CUT_ROPE_VICTORY_NPC_DIALOGUE_ID: &str = "smirking_behemoth_victory_npc";
const CUT_ROPE_ROOM_ID: &str = "you_have_to_cut_the_rope";
const CUT_ROPE_VICTORY_NPC_NAME: &str = "The Rope Appreciator";
const CUT_ROPE_VICTORY_NPC_W: f32 = 28.0;
const CUT_ROPE_VICTORY_NPC_H: f32 = 48.0;
const ROPE_KIND: &str = "cut_rope_rope";
const ANVIL_KIND: &str = "cut_rope_anvil";
const PIANO_KIND: &str = "cut_rope_piano";
const ANVIL_GRAVITY: f32 = 1400.0;
const ANVIL_TERMINAL_SPEED: f32 = 920.0;
const ANVIL_Z_OFFSET: f32 = 0.75;
const ROPE_ALIGNMENT_TOLERANCE: f32 = 42.0;
const ROPE_LURE_SPEED: f32 = 150.0;
const ROPE_SPARK_INTERVAL: f32 = 0.22;

pub fn is_cut_rope_boss(id: &str) -> bool {
    id == CUT_ROPE_BOSS_ID
}

// The replay request itself is the ENGINE's generic
// `session::reset::RoomReplayRequested` — content emits it; no
// content-named replay message exists.

/// Latched by the Yarn `<<reset_cut_rope_room>>` command once the player chooses the
/// replay option. The actual room reset intentionally waits until the dialog UI has
/// closed, so the final NPC line remains visible until the player dismisses it.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PendingCutRopeRoomReplay {
    pub requested: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CutRopeHeavyObjectKind {
    Anvil,
    Piano,
}

impl CutRopeHeavyObjectKind {
    const fn prop_kind(self) -> &'static str {
        match self {
            Self::Anvil => ANVIL_KIND,
            Self::Piano => PIANO_KIND,
        }
    }

    const fn display_name(self) -> &'static str {
        match self {
            Self::Anvil => "anvil",
            Self::Piano => "piano",
        }
    }
}

const CUT_ROPE_HEAVY_OBJECT_CYCLE: [CutRopeHeavyObjectKind; 2] =
    [CutRopeHeavyObjectKind::Anvil, CutRopeHeavyObjectKind::Piano];

/// Tracks which heavy object is currently hanging from the cut-rope trap.
///
/// This lives outside [`CutRopeBossArenaState`] so leaving/re-entering the room
/// can rebuild transient fall/rope state without changing the chosen prop. The
/// choice advances only on an actual room reset, which makes the variation
/// deterministic and easy to test.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct CutRopeHeavyObjectCycle {
    index: usize,
}

impl Default for CutRopeHeavyObjectCycle {
    fn default() -> Self {
        Self { index: 0 }
    }
}

impl CutRopeHeavyObjectCycle {
    fn current(&self) -> CutRopeHeavyObjectKind {
        CUT_ROPE_HEAVY_OBJECT_CYCLE[self.index % CUT_ROPE_HEAVY_OBJECT_CYCLE.len()]
    }

    fn advance(&mut self) {
        self.index = (self.index + 1) % CUT_ROPE_HEAVY_OBJECT_CYCLE.len();
    }

    /// Stable Yarn-facing id for the currently hung heavy object.
    pub fn current_dialogue_id(&self) -> &'static str {
        self.current().display_name()
    }
}

/// Convert a pending dialogue-authored replay into the ENGINE's generic
/// [`RoomReplayRequested`](ambition_actors::session::reset::RoomReplayRequested)
/// after the final dialog line has been dismissed. Registered in the engine's
/// `ContentDialogueFollowupSet` slot by `AmbitionBossContentPlugin`, so the
/// host never names this system.
pub fn emit_cut_rope_room_replay_after_dialogue_closes(
    dialogue: Res<ambition_dialog::DialogState>,
    mut pending: ResMut<PendingCutRopeRoomReplay>,
    mut replay_requests: MessageWriter<ambition_actors::session::reset::RoomReplayRequested>,
) {
    if !pending.requested || dialogue.active() {
        return;
    }
    pending.requested = false;
    replay_requests.write(ambition_actors::session::reset::RoomReplayRequested);
}

/// Reset the Smirking Behemoth encounter so the room can be replayed in-place.
///
/// R3: the boss's live state is entity-local, so the actual reset happens when
/// the caller's `ResetRoomFeaturesEvent` despawns + respawns the boss (a fresh
/// boss re-seeds clean Dormant state via `update_boss_encounters`). This helper
/// only clears the *persisted* "cleared" record (so the respawned boss isn't
/// pre-marked defeated), re-hides the victory NPC, and restores the intro music
/// from the read-only profile catalog.
///
/// R4: "cleared" is keyed by PLACEMENT (the boss's `config.id`), so the caller
/// passes the cut-rope boss placement ids currently in the room to clear.
pub fn reset_cut_rope_boss_attempt(
    registry: &BossEncounterRegistry,
    save: Option<&mut ambition_persistence::save::SandboxSave>,
    music_request: Option<&mut ambition_encounter::EncounterMusicRequest>,
    placement_ids: &[String],
) {
    let intro_track = registry
        .profile(CUT_ROPE_BOSS_ID)
        .map(|profile| profile.encounter.music_intro.clone());
    if let Some(save) = save {
        let data = save.data_mut();
        for placement_id in placement_ids {
            data.set_boss(
                placement_id,
                ambition_persistence::save_data::PersistedEncounterState::Untouched,
            );
        }
        // The NPC appears only after the victory beat. Replaying the room should
        // make the post-boss conversation available again only after the next kill.
        data.set_flag("smirking_behemoth_victory_npc_seen", false);
    }
    if let Some(music) = music_request {
        music.priority_track = intro_track.filter(|track| !track.is_empty());
    }
}

/// Express the WHOLE Smirking Behemoth fight as the generic encounter pieces
/// (R5): attach `ReleaseOnDeath` to the entity (frees the victory NPC on death)
/// + an `EncounterScript` to its encounter that DATA-drives the fight:
///   rope-cut → lure the behemoth under the anvil (`CommandMoveTo` → generic
///   `CommandedMove`) + drop the anvil (`DropHazard` → generic `FallingHazard`)
///   → on the hazard's impact gate, `ForceKill`.
/// No cut-rope-specific physics or steering — those are reusable mechanics now.
/// Idempotent + waits until the authored anvil prop is available.
pub fn setup_cut_rope_encounter(
    mut commands: Commands,
    room_set: Res<RoomSet>,
    bosses: Query<&BossConfig>,
    encounters: Query<(Entity, &EncounterParticipants), Without<EncounterScript>>,
    behemoths: Query<(Entity, &BossConfig), Without<ReleaseOnDeath>>,
) {
    // The swallowed victory NPC is freed by the generic on-death capability.
    for (entity, config) in &behemoths {
        if is_cut_rope_boss(&config.behavior.id) {
            commands.entity(entity).insert(ReleaseOnDeath);
        }
    }

    // The script needs the authored anvil's position + size to author the lure
    // target + the falling hazard. Wait for the prop to load.
    let Some(anvil) = room_set
        .active_props()
        .iter()
        .find(|prop| prop.kind == ANVIL_KIND || prop.name == ANVIL_KIND)
    else {
        return;
    };

    for (encounter, participants) in &encounters {
        // Only the behemoth's encounter gets the cut-rope script; member 0 is the
        // single boss the encounter wraps.
        let Some(config) = participants
            .members
            .first()
            .and_then(|p| p.entity)
            .and_then(|m| bosses.get(m).ok())
            .filter(|c| is_cut_rope_boss(&c.behavior.id))
        else {
            continue;
        };
        // Drop fires once the boss is within this x-tolerance of the anvil — the
        // old `boss_alignment_tolerance` (scaled by the boss's combat width).
        let align_tolerance =
            ROPE_ALIGNMENT_TOLERANCE.max(config.behavior.combat_size.map_or(0.0, |s| s.x) * 0.18);
        commands.entity(encounter).insert(EncounterScript::new(vec![
            EncounterBeat::new(
                EncounterTrigger::Gate("rope_cut".to_string()),
                vec![
                    EncounterEffect::CommandMoveTo {
                        member: 0,
                        target: anvil.pos,
                        speed: ROPE_LURE_SPEED,
                        arrive_tolerance: align_tolerance,
                    },
                    EncounterEffect::DropHazard {
                        anchor: anvil.pos,
                        size: anvil.size,
                        gravity: ANVIL_GRAVITY,
                        terminal: ANVIL_TERMINAL_SPEED,
                        align_tolerance,
                        target_member: 0,
                        impact_gate: "cut_rope_impact".to_string(),
                    },
                ],
            ),
            EncounterBeat::new(
                EncounterTrigger::Gate("cut_rope_impact".to_string()),
                vec![EncounterEffect::ForceKill(0)],
            ),
        ]));
    }
}

mod arena;
mod victory;
pub use arena::*;
pub use victory::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_cut_rope_boss_matches_only_the_cut_rope_id() {
        assert!(is_cut_rope_boss(CUT_ROPE_BOSS_ID));
        assert!(!is_cut_rope_boss("gnu_ton_rider"));
        assert!(!is_cut_rope_boss(""));
    }

    #[test]
    fn heavy_object_cycle_alternates_anvil_and_piano_on_advance() {
        let mut cycle = CutRopeHeavyObjectCycle::default();
        assert_eq!(cycle.current_dialogue_id(), "anvil");
        cycle.advance();
        assert_eq!(cycle.current_dialogue_id(), "piano");
        cycle.advance();
        assert_eq!(cycle.current_dialogue_id(), "anvil", "two-step cycle wraps");
    }
}
