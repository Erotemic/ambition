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
/// constant (`smash_fighter::FIGHTER_DAMAGE`). A scale is a game's reading of
/// a character's one move table: its damage ([`MoveDamage`]) and its verbs
/// ([`VerbMoves`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize)]
pub struct DamageScale(pub &'static str);

/// A character's damage per scale. Empty: every move keeps its moveset damage
/// in every scale.
pub type ScaledMoveDamage = std::collections::BTreeMap<DamageScale, MoveDamage>;

/// The move each verb names in a scale other than the moveset's own: a verb,
/// and the id of a move of the same moveset. A verb the map does not name
/// keeps the move its moveset gives it.
///
/// A character's moves are one table for every game. Which of them a button
/// plays can differ: at home the robot's attack is a walking swing, and on a
/// platform-fighter stage it is a jab that roots. Each game states its own
/// verbs, as it states its own damage, and the frames and the geometry of
/// each move stay in one place.
pub type VerbMoves = std::collections::BTreeMap<String, String>;

/// A character's verbs per scale. Empty: every verb names its moveset's own
/// move in every scale.
pub type ScaledVerbMoves = std::collections::BTreeMap<DamageScale, VerbMoves>;

/// `moveset` with each verb of `verbs` naming that move.
///
/// `Err` names every move id the moveset does not have. On `Err` nothing is
/// applied: the body keeps one game's verbs, and not a part of each.
pub fn verb_moves_over(
    verbs: &VerbMoves,
    mut moveset: ambition_entity_catalog::MovesetContract,
) -> Result<ambition_entity_catalog::MovesetContract, Vec<String>> {
    let problems: Vec<String> = verbs
        .iter()
        .filter(|(_, move_id)| moveset.move_by_id(move_id).is_none())
        .map(|(verb, move_id)| format!("`verbs.{verb}` names move `{move_id}`, which the moveset does not have"))
        .collect();
    if !problems.is_empty() {
        return Err(problems);
    }
    for (verb, move_id) in verbs {
        moveset.verbs.insert(verb.clone(), move_id.clone());
    }
    Ok(moveset)
}

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

/// A MOVE'S CLOCK on a platform-fighter stage: when its first strike comes
/// out, when its last strike ends, when it is over, and what landing in it
/// costs. Seconds, as every time in a moveset is. A game whose moves must
/// commit (a fighter's aerial of forty frames with landing lag) states these
/// over a moveset authored for another game's pace (the same character's
/// home), the way `MoveDamage` states that game's percent over its health.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MoveTiming {
    /// When the first `Active` window opens.
    pub hit_s: f32,
    /// When the last `Active` window closes.
    pub last_s: f32,
    /// The move's duration.
    pub total_s: f32,
    /// Landing lag, for an aerial; `None` keeps the move's own.
    #[serde(default)]
    pub landing_lag_s: Option<f32>,
    /// The whoosh the move starts with (an `Sfx` event early in the startup),
    /// for a move that authors no sound before its first strike: a fighter's
    /// swing is heard before it lands. `None` adds none.
    #[serde(default)]
    pub swing: Option<String>,
}

impl MoveTiming {
    pub fn problems(&self, move_id: &str, out: &mut Vec<String>) {
        let ordered = 0.0 < self.hit_s && self.hit_s <= self.last_s && self.last_s <= self.total_s;
        if !(self.hit_s.is_finite() && self.last_s.is_finite() && self.total_s.is_finite() && ordered) {
            out.push(format!(
                "`move_timing.{move_id}` is hit {} <= last {} <= total {}; it needs 0 < hit <= last <= total",
                self.hit_s, self.last_s, self.total_s
            ));
        }
        if let Some(lag) = self.landing_lag_s {
            if !(lag.is_finite() && lag >= 0.0) {
                out.push(format!("`move_timing.{move_id}.landing_lag_s` is {lag}; it is finite and not negative"));
            }
        }
    }
}

/// Where in the startup the derived swing sounds: a third of the way to the
/// first strike, so the whoosh leads the contact and does not sit on it.
const SWING_AT_HIT_SHARE: f32 = 0.3;

/// Move id -> its clock in one scale.
pub type MoveTimings = std::collections::BTreeMap<String, MoveTiming>;

/// Each scale's move clocks.
pub type ScaledMoveTimings = std::collections::BTreeMap<DamageScale, MoveTimings>;

