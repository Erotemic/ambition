//! **Three ways to stage a cast, one projection.** (§4.8)
//!
//! A room, a match, and a direct startup are semantically different things and
//! keep their own schemas — a room places NPCs at coordinates, a match seats
//! participants on teams, a startup spec names who you begin as. They are NOT
//! variants of one object, and deliberately so: the tempting move is a rich
//! universal `StagedCast` that every subsystem reads, and it would immediately
//! accumulate everyone's fields.
//!
//! What they genuinely share is one thing:
//!
//! ```text
//! RoomStagingPlan          ─┐
//! MatchParticipantRoster   ─┼─→ CharacterLoadDemand { tokens… }
//! DirectStartupSpec        ─┘
//! ```
//!
//! Because the projection is the only shared surface, transformations, summons,
//! assists, alternate forms, and a boss revealed mid-fight all arrive the same
//! way — by demanding more tokens later — with no new staging concept.

use bevy::prelude::Resource;

use super::CharacterLoadDemand;

/// Anything that knows which characters it needs art for.
///
/// The whole contract. An implementor does not learn what materialization is,
/// what an asset profile is, or when the reveal barrier opens.
pub trait StagesCharacters {
    /// Every character token this staging needs.
    fn character_tokens(&self) -> Vec<String>;

    /// Submit those tokens as demand.
    fn project_demand(&self, demand: &mut CharacterLoadDemand) {
        demand.request_all(self.character_tokens());
    }
}

/// What a ROOM stages: placement NPCs, authored enemies, staged actors.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RoomStagingPlan {
    pub placement_characters: Vec<String>,
    pub enemy_names: Vec<String>,
    pub staged_actor_names: Vec<String>,
}

impl StagesCharacters for RoomStagingPlan {
    fn character_tokens(&self) -> Vec<String> {
        self.placement_characters
            .iter()
            .chain(&self.enemy_names)
            .chain(&self.staged_actor_names)
            .cloned()
            .collect()
    }
}

/// One seat in a match. Control assignment lives HERE, not on the character
/// definition (§4.7): a definition describes a body, and who drives it is a
/// session binding.
// ⚠ `Eq` dropped when `action_set` arrived: an `ActionSet` carries reach and
// timing in `f32`, so equality on it is `PartialEq` by construction. Nothing
// compares rosters for total equality; `PartialEq` is what the tests use.
#[derive(Debug, Clone, PartialEq)]
pub struct MatchParticipant {
    /// The stable `CharacterDefinitionId` this seat wears.
    ///
    /// ⭐ **typed, so a seat cannot be handed a display name** (P0.3). It was a
    /// bare `String` for as long as the roster existed, which made
    /// `MatchParticipant::new("Iron Mary", ..)` — a display name where an id
    /// belongs — a thing the compiler had no opinion about.
    pub character: ambition_entity_catalog::CharacterId,
    /// Who drives it. Lives HERE and not on the definition, because the same
    /// character must be playable by a human, a CPU, a replay, and an RL policy
    /// without four definitions.
    pub controller: ControllerBinding,
    /// Free-form team/slot label. The load projection does not interpret it; it
    /// exists so this type is usable as the real roster rather than a stub that
    /// gets replaced.
    pub team: Option<String>,
    /// **The kit this MATCH gives this fighter**, outranking the character's own
    /// catalog row.
    ///
    /// `None` keeps the authored persona, which is every existing roster and the
    /// right answer for a scripted encounter or a boss.
    ///
    /// ⛔ **it exists because a crossover stage borrows a cast.** A Hall NPC's
    /// row says `peaceful` and that is CORRECT where they live — Alice was
    /// authored to stand in a room and talk. Measured 2026-08-05, seven of the
    /// twelve fighters on the smash grid had no melee at all and would have
    /// reached a platform-fighter stage unable to hit anybody.
    ///
    /// ⚠ **per SEAT, where `fighter_abilities` is per MATCH, and the difference
    /// is the whole point.** An ability is *may this body attack* and levelling
    /// it is fairness; a moveset is *what the attack IS* and levelling it would
    /// erase the character. The roster is the one place that can say "on THIS
    /// stage, this character fights like this" without editing the row that
    /// belongs to the game they came from — which is the duplication the
    /// single-registration campaign exists to remove.
    pub action_set: Option<ambition_characters::brain::ActionSet>,
}

