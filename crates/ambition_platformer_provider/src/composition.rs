//! **Hosting an experience: the seven steps, minus the six that are not
//! decisions.**
//!
//! [`crate::PlatformerExperienceAuthoring`] is the AUTHORING half — what a
//! provider declares about itself. This is the HOSTING half: what an app has to
//! do to put that provider on screen. Until now every host wrote it out by
//! hand, and three of them agree line for line: the shipped app's standalone
//! demo shells and `fixtures/external_consumer`, which is a third party outside
//! the workspace and is therefore the only honest witness to what the seam
//! actually costs a stranger.
//!
//! What it cost, gathered from the consumer side (queue R4):
//!
//! ```text
//! MinimalShellPlugins → FrontendAudioProfile → AmbitionLoadPlugin →
//! MinimalShellLoadPresentationPlugins → the experience plugin →
//! ShellRouteSpec into ShellRouteCatalog → ShellHostSpec into
//! ShellHostConfiguration
//! ```
//!
//! Seven steps whose ORDER is enforced by a resource-missing panic rather than
//! by a type, and two of whose omissions are silent: no `FrontendAudioProfile`
//! and the launcher inherits whatever provider's audio was cached last; no host
//! spec and the app boots to a router pointing nowhere.
//!
//! ## Why this is a struct and not a plugin group
//!
//! Because it is not only plugins. Two of the steps write RESOURCES that
//! `MinimalShellPlugins` inserts, so they must run after it — which is exactly
//! the ordering a plugin group cannot express and a caller cannot see.
//!
//! ## What it deliberately does NOT do
//!
//! The risk written down with this row was that a builder swallowing everything
//! makes composition unreadable and turns every deviation into a feature
//! request. So this takes the three ids and the experience plugin and stops
//! there. The foundation, the engine group, the host group, the asset source,
//! the renderer and any extra routes stay in the caller's hands, because those
//! are the parts a host can get wrong in an INTERESTING way — the parts worth
//! reading in a composition function.
//!
//! Everything here is also still expressible by hand: `install` writes ordinary
//! resources, so a caller who wants different frontend audio inserts its own
//! `FrontendAudioProfile` afterwards and the later write wins.

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

    /// **Boot into a route that is neither the gameplay nor the launcher one.**
    ///
    /// ⚠ added 2026-07-31, and it is the FOURTH time this exact shape has been
    /// recorded: the SDK expressed the options its first consumer needed, and a
    /// later real host needed another. `StartAt` in the facade already names the
    /// first three (*"one face, one experience, one start policy"*); this is the
    /// same note one layer down.
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
        // The silent one. An app that skips this does not fail — it plays
        // whichever provider's frontend audio was cached last, which on a first
        // run is nothing and after a route change is somebody else's music.
        app.insert_resource(self.frontend_audio.clone().unwrap_or_else(|| {
            ambition_audio::selection::FrontendAudioProfile::new(self.experience_id.clone())
        }));
        // Stated rather than inherited. `AmbitionLoadPlugin` is idempotent, so
        // a host may add it, omit it, or add it twice; adding it here means a
        // consumer never has to know which engine group already satisfied the
        // dependency. (It was a hard Bevy panic until 2026-07-27, and the rule
        // "which group owes the load coordinator" was documented only in the
        // comments of the hosts that had already been bitten.)
        app.add_plugins(ambition_load::AmbitionLoadPlugin);
        app.add_plugins(ambition_load_presentation::MinimalShellLoadPresentationPlugins);
        app.add_plugins(experience);

        app.world_mut()
            .resource_mut::<ShellRouteCatalog>()
            .register(ShellRouteSpec::new(
                self.launcher_route.as_str(),
                ShellLaunchCatalog::basic_experience_id(),
            ));
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
    /// frontend context, or the launcher plays whatever was cached last.
    #[test]
    fn the_frontend_audio_context_is_this_experiences_own() {
        let app = composed();
        assert_eq!(
            app.world()
                .resource::<ambition_audio::selection::FrontendAudioProfile>()
                .provider_id(),
            "test_experience"
        );
    }

    /// A host may insert its own profile afterwards — the composition is
    /// ordinary resource writes, not a policy.
    #[test]
    fn a_host_can_still_override_what_the_composition_wrote() {
        let mut app = composed();
        app.insert_resource(ambition_audio::selection::FrontendAudioProfile::new(
            "somebody_elses_frontend",
        ));
        assert_eq!(
            app.world()
                .resource::<ambition_audio::selection::FrontendAudioProfile>()
                .provider_id(),
            "somebody_elses_frontend"
        );
    }
}
