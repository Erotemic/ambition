//! The bounded log and the tick explainer.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

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
///  write-only from the simulation, read-only from a tool. Nothing in the
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
    /// The execution identity of the frame being stepped, when the host knows.
    ///
    /// Same reason as `tick`: a domain publishing a movement fact does not know
    /// whether the host is replaying history over this frame, and a fact that
    /// guessed `Original` would make a resimulated tick indistinguishable from
    /// its original — the exact thing `Execution` exists to prevent.
    frame: Option<(Execution, u32)>,
    /// Which attempt at the current tick. See [`ExecutionKey::attempt`].
    attempt: u32,
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
            frame: None,
            attempt: 0,
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

    /// Stamp the execution identity every subsequent fact belongs to. Called by
    /// the host, once per frame, BEFORE any publisher runs.
    pub fn set_frame(&mut self, execution: Execution, generation: u32) -> &mut Self {
        self.set_frame_attempt(execution, generation, 0)
    }

    /// The frame, including WHICH ATTEMPT at this tick. See
    /// [`ExecutionKey::attempt`].
    pub fn set_frame_attempt(
        &mut self,
        execution: Execution,
        generation: u32,
        attempt: u32,
    ) -> &mut Self {
        self.frame = Some((execution, generation));
        self.attempt = attempt;
        self
    }

    pub fn frame(&self) -> Option<(Execution, u32)> {
        self.frame
    }

    /// Which attempt at the current tick facts will be stamped with.
    pub fn attempt(&self) -> u32 {
        self.attempt
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
        // A publisher that set its own execution keeps it; everything else
        // inherits the host's, which is the only place that knows.
        if let Some((execution, generation)) = self.frame {
            if fact.execution == Execution::default() && fact.generation == 0 {
                fact.execution = execution;
                fact.generation = generation;
                // The attempt rides with the execution it belongs to: a
                // publisher that stamped its own execution has said which frame
                // it means, and overwriting half of that would produce a key
                // neither side chose.
                fact.attempt = self.attempt;
            }
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

    /// Explain the latest execution for `subject` on `tick`, including global facts.
    ///
    /// Generation and original/resimulated execution are distinct keys. Multiple
    /// resimulations within one generation still group together because no attempt
    /// counter is recorded; use [`Self::explanations`] to retrieve every keyed execution.
    pub fn explain(&self, tick: u64, subject: &SubjectKey) -> Explanation {
        self.explanations(tick, subject)
            .pop()
            .unwrap_or_else(|| Explanation {
                tick,
                subject: subject.clone(),
                key: ExecutionKey::default(),
                truncated: self.dropped > 0,
                facts: Vec::new(),
            })
    }

    /// Every execution of this tick, oldest first.
    ///
    /// A rewound tick appears twice — once original, once resimulated — and a
    /// tick reached in two sessions appears once per generation. They are
    /// separate answers because they are separate moments.
    pub fn explanations(&self, tick: u64, subject: &SubjectKey) -> Vec<Explanation> {
        let mut groups: BTreeMap<ExecutionKey, Vec<CausalFact>> = BTreeMap::new();
        for fact in self.facts.iter().filter(|fact| fact.tick == tick) {
            let mine = match &fact.subject {
                Some(other) => other == subject,
                // A fact about the WORLD explains every body on that tick.
                None => true,
            };
            if !mine {
                continue;
            }
            groups
                .entry(ExecutionKey {
                    generation: fact.generation,
                    execution: fact.execution,
                    attempt: fact.attempt,
                })
                .or_default()
                .push(fact.clone());
        }
        groups
            .into_iter()
            .map(|(key, facts)| Explanation {
                tick,
                subject: subject.clone(),
                key,
                truncated: self.dropped > 0,
                facts,
            })
            .collect()
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

/// Which EXECUTION of a tick an explanation is about.
///
/// A tick number alone does not identify a moment: frames restart at zero on
/// every session, and a rollback host runs the same tick more than once.
/// Ordering is `(generation, execution)`, so `Original` sorts before
/// `Resimulated` within a generation and the latest generation sorts last —
/// which is what makes `explain` return the most recent one.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct ExecutionKey {
    pub generation: u32,
    pub execution: Execution,
    /// Which attempt at this tick. Rollback can execute one tick several
    /// times inside a generation, and those attempts can produce different
    /// facts — which is exactly when somebody looks. Without this they grouped
    /// into one explanation and the inspector could not say which attempt
    /// produced a result
    ///
    /// `0` is the original execution; the host bumps it per rollback.
    pub attempt: u32,
}

/// The answer to one question: this subject, this tick, ONE execution of it.
#[derive(Clone, Debug, PartialEq)]
pub struct Explanation {
    pub tick: u64,
    pub subject: SubjectKey,
    /// Which execution of `tick` these facts came from.
    pub key: ExecutionKey,
    /// The ring wrapped, so an earlier link may be missing. Reported, so a
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
    /// One of the program's required questions. `None` when NOTHING was
    /// recorded — honest, because a composition with no rollback host has no
    /// answer and reporting `Original` would be a claim nobody made.
    ///
    /// It reads the explanation's OWN key rather than its first fact: every
    /// fact in one explanation shares an execution by construction, which is
    /// what stops this being "whichever fact happened to sort first".
    pub fn execution(&self) -> Option<Execution> {
        (!self.facts.is_empty()).then_some(self.key.execution)
    }

    /// The lifecycle generation these facts were recorded in.
    pub fn generation(&self) -> Option<u32> {
        (!self.facts.is_empty()).then_some(self.key.generation)
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
            "why {} changed on tick {} (generation {}, {}) — {} fact(s)\n",
            self.subject,
            self.tick,
            self.key.generation,
            self.key.execution,
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
