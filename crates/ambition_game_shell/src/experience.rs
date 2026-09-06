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
    /// Whether the launcher ADVERTISES this experience.
    ///
    /// not the same question as [`availability`](Self::availability), and
    /// conflating them was the gap. An unavailable experience is shown and
    /// greyed with a reason, because the player is meant to know it exists and
    /// why they cannot have it. An UNLISTED one is composed, routed and
    /// reachable — and simply not offered, because it is a test fixture or a
    /// development stage rather than something anyone came here to play.
    ///
    /// it stays fully registered, which is the whole point: its route is
    /// in the catalog, its characters join the roster, and a test that activates
    /// it by route id works unchanged. Removing the composition instead would
    /// have deleted the only place two providers' casts coexist.
    pub listed: bool,
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
            listed: true,
        }
    }

    /// Compose and route this experience, but keep it out of the launcher.
    ///
    /// For a stage that exists to be tested or developed against rather than
    /// chosen: the route works, the roster is installed, and the player's game
    /// list stays a list of games.
    pub fn unlisted(mut self) -> Self {
        self.listed = false;
        self
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

    /// Enter through a route other than the one that owns the session.
    ///
    /// That is right for a game whose first frame IS gameplay, and wrong for one that asks a
    /// question first: a character select, a stage select, a save-slot picker. Those are not
    /// loading screens (nothing is loading) and not the launcher (the launcher lists games, not
    /// fighters), so the shell had nowhere to put them and the smash demo's select screen could
    /// only exist as its own app's HOME — unreachable from a host that lists more than one
    /// game.
    ///
    /// The entry route is an ordinary shell route the provider registers itself,
    /// under an experience id of its own that is NOT a gameplay session (a
    /// frontend screen the provider draws). It must already be in the
    /// [`ShellRouteCatalog`] when the experience registers — advertising an
    /// entry nobody registered is the one failure this cannot detect later.
    pub fn entered_at(mut self, route: impl Into<ShellRouteId>) -> Self {
        self.launch_route = route.into();
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
    ///
    /// UNLISTED registrations are omitted, and they are the only thing
    /// omitted. An *unavailable* experience still appears here — greyed, with
    /// its reason — because the player is meant to see that it exists. This
    /// filter is for the other case: a stage that is composed and routed but was
    /// never for the player to choose.
    pub fn launch_entries(&self) -> Vec<ShellLaunchEntry> {
        self.entries
            .iter()
            .filter(|registration| registration.listed)
            .map(ExperienceRegistration::launch_entry)
            .collect()
    }
}

/// Ergonomic provider registration at app-build time.
pub trait ShellExperienceAppExt {
    /// Register one experience: install its `route` in the [`ShellRouteCatalog`]
    /// and publish its `registration` in the [`ShellExperienceRegistry`].
    ///
    /// `route` is the experience's SESSION route. The registration's
    /// `launch_route` is where the launcher sends the player, which is normally
    /// the same route — an experience that opens on a screen of its own
    /// ([`ExperienceRegistration::entered_at`]) must have registered that route
    /// FIRST, and this refuses an entry route nobody registered.
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
        let world = self.world_mut();
        // The entry route either IS this session route, or is a route somebody
        // already registered. A launcher row pointing at an unknown route is a
        // dead entry that fails at the worst possible moment — when a player
        // presses it — so it fails here, naming what does exist.
        if registration.launch_route != route.id {
            let registered = world
                .get_resource::<ShellRouteCatalog>()
                .is_some_and(|catalog| catalog.contains(&registration.launch_route));
            assert!(
                registered,
                "experience {} enters at route '{}', which no one registered; its \
                 session route is '{}' and the registered routes are [{}]",
                registration.id,
                registration.launch_route.as_str(),
                route.id.as_str(),
                world
                    .get_resource::<ShellRouteCatalog>()
                    .map(|catalog| catalog.ids().collect::<Vec<_>>().join(", "))
                    .unwrap_or_default(),
            );
        }
        // Two failure modes, both deterministic composition errors with order-independent
        // diagnostics:
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
mod register_tests;
