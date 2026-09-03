//! Three ways to stage a cast, one projection. (§4.8)
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

use ambition_characters::load_demand::CharacterLoadDemand;

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
// `Eq` dropped when `action_set` arrived: an `ActionSet` carries reach and
// timing in `f32`, so equality on it is `PartialEq` by construction. Nothing
// compares rosters for total equality; `PartialEq` is what the tests use.
#[derive(Debug, Clone, PartialEq)]
pub struct MatchParticipant {
    /// The stable `CharacterDefinitionId` this seat wears.
    ///
    /// typed, so a seat cannot be handed a display name (P0.3). It was a
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
    /// The kit this MATCH gives this fighter, outranking the character's own
    /// catalog row.
    ///
    /// `None` keeps the authored persona, which is every existing roster and the
    /// right answer for a scripted encounter or a boss.
    ///
    /// per SEAT, where `fighter_abilities` is per MATCH, and the difference is the whole
    /// point. An ability is *may this body attack* and levelling it is fairness; a moveset is
    /// *what the attack IS* and levelling it would erase the character.
    pub action_set: Option<ambition_characters::brain::ActionSet>,
    /// The BODY this match gives this fighter, outranking the character's own
    /// catalog row — the movement twin of [`Self::action_set`], and per SEAT for
    /// the same reason.
    ///
    /// ⭐⭐ THIS IS WHERE A FIGHTER SELF DIFFERS FROM A HOME SELF. A catalog
    /// row's `axis_tuning` is that character's feel EVERYWHERE it appears, so a
    /// character that walks around a hub and also fights on a stage cannot state
    /// two gravities there. It states the second one here, and a composition
    /// fills it from whatever it uses to author fighters.
    ///
    /// ⛔ NOT a `MatchBody`, and the distinction is the one that type's own doc
    /// draws: a `MatchBody` is the small set of numbers a MODE owns for every
    /// fighter alive, and gravity is deliberately not among them. This is the
    /// whole body, stated for ONE seat, which is what makes a heavy heavy.
    ///
    /// `None` keeps whatever the character brought, which is every existing
    /// roster.
    pub body: Option<ambition_platformer2d_core::MovementTuning>,
}

impl MatchParticipant {
    pub fn new(character: impl Into<ambition_entity_catalog::CharacterId>) -> Self {
        Self {
            character: character.into(),
            // the first PAD, not "seat zero". A roster that seats two of
            // these without saying otherwise is two people on one controller,
            // and preparation refuses it by name — which is the honest outcome:
            // whoever built that roster has not said who is holding what.
            controller: ControllerBinding::Human {
                source: ambition_input::LocalInputSource::FIRST_PAD,
            },
            team: None,
            action_set: None,
            body: None,
        }
    }

    /// Give this seat a kit for the duration of the match. See
    /// [`MatchParticipant::action_set`].
    pub fn with_action_set(mut self, action_set: ambition_characters::brain::ActionSet) -> Self {
        self.action_set = Some(action_set);
        self
    }

    /// Give this seat a body for the duration of the match. See
    /// [`MatchParticipant::body`].
    pub fn with_body(mut self, body: ambition_platformer2d_core::MovementTuning) -> Self {
        self.body = Some(body);
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

/// Who drives a body. (§4.7)
///
/// Not on the character definition. A definition describes physical limits,
/// vitals, moves, abilities, and hurt behaviour — a BODY — and the same body must
/// be drivable by any of these without becoming four characters. `default_brain`
/// on an identity is the shape this replaces.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ControllerBinding {
    /// A person at this machine drives it, on the source they picked up.
    ///
    /// Feeding one into the other made `PlayerSlot(3)` in a session that only ever opened handles
    /// `0..2`, so that fighter received no input at all . The dense channel is now derived at
    /// preparation; this is only ever the source.
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

    /// The LOCAL INPUT SOURCE this binding occupies, if any.
    ///
    /// A one-human-one-CPU match therefore built a two-handle session whose second handle nothing
    /// ever wrote.
    ///
    /// a participant is not a channel. A CPU is a full participant with a
    /// body, a team and a stock count, and it occupies no channel at all; a
    /// spectator would be a participant with no body. Those are only sayable
    /// once the two counts are allowed to differ.
    ///
    /// and a source is not a channel either — see
    /// [`MatchParticipantRoster::local_channel_plan`], which is what turns these
    /// into dense channels.
    pub fn local_source(&self) -> Option<ambition_input::LocalInputSource> {
        match self {
            Self::Human { source } => Some(*source),
            _ => None,
        }
    }
}

/// Normalized exertion, the only thing locomotion intent may cross the seam
/// as. (§4.7)
///
/// A brain says how hard to try; the BODY turns that into its own acceleration,
/// speed cap, and traction. `patrol_speed` / `chase_speed` / `aggro_radius` /
/// `attack_range` on `ArchetypeSpec` are the standing inconsistency:
/// they are brain or encounter policy that knows absolute world speeds, so a
/// heavy and a light "chasing" move at the same authored number regardless of
/// what their bodies are.
///
/// A heavy at `0.9` and a light at `0.35` sometimes reaching the same absolute
/// speed is not wrong — effort is relative exertion, not a cross-character
/// ranking. Navigation that must reach a point by a deadline is a separate
/// concern and may legitimately use world-space constraints.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct NormalizedEffort(f32);

impl NormalizedEffort {
    pub const IDLE: Self = Self(0.0);
    pub const FULL: Self = Self(1.0);

