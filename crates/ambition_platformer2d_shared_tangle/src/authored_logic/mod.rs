//! **The condition contract: how a domain lets authored content ask it a
//! question.**
//!
//! # What this is for
//!
//! Authoring in this engine is strong for **nouns** — characters, items, rooms,
//! encounters, platforms, portals — and weak for **verbs and relationships over
//! time**. *"When two switches are active, power a lift"*, *"when this flag is
//! set, the wall is gone"*, *"while the player is carrying the key"* all
//! currently fall through into bespoke Rust, one hand-written system and one
//! hand-kept const table at a time.
//!
//! ⭐ **the census that opened this found the gap is on the CONDITION side.** The
//! effect side already has five-plus typed command buses and already learned the
//! lesson the hard way: a monolithic `GameplayEffect` enum was built here and
//! deleted. There is no shared condition type anywhere in the workspace, which is
//! why every gate re-derives its own question.
//!
//! # The one rule that shapes everything here
//!
//! > **A new domain must be able to publish a condition without editing anything
//! > central.**
//!
//! That is the falsifier, and it is tested rather than asserted: the test suite
//! registers a provider **from the test crate**, using only this module's public
//! surface, and requires it to appear in the catalog and evaluate. If that works,
//! no central registry of condition *kinds* exists, because a test crate could
//! not have edited one.
//!
//! ⚠ **[`ConditionArg`] is a closed enum and is NOT the thing that rule
//! forbids.** The non-goal is a central enum of *operations* — a god
//! `EngineEffect` every domain must extend. This is a scalar value type, the
//! same role JSON's value enum plays: domains extend the set of *questions*
//! freely, and nobody needs a new kind of *number*.
//!
//! # What this deliberately is NOT
//!
//! ⛔ **no expression language, no interpreter, no `UniversalRuleVM`, no
//! Lua/Rhai.** Nothing here parses a string during simulation. A condition is a
//! registered id plus prepared arguments; evaluating it calls the owning
//! domain's function.
//!
//! ⛔ **no sequencer.** The census found three genuinely different execution
//! machines already shipping — a monotonic cursor, a reversible cycling timer,
//! and a subroutine stack with interrupts — and one shared form covering all
//! three needs a branch naming its customer. A domain that needs a timeline
//! keeps its own.
//!
//! ⛔ **conditions are not phrased against ECS component layout.** A rule that
//! reads a component is coupled to an implementation detail the owning domain is
//! entitled to change. The domain answers; the rule asks.
//!
//! # ⚠ THE DELETION GATE, stated because this slice has none
//!
//! This module is pure addition, and by this project's own rule that is not yet
//! progress. The deletion it is aimed at is named and specific:
//! `INTRO_FLAG_GATED_LOCK_WALLS` in `ambition_content` — a hand-kept const table
//! pairing lock-wall ids with save flags, read by a bespoke system that walks
//! every LDtk level to rediscover which wall is which. When a `LockWall` can
//! carry its own authored gating condition, the table, the id matching and the
//! walk all go. ⛔ **if that deletion does not land, this contract has not earned
//! its place** and should be removed rather than left as vocabulary nobody
//! speaks.

use std::collections::BTreeMap;

use bevy::prelude::{App, Resource, World};

use crate::sim_id::SimId;

/// **A namespaced identifier for one condition a domain can answer.**
///
/// The namespace is the owning domain (`custody.is_held`, `world.flag_set`), and
/// it is a convention this type enforces rather than merely documents: two
/// domains that both wanted `is_held` would otherwise collide silently at
/// registration, and the loser would be whichever plugin happened to build last.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ConditionId(String);

impl ConditionId {
    /// Build an id from its domain and its question.
    ///
    /// ⛔ **panics on a segment containing `.`**, because an id that can be
    /// spelled two ways is an id that can be registered twice.
    pub fn new(domain: &str, question: &str) -> Self {
        assert!(
            !domain.is_empty() && !question.is_empty(),
            "a condition id needs both a domain and a question, got `{domain}.{question}`"
        );
        assert!(
            !domain.contains('.') && !question.contains('.'),
            "`.` separates a condition id's segments and may not appear inside one: \
             `{domain}.{question}`"
        );
        Self(format!("{domain}.{question}"))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The owning domain — everything before the first `.`.
    pub fn domain(&self) -> &str {
        self.0.split_once('.').map_or(self.0.as_str(), |(d, _)| d)
    }
}

impl std::fmt::Display for ConditionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// **What kind of value one condition argument carries.**
///
/// Deliberately tiny. ⚠ this is the *scalar* vocabulary, not the operation
/// vocabulary — see this module's header on why extending it is not the thing
/// the no-central-enum rule forbids.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParamKind {
    /// A prepared reference to a runtime occurrence.
    ///
    /// ⭐ **a [`SimId`], never a raw string and never an `Entity`.** A string
    /// makes the rule un-renameable and un-validatable; an `Entity` is a slot in
    /// an allocator that does not survive the thing it names.
    Reference,
    /// A short authored name that is not a reference — a save flag id, a
    /// mechanism channel. ⚠ this is the escape hatch, and every use of it is a
    /// small bet that the thing named will never need renaming.
    Name,
    Number,
    Truth,
}

