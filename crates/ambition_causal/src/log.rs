//! The bounded log and the tick explainer.

use std::collections::{BTreeSet, VecDeque};

use crate::fact::{CausalDomain, CausalFact, Execution, FactId, SubjectKey};

/// What to retain. Instrumentation must be bounded and selectively enabled, so
/// this is a first-class choice rather than a compile flag somebody forgets.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RecordingPolicy {
    /// Record nothing. The default in a shipped build.
    Off,
    /// Record every domain.
    All,
    /// Record only these domains — the usual answer, because the expensive
    /// domains are rarely the ones under investigation.
    Domains(BTreeSet<CausalDomain>),
}

impl Default for RecordingPolicy {
    fn default() -> Self {
        Self::Off
    }
}

impl RecordingPolicy {
    pub fn only(domains: impl IntoIterator<Item = CausalDomain>) -> Self {
        Self::Domains(domains.into_iter().collect())
    }

    pub fn admits(&self, domain: CausalDomain) -> bool {
        match self {
            Self::Off => false,
            Self::All => true,
            Self::Domains(set) => set.contains(&domain),
        }
    }

    pub fn is_off(&self) -> bool {
        matches!(self, Self::Off)
    }
}

/// A bounded ring of facts.
///
/// ⛔ **write-only from the simulation, read-only from a tool.** Nothing in the
/// sim may branch on a fact: the ring is lossy, it is not rewound by a rollback
/// host, and a decision that consulted it would desync the moment history was
/// replayed.
#[derive(Clone, Debug)]
pub struct CausalLog {
    facts: VecDeque<CausalFact>,
    capacity: usize,
    next_id: u64,
    policy: RecordingPolicy,
    /// The simulation tick the scope owner is currently stepping, when it knows.
    ///
    /// Pure code five hops below the ECS does NOT know the sim tick and must
    /// not guess one — a brain's own decision counter is not the world's clock,
    /// and two domains stamping different clocks cannot be joined. So the scope
    /// owner stamps it here and `record` applies it, which is the one place
    /// that genuinely has the answer.
    tick: Option<u64>,
    /// Facts refused because the ring was full. Reported rather than silent —
    /// an explanation missing its first link because the buffer wrapped must
    /// not look like an explanation whose first link never happened.
    dropped: u64,
}

impl Default for CausalLog {
    fn default() -> Self {
        Self::with_capacity(Self::DEFAULT_CAPACITY)
    }
}

impl CausalLog {
    /// Enough for a few seconds of a busy fight at 60 Hz, which is the window
    /// an investigation actually looks at.
    pub const DEFAULT_CAPACITY: usize = 4096;

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            facts: VecDeque::with_capacity(capacity.min(1024)),
            capacity: capacity.max(1),
            next_id: 0,
            policy: RecordingPolicy::default(),
            tick: None,
            dropped: 0,
        }
    }

    pub fn set_policy(&mut self, policy: RecordingPolicy) -> &mut Self {
        self.policy = policy;
        self
    }

    pub fn policy(&self) -> &RecordingPolicy {
        &self.policy
    }

    /// Stamp the simulation tick every subsequent fact belongs to. Called by
    /// whoever is stepping the simulation, once per tick.
    pub fn set_tick(&mut self, tick: u64) -> &mut Self {
        self.tick = Some(tick);
        self
    }

    pub fn tick(&self) -> Option<u64> {
        self.tick
    }

    pub fn is_recording(&self) -> bool {
        !self.policy.is_off()
    }

    /// Record a fact and return its id, or `None` when the policy excludes it.
    /// The id is what a follow-on fact names as its `cause`.
    pub fn record(&mut self, mut fact: CausalFact) -> Option<FactId> {
        if !self.policy.admits(fact.domain) {
            return None;
        }
        if let Some(tick) = self.tick {
            fact.tick = tick;
        }
        fact.id = FactId(self.next_id);
        self.next_id += 1;
        if self.facts.len() == self.capacity {
            self.facts.pop_front();
            self.dropped += 1;
        }
        let id = fact.id;
        self.facts.push_back(fact);
        Some(id)
    }

    pub fn len(&self) -> usize {
        self.facts.len()
    }

    pub fn is_empty(&self) -> bool {
        self.facts.is_empty()
    }

    /// How many facts fell off the front. Non-zero means an explanation may be
    /// truncated, and [`Explanation::truncated`] says so.
    pub fn dropped(&self) -> u64 {
        self.dropped
    }

    pub fn clear(&mut self) {
        self.facts.clear();
        self.dropped = 0;
    }

    pub fn facts(&self) -> impl Iterator<Item = &CausalFact> {
        self.facts.iter()
    }

    /// **Why did this subject change on this tick?**
    ///
    /// Everything about `subject` on `tick`, plus facts on that tick with no
    /// subject at all — a session rebase or a rules change is about the world
    /// and still explains a body.
    ///
    /// ⚠ **a missing domain is not an error.** A movement-only composition
    /// publishes no combat facts, and its explanation is simply shorter. An
    /// explainer that required every domain would be unusable in exactly the
    /// small games this engine is for.
    pub fn explain(&self, tick: u64, subject: &SubjectKey) -> Explanation {
        let facts: Vec<CausalFact> = self
            .facts
            .iter()
            .filter(|fact| fact.tick == tick)
            .filter(|fact| match &fact.subject {
                Some(other) => other == subject,
                None => true,
            })
            .cloned()
            .collect();
        Explanation {
            tick,
            subject: subject.clone(),
            truncated: self.dropped > 0,
            facts,
        }
    }

    /// Every subject that has a fact on this tick — for a tool offering a list
    /// to pick from.
    pub fn subjects_on(&self, tick: u64) -> Vec<SubjectKey> {
        let mut subjects: Vec<SubjectKey> = self
            .facts
            .iter()
            .filter(|fact| fact.tick == tick)
            .filter_map(|fact| fact.subject.clone())
            .collect();
        subjects.sort();
        subjects.dedup();
        subjects
    }

    /// A deterministic dump of everything retained, for a CI artifact.
    ///
    /// Ordered by (tick, fact id) — insertion order within a tick, which is the
    /// order things actually happened, and stable across runs because the ids
    /// are assigned sequentially by this log rather than by a hash.
    pub fn dump(&self) -> String {
        let mut facts: Vec<&CausalFact> = self.facts.iter().collect();
        facts.sort_by_key(|fact| (fact.tick, fact.id));
        let mut out = String::new();
        if self.dropped > 0 {
            out.push_str(&format!(
                "# {} earlier fact(s) fell off the ring (capacity {})\n",
                self.dropped, self.capacity
            ));
        }
        for fact in facts {
            out.push_str(&fact.render());
            out.push('\n');
        }
        out
    }
}