    /// Clamped to `0.0..=1.0`.
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

/// WHAT A MATCH DROPS, AND HOW OFTEN.
///
/// ⭐ ONE STRUCT rather than three roster fields, because the three are
/// meaningless apart: an interval with no table drops nothing, a table with no
/// interval is never read, and points with neither are scenery. Declaring them
/// together is how "items are on" becomes one statement a rules screen can
/// toggle.
#[derive(Clone, Debug, PartialEq)]
pub struct MatchItemSpawns {
    /// Ticks between drops. `0` disables — the same "off" the `None` above
    /// means, reachable without deleting the table, which is what a rules
    /// screen needs.
    pub every_ticks: u32,
    /// `(held-item id, weight)`. A zero weight is a row switched OFF and is
    /// genuinely unreachable — see `sim_random_weighted`.
    pub table: Vec<(String, u32)>,
    /// Where they land, in world space. ⛔ THE STAGE'S, not the item domain's: a
    /// spawn point is a fact about the geometry somebody authored, and an item
    /// system that guessed one would be a system with an opinion about level
    /// design.
    pub points: Vec<ambition_platformer2d_core::Vec2>,
}

impl MatchItemSpawns {
    /// Is this declaration capable of dropping anything at all?
    ///
    /// Three ways to be off and one to be on, asked in one place so a caller
    /// cannot check two of them and miss the third.
    pub fn active(&self) -> bool {
        self.every_ticks > 0
            && !self.points.is_empty()
            && self.table.iter().any(|(_, weight)| *weight > 0)
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
    /// WHAT THIS MATCH IS PLAYED UNDER — stocks, abilities, the body, the
    /// health pool, the opening ceremony, the clock, the items.
    ///
    /// ⭐⭐ ONE FIELD, AND IT IS THE TYPE THE PREPARED PLAN ALREADY PUBLISHES.
    /// These were EIGHT loose fields here whose only consumer was a transcription
    /// block in `prepare_match` copying them one by one into `MatchRules` — two
    /// representations of one fact, and a new rule cost a field here, a line
    /// there, and an initializer in every roster literal in the tree.
    ///
    /// ⛔ THE ROSTER STILL OWNS THE QUESTION, which is why the field is here at
    /// all rather than the rules being decided by construction: the engine does
    /// not get an opinion about a match's economy. What changed is that it says
    /// so ONCE.
    pub rules: super::prepared_match::MatchRules,
    /// Whether anybody has agreed to seat this roster yet. See
    /// [`RosterSeating`].
    pub seating: RosterSeating,
    /// Which experience published this roster.
    ///
    /// The versus stage's exit rule read *"not on my route and a roster exists → remove it"*,
    /// which was exactly right while it was the only publisher and became "delete the other
    /// game's match" the day the smash demo's character select published one from a different
    /// route.
    ///
    /// `None` is an unowned roster — a fixture, a scripted encounter, anything
    /// with one publisher — and the rule for a consumer is the same either way:
    /// clear what YOU published, not "the roster".
    pub published_by: Option<String>,
}

/// Whether anybody has agreed to seat a [`MatchParticipantRoster`].
///
/// One field, two meanings, and the difference is exactly whether a session is allowed to have
/// an opinion.
///
/// The reconciler compensated by gating on `published_by`.
///
/// `Default` is `Activated`, and that is the load-bearing choice. Every
/// fixture, every `MatchParticipantRoster::of(..)`, every scripted encounter
/// keeps seating exactly as it does today without naming this type at all. Only
/// a route that builds a roster from live devices opts into [`Self::Proposed`],
/// and only that route pays for the extra step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RosterSeating {
    /// Nobody has agreed to seat this roster. `seat_match_participants`
    /// refuses it, so the route that proposed it must activate it first.
    ///
    /// This is what closes the window `status.md` calls *"MECHANISMS DONE, ACTIVATION OPEN"*.
    /// Refusing is the only way to be first.
    Proposed,
    /// This roster may seat. `seat_topology` records which frozen seat
    /// topology agreed to it, when one did.
    ///
    /// `None` there is an honest `None`: nothing had an opinion. The roster, the
    /// GGRS handle count and the per-seat latches must agree about how many
    /// people are playing, and this is the stamp that turns "they should match"
    /// into a question the code can ask.
    Activated { seat_topology: Option<u64> },
}

impl Default for RosterSeating {
    /// `Activated`, and the reason is in [`RosterSeating`]'s own doc.
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
    /// a `Proposed` roster answers `None`, and so does an activated one that
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

