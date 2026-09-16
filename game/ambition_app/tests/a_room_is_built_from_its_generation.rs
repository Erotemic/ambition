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

/// Every distinct peer-stable CONTENT term the live world's construction stamps
/// carry.
///
/// ⛔ **THE TERM, NOT THE CHECKSUM, AND THAT IS WHAT MAKES THE ARM BELOW
/// POSSIBLE.** `peer_stable_checksum` folds the content term together with the
/// room, so any two rooms differ no matter what their content term says — which
/// is precisely how a collapsed term hides inside a projection that still looks
/// well-behaved. `TransactionId::peer_content_term` is the one rule that decides
/// which content two peers compare, read from its owner rather than re-spelled
/// here.
///
/// ⚠ `"runtime-dynamic"` IS A LEGITIMATE ANSWER and is kept in the output rather
/// than filtered: a projectile or a dropped item is not content-derived at all.
/// `"content-unstated"` is the one that means *"content-derived, and nobody said
/// which"*.
/// Does this term name WHICH prepared content, as opposed to declining to?
///
/// The three shapes `TransactionId::peer_content_term` can answer are a stated
/// digest, the constant `"content-unstated"` and the constant `"runtime-dynamic"`.
/// Only the first is an identity two peers can disagree about, so a floor over
/// "stamps exist" has to be a floor over THESE.
fn is_stated_content(term: &str) -> bool {
    term != "content-unstated" && term != "runtime-dynamic"
}

/// The same predicate over a `&String`, for `Iterator::any` on a `Vec<String>`.
fn is_stated_content_ref(term: &String) -> bool {
    is_stated_content(term)
}

