//! Publish this game's dialogue and cutscene settings for its own rooms.

use ambition_content_pack::PreparedContentPack;
use ambition_platformer2d_world::scoped_rules::{DeclareRulesExt, DeclaredRules};
use bevy::prelude::*;

pub(crate) fn register(app: &mut App, pack: &PreparedContentPack) {
    app.declare_rules(
        crate::AMBITION_RULES,
        ambition_dialog::lowered_dialogue_settings(pack)
            .copied()
            .unwrap_or_default(),
    );
    app.declare_rules(
        crate::AMBITION_RULES,
        ambition_cutscene::content_schema::lowered_cutscene_settings(pack)
            .copied()
            .unwrap_or_default(),
    );
}

pub(crate) fn publish_dialogue(world: &mut World, pack: &PreparedContentPack) {
    world
        .get_resource_or_insert_with(DeclaredRules::<ambition_dialog::DialogSettings>::default)
        .restate(
            crate::AMBITION_RULES,
            ambition_dialog::lowered_dialogue_settings(pack).copied(),
        );
}

pub(crate) fn publish_cutscenes(world: &mut World, pack: &PreparedContentPack) {
    world
        .get_resource_or_insert_with(DeclaredRules::<ambition_cutscene::CutsceneSettings>::default)
        .restate(
            crate::AMBITION_RULES,
            ambition_cutscene::content_schema::lowered_cutscene_settings(pack).copied(),
        );
}
