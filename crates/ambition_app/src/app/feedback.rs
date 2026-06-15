#[allow(unused_imports)]
use super::cli::*;
#[allow(unused_imports)]
use super::dev_runtime::*;
#[allow(unused_imports)]
use super::hud::*;
#[allow(unused_imports)]
use super::phases::*;
#[allow(unused_imports)]
use super::player_tick::*;
#[allow(unused_imports)]
use super::plugins::*;
#[allow(unused_imports)]
use super::resources::*;
#[allow(unused_imports)]
use super::setup_systems::*;
#[allow(unused_imports)]
use super::world_flow::*;
#[allow(unused_imports)]
use super::*;
#[allow(unused_imports)]
use ambition_sandbox::app::*;

/// Bundled `MessageWriter`s for the sim → presentation event channels
/// the player tick (and the inline `*_phase` helpers it calls) writes
/// to during the gameplay tick.
///
/// Bundling them in a single `SystemParam` keeps the player tick's
/// signature under Bevy's 16-`SystemParam` budget. The inline phase
/// helpers (`player_control_phase`, `player_simulation_phase`) take
/// `&mut event_writers.sfx` / `&mut event_writers.vfx` via split
/// borrows and write directly — no intermediate Vec collectors. Other
/// channels (`PlayerDiedMessage`, `DebrisBurstMessage`,
/// `RoomTransitionRequested`) are written directly from their own
/// extracted systems' `MessageWriter` params.
#[derive(SystemParam)]
pub struct SandboxEventWriters<'w> {
    pub(super) sfx: MessageWriter<'w, SfxMessage>,
    pub(super) vfx: MessageWriter<'w, VfxMessage>,
}

/// Bundled combat-state resources that need to be torn down on a
/// room transition or same-room reset (per-target slot reservations,
/// in-flight enemy projectiles, …) PLUS the feature-overlay
/// read-side that the transition logger needs. Bundling keeps
/// consumers like `apply_room_transition_system` under Bevy's
/// 16-`SystemParam` budget — without this they'd need a separate
/// ResMut/Res for each piece.
#[derive(SystemParam)]
pub struct CombatRoomReset<'w, 's> {
    pub commands: Commands<'w, 's>,
    // In-flight enemy projectiles are ECS entities now (Phase 3c-iii); despawn
    // them instead of clearing a Vec.
    pub enemy_projectiles:
        Query<'w, 's, Entity, With<ambition_sandbox::enemy_projectile::EnemyProjectile>>,
    pub slot_board: ResMut<'w, ambition_sandbox::combat::slots::CombatSlotsRes>,
    pub feature_overlay: Res<'w, ambition_sandbox::features::FeatureEcsWorldOverlay>,
    pub gravity: ResMut<'w, ambition_sandbox::physics::GravityField>,
    pub base_gravity: ResMut<'w, ambition_sandbox::physics::BaseGravity>,
}

impl<'w, 's> CombatRoomReset<'w, 's> {
    /// Drop every in-flight enemy projectile + every slot
    /// reservation. Called by the room-transition path so a fresh
    /// arena doesn't inherit hostile shots or stale assignments
    /// from the room the player just left, AND by the same-room
    /// reset path so a player death + respawn comes back to a
    /// clean combat state.
    pub fn clear_carryover(&mut self) {
        for entity in &self.enemy_projectiles {
            self.commands.entity(entity).despawn();
        }
        self.slot_board.0.clear_assignments();
        *self.gravity = ambition_sandbox::physics::GravityField::default();
        *self.base_gravity = ambition_sandbox::physics::BaseGravity::default();
    }
}

/// Mutable producer streams the player tick writes into during the gameplay
/// tick.
///
/// Phase-1 strangler rule: typed gameplay effects now travel through focused
/// Bevy messages (`SetFlagRequested` / `QuestAdvanceRequested` /
/// `SwitchActivated` / `GameplaySfxRequested`) rather than a custom
/// `FeatureEventBus` resource or a single mixed-purpose `GameplayEffect` enum.
/// Bundling the remaining sim→sim writers here keeps the player tick under
/// Bevy's 16-`SystemParam` budget while making the cross-system transport
/// explicit.
///
/// Add new sim → sim streams (NOT sim → presentation, which is
/// `SandboxEventWriters`) here when they grow naturally; resist the urge to
/// thread them through the system signature directly.
#[derive(SystemParam)]
pub struct SandboxQueues<'w> {
    /// Single canonical channel for attacker-direction hits (player
    /// slash, player projectile, pogo bounce). Replaced the prior
    /// split `DamageEvent` + `PogoBounceEvent` writers.
    pub hit_events: MessageWriter<'w, ambition_sandbox::features::HitEvent>,
    pub reset_room_features: MessageWriter<'w, ambition_sandbox::features::ResetRoomFeaturesEvent>,
    pub feature_ecs_overlay: Res<'w, ambition_sandbox::features::FeatureEcsWorldOverlay>,
    pub dialogue: ResMut<'w, ambition_sandbox::dialog::DialogState>,
    pub physics_settings: Res<'w, ambition_sandbox::world::physics::PhysicsSandboxSettings>,
    pub moving_platforms: ResMut<'w, ambition_sandbox::MovingPlatformSet>,
    pub sim_state: ResMut<'w, ambition_sandbox::SandboxSimState>,
    pub clock: ResMut<'w, ambition_sandbox::time::clock_state::ClockState>,
    pub dev_state: ResMut<'w, ambition_sandbox::SandboxDevState>,
}

/// Read-only progression-state bundle for the HUD and pause menu.
///
/// Same `SystemParam`-packing trick as `SandboxQueues` — the HUD reads
/// from many independent registries (quests, cutscene state, bosses,
/// encounters, world map) and would otherwise blow the 16-param budget
/// when combined with windowing / camera / font handles. Grouping them
/// behind a single param both keeps the budget headroom and documents
/// the intentional read-only contract: HUD systems must not mutate
/// progression state. Mutators live in the producer side
/// (the player tick, `ambition_sandbox::quest`, `ambition_sandbox::boss_encounter`, etc.).
#[derive(SystemParam)]
pub struct ProgressionResources<'w> {
    pub quests: Res<'w, ambition_content::quest::QuestRegistry>,
    pub cutscene: Res<'w, ambition_render::cutscene::ActiveCutscene>,
    pub cutscene_request: Res<'w, ambition_render::cutscene::CutsceneAdvanceRequest>,
    pub bosses: Res<'w, ambition_sandbox::boss_encounter::BossEncounterRegistry>,
    pub encounters: Res<'w, ambition_sandbox::encounter::EncounterRegistry>,
    pub map: Res<'w, ambition_sandbox::menu::map::MapMenuState>,
    pub banner: Res<'w, ambition_sandbox::features::GameplayBanner>,
}

/// Local control-flow signal for the player tick's inline `*_phase`
/// helpers. `Return` means the phase wants the player tick to stop the
/// frame here; `Continue` means proceed to the next phase. Only used
/// by the two-clock inline phases that can short-circuit on an
/// engine-driven reset.
#[must_use]
pub(super) enum PhaseOutcome {
    Continue,
    Return,
}
