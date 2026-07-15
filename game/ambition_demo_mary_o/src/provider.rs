//! The Mary-O experience provider.

use bevy::prelude::*;

use ambition::engine_core as ae;
use ambition::game_shell::{
    GameplaySessionEvent, GameplaySessionSet, PreparedSessionRegistry, ShellEvent,
};
use ambition::provider::{
    cleanup_prepared_platformer_sessions, AuthoredCatalogFragments, PlatformerExperienceAuthoring,
    PlatformerPreparation, PlatformerSessionBuilder, PreparedPlatformerSessions,
};
use ambition::runtime::demo_fixture::{
    ActiveRoomMetadata, LdtkRuntimeIndex, RoomSet, StartingCharacter,
};
use ambition::runtime::PlatformerSessionWorld;

use crate::{level_1_1, MaryORulesPlugin, LEVEL_1_1_ROOM_ID};

pub const MARY_O_EXPERIENCE: &str = "mary_o";
pub const MARY_O_GAMEPLAY_ROUTE: &str = "mary_o_gameplay";
pub const MARY_O_LAUNCHER_ROUTE: &str = "mary_o_launcher";
pub const MARY_O_CHARACTER_ID: &str = "mary_o";
pub const MARY_O_MUSIC_TRACK: &str = "support_theme";
pub const MARY_O_MUSIC_ASSET_PATH: &str = "audio/music/generated/support_theme/full.ogg";

#[derive(Clone)]
pub struct MaryOSessionWorld {
    pub geometry: ae::RoomGeometry,
    pub room_set: RoomSet,
    pub metadata: ActiveRoomMetadata,
    pub starting_character: StartingCharacter,
}

pub fn mary_o_session_world() -> MaryOSessionWorld {
    let room = level_1_1();
    let geometry = ae::RoomGeometry(room.world.clone());
    let metadata = ActiveRoomMetadata(room.metadata.clone());
    let room_set = RoomSet::from_parts(LEVEL_1_1_ROOM_ID, vec![room], Vec::new());
    MaryOSessionWorld {
        geometry,
        room_set,
        metadata,
        starting_character: StartingCharacter::new(MARY_O_CHARACTER_ID),
    }
}

struct MaryOProviderMarker;
type PreparedMaryOSessions = PreparedPlatformerSessions<MaryOProviderMarker>;

pub struct MaryOExperiencePlugin;

impl Plugin for MaryOExperiencePlugin {
    fn build(&self, app: &mut App) {
        crate::install_mary_o_content(app);
        {
            use ambition::audio::catalog::{AudioCatalogAppExt, AudioCatalogFragment};
            app.register_audio_catalog_fragment(
                AudioCatalogFragment::new(
                    MARY_O_EXPERIENCE,
                    // Mary-O runs on the "Support Theme" cue. Declaring it in the
                    // provider fragment is what authorizes the session to select
                    // and play it under provider-relative audio.
                    Some(ambition::audio::spec::MusicRegistry {
                        default_track: MARY_O_MUSIC_TRACK.to_string(),
                        tracks: vec![ambition::audio::spec::MusicTrack {
                            id: MARY_O_MUSIC_TRACK.to_string(),
                            display_name: "Support Theme".to_string(),
                            asset_path: Some(MARY_O_MUSIC_ASSET_PATH.to_string()),
                        }],
                    }),
                    // Mary-O AUTHORS the cues she emits. The movement kernel writes
                    // `SfxMessage::Jump` on every jump, but under provider-relative
                    // audio a session only plays cues its fragment declares — an
                    // undeclared `player.jump` is gated to silence. Declaring the
                    // Jump cue (the classic run+jump grammar's one voice) is what
                    // makes her jump audible. Procedurally synthesized from this
                    // spec; no asset file needed.
                    Some(ambition::audio::spec::SfxRegistry {
                        sample_rate: 44_100,
                        sfx: vec![ambition::audio::spec::SfxSpec {
                            cue: Some(ambition::audio::spec::SoundCueKey::Jump),
                            id: None,
                            waveform: ambition::audio::spec::WaveformSpec::Sine,
                            frequency: 460.0,
                            frequency_end: 720.0,
                            duration: 0.085,
                            volume: 0.22,
                            attack: 0.003,
                            release: 0.045,
                            noise: 0.0,
                        }],
                    }),
                )
                .expect("Mary-O audio catalog is valid"),
            );
        }
        PlatformerExperienceAuthoring::new(
            MARY_O_EXPERIENCE,
            MARY_O_GAMEPLAY_ROUTE,
            "Mary-O",
            "Level 1-1: run, jump, grab the flag",
            "Prepare Mary-O",
            AuthoredCatalogFragments::new(MARY_O_CHARACTER_ID, MARY_O_EXPERIENCE),
        )
        .register(app);

        app.init_resource::<PreparedMaryOSessions>()
            .add_systems(
                Update,
                (
                    mary_o_prepare_session,
                    cleanup_prepared_platformer_sessions::<MaryOProviderMarker>,
                )
                    .chain()
                    .in_set(ambition::load::AmbitionLoadSet::Contributors),
            )
            .add_systems(
                Update,
                mary_o_activate_session.in_set(GameplaySessionSet::Providers),
            )
            .add_plugins(MaryORulesPlugin::hosted());
    }
}

fn mary_o_prepare_session(
    mut shell_events: MessageReader<ShellEvent>,
    mut prepared_sessions: ResMut<PreparedMaryOSessions>,
    mut preparation: PlatformerPreparation,
) {
    for event in shell_events.read() {
        let ShellEvent::PreparationRequested(transaction) = event else {
            continue;
        };
        if transaction.experience_id.as_str() != MARY_O_EXPERIENCE {
            continue;
        }
        let source = mary_o_session_world();
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

fn mary_o_activate_session(
    mut events: MessageReader<GameplaySessionEvent>,
    mut prepared_sessions: ResMut<PreparedMaryOSessions>,
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
