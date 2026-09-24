//! Match preparation resolves all fallible character, brain, and
//! control-authority questions before construction. [`PreparedMatch`] is
//! immutable and activation does no authority lookups, so activation is
//! deterministic and replayable. The plan itself is not rollback state; the
//! active receipt and spawned bodies are.

use ambition_characters::prepared::PreparedCharacterDefinition;
use ambition_characters::prepared::PreparedCharacterRegistry;
use bevy::prelude::*;

use ambition_platformer2d_core::Vec2;

use crate::staging::{ControllerBinding, MatchParticipantRoster, RosterProblem};

/// What will drive a fighter, once the fighter exists.
///
/// "A person" and "a local input channel" are different facts. A remote human
/// is a participant with no local channel; a spectator is a participant with
/// no fighter.
///
/// Only the two kinds this engine can attach are variants. `ControllerBinding`
/// also names `Replay` and `Policy`, but no code binds a driver for them, so
/// preparation refuses them by name. Add a variant together with the code
/// that attaches it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ControlAuthority {
    /// A local person drives it: the source they hold, and the dense channel
    /// the simulation reads them on.
    ///
    /// These are two facts. A roster keeps source numbers (pad 3, keyboard)
    /// stable when a seat empties, but a rollback host needs dense handles
    /// `0..player_count`. The channel is derived only from
    /// [`MatchParticipantRoster::local_channel_plan`], so the number that
    /// sizes the session and the number the fighter reads are the same.
    LocalInput {
        channel: ambition_input::ParticipantId,
        source: ambition_input::LocalInputSource,
    },
    /// A named brain profile drives it. Deterministic, and needs no channel.
    Brain { profile: String },
}

impl ControlAuthority {
    /// The local rollback channel this authority occupies, if any.
    pub fn local_channel(&self) -> Option<ambition_input::ParticipantId> {
        match self {
            Self::LocalInput { channel, .. } => Some(*channel),
            _ => None,
        }
    }

    /// The physical source this authority listens to, if any.
    pub fn local_source(&self) -> Option<ambition_input::LocalInputSource> {
        match self {
            Self::LocalInput { source, .. } => Some(*source),
            _ => None,
        }
    }

    /// Resolve a roster's binding into the authority to attach.
    ///
    /// `channel` is the plan's dense position of this human among the
    /// roster's humans; it makes a sparse source safe to carry. The match
    /// lists every variant with no catch-all: `brain_profile()` is `None` for
    /// `Replay` and `Policy` by design, and a catch-all would drop them
    /// silently.
    fn resolve(
        controller: &ControllerBinding,
        channel: Option<ambition_input::ParticipantId>,
    ) -> Result<Self, String> {
        match controller {
            ControllerBinding::Human { source } => Ok(Self::LocalInput {
                channel: channel.ok_or_else(|| {
                    format!(
                        "plays on {source:?}, which the match's own channel plan does not \
                         list. The plan is built from this roster's human seats, so a seat \
                         missing from it means the two disagree about who is playing."
                    )
                })?,
                source: *source,
            }),
            ControllerBinding::Cpu { brain_profile } => match brain_profile {
                Some(profile) => Ok(Self::Brain {
                    profile: profile.clone(),
                }),
                None => Err(
                    "is driven by a CPU that names no brain profile, so nothing would decide \
                     what it does. A seat with no driver stands still, which is \
                     indistinguishable from a brain that failed to install."
                        .to_owned(),
                ),
            },
            // The engine has no driver to attach for either.
            ControllerBinding::Replay => Err(
                "is driven by a REPLAY, and nothing in this engine attaches a \
                 recorded control stream to a seated fighter yet. The roster \
                 vocabulary is real; the driver is not written."
                    .to_owned(),
            ),
            ControllerBinding::Policy { .. } => Err(
                "is driven by an external POLICY, and nothing in this engine \
                 attaches one to a seated fighter yet. The roster vocabulary is \
                 real; the driver is not written."
                    .to_owned(),
            ),
        }
    }
}

