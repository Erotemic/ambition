//! Match preparation resolves all fallible character, brain, and control-authority questions
//! before construction. [`PreparedMatch`] is immutable and activation performs no authority
//! lookups, so activation is deterministic and replayable. The plan itself is not rollback state;
//! the active receipt and spawned bodies are.

use ambition_characters::prepared::PreparedCharacterDefinition;
use ambition_characters::prepared::PreparedCharacterRegistry;
use bevy::prelude::*;

use ambition_platformer2d_core::Vec2;

use super::{ControllerBinding, MatchParticipantRoster, RosterProblem};

/// What will drive a fighter, once the fighter exists.
///
/// "a person" and "a local input channel" are not one fact. Conflating
/// them is how a CPU seat came to size a rollback session: the GGRS handle count
/// was `participants.len()` and the frozen input topology counted
/// `ControllerBinding::Human`, each with a comment claiming to be the
/// authoritative number. A remote human would be a participant with no local
/// channel; a spectator is a participant with no fighter. Neither is
/// expressible while one word means both.
///
/// exactly the two kinds this engine can ATTACH, and no more.
/// `ControllerBinding` also names `Replay` and `Policy`; those are real roster
/// vocabulary and there is no code anywhere that binds a driver for either. A
/// variant here for each would be a set with no members — the shape this repo
/// keeps mistaking for rigour — so preparation REFUSES them by name instead,
/// and whoever wires a replay seat adds the variant with the code that attaches
/// it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ControlAuthority {
    /// A local person drives it: the SOURCE they are holding, and the dense
    /// CHANNEL the simulation reads them on.
    ///
    /// two fields because they are two facts, and collapsing them broke
    /// every sparse lobby. A roster names a source —
    /// pad 3, the keyboard — and a lobby is right to keep those numbers stable
    /// when a seat empties. A rollback host requires the opposite: handles
    /// `0..player_count`, dense, no holes. This carried only the first and
    /// handed it to `PlayerSlot`, so a fighter could sit on a channel the
    /// session never opened and receive nothing for the whole match.
    ///
    /// the channel is DERIVED here and nowhere else, from
    /// [`MatchParticipantRoster::local_channel_plan`], so the number that sizes
    /// the session and the number the fighter reads are the same number by
    /// construction rather than by two matching derivations.
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

    /// Resolve a roster's stated binding into the authority to attach.
    ///
    /// `channel` is the plan's answer for this seat — the dense position of this
    /// human among the roster's humans — and is what makes a sparse source safe
    /// to carry.
    ///
    /// every variant, no catch-all — and the catch-all is what this fixes. The pass this
    /// replaces ended in `_ => { let Some(profile) = controller.brain_profile() else { return;
    /// }; .. }`, and `brain_profile()` answers `None` for `Replay` and `Policy` BY DESIGN — its
    /// own doc says *"a replay that consulted a brain profile would stop being a replay"*.
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
            // The engine genuinely has no driver to attach for either; saying so is the honest
            // version of what it already did.
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
    /// `Entity`, which is why placement and the view policy are keyed on it.
    pub seat: usize,
    /// typed for the same reason the participant's is (P0.3): this is the id
    /// the prepared registry is keyed on, and `Borrow<str>` keeps every existing
    /// `&str` lookup working without minting an id to ask a question.
    pub character_id: ambition_entity_catalog::CharacterId,
    /// This BODY's stable identity, distinct from the character it wears.
    ///
    /// a match may legitimately be a MIRROR — two seats, one character — and
    /// this id is what presentation, the anti-clump slot board, the steering
    /// neighbour index and the target/faction maps are ALL keyed on
    /// (`HashMap<String, _>`, every one of them). Keying it on the character made
    /// two fighters one entity to every one of them, and
    /// `spawn_dynamic_feature_visuals` dedupes by id — so one of the pair could
    /// never be drawn.
    pub feature_id: String,
    /// The owned definition, so activation can read the physical baseline
    /// without asking the registry what it currently says.
    pub definition: PreparedCharacterDefinition,
    /// The owned pre-spawn cluster, built from the character authorities
    /// during preparation and never re-derived.
    pub seed: crate::features::ecs::actor_clusters::ActorClusterSeed,
    /// The body box this fighter was resolved to occupy.
    pub body_px: Vec2,
    pub faction: ambition_combat::components::ActorFaction,
    pub team: Option<ambition_combat::targeting::MatchTeam>,
    /// What will drive it, attached AFTER the body exists — never a fork in how
    /// the body is built.
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
    /// See [`Self::identity_kit`]. Derived from [`Self::action_set`] by the same
    /// overlay call, so it can never describe a different repertoire.
    pub combat_kit: ambition_combat::components::CombatKit,
    /// Effective ability set after match guarantees and permissions are applied.
    /// Kit derivation must use this resolved set.
    pub effective_abilities: Option<ambition_platformer2d_core::AbilitySet>,
    /// The body this seat plays with — the character's own movement feel, or
    /// the one the match supplies to a character that authored none (see
    /// [`MatchRules::body_over`]).
    ///
    /// resolved here for the same reason
    /// [`Self::effective_abilities`](Self::effective_abilities) is: the seat
    /// carries the answer, so the body that is BUILT and the body a test or a
    /// UI reads cannot be two derivations of one question. It is handed to
    /// `grant_prepared_character_body`, the one place a prepared definition
    /// becomes a body.
    pub effective_movement_tuning: Option<ambition_platformer2d_core::MovementTuning>,
}

/// What every fighter in this match plays under.
///
/// On the match rather than decided by construction, for the reason the roster's
/// own fields state: the engine does not get an opinion about a match's economy.
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
    /// character's own. See
    /// [`MatchParticipantRoster::fighter_health_pool`](super::staging::MatchParticipantRoster::fighter_health_pool)
    /// — the field it is carried from, and where the reasoning lives.
    pub health_pool: Option<i32>,
    pub opens_suspended: bool,
    /// How long the opening ceremony holds the cast, in simulation ticks.
    ///
    /// `0` means no ceremony: a suspended cast is released on the tick it is
    /// built, which is what every match did before this existed and is the
    /// honest reading of `opens_suspended` for a ruleset with no opening.
    ///
    /// TICKS, not seconds, and that is a determinism requirement rather
    /// than a taste. The release is a comparison against the sim clock, so a
    /// rollback re-runs it and reaches the same answer; a wall-clock timer
    /// would drift a peer's release by a frame and diverge the whole cast.
    pub opening_countdown_ticks: u32,
    /// See [`MatchRoster::time_limit_ticks`](crate::character_runtime::MatchRoster::time_limit_ticks).
    pub time_limit_ticks: u32,
    /// What this match drops and how often, carried from
    /// [`MatchParticipantRoster::item_spawns`](super::staging::MatchParticipantRoster::item_spawns).
    /// `None` = no items.
    pub item_spawns: Option<super::staging::MatchItemSpawns>,
}