impl MatchParticipant {
    pub fn new(character: impl Into<ambition_entity_catalog::CharacterId>) -> Self {
        Self {
            character: character.into(),
            // ⚠ **the first PAD, not "seat zero".** A roster that seats two of
            // these without saying otherwise is two people on one controller,
            // and preparation refuses it by name — which is the honest outcome:
            // whoever built that roster has not said who is holding what.
            controller: ControllerBinding::Human {
                source: ambition_input::LocalInputSource::FIRST_PAD,
            },
            team: None,
            action_set: None,
        }
    }

    /// Give this seat a kit for the duration of the match. See
    /// [`MatchParticipant::action_set`].
    pub fn with_action_set(mut self, action_set: ambition_characters::brain::ActionSet) -> Self {
        self.action_set = Some(action_set);
        self
    }

    pub fn driven_by(mut self, controller: ControllerBinding) -> Self {
        self.controller = controller;
        self
    }

    pub fn on_team(mut self, team: impl Into<String>) -> Self {
        self.team = Some(team.into());
        self
    }
}

/// **Who drives a body.** (§4.7)
///
/// Not on the character definition. A definition describes physical limits,
/// vitals, moves, abilities, and hurt behaviour — a BODY — and the same body must
/// be drivable by any of these without becoming four characters. `default_brain`
/// on an identity is the shape this replaces.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ControllerBinding {
    /// A person at this machine drives it, on the source they picked up.
    ///
    /// ⛔ **this used to say `device_slot: u8`, and the word `slot` was doing
    /// two jobs.** A lobby's source number is SPARSE — pick three pads and
    /// unseat the middle one and the survivors are still 0 and 2, because
    /// renumbering hands somebody the wrong controller — while every seat
    /// number downstream (participant, `PlayerSlot`, GGRS handle) is DENSE.
    /// Feeding one into the other made `PlayerSlot(3)` in a session that only
    /// ever opened handles `0..2`, so that fighter received no input at all
    /// (GPT 5.6, 2026-08-07). The dense channel is now derived at preparation;
    /// this is only ever the source.
    Human {
        source: ambition_input::LocalInputSource,
    },
    /// A brain profile drives it. The profile is AI policy, and it is the only
    /// variant that carries one.
    Cpu { brain_profile: Option<String> },
    /// A recorded control-frame stream drives it.
    Replay,
    /// An external policy drives it (the RL harness).
    Policy { policy_id: Option<String> },
}

impl ControllerBinding {
    /// The AI brain profile, if this binding has one. Human, replay, and policy
    /// bindings deliberately do not: a replay that consulted a brain profile
    /// would stop being a replay.
    pub fn brain_profile(&self) -> Option<&str> {
        match self {
            Self::Cpu { brain_profile } => brain_profile.as_deref(),
            _ => None,
        }
    }

    /// **The LOCAL INPUT SOURCE this binding occupies, if any.**
    ///
    /// ⛔ **the one definition of "who is playing on this machine", and it
    /// needed to be one.** Two authorities used to answer it and disagreed: the
    /// rollback session was sized from `roster.participants.len()` — every seat,
    /// CPUs included — while the frozen input topology counted
    /// `ControllerBinding::Human`, each with a comment claiming to be the
    /// authoritative number. A one-human-one-CPU match therefore built a
    /// two-handle session whose second handle nothing ever wrote.
    ///
    /// ⚠ **a participant is not a channel.** A CPU is a full participant with a
    /// body, a team and a stock count, and it occupies no channel at all; a
    /// spectator would be a participant with no body. Those are only sayable
    /// once the two counts are allowed to differ.
    ///
    /// ⚠ **and a source is not a channel either** — see
    /// [`MatchParticipantRoster::local_channel_plan`], which is what turns these
    /// into dense channels.
    pub fn local_source(&self) -> Option<ambition_input::LocalInputSource> {
        match self {
            Self::Human { source } => Some(*source),
            _ => None,
        }
    }
}

/// **Normalized exertion, the only thing locomotion intent may cross the seam
/// as.** (§4.7)
///
/// A brain says how hard to try; the BODY turns that into its own acceleration,
/// speed cap, and traction. `patrol_speed` / `chase_speed` / `aggro_radius` /
/// `attack_range` on `ArchetypeSpec` are the standing inconsistency:
/// they are brain or encounter policy that knows absolute world speeds, so a
/// heavy and a light "chasing" move at the same authored number regardless of
/// what their bodies are.
///
/// A heavy at `0.9` and a light at `0.35` sometimes reaching the same absolute
/// speed is **not** wrong — effort is relative exertion, not a cross-character
/// ranking. Navigation that must reach a point by a deadline is a separate
/// concern and may legitimately use world-space constraints.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct NormalizedEffort(f32);

impl NormalizedEffort {
    pub const IDLE: Self = Self(0.0);
    pub const FULL: Self = Self(1.0);

