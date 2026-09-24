//! Route-scoped state ownership for one provider/experience family.
//!
//! [`ExperienceScope`] names an owner, the experiences considered inside the
//! scope, and state to release when routing leaves that set. Release is
//! owner-aware rather than unconditional because several experiences may publish
//! the same global resource. A provider can cover frontend and gameplay routes
//! in one scope so transitions between its own routes do not discard state.
//! Shell composition installs the release systems; standalone harnesses may call
//! the public release system explicitly.

use std::collections::BTreeSet;

use bevy::prelude::{App, Res, Resource, World};

use crate::{ShellExperienceId, ShellRouter};

/// The ownership claim a giveback makes about the state it releases.
///
/// [`ReleaseKind::SoleRemoval`] claims that no other experience publishes the
/// resource. Two scopes that make that claim for one resource contradict each
/// other; recording the kind lets a check across all scopes find this.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReleaseKind {
    /// [`ExperienceScopeBuilder::releasing`]: removed, on the claim that only
    /// this provider publishes it.
    SoleRemoval,
    /// [`ExperienceScopeBuilder::releasing_owned`]: removed only when the value
    /// says this owner published it. For shared state.
    OwnedRemoval,
    /// [`ExperienceScopeBuilder::resetting`]: set back to its default. Not a
    /// removal, so it makes no ownership claim.
    Reset,
    /// [`ExperienceScopeBuilder::releasing_with`]: a custom giveback. Any
    /// ownership rule is inside the closure and cannot be checked.
    Custom,
}

/// One thing a scope gives back when its experience leaves.
struct ScopedRelease {
    what: &'static str,
    kind: ReleaseKind,
    release: Box<dyn Fn(&mut World, &ShellExperienceId) + Send + Sync>,
}

/// One provider's claim over a set of shell experiences, and the state that
/// leaves with it.
pub struct ExperienceScope {
    owner: ShellExperienceId,
    inside: BTreeSet<ShellExperienceId>,
    releases: Vec<ScopedRelease>,
    /// Whether the active route was inside this scope at the last release pass.
    /// Release happens on the `true → false` edge.
    inside_now: bool,
}

impl ExperienceScope {
    pub fn owner(&self) -> &ShellExperienceId {
        &self.owner
    }

    /// Whether `experience` is one of the ids this scope treats as itself.
    pub fn covers(&self, experience: &ShellExperienceId) -> bool {
        self.inside.contains(experience)
    }

    /// The names of the state this scope releases, for diagnostics.
    pub fn released_state(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.releases.iter().map(|release| release.what)
    }

    /// Every giveback this scope declares, with its ownership claim. Read
    /// across all scopes to find two sole publishers of one resource.
    pub fn releases(&self) -> impl Iterator<Item = (&'static str, ReleaseKind)> + '_ {
        self.releases
            .iter()
            .map(|release| (release.what, release.kind))
    }
}

/// Every registered scope. Read it to ask whether a provider is currently on its
/// own routes; the shell writes it.
#[derive(Resource, Default)]
pub struct ShellExperienceScopes {
    scopes: Vec<ExperienceScope>,
}

impl ShellExperienceScopes {
    pub fn iter(&self) -> impl Iterator<Item = &ExperienceScope> {
        self.scopes.iter()
    }

    pub fn get(&self, owner: &str) -> Option<&ExperienceScope> {
        self.scopes
            .iter()
            .find(|scope| scope.owner.as_str() == owner)
    }

    fn entry(&mut self, owner: &ShellExperienceId) -> &mut ExperienceScope {
        if let Some(index) = self.scopes.iter().position(|scope| &scope.owner == owner) {
            return &mut self.scopes[index];
        }
        self.scopes.push(ExperienceScope {
            owner: owner.clone(),
            inside: BTreeSet::from([owner.clone()]),
            releases: Vec::new(),
            inside_now: false,
        });
        self.scopes.last_mut().expect("a scope was just pushed")
    }
}

/// Is this experience the one on screen right now?
///
/// A run condition read from the router, not a cached flag, so it is correct
/// anywhere in `Update`. A host with no router reads as inactive.
pub fn shell_experience_is_active(
    experience: impl Into<ShellExperienceId>,
) -> impl Fn(Option<Res<ShellRouter>>) -> bool + Clone {
    let experience = experience.into();
    move |router| {
        router.is_some_and(|router| {
            router
                .active
                .as_ref()
                .is_some_and(|active| active.experience_id == experience)
        })
    }
}

/// Release the state of every scope the shell has just left.
///
/// Only `router.active` matters, not `pending`. `ShellRouter::activate`
/// replaces the old activation in one call, and while a route waits on its
/// load, `active` still names the route being left. A departure is a change of
/// `active`.
///
/// Exclusive world access, not `Commands`: a deferred release would stay
/// visible for one more frame of the next experience.
///
/// Public so a harness without the shell plugin can run the real release
/// against the real declarations. The shell registers it in
/// `AmbitionGameShellPlugin` (Cleanup).
pub fn release_departed_experience_state(world: &mut World) {
    if !world.contains_resource::<ShellExperienceScopes>() {
        return;
    }
    let on_screen = {
        let Some(router) = world.get_resource::<ShellRouter>() else {
            return;
        };
        router
            .active
            .as_ref()
            .map(|active| active.experience_id.clone())
    };
    world.resource_scope(
        |world, mut scopes: bevy::prelude::Mut<ShellExperienceScopes>| {
            for scope in &mut scopes.scopes {
                let inside = on_screen
                    .as_ref()
                    .is_some_and(|experience| scope.inside.contains(experience));
                if scope.inside_now && !inside {
                    for release in &scope.releases {
                        (release.release)(world, &scope.owner);
                    }
                }
                scope.inside_now = inside;
            }
        },
    );
}

/// Declare what a provider owns and what leaves with it.
pub struct ExperienceScopeBuilder<'a> {
    app: &'a mut App,
    owner: ShellExperienceId,
}

