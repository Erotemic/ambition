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
    ///
    /// ⛔⛔ **AND A SECOND BET NOBODY WROTE DOWN: THAT A MISSPELLING IS
    /// DISTINGUISHABLE FROM A "NO".** `prepare_one` accepts any string for a
    /// `Name`, so this kind is the one hole in the promise
    /// [`ConditionCatalog::ask`] makes — *"everything that could have been wrong
    /// about the call was already wrong at prepare time."* `Number`, `Truth` and
    /// `Reference` all parse and can refuse; `Name` cannot, because
    /// preparation holds no `World` and the roster it would check lives in one.
    ///
    /// ⇒ **So the refusal has to happen in the EVALUATOR, and the evaluator must
    /// want to.** A name drawn from an authored roster has two absences in it —
    /// *"no such subject"* and *"that subject, and the answer is no"* — and
    /// answering both with `NotSatisfied` is the permissive default that makes a
    /// typo a gate which never opens, silently, forever. Split them:
    /// [`ConditionOutcome::unanswerable`] is the "no such subject" arm and it
    /// reaches a content diagnostic; `false` is an answer about a real thing.
    ///
    /// ⭐ **`body.can` IS THE SHAPE TO COPY** (`actor_monolith/src/body_conditions.rs`).
    /// It resolves the verb against a DEFAULT `AbilitySet` *before* looking at
    /// any body, and its comment says why: *"so an unknown verb is a content
    /// diagnostic even in a composition with no body in it. Resolving the body
    /// first would report 'nothing is driving' for a misspelling."*
    ///
    /// ⚠ **AND THE CHOICE IS NOT ALWAYS THE EVALUATOR'S TO MAKE. MEASURED
    /// 2026-09-06 across all 12 published `ParamSpec`s**, three stances are in
    /// the tree, and which one is right turns on ONE question — *is the roster
    /// reachable where the check would go?*
    ///
    /// 1. **REFUSE AT EVALUATE, because the roster is reachable.** `body.can`
    ///    (`AbilitySet`'s fields are compiled in, so `default()` is a complete
    ///    roster needing no world), `inventory.holds` (`Item::from_dialog_id`
    ///    resolves the spelling or refuses), and `quest.active` (the runtime
    ///    `QuestRegistry`, consulted only once `initialized` — a roster is not
    ///    authoritative until something has filled it).
    /// 2. **DEFER TO CONTENT VALIDATION, stated.** `world.switch_on` rules
    ///    exactly this and says so — *"a MISSPELT id is indistinguishable from
    ///    an unflipped one; that is a content-validation question about authored
    ///    ids, not a runtime one."* `world.flag_set` needs no roster at all: a
    ///    save flag is genuinely free-form, which is the honest escape hatch this
    ///    kind was named for.
    /// 3. **UNEXAMINED** — `encounter.cleared` alone. It reads a save whose
    ///    accessor reconstructs ANY string as an "untouched" row and does not
    ///    say which of (1) or (2) it intends. ⚠ **And it has no authored callers
    ///    to protect: measured 2026-09-06, zero by every road** — no Yarn alias
    ///    is registered for it, no `gated_by` names it (there are exactly two
    ///    authored `gated_by` rows tree-wide, both `intro.ldtk` LockWalls, both
    ///    `bob_field_survey_received`), and no authored condition line mentions
    ///    it. ⇒ A guard modelled on the boss one would certify the empty set, so
    ///    the honest reason this is unexamined is that **nobody has typed a
    ///    misspelling yet** — not that the question was dodged. It becomes a real
    ///    question the day the first author writes one.
    ///
    /// ⚠ **`boss.cleared` BELONGS IN (2) AND I FILED IT WRONG — twice in one
    /// day, on the same condition.** It is the worked example of this whole
    /// entry: authored dialogue asked `boss_cleared("mockingbird")`, a real
    /// BEHAVIOUR id, against a save keyed by PLACEMENT, and three branches could
    /// never open for weeks with no error anywhere. Jon ruled it (decision 57,
    /// 2026-09-05), the callers were migrated, and TWO guards hold it — one
    /// resolving through the production `boss_placement_id` so a behaviour id is
    /// a RED. ⇒ Its roster lives in the authored worlds, which no `&World` here
    /// can reach, so deferring is not a shrug: the check runs where the roster
    /// is.
    ///
    /// ⛔ **I MISCOUNTED THIS TWICE, IN THE SAME DIRECTION, AND THE REASON IS
    /// WORTH MORE THAN THE TALLY.** First draft called `world.switch_on` an
    /// oversight when it had ruled; second called `inventory.holds` unexamined
    /// when it already refuses in its third line and says so in its doc. Both
    /// times I classified from the `ParamSpec` — which is where the KIND is —
    /// without reading the evaluator to its end, which is where the CHOICE is.
    /// ⇒ A param's kind tells you what preparation could not check; only the
    /// evaluator tells you whether anyone checked it anyway.
    ///
    /// ⇒ Stance 2 is not a shrug; it names an owner. The fallback for a roster a
    /// type cannot hold is an authored-integrity guard, which is why this project
    /// already runs several. Adding a `Name` param means picking 1, 2 or 3, and
    /// only the first two are choices.
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
        let outcome = self.answer(world, id, args);
        // ⛔⛤ **ONE DOOR, ONE RECORDER — AND THAT INCLUDES THE REFUSALS THIS
        // FUNCTION ISSUES ITSELF.** A misspelled id and a wrong argument kind
        // never reach a domain, so an evaluator-side recorder would miss
        // exactly the answers whose caller is least able to explain them. See
        // [`ConditionVerdictLog`] for why this is not at the twelve call
        // sites.
        if let Some(log) = world.get_resource::<ConditionVerdictLog>() {
            log.record(ConditionVerdict {
                id: id.clone(),
                args: args.to_vec(),
                outcome: outcome.clone(),
            });
        }
        outcome
    }

    /// The answer itself, with no diagnostic on the path: arity, kinds, then
    /// the owning domain.
    fn answer(&self, world: &World, id: &ConditionId, args: &[AuthoredArg]) -> ConditionOutcome {
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

/// **WHAT WAS ASKED OF THE ENGINE, AND WHAT CAME BACK** — the per-call verdict
/// surface `engine/inspection-diagnostics-and-workbench.md` has carried as the
/// open half of M5 since the `WhyNot` vocabulary landed on 2026-09-02.
///
/// ⛔⛤ **THE STRUCTURE EXISTED AND NOBODY COULD READ IT.** Every production
/// evaluator states a [`WhyNot`] — the term that blocked, the object it names,
/// that object's state — and exactly one consumer published it:
/// `GatedLockWallVerdicts`, keyed by wall id, for the walls of the active room.
/// Every other `no` in the engine was built, returned to its one caller, and
/// dropped. An agent asking *"why did this rule not fire"* about a quest gate,
/// a dialogue branch, an item condition or a boss phase had a structured answer
/// produced on the tick it wanted and no way to see it without a debugger.
///
/// ⭐ **RECORDED AT THE CATALOG, NOT AT THE CALLERS, FOR THE REASON THE CATALOG
/// ALREADY GIVES ABOUT ARITY:** *"an evaluator that had to validate its own
/// arguments would be fifty domains each writing the same four lines, and the
/// day one of them wrote them differently the catalog's schema would stop
/// meaning anything."* The same argument applies to recording, one step out:
/// twelve call sites remembering to log is twelve chances to forget, and the
/// ones that forgot would be invisible. [`ConditionCatalog::evaluate`] is the
/// one door, so it is the one recorder — including for the three
/// `Unanswerable` refusals it issues itself, which are the answers a caller is
/// least able to explain and the ones a misspelled id produces.
///
/// ⚠ **ABSENT BY DEFAULT, AND ABSENCE IS THE OFF SWITCH.** A composition that
/// wants the log inserts it; one that does not pays a resource lookup per
/// evaluation and nothing else. There is no env var and no feature flag,
/// because the question *"is this world recording"* is already answerable by
/// asking the world.
///
/// ⛔ **IT IS NOT SIMULATION STATE AND IT IS NOT REGISTERED FOR ROLLBACK.**
/// Two consequences, both deliberate. Nothing in the simulation may READ it —
/// a rule that branched on what the log remembers would be a rule whose
/// behaviour depends on whether a diagnostic is installed. And its ORDER is
/// not deterministic: conditions evaluate from `&World`, so two systems may
/// ask in parallel and the ring interleaves them however the scheduler ran. A
/// test may assert on WHAT is in the log; asserting on the order of two
/// entries from different systems is asserting on the scheduler.
#[derive(Resource)]
pub struct ConditionVerdictLog {
    entries: std::sync::Mutex<std::collections::VecDeque<ConditionVerdict>>,
    capacity: usize,
}

/// One answered question: the id, the arguments it was asked with, and the
/// outcome the owning domain returned.
#[derive(Clone, Debug, PartialEq)]
pub struct ConditionVerdict {
    pub id: ConditionId,
    pub args: Vec<AuthoredArg>,
    pub outcome: ConditionOutcome,
}

impl ConditionVerdict {
    /// The structure behind a `no`, when the answer was one.
    pub fn why_not(&self) -> Option<&WhyNot> {
        self.outcome.why_not()
    }
}

impl std::fmt::Display for ConditionVerdict {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}(", self.id)?;
        for (i, arg) in self.args.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            match arg {
                AuthoredArg::Reference(id) => write!(f, "{id}")?,
                AuthoredArg::Name(name) => write!(f, "{name:?}")?,
                AuthoredArg::Number(n) => write!(f, "{n}")?,
                AuthoredArg::Truth(t) => write!(f, "{t}")?,
            }
        }
        write!(f, ") => ")?;
        match &self.outcome {
            ConditionOutcome::Satisfied => write!(f, "yes"),
            ConditionOutcome::NotSatisfied(why) => write!(f, "no, {why}"),
            ConditionOutcome::Unanswerable(reason) => write!(f, "unanswerable: {reason}"),
        }
    }
}