/// One fighter, fully resolved.
#[derive(Clone, Debug)]
pub struct PreparedSeat {
    /// Which seat of the match this is. Stable across a rewind, unlike an
    /// `Entity`, so placement and the view policy key on it.
    pub seat: usize,
    /// The key of the prepared registry. `Borrow<str>` keeps `&str` lookups
    /// working.
    pub character_id: ambition_entity_catalog::CharacterId,
    /// This body's stable identity, distinct from the character it wears.
    ///
    /// A match can be a mirror (two seats, one character). Presentation, the
    /// anti-clump slot board, the steering neighbor index, and the
    /// target/faction maps all key on this id, and
    /// `spawn_dynamic_feature_visuals` dedupes by it. A character key would
    /// merge the two fighters and one would never draw.
    pub feature_id: String,
    /// The owned definition, so activation can read the physical baseline
    /// without asking the registry what it currently says.
    pub definition: PreparedCharacterDefinition,
    /// The owned pre-spawn cluster, built from the character authorities
    /// during preparation and never re-derived.
    pub seed: ambition_body_seed::ActorClusterSeed,
    /// The body box this fighter was resolved to occupy.
    pub body_px: Vec2,
    pub faction: ambition_combat::components::ActorFaction,
    pub team: Option<ambition_combat::targeting::MatchTeam>,
    /// What will drive it. Attached after the body exists; it never changes
    /// how the body is built.
    pub authority: ControlAuthority,
    /// Match-owned kit override for this seat. `None` keeps the character kit.
    pub match_kit: Option<ambition_characters::brain::ActionSet>,
    /// Identity kit resolved once during preparation while the catalog is in scope.
    pub identity_kit: ambition_characters::brain::action_set::IdentityKit,
    /// See [`Self::identity_kit`]. The moveset the same overlay derived.
    pub moveset: ambition_entity_catalog::MovesetContract,
    /// See [`Self::identity_kit`]. The repertoire this seat actually has:
    /// the character's, overlaid with the match's own override.
    pub action_set: ambition_characters::brain::ActionSet,
    /// Effective ability set after match guarantees and permissions are applied.
    /// Kit derivation must use this resolved set.
    pub effective_abilities: Option<ambition_platformer2d_core::AbilitySet>,
    /// The body this seat plays with: the character's own movement feel, or
    /// the one the match supplies (see [`MatchRules::body_over`]).
    ///
    /// Resolved here, like [`Self::effective_abilities`], so the body that is
    /// built and the body a test or UI reads are the same value. It goes to
    /// `grant_prepared_character_body`, the one place a prepared definition
    /// becomes a body.
    pub effective_movement_tuning: Option<ambition_platformer2d_core::MovementTuning>,
}

/// What every fighter in this match plays under.
///
/// Set by the match, not by construction: the engine has no opinion about a
/// match's economy.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MatchRules {
    pub stocks: Option<u32>,
    /// What this match says its fighters may do — a floor and a ceiling, or
    /// `None` to leave every character's own kit alone. See
    /// [`MatchAbilities`](ambition_platformer2d_core::MatchAbilities).
    pub abilities: Option<ambition_platformer2d_core::MatchAbilities>,
    /// The body this match supplies to a fighter whose character authored
    /// none — see
    /// [`MatchParticipantRoster::fighter_body`](super::staging::MatchParticipantRoster::fighter_body),
    /// the field it is carried from, and [`Self::body_over`] for the precedence.
    pub body: Option<ambition_platformer2d_core::MatchBody>,
    /// The pool this match gives every seat, or `None` to keep each
    /// character's own. Carried on
    /// [`MatchParticipantRoster::rules`](super::staging::MatchParticipantRoster::rules),
    /// whose doc holds the reasoning.
    pub health_pool: Option<i32>,
    /// The resources this match gives every seat, for example a Smash Limit
    /// declared empty so no fighter can spend it on the first frame. A seat
    /// holds exactly these; empty means none.
    pub resources: Vec<ambition_resource_spec::ResourceDeclaration>,
    pub opens_suspended: bool,
    /// How long the opening ceremony holds the cast, in simulation ticks.
    ///
    /// `0` means no ceremony: a suspended cast is released on the tick it is
    /// built.
    ///
    /// Ticks, not seconds, for determinism. The release compares against the
    /// sim clock, so a rollback reaches the same answer. A wall-clock timer
    /// could move a peer's release by a frame and diverge the cast.
    pub opening_countdown_ticks: u32,
    /// See [`MatchRoster::time_limit_ticks`](crate::character_runtime::MatchRoster::time_limit_ticks).
    pub time_limit_ticks: u32,
    /// What this match drops and how often, carried from
    /// [`MatchParticipantRoster::item_spawns`](super::staging::MatchParticipantRoster::item_spawns).
    /// `None` = no items.
    pub item_spawns: Option<super::staging::MatchItemSpawns>,
}

