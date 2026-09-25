#![cfg(feature = "rl_sim")]
//! A character built through NPC placement and runtime summon retains the same
//! intrinsic body definition while contextual policy differs.
//!
//! The Hall path and summon path are intentionally distinct construction entry
//! points. The test asserts both intrinsic equality and contextual difference.

use crate::common::{base, fixed_60hz_room_sim};

use ambition_app::{AgentAction, Platformer2dSimHarness};
use ambition_platformer2d::characters::actor::BodyHealth;
use ambition_platformer2d::characters::actor::WornCharacter;
use ambition_platformer2d::combat::actor_tuning::ActorConfig;
use ambition_platformer2d::combat::components::{ActorDisposition, ActorIdentity};

const CHARACTER: &str = "npc_puppy_slug";

/// Intrinsic body facts that must not depend on construction context.
///
/// Geometry is excluded because the NPC and summon paths still disagree on body
/// size. Add it once that scaling discrepancy is resolved.
#[derive(Debug, PartialEq)]
struct IntrinsicBody {
    max_health: i32,
    max_run_speed: f32,
    weight: f32,
}

/// Read the one body WEARING `CHARACTER`, plus its disposition — which is
/// CONTEXT and is read separately on purpose.
///
/// `WornCharacter`, not `ActorIdentity`. The identity component carries
/// the PLACEMENT's feature id and display label; the character a body IS is what
/// it wears. Keying this test on the wrong one is how it would compare two
/// bodies that merely share a name.
///
/// `placement` is not optional decoration. `combat_calibration_lab` stages
/// a Puppy Slug of its own, so a query that asked only "who wears this
/// character" would have found the ROOM's body and compared it to the Hall's —
/// two authored placements, not two construction roads. Found while poisoning
/// this test, which is what a poison is for.
fn body_of(
    sim: &mut Platformer2dSimHarness,
    placement: Option<&str>,
) -> Option<(IntrinsicBody, ActorDisposition)> {
    let world = sim.world_mut();
    let mut q = world.query::<(
        &WornCharacter,
        &ActorIdentity,
        &ActorConfig,
        &BodyHealth,
        &ActorDisposition,
    )>();
    q.iter(world)
        .find(|(worn, identity, ..)| {
            worn.id() == CHARACTER && placement.is_none_or(|want| identity.id() == want)
        })
        .map(|(_, _, config, health, disposition)| {
            (
                IntrinsicBody {
                    max_health: health.max(),
                    max_run_speed: config.tuning.max_run_speed,
                    weight: config.tuning.weight,
                },
                *disposition,
            )
        })
}

/// The two roads agree about the body and differ about the context.
#[test]
fn one_character_built_as_an_npc_and_as_a_summon_is_the_same_body() {
    // ── Road 1: the authored NPC placement, through the Hall's own staging.
    let mut hall = fixed_60hz_room_sim("hall_of_characters");
    for _ in 0..90 {
        hall.step(base());
    }
    let (as_npc, npc_disposition) = body_of(&mut hall, None).unwrap_or_else(|| {
        panic!(
            "the Hall stages no body identified as `{CHARACTER}`, so this test \
             is comparing one road against nothing. Either the Hall's cast \
             changed or the identity stopped being written at construction"
        )
    });

    // ── Road 2: the runtime summon, the entry point a wave spawner or a boss
    // cascade uses. Different road, different context, same named character.
    let mut summoned = fixed_60hz_room_sim("combat_calibration_lab");
    for _ in 0..30 {
        summoned.step(base());
    }
    summoned.spawn_enemy_character_at(
        "cross_context_probe",
        // a display name that is NOT the character's, on purpose: if the body
        // resolved its facts by matching a name, this road would resolve nothing
        // and the comparison below would fail loudly rather than agree by luck.
        "Summoned Probe",
        (600.0, 300.0),
        (12.0, 16.0),
        ambition_platformer2d::entity_catalog::placements::CharacterBrain::Passive,
        CHARACTER,
    );
    for _ in 0..4 {
        summoned.step(AgentAction::default());
    }
    let (as_summon, summon_disposition) = body_of(&mut summoned, Some("cross_context_probe"))
        .unwrap_or_else(|| {
            panic!(
                "the summon road built no body identified as `{CHARACTER}` — a \
             programmatic spawn naming a registered character must produce that \
             character, which is the whole of P1.12"
            )
        });

    // non-degenerate, or two empty bodies would compare equal. A slug with
    // no health and no speed is what a body built from nothing looks like, and
    // this test would pass on two of them.
    assert!(
        as_npc.max_health > 0 && as_npc.max_run_speed > 0.0 && as_npc.weight > 0.0,
        "the NPC road produced a body with a zeroed intrinsic fact ({as_npc:?}), \
         so the comparison below could be satisfied by two empty bodies"
    );

    // ── The intrinsic facts: identical, or a character means different things
    // in different rooms.
    assert_eq!(
        as_npc, as_summon,
        "the same character built through the NPC road and the summon road \
         produced DIFFERENT bodies. One of the two is reading facts from its \
         context — the placement's size, a disposition-dependent pool — and that \
         is the authority split this campaign exists to remove"
    );

    // ── The contextual facts: allowed to differ, and here they must, or the
    // assertion above is satisfied by two copies of one context.
    assert_ne!(
        npc_disposition, summon_disposition,
        "both roads produced the same DISPOSITION ({npc_disposition:?}), so this \
         test compared two bodies in the same context and proved nothing about \
         context-independence. The Hall's slug is peaceful; a summoned one is \
         hostile"
    );
}