/// Where an opening ceremony has got to — derived from the clock, never
/// stored.
///
/// no ticking timer anywhere, on purpose. A countdown is the obvious
/// place to put a `f32` that counts down, and doing that would add authoritative
/// mutable state inside the rollback window — the trap this file already paid
/// for once with `effective_from` (*"the original frame 8 ran with no plan and
/// activated on 9, while the RESIMULATED frame 8 found the plan already standing
/// and activated on 8"*). A phase computed from `now - activated_on` is a pure
/// function of the clock and the receipt, so a rewind cannot land the ceremony
/// on a different beat than the first run did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OpeningPhase {
    /// The cast is held. `beats_remaining` counts DOWN to the release: 3, 2, 1.
    Counting { beats_remaining: u32 },
    /// The hold is over. The tick this first reads `Live` is the release tick.
    Live,
}

impl MatchRules {
    /// S4: a stocks match's fighters die to the WORLD, not to the meter.
    /// Declared once so no two seats can disagree about it — a divergence this
    /// file's predecessor had three times.
    /// The pool a seat gets: the match's, if it declared one, else the
    /// character's own.
    ///
    /// the same shape as [`Self:death_policy`] one function down, and the pairing is the point.
    pub fn pool_over(&self, authored: i32) -> i32 {
        self.health_pool.map(|pool| pool.max(1)).unwrap_or(authored)
    }

    /// THE BODY A SEAT PLAYS WITH: this match's own numbers, over the body the
    /// fighter brought. Stated once, here.
    ///
    /// ```text
    ///   match says nothing   ->  whatever the fighter brought, untouched
    ///   match declares       ->  MatchBody::over(the character's body, else the built one)
    /// ```
    ///
    /// it is not a precedence question, which is why it does not read like
    /// [`Self::pool_over`] one function up. A pool is ONE number and two
    /// authorities have to be ranked; a body is fifty numbers and a mode has an
    /// opinion about six of them.
    /// [`MatchBody`](ambition_platformer2d_core::MatchBody) is exactly those
    /// six, so the composition disturbs nothing else and the character keeps its
    /// gait, its jump arc and its gravity whether or not it authored a
    /// `MovementTuning` at all.
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

    pub fn death_policy(&self) -> ambition_characters::actor::DeathPolicy {
        if self.stocks.is_some() {
            ambition_characters::actor::DeathPolicy::Unbounded
        } else {
            ambition_characters::actor::DeathPolicy::default()
        }
    }

    /// How many BEATS the ceremony has, one per counted number.
    ///
    /// Three beats is "3, 2, 1" — the ticks are divided evenly and the
    /// remainder lands on the last beat, so a 180-tick countdown at 60Hz is one
    /// second a number.
    pub fn opening_beats(&self) -> u32 {
        if self.opening_countdown_ticks == 0 {
            0
        } else {
            OPENING_BEATS
        }
    }

    /// Where the ceremony stands `elapsed` ticks after the cast was built.
    ///
    /// a match with no ceremony is `Live` from tick zero, which is what
    /// makes this safe to consult unconditionally: a ruleset that never asked
    /// for a countdown cannot accidentally acquire one.
    pub fn opening_phase(&self, elapsed: u64) -> OpeningPhase {
        let total = u64::from(self.opening_countdown_ticks);
        if elapsed >= total {
            return OpeningPhase::Live;
        }
        let beats = u64::from(self.opening_beats().max(1));
        // Ticks per beat, rounded UP, so the final beat is the short one rather
        // than the first — a "1" that lingers reads as a stall on the tick the
        // fighters are about to be released.
        let per_beat = total.div_ceil(beats);
        let elapsed_beats = elapsed / per_beat;
        OpeningPhase::Counting {
            beats_remaining: (beats - elapsed_beats.min(beats - 1)) as u32,
        }
    }

    /// How many ticks are left on the match clock, or `None` for an untimed
    /// match — which is every roster that declares no limit.
    ///
    /// derived, never counted down. `elapsed` is
    /// `ActiveMatch::ticks_since_activation`, so this is a pure function of two
    /// numbers the rollback window already carries: a rewind RECOMPUTES the
    /// clock rather than restoring it, and a match clock costs no wire format.
    pub fn time_remaining(&self, elapsed: u64) -> Option<u64> {
        (self.time_limit_ticks > 0)
            .then(|| u64::from(self.time_limit_ticks).saturating_sub(elapsed))
    }

    /// Has the clock run out? `false` for an untimed match, which is what
    /// makes this safe to consult unconditionally.
    pub fn time_expired(&self, elapsed: u64) -> bool {
        self.time_remaining(elapsed) == Some(0)
    }
}

/// How many numbers an opening ceremony counts: 3, 2, 1.
///
/// A constant rather than a rule field because it is the GENRE's shape — every
/// platform fighter counts three — while how LONG each number holds is the
/// ruleset's call and lives in `opening_countdown_ticks`.
pub const OPENING_BEATS: u32 = 3;

/// The match, resolved.
#[derive(Resource, Clone, Debug)]
pub struct PreparedMatch {
    seats: Vec<PreparedSeat>,
    rules: MatchRules,
    /// The [`PreparedCharacterRegistry`] generation these seats were resolved
    /// against.
    ///
    /// Silently re-resolving would put a live authority back inside activation, which is the
    /// one property this module exists to remove.
    cast_generation: ambition_characters::prepared::CharacterCatalogGeneration,
    /// The frozen seat topology the ROSTER was agreed under, carried through so
    /// the activation can cite it.
    ///
    /// carried rather than re-read: a later disagreement about who is playing
    /// is only answerable if the live match can say which topology decided it,
    /// and asking the world at activation would answer with whatever is true
    /// then rather than with what this plan was built from.
    seat_topology: Option<u64>,
    /// The first `SimTick` this plan may build on.
    ///
    /// WHEN a decision takes effect is part of the decision, and leaving it out is a
    /// determinism hole. The plan is deliberately not rollback state, so it survives a rewind
    /// — but its ARRIVAL did not: the original frame 8 ran with no plan and activated on 9,
    /// while the RESIMULATED frame 8 found the plan already standing and activated on 8.
    ///
    /// stamping the tick makes activation a pure function of the plan and the
    /// clock, which is the property that lets a rewind reconstruct the SAME
    /// match instead of a similar one built a frame early.
    effective_from: u64,
    /// The gameplay session this plan was decided FOR.
    ///
    /// content cannot answer this. The obvious repair — re-prepare when
    /// the roster differs — fails on the case that actually happens: a rematch
    /// with the same two picks publishes an IDENTICAL roster. What changed is
    /// not what was chosen, it is that this is a different SESSION.
    session: Option<ambition_platformer2d_shared_tangle::lifecycle::SessionScopeId>,
    /// Which experience's roster this plan was built from.
    ///
    /// inherited, never authored. It is copied from
    /// [`MatchParticipantRoster::published_by`] in [`prepare_match`], so a
    /// provider that already says who it is on the roster says it here for free
    /// — and a plan whose owner disagreed with its roster's would be describing
    /// a match nobody asked for.
    ///
    /// this is what makes teardown safe in a host that runs more than one
    /// game. `PreparedMatch` is a GLOBAL resource shared by every experience
    /// that stages a cast, so a scope that removed it by type would be one game
    /// deleting another's plan — the roster's own lesson, one resource later.
    /// See `ExperienceScopeBuilder::releasing_owned`.
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

