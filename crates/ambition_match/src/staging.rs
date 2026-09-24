//! Three ways to stage a cast, one projection. (§4.8)
//!
//! A room, a match, and a direct startup are different things and keep their
//! own schemas: a room places NPCs at coordinates, a match seats participants
//! on teams, a startup spec names who you begin as. They are not variants of
//! one object. A universal `StagedCast` would collect every subsystem's fields.
//!
//! What they share is one projection:
//!
//! ```text
//! RoomStagingPlan          ─┐
//! MatchParticipantRoster   ─┼─→ CharacterLoadDemand { tokens… }
//! DirectStartupSpec        ─┘
//! ```
//!
//! Because the projection is the only shared surface, transformations,
//! summons, assists, alternate forms, and a boss revealed mid-fight all
//! arrive the same way: by demanding more tokens later.

use bevy::prelude::Resource;

use ambition_characters::load_demand::CharacterLoadDemand;

/// Anything that knows which characters it needs art for.
///
/// That is the whole contract. An implementor does not learn about
/// materialization, asset profiles, or the reveal barrier.
pub trait StagesCharacters {
    /// Every character token this staging needs.
    fn character_tokens(&self) -> Vec<String>;

    /// Submit those tokens as demand.
    fn project_demand(&self, demand: &mut CharacterLoadDemand) {
        demand.request_all(self.character_tokens());
    }
}

/// What a room stages: placement NPCs, authored enemies, staged actors.
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

// Not `Eq`: `ActionSet` holds `f32` reach and timing.
/// One seat in a match. Control assignment lives here, not on the character
/// definition (§4.7): a definition describes a body, and who drives it is a
/// session binding.
#[derive(Debug, Clone, PartialEq)]
pub struct MatchParticipant {
    /// The stable `CharacterDefinitionId` this seat wears.
    ///
    /// Typed, so a seat cannot receive a display name (P0.3).
    pub character: ambition_entity_catalog::CharacterId,
    /// Who drives it. The same character must be playable by a human, a CPU,
    /// a replay, and an RL policy without four definitions.
    pub controller: ControllerBinding,
    /// Free-form team/slot label. The load projection does not interpret it.
    pub team: Option<String>,
    /// The kit this match gives this fighter, over the character's own
    /// catalog row. `None` keeps the authored persona (the right answer for a
    /// scripted encounter or a boss).
    ///
    /// Per seat, while `fighter_abilities` is per match. An ability is "may
    /// this body attack", and leveling it is fairness. A moveset is "what the
    /// attack is", and leveling it would erase the character.
    pub action_set: Option<ambition_characters::brain::ActionSet>,
    /// The body this match gives this fighter, over the character's own
    /// catalog row. The movement twin of [`Self::action_set`], per seat for
    /// the same reason.
    ///
    /// This is where a fighter self differs from a home self. A catalog row's
    /// `axis_tuning` applies everywhere the character appears, so a character
    /// that walks a hub and fights on a stage states its fighter body here.
    ///
    /// Not a `MatchBody`: that is the few numbers a mode sets for every
    /// fighter, and it excludes gravity. This is the whole body for one seat.
    ///
    /// `None` keeps whatever the character brought.
    pub body: Option<ambition_platformer2d_core::MovementTuning>,
}

