//! Authored provider identity: what an experience declares before any session
//! exists, and the one registration call that installs the shared lifecycle.

use bevy::prelude::*;

use ambition_game_shell::{
    standard_platformer_preparation_plan, ExperienceRegistration, GameplaySessionAppExt,
    ShellCompletionPolicy, ShellRouteId, ShellRouteSpec,
};
use ambition_platformer2d_shared_tangle::gameplay_presentation::{
    ActorGlow, DefensePresentationCatalog, DefensePresentationPolicy,
    GameplayPresentationProfileCatalog, GameplayPresentationProfiles, HudDeclarationCatalog,
    PlayerGlowCatalog,
};

use crate::catalogs::{AuthoredCatalogFragments, PlatformerAuthoredCatalogRegistry};
use crate::lifecycle::{self, PlatformerProviderRuntimePlugin, PlatformerStreamingReadiness};
use crate::world_source::IntoPreparedSource;

/// Everything a provider authors about its experience, plus [`install`] — the
/// single registration seam that wires the experience into the shared
/// preparation/activation lifecycle.
///
/// [`install`]: PlatformerExperienceAuthoring::install
#[derive(Clone, Debug)]
pub struct PlatformerExperienceAuthoring {
    pub experience_id: String,
    pub route_id: String,
    /// Where the launcher sends the player, when that is not the session route.
    /// See [`PlatformerExperienceAuthoring::entered_at`].
    pub entry_route: Option<String>,
    pub label: String,
    pub description: String,
    pub preparation_label: String,
    pub catalogs: AuthoredCatalogFragments,
    pub loading: Option<ambition_load_presentation::LoadExperienceSpec>,
    /// How this experience wants gameplay framed on the physical display.
    /// `None` keeps the engine default (full-bleed, normal framing).
    pub presentation: Option<GameplayPresentationProfiles>,
    /// Which shared defense presentation effects this experience opts into.
    pub defense_presentation: Option<DefensePresentationPolicy>,
    /// The glow on each body a player drives. `None`: no glow.
    pub player_glow: Option<ActorGlow>,
    /// What this experience's HUD reads out.
    pub hud: Option<ambition_platformer2d_shared_tangle::gameplay_presentation::HudDeclaration>,
    /// Whether the launcher offers this experience to a player.
    ///
    /// `true` by default — a provider that goes to the trouble of authoring an
    /// experience usually means it to be playable. See
    /// [`PlatformerExperienceAuthoring::unlisted`].
    pub listed: bool,
    /// The look of its launcher row. See [`Self::with_launcher_look`].
    pub launcher_look: Option<String>,
    /// What this experience puts into its own session while the engine builds
    /// it. See [`Self::with_session_contents`].
    pub session_contents: Option<crate::SessionContentsFn>,
    /// The bag that a session of this experience begins with. `None`: an
    /// empty bag. See [`Self::with_initial_inventory`].
    pub initial_inventory: Option<ambition_platformer2d_items::starting_bag::InitialInventoryFn>,
}

impl PlatformerExperienceAuthoring {
    pub fn new(
        experience_id: impl Into<String>,
        route_id: impl Into<String>,
        label: impl Into<String>,
        description: impl Into<String>,
        preparation_label: impl Into<String>,
        catalogs: AuthoredCatalogFragments,
    ) -> Self {
        Self {
            experience_id: experience_id.into(),
            route_id: route_id.into(),
            entry_route: None,
            label: label.into(),
            description: description.into(),
            preparation_label: preparation_label.into(),
            catalogs,
            loading: None,
            presentation: None,
            defense_presentation: None,
            player_glow: None,
            hud: None,
            listed: true,
            launcher_look: None,
            session_contents: None,
            initial_inventory: None,
        }
    }

    /// Compose and route this experience, but keep it out of the launcher.
    ///
    /// for a stage that exists to be TESTED or DEVELOPED against, not
    /// chosen: a fixture, a scratch arena, a crossover that only one composition
    /// can host. Everything else is unchanged — the route is registered, the
    /// characters join the roster, the catalogs are installed, and a test that
    /// activates the route by id works exactly as before.
    ///
    /// not the same as declaring it unavailable. An unavailable
    /// experience is SHOWN, greyed, with a reason, because the player is meant
    /// to know it exists. This one is simply not offered.
    pub fn unlisted(mut self) -> Self {
        self.listed = false;
        self
    }

    /// Name the look of this experience's launcher row: a semantic key, such
    /// as a backdrop theme's name (`"starlight"`). The host's launcher
    /// presentation resolves it; a key it does not know, or none, gives its
    /// default look. So a provider with a new route name needs no edit of
    /// the host to have its own look.
    pub fn with_launcher_look(mut self, look: impl Into<String>) -> Self {
        self.launcher_look = Some(look.into());
        self
    }

