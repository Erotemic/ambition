use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use ambition_platformer2d::combat::GameplayBanner;
/// Read-only progression-state bundle for the HUD and pause menu.
///
/// The HUD reads several registries (quests, cutscene state, bosses,
/// encounters, world map) and would otherwise blow the 16-param budget
/// when combined with windowing / camera / font handles. Grouping them
/// behind a single param both keeps the budget headroom and documents
/// the intentional read-only contract: HUD systems must not mutate
/// progression state. Mutators live in the producer side
/// (the player tick, `ambition_platformer2d::runtime::quest`, `ambition_platformer2d::boss_encounter`, etc.).
#[derive(SystemParam)]
pub struct ProgressionResources<'w> {
    pub quests: Res<'w, ambition_content::quest::QuestRegistry>,
    pub cutscene: Res<'w, ambition_platformer2d::cutscene::ActiveCutscene>,
    pub bosses: Res<'w, ambition_platformer2d::boss_encounter::BossEncounterRegistry>,
    pub map: Res<'w, ambition_platformer2d::menu::map::MapMenuState>,
    pub banner: Res<'w, GameplayBanner>,
}