impl MatchParticipant {
    pub fn new(character: impl Into<ambition_entity_catalog::CharacterId>) -> Self {
        Self {
            character: character.into(),
            // The first pad, not "seat zero". Two such seats with no other
            // binding are two people on one controller, and preparation
            // refuses that by name.
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
/// Not on the character definition. A definition describes a body (limits,
/// vitals, moves, abilities, hurt behavior), and any of these must be able to
/// drive it without making four characters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ControllerBinding {
    /// A person at this machine drives it, on the source they picked up.
    ///
    /// Only the source. The dense channel is derived at preparation; using
    /// the source as a channel can name a handle the session never opened.
    Human {
        source: ambition_input::LocalInputSource,
    },
    /// A brain profile drives it. The only variant that carries AI policy.
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

    /// The local input source this binding occupies, if any.
    ///
    /// A participant is not a channel: a CPU is a full participant with no
    /// channel, and a spectator would be a participant with no body. A source
    /// is not a channel either; see
    /// [`MatchParticipantRoster::local_channel_plan`], which makes dense
    /// channels.
    pub fn local_source(&self) -> Option<ambition_input::LocalInputSource> {
        match self {
            Self::Human { source } => Some(*source),
            _ => None,
        }
    }
}

/// Normalized exertion: the only form locomotion intent may take across the
/// seam. (§4.7)
///
/// A brain says how hard to try; the body turns that into its own
/// acceleration, speed cap, and traction. `patrol_speed` / `chase_speed` /
/// `aggro_radius` / `attack_range` on `ArchetypeSpec` are a known
/// inconsistency: they use absolute world speeds, so a heavy and a light
/// "chase" at the same speed.
///
/// Effort is relative, not a cross-character ranking. Navigation that must
/// reach a point by a deadline is a separate concern and may use world-space
/// constraints.
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

    /// Apply this exertion to the body's own maximum. The body owns the number;
    /// the brain owns only the fraction.
    pub fn applied_to(self, body_max: f32) -> f32 {
        body_max * self.0
    }
}

/// What a match drops, and how often.
///
/// One struct, not three roster fields: the parts mean nothing apart. An
/// interval with no table drops nothing, and a table with no interval is
/// never read. Together, "items on" is one statement a rules screen can
/// toggle.
#[derive(Clone, Debug, PartialEq)]
pub struct MatchItemSpawns {
    /// Ticks between drops. `0` disables drops without deleting the table.
    pub every_ticks: u32,
    /// `(held-item id, weight)`. A zero weight is switched off and cannot be
    /// drawn (see `sim_random_weighted`).
    pub table: Vec<(String, u32)>,
    /// Where items land, in world space. These belong to the stage: spawn
    /// points are level geometry, not item-system choices.
    pub points: Vec<ambition_platformer2d_core::Vec2>,
}

impl MatchItemSpawns {
    /// Whether this declaration can drop anything. Checks all three
    /// conditions in one place.
    pub fn active(&self) -> bool {
        self.every_ticks > 0
            && !self.points.is_empty()
            && self.table.iter().any(|(_, weight)| *weight > 0)
    }
}

/// What a match stages: one character per seat. Several seats may name the
/// same character (a mirror match); the demand set collapses them.
/// A `Resource` because the roster is session state with one owner: the
/// seating pass reads it to make bodies (C4), and the load projection reads it
/// to demand art.
#[derive(Resource, Debug, Clone, Default, PartialEq)]
pub struct MatchParticipantRoster {
    pub participants: Vec<MatchParticipant>,
    /// What this match is played under: stocks, abilities, body, health pool,
    /// opening ceremony, clock, items. The same `MatchRules` type the prepared
    /// plan publishes, so there is one representation. The roster owns the
    /// rules; the engine has no opinion about a match's economy.
    pub rules: crate::prepared::MatchRules,
    /// Whether anybody has agreed to seat this roster yet. See
    /// [`RosterSeating`].
    pub seating: RosterSeating,
    /// Which experience published this roster.
    ///
    /// Consumers must clear only a roster they published, never "the roster":
    /// several experiences can publish one. `None` is an unowned roster (a
    /// fixture, a scripted encounter).
    pub published_by: Option<String>,
}

/// Whether anybody has agreed to seat a [`MatchParticipantRoster`].
///
/// A roster built from live devices is `Proposed` until a session agrees to
/// it. `Default` is `Activated`, so fixtures, `MatchParticipantRoster::of(..)`,
/// and scripted encounters seat as before without naming this type. Only a
/// route that builds a roster from live devices opts into [`Self::Proposed`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RosterSeating {
    /// Nobody has agreed to seat this roster. `seat_match_participants`
    /// refuses it, so the proposing route must activate it first.
    Proposed,
    /// This roster may seat. `seat_topology` records which frozen seat
    /// topology agreed to it, or `None` if nothing had an opinion. The roster,
    /// the GGRS handle count, and the per-seat latches must agree on how many
    /// people play; this stamp lets code check that.
    Activated { seat_topology: Option<u64> },
}