/// Where an opening ceremony is. Derived from the clock, never stored.
///
/// There is no countdown timer on purpose: that would add mutable state
/// inside the rollback window. A phase computed from `now - activated_on` is a
/// pure function of the clock and the receipt, so a rewind reaches the same
/// beat.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OpeningPhase {
    /// The cast is held. `beats_remaining` counts down to the release: 3, 2, 1.
    Counting { beats_remaining: u32 },
    /// The hold is over. The tick this first reads `Live` is the release tick.
    Live,
}

impl MatchRules {
    /// The pool a seat gets: the match's, if it declared one (at least 1),
    /// else the character's own.
    pub fn pool_over(&self, authored: i32) -> i32 {
        self.health_pool.map(|pool| pool.max(1)).unwrap_or(authored)
    }

    /// The body a seat plays with: this match's numbers over the body the
    /// fighter brought.
    ///
    /// ```text
    ///   match says nothing   ->  whatever the fighter brought, untouched
    ///   match declares       ->  MatchBody::over(the character's body, else the built one)
    /// ```
    ///
    /// This is not a precedence choice like [`Self::pool_over`]. A body has
    /// many numbers and a mode sets only a few;
    /// [`MatchBody`](ambition_platformer2d_core::MatchBody) holds exactly
    /// those. The character keeps its gait, jump arc, and gravity whether or
    /// not it authored a `MovementTuning`.
    pub fn body_over(
        &self,
        authored: Option<ambition_platformer2d_core::MovementTuning>,
        built: ambition_platformer2d_core::MovementTuning,
    ) -> Option<ambition_platformer2d_core::MovementTuning> {
        match self.body {
            Some(body) => Some(body.over(authored.unwrap_or(built))),
            None => authored,
        }
    }

    /// A stocks match's fighters die to the world, not to the meter. Declared
    /// once so no two seats disagree.
    pub fn death_policy(&self) -> ambition_characters::actor::DeathPolicy {
        if self.stocks.is_some() {
            ambition_characters::actor::DeathPolicy::Unbounded
        } else {
            ambition_characters::actor::DeathPolicy::default()
        }
    }

    /// How many beats the ceremony has, one per counted number ("3, 2, 1").
    /// The ticks are split evenly and the last beat is the shortest, so a
    /// 180-tick countdown at 60Hz is one second per number.
    pub fn opening_beats(&self) -> u32 {
        if self.opening_countdown_ticks == 0 {
            0
        } else {
            OPENING_BEATS
        }
    }

    /// Where the ceremony stands `elapsed` ticks after the cast was built.
    ///
    /// A match with no ceremony is `Live` from tick zero, so this is safe to
    /// call unconditionally.
    pub fn opening_phase(&self, elapsed: u64) -> OpeningPhase {
        let total = u64::from(self.opening_countdown_ticks);
        if elapsed >= total {
            return OpeningPhase::Live;
        }
        let beats = u64::from(self.opening_beats().max(1));
        // Ticks per beat, rounded up, so the last beat is the short one. A
        // "1" that lingers reads as a stall just before release.
        let per_beat = total.div_ceil(beats);
        let elapsed_beats = elapsed / per_beat;
        OpeningPhase::Counting {
            beats_remaining: (beats - elapsed_beats.min(beats - 1)) as u32,
        }
    }

