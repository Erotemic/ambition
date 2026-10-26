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
    pub starting_character: ambition_platformer2d_session::initial_body::StartingCharacter,
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

pub fn ambition_authored_catalogs(pack: &ambition_content_pack::PreparedContentPack) -> AuthoredCatalogFragments {
    AuthoredCatalogFragments::new(
        crate::character_catalog::starting_character(pack),
        crate::AMBITION_CONTENT_PROVIDER,
    )
    .with_music()
    .with_procedural_sfx()
    .with_adaptive_cues()
    .with_packed_sfx()
}

/// The launcher row's words are the pack's (`data/ambition_text.ron`).
#[derive(Clone, Debug)]
pub struct AmbitionExperienceConfig {
    pub route_id: String,
}

impl Default for AmbitionExperienceConfig {
    fn default() -> Self {
        Self {
            route_id: AMBITION_GAMEPLAY_ROUTE.to_owned(),
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
        // Ambition's game is installed by its provider, as Mary-O's is by
        // hers: a hub that selects Ambition names this plugin and no other
        // Ambition plugin. Its world first: the content's reload reads it.
        crate::world_preparation::prepare_ambition_world(app);
        install_ambition_game(app);
        let text = ambition_content_schemas::text::lowered_ambition_text(&crate::pack::select(app.world_mut()))
            .expect("Ambition's pack states its launcher row (`data/ambition_text.ron`)");
        PlatformerExperienceAuthoring::new(
            AMBITION_EXPERIENCE,
            self.config.route_id.clone(),
            text.line("launcher.label"),
            text.line("launcher.description"),
            text.line("launcher.preparing"),
            ambition_authored_catalogs(&crate::pack::select(app.world_mut())),
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
        // Ambition's cast authors fighter specials (a counter, a placed
        // spring, a portal pair, ...): install them here, so a hub without the
        // Smash demo admits its moves. Smash installs the same ones; once.
        ambition_fighter_specials::install(app);
        // The refill rate for the Mana this experience declares. Inert for every
        // body that holds no Mana, so composing this beside other experiences
        // changes nothing of theirs.
        app.insert_resource(ambition_platformer2d::actors::avatar::systems::PlayerManaRegen(
            ambition_platformer2d::abilities::mana::REGEN_PER_SEC,
        ));
        declare_ambition_seating(app, &self.config.route_id);
        // Two players in one room share a view until one leaves what it shows
        // (A2). The values are the default feel, until the maintainer rules.
        app.init_resource::<ambition_sim_view::AdaptiveSplit>();
    }
}

/// Install Ambition's content, its dialogue, its rollback rows and, where the
/// App presents, its looks and item art.
///
/// The provider builds after the engine plugins, so each install here finds
/// the backend and the presentation layer already composed. A caller that
/// installed one of these plugins first keeps it.
fn install_ambition_game(app: &mut App) {
    if !app.is_plugin_added::<crate::AmbitionContentPlugin>() {
        app.add_plugins(crate::AmbitionContentPlugin);
    }
    // Ambition's rooms are painted in LDtk. The painting needs a render app
    // (`bevy_ecs_tilemap` panics without one); with none, the rooms still
    // draw their own sprites.
    #[cfg(feature = "ldtk_runtime")]
    if app.get_sub_app(bevy::render::RenderApp).is_some()
        && !app.is_plugin_added::<ambition_platformer2d_ldtk::LdtkRuntimePlugin>()
    {
        use bevy_asset_loader::asset_collection::AssetCollectionApp;
        app.add_plugins(ambition_platformer2d_ldtk::LdtkRuntimePlugin);
        // After `LdtkPlugin`: the collection holds a typed LDtk handle, so the
        // LDtk asset type must be registered before the handles are made.
        app.init_collection::<ambition_platformer2d_assets::loading::Platformer2dStartupAssets>();
    }
    // Yarn compiles its project through the asset server; an App with no
    // asset server (a provider declaration test) gets no dialogue runtime.
    #[cfg(feature = "ui")]
    if !app.is_plugin_added::<bevy_yarnspinner::prelude::YarnSpinnerPlugin>()
        && app.world().contains_resource::<bevy::asset::AssetServer>()
    {
        use ambition_platformer2d::platformer::capability::Capability;
        use bevy::ecs::schedule::common_conditions::{not, resource_exists};
        app.add_plugins(crate::dialogue::yarn_spinner_plugin());
        // Yarn's work each frame (its line fetch, its runtime, its waits)
        // sleeps where the active room's game uses no dialogue: in the hub
        // with Mary-O, its fetch was ~0.16 ms a frame for no line. It runs
        // until its project is compiled, so a session that starts in such a
        // room still has the project when it reaches a room that talks.
        app.configure_sets(
            Update,
            bevy_yarnspinner::prelude::YarnSpinnerSystemSet.run_if(
                not(resource_exists::<bevy_yarnspinner::prelude::YarnProject>)
                    .or_else(ambition_platformer2d::runtime::room_uses(Capability::Dialogue)),
            ),
        );
    }
    // The rows of Ambition's own state, through the backend the host
    // selected. A host with no rollback backend has no registrar to take them.
    if app
        .world()
        .get_resource::<ambition_platformer2d::runtime::SimulationHost>()
        .copied()
        .is_some_and(|host| host.is_rollback())
    {
        let mut registrar = ambition_platformer2d::rollback::GgrsRollbackRegistrar::new(app);
        crate::register_rollback_state(&mut registrar);
    }
    // Ambition's looks compose onto the renderer's presentation seams, so they
    // install where that presentation is installed, and only there.
    if app.is_plugin_added::<ambition_render::rendering::PresentationVisualAnimationPlugin>() {
        if !app.is_plugin_added::<crate::presentation::AmbitionPresentationPlugin>() {
            app.add_plugins(crate::presentation::AmbitionPresentationPlugin);
        }
        if !app.is_plugin_added::<crate::items::AmbitionItemRosterPlugin>() {
            app.add_plugins(crate::items::AmbitionItemRosterPlugin);
        }
        // Breakables and defeated enemies throw Avian debris. It is a look
        // (ADR 0007: the bodies stay kinematic), so a composition with no
        // presentation does not pay for the physics engine.
        #[cfg(feature = "physics_debris")]
        install_physics_debris(app);
    }
}

#[cfg(feature = "physics_debris")]
fn install_physics_debris(app: &mut App) {
    use ambition_platformer2d::world::physics;
    if app.is_plugin_added::<physics::AmbitionPhysicsPlugin>() {
        return;
    }
    app.add_plugins(physics::AmbitionPhysicsPlugin).add_systems(
        Update,
        physics::physics_spawn_debris_messages
            .run_if(ambition_platformer2d_core::a_message_waits::<ambition_platformer2d::vfx::vfx::DebrisBurstMessage>)
            .after(ambition_platformer2d::platformer::schedule::Platformer2dSimulationPhase::CoreSimulation)
            .run_if(ambition_platformer2d::platformer::lifecycle::session_world_exists),
    );
}

/// The most local seats an Ambition session offers.
pub const AMBITION_SEATS: u8 = 2;

/// The seating Ambition's route states (Q153, ruled 2026-10-19).
///
/// Single-player (the default): one channel on the shared source, so the
/// keyboard and every pad drive player one, also a pad connected late, and no
/// press seats anybody else. Accepting joins: the shared channel and
/// [`AMBITION_SEATS`]` - 1` claimable channels. A claimable channel is reserved
/// capacity: the session holds its handle from the start, idle until a pad
/// claims it with Start, so a join needs no rebase. A claim that still stands
/// keeps its channel when the setting goes off, so a joined player stays until
/// they leave; the setting only says whether another join is accepted.
pub fn ambition_route_seating(
    accept_joins: bool,
    claimed: bool,
) -> ambition_platformer2d::game_shell::RouteSeating {
    use ambition_platformer2d::game_shell::{RouteSeating, SeatCount};
    use ambition_platformer2d::input::{InputAssignmentPolicy, LocalChannelPlan, LocalInputSource};
    let seats = if accept_joins || claimed { AMBITION_SEATS } else { 1 };
    let sources = std::iter::once(LocalInputSource::Shared)
        .chain((1..seats).map(|_| LocalInputSource::Claimed));
    let policy = if seats > 1 {
        InputAssignmentPolicy::JoinToClaim
    } else {
        InputAssignmentPolicy::UnifiedPrimary
    };
    RouteSeating::new(SeatCount::Fixed(seats), policy)
        .with_channels(LocalChannelPlan::from_sources(sources))
}

/// Declare Ambition's route seating (single-player until the setting says
/// otherwise) and keep it in step with the setting and the claims.
pub fn declare_ambition_seating(app: &mut App, route: &str) {
    use ambition_platformer2d::game_shell::RouteSeatingAppExt;
    use bevy::prelude::IntoScheduleConfigs as _;
    app.declare_route_seating(route.to_owned(), ambition_route_seating(false, false));
    app.insert_resource(AmbitionSeatingRoute(
        ambition_platformer2d::game_shell::ShellRouteId::new(route),
    ));
    app.add_systems(
        bevy::prelude::Update,
        restate_ambition_seating
            .run_if(seating_inputs_changed)
            .before(ambition_platformer2d::game_shell::project_route_seating)
            .in_set(ambition_platformer2d::input::SeatingDeclared),
    );
}

/// The route whose seating [`restate_ambition_seating`] keeps.
#[derive(bevy::prelude::Resource, Clone, Debug)]
struct AmbitionSeatingRoute(ambition_platformer2d::game_shell::ShellRouteId);

/// Run condition of [`restate_ambition_seating`]: the join setting, the local
/// claims or the catalog changed (or came to exist). Otherwise the restated
/// entry already matches.
fn seating_inputs_changed(
    settings: Option<bevy::prelude::Res<ambition_persistence::settings::UserSettings>>,
    claims: Option<bevy::prelude::Res<ambition_platformer2d::input::LocalSeatClaims>>,
    catalog: Option<bevy::prelude::Res<ambition_platformer2d::game_shell::RouteSeatingCatalog>>,
) -> bool {
    settings.is_some_and(|settings| settings.is_changed())
        || claims.is_some_and(|claims| claims.is_changed())
        || catalog.is_some_and(|catalog| catalog.is_changed())
}

fn restate_ambition_seating(
    route: Res<AmbitionSeatingRoute>,
    settings: Option<Res<ambition_persistence::settings::UserSettings>>,
    claims: Option<Res<ambition_platformer2d::input::LocalSeatClaims>>,
    catalog: Option<ResMut<ambition_platformer2d::game_shell::RouteSeatingCatalog>>,
) {
    let Some(mut catalog) = catalog else {
        return;
    };
    let accept = settings.is_some_and(|settings| settings.gameplay.accept_local_joins);
    let claimed = claims.is_some_and(|claims| !claims.is_empty());
    let wanted = ambition_route_seating(accept, claimed);
    // Read first: a `ResMut` deref marks the catalog changed every frame.
    if catalog.get(&route.0) != Some(&wanted) {
        catalog.restate(&route.0, wanted);
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
        // The launcher row's words are the pack's text file's.
        let text = ambition_content_schemas::text::lowered_ambition_text(crate::pack::shipped())
            .expect("Ambition's pack states its launcher row");
        assert_eq!(registration.display_name, text.line("launcher.label"));
        let route = app
            .world()
            .resource::<ShellRouteCatalog>()
            .get(&ShellRouteId::new(AMBITION_GAMEPLAY_ROUTE))
            .expect("provider registered its route");
        assert!(route.preparation.is_some());
    }

    /// Ambition's cast authors fighter specials (a counter, a placed spring,
    /// a portal pair, ...), and its provider installs them, so a hub that
    /// does not install the Smash demo still admits those moves. Every such
    /// key its shipped movesets author is declared by the provider alone.
    #[test]
    fn the_provider_installs_the_fighter_specials_its_cast_authors() {
        let mut app = App::new();
        app.add_plugins(ambition_platformer2d::extension::ExtensionHostPlugin::new(bevy::prelude::Update));
        app.add_plugins(MinimalShellPlugins);
        app.add_plugins(ambition_platformer2d::load::AmbitionLoadPlugin);
        app.add_plugins(crate::AmbitionContentPlugin);
        app.add_plugins(AmbitionExperiencePlugin::new(AmbitionExperienceConfig::default()));

        let offered: Vec<&str> = ambition_fighter_specials::specials().iter().map(|special| special.key).collect();
        let mut authored: Vec<String> = crate::authored_movesets::tables()
            .iter()
            .flat_map(|(_, contract)| contract.moves.iter())
            .flat_map(|spec| spec.effect_refs().into_iter().map(|(_, effect)| effect.key.clone()))
            .filter(|key| offered.contains(&key.as_str()))
            .collect();
        authored.sort();
        authored.dedup();
        assert!(authored.len() >= 10, "Ambition's cast authors only {authored:?}, so this is about nobody");
        let support = &app
            .world()
            .get_resource::<ambition_platformer2d::combat::technique::InstalledTechniques>()
            .expect("the provider installed no technique at all")
            .0;
        let missing: Vec<&String> = authored.iter().filter(|key| support.offer(key).is_none()).collect();
        assert!(missing.is_empty(), "Ambition's cast authors {missing:?}, which only Smash would install");
    }

    /// Q153 (2026-10-19): single-player by default, whatever the pads. The
    /// route decides a one-channel plan on the shared source, so the session
    /// is not sized from devices: with 0..3 pads it offers one seat. With the
    /// setting on, it offers two, the second claimable. A standing claim keeps
    /// the second channel after the setting goes off. Control: the old
    /// device-sized offer gave two seats for two pads.
    #[test]
    fn ambition_is_single_player_until_the_setting_accepts_a_join() {
        use ambition_platformer2d::game_shell::{
            project_route_seating, ActiveShellExperience, ShellActivationId, ShellRouter,
        };
        use ambition_platformer2d::input::{
            LocalDeviceOrder, LocalInputSource, LocalSeatClaims, LocalSeatOffer, ParticipantId,
            SessionSeatingSource,
        };
        let seated = |pads: usize, accept: bool, claimed: bool| {
            let mut app = App::new();
            app.init_resource::<ShellRouter>();
            let mut settings = ambition_persistence::settings::UserSettings::default();
            settings.gameplay.accept_local_joins = accept;
            app.insert_resource(settings);
            declare_ambition_seating(&mut app, AMBITION_GAMEPLAY_ROUTE);
            app.add_systems(bevy::prelude::Update, project_route_seating);
            let devices: Vec<Entity> = (0..pads).map(|_| app.world_mut().spawn_empty().id()).collect();
            let order = LocalDeviceOrder::from_devices(devices.clone());
            let mut claims = LocalSeatClaims::default();
            if claimed {
                claims.claim(ParticipantId::SECONDARY, order.mark(devices[1]).expect("pad 1 is connected"));
            }
            app.insert_resource(claims);
            app.insert_resource(order);
            app.world_mut().resource_mut::<ShellRouter>().active = Some(ActiveShellExperience {
                activation_id: ShellActivationId(1),
                route_id: ShellRouteId::new(AMBITION_GAMEPLAY_ROUTE),
                experience_id: ShellExperienceId::new(AMBITION_EXPERIENCE),
                parameters: Default::default(),
                load_authorization: None,
                prepared_session: None,
            });
            app.update();
            let offer = app.world().resource::<LocalSeatOffer>().seats();
            let plan = app
                .world()
                .resource::<SessionSeatingSource>()
                .channel_plan()
                .expect("Ambition decides its plan, so devices do not size the session")
                .sources()
                .to_vec();
            (offer, plan)
        };
        let solo = (1, vec![LocalInputSource::Shared]);
        let couch = (2, vec![LocalInputSource::Shared, LocalInputSource::Claimed]);
        for pads in 0..4 {
            assert_eq!(seated(pads, false, false), solo, "{pads} pad(s), setting off");
        }
        assert_eq!(seated(2, true, false), couch, "setting on: a reserved claimable seat");
        assert_eq!(seated(2, false, true), couch, "setting off with a standing claim");
    }
}
