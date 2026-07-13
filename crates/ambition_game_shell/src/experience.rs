//! Registered top-level experiences: the launcher-visible identity of a provider.
//!
//! A *provider* is any Bevy plugin that owns one top-level experience (its
//! plugins, session setup, load plan, activation, teardown, and semantic
//! completion). It advertises itself to the host by registering an
//! [`ExperienceRegistration`] and a [`ShellRouteSpec`]. The host chooses which
//! provider plugins to compile in; the launcher catalog and route activation are
//! then *derived* from these registrations, never from a central match over demo
//! identities.

use bevy::prelude::{App, DetectChanges, Resource};

use crate::{
    ShellExperienceId, ShellLaunchCatalog, ShellLaunchEntry, ShellRouteCatalog, ShellRouteId,
    ShellRouteSpec,
};

/// Whether a registered experience can currently be launched, and why not.
///
/// Availability is host- and build-dependent (a feature-limited binary may omit
/// a provider's plugins, or a save slot may be missing). The launcher shows an
/// unavailable entry with its reason instead of silently dropping it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExperienceAvailability {
    Available,
    Unavailable { reason: String },
}

impl ExperienceAvailability {
    pub fn unavailable(reason: impl Into<String>) -> Self {
        Self::Unavailable {
            reason: reason.into(),
        }
    }

    pub fn is_available(&self) -> bool {
        matches!(self, Self::Available)
    }

    pub fn reason(&self) -> Option<&str> {
        match self {
            Self::Unavailable { reason } => Some(reason),
            Self::Available => None,
        }
    }
}

/// The launcher-facing registration of one top-level experience/provider.
///
/// This is pure data. Constructing it does not install any behavior — a provider
/// plugin installs its own routes, load plan, and systems, then publishes this so
/// the host can list and launch it without knowing the provider by name.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExperienceRegistration {
    pub id: ShellExperienceId,
    pub display_name: String,
    pub description: String,
    pub launch_route: ShellRouteId,
    pub availability: ExperienceAvailability,
}

impl ExperienceRegistration {
    /// A launchable experience identified by `id`, entered through `launch_route`.
    pub fn new(
        id: impl Into<ShellExperienceId>,
        display_name: impl Into<String>,
        launch_route: impl Into<ShellRouteId>,
    ) -> Self {
        Self {
            id: id.into(),
            display_name: display_name.into(),
            description: String::new(),
            launch_route: launch_route.into(),
            availability: ExperienceAvailability::Available,
        }
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    /// Mark this experience present-but-unavailable with a player-facing reason.
    pub fn unavailable(mut self, reason: impl Into<String>) -> Self {
        self.availability = ExperienceAvailability::unavailable(reason);
        self
    }

    /// The derived launcher entry for this registration.
    pub fn launch_entry(&self) -> ShellLaunchEntry {
        ShellLaunchEntry {
            route_id: self.launch_route.clone(),
            label: self.display_name.clone(),
            description: self.description.clone(),
            available: self.availability.is_available(),
            unavailable_reason: self.availability.reason().map(str::to_owned),
        }
    }
}

/// Ordered set of registered experiences. The launcher catalog is a projection
/// of this registry, so a host that registers a provider gets a launcher entry
/// with no host-side match logic.
#[derive(Resource, Default)]
pub struct ShellExperienceRegistry {
    entries: Vec<ExperienceRegistration>,
}

impl ShellExperienceRegistry {
    /// Register (or replace, matched by experience id) one experience. Returns
    /// the previous registration for that id, if any. Insertion order is stable;
    /// a replacement keeps its original slot.
    pub fn register(
        &mut self,
        registration: ExperienceRegistration,
    ) -> Option<ExperienceRegistration> {
        if let Some(existing) = self.entries.iter_mut().find(|e| e.id == registration.id) {
            return Some(std::mem::replace(existing, registration));
        }
        self.entries.push(registration);
        None
    }

    pub fn get(&self, id: &ShellExperienceId) -> Option<&ExperienceRegistration> {
        self.entries.iter().find(|e| &e.id == id)
    }

