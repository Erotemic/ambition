//! Reusable Ambition gameplay provider.

use bevy::prelude::*;

use ambition_platformer2d::presentation::profiles;
use ambition_platformer2d::provider::{AuthoredCatalogFragments, PlatformerExperienceAuthoring};
use ambition_platformer2d_ldtk::LdtkRuntimeIndex;
use ambition_platformer2d::world::rooms::RoomSet;
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
        let geometry = RoomGeometry(room_set.activation_spec().world.clone());
        if self.builds_a_home_body {
            PreparedPlatformerSource::new(
                AMBITION_EXPERIENCE,
                room_set.clone(),
                geometry,
                self.starting_character.clone(),
            )
            // Ambition's player holds Mana: the pool its held abilities spend.
            // Declared here, on the home body, so no other body has it.
            .with_home_body_resources(
                ambition_platformer2d_runtime::demo_fixture::HomeBodyResources::declared(&[
                    ambition_platformer2d::abilities::mana::POOL,
                ])
                .expect("one declared pool is a valid layout"),
            )
            // Morph Ball is this game's progression, granted to its home body;
            // the character seated anywhere else does not curl.
            .with_home_body_abilities(
                ambition_platformer2d_runtime::demo_fixture::HomeBodyAbilities::granting(
                    ambition_platformer2d::engine_core::AbilityGrant::MorphBall.to_set(),
                ),
            )
            .with_installed_ldtk_index(self.ldtk_index.clone())
        } else {
            PreparedPlatformerSource::for_match(
                AMBITION_EXPERIENCE,
                room_set.clone(),
                geometry,
                self.starting_character.clone(),
            )
            .with_installed_ldtk_index(self.ldtk_index.clone())
        }
    }
}

pub fn ambition_authored_catalogs() -> AuthoredCatalogFragments {
    AuthoredCatalogFragments::new(
        crate::character_catalog::DEFAULT_CHARACTER,
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
        // The first bag of an Ambition session: the starter set. Its save
        // replaces it when the save holds an inventory.
        .with_initial_inventory(ambition_items::OwnedItems::starter)
        .install(app, ambition_session_world);
        // The refill rate for the Mana this experience declares. Inert for every
        // body that holds no Mana, so composing this beside other experiences
        // changes nothing of theirs.
        app.insert_resource(ambition_platformer2d::actors::avatar::systems::PlayerManaRegen(
            ambition_platformer2d::abilities::mana::REGEN_PER_SEC,
        ));
        declare_ambition_seating(app, &self.config.route_id);
    }
}

/// The most local seats an Ambition session offers.
pub const AMBITION_SEATS: u8 = 2;

/// Ambition offers one local seat for each connected pad, at most
/// [`AMBITION_SEATS`] (Q153 default, until the maintainer rules how a second
/// player joins).
///
/// The policy is the default `UnifiedPrimary`: with one pad, the keyboard and
/// the pad both drive the primary seat, as before. With two pads, the second
/// pad drives seat 1, and the session opens a handle for it. The rollback
/// session is never resized, so a pad that connects after the gameplay session
/// started gets a seat only in the next session. No channel plan is declared:
/// a fixed plan of keyboard and first pad would take the pad away from a
/// player who plays alone on it.
pub fn declare_ambition_seating(app: &mut App, route: &str) {
    use ambition_platformer2d::game_shell::{RouteSeating, RouteSeatingAppExt, SeatCount};
    app.declare_route_seating(
        route.to_owned(),
        RouteSeating::new(
            SeatCount::OnePerSource { max: AMBITION_SEATS },
            ambition_platformer2d::input::InputAssignmentPolicy::UnifiedPrimary,
        ),
    );
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

    /// Q153 default: one seat for each pad, at most two, and the keyboard
    /// stays with the primary. Zero and one pad offer one seat (the solo
    /// session of before); two pads offer two; a third pad offers no third.
    /// No channel plan is decided, so the session is sized from devices.
    #[test]
    fn ambition_offers_a_seat_for_each_pad_up_to_two() {
        use ambition_platformer2d::game_shell::{
            project_route_seating, ActiveShellExperience, ShellActivationId, ShellRouter,
        };
        use ambition_platformer2d::input::{
            InputAssignmentPolicy, LocalDeviceOrder, LocalSeatOffer, SessionSeatingSource,
        };
        let offered = |pads: usize| {
            let mut app = App::new();
            app.init_resource::<ShellRouter>()
                .add_systems(bevy::prelude::Update, project_route_seating);
            declare_ambition_seating(&mut app, AMBITION_GAMEPLAY_ROUTE);
            let devices = (0..pads).map(|_| app.world_mut().spawn_empty().id()).collect();
            app.insert_resource(LocalDeviceOrder::from_devices(devices));
            app.world_mut().resource_mut::<ShellRouter>().active = Some(ActiveShellExperience {
                activation_id: ShellActivationId(1),
                route_id: ShellRouteId::new(AMBITION_GAMEPLAY_ROUTE),
                experience_id: ShellExperienceId::new(AMBITION_EXPERIENCE),
                parameters: Default::default(),
                load_authorization: None,
                prepared_session: None,
            });
            app.update();
            let offer = app.world().resource::<LocalSeatOffer>().clone();
            assert_eq!(offer.policy(), InputAssignmentPolicy::UnifiedPrimary);
            assert_eq!(
                app.world().resource::<SessionSeatingSource>().channel_plan(),
                None,
                "a decided plan would size the session instead of the devices"
            );
            offer.seats()
        };
        assert_eq!([0, 1, 2, 3].map(offered), [1, 1, 2, 2], "[seats with 0, 1, 2, 3 pads]");
    }
}
