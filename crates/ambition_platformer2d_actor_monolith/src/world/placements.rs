//! Actor-runtime facade for authored placement records.
//!
//! `ambition_platformer2d_world` owns the pure generic lowering registry. The actor runtime
//! specializes that registry with the App-local [`CharacterCatalog`] context,
//! so placement interpreters can resolve authored character ids without adding
//! an upward dependency to the world IR or consulting process-global state.

pub use ambition_platformer2d_world::placements::{
    PlacementLoweringAppExt, PlacementLoweringError, PlacementLoweringPlan, PlacementRecord,
};

use crate::features::CharacterRoster;
use ambition_characters::actor::character_catalog::CharacterCatalog;

/// Immutable App-local authored context supplied to room placement lowering.
#[derive(Clone, Debug)]
pub struct ActorPlacementContext {
    pub characters: CharacterCatalog,
    /// Sheets this app's providers authored (U1). The same authored-content
    /// class as the catalog and the roster: what a body looks like decides how
    /// big its collision box is, so lowering needs it exactly where it needs
    /// the other two.
    pub sheets: ambition_sprite_sheet::character::sheets::AuthoredSheets,
    pub roster: CharacterRoster,
}

impl ActorPlacementContext {
    pub fn new(
        characters: &CharacterCatalog,
        sheets: &ambition_sprite_sheet::character::sheets::AuthoredSheets,
        roster: &CharacterRoster,
    ) -> Self {
        Self {
            characters: characters.clone(),
            sheets: sheets.clone(),
            roster: roster.clone(),
        }
    }
}

pub type LoweringCtx<'w, 's, 'a> =
    ambition_platformer2d_world::placements::LoweringCtx<'w, 's, 'a, ActorPlacementContext>;
pub type LoweringFn = ambition_platformer2d_world::placements::LoweringFn<ActorPlacementContext>;
pub type PlacementLoweringRegistry =
    ambition_platformer2d_world::placements::PlacementLoweringRegistry<ActorPlacementContext>;
