#[allow(unused_imports)]
use super::cli::*;
#[allow(unused_imports)]
use super::dev_runtime::*;
#[allow(unused_imports)]
use super::hud::*;
#[allow(unused_imports)]
use super::input_systems::*;
#[allow(unused_imports)]
use super::phases::*;
#[allow(unused_imports)]
use super::plugins::*;
#[allow(unused_imports)]
use super::resources::*;
#[allow(unused_imports)]
use super::setup_systems::*;
#[allow(unused_imports)]
use super::update::*;
#[allow(unused_imports)]
use super::world_flow::*;
#[allow(unused_imports)]
use super::*;

/// Bundled `MessageWriter`s for the sim → presentation event channels
/// `sandbox_update` (and the inline `*_phase` helpers it calls) writes
/// to during the gameplay tick.
///
/// Bundling them in a single `SystemParam` keeps `sandbox_update`'s
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
pub struct CombatRoomReset<'w> {
    pub enemy_projectiles: ResMut<'w, crate::enemy_projectile::EnemyProjectileState>,
    pub slot_board: ResMut<'w, crate::combat_slots::CombatSlotsRes>,
    pub feature_overlay: Res<'w, crate::features::FeatureEcsWorldOverlay>,
}

impl<'w> CombatRoomReset<'w> {
    /// Drop every in-flight enemy projectile + every slot
    /// reservation. Called by the room-transition path so a fresh
    /// arena doesn't inherit hostile shots or stale assignments
    /// from the room the player just left, AND by the same-room
    /// reset path so a player death + respawn comes back to a
    /// clean combat state.
    pub fn clear_carryover(&mut self) {
        self.enemy_projectiles.clear();
        self.slot_board.0.clear_assignments();
    }
}

/// Mutable producer streams `sandbox_update` writes into during the gameplay
/// tick.
///
/// Phase-1 strangler rule: typed gameplay effects now travel through Bevy
/// `Message<GameplayEffect>` rather than a custom `FeatureEventBus` resource.
/// Bundling the writer here keeps `sandbox_update` under Bevy's
/// 16-`SystemParam` budget while making the new cross-system transport
/// explicit.
///
/// Add new sim → sim streams (NOT sim → presentation, which is
/// `SandboxEventWriters`) here when they grow naturally; resist the urge to
/// thread them through the system signature directly.
#[derive(SystemParam)]
pub struct SandboxQueues<'w> {
    pub gameplay_effects: MessageWriter<'w, crate::features::GameplayEffect>,
    pub pogo_bounces: MessageWriter<'w, crate::features::PogoBounceEvent>,
    pub reset_room_features: MessageWriter<'w, crate::features::ResetRoomFeaturesEvent>,
    pub feature_ecs_overlay: Res<'w, crate::features::FeatureEcsWorldOverlay>,
    pub dialogue: ResMut<'w, crate::dialog::DialogState>,
    pub physics_settings: Res<'w, crate::world::physics::PhysicsSandboxSettings>,
    pub moving_platforms: ResMut<'w, crate::MovingPlatformSet>,
    pub sim_state: ResMut<'w, crate::SandboxSimState>,
    pub dev_state: ResMut<'w, crate::SandboxDevState>,
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
/// (`sandbox_update`, `crate::quest`, `crate::boss_encounter`, etc.).
#[derive(SystemParam)]
pub struct ProgressionResources<'w> {
    pub quests: Res<'w, crate::content::quest::QuestRegistry>,
    pub cutscene: Res<'w, crate::presentation::cutscene::ActiveCutscene>,
    pub cutscene_request: Res<'w, crate::presentation::cutscene::CutsceneAdvanceRequest>,
    pub bosses: Res<'w, crate::boss_encounter::BossEncounterRegistry>,
    pub encounters: Res<'w, crate::encounter::EncounterRegistry>,
    pub map: Res<'w, crate::map_menu::MapMenuState>,
    pub banner: Res<'w, crate::features::GameplayBanner>,
}

/// Local control-flow signal for `sandbox_update`'s inline `*_phase`
/// helpers. `Return` means the phase wants `sandbox_update` to stop the
/// frame here; `Continue` means proceed to the next phase. Only used
/// by the two-clock inline phases that can short-circuit on an
/// engine-driven reset.
#[must_use]
pub(super) enum PhaseOutcome {
    Continue,
    Return,
}
