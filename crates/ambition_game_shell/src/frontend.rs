//! Explicit ownership vocabulary for shell/frontend presentation.
//!
//! These types let acceptance tests and presentation plugins name which
//! authority owns every title/startup/loading/debug entity instead of inferring
//! ownership from route names or entity shape.

use std::collections::BTreeMap;

use ambition_load::LoadId;
use bevy::prelude::{Component, Resource};

use crate::{ActiveShellExperience, ShellActivationId};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FrontendEntityOwner {
    Host,
    Shell(ShellActivationId),
    Load(LoadId),
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum FrontendPresentationKind {
    HostCamera,
    FrontendUiCamera,
    LauncherRoot,
    StartupRoot,
    LoadingRoot,
    LoadingActivity,
    Kaleidoscope,
    DeveloperOverlay,
    DebugPresentation,
}

#[derive(Component, Clone, Debug, Eq, PartialEq)]
pub struct FrontendOwnedEntity {
    pub owner: FrontendEntityOwner,
    pub kind: FrontendPresentationKind,
}

impl FrontendOwnedEntity {
    pub fn host(kind: FrontendPresentationKind) -> Self {
        Self {
            owner: FrontendEntityOwner::Host,
            kind,
        }
    }

    pub fn shell(activation_id: ShellActivationId, kind: FrontendPresentationKind) -> Self {
        Self {
            owner: FrontendEntityOwner::Shell(activation_id),
            kind,
        }
    }

    pub fn load(load_id: LoadId, kind: FrontendPresentationKind) -> Self {
        Self {
            owner: FrontendEntityOwner::Load(load_id),
            kind,
        }
    }
}

/// Exact shell activation currently authorized to own frontend presentation.
/// It is `None` whenever a registered gameplay experience is active.
#[derive(Resource, Default, Clone, Debug, Eq, PartialEq)]
pub struct ActiveFrontendAuthority(pub Option<ActiveShellExperience>);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PresentationOwnershipClass {
    Frontend,
    GameplaySession,
}

/// Explicit classification for cross-cutting presentation systems that used to
/// rely on convention. Hosts may extend this table, but changing a class is a
/// deliberate architecture decision visible to policy tests.
#[derive(Resource, Clone, Debug, Eq, PartialEq)]
pub struct PresentationOwnershipPolicy {
    classes: BTreeMap<&'static str, PresentationOwnershipClass>,
}

impl Default for PresentationOwnershipPolicy {
    fn default() -> Self {
        Self {
            classes: BTreeMap::from([
                ("map", PresentationOwnershipClass::GameplaySession),
                ("room_visuals", PresentationOwnershipClass::GameplaySession),
                ("moving_platforms", PresentationOwnershipClass::GameplaySession),
                ("kaleidoscope", PresentationOwnershipClass::Frontend),
                ("developer_overlays", PresentationOwnershipClass::Frontend),
                ("debug_presentation", PresentationOwnershipClass::Frontend),
            ]),
        }
    }
}

impl PresentationOwnershipPolicy {
    pub fn class(&self, name: &str) -> Option<PresentationOwnershipClass> {
        self.classes.get(name).copied()
    }

    pub fn iter(
        &self,
    ) -> impl Iterator<Item = (&'static str, PresentationOwnershipClass)> + '_ {
        self.classes.iter().map(|(name, class)| (*name, *class))
    }
}
