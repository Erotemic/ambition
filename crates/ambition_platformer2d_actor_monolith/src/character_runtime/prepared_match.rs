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
}

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
            if !archetypes.has_brain_key(profile) {
                let mut known = archetypes.brain_keys();
                known.sort();
                seat_problem(format!(
                    "asks for brain profile `{profile}`, which this composition's \
                     CharacterRoster does not have. Known keys: {known:?}. \
                     ⚠ this is the ARCHETYPE table a seated CPU consults, not the \
                     catalog's `brain_presets` — the two share the word `brain` and \
                     nothing else."
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
        // `new_in`, not the test-only `new`: production construction never has a
        // hidden catalog fallback.
        let mut seed = crate::features::ecs::actor_clusters::ActorClusterSeed::new_in(
            authored_sheets,
            catalog,
            archetypes,
            body_id.clone(),
            definition.display_name.clone(),
            // ⭐ the id, not the display name. Two characters may legitimately
            // share a display name; only the id is unique.
            Some(participant.character.as_str()),
            aabb,
            seed_brain,
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
            faction: faction_for(index),
            team: participant
                .team
                .as_ref()
                .map(|team| crate::combat::targeting::MatchTeam::new(team.clone())),
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

/// Alternating sides, so two fighters can actually hit each other:
/// `effective_faction` refuses a strike between same-faction bodies, and a
/// roster seated all one way would stand and stare.
fn faction_for(index: usize) -> crate::combat::components::ActorFaction {
    if index % 2 == 0 {
        crate::combat::components::ActorFaction::Player
    } else {
        crate::combat::components::ActorFaction::Enemy
    }
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
        ControlAuthority::Brain { .. } => crate::features::ecs::enemy_default_brain(&seed.config),
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
                // The MATCH owns this fighter's death, not the world. Without it
                // a KO runs the exploration economy — a bounty coin, a heart, an
                // in-place respawn timer — none of which an arena has a use for.
                crate::combat::components::RulesetOwnsDeath,
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
    if tick.is_some_and(|tick| tick.get() < prepared.effective_from()) {
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
    let mut bodies: Vec<Entity> = Vec::new();
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
        bodies.push(body);
    }

    // **THE MATCH'S RULES, in the same flush that builds the bodies**, so no
    // fighter is ever observable in a state the ruleset did not ask for.
    for body in &bodies {
        let mut entity = commands.entity(*body);
        if let Some(abilities) = rules.abilities {
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
    ));
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