    /// Ticks left on the match clock, or `None` for an untimed match.
    ///
    /// Derived, never counted down. `elapsed` is
    /// `ActiveMatch::ticks_since_activation`, so a rewind recomputes the clock
    /// instead of restoring it, and the clock needs no wire format.
    pub fn time_remaining(&self, elapsed: u64) -> Option<u64> {
        (self.time_limit_ticks > 0)
            .then(|| u64::from(self.time_limit_ticks).saturating_sub(elapsed))
    }

    /// Whether the clock ran out. `false` for an untimed match, so this is safe
    /// to call unconditionally.
    pub fn time_expired(&self, elapsed: u64) -> bool {
        self.time_remaining(elapsed) == Some(0)
    }
}

/// How many numbers an opening ceremony counts: 3, 2, 1.
///
/// A constant, not a rule field: every platform fighter counts three. How
/// long each number holds is the ruleset's `opening_countdown_ticks`.
pub const OPENING_BEATS: u32 = 3;

/// The match, resolved.
#[derive(Resource, Clone, Debug)]
pub struct PreparedMatch {
    seats: Vec<PreparedSeat>,
    rules: MatchRules,
    /// The [`PreparedCharacterRegistry`] generation these seats were resolved
    /// against.
    ///
    /// Activation never re-resolves against a newer generation; that would put
    /// a live authority back inside activation.
    cast_generation: ambition_characters::prepared::CharacterCatalogGeneration,
    /// The frozen seat topology the roster was agreed under, carried so the
    /// activation can cite it. Reading the world at activation would give the
    /// current topology, not the one this plan was built from.
    seat_topology: Option<u64>,
    /// The first `SimTick` this plan may build on.
    ///
    /// When a decision takes effect is part of the decision. The plan is not
    /// rollback state and survives a rewind, but its arrival does not. Without
    /// this tick, a resimulated frame could find the plan already present and
    /// activate one frame earlier than the original run. The tick makes
    /// activation a pure function of the plan and the clock.
    effective_from: u64,
    /// The gameplay session this plan was decided for.
    ///
    /// Roster content cannot answer this: a rematch with the same picks
    /// publishes an identical roster. What changed is the session.
    session: Option<ambition_platformer2d_shared_tangle::lifecycle::SessionScopeId>,
    /// Which experience's roster this plan was built from.
    ///
    /// Copied from [`MatchParticipantRoster::published_by`] in
    /// [`prepare_match`], never authored separately. `PreparedMatch` is a
    /// global resource shared by every experience that stages a cast, so
    /// teardown must check the owner, not remove by type. See
    /// `ExperienceScopeBuilder::releasing_owned`.
    published_by: Option<String>,
}

impl PreparedMatch {
    /// The gameplay session this plan was decided for. See [`Self::session`].
    pub fn session(
        &self,
    ) -> Option<ambition_platformer2d_shared_tangle::lifecycle::SessionScopeId> {
        self.session
    }

    pub fn seats(&self) -> &[PreparedSeat] {
        &self.seats
    }

    /// Count the seats originally prepared for a ruleset side.
    ///
    /// Match membership is frozen by the prepared plan and remains stable after
    /// eliminated bodies despawn. Side labels use the same `stocks::side_label`
    /// rule as stocks outcome resolution.
    pub fn seats_on_side(&self, side: &str) -> usize {
        self.seats
            .iter()
            .filter(|seat| {
                ambition_combat::stocks::side_label(seat.seat, seat.team.as_ref()) == side
            })
            .count()
    }

    /// The first `SimTick` this plan may build on. See [`Self::effective_from`].
    pub fn effective_from(&self) -> u64 {
        self.effective_from
    }

    pub fn rules(&self) -> &MatchRules {
        &self.rules
    }

    /// The cast generation this plan was prepared against.
    pub fn cast_generation(&self) -> ambition_characters::prepared::CharacterCatalogGeneration {
        self.cast_generation
    }

    /// Whether the live cast generation differs from this plan's frozen
    /// generation. A staleness diagnostic only; activation never re-resolves.
    pub fn cast_moved_on(
        &self,
        live: ambition_characters::prepared::CharacterCatalogGeneration,
    ) -> bool {
        self.cast_generation != live
    }

