//! The Mary-O **experience provider**: SMB1 as a launchable, teardown-clean,
//! host-independent shell experience.
//!
//! This is the second customer of the exact architecture Sanic proved
//! (`game/ambition_demo_sanic/src/provider.rs`): the lifecycle mechanics —
//! begin a session scope on activation, build the real world through
//! `simulation_world`, retire the scope on deactivation — are identical, and only
//! the game-specific content (the level, the mode tag, the character) differs.
//! That two unrelated demos share this shape is the point.

use bevy::prelude::*;

use ambition::engine_core as ae;
use ambition::game_shell::{
    ExperienceRegistration, ShellActivationId, ShellCompletionPolicy, ShellEvent,
    ShellExperienceAppExt, ShellRouteSpec,
};
use ambition::platformer::lifecycle::{ActiveSessionScope, SessionScopeId, SessionScopeRetired};
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

/// Maps a shell activation to the session scope it began. Ordered `Vec`, not a
/// hash map, so the mapping is deterministic (ADR 0023).
#[derive(Resource, Default)]
pub struct Smb1SessionLink {
    bindings: Vec<(ShellActivationId, SessionScopeId)>,
}

impl Smb1SessionLink {
    fn bind(&mut self, activation: ShellActivationId, scope: SessionScopeId) {
        self.bindings.push((activation, scope));
    }

    fn unbind(&mut self, activation: ShellActivationId) -> Option<SessionScopeId> {
        let index = self.bindings.iter().position(|(a, _)| *a == activation)?;
        Some(self.bindings.remove(index).1)
    }
}

/// The reusable Mary-O provider: content, experience/route registration, the
/// gameplay rules, and the session activation/teardown lifecycle. Host-independent.
pub struct Smb1ExperiencePlugin;

impl Plugin for Smb1ExperiencePlugin {
    fn build(&self, app: &mut App) {
        crate::install_smb1_content();

        app.register_experience(
            ExperienceRegistration::new(MARY_O_EXPERIENCE, "Mary-O", MARY_O_GAMEPLAY_ROUTE)
                .with_description("SMB1 level 1-1: run, jump, grab the flag"),
            ShellRouteSpec::new(MARY_O_GAMEPLAY_ROUTE, MARY_O_EXPERIENCE)
                .on_complete(ShellCompletionPolicy::ReturnHome),
        );

        app.init_resource::<Smb1SessionLink>();
        app.add_systems(
            Update,
            (smb1_activate_session, smb1_retire_session)
                .after(ambition::game_shell::AmbitionGameShellSet::Pending),
        );

        app.add_plugins(Smb1RulesPlugin::hosted());
    }
}

/// Build the real Mary-O session when the shell activates the gameplay route.
#[allow(clippy::too_many_arguments)]
fn smb1_activate_session(
    mut events: MessageReader<ShellEvent>,
    mut active: ResMut<ActiveSessionScope>,
    mut link: ResMut<Smb1SessionLink>,
    mut commands: Commands,
    ldtk_index: Res<LdtkRuntimeIndex>,
    editable_abilities: Res<EditableAbilitySet>,
    editable_tuning: Res<EditableMovementTuning>,
    asset_server: Res<AssetServer>,
) {
    for event in events.read() {
        let ShellEvent::RouteActivated(route) = event else {
            continue;
        };
        if route.experience_id.as_str() != MARY_O_EXPERIENCE {
            continue;
        }
        let scope = active.begin();
        link.bind(route.activation_id, scope);

        let world = smb1_session_world();
        simulation_world(
            &mut commands,
            SimulationSetup {
                world: &world.geometry,
                room_set: &world.room_set,
                ldtk_index: &ldtk_index,
                editable_abilities: &editable_abilities,
                editable_tuning: &editable_tuning,
                starting_character: &world.starting_character,
                sandbox_data_asset: None,
                sandbox_asset_collection: None,
                asset_server: &asset_server,
            },
        );

        commands.insert_resource(world.geometry);
        commands.insert_resource(world.room_set);
        commands.insert_resource(world.metadata);
        commands.insert_resource(world.starting_character);
    }
}

/// Retire the session when the shell deactivates it — one `SessionScopeRetired`;
/// the generic sweep despawns everything the session spawned.
fn smb1_retire_session(
    mut events: MessageReader<ShellEvent>,
    mut link: ResMut<Smb1SessionLink>,
    mut retired: MessageWriter<SessionScopeRetired>,
) {
    for event in events.read() {
        let ShellEvent::RouteDeactivated(route) = event else {
            continue;
        };
        if let Some(scope) = link.unbind(route.activation_id) {
            retired.write(SessionScopeRetired(scope));
        }
    }
}
