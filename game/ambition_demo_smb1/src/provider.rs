//! The Mary-O **experience provider**: SMB1 as a launchable, teardown-clean,
//! host-independent shell experience.
//!
//! This is the second customer of the same shell-to-session bridge used by
//! Sanic. The shared bridge mints and retires the engine-neutral session scope;
//! this provider contributes only Mary-O registration, rules, and session
//! construction. Two unrelated games therefore exercise one lifecycle rather
//! than parallel activation bookkeeping.

use bevy::prelude::*;

use ambition::engine_core as ae;
use ambition::game_shell::{
    ExperienceRegistration, GameplaySessionAppExt, GameplaySessionEvent, GameplaySessionSet,
    ShellCompletionPolicy, ShellRouteSpec,
};
use ambition::platformer::lifecycle::{SessionRoot, SessionSpawnScope, SpawnSessionScopedExt};
use ambition::runtime::demo_fixture::{
    simulation_world, ActiveRoomMetadata, EditableAbilitySet, EditableMovementTuning,
    LdtkRuntimeIndex, RoomSet, SimulationSetup, StartingCharacter,
};

use crate::{level_1_1, Smb1RulesPlugin, LEVEL_1_1_ROOM_ID};

/// The launcher-visible identity of this experience.
pub const MARY_O_EXPERIENCE: &str = "mary_o";
/// The route a host activates to enter Mary-O gameplay.
pub const MARY_O_GAMEPLAY_ROUTE: &str = "mary_o_gameplay";
/// The conventional home route for the standalone Mary-O host. A host may choose
/// a different home; the provider never names it.
pub const MARY_O_LAUNCHER_ROUTE: &str = "mary_o_launcher";
/// The catalog character the Mary-O session's player wears.
pub const MARY_O_CHARACTER_ID: &str = "mary_o";

/// The process-resident "current world" resources for one Mary-O session. See
/// [`ambition_demo_sanic::SanicSessionWorld`] for the shared rationale.
pub struct Smb1SessionWorld {
    pub geometry: ae::RoomGeometry,
    pub room_set: RoomSet,
    pub metadata: ActiveRoomMetadata,
    pub starting_character: StartingCharacter,
}

/// Build the "current world" resources for a Mary-O session from level 1-1.
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

/// The reusable Mary-O provider: content, experience/route registration, the
/// gameplay rules, and the session activation/teardown lifecycle. Host-independent.
pub struct Smb1ExperiencePlugin;

impl Plugin for Smb1ExperiencePlugin {
    fn build(&self, app: &mut App) {
        crate::install_smb1_content(app);

        app.register_gameplay_experience(
            ExperienceRegistration::new(MARY_O_EXPERIENCE, "Mary-O", MARY_O_GAMEPLAY_ROUTE)
                .with_description("SMB1 level 1-1: run, jump, grab the flag"),
            ShellRouteSpec::new(MARY_O_GAMEPLAY_ROUTE, MARY_O_EXPERIENCE)
                .on_complete(ShellCompletionPolicy::ReturnHome),
        );
        app.add_systems(
            Update,
            smb1_activate_session.in_set(GameplaySessionSet::Providers),
        );

        app.add_plugins(Smb1RulesPlugin::hosted());
    }
}

/// Build the real Mary-O session when the shell activates the gameplay route.
#[allow(clippy::too_many_arguments)]
fn smb1_activate_session(
    mut events: MessageReader<GameplaySessionEvent>,
    mut commands: Commands,
    ldtk_index: Res<LdtkRuntimeIndex>,
    editable_abilities: Res<EditableAbilitySet>,
    editable_tuning: Res<EditableMovementTuning>,
    asset_server: Res<AssetServer>,
    character_catalog: Res<ambition::characters::actor::character_catalog::CharacterCatalog>,
    mut geometry: ResMut<ae::RoomGeometry>,
    mut room_set: ResMut<RoomSet>,
    mut metadata: ResMut<ActiveRoomMetadata>,
    mut starting_character: ResMut<StartingCharacter>,
) {
    for event in events.read() {
        let GameplaySessionEvent::Activated { activation, scope } = event else {
            continue;
        };
        if activation.experience_id.as_str() != MARY_O_EXPERIENCE {
            continue;
        }
        let scope = *scope;

        commands.spawn_in_session(
            scope,
            (
                Name::new(format!("{} session root", MARY_O_EXPERIENCE)),
                SessionRoot(scope),
            ),
        );

        let world = smb1_session_world();
        simulation_world(
            &mut commands,
            SessionSpawnScope::scoped(scope),
            SimulationSetup {
                world: &world.geometry,
                room_set: &world.room_set,
                ldtk_index: &ldtk_index,
                editable_abilities: &editable_abilities,
                editable_tuning: &editable_tuning,
                starting_character: &world.starting_character,
                character_catalog: &character_catalog,
                default_character_id: MARY_O_CHARACTER_ID,
                sandbox_data_asset: None,
                sandbox_asset_collection: None,
                asset_server: &asset_server,
            },
        );

        *geometry = world.geometry;
        *room_set = world.room_set;
        *metadata = world.metadata;
        *starting_character = world.starting_character;
    }
}