/// One argument a condition takes.
#[derive(Clone, Copy, Debug)]
pub struct ParamSpec {
    pub name: &'static str,
    pub kind: ParamKind,
    /// One line, written for an agent reading the catalog rather than for a
    /// compiler. This is the text that makes discovery useful.
    pub summary: &'static str,
}

/// **A prepared argument value.** Prepared, so nothing here is parsed on a tick.
#[derive(Clone, Debug, PartialEq)]
pub enum ConditionArg {
    Reference(SimId),
    Name(String),
    Number(f64),
    Truth(bool),
}

impl ConditionArg {
    pub fn kind(&self) -> ParamKind {
        match self {
            Self::Reference(_) => ParamKind::Reference,
            Self::Name(_) => ParamKind::Name,
            Self::Number(_) => ParamKind::Number,
            Self::Truth(_) => ParamKind::Truth,
        }
    }

    pub fn as_reference(&self) -> Option<&SimId> {
        match self {
            Self::Reference(id) => Some(id),
            _ => None,
        }
    }

    pub fn as_name(&self) -> Option<&str> {
        match self {
            Self::Name(name) => Some(name),
            _ => None,
        }
    }
}

/// **What a domain publishes about one question it can answer.**
#[derive(Clone, Debug)]
pub struct ConditionDescriptor {
    pub id: ConditionId,
    /// One line, for an agent choosing between conditions.
    pub summary: &'static str,
    pub params: &'static [ParamSpec],
}

/// **The answer, and there are THREE of them.**
///
/// ⭐⭐ **`Unanswerable` is not a failure mode; it is the reason this is an enum
/// and not a `bool`.** *"Is the key held?"* asked about an occurrence that does
/// not exist is not false — false would mean "it exists and nobody has it", and a
/// gate that opens on the negation would swing open for a world that never
/// authored the key at all. It is also the value that makes M5's *"explain why
/// this rule did not fire"* answerable at all.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConditionOutcome {
    Satisfied,
    NotSatisfied,
    /// The domain could not answer, with a reason written for a human or an
    /// agent reading a diagnostic.
    Unanswerable(String),
}

impl ConditionOutcome {
    /// ⚠ **an unanswerable condition is NOT satisfied**, and callers that want
    /// the opposite must say so. Folding the third answer into `false` silently
    /// is exactly what the enum exists to prevent.
    pub fn is_satisfied(&self) -> bool {
        matches!(self, Self::Satisfied)
    }

    pub fn unanswerable(reason: impl Into<String>) -> Self {
        Self::Unanswerable(reason.into())
    }

    pub fn from_bool(value: bool) -> Self {
        if value {
            Self::Satisfied
        } else {
            Self::NotSatisfied
        }
    }
}

/// How a domain answers its own question.
///
/// ⚠ **a plain `fn`, not a boxed closure, and that is deliberate**: it keeps the
/// catalog `Clone` and captures nothing, so a registration cannot smuggle state
/// into a value that is supposed to be immutable for the whole run.
///
/// ⛔ **an evaluator must not depend on query iteration order.** It gets `&World`
/// and Bevy's iteration order is an archetype accident; a condition that counted
/// entities in order would be a determinism bug that reproduces perfectly on one
/// machine. Answer a question about identities, not about a sequence.
///
/// ⭐ **`&World` and not `&mut World`, which a first draft assumed was
/// impossible.** `World::try_query` builds a `QueryState` from a shared
/// reference, so an evaluator can query without the exclusive access that would
/// make every condition serialise against every other. ⚠ and its `None` arm is
/// meaningful rather than a nuisance: a domain asking about a component no
/// installed plugin registered is genuinely *unanswerable*, not false — which is
/// the same distinction [`ConditionOutcome`] exists to keep.
pub type ConditionEvaluator = fn(&World, &[ConditionArg]) -> ConditionOutcome;

#[derive(Clone)]
struct Registered {
    descriptor: ConditionDescriptor,
    evaluate: ConditionEvaluator,
}