impl Default for RosterSeating {
    /// `Activated`; see [`RosterSeating`]. A `Proposed` default would stop
    /// every roster that seats on publication.
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
    /// A `Proposed` roster answers `None`, and so does an activated roster
    /// that nothing had an opinion about. Match on the variant to tell them
    /// apart.
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
    /// Activation is the agreement, in one call, so a roster is never
    /// seatable but unstamped. Unvalidated: use
    /// [`Self::activate_if_seatable`] when a profile registry is available.
    /// This is for callers without one (a rebuild of a decision already made,
    /// a test).
    pub fn activate(&mut self, seat_topology: Option<u64>) {
        self.seating = RosterSeating::Activated { seat_topology };
    }

    /// Validate every participant AND activate, or neither.
    ///
    /// The validation is inside the activation, so a route cannot activate a
    /// match its composition cannot fill. If the check ran only in
    /// `seat_match_participants`, seating would refuse after the roster was
    /// live and the stage would wait on a roster that never seats. There is no
    /// call order that activates without validating.
    ///
    /// Returns the problems on refusal, so a caller can report them instead of
    /// retrying forever.
    pub fn activate_if_seatable(
        &mut self,
        // See [`Self::unsatisfiable_seats`]: a seat's policy must be published.
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
    /// Seat order is channel order: channel `n` is the `n`-th human seat,
    /// whatever source it holds. So `[CPU, human on pad 1]` is one channel
    /// that listens to pad 1. A count is not enough: source numbers are
    /// sparse, and re-deriving channels from them can leave a fighter on a
    /// handle the session never opened.
    ///
    /// This is the one place the mapping is decided. `prepare_match` reads
    /// it, the session is sized from `plan.channels()`, and the frozen
    /// topology stores the plan.
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

    /// Whether `experience_id` published this roster. A teardown asks this
    /// before removing a roster, and an entry asks it before seating one.
    pub fn is_published_by(&self, experience_id: &str) -> bool {
        self.published_by.as_deref() == Some(experience_id)
    }

    /// Whether `experience_id` may write over this roster.
    ///
    /// A roster owned by another experience is not writable. For example, a
    /// rebuild that stamps a different owner transfers the roster, and the new
    /// owner's teardown can then delete it on the old owner's route.
    ///
    /// An unowned roster is writable. A `None` roster comes from a fixture or
    /// predates ownership, and refusing it would strand it. "Nobody claimed
    /// this" is not a refusal; "somebody else claimed this" is.
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
    /// Seats whose requested controller policy this composition cannot
    /// resolve.
    ///
    /// Character ids are validated separately by `PreparedCharacterRegistry`.
    /// This uses the same brain-profile authority as match seating. With no
    /// published registry, a named profile cannot be satisfied.
    pub fn unsatisfiable_seats(
        &self,
        // The published controller policies, resolved in this roster's own
        // provider. An unpublished roster resolves nothing.
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

/// What direct startup stages: whoever the session begins as, plus anything
/// the opening scene shows before any room transition.
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

    /// §4.8's one shared projection: two stagings that name the same cast
    /// must produce the same demand.
    #[test]
    fn match_roster_and_room_plan_project_to_the_same_demand() {
        let room = RoomStagingPlan {
            placement_characters: vec!["mary_o".into()],
            enemy_names: vec!["ai_slop".into()],
            staged_actor_names: vec!["solid_snake".into()],
        };
        // The same three characters, seated instead of placed. One seat is a
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
    /// revealed mid-fight arrive by demanding more tokens later, with no new
    /// staging concept.
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

    /// Effort is a fraction of the body's own maximum. Two bodies at the same
    /// effort move at different speeds, and a brain cannot push a body past
    /// its cap.
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
    use crate::staging::ControllerBinding;

    /// The policies a composition publishes, keyed the way assembly keys them
    /// (P2.18).
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

    /// The provider these fixture rosters publish under. A seat's policy
    /// reference resolves in it, as in production.
    const PROVIDER: &str = "fixture_game";

    fn roster_of(characters: [&str; 2]) -> MatchParticipantRoster {
        let mut roster = MatchParticipantRoster::of(characters);
        roster.published_by = Some(PROVIDER.to_string());
        roster
    }

    /// A CPU seat naming a brain profile the composition never registered.
    /// Without this check, `spec_for_brain` falls back to a `stand_still`
    /// row, and the opponent never moves.
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

    /// A human seat names no controller policy, so it cannot fail this check;
    /// flagging it would block every couch game. The composition publishes
    /// nothing, so the test is about the seat, not the registry.
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
