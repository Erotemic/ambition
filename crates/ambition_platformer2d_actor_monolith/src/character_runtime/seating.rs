//! **Seating a registered character as a body.** (C4 slice 1)
//!
//! [`MatchParticipantRoster`](super::MatchParticipantRoster) has been able to say
//! WHO is in a match since §7.8, and all it could do with that was project art
//! demand. Nothing turned a participant into a body, so the closest thing to a
//! versus mode was a test that hand-assembled two fighters — which is why C4 read
//! "no versus mode in a running game; the fight is proven only in a test".
//!
//! This is the missing verb. It is deliberately NOT a new construction path:
//! seating builds the same [`ActorClusterSeed`] the room stager and the
//! programmatic spawn seam build, so a seated fighter is an ordinary actor body
//! and every system that already works on one keeps working. What seating adds is
//! the join from a `CharacterDefinition` — registered, prepared, art-demanded — to
//! a body wearing it.
//!
//! ## Why not `SpawnActorRequest`
//!
//! That seam exists and is the right one for an ENEMY: it resolves an archetype
//! out of the roster fragment by name. A match participant is a CHARACTER, and a
//! character registered only through `register_character` has no roster archetype
//! to resolve — which is exactly the population C3 exists for. Routing seating
//! through the enemy spawner would mean every fighter needs a second declaration
//! in a roster fragment, which is the duplication the character seam removed.
//!
//! ## Human seats ADOPT, they do not spawn
//!
//! A stage already has a primary player body: the session's `StartingCharacter`
//! spawns one, and it is what the camera follows and what device input drives. A
//! `Human` participant is that body, not a second one beside it.
//!
//! Getting this wrong is not subtle and it shipped for an hour: the versus stage
//! seated a CPU `mary_o` while the session had already spawned a player body
//! wearing `mary_o`, and the arena held two of her. The test passed because it
//! asserted both fighters were PRESENT rather than that the roster was the cast —
//! presence is the assertion you write when you have not looked at the screen.
//!
//! So a human seat binds to the existing body and spawns nothing. That is also
//! why it is the fix for the duplicate rather than a separate feature: a stage's
//! starting character IS a seat, and seating had no way to say so.

use bevy::prelude::*;

use ambition_platformer2d_core::Vec2;

use super::{MatchParticipant, MatchParticipantRoster, PreparedCharacterRegistry};

/// Half the horizontal gap between two seated fighters, in world pixels.
///
/// Wide enough that neither starts inside the other's authored silhouette —
/// seating two bodies overlapping would resolve on the first tick and read as a
/// physics bug rather than a seating one.
const SEAT_SPREAD_PX: f32 = 96.0;

/// The body box a seated fighter gets before its character projects its own.
///
/// A placeholder ON PURPOSE, and a small one: `SpritePosedBody` replaces it from
/// the character's authored sheet within a tick, and
/// `project_prepared_character_definitions` puts the authored silhouette on top.
/// Making this generous would hide a character whose art never resolved behind a
/// plausible-looking rectangle.
const SEAT_BODY_PX: Vec2 = Vec2::new(30.0, 48.0);

/// Seat one registered character as a body at `at`, facing `facing`.
///
/// Returns `None` when the character is not in the prepared registry — seating an
/// unregistered id would produce a body wearing a character nothing can describe,
/// and the load ledger already reports unknown tokens, so this stays quiet rather
/// than becoming a second reporter of the same fact.
///
/// Facing is not decoration: a move's authored offsets are mirrored through it, so
/// a fighter looking the wrong way swings into empty space. Seating faces each
/// participant toward the centre of the stage for that reason.
#[allow(clippy::too_many_arguments)]
pub fn seat_character(
    commands: &mut Commands,
    // The seated body's session owner. A fighter belongs to the MATCH's
    // session: without this it is spawned unscoped and survives leaving the
    // stage, so the next visit finds the previous match's knocked-out body
    // still lying at zero health and immediately awards round one to whoever
    // did not die (found 2026-07-27 while testing re-entry).
    session_scope: ambition_platformer2d_shared_tangle::lifecycle::SessionSpawnScope,
    registry: &PreparedCharacterRegistry,
    catalog: &ambition_characters::actor::character_catalog::CharacterCatalog,
    // Provider-authored sheets (U1 stage B): a seated fighter's collision box is
    // derived from its sheet, so a consumer's character must reach its own.
    authored_sheets: &ambition_sprite_sheet::character::sheets::AuthoredSheets,
    roster: &crate::features::CharacterRoster,
    character_id: &str,
    at: Vec2,
    facing: f32,
    faction: crate::combat::components::ActorFaction,
    brain: ambition_entity_catalog::placements::CharacterBrain,
    // S4: the death policy this body plays under. Applied to the SEED rather
    // than to the spawned component, because a second `&mut BodyHealth` query in
    // `seat_match_participants` conflicts with the primary-player one it already
    // holds — and construction is where a body's policy belongs anyway.
    death_policy: ambition_characters::actor::DeathPolicy,
) -> Option<Entity> {
    let prepared = registry.get(character_id)?;
    Some(seat_prepared_character(
        commands,
        session_scope,
        prepared,
        catalog,
        authored_sheets,
        roster,
        character_id,
        at,
        facing,
        faction,
        brain,
        death_policy,
    ))
}