    /// The frozen seat topology this plan was agreed under, if anything had an
    /// opinion when the roster was built.
    pub fn seat_topology(&self) -> Option<u64> {
        self.seat_topology
    }

    /// Whether this plan was built from `experience_id`'s roster.
    ///
    /// An unowned plan answers `false` to everyone. That leaks instead of
    /// deleting, which is the safe direction: a leak costs one stale plan, a
    /// wrong delete costs another game's live match.
    pub fn is_published_by(&self, experience_id: &str) -> bool {
        self.published_by.as_deref() == Some(experience_id)
    }

    /// Build a plan with only an owner, for teardown tests. The fields stay
    /// private so production has one builder ([`prepare_match`]).
    #[doc(hidden)]
    pub fn for_test_published_by(experience_id: Option<&str>) -> Self {
        Self {
            seats: Vec::new(),
            rules: MatchRules::default(),
            cast_generation: ambition_characters::prepared::CharacterCatalogGeneration::default(),
            seat_topology: None,
            // Tick zero is reached by every clock, and no session matches a
            // bare test world.
            effective_from: 0,
            session: None,
            published_by: experience_id.map(str::to_owned),
        }
    }

    /// Which local source drives which channel in this match.
    ///
    /// Not `seats().len()`: a CPU is a participant but not a channel, and two
    /// CPUs need no channels. `plan.channels()` is the handle count; the rest
    /// of the plan says whose controller each handle listens to.
    pub fn channel_plan(&self) -> ambition_input::LocalChannelPlan {
        ambition_input::LocalChannelPlan::from_sources(
            self.seats
                .iter()
                .filter_map(|seat| seat.authority.local_source()),
        )
    }
}

/// What a composition could not answer about a roster.
///
/// Published so a consumer can tell the player, and tests can read it.
/// Present only while an unpreparable roster stands.
#[derive(Resource, Debug, Clone, PartialEq, Eq)]
pub struct MatchPreparationProblems {
    pub problems: Vec<RosterProblem>,
}

impl std::fmt::Display for MatchPreparationProblems {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let joined = self
            .problems
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("; ");
        f.write_str(&joined)
    }
}

/// The body box a fighter gets when its character authored none.
///
/// A small placeholder on purpose: a generous box would hide a character whose
/// art never resolved.
const SEAT_BODY_PX: Vec2 = Vec2::new(30.0, 48.0);

