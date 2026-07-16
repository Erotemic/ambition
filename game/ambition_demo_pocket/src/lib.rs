//! Tiny fourth-provider acceptance fixture.

use bevy::prelude::*;

use ambition::engine_core as ae;
use ambition::provider::{AuthoredCatalogFragments, PlatformerExperienceAuthoring};
use ambition::runtime::demo_fixture::{
    ActiveRoomMetadata, LdtkRuntimeIndex, RoomSet, StartingCharacter,
};
use ambition::runtime::PlatformerSessionWorld;
use ambition::world::rooms::RoomSpec;

pub const POCKET_EXPERIENCE: &str = "pocket";
pub const POCKET_GAMEPLAY_ROUTE: &str = "pocket_gameplay";
pub const POCKET_CHARACTER_ID: &str = "pocket_runner";
pub const POCKET_ROOM_ID: &str = "pocket_room";

const POCKET_CATALOG_RON: &str = r#"(
    brain_presets: { "stand_still": StandStill },
    action_set_presets: {
        "peaceful": (
            move_style: Walk,
            melee: None,
            ranged: None,
            special: None,
        ),
    },
    characters: {
        "pocket_runner": (
            display_name: "Pocket Runner",
            spritesheet: "sprites/mary_o_spritesheet.png",
            manifest: "sprites/mary_o_spritesheet.ron",
            tier: MainHall,
            body_kind: Standard,
            composition: None,
            default_brain: "stand_still",
            default_action_set: "peaceful",
            playable_kit: HostCode,
            tags: ["player", "provider_acceptance"],
        ),
    },
)"#;

pub fn pocket_room() -> RoomSpec {
    let size = ae::Vec2::new(640.0, 360.0);
    let floor_top = 312.0;
    let world = ae::World::new(
        "Pocket Provider Room",
        size,
        ae::Vec2::new(80.0, floor_top - 64.0),
        vec![
            ae::Block::solid(
                "pocket_floor",
                ae::Vec2::new(0.0, floor_top),
                ae::Vec2::new(size.x, 48.0),
            ),
            ae::Block::one_way(
                "pocket_ledge",
                ae::Vec2::new(250.0, floor_top - 96.0),
                ae::Vec2::new(140.0, 16.0),
            ),
        ],
    );
    let mut room = RoomSpec::new(POCKET_ROOM_ID, world);
    room.metadata.mode = Some(POCKET_EXPERIENCE.to_owned());
    room
}

pub fn pocket_frontend_audio_profile() -> ambition::audio::selection::FrontendAudioProfile {
    ambition::audio::selection::FrontendAudioProfile::new(POCKET_EXPERIENCE).with_sfx([
        ambition::sfx::ids::UI_MENU_MOVE,
        ambition::sfx::ids::UI_MENU_ACCEPT,
        ambition::sfx::ids::UI_MENU_BACK,
    ])
}

fn cue(id: Option<&str>, frequency: f32) -> ambition::audio::spec::SfxSpec {
    ambition::audio::spec::SfxSpec {
        cue: id
            .is_none()
            .then_some(ambition::audio::spec::SoundCueKey::Jump),
        id: id.map(str::to_owned),
        waveform: ambition::audio::spec::WaveformSpec::Square,
        frequency,
        frequency_end: frequency * 1.25,
        duration: 0.08,
        volume: 0.3,
        attack: 0.004,
        release: 0.04,
        noise: 0.0,
    }
}

pub fn install_pocket_content(app: &mut App) {
    use ambition::audio::catalog::{AudioCatalogAppExt, AudioCatalogFragment};
    use ambition::characters::actor::character_catalog::{
        CharacterCatalogAppExt, CharacterCatalogFragment,
    };

    app.register_character_catalog_fragment(
        CharacterCatalogFragment::from_ron(
            POCKET_EXPERIENCE,
            Some(POCKET_CHARACTER_ID),
            POCKET_CATALOG_RON,
        )
        .expect("Pocket character catalog should be valid"),
    );
    app.register_audio_catalog_fragment(
        AudioCatalogFragment::new(
            POCKET_EXPERIENCE,
            None,
            Some(ambition::audio::spec::SfxRegistry {
                sample_rate: 44_100,
                sfx: vec![
                    cue(None, 520.0),
                    cue(Some("ui.menu.move_icon"), 620.0),
                    cue(Some("ui.menu.accept"), 780.0),
                    cue(Some("ui.menu.back"), 440.0),
                ],
            }),
        )
        .expect("Pocket audio catalogs should be valid"),
    );
}

