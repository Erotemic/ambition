//! The fact vocabulary — enough identity to CORRELATE, and no more.
//!
//! A fact nobody can join to another fact is a log line with extra steps. Every
//! field here exists because some question in the program's list cannot be
//! answered without it.

use std::fmt;

/// Which domain published this. Open, like a schema id: a capability mints its
/// own without editing an enum.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CausalDomain(pub &'static str);

impl fmt::Display for CausalDomain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

/// The domains the first slice covers. Named constants rather than an enum so
/// a game's own domain sits beside them as an equal.
pub mod domains {
    use super::CausalDomain;

    /// Physical device → participant → semantic action.
    pub const INPUT: CausalDomain = CausalDomain("input");
    /// A brain or ruleset choosing.
    pub const BRAIN: CausalDomain = CausalDomain("brain");
    /// An authored move being scored, begun, advanced or cancelled.
    pub const MOVESET: CausalDomain = CausalDomain("moveset");
    /// Locomotion, forces, support, contacts.
    pub const MOVEMENT: CausalDomain = CausalDomain("movement");
    /// Hit acceptance/rejection, damage, knockback.
    pub const DAMAGE: CausalDomain = CausalDomain("damage");
    /// Spawn, death, stocks, respawn, elimination.
    pub const LIFECYCLE: CausalDomain = CausalDomain("lifecycle");
    /// Session generation, rebases, original-vs-resimulated execution.
    pub const ROLLBACK: CausalDomain = CausalDomain("rollback");
}

/// Which body a fact is about.
///
/// A raw `Entity` will not do: indices are recycled and `to_bits` ordering is
/// a trap this repo has already been bitten by. A subject is whatever STABLE
/// identity the publishing domain has — a `SimId` string, a seat, or a bare
/// index when that is genuinely all there is — and the explainer joins on
/// equality without caring which.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SubjectKey {
    /// A stable simulation id (`SimId`).
    Sim(String),
    /// A seat / participant slot, for facts about a person rather than a body.
    Seat(u8),
    /// An opaque index, for a domain that has no stable id yet. Say so —
    /// this variant is a recorded API leak, not a design.
    Unstable(u64),
}

impl fmt::Display for SubjectKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Sim(id) => write!(f, "sim:{id}"),
            Self::Seat(slot) => write!(f, "seat:{slot}"),
            Self::Unstable(index) => write!(f, "entity:{index}"),
        }
    }
}

/// Whether this tick was run for the first time or replayed.
///
/// One of the program's required questions, and the reason the old text trace
/// could not answer it: under a rollback host a resimulated frame decides again
/// and logs again, and two identical lines are indistinguishable from one
/// decision made twice.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum Execution {
    #[default]
    Original,
    /// The host is replaying confirmed history over this tick.
    Resimulated,
}

impl fmt::Display for Execution {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Original => "original",
            Self::Resimulated => "resim",
        })
    }
}

/// A fact's position in the log, so another fact can name it as its cause.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FactId(pub u64);

/// A structured value. Deliberately small: a causal fact records the numbers a
/// question turns on, not a serialized world.
#[derive(Clone, Debug, PartialEq)]
pub enum FactValue {
    Bool(bool),
    Int(i64),
    Float(f64),
    Text(String),
    /// A prepared content identity (`ambition:character/goblin`). Quote the
    /// compiler's identity; never reconstruct a name from a runtime internal.
    Content(String),
}

impl fmt::Display for FactValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Bool(v) => write!(f, "{v}"),
            Self::Int(v) => write!(f, "{v}"),
            Self::Float(v) => write!(f, "{v:.2}"),
            Self::Text(v) => f.write_str(v),
            Self::Content(v) => write!(f, "{v}"),
        }
    }
}

impl From<bool> for FactValue {
    fn from(v: bool) -> Self {
        Self::Bool(v)
    }
}
impl From<i64> for FactValue {
    fn from(v: i64) -> Self {
        Self::Int(v)
    }
}
impl From<i32> for FactValue {
    fn from(v: i32) -> Self {
        Self::Int(v.into())
    }
}
impl From<u32> for FactValue {
    fn from(v: u32) -> Self {
        Self::Int(v.into())
    }
}
impl From<f32> for FactValue {
    fn from(v: f32) -> Self {
        Self::Float(v.into())
    }
}
impl From<f64> for FactValue {
    fn from(v: f64) -> Self {
        Self::Float(v)
    }
}
impl From<String> for FactValue {
    fn from(v: String) -> Self {
        Self::Text(v)
    }
}
impl From<&str> for FactValue {
    fn from(v: &str) -> Self {
        Self::Text(v.to_string())
    }
}