    /// Agree to seat this roster, recording the frozen topology that decided
    /// it (`None` when nothing had an opinion).
    ///
    /// one call, not a stamp applied after a separate "allow it" step —
    /// activation IS the agreement, and splitting them would leave a window
    /// where a roster is seatable and unstamped, which is the shape this type
    /// exists to remove.
    ///
    /// unvalidated. Use [`Self::activate_if_seatable`] where an archetype
    /// table is in hand; this exists for the callers that have none (a rebuild
    /// carrying a decision already made, a test).
    pub fn activate(&mut self, seat_topology: Option<u64>) {
        self.seating = RosterSeating::Activated { seat_topology };
    }

    /// Validate every participant AND activate, or neither.
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
    /// the validation is INSIDE the activation, not a call before it.
    /// An authority that needs a FOLLOW-UP CALL has the wrong shape: a separate
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
        // that is the only place it can be.
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

    /// Which local source drives which control channel, in seat order.
    ///
    /// this was a COUNT, and a count is not enough. It answered *"how many
    /// local input channels does this match need"* — the number that sizes a
    /// rollback session and picks solo-versus-couch input assignment — and threw
    /// away the half that says whose controller feeds each one. Everything
    /// downstream then re-derived the missing half from the SOURCE number, which
    /// is sparse: a lobby that seats a CPU first and the human holding pad 1
    /// second produced one handle and a fighter reading `PlayerSlot(1)`, so
    /// nobody could move.
    ///
    /// seat order is the channel order, and that is the whole definition.
    /// Channel `n` is the `n`-th human seat in the roster, whatever source it
    /// holds — so `[CPU, human on pad 1]` is one channel listening to pad 1.
    ///
    /// the ONE place the correspondence is decided. `prepare_match` reads
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

    /// Whether `experience_id` may write over this roster.
    ///
    /// The rule this answers has been learned three times and stated in three
    /// different places, and the third site did not have it: Versus's
    /// reconciler rebuilt Smash's roster with a builder that stamps VERSUS
    /// ownership, so the rebuild transferred the roster; Versus's own teardown
    /// then deleted it, correctly, on a route that was not Versus; and Smash's
    /// match opened with one fighter instead of two.
    ///
    /// an UNOWNED roster is writable. A roster stamped `None` predates the
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

/// What a roster asked for that its composition cannot provide.
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
    /// Return seats whose requested controller policy this composition cannot resolve.
    ///
    /// Character ids are validated separately by `PreparedCharacterRegistry`; this
    /// check uses the same brain-profile authority as match seating. A composition
    /// with no published profile registry cannot satisfy a named profile.
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
    use ambition_characters::load_demand::CharacterLoadDemand;

    /// §4.8's one shared projection. Two semantically different stagings that
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

    /// The policies a composition PUBLISHES, keyed the way assembly keys
    /// them.
    ///
    /// It knows the only one now (P2.18).
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
