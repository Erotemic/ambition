//! Mary-O scenery props — the flagpole and the warp pipes, drawn from their
//! generated construction sheets instead of flat placeholder quads.
//!
//! The goal pole and the pipes are `ae::Block`s: the pole is the level-end
//! grab target (`flag.rs`) and the pipes are solid warp geometry (`warp_through
//! _secret_pipe`). Their COLLISION lives on those blocks and never moves — this
//! module only supplies the LOOK. Each block is made visually transparent (a
//! zero-alpha `art_color`, so the shared block-art never draws a smear over it)
//! and a decorative `PropSpec` is layered on top, resolved to the matching
//! `super_mary_o_*` construction sheet.
//!
//! The sheets are CONTENT, not carried in any lean asset catalog, so — exactly
//! like Sanic's animated ring prop — a tiny per-frame "insert if missing" system
//! loads each into `GameAssets.characters.props`. That self-heals across the
//! host's wholesale `GameAssets` rebuild (a quality-scale reload wipes props) and
//! flips `GameAssets::is_changed()` so the prop-rebind pass runs. It no-ops in a
//! headless / `--no-assets` build.

use bevy::prelude::*;

use ambition::engine_core as ae;
use ambition::world::rooms::PropSpec;

/// The pipe's mouth lip (the wide rim you stand on) — a distinct sheet from the
/// shaft so a tall pipe reads as a mouth on a body, not a stretched tube.
pub const PIPE_TOP_SPRITE: &str = "super_mary_o_pipe_top";
/// The pipe's shaft, beneath the lip.
pub const PIPE_BODY_SPRITE: &str = "super_mary_o_pipe_body";
/// The flagpole's shaft.
pub const POLE_BODY_SPRITE: &str = "super_mary_o_flag_pole_body";
/// The finial capping the pole.
pub const POLE_TOP_SPRITE: &str = "super_mary_o_flag_pole_top";
/// The banner hanging off the pole (its own row is a 4-frame wave).
pub const FLAG_SPRITE: &str = "super_mary_o_flag";

/// Every construction-prop kind this level draws, so the loader below can walk
/// them uniformly.
const CONSTRUCTION_PROPS: &[&str] = &[
    PIPE_TOP_SPRITE,
    PIPE_BODY_SPRITE,
    POLE_BODY_SPRITE,
    POLE_TOP_SPRITE,
    FLAG_SPRITE,
];

/// A fully transparent block-art override. Applied to the pole/pipe collision
/// blocks so the shared per-`BlockKind` art draws nothing behind the decorative
/// prop — the prop IS the art, the block is only its hitbox.
pub const TRANSPARENT: [f32; 4] = [0.0, 0.0, 0.0, 0.0];

/// One decorative prop for a rectangular region given by a block's top-left
/// `min` and `size`. Blocks are min-anchored; a `PropSpec` is CENTER-anchored,
/// so convert once here.
pub fn prop_over(id: &str, kind: &str, min: ae::Vec2, size: ae::Vec2) -> PropSpec {
    PropSpec {
        id: id.to_string(),
        name: id.to_string(),
        kind: kind.to_string(),
        pos: ae::Vec2::new(min.x + size.x * 0.5, min.y + size.y * 0.5),
        size,
    }
}

/// Load the construction sheets into `GameAssets.characters.props`, keyed by the
/// sheet target name (which is also the `PropSpec.kind` the level authors).
/// Insert-if-missing so it self-heals after a wholesale `GameAssets` rebuild.
pub fn register_mary_o_construction_props(
    game_assets: Option<ResMut<ambition::sprite_sheet::game_assets::GameAssets>>,
    config: Option<Res<ambition::sprite_sheet::game_assets::GameAssetConfig>>,
    asset_server: Option<Res<AssetServer>>,
    layouts: Option<ResMut<Assets<TextureAtlasLayout>>>,
) {
    let (Some(mut game_assets), Some(config), Some(asset_server), Some(mut layouts)) =
        (game_assets, config, asset_server, layouts)
    else {
        return;
    };
    if config.no_assets {
        return;
    }
    for kind in CONSTRUCTION_PROPS {
        if game_assets.characters.props.contains_key(*kind) {
            continue;
        }
        if let Some(asset) = ambition::actors::character_sprites::load_prop_sheet_for_target(
            &asset_server,
            &mut layouts,
            &config.sprite_folder,
            kind,
            &ambition::sprite_sheet::character::SheetTuning::new(1.0, 0),
        ) {
            game_assets
                .characters
                .props
                .insert((*kind).to_string(), asset);
        }
    }
}