fn stamped_content_terms(sim: &mut ambition_app::Platformer2dSimHarness) -> Vec<String> {
    let world = sim.world_mut();
    let mut query =
        world.query::<&ambition_platformer2d::platformer::construction::TransactionId>();
    let mut out: Vec<String> = query
        .iter(world)
        .map(|stamp| stamp.peer_content_term().to_string())
        .collect();
    out.sort();
    out.dedup();
    out
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

    // ⛔⛤ A SECOND, INDEPENDENT CLAIM ABOUT THE SAME RECONSTRUCTION: it must
    // stamp what it built with the session's prepared content. The death road
    // reaches `room_transition/loading.rs`, which answered `content_unstated`
    // for the INCOMING binding until 2026-09-16 — so every body rebuilt here
    // carried a construction provenance no peer could disagree with. Stated as
    // its own assertion, with its own message, so a failure says WHICH of the
    // two claims broke. See
    // `an_ordinary_room_transition_stamps_its_roots_with_the_session_content`.
    let terms = stamped_content_terms(&mut sim);
    assert!(
        terms.iter().any(is_stated_content_ref),
        "after the death rebuild no construction stamp names any prepared \
         content (terms={terms:?}), so the assertion below would be vacuous"
    );
    assert!(
        !terms.iter().any(|term| term == "content-unstated"),
        "the death rebuild stamped its roots `content-unstated` (terms={terms:?}). \
         A rebuild is made of the generation already running, so that generation \
         is its incoming content identity — two peers at different prepared \
         content would project the same construction provenance."
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

    // ⛔⛤ THE SAME SECOND CLAIM, ON THE ONE ROAD THE OTHER TWO ARMS CANNOT REACH.
    // `session/reset/mod.rs` stated `content_unstated(ContentEpoch::default())`
    // for its incoming binding — the DEFAULT epoch, on a road that has a real
    // live one — so a reset erased the prepared content identity from every root
    // it rebuilt. Measured separately from the transition road, because the
    // poison separation in this file's headers shows the two roads are distinct.
    let terms = stamped_content_terms(&mut sim);
    assert!(
        terms.iter().any(is_stated_content_ref),
        "after the new-game reset no construction stamp names any prepared \
         content (terms={terms:?}), so the assertion below would be vacuous"
    );
    assert!(
        !terms.iter().any(|term| term == "content-unstated"),
        "the new-game reset stamped its roots `content-unstated` \
         (terms={terms:?}), so the reset road erased the prepared content \
         identity the session activated under."
    );
}

/// ⛔⛤ **AN ORDINARY ROOM TRANSITION STAMPS ITS ROOTS WITH THE SESSION'S
/// PREPARED CONTENT — AND IT DID NOT.**
///
/// `ConstructionScope` holds two bindings on purpose: `expected_live` is the
/// generation a plan will be COMMITTED INTO, `incoming` is the generation its
/// content CAME FROM, and `incoming` is what every root's `TransactionId`
/// carries. For every road except a content replacement the two coincide —
/// which is why `ConstructionScope::in_generation` cannot express a split.
///
/// ⇒ **THE LAYER ABOVE REOPENED THE HOLE.** `ActorConstructionContext::
/// for_room_construction` took `content` and `active_binding` as SEPARATE
/// parameters and applied the second to the expected-live half only, so a
/// caller could state one generation for the boundary and another for the
/// roots. Three production roads — the door transition, the reset and the
/// neighbour prefetch — read that parameter as *"the content THIS ROAD
/// publishes"*, answered `content_unstated` because a transition publishes
/// none, and stamped every root they built `content-unstated + room` instead of
/// `PreparedContentIdentity(C) + room`. Each of their comments reasoned
/// correctly about the boundary half and none about the identity half.
///
/// ⛔⛤ **AND THE CAMPAIGN'S OWN GUARD WAS GREEN OVER IT, STRUCTURALLY.**
/// `two_hosts_at_different_content_epochs_share_one_construction_provenance`
/// asserts two hosts holding the same content AGREE — and `content-unstated`
/// agrees with `content-unstated` perfectly, so erasing the discriminating term
/// makes that assertion MORE true. An equality arm is satisfied by every
/// function that throws information away, the constant function included.
/// ⇒ The missing half is DISAGREEMENT: two hosts at different prepared content
/// must project differently. Until a fixture can activate two distinct prepared
/// packs, this arm asserts the term itself rather than the projection.
///
/// ⚠ AND IT ASSERTS THE TERM RATHER THAN `peer_stable_checksum` FOR A SECOND
/// REASON: the projection is `content ⊗ room`, so any two ROOMS differ whatever
/// their content term says. The collapse was invisible inside a value that
/// still behaved well.
#[test]
fn an_ordinary_room_transition_stamps_its_roots_with_the_session_content() {
    let mut sim = fixed_60hz_sim();
    for _ in 0..8 {
        sim.step(base());
    }

    // ⛔ PREMISE: this composition activated a generation that NAMED its
    // content. Without a stated fingerprint on the session binding,
    // `content-unstated` is the honest answer everywhere and the arm would be
    // about the fixture rather than about the road.
    let live_binding = ambition_platformer2d::platformer::lifecycle::session_world_component::<
        ambition_platformer2d::actors::rooms::ActiveContentBinding,
    >(sim.world())
    .map(|binding| binding.0);
    assert!(
        live_binding
            .and_then(|binding| binding.peer_content())
            .is_some_and(|content| content.is_stated()),
        "this session's `ActiveContentBinding` names no prepared content \
         ({live_binding:?}), so `content-unstated` is the honest answer \
         everywhere and this arm cannot tell the defect from the fixture"
    );

    let before_room = active_room(&mut sim);
    let initial = stamped_content_terms(&mut sim);

    // ⛔ THE ANTI-VACUITY FLOOR, ON THE STARTING ROOM — the activation road is
    // the one that was already correct, so if it states nothing then the
    // comparison after the door has no working reference to be measured against.
    assert!(
        initial.iter().any(|term| is_stated_content(term)),
        "not one construction stamp in the STARTING room names a prepared \
         content identity (terms={initial:?}), so the activation road states \
         none either and the door below cannot be shown to lose one"
    );

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
         below would be about the room this test started in"
    );

    let after = stamped_content_terms(&mut sim);
    assert!(
        after.iter().any(|term| is_stated_content(term)),
        "room `{after_room}` holds no construction stamp naming any prepared \
         content (terms={after:?}), so 'none of them is unstated' would be \
         vacuously true"
    );
    assert!(
        !after.iter().any(|term| term == "content-unstated"),
        "a room reached through an ORDINARY DOOR stamped its roots \
         `content-unstated` (terms={after:?}, room `{after_room}`), while the \
         room this session activated into stated {initial:?}. A transition \
         rebuilds a room the ALREADY-ACTIVE prepared generation defines, so \
         that generation IS its incoming content identity — publishing no \
         content of its own is a fact about the commit boundary, not about \
         provenance. Two peers running DIFFERENT prepared content therefore \
         project the SAME construction provenance, which is the peer-stable \
         identity `TransactionId::peer_stable_checksum` exists to provide."
    );
}
