//! Extensible authored condition and command contracts.
//!
//! Domains register condition ids that evaluate from `&World` and command ids
//! that mutate through the command runner; adding a domain does not require a
//! central operation enum. Both sides share scalar [`AuthoredArg`] vocabulary
//! and `<domain>.<leaf>` id spelling. Conditions remain domain-owned questions,
//! not expressions over ECS component layout, and commands remain separately
//! scheduled/authorized mutations.

use std::collections::BTreeMap;

use bevy::prelude::{App, Resource, World};

use crate::sim_id::SimId;

pub mod commands;
pub mod prepared;

pub use commands::{
    AuthoredCommandPlugin, AuthoredCommandSet, CommandCatalog, CommandDescriptor, CommandId,
    CommandOutcome, CommandRunner, PublishCommand, RunAuthoredCommand,
};
pub use prepared::{prepare_authored_arg, PreparationError, PreparedCommand, PreparedCondition};

/// The one spelling rule both halves obey: `<domain>.<leaf>`, exactly one
/// dot, neither side empty.
///
///  it lives here once because a condition id and a command id are the same
/// SHAPE with different meanings. Two copies would be two chances to disagree
/// about whether `a.b.c` is legal — and the day they disagreed, an id that
/// parsed on one side and not the other would look like a missing registration.
fn split_namespaced(raw: &str) -> Option<(&str, &str)> {
    let (domain, leaf) = raw.split_once('.')?;
    if domain.is_empty() || leaf.is_empty() || leaf.contains('.') {
        return None;
    }
    Some((domain, leaf))
}

/// Build a namespaced id, asserting the spelling rule.
///
///  panics, and the two nouns exist so the panic names the caller's world:
/// a provider that spelled its own id wrongly is a bug in the engine, and the
/// message should say `condition`/`question` or `command`/`verb` rather than a
/// generic complaint about segments.
fn join_namespaced(noun: &str, leaf_noun: &str, domain: &str, leaf: &str) -> String {
    assert!(
        !domain.is_empty() && !leaf.is_empty(),
        "a {noun} id needs both a domain and a {leaf_noun}, got `{domain}.{leaf}`"
    );
    assert!(
        !domain.contains('.') && !leaf.contains('.'),
        "`.` separates a {noun} id's segments and may not appear inside one: \
         `{domain}.{leaf}`"
    );
    format!("{domain}.{leaf}")
}

/// A namespaced identifier for one condition a domain can answer.
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
    ///  panics on a segment containing `.`, because an id that can be
    /// spelled two ways is an id that can be registered twice.
    pub fn new(domain: &str, question: &str) -> Self {
        Self(join_namespaced("condition", "question", domain, question))
    }

    /// Read an id back out of one authored string (`"world.flag_set"`).
    ///
    ///  this exists because [`ConditionId::new`] PANICS, and authored
    /// content is exactly the caller that must never be able to do that. A
    /// `.yarn` line asking `condition("worldflag_set", …)` is a typo in content,
    /// not a broken invariant in the engine — the right answer is a diagnostic
    /// and an unsatisfied gate, not a crashed game. So the fallible road in and
    /// the asserting road in are two functions rather than one function with a
    /// mode.
    ///
    ///  it never repairs. No trimming, no case folding, no "did you mean".
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

impl std::fmt::Display for ConditionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// What kind of value one condition argument carries.
///
/// Deliberately tiny.  this is the *scalar* vocabulary, not the operation
/// vocabulary — see this module's header on why extending it is not the thing
/// the no-central-enum rule forbids.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParamKind {
    /// A prepared reference to a runtime occurrence.
    ///
    ///  a [`SimId`], never a raw string and never an `Entity`. A string
    /// makes the rule un-renameable and un-validatable; an `Entity` is a slot in
    /// an allocator that does not survive the thing it names.
    Reference,
    /// A short authored name that is not a reference — a save flag id, a
    /// mechanism channel.  this is the escape hatch, and every use of it is a
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

/// A prepared argument value. Prepared, so nothing here is parsed on a tick.
#[derive(Clone, Debug, PartialEq)]
pub enum AuthoredArg {
    Reference(SimId),
    Name(String),
    Number(f64),
    Truth(bool),
}

impl AuthoredArg {
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

    pub fn as_number(&self) -> Option<f64> {
        match self {
            Self::Number(value) => Some(*value),
            _ => None,
        }
    }
}

/// What a domain publishes about one question it can answer.
#[derive(Clone, Debug)]
pub struct ConditionDescriptor {
    pub id: ConditionId,
    /// One line, for an agent choosing between conditions.
    pub summary: &'static str,
    pub params: &'static [ParamSpec],
}

/// The answer, and there are THREE of them.
///
///  `Unanswerable` is not a failure mode; it is the reason this is an enum and not a
/// `bool`. *"Is the key held?"* asked about an occurrence that does not exist is not false —
/// false would mean "it exists and nobody has it", and a gate that opens on the negation would
/// swing open for a world that never authored the key at all.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConditionOutcome {
    Satisfied,
    /// Answered, and the answer is no — WITH the structure that says why (M5).
    NotSatisfied(WhyNot),
    /// The domain could not answer, with a reason written for a human or an
    /// agent reading a diagnostic.
    Unanswerable(String),
}