/// The answer to one question: this subject, this tick.
#[derive(Clone, Debug, PartialEq)]
pub struct Explanation {
    pub tick: u64,
    pub subject: SubjectKey,
    /// The ring wrapped, so an earlier link may be missing. **Reported**, so a
    /// gap caused by the buffer is distinguishable from a gap caused by the
    /// simulation.
    pub truncated: bool,
    facts: Vec<CausalFact>,
}

impl Explanation {
    pub fn facts(&self) -> &[CausalFact] {
        &self.facts
    }

    pub fn is_empty(&self) -> bool {
        self.facts.is_empty()
    }

    /// Facts from one domain. A domain with nothing to say yields nothing —
    /// which is what "tolerate missing domains" means concretely.
    pub fn domain(&self, domain: CausalDomain) -> impl Iterator<Item = &CausalFact> {
        self.facts.iter().filter(move |fact| fact.domain == domain)
    }

    /// The first fact of a kind.
    pub fn first(&self, kind: &str) -> Option<&CausalFact> {
        self.facts.iter().find(|fact| fact.kind() == kind)
    }

    pub fn all(&self, kind: &'static str) -> impl Iterator<Item = &CausalFact> {
        self.facts.iter().filter(move |fact| fact.kind() == kind)
    }

    /// Was this tick original execution or rollback resimulation?
    ///
    /// One of the program's required questions. `None` when no domain said —
    /// which is honest: a composition with no rollback host has no answer,
    /// and reporting `Original` would be a claim nobody made.
    pub fn execution(&self) -> Option<Execution> {
        self.facts.first().map(|fact| fact.execution)
    }

    /// The lifecycle generation these facts were recorded in.
    pub fn generation(&self) -> Option<u32> {
        self.facts.first().map(|fact| fact.generation)
    }

    /// The causal chain ending at `fact`, oldest first — each link is the
    /// `cause` of the next.
    pub fn chain_to(&self, fact: &CausalFact) -> Vec<&CausalFact> {
        let mut chain = Vec::new();
        let mut cursor = Some(fact.id);
        // Bounded by the fact count: a cause cycle would otherwise hang a
        // debugger, and a malformed publisher is exactly when you are debugging.
        let mut budget = self.facts.len() + 1;
        while let Some(id) = cursor {
            budget -= 1;
            if budget == 0 {
                break;
            }
            let Some(found) = self.facts.iter().find(|candidate| candidate.id == id) else {
                break;
            };
            chain.push(found);
            cursor = found.cause;
        }
        chain.reverse();
        chain
    }

    /// One screen: every fact, in the order they were recorded.
    pub fn render(&self) -> String {
        let mut out = format!(
            "why {} changed on tick {} — {} fact(s)\n",
            self.subject,
            self.tick,
            self.facts.len()
        );
        if self.truncated {
            out.push_str("  ⚠ the log ring wrapped; an earlier link may be missing\n");
        }
        if self.facts.is_empty() {
            out.push_str(
                "  (no domain published anything — either nothing happened, or no domain that \
                 would know is recording)\n",
            );
        }
        for fact in &self.facts {
            out.push_str("  ");
            out.push_str(&fact.render());
            out.push('\n');
        }
        out
    }
}
