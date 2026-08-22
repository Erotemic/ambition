//! Shared shell-host composition for one platformer experience.
//!
//! [`ShellComposition`] installs the provider plugin plus its launcher/gameplay routes,
//! host configuration, and optional frontend audio after the shell foundation exists.
//! Engine, renderer, asset-source, and extra-route choices remain explicit at the call
//! site. Every installed resource can still be overridden by a later caller write.

use bevy::app::{App, Plugins};

use ambition_game_shell::{
    ShellHostConfiguration, ShellHostSpec, ShellLaunchCatalog, ShellRouteCatalog, ShellRouteSpec,
};

/// The three ids a shell host needs about the experience it is hosting.
///
/// They are separate strings because they are separate facts: two routes and an
/// audio/catalog identity. A host that derives one from another (appending
/// `"_gameplay"`, say) has invented a naming convention the engine does not
/// have.
#[derive(Clone, Debug)]
pub struct ShellComposition {
    experience_id: String,
    launcher_route: String,
    gameplay_route: String,
    frontend_audio: Option<ambition_audio::selection::FrontendAudioProfile>,
    /// Where the host lands on boot. `None` means the gameplay route, which is
    /// what every caller got before [`ShellComposition::starting_at`] existed.
    initial_route: Option<String>,
}

impl ShellComposition {
    /// * `experience_id` — the provider's own id, the one it registered its
    ///   catalog fragments under. Used for the frontend audio context, so
    ///   loading and launcher frames play THIS provider's sounds (or, for a
    ///   provider that authors none, deliberate silence) instead of inheriting
    ///   another's cache.
    /// * `launcher_route` — where the host starts and what `QuitToHome`
    ///   resolves to.
    /// * `gameplay_route` — the route the experience registered for its
    ///   session.
    pub fn new(
        experience_id: impl Into<String>,
        launcher_route: impl Into<String>,
        gameplay_route: impl Into<String>,
    ) -> Self {
        Self {
            experience_id: experience_id.into(),
            launcher_route: launcher_route.into(),
            gameplay_route: gameplay_route.into(),
            frontend_audio: None,
            initial_route: None,
        }
    }

    /// Boot into a route that is neither the gameplay nor the launcher one.
    ///
    /// The smash demo is the consumer. It opens on CHARACTER SELECT, because a
    /// platform fighter that boots onto the stage has already decided who you
    /// are — and that is neither `PrimaryGameplay` nor `Launcher`, so a
    /// two-variant policy could not say it. A named route can say all three.
    pub fn starting_at(mut self, route: impl Into<String>) -> Self {
        self.initial_route = Some(route.into());
        self
    }

    /// A frontend audio context richer than "this experience, no cues".
    ///
    /// Present because a host in the tree already needs it: the Sanic demo's
    /// launcher declares the three menu cues, and a composition that could only
    /// write the bare profile would have made a real host opt out of the seam to
    /// keep a feature it already had. The default — `FrontendAudioProfile::new`
    /// on the experience id — is the answer for a provider that authors no
    /// frontend sound, which is most of them.
    ///
    /// this is the DEFAULT for the routes this composition owns, not a
    /// declaration about any one screen. A composition is one app hosting one
    /// experience, so its default is the honest scope. A provider that wants a
    /// particular screen to sound like itself IN ANY HOST declares that beside
    /// its other content, with
    /// [`ambition_audio::selection::FrontendAudioAppExt::declare_route_frontend_audio`]
    /// — a declaration that travels, which a composition-level default cannot.
    pub fn with_frontend_audio(
        mut self,
        profile: ambition_audio::selection::FrontendAudioProfile,
    ) -> Self {
        self.frontend_audio = Some(profile);
        self
    }