/// A Hall body can do what its catalog row grants, not merely what its
/// definition restates.
///
/// The Hall's `mary_o` has no body blueprint, so it is built on the peaceful
/// road, and its `RunJump` is authored on the CATALOG row rather than on the
/// definition. A seed that consults only the definition builds it with no verbs,
/// and the per-body gate then holds a possessed Mary-O still.
#[test]
fn a_hall_character_wears_the_abilities_its_catalog_row_grants() {
    use ambition_platformer2d::engine_core::body_clusters::{AbilityBase, BodyAbilities};

    let mut hall = fixed_60hz_room_sim("hall_of_characters");
    for _ in 0..90 {
        hall.step(base());
    }
    let world = hall.world_mut();
    let mut q = world.query::<(&WornCharacter, &ActorIdentity, &AbilityBase, &BodyAbilities)>();
    let found: Vec<_> = q
        .iter(world)
        .filter(|(worn, ..)| worn.id() == "mary_o")
        .map(|(_, identity, base, live)| (identity.id().to_string(), base.abilities, live.abilities))
        .collect();
    assert!(!found.is_empty(), "the Hall staged no body wearing `mary_o`");
    for (placement, base, live) in found {
        for (which, set) in [("AbilityBase", base), ("BodyAbilities", live)] {
            assert!(
                set.move_horizontal && set.jump,
                "the Hall's `{placement}` wears `mary_o`, whose catalog row grants RunJump, \
                 but its {which} cannot run or jump: {set:?}"
            );
        }
    }
}

/// Every Hall body wearing a prepared character carries exactly that
/// character's resolved actor verbs.
///
/// The census is the whole Hall, not one sample: a constructor that reached for
/// a default of its own would disagree with preparation on the characters that
/// state no verbs, and most of the Hall states none.
#[test]
fn every_hall_body_wears_the_actor_verbs_preparation_resolved() {
    use ambition_platformer2d::characters::prepared::PreparedCharacterRegistry;
    use ambition_platformer2d::engine_core::body_clusters::AbilityBase;

    let mut hall = fixed_60hz_room_sim("hall_of_characters");
    for _ in 0..90 {
        hall.step(base());
    }
    let world = hall.world_mut();
    let registry = world.resource::<PreparedCharacterRegistry>().clone();
    let mut q = world.query::<(&WornCharacter, &ActorIdentity, &AbilityBase)>();
    let mut unauthored = 0;
    let mut checked = 0;
    for (worn, identity, base) in q.iter(world) {
        let Some(prepared) = registry.get(worn.id()) else {
            continue;
        };
        // Flight is the body's locomotion, which the constructor adds on top.
        let mut built = base.abilities;
        built.fly = prepared.actor_abilities.fly;
        assert_eq!(
            built,
            prepared.actor_abilities,
            "the Hall's `{}` wears `{}` but was built with verbs preparation did not resolve",
            identity.id(),
            worn.id(),
        );
        checked += 1;
        if prepared.abilities.is_none() {
            unauthored += 1;
        }
    }
    assert!(
        unauthored > 0 && checked > unauthored,
        "the census needs both populations to mean anything: {checked} bodies \
         checked, {unauthored} of them unauthored"
    );
}