/// Retime each named move to its `MoveTiming`.
///
/// A piecewise-linear map of the move's own clock: `[0, first active]` onto
/// `[0, hit]`, the active span onto `[hit, last]`, and the rest onto
/// `[last, total]`. Every window edge, event time and autocancel goes through
/// the one map, so hit volumes, events, upper-body and cancel windows keep
/// their order and their phase of the swing; the clip follows the move's
/// phase, so the art stretches with it. Damage, volumes and launches are
/// untouched. A move with no `Active` window is refused: it has no swing to
/// map the clock around.
pub fn move_timing_over(
    timings: &MoveTimings,
    mut moveset: ambition_entity_catalog::MovesetContract,
) -> Result<ambition_entity_catalog::MovesetContract, Vec<String>> {
    use ambition_entity_catalog::WindowTag;
    let mut problems = Vec::new();
    for (move_id, timing) in timings {
        let Some(spec) = moveset.moves.iter_mut().find(|spec| &spec.id == move_id) else {
            problems.push(format!("`move_timing` names move `{move_id}`, which the moveset does not have"));
            continue;
        };
        let active: Vec<(f32, f32)> = spec
            .windows
            .iter()
            .filter(|window| matches!(window.tag, WindowTag::Active))
            .map(|window| (window.start_s, window.end_s))
            .collect();
        let (Some(first), Some(last)) = (active.first(), active.last()) else {
            problems.push(format!("`move_timing` names move `{move_id}`, which has no Active window to time"));
            continue;
        };
        let (a0, a1, duration) = (first.0, last.1, spec.duration_s);
        let map = |t: f32| -> f32 {
            let mapped = if t <= a0 {
                if a0 > 0.0 { timing.hit_s * (t / a0) } else { 0.0 }
            } else if t <= a1 {
                let share = if a1 > a0 { (t - a0) / (a1 - a0) } else { 1.0 };
                timing.hit_s + (timing.last_s - timing.hit_s) * share
            } else {
                let tail = duration - a1;
                let share = if tail > 0.0 { (t - a1) / tail } else { 1.0 };
                timing.last_s + (timing.total_s - timing.last_s) * share
            };
            mapped.min(timing.total_s)
        };
        for window in &mut spec.windows {
            window.start_s = map(window.start_s);
            window.end_s = map(window.end_s);
        }
        for event in &mut spec.events {
            event.at_s = map(event.at_s);
        }
        if let Some(repeat) = &mut spec.repeat {
            repeat.from_s = map(repeat.from_s);
            repeat.to_s = map(repeat.to_s);
        }
        spec.autocancel_after_s = spec.autocancel_after_s.map(map);
        spec.duration_s = timing.total_s;
        if let Some(cue) = &timing.swing {
            let authored = spec.events.iter().any(|event| {
                matches!(event.kind, ambition_entity_catalog::MoveEventKind::Sfx { .. }) && event.at_s <= timing.hit_s
            });
            if !authored {
                spec.events.push(ambition_entity_catalog::MoveEvent {
                    at_s: timing.hit_s * SWING_AT_HIT_SHARE,
                    kind: ambition_entity_catalog::MoveEventKind::Sfx { cue: cue.clone() },
                });
            }
        }
        if let Some(lag) = timing.landing_lag_s {
            spec.landing_lag_s = Some(lag);
            // Landing in the last two fifths of the move is clean: the swing
            // is over. Only an aerial with landing lag can be autocancelled.
            spec.autocancel_after_s = Some(timing.last_s + 0.6 * (timing.total_s - timing.last_s));
        }
    }
    if problems.is_empty() {
        Ok(moveset)
    } else {
        Err(problems)
    }
}

/// Move id -> the factor its base knockback is scaled by in one scale. A
/// platform fighter's tilt or aerial must launch hard enough at 0% that the
/// stun outlasts the swing (the follow-up); a move authored for a health game
/// often does not. `1.0` is the move's own.
pub type MoveKnockback = std::collections::BTreeMap<String, f32>;

/// Each scale's move knockback.
pub type ScaledMoveKnockback = std::collections::BTreeMap<DamageScale, MoveKnockback>;

/// Scale the base knockback of each named move's striking volumes. The growth
/// (`knockback_growth`) is untouched: this moves the floor of the launch, not
/// how it grows with percent, so KO percents move little and the low-percent
/// stun moves much.
pub fn move_knockback_over(
    knockback: &MoveKnockback,
    mut moveset: ambition_entity_catalog::MovesetContract,
) -> Result<ambition_entity_catalog::MovesetContract, Vec<String>> {
    let mut problems = Vec::new();
    for (move_id, scale) in knockback {
        let Some(spec) = moveset.moves.iter_mut().find(|spec| &spec.id == move_id) else {
            problems.push(format!("`move_knockback` names move `{move_id}`, which the moveset does not have"));
            continue;
        };
        if !(scale.is_finite() && *scale > 0.0) {
            problems.push(format!("`move_knockback.{move_id}` is {scale}; it is a positive factor"));
            continue;
        }
        for volume in spec.windows.iter_mut().flat_map(|window| window.volumes.iter_mut()) {
            if volume.damage > 0 {
                volume.knockback *= scale;
            }
        }
    }
    if problems.is_empty() {
        Ok(moveset)
    } else {
        Err(problems)
    }
}