/// Resolve a roster into a match, or say why it cannot be one.
///
/// Reports every problem, not the first, so a lobby can fix everything at
/// once.
#[allow(clippy::too_many_arguments)]
pub fn prepare_match(
    roster: &MatchParticipantRoster,
    registry: &PreparedCharacterRegistry,
    catalog: &ambition_characters::actor::character_catalog::CharacterCatalog,
    authored_sheets: &ambition_sprite_sheet::character::sheets::AuthoredSheets,
    // The controller policies this composition published: the only source of
    // a seat's policy.
    profiles: Option<&ambition_characters::actor::character_catalog::BrainProfileRegistry>,
    centre: Vec2,
    // The first `SimTick` the plan may build on (see
    // `PreparedMatch::effective_from`). Preparation runs in `Update`, after
    // the frame's simulation, so the caller passes the next tick.
    effective_from: u64,
    // Which gameplay session this plan is for (see `PreparedMatch::session`).
    session: Option<ambition_platformer2d_shared_tangle::lifecycle::SessionScopeId>,
    // Whether the session also lowers its own home avatar. Preparation uses
    // it to refuse a local seat in that case (see the seat loop).
    home_body_spawns_a_body: bool,
) -> Result<PreparedMatch, MatchPreparationProblems> {
    // The roster's own rules, not a field-by-field copy.
    let rules = roster.rules.clone();
    let death_policy = rules.death_policy();

    let mut problems: Vec<RosterProblem> = Vec::new();
    let mut seats: Vec<PreparedSeat> = Vec::new();

    // Who plays on what, decided once by the roster. Every seat reads its
    // channel from this plan, so the GGRS session size, the frozen topology,
    // and the fighter's `PlayerSlot` use the same number.
    let plan = roster.local_channel_plan();
    // One controller cannot drive two fighters. Refuse before the match
    // exists: the second seat's channel would open but never receive input,
    // and the fighter would stand still with no error. Two seats defaulting
    // to pad 0 is the common cause.
    for repeated in plan.repeated_sources() {
        let seat = roster
            .participants
            .iter()
            .enumerate()
            .filter(|(_, participant)| participant.controller.local_source() == Some(repeated))
            .map(|(index, _)| index)
            .next_back()
            .unwrap_or(0);
        problems.push(RosterProblem {
            seat,
            detail: format!(
                "plays on {repeated:?}, which another seat in this match also claims. \
                 One controller cannot drive two fighters: the second seat would open a \
                 rollback channel nothing ever writes, and stand still for the whole \
                 match."
            ),
        });
    }
    let mut humans_seated: u8 = 0;

    for (index, participant) in roster.participants.iter().enumerate() {
        let mut seat_problem = |detail: String| {
            problems.push(RosterProblem {
                seat: index,
                detail,
            });
        };

        // Only registered characters can be built.
        let Some(definition) = registry.get(participant.character.as_str()) else {
            seat_problem(format!(
                "asks for character `{}`, which this composition has not REGISTERED. \
                 ⚠ a catalog row is not a registration: the catalog says what a \
                 character IS and `register_character` is what makes one \
                 buildable, and a surface that offers a fighter must filter on \
                 the second.",
                participant.character
            ));
            continue;
        };

        // The dense channel for this seat, taken from the plan in seat order
        // and checked against it, so nobody sits on another person's
        // controller.
        let channel = participant.controller.local_source().and_then(|source| {
            let channel = ambition_input::ParticipantId(humans_seated);
            humans_seated = humans_seated.saturating_add(1);
            (plan.source_for(channel) == Some(source)).then_some(channel)
        });
        let authority = match ControlAuthority::resolve(&participant.controller, channel) {
            Ok(authority) => authority,
            Err(detail) => {
                seat_problem(detail);
                continue;
            }
        };

        // Two claimants on one local channel: refuse here, before any entity
        // exists. A match experience must declare
        // `InitialBodyPolicy::NoInitialBody`.
        if home_body_spawns_a_body && authority.local_channel().is_some() {
            seat_problem(
                "asks for a LOCAL control channel in a session that also lowers \
                 its own home avatar, so two bodies would claim the same \
                 channel. A match experience must declare \
                 `InitialBodyPolicy::NoInitialBody` — the match owns its whole \
                 cast, and there is no privileged avatar for a seat to share a \
                 channel with."
                    .to_string(),
            );
            continue;
        }

        // A CPU's profile must name a policy this composition published.
        if let ControlAuthority::Brain { profile } = &authority {
            if seat_brain_profile(
                profile,
                roster.published_by.as_deref(),
                &definition.provider,
                profiles,
            )
            .is_none()
            {
                // List only published policies: `seat_brain_profile` reads
                // nothing else.
                let mut published: Vec<&str> =
                    profiles.map(|p| p.ids().collect()).unwrap_or_default();
                published.sort_unstable();
                seat_problem(format!(
                    "asks for brain profile `{profile}`, which this composition \
                     does not publish. Published policies: {published:?}. \
                     ⚠ a bare name resolves in the MATCH's provider first and the \
                     CHARACTER's second, so `{profile}` alone never means another \
                     game's policy of the same name."
                ));
                continue;
            }
        }

        // The authored brain for the seed. A local-input seat uses `Passive`
        // because its real driver is attached later; a body whose driver
        // never arrives stands still.
        let seed_brain = match &authority {
            ControlAuthority::Brain { profile } => {
                ambition_entity_catalog::placements::CharacterBrain::Custom(profile.clone())
            }
            _ => ambition_entity_catalog::placements::CharacterBrain::Passive,
        };

        let (at, facing) = seat_placement(index, centre);

        // The authored physical identity, read through `PhysicalBaseline`
        // like the exploration player.
        let baseline = ambition_body_seed::PhysicalBaseline::of(definition);
        // `hint_px` is only a hint: for a named catalog character,
        // `ActorClusterSeed::new_peaceful_npc_in` resizes to the authored
        // sprite's collision, and `seat.body_px` reads that size back.
        let hint_px = baseline.explicit_size().unwrap_or(SEAT_BODY_PX);
        let aabb = ambition_platformer2d_core::Aabb::new(at, hint_px / 2.0);
        // Key the body on the seat, not the character. A mirror match has two
        // bodies with one character, and every id-keyed index in the actor
        // runtime (`entity_to_id`, the anti-clump board's `requests`,
        // `faction_by_id`, `target_entity_by_id`, `ActorIdentity`) would merge
        // them. The art still resolves from the character.
        let body_id = format!("{}#seat{index}", participant.character);
        // Build character-first, not from an archetype creature with the
        // character patched over it. The controller's policy is resolved here
        // and passed in as a value.
        let profile = match &authority {
            ControlAuthority::Brain { profile } => seat_brain_profile(
                profile,
                roster.published_by.as_deref(),
                &definition.provider,
                profiles,
            ),
            _ => None,
        }
        .unwrap_or_default();
        // The character as one value, with the match's three overrides below.
        let mut body = definition.seat_blueprint(ambition_platformer2d_core::MAX_RUN_SPEED);
        // The match's pool, or the character's own. A crossover match needs one
        // pool: a percent against four games' authored maxima would mean four
        // different things.
        body.max_health = rules.pool_over(baseline.max_health_over(1));
        // The policy is the match's decision. A human seat's body has none.
        body.autonomous_profile = Some(profile);
        // A match seat is never a practice target: such a body is skipped by
        // targeting and nobody could fight it.
        body.practice_target = false;
        let mut seed = ambition_body_seed::ActorClusterSeed::new_character_in(
            authored_sheets,
            catalog,
            body_id.clone(),
            body,
            aabb,
            seed_brain,
            // A stage has no authored patrol paths; a seat is driven.
            &[],
        );
        // The seed's own pool applies to a character that authored none.
        seed.health =
            ambition_characters::actor::BodyHealth::new(ambition_characters::actor::Health::new(
                rules.pool_over(baseline.max_health_over(seed.health.health.max.max(1))),
            ))
            .with_policy(death_policy);
        // The match's resources, set at their declared start before the body
        // exists, so its first frame holds them.
        match ambition_platformer2d_core::resources::ActorResources::declared(&rules.resources) {
            Ok(bank) => seed.body.0.resources = bank,
            Err(error) => {
                seat_problem(format!("the match's resource declarations are invalid: {error:?}"));
                continue;
            }
        }
        // The authored knockback weight, set on the seed so `into_components`
        // projects it onto `CombatTuning` from the first frame. A character
        // that authors none keeps its roster archetype's weight.
        if let Some(weight) = baseline.knockback_weight() {
            seed.config.tuning.weight = weight;
        }
        seed.kin.facing = facing;

        // Use the size the seed resolved. The small placeholder applies only
        // to a character that authors no size.
        let body_px = seed.kin.size;
        // Every "what can this body do" question below reads this, so the kit,
        // abilities, and AI capability read agree.
        let seat_abilities = effective_abilities(definition.abilities, rules.abilities);
        // The kit this seat wears, from the same compiler as spawn and
        // re-wear, so a seated fighter and a room fighter agree.
        let worn = ambition_combat::worn_kit::WornKit::resolve(
            catalog,
            Some(registry),
            participant.character.as_str(),
            participant.action_set.as_ref(),
        );
        // See `MatchRules::body_over`.
        let built_body = seed
            .config
            .tuning
            .movement
            .body_tuning(seed.config.tuning.max_run_speed);
        seats.push(PreparedSeat {
            seat: index,
            feature_id: body_id,
            character_id: participant.character.clone(),
            definition: definition.clone(),
            seed,
            body_px,
            // Every seat has a team, so match relationships never fall back to
            // faction, and faction keeps its world meaning.
            faction: ambition_combat::components::ActorFaction::Player,
            team: Some(team_for(index, participant.team.as_ref())),
            authority,
            match_kit: participant.action_set.clone(),
            identity_kit: worn.identity,
            action_set: worn.action_set,
            effective_abilities: seat_abilities,
            // Resolve the body beside the verbs, or the stage can grant a verb
            // whose window never opens. See `MatchRules::body_over`. The base
            // is the seed's own tuning, so a mode's numbers change only what
            // they name. The seat's own body outranks the character's: a
            // catalog row's feel applies everywhere that character appears.
            // See `MatchParticipant::body`.
            effective_movement_tuning: rules
                .body_over(participant.body.or(definition.movement_tuning), built_body),
            moveset: worn.moveset,
        });
    }

    if !problems.is_empty() {
        return Err(MatchPreparationProblems { problems });
    }

    // Camera framing is not decided here. The match declares its cast
    // (`FramedCast`) once the bodies exist, and the camera resolver frames
    // them when nothing local drives one.

    Ok(PreparedMatch {
        seats,
        rules,
        cast_generation: registry.generation(),
        seat_topology: roster.seat_topology(),
        effective_from,
        session,
        published_by: roster.published_by.clone(),
    })
}

