//! Tiny fourth-provider acceptance fixture.

use bevy::prelude::*;

use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::provider::{AuthoredCatalogFragments, PlatformerExperienceAuthoring};
use ambition_platformer2d::runtime::demo_fixture::{
    ActiveRoomMetadata, RoomSet, StartingCharacter,
};
use ambition_platformer2d::runtime::PreparedPlatformerSource;
use ambition_platformer2d::world::rooms::RoomSpec;

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
            spritesheet: "sprites/mary_o_v2_spritesheet.png",
            manifest: "sprites/mary_o_v2_spritesheet.ron",
            tier: MainHall,
            body_kind: Standard,
            composition: None,
            default_brain: "stand_still",
            default_action_set: "peaceful",
            // The pocket demo is a provider-acceptance fixture — it proves a provider can stand up
            // a room and a character, not that it can fight — so the row's own peaceful kit is the
            // honest answer and `HostCode` was borrowing the protagonist's combat to say nothing
            // with it.
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

pub fn pocket_frontend_audio_profile(
) -> ambition_platformer2d::audio::selection::FrontendAudioProfile {
    ambition_platformer2d::audio::selection::FrontendAudioProfile::new(POCKET_EXPERIENCE).with_sfx(
        [
            ambition_platformer2d::sfx::ids::UI_MENU_MOVE,
            ambition_platformer2d::sfx::ids::UI_MENU_ACCEPT,
            ambition_platformer2d::sfx::ids::UI_MENU_BACK,
        ],
    )
}

fn cue(id: Option<&str>, frequency: f32) -> ambition_platformer2d::audio::spec::SfxSpec {
    ambition_platformer2d::audio::spec::SfxSpec {
        cue: id
            .is_none()
            .then_some(ambition_platformer2d::audio::spec::SoundCueKey::Jump),
        id: id.map(str::to_owned),
        waveform: ambition_platformer2d::audio::spec::WaveformSpec::Square,
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
    use ambition_platformer2d::audio::catalog::{AudioCatalogAppExt, AudioCatalogFragment};
    use ambition_platformer2d::characters::actor::character_catalog::{
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
    // REGISTER THE CHARACTER, not only its catalog row.
    //
    // A catalog fragment declares what a character IS; `register_character` is what makes the
    // art pipeline know it exists — `declare_registered_characters` reads the PREPARED
    // REGISTRY, so a catalog-only character is `UnknownCharacter` to the materializer and draws
    // the marked placeholder.
    //
    // Which is what it did: picking "Pocket" from the launcher showed a plain blue
    // box standing on the platform. No test failed, because no test looked at the
    // screen — and the art guard that names Pocket in its own comment inspects the
    // registry, which Pocket was absent from (found by capturing the route,
    // ).
    //
    // the sheet TARGET, not the sheet FILE. `mary_o_v2_spritesheet.ron`
    // declares `target: "mary_o_v2"` and the registry is keyed by that. The
    // catalog row above pointed at `sprites/mary_o_spritesheet.*`, which does not
    // exist in this repository at all, so even the legacy path had nothing to load.
    {
        use ambition_platformer2d::character::CharacterDefinition;
        use ambition_platformer2d::actors::character_runtime::{CharacterDefinitionAppExt};
        app.register_character(
            CharacterDefinition::new(POCKET_CHARACTER_ID, "Pocket Runner", POCKET_EXPERIENCE)
                .with_sheet("mary_o_v2")
                // A VOICE, so this one is not mute on a pedestal. A
                // registered-only character has no catalog row to hold bark
                // pools, and the ambient ticker skips whoever has nothing to say.
                .with_voice([
                    "Small screen, same distance.",
                    "I run in your pocket. Mind the lint.",
                    "Everything here had to fit through a thumb.",
                ]),
        );
    }
    app.register_audio_catalog_fragment(
        AudioCatalogFragment::new(
            POCKET_EXPERIENCE,
            None,
            Some(ambition_platformer2d::audio::spec::SfxRegistry {
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
        .with_loading_activity(
            ambition_platformer2d::load_presentation::DETERMINISTIC_LOADING_ACTIVITY_ID,
        )
        // its own description says what it is: an architecture proof.
        // is a fixture that exists so a FOURTH provider proves the seam composes,
        // and every test that drives it activates its route directly — so it
        // loses nothing by not being offered.
        .unlisted()
        .with_defense_presentation(
            ambition_platformer2d::presentation::DefensePresentationPolicy::shared_iframe_blink(),
        )
        .install(app, pocket_prepared_session_world);
    }
}

/// The provider's authored pocket-room source for the shared preparation lifecycle.
fn pocket_prepared_session_world() -> PreparedPlatformerSource {
    let room = pocket_room();
    let geometry = ae::RoomGeometry(room.world.clone());
    let metadata = ActiveRoomMetadata(room.metadata.clone());
    PreparedPlatformerSource::new(
        POCKET_EXPERIENCE,
        RoomSet::from_parts(POCKET_ROOM_ID, vec![room], Vec::new()),
        geometry,
        metadata,
        StartingCharacter::new(POCKET_CHARACTER_ID),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_platformer2d::game_shell::{
        MinimalShellPlugins, ShellExperienceId, ShellExperienceRegistry, ShellRouteCatalog,
        ShellRouteId,
    };

    #[test]
    fn standalone_host_composes_the_same_provider_plugin() {
        let mut app = App::new();
        app.add_plugins(MinimalShellPlugins);
        app.add_plugins(ambition_platformer2d::load::AmbitionLoadPlugin);
        app.insert_resource(
            ambition_platformer2d::audio::selection::FrontendAudioRegistry::direct(
                pocket_frontend_audio_profile(),
            ),
        );
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
            .resource::<ambition_platformer2d::provider::PlatformerAuthoredCatalogRegistry>()
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
            .resource::<ambition_platformer2d::load_presentation::ShellLoadPresentationCatalog>()
            .for_route(&ShellRouteId::new(POCKET_GAMEPLAY_ROUTE));
        assert_eq!(
            loading.activity.as_ref().map(|activity| activity.as_str()),
            Some(ambition_platformer2d::load_presentation::DETERMINISTIC_LOADING_ACTIVITY_ID),
            "provider authoring selects the reusable load activity without host wiring",
        );
    }
}