    pub fn iter(&self) -> impl Iterator<Item = &ExperienceRegistration> {
        self.entries.iter()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// The derived launcher entries, in registration order.
    pub fn launch_entries(&self) -> Vec<ShellLaunchEntry> {
        self.entries
            .iter()
            .map(ExperienceRegistration::launch_entry)
            .collect()
    }
}

/// Ergonomic provider registration at app-build time.
pub trait ShellExperienceAppExt {
    /// Register one experience: install its `route` in the [`ShellRouteCatalog`]
    /// and publish its `registration` in the [`ShellExperienceRegistry`]. The
    /// route's id must equal the registration's `launch_route`.
    ///
    /// A provider plugin calls this in its `build`; the host installs the
    /// provider plugin. There is no central match over demo identities.
    fn register_experience(
        &mut self,
        registration: ExperienceRegistration,
        route: ShellRouteSpec,
    ) -> &mut Self;
}

impl ShellExperienceAppExt for App {
    fn register_experience(
        &mut self,
        registration: ExperienceRegistration,
        route: ShellRouteSpec,
    ) -> &mut Self {
        assert_eq!(
            registration.launch_route, route.id,
            "experience {} launch_route must match its route spec id",
            registration.id
        );
        let world = self.world_mut();
        // Validate the COMPLETE candidate against BOTH catalogs before mutating
        // either, so a conflicting registration leaves prior valid state intact
        // (transactional). Two failure modes, both deterministic composition
        // errors with order-independent diagnostics:
        //
        //  1. duplicate experience id — two providers claiming one launcher
        //     identity would make launcher order and routing ambiguous;
        //  2. duplicate route id — two experiences claiming one route would make
        //     activation ambiguous (and `BTreeMap::insert` would silently clobber
        //     the first route).
        //
        // An IDENTICAL re-registration (same plugin composed twice) is
        // idempotent and returns before any mutation.
        if let Some(existing) = world
            .get_resource::<ShellExperienceRegistry>()
            .and_then(|registry| registry.get(&registration.id).cloned())
        {
            assert!(
                existing == registration,
                "{}",
                duplicate_experience_diagnostic(&registration.id, &existing, &registration),
            );
            // Same id AND identical spec: the route is already registered from the
            // first call, so re-registering it would trip the duplicate-route
            // check below. Return here — idempotent, no mutation.
            return self;
        }
        // The experience id is NEW. Any existing route under this id therefore
        // belongs to a DIFFERENT experience — a genuine collision.
        if let Some(existing_route) = world
            .get_resource::<ShellRouteCatalog>()
            .and_then(|catalog| catalog.get(&route.id).cloned())
        {
            panic!(
                "{}",
                duplicate_route_diagnostic(&route.id, &existing_route.experience, &registration.id),
            );
        }
        world
            .get_resource_or_insert_with(ShellRouteCatalog::default)
            .register(route);
        world
            .get_resource_or_insert_with(ShellExperienceRegistry::default)
            .register(registration);
        self
    }
}

/// Order-independent diagnostic for two experiences claiming one id. Both
/// descriptors are sorted before formatting so registering A-then-B and
/// B-then-A produce the byte-identical message.
fn duplicate_experience_diagnostic(
    id: &ShellExperienceId,
    a: &ExperienceRegistration,
    b: &ExperienceRegistration,
) -> String {
    let describe = |reg: &ExperienceRegistration| {
        format!(
            "'{}' (route '{}')",
            reg.display_name,
            reg.launch_route.as_str()
        )
    };
    let (first, second) = canonical_pair(describe(a), describe(b));
    format!(
        "duplicate shell experience id '{}': two experiences claim it: {first} and {second}",
        id.as_str(),
    )
}

/// Order-independent diagnostic for two experiences claiming one route id.
fn duplicate_route_diagnostic(
    route: &ShellRouteId,
    a: &ShellExperienceId,
    b: &ShellExperienceId,
) -> String {
    let (first, second) = canonical_pair(
        format!("experience '{}'", a.as_str()),
        format!("experience '{}'", b.as_str()),
    );
    format!(
        "duplicate shell route id '{}': claimed by {first} and {second}",
        route.as_str()
    )
}

/// Sort two descriptors so a diagnostic reads the same regardless of which
/// registration arrived first.
fn canonical_pair(a: String, b: String) -> (String, String) {
    if a <= b {
        (a, b)
    } else {
        (b, a)
    }
}

/// Rebuild the launcher catalog from the experience registry.
///
/// Runs whenever the registry changes (registrations happen at app build, so
/// this fires on the first frame). The launcher catalog is a pure projection:
/// the registry is the single source of truth for what a host can launch.
pub(crate) fn sync_registry_into_launch_catalog(
    registry: bevy::prelude::Res<ShellExperienceRegistry>,
    mut catalog: bevy::prelude::ResMut<ShellLaunchCatalog>,
) {
    if !registry.is_changed() {
        return;
    }
    // A host with no registered experiences (e.g. a pure headless load test) must
    // not have its manually-seeded catalog wiped. Only project when non-empty.
    if registry.is_empty() {
        return;
    }
    catalog.entries = registry.launch_entries();
}

#[cfg(test)]
mod register_tests {
    use super::*;
    use bevy::prelude::App;

    fn reg(id: &str, name: &str, route: &str) -> ExperienceRegistration {
        ExperienceRegistration::new(id, name, route)
    }

    #[test]
    fn identical_re_registration_is_idempotent() {
        let mut app = App::new();
        app.register_experience(
            reg("sanic", "Sanic", "sanic_gameplay"),
            ShellRouteSpec::new("sanic_gameplay", "sanic"),
        );
        app.register_experience(
            reg("sanic", "Sanic", "sanic_gameplay"),
            ShellRouteSpec::new("sanic_gameplay", "sanic"),
        );
        assert_eq!(
            app.world().resource::<ShellExperienceRegistry>().len(),
            1,
            "an identical re-registration is a no-op, not a second entry"
        );
    }

