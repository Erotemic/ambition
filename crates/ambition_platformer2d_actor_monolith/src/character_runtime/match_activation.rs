//! Match ACTIVATION: the kernel's side of a prepared match.
//!
//! Preparation is `ambition_match` (`prepare_match` and the immutable
//! [`PreparedMatch`] it produces). This module spawns the bodies that plan
//! names, binds their control, releases the opening hold and declares the
//! cast as the view -- construction and lifecycle, which are the kernel's.

use ambition_characters::prepared::PreparedCharacterRegistry;
use ambition_match::prepared::{
    prepare_match, ControlAuthority, MatchPreparationProblems, OpeningPhase, PreparedMatch,
    PreparedSeat,
};
use ambition_match::{ActiveMatch, MatchParticipantRoster, MatchSeat};
use bevy::prelude::*;

#[cfg(test)]
mod tests;

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
    ambition_body_seed::PhysicalBaseline::of(&seat.definition).apply_to_body(
        ambition_body_seed::BaselineBoundary::Construction,
        &mut commands.entity(body),
        None,
        // The weight rode the SEED (`seat.body_weight`), the way health and
        // geometry did — this boundary has only the mass left to write.
        None,
        None,
        ambition_body_seed::PhysicalRetraction::NONE,
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
    // No policy component at all means no home body (the match's own default).
    let home_body_spawns_a_body = home_body.is_some_and(|policy| policy.spawns_a_body());
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
        home_body_spawns_a_body,
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
/// [`ActiveMatch`](ActiveMatch), so activation re-runs — and it must find
/// the same plan waiting, or the cast it rebuilds is not the cast it built.
pub fn activate_the_prepared_match(
    mut commands: Commands,
    prepared: Option<Res<PreparedMatch>>,
    active: Option<Res<ActiveMatch>>,
    active_session: Option<Res<ambition_platformer2d_shared_tangle::lifecycle::ActiveSessionScope>>,
    // Seats that already have a body — derived from the world, so a rewind that
    // restored the fighters without the latch cannot double-build them.
    already_seated: Query<&MatchSeat>,
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
        let body = realize_seat(
            &mut commands,
            session_scope,
            seat,
            prepared.cast_generation(),
        );
        let mut seated = commands.entity(body);
        seated.insert(MatchSeat(seat.seat));
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
    commands.insert_resource(ActiveMatch::activated(
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
    active: Option<Res<ActiveMatch>>,
    prepared: Option<Res<PreparedMatch>>,
    mut held: Query<(Entity, &mut ambition_characters::control::ControlHolds), With<MatchSeat>>,
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
    active: Option<Res<ActiveMatch>>,
    seats: Query<(Entity, &MatchSeat)>,
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