/// [`seat_character`] with the registry lookup already done — **and therefore
/// infallible.**
///
/// ⚠ this exists so seating's COMMIT pass cannot fail. That pass says of itself:
/// *"Nothing below may return early: a `return` here would reintroduce exactly
/// the partial activation the resolve pass exists to prevent"* — and then called
/// `seat_character`, whose only failure mode is the registry lookup the RESOLVE
/// pass had already performed, and returned on `None` under a `debug_assert`. So
/// in a debug build it panicked and in a release build it did the one thing the
/// pass forbids, silently.
///
/// The fix is not a louder refusal: it is carrying the resolved value forward so
/// there is nothing left to refuse. The resolve pass looked the character up and
/// threw the answer away; now it keeps it.
#[allow(clippy::too_many_arguments)]
pub fn seat_prepared_character(
    commands: &mut Commands,
    session_scope: ambition_platformer2d_shared_tangle::lifecycle::SessionSpawnScope,
    prepared: &super::PreparedCharacterDefinition,
    catalog: &ambition_characters::actor::character_catalog::CharacterCatalog,
    authored_sheets: &ambition_sprite_sheet::character::sheets::AuthoredSheets,
    roster: &crate::features::CharacterRoster,
    character_id: &str,
    at: Vec2,
    facing: f32,
    faction: crate::combat::components::ActorFaction,
    brain: ambition_entity_catalog::placements::CharacterBrain,
    death_policy: ambition_characters::actor::DeathPolicy,
) -> Entity {
    // **THE AUTHORED PHYSICAL IDENTITY**, resolved once and read three times
    // below — the box here, the health pool on the seed, the mass on the bundle.
    //
    // Read through [`PhysicalBaseline`] rather than off `prepared.vitals` and
    // `prepared.body` directly, because the exploration player reads the same
    // value through the same accessors. Three paths each interpreting the fields
    // for themselves is precisely how the worn player ended up with the catalog's
    // health and none of the character's mass (GPT 5.6, 2026-07-29).
    let baseline = super::PhysicalBaseline::of(prepared);
    // `SEAT_BODY_PX` is a placeholder ON PURPOSE and stays the answer for a
    // character that authored nothing. But `BodySource::Explicit` had no consumer
    // anywhere in the repository, so a provider could author half-extents and
    // receive this constant instead (GPT 5.6, 2026-07-29).
    //
    // Spawn time is the right place for it: the box is a construction fact, and a
    // per-tick projection writing a live body's size would be a second geometry
    // authority beside the transit seam (ADR 0024).
    let body_px = baseline.explicit_size().unwrap_or(SEAT_BODY_PX);
    let aabb = ambition_platformer2d_core::Aabb::new(at, body_px / 2.0);
    // `new_in`, not the test-only `new`: production construction never has a
    // hidden catalog fallback, and a seated fighter resolves its sprite identity
    // from the SAME App-local catalog every other spawn path uses.
    let mut seed = crate::features::ecs::actor_clusters::ActorClusterSeed::new_in(
        authored_sheets,
        catalog,
        roster,
        character_id.to_string(),
        prepared.display_name.clone(),
        // ⭐ **the id, not the display name.** A seated fighter has always KNOWN
        // its catalog id — it is the first argument — and resolved its art by
        // round-tripping the display name back through the catalog anyway. Two
        // characters may legitimately share a display name; only the id is
        // unique, so this is the identity to hand the resolver.
        Some(character_id),
        aabb,
        brain,
        &[],
    );
    // The seed's own pool stands for a character that authored none — which used
    // to be impossible to express, because an unauthored `Vitals` defaulted to a
    // one-hit pool and every seated fighter silently took it.
    seed.health =
        ambition_characters::actor::BodyHealth::new(ambition_characters::actor::Health::new(
            baseline.max_health_over(seed.health.health.max.max(1)),
        ))
        .with_policy(death_policy);
    seed.kin.facing = facing;
    let centered = ambition_platformer2d_core::CenteredAabb::from_center_size(at, body_px);
    let motion_model = seed.config.tuning.motion_model();
    let (identity, _seed_disposition, combat, intent, cooldowns) =
        crate::features::ecs::enemy_component_snapshot(&seed);
    // A match participant is a COMBATANT, whatever brain drives it.
    //
    // The disposition the seed derives follows the authored brain, and a human
    // seat authors `Passive` (its real driver is the player). `apply_actor_hit`
    // reads the disposition first: a peaceful body takes NO health damage — it
    // barks and turns hostile instead. So a seated fighter was unkillable, and
    // the symptom was a swing that connected, played its sound, and did nothing.
    let disposition = crate::combat::components::ActorDisposition::Hostile;
    // A default action set, matching what an enemy spawn does before its
    // archetype fills one in. The character's real attacks arrive from
    // `apply_worn_character_gameplay`, which derives the persona from
    // `WornCharacter` — that is the ONE writer for a worn body's moves, and
    // seating must not author a second opinion about them.
    let action_set = ambition_characters::brain::ActionSet::default();
    // The BRAIN, derived from the seed's archetype exactly as the enemy spawner
    // derives it. `into_components()` does not carry one — the enemy spawn path
    // inserts it beside the cluster, and seating did not, so a CPU fighter had a
    // body, a target, an empty `ActorControl` and no brain to write it. It stood
    // perfectly still while every component that would explain why was present
    // and correct. (Same shape as the missing `ActorTarget`: a construction path
    // that copies most of another one is a claim, not a guarantee.)
    let derived_brain = crate::features::ecs::enemy_default_brain(&seed.config);
    let combat_kit = crate::combat::components::CombatKit::from_action_set(&action_set);
    let cluster = seed.into_components();
    use ambition_platformer2d_shared_tangle::lifecycle::SpawnSessionScopedExt;
    let body = commands
        .spawn_session_scoped(
            session_scope,
            (
                // The SAME bundle every other actor spawn builds. Seating used to
                // hand-pick a subset of these components, and the subset was
                // missing `ActorTarget` — which `tick_actor_brains` requires
                // non-optionally, so a seated fighter silently dropped out of the
                // brain tick entirely and stood still. It looked like a body in
                // every way a test that queries components can see.
                crate::features::EnemyActorBundle::new(
                    crate::features::FeatureBaseBundle::new(
                        character_id,
                        prepared.display_name.clone(),
                        centered,
                    ),
                    identity,
                    disposition,
                    faction,
                    crate::features::ActorPose::from_parts(at, body_px / 2.0, facing),
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
                // **Required columns of `apply_worn_character_gameplay`**, the
                // ONE writer that turns `WornCharacter` into a persona — name,
                // action set, moveset, identity baseline. A body missing any of
                // them does not match the derive at all: it wears a character
                // and derives nothing from it, and the fighter walks and cannot
                // swing. Placeholders; the derive replaces them on the tick the
                // worn character lands.
                //
                // ⚠ **a correction (2026-07-29).** The comment here used to say
                // the derive's query "requires `IdentityKit` and
                // `BodyAbilities` — and `EnemyActorBundle` carries neither, so
                // a seated body never matched it", and that reasoning was
                // carried into the queue, into a campaign doc, and into the
                // design of the projection that grew up to serve seated bodies.
                // It is FALSE and was checked rather than inherited:
                // `WornCharacter` is `#[require(IdentityKit)]`, so every worn
                // body has one, and `BodyAbilities` arrives with
                // `AncillaryMovementBundle` inside the cluster below — adding
                // it here is a duplicate component and Bevy panics on the
                // bundle, which is how this got caught.
                //
                // Seated bodies therefore ALWAYS matched the derive. The second
                // kit writer was never necessary; it was built on a diagnosis
                // nobody re-read. That is the more useful lesson than the one
                // the old comment taught.
                Name::new(prepared.display_name.clone()),
                crate::combat::moveset::ActorMoveset(Default::default()),
                // The body WEARS the character. Everything that makes it that
                // fighter rather than a generic actor follows from this one
                // component: the moveset and silhouette arrive via
                // `project_prepared_character_definitions`, and the presentation
                // source via `publish_body_presentation_sources`. Seating does not
                // insert any of them by hand — that hand projection is exactly
                // what made the old fixture prove less than it looked like.
                ambition_characters::actor::WornCharacter::new(character_id),
                // The MATCH owns this fighter's death, not the world. Without
                // this a KO runs the exploration economy — a bounty coin, a
                // heart, a death explosion, an in-place respawn timer — none of
                // which an arena has any use for.
                crate::combat::components::RulesetOwnsDeath,
                // WITHOUT THIS THE FIGHTER IS INVISIBLE. The marker's own
                // documentation says so: "the authored render pass only spawns
                // visuals for `spec.enemy_spawns`, and the dynamic pass only for
                // EncounterMob / reward chests, so a directly-staged actor would
                // render invisibly."
                //
                // A seated fighter is a directly-staged actor, and seating did
                // not apply it — so the versus arena had a body with a published
                // view, a hurtbox, a moveset and no picture. Jon found it by
                // picking Versus and seeing one fighter (2026-07-27); the seat-0
                // fighter looked fine only because it is the adopted PRIMARY
                // PLAYER, which renders through the player path entirely.
                crate::combat::components::RuntimeStagedActor,
                ambition_characters::brain::ActorControl::default(),
                ambition_characters::actor::attack_gesture::AttackGestureState::default(),
                ambition_characters::actor::attack_gesture::AttackGestureTuning::default(),
                ambition_characters::actor::attack_gesture::ResolvedAttackGesture::default(),
            ),
        )
        .id();
    // **THE AUTHORED MASS**, which reached nothing until 2026-07-29.
    //
    // `Mass` is real and has one consumer: the mount pair's mass-weighted centre
    // of gravity (ADR 0020) — a heavy mount keeps the COG near itself so the
    // lighter rider orbits it on a gravity flip. It was populated from the ROSTER
    // archetype and never from the character definition, so `Vitals.mass` was a
    // second declaration of a fact only the roster could state (GPT 5.6 flagged
    // it as dead; it was not dead, it was disconnected).
    //
    // ⚠ CONDITIONAL now, and that is the fix to the fix: writing it into the
    // bundle unconditionally meant a character that authored no mass overwrote
    // its own archetype's with the ambient 1.0. Health and geometry are already
    // applied by the seed above, so this is all the boundary has left to do.
    baseline.apply_to_body(
        super::BaselineBoundary::Construction,
        &mut commands.entity(body),
        None,
        None,
        // No outgoing persona: this body is being built. Silence keeps the
        // archetype's own mass rather than retracting to anything.
        super::PhysicalRetraction::NONE,
    );
    body
}

/// Where a participant sits and which way it looks.
///
/// Symmetric about `centre`, alternating sides, facing inward. Two participants
/// is the case that matters now and the general rule degrades sensibly: four
/// fighters get two per side, and everyone still looks at the middle.
fn seat_for(index: usize, centre: Vec2) -> (Vec2, f32) {
    let side = if index % 2 == 0 { -1.0 } else { 1.0 };
    let rank = (index / 2) as f32;
    let x = centre.x + side * (SEAT_SPREAD_PX + rank * SEAT_SPREAD_PX * 0.5);
    // Facing points back toward the centre: a left-hand seat looks right.
    (Vec2::new(x, centre.y), -side)
}

/// Alternating sides, so the two can actually hit each other:
/// `effective_faction` refuses a strike between same-faction bodies, and a
/// roster seated all one way would stand and stare.
fn faction_for(index: usize) -> crate::combat::components::ActorFaction {
    if index % 2 == 0 {
        crate::combat::components::ActorFaction::Player
    } else {
        crate::combat::components::ActorFaction::Enemy
    }
}

/// Which seat of the match this body is.
///
/// The roster's index, on the body. Match RULES need to name a fighter — whose
/// health bar is on the left, who won the round, where to put them back — and
/// every other way to identify one is a guess: `Brain::Player(slot)` misses the
/// CPU seat, the worn character id collides in a mirror match, and entity order
/// is not an order. Seating is the only place that knows, so seating says so.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct MatchSeat(pub usize);

/// Where seat `index` stands, given the stage centre. Public so a rules layer
/// can put a fighter BACK between rounds without re-deriving the geometry and
/// drifting from it.
pub fn seat_placement(index: usize, centre: Vec2) -> (Vec2, f32) {
    seat_for(index, centre)
}

/// **The bodies in a live match, in seat order, DERIVED from the world.**
///
/// The seat binding lives on the fighters as [`MatchSeat`], which is where it
/// belongs: a body knows which seat it is, and a body that no longer exists
/// cannot claim one. Anything that needs the cast asks the world through this,
/// rather than reading a list somebody remembered.
///
/// ⚠ **this used to be a `Vec<Entity>` on [`ActiveMatch`], and that was the bug**
/// (GPT 5.6, 2026-07-29). A resource holding live `Entity` values, mutated from
/// inside the rollback schedule and not registered as rollback state, keeps its
/// future contents across a rewind: the bodies are restored to an earlier state —
/// or to not existing — while the list still names them. Deriving costs one
/// query and cannot go stale, because there is nothing to keep in step.
///
/// Sorted by seat, so `participants[i]` is seat `i` however the entities were
/// spawned. Entity order is not an order; a set that arrives in spawn order makes
/// indexing it mean nothing.
pub fn match_participants(seated: &Query<(Entity, &MatchSeat)>) -> Vec<Entity> {
    let mut by_seat: Vec<(usize, Entity)> = seated
        .iter()
        .map(|(entity, seat)| (seat.0, entity))
        .collect();
    by_seat.sort_by_key(|(seat, _)| *seat);
    by_seat.into_iter().map(|(_, entity)| entity).collect()
}

/// **The match that is LIVE.**
///
/// Present means every participant in the roster has a body. Absent means no
/// match is running — either none was requested, or seating is still retrying.
/// Seating is a one-shot per match: without this the system would re-seat every
/// tick the roster exists, which is a fresh pair of fighters per frame, the kind
/// of runaway that looks like a spawn bug three systems away.
///
/// ⚠ this replaced a `MatchSeated(bool)`, and the difference is the whole point.
/// A bool said seating had FINISHED and never said WHO, so nothing could ask
/// whether the live fighters are still the set the match was built from — which
/// is exactly why a roster that disagrees with its session after seating could
/// only be REPORTED and not repaired (queue Y′9).
///
/// ⚠ **and it says how MANY, not WHICH** (GPT 5.6, 2026-07-29). Naming the
/// bodies meant holding `Vec<Entity>` in a resource that is written from inside
/// the rollback schedule and is not rollback state, so a rewind across activation
/// would restore the fighters and leave the list pointing at the future. The
/// review's rule is the right one — *"do not snapshot raw entity references
/// without a complete remapping and reconstruction contract"* — and the cheapest
/// way to obey it was to stop snapshotting: [`match_participants`] derives the
/// cast from [`MatchSeat`] on the bodies themselves, which rewinds because the
/// bodies do.
///
/// What is left here is plain data with no identity in it: a count and a
/// generation number. Both are facts about the DECISION to activate, and both
/// only ever move forward within a match.
///
/// Published in ONE insert, on the tick the last seat is filled. Never partially:
/// a roster whose seat 0 cannot adopt yet while seat 1 spawned fine must not
/// activate on the strength of seat 1.
#[derive(Resource, Debug, Clone, PartialEq)]
pub struct ActiveMatch {
    /// How many seats this match activated with. Compare it against
    /// [`match_participants`] to ask whether the cast is still whole.
    seats: usize,
    /// The frozen seat topology this match was activated against, copied from
    /// the roster so the two can be COMPARED rather than assumed equal.
    seat_topology: Option<u64>,
}

impl ActiveMatch {
    /// How many fighters this match activated with.
    ///
    /// Deliberately not "how many are alive now" — that is a question for the
    /// world, and [`match_participants`] answers it. The difference between the
    /// two is exactly the signal a rules layer wants.
    pub fn seats(&self) -> usize {
        self.seats
    }

    /// Which frozen topology decided this match's seating, if a session had
    /// frozen one when the roster was built.
    pub fn seat_topology(&self) -> Option<u64> {
        self.seat_topology
    }

    /// **Adopt a frozen topology this match already agrees with.**
    ///
    /// The ONLY legitimate mutation of a live activation, and the narrowness is
    /// the point. It records which topology decided a seating that has not
    /// changed — it cannot move a body, add a seat or drop one.
    ///
    /// The case it exists for is ordinary: a route's roster is built from live
    /// device discovery when the route is entered, and the rollback session
    /// freezes its topology afterwards. If the fighters the frozen topology
    /// would produce are the fighters already on the stage, the two agree about
    /// WHO and disagree only about the paperwork, and correcting paperwork is a
    /// repair. A real disagreement — different fighters — is still reported and
    /// still not reseated, because reseating mid-match is the worse bug
    /// (queue Y′9, 2026-07-29).
    pub fn adopt_seat_topology(&mut self, generation: u64) {
        self.seat_topology = Some(generation);
    }

    /// Build an activation directly, for a test that needs a LIVE match without
    /// standing up seating to produce one.
    ///
    /// The fields stay private so production has exactly one publisher; this is
    /// the hatch, and it is named for what it is.
    #[doc(hidden)]
    pub fn for_test(seats: usize, seat_topology: Option<u64>) -> Self {
        Self {
            seats,
            seat_topology,
        }
    }

    /// Rebuild an activation from a rollback snapshot.
    ///
    /// A second constructor rather than reusing `for_test`, because the two say
    /// different things to a reader and one of them is production. This is the
    /// ONLY non-seating production path that may produce an `ActiveMatch`, and
    /// it reproduces one that seating already published.
    ///
    /// ⚠ **what makes registering this correct is that `bevy_ggrs` restores
    /// ABSENCE**: `ResourceSnapshotPlugin::load` maps `(Some(_), None)` to
    /// `remove_resource`. So a rewind to a frame before activation does not
    /// merely stale the latch, it deletes it — seating sees no active match,
    /// re-runs, and rebuilds the roster. Registration would have been decorative
    /// if the plugin only overwrote a present value, which is worth stating
    /// because that is the assumption the fix rests on (AA2 / AC2).
    #[doc(hidden)]
    pub fn from_snapshot(seats: usize, seat_topology: Option<u64>) -> Self {
        Self {
            seats,
            seat_topology,
        }
    }
}

/// **What one seat needs, decided before anything is built.**
///
/// A plan value, not a body: it names the seat, where it sits and what will fill
/// it, and producing one is guaranteed not to touch the world. That guarantee is
/// the whole point — see the RESOLVE block in [`seat_match_participants`] for the
/// half-applied adoption it exists to prevent.
///
/// It borrows the character id from the roster rather than cloning it: the plan
/// never outlives the pass that built it, and a `String` per seat per retried
/// tick is allocation for nothing.
enum SeatPlan<'roster> {
    /// Construct a new body for this seat.
    Spawn {
        index: usize,
        character: &'roster str,
        /// **The resolved definition, carried rather than re-looked-up.**
        ///
        /// The resolve pass asked the registry whether this seat was
        /// satisfiable and threw the answer away; the commit pass then asked
        /// again and had to handle a `None` it had already ruled out. Keeping it
        /// is what makes the commit pass infallible instead of merely unlikely
        /// to fail.
        prepared: &'roster super::PreparedCharacterDefinition,
        at: Vec2,
        facing: f32,
        faction: crate::combat::components::ActorFaction,
        brain: ambition_entity_catalog::placements::CharacterBrain,
        /// A human seat past `PRIMARY`: the spawned body is handed this slot, so
        /// the couch's second writer drives it.
        player_slot: Option<ambition_characters::brain::PlayerSlot>,
        team: Option<crate::combat::targeting::MatchTeam>,
    },
    /// Take the body the primary player already has.
    Adopt {
        index: usize,
        body: Entity,
        character: &'roster str,
        at: Vec2,
        facing: f32,
        team: Option<crate::combat::targeting::MatchTeam>,
    },
}

