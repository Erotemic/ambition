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
        world
            .get_resource_or_insert_with(ShellRouteCatalog::default)
            .register(route);
        world
            .get_resource_or_insert_with(ShellExperienceRegistry::default)
            .register(registration);
        self
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
