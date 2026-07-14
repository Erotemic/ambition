//! Canonical live platformer-session world data.
//!
//! A shell-routed host attaches this component to the exact session-owned world
//! entity. The runtime knows nothing about shell routes or host policy.

use bevy::prelude::*;

use ambition_actors::avatar::StartingCharacter;
use ambition_actors::ldtk_world::LdtkRuntimeIndex;
use ambition_actors::rooms::{ActiveRoomMetadata, RoomMusicRequest, RoomSet};
use ambition_encounter::EncounterMusicRequest;
use ambition_engine_core::RoomGeometry;

#[derive(Clone, Debug, Eq, PartialEq)]
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

#[derive(Clone, Debug, Default)]
pub struct PlatformerSessionRequests {
    pub room_music: RoomMusicRequest,
    pub encounter_music: EncounterMusicRequest,
}

/// Live mutable world for one exact gameplay activation.
#[derive(Component, Clone)]
pub struct PlatformerSessionWorld {
    pub room_set: RoomSet,
    pub geometry: RoomGeometry,
    pub active_room: ActiveRoomMetadata,
    pub starting_character: StartingCharacter,
    pub runtime_rooms: LdtkRuntimeIndex,
    pub catalogs: PlatformerSessionCatalogs,
    pub requests: PlatformerSessionRequests,
    pub revision: u64,
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
            revision: 0,
        }
    }

    pub fn active_room_id(&self) -> &str {
        self.room_set.active_spec().id.as_str()
    }
}
