//! Authored command registry and execution boundary.
//!
//! Domains publish immutable command descriptors during app construction. Runtime
//! callers can only request execution through [`RunAuthoredCommand`];
//! [`run_requested_authored_commands`] is the sole runner. [`AuthoredCommandSet`]
//! runs after core simulation and before `GameplayEffects`, so commands enter the
//! owning domain through its existing typed request bus on the same tick.
//!
//! Commands are namespaced verbs, not an expression language, sequencer, general
//! effect enum, or ECS-component scripting surface.

use std::collections::BTreeMap;

use bevy::prelude::{App, IntoScheduleConfigs, Message, Plugin, Resource, SystemSet, World};

use super::{join_namespaced, split_namespaced, AuthoredArg, ParamSpec};

/// A namespaced identifier for one verb a domain can perform.
///
/// The namespace is the owning domain (`world.set_flag`), and the shape is the
/// one [`ConditionId`](super::ConditionId) uses — see
/// [`super::split_namespaced`] on why that rule lives in one place.
///
/// a separate TYPE from `ConditionId` even though the spelling rule is
/// shared, and that is the point. `world.flag_set` and `world.set_flag` are a
/// question and a verb that differ by a word order; a single id type would make
/// asking the catalog to perform a question, or to answer a verb, a runtime
/// miss instead of a compile error.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CommandId(String);

impl CommandId {
    /// Build an id from its domain and its verb.
    ///
    /// panics on a segment containing `.` — an id that can be spelled two
    /// ways is an id that can be published twice.
    pub fn new(domain: &str, verb: &str) -> Self {
        Self(join_namespaced("command", "verb", domain, verb))
    }

