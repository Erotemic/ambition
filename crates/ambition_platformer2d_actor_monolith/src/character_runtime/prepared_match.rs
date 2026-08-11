//! **A match, decided completely before anything is built.**
//!
//! ## Why this exists
//!
//! Seating used to answer three different questions in one pass — *can this
//! fighter be built*, *what is it physically*, and *who drives it* — one seat at
//! a time, mid-construction, against live authority tables. Every failure it had
//! came from that shape: a seat it could not resolve made it `return` from the
//! whole system, so ONE unbuildable fighter meant no match at all, silently and
//! forever. Jon met it twice in one sitting (2026-08-06): *"only one character
//! spawns in"* — and the one character was the session's home body, which was
//! never a fighter.
//!
//! ```text
//! MatchParticipantRoster   stable, serializable INTENT — what somebody chose
//!         ↓ prepare_match          the ONE place questions are answered
//! PreparedMatch            immutable, owned, authority-free
//!         ↓ activate                infallible, deterministic, replayable
//! ActiveMatch              the receipt
//! ```
//!
//! ## The two invariants that make this worth doing
//!
//! **1. Preparation answers every permanent question.** A character no
//! composition can build, a brain profile nothing registered, a control
//! authority this build cannot honour — each is a named
//! [`MatchPreparationProblems`] entry before a single entity exists. Activation
//! has nothing left to refuse, so it cannot express "still waiting" about
//! something that will never arrive.
//!
//! **2. Activation reads NO character authority.** Not the registry, not the
//! catalog, not the sheets, not the archetype table. Everything construction
//! needs is already in the plan. A plan that looked anything up would resolve
//! against whatever the world holds at activation — which is precisely what a
//! rollback rewind can change underneath it.
//!
//! Together those make activation replayable: rewinding past it removes
//! [`ActiveMatch`](super::ActiveMatch) — `bevy_ggrs` restores ABSENCE, not just
//! earlier values — activation runs again from the same immutable plan, and
//! rebuilds the same cast.
//!
//! ⚠ **the plan is NOT rollback state, and that is deliberate.** It is a
//! DECISION, made before the session it describes; registering it would delete
//! it on a rewind to before it was made and leave activation with nothing to
//! replay. What rewinds is the receipt and the bodies.

use bevy::prelude::*;

use ambition_platformer2d_core::Vec2;

use super::{
    ControllerBinding, MatchParticipantRoster, PreparedCharacterDefinition,
    PreparedCharacterRegistry, RosterProblem,
};

/// **What will drive a fighter, once the fighter exists.**
///
/// ⚠ [`ControllerBinding`] is what a lobby or a save file SAYS; this is what the
/// engine will attach. The difference matters most for the variant that used to
/// be spelled `Human`.
///
/// ⛔ **"a person" and "a local input channel" are not one fact.** Conflating
/// them is how a CPU seat came to size a rollback session: the GGRS handle count
/// was `participants.len()` and the frozen input topology counted
/// `ControllerBinding::Human`, each with a comment claiming to be the
/// authoritative number. A remote human would be a participant with no local
/// channel; a spectator is a participant with no fighter. Neither is
/// expressible while one word means both.
///
/// ⚠ **exactly the two kinds this engine can ATTACH, and no more.**
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
    /// ⛔ **two fields because they are two facts, and collapsing them broke
    /// every sparse lobby** (GPT 5.6, 2026-08-07). A roster names a source —
    /// pad 3, the keyboard — and a lobby is right to keep those numbers stable
    /// when a seat empties. A rollback host requires the opposite: handles
    /// `0..player_count`, dense, no holes. This carried only the first and
    /// handed it to `PlayerSlot`, so a fighter could sit on a channel the
    /// session never opened and receive nothing for the whole match.
    ///
    /// ⭐ **the channel is DERIVED here and nowhere else**, from
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
    ///
    /// ⭐ **the ONE definition of "how many people are playing on this
    /// machine".** Two call sites used to answer it separately and disagreed.
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
    /// ⛔ **every variant, no catch-all — and the catch-all is what this fixes.**
    /// The pass this replaces ended in `_ => { let Some(profile) =
    /// controller.brain_profile() else { return; }; .. }`, and `brain_profile()`
    /// answers `None` for `Replay` and `Policy` BY DESIGN — its own doc says *"a
    /// replay that consulted a brain profile would stop being a replay"*. So a
    /// replay seat silently returned from the whole system and no fighter was
    /// ever built: the identical defect as the unbuildable character, already
    /// present on two more paths and never reported by anything.
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
            // ⛔ **REFUSED, out loud, and that is an improvement.** Both were
            // silently unbuildable before — they fell into the catch-all, had no
            // brain profile by design, and returned from the whole system — so a
            // roster naming one produced an empty stage and no explanation. The
            // engine genuinely has no driver to attach for either; saying so is
            // the honest version of what it already did.
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
    pub character_id: String,
    /// **This BODY's stable identity**, distinct from the character it wears.
    ///
    /// ⛔ a match may legitimately be a MIRROR — two seats, one character — and
    /// this id is what presentation, the anti-clump slot board, the steering
    /// neighbour index and the target/faction maps are ALL keyed on
    /// (`HashMap<String, _>`, every one of them). Keying it on the character made
    /// two fighters one entity to every one of them, and
    /// `spawn_dynamic_feature_visuals` dedupes by id — so one of the pair could
    /// never be drawn.
    pub feature_id: String,
    /// **The owned definition**, so activation can read the physical baseline
    /// without asking the registry what it currently says.
    pub definition: PreparedCharacterDefinition,
    /// **The owned pre-spawn cluster**, built from the character authorities
    /// during preparation and never re-derived.
    ///
    /// ⭐ `ActorClusterSeed` already described itself as *"Owned seed used to
    /// construct the enemy ECS component cluster before spawn"*. The engine had
    /// the right value the whole time; seating simply built one at spawn time
    /// against live tables instead of carrying one.
    pub seed: crate::features::ecs::actor_clusters::ActorClusterSeed,
    /// The body box this fighter was resolved to occupy.
    pub body_px: Vec2,
    pub faction: crate::combat::components::ActorFaction,
    pub team: Option<crate::combat::targeting::MatchTeam>,
    /// What will drive it, attached AFTER the body exists — never a fork in how
    /// the body is built.
    pub authority: ControlAuthority,
}