/// Seat every participant in [`MatchParticipantRoster`] in ONE transaction.
///
/// Runs on the sim schedule so a seated body exists on a tick boundary like every
/// other constructed entity, rather than mid-frame where half the pipeline has
/// already run.
///
/// **Resolve, validate, commit.** Every seat is proven satisfiable before any
/// seat is built, so a roster that cannot yet be filled leaves the world byte-for-
/// byte as it found it and retries next tick. A roster that can be filled is
/// filled completely, in one command flush, together with the [`ActiveMatch`]
/// latch that says so.
/// **Why the last seating attempt refused**, when it did.
///
/// ⚠ the refusal used to be a `debug_assert!` and nothing else, so a release
/// build silently declined to seat and the match activated around the hole. That
/// is the shape of the very bug it was added for — a fighter that is not the one
/// the roster asked for — reintroduced by the guard's own build configuration.
///
/// Present only while a roster the composition cannot seat is published; removed
/// as soon as one it can is. A consumer reads it to tell the player something
/// true, and a TEST reads it instead of relying on a panic that only fires in
/// debug.
#[derive(bevy::prelude::Resource, Debug, Clone, PartialEq, Eq)]
pub struct MatchSeatingRefused {
    pub problems: Vec<crate::character_runtime::RosterProblem>,
}

