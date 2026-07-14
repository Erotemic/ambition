//! Canonical live platformer-session world data.
//!
//! The prepared value is a Bevy [`Bundle`]. A shell-routed host attaches it to
//! the exact [`SessionRoot`](ambition_platformer_primitives::lifecycle::SessionRoot)
//! entity for one gameplay activation; a direct host does the same once at
//! startup. Every consumer reads or mutates these components on that root.
//! There is no process-resident mirror and no two-way synchronization bridge.

use bevy::prelude::*;

use ambition_actors::avatar::StartingCharacter;
use ambition_actors::ldtk_world::LdtkRuntimeIndex;
use ambition_actors::rooms::{ActiveRoomMetadata, RoomMusicRequest, RoomSet};
use ambition_encounter::EncounterMusicRequest;
use ambition_engine_core::RoomGeometry;

#[derive(Component, Clone, Debug, Eq, PartialEq)]
pub struct PlatformerSessionCatalogs {
    pub world_provider: String,
    pub character_provider: String,
    pub audio_provider: String,
}

impl PlatformerSessionCatalogs {
    pub fn provider(provider: impl Into<String>) -> Self {
        let provider = provider.into();
        Self {
            world_provider: provider.clone(),
            character_provider: provider.clone(),
            audio_provider: provider,
        }
    }
}

#[derive(Bundle, Clone, Debug, Default)]
pub struct PlatformerSessionRequests {
    pub room_music: RoomMusicRequest,
    pub encounter_music: EncounterMusicRequest,
}

/// Prepared and live world data for one exact gameplay activation.
///
/// Before activation this value is immutable preparation output. Activation
/// inserts the bundle on the canonical session root; from that moment its
/// component fields are the only mutable world authority.
#[derive(Bundle, Clone)]
pub struct PlatformerSessionWorld {
    pub catalogs: PlatformerSessionCatalogs,
    pub room_set: RoomSet,
    pub geometry: RoomGeometry,
    pub active_room: ActiveRoomMetadata,
    pub starting_character: StartingCharacter,
    pub runtime_rooms: LdtkRuntimeIndex,
    pub requests: PlatformerSessionRequests,
}

impl PlatformerSessionWorld {
    pub fn new(
        provider: impl Into<String>,
        room_set: RoomSet,
        geometry: RoomGeometry,
        active_room: ActiveRoomMetadata,
        starting_character: StartingCharacter,
        runtime_rooms: LdtkRuntimeIndex,
    ) -> Self {
        Self {
            catalogs: PlatformerSessionCatalogs::provider(provider),
            room_set,
            geometry,
            active_room,
            starting_character,
            runtime_rooms,
            requests: PlatformerSessionRequests::default(),
        }
    }

    pub fn active_room_id(&self) -> &str {
        self.room_set.active_spec().id.as_str()
    }
}