    /// Install the shell around `experience`.
    ///
    /// `experience` is whatever the provider hands out — a plugin, or a tuple
    /// of them. It is added AFTER the shell so its own registrations can reach
    /// the catalogs, and it is a parameter rather than a step the caller writes
    /// itself because the ordering between "shell exists" and "experience
    /// registers into it" is the one this type is here to own.
    pub fn install<M>(self, app: &mut App, experience: impl Plugins<M>) {
        app.add_plugins(ambition_game_shell::MinimalShellPlugins);
        {
            use ambition_audio::selection::FrontendAudioAppExt;
            app.set_host_frontend_audio(self.frontend_audio.clone().unwrap_or_else(|| {
                ambition_audio::selection::FrontendAudioProfile::new(self.experience_id.clone())
            }));
        }
        // Stated rather than inherited. `AmbitionLoadPlugin` is idempotent, so a host may add it,
        // omit it, or add it twice; adding it here means a consumer never has to know which engine
        // group already satisfied the dependency.
        app.add_plugins(ambition_load::AmbitionLoadPlugin);
        app.add_plugins(ambition_load_presentation::MinimalShellLoadPresentationPlugins);
        app.add_plugins(experience);

        // Home is the generic launcher list — UNLESS the experience already
        // registered this route, in which case home is a screen the provider
        // draws itself and overwriting it here would replace a character select
        // with a list of one game. (That was literally the smash demo's home
        // for a day: its select panels rendered over the launcher's own rows.)
        let home_is_the_providers_own = app.world().resource::<ShellRouteCatalog>().contains(
            &ambition_game_shell::ShellRouteId::new(self.launcher_route.as_str()),
        );
        if !home_is_the_providers_own {
            app.world_mut()
                .resource_mut::<ShellRouteCatalog>()
                .register(ShellRouteSpec::new(
                    self.launcher_route.as_str(),
                    ShellLaunchCatalog::basic_experience_id(),
                ));
        }
        app.world_mut()
            .resource_mut::<ShellHostConfiguration>()
            .spec = Some(ShellHostSpec::new(
            self.initial_route
                .as_deref()
                .unwrap_or(self.gameplay_route.as_str()),
            self.launcher_route.as_str(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_game_shell::ShellRouteId;

    struct SilentExperience;
    impl bevy::app::Plugin for SilentExperience {
        fn build(&self, app: &mut App) {
            // The one thing an experience plugin must be able to do at build
            // time: reach the catalogs the shell just inserted. A composition
            // that added the experience FIRST would panic here, which is the
            // ordering this type exists to own.
            app.world_mut()
                .resource_mut::<ShellRouteCatalog>()
                .register(ShellRouteSpec::new(
                    "test_gameplay",
                    ShellLaunchCatalog::basic_experience_id(),
                ));
        }
    }

    fn composed() -> App {
        let mut app = App::new();
        ShellComposition::new("test_experience", "test_launcher", "test_gameplay")
            .install(&mut app, SilentExperience);
        app
    }

    /// The host spec is the step whose omission is silent: without it the app
    /// boots to a router with no initial route and simply sits there.
    #[test]
    fn the_host_starts_at_the_launcher_and_goes_home_to_it() {
        let app = composed();
        let spec = app
            .world()
            .resource::<ShellHostConfiguration>()
            .spec
            .clone()
            .expect("the composition declared a host spec");
        assert_eq!(spec.initial_route, ShellRouteId::new("test_gameplay"));
        assert_eq!(spec.home_route, ShellRouteId::new("test_launcher"));
    }

    /// The other silent one. A provider authoring no sounds still needs its own
    /// frontend context, or its routes play nothing.
    #[test]
    fn the_frontend_audio_context_is_this_experiences_own() {
        let app = composed();
        assert_eq!(
            app.world()
                .resource::<ambition_audio::selection::FrontendAudioRegistry>()
                .host_default()
                .map(|profile| profile.provider_id()),
            Some("test_experience")
        );
    }

    /// A host may declare its own default afterwards — the composition is
    /// ordinary registry writes, not a policy.
    #[test]
    fn a_host_can_still_override_what_the_composition_wrote() {
        use ambition_audio::selection::FrontendAudioAppExt;

        let mut app = composed();
        app.set_host_frontend_audio(ambition_audio::selection::FrontendAudioProfile::new(
            "somebody_elses_frontend",
        ));
        assert_eq!(
            app.world()
                .resource::<ambition_audio::selection::FrontendAudioRegistry>()
                .host_default()
                .map(|profile| profile.provider_id()),
            Some("somebody_elses_frontend")
        );
    }

    /// a route's own declaration outranks the composition's default, and a route without one
    /// still gets the default.
    #[test]
    fn a_route_that_declares_its_own_sound_is_not_overruled_by_the_default() {
        use ambition_audio::selection::{FrontendAudioAppExt, FrontendAudioProfile};

        let mut app = composed();
        app.declare_route_frontend_audio(
            "test_launcher",
            FrontendAudioProfile::new("a_provider_of_its_own"),
        );
        let registry = app
            .world()
            .resource::<ambition_audio::selection::FrontendAudioRegistry>();
        assert_eq!(
            registry.resolve("test_launcher").map(|p| p.provider_id()),
            Some("a_provider_of_its_own"),
            "the route's own declaration answers for that route",
        );
        assert_eq!(
            registry
                .resolve("some_other_route")
                .map(|p| p.provider_id()),
            Some("test_experience"),
            "a route that declares nothing still gets the composition's default",
        );
    }
}
