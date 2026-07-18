//! Prepared platformer definitions and canonical live session components.
//!
//! [`PreparedPlatformerSource`] is immutable preparation input/output: it owns
//! authored catalogs, room graph/geometry, starting-character selection, and
//! the LDtk runtime index. [`PlatformerSessionWorld`] is the mutable live bundle
//! lowered from that source during activation. Runtime requests are created at
//! activation and never participate in content identity.

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

/// Immutable provider-owned definitions before activation. Fields are private
/// so successful assembly cannot be mutated through a shared candidate.
#[derive(Clone)]
pub struct PreparedPlatformerSource {
    catalogs: PlatformerSessionCatalogs,
    room_set: RoomSet,
    geometry: RoomGeometry,
    active_room: ActiveRoomMetadata,
    starting_character: StartingCharacter,
    runtime_rooms: LdtkRuntimeIndex,
}

impl PreparedPlatformerSource {
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
        }
    }

    pub fn catalogs(&self) -> &PlatformerSessionCatalogs {
        &self.catalogs
    }
    pub fn room_set(&self) -> &RoomSet {
        &self.room_set
    }
    pub fn geometry(&self) -> &RoomGeometry {
        &self.geometry
    }
    pub fn active_room(&self) -> &ActiveRoomMetadata {
        &self.active_room
    }
    pub fn starting_character(&self) -> &StartingCharacter {
        &self.starting_character
    }
    pub fn runtime_rooms(&self) -> &LdtkRuntimeIndex {
        &self.runtime_rooms
    }
    pub fn active_room_id(&self) -> &str {
        self.room_set.active_spec().id.as_str()
    }

    /// Build an off-to-the-side candidate with a replacement authored world.
    /// The active prepared object is untouched until the caller commits it.
    pub fn with_world(
        &self,
        room_set: RoomSet,
        geometry: RoomGeometry,
        active_room: ActiveRoomMetadata,
        runtime_rooms: LdtkRuntimeIndex,
    ) -> Self {
        Self {
            catalogs: self.catalogs.clone(),
            room_set,
            geometry,
            active_room,
            starting_character: self.starting_character.clone(),
            runtime_rooms,
        }
    }

    /// Normalize a replacement world to the immutable definition's activation
    /// room. Live room movement mutates the session's `RoomSet::active`, but it
    /// must not become authored content merely because hot reload was requested
    /// from another room.
    pub fn with_definition_active_room(&self, room_id: &str) -> Option<Self> {
        let mut room_set = self.room_set.clone();
        room_set.active = room_set.room_index_by_id(room_id)?;
        let active_spec = room_set.active_spec().clone();
        let mut runtime_rooms = self.runtime_rooms.clone();
        runtime_rooms.set_active_area(active_spec.id.clone());
        Some(self.with_world(
            room_set,
            RoomGeometry(active_spec.world),
            ActiveRoomMetadata(active_spec.metadata),
            runtime_rooms,
        ))
    }

    pub fn instantiate_live(&self) -> PlatformerSessionWorld {
        PlatformerSessionWorld {
            catalogs: self.catalogs.clone(),
            room_set: self.room_set.clone(),
            geometry: self.geometry.clone(),
            active_room: self.active_room.clone(),
            starting_character: self.starting_character.clone(),
            runtime_rooms: self.runtime_rooms.clone(),
            requests: PlatformerSessionRequests::default(),
        }
    }
}

#[derive(Bundle, Clone, Debug, Default)]
pub struct PlatformerSessionRequests {
    pub room_music: RoomMusicRequest,
    pub encounter_music: EncounterMusicRequest,
}

/// Mutable components owned by the canonical live session root. This bundle is
/// constructed only by lowering an immutable [`PreparedPlatformerSource`].
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
    pub fn active_room_id(&self) -> &str {
        self.room_set.active_spec().id.as_str()
    }
}
