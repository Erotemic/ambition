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
use ambition_runtime::PlatformerSessionWorld;

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

impl PlatformerAuthoredCatalogRegistry {
    pub fn get(&self, experience_id: &str) -> Option<&AuthoredCatalogFragments> {
        self.by_experience.get(experience_id)
    }

    fn register(&mut self, experience_id: &str, fragments: AuthoredCatalogFragments) {
        if let Some(existing) = self.by_experience.get(experience_id) {
            assert_eq!(
                existing, &fragments,
                "platformer experience '{experience_id}' registered conflicting authored catalogs",
            );
            return;
        }
        self.by_experience
            .insert(experience_id.to_owned(), fragments);
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
        }
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
    /// builds the authored [`PlatformerSessionWorld`] this experience plays in
    /// (it may read the provider's own resources). The shared lifecycle runs it
    /// once on an update containing matching preparation requests, gives each
    /// transaction an owned copy, validates the authored catalogs, publishes
    /// the typed prepared-session identity, and constructs the live session on
    /// activation.
    pub fn install<S, Marker>(self, app: &mut App, source: S)
    where
        S: IntoSystem<(), PlatformerSessionWorld, Marker>,
    {
        self.register(app);
        let experience_id = self.experience_id.clone();
        let tag = move |In(world): In<PlatformerSessionWorld>| (experience_id.clone(), world);
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
            .register(self.experience_id.as_str(), self.catalogs.clone());
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
        if let Some(loading) = self.loading.clone() {
            app.init_resource::<ambition_load_presentation::LoadPresentationCatalog>();
            app.world_mut()
                .resource_mut::<ambition_load_presentation::LoadPresentationCatalog>()
                .by_route
                .insert(ShellRouteId::new(self.route_id.clone()), loading);
        }
    }
}