/// What every fighter in this match plays under.
///
/// On the match rather than decided by construction, for the reason the roster's
/// own fields state: the engine does not get an opinion about a match's economy.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MatchRules {
    pub stocks: Option<u32>,
    pub abilities: Option<ambition_platformer2d_core::AbilitySet>,
    pub opens_suspended: bool,
    /// **How long the opening ceremony holds the cast**, in simulation ticks.
    ///
    /// `0` means no ceremony: a suspended cast is released on the tick it is
    /// built, which is what every match did before this existed and is the
    /// honest reading of `opens_suspended` for a ruleset with no opening.
    ///
    /// ⛔ **TICKS, not seconds, and that is a determinism requirement rather
    /// than a taste.** The release is a comparison against the sim clock, so a
    /// rollback re-runs it and reaches the same answer; a wall-clock timer
    /// would drift a peer's release by a frame and diverge the whole cast.
    pub opening_countdown_ticks: u32,
}

/// **Where an opening ceremony has got to** — derived from the clock, never
/// stored.
///
/// ⭐ **no ticking timer anywhere, on purpose.** A countdown is the obvious
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
    pub fn death_policy(&self) -> ambition_characters::actor::DeathPolicy {
        if self.stocks.is_some() {
            ambition_characters::actor::DeathPolicy::Unbounded
        } else {
            ambition_characters::actor::DeathPolicy::default()
        }
    }

    /// **How many BEATS the ceremony has**, one per counted number.
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
    /// ⚠ **a match with no ceremony is `Live` from tick zero**, which is what
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
}

/// How many numbers an opening ceremony counts: 3, 2, 1.
///
/// A constant rather than a rule field because it is the GENRE's shape — every
/// platform fighter counts three — while how LONG each number holds is the
/// ruleset's call and lives in `opening_countdown_ticks`.
pub const OPENING_BEATS: u32 = 3;

/// **The match, resolved.**
#[derive(Resource, Clone, Debug)]
pub struct PreparedMatch {
    seats: Vec<PreparedSeat>,
    rules: MatchRules,
    /// The [`PreparedCharacterRegistry`] generation these seats were resolved
    /// against.
    ///
    /// ⚠ **a staleness ASSERTION, never a re-resolution trigger.** If the
    /// published cast has moved on, this plan describes a world that no longer
    /// exists and the honest response is to say so. Silently re-resolving would
    /// put a live authority back inside activation, which is the one property
    /// this module exists to remove.
    cast_generation: super::CharacterCatalogGeneration,
    /// The frozen seat topology the ROSTER was agreed under, carried through so
    /// the activation can cite it.
    ///
    /// ⚠ carried rather than re-read: a later disagreement about who is playing
    /// is only answerable if the live match can say which topology decided it,
    /// and asking the world at activation would answer with whatever is true
    /// then rather than with what this plan was built from.
    seat_topology: Option<u64>,
    /// **The first `SimTick` this plan may build on.**
    ///
    /// ⛔ **WHEN a decision takes effect is part of the decision, and leaving it
    /// out is a determinism hole.** The plan is deliberately not rollback state,
    /// so it survives a rewind — but its ARRIVAL did not: the original frame 8
    /// ran with no plan and activated on 9, while the RESIMULATED frame 8 found
    /// the plan already standing and activated on 8. Every actor component then
    /// diverged at once, which is the signature of a cast that exists in one
    /// run of a frame and not the other, and GGRS reported it as a checksum
    /// mismatch three frames wide.
    ///
    /// ⭐ stamping the tick makes activation a pure function of the plan and the
    /// clock, which is the property that lets a rewind reconstruct the SAME
    /// match instead of a similar one built a frame early.
    effective_from: u64,
    /// **The gameplay session this plan was decided FOR.**
    ///
    /// ⛔ **a plan is about ONE activation of one route, and without this it was
    /// about the process.** Jon, 2026-08-07: *"a fresh restart and then player
    /// vs cpu works, but the next match does not work … there is some bad
    /// global state."* Returning to the select screen and starting a second
    /// match left the FIRST match's plan and receipt standing, so preparation
    /// returned early (a plan exists) and activation returned early (a receipt
    /// exists) and the new match seated nothing at all.
    ///
    /// ⚠ **content cannot answer this.** The obvious repair — re-prepare when
    /// the roster differs — fails on the case that actually happens: a rematch
    /// with the same two picks publishes an IDENTICAL roster. What changed is
    /// not what was chosen, it is that this is a different SESSION.
    session: Option<ambition_platformer2d_shared_tangle::lifecycle::SessionScopeId>,
    /// **Which experience's roster this plan was built from.**
    ///
    /// ⭐ **inherited, never authored.** It is copied from
    /// [`MatchParticipantRoster::published_by`] in [`prepare_match`], so a
    /// provider that already says who it is on the roster says it here for free
    /// — and a plan whose owner disagreed with its roster's would be describing
    /// a match nobody asked for.
    ///
    /// ⚠ **this is what makes teardown safe in a host that runs more than one
    /// game.** `PreparedMatch` is a GLOBAL resource shared by every experience
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

