//! ⛔⛤ **`Q121` HALF TWO: A ROOM BUILT AFTER ACTIVATION IS BUILT FROM THE
//! GENERATION THAT WAS ACTIVATED — NOT FROM WHATEVER THE App HOLDS BY THEN.**
//!
//! The review that reopened `Q121` measured the lifetime defect:
//! `PreparedPlatformerSessions::take` handed `FrozenMechanicalState` transiently
//! to the builder and nothing promoted it, so *"a prepared generation means its
//! frozen mechanical values"* was true of the FIRST construction call and of
//! nothing else. A session prepared under N could walk through a door and get
//! N+1's fighters.
//!
//! ⭐⭐ **THE MECHANISM LANDED (`SessionMechanics` + `GenerationMechanics`) AND
//! THE WITNESS DID NOT, WHICH IS WHY THIS FILE EXISTS.** The unit arms in
//! `session::mechanics` prove the PROJECTION ranks the generation over the App.
//! They cannot prove the shipped ROOM-TRANSITION road hands it the generation —
//! that is a fact about `room_transition/loading.rs`'s `SystemParam`, not about
//! `GenerationMechanics::characters()`. This repository has been bitten by
//! exactly that gap before: *"a guard can certify the DECLARATION while the
//! MECHANISM is a REGISTRATION."*
//!
//! ⛔ **THE POISON THIS MUST SURVIVE** is the one the defect was: make the
//! transition road prefer the App's registry, and the destination room must be
//! built from a cast this session never activated.

use ambition_platformer2d::engine_core::AabbExt;
use bevy::prelude::With;

use crate::common::{base, fixed_60hz_sim};

/// An EMPTY cast — the loudest possible replacement.
///
/// ⭐ **A REPLACEMENT ROSTER, NOT A MUTATED ONE, AND THE REGISTRY'S OWN DESIGN
/// IS WHY.** `PreparedCharacterRegistry` exposes no mutable accessor — that is
/// deliberate, and `Q111` and `Q121` both lean on it — so "bump every
/// character's max_health" is not available from outside the crate that owns it.
///
/// ⛔ **EMPTY IS ALSO THE STRONGEST PROBE, not a convenience.** A road that reads
/// the App would find EVERY destination character absent, which shows up as
/// bodies that do not exist rather than as a changed number nobody notices. And
/// the arm asserts the live cast is non-empty first, so "empty" is a real
/// difference rather than two empties agreeing.
fn an_empty_cast() -> ambition_platformer2d::character::PreparedCharacterRegistry {
    ambition_platformer2d::character::PreparedCharacterRegistry::default()
}

/// Every CHARACTER id the live world has a body for.
///
/// ⛔⛤ **`WornCharacter`, NOT `ActorIdentity` — AND THE FIRST VERSION OF THIS ARM
/// USED THE WRONG ONE AND FAILED FOR IT.** `ActorIdentity.id` is the PLACEMENT's
/// identity (`NpcSpawn-0017`, or a bare UUID), so comparing it to a cast's ids
/// compares two different vocabularies and answers "no overlap" for every room
/// in the game. The failure looked exactly like the defect: *"not one of its
/// bodies carries an id from the cast this session activated."* ⇒ A key that is
/// the wrong kind of name is not an identity, and a census keyed on one reports
/// a devastating finding about nothing.
fn constructed_characters(sim: &mut ambition_app::Platformer2dSimHarness) -> Vec<String> {
    let world = sim.world_mut();
    let mut query = world.query::<&ambition_platformer2d::characters::actor::WornCharacter>();
    let mut out: Vec<String> = query.iter(world).map(|worn| worn.0.to_string()).collect();
    out.sort();
    out.dedup();
    out
}

fn active_room(sim: &mut ambition_app::Platformer2dSimHarness) -> String {
    sim.observation().active_room.clone()
}

/// Stand the controlled body in an authored `Door` zone of the active room.
fn stand_in_a_door(sim: &mut ambition_app::Platformer2dSimHarness) -> Option<String> {
    let door = {
        let world = sim.world_mut();
        let mut query = world.query::<&ambition_platformer2d::world::rooms::RoomSet>();
        let room_set = query.iter(world).next()?;
        room_set
            .active_loading_zones()
            .iter()
            .find(|zone| {
                zone.activation == ambition_platformer2d::world::rooms::LoadingZoneActivation::Door
            })
            .cloned()?
    };
    let world = sim.world_mut();
    let mut player = world.query_filtered::<&mut ambition_platformer2d::platformer::body::BodyKinematics, With<ambition_platformer2d::platformer::markers::PrimaryPlayer>>();
    let mut kin = player.single_mut(world).ok()?;
    kin.pos = door.aabb.center();
    kin.vel = ambition_platformer2d::engine_core::Vec2::ZERO;
    Some(door.name.clone())
}

