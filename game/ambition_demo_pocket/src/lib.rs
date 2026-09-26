//! Tiny fourth-provider acceptance fixture.

use bevy::prelude::*;

pub mod pack;

use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::provider::{AuthoredCatalogFragments, PlatformerExperienceAuthoring};
use ambition_platformer2d::runtime::demo_fixture::{RoomSet, StartingCharacter};
use ambition_platformer2d::runtime::PreparedPlatformerSource;
use ambition_platformer2d::world::rooms::RoomSpec;

pub const POCKET_EXPERIENCE: &str = "pocket";
pub const POCKET_GAMEPLAY_ROUTE: &str = "pocket_gameplay";
pub const POCKET_CHARACTER_ID: &str = "pocket_runner";
pub const POCKET_ROOM_ID: &str = "pocket_room";

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

    // The cast is the pack's `character_catalog`, registered with one call.
    pack::PACK
        .cast(POCKET_EXPERIENCE, Some(POCKET_CHARACTER_ID))
        .register(app);
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
    PreparedPlatformerSource::new(
        POCKET_EXPERIENCE,
        RoomSet::from_parts_or_panic(POCKET_ROOM_ID, vec![room], Vec::new()),
        geometry,
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

    /// The runner is a prepared character with its sheet, so the art pipeline
    /// knows it: picking Pocket in the launcher once showed a plain box standing
    /// on the platform. Preparation publishes every catalog row as a character,
    /// so the pack's row alone is enough; the cast's registration adds nothing
    /// to a row that states no body scale and no moves.
    #[test]
    fn the_pocket_cast_registers_the_runner_with_its_sheet() {
        let mut app = App::new();
        install_pocket_content(&mut app);
        ambition_platformer2d::characters::prepared::close_preparation_barrier(
            app.world_mut(),
        );
        let runner = app
            .world()
            .resource::<ambition_platformer2d::characters::prepared::PreparedCharacterRegistry>()
            .get(POCKET_CHARACTER_ID)
            .expect("the pack's cast registers the runner");
        assert_eq!(runner.sheet.as_deref(), Some("mary_o_v2"));
    }
}