impl Default for ConditionVerdictLog {
    fn default() -> Self {
        Self::with_capacity(Self::DEFAULT_CAPACITY)
    }
}

impl ConditionVerdictLog {
    /// ⚠ A BOUND, NOT A BUDGET. A gated wall asks its condition every sync, so
    /// an unbounded log is a leak measured in ticks. This is large enough that
    /// a question asked once during a transition survives the walls asking
    /// theirs for a few frames afterwards, and small enough to be free.
    pub const DEFAULT_CAPACITY: usize = 256;

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            entries: std::sync::Mutex::new(std::collections::VecDeque::new()),
            capacity: capacity.max(1),
        }
    }

    /// ⚠ **A POISONED LOCK IS DROPPED, NOT PROPAGATED.** Another thread
    /// panicking while holding a diagnostic's lock must not turn every
    /// subsequent condition evaluation into a panic — the engine would die of
    /// its own instrument. The record is lost and the simulation continues,
    /// which is the correct ordering of those two costs.
    pub fn record(&self, verdict: ConditionVerdict) {
        let Ok(mut entries) = self.entries.lock() else {
            return;
        };
        if entries.len() == self.capacity {
            entries.pop_front();
        }
        entries.push_back(verdict);
    }

    /// Every verdict still in the ring, oldest first.
    pub fn recent(&self) -> Vec<ConditionVerdict> {
        self.entries
            .lock()
            .map(|entries| entries.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// The last answer to this question, whatever it was asked with.
    pub fn latest_for(&self, id: &ConditionId) -> Option<ConditionVerdict> {
        let entries = self.entries.lock().ok()?;
        entries.iter().rev().find(|v| &v.id == id).cloned()
    }

    /// The last STRUCTURED NO for this question — the M5 answer, without a
    /// debugger.
    ///
    /// ⚠ `None` has three causes and they are different: never asked, last
    /// answered yes, or answered `Unanswerable`. Use [`Self::latest_for`] to
    /// tell them apart; a consumer that reads `None` as *"it passed"* has
    /// made the same collapse [`ConditionOutcome`] is an enum to prevent.
    pub fn why_not_for(&self, id: &ConditionId) -> Option<WhyNot> {
        self.latest_for(id)
            .and_then(|verdict| verdict.why_not().cloned())
    }

    pub fn clear(&self) {
        if let Ok(mut entries) = self.entries.lock() {
            entries.clear();
        }
    }

    pub fn len(&self) -> usize {
        self.entries.lock().map(|e| e.len()).unwrap_or(0)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
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
