//! Actor-runtime facade for authored placement records.
//!
//! `ambition_world` owns the pure generic lowering registry. The actor runtime
//! specializes that registry with the App-local [`CharacterCatalog`] context,
//! so placement interpreters can resolve authored character ids without adding
//! an upward dependency to the world IR or consulting process-global state.

pub use ambition_world::placements::{
    PlacementLoweringAppExt, PlacementLoweringError, PlacementLoweringPlan, PlacementRecord,
};

use crate::features::CharacterRoster;
use ambition_characters::actor::character_catalog::CharacterCatalog;

/// Immutable App-local authored context supplied to room placement lowering.
#[derive(Clone, Debug)]
pub struct ActorPlacementContext {
    pub characters: CharacterCatalog,
    pub roster: CharacterRoster,
}

impl ActorPlacementContext {
    pub fn new(characters: &CharacterCatalog, roster: &CharacterRoster) -> Self {
        Self {
            characters: characters.clone(),
            roster: roster.clone(),
        }
    }
}

pub type LoweringCtx<'w, 's, 'a> =
    ambition_world::placements::LoweringCtx<'w, 's, 'a, ActorPlacementContext>;
pub type LoweringFn = ambition_world::placements::LoweringFn<ActorPlacementContext>;
pub type PlacementLoweringRegistry =
    ambition_world::placements::PlacementLoweringRegistry<ActorPlacementContext>;