/// WHY a condition was not satisfied, as structure rather than a log line: the
/// term that blocked it, the object that term names, and that object's state
/// as the domain saw it. A standing lock wall, a dialogue branch that did not
/// open, an agent asking "why not" — all read this instead of re-deriving it.
///
/// Every production evaluator states one; [`ConditionOutcome::from_bool_unexplained`]
/// exists for test fixtures and is grep-able for exactly that reason.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WhyNot {
    /// The condition (or the sub-question inside it) that answered no, e.g.
    /// `world.flag_set`, `inventory.holds/bag`, `inventory.holds/hand`.
    pub term: String,
    /// The object the term named: a flag id, an item kind, an occurrence.
    pub subject: String,
    /// The subject's current state, in the domain's own words: `unset`,
    /// `bag holds 0 and no player hand wields it`, `in the world at (x, y)`.
    pub observed: String,
}

impl WhyNot {
    pub fn new(
        term: impl Into<String>,
        subject: impl Into<String>,
        observed: impl Into<String>,
    ) -> Self {
        Self {
            term: term.into(),
            subject: subject.into(),
            observed: observed.into(),
        }
    }

    /// The fixture arm: a `no` whose structure nobody stated.
    pub fn unexplained() -> Self {
        Self::new("<unstated>", "<unstated>", "<unstated>")
    }
}

impl std::fmt::Display for WhyNot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} on `{}`: {}", self.term, self.subject, self.observed)
    }
}

impl ConditionOutcome {
    ///  an unanswerable condition is NOT satisfied, and callers that want
    /// the opposite must say so. Folding the third answer into `false` silently
    /// is exactly what the enum exists to prevent.
    pub fn is_satisfied(&self) -> bool {
        matches!(self, Self::Satisfied)
    }

    pub fn unanswerable(reason: impl Into<String>) -> Self {
        Self::Unanswerable(reason.into())
    }

    /// `Satisfied`, or `NotSatisfied` carrying the structure `why` builds —
    /// built lazily, because the satisfied arm never needs it.
    pub fn from_bool(value: bool, why: impl FnOnce() -> WhyNot) -> Self {
        if value {
            Self::Satisfied
        } else {
            Self::NotSatisfied(why())
        }
    }

    /// A `no` with no structure. FIXTURES ONLY: a production evaluator that
    /// reaches for this has a why-not it is not stating, and the grep for this
    /// name is the list of them.
    pub fn from_bool_unexplained(value: bool) -> Self {
        Self::from_bool(value, WhyNot::unexplained)
    }

    /// The structure, when the answer was no.
    pub fn why_not(&self) -> Option<&WhyNot> {
        match self {
            Self::NotSatisfied(why) => Some(why),
            _ => None,
        }
    }
}

/// How a domain answers its own question.
///
///  a plain `fn`, not a boxed closure, and that is deliberate: it keeps the
/// catalog `Clone` and captures nothing, so a registration cannot smuggle state
/// into a value that is supposed to be immutable for the whole run.
///
/// Answer a question about identities, not about a sequence.
///
/// `&World` is sufficient: `World::try_query` (cite-ok: bevy's World, not ours) permits evaluation without
/// exclusive world access. Its `None` arm is
/// meaningful rather than a nuisance: a domain asking about a component no
/// installed plugin registered is genuinely *unanswerable*, not false — which is
/// the same distinction [`ConditionOutcome`] exists to keep.
pub type ConditionEvaluator = fn(&World, &[AuthoredArg]) -> ConditionOutcome;

#[derive(Clone)]
struct Registered {
    descriptor: ConditionDescriptor,
    evaluate: ConditionEvaluator,
}

/// The composed, read-only catalog of every condition the installed engine can
/// answer.
///
///  derived and read-only is the whole point, and this project has confused
/// this axis before. A central *authoritative* census that every new domain
/// must edit is the thing to avoid; a central *derived index* that domains
/// contribute to is required — it is how an agent finds out what it can ask
/// without reading the engine's source.
///
///  not rollback state, and the reason is structural rather than a promise.
/// Every row is written during plugin build and nothing mutates it afterwards;
/// there is no `&mut` accessor to mutate one with. A rewind that restored it
/// would restore an identical value.
#[derive(Resource, Clone, Default)]
pub struct ConditionCatalog {
    rows: BTreeMap<ConditionId, Registered>,
}

impl ConditionCatalog {
    /// Publish one condition.  panics on a duplicate id, at startup, by
    /// design: the alternative is that the winner is whichever plugin happened
    /// to build last, which is a bug that only appears when a host changes its
    /// plugin order.
    ///
    ///  PRIVATE ON PURPOSE, and that privacy is what earns this value its
    /// rollback waiver. The only way in is [`PublishCondition`] on `App`, and
    /// a simulation tick holds a `World`, never an `App` — so "immutable once
    /// the simulation starts" is a property of the type rather than a promise in
    /// a comment.  making this `pub` for convenience would silently convert the
    /// waiver into a lie.
    fn publish(&mut self, descriptor: ConditionDescriptor, evaluate: ConditionEvaluator) {
        let id = descriptor.id.clone();
        if let Some(existing) = self.rows.get(&id) {
            panic!(
                "condition `{id}` is already published (`{}`); two domains cannot own one id",
                existing.descriptor.summary
            );
        }
        self.rows.insert(
            id,
            Registered {
                descriptor,
                evaluate,
            },
        );
    }

    /// Every published condition, in id order.
    ///
    ///  ordered because this is what a diagnostic prints and what a test
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

    /// Ask the owning domain.
    ///
    ///  arity and kinds are checked HERE rather than in each evaluator. An
    /// evaluator that had to validate its own arguments would be fifty domains
    /// each writing the same four lines, and the day one of them wrote them
    /// differently the catalog's schema would stop meaning anything.
    pub fn evaluate(
        &self,
        world: &World,
        id: &ConditionId,
        args: &[AuthoredArg],
    ) -> ConditionOutcome {
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
///  this trait is the entire contract surface a provider needs, which is
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