    /// The first `SimTick` this plan may build on. See [`Self::effective_from`].
    pub fn effective_from(&self) -> u64 {
        self.effective_from
    }

    pub fn rules(&self) -> &MatchRules {
        &self.rules
    }

    pub fn cast_generation(&self) -> super::CharacterCatalogGeneration {
        self.cast_generation
    }

    /// The frozen seat topology this plan was agreed under, if anything had an
    /// opinion when the roster was built.
    pub fn seat_topology(&self) -> Option<u64> {
        self.seat_topology
    }

    /// **Was this plan built from `experience_id`'s roster?**
    ///
    /// The question a shell experience scope asks before tearing a plan — and
    /// the activation that came from it — down. An UNOWNED plan (no publisher on
    /// its roster) answers `false` to everyone, which leaks rather than deletes:
    /// the safe direction, because the cost of a leak is one stale plan and the
    /// cost of a wrong delete is another game's live match.
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
            cast_generation: super::CharacterCatalogGeneration::default(),
            seat_topology: None,
            // A teardown test cares about the OWNER and nothing else: tick zero
            // is a plan every clock has already reached, and no session means
            // this plan matches the composition a bare test world has.
            effective_from: 0,
            session: None,
            published_by: experience_id.map(str::to_owned),
        }
    }

    /// **Which local source drives which channel in this match.**
    ///
    /// ⛔ **NOT `seats().len()`.** That number sized the GGRS session while the
    /// frozen input topology used a different one. A CPU is a participant and
    /// not a channel; a match of two CPUs needs none at all.
    ///
    /// ⛔ **and not a count either.** `plan.channels()` is the handle count, and
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

/// **What a composition could not answer about a roster.**
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