    /// Clamped to `0.0..=1.0`. Out-of-range is a caller bug, but silently
    /// admitting `4.0` would let a brain outrun its own body's cap by naming a
    /// bigger number — which is exactly the world-space coupling this removes.
    pub fn new(effort: f32) -> Self {
        Self(if effort.is_finite() {
            effort.clamp(0.0, 1.0)
        } else {
            0.0
        })
    }

    pub fn get(self) -> f32 {
        self.0
    }

    /// Apply this exertion to the body's OWN maximum. The body owns the number;
    /// the brain owns only the fraction.
    pub fn applied_to(self, body_max: f32) -> f32 {
        body_max * self.0
    }
}

/// What a MATCH stages: one character per seat. Several seats may name the SAME
/// character (a mirror match), which the demand set collapses.
/// A `Resource` because a match's roster is SESSION state: it is what the seating
/// pass reads to turn participants into bodies (C4), and what the load projection
/// reads to demand their art. Both are per-session facts with one owner.
#[derive(Resource, Debug, Clone, Default, PartialEq)]
pub struct MatchParticipantRoster {
    pub participants: Vec<MatchParticipant>,
    /// **Whether a seated fighter may act on the tick it appears.**
    ///
    /// A ruleset that opens on a countdown wants `true`: the fighters are reset,
    /// placed and VISIBLE through "3, 2, 1" and none of them — human or CPU —
    /// may decide anything until it ends.
    ///
    /// It lives on the ROSTER because it is a fact about the match, and seating
    /// is the only place that can act on it without a window. The versus stage
    /// suspends control when its countdown begins, and a fighter that seats on
    /// the same tick could take a simulation step before that insert lands: one
    /// tick of a CPU deciding, or of a held direction, before the count starts
    /// (GPT 5.6, 2026-07-29). Applying it AT seating closes the window rather
    /// than narrowing it, which is the difference between a fix and a smaller bug.
    ///
    /// Taken off by whoever put the countdown up — for versus, the `Starting`
    /// arm reaching zero, which is the one place a round goes live.
    pub opens_suspended: bool,
    /// **How long the opening ceremony holds the cast**, in simulation ticks.
    ///
    /// Meaningless without [`Self::opens_suspended`] — there is nothing to hold
    /// — and `0` there means the hold ends on the tick the cast is built, which
    /// is what every roster did before a countdown existed.
    ///
    /// ⚠ **the hold is RELEASED by the engine now**, keyed on this number and
    /// the sim clock. It used to be released by whichever stage happened to own
    /// the ceremony, and the Smash stage owned none — so its release fired the
    /// instant the match went live, and the comment above (*"taken off by
    /// whoever put the countdown up"*) described a countdown that did not exist.
    pub opening_countdown_ticks: u32,
    /// **Whether anybody has agreed to seat this roster yet.** See
    /// [`RosterSeating`].
    pub seating: RosterSeating,
    /// **What every fighter in this match may do, physically.**
    ///
    /// `None` leaves each body with whatever it already had — the right answer
    /// for a roster that is not a fair fight (a scripted encounter, a boss).
    ///
    /// A versus match sets it, because the two seats arrive by different routes:
    /// a SPAWNED seat gets the basic run-and-jump floor from its bundle, while the
    /// ADOPTED primary player brings whatever the session granted it. In the
    /// shipped host that is the sandbox dev kit, so player one could fly and
    /// teleport and the opponent could not.
    ///
    /// It is a rule of the MATCH rather than something seating decides, for the
    /// same reason `opens_suspended` is: the engine does not get an opinion about
    /// what a fighter may do.
    pub fighter_abilities: Option<ambition_platformer2d_core::AbilitySet>,
    /// **How many stocks each fighter starts with, if this match runs on
    /// stocks.** (S4)
    ///
    /// `None` is a match with no stock economy — every existing roster, and the
    /// right answer for a scripted encounter or a boss.
    ///
    /// `Some(n)` declares BOTH halves at once, and that is deliberate: a fighter
    /// gets `FighterStocks::new(n)` AND `DeathPolicy::Unbounded`, because the two
    /// are not independently meaningful. Stocks over a meter that kills at max
    /// are never consulted — the body dies of damage before the world can throw
    /// it out — and an unbounded meter with no stocks is a fighter that cannot
    /// lose. Letting a roster set one without the other would let a match
    /// declare a rule that silently does nothing, which is the failure mode this
    /// whole slice exists to remove.
    ///
    /// On the roster rather than in seating for the same reason as
    /// `fighter_abilities` and `opens_suspended`: the engine does not get an
    /// opinion about what a match's economy is.
    pub fighter_stocks: Option<u32>,
    /// **Which experience published this roster.**
    ///
    /// ⚠ **added because one host now has TWO stages that publish one**, and the
    /// resource is global: it has to exist before the session it describes, so it
    /// cannot be session-scoped. The versus stage's exit rule read *"not on my
    /// route and a roster exists → remove it"*, which was exactly right while it
    /// was the only publisher and became **"delete the other game's match"** the
    /// day the smash demo's character select published one from a different
    /// route. The symptom was not a crash: the stage simply never opened, because
    /// the select screen re-published and re-requested the route every frame
    /// against a resource something else deleted every frame.
    ///
    /// `None` is an unowned roster — a fixture, a scripted encounter, anything
    /// with one publisher — and the rule for a consumer is the same either way:
    /// clear what YOU published, not "the roster".
    pub published_by: Option<String>,
}

