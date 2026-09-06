//! Authored provider identity: what an experience declares before any session
//! exists, and the one registration call that installs the shared lifecycle.

use std::collections::BTreeMap;

use bevy::prelude::*;

use ambition_audio::catalog::AudioCatalogRegistry;
use ambition_characters::actor::character_catalog::CharacterCatalog;
use ambition_game_shell::{
    standard_platformer_preparation_plan, ExperienceRegistration, GameplaySessionAppExt,
    ShellCompletionPolicy, ShellRouteId, ShellRouteSpec, PREPARE_AUDIO_WORK_ID,
    PREPARE_CATALOGS_WORK_ID,
};
use ambition_platformer2d_shared_tangle::gameplay_presentation::{
    ActiveDefensePresentationPolicy, ActiveGameplayPresentationProfiles, ActiveHudDeclaration,
    DefensePresentationCatalog, DefensePresentationPolicy, GameplayPresentationProfileCatalog,
    GameplayPresentationProfiles, HudDeclaration, HudDeclarationCatalog,
};
use ambition_platformer2d_runtime::PreparedPlatformerSource;

use crate::lifecycle::{self, PlatformerProviderRuntimePlugin, PlatformerStreamingReadiness};

/// The catalog identity a provider authors: its starting character, its audio
/// provider id, and which audio fragments a prepared session must find.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthoredCatalogFragments {
    pub starting_character: String,
    pub audio_provider: String,
    pub expects_music: bool,
    pub expects_procedural_sfx: bool,
    pub expects_adaptive_cues: bool,
    pub expects_packed_sfx: bool,
}

impl AuthoredCatalogFragments {
    pub fn new(starting_character: impl Into<String>, audio_provider: impl Into<String>) -> Self {
        Self {
            starting_character: starting_character.into(),
            audio_provider: audio_provider.into(),
            expects_music: false,
            expects_procedural_sfx: false,
            expects_adaptive_cues: false,
            expects_packed_sfx: false,
        }
    }

    pub fn with_music(mut self) -> Self {
        self.expects_music = true;
        self
    }

    pub fn with_procedural_sfx(mut self) -> Self {
        self.expects_procedural_sfx = true;
        self
    }

    pub fn with_adaptive_cues(mut self) -> Self {
        self.expects_adaptive_cues = true;
        self
    }

    pub fn with_packed_sfx(mut self) -> Self {
        self.expects_packed_sfx = true;
        self
    }

    pub fn validate(
        &self,
        character_catalog: &CharacterCatalog,
        audio_catalogs: &AudioCatalogRegistry,
    ) -> Option<(&'static str, ambition_load::LoadFailure)> {
        if character_catalog
            .get(self.starting_character.as_str())
            .is_none()
        {
            return Some((
                PREPARE_CATALOGS_WORK_ID,
                ambition_load::LoadFailure::new(
                    "Starting character data is unavailable",
                    format!("character catalog has no '{}' row", self.starting_character),
                )
                .retryable(true),
            ));
        }
        if !audio_catalogs.has_provider(self.audio_provider.as_str()) {
            return Some((
                PREPARE_AUDIO_WORK_ID,
                ambition_load::LoadFailure::new(
                    "Provider audio intent is unavailable",
                    format!(
                        "provider '{}' registered no explicit audio fragment",
                        self.audio_provider
                    ),
                )
                .retryable(true),
            ));
        }
        None
    }
}

/// App-local map from experience id to its authored catalog fragments — the
/// authority the shared preparation systems validate against.
#[derive(Resource, Default)]
pub struct PlatformerAuthoredCatalogRegistry {
    by_experience: BTreeMap<String, AuthoredCatalogFragments>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PlatformerAuthoringRegistrationError {
    EmptyExperienceId,
    Conflict {
        experience_id: String,
        existing: AuthoredCatalogFragments,
        candidate: AuthoredCatalogFragments,
    },
}

impl std::fmt::Display for PlatformerAuthoringRegistrationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyExperienceId => write!(f, "platformer experience id must not be empty"),
            Self::Conflict { experience_id, .. } => write!(
                f,
                "platformer experience '{experience_id}' registered conflicting authored catalogs"
            ),
        }
    }
}
impl std::error::Error for PlatformerAuthoringRegistrationError {}

impl PlatformerAuthoredCatalogRegistry {
    pub fn get(&self, experience_id: &str) -> Option<&AuthoredCatalogFragments> {
        self.by_experience.get(experience_id)
    }

