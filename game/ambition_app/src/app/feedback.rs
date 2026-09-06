use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use ambition_platformer2d::combat::GameplayBanner;
use ambition_platformer2d::sfx::SfxWriter;
use ambition_platformer2d::vfx::VfxMessage;

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
pub struct GameplayFeedbackWriters<'w> {
    pub(super) sfx: SfxWriter<'w>,
    pub(super) vfx: MessageWriter<'w, VfxMessage>,
}

/// Read-only progression-state bundle for the HUD and pause menu.
///
/// Same `SystemParam`-packing trick as `GameplayFeedbackWriters` — the HUD reads
/// from many independent registries (quests, cutscene state, bosses,
/// encounters, world map) and would otherwise blow the 16-param budget
/// when combined with windowing / camera / font handles. Grouping them
/// behind a single param both keeps the budget headroom and documents
/// the intentional read-only contract: HUD systems must not mutate
/// progression state. Mutators live in the producer side
/// (the player tick, `ambition_platformer2d::actors::quest`, `ambition_platformer2d::boss_encounter`, etc.).
#[derive(SystemParam)]
pub struct ProgressionResources<'w> {
    pub quests: Res<'w, ambition_content::quest::QuestRegistry>,
    pub cutscene: Res<'w, ambition_platformer2d::cutscene::ActiveCutscene>,
    pub cutscene_request: Res<'w, ambition_platformer2d::cutscene::CutsceneAdvanceRequest>,
    pub bosses: Res<'w, ambition_platformer2d::boss_encounter::BossEncounterRegistry>,
    pub map: Res<'w, ambition_platformer2d::menu::map::MapMenuState>,
    pub banner: Res<'w, GameplayBanner>,
}