pub fn seat_match_participants(
    mut commands: Commands,
    roster: Option<Res<MatchParticipantRoster>>,
    registry: Option<Res<PreparedCharacterRegistry>>,
    // REQUIRED, not optional: `engine.character-authority-is-app-local` forbids
    // making the character authority optional, and it is right to. A composition
    // with no catalog must be NAMED by the capability audit, not silently seat
    // fighters that resolve their sprite identity against nothing.
    catalog: Res<ambition_characters::actor::character_catalog::CharacterCatalog>,
    // Same authority class as the catalog, and required for the same reason
    // (U1 stage B): a seated fighter sizes its body from its sheet.
    authored_sheets: Res<ambition_sprite_sheet::character::sheets::AuthoredSheets>,
    archetypes: Res<crate::features::CharacterRoster>,
    geometry: Option<
        ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<
            ambition_platformer2d_core::RoomGeometry,
        >,
    >,
    active_session: Option<Res<ambition_platformer2d_shared_tangle::lifecycle::ActiveSessionScope>>,
    active: Option<Res<ActiveMatch>>,
    // Seats that already have a body. Derived from the world rather than
    // remembered, so a seat can never be counted twice and the retry below
    // cannot spawn a second copy of a fighter that seated fine last tick.
    already_seated: Query<(Entity, &MatchSeat)>,
    mut player: Query<
        (
            Entity,
            &mut ambition_characters::actor::BodyHealth,
            ambition_platformer2d_core::BodyClusterQueryData,
            &mut crate::features::MotionModel,
            &ambition_characters::actor::WornCharacter,
        ),
        crate::actor::PrimaryPlayerOnly,
    >,
    // The last (worn, wanted) pair this system complained about, so a retry that
    // runs every tick says it once rather than every frame.
    mut reported: Local<Option<(String, String)>>,
) {
    if active.is_some() {
        return;
    }
    let (Some(roster), Some(registry), Some(geometry)) = (roster, registry, geometry) else {
        return;
    };
    if roster.participants.is_empty() {
        return;
    }
    // No active session means no owner for the bodies. Seating waits rather
    // than spawning orphans; the roster is still there next tick.
    let Some(session_scope) =
        ambition_platformer2d_shared_tangle::lifecycle::SessionSpawnScope::for_optional_active_session(
            active_session.as_deref(),
        )
    else {
        return;
    };
    // The stage centre is the room's authored spawn: the one point a room
    // guarantees is standable, which is the only guarantee seating needs.
    let centre = geometry.0.spawn;
    // Seat index -> body, for the seats that were filled on an EARLIER tick.
    // Seating retries, so the closing tick may add only the last one.
    let mut by_seat: std::collections::BTreeMap<usize, Entity> = already_seated
        .iter()
        .map(|(entity, seat)| (seat.0, entity))
        .collect();
    let occupied: std::collections::BTreeSet<usize> = by_seat.keys().copied().collect();
    // ── RESOLVE ─────────────────────────────────────────────────────────────
    //
    // **Every seat is decided before any seat is built.** Nothing in this loop
    // touches the world: it reads the roster, the registry and the player's own
    // body through a READ-ONLY view, and produces one plan value per unfilled
    // seat. A seat that cannot be satisfied yet returns from the system having
    // mutated NOTHING, and seating retries next tick exactly as it always has.
    //
    // ⚠ **this replaced a loop that resolved and constructed one seat at a
    // time, and the defect it closes is sharper than "only the latch was
    // atomic".** The ADOPTION path below writes the primary player's health,
    // body size and pose THROUGH THE QUERY — immediately, not through deferred
    // `Commands`. So seat 0 could be re-pooled, resized and teleported to its
    // mark on a tick where seat 1 then failed to resolve, and the player wore
    // that half-applied match state for as many ticks as the roster took to
    // complete. Validation completing before ANY mutation is the fix, and it is
    // why this is a resolve/commit pair rather than a tidier single loop.
    //
    // What the one-flush commit buys beyond that: activation happens entirely
    // within one tick, so a rewind lands either BEFORE it (no bodies, no latch —
    // `bevy_ggrs` restores ABSENCE, so seating re-runs and rebuilds) or AFTER it
    // (both restored). There is no "between two seats" state left to land in,
    // which is AA2's lifecycle half — and it is reached without the route
    // reordering the reviews assumed was necessary.
    let mut plans: Vec<SeatPlan<'_>> = Vec::new();
    for (index, participant) in roster.participants.iter().enumerate() {
        let MatchParticipant {
            character,
            controller,
            team,
            ..
        } = participant;
        // Already has a body from an earlier tick, or from a rewind that
        // restored the bodies without the latch. Derived from the world, so a
        // seat can never be counted twice.
        if occupied.contains(&index) {
            continue;
        }
        // The roster's declared TEAM, on the body. It had been declared and read
        // by nothing since §7.8; `MatchTeam` is what the damage relation
        // consults, and it is what lets two human seats hit each other without
        // the stage switching on GLOBAL friendly fire (which also makes
        // teammates hittable, and is therefore wrong the moment a 2v2 exists).
        let team_tag = team
            .as_ref()
            .map(|team| crate::combat::targeting::MatchTeam::new(team.clone()));
        let (at, facing) = seat_for(index, centre);
        // **A seat is satisfiable when its body can be produced from what is
        // already here.** `seat_character` has exactly one failure mode —
        // `registry.get(character_id)?` — so asking the registry the same
        // question is a COMPLETE precondition rather than an optimistic one.
        // That completeness is what makes the commit below infallible, and it is
        // the property to preserve if `seat_character` ever grows a second way
        // to fail.
        let plan = match controller {
            // A HUMAN seat is the body the player already has. Adopt it — move
            // it to its seat and face it inward — rather than spawning a second
            // body wearing the same character, which is what produced two
            // Mary-Os in the arena the first time this stage shipped.
            super::ControllerBinding::Human { device_slot } => {
                let slot = ambition_characters::brain::PlayerSlot(*device_slot);
                if slot == ambition_characters::brain::PlayerSlot::PRIMARY {
                    // Read-only: `single()` on a mutable query yields the
                    // read-only item, which is what keeps RESOLVE pure. The
                    // `single_mut()` that actually writes lives in COMMIT.
                    let Ok((body, .., worn)) = player.single() else {
                        // No primary body yet. Seating retries next tick, and
                        // this is the ordinary case on the frame a stage opens
                        // — quiet on purpose.
                        return;
                    };
                    if worn.id() != character {
                        // The stage's starting character and this seat disagree.
                        // Seating does not re-dress the player body — that is
                        // `WornCharacter`'s job, and a stage that wants a
                        // different fighter should say so in its
                        // `StartingCharacter`.
                        //
                        // ⛔ **BUT IT SAYS SO NOW, and it cost hours not to.**
                        // This `return` leaves the whole system, so ONE seat
                        // disagreeing seats NOBODY — including every other
                        // participant — silently, every tick, forever. A
                        // character-select screen decides seat 0's fighter at
                        // runtime, which is exactly the case this predates: the
                        // symptom was a match that opened onto an empty stage
                        // with a published roster, a correct route, and no
                        // `MatchSeatingRefused` to read.
                        //
                        // Three lines below, an unknown brain profile refuses
                        // the roster OUT LOUD with a recorded reason, and its
                        // comment explains why: *"the per-seat `debug_assert!`
                        // this replaced was invisible in release."* Same class,
                        // same fix — throttled to once per (worn, wanted) pair
                        // so a retry loop does not become a log flood.
                        if reported.replace((worn.id().to_string(), character.to_string()))
                            != Some((worn.id().to_string(), character.to_string()))
                        {
                            bevy::log::warn!(
                                target: "ambition_platformer2d::seating",
                                "match seating is waiting: seat {index} asks for \
                                 `{character}` and the primary player's body wears \
                                 `{}`. NOBODY seats until they agree — re-dress the \
                                 body (`WornCharacter`) or name that fighter in the \
                                 stage's `StartingCharacter`.",
                                worn.id(),
                            );
                        }
                        return;
                    }
                    SeatPlan::Adopt {
                        index,
                        body,
                        character,
                        at,
                        facing,
                        team: team_tag,
                    }
                } else {
                    // A SECOND human is a second body. Only slot 0 has a body
                    // already — the one the session spawned as the primary
                    // player — so every other seat is spawned and handed its own
                    // `Brain::Player(slot)`.
                    //
                    // `tick_player_brains` already drives any body whose brain
                    // names a slot, and `SlotControls` already holds four. What
                    // couch versus was missing is not the engine: it is a writer
                    // for the second slot. This seats the body that writer will
                    // drive.
                    let Some(prepared) = registry.get(character) else {
                        return;
                    };
                    SeatPlan::Spawn {
                        index,
                        character,
                        prepared,
                        at,
                        facing,
                        faction: faction_for(index),
                        // `Passive` is the authored brain the seed needs; the
                        // player slot below replaces the runtime `Brain`. A
                        // passive placeholder rather than a wandering one so a
                        // body whose player writer never arrives stands still
                        // instead of strolling off looking possessed.
                        brain: ambition_entity_catalog::placements::CharacterBrain::Passive,
                        player_slot: Some(slot),
                        team: team_tag,
                    }
                }
            }
            _ => {
                let Some(profile) = controller.brain_profile() else {
                    return;
                };
                let Some(prepared) = registry.get(character) else {
                    return;
                };
                // **A CPU SEAT NAMING AN UNKNOWN BRAIN IS UNSATISFIABLE, not a
                // generic enemy.** `spec_for_brain` falls back to the roster's
                // `combatant` row for an unknown key — its own doc says a
                // provider that misspells an archetype "gets a generic enemy
                // instead of an error" — and for a PLACEMENT that is a defensible
                // default. For a match seat it is not: the fighter the roster
                // asked for is not the fighter that arrives, the match activates
                // anyway, and the symptom is a duelist standing still, which is
                // indistinguishable from a brain that was never installed. That
                // cost an hour to find in the smash demo on 2026-07-31, with a
                // diagram, because nothing anywhere said no.
                //
                // `resolve_initial_brain` already holds this line for placement
                // overrides — an override that does not resolve is a loud
                // `UnknownPreset`, "never a silent fall back to the default" —
                // and this is the same class of mistake on the other path.
                if !archetypes.has_brain_key(profile) {
                    // Refuse the WHOLE roster, out loud, in every build. The
                    // per-seat `debug_assert!` this replaced was invisible in
                    // release — which reintroduced the exact bug it guards.
                    let problems = roster.unsatisfiable_seats(&archetypes);
                    bevy::log::error!(
                        target: "ambition_platformer2d::seating",
                        "match seating refused: {}",
                        problems
                            .iter()
                            .map(|problem| problem.to_string())
                            .collect::<Vec<_>>()
                            .join("; ")
                    );
                    commands.insert_resource(MatchSeatingRefused { problems });
                    return;
                }
                SeatPlan::Spawn {
                    index,
                    character,
                    prepared,
                    at,
                    facing,
                    faction: faction_for(index),
                    brain: ambition_entity_catalog::placements::CharacterBrain::Custom(
                        profile.to_string(),
                    ),
                    player_slot: None,
                    team: team_tag,
                }
            }
        };
        plans.push(plan);
    }

    // ── COMMIT ──────────────────────────────────────────────────────────────
    //
    // Past this point every seat is known to be satisfiable, so the match is
    // built to completion in ONE command flush. Nothing below may return early:
    // a `return` here would reintroduce exactly the partial activation the
    // resolve pass exists to prevent.
    //
    // Every body this pass produced, so the roster's abilities and suspension
    // land on all of them in the SAME flush that creates them. A body is
    // therefore never observable in a state the ruleset did not ask for — no
    // window to narrow.
    // The refusal, if one is standing, is over: this roster resolved. Removed
    // here rather than on roster CHANGE so it cannot go stale — a refusal that
    // outlives the roster it was about is a worse lie than no refusal.
    commands.remove_resource::<MatchSeatingRefused>();
    let mut seated_bodies: Vec<Entity> = Vec::new();
    let mut seated_this_pass: Vec<(usize, Entity)> = Vec::new();
    // S4: a stocks match's fighters die to the WORLD, not to the meter. Declared
    // once here so a spawned seat and the adopted player cannot disagree about
    // it — that divergence is the one this file has now had three times.
    let stocks_policy = if roster.fighter_stocks.is_some() {
        ambition_characters::actor::DeathPolicy::Unbounded
    } else {
        ambition_characters::actor::DeathPolicy::default()
    };
    for plan in plans {
        match plan {
            SeatPlan::Spawn {
                index,
                character,
                prepared,
                at,
                facing,
                faction,
                brain,
                player_slot,
                team,
            } => {
                // **INFALLIBLE.** The resolve pass looked this character up and
                // KEPT the answer, so there is nothing here to refuse — which is
                // what the commit pass's own rule requires: *"Nothing below may
                // return early: a `return` here would reintroduce exactly the
                // partial activation the resolve pass exists to prevent."*
                //
                // ⚠ this used to call `seat_character`, whose only failure is the
                // registry lookup already performed, and returned on `None` under
                // a `debug_assert!`. Debug panicked; release did the one thing the
                // pass forbids, silently. A louder refusal was the wrong fix: the
                // right one is having nothing left to refuse.
                let body = seat_prepared_character(
                    &mut commands,
                    session_scope,
                    prepared,
                    &catalog,
                    &authored_sheets,
                    &archetypes,
                    character,
                    at,
                    facing,
                    faction,
                    brain,
                    stocks_policy,
                );
                let mut seated = commands.entity(body);
                seated.insert(MatchSeat(index));
                if let Some(team) = team {
                    seated.insert(team);
                }
                if let Some(slot) = player_slot {
                    seated.insert((
                        ambition_characters::brain::Brain::Player(slot),
                        crate::control::components::LocalPlayer,
                        crate::control::components::PlayerInputFrame::default(),
                    ));
                }
                seated_bodies.push(body);
                seated_this_pass.push((index, body));
            }
            SeatPlan::Adopt {
                index,
                body,
                character,
                at,
                facing,
                team,
            } => {
                // Resolved above through the read-only view; the mutable single
                // cannot disagree with it within one system run.
                //
                // ⚠ **`expect`, not `debug_assert!` + `return`.** The two builds
                // used to disagree: debug panicked, release skipped the seat —
                // and skipping is the partial activation this pass exists to
                // prevent, so the release behaviour was the worse one. Between a
                // loud stop and a half-built match the repo has already made this
                // call once, at initial session setup: "a silent partial start
                // would be worse than a loud stop".
                //
                // Unlike the spawn arm above, there is nothing to carry forward —
                // the commit needs a MUTABLE borrow the resolve pass cannot hold.
                let (_, mut health, clusters, mut model, _) = player
                    .single_mut()
                    .expect("seat adopted a player body that vanished mid-system");
                // The adopted PRIMARY PLAYER needs `RulesetOwnsDeath` most. Its
                // death runs `death_respawn_player`, which teleports it to the
                // room spawn and restores full health BEFORE any rules layer can
                // look — so seat 0 could never be seen at zero health, and the
                // match was rigged in its favour (GPT 5.6, 2026-07-27).
                let mut adopted = commands.entity(body);
                adopted.insert((
                    MatchSeat(index),
                    crate::combat::components::RulesetOwnsDeath,
                ));
                // The TEAM, which this branch dropped when the death-ownership
                // insert was added over it. A seat with no team is judged by
                // FACTION alone, and `effective_faction` maps every
                // player-brained body to `Player` — so in an all-human match the
                // adopted seat 0 could not be hit by anybody, which is the 1v1
                // rigging bug again wearing a different hat. Found by the 2v2
                // test (2026-07-27).
                if let Some(team) = team {
                    adopted.insert(team);
                }
                let mut item = clusters;
                let mut clusters = item.as_clusters_mut();
                // **MATCH ACTIVATION IS A CONSTRUCTION BOUNDARY**, so the adopted
                // body gets the same physical identity a spawned seat gets — from
                // the same resolver, through the same call.
                //
                // Each of these three was a separate divergence, found one at a
                // time, and the pattern is what made them worth unifying (GPT
                // 5.6, 2026-07-29):
                //
                // * **health** — a spawned seat took the authored maximum; the
                //   adopted player kept whatever its session established. The
                //   versus duelists author 60 and 52, a deliberate trade with one
                //   fighter paying for a faster smash, and that trade did not
                //   apply to seat 0.
                // * **box** — a mirror match could put two different body shapes
                //   on the stage, and the wrong one was always player one.
                // * **mass** — the same character weighed different amounts
                //   depending on which seat it took.
                //
                // Written BEFORE `transit_body`, deliberately: the transit seam is
                // the one authority that re-resolves a body's pose against the
                // world (ADR 0024), so a size change followed by a transit is a
                // body arriving at its seat correctly sized, not a live resize
                // that could leave it intersecting the floor it was standing on.
                // That ordering is exactly why this path may pass a `size` and the
                // re-wear path may not.
                if let Some(prepared) = registry.get(character) {
                    super::PhysicalBaseline::of(prepared).apply_to_body(
                        super::BaselineBoundary::Construction,
                        &mut adopted,
                        Some(&mut health),
                        Some(super::BodyGeometry {
                            live: &mut clusters.kinematics.size,
                            base: &mut clusters.base_size.base_size,
                        }),
                        // Adoption into a match is CONSTRUCTION, so there is
                        // nothing to retract: what the body brought is what a
                        // silent character keeps.
                        super::PhysicalRetraction::NONE,
                    );
                } else {
                    // No prepared character to speak for it; a seat still opens
                    // full.
                    health.health.current = health.health.max;
                }
                ambition_platformer2d_core::movement::transit_body(
                    &mut model,
                    &mut clusters,
                    at,
                    ambition_platformer2d_core::movement::TransitVelocity::Zero,
                );
                clusters.kinematics.facing = facing;
                // The adopted player plays under the same death policy as every
                // spawned seat. `set_policy` rather than a fresh `BodyHealth`:
                // the pool was resolved from the authored baseline three lines
                // ago and rebuilding the component would throw that away.
                health.set_policy(stocks_policy);
                seated_bodies.push(body);
                seated_this_pass.push((index, body));
            }
        }
    }
    let seated_count = occupied.len() + seated_this_pass.len();
    // **A fighter that opens suspended is suspended BEFORE it exists.**
    //
    // Not narrowed — closed. These inserts flush with the spawns above, so no
    // schedule ordering, no first-tick exemption, and nothing for a future system
    // to run in between.
    // **EVERY FIGHTER IN A MATCH HAS THE SAME MOVEMENT CAPABILITIES.**
    //
    // A SPAWNED seat's abilities come from `AncillaryMovementBundle` — the basic
    // run-and-jump floor. The ADOPTED primary player brought whatever the session
    // granted it, which in the shipped host is the sandbox dev kit: blink, fly,
    // shield. So player one could teleport and FLY in a versus match while the
    // opponent could not, and the on-screen control legend advertised it. Found
    // by capturing the stage and looking at it (2026-07-29); no test had thought
    // to ask.
    //
    // ⚠ **`AbilityBase` too, not only the effective set.** The effective set is
    // `base ∩ editable_mask`, recomputed every frame for the primary player by the
    // dev-tools sync — so writing only `BodyAbilities` would be undone on the next
    // tick by a system that is behaving correctly. Read before it was tested.
    //
    // Declared by the ROSTER, like `opens_suspended`: what a fighter may do is a
    // rule of the match, not something seating decides on its own. A match that
    // says nothing leaves every body exactly as it found it.
    if let Some(abilities) = roster.fighter_abilities {
        for body in &seated_bodies {
            commands.entity(*body).try_insert((
                ambition_platformer2d_core::BodyAbilities::new(abilities),
                ambition_platformer2d_core::AbilityBase::new(abilities),
            ));
        }
    }
    // **THE STOCK ECONOMY, handed out in the same flush that builds the bodies.**
    // (S4)
    //
    // Both halves together, because neither is meaningful alone: stocks over a
    // meter that kills at max are never consulted (the body dies of damage
    // before the world can throw it out), and an unbounded meter with no stocks
    // is a fighter that cannot lose. The roster declares one number and gets
    // both, so a match cannot express the broken half.
    //
    // `set_policy` rather than a fresh `BodyHealth`: the adopted primary player's
    // pool was already resolved from its authored baseline earlier in this same
    // transaction, and rebuilding the component here would throw that away.
    if let Some(stocks) = roster.fighter_stocks {
        for body in &seated_bodies {
            commands
                .entity(*body)
                .try_insert(crate::combat::components::FighterStocks::new(stocks));
        }
    }
    if roster.opens_suspended {
        for body in seated_bodies {
            commands
                .entity(body)
                .try_insert(ambition_characters::brain::ScriptedControl);
        }
    }
    // ATOMIC: the latch closes only when EVERY participant got a body.
    //
    // `any` was the wrong question. A roster whose seat 0 could not be adopted
    // yet (the primary player has not spawned, or wears a different character)
    // while seat 1 spawned fine would set the latch on the strength of seat 1,
    // and seat 0 would never be retried — a one-fighter match, permanently
    // (GPT 5.6, 2026-07-27). Seating runs every tick until the roster is
    // complete, and `seat_character`/adoption are the only things that decide
    // whether a seat is possible yet.
    if seated_count == roster.participants.len() {
        // ACTIVATION, in one insert. `by_seat` already holds the seats filled on
        // earlier ticks; this pass's go in on top, so the count is the whole cast
        // however many ticks it took to assemble.
        //
        // The BODIES are not recorded. They wear `MatchSeat` and
        // `match_participants` reads them off the world, which is what keeps the
        // activation free of entity references a rewind could invalidate.
        //
        // ⚠ **the ADOPTED seat belongs in here too**, and it did not used to be.
        // The old adoption branch pushed to `seated_bodies` and bumped the count
        // but never recorded `(index, body)`, so a match that adopted seat 0 and
        // spawned seat 1 on the SAME tick activated with `seats: 1` — a two-seat
        // match whose latch said one, which is precisely the "is the cast still
        // whole?" comparison reading false from the moment it opened. Invisible
        // until now because no test seated an adoption and a spawn on one tick;
        // the transaction makes that the ONLY way a match can activate, so it
        // would have gone from latent to certain.
        by_seat.extend(seated_this_pass);
        commands.insert_resource(ActiveMatch {
            seats: by_seat.len(),
            seat_topology: roster.seat_topology,
        });
    }
}

#[cfg(test)]
mod tests;