/// **Whether anybody has agreed to seat a [`MatchParticipantRoster`].**
///
/// ⛔ **this replaced `seat_topology: Option<u64>`, which meant two things.**
/// `None` was both *"no device discovery went into this roster"* — a fixture, a
/// scripted encounter, a boss, where a frozen topology has nothing to disagree
/// WITH — and *"built from live device discovery on route entry, before any
/// session decided its seating"*, which is a claim awaiting confirmation. One
/// field, two meanings, and the difference is exactly whether a session is
/// allowed to have an opinion.
///
/// The reconciler compensated by gating on `published_by`. That check is still
/// needed and still right — rebuilding somebody else's roster transfers
/// ownership, which cost Smash its second fighter on 2026-08-01 — but it is an
/// OWNERSHIP field, and it was standing in for a lifecycle nobody had written
/// down.
///
/// ⭐ **`Default` is `Activated`, and that is the load-bearing choice.** Every
/// fixture, every `MatchParticipantRoster::of(..)`, every scripted encounter
/// keeps seating exactly as it does today without naming this type at all. Only
/// a route that builds a roster from live devices opts into [`Self::Proposed`],
/// and only that route pays for the extra step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RosterSeating {
    /// **Nobody has agreed to seat this roster.** `seat_match_participants`
    /// refuses it, so the route that proposed it must activate it first.
    ///
    /// This is what closes the window `status.md` calls *"MECHANISMS DONE,
    /// ACTIVATION OPEN"*. It is not a race: seating runs on the SIM schedule and
    /// a route's reconciliation runs in `Update`, and the frame order is
    /// `PreUpdate` → Fixed → `Update`, so a reconciler is structurally incapable
    /// of arriving before the bodies. Refusing is the only way to be first.
    Proposed,
    /// **This roster may seat.** `seat_topology` records which frozen seat
    /// topology agreed to it, when one did.
    ///
    /// `None` there is an honest `None`: nothing had an opinion. The roster, the
    /// GGRS handle count and the per-seat latches must agree about how many
    /// people are playing, and this is the stamp that turns "they should match"
    /// into a question the code can ask (GPT 5.6, 2026-07-29).
    Activated { seat_topology: Option<u64> },
}

impl Default for RosterSeating {
    /// ⭐ **`Activated`, and the reason is in [`RosterSeating`]'s own doc.**
    /// Every roster that existed before this type did seats on publication, and
    /// a `Proposed` default would have made all of them stop.
    fn default() -> Self {
        Self::Activated {
            seat_topology: None,
        }
    }
}

impl RosterSeating {
    /// A roster a session's frozen topology agreed to, at `generation`.
    pub fn activated_at(generation: u64) -> Self {
        Self::Activated {
            seat_topology: Some(generation),
        }
    }

    /// May a seating pass build bodies from this roster?
    pub fn may_seat(self) -> bool {
        matches!(self, Self::Activated { .. })
    }

    /// The frozen topology generation this roster was agreed under, if any.
    ///
    /// ⚠ a `Proposed` roster answers `None`, and so does an activated one that
    /// nothing had an opinion about. A caller that needs to tell those apart is
    /// asking about the LIFECYCLE and should match on the variant.
    pub fn seat_topology(self) -> Option<u64> {
        match self {
            Self::Proposed => None,
            Self::Activated { seat_topology } => seat_topology,
        }
    }
}

impl MatchParticipantRoster {
    /// Which frozen topology generation this roster was agreed under, if any.
    pub fn seat_topology(&self) -> Option<u64> {
        self.seating.seat_topology()
    }