fn interact() -> ambition_app::AgentAction {
    ambition_app::AgentAction {
        interact: true,
        ..ambition_app::AgentAction::default()
    }
}

#[test]
fn a_room_entered_after_activation_is_built_from_the_generation_not_the_app() {
    let mut sim = fixed_60hz_sim();
    for _ in 0..8 {
        sim.step(base());
    }

    // ⛔ PREMISE 1: this composition really did activate a generation. Without
    // it `GenerationMechanics` falls back to the App by design and the whole arm
    // is about the fallback rather than the freeze.
    assert!(
        sim.world_mut()
            .get_resource::<ambition_platformer2d::actors::session::mechanics::SessionMechanics>()
            .is_some(),
        "no `SessionMechanics` in this harness, so there is no activated \
         generation for the App's registry to be stale against"
    );

    let before_room = active_room(&mut sim);
    let stranger = an_empty_cast();

    // ⛔ PREMISE 2: the replacement roster really is a stranger's. An overlap
    // would let a road that reads the App still build the right bodies.
    let live_ids: Vec<String> = sim
        .world_mut()
        .resource::<ambition_platformer2d::character::PreparedCharacterRegistry>()
        .ids()
        .map(str::to_string)
        .collect();
    assert!(
        !live_ids.is_empty(),
        "the App's published cast is EMPTY, so replacing it changes nothing"
    );
    assert!(
        stranger.is_empty(),
        "the replacement cast is not empty, so a road reading the App might \
         still find the destination room's characters in it"
    );

    // THE EDIT: a cast published into the App with NO admitted transition for
    // this session. Exactly what a developer reload does between a door and the
    // room behind it.
    sim.world_mut().insert_resource(stranger);

    // ⭐ AND THE FREEZE IS ASSERTED SEPARATELY FROM THE ROAD, so a failure below
    // says WHICH half broke: the generation still holds its own cast...
    let frozen_ids: Vec<String> = sim
        .world_mut()
        .resource::<ambition_platformer2d::actors::session::mechanics::SessionMechanics>()
        .characters
        .as_ref()
        .expect("the activated generation published a cast")
        .ids()
        .map(str::to_string)
        .collect();
    assert!(
        frozen_ids.iter().any(|id| live_ids.contains(id)),
        "replacing the App's registry also changed the generation's, so the \
         freeze is a HANDLE and not a snapshot"
    );

    // ...and now the ROAD. Take a real door, which reconstructs a room.
    assert!(
        stand_in_a_door(&mut sim).is_some(),
        "the starting room authors no door, so this arm cannot reach a room \
         transition at all"
    );
    for _ in 0..90 {
        sim.step(interact());
        if active_room(&mut sim) != before_room {
            break;
        }
    }
    let after_room = active_room(&mut sim);
    assert_ne!(
        after_room, before_room,
        "the door never opened, so no room was reconstructed and the assertion \
         below would pass against the room this test started in"
    );

    // ⛔ THE ASSERTION THE ARM IS FOR.
    let built = constructed_characters(&mut sim);
    assert!(
        !built.is_empty(),
        "room `{after_room}` constructed NO body carrying a character, so 'it \
         was built \
         from the generation' is vacuously true — pick a destination with a cast"
    );
    assert!(
        built.iter().any(|id| live_ids.contains(id)),
        "room `{after_room}` was constructed while the App's published cast was \
         EMPTY, and not one of its bodies carries an id from the cast this \
         session activated. The room was rebuilt from whatever the App holds \
         NOW — which is `Q121`'s lifetime defect, reached through a door. \
         built={built:?}"
    );
}

