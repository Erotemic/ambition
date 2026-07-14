//! The Mary-O experience provider.

use bevy::prelude::*;

use ambition::engine_core as ae;
use ambition::game_shell::{
    GameplaySessionEvent, GameplaySessionSet, PreparedSessionRegistry, ShellEvent,
};
use ambition::provider::{
    cleanup_prepared_platformer_sessions, AuthoredCatalogFragments,
    PlatformerExperienceAuthoring, PlatformerPreparation, PlatformerSessionBuilder,
    PreparedPlatformerSessions,
};
use ambition::runtime::demo_fixture::{
    ActiveRoomMetadata, LdtkRuntimeIndex, RoomSet, StartingCharacter,
};
use ambition::runtime::PlatformerSessionWorld;

use crate::{level_1_1, Smb1RulesPlugin, LEVEL_1_1_ROOM_ID};

pub const MARY_O_EXPERIENCE: &str = "mary_o";
pub const MARY_O_GAMEPLAY_ROUTE: &str = "mary_o_gameplay";
pub const MARY_O_LAUNCHER_ROUTE: &str = "mary_o_launcher";
pub const MARY_O_CHARACTER_ID: &str = "mary_o";

#[derive(Clone)]
pub struct Smb1SessionWorld {
    pub geometry: ae::RoomGeometry,
    pub room_set: RoomSet,
    pub metadata: ActiveRoomMetadata,
    pub starting_character: StartingCharacter,
}

pub fn smb1_session_world() -> Smb1SessionWorld {
    let room = level_1_1();
    let geometry = ae::RoomGeometry(room.world.clone());
    let metadata = ActiveRoomMetadata(room.metadata.clone());
    let room_set = RoomSet::from_parts(LEVEL_1_1_ROOM_ID, vec![room], Vec::new());
    Smb1SessionWorld {
        geometry,
        room_set,
        metadata,
        starting_character: StartingCharacter::new(MARY_O_CHARACTER_ID),
    }
}

struct Smb1ProviderMarker;
type PreparedSmb1Sessions = PreparedPlatformerSessions<Smb1ProviderMarker>;

pub struct Smb1ExperiencePlugin;

impl Plugin for Smb1ExperiencePlugin {
    fn build(&self, app: &mut App) {
        crate::install_smb1_content(app);
        {
            use ambition::audio::catalog::{AudioCatalogAppExt, AudioCatalogFragment};
            app.register_audio_catalog_fragment(
                AudioCatalogFragment::new(MARY_O_EXPERIENCE, None, None)
                    .expect("Mary-O silent audio fragment is valid"),
            );
        }
        PlatformerExperienceAuthoring::new(
            MARY_O_EXPERIENCE,
            MARY_O_GAMEPLAY_ROUTE,
            "Mary-O",
            "SMB1 level 1-1: run, jump, grab the flag",
            "Prepare Mary-O",
            AuthoredCatalogFragments::new(MARY_O_CHARACTER_ID, MARY_O_EXPERIENCE),
        )
        .register(app);

        app.init_resource::<PreparedSmb1Sessions>()
            .add_systems(
                Update,
                (
                    smb1_prepare_session,
                    cleanup_prepared_platformer_sessions::<Smb1ProviderMarker>,
                )
                    .chain()
                    .in_set(ambition::load::AmbitionLoadSet::Contributors),
            )
            .add_systems(
                Update,
                smb1_activate_session.in_set(GameplaySessionSet::Providers),
            )
            .add_plugins(Smb1RulesPlugin::hosted());
    }
}

fn smb1_prepare_session(
    mut shell_events: MessageReader<ShellEvent>,
    mut prepared_sessions: ResMut<PreparedSmb1Sessions>,
    mut preparation: PlatformerPreparation,
) {
    for event in shell_events.read() {
        let ShellEvent::PreparationRequested(transaction) = event else {
            continue;
        };
        if transaction.experience_id.as_str() != MARY_O_EXPERIENCE {
            continue;
        }
        let source = smb1_session_world();
        let live_world = PlatformerSessionWorld::new(
            MARY_O_EXPERIENCE,
            source.room_set,
            source.geometry,
            source.metadata,
            source.starting_character,
            LdtkRuntimeIndex::default(),
        );
        preparation.prepare(transaction, live_world, &mut prepared_sessions);
    }
}

fn smb1_activate_session(
    mut events: MessageReader<GameplaySessionEvent>,
    mut prepared_sessions: ResMut<PreparedSmb1Sessions>,
    mut prepared_registry: ResMut<PreparedSessionRegistry>,
    mut builder: PlatformerSessionBuilder,
) {
    for event in events.read() {
        let GameplaySessionEvent::Activated { activation, scope } = event else {
            continue;
        };
        if activation.experience_id.as_str() != MARY_O_EXPERIENCE {
            continue;
        }
        let prepared = activation
            .prepared_session
            .as_ref()
            .expect("Mary-O routes require an exact prepared-session publication");
        let live_world = prepared_sessions
            .take(prepared, &mut prepared_registry)
            .expect("Mary-O prepared data must match the authorized transaction");
        builder.build(activation, *scope, live_world, MARY_O_CHARACTER_ID);
    }
}