    #[test]
    #[should_panic(expected = "duplicate shell experience id 'sanic'")]
    fn conflicting_duplicate_experience_id_panics() {
        let mut app = App::new();
        app.register_experience(
            reg("sanic", "Sanic", "sanic_gameplay"),
            ShellRouteSpec::new("sanic_gameplay", "sanic"),
        );
        // Same id, different owner/route — a genuine conflict.
        app.register_experience(
            reg("sanic", "Impostor", "impostor_route"),
            ShellRouteSpec::new("impostor_route", "sanic"),
        );
    }

    /// Capture the panic message from `build`, suppressing the default hook so
    /// the test output stays clean.
    fn capture_panic(build: impl FnOnce() + std::panic::UnwindSafe) -> String {
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));
        let result = std::panic::catch_unwind(build);
        std::panic::set_hook(previous);
        let payload = result.expect_err("expected a panic");
        payload
            .downcast_ref::<String>()
            .cloned()
            .or_else(|| payload.downcast_ref::<&str>().map(|s| s.to_string()))
            .expect("panic payload is a string")
    }

    /// Issue 7: two different experiences claiming one route id is a collision,
    /// not a silent clobber. Issue 8: the diagnostic is byte-identical regardless
    /// of which registered first.
    #[test]
    fn duplicate_route_id_is_rejected_in_both_orders_with_one_message() {
        let forward = capture_panic(|| {
            let mut app = App::new();
            app.register_experience(
                reg("alpha", "Alpha", "shared_route"),
                ShellRouteSpec::new("shared_route", "alpha"),
            );
            app.register_experience(
                reg("beta", "Beta", "shared_route"),
                ShellRouteSpec::new("shared_route", "beta"),
            );
        });
        let reverse = capture_panic(|| {
            let mut app = App::new();
            app.register_experience(
                reg("beta", "Beta", "shared_route"),
                ShellRouteSpec::new("shared_route", "beta"),
            );
            app.register_experience(
                reg("alpha", "Alpha", "shared_route"),
                ShellRouteSpec::new("shared_route", "alpha"),
            );
        });
        assert!(
            forward.contains("duplicate shell route id 'shared_route'"),
            "message names the colliding route: {forward}"
        );
        assert_eq!(
            forward, reverse,
            "the route-collision diagnostic is registration-order-independent"
        );
    }

    /// Issue 8: the duplicate-experience-id diagnostic is also order-independent.
    #[test]
    fn duplicate_experience_id_diagnostic_is_order_independent() {
        let forward = capture_panic(|| {
            let mut app = App::new();
            app.register_experience(
                reg("dup", "First", "route_a"),
                ShellRouteSpec::new("route_a", "dup"),
            );
            app.register_experience(
                reg("dup", "Second", "route_b"),
                ShellRouteSpec::new("route_b", "dup"),
            );
        });
        let reverse = capture_panic(|| {
            let mut app = App::new();
            app.register_experience(
                reg("dup", "Second", "route_b"),
                ShellRouteSpec::new("route_b", "dup"),
            );
            app.register_experience(
                reg("dup", "First", "route_a"),
                ShellRouteSpec::new("route_a", "dup"),
            );
        });
        assert!(forward.contains("duplicate shell experience id 'dup'"));
        assert_eq!(forward, reverse);
    }

    /// A route registered by a host directly (e.g. a non-gameplay home route)
    /// still collides deterministically with a later experience claiming it.
    #[test]
    fn preexisting_route_blocks_a_later_experience_claiming_it() {
        let message = capture_panic(|| {
            let mut app = App::new();
            app.world_mut()
                .get_resource_or_insert_with(ShellRouteCatalog::default)
                .register(ShellRouteSpec::new("home", "host_home"));
            app.register_experience(
                reg("game", "Game", "home"),
                ShellRouteSpec::new("home", "game"),
            );
        });
        assert!(
            message.contains("duplicate shell route id 'home'"),
            "a manually-registered route is still protected: {message}"
        );
    }

    #[test]
    fn launcher_entries_stay_unique_and_ordered() {
        let mut app = App::new();
        for (id, name) in [
            ("ambition", "Ambition"),
            ("sanic", "Sanic"),
            ("mary_o", "Mary-O"),
        ] {
            let route = format!("{id}_gameplay");
            app.register_experience(
                reg(id, name, &route),
                ShellRouteSpec::new(route.as_str(), id),
            );
        }
        let registry = app.world().resource::<ShellExperienceRegistry>();
        let ids: Vec<_> = registry.iter().map(|e| e.id.as_str().to_owned()).collect();
        assert_eq!(
            ids,
            vec!["ambition", "sanic", "mary_o"],
            "registration order is stable"
        );
        assert_eq!(
            registry.launch_entries().len(),
            3,
            "each provider appears exactly once"
        );
    }
}