/// Resolve fighter abilities before construction. Character-authored verbs
/// are the base; match rules grant more and/or cap the result:
/// `effective = (authored ∪ granted) ∩ permitted`.
pub fn effective_abilities(
    authored: Option<ambition_platformer2d_core::AbilitySet>,
    rules: Option<ambition_platformer2d_core::MatchAbilities>,
) -> Option<ambition_platformer2d_core::AbilitySet> {
    match rules {
        Some(rules) => Some(rules.apply(authored)),
        None => authored,
    }
}

/// Where seat `index` stands, given the stage centre, and which way it looks.
///
/// Symmetric about `centre`, alternating sides, facing inward. Public so a
/// rules layer can reset a fighter between rounds with the same geometry.
pub fn seat_placement(index: usize, centre: Vec2) -> (Vec2, f32) {
    /// Half the horizontal gap between two seated fighters, in world pixels.
    /// Wide enough that neither starts inside the other's authored silhouette.
    const SEAT_SPREAD_PX: f32 = 96.0;
    let side = if index % 2 == 0 { -1.0 } else { 1.0 };
    let rank = (index / 2) as f32;
    let x = centre.x + side * (SEAT_SPREAD_PX + rank * SEAT_SPREAD_PX * 0.5);
    // Facing points back toward the centre: a left-hand seat looks right.
    (Vec2::new(x, centre.y), -side)
}

