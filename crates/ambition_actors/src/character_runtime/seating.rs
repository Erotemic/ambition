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

use ambition_engine_core::Vec2;

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
    session_scope: ambition_platformer_primitives::lifecycle::SessionSpawnScope,
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
) -> Option<Entity> {
    let prepared = registry.get(character_id)?;
    // **THE AUTHORED BODY BOX, when the character declared one.**
    //
    // `SEAT_BODY_PX` is a placeholder ON PURPOSE and stays the answer for a
    // character that authored nothing. But `BodySource::Explicit` had no consumer
    // anywhere in the repository, so a provider could author half-extents and
    // receive this constant instead (GPT 5.6, 2026-07-29).
    //
    // Spawn time is the right place for it: the box is a construction fact, and a
    // per-tick projection writing a live body's size would be a second geometry
    // authority beside the transit seam (ADR 0024).
    let body_px = match prepared.body.as_ref() {
        Some(super::BodySource::Explicit { half_extents }) => {
            Vec2::new(half_extents.0 * 2.0, half_extents.1 * 2.0)
        }
        _ => SEAT_BODY_PX,
    };
    let aabb = ambition_engine_core::Aabb::new(at, body_px / 2.0);
    // `new_in`, not the test-only `new`: production construction never has a
    // hidden catalog fallback, and a seated fighter resolves its sprite identity
    // from the SAME App-local catalog every other spawn path uses.
    let mut seed = crate::features::ecs::actor_clusters::ActorClusterSeed::new_in(
        authored_sheets,
        catalog,
        roster,
        character_id.to_string(),
        prepared.display_name.clone(),
        aabb,
        brain,
        &[],
    );
    seed.health = ambition_characters::actor::BodyHealth::new(
        ambition_characters::actor::Health::new(prepared.vitals.max_health.max(1)),
    );
    seed.kin.facing = facing;
    let centered = ambition_engine_core::CenteredAabb::from_center_size(at, body_px);
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
    use ambition_platformer_primitives::lifecycle::SpawnSessionScopedExt;
    Some(
        commands
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
            .id(),
    )
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

/// **The match that is LIVE, and the bodies that are in it.**
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
/// only be REPORTED and not repaired (queue Y′9). Naming the bodies and the
/// topology generation they were activated against makes that a question the
/// code can answer (2026-07-29).
///
/// Published in ONE insert, on the tick the last seat is filled. Never partially:
/// a roster whose seat 0 cannot adopt yet while seat 1 spawned fine must not
/// activate on the strength of seat 1.
#[derive(Resource, Debug, Clone, PartialEq)]
pub struct ActiveMatch {
    /// Seat order: `participants[i]` is the body seated at seat `i`.
    participants: Vec<Entity>,
    /// The frozen seat topology this match was activated against, copied from
    /// the roster so the two can be COMPARED rather than assumed equal.
    seat_topology: Option<u64>,
}

impl ActiveMatch {
    /// The bodies in this match, in seat order.
    pub fn participants(&self) -> &[Entity] {
        &self.participants
    }

    /// Which frozen topology decided this match's seating, if a session had
    /// frozen one when the roster was built.
    pub fn seat_topology(&self) -> Option<u64> {
        self.seat_topology
    }

    /// Build an activation directly, for a test that needs a LIVE match without
    /// standing up seating to produce one.
    ///
    /// The fields stay private so production has exactly one publisher; this is
    /// the hatch, and it is named for what it is.
    #[doc(hidden)]
    pub fn for_test(participants: Vec<Entity>, seat_topology: Option<u64>) -> Self {
        Self {
            participants,
            seat_topology,
        }
    }
}

/// Seat every CPU participant in [`MatchParticipantRoster`], once.
///
/// Runs on the sim schedule so a seated body exists on a tick boundary like every
/// other constructed entity, rather than mid-frame where half the pipeline has
/// already run.
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
        ambition_platformer_primitives::lifecycle::SessionWorldRef<
            ambition_engine_core::RoomGeometry,
        >,
    >,
    active_session: Option<Res<ambition_platformer_primitives::lifecycle::ActiveSessionScope>>,
    active: Option<Res<ActiveMatch>>,
    // Seats that already have a body. Derived from the world rather than
    // remembered, so a seat can never be counted twice and the retry below
    // cannot spawn a second copy of a fighter that seated fine last tick.
    already_seated: Query<(Entity, &MatchSeat)>,
    mut player: Query<
        (
            Entity,
            &mut ambition_characters::actor::BodyHealth,
            ambition_engine_core::BodyClusterQueryData,
            &mut crate::features::MotionModel,
            &ambition_characters::actor::WornCharacter,
        ),
        crate::actor::PrimaryPlayerOnly,
    >,
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
        ambition_platformer_primitives::lifecycle::SessionSpawnScope::for_optional_active_session(
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
    let mut seated_count = occupied.len();
    // Every body this pass produced, so the roster's suspension lands on all of
    // them in the SAME command flush that creates them. A body is therefore never
    // observable in a state the ruleset did not ask for — no window to narrow.
    let mut seated_bodies: Vec<Entity> = Vec::new();
    let mut seated_this_pass: Vec<(usize, Entity)> = Vec::new();
    for (index, participant) in roster.participants.iter().enumerate() {
        let MatchParticipant {
            character,
            controller,
            team,
            ..
        } = participant;
        // The roster's declared TEAM, on the body. It had been declared and read
        // by nothing since §7.8; `MatchTeam` is what the damage relation
        // consults, and it is what lets two human seats hit each other without
        // the stage switching on GLOBAL friendly fire (which also makes
        // teammates hittable, and is therefore wrong the moment a 2v2 exists).
        let team_tag = team
            .as_ref()
            .map(|team| crate::combat::targeting::MatchTeam::new(team.clone()));
        // Already has a body from an earlier tick's partial seating.
        if occupied.contains(&index) {
            continue;
        }
        let (at, facing) = seat_for(index, centre);
        // A HUMAN seat is the body the player already has. Adopt it — move it to
        // its seat and face it inward — rather than spawning a second body
        // wearing the same character, which is what produced two Mary-Os in the
        // arena the first time this stage shipped.
        if let super::ControllerBinding::Human { device_slot } = controller {
            let slot = ambition_characters::brain::PlayerSlot(*device_slot);
            // A SECOND human is a second body. Only slot 0 has a body already —
            // the one the session spawned as the primary player — so every other
            // seat is spawned and handed its own `Brain::Player(slot)`.
            //
            // `tick_player_brains` already drives any body whose brain names a
            // slot, and `SlotControls` already holds four. What couch versus was
            // missing is not the engine: it is a writer for the second slot. This
            // seats the body that writer will drive.
            if slot != ambition_characters::brain::PlayerSlot::PRIMARY {
                if let Some(body) = seat_character(
                    &mut commands,
                    session_scope,
                    &registry,
                    &catalog,
                    &authored_sheets,
                    &archetypes,
                    character,
                    at,
                    facing,
                    faction_for(index),
                    // `Passive` is the authored brain the seed needs; the insert
                    // below replaces the runtime `Brain` with the player slot. A
                    // passive placeholder rather than a wandering one so a body
                    // whose player writer never arrives stands still instead of
                    // strolling off looking possessed.
                    ambition_entity_catalog::placements::CharacterBrain::Passive,
                ) {
                    seated_count += 1;
                    if let Some(team) = team_tag.clone() {
                        commands.entity(body).insert(team);
                    }
                    commands.entity(body).insert((
                        MatchSeat(index),
                        ambition_characters::brain::Brain::Player(slot),
                        crate::control::components::LocalPlayer,
                        crate::control::components::PlayerInputFrame::default(),
                    ));
                    seated_bodies.push(body);
                    seated_this_pass.push((index, body));
                }
                continue;
            }
            let Ok((body, mut health, clusters, mut model, worn)) = player.single_mut() else {
                continue;
            };
            if worn.id() != character {
                // The stage's starting character and this seat disagree. Seating
                // does not re-dress the player body — that is `WornCharacter`'s
                // job and a stage that wants a different fighter should say so in
                // its `StartingCharacter`.
                continue;
            }
            // Through `transit_body`, the ONE transit authority (ADR 0024): a
            // bare `kin.pos = at` is a pose write the kernel never sees, so the
            // body arrives believing it is still standing on the floor it left.
            // The workspace policy caught that draft, and was right to.
            // **THE AUTHORED MAXIMUM, on the adopted body too.** A seat starts at
            // FULL health, adopted or spawned — and at the SAME maximum its
            // character authored, whichever way it got its body.
            //
            // A spawned seat took `prepared.vitals.max_health` from the seed. The
            // adopted primary player did not: it kept the maximum its session
            // established from the legacy catalog or the default player health, so
            // the same character could bring 60 HP as player two and something else
            // entirely as player one. The versus duelists author 60 and 52 — a
            // deliberate trade, one fighter paying for a faster smash — and that
            // trade simply did not apply to seat 0 (GPT 5.6, 2026-07-29).
            //
            // Match activation is the right long-term home for this (it is a
            // once-per-match decision, not a per-tick projection, and health is
            // live state the rest of the time). Until that seam exists, adoption
            // is the one place a body becomes a fighter, so it is where the
            // authored maximum has to land.
            if let Some(prepared) = registry.get(character) {
                health.health.max = prepared.vitals.max_health.max(1);
            }
            health.health.current = health.health.max;
            let mut item = clusters;
            let mut clusters = item.as_clusters_mut();
            ambition_engine_core::movement::transit_body(
                &mut model,
                &mut clusters,
                at,
                ambition_engine_core::movement::TransitVelocity::Zero,
            );
            clusters.kinematics.facing = facing;
            // The adopted PRIMARY PLAYER needs it most. Its death runs
            // `death_respawn_player`, which teleports it to the room spawn and
            // restores full health BEFORE any rules layer can look — so seat 0
            // could never be seen at zero health, and the match was rigged in
            // its favour (GPT 5.6, 2026-07-27).
            commands.entity(body).insert((
                MatchSeat(index),
                crate::combat::components::RulesetOwnsDeath,
            ));
            seated_bodies.push(body);
            // The TEAM, which this branch dropped when the death-ownership
            // insert was added over it. A seat with no team is judged by FACTION
            // alone, and `effective_faction` maps every player-brained body to
            // `Player` — so in an all-human match the adopted seat 0 could not
            // be hit by anybody, which is the 1v1 rigging bug again wearing a
            // different hat. Found by the 2v2 test (2026-07-27).
            if let Some(team) = team_tag.clone() {
                commands.entity(body).insert(team);
            }
            seated_count += 1;
            continue;
        }
        let Some(profile) = controller.brain_profile() else {
            continue;
        };
        let faction = faction_for(index);
        if let Some(body) = seat_character(
            &mut commands,
            session_scope,
            &registry,
            &catalog,
            &authored_sheets,
            &archetypes,
            character,
            at,
            facing,
            faction,
            ambition_entity_catalog::placements::CharacterBrain::Custom(profile.to_string()),
        ) {
            commands.entity(body).insert(MatchSeat(index));
            if let Some(team) = team_tag.clone() {
                commands.entity(body).insert(team);
            }
            seated_bodies.push(body);
            seated_this_pass.push((index, body));
            seated_count += 1;
        }
    }
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
                ambition_engine_core::BodyAbilities::new(abilities),
                ambition_engine_core::AbilityBase::new(abilities),
            ));
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
        // earlier ticks; this pass's go in on top, so the published list is the
        // whole cast in seat order however many ticks it took to assemble.
        by_seat.extend(seated_this_pass);
        commands.insert_resource(ActiveMatch {
            participants: by_seat.into_values().collect(),
            seat_topology: roster.seat_topology,
        });
    }
}

#[cfg(test)]
mod tests;