    /// Read an id back out of one authored string (`"world.set_flag"`).
    ///
    /// fallible where [`CommandId::new`] panics, and for the same reason
    /// its condition twin is: authored content is exactly the caller that must
    /// never be able to take the process down. A `.yarn` line naming
    /// `worldset_flag` is a typo in content — the right answer is a diagnostic
    /// and nothing happening, not a crash.
    ///
    /// it never repairs. No trimming, no case folding. An id accepted in
    /// two spellings is an id that can be published twice.
    pub fn parse(raw: &str) -> Option<Self> {
        split_namespaced(raw)?;
        Some(Self(raw.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The owning domain — everything before the first `.`.
    pub fn domain(&self) -> &str {
        self.0.split_once('.').map_or(self.0.as_str(), |(d, _)| d)
    }
}

impl std::fmt::Display for CommandId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// What a domain publishes about one verb it can perform.
#[derive(Clone, Debug)]
pub struct CommandDescriptor {
    pub id: CommandId,
    /// One line, for an agent choosing between commands.
    pub summary: &'static str,
    pub params: &'static [ParamSpec],
}

/// What happened, and there are only TWO answers.
///
/// deliberately one fewer than [`ConditionOutcome`](super::ConditionOutcome),
/// and the missing one is the interesting part. A question has three answers
/// because *"I cannot tell"* is genuinely different from *"no"*. A verb does not:
/// either the domain did the thing or it did not, and every reason it did not —
/// no such command, wrong arguments, no save layer installed — is the same fact
/// to the caller, which is that the world did not change.
///
/// so `Refused` always carries a reason, because that reason is the only
/// thing distinguishing the cases and it is what an author reads.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CommandOutcome {
    /// The owning domain performed it.
    Done,
    /// Nothing happened, with a reason written for a human or an agent reading a
    /// diagnostic.
    Refused(String),
}

impl CommandOutcome {
    pub fn refused(reason: impl Into<String>) -> Self {
        Self::Refused(reason.into())
    }

    pub fn is_done(&self) -> bool {
        matches!(self, Self::Done)
    }
}

/// How a domain performs its own verb.
///
/// a plain `fn`, not a boxed closure, for the reason the condition
/// evaluator is one: it keeps the catalog `Clone` and captures nothing, so a
/// registration cannot smuggle state into a value that is supposed to be
/// immutable for the whole run.
///
/// `&mut World`, unlike a condition's `&World` — that is the entire
/// difference between the two halves, and it is why this type may only be called
/// from one place in the frame.
///
/// a runner must not depend on query iteration order, and it must not
/// write anything the owning domain has not registered for rollback. The
/// preferred shape is that a runner writes its domain's EXISTING typed request
/// message and lets the domain's own consumer apply it — which is what
/// `world.set_flag` does, and what makes it snapshot-safe without inventing
/// anything.
pub type CommandRunner = fn(&mut World, &[AuthoredArg]) -> CommandOutcome;

#[derive(Clone)]
struct Registered {
    descriptor: CommandDescriptor,
    run: CommandRunner,
}

/// The composed, read-only catalog of every verb the installed engine can be
/// told to perform.
///
/// derived and read-only, the same axis the condition catalog sits on: a
/// central *authoritative* census every new domain must edit is the thing to
/// avoid; a central *derived index* domains contribute to is how an agent finds
/// out what it can ask for without reading the engine's source.
///
/// not rollback state, and structurally rather than by promise. Every row
/// is written during plugin build; there is no `&mut` accessor to mutate one
/// with, and no public way to perform one either. A rewind that restored it
/// would restore an identical value.
#[derive(Resource, Clone, Default)]
pub struct CommandCatalog {
    rows: BTreeMap<CommandId, Registered>,
}

impl CommandCatalog {
    /// Publish one command. panics on a duplicate id, at startup, by
    /// design: the alternative is that the winner is whichever plugin happened
    /// to build last.
    ///
    /// PRIVATE ON PURPOSE, and that privacy is what earns this value its
    /// rollback waiver. The only way in is [`PublishCommand`] on `App`, and a
    /// simulation tick holds a `World`, never an `App`. making this `pub` for
    /// convenience would silently convert the waiver into a lie — and this half
    /// is the one where the lie would be expensive, because a mutable registry
    /// of verbs is a registry the simulation could rewrite mid-tick.
    fn publish(&mut self, descriptor: CommandDescriptor, run: CommandRunner) {
        let id = descriptor.id.clone();
        if let Some(existing) = self.rows.get(&id) {
            panic!(
                "command `{id}` is already published (`{}`); two domains cannot own one id",
                existing.descriptor.summary
            );
        }
        self.rows.insert(id, Registered { descriptor, run });
    }

    /// Perform one command.
    ///
    /// PRIVATE, and this is the authority answer. A caller holding the
    /// catalog can read the whole vocabulary and cannot speak a word of it. The
    /// only caller is [`run_requested_authored_commands`], twenty lines below —
    /// so *"when in the frame does an authored verb happen"* has exactly one
    /// answer, and it is a schedule position rather than a convention.
    ///
    /// arity and kinds are checked HERE rather than in each runner, the
    /// same way and for the same reason the condition catalog checks them: fifty
    /// domains each writing the same four lines is fifty chances for one of them
    /// to write them differently.
    fn run(&self, world: &mut World, id: &CommandId, args: &[AuthoredArg]) -> CommandOutcome {
        let Some(row) = self.rows.get(id) else {
            return CommandOutcome::refused(format!(
                "no command `{id}` is published; the installed engine knows {} others",
                self.rows.len()
            ));
        };
        let expected = row.descriptor.params;
        if args.len() != expected.len() {
            return CommandOutcome::refused(format!(
                "`{id}` takes {} argument(s) ({}), got {}",
                expected.len(),
                expected
                    .iter()
                    .map(|p| p.name)
                    .collect::<Vec<_>>()
                    .join(", "),
                args.len()
            ));
        }
        for (spec, arg) in expected.iter().zip(args) {
            if arg.kind() != spec.kind {
                return CommandOutcome::refused(format!(
                    "`{id}` argument `{}` is a {:?}, got a {:?}",
                    spec.name,
                    spec.kind,
                    arg.kind()
                ));
            }
        }
        (row.run)(world, args)
    }

    /// Every published command, in id order.
    ///
    /// ordered because this is what a diagnostic prints and what a test
    /// compares.
    pub fn describe_all(&self) -> impl Iterator<Item = &CommandDescriptor> {
        self.rows.values().map(|row| &row.descriptor)
    }

    pub fn describe(&self, id: &CommandId) -> Option<&CommandDescriptor> {
        self.rows.get(id).map(|row| &row.descriptor)
    }

    /// Every command owned by one domain.
    pub fn describe_domain<'a>(
        &'a self,
        domain: &'a str,
    ) -> impl Iterator<Item = &'a CommandDescriptor> + 'a {
        self.describe_all()
            .filter(move |descriptor| descriptor.id.domain() == domain)
    }

    pub fn len(&self) -> usize {
        self.rows.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }
}

/// Publish a command from a domain's own plugin.
///
/// this trait is the entire contract surface a provider needs, which is
/// what makes the no-central-enum claim testable: a crate that can call this can
/// publish a command, and it never names another domain to do so.
pub trait PublishCommand {
    fn publish_command(&mut self, descriptor: CommandDescriptor, run: CommandRunner) -> &mut Self;
}

impl PublishCommand for App {
    fn publish_command(&mut self, descriptor: CommandDescriptor, run: CommandRunner) -> &mut Self {
        self.init_resource::<CommandCatalog>();
        self.world_mut()
            .resource_mut::<CommandCatalog>()
            .publish(descriptor, run);
        self
    }
}

/// Ask for an authored command to be performed.
///
/// a requester outside the simulation must NOT write this channel
/// directly. The Yarn runner executes in `Update`, outside rollback; a write
/// from there is wiped by the next rewind and never re-derived. It goes through
/// `ambition_conversation`'s narrative-input ledger like every other narrative
/// fact, which stamps the tick it applies from.
#[derive(Message, Clone, Debug, PartialEq)]
pub struct RunAuthoredCommand {
    pub id: CommandId,
    pub args: Vec<AuthoredArg>,
}

impl RunAuthoredCommand {
    pub fn new(id: CommandId, args: Vec<AuthoredArg>) -> Self {
        Self { id, args }
    }
}

/// When in the frame an authored verb happens.
///
/// one set, so the answer to *"when does authored content's effect land"* is
/// a schedule position anyone can read, rather than a property of whichever
/// system happened to request it. See this module's header on why the window is
/// after the core simulation and before the domain effect buses are routed.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct AuthoredCommandSet;

/// Perform every command asked for this tick. (sim)
///
/// EXCLUSIVE, and it has to be. A runner takes `&mut World` because the
/// catalog cannot know which domain's state a verb touches. That is one schedule
/// sync point per frame for the whole authored-command vocabulary — the shape
/// to refuse is one exclusive system per command, which is the version of this
/// that gets slow without anybody noticing which change did it.
///
/// it DRAINS rather than reading with a cursor, and that is a determinism
/// choice. A `MessageReader`'s cursor is `Local` state GGRS never rewinds, so
/// after a load it resumes wherever an abandoned future left it. This system
/// holds no `Local` at all: what is in the buffer when it runs is what it
/// performs, and the buffer is empty afterwards.
pub fn run_requested_authored_commands(world: &mut World) {
    let Some(mut messages) =
        world.get_resource_mut::<bevy::ecs::message::Messages<RunAuthoredCommand>>()
    else {
        return;
    };
    if messages.is_empty() {
        return;
    }
    let requests: Vec<RunAuthoredCommand> = messages.drain().collect();
    if !world.contains_resource::<CommandCatalog>() {
        tracing::warn!(
            target: "ambition_platformer2d_shared_tangle::authored_logic",
            "{} authored command(s) were requested but no domain in this composition \
             has published any command",
            requests.len(),
        );
        return;
    }
    world.resource_scope::<CommandCatalog, _>(|world, catalog| {
        for request in requests {
            if let CommandOutcome::Refused(reason) = catalog.run(world, &request.id, &request.args)
            {
                tracing::warn!(
                    target: "ambition_platformer2d_shared_tangle::authored_logic",
                    "authored command `{}` was refused: {reason}",
                    request.id,
                );
            }
        }
    });
}

/// Installs the channel, the set and the one runner.
///
/// it publishes NO command, which is the same separation the condition
/// half keeps: this is the machinery, and a domain adds its verbs from its own
/// plugin with [`PublishCommand`] and no edit here.
pub struct AuthoredCommandPlugin;

impl Plugin for AuthoredCommandPlugin {
    fn build(&self, app: &mut App) {
        use crate::schedule::{
            GameplaySimulationRoot, Platformer2dSimulationPhaseMonolith, SimScheduleExt as _,
        };

        let sim = app.sim_schedule();
        app.init_resource::<CommandCatalog>()
            .add_message::<RunAuthoredCommand>()
            .configure_sets(
                sim,
                // INSIDE the root set, so a frozen session at a title or
                // loading route does not perform authored verbs, and
                // after `CoreSimulation` / before `GameplayEffects` — both
                // sets live in this same schedule, so neither pin is the
                // silently-vacuous cross-schedule kind.
                AuthoredCommandSet
                    .in_set(GameplaySimulationRoot)
                    .after(Platformer2dSimulationPhaseMonolith::CoreSimulation)
                    .before(Platformer2dSimulationPhaseMonolith::GameplayEffects),
            )
            .add_systems(
                sim,
                run_requested_authored_commands.in_set(AuthoredCommandSet),
            );
    }
}

#[cfg(test)]
mod tests;