    /// The launcher opens this route; the session still lives on the
    /// gameplay one.
    ///
    /// For an experience that asks a question before it starts — a character
    /// select, a stage select — and therefore cannot be entered by activating
    /// its stage. The route must be one the provider has ALREADY registered
    /// (a frontend screen under an experience id of its own, not a gameplay
    /// session), because a launcher row pointing at an unregistered route is a
    /// dead row and the shell refuses it at build time.
    pub fn entered_at(mut self, route: impl Into<String>) -> Self {
        self.entry_route = Some(route.into());
        self
    }

    /// Declare gameplay presentation with one tested preset, e.g.
    /// `profiles::fixed_four_by_three()`.
    ///
    /// Optional on purpose: a provider that says nothing gets full-bleed
    /// normal framing, which is what every game got before this existed.
    pub fn with_presentation_profiles(mut self, profiles: GameplayPresentationProfiles) -> Self {
        self.presentation = Some(profiles);
        self
    }

    /// Declare which semantic iframe/defense causes opt into the engine's
    /// shared presentation effects for this experience. Character-owned effects
    /// remain independent and therefore compose with these cues.
    pub fn with_defense_presentation(mut self, policy: DefensePresentationPolicy) -> Self {
        self.defense_presentation = Some(policy);
        self
    }

    /// Declare the glow on each body a player drives in this experience
    /// (for example [`ActorGlow::player_highlight`]). `None` draws no glow,
    /// which is also what a provider that says nothing gets.
    pub fn with_player_glow(mut self, glow: Option<ActorGlow>) -> Self {
        self.player_glow = glow;
        self
    }

    /// Declare this experience's HUD readouts.
    ///
    /// The declaration says which slots exist, in what order, preferring which
    /// surround region — never what they mean. The live values arrive each
    /// frame through
    /// [`HudReadouts`](ambition_platformer2d_shared_tangle::gameplay_presentation::HudReadouts),
    /// written by a system the GAME owns, so the engine holds no content
    /// vocabulary and a second game needs no core edit to get a HUD.
    pub fn with_hud(
        mut self,
        hud: ambition_platformer2d_shared_tangle::gameplay_presentation::HudDeclaration,
    ) -> Self {
        self.hud = Some(hud);
        self
    }

    /// Declare what this experience puts into its own session: components on
    /// the session root, capabilities on the home body, and the entities the
    /// session owns.
    ///
    /// The engine runs `contents` inside the transaction that builds the
    /// session, so the session is published whole. Use it for facts that are
    /// the experience's and not a room's: a room occupant is a room placement.
    /// See [`crate::session_contents`].
    pub fn with_session_contents(mut self, contents: crate::SessionContentsFn) -> Self {
        self.session_contents = Some(contents);
        self
    }

    /// Declare the bag that each session of this experience begins with.
    ///
    /// The starting bag is an authored first condition of the experience. The
    /// candidate session carries it and its adoption installs it; the save of
    /// the session replaces it when the save holds an inventory. An
    /// experience that declares none begins with an empty bag, whatever bag
    /// the composition or the session before it has.
    pub fn with_initial_inventory(
        mut self,
        bag: ambition_platformer2d_items::starting_bag::InitialInventoryFn,
    ) -> Self {
        self.initial_inventory = Some(bag);
        self
    }

    pub fn with_loading_activity(mut self, activity_id: impl Into<String>) -> Self {
        let mut loading = ambition_load_presentation::LoadExperienceSpec::basic(format!(
            "{}.loading",
            self.experience_id
        ));
        loading.activity = Some(ambition_load_presentation::LoadActivityId::new(activity_id));
        loading.ready_policy = ambition_load_presentation::ReadyTransitionPolicy::AutoUnlessEngaged;
        self.loading = Some(loading);
        self
    }

    pub fn with_loading_spec(
        mut self,
        loading: ambition_load_presentation::LoadExperienceSpec,
    ) -> Self {
        self.loading = Some(loading);
        self
    }