/// Resolve a seat's match team without changing the character's authored world
/// faction. An authored team is preserved; otherwise each seat gets its own team,
/// producing free-for-all relationships.
pub fn team_for(index: usize, authored: Option<&String>) -> ambition_combat::targeting::MatchTeam {
    ambition_combat::targeting::MatchTeam::new(
        authored
            .cloned()
            .unwrap_or_else(|| format!("seat {}", index + 1)),
    )
}

/// Resolve a seat's brain profile from namespaced policy registries.
///
/// Match-provider policy wins, then character-provider policy. Bare keys are
/// never matched globally, so one provider cannot accidentally satisfy another
/// provider's seat policy.
pub fn seat_brain_profile(
    key: &str,
    match_provider: Option<&str>,
    provider: &str,
    profiles: Option<&ambition_characters::actor::character_catalog::BrainProfileRegistry>,
) -> Option<ambition_characters::brain::BrainProfile> {
    profiles.and_then(|profiles| {
        let reference = ambition_entity_catalog::BrainProfileRef::new(key);
        // No global bare-key fallback: provider ownership is part of identity,
        // so one game's `duelist` cannot drive another's fighter. An
        // already-qualified name is handled by `resolve_in`.
        match_provider
            .and_then(|owner| profiles.get(&reference.resolve_in(owner)))
            .or_else(|| profiles.get(&reference.resolve_in(provider)))
            .copied()
    })
}