    pub fn try_register(
        &mut self,
        experience_id: &str,
        fragments: AuthoredCatalogFragments,
    ) -> Result<(), PlatformerAuthoringRegistrationError> {
        if experience_id.trim().is_empty() {
            return Err(PlatformerAuthoringRegistrationError::EmptyExperienceId);
        }
        if let Some(existing) = self.by_experience.get(experience_id) {
            if existing == &fragments {
                return Ok(());
            }
            return Err(PlatformerAuthoringRegistrationError::Conflict {
                experience_id: experience_id.to_owned(),
                existing: existing.clone(),
                candidate: fragments,
            });
        }
        self.by_experience
            .insert(experience_id.to_owned(), fragments);
        Ok(())
    }

    pub fn deterministic_dump(&self) -> String {
        let mut out = String::new();
        for (experience, fragment) in &self.by_experience {
            out.push_str(&format!(
                "{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
                experience,
                fragment.starting_character,
                fragment.audio_provider,
                fragment.expects_music,
                fragment.expects_procedural_sfx,
                fragment.expects_adaptive_cues,
                fragment.expects_packed_sfx,
            ));
        }
        out
    }
}

/// Copy the active route's declared profiles into the resource the host
/// consumes.
///
/// This is the ONLY place routes and presentation meet. It lives in the
/// provider crate because that is the layer that already knows about the shell
/// router; `ambition_platformer2d_host` deliberately cannot see routes at all, and so cannot
/// grow a game-name branch even by accident.
///
/// A route with no declaration — the launcher, a menu, a provider that opted
/// out — resolves to the engine default, which is today's full-bleed normal
/// framing. That is what keeps full-screen menus and startup presentation
/// usable across the whole display.
pub fn select_active_presentation_profiles(
    router: Res<ambition_game_shell::ShellRouter>,
    catalog: Option<Res<GameplayPresentationProfileCatalog>>,
    mut active: ResMut<ActiveGameplayPresentationProfiles>,
) {
    let declared = router
        .active
        .as_ref()
        .zip(catalog.as_deref())
        .and_then(|(active, catalog)| catalog.get(active.route_id.as_str()))
        .copied()
        .unwrap_or_default();
    if active.0 != declared {
        active.0 = declared;
    }
}

/// Publish the active route's CAMERA FEEL into the resources the camera reads.
///
/// Exactly parallel to [`select_active_presentation_profiles`], and deliberately
/// a separate system rather than a few more lines inside it: the camera tuning
/// resources live in `camera_ease` and are read by the snapshot resolve and the
/// shake tick, not by the presentation resolve, so folding them together would
/// couple two schedules that have no other reason to meet.
///
/// A route with no declaration resolves to the profile default, which is exactly
/// the historical constants — so the launcher, menus and any provider that opted
/// out ease and shake as they always did.
pub fn publish_active_camera_feel(
    active: Res<ActiveGameplayPresentationProfiles>,
    mut ease: ResMut<ambition_platformer2d_shared_tangle::camera_ease::CameraEaseTuning>,
    mut shake: ResMut<ambition_platformer2d_shared_tangle::camera_ease::CameraShakeTuning>,
) {
    // Read the DEFAULT profile's feel rather than the environment-selected one.
    let feel = active.0.default.camera_feel;
    if *ease != feel.ease {
        *ease = feel.ease;
    }
    if *shake != feel.shake {
        *shake = feel.shake;
    }
}

/// Copy the active route's defense presentation declaration into the resource
/// the renderer consumes.
///
/// The provider owns the route lookup; the renderer sees only the selected
/// policy and therefore cannot grow game-name branches. A route that declares
/// nothing gets no shared defense effect — games opt into those cues explicitly.
pub fn select_active_defense_presentation(
    router: Res<ambition_game_shell::ShellRouter>,
    catalog: Option<Res<DefensePresentationCatalog>>,
    mut active: ResMut<ActiveDefensePresentationPolicy>,
) {
    let declared = router
        .active
        .as_ref()
        .zip(catalog.as_deref())
        .and_then(|(active, catalog)| catalog.get(active.route_id.as_str()))
        .copied()
        .unwrap_or_else(DefensePresentationPolicy::none);
    if active.0 != declared {
        active.0 = declared;
    }
}