/// What happened, in fields rather than in prose.
#[derive(Clone, Debug, PartialEq)]
pub struct FactDetail {
    /// The fact kind, e.g. `"movement_verb_chosen"`. A `&'static str` so a
    /// capability mints its own; stable, because tools and tests match on it.
    pub kind: &'static str,
    /// One line for a human. Never the only place a value appears — if a
    /// tool would have to parse this to learn something, that something belongs
    /// in `fields`.
    pub summary: String,
    pub fields: Vec<(&'static str, FactValue)>,
}

impl FactDetail {
    pub fn new(kind: &'static str, summary: impl Into<String>) -> Self {
        Self {
            kind,
            summary: summary.into(),
            fields: Vec::new(),
        }
    }

    pub fn field(mut self, name: &'static str, value: impl Into<FactValue>) -> Self {
        self.fields.push((name, value.into()));
        self
    }

    pub fn get(&self, name: &str) -> Option<&FactValue> {
        self.fields
            .iter()
            .find(|(field, _)| *field == name)
            .map(|(_, value)| value)
    }
}

/// One published fact.
#[derive(Clone, Debug, PartialEq)]
pub struct CausalFact {
    pub id: FactId,
    /// The simulation tick it is about.
    pub tick: u64,
    pub domain: CausalDomain,
    pub subject: Option<SubjectKey>,
    /// The seat, when the fact is about a person's input rather than a body.
    pub participant: Option<u8>,
    /// The fact this one followed from, when the publisher knows. This is what
    /// makes a CHAIN rather than a list.
    pub cause: Option<FactId>,
    /// The lifecycle/session generation. Frames restart at zero on every
    /// session, so a tick number alone cannot tell a restart from a rewind —
    /// the same reason `RollbackHealth` had to start carrying one.
    pub generation: u32,
    pub execution: Execution,
    /// Which ATTEMPT at this tick. Rollback can execute one tick more than
    /// once within a generation, and two attempts can produce different facts —
    /// that is the whole reason to look. Without this the inspector groups them
    /// into one explanation and cannot say which attempt produced a result
    ///
    /// `0` is the original execution. The HOST bumps it, for the same reason it
    /// stamps the tick: a domain five hops down cannot know a rewind happened.
    pub attempt: u32,
    /// The authored content that supplied the active value, when there is one.
    pub content: Option<String>,
    pub detail: FactDetail,
}

impl CausalFact {
    /// Start a fact. `id` is assigned by the log on record.
    pub fn new(domain: CausalDomain, tick: u64, detail: FactDetail) -> Self {
        Self {
            id: FactId(0),
            tick,
            domain,
            subject: None,
            participant: None,
            cause: None,
            generation: 0,
            execution: Execution::Original,
            attempt: 0,
            content: None,
            detail,
        }
    }

    pub fn about(mut self, subject: SubjectKey) -> Self {
        self.subject = Some(subject);
        self
    }

    pub fn by_participant(mut self, slot: u8) -> Self {
        self.participant = Some(slot);
        self
    }

    pub fn caused_by(mut self, cause: FactId) -> Self {
        self.cause = Some(cause);
        self
    }

    pub fn in_generation(mut self, generation: u32) -> Self {
        self.generation = generation;
        self
    }

    pub fn executed(mut self, execution: Execution) -> Self {
        self.execution = execution;
        self
    }

    /// Which attempt at this tick produced the fact. Stamped by the host.
    pub fn on_attempt(mut self, attempt: u32) -> Self {
        self.attempt = attempt;
        self
    }

    pub fn from_content(mut self, content: impl Into<String>) -> Self {
        self.content = Some(content.into());
        self
    }

    pub fn field(mut self, name: &'static str, value: impl Into<FactValue>) -> Self {
        self.detail = self.detail.field(name, value);
        self
    }

    pub fn kind(&self) -> &'static str {
        self.detail.kind
    }

    pub fn get(&self, field: &str) -> Option<&FactValue> {
        self.detail.get(field)
    }

    /// One line, for a terminal or a dump.
    pub fn render(&self) -> String {
        let subject = self
            .subject
            .as_ref()
            .map(ToString::to_string)
            .unwrap_or_else(|| "-".into());
        let fields: Vec<String> = self
            .detail
            .fields
            .iter()
            .map(|(name, value)| format!("{name}={value}"))
            .collect();
        let mut line = format!(
            "t{} g{} {} [{}] {} {}",
            self.tick, self.generation, self.execution, self.domain, subject, self.detail.summary
        );
        if !fields.is_empty() {
            line.push_str(&format!("  ({})", fields.join(" ")));
        }
        if let Some(content) = &self.content {
            line.push_str(&format!("  <- {content}"));
        }
        line
    }
}