    /// Register the experience AND its session lifecycle in one call.
    ///
    /// `source` is the provider's whole remaining obligation: a system that
    /// builds the authored [`PreparedPlatformerSource`](ambition_platformer2d_runtime::PreparedPlatformerSource) this experience plays in
    /// (it may read the provider's own resources), or a [`WorldRefusal`]
    /// ([`IntoPreparedSource`]). The shared lifecycle runs it once on an
    /// update containing matching preparation requests, gives each transaction
    /// an owned copy, validates the authored catalogs, publishes the typed
    /// prepared-session identity, and constructs the live session on
    /// activation. A refusal fails each transaction's world preparation with
    /// its reasons, and a retry runs the source again.
    pub fn install<S, Out, Marker>(self, app: &mut App, source: S)
    where
        S: IntoSystem<(), Out, Marker>,
        Out: IntoPreparedSource,
    {
        self.register(app);
        let experience_id = self.experience_id.clone();
        let tag = move |In(world): In<Out>| (experience_id.clone(), world.into_prepared_source());
        app.add_systems(
            Update,
            source
                .pipe(tag)
                .pipe(crate::lifecycle::prepare_requested_sessions)
                .run_if(crate::lifecycle::preparation_requested(self.experience_id))
                .in_set(lifecycle::PlatformerPreparationSet),
        );
    }

    /// Authoring-only registration: experience, route, authored catalogs, and
    /// loading presentation. [`install`](Self::install) is the public seam;
    /// this stays separate so registration remains readable on its own.
    fn register(&self, app: &mut App) {
        // Provider registration is the authoritative composition seam. Install
        // both preparation resources synchronously here before any provider
        // systems can be initialized. The runtime plugin also uses `init`, but
        // relying on a nested plugin build to publish the private streaming
        // resource left thin standalone hosts vulnerable to first-update
        // SystemParam validation failures.
        app.init_resource::<PlatformerAuthoredCatalogRegistry>()
            .init_resource::<PlatformerStreamingReadiness>()
            .init_resource::<crate::SessionContentsCatalog>();
        if !app.is_plugin_added::<PlatformerProviderRuntimePlugin>() {
            app.add_plugins(PlatformerProviderRuntimePlugin);
        }
        app.world_mut()
            .resource_mut::<PlatformerAuthoredCatalogRegistry>()
            .try_register(self.experience_id.as_str(), self.catalogs.clone())
            .unwrap_or_else(|error| panic!("{error}"));
        let registration = ExperienceRegistration::new(
            self.experience_id.clone(),
            self.label.clone(),
            self.route_id.clone(),
        )
        .with_description(self.description.clone());
        let registration = match self.entry_route.as_deref() {
            Some(entry) => registration.entered_at(entry),
            None => registration,
        };
        let registration = if self.listed {
            registration
        } else {
            registration.unlisted()
        };
        let registration = match self.launcher_look.as_deref() {
            Some(look) => registration.with_launcher_look(look),
            None => registration,
        };
        app.register_gameplay_experience(
            registration,
            ShellRouteSpec::new(self.route_id.clone(), self.experience_id.clone())
                .preparing_with(standard_platformer_preparation_plan(
                    self.preparation_label.clone(),
                ))
                .on_complete(ShellCompletionPolicy::ReturnHome),
        );
        if let Some(presentation) = self.presentation {
            app.init_resource::<GameplayPresentationProfileCatalog>();
            app.world_mut()
                .resource_mut::<GameplayPresentationProfileCatalog>()
                .insert(self.route_id.clone(), presentation);
        }
        if let Some(policy) = self.defense_presentation {
            app.init_resource::<DefensePresentationCatalog>();
            app.world_mut()
                .resource_mut::<DefensePresentationCatalog>()
                .insert(self.route_id.clone(), policy);
        }
        if let Some(glow) = self.player_glow {
            app.init_resource::<PlayerGlowCatalog>();
            app.world_mut().resource_mut::<PlayerGlowCatalog>().insert(self.route_id.clone(), glow);
        }
        if let Some(contents) = self.session_contents {
            app.world_mut()
                .resource_mut::<crate::SessionContentsCatalog>()
                .declare(&self.experience_id, contents);
        }
        if let Some(bag) = self.initial_inventory {
            app.init_resource::<ambition_platformer2d_items::starting_bag::InitialInventories>();
            app.world_mut()
                .resource_mut::<ambition_platformer2d_items::starting_bag::InitialInventories>()
                .declare(&self.experience_id, bag);
        }
        if let Some(hud) = self.hud.clone() {
            app.init_resource::<HudDeclarationCatalog>();
            app.world_mut()
                .resource_mut::<HudDeclarationCatalog>()
                .insert(self.route_id.clone(), hud);
        }
        if let Some(loading) = self.loading.clone() {
            app.init_resource::<ambition_load_presentation::ShellLoadPresentationCatalog>();
            app.world_mut()
                .resource_mut::<ambition_load_presentation::ShellLoadPresentationCatalog>()
                .by_route
                .insert(ShellRouteId::new(self.route_id.clone()), loading);
        }
    }
}