/// Copy the active route's declared HUD into the resource the renderer
/// consumes.
///
/// Exactly parallel to [`select_active_presentation_profiles`], and here for
/// the same reason: this is the only place routes and HUD declarations meet,
/// and `ambition_render` cannot see routes at all, so it cannot grow a
/// game-name branch even by accident.
///
/// A route with no declaration — the launcher, a menu, a game that wants no
/// HUD — resolves to `None`, and the renderer draws no HUD surface.
pub fn select_active_hud_declaration(
    router: Res<ambition_game_shell::ShellRouter>,
    catalog: Option<Res<HudDeclarationCatalog>>,
    mut active: ResMut<ActiveHudDeclaration>,
) {
    let declared = router
        .active
        .as_ref()
        .zip(catalog.as_deref())
        .and_then(|(active, catalog)| catalog.get(active.route_id.as_str()))
        .cloned();
    update_active_hud_declaration(&mut active, declared);
}

fn update_active_hud_declaration(
    active: &mut ActiveHudDeclaration,
    declared: Option<HudDeclaration>,
) -> bool {
    // Equal slot COUNTS do not imply equal declarations. Two routes can each
    // have one slot while disagreeing on id, style, region, or centering; the
    // old length-only check left the previous route's HUD active indefinitely.
    if active.0 == declared {
        return false;
    }
    active.0 = declared;
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_platformer2d_shared_tangle::gameplay_presentation::HudSlotSpec;

    #[test]
    fn defense_presentation_follows_the_active_route_in_a_multi_game_host() {
        use ambition_game_shell::{
            ActiveShellExperience, ShellActivationId, ShellExperienceId, ShellRouteId, ShellRouter,
        };

        use ambition_platformer2d_shared_tangle::gameplay_presentation::DefenseCueCauses;

        let shared = DefensePresentationPolicy::shared_iframe_blink();
        let empowerment_blinks = shared.with_blink(DefenseCueCauses::EMPOWERED);

        let mut catalog = DefensePresentationCatalog::default();
        catalog.insert("shared", shared);
        catalog.insert("empowerment-blinks", empowerment_blinks);

        let active = |route: &str| ActiveShellExperience {
            activation_id: ShellActivationId(1),
            route_id: ShellRouteId::new(route),
            experience_id: ShellExperienceId::new(route),
            parameters: Default::default(),
            load_authorization: None,
            prepared_session: None,
        };

        let mut app = bevy::app::App::new();
        // ⛔ NOT `ShellRouter { active, ..Default::default() }`. Functional
        // update needs every field VISIBLE even where it names none of them,
        // and the router keeps its activation counters private — so that form
        // stopped compiling the day they became private, taking the whole
        // `--workspace --lib` gate with it.
        let mut router = ShellRouter::default();
        router.active = Some(active("shared"));
        app.insert_resource(router)
        .insert_resource(catalog)
        .init_resource::<ActiveDefensePresentationPolicy>()
        .add_systems(bevy::app::Update, select_active_defense_presentation);

        app.update();
        assert_eq!(
            app.world().resource::<ActiveDefensePresentationPolicy>().0,
            shared
        );

        app.world_mut().resource_mut::<ShellRouter>().active =
            Some(active("empowerment-blinks"));
        app.update();
        assert_eq!(
            app.world().resource::<ActiveDefensePresentationPolicy>().0,
            empowerment_blinks,
            "the previous game's shared-effect policy leaked across the route switch"
        );

        app.world_mut().resource_mut::<ShellRouter>().active = Some(active("undeclared"));
        app.update();
        assert_eq!(
            app.world().resource::<ActiveDefensePresentationPolicy>().0,
            DefensePresentationPolicy::none(),
            "an undeclared route inherited the last gameplay route's defense effects"
        );
    }

    #[test]
    fn equal_sized_route_huds_still_replace_each_other() {
        let old = HudDeclaration::new().slot(HudSlotSpec::new("rings"));
        let next =
            HudDeclaration::new().slot(HudSlotSpec::new("score").centered().with_font_size(30.0));
        let mut active = ActiveHudDeclaration(Some(old));

        assert!(update_active_hud_declaration(
            &mut active,
            Some(next.clone()),
        ));
        assert_eq!(active.0, Some(next));
    }

    /// : two games in one host can ease and shake differently.
    ///
    /// The zoom rates and the snap epsilon were one global resource and the
    /// shake ceiling was a `const` inside `kick`, so a multi-game host had one
    /// camera feel for every game in it — a one-game-shaped limit sitting in the
    /// middle of the thing that exists to host several.
    ///
    /// Driven through the real system against the real resources, because the
    /// claim is not "the struct has a field" — it is that SWITCHING ROUTES
    /// changes what the camera does.
    #[test]
    fn each_route_publishes_its_own_camera_feel() {
        use ambition_platformer2d_shared_tangle::camera_ease::{CameraEaseTuning, CameraShakeTuning};
        use ambition_platformer2d_shared_tangle::gameplay_presentation::{
            CameraFeelPolicy, GameplayPresentationProfile, GameplayPresentationProfiles,
        };

        let snappy = CameraFeelPolicy {
            ease: CameraEaseTuning {
                zoom_out_rate: 9.0,
                zoom_in_rate: 8.0,
                snap_epsilon: 0.5,
            },
            shake: CameraShakeTuning {
                max_amplitude_px: 40.0,
                decay_px_per_s: 200.0,
            },
        };

        let mut app = bevy::app::App::new();
        app.init_resource::<CameraEaseTuning>()
            .init_resource::<CameraShakeTuning>()
            .insert_resource(ActiveGameplayPresentationProfiles(
                GameplayPresentationProfiles::uniform(
                    GameplayPresentationProfile::full_bleed().with_camera_feel(snappy),
                ),
            ))
            .add_systems(bevy::app::Update, publish_active_camera_feel);
        app.update();

        assert_eq!(*app.world().resource::<CameraEaseTuning>(), snappy.ease);
        assert_eq!(*app.world().resource::<CameraShakeTuning>(), snappy.shake);

        // Switch to a route that declares nothing — a menu, the launcher, a
        // provider that opted out. It must land back on the historical defaults
        // rather than inherit the last game's feel, or leaving Sanic would leave
        // his camera behind in Mary-O.
        *app.world_mut()
            .resource_mut::<ActiveGameplayPresentationProfiles>() =
            ActiveGameplayPresentationProfiles::default();
        app.update();

        assert_eq!(
            *app.world().resource::<CameraEaseTuning>(),
            CameraEaseTuning::default(),
            "an undeclared route inherited the previous game's ease"
        );
        assert_eq!(
            *app.world().resource::<CameraShakeTuning>(),
            CameraShakeTuning::default(),
        );
    }

    /// The ceiling is the ROUTE's now, and a kick obeys it.
    #[test]
    fn a_kick_is_clamped_by_the_routes_ceiling_not_a_constant() {
        use ambition_platformer2d_shared_tangle::camera_ease::{CameraShakeState, CameraShakeTuning};

        let mut shake = CameraShakeState::default();
        shake.kick(
            1000.0,
            CameraShakeTuning {
                max_amplitude_px: 3.0,
                ..CameraShakeTuning::default()
            },
        );
        assert_eq!(
            shake.amplitude_px, 3.0,
            "the kick used the old hardcoded 14px cap instead of the route's"
        );
    }

    #[test]
    fn identical_route_hud_is_left_unchanged() {
        let declaration = HudDeclaration::new().slot(HudSlotSpec::new("rings"));
        let mut active = ActiveHudDeclaration(Some(declaration.clone()));
        assert!(!update_active_hud_declaration(
            &mut active,
            Some(declaration.clone()),
        ));
        assert_eq!(active.0, Some(declaration));
    }
}

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
    /// What this experience's HUD reads out.
    pub hud: Option<ambition_platformer2d_shared_tangle::gameplay_presentation::HudDeclaration>,
    /// Whether the launcher offers this experience to a player.
    ///
    /// `true` by default — a provider that goes to the trouble of authoring an
    /// experience usually means it to be playable. See
    /// [`PlatformerExperienceAuthoring::unlisted`].
    pub listed: bool,
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
            hud: None,
            listed: true,
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
    /// builds the authored [`PreparedPlatformerSource`] this experience plays in
    /// (it may read the provider's own resources). The shared lifecycle runs it
    /// once on an update containing matching preparation requests, gives each
    /// transaction an owned copy, validates the authored catalogs, publishes
    /// the typed prepared-session identity, and constructs the live session on
    /// activation.
    pub fn install<S, Marker>(self, app: &mut App, source: S)
    where
        S: IntoSystem<(), PreparedPlatformerSource, Marker>,
    {
        self.register(app);
        let experience_id = self.experience_id.clone();
        let tag = move |In(world): In<PreparedPlatformerSource>| (experience_id.clone(), world);
        app.add_systems(
            Update,
            source
                .pipe(tag)
                .pipe(lifecycle::prepare_requested_sessions)
                .run_if(lifecycle::preparation_requested(self.experience_id))
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
            .init_resource::<PlatformerStreamingReadiness>();
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
