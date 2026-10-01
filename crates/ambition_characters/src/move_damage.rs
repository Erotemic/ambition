//! The damage a character's moves deal in a damage scale other than its
//! moveset's own.
//!
//! ⭐ A MOVE HAS ONE SHAPE AND ONE DAMAGE PER SCALE. The moveset states the
//! frames, the geometry, and the damage of the character's HOME game. A game
//! whose damage means something else (percent, not health) states its own
//! numbers under its own [`DamageScale`], so each game balances its own
//! damage and neither rewrites the other.
//!
//! Generic on purpose: the capability that authors a scale folds it into the
//! definition under its own name, and a match names the scale it plays in.
//! The match, the seat and the kit resolver read only the scale, so a second
//! game's damage needs no new variant here or in them (queue AP78).

/// Damage per move id, one value for each damaging volume, in authoring order
/// (window by window). A move the map does not name keeps its moveset damage.
pub type MoveDamage = std::collections::BTreeMap<String, Vec<i32>>;

/// The name of a damage scale. The capability that authors one owns its
/// constant (`smash_fighter::FIGHTER_DAMAGE`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize)]
pub struct DamageScale(pub &'static str);

/// A character's damage per scale. Empty: every move keeps its moveset damage
/// in every scale.
pub type ScaledMoveDamage = std::collections::BTreeMap<DamageScale, MoveDamage>;

/// `moveset` with each named move's damaging volumes set to `damage`.
///
/// `Err` names every move id the moveset does not have and every move whose
/// list does not have one value for each of its damaging volumes. On `Err`
/// nothing is applied, so the body never deals half one game's damage and half
/// the other's.
pub fn move_damage_over(
    damage: &MoveDamage,
    mut moveset: ambition_entity_catalog::MovesetContract,
) -> Result<ambition_entity_catalog::MovesetContract, Vec<String>> {
    let mut problems = Vec::new();
    for (move_id, values) in damage {
        let Some(spec) = moveset.moves.iter_mut().find(|spec| &spec.id == move_id) else {
            problems.push(format!(
                "`move_damage` names move `{move_id}`, which the moveset does not have"
            ));
            continue;
        };
        let mut volumes: Vec<&mut i32> = spec
            .windows
            .iter_mut()
            .flat_map(|window| window.volumes.iter_mut())
            .map(|volume| &mut volume.damage)
            .filter(|damage| **damage > 0)
            .collect();
        if volumes.len() != values.len() {
            problems.push(format!(
                "`move_damage` gives move `{move_id}` {} value(s), and the move has {} \
                 volume(s) that deal damage",
                values.len(),
                volumes.len()
            ));
            continue;
        }
        for (slot, value) in volumes.iter_mut().zip(values) {
            **slot = *value;
        }
    }
    if problems.is_empty() {
        Ok(moveset)
    } else {
        Err(problems)
    }
}
