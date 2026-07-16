//! Reusable Ambition gameplay provider.

use bevy::prelude::*;

use ambition::provider::{AuthoredCatalogFragments, PlatformerExperienceAuthoring};
use ambition_actors::ldtk_world::LdtkRuntimeIndex;
use ambition_actors::rooms::{ActiveRoomMetadata, RoomSet};
use ambition_engine_core::RoomGeometry;
use ambition_runtime::PlatformerSessionWorld;

pub const AMBITION_EXPERIENCE: &str = crate::AMBITION_CONTENT_PROVIDER;
pub const AMBITION_GAMEPLAY_ROUTE: &str = "ambition_gameplay";

#[derive(Resource, Clone)]
pub struct AmbitionPreparedWorld {
    pub room_set: RoomSet,
    pub ldtk_index: LdtkRuntimeIndex,
    pub starting_character: ambition_actors::avatar::StartingCharacter,
}

#[derive(Clone, Debug)]
pub struct AmbitionExperienceConfig {
    pub route_id: String,
    pub label: String,
    pub description: String,
}

impl Default for AmbitionExperienceConfig {
    fn default() -> Self {
        Self {
            route_id: AMBITION_GAMEPLAY_ROUTE.to_owned(),
            label: "Ambition".to_owned(),
            description: "The main Ambition campaign".to_owned(),
        }
    }
}

pub struct AmbitionExperiencePlugin {
    config: AmbitionExperienceConfig,
}

impl Default for AmbitionExperiencePlugin {
    fn default() -> Self {
        Self::new(AmbitionExperienceConfig::default())
    }
}

impl AmbitionExperiencePlugin {
    pub fn new(config: AmbitionExperienceConfig) -> Self {
        Self { config }
    }
}

impl Plugin for AmbitionExperiencePlugin {
    fn build(&self, app: &mut App) {
        PlatformerExperienceAuthoring::new(
            AMBITION_EXPERIENCE,
            self.config.route_id.clone(),
            self.config.label.clone(),
            self.config.description.clone(),
            "Prepare Ambition",
            AuthoredCatalogFragments::new(
                crate::character_catalog::PLAYABLE_ROSTER[0],
                crate::AMBITION_CONTENT_PROVIDER,
            )
            .with_music()
            .with_procedural_sfx()
            .with_adaptive_cues()
            .with_packed_sfx(),
        )
        .install(app, ambition_session_world);
    }
}

/// The provider's session-world source: every shell activation clones the
/// boot-prepared LDtk world published by the app in [`AmbitionPreparedWorld`].
fn ambition_session_world(prepared_world: Res<AmbitionPreparedWorld>) -> PlatformerSessionWorld {
    let room_set = prepared_world.room_set.clone();
    PlatformerSessionWorld::new(
        AMBITION_EXPERIENCE,
        room_set.clone(),
        RoomGeometry(room_set.active_world().clone()),
        ActiveRoomMetadata(room_set.active_spec().metadata.clone()),
        prepared_world.starting_character.clone(),
        prepared_world.ldtk_index.clone(),
    )
}

#[cfg(test)]
mod tests {
    use bevy::prelude::App;

    use ambition::game_shell::{
        MinimalShellPlugins, ShellExperienceId, ShellExperienceRegistry, ShellRouteCatalog,
        ShellRouteId,
    };

    use super::*;

    #[test]
    fn alternate_host_composes_provider_without_ambition_app_initializers() {
        let mut app = App::new();
        app.add_plugins(MinimalShellPlugins);
        app.add_plugins(ambition::load::AmbitionLoadPlugin);
        app.add_plugins(crate::AmbitionContentPlugin);
        app.add_plugins(AmbitionExperiencePlugin::new(
            AmbitionExperienceConfig::default(),
        ));

        let experience_id = ShellExperienceId::new(AMBITION_EXPERIENCE);
        let registration = app
            .world()
            .resource::<ShellExperienceRegistry>()
            .get(&experience_id)
            .expect("provider registered itself in an alternate host");
        assert_eq!(registration.launch_route.as_str(), AMBITION_GAMEPLAY_ROUTE);
        let route = app
            .world()
            .resource::<ShellRouteCatalog>()
            .get(&ShellRouteId::new(AMBITION_GAMEPLAY_ROUTE))
            .expect("provider registered its route");
        assert!(route.preparation.is_some());
    }
}