/// ⛔⛤ **THE OTHER ROAD `Q121` NAMES — AND THE NEW-GAME RESET IS NOT WHAT THIS
/// ARM REACHES. THE POISON SAID SO.**
///
/// It was written as the reset half, poisoning
/// `session/reset/mod.rs`'s `for_live_session` call to prove it. **The poison did
/// not fire and the arm stayed green**, which is a finding about the FIXTURE:
/// `resume_at_checkpoint_on_reset` routes a death through the room-TRANSITION
/// road DELIBERATELY — *"a session opening at a checkpoint and a death returning
/// to one are the same question asked twice"* — so a death is a second exercise
/// of the road the arm above already covers, not a first exercise of a new one.
///
/// ⭐⭐ **AND THE TWO POISONS NOW SEPARATE THE ROADS, WHICH IS THE RESULT.**
/// MEASURED at HEAD: poisoning `room_transition/loading.rs`'s resolver reddens
/// this arm; poisoning `session/reset/mod.rs`'s does NOT. ⇒ A death reaches the
/// TRANSITION road, exactly as `resume_at_checkpoint_on_reset` says it should.
///
/// ⇒ **SO IT IS KEPT, RETITLED, FOR WHAT IT PROVES:** the transition road holds
/// the freeze when it is reached through a DEATH rather than through a door —
/// two different callers of one reconstruction, and the door arm alone does not
/// cover the second.
///
/// ✅ **AND THE NEW-GAME RESET ROAD NOW HAS ITS OWN WITNESS** —
/// `a_new_game_reset_rebuilds_the_world_from_the_generation_not_the_app`, below. The
/// three arms are MEASURED to cover three distinct roads with no overlap: the
/// transition poison reddens the door and death arms and not the reset arm; the reset
/// poison reddens the reset arm and not the other two.
#[test]
fn a_death_rebuilds_the_world_from_the_generation_not_the_app() {
    let mut sim = fixed_60hz_sim();
    for _ in 0..8 {
        sim.step(base());
    }

    assert!(
        sim.world_mut()
            .get_resource::<ambition_platformer2d::actors::session::mechanics::SessionMechanics>()
            .is_some(),
        "no `SessionMechanics` in this harness, so there is no activated \
         generation for the App's registry to be stale against"
    );
    let live_ids: Vec<String> = sim
        .world_mut()
        .resource::<ambition_platformer2d::character::PreparedCharacterRegistry>()
        .ids()
        .map(str::to_string)
        .collect();
    assert!(
        !live_ids.is_empty(),
        "the App's published cast is EMPTY, so replacing it changes nothing"
    );
    // ⚠ AND THE ROOM MUST ALREADY HOLD SOMEBODY, or "the rebuild kept them" is a
    // statement about a room that never had anyone.
    let before_entities = {
        let world = sim.world_mut();
        let mut q = world.query_filtered::<bevy::prelude::Entity, bevy::prelude::With<ambition_platformer2d::characters::actor::WornCharacter>>();
        let mut v: Vec<String> = q.iter(world).map(|e| format!("{e:?}")).collect();
        v.sort();
        v
    };
    let before = constructed_characters(&mut sim);
    assert!(
        !before.is_empty(),
        "the starting room holds no body carrying a character, so a rebuild has \
         nothing to get right"
    );

    sim.world_mut().insert_resource(an_empty_cast());

    // THE SHIPPED DEATH ROAD: a hazard kill, the interlude, and the rebuild it
    // asks for.
    let victim = {
        let world = sim.world_mut();
        let mut query = world.query_filtered::<bevy::prelude::Entity, With<ambition_platformer2d::platformer::markers::PrimaryPlayer>>();
        query.single(world).expect("the harness drives a player body")
    };
    let pos = {
        let world = sim.world_mut();
        let mut query = world.query_filtered::<&ambition_platformer2d::platformer::body::BodyKinematics, With<ambition_platformer2d::platformer::markers::PrimaryPlayer>>();
        query.single(world).expect("the player body has kinematics").pos
    };
    sim.world_mut().write_message(
        ambition_platformer2d::combat::death_rules::ActorDiedMessage {
            victim,
            pos,
            cause: ambition_platformer2d::combat::death_rules::DeathCause {
                source: ambition_platformer2d::combat::HitSource::Hazard,
                attacker: None,
            },
        },
    );
    sim.step_n(base(), 240);

    let after_entities = {
        let world = sim.world_mut();
        let mut q = world.query_filtered::<bevy::prelude::Entity, bevy::prelude::With<ambition_platformer2d::characters::actor::WornCharacter>>();
        let mut v: Vec<String> = q.iter(world).map(|e| format!("{e:?}")).collect();
        v.sort();
        v
    };
    // ⛔⛤ **ONLY THE BODIES THE DEATH ACTUALLY BUILT, AND THE FIRST VERSION OF
    // THIS ARM DID NOT MAKE THAT DISTINCTION.** A death in this harness rebuilds
    // PART of the room: measured, one body was replaced (`495v0` → `524v0`) and
    // one SURVIVED (`513v0`). Asserting over every body therefore passed on the
    // survivor, and BOTH poisons — the transition road's resolver and the reset
    // road's — left it green. A fixture satisfied by something the subject did
    // not touch is not measuring the subject.
    let newly_built: Vec<String> = {
        let world = sim.world_mut();
        let mut query = world.query_filtered::<(
            bevy::prelude::Entity,
            &ambition_platformer2d::characters::actor::WornCharacter,
        ), ()>();
        let mut out: Vec<String> = query
            .iter(world)
            .filter(|(entity, _)| !before_entities.contains(&format!("{entity:?}")))
            .map(|(_, worn)| worn.0.to_string())
            .collect();
        out.sort();
        out.dedup();
        out
    };
    assert!(
        !newly_built.is_empty(),
        "the death CONSTRUCTED NOTHING — every body carrying a character was \
         already there before it. before={before_entities:?} after={after_entities:?}. \
         ⇒ This arm would then be a statement about survivors, which is how its \
         first version passed under two different poisons."
    );
    assert!(
        newly_built.iter().any(|id| live_ids.contains(id)),
        "a body CONSTRUCTED by the death carries no id from the cast this \
         session activated, while the App's published cast was EMPTY. The \
         reconstruction went back to whatever the App holds now — `Q121`'s \
         lifetime defect, reached through a death rather than a door. \
         newly_built={newly_built:?} before={before:?}"
    );
}

