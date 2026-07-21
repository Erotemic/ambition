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
use ambition_platformer_primitives::gameplay_presentation::{
    ActiveGameplayPresentationProfiles, ActiveHudDeclaration, GameplayPresentationProfileCatalog,
    GameplayPresentationProfiles, HudDeclaration, HudDeclarationCatalog,
};
use ambition_runtime::PreparedPlatformerSource;

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
/// router; `ambition_host` deliberately cannot see routes at all, and so cannot
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
    if active.0 == declared {
        return false;
    }
    active.0 = declared;
    true
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
    use ambition_platformer_primitives::gameplay_presentation::HudSlotSpec;

    #[test]
    fn equal_sized_route_huds_still_replace_each_other() {
        let old = HudDeclaration::new().slot(HudSlotSpec::new("rings"));
        let next = HudDeclaration::new().slot(
            HudSlotSpec::new("score")
                .centered()
                .with_font_size(30.0),
        );
        let mut active = ActiveHudDeclaration(Some(old));

        assert!(update_active_hud_declaration(
            &mut active,
            Some(next.clone()),
        ));
        assert_eq!(active.0, Some(next));
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
    pub label: String,
    pub description: String,
    pub preparation_label: String,
    pub catalogs: AuthoredCatalogFragments,
    pub loading: Option<ambition_load_presentation::LoadExperienceSpec>,
    /// How this experience wants gameplay framed on the physical display.
    /// `None` keeps the engine default (full-bleed, normal framing).
    pub presentation: Option<GameplayPresentationProfiles>,
    /// What this experience's HUD reads out. `None` means it has no declared
    /// HUD — the default, and what every experience did before this seam
    /// existed.
    pub hud: Option<ambition_platformer_primitives::gameplay_presentation::HudDeclaration>,
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
            label: label.into(),
            description: description.into(),
            preparation_label: preparation_label.into(),
            catalogs,
            loading: None,
            presentation: None,
            hud: None,
        }
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

    /// Declare this experience's HUD readouts.
    ///
    /// The declaration says which slots exist, in what order, preferring which
    /// surround region — never what they mean. The live values arrive each
    /// frame through
    /// [`HudReadouts`](ambition_platformer_primitives::gameplay_presentation::HudReadouts),
    /// written by a system the GAME owns, so the engine holds no content
    /// vocabulary and a second game needs no core edit to get a HUD.
    pub fn with_hud(
        mut self,
        hud: ambition_platformer_primitives::gameplay_presentation::HudDeclaration,
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
        app.register_gameplay_experience(
            ExperienceRegistration::new(
                self.experience_id.clone(),
                self.label.clone(),
                self.route_id.clone(),
            )
            .with_description(self.description.clone()),
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
