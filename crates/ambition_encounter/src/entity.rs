//! The encounter as a first-class ENTITY.
//!
//! E1 makes the live encounter a Bevy entity rather than a value in a
//! resource-owned map: [`Encounter`] is its stable identity and the entity
//! carries the live [`EncounterState`](crate::EncounterState) component. The
//! [`EncounterRegistry`](crate::EncounterRegistry) is reduced to an
//! `id -> Entity` index (no duplicate live state).
//!
//! [`EncounterView`] is the one cross-crate PRESENTATION read-model (§6): the
//! host publishes it each tick from the live encounter entities so presentation
//! adapters in other crates (the camera) read a stable resource instead of
//! reaching into the entity representation.

use bevy::prelude::*;

/// Stable identity of a live encounter entity — matches the authored id (the
/// LDtk `EncounterTrigger.id` for waves; the boss placement id for a boss
/// fight). The [`EncounterRegistry`](crate::EncounterRegistry) indexes entities
/// by this.
#[derive(Component, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Encounter {
    pub id: String,
}

impl Encounter {
    pub fn new(id: impl Into<String>) -> Self {
        Self { id: id.into() }
    }
}

/// The one encounter PRESENTATION read-model (§6, started minimal at E1).
///
/// Cross-crate presentation adapters (the camera today) must not query the
/// encounter entities directly — the host publishes the derived presentation
/// intent here each tick, so those adapters stay decoupled from the encounter
/// state representation. Grows (music already has its own stream) as later
/// slices route HUD/camera/lock intent through the read model.
#[derive(Resource, Clone, Copy, Debug)]
pub struct EncounterView {
    /// Camera zoom the active encounters want this frame (`1.0` = no zoom).
    pub camera_zoom: f32,
}

impl Default for EncounterView {
    fn default() -> Self {
        Self { camera_zoom: 1.0 }
    }
}