/// ⛔⛤ **THE NEW-GAME RESET ROAD, WHICH THE OTHER TWO ARMS DO NOT REACH — `Q121`,
/// 2026-09-13.**
///
/// The door arm and the death arm both exercise `room_transition/loading.rs`'s
/// resolver; MEASURED, poisoning `session/reset/mod.rs`'s reddens neither,
/// because `resume_at_checkpoint_on_reset` routes a death through the TRANSITION
/// road deliberately. This one drives `NewGameResetRequested` →
/// `process_new_game_reset_request`, which is the only road that reaches the
/// reset resolver.
///
/// ⚠ **IT ASSERTS OVER BODIES THE RESET ACTUALLY REBUILT**, not over every body
/// present — the vacuity the death arm had to be repaired for, where a SURVIVOR
/// satisfied the postcondition under two different poisons.
///
/// ⭐ **POISON-VERIFIED, AND THE SEPARATION IS THE RESULT:** pointing
/// `session/reset/mod.rs`'s resolver at the App reddens THIS arm and leaves the
/// door and death arms green; pointing `room_transition/loading.rs`'s at the App
/// reddens those two and leaves this one green. Three arms, three roads, no
/// overlap — which is what makes the set a coverage claim rather than three
/// copies of one.
#[test]
fn a_new_game_reset_rebuilds_the_world_from_the_generation_not_the_app() {
    let mut sim = fixed_60hz_sim();
    for _ in 0..8 {
        sim.step(base());
    }

    assert!(
        sim.world_mut()
            .get_resource::<ambition_platformer2d::actors::session::mechanics::SessionMechanics>()
            .is_some(),
        "no `SessionMechanics` in this harness, so there is no activated \
         generation for the App's registry to be stale against"
    );
    let live_ids: Vec<String> = sim
        .world_mut()
        .resource::<ambition_platformer2d::character::PreparedCharacterRegistry>()
        .ids()
        .map(str::to_string)
        .collect();
    assert!(!live_ids.is_empty(), "the App's published cast is EMPTY");

    let before_entities = {
        let world = sim.world_mut();
        let mut q = world.query_filtered::<bevy::prelude::Entity, bevy::prelude::With<ambition_platformer2d::characters::actor::WornCharacter>>();
        let mut v: Vec<String> = q.iter(world).map(|e| format!("{e:?}")).collect();
        v.sort();
        v
    };

    sim.world_mut().insert_resource(an_empty_cast());

    // THE SHIPPED NEW-GAME RESET ROAD, by its own resource — the only way to
    // execute `process_new_game_reset_request` and its paired sweep.
    sim.world_mut()
        .resource_mut::<ambition_platformer2d::actors::session::reset::NewGameResetRequested>()
        .request();
    sim.rebase_rollback_history()
        .expect("the pending reset folds into the rollback baseline");
    for _ in 0..8 {
        sim.step(base());
    }

    let newly_built: Vec<String> = {
        let world = sim.world_mut();
        let mut query = world.query_filtered::<(
            bevy::prelude::Entity,
            &ambition_platformer2d::characters::actor::WornCharacter,
        ), ()>();
        let mut out: Vec<String> = query
            .iter(world)
            .filter(|(entity, _)| !before_entities.contains(&format!("{entity:?}")))
            .map(|(_, worn)| worn.0.to_string())
            .collect();
        out.sort();
        out.dedup();
        out
    };
    assert!(
        !newly_built.is_empty(),
        "the new-game reset CONSTRUCTED NOTHING — every body carrying a character \
         was already there before it. ⇒ This arm would then be a statement about \
         survivors, which is how the death arm passed under two poisons."
    );
    assert!(
        newly_built.iter().any(|id| live_ids.contains(id)),
        "a body CONSTRUCTED by the new-game reset carries no id from the cast \
         this session activated, while the App's published cast was EMPTY. The \
         reset road went back to whatever the App holds now — `Q121`'s lifetime \
         defect on the one road the door and death arms cannot reach. \
         newly_built={newly_built:?}"
    );
}
