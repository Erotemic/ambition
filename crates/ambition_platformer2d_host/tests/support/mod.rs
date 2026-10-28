//! The content-free fixture every host witness shares: one empty room, one
//! `player` character, and the engine's own `simulation_world` to spawn the body.
//! Shared by `demo_shell_smoke.rs` and `supported_profiles.rs`.

use bevy::prelude::*;

use ambition_platformer2d_core as ae;
use ambition_platformer2d_core::RoomGeometry;
use ambition_platformer2d_world::rooms::{RoomSet, RoomSpec};

/// The demo content plugin: one empty room + the engine's own sim-world
/// setup (spawns the player box). This is the shape every demo app copies.
pub struct FixtureContentPlugin;

/// A one-character catalog: the demo's content choice. Every demo installs
/// its own roster; the engine ships none (ADR 0017).
const FIXTURE_CATALOG_RON: &str = r#"(
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
        "player": (
            display_name: "Fixture Player",
            manifest: "sprites/fixture.ron",
            tier: MainHall,
            body_kind: Standard,
            composition: None,
            default_brain: "stand_still",
            default_action_set: "peaceful",
            // That variant is deleted -- no character says engine code owns its repertoire any more
            // -- so the row states what it actually is: a peaceful shell character.
            tags: ["player"],
        ),
    },
)"#;

impl Plugin for FixtureContentPlugin {
    fn build(&self, app: &mut App) {
        use ambition_characters::actor::character_catalog::CharacterCatalogFragment;
        use ambition_characters::prepared::CharacterCatalogAppExt;
        app.register_character_catalog_fragment(
            CharacterCatalogFragment::from_ron("fixture", Some("player"), FIXTURE_CATALOG_RON)
                .expect("fixture character catalog should be valid"),
        );
        let world = ae::World::new(
            "fixture_room",
            ae::Vec2::new(640.0, 480.0),
            ae::Vec2::new(96.0, 96.0),
            Vec::new(),
        );
        let room = RoomSpec::new("fixture_room", world.clone());
        let source = ambition_platformer2d_runtime::PreparedPlatformerSource::new(
            "fixture",
            RoomSet::from_parts_or_panic("fixture_room", vec![room], Vec::new()),
            RoomGeometry(world),
            ambition_platformer2d_session::initial_body::StartingCharacter::default(),
        );
        ambition_platformer2d_provider::install_direct_session_root(
            app,
            source,
            &ambition_platformer2d_provider::AuthoredCatalogFragments::new("player", "fixture"),
        )
        .expect("fixture direct prepared-content assembly must succeed");
        app.add_systems(
            Startup,
            fixture_setup.in_set(ambition_platformer2d_shared_tangle::schedule::SimulationSetupSet),
        );
    }
}

/// The demo's world construction: the engine's `simulation_world` with the
/// fixture room. Labeled `SimulationSetupSet` so the host's input attach
/// (and any other "after the world exists" startup work) orders correctly.
fn fixture_setup(
    mut commands: Commands,
    // The root the fixture's world hangs off: setup publishes the session's
    // content generation ON it.
    session_root: bevy::prelude::Single<
        bevy::prelude::Entity,
        bevy::prelude::With<ambition_platformer2d_shared_tangle::lifecycle::SessionRoot>,
    >,
    world: ambition_platformer2d_shared_tangle::lifecycle::SoleLiveRoom<RoomGeometry>,
    room_set: ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<RoomSet>,
    body_setup: ambition_platformer2d_session::setup::LiveInitialBodySetup,
    character_catalog: Res<ambition_characters::actor::character_catalog::CharacterCatalog>,
    boss_catalog: Res<ambition_boss_encounter::BossCatalog>,
    placement_lowering: Res<ambition_platformer2d_construction::placements::PlacementLoweringRegistry>,
    content_staging: Res<ambition_platformer2d_publication::construction::RoomContentStagingRegistry>,
    construction_recipes: Res<
        ambition_platformer2d_construction::ActorConstructionRegistry,
    >,
) {
    ambition_platformer2d_publication::publish_first_room(
        &mut commands,
        ambition_platformer2d_shared_tangle::lifecycle::SessionSpawnScope::UNSCOPED,
        ambition_platformer2d_publication::FirstRoom {
            session_root: *session_root,
            // A smoke fixture drops the first room's receipt.
            retention:
                ambition_platformer2d_publication::PublicationRetention::UntilTheVerdictIsRecorded,
            // One experience: the save of the world is the save of this room.
            facts:
                ambition_platformer2d_publication::CommitFactsSource::TheWorldAtTheCommit,
            room_set: &room_set,
            placement_lowering: &placement_lowering,
            content_staging: &content_staging,
            construction:
                ambition_platformer2d_publication::construction::ActorConstructionContext::new(
                    &construction_recipes,
                    &character_catalog,
                    // A smoke fixture authors no sheets; empty is the honest
                    // value and resolves exactly as this test did before U1.
                    &Default::default(),
                    ambition_platformer2d_shared_tangle::construction::ContentBinding::content_unstated(Default::default()),
                ),
            boss_catalog: &boss_catalog,
        },
    );
    body_setup.spawn(
        &mut commands,
        ambition_platformer2d_shared_tangle::lifecycle::SessionSpawnScope::UNSCOPED,
        &world,
        "player",
    );
}