    /// **Agree to seat this roster**, recording the frozen topology that decided
    /// it (`None` when nothing had an opinion).
    ///
    /// ⭐ one call, not a stamp applied after a separate "allow it" step —
    /// activation IS the agreement, and splitting them would leave a window
    /// where a roster is seatable and unstamped, which is the shape this type
    /// exists to remove.
    ///
    /// ⚠ **unvalidated.** Use [`Self::activate_if_seatable`] where an archetype
    /// table is in hand; this exists for the callers that have none (a rebuild
    /// carrying a decision already made, a test).
    pub fn activate(&mut self, seat_topology: Option<u64>) {
        self.seating = RosterSeating::Activated { seat_topology };
    }

    /// **Validate every participant AND activate, or neither.**
    ///
    /// `status.md`'s activation row asks for *"validate every participant,
    /// activate the roster atomically, publish it, start the countdown from
    /// that"*. The validation existed
    /// ([`Self::unsatisfiable_seats`]) and the caller that mattered was
    /// `seat_match_participants` — so the check ran one step AFTER the roster
    /// was live, and a route could activate a match its own composition cannot
    /// fill. Seating then refuses, publishes `MatchSeatingRefused`, and the
    /// stage sits on a roster that will never seat.
    ///
    /// ⭐ **the validation is INSIDE the activation, not a call before it.**
    /// [[an authority that needs a follow-up call]]: a separate
    /// `check_then_activate` leaves a window where a caller did the second half
    /// and not the first, and this repo has paid for that shape more than once.
    /// A caller cannot activate without validating because there is no argument
    /// order in which it can.
    ///
    /// Returns the problems on refusal, so a caller can say something true
    /// instead of retrying forever.
    pub fn activate_if_seatable(
        &mut self,
        // See [`Self::unsatisfiable_seats`]: a seat's policy is PUBLISHED, and
        // since 2026-08-13 that is the only place it can be.
        profiles: Option<&ambition_characters::actor::character_catalog::BrainProfileRegistry>,
        seat_topology: Option<u64>,
    ) -> Result<(), Vec<RosterProblem>> {
        let problems = self.unsatisfiable_seats(profiles);
        if !problems.is_empty() {
            return Err(problems);
        }
        self.activate(seat_topology);
        Ok(())
    }

    pub fn of<I, S>(characters: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<ambition_entity_catalog::CharacterId>,
    {
        Self {
            participants: characters
                .into_iter()
                .map(|c| MatchParticipant::new(c))
                .collect(),
            ..Default::default()
        }
    }

    /// **Which local source drives which control channel**, in seat order.
    ///
    /// ⛔ **this was a COUNT, and a count is not enough.** It answered *"how many
    /// local input channels does this match need"* — the number that sizes a
    /// rollback session and picks solo-versus-couch input assignment — and threw
    /// away the half that says whose controller feeds each one. Everything
    /// downstream then re-derived the missing half from the SOURCE number, which
    /// is sparse: a lobby that seats a CPU first and the human holding pad 1
    /// second produced one handle and a fighter reading `PlayerSlot(1)`, so
    /// nobody could move (GPT 5.6, 2026-08-07).
    ///
    /// ⭐ **seat order is the channel order**, and that is the whole definition.
    /// Channel `n` is the `n`-th human seat in the roster, whatever source it
    /// holds — so `[CPU, human on pad 1]` is one channel listening to pad 1.
    ///
    /// ⚠ **the ONE place the correspondence is decided.** `prepare_match` reads
    /// it rather than counting again, the session is sized from
    /// `plan.channels()`, and the frozen topology stores the plan itself; three
    /// consumers citing one fact instead of three derivations that agree by
    /// inspection.
    pub fn local_channel_plan(&self) -> ambition_input::LocalChannelPlan {
        ambition_input::LocalChannelPlan::from_sources(
            self.participants
                .iter()
                .filter_map(|participant| participant.controller.local_source()),
        )
    }

    /// Stamp the experience that published this roster. See
    /// [`Self::published_by`] for why a global roster needs an owner.
    pub fn published_by(mut self, experience_id: impl Into<String>) -> Self {
        self.published_by = Some(experience_id.into());
        self
    }

    /// Was this roster published by `experience_id`?
    ///
    /// The question a teardown must ask before removing one, and an entry must
    /// ask before seating one.
    pub fn is_published_by(&self, experience_id: &str) -> bool {
        self.published_by.as_deref() == Some(experience_id)
    }

    /// **Whether `experience_id` may write over this roster.**
    ///
    /// ⛔ The rule this answers has been learned three times and stated in three
    /// different places, and the third site did not have it: Versus's
    /// reconciler rebuilt Smash's roster with a builder that stamps VERSUS
    /// ownership, so the rebuild transferred the roster; Versus's own teardown
    /// then deleted it, correctly, on a route that was not Versus; and Smash's
    /// match opened with one fighter instead of two (2026-08-01).
    ///
    /// ⚠ **an UNOWNED roster is writable.** A roster stamped `None` predates the
    /// ownership rule or came from a fixture, and refusing to touch it would
    /// strand it forever with no way to clear it. "Nobody claimed this" and
    /// "somebody else claimed this" are different answers and only the second is
    /// a refusal.
    pub fn is_writable_by(&self, experience_id: &str) -> bool {
        match self.published_by.as_deref() {
            None => true,
            Some(owner) => owner == experience_id,
        }
    }
}

/// **What a roster asked for that its composition cannot provide.**
///
/// One entry per unsatisfiable seat, phrased for a human reading a refusal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RosterProblem {
    /// Which seat, by roster index — the same numbering `MatchSeat` uses.
    pub seat: usize,
    pub detail: String,
}