pub struct PocketExperiencePlugin;

impl Plugin for PocketExperiencePlugin {
    fn build(&self, app: &mut App) {
        install_pocket_content(app);
        PlatformerExperienceAuthoring::new(
            POCKET_EXPERIENCE,
            POCKET_GAMEPLAY_ROUTE,
            "Pocket",
            "Minimal fourth-provider architecture proof",
            "Prepare Pocket",
            AuthoredCatalogFragments::new(POCKET_CHARACTER_ID, POCKET_EXPERIENCE)
                .with_procedural_sfx(),
        )
        .with_loading_activity(ambition::load_presentation::DETERMINISTIC_LOADING_ACTIVITY_ID)
        .install(app, pocket_prepared_session_world);
    }
}

/// The provider's authored pocket-room source for the shared preparation lifecycle.
fn pocket_prepared_session_world() -> PlatformerSessionWorld {
    let room = pocket_room();
    let geometry = ae::RoomGeometry(room.world.clone());
    let metadata = ActiveRoomMetadata(room.metadata.clone());
    PlatformerSessionWorld::new(
        POCKET_EXPERIENCE,
        RoomSet::from_parts(POCKET_ROOM_ID, vec![room], Vec::new()),
        geometry,
        metadata,
        StartingCharacter::new(POCKET_CHARACTER_ID),
        LdtkRuntimeIndex::default(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition::game_shell::{
        MinimalShellPlugins, ShellExperienceId, ShellExperienceRegistry, ShellRouteCatalog,
        ShellRouteId,
    };

    #[test]
    fn standalone_host_composes_the_same_provider_plugin() {
        let mut app = App::new();
        app.add_plugins(MinimalShellPlugins);
        app.add_plugins(ambition::load::AmbitionLoadPlugin);
        app.insert_resource(pocket_frontend_audio_profile());
        app.add_plugins(PocketExperiencePlugin);
        let registration = app
            .world()
            .resource::<ShellExperienceRegistry>()
            .get(&ShellExperienceId::new(POCKET_EXPERIENCE))
            .expect("standalone host sees Pocket registration");
        assert_eq!(registration.launch_route.as_str(), POCKET_GAMEPLAY_ROUTE);
        assert!(app
            .world()
            .resource::<ShellRouteCatalog>()
            .get(&ShellRouteId::new(POCKET_GAMEPLAY_ROUTE))
            .expect("standalone host sees Pocket route")
            .preparation
            .is_some());
        let authored = app
            .world()
            .resource::<ambition::provider::PlatformerAuthoredCatalogRegistry>()
            .get(POCKET_EXPERIENCE)
            .expect("standalone host sees Pocket's authoritative authored catalogs");
        assert_eq!(authored.starting_character, POCKET_CHARACTER_ID);
        assert_eq!(authored.audio_provider, POCKET_EXPERIENCE);
        assert!(!authored.expects_music);
        assert!(authored.expects_procedural_sfx);
        assert!(!authored.expects_adaptive_cues);
        assert!(!authored.expects_packed_sfx);
        let loading = app
            .world()
            .resource::<ambition::load_presentation::LoadPresentationCatalog>()
            .for_route(&ShellRouteId::new(POCKET_GAMEPLAY_ROUTE));
        assert_eq!(
            loading.activity.as_ref().map(|activity| activity.as_str()),
            Some(ambition::load_presentation::DETERMINISTIC_LOADING_ACTIVITY_ID),
            "provider authoring selects the reusable load activity without host wiring",
        );
    }
}
