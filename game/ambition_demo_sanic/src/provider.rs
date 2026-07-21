//! The Sanic experience provider.

use bevy::prelude::*;

use ambition::engine_core as ae;
use ambition::presentation::profiles;
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
        // Speed is the whole game: soft velocity-aware framing on every
        // platform, so the camera leads the runner instead of trailing it.
        .with_presentation_profiles(profiles::high_speed_full_bleed())
        // The ring tally. One declared readout; the engine never learns what a
        // ring is — `publish_sanic_ring_readout` writes the word "RINGS".
        .with_hud(
            ambition::presentation::HudDeclaration::new().slot(
                ambition::presentation::HudSlotSpec::new(RINGS_HUD_SLOT)
                    .with_region(ambition::presentation::SurroundRegion::Top)
                    .with_font_size(22.0)
                    .with_color([1.0, 0.85, 0.25, 1.0]),
            ),
        )
        .install(app, sanic_prepared_session_world);
        app.add_systems(bevy::prelude::Update, publish_sanic_ring_readout);
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

/// The slot id Sanic's ring tally publishes into. Opaque to the engine.
pub const RINGS_HUD_SLOT: &str = "sanic_rings";

/// Publish the ring count into the declared HUD.
///
/// The count needs no new simulation: rings are authored as ordinary
/// `currency:1` pickups, the shared economy credits the collector's
/// `BodyWallet`, and `PlayerHudFacts` already republishes that balance for the
/// controlled subject every tick. So this is the whole feature — read the fact,
/// name it "RINGS", hand it to the slot.
fn publish_sanic_ring_readout(
    facts: bevy::prelude::Res<ambition::sim_view::PlayerHudFacts>,
    mut readouts: bevy::prelude::ResMut<ambition::presentation::HudReadouts>,
) {
    if !facts.present {
        return;
    }
    readouts.set_labelled(RINGS_HUD_SLOT, "RINGS", facts.balance);
}