/// **The composed, read-only catalog of every condition the installed engine can
/// answer.**
///
/// ⭐⭐ **derived and read-only is the whole point, and this project has confused
/// this axis before.** A central *authoritative* census that every new domain
/// must edit is the thing to avoid; a central *derived index* that domains
/// contribute to is required — it is how an agent finds out what it can ask
/// without reading the engine's source.
///
/// ⚠ **not rollback state, and the reason is structural rather than a promise.**
/// Every row is written during plugin build and nothing mutates it afterwards;
/// there is no `&mut` accessor to mutate one with. A rewind that restored it
/// would restore an identical value.
#[derive(Resource, Clone, Default)]
pub struct ConditionCatalog {
    rows: BTreeMap<ConditionId, Registered>,
}

impl ConditionCatalog {
    /// Publish one condition. ⛔ **panics on a duplicate id**, at startup, by
    /// design: the alternative is that the winner is whichever plugin happened
    /// to build last, which is a bug that only appears when a host changes its
    /// plugin order.
    ///
    /// ⭐⭐ **PRIVATE ON PURPOSE, and that privacy is what earns this value its
    /// rollback waiver.** The only way in is [`PublishCondition`] on `App`, and
    /// a simulation tick holds a `World`, never an `App` — so "immutable once
    /// the simulation starts" is a property of the type rather than a promise in
    /// a comment. ⛔ making this `pub` for convenience would silently convert the
    /// waiver into a lie.
    fn publish(&mut self, descriptor: ConditionDescriptor, evaluate: ConditionEvaluator) {
        let id = descriptor.id.clone();
        if let Some(existing) = self.rows.get(&id) {
            panic!(
                "condition `{id}` is already published (`{}`); two domains cannot own one id",
                existing.descriptor.summary
            );
        }
        self.rows.insert(id, Registered { descriptor, evaluate });
    }

    /// Every published condition, in id order.
    ///
    /// ⭐ ordered because this is what a diagnostic prints and what a test
    /// compares; an unordered listing would be a flaky snapshot.
    pub fn describe_all(&self) -> impl Iterator<Item = &ConditionDescriptor> {
        self.rows.values().map(|row| &row.descriptor)
    }

    pub fn describe(&self, id: &ConditionId) -> Option<&ConditionDescriptor> {
        self.rows.get(id).map(|row| &row.descriptor)
    }

    /// Every condition owned by one domain.
    pub fn describe_domain<'a>(
        &'a self,
        domain: &'a str,
    ) -> impl Iterator<Item = &'a ConditionDescriptor> + 'a {
        self.describe_all()
            .filter(move |descriptor| descriptor.id.domain() == domain)
    }

    pub fn len(&self) -> usize {
        self.rows.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// **Ask the owning domain.**
    ///
    /// ⚠ **arity and kinds are checked HERE rather than in each evaluator.** An
    /// evaluator that had to validate its own arguments would be fifty domains
    /// each writing the same four lines, and the day one of them wrote them
    /// differently the catalog's schema would stop meaning anything.
    pub fn evaluate(&self, world: &World, id: &ConditionId, args: &[ConditionArg]) -> ConditionOutcome {
        let Some(row) = self.rows.get(id) else {
            return ConditionOutcome::unanswerable(format!(
                "no condition `{id}` is published; the installed engine knows {} others",
                self.rows.len()
            ));
        };
        let expected = row.descriptor.params;
        if args.len() != expected.len() {
            return ConditionOutcome::unanswerable(format!(
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
                return ConditionOutcome::unanswerable(format!(
                    "`{id}` argument `{}` is a {:?}, got a {:?}",
                    spec.name,
                    spec.kind,
                    arg.kind()
                ));
            }
        }
        (row.evaluate)(world, args)
    }
}

/// Publish a condition from a domain's own plugin.
///
/// ⭐ **this trait is the entire contract surface a provider needs**, which is
/// what makes the no-central-enum claim testable: a crate that can call this can
/// publish a condition, and it never names another domain to do so.
pub trait PublishCondition {
    fn publish_condition(
        &mut self,
        descriptor: ConditionDescriptor,
        evaluate: ConditionEvaluator,
    ) -> &mut Self;
}

impl PublishCondition for App {
    fn publish_condition(
        &mut self,
        descriptor: ConditionDescriptor,
        evaluate: ConditionEvaluator,
    ) -> &mut Self {
        self.init_resource::<ConditionCatalog>();
        self.world_mut()
            .resource_mut::<ConditionCatalog>()
            .publish(descriptor, evaluate);
        self
    }
}

#[cfg(test)]
mod tests;
