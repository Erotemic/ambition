//! The Sanic experience provider.

use bevy::prelude::*;

use ambition::engine_core as ae;
use ambition::game_shell::{
    standard_preparation_failed_commands, standard_preparation_succeeded_commands,
    GameplaySessionEvent, GameplaySessionSet, PreparedSessionRegistry, ShellEvent,
};
use ambition::provider::{
    cleanup_prepared_platformer_sessions, AuthoredCatalogFragments,
    PlatformerExperienceAuthoring, PlatformerSessionBuilder, PreparedPlatformerSessions,
};
use ambition::runtime::demo_fixture::{
    ActiveRoomMetadata, LdtkRuntimeIndex, RoomSet, StartingCharacter,
};
use ambition::runtime::PlatformerSessionWorld;

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

struct SanicProviderMarker;
type PreparedSanicSessions = PreparedPlatformerSessions<SanicProviderMarker>;

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
            AuthoredCatalogFragments::new(SANIC_CHARACTER_ID, SANIC_EXPERIENCE),
        )
        .register(app);

        app.init_resource::<PreparedSanicSessions>()
            .add_systems(
                Update,
                (
                    sanic_prepare_session,
                    cleanup_prepared_platformer_sessions::<SanicProviderMarker>,
                )
                    .chain()
                    .in_set(ambition::load::AmbitionLoadSet::Contributors),
            )
            .add_systems(
                Update,
                sanic_activate_session.in_set(GameplaySessionSet::Providers),
            )
            .add_plugins(SanicRulesPlugin::hosted());
    }
}

fn sanic_prepare_session(
    mut shell_events: MessageReader<ShellEvent>,
    ldtk_index: Res<LdtkRuntimeIndex>,
    character_catalog: Res<ambition::characters::actor::character_catalog::CharacterCatalog>,
    audio_catalogs: Res<ambition::audio::catalog::AudioCatalogRegistry>,
    mut prepared_sessions: ResMut<PreparedSanicSessions>,
    mut prepared_registry: ResMut<PreparedSessionRegistry>,
    mut load_commands: MessageWriter<ambition::load::LoadCommand>,
) {
    let catalogs = AuthoredCatalogFragments::new(SANIC_CHARACTER_ID, SANIC_EXPERIENCE);
    for event in shell_events.read() {
        let ShellEvent::PreparationRequested(transaction) = event else {
            continue;
        };
        if transaction.experience_id.as_str() != SANIC_EXPERIENCE {
            continue;
        }
        if let Some((work_id, failure)) = catalogs.validate(&character_catalog, &audio_catalogs) {
            for command in standard_preparation_failed_commands(transaction, work_id, failure) {
                load_commands.write(command);
            }
            continue;
        }
        let source = sanic_session_world();
        let live_world = PlatformerSessionWorld::new(
            SANIC_EXPERIENCE,
            source.room_set,
            source.geometry,
            source.metadata,
            source.starting_character,
            ldtk_index.clone(),
        );
        if prepared_sessions
            .publish(transaction, live_world, &mut prepared_registry)
            .is_none()
        {
            continue;
        }
        for command in standard_preparation_succeeded_commands(transaction) {
            load_commands.write(command);
        }
    }
}

fn sanic_activate_session(
    mut events: MessageReader<GameplaySessionEvent>,
    mut prepared_sessions: ResMut<PreparedSanicSessions>,
    mut prepared_registry: ResMut<PreparedSessionRegistry>,
    mut builder: PlatformerSessionBuilder,
) {
    for event in events.read() {
        let GameplaySessionEvent::Activated { activation, scope } = event else {
            continue;
        };
        if activation.experience_id.as_str() != SANIC_EXPERIENCE {
            continue;
        }
        let prepared = activation
            .prepared_session
            .as_ref()
            .expect("Sanic routes require an exact prepared-session publication");
        let live_world = prepared_sessions
            .take(prepared, &mut prepared_registry)
            .expect("Sanic prepared data must match the authorized transaction");
        builder.build(activation, *scope, live_world, SANIC_CHARACTER_ID);
    }
}
