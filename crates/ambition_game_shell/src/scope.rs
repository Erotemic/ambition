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

/// What OWNERSHIP CLAIM a giveback makes about the state it releases.
///
/// this is a claim about the world, not an implementation detail.
/// [`ReleaseKind::SoleRemoval`] asserts *no other experience publishes this
/// resource* — and two experiences making that claim about one resource is a
/// contradiction that no amount of reading either declaration in isolation can
/// reveal. Recording the kind is what lets the claim be CHECKED across every
/// scope at once, which is the only place the contradiction is visible.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReleaseKind {
    /// [`ExperienceScopeBuilder::releasing`] — removed outright, on the claim
    /// that this provider ALONE publishes it.
    SoleRemoval,
    /// [`ExperienceScopeBuilder::releasing_owned`] — removed only when the value
    /// itself says this owner published it. The shape for SHARED state.
    OwnedRemoval,
    /// [`ExperienceScopeBuilder::resetting`] — put back to its default. Never a
    /// removal, so it makes no ownership claim: a stranger's value is
    /// overwritten rather than deleted, which is a different question.
    Reset,
    /// [`ExperienceScopeBuilder::releasing_with`] — a custom giveback whose
    /// ownership rule, if it has one, lives inside the closure where nothing
    /// outside can read it.
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

    /// Every giveback this scope declares, WITH the ownership claim it makes.
    ///
    /// read across every scope at once, this is what makes "two experiences
    /// both claim to be the sole publisher of one resource" a checkable
    /// question instead of a thing you have to notice by reading two files.
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
/// `router.active` is the whole answer, and a pending route adds nothing.
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
    /// only for a resource every reader takes as `Option<Res<R>>`. A Bevy
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
                kind: ReleaseKind::SoleRemoval,
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
                kind: ReleaseKind::Reset,
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
    /// this is the shape that keeps cleanup from being one game deleting
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

    /// A resource whose OWNER is written on a different resource — the receipt
    /// and the plan it was issued from.
    ///
    /// the shape for state that cannot carry its own publisher. An `ActiveMatch` is
    /// rollback state and deliberately holds nothing but the facts of the activation; stamping
    /// a shell experience id into it would put frontend identity on the rollback wire to answer
    /// a teardown question. So the activation is released by asking the WITNESS.
    ///
    /// the witness must outlive the release, so it may not already be
    /// declared in this scope. Givebacks run in declaration order, and a
    /// witness released first would leave every later release reading a resource
    /// that is gone — silently answering "not mine" forever, which is a release
    /// that stops working rather than one that fails. That ordering is invisible
    /// at the call site, so it is checked HERE, at declaration, where the panic
    /// names both resources.
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
                // An owner-scoped removal, and truthfully so: it removes only
                // what this owner published. Where the proof is written does not
                // change what is being claimed.
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