impl std::fmt::Display for RosterProblem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "seat {}: {}", self.seat, self.detail)
    }
}

impl MatchParticipantRoster {
    /// **Can this roster actually be seated by this composition?** (API 1.0
    /// row (g))
    ///
    /// The campaign's words: *"a composition can declare four participants and
    /// seat two, and no error says so."* This is the seam that says so, and it
    /// is callable BEFORE the roster is published — which is the whole point,
    /// because seating is past the point of no return. `seat_match_participants`
    /// refuses an unresolvable profile, and a refusal at that point is a
    /// half-built match in a release build and a panic in a debug one.
    ///
    /// ⚠ **it checks what the composition can ANSWER, not what the content
    /// means.** A brain profile is looked up in the `CharacterRoster` archetype
    /// table — the table a seated CPU actually consults — because that is the
    /// lookup that silently fell back to a stand-still body twice on
    /// 2026-07-31: the versus stage naming `medium_striker`, an
    /// `ambition_content` row its own composition never registered, and the
    /// smash demo naming `duelist` before it registered one. Both shipped, both
    /// looked composed, and both were fights against a statue.
    ///
    /// Character ids are NOT checked here: `PreparedCharacterRegistry` answers
    /// that, refuses on its own, and asking twice would put two authorities on
    /// one question.
    /// ⛔⛔ **IT ASKS THE AUTHORITY SEATING ASKS, and has twice been asking a
    /// different one.** First it consulted only the archetype table — so the day
    /// Smash published its CPU ladder as real `BrainProfile`s and deleted its
    /// archetype fragment, this reported four perfectly seatable fighters as
    /// unseatable. Then it asked BOTH, which was right while a policy had two
    /// possible homes. Since P2.18 `seat_brain_profile` has ONE arm, and a
    /// checker that still consulted the archetype table would be the other
    /// failure — approving a seat that seating then refuses, which is the shape
    /// `ambition_combat::content_schema` names: the compiler must not approve
    /// what the runtime refuses.
    ///
    /// The check's INTENT — a composition must not declare a seat it cannot
    /// fill — has been right throughout; only its instrument moved.
    ///
    /// ⚠ **the published registry is `Option`** because a composition may have
    /// none. That is a real state rather than an error, and it is now an
    /// ANSWER: a seat naming a policy in a composition that publishes nothing is
    /// unsatisfiable, because seating will refuse it too.
    pub fn unsatisfiable_seats(
        &self,
        // The published controller policies, resolved exactly as
        // `prepared_match::seat_brain_profile` resolves them: this roster's own
        // provider first, then the bare name.
        profiles: Option<&ambition_characters::actor::character_catalog::BrainProfileRegistry>,
    ) -> Vec<RosterProblem> {
        self.participants
            .iter()
            .enumerate()
            .filter_map(|(seat, participant)| {
                let profile = participant.controller.brain_profile()?;
                let reference = ambition_entity_catalog::BrainProfileRef::new(profile);
                let published = profiles.is_some_and(|profiles| {
                    self.published_by
                        .as_deref()
                        .is_some_and(|owner| profiles.get(&reference.resolve_in(owner)).is_some())
                });
                if published {
                    return None;
                }
                let owner = self.published_by.as_deref().unwrap_or("<unpublished>");
                let mut known: Vec<&str> = profiles
                    .map(|profiles| profiles.ids().collect())
                    .unwrap_or_default();
                known.sort_unstable();
                Some(RosterProblem {
                    seat,
                    detail: format!(
                        "asks for brain profile `{profile}`, which this composition \
                         does not publish (it would resolve as `{owner}::{profile}`). \
                         Published policies: {known:?}. \
                         ⚠ an UNPUBLISHED roster resolves nothing — a provider-relative \
                         policy name has no provider to resolve against."
                    ),
                })
            })
            .collect()
    }
}

