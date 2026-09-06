//! Summon-a-mount-and-ride: the authored vocabulary.
//!
//! ⭐ THE SAME SPLIT `smash_capture` USES, and for the same reason. A key and its
//! params are what a MOVESET authors, so they live where movesets can name them;
//! recognising the key and doing something about it is a RULESET's job, and that
//! half is `ambition_demo_smash::shark_ride`. A game that never installs the
//! smash rules can still author a move carrying this key — it simply does
//! nothing, which is what an unrecognised technique should do.

use serde::{Deserialize, Serialize};

use ambition_entity_catalog::{EffectRef, MoveEvent, MoveEventKind, MoveSpec, ParamValue};

/// The authored effect key. Namespaced like every other smash technique so an
/// unrecognised key falls through other rulesets untouched.
pub const SUMMON_RIDE: &str = "smash.summon_ride";

/// The mount class the pirate is licensed for. ADR 0020's compatibility check
/// reads this off the summoned body's `Mountable`.
pub const SHARK_CLASS: &str = "shark";

/// Authored parameters of one summon-and-ride.
///
/// ⛔ THE CHARACTER ID IS AUTHORED, not hardcoded here. The mechanic is "summon
/// a mount and get on it"; that this pirate's mount is a burning flying shark is
/// the pirate's statement, and a second character wanting a different vehicle
/// authors a different id rather than editing this module.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SummonRideParams {
    /// Which character to summon as the mount.
    pub character_id: String,
    /// The mount body's half-extents.
    pub half_extents: (f32, f32),
    /// How long the rider may stay aboard, in seconds of sim time.
    pub seconds: f32,
    /// HOW FAR THIS RIDE CLAIMS TO CARRY ITS RIDER, in world px.
    ///
    /// ⭐⭐ THE PLANNER'S HALF OF THE SAME STATEMENT, and it is authored beside
    /// the length because the two are one decision: a five-second ride on a
    /// 260px/s shark is a different recovery from a five-second ride on a slow
    /// one. Nothing at runtime reads it — the ride is steered by a player — and
    /// that is the point: it is what the move PROMISES, which is exactly what a
    /// search can spend. See `AuthoredRecoveryRoute::SustainedAuthority`.
    ///
    /// ⛔ UNDER-CLAIM IT. A recovery search that says "no route" costs a missed
    /// option; one that says "you'll make it" and is wrong costs the stock.
    pub reach: f32,
}

/// Author a summon-and-ride onto a move's timeline.
///
/// # Panics
///
/// If `at_s` is past the move's own duration. A summon scheduled after the move
/// ends never fires, and the move would cost its recovery to do nothing —
/// exactly the failure the helper exists to catch at authoring time.
pub fn author_summon_ride(mut spec: MoveSpec, at_s: f32, params: SummonRideParams) -> MoveSpec {
    assert!(
        at_s <= spec.duration_s,
        "move `{}` summons its mount at {at_s}s but only lasts {}s, so the \
         summon would never fire and the move would spend a recovery to do nothing",
        spec.id,
        spec.duration_s,
    );
    spec.events.push(MoveEvent {
        at_s,
        kind: MoveEventKind::Effect(EffectRef {
            key: SUMMON_RIDE.to_string(),
            params: ParamValue::from_typed(&params).expect("summon-ride params serialize"),
        }),
    });
    // ⛔ THE COST IS THE SLOT'S TO STATE, and it used to be set here as two
    // booleans that had to agree. A vehicle that could be summoned forever is
    // not a recovery, it is flight — and a rider that cannot act is not riding —
    // so this move is `RecoveryUse::SpendWithoutFreefall`, which is now ONE
    // value that cannot half-disagree with itself. The pirate says it where
    // every other fighter says its up-B's cost: `UpSpecial::NoFreefall`.
    // ⛔⛔ REFUSED FROM THE SADDLE, AT ACCEPTANCE. Jon: *"No you cannot cast it
    // from the saddle."* This used to be enforced downstream, where the summon
    // effect was translated — so a mounted pirate who got flinched (which
    // refunds the recovery) could press up-B, start the move, spend the use,
    // play the startup, and get nothing.
    //
    // ⭐ NOT AN ARM OF `RecoveryUse`, and deliberately beside it: what a move
    // COSTS and whether it may BEGIN are different questions. See
    // `MoveGates::forbidden_while_held`.
    spec.gates.forbidden_while_held = true;
    // ⭐⭐ AND THE CPU CAN NOW SEE IT AS A WAY HOME. This move commands no
    // impulse, so `lift_speed` is `0.0` and the recovery planner — which modelled
    // every route as one thrown velocity — saw a fighter with no recovery at all
    // (D250). ⛔ Not by fabricating a lift: a burst the move does not throw would
    // make the search certify a rise that never happens. What it offers is
    // SECONDS OF MOVEMENT AUTHORITY, so that is what it says.
    spec.gates.recovery_route = Some(
        ambition_entity_catalog::AuthoredRecoveryRoute::SustainedAuthority {
            seconds: params.seconds,
            reach: params.reach,
        },
    );
    spec
}
