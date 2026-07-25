use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use ambition::combat::{GameplayBanner, HitEvent, ResetRoomFeaturesEvent};
use ambition::sfx::SfxWriter;
use ambition::vfx::VfxMessage;

/// Bundled `MessageWriter`s for the sim → presentation event channels
/// the player tick (and the `player_body_phase` helper it calls) writes
/// to during the gameplay tick.
///
/// Bundling them in a single `SystemParam` keeps the player tick's
/// signature under Bevy's 16-`SystemParam` budget. The phase helper
/// (`player_body_phase`) takes `&mut event_writers.sfx` /
/// `&mut event_writers.vfx` via split borrows and writes directly — no
/// intermediate Vec collectors. Other
/// channels (`ActorDiedMessage`, `DebrisBurstMessage`,
/// `RoomTransitionRequested`) are written directly from their own
/// extracted systems' `MessageWriter` params.
#[derive(SystemParam)]
pub struct SandboxEventWriters<'w> {
    pub(super) sfx: SfxWriter<'w>,
    pub(super) vfx: MessageWriter<'w, VfxMessage>,
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
    pub hit_events: MessageWriter<'w, HitEvent>,
    pub reset_room_features: MessageWriter<'w, ResetRoomFeaturesEvent>,
    pub feature_ecs_overlay: Res<'w, ambition::platformer::feature_overlay::FeatureEcsWorldOverlay>,
    pub dialogue: ResMut<'w, ambition::dialog::DialogState>,
    pub physics_settings: Res<'w, ambition::actors::world::physics::PhysicsSandboxSettings>,
    pub moving_platforms: ResMut<'w, ambition::world::collision::MovingPlatformSet>,
    pub sim_state: ResMut<'w, ambition::actors::SandboxSimState>,
    pub clock: ResMut<'w, ambition::time::ClockState>,
    pub dev_state: ResMut<'w, ambition::dev_tools::SandboxDevState>,
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
/// (the player tick, `ambition::actors::quest`, `ambition::actors::boss_encounter`, etc.).
#[derive(SystemParam)]
pub struct ProgressionResources<'w> {
    pub quests: Res<'w, ambition_content::quest::QuestRegistry>,
    pub cutscene: Res<'w, ambition::cutscene::ActiveCutscene>,
    pub cutscene_request: Res<'w, ambition::cutscene::CutsceneAdvanceRequest>,
    pub bosses: Res<'w, ambition::actors::boss_encounter::BossEncounterRegistry>,
    pub map: Res<'w, ambition::menu::map::MapMenuState>,
    pub banner: Res<'w, GameplayBanner>,
}
