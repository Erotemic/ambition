//! Prepared platformer definitions and canonical live session components.
//!
//! [`PreparedPlatformerSource`] is immutable preparation input/output: it owns
//! authored catalogs, room graph/geometry and starting-character selection.
//! [`PlatformerSessionWorld`] is the mutable live bundle lowered from that
//! source during activation. Runtime requests are created at activation and
//! never participate in content identity.
//!
//! The measurement that decided the shape: every system that consumes the index for real work lives
//! in `ambition_platformer2d_ldtk` and reads it through a `SessionWorldRef`, the monolith's
//! `SimulationSetup::ldtk_index` was dead (`let _ = ldtk_index;`), and exactly ONE site in the
//! workspace ever built a non-default index.  the index is state a FORMAT INSTALLS — see
//! [`PreparedPlatformerSource::with_installed_ldtk_index`] — not a field every game owes.

use bevy::prelude::*;

use ambition_encounter::EncounterMusicRequest;
use ambition_platformer2d_actor_monolith::avatar::{InitialBodyPolicy, StartingCharacter};
use ambition_platformer2d_world::rooms::{ActiveRoomMetadata, RoomMusicRequest, RoomSet};
use ambition_platformer2d_core::RoomGeometry;
#[cfg(feature = "ldtk")]
use ambition_platformer2d_ldtk::LdtkRuntimeIndex;

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
    /// Whether this session builds a home body at all. See
    /// [`InitialBodyPolicy`]; a match experience declares
    /// [`InitialBodyPolicy::NoInitialBody`] and realizes its own cast.
    initial_body: InitialBodyPolicy,
    /// The active-area index an authoring FORMAT installed, if any. `None`
    /// for every RON-authored game, which is what makes this optional rather
    /// than a field they fill with an empty value — see the module header.
    #[cfg(feature = "ldtk")]
    installed_ldtk_index: Option<LdtkRuntimeIndex>,
}

impl PreparedPlatformerSource {
    /// A session with a home body — every exploration experience.
    pub fn new(
        provider: impl Into<String>,
        room_set: RoomSet,
        geometry: RoomGeometry,
        active_room: ActiveRoomMetadata,
        starting_character: StartingCharacter,
    ) -> Self {
        Self {
            catalogs: PlatformerSessionCatalogs::provider(provider),
            room_set,
            geometry,
            active_room,
            initial_body: InitialBodyPolicy::SpawnCharacter(starting_character.clone()),
            starting_character,
            #[cfg(feature = "ldtk")]
            installed_ldtk_index: None,
        }
    }

    /// A session that builds NO home body, because the experience realizes
    /// its own cast.
    ///
    /// this is the constructor a MATCH wants, and until it existed the engine
    /// forced one on every session. Match seating then had to reinterpret that
    /// privileged body as a fighter — an adoption path whose costume handshake
    /// could deadlock a whole match, and which made a match with nobody local in
    /// it inexpressible.
    ///
    /// `catalog_default` still names the experience's default character: a match
    /// has no body of its own and its worn fighters still need a fallback id.
    /// That is a different question from whether a body is built, which is
    /// exactly why it is a different argument.
    pub fn for_match(
        provider: impl Into<String>,
        room_set: RoomSet,
        geometry: RoomGeometry,
        active_room: ActiveRoomMetadata,
        catalog_default: StartingCharacter,
    ) -> Self {
        Self {
            catalogs: PlatformerSessionCatalogs::provider(provider),
            room_set,
            geometry,
            active_room,
            starting_character: catalog_default,
            initial_body: InitialBodyPolicy::NoInitialBody,
            #[cfg(feature = "ldtk")]
            installed_ldtk_index: None,
        }
    }

    /// Install an authoring format's active-area index onto this definition.
    ///
    /// The one road that calls this is the LDtk one — initial preparation and
    /// hot reload, both in the LDtk-authored game. A RON-authored game never
    /// calls it and therefore carries no index at all, which is the whole point:
    /// the absence is the honest statement that no world was installed, where
    /// an empty index was a value five games had to invent.
    #[must_use]
    #[cfg(feature = "ldtk")]
    pub fn with_installed_ldtk_index(mut self, index: LdtkRuntimeIndex) -> Self {
        self.installed_ldtk_index = Some(index);
        self
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
    /// Whether this session builds a home body, and who it wears if so.
    pub fn initial_body(&self) -> &InitialBodyPolicy {
        &self.initial_body
    }
    /// The installed active-area index, or `None` when no authoring format
    /// installed one. read it as a question, never unwrapped: a RON-authored
    /// session legitimately has no answer.
    #[cfg(feature = "ldtk")]
    pub fn installed_ldtk_index(&self) -> Option<&LdtkRuntimeIndex> {
        self.installed_ldtk_index.as_ref()
    }
    pub fn active_room_id(&self) -> &str {
        self.room_set.active_spec().id.as_str()
    }

    /// Build an off-to-the-side candidate with a replacement authored world.
    /// The active prepared object is untouched until the caller commits it.
    ///
    /// the installed index CARRIES OVER unchanged. A format that reloaded its
    /// own world states the replacement with
    /// [`Self::with_installed_ldtk_index`]; a format that did not, and a game
    /// that installed nothing, both get the honest answer without saying
    /// anything.
    pub fn with_world(
        &self,
        room_set: RoomSet,
        geometry: RoomGeometry,
        active_room: ActiveRoomMetadata,
    ) -> Self {
        Self {
            catalogs: self.catalogs.clone(),
            room_set,
            geometry,
            active_room,
            starting_character: self.starting_character.clone(),
            initial_body: self.initial_body.clone(),
            #[cfg(feature = "ldtk")]
            installed_ldtk_index: self.installed_ldtk_index.clone(),
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
        // `mut` only under `ldtk`: the block that mutates the installed index is
        // behind that feature, so without it the binding is read-only.
        #[cfg_attr(not(feature = "ldtk"), allow(unused_mut))]
        let mut candidate = self.with_world(
            room_set,
            RoomGeometry(active_spec.world),
            ActiveRoomMetadata(active_spec.metadata),
        );
        // Only a session that HAS an installed index has an active area to
        // normalize; a RON-authored one tracks its active room in `RoomSet`
        // alone, which the clone above already carries.
        #[cfg(feature = "ldtk")]
        if let Some(index) = candidate.installed_ldtk_index.as_mut() {
            index.set_active_area(active_spec.id.clone());
        }
        Some(candidate)
    }

    pub fn instantiate_live(&self) -> PlatformerSessionWorld {
        PlatformerSessionWorld {
            catalogs: self.catalogs.clone(),
            room_set: self.room_set.clone(),
            geometry: self.geometry.clone(),
            active_room: self.active_room.clone(),
            starting_character: self.starting_character.clone(),
            initial_body: self.initial_body.clone(),
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
///
/// every field here is something EVERY platformer session has. An authoring format's own state
/// — the LDtk active-area index is the only current example — is installed as a SEPARATE component
/// on the same root by the road that installed the format, so a game that uses no such format
/// carries nothing for it.
#[derive(Bundle, Clone)]
pub struct PlatformerSessionWorld {
    pub catalogs: PlatformerSessionCatalogs,
    pub room_set: RoomSet,
    pub geometry: RoomGeometry,
    pub active_room: ActiveRoomMetadata,
    pub starting_character: StartingCharacter,
    pub initial_body: InitialBodyPolicy,
    pub requests: PlatformerSessionRequests,
}

impl PlatformerSessionWorld {
    pub fn active_room_id(&self) -> &str {
        self.room_set.active_spec().id.as_str()
    }
}