/// **Resolve a roster into a match, or say exactly why it cannot be one.**
///
/// ⭐ **every problem, not the first.** A lobby wants to be told everything that
/// is wrong with its choice; returning on the first would make fixing a
/// four-seat roster a four-attempt guessing game.
#[allow(clippy::too_many_arguments)]
pub fn prepare_match(
    roster: &MatchParticipantRoster,
    registry: &PreparedCharacterRegistry,
    catalog: &ambition_characters::actor::character_catalog::CharacterCatalog,
    authored_sheets: &ambition_sprite_sheet::character::sheets::AuthoredSheets,
    archetypes: &crate::features::CharacterRoster,
    // **THE PUBLISHED CONTROLLER POLICIES**, asked before the archetype table
    // (Jon's second redirect, P4).
    //
    // ⭐ a match's public API is *character + controller + team*, and the
    // implementation resolved the controller half through `CharacterRoster` —
    // an ENEMY ARCHETYPE table — so Smash was not yet proving the controller
    // architecture it advertises. A CPU seat naming a published profile resolves
    // here; one naming an archetype brain key still falls through, which is what
    // shrinks as profiles get published.
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
    let rules = MatchRules {
        stocks: roster.fighter_stocks,
        abilities: roster.fighter_abilities,
        opens_suspended: roster.opens_suspended,
        opening_countdown_ticks: roster.opening_countdown_ticks,
    };
    let death_policy = rules.death_policy();

    let mut problems: Vec<RosterProblem> = Vec::new();
    let mut seats: Vec<PreparedSeat> = Vec::new();

    // **WHO IS PLAYING, AND ON WHAT — asked once, by the roster.**
    //
    // Every seat below reads its channel out of this rather than deriving one,
    // so the number that sizes the GGRS session, the number the frozen topology
    // maps to a controller, and the number the fighter's `PlayerSlot` carries
    // are one number by construction.
    let plan = roster.local_channel_plan();
    // ⛔ **ONE CONTROLLER CANNOT DRIVE TWO FIGHTERS.** Refused before the match
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

        // **THE POPULATION THAT CAN ACTUALLY BE BUILT.**
        //
        // ⛔ this check did not exist anywhere. `unsatisfiable_seats` validated
        // brain profiles and said of character ids: *"`PreparedCharacterRegistry`
        // answers that, refuses on its own, and asking twice would put two
        // authorities on one question."* It did refuse on its own — with a bare
        // `return` from inside the construction pass, no log and no record — so
        // the failure had no words anywhere in the engine, and a select screen
        // that filtered its grid by the CATALOG could offer eight fighters this
        // host cannot seat.
        let Some(definition) = registry.get(&participant.character) else {
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

        // **THE DENSE CHANNEL FOR THIS SEAT**, taken from the plan in step with
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

        // **TWO CLAIMANTS ON ONE LOCAL CHANNEL, NAMED HERE INSTEAD OF PANICKING
        // FOUR SYSTEMS DEEP.**
        //
        // A session that lowers a home avatar has already given that body the
        // session's local control channel. A match seat asking for a local
        // channel in the same session is a SECOND claimant, and the engine's
        // answer used to be an adopted body: seat zero silently became the home
        // avatar. With construction unified, both bodies get built and
        // `resolve_controlled_subject` aborts the frame with
        // *"2 entities carry Brain::Player(PRIMARY)"* — true, useless, and
        // reported by a system that had nothing to do with the mistake.
        //
        // ⭐ a match experience declares `InitialBodyPolicy::NoInitialBody`;
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

        // A CPU's profile must name a rig this composition registered.
        //
        // `spec_for_brain` falls back to a generic `combatant` row for an
        // unknown key — defensible for a placement, and for a match seat it
        // means the fighter that arrives is not the fighter the roster asked
        // for. It cost an hour in this demo on 2026-07-31, with a diagram.
        if let ControlAuthority::Brain { profile } = &authority {
            if seat_brain_profile(profile, profiles, archetypes).is_none() {
                let mut known = archetypes.brain_keys();
                known.sort();
                let published: Vec<&str> = profiles.map(|p| p.ids().collect()).unwrap_or_default();
                seat_problem(format!(
                    "asks for brain profile `{profile}`, which is neither a \
                     published controller policy nor a key in this composition's \
                     CharacterRoster. Published policies: {published:?}. Archetype \
                     keys: {known:?}. ⚠ the archetype table is the LEGACY half — a \
                     seat should name a published policy."
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

        // **THE AUTHORED PHYSICAL IDENTITY**, read through `PhysicalBaseline`
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
        // ⛔ **THE SEAT, not the character.** A mirror match is two bodies
        // wearing one character, and every id-keyed index in the actor runtime
        // would collapse them into one: `entity_to_id`, the anti-clump slot
        // board's `requests`, `faction_by_id` and `target_entity_by_id`
        // (`features/ecs/actors/update.rs`), plus `ActorIdentity` itself. The
        // ART still resolves from the character, which is exactly what
        // `art_identity` is for — a body whose id is not its costume's name.
        let body_id = format!("{}#seat{index}", participant.character);
        // ⭐ **CHARACTER-FIRST.** This built through `new_in`, which starts
        // `roster.spec_for_brain(&brain)` — so every fighter on the grid was
        // physically a `combatant` with a character painted over it, and the
        // seat then took the health and the weight back one field at a time.
        // Jon's brief forbids exactly that shape: *"No ordinary constructor
        // should first build an `ArchetypeSpec` creature and then patch the
        // character over it."*
        //
        // The CONTROLLER's policy is still resolved from the roster below and
        // handed in as a value — a profile is a decision, and this constructor
        // takes it rather than looking up a body to get one.
        let profile = match &authority {
            ControlAuthority::Brain { profile } => {
                seat_brain_profile(profile, profiles, archetypes)
            }
            // A human seat's body carries no autonomous policy at all. It used
            // to inherit `combatant`'s, which nothing read and which said this
            // body would chase somebody.
            _ => None,
        }
        .unwrap_or_default();
        // ⭐ **the character, as ONE value, with the MATCH's overrides named.**
        // A seat differs from a room placement in exactly three ways, and each
        // is a line rather than a parameter buried in a list of fourteen.
        let mut body = definition.seat_blueprint(ambition_platformer2d_core::MAX_RUN_SPEED);
        // The character's own pool, or the reference body's. `baseline` already
        // folded the definition's authored maximum.
        body.max_health = baseline.max_health_over(1);
        // The policy is the MATCH's decision, not the character's default: a
        // human seat's body carries none at all.
        body.autonomous_profile = Some(profile);
        // ⚠ a MATCH seat is never a practice target, whatever the character
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
        // The seed's own pool stands for a character that authored none — which
        // used to be impossible to express, because an unauthored `Vitals`
        // defaulted to a one-hit pool and every seated fighter silently took it.
        seed.health =
            ambition_characters::actor::BodyHealth::new(ambition_characters::actor::Health::new(
                baseline.max_health_over(seed.health.health.max.max(1)),
            ))
            .with_policy(death_policy);
        // **THE AUTHORED KNOCKBACK WEIGHT**, onto the seed for the same reason
        // the pool is: `into_components` projects `config.tuning.weight` onto
        // the combat-owned `CombatTuning` the damage paths read, so setting it
        // here is what makes a heavy fighter heavy from its first frame rather
        // than after a re-wear. A character that authors none keeps its roster
        // archetype's, which is every character that has not thought about it.
        if let Some(weight) = baseline.knockback_weight() {
            seed.config.tuning.weight = weight;
        }
        seed.kin.facing = facing;

        // ⛔ **ONE BODY, ONE BOX — and for a day it was two.** This recorded
        // `hint_px`, so a fighter's POSE and `CenteredAabb` were the 30x48
        // placeholder while its `BodyKinematics` carried the authored sprite
        // collision. Hit tests read the pose, so a seated fighter could not
        // reach another one: `fb6_shadow_fidelity` measured the shadow model
        // predicting a hit at a 34px gap with 51px of reach and the real sim
        // landing nothing, and `duel_arena` watched two fighters throw zero
        // melee swings across a whole bout.
        //
        // ⚠ the placeholder is still right for a character that authors no
        // size — it is deliberately small so an unresolved body looks wrong
        // rather than plausible — but it must not outrank a size the seed
        // actually resolved.
        let body_px = seed.kin.size;
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
            faction: crate::combat::components::ActorFaction::Player,
            team: Some(team_for(index, participant.team.as_ref())),
            authority,
        });
    }

    if !problems.is_empty() {
        return Err(MatchPreparationProblems { problems });
    }

    // ⚠ **WHERE THE CAMERA LOOKS IS NOT DECIDED HERE, YET.** A draft of this
    // module returned a `MatchViewPolicy` (follow the first local seat, else
    // frame the whole cast) and nothing read it. An unread value is dead code
    // dressed as intent — the same objection this module makes to control
    // authorities nothing can attach — so it is not here.
    //
    // ⭐ **WHERE THE CAMERA LOOKS IS ANSWERED NOW**, and the answer is not here:
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

/// **What this fighter may do: the CHARACTER supplies the verbs, the RULESET
/// takes some away.**
///
/// ⛔ **the direction is the architecture, and it used to run backwards.** A
/// match declared one flat set — *"every fighter in this match has the same
/// verbs"* — and stamped it onto every body, because seats disagreed wildly
/// (an adopted seat had fly, blink and shield; a spawned one had jump) and
/// nothing else could level them. That levelling is the reason a Puppy Slug in
/// a fighter seat would jump and dash like a humanoid: the match manufactured
/// capabilities the body never had.
///
/// ```text
/// character authors verbs   +   ruleset masks   ⇒   what this body may do
/// authored, mode says nothing        →  the character's own kit
/// authored, mode declares a set      →  the intersection: a mode may FORBID
/// unauthored, mode declares a set    →  the mode's set  (migration bridge)
/// unauthored, mode says nothing      →  whatever construction built
/// ```
///
/// ⚠ **the third row is a bridge and is meant to shrink.** Almost every
/// character in the repo authors no verbs, so removing it today would strip the
/// Smash cast down to whatever the archetype happened to grant. It disappears
/// one character at a time, and the day it is unreachable this function is two
/// lines shorter.
fn seat_abilities(
    seat: &PreparedSeat,
    rules: &MatchRules,
) -> Option<ambition_platformer2d_core::AbilitySet> {
    match (seat.definition.abilities, rules.abilities) {
        (Some(authored), Some(mask)) => Some(authored.intersect(mask)),
        (Some(authored), None) => Some(authored),
        (None, mode) => mode,
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

/// **A seat's team, which is what decides who it may fight.**
///
/// ⛔ **this replaces `faction_for(index)`, which handed seats alternating
/// FACTIONS — `Player, Enemy, Player, Enemy` — so that two fighters could hit
/// each other at all.** That was a hack with a real cost: seat 2 of a
/// four-player free-for-all was an ally of seat 4 and could not touch it, a
/// character's authored world allegiance was overwritten by where it happened to
/// sit, and the AI's target selection (which read the faction matrix) and the
/// damage rule (which read teams) disagreed about the same pair.
///
/// ⭐ **a match relationship is what a match runs on**, so every seat gets one.
/// An authored team is honoured; a seat with none gets a team of its own, which
/// is the literal statement of free-for-all — everyone opposes everyone. The
/// authored faction is then free to stay what the CHARACTER says it is.
fn team_for(index: usize, authored: Option<&String>) -> crate::combat::targeting::MatchTeam {
    crate::combat::targeting::MatchTeam::new(
        authored
            .cloned()
            .unwrap_or_else(|| format!("seat {}", index + 1)),
    )
}

/// **Build one fighter's body. Infallible, and reads no authority.**
///
/// ⭐ **ONE path for every fighter, whatever drives it.** The function this
/// replaces had two: a local player's seat ADOPTED the session's existing body
/// and a CPU's seat SPAWNED a new one, and that fork is what every symptom of
/// the 2026-08-06 report came from — the costume handshake, seat 0's privilege,
/// the health/box/mass/ability divergences unified one at a time over three
/// weeks, and the impossibility of a match with nobody local in it. Control is
/// attached to the finished body by [`bind_seat_control`]; it does not get to
/// decide how the body is made.
/// **The controller policy a CPU seat names**, published first, archetype second.
///
/// ⭐ **the direction is the migration** (Jon's second redirect, P4). A match's
/// public API is *character + controller + team*, and the controller half was
/// resolved through `CharacterRoster` — an enemy ARCHETYPE table — so a seat
/// asking for a policy got one by way of a body definition. A published
/// `BrainProfile` is what a controller policy IS; the archetype arm is the
/// legacy half and shrinks as policies are published.
///
/// ⚠ the reference is provider-relative and this resolves it with no provider to
/// resolve against, so an already-qualified name works and a bare one is tried
/// verbatim. A seat names a policy the MATCH published, not one a character owns.
fn seat_brain_profile(
    key: &str,
    profiles: Option<&ambition_characters::actor::character_catalog::BrainProfileRegistry>,
    archetypes: &crate::features::CharacterRoster,
) -> Option<ambition_characters::brain::BrainProfile> {
    profiles
        .and_then(|profiles| {
            profiles
                .get(&ambition_entity_catalog::BrainProfileId::new(key))
                .copied()
        })
        .or_else(|| {
            archetypes
                .has_brain_key(key)
                .then(|| archetypes.brain_profile_for(key))
                .flatten()
        })
}

fn realize_seat(
    commands: &mut Commands,
    session_scope: ambition_platformer2d_shared_tangle::lifecycle::SessionSpawnScope,
    seat: &PreparedSeat,
) -> Entity {
    let seed = seat.seed.clone();
    let at = seed.kin.pos;
    let facing = seed.kin.facing;
    let centered = ambition_platformer2d_core::CenteredAabb::from_center_size(at, seat.body_px);
    let motion_model = seed.config.tuning.motion_model();
    let (identity, _seed_disposition, combat, intent, cooldowns) =
        crate::features::ecs::enemy_component_snapshot(&seed);
    // A match participant is a COMBATANT, whatever drives it. The disposition
    // the seed derives follows the authored brain, and a local-input seat
    // authors `Passive` — `apply_actor_hit` reads the disposition first, and a
    // peaceful body takes NO health damage. A seated fighter was once
    // unkillable, and the symptom was a swing that connected, played its sound
    // and did nothing.
    let disposition = crate::combat::components::ActorDisposition::Hostile;
    // A default action set, matching what an enemy spawn does before its
    // archetype fills one in. The character's real attacks arrive from
    // `apply_worn_character_gameplay`, the ONE writer for a worn body's moves.
    let action_set = ambition_characters::brain::ActionSet::default();
    // **THE BRAIN THIS SEAT WILL HAVE, chosen once and spawned WITH the body.**
    //
    // ⛔ this used to be the archetype's brain unconditionally, with a follow-up
    // `commands.entity(body).insert(Brain::Player(..))` for a local seat. Two
    // steps to say one thing, and this repo's own rule about that is
    // "an authority that needs a FOLLOW-UP CALL — the second step belongs inside
    // the first". Here the cost is concrete: for one command-queue ordering
    // there exists a world in which a seated fighter is AI-brained, and a
    // rollback snapshot that captures THAT world restores it forever, because
    // activation is one-shot and never rebinds.
    let derived_brain = match &seat.authority {
        // ⭐ **through the one correspondence**, not `PlayerSlot(raw)`: the seat
        // the simulation reads is a projection of the participant channel, and
        // `participant_seat` is where that projection lives.
        ControlAuthority::LocalInput { channel, .. } => ambition_characters::brain::Brain::Player(
            crate::participant_seat::player_slot_of(*channel),
        ),
        ControlAuthority::Brain { .. } => {
            crate::features::ecs::enemy_default_brain(&seed.config, seed.body.0.abilities.abilities)
        }
    };
    let combat_kit = crate::combat::components::CombatKit::from_action_set(&action_set);
    let cluster = seed.into_components();
    use ambition_platformer2d_shared_tangle::lifecycle::SpawnSessionScopedExt;
    let body = commands
        .spawn_session_scoped(
            session_scope,
            (
                crate::features::EnemyActorBundle::new(
                    crate::features::FeatureBaseBundle::new(
                        // ⛔ **THE SEAT, not the character.** This passed
                        // `character_id`, so two fighters wearing one character
                        // WERE ONE FEATURE: `spawn_dynamic_feature_visuals`
                        // dedupes by id and drew only the first, and the slot
                        // board, the steering neighbour index and the
                        // target/faction maps are all `HashMap<String, _>` keyed
                        // on it — a mirror match corrupted every one of them.
                        //
                        // ⚠ it was UNREACHABLE before and is not a new mistake so
                        // much as a newly possible one: seat 0 used to be the
                        // ADOPTED player body, which carries no `FeatureId` at
                        // all, so a match could never hold two seated features.
                        // Unifying construction is what made two of them exist —
                        // the same shadow the registration gap and the couch
                        // crosstalk came out of.
                        //
                        // ⭐ a body's render/simulation identity is the BODY.
                        // What it WEARS is `WornCharacter`, and a mirror match is
                        // an ordinary thing a platform fighter must allow.
                        seat.feature_id.as_str(),
                        seat.definition.display_name.clone(),
                        centered,
                    ),
                    identity,
                    disposition,
                    seat.faction,
                    crate::features::ActorPose::from_parts(at, seat.body_px / 2.0, facing),
                    combat_kit,
                    crate::features::ActorAggression::hostile(),
                    combat,
                    intent,
                    cooldowns,
                )
                .with_motion_model(motion_model),
                cluster,
                action_set,
                derived_brain,
                Name::new(seat.definition.display_name.clone()),
                crate::combat::moveset::ActorMoveset(Default::default()),
                // The body WEARS the character. Everything that makes it that
                // fighter rather than a generic actor follows from this one
                // component.
                ambition_characters::actor::WornCharacter::new(seat.character_id.as_str()),
                // ⭐ **and it ASKS for the template to be applied.** Seating used
                // to rely on the persona derive noticing a fresh
                // `WornCharacter` through its change tick; that edge is gone
                // (Jon's redirect §2), so the one writer that needs it says so.
                // ⚠ a seat genuinely needs the derive rather than the
                // construction grant: `seat_blueprint` resolves the BODY, and
                // the match's own kit (`MatchParticipant::action_set`) is
                // layered by that derive.
                ambition_characters::actor::RecharacterizeBody,
                // The MATCH owns this fighter's death, not the world. Without it
                // a KO runs the exploration economy — a bounty coin, a heart, an
                // in-place respawn timer — none of which an arena has a use for.
                crate::combat::components::RulesetOwnsDeath,
                // And it is IN the fight — which is a different fact from whose
                // business its death is, and the one every other combat system
                // actually wants. Removed again when the fighter is eliminated.
                crate::combat::components::ActiveCombatant,
                // WITHOUT THIS THE FIGHTER IS INVISIBLE: the authored render pass
                // only spawns visuals for `spec.enemy_spawns`, so a directly
                // staged actor would render nothing.
                crate::combat::components::RuntimeStagedActor,
                ambition_characters::brain::ActorControl::default(),
                ambition_characters::actor::attack_gesture::AttackGestureState::default(),
                ambition_characters::actor::attack_gesture::AttackGestureTuning::default(),
                ambition_characters::actor::attack_gesture::ResolvedAttackGesture::default(),
            ),
        )
        .id();
    // **THE AUTHORED MASS.** Conditional: a character that authored none must
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
    body
}

/// **Attach the driver to a finished body.**
///
/// The whole of what "who plays this fighter" now costs. It runs after the body
/// exists and can therefore be the same two lines for every fighter, which is
/// the property that made a CPU-only match impossible to express before.
fn bind_seat_control(commands: &mut Commands, body: Entity, authority: &ControlAuthority) {
    match authority {
        ControlAuthority::LocalInput { .. } => {
            // ⚠ the BRAIN is not here — it is in the spawn bundle, deliberately;
            // see `realize_seat`. What is left is the local-input plumbing that
            // has no archetype counterpart to race with.
            commands.entity(body).insert((
                crate::control::components::LocalPlayer,
                crate::control::components::PlayerInputFrame::default(),
            ));
        }
        // The seed already carries the archetype's brain, derived in
        // `realize_seat` exactly as the enemy spawner derives it.
        ControlAuthority::Brain { .. } => {}
    }
}

/// **Resolve the published roster into a plan, once.**
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
    archetypes: Res<crate::features::CharacterRoster>,
    // The published controller policies a CPU seat may name (P4). `Option` like
    // every other reader: a composition that publishes none is ordinary.
    profiles: Option<Res<ambition_characters::actor::character_catalog::BrainProfileRegistry>>,
    geometry: Option<
        ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<
            ambition_platformer2d_core::RoomGeometry,
        >,
    >,
    // **WHAT THIS SESSION DECLARED ABOUT A HOME AVATAR.** Optional because a
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
    // ⛔ **`Option`, for the reason spelled out on `FramedCast` below.** A plain
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
    // ONE plan per match — but a plan belongs to ONE SESSION, and this used to
    // read `if prepared.is_some() { return; }`, which made it one plan per
    // PROCESS. Re-preparing every tick would rebuild the seeds under a live
    // match and re-resolve them against a possibly-republished registry, which
    // is the authority-in-activation this module exists to remove; re-preparing
    // for a NEW session is the opposite of that — it is the only way the second
    // match of a sitting gets a plan at all.
    if prepared.is_some_and(|prepared| prepared.session() == session) {
        return;
    }
    let (Some(roster), Some(registry), Some(geometry)) = (roster, registry, geometry) else {
        return;
    };
    if roster.participants.is_empty() {
        return;
    }
    // ⛔ **A PROPOSED ROSTER DOES NOT PREPARE.** A roster nobody has agreed to
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
        &archetypes,
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
            // ⛔ **OUT LOUD, in every build.** The per-seat `debug_assert!` this
            // class of check used to rely on was invisible in release, which
            // reintroduced the very bug it guarded.
            bevy::log::error!(
                target: "ambition_platformer2d::match_preparation",
                "this composition cannot prepare the published match: {problems}"
            );
            commands.insert_resource(problems);
        }
    }
}

/// **Build the whole cast, in one flush. Infallible.**
///
/// Every permanent question was answered by [`prepare_the_match`], so there is
/// nothing here to refuse and no way for this to half-apply: either the match
/// activates completely on one tick or it has not started.
///
/// ⚠ **it BORROWS the plan.** Consuming it would make a rewind to before
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
) {
    let Some(prepared) = prepared else {
        return;
    };
    let session = active_session
        .as_deref()
        .and_then(ambition_platformer2d_shared_tangle::lifecycle::ActiveSessionScope::current);

    // **A PLAN BELONGS TO ONE SESSION, AND MAY ONLY BE BUILT INTO THAT ONE.**
    //
    // ⛔ preparation stamps the session and NOTHING CHECKED IT HERE. The stamp
    // was consulted only by preparation, deciding whether to re-prepare — so a
    // plan that outlived its session was structurally able to realize its cast
    // into the next one. Saying it here makes that unreachable rather than
    // unlikely.
    if prepared.session() != session {
        return;
    }

    // **HAVE I ALREADY BUILT *THIS* MATCH?** — asked of the receipt's identity,
    // not of the world's fighters.
    //
    // ⛔ this read `active.is_some() && !already_seated.is_empty()`, and the
    // presence half was wrong for a platform fighter. `ActiveMatch` with no
    // `MatchSeat` bodies is not a dead session's paperwork: eliminated fighters
    // are DESPAWNED, and a simultaneous final-stock ring-out is a supported
    // draw, so a match that has legitimately just finished sits at zero seats
    // for the whole time the winner card is up. Activation would have fallen
    // through and rebuilt the cast with fresh stocks underneath the
    // announcement. (GPT 5.6 review, 2026-08-07 — caught by reading, not by
    // playing.)
    //
    // ⭐ the receipt names its session now, so the question is one comparison
    // and fighter presence cannot affect the answer. A receipt for a DIFFERENT
    // session is stale and this same call replaces it below — activation
    // remains the single writer, which is what the
    // `a-second-writer-of-a-match-global-must-answer-ownership` contract asks
    // for, and it can answer whose receipt it is replacing because the receipt
    // says so.
    if active.is_some_and(|active| active.session() == prepared.session()) {
        return;
    }
    // ⛔ **NOT "the first tick the plan exists".** The plan does not rewind, so
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
    // ⚠ this is the SPAWN policy, not the identity above: a composition with no
    // session lifecycle at all resolves to `UNSCOPED` and still builds, and its
    // plan is stamped `None` so the identity comparison holds too.
    let Some(session_scope) =
        ambition_platformer2d_shared_tangle::lifecycle::SessionSpawnScope::for_optional_active_session(
            active_session.as_deref(),
        )
    else {
        return;
    };
    let occupied: std::collections::BTreeSet<usize> =
        already_seated.iter().map(|seat| seat.0).collect();

    let rules = prepared.rules();
    let mut bodies: Vec<(Entity, &PreparedSeat)> = Vec::new();
    for seat in prepared.seats() {
        if occupied.contains(&seat.seat) {
            continue;
        }
        let body = realize_seat(&mut commands, session_scope, seat);
        let mut seated = commands.entity(body);
        seated.insert(super::MatchSeat(seat.seat));
        if let Some(team) = seat.team.clone() {
            seated.insert(team);
        }
        bind_seat_control(&mut commands, body, &seat.authority);
        bodies.push((body, seat));
    }

    // **THE MATCH'S RULES, in the same flush that builds the bodies**, so no
    // fighter is ever observable in a state the ruleset did not ask for.
    for (body, seat) in &bodies {
        let mut entity = commands.entity(*body);
        if let Some(abilities) = seat_abilities(seat, rules) {
            // `AbilityBase` too, not only the effective set: the effective set
            // is `base ∩ editable_mask`, recomputed every frame for a
            // player-driven body, so writing only `BodyAbilities` would be
            // undone next tick by a system behaving correctly.
            entity.try_insert((
                ambition_platformer2d_core::BodyAbilities::new(abilities),
                ambition_platformer2d_core::AbilityBase::new(abilities),
            ));
        }
        if let Some(stocks) = rules.stocks {
            entity.try_insert(crate::combat::components::FighterStocks::new(stocks));
        }
        if rules.opens_suspended {
            entity.try_insert(ambition_characters::brain::ScriptedControl);
        }
    }

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

/// **Release the opening hold when the ceremony ends — every seat on ONE tick.**
///
/// `opens_suspended` stamps `ScriptedControl` on every fighter in the flush that
/// creates them, so no body is ever observable in a state the ruleset did not
/// ask for. This is the other half: the tick the hold comes off.
///
/// ⛔ **it lived in the Smash STAGE and released on "the match is live"**, which
/// was the honest reading while no ruleset had a ceremony — and it meant the
/// countdown could never be added without also moving this, because a stage
/// system that fires on activation cannot be talked out of it by a rule. The
/// release belongs to match FLOW, next to the thing that applied the hold.
///
/// ⭐ **atomic by construction.** Every held seat is released in one command
/// flush against one clock reading, so there is no tick on which one fighter can
/// act and another cannot — the property a countdown exists to give and the one
/// a per-fighter timer would quietly lose.
pub fn release_the_opening_hold(
    mut commands: Commands,
    active: Option<Res<super::ActiveMatch>>,
    prepared: Option<Res<PreparedMatch>>,
    held: Query<
        Entity,
        (
            With<super::MatchSeat>,
            With<ambition_characters::brain::ScriptedControl>,
        ),
    >,
    tick: Option<Res<ambition_time::SimTick>>,
) {
    let (Some(active), Some(prepared)) = (active, prepared) else {
        return;
    };
    // ⛔ **A CEREMONY THIS RULESET DID NOT DECLARE IS NOT THIS SYSTEM'S TO
    // END.** `opens_suspended` with no countdown means somebody ELSE owns the
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
    for body in held.iter() {
        commands
            .entity(body)
            .try_remove::<ambition_characters::brain::ScriptedControl>();
    }
}

/// **Declare the match's cast as what the camera frames.**
///
/// ⛔ **the missing reader.** An earlier draft of this module computed a
/// `MatchViewPolicy` and it was deleted as "a value nothing reads" — which was
/// the right objection to the wrong half. The value was not the mistake; having
/// no consumer was. Jon's run found the consequence directly: *"when I seated 2
/// CPUs and pressed start, nothing shows up. No stage."* The camera resolves its
/// subject from `ControlledSubject` and returns without one, so a match with no
/// local participant framed nothing at all.
///
/// ⭐ **published, not guessed.** The camera could scan for `MatchSeat` bodies
/// itself, and then presentation would own a question about what a session is
/// FOR. The match already knows; saying so is one line and keeps the resolver
/// generic — a cutscene or a replay viewer publishes the same resource.
///
/// ⚠ **ordered by SEAT.** A `Vec<Entity>` built in query order is a different
/// vector frame to frame, and this one is read by a framing computation whose
/// output a person looks at.
///
/// Runs in `Update`: it is a projection of simulation state for presentation to
/// consume, not simulation state, so it must not be written inside the rollback
/// window.
pub fn declare_the_match_cast_as_the_view(
    active: Option<Res<super::ActiveMatch>>,
    seats: Query<(Entity, &super::MatchSeat)>,
    // ⛔ **`Option`, and it is not defensive.** A plain `ResMut` here panicked 53
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
