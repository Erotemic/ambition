//! Answer a parry with the blade: the authored vocabulary.
//!
//! ⭐⭐ THE COUNTER'S MOST CONVENTIONAL ANSWER WAS THE ONE IT COULD NOT GIVE.
//! Six counters ship — the stand-ins' `riposte`, the Author's second draft, the
//! ninja's seal, Emmy's field, the officer's shield, the clerk's clocks — and
//! their responses are a grab, a teleport, a sleep, a heal, an absorb and a
//! slow. Not one of them HITS BACK, because `counter_move` builds a stance with
//! no volumes and the answer is entirely its response technique. ⇒ The genre's
//! plainest counter (parry, then cut) was inexpressible, and the fighter who
//! most needed it is the sword archetype whose whole table is fundamentals.
//!
//! ⛔ IT OWNS NO DAMAGE. The cut is an ordinary BODY STRIKE — `HitSide::Player`
//! anchored `FollowOwner`, spawned through `strike::spawn_body_strike` and
//! resolved by the one hitbox authority every swing in the workspace goes
//! through. This module decides how hard, how far and how wide, and nothing else.
//!
//! ⛔⛔ IT IS DELIBERATELY *NOT* A `DamageBoxEffect`, and the reason is the kind
//! of thing that only shows up when you read the citation. A `DamageBox` is
//! anchored in the WORLD, and the resolver's table reads
//! `(HitSide::Player, HitboxAnchor::World { .. }) => None` — a player-sided
//! world box damages nobody. The one side that reaches bodies from a world
//! anchor is `Environment`, which by design consults **no self-exclusion**
//! (*"your own bomb hurts you, and you still placed it"*). ⇒ Both spellings
//! available through the effect road are wrong for a counter: one hurts no one,
//! the other cuts the fighter who parried.
//!
//! ⚠ AND IT IS NOT LIMITED TO COUNTERS. A response is delivered as an ordinary
//! `ActorActionMessage`, so any move that names this key gets a cut in front of
//! it. It lives here rather than in `smash_counter` for that reason.
//!
//! The ruleset half is `ambition_demo_smash::riposte`.

use serde::{Deserialize, Serialize};

use ambition_entity_catalog::{EffectRef, MoveEvent, MoveEventKind, MoveSpec, ParamValue};

/// The authored effect key.
pub const RIPOSTE_STRIKE: &str = "smash.riposte_strike";

/// Authored parameters of one answering cut.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RiposteStrikeParams {
    /// Percent dealt by the cut.
    pub damage: u32,
    /// ⛔⛔ A FEEL MULTIPLIER, NOT A LAUNCH SPEED, and this field has already
    /// cost this repository three shipped moves. `DamageBoxEffect::knockback`
    /// becomes `HitboxKnockback::FeelScale`, whose authored band is roughly
    /// 1.1–1.6; `ambition_combat::strike::MAX_PLAUSIBLE_FEEL_SCALE` is 8.0 and
    /// warns above it. A value like `104.0` copied off a `Strike`'s knockback
    /// is not a stronger hit, it is a nonsense one.
    pub knockback: f32,
    /// How far in front of the fighter the cut lands, in world px.
    ///
    /// ⭐ THE SWORD ARCHETYPE'S WHOLE DISTINCTION IS REACH, so this is the
    /// number that makes one riposte different from another's.
    pub reach: f32,
    /// Half-extents of the cut, in world px.
    pub half_extents: (f32, f32),
    /// How long the cut stays live, in seconds.
    ///
    /// ⚠ SHORT. This is a hitbox that appears in one place rather than sweeping,
    /// so a long life is a lingering trap rather than a swing.
    pub lifetime_s: f32,
}

impl RiposteStrikeParams {
    /// Everything wrong with these params, as sentences an author can act on.
    ///
    /// ⭐ A LIST RATHER THAN A PANIC, matching `TimeDilationParams::problems`:
    /// a response's params are authored inside a `CounterParams`, where there is
    /// no constructor to assert in. The ruleset checks them when it hydrates and
    /// names the move, which is the only place both facts are in hand.
    pub fn problems(&self) -> Vec<String> {
        let mut problems = Vec::new();
        if self.damage == 0 {
            problems.push(
                "deals 0 damage, so the parry is answered by an invisible box \
                 that costs the attacker nothing"
                    .to_string(),
            );
        }
        if !(self.knockback > 0.0) {
            problems.push(format!(
                "authors {} knockback: the field is a FEEL MULTIPLIER (1.1–1.6), \
                 and zero or less is not a weaker hit but an absent one",
                self.knockback,
            ));
        }
        if self.knockback > 8.0 {
            problems.push(format!(
                "authors {} knockback, above `MAX_PLAUSIBLE_FEEL_SCALE` (8.0) — \
                 this reads like a launch SPEED copied off a `Strike`, which is \
                 a different unit and the mistake three shipped moves made",
                self.knockback,
            ));
        }
        if self.reach <= 0.0 {
            problems.push(format!(
                "cuts {}px in front of itself, so the answer lands inside the \
                 fighter who threw it",
                self.reach,
            ));
        }
        if self.half_extents.0 <= 0.0 || self.half_extents.1 <= 0.0 {
            problems.push(format!(
                "authors a {:?} cut, which has no area and can never overlap \
                 anybody",
                self.half_extents,
            ));
        }
        if self.lifetime_s <= 0.0 {
            problems.push(format!(
                "lives {}s, so it is spawned and gone before a frame resolves",
                self.lifetime_s,
            ));
        }
        problems
    }
}

/// Author a cut onto a move's own timeline.
///
/// ⭐⭐ THE COUNTER IS NOT THE ONLY CUSTOMER, and this helper is what makes that
/// true in practice rather than in a comment. A counter reaches this technique
/// by naming it as its `response`; a MOVE reaches it by putting the key on its
/// timeline, and both arrive as the same `ActorActionMessage`. ⇒ Anything that
/// wants a second, differently-shaped hit at a chosen instant — a slam whose
/// shock runs along the ground, a swing with a late tip — can have one without a
/// new technique and without hand-building windows.
///
/// ⛔ WHY NOT `multihit`: its pulses are a LEAD-IN. Its own implementation shifts
/// the finisher back by the pulse train's length, because a multi-hit is a
/// wind-up into a finisher. A follow-up is the other direction and it cannot be
/// spelled that way.
///
/// # Panics
///
/// If `at_s` is past the move's duration, or if the cut is unusable — the same
/// list [`RiposteStrikeParams::problems`] gives the ruleset at runtime, asked
/// here where the MOVE'S NAME is in hand and the failure is a build error rather
/// than a log line nobody reads.
pub fn author_cut(mut spec: MoveSpec, at_s: f32, params: RiposteStrikeParams) -> MoveSpec {
    assert!(
        at_s <= spec.duration_s,
        "move `{}` cuts at {at_s}s but only lasts {}s",
        spec.id,
        spec.duration_s,
    );
    let problems = params.problems();
    assert!(
        problems.is_empty(),
        "move `{}` authors an unusable cut: {}",
        spec.id,
        problems.join("; "),
    );
    spec.events.push(MoveEvent {
        at_s,
        kind: MoveEventKind::Effect(EffectRef {
            key: RIPOSTE_STRIKE.to_string(),
            params: ParamValue::from_typed(&params).expect("riposte-strike params serialize"),
        }),
    });
    spec
}

