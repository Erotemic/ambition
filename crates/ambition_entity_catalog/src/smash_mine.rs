//! Authored payload for a remotely triggered mine.
//! The ruleset represents the mine as a ground item. Normal item custody therefore applies while the placer keeps detonation authority.

use serde::{Deserialize, Serialize};

use crate::{EffectRef, MoveEvent, MoveEventKind, MoveSpec, ParamValue};

/// The authored effect key. Namespaced like every other smash technique so an
/// unrecognised key falls through other rulesets untouched.
pub const PLACE_MINE: &str = "smash.place_mine";

/// Authored parameters for one placed mine. The mine has no fuse; its owner triggers it after the arming delay.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlaceMineParams {
    /// Held-item id used when the mine enters item custody. It must name a registered held item.
    pub item_id: String,
    /// Seconds before the mine accepts remote detonation.
    pub arm_s: f32,
    /// Damage at the centre of the blast.
    pub damage: i32,
    /// How far the blast reaches, in world px.
    pub blast_radius: f32,
    /// The object's own size in the world.
    pub half_extents: (f32, f32),
    /// Where it appears, body-local (`+x` toward facing, `+y` gravity-down).
    pub offset: (f32, f32),
    /// How hard the blast throws what it catches: the feel multiplier
    /// `DamageBoxEffect::knockback` is, not a launch speed.
    pub knockback: f32,
    /// How long the blast stays live, in seconds: long enough to meet each
    /// body it covers, short enough that a body that walks in later is not
    /// caught by it.
    pub blast_s: f32,
}

/// The domain rule for a mine's params, not only "serde can build it".
pub fn check_place_mine_params(params: &crate::ParamValue) -> Result<(), String> {
    let typed: PlaceMineParams = params.hydrate().map_err(|error| error.to_string())?;
    let mut problems = Vec::new();
    if !(typed.knockback.is_finite() && typed.knockback > 0.0) {
        problems.push(format!("knockback {} throws nobody: a blast throws with a finite, positive feel multiplier", typed.knockback));
    }
    if !(typed.arm_s.is_finite() && typed.arm_s >= 0.0) {
        problems.push(format!("arm_s {} is not a delay: it is finite and not negative", typed.arm_s));
    }
    problems.extend(blast_problem(typed.blast_s));
    if problems.is_empty() { Ok(()) } else { Err(problems.join("; ")) }
}

/// The rule for a blast's live time (`blast_s` of a mine, a bomb or a mark):
/// a blast that is never live hurts nobody.
pub fn blast_problem(blast_s: f32) -> Option<String> {
    (!(blast_s.is_finite() && blast_s > 0.0)).then(|| format!("blast_s {blast_s} is never live: a blast lasts a finite, positive time"))
}

/// Return the held-item id referenced by a mine payload.
/// Malformed params return no references because parameter hydration is validated separately.
pub fn mine_held_item_refs(effect: &crate::EffectRef) -> Vec<String> {
    effect
        .params
        .hydrate::<PlaceMineParams>()
        .map(|params| vec![params.item_id])
        .unwrap_or_default()
}

/// Author the mine technique on a move timeline. The ruleset decides whether the event places a mine or detonates the owner's armed mine.
///
/// # Panics
///
/// Panics if `at_s` is after the move duration.
pub fn author_place_mine(mut spec: MoveSpec, at_s: f32, params: PlaceMineParams) -> MoveSpec {
    assert!(
        at_s <= spec.duration_s,
        "move `{}` places its mine at {at_s}s but only lasts {}s, so the mine \
         would never appear and the move would spend a recovery to do nothing",
        spec.id,
        spec.duration_s,
    );
    spec.events.push(MoveEvent {
        at_s,
        kind: MoveEventKind::Effect(EffectRef {
            key: PLACE_MINE.to_string(),
            params: ParamValue::from_typed(&params).expect("place-mine params serialize"),
        }),
    });
    spec
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::smash_bomb::{check_drop_bomb_params, DropBombParams};
    use crate::smash_mark::{check_mark_body_params, MarkBodyParams};

    fn mine(blast_s: f32) -> ParamValue {
        ParamValue::from_typed(&PlaceMineParams {
            item_id: "polygon_mine".into(),
            arm_s: 1.2,
            damage: 10,
            blast_radius: 52.0,
            half_extents: (8.0, 8.0),
            offset: (0.0, 0.0),
            knockback: 1.7,
            blast_s,
        })
        .unwrap()
    }

    fn bomb(blast_s: f32) -> ParamValue {
        ParamValue::from_typed(&DropBombParams {
            item_id: "polygon_bomb".into(),
            fuse_s: 4.0,
            damage: 12,
            blast_radius: 56.0,
            impact_speed: 260.0,
            half_extents: (9.0, 9.0),
            offset: (0.0, 0.0),
            knockback: 2.0,
            blast_s,
        })
        .unwrap()
    }

    fn mark(blast_s: f32) -> ParamValue {
        ParamValue::from_typed(&MarkBodyParams { fuse_s: 1.4, damage: 6, blast_radius: 44.0, knockback: 1.2, blast_s }).unwrap()
    }

    /// A blast that is never live is refused by its field, for each of the
    /// three techniques that set one off. The control first: a live blast of
    /// each is admitted, so the refusals are not a check that refuses all.
    #[test]
    fn a_blast_that_is_never_live_is_refused() {
        let checks: [(&str, fn(f32) -> ParamValue, crate::ParamCheck); 3] =
            [("mine", mine, check_place_mine_params), ("bomb", bomb, check_drop_bomb_params), ("mark", mark, check_mark_body_params)];
        for (name, params, check) in checks {
            check(&params(0.08)).unwrap_or_else(|error| panic!("control: a live {name} blast is refused: {error}"));
            for bad in [0.0, -0.08, f32::NAN] {
                let said = check(&params(bad)).expect_err("a blast that is never live was admitted");
                assert!(said.contains("blast_s"), "a {name} blast live for {bad} s was not refused by name: {said}");
            }
        }
    }
}
