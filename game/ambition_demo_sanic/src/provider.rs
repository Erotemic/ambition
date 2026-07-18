//! The Sanic experience provider.

use bevy::prelude::*;

use ambition::engine_core as ae;
use ambition::provider::{AuthoredCatalogFragments, PlatformerExperienceAuthoring};
use ambition::runtime::demo_fixture::{
    ActiveRoomMetadata, LdtkRuntimeIndex, RoomSet, StartingCharacter,
};
use ambition::runtime::PreparedPlatformerSource;

use crate::{sanic_speedway, SanicRulesPlugin, SANIC_CHARACTER_ID, SPEEDWAY_ROOM_ID};

pub const SANIC_EXPERIENCE: &str = "sanic";
pub const SANIC_GAMEPLAY_ROUTE: &str = "sanic_gameplay";
pub const SANIC_LAUNCHER_ROUTE: &str = "sanic_launcher";

#[derive(Clone)]
pub struct SanicSessionWorld {
    pub geometry: ae::RoomGeometry,
    pub room_set: RoomSet,
    pub metadata: ActiveRoomMetadata,
    pub starting_character: StartingCharacter,
}

pub fn sanic_session_world() -> SanicSessionWorld {
    let room = sanic_speedway();
    let geometry = ae::RoomGeometry(room.world.clone());
    let metadata = ActiveRoomMetadata(room.metadata.clone());
    let room_set = RoomSet::from_parts(SPEEDWAY_ROOM_ID, vec![room], Vec::new());
    SanicSessionWorld {
        geometry,
        room_set,
        metadata,
        starting_character: StartingCharacter::new(SANIC_CHARACTER_ID),
    }
}

pub fn sanic_authored_catalogs() -> AuthoredCatalogFragments {
    AuthoredCatalogFragments::new(SANIC_CHARACTER_ID, SANIC_EXPERIENCE)
        .with_music()
        .with_procedural_sfx()
        .with_packed_sfx()
}

pub struct SanicExperiencePlugin;

impl Plugin for SanicExperiencePlugin {
    fn build(&self, app: &mut App) {
        crate::install_sanic_content(app);
        PlatformerExperienceAuthoring::new(
            SANIC_EXPERIENCE,
            SANIC_GAMEPLAY_ROUTE,
            "Sanic",
            "Momentum speedway with a rideable loop",
            "Prepare Sanic",
            sanic_authored_catalogs(),
        )
        .install(app, sanic_prepared_session_world);
        app.add_plugins(SanicRulesPlugin::hosted());
    }
}

/// The provider's authored speedway source for the shared preparation lifecycle.
fn sanic_prepared_session_world() -> PreparedPlatformerSource {
    let source = sanic_session_world();
    PreparedPlatformerSource::new(
        SANIC_EXPERIENCE,
        source.room_set,
        source.geometry,
        source.metadata,
        source.starting_character,
        LdtkRuntimeIndex::default(),
    )
}
