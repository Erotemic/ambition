//! The Sanic **experience provider**: Sanic as a launchable, teardown-clean,
//! host-independent shell experience.
//!
//! A provider owns its experience identity, its route, its session construction,
//! and its teardown — but NOT the host's initial route, home route, launcher, or
//! process-exit policy. The same [`SanicExperiencePlugin`] therefore runs
//! unchanged under the standalone Sanic host and (later) the Ambition host; only
//! the host's [`ambition::game_shell::ShellHostSpec`] differs.
//!
//! World construction moved off `Startup` (which runs once and cannot rebuild)
//! onto the shared gameplay-session bridge for this experience, so a
//! launch → quit → relaunch cycle rebuilds a genuinely fresh session. Every
//! entity the session spawns inherits the activation's
//! [`SessionScopeId`](ambition::platformer::lifecycle::SessionScopeId) (the player
//! body via `simulation_world`, the act state via the rules), so
//! the bridge retires them together from the exact shell activation.

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

use crate::{sanic_speedway, SanicRulesPlugin, SANIC_CHARACTER_ID, SPEEDWAY_ROOM_ID};

/// The launcher-visible identity of this experience.
pub const SANIC_EXPERIENCE: &str = "sanic";
/// The route a host activates to enter Sanic gameplay.
pub const SANIC_GAMEPLAY_ROUTE: &str = "sanic_gameplay";
/// The conventional home route for the standalone Sanic host. A host is free to
/// choose a different home; the provider never names it (that is the
/// host-independence claim).
pub const SANIC_LAUNCHER_ROUTE: &str = "sanic_launcher";

/// The process-resident "current world" resources for one Sanic session.
///
/// Both a host (for its build-time initial world, so the fixed-tick sim has a
/// `RoomGeometry`/`RoomSet` before the first activation) and the activation
/// handler (per relaunch) build these from the SAME source, so there is one
/// definition of what a Sanic session's world is.
pub struct SanicSessionWorld {
    pub geometry: ae::RoomGeometry,
    pub room_set: RoomSet,
    pub metadata: ActiveRoomMetadata,
    pub starting_character: StartingCharacter,
}

/// Build the "current world" resources for a Sanic session from the speedway.
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

/// The reusable Sanic provider: content registries, experience/route
/// registration, the gameplay rules, and the session activation/teardown
/// lifecycle. It does NOT insert a build-time world or configure a host — those
/// are the host's job.
pub struct SanicExperiencePlugin;

impl Plugin for SanicExperiencePlugin {
    fn build(&self, app: &mut App) {
        // Immutable, process-resident content definitions (roster + audio
        // registries). App-local composition is Phase 5; today these remain the
        // existing first-install-wins global installs.
        crate::install_sanic_content(app);

        // Advertise the experience + its gameplay route. The launcher catalog is
        // derived from this registration, so no host writes a Sanic match.
        app.register_gameplay_experience(
            ExperienceRegistration::new(SANIC_EXPERIENCE, "Sanic", SANIC_GAMEPLAY_ROUTE)
                .with_description("Momentum speedway with a rideable loop"),
            ShellRouteSpec::new(SANIC_GAMEPLAY_ROUTE, SANIC_EXPERIENCE)
                .on_complete(ShellCompletionPolicy::ReturnHome),
        );
        app.add_systems(
            Update,
            sanic_activate_session.in_set(GameplaySessionSet::Providers),
        );

        // The mode-gated gameplay rules. `spawn_sanic_mode_owner` additionally
        // sleeps when no session is live, so the launcher does not resurrect the
        // act state from stale "sanic" room metadata.
        app.add_plugins(SanicRulesPlugin::hosted());
    }
}

/// Build the real Sanic session when the shell activates the gameplay route.
///
/// Receives the fresh scope minted by the shared bridge, constructs the world
/// with that captured ownership context, and publishes the session's world
/// resources.
#[allow(clippy::too_many_arguments)]
fn sanic_activate_session(
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
        if activation.experience_id.as_str() != SANIC_EXPERIENCE {
            continue;
        }
        let scope = *scope;

        commands.spawn_in_session(
            scope,
            (
                Name::new(format!("{} session root", SANIC_EXPERIENCE)),
                SessionRoot(scope),
            ),
        );

        let world = sanic_session_world();
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
                default_character_id: crate::SANIC_CHARACTER_ID,
                sandbox_data_asset: None,
                sandbox_asset_collection: None,
                asset_server: &asset_server,
            },
        );

        // Republish the session's "current world" (a relaunch must overwrite any
        // stale world left by the previous session).
        *geometry = world.geometry;
        *room_set = world.room_set;
        *metadata = world.metadata;
        *starting_character = world.starting_character;
    }
}
