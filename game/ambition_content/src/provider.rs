//! Reusable Ambition gameplay provider.

use bevy::prelude::*;

use ambition_platformer2d::presentation::profiles;
use ambition_platformer2d::provider::{AuthoredCatalogFragments, PlatformerExperienceAuthoring};
use ambition_platformer2d_ldtk::LdtkRuntimeIndex;
use ambition_platformer2d::world::rooms::{ActiveRoomMetadata, RoomSet};
use ambition_platformer2d_core::RoomGeometry;
use ambition_platformer2d_runtime::PreparedPlatformerSource;

pub const AMBITION_EXPERIENCE: &str = crate::AMBITION_CONTENT_PROVIDER;
pub const AMBITION_GAMEPLAY_ROUTE: &str = "ambition_gameplay";

#[derive(Resource, Clone)]
pub struct AmbitionPreparedWorld {
    pub room_set: RoomSet,
    pub ldtk_index: LdtkRuntimeIndex,
    /// The experience's catalog DEFAULT character. Meaningful whatever
    /// [`Self::builds_a_home_body`] says: a worn fighter still needs a fallback
    /// id even in a session that lowers no avatar of its own.
    pub starting_character: ambition_platformer2d_actor_monolith::avatar::StartingCharacter,
    /// Whether this composition lowers Ambition's home avatar.
    ///
    /// True for every ordinary entry — the world is an exploration world and the
    /// player has a body in it. A composition that instead SEATS A MATCH into
    /// this world sets it false, because a match owns its whole cast: with a
    /// home avatar present, a local seat and the avatar would both claim the
    /// session's control channel, and `prepare_match` refuses that outright
    /// rather than building two bodies that fight over it.
    ///
    /// a different question from `starting_character`, deliberately — see
    /// `PreparedPlatformerSource::for_match`, which takes both for the same
    /// reason.
    pub builds_a_home_body: bool,
}

impl AmbitionPreparedWorld {
    pub fn prepared_source(&self) -> PreparedPlatformerSource {
        let room_set = self.room_set.clone();
        let geometry = RoomGeometry(room_set.active_world().clone());
        let active_room = ActiveRoomMetadata(room_set.active_spec().metadata.clone());
        if self.builds_a_home_body {
            PreparedPlatformerSource::new(
                AMBITION_EXPERIENCE,
                room_set.clone(),
                geometry,
                active_room,
                self.starting_character.clone(),
            )
            .with_installed_ldtk_index(self.ldtk_index.clone())
        } else {
            PreparedPlatformerSource::for_match(
                AMBITION_EXPERIENCE,
                room_set.clone(),
                geometry,
                active_room,
                self.starting_character.clone(),
            )
            .with_installed_ldtk_index(self.ldtk_index.clone())
        }
    }
}

pub fn ambition_authored_catalogs() -> AuthoredCatalogFragments {
    AuthoredCatalogFragments::new(
        crate::character_catalog::PLAYABLE_ROSTER[0],
        crate::AMBITION_CONTENT_PROVIDER,
    )
    .with_music()
    .with_procedural_sfx()
    .with_adaptive_cues()
    .with_packed_sfx()
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
            ambition_authored_catalogs(),
        )
        // Desktop keeps ordinary framing; touch-primary sessions get
        // occlusion-aware soft framing so the controlled body does not live
        // under a thumb.
        .with_presentation_profiles(profiles::adaptive_platformer())
        .with_defense_presentation(
            ambition_platformer2d::presentation::DefensePresentationPolicy::shared_iframe_blink(),
        )
        .install(app, ambition_session_world);
    }
}

/// The provider's session-world source: matching preparation requests clone
/// the boot-prepared LDtk world published by the app in [`AmbitionPreparedWorld`].
fn ambition_session_world(prepared_world: Res<AmbitionPreparedWorld>) -> PreparedPlatformerSource {
    prepared_world.prepared_source()
}

#[cfg(test)]
mod tests {
    use bevy::prelude::App;

    use ambition_platformer2d::game_shell::{
        MinimalShellPlugins, ShellExperienceId, ShellExperienceRegistry, ShellRouteCatalog,
        ShellRouteId,
    };

    use super::*;

    #[test]
    fn alternate_host_composes_provider_without_ambition_app_initializers() {
        let mut app = App::new();
        app.add_plugins(MinimalShellPlugins);
        app.add_plugins(ambition_platformer2d::load::AmbitionLoadPlugin);
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
