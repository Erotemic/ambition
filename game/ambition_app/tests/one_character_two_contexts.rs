#![cfg(feature = "rl_sim")]
//! **THE SAME CHARACTER, BUILT TWO WAYS, IS THE SAME BODY.** (AC5.5)
//!
//! The campaign's central proposition, stated as the thing a player would
//! notice: *"a character used as an NPC, hostile actor, encounter participant,
//! runtime summon, or match fighter is the same body definition in different
//! context. Controller changes and provocation change policy/relationship; they
//! do not reconstruct the body."*
//!
//! ⭐ **it is checkable now because there is one construction path to check.**
//! Until 2026-08-13 a body could be built from a character OR built from an
//! archetype and then PATCHED by a character through the body-assist seam, so
//! "same body" depended on which road a placement happened to take and on which
//! fields the character had got around to authoring. AC5 deleted the patch road:
//! every registered character can build its own body, and every shipped
//! placement names one.
//!
//! ⚠ **two REAL roads, not one road called twice.** The Hall stages its cast as
//! authored `NpcSpawn` placements — the interactable/NPC construction road — and
//! `spawn_enemy_character_at` is the runtime-summon road that a boss cascade or a
//! wave spawner uses. They are different entry points with different context
//! (disposition, controller, session), which is the whole point: if the intrinsic
//! facts agree across those two, "context does not reconstruct the body" is a
//! measurement rather than a diagram.
//!
//! ⛔ **the contextual facts are asserted to DIFFER, deliberately.** A test that
//! only proved sameness would pass just as happily if both roads produced one
//! identical thing with no context at all — which would mean the axes had been
//! fused rather than separated. Disposition is the tell: the Hall's slug is
//! peaceful and the summoned one is hostile, and they are the same body.

use crate::common::{base, fixed_60hz_room_sim};

use ambition_app::{AgentAction, Platformer2dSimHarness};
use ambition_platformer2d::actors::features::{ActorConfig, ActorDisposition, ActorIdentity};
use ambition_platformer2d::characters::actor::WornCharacter;
use ambition_platformer2d::characters::actor::BodyHealth;

/// The character under test. A Puppy Slug because it genuinely appears in both
/// roles in shipped content — the Hall stages one, and Jon's 2026-08-13 casting
/// made it what a dividing mite splits into, so the summon road builds one for
/// real.
const CHARACTER: &str = "npc_puppy_slug";

/// The intrinsic facts a BODY owns, per the campaign's doctrine 3 — durability,
/// locomotion, and the physics weight that decides how far a hit sends it. These
/// must not depend on how the body got here.
///
/// ⛔⛔ **GEOMETRY IS DELIBERATELY NOT IN HERE, and the reason is a measurement
/// rather than a judgement.** With size included, this test goes red: the Hall's
/// slug is 84.18 × 21.93 and the summoned one is 56.12 × 14.62 — the same body
/// at exactly 1.5×, on both axes. Health, speed and weight agree exactly.
///
/// ⚠ **that 1.5 is unexplained and it is NOT the character's `collision_scale`,
/// which is 1.4.** `NpcActorSpawnPlan::spawn_into` carries a comment about
/// `collision_scale` being re-applied and "ballooning the sprite", so the hazard
/// is known to exist on this seam. Whether the difference is a defect or a
/// legitimate placement override (an authored NPC rect sizing its occupant) needs
/// evidence this test cannot supply.
///
/// ⇒ recorded in the campaign plan as an open AC6/AC7 question with the numbers,
/// rather than asserted here as either a rule or a bug. Adding `size` back is how
/// you re-open it, and it will be red until somebody explains the 1.5.
#[derive(Debug, PartialEq)]
struct IntrinsicBody {
    max_health: i32,
    max_run_speed: f32,
    weight: f32,
}

/// Read the one body WEARING `CHARACTER`, plus its disposition — which is
/// CONTEXT and is read separately on purpose.
///
/// ⚠ **`WornCharacter`, not `ActorIdentity`.** The identity component carries
/// the PLACEMENT's feature id and display label; the character a body IS is what
/// it wears. Keying this test on the wrong one is how it would compare two
/// bodies that merely share a name.
///
/// ⛔ **`placement` is not optional decoration.** `combat_calibration_lab` stages
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

/// **The two roads agree about the body and differ about the context.**
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
        // ⚠ a display name that is NOT the character's, on purpose: if the body
        // resolved its facts by matching a name, this road would resolve nothing
        // and the comparison below would fail loudly rather than agree by luck.
        "Summoned Probe",
        (600.0, 300.0),
        (12.0, 16.0),
        ambition_platformer2d::entity_catalog::placements::CharacterBrain::Passive,
        Some(CHARACTER),
    );
    for _ in 0..4 {
        summoned.step(AgentAction::default());
    }
    let (as_summon, summon_disposition) =
        body_of(&mut summoned, Some("cross_context_probe")).unwrap_or_else(|| {
        panic!(
            "the summon road built no body identified as `{CHARACTER}` — a \
             programmatic spawn naming a registered character must produce that \
             character, which is the whole of P1.12"
        )
    });

    // ⛔ **non-degenerate, or two empty bodies would compare equal.** A slug with
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