    /// Whether the live cast generation differs from the frozen generation this
    /// plan was prepared against. This is a staleness diagnostic only; activation
    /// never re-resolves the plan.
    pub(crate) fn cast_moved_on(
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

    /// Was this plan built from `experience_id`'s roster?
    ///
    /// An UNOWNED plan (no publisher on its roster) answers `false` to everyone, which leaks
    /// rather than deletes: the safe direction, because the cost of a leak is one stale plan
    /// and the cost of a wrong delete is another game's live match.
    pub fn is_published_by(&self, experience_id: &str) -> bool {
        self.published_by.as_deref() == Some(experience_id)
    }

    /// Build a plan carrying nothing but an OWNER, for a test about teardown.
    ///
    /// The fields stay private so production has exactly one builder
    /// ([`prepare_match`]); this is the hatch, and it is named for what it is.
    /// A scope test needs a plan that says whose it is and needs nothing else to
    /// be true about it.
    #[doc(hidden)]
    pub fn for_test_published_by(experience_id: Option<&str>) -> Self {
        Self {
            seats: Vec::new(),
            rules: MatchRules::default(),
            cast_generation: ambition_characters::prepared::CharacterCatalogGeneration::default(),
            seat_topology: None,
            // A teardown test cares about the OWNER and nothing else: tick zero
            // is a plan every clock has already reached, and no session means
            // this plan matches the composition a bare test world has.
            effective_from: 0,
            session: None,
            published_by: experience_id.map(str::to_owned),
        }
    }

    /// Which local source drives which channel in this match.
    ///
    /// NOT `seats().len()`. That number sized the GGRS session while the
    /// frozen input topology used a different one. A CPU is a participant and
    /// not a channel; a match of two CPUs needs none at all.
    ///
    /// and not a count either. `plan.channels()` is the handle count, and
    /// the rest of the plan is the half that says whose controller each handle
    /// listens to — which is what a sparse lobby cannot survive losing.
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
/// Published where a consumer can say something true to a player, and read by
/// tests instead of a log line. Present only while an unpreparable roster is
/// standing.
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
/// A placeholder ON PURPOSE and a small one: making it generous would hide a
/// character whose art never resolved behind a plausible-looking rectangle.
const SEAT_BODY_PX: Vec2 = Vec2::new(30.0, 48.0);

/// Resolve a roster into a match, or say exactly why it cannot be one.
///
/// every problem, not the first. A lobby wants to be told everything that
/// is wrong with its choice; returning on the first would make fixing a
/// four-seat roster a four-attempt guessing game.
#[allow(clippy::too_many_arguments)]
pub fn prepare_match(
    roster: &MatchParticipantRoster,
    registry: &PreparedCharacterRegistry,
    catalog: &ambition_characters::actor::character_catalog::CharacterCatalog,
    authored_sheets: &ambition_sprite_sheet::character::sheets::AuthoredSheets,
    // THE CONTROLLER POLICIES THIS COMPOSITION PUBLISHED — the only place a
    // seat's policy can come from.
    //
    // a `&CharacterRoster` stood beside this and was asked FIRST. A match's public API is
    // *character + controller + team*, and the implementation resolved the controller half
    // through an ENEMY ARCHETYPE table, so Smash was not yet proving the controller
    // architecture it advertises.
    profiles: Option<&ambition_characters::actor::character_catalog::BrainProfileRegistry>,
    centre: Vec2,
    // The first `SimTick` the resulting plan may build on — see
    // `PreparedMatch::effective_from`. Preparation runs in `Update`, after the
    // frame's simulation, so the caller passes the NEXT tick.
    effective_from: u64,
    // Which gameplay session this plan is FOR — see `PreparedMatch::session`.
    session: Option<ambition_platformer2d_shared_tangle::lifecycle::SessionScopeId>,
    // What the SESSION declared about a home avatar. Preparation needs it to
    // refuse a local seat in a session that already has one — see the seat
    // loop below.
    home_body: &crate::avatar::starting_character::InitialBodyPolicy,
) -> Result<PreparedMatch, MatchPreparationProblems> {
    // ⭐ THE ROSTER'S OWN RULES, not a transcription of them. This copied eight
    // loose roster fields into `MatchRules` one by one — two representations of
    // one fact, so every new rule cost a field, a line here, and an initializer
    // in every roster literal in the tree.
    let rules = roster.rules.clone();
    let death_policy = rules.death_policy();

    let mut problems: Vec<RosterProblem> = Vec::new();
    let mut seats: Vec<PreparedSeat> = Vec::new();

    // WHO IS PLAYING, AND ON WHAT — asked once, by the roster.
    //
    // Every seat below reads its channel out of this rather than deriving one,
    // so the number that sizes the GGRS session, the number the frozen topology
    // maps to a controller, and the number the fighter's `PlayerSlot` carries
    // are one number by construction.
    let plan = roster.local_channel_plan();
    // ONE CONTROLLER CANNOT DRIVE TWO FIGHTERS. Refused before the match
    // exists rather than after: the second claimant's channel is real, its
    // handle is opened, and nothing ever writes it — a fighter that stands
    // still all match with no error anywhere. Two seats defaulting to pad 0 is
    // the ordinary way to arrive here.
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

        // THE POPULATION THAT CAN ACTUALLY BE BUILT.
        //
        // this check did not exist anywhere.
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

        // THE DENSE CHANNEL FOR THIS SEAT, taken from the plan in step with
        // the seats it was built from — and checked against it, because a plan
        // that disagreed with the seat it describes would seat somebody on
        // another person's controller.
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

        // TWO CLAIMANTS ON ONE LOCAL CHANNEL, NAMED HERE INSTEAD OF PANICKING
        // FOUR SYSTEMS DEEP.
        //
        // a match experience declares `InitialBodyPolicy::NoInitialBody`;
        // that is what the policy is FOR. Seating a local match into an
        // exploration session is a composition error, and this is the boundary
        // that knows it — before one entity exists.
        if home_body.spawns_a_body() && authority.local_channel().is_some() {
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

        // A CPU's profile must name a policy this composition PUBLISHED.
        if let ControlAuthority::Brain { profile } = &authority {
            if seat_brain_profile(
                profile,
                roster.published_by.as_deref(),
                &definition.provider,
                profiles,
            )
            .is_none()
            {
                // this listed the roster's archetype keys beside the published
                // ones and called the archetype table "the LEGACY half — a seat
                // should name a published policy".
                // A seat CANNOT name one now: `seat_brain_profile` has one arm.
                // Printing an authority that cannot answer sends the reader to
                // add a row that would change nothing.
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

        // The authored BRAIN the seed is built from. A local-input seat authors
        // `Passive` because its real driver is attached afterwards; a passive
        // placeholder rather than a wandering one, so a body whose writer never
        // arrives stands still instead of strolling off looking possessed.
        let seed_brain = match &authority {
            ControlAuthority::Brain { profile } => {
                ambition_entity_catalog::placements::CharacterBrain::Custom(profile.clone())
            }
            _ => ambition_entity_catalog::placements::CharacterBrain::Passive,
        };

        let (at, facing) = seat_placement(index, centre);

        // THE AUTHORED PHYSICAL IDENTITY, read through `PhysicalBaseline`
        // rather than off `vitals`/`body` directly, because the exploration
        // player reads the same value through the same accessors.
        let baseline = super::PhysicalBaseline::of(definition);
        // The box the SEED is built around. A hint, not the answer: for a named
        // catalog character `ActorClusterSeed::new_in` resizes to the AUTHORED
        // SPRITE's collision — the same resolution a peaceful NPC of that
        // character gets — and the seat has to take that size back, which is
        // what `seat.body_px` below reads.
        let hint_px = baseline.explicit_size().unwrap_or(SEAT_BODY_PX);
        let aabb = ambition_platformer2d_core::Aabb::new(at, hint_px / 2.0);
        // THE SEAT, not the character. A mirror match is two bodies
        // wearing one character, and every id-keyed index in the actor runtime
        // would collapse them into one: `entity_to_id`, the anti-clump slot
        // board's `requests`, `faction_by_id` and `target_entity_by_id`
        // (`features/ecs/actors/update.rs`), plus `ActorIdentity` itself. The
        // ART still resolves from the character — a body whose id is not its
        // costume's name. this cited an `art_identity` accessor that no longer
        // exists anywhere in the workspace (AC7.1).
        let body_id = format!("{}#seat{index}", participant.character);
        // CHARACTER-FIRST. This built through `new_in`, which starts
        // `roster.spec_for_brain(&brain)` — so every fighter on the grid was
        // physically a `combatant` with a character painted over it, and the
        // seat then took the health and the weight back one field at a time.
        // should first build an `ArchetypeSpec` creature and then patch the
        // character over it."*
        //
        // The CONTROLLER's policy is resolved below and handed in as a VALUE — a profile is a
        // decision, and this constructor takes it rather than looking up a body to get one.
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
        // the character, as ONE value, with the MATCH's overrides named.
        // A seat differs from a room placement in exactly three ways, and each
        // is a line rather than a parameter buried in a list of fourteen.
        let mut body = definition.seat_blueprint(ambition_platformer2d_core::MAX_RUN_SPEED);
        // THE MATCH'S POOL, or the character's own. `baseline` already folded the
        // definition's authored maximum; `pool_over` is where a crossover match overrules it,
        // because a percent read against four different games' authored maxima is four
        // different percents.
        body.max_health = rules.pool_over(baseline.max_health_over(1));
        // The policy is the MATCH's decision, not the character's default: a
        // human seat's body carries none at all.
        body.autonomous_profile = Some(profile);
        // a MATCH seat is never a practice target, whatever the character
        // says: a stage seats fighters, and a body excluded from the save and
        // skipped by targeting would be a seat nobody can fight.
        body.practice_target = false;
        let mut seed = crate::features::ecs::actor_clusters::ActorClusterSeed::new_character_in(
            authored_sheets,
            catalog,
            body_id.clone(),
            body,
            aabb,
            seed_brain,
            // A stage has no authored patrol paths; a seat is driven.
            &[],
        );
        // The seed's own pool stands for a character that authored none
        seed.health =
            ambition_characters::actor::BodyHealth::new(ambition_characters::actor::Health::new(
                rules.pool_over(baseline.max_health_over(seed.health.health.max.max(1))),
            ))
            .with_policy(death_policy);
        // THE AUTHORED KNOCKBACK WEIGHT, onto the seed for the same reason
        // the pool is: `into_components` projects `config.tuning.weight` onto
        // the combat-owned `CombatTuning` the damage paths read, so setting it
        // here is what makes a heavy fighter heavy from its first frame rather
        // than after a re-wear. A character that authors none keeps its roster
        // archetype's, which is every character that has not thought about it.
        if let Some(weight) = baseline.knockback_weight() {
            seed.config.tuning.weight = weight;
        }
        seed.kin.facing = facing;

        // the placeholder is still right for a character that authors no
        // size — it is deliberately small so an unresolved body looks wrong
        // rather than plausible — but it must not outrank a size the seed
        // actually resolved.
        let body_px = seed.kin.size;
        // Everything below that asks "what can this body do" asks THIS, so the kit, the body's
        // abilities and the AI's capability read can never disagree.
        let seat_abilities = effective_abilities(definition.abilities, rules.abilities);
        let mut overlay_name = Name::new(definition.display_name.clone());
        let mut overlay_action_set = participant
            .action_set
            .clone()
            .or_else(|| definition.kit.action_set().cloned())
            .unwrap_or_default();
        let mut moveset = ambition_combat::moveset::ActorMoveset(Default::default());
        let mut identity_kit = ambition_characters::brain::action_set::IdentityKit::default();
        let mut overlay_combat_kit =
            ambition_combat::components::CombatKit::from_action_set(&overlay_action_set);
        let _ = crate::avatar::apply_worn_character_overlay(
            catalog,
            Some(registry),
            &mut overlay_name,
            &mut overlay_action_set,
            &mut moveset,
            &mut identity_kit,
            Some(&mut overlay_combat_kit),
            participant.character.as_str(),
            // NOT `seed.body.0.abilities.abilities` — that is the pre-mask set,
            // and deriving the kit against it is exactly the ordering §3 names.
            seat_abilities.unwrap_or(seed.body.0.abilities.abilities),
            participant.action_set.as_ref(),
        );
        let moveset = moveset.0;
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
            // A participant fights as itself. Every seat carries a team, so the
            // relationship policy never has to fall back to faction inside a
            // match — and the faction is left to mean what it means everywhere
            // else in the world.
            faction: ambition_combat::components::ActorFaction::Player,
            team: Some(team_for(index, participant.team.as_ref())),
            authority,
            match_kit: participant.action_set.clone(),
            identity_kit,
            action_set: overlay_action_set,
            combat_kit: overlay_combat_kit,
            effective_abilities: seat_abilities,
            // THE BODY, RESOLVED BESIDE THE VERBS — and it has to be, or
            // the stage grants a verb whose window never opens. See
            // `MatchRules::body_over`.
            //
            // the base is the seed's OWN tuning, which is what the integrator
            // falls back to when a body carries no marker — so a mode's six
            // numbers land on the body this fighter would otherwise have had,
            // and nothing else about it moves.
            //  THE SEAT'S OWN BODY OUTRANKS THE CHARACTER'S, and it has to:
            // a catalog row's feel is that character's feel everywhere it
            // appears, so a fighter self and a home self cannot both state one
            // there. See `MatchParticipant::body`.
            effective_movement_tuning: rules
                .body_over(participant.body.or(definition.movement_tuning), built_body),
            moveset,
        });
    }

    if !problems.is_empty() {
        return Err(MatchPreparationProblems { problems });
    }

    // WHERE THE CAMERA LOOKS IS NOT DECIDED HERE, YET. A draft of this
    // module returned a `MatchViewPolicy` (follow the first local seat, else
    // frame the whole cast) and nothing read it. An unread value is dead code
    // dressed as intent — the same objection this module makes to control
    // authorities nothing can attach — so it is not here.
    //
    // WHERE THE CAMERA LOOKS IS ANSWERED NOW, and the answer is not here:
    // the match DECLARES its cast (`FramedCast`) once the bodies exist, and the
    // camera resolver frames them when nothing local is driving one. A draft of
    // this function returned a `MatchViewPolicy` and nothing read it; the value
    // was never the mistake, having no consumer was.

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

/// Resolve fighter abilities before construction. Character-authored verbs provide the base;
/// match rules grant additional verbs and/or cap the result:
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
/// Symmetric about `centre`, alternating sides, facing inward. Public so a rules
/// layer can put a fighter BACK between rounds without re-deriving the geometry
/// and drifting from it.
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

#[cfg(test)]
mod tests;

/// Resolve a seat's match team without changing the character's authored world
/// faction. An authored team is preserved; otherwise each seat gets its own team,
/// producing free-for-all relationships.
fn team_for(index: usize, authored: Option<&String>) -> ambition_combat::targeting::MatchTeam {
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
fn seat_brain_profile(
    key: &str,
    match_provider: Option<&str>,
    provider: &str,
    profiles: Option<&ambition_characters::actor::character_catalog::BrainProfileRegistry>,
) -> Option<ambition_characters::brain::BrainProfile> {
    profiles.and_then(|profiles| {
        let reference = ambition_entity_catalog::BrainProfileRef::new(key);
        // No global bare-key fallback: provider ownership is part of identity.
        //
        // provider decorative, so one game's `duelist` can drive another's
        // fighter. An already-qualified name is handled by `resolve_in`.
        match_provider
            .and_then(|owner| profiles.get(&reference.resolve_in(owner)))
            .or_else(|| profiles.get(&reference.resolve_in(provider)))
            .copied()
    })
}

fn realize_seat(
    commands: &mut Commands,
    session_scope: ambition_platformer2d_shared_tangle::lifecycle::SessionSpawnScope,
    seat: &PreparedSeat,
    // Which cast this match was prepared against — the applied-template stamp
    // records it, so a hot reload can tell a seated body apart from a current one.
    cast_generation: ambition_characters::prepared::CharacterCatalogGeneration,
) -> Entity {
    let mut seed = seat.seed.clone();
    // set on the SEED, not inserted beside it. `CombatCapabilities` is
    // already a member of the cluster bundle, so a second insert in the same
    // spawn is a duplicate component and Bevy refuses the whole bundle — which
    // is what a first attempt did, taking five seat tests down with it. The
    // seed's own note says the persona brings this; a seat that will stop asking
    // for a persona pass brings its own, and the enemy road already does exactly
    // this at construction.
    seed.caps = ambition_combat::CombatCapabilities::from(
        &seat.definition.death_traits.clone().unwrap_or_default(),
    );
    let at = seed.kin.pos;
    let facing = seed.kin.facing;
    let centered = ambition_platformer2d_core::CenteredAabb::from_center_size(at, seat.body_px);
    // the seed's model, which `grant_prepared_character_body` then switches to
    // the CHARACTER's below. Switching rather than replacing is ADR 0024: a
    // cross-model change preserves every shared body fact and initializes only
    // the destination solver's private state.
    let motion_model = seed.config.tuning.motion_model();
    let (identity, _seed_disposition, combat) =
        crate::features::ecs::enemy_component_snapshot(&seed);
    // A match participant is a COMBATANT, whatever drives it. The disposition the seed derives
    // follows the authored brain, and a local-input seat authors `Passive` — `apply_actor_hit`
    // reads the disposition first, and a peaceful body takes NO health damage.
    let disposition = ambition_combat::components::ActorDisposition::Hostile;
    // Use the action set resolved by preparation rather than deriving a second
    // answer from the same inputs.
    let action_set = seat.action_set.clone();
    // THE AUTONOMOUS POLICY THIS SEAT WILL HAVE, chosen once and spawned WITH
    // the body.
    //
    // Two steps to say one thing, and this repo's own rule about that is "an authority that needs a
    // FOLLOW-UP CALL — the second step belongs inside the first". Here the cost was concrete: for
    // one command-queue ordering there existed a world in which a seated fighter is AI-brained, and
    // a rollback snapshot that captures THAT world restores it forever, because activation is
    // one-shot and never rebinds.
    //
    // and that race is now structurally impossible, not merely avoided.
    // A local seat's driver is [`DrivingParticipant`], a SEPARATE component from
    // the policy — the two are no longer competing values of one field, so a
    // dropped write cannot leave the body wearing the other one's answer. What
    // a human-driven fighter gets here is the policy it falls back to when
    // nobody is driving, and for a seat the roster describes only as "a person
    // plays it", that is standing still.
    let derived_brain = match &seat.authority {
        ControlAuthority::LocalInput { .. } => ambition_characters::brain::Brain::stand_still(),
        ControlAuthority::Brain { .. } => {
            // the AI's capability read asks the SAME effective set the kit was
            // derived against: a driver that believes it may shield in a match
            // that forbids shielding reaches for a verb the body does not have.
            crate::features::ecs::enemy_default_brain(
                &seed.config,
                seat.effective_abilities
                    .unwrap_or(seed.body.0.abilities.abilities),
            )
        }
    };
    // Likewise: derived beside the action set by the ONE overlay call, so the two
    // can never describe different repertoires.
    let combat_kit = seat.combat_kit.clone();
    let cluster = seed.into_components();
    use ambition_platformer2d_shared_tangle::lifecycle::SpawnSessionScopedExt;
    let body = commands
        .spawn_session_scoped(
            session_scope,
            (
                crate::features::EnemyActorBundle::new(
                    crate::features::FeatureBaseBundle::new(
                        // THE SEAT, not the character. This passed
                        // `character_id`, so two fighters wearing one character
                        // WERE ONE FEATURE: `spawn_dynamic_feature_visuals`
                        // dedupes by id and drew only the first, and the slot
                        // board, the steering neighbour index and the
                        // target/faction maps are all `HashMap<String, _>` keyed
                        // on it — a mirror match corrupted every one of them.
                        //
                        // Unifying construction is what made two of them exist — the same shadow
                        // the registration gap and the couch crosstalk came out of.
                        //
                        // a body's render/simulation identity is the BODY.
                        // What it WEARS is `WornCharacter`, and a mirror match is
                        // an ordinary thing a platform fighter must allow.
                        seat.feature_id.as_str(),
                        seat.definition.display_name.clone(),
                        centered,
                    ),
                    identity,
                    disposition,
                    seat.faction,
                    ambition_combat::components::ActorPose::from_parts(
                        at,
                        seat.body_px / 2.0,
                        facing,
                    ),
                    combat_kit,
                    ambition_combat::components::ActorAggression::hostile(),
                    combat,
                )
                .with_motion_model(motion_model),
                cluster,
                action_set,
                derived_brain,
                Name::new(seat.definition.display_name.clone()),
                // This was an empty contract with the persona derive expected to fill it on the
                // body's first tick.
                (
                    ambition_combat::moveset::ActorMoveset(seat.moveset.clone()),
                    // AND THE IDENTITY KIT — grant three.
                    seat.identity_kit.clone(),
                ),
                // The body WEARS the character. Everything that makes it that
                // fighter rather than a generic actor follows from this one
                // component.
                ambition_characters::actor::WornCharacter::new(seat.character_id.as_str()),
                // Both applied-template records are stamped by `grant_prepared_character_body`
                // below, which is also what installs the hurtboxes, the posed body, the movement
                // tuning and the motion model — see the call for why they are not here. The MATCH
                // owns this fighter's death, not the world. Without it a KO runs the exploration
                // economy — a bounty coin, a heart, an in-place respawn timer — none of which an
                // arena has a use for.
                ambition_combat::components::RulesetOwnsDeath,
                // And it is IN the fight — which is a different fact from whose
                // business its death is, and the one every other combat system
                // actually wants. Removed again when the fighter is eliminated.
                ambition_combat::components::ActiveCombatant,
                // WITHOUT THIS THE FIGHTER IS INVISIBLE: the authored render pass
                // only spawns visuals for `spec.enemy_spawns`, so a directly
                // staged actor would render nothing.
                ambition_combat::components::RuntimeStagedActor,
                ambition_characters::control::ActorControl::default(),
                ambition_characters::actor::attack_gesture::AttackGestureState::default(),
                ambition_characters::actor::attack_gesture::AttackGestureTuning::default(),
                ambition_characters::actor::attack_gesture::ResolvedAttackGesture::default(),
            ),
        )
        .id();
    // THE AUTHORED MASS. Conditional: a character that authored none must
    // keep its archetype's rather than be overwritten with the ambient 1.0.
    // Health and geometry are already on the seed, so this is all the boundary
    // has left to do.
    super::PhysicalBaseline::of(&seat.definition).apply_to_body(
        super::BaselineBoundary::Construction,
        &mut commands.entity(body),
        None,
        // The weight rode the SEED (`seat.body_weight`), the way health and
        // geometry did — this boundary has only the mass left to write.
        None,
        None,
        super::PhysicalRetraction::NONE,
    );
    // removing `RecharacterizeBody` silences the PERSONA derive and nothing else.
    // `project_prepared_character_definitions` is a SECOND template observer, it fires on
    // `Changed<WornCharacter>`, and a seated body had no `ProjectedCharacterKit` — so a seat
    // that asked the derive for nothing was still finished a tick later by the projector:
    // hurtboxes, the authored posed body, movement tuning, the motion model. ⛔ TWO
    // observers reach this body, so silencing one settles nothing.
    //
    // so the seat calls the ONE materializer instead of hand-copying a
    // third subset of it. `CallerResolved` says what is true here and nowhere
    // else: this caller already resolved its own kit — a match repertoire is the
    // character's overlaid with the match's override — so the grant must not
    // write the kit, but must do everything else and stamp BOTH records.
    crate::character_runtime::presentation::grant_prepared_character_body(
        commands,
        body,
        &seat.definition,
        cast_generation,
        crate::character_runtime::presentation::KitOwnership::CallerResolved,
        // AND THE BODY THE MATCH RESOLVED, for the same reason the kit is `CallerResolved`:
        // preparation already weighed the character's own feel against the stage's
        // (`MatchRules:body_over`), and a materializer that re-read the definition would
        // silently drop the stage's body — which is the whole of slice 1b.
        seat.effective_movement_tuning,
    );
    body
}

/// Attach the driver to a finished body.
///
/// The whole of what "who plays this fighter" now costs. It runs after the body
/// exists and can therefore be the same two lines for every fighter, which is
/// the property that made a CPU-only match impossible to express before.
fn bind_seat_control(commands: &mut Commands, body: Entity, authority: &ControlAuthority) {
    match authority {
        ControlAuthority::LocalInput { channel, .. } => {
            // the POLICY is not here — it is in the spawn bundle, deliberately;
            // see `realize_seat`. What is left is the local-input plumbing, and
            // it has no archetype counterpart to race with: an autonomous seat
            // writes no seat at all, so a dropped write here can only leave the
            // component ABSENT, never holding somebody else's answer.
            //
            // through the one correspondence, not `PlayerSlot(raw)`: the
            // seat the simulation reads is a projection of the participant
            // channel, and `participant_seat` is where that projection lives.
            commands.entity(body).insert((
                crate::control::components::LocalPlayer,
                ambition_characters::control::DrivingParticipant(
                    crate::participant_seat::player_slot_of(*channel),
                ),
            ));
        }
        // The seed already carries the archetype's brain, derived in
        // `realize_seat` exactly as the enemy spawner derives it, and nobody
        // drives it.
        ControlAuthority::Brain { .. } => {}
    }
}

/// Resolve the published roster into a plan, once.
///
/// Runs on the sim schedule beside activation, because it needs the session's
/// room geometry to place seats and the session world is what owns it.
pub fn prepare_the_match(
    mut commands: Commands,
    roster: Option<Res<MatchParticipantRoster>>,
    registry: Option<Res<PreparedCharacterRegistry>>,
    // REQUIRED, not optional: `engine.character-authority-is-app-local` forbids
    // making the character authority optional. A composition with no catalog
    // must be NAMED by the capability audit, not silently prepare fighters that
    // resolve their sprite identity against nothing.
    catalog: Res<ambition_characters::actor::character_catalog::CharacterCatalog>,
    authored_sheets: Res<ambition_sprite_sheet::character::sheets::AuthoredSheets>,
    // The published controller policies a CPU seat may name (P4). `Option` like
    // every other reader: a composition that publishes none is ordinary.
    //
    // a `Res<CharacterRoster>` stood beside this — REQUIRED, so every host
    // that prepared a match had to install an enemy archetype table to seat a
    // fighter. Preparation reads no roster at all now (P2.18).
    profiles: Option<Res<ambition_characters::actor::character_catalog::BrainProfileRegistry>>,
    geometry: Option<
        ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<
            ambition_platformer2d_core::RoomGeometry,
        >,
    >,
    // WHAT THIS SESSION DECLARED ABOUT A HOME AVATAR. Optional because a
    // minimal composition may publish a root that carries no policy at all;
    // absent is read as `NoInitialBody`, which is what such a root behaves like
    // — it lowered no avatar, so no seat can collide with one.
    home_body: Option<
        ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<
            crate::avatar::starting_character::InitialBodyPolicy,
        >,
    >,
    // WHICH session this plan is for; a plan from the previous one is stale.
    active_session: Option<Res<ambition_platformer2d_shared_tangle::lifecycle::ActiveSessionScope>>,
    prepared: Option<Res<PreparedMatch>>,
    // WHEN this plan becomes effective is part of the plan; see
    // `PreparedMatch::effective_from`.
    //
    // `Option`, for the reason spelled out on `FramedCast` below. A plain
    // `Res` is a whole-app panic in any composition that assembles this crate
    // without the timeline, and 32 of this crate's own unit tests were exactly
    // that. A world with no `SimTick` has no rollback timeline, so the tick the
    // plan names is stamped zero and the gate it feeds is trivially open —
    // correct, because the gate exists to make a frame and its REPLAY agree and
    // there is no replay without a timeline.
    tick: Option<Res<ambition_time::SimTick>>,
) {
    let session = active_session
        .as_deref()
        .and_then(ambition_platformer2d_shared_tangle::lifecycle::ActiveSessionScope::current);
    // Re-preparing every tick would rebuild the seeds under a live match and re-resolve them
    // against a possibly-republished registry, which is the authority-in-activation this module
    // exists to remove; re-preparing for a NEW session is the opposite of that — it is the only way
    // the second match of a sitting gets a plan at all.
    if prepared.is_some_and(|prepared| prepared.session() == session) {
        return;
    }
    let (Some(roster), Some(registry), Some(geometry)) = (roster, registry, geometry) else {
        return;
    };
    if roster.participants.is_empty() {
        return;
    }
    // A PROPOSED ROSTER DOES NOT PREPARE. A roster nobody has agreed to
    // is a WAIT, not a problem — publishing a refusal for one would cry wolf on
    // every ordinary route entry.
    if !roster.seating.may_seat() {
        return;
    }
    // The stage centre is the room's authored spawn: the one point a room
    // guarantees is standable, which is the only guarantee placement needs.
    let centre = geometry.0.spawn;
    let home_body = home_body.map_or(
        crate::avatar::starting_character::InitialBodyPolicy::NoInitialBody,
        |policy| policy.clone(),
    );
    // Preparation runs in `Update`, which follows the frame's simulation, so the
    // earliest tick this plan can be acted on is the NEXT one. Naming it is what
    // makes activation a pure function of the plan and the clock rather than of
    // when a non-rewinding resource happened to appear.
    let effective_from = tick.map_or(0, |tick| tick.get().saturating_add(1));
    match prepare_match(
        &roster,
        &registry,
        &catalog,
        &authored_sheets,
        profiles.as_deref(),
        centre,
        effective_from,
        session,
        &home_body,
    ) {
        Ok(plan) => {
            // A standing refusal is over: this roster resolved. Removed here
            // rather than on roster CHANGE so it cannot go stale — a refusal
            // that outlives the roster it was about is a worse lie than none.
            commands.remove_resource::<MatchPreparationProblems>();
            commands.insert_resource(plan);
        }
        Err(problems) => {
            bevy::log::error!(
                target: "ambition_platformer2d::match_preparation",
                "this composition cannot prepare the published match: {problems}"
            );
            commands.insert_resource(problems);
        }
    }
}

/// Build the whole cast, in one flush. Infallible.
///
/// Every permanent question was answered by [`prepare_the_match`], so there is
/// nothing here to refuse and no way for this to half-apply: either the match
/// activates completely on one tick or it has not started.
///
/// it BORROWS the plan. Consuming it would make a rewind to before
/// activation unreplayable: `bevy_ggrs` restores the ABSENCE of
/// [`ActiveMatch`](super::ActiveMatch), so activation re-runs — and it must find
/// the same plan waiting, or the cast it rebuilds is not the cast it built.
pub fn activate_the_prepared_match(
    mut commands: Commands,
    prepared: Option<Res<PreparedMatch>>,
    active: Option<Res<super::ActiveMatch>>,
    active_session: Option<Res<ambition_platformer2d_shared_tangle::lifecycle::ActiveSessionScope>>,
    // Seats that already have a body — derived from the world, so a rewind that
    // restored the fighters without the latch cannot double-build them.
    already_seated: Query<&super::MatchSeat>,
    // `Option` for the reason given on preparation's own `tick`.
    tick: Option<Res<ambition_time::SimTick>>,
    // not to re-resolve anything: see `PreparedMatch:cast_moved_on`.
    registry: Option<Res<ambition_characters::prepared::PreparedCharacterRegistry>>,
) {
    let Some(prepared) = prepared else {
        return;
    };
    let session = active_session
        .as_deref()
        .and_then(ambition_platformer2d_shared_tangle::lifecycle::ActiveSessionScope::current);

    // A PLAN BELONGS TO ONE SESSION, AND MAY ONLY BE BUILT INTO THAT ONE.
    //
    // preparation stamps the session and NOTHING CHECKED IT HERE. The stamp
    // was consulted only by preparation, deciding whether to re-prepare — so a
    // plan that outlived its session was structurally able to realize its cast
    // into the next one. Saying it here makes that unreachable rather than
    // unlikely.
    if prepared.session() != session {
        return;
    }

    // HAVE I ALREADY BUILT *THIS* MATCH? — asked of the receipt's identity,
    // not of the world's fighters.
    //
    // the receipt names its session now, so the question is one comparison
    // and fighter presence cannot affect the answer. A receipt for a DIFFERENT
    // session is stale and this same call replaces it below — activation
    // remains the single writer, which is what the
    // `a-second-writer-of-a-match-global-must-answer-ownership` contract asks
    // for, and it can answer whose receipt it is replacing because the receipt
    // says so.
    if active.is_some_and(|active| active.session() == prepared.session()) {
        return;
    }
    // NOT "the first tick the plan exists". The plan does not rewind, so
    // its arrival time is not a fact the simulation shares between a frame and
    // that frame's replay — the original ran without it and the resimulation
    // found it standing, and the cast appeared a tick early. The plan names the
    // tick instead.
    let now = tick.as_deref().map(|tick| tick.get());
    if now.is_some_and(|now| now < prepared.effective_from()) {
        return;
    }
    // No active session means no owner for the bodies. Activation waits rather
    // than spawning orphans; the plan is still there next tick.
    //
    // this is the SPAWN policy, not the identity above: a composition with no
    // session lifecycle at all resolves to `UNSCOPED` and still builds, and its
    // plan is stamped `None` so the identity comparison holds too.
    let Some(session_scope) =
        ambition_platformer2d_shared_tangle::lifecycle::SessionSpawnScope::for_optional_active_session(
            active_session.as_deref(),
        )
    else {
        return;
    };
    // A plan is frozen against the cast it was made from, so a character registered between
    // planning and activation is simply not in this match — correct, and for a match an anomaly
    // worth one line: somebody expected a fighter that will not appear, and no other signal
    // would tell them why.
    //
    // `warn_once!` because activation re-runs on every rewind to before it
    // (`bevy_ggrs` restores the ABSENCE of `ActiveMatch`), and a per-frame line
    // about a condition that cannot change is how a real warning gets muted.
    if let Some(registry) = registry.as_deref() {
        if prepared.cast_moved_on(registry.generation()) {
            bevy::log::warn_once!(
                target: "ambition_platformer2d::match_preparation",
                "the published cast has changed since this match was prepared, so \
                 the fighters below are the ones that existed at PLAN time — a \
                 character registered since is not in this match. That is the \
                 frozen-plan contract, not a bug; if a fighter is missing, this \
                 is why."
            );
        }
    }

    let occupied: std::collections::BTreeSet<usize> =
        already_seated.iter().map(|seat| seat.0).collect();

    let rules = prepared.rules();
    let mut bodies: Vec<(Entity, &PreparedSeat)> = Vec::new();
    for seat in prepared.seats() {
        if occupied.contains(&seat.seat) {
            continue;
        }
        let body = realize_seat(&mut commands, session_scope, seat, prepared.cast_generation);
        let mut seated = commands.entity(body);
        seated.insert(super::MatchSeat(seat.seat));
        if let Some(team) = seat.team.clone() {
            seated.insert(team);
        }
        bind_seat_control(&mut commands, body, &seat.authority);
        bodies.push((body, seat));
    }

    // THE MATCH'S RULES, in the same flush that builds the bodies, so no
    // fighter is ever observable in a state the ruleset did not ask for.
    for (body, seat) in &bodies {
        let mut entity = commands.entity(*body);
        // resolved at PREPARATION, and the same value the kit was derived
        // against — not a second intersection reached after the body exists.
        if let Some(abilities) = seat.effective_abilities {
            // `AbilityBase` too, not only the effective set: the effective set
            // is `base ∩ editable_mask`, recomputed every frame for a
            // player-driven body, so writing only `BodyAbilities` would be
            // undone next tick by a system behaving correctly.
            entity.try_insert((
                ambition_platformer2d_core::BodyAbilities::new(abilities),
                ambition_platformer2d_core::AbilityBase::new(abilities),
            ));
        }
        // WHICH MOUNT CLASSES THIS BODY MAY PILOT, and there is exactly one
        // source: the character. A raider seated in a match keeps the shark its
        // character authors, and so does an admiral.
        //
        // ⛔⛔ A SEAT HALF USED TO SIT BESIDE THIS and it was the wrong shape.
        // The idea was that shark-riding was Smash-only, so the MATCH would hand
        // it to one seat — but a match that manufactures a capability has to do
        // it on every road that builds a roster, and the character-select screen
        // was a road that did not. The admiral reached the match unable to board
        // its own summon. Jon settled the premise: the admiral rides sharks in
        // Ambition too, so the character says so and every road inherits it.
        //
        // ⚠ WHICH PARTICULAR MOUNT is a separate question and is NOT answered
        // here — see `ambition_mount::MountReservedFor`.
        let mut classes: Vec<ambition_mount::MountClass> = seat
            .definition
            .mount
            .as_ref()
            .map(|mount| mount.pilotable_classes.clone())
            .unwrap_or_default()
            .into_iter()
            .map(ambition_mount::MountClass)
            .collect();
        // Sorted and deduped so the component is a set rather than an
        // append-order list — two roads naming one class must not make a body
        // that pilots it twice.
        classes.sort_by(|a, b| a.as_str().cmp(b.as_str()));
        classes.dedup_by(|a, b| a.as_str() == b.as_str());
        if !classes.is_empty() {
            entity.try_insert(ambition_mount::CanPilot { classes });
        }
        if let Some(stocks) = rules.stocks {
            entity.try_insert(ambition_combat::components::FighterStocks::new(stocks));
        }
        if rules.opens_suspended {
            // the OPENING bit, distinct from the interlude a KO card claims:
            // two authorities that can hold the same fighter need two bits, or
            // whichever released first would free a body the other still holds.
            //
            // deferred like everything else in this flush, so it is written
            // through `commands` rather than the `entity` builder above.
            ambition_characters::control::claim_control_hold(
                &mut commands,
                *body,
                ambition_characters::control::ControlHold::Opening,
            );
        }
    }

    // This road inserted a fresh `false` beside the receipt, in the same flush, and that
    // WORKED.
    //
    // the receipt below IS the retraction now. A stocks verdict is
    // stamped with the `MatchInstance` it is about, so a new activation is a new
    // identity and the previous match's verdict simply stops applying — no
    // clearing, no ordering, and nothing here that names a ruleset.
    //
    // ATOMIC: the receipt goes in with the bodies, in the same flush. There is
    // no partial state to land a rewind in — either the tick that activated
    // happened or it did not.
    commands.insert_resource(super::ActiveMatch::activated(
        prepared.seats().len(),
        prepared.seat_topology(),
        prepared.session(),
        // WHEN, so the opening ceremony is a function of the clock rather than
        // a timer somebody has to remember to rewind.
        now,
    ));
}

/// Release the opening hold when the ceremony ends — every seat on ONE tick.
///
/// `opens_suspended` stamps `ScriptedControl` on every fighter in the flush that
/// creates them, so no body is ever observable in a state the ruleset did not
/// ask for. This is the other half: the tick the hold comes off.
///
/// The release belongs to match FLOW, next to the thing that applied the hold.
///
/// atomic by construction. Every held seat is released in one command
/// flush against one clock reading, so there is no tick on which one fighter can
/// act and another cannot — the property a countdown exists to give and the one
/// a per-fighter timer would quietly lose.
pub fn release_the_opening_hold(
    mut commands: Commands,
    active: Option<Res<super::ActiveMatch>>,
    prepared: Option<Res<PreparedMatch>>,
    mut held: Query<
        (Entity, &mut ambition_characters::control::ControlHolds),
        With<super::MatchSeat>,
    >,
    tick: Option<Res<ambition_time::SimTick>>,
) {
    let (Some(active), Some(prepared)) = (active, prepared) else {
        return;
    };
    // A CEREMONY THIS RULESET DID NOT DECLARE IS NOT THIS SYSTEM'S TO
    // END. `opens_suspended` with no countdown means somebody ELSE owns the
    // opening — the versus stage's `Starting` arm reaching zero is the live
    // case — and releasing here would take the hold off underneath them on the
    // tick the cast appears, which is precisely the window the flag exists to
    // close.
    if prepared.rules().opening_countdown_ticks == 0 {
        return;
    }
    // A composition with no clock cannot time a ceremony; the honest answer is
    // to release rather than to hold a cast forever waiting for a tick that
    // never arrives.
    let elapsed = tick
        .as_deref()
        .and_then(|now| active.ticks_since_activation(now.get()));
    if let Some(elapsed) = elapsed {
        if prepared.rules().opening_phase(elapsed) != OpeningPhase::Live {
            return;
        }
    }
    for (body, mut holds) in &mut held {
        // ONLY the opening's hold. A fighter this ceremony never suspended,
        // or one a capture is holding when the countdown ends, keeps whatever
        // else has a claim on it.
        ambition_characters::control::release_control_hold(
            &mut commands,
            body,
            Some(&mut holds),
            ambition_characters::control::ControlHold::Opening,
        );
    }
}

/// Declare the match's cast as what the camera frames.
///
/// The value was not the mistake; having no consumer was. No stage."* The camera resolves its
/// subject from `ControlledSubject` and returns without one, so a match with no local
/// participant framed nothing at all.
///
/// published, not guessed. The camera could scan for `MatchSeat` bodies
/// itself, and then presentation would own a question about what a session is
/// FOR. The match already knows; saying so is one line and keeps the resolver
/// generic — a cutscene or a replay viewer publishes the same resource.
///
/// ordered by SEAT. A `Vec<Entity>` built in query order is a different
/// vector frame to frame, and this one is read by a framing computation whose
/// output a person looks at.
///
/// Runs in `Update`: it is a projection of simulation state for presentation to
/// consume, not simulation state, so it must not be written inside the rollback
/// window.
pub fn declare_the_match_cast_as_the_view(
    active: Option<Res<super::ActiveMatch>>,
    seats: Query<(Entity, &super::MatchSeat)>,
    // `Option`, and it is not defensive. A plain `ResMut` here panicked 53
    // of this crate's own unit tests with *"Parameter `ResMut<FramedCast>` failed
    // validation: Resource does not exist"* — the resource is initialised by the
    // ABILITIES plugin and this system is registered by `character_runtime`, so
    // every composition that takes one without the other dies on its first
    // frame. A Bevy param panic is a hard stop for the whole app, not a skipped
    // system; the correct answer for a projection nobody has asked for yet is to
    // have nothing to say.
    mut framed: Option<ResMut<ambition_platformer2d_shared_tangle::markers::FramedCast>>,
) {
    let Some(framed) = framed.as_mut() else {
        return;
    };
    if active.is_none() {
        if !framed.0.is_empty() {
            framed.0.clear();
        }
        return;
    }
    let mut cast: Vec<(usize, Entity)> = seats
        .iter()
        .map(|(entity, seat)| (seat.0, entity))
        .collect();
    cast.sort_by_key(|(slot, _)| *slot);
    let cast: Vec<Entity> = cast.into_iter().map(|(_, entity)| entity).collect();
    // Written only on change: this is read every frame by the camera, and a
    // resource touched every frame is a change signal that means nothing.
    if framed.0 != cast {
        framed.0 = cast;
    }
}
