//! The primeval trail: the level of the dinosaurs.
//!
//! Four rooms in a row in the `primeval` biome
//! (`tools/ambition_ldtk_tools/specs/primeval/make_specs.py`): the T-rex's kin
//! are its enemies and the T-rex is at its end. Its rooms, its enemies and its
//! coconuts are authored in the map. What the map cannot author is here: the
//! sheet of the scenery the rooms name.

use ambition_platformer2d_actor_monolith::assets::game_assets::{PropSheetSource, PropSheetsAppExt};
use bevy::prelude::*;

/// The `Prop` kind of the coconut palm, and its renderer target.
pub const COCONUT_PALM_KIND: &str = "coconut_palm";

pub struct PrimevalPlugin;

impl Plugin for PrimevalPlugin {
    fn build(&self, app: &mut App) {
        // The palm is scenery: its sheet loads with the rest of the art.
        app.register_prop_sheet(
            COCONUT_PALM_KIND,
            PropSheetSource::SpriteFolder {
                target: COCONUT_PALM_KIND.to_string(),
                tuning: ambition_platformer2d::sprite_sheet::character::SheetTuning::new(1.0, 0),
            },
        );
    }
}