impl ExperienceScopeBuilder<'_> {
    fn with(&mut self, edit: impl FnOnce(&mut ExperienceScope)) -> &mut Self {
        let owner = self.owner.clone();
        edit(
            self.app
                .world_mut()
                .get_resource_or_insert_with(ShellExperienceScopes::default)
                .into_inner()
                .entry(&owner),
        );
        self
    }

    /// Another experience id of this provider (e.g. its select or results
    /// screen). Moving between covered ids is not leaving.
    pub fn covering(&mut self, experience: impl Into<ShellExperienceId>) -> &mut Self {
        let experience = experience.into();
        self.with(|scope| {
            scope.inside.insert(experience);
        })
    }

    /// A resource only this provider publishes: removed on the way out.
    ///
    /// Use only when every reader takes `Option<Res<R>>`. A plain `Res<R>` or
    /// `ResMut<R>` parameter panics when the resource is missing. For a
    /// resource that is always read, use [`Self::resetting`].
    pub fn releasing<R: Resource>(&mut self) -> &mut Self {
        let what = std::any::type_name::<R>();
        self.with(move |scope| {
            scope.releases.push(ScopedRelease {
                what,
                kind: ReleaseKind::SoleRemoval,
                release: Box::new(|world, _owner| {
                    world.remove_resource::<R>();
                }),
            });
        })
    }

    /// A resource that must always exist but must not carry state into the
    /// next visit: set back to its default on the way out. Example: the select
    /// screen's value, cursor, and start latch.
    pub fn resetting<R: Resource + Default>(&mut self) -> &mut Self {
        let what = std::any::type_name::<R>();
        self.with(move |scope| {
            scope.releases.push(ScopedRelease {
                what,
                kind: ReleaseKind::Reset,
                release: Box::new(|world, _owner| {
                    if world.contains_resource::<R>() {
                        world.insert_resource(R::default());
                    }
                }),
            });
        })
    }

    /// A resource shared with other experiences: removed only when the value
    /// says this owner published it (e.g.
    /// `MatchParticipantRoster::is_published_by`). This keeps one game from
    /// deleting another's state.
    pub fn releasing_owned<R: Resource>(
        &mut self,
        owned_by: fn(&R, &ShellExperienceId) -> bool,
    ) -> &mut Self {
        let what = std::any::type_name::<R>();
        self.with(move |scope| {
            scope.releases.push(ScopedRelease {
                what,
                kind: ReleaseKind::OwnedRemoval,
                release: Box::new(move |world, owner| {
                    if world
                        .get_resource::<R>()
                        .is_some_and(|value| owned_by(value, owner))
                    {
                        world.remove_resource::<R>();
                    }
                }),
            });
        })
    }

    /// A resource whose owner is recorded on a different resource (the
    /// witness). For state that cannot carry its own publisher: `ActiveMatch` is
    /// rollback state and must not hold a shell experience id.
    ///
    /// The witness must outlive the release, so it must not be declared earlier
    /// in this scope. Givebacks run in declaration order, and a missing witness
    /// would always answer "not mine". This is checked here, and the panic names
    /// both resources.
    pub fn releasing_witnessed<R: Resource, W: Resource>(
        &mut self,
        witness_owns: fn(&W, &ShellExperienceId) -> bool,
    ) -> &mut Self {
        let what = std::any::type_name::<R>();
        let witness = std::any::type_name::<W>();
        self.with(move |scope| {
            assert!(
                !scope.releases.iter().any(|release| release.what == witness),
                "{what} is released on the word of {witness}, but {witness} is \
                 already released earlier in this scope — by the time {what} is \
                 asked about, its witness would be gone and the answer would \
                 always be \"not mine\". Declare the witnessed release first.",
            );
            scope.releases.push(ScopedRelease {
                what,
                // It removes only what this owner published.
                kind: ReleaseKind::OwnedRemoval,
                release: Box::new(move |world, owner| {
                    if world
                        .get_resource::<W>()
                        .is_some_and(|witness| witness_owns(witness, owner))
                    {
                        world.remove_resource::<R>();
                    }
                }),
            });
        })
    }

    /// State whose release is not a removal — a resource that returns to a
    /// default, a component to strip, a latch to lower.
    pub fn releasing_with(
        &mut self,
        what: &'static str,
        release: impl Fn(&mut World, &ShellExperienceId) + Send + Sync + 'static,
    ) -> &mut Self {
        self.with(move |scope| {
            scope.releases.push(ScopedRelease {
                what,
                kind: ReleaseKind::Custom,
                release: Box::new(release),
            });
        })
    }
}

/// Declare an experience scope at app-build time.
pub trait ShellExperienceScopeAppExt {
    /// Begin (or extend) the scope owned by `experience`.
    fn experience_owns(
        &mut self,
        experience: impl Into<ShellExperienceId>,
    ) -> ExperienceScopeBuilder<'_>;
}

impl ShellExperienceScopeAppExt for App {
    fn experience_owns(
        &mut self,
        experience: impl Into<ShellExperienceId>,
    ) -> ExperienceScopeBuilder<'_> {
        let owner = experience.into();
        let mut builder = ExperienceScopeBuilder { app: self, owner };
        // Register the scope even if nothing is declared on it, so `get` can
        // tell "no such owner" from "owns nothing yet".
        builder.with(|_| {});
        builder
    }
}

#[cfg(test)]
mod tests;
