//! Flying Mary-O enemy archetypes.
//! Aerial/float behavior is character-authored; this provider also publishes their art.

/// Register both flying swarms as characters.
/// Zero aggro/attack radii keep the aerial brain roaming instead of diving;
/// `patrol_effort: 1.0` preserves the authored full-speed patrol.
pub(crate) fn register_snakes_on_a_plane_characters(app: &mut bevy::prelude::App) {
    use ambition_platformer2d::character::CharacterDefinition;
    use ambition_platformer2d::actors::character_runtime::{CharacterDefinitionAppExt};
    use ambition_platformer2d::characters::actor::{CharacterLocomotion, ContactDamage};
    use ambition_platformer2d::characters::brain::{
        BrainProfile, CharacterBrainTemplate, MoveStyleSpec,
    };

    for (id, display, sheet, run_speed, max_health) in [
        (
            PAPER_PLANE_CHARACTER_ID,
            PAPER_PLANE_DISPLAY_NAME,
            PAPER_PLANE_SHEET_TARGET,
            58.0,
            1,
        ),
        (
            CARTESIAN_PLANE_CHARACTER_ID,
            CARTESIAN_PLANE_DISPLAY_NAME,
            CARTESIAN_PLANE_SHEET_TARGET,
            38.0,
            2,
        ),
    ] {
        let mut definition =
            CharacterDefinition::new(id, display, crate::provider::MARY_O_EXPERIENCE)
                .with_sheet(sheet)
                .with_locomotion(CharacterLocomotion {
                    run_speed,
                    move_style: MoveStyleSpec::Float,
                    // Free flight is a body capability and must travel with the character definition.
                    baseline_free_flight: Some(true),
                    ..Default::default()
                })
                .with_contact_damage(ContactDamage {
                    strength: 0.5,
                    amount: 1,
                })
                .with_autonomous_profile(BrainProfile {
                    template: CharacterBrainTemplate::Aerial,
                    aggro_radius: 0.0,
                    attack_range: 0.0,
                    patrol_effort: 1.0,
                    chase_effort: 1.0,
                    ..Default::default()
                });
        definition.vitals.max_health = Some(max_health);
        app.register_character(definition);
    }
}

/// Brain key carried by the level placement alongside `character_id`.
pub const PAPER_PLANE_BRAIN_KEY: &str = "mary_o_snakes_on_a_paper_plane";
/// See [`PAPER_PLANE_BRAIN_KEY`].
pub const CARTESIAN_PLANE_BRAIN_KEY: &str = "mary_o_snakes_on_a_cartesian_plane";

/// Character id authored by the level placement.
pub const PAPER_PLANE_CHARACTER_ID: &str = "npc_snakes_on_a_paper_plane";
/// See [`PAPER_PLANE_CHARACTER_ID`].
pub const CARTESIAN_PLANE_CHARACTER_ID: &str = "npc_snakes_on_a_cartesian_plane";

/// Sprite-sheet target; deliberately distinct from the catalog character id.
pub const PAPER_PLANE_SHEET_TARGET: &str = "snakes_on_a_paper_plane";
/// See [`PAPER_PLANE_SHEET_TARGET`].
pub const CARTESIAN_PLANE_SHEET_TARGET: &str = "snakes_on_a_cartesian_plane";

/// Display name used by the enemy-render lookup.
pub const PAPER_PLANE_DISPLAY_NAME: &str = "Snakes on a Paper Plane";
/// See [`PAPER_PLANE_DISPLAY_NAME`].
pub const CARTESIAN_PLANE_DISPLAY_NAME: &str = "Snakes on a Cartesian Plane";

/// Publish each plane sheet under its sheet target, character id, and display name.
/// Demo-staged enemies cannot rely on the app host's deferred room-staging barrier.
pub fn register_snakes_on_a_plane_sheets(
    game_assets: Option<
        bevy::prelude::ResMut<ambition_platformer2d::sprite_sheet::game_assets::GameAssets>,
    >,
    config: Option<
        bevy::prelude::Res<ambition_platformer2d::sprite_sheet::game_assets::GameAssetConfig>,
    >,
    asset_server: Option<bevy::prelude::Res<bevy::prelude::AssetServer>>,
    layouts: Option<
        bevy::prelude::ResMut<bevy::prelude::Assets<bevy::prelude::TextureAtlasLayout>>,
    >,
) {
    let (Some(mut game_assets), Some(config), Some(asset_server), Some(mut layouts)) =
        (game_assets, config, asset_server, layouts)
    else {
        return;
    };
    if config.no_assets {
        return;
    }
    for (target, character_id, display_name) in [
        (
            PAPER_PLANE_SHEET_TARGET,
            PAPER_PLANE_CHARACTER_ID,
            PAPER_PLANE_DISPLAY_NAME,
        ),
        (
            CARTESIAN_PLANE_SHEET_TARGET,
            CARTESIAN_PLANE_CHARACTER_ID,
            CARTESIAN_PLANE_DISPLAY_NAME,
        ),
    ] {
        if game_assets.characters.sheet(display_name).is_some() {
            continue;
        }
        let Some(asset) =
            ambition_platformer2d::actors::character_sprites::load_prop_sheet_for_target(
                &asset_server,
                &mut layouts,
                &config.sprite_folder,
                target,
                &ambition_platformer2d::sprite_sheet::character::SheetTuning::new(1.0, 0),
            )
        else {
            continue;
        };
        game_assets.characters.publish_under(target, asset.clone());
        game_assets
            .characters
            .publish_under(character_id, asset.clone());
        game_assets.characters.publish_under(display_name, asset);
    }
}