impl StagesCharacters for MatchParticipantRoster {
    fn character_tokens(&self) -> Vec<String> {
        self.participants
            .iter()
            .map(|p| p.character.to_string())
            .collect()
    }
}

/// What DIRECT STARTUP stages: whoever the session begins as, plus anything the
/// opening scene shows before a room transition has ever run.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DirectStartupSpec {
    pub starting_characters: Vec<String>,
}

impl DirectStartupSpec {
    pub fn of<I, S>(characters: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            starting_characters: characters.into_iter().map(Into::into).collect(),
        }
    }
}

impl StagesCharacters for DirectStartupSpec {
    fn character_tokens(&self) -> Vec<String> {
        self.starting_characters.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::character_runtime::CharacterLoadDemand;

    /// **§4.8's one shared projection.** Two semantically different stagings that
    /// name the same cast must produce the same demand, or "the room worked and the
    /// match did not" becomes a real bug class again.
    #[test]
    fn match_roster_and_room_plan_project_to_the_same_demand() {
        let room = RoomStagingPlan {
            placement_characters: vec!["mary_o".into()],
            enemy_names: vec!["ai_slop".into()],
            staged_actor_names: vec!["solid_snake".into()],
        };
        // The same three characters, seated instead of placed — and one seat is a
        // CPU, which must not change what art is needed.
        let match_roster = MatchParticipantRoster {
            participants: vec![
                MatchParticipant::new("solid_snake").driven_by(ControllerBinding::Cpu {
                    brain_profile: Some("aggressive".into()),
                }),
                MatchParticipant::new("mary_o").on_team("blue"),
                MatchParticipant::new("ai_slop").driven_by(ControllerBinding::Replay),
            ],
            ..Default::default()
        };

        let mut from_room = CharacterLoadDemand::default();
        room.project_demand(&mut from_room);
        let mut from_match = CharacterLoadDemand::default();
        match_roster.project_demand(&mut from_match);

        let room_tokens: Vec<&str> = from_room.pending().collect();
        assert_eq!(
            room_tokens,
            from_match.pending().collect::<Vec<_>>(),
            "a room and a match naming the same cast must demand the same art"
        );
        // Deterministic and deduplicated, whatever order the sources listed them in.
        assert_eq!(room_tokens, vec!["ai_slop", "mary_o", "solid_snake"]);
    }

    /// A mirror match is one decode, not two.
    #[test]
    fn a_mirror_match_demands_one_decode_per_character() {
        let roster = MatchParticipantRoster::of(["mary_o", "mary_o", "mary_o"]);
        let mut demand = CharacterLoadDemand::default();
        roster.project_demand(&mut demand);
        assert_eq!(demand.pending().collect::<Vec<_>>(), vec!["mary_o"]);
    }

    /// §4.8: transformations, summons, assists, alternate forms, and a boss
    /// revealed mid-fight all arrive by demanding MORE tokens later. No new
    /// staging concept, and nothing has to know it was a late arrival.
    #[test]
    fn a_late_arrival_needs_no_new_staging_concept() {
        let mut demand = CharacterLoadDemand::default();
        MatchParticipantRoster::of(["mary_o"]).project_demand(&mut demand);
        // Mary-O grows mid-match; the grown form is its own character (§4.3).
        demand.request("mary_o_tall");
        assert_eq!(
            demand.pending().collect::<Vec<_>>(),
            vec!["mary_o", "mary_o_tall"]
        );
    }

    /// Only a CPU seat carries a brain profile. A replay that consulted one would
    /// stop being a replay.
    #[test]
    fn only_a_cpu_binding_carries_a_brain_profile() {
        assert_eq!(
            ControllerBinding::Cpu {
                brain_profile: Some("aggressive".into())
            }
            .brain_profile(),
            Some("aggressive")
        );
        for binding in [
            ControllerBinding::Human {
                source: ambition_input::LocalInputSource::Pad(1),
            },
            ControllerBinding::Replay,
            ControllerBinding::Policy {
                policy_id: Some("ppo_7".into()),
            },
        ] {
            assert_eq!(binding.brain_profile(), None, "{binding:?}");
        }
    }

    /// Effort is a fraction of the BODY's own maximum, so two bodies at the same
    /// exertion legitimately move at different speeds — and the same body cannot be
    /// made to exceed its cap by a brain naming a bigger number.
    #[test]
    fn effort_scales_the_bodys_own_maximum_and_cannot_exceed_it() {
        let chasing = NormalizedEffort::new(0.9);
        assert_eq!(chasing.applied_to(100.0), 90.0);
        assert_eq!(
            chasing.applied_to(400.0),
            360.0,
            "a faster body goes faster"
        );

        // A brain cannot outrun its body by asking for more than everything.
        assert_eq!(NormalizedEffort::new(4.0), NormalizedEffort::FULL);
        assert_eq!(NormalizedEffort::new(-1.0), NormalizedEffort::IDLE);
        assert_eq!(NormalizedEffort::new(f32::NAN), NormalizedEffort::IDLE);

        // A heavy trying hard and a light loafing may coincide in world speed.
        // That is not a bug: effort is relative exertion, not a ranking (§4.7).
        assert_eq!(
            NormalizedEffort::new(0.9).applied_to(100.0),
            NormalizedEffort::new(0.3).applied_to(300.0)
        );
    }
}

#[cfg(test)]
mod roster_validation_tests {
    use super::*;
    use crate::character_runtime::ControllerBinding;

