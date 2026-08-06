//! **State that lives exactly as long as one experience's stay on its routes.**
//!
//! A provider may publish world state that only makes sense while it is the
//! experience on screen: a decided roster, a lobby's cursor, the seat count a
//! match agreed on. Those are global resources, so leaving the experience does
//! not remove them and the next experience inherits them.
//!
//! # The rule
//!
//! An [`ExperienceScope`] names one OWNER (a [`ShellExperienceId`]), the set of
//! experiences that count as *inside* it, and the state to release the moment
//! the active route stops being inside. Entering is not an event anything has to
//! catch — the scope is inside whenever the router says so, and leaving is the
//! edge that releases.
//!
//! ⚠ **release is OWNER-SCOPED, never "remove the resource".** Two experiences
//! publish into the same global resource (`MatchParticipantRoster` is the one
//! this was built for), so a scope that removed it unconditionally would be one
//! game deleting another's match. [`ExperienceScopeBuilder::releasing_owned`]
//! asks the value who published it and leaves a stranger's alone.
//!
//! ⚠ **a scope covers more than one experience id when a provider has more than
//! one.** A character select is a frontend experience and the match is a
//! gameplay one; moving between them is not leaving, and a scope that named only
//! the gameplay id would release the roster on the frame the lobby handed it
//! over. `covering` is how a provider says which ids are still itself.
//!
//! ⚠ the release systems live in [`crate::AmbitionGameShellPlugin`]. A harness
//! that composes a provider without the shell registers scopes that nothing
//! runs, which is the same deal every other shell facility offers — unless it
//! registers [`release_departed_experience_state`] itself, which is public for
//! exactly that composition.

use std::collections::BTreeSet;

use bevy::prelude::{App, Res, Resource, World};

use crate::{ShellExperienceId, ShellRouter};

/// One thing a scope gives back when its experience leaves.
struct ScopedRelease {
    what: &'static str,
    release: Box<dyn Fn(&mut World, &ShellExperienceId) + Send + Sync>,
}

/// One provider's claim over a set of shell experiences, and the state that
/// leaves with it.
pub struct ExperienceScope {
    owner: ShellExperienceId,
    inside: BTreeSet<ShellExperienceId>,
    releases: Vec<ScopedRelease>,
    /// Whether the active route was inside this scope at the last release pass.
    /// The `true → false` edge is the whole mechanism.
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

/// **Is this experience the one on screen right now?**
///
/// A run condition, answered from the router rather than from a cached flag, so
/// it is correct wherever in `Update` the caller happens to be scheduled. A host
/// with no router installed reads as inactive: a system gated on "my experience
/// owns the route" must not run in a composition that has no routes.
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
/// ⚠ **`router.active` is the whole answer, and a pending route adds nothing.**
/// `ShellRouter::activate` takes the old activation and installs the new one in
/// one non-yielding call, so nothing ever observes `active` empty in the middle
/// of a transition: while a route waits on its load barrier, `active` still
/// names the route being left. A departure is therefore exactly a change of
/// `active`, and consulting `pending` as well would be defensive code for a
/// state the router cannot produce.
///
/// Exclusive-world, and deliberately not `Commands`: a release that landed at
/// the next command flush would be visible to one more frame of the experience
/// that inherited it, which is exactly the window the leak lived in.
///
/// Public so a harness that composes a provider WITHOUT the shell plugin can
/// still run the real release mechanism against the real declarations —
/// otherwise a scope-owned invariant ("the match's rules leave with the
/// match") is untestable except through the whole shell. The shipped
/// registration stays the shell's (`AmbitionGameShellPlugin`, Cleanup).
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

    /// Another experience id that is still this provider (its select screen, its
    /// results screen). Moving between covered ids is not leaving.
    pub fn covering(&mut self, experience: impl Into<ShellExperienceId>) -> &mut Self {
        let experience = experience.into();
        self.with(|scope| {
            scope.inside.insert(experience);
        })
    }

    /// A resource this provider alone publishes: removed outright on the way out.
    ///
    /// ⛔ **only for a resource every reader takes as `Option<Res<R>>`.** A Bevy
    /// system with a plain `Res<R>`/`ResMut<R>` parameter PANICS when the
    /// resource is missing, so releasing one by removal turns a leak into a
    /// crash — measured, on the smash select screen's own `ResMut<SmashSelect>`.
    /// A resource that is `init_resource`'d and always read wants
    /// [`Self::resetting`] instead.
    pub fn releasing<R: Resource>(&mut self) -> &mut Self {
        let what = std::any::type_name::<R>();
        self.with(move |scope| {
            scope.releases.push(ScopedRelease {
                what,
                release: Box::new(|world, _owner| {
                    world.remove_resource::<R>();
                }),
            });
        })
    }

    /// A resource that must always EXIST but must not carry a decision across
    /// the experience that made it: put back to its default on the way out.
    ///
    /// The select screen's value, its cursor and its start latch are this shape
    /// — always present, always read, and a restart that inherited them would
    /// open on the previous match's answer.
    pub fn resetting<R: Resource + Default>(&mut self) -> &mut Self {
        let what = std::any::type_name::<R>();
        self.with(move |scope| {
            scope.releases.push(ScopedRelease {
                what,
                release: Box::new(|world, _owner| {
                    if world.contains_resource::<R>() {
                        world.insert_resource(R::default());
                    }
                }),
            });
        })
    }

    /// A resource SHARED with other experiences: removed only when the value
    /// itself says this owner published it.
    ///
    /// ⭐ this is the shape that keeps cleanup from being one game deleting
    /// another's state, and the predicate is the value's own ownership question
    /// (`MatchParticipantRoster::is_published_by`) rather than a second table
    /// this module would have to keep in step.
    pub fn releasing_owned<R: Resource>(
        &mut self,
        owned_by: fn(&R, &ShellExperienceId) -> bool,
    ) -> &mut Self {
        let what = std::any::type_name::<R>();
        self.with(move |scope| {
            scope.releases.push(ScopedRelease {
                what,
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