    /// **The policies a composition PUBLISHES**, keyed the way assembly keys
    /// them.
    ///
    /// ⛔ this built a `CharacterRoster` until 2026-08-13 — an enemy archetype
    /// table — because a seat's controller question had two authorities and this
    /// checker knew the legacy one. It knows the only one now (P2.18).
    fn published(
        keys: &[&str],
    ) -> ambition_characters::actor::character_catalog::BrainProfileRegistry {
        use ambition_characters::actor::character_catalog::{
            parse_catalog, BrainProfileRegistry, CharacterCatalog,
        };
        let rows: String = keys
            .iter()
            .map(|key| format!("\"{PROVIDER}::{key}\": (template: StandStill),"))
            .collect();
        let ron = format!(
            "( autonomous_profiles: {{ {rows} }}, brain_presets: {{}}, \
              action_set_presets: {{}}, characters: {{}} )"
        );
        BrainProfileRegistry::from_catalog_for_test(
            "unused: every name above is already qualified",
            &CharacterCatalog::from_data(parse_catalog(&ron)),
        )
    }

    /// The provider a roster in these fixtures publishes under — a seat's policy
    /// reference resolves in it, so the fixture has to name one to be modelling
    /// production at all.
    const PROVIDER: &str = "fixture_game";

    fn roster_of(characters: [&str; 2]) -> MatchParticipantRoster {
        let mut roster = MatchParticipantRoster::of(characters);
        roster.published_by = Some(PROVIDER.to_string());
        roster
    }

    /// **The bug this seam exists for, twice on 2026-07-31.**
    ///
    /// A CPU seat naming a brain profile the composition never registered.
    /// `spec_for_brain` falls back to a generic row whose brain is
    /// `stand_still`, so the match composes, seats, runs — and the opponent
    /// never moves.
    #[test]
    fn a_cpu_seat_naming_an_unregistered_profile_is_unsatisfiable() {
        let mut roster = roster_of(["fighter_a", "fighter_b"]);
        roster.participants[1] = roster.participants[1]
            .clone()
            .driven_by(ControllerBinding::Cpu {
                brain_profile: Some("medium_striker".into()),
            });

        let problems = roster.unsatisfiable_seats(Some(&published(&["duelist"])));
        assert_eq!(problems.len(), 1, "{problems:?}");
        assert_eq!(problems[0].seat, 1);
        assert!(
            problems[0].detail.contains("medium_striker"),
            "the refusal has to name what was asked for: {}",
            problems[0].detail
        );
    }

    #[test]
    fn a_roster_its_composition_can_seat_reports_nothing() {
        let mut roster = roster_of(["fighter_a", "fighter_b"]);
        roster.participants[1] = roster.participants[1]
            .clone()
            .driven_by(ControllerBinding::Cpu {
                brain_profile: Some("duelist".into()),
            });
        assert!(roster
            .unsatisfiable_seats(Some(&published(&["duelist"])))
            .is_empty());
    }

    /// A HUMAN seat names no controller policy, so it cannot be unsatisfiable
    /// this way — and a check that flagged it would make every couch game
    /// unpublishable. The composition here publishes NOTHING, which is what
    /// makes the claim about the seat rather than about the registry.
    #[test]
    fn a_human_seat_needs_no_published_policy() {
        let mut roster = roster_of(["fighter_a", "fighter_b"]);
        roster.participants[0] =
            roster.participants[0]
                .clone()
                .driven_by(ControllerBinding::Human {
                    source: ambition_input::LocalInputSource::Pad(0),
                });
        roster.participants.truncate(1);
        assert!(roster.unsatisfiable_seats(Some(&published(&[]))).is_empty());
    }
}
