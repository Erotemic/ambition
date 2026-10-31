//! The tactical intent: one objective a fighter holds across decisions, which
//! CONSTRAINS the option lists the scorer chooses from. It never chooses an
//! action itself: legality, ranking, noise and emission stay the reactive
//! brain's. Recovery, edge-guarding and reeling clear it; the reactive brain
//! answers those alone.
//!
//! Two kinds, and only two until the experiment's gate passes
//! (`docs/planning/engine/fighter-tactical-intent.md`):
//!
//! * [`IntentKind::Space`] — stand in a band just outside the foe's threat
//!   (`PerceivedActor::threat_reach`): close from beyond it, back out from
//!   inside it, hold within it, and swing only at a foe already in reach.
//! * [`IntentKind::Punish`] — the foe is committed (a whiffed swing, a
//!   landing) and the window, measured from now, admits an attack this body
//!   can land in time: move toward it, and swing only with what lands inside
//!   the window.
//!
//! The profile's `tactics` flag turns it on; every shipped rung is off.

use super::options::{foe_commitment, side_gap, MovementVerb, OptionSet};
use super::situation::{is_landing, Situation};
use crate::brain::attack_kit::AttackCandidate;
use crate::perception::{BodyPhase, Perceived, PerceivedActor};

/// How far outside the foe's threat the band begins, px of gap.
pub const SPACE_MARGIN_PX: f32 = 10.0;
/// How wide the band is, px of gap.
pub const SPACE_BAND_PX: f32 = 30.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IntentKind {
    Space,
    Punish,
}

impl IntentKind {
    /// The rollback wire code. Here, beside the kinds, so the brain codec
    /// names no fighter type.
    pub fn code(self) -> u8 {
        match self {
            Self::Space => 0,
            Self::Punish => 1,
        }
    }
}

/// The objective held across decisions.
#[derive(Clone, Debug, PartialEq)]
pub struct Intent {
    pub kind: IntentKind,
    /// The foe it is held against (`PerceivedActor::id`).
    pub target: String,
    /// Ticks the intent holds before it is asked again from scratch (a
    /// punish's window); `0` for one that is re-asked every decision.
    pub ticks_left: u32,
}

/// The seconds `attack` needs to land on a body `gap` px past this body's
/// front: the run to its reach, then its startup.
fn time_to_land(gap: f32, reach: f32, startup_s: f32, run_speed: f32) -> f32 {
    (gap - reach).max(0.0) / run_speed.max(1.0) + startup_s
}

/// Is `foe` open to a punish: a swing that missed, or a landing?
fn opened(view: &Perceived<'_>, foe: &PerceivedActor) -> bool {
    foe.phase == BodyPhase::AttackRecovery || is_landing(foe, view.self_view.gravity_down)
}

/// The intent for this decision.
///
/// `previous` keeps a punish going while its window is open, so a decision
/// that sees the foe a tick nearer the end of its recovery does not drop an
/// approach already under way. `interval_ticks` is the decision cadence.
///
/// ⛔ A PUNISH IS ARMED FROM THE KIT, NOT FROM THE ADMITTED ATTACKS. The
/// ranking admits only swings that meet the foe where it will be, so from a
/// spacing band — outside the foe's threat, and so mostly outside this
/// body's reach — the admitted list is empty, and a punish that asked it
/// could never arm (MEASURED: 0 of 28 whiffs at rung 9). The kit says what
/// could land after a run; the ranking, once there, says what does.
pub fn select(
    previous: Option<&Intent>,
    view: Perceived<'_>,
    situation: Situation,
    kit: &[AttackCandidate],
    run_speed: f32,
    tick_hz: f32,
    interval_ticks: u32,
) -> Option<Intent> {
    if !matches!(situation, Situation::Neutral | Situation::Advantage) {
        return None;
    }
    let foe = view.nearest_hostile()?;
    let commitment = foe_commitment(&view, foe);
    let gap = side_gap(&view);
    if commitment > 0.0 && opened(&view, foe) {
        let lands = kit
            .iter()
            .filter(|c| c.frames.reach > 0.0)
            .any(|c| time_to_land(gap, c.frames.reach, c.frames.startup_s, run_speed) <= commitment);
        let continuing = previous.is_some_and(|p| {
            p.kind == IntentKind::Punish && p.target == foe.id && p.ticks_left > interval_ticks
        });
        if lands || continuing {
            return Some(Intent {
                kind: IntentKind::Punish,
                target: foe.id.clone(),
                ticks_left: match previous {
                    Some(p) if continuing && !lands => p.ticks_left - interval_ticks,
                    _ => (commitment * tick_hz).ceil() as u32,
                },
            });
        }
    }
    foe.threat_reach.map(|_| Intent {
        kind: IntentKind::Space,
        target: foe.id.clone(),
        ticks_left: 0,
    })
}

/// Movement that carries this body toward the foe.
fn toward(verb: MovementVerb) -> bool {
    matches!(verb, MovementVerb::Approach | MovementVerb::Dash | MovementVerb::ShortHop)
}

/// Narrow `options` to what `intent` allows. The scorer then chooses among
/// what is left, as it always does; an empty movement list stands still.
pub fn constrain(intent: &Intent, view: Perceived<'_>, options: &mut OptionSet, run_speed: f32) {
    let Some(foe) = view.hostiles().find(|actor| actor.id == intent.target) else {
        return;
    };
    let gap = side_gap(&view);
    match intent.kind {
        IntentKind::Space => {
            let Some(threat) = foe.threat_reach else {
                return;
            };
            let low = threat + SPACE_MARGIN_PX;
            let high = low + SPACE_BAND_PX;
            options.movement.retain(|option| {
                if gap < low {
                    !toward(option.verb)
                } else if gap <= high {
                    !toward(option.verb) && option.verb != MovementVerb::Retreat
                } else {
                    option.verb != MovementVerb::Retreat
                }
            });
            // No swing on a lead: only what meets the foe where it stands.
            options.attacks.retain(|a| gap <= a.frames.reach);
        }
        IntentKind::Punish => {
            let window = foe_commitment(&view, foe);
            options
                .attacks
                .retain(|a| time_to_land(gap, a.frames.reach, a.frames.startup_s, run_speed) <= window);
            // Out of reach of every swing that lands in time, the window is
            // spent closing: toward the foe, on the ground, and nothing else.
            let in_reach = options.attacks.iter().any(|a| gap <= a.frames.reach);
            if !in_reach {
                options.movement.retain(|option| option.verb == MovementVerb::Approach);
                if options.movement.is_empty() {
                    options.movement.push(super::options::MoveOption {
                        verb: MovementVerb::Approach,
                        score: 0.0,
                    });
                }
            } else {
                options
                    .movement
                    .retain(|option| !matches!(option.verb, MovementVerb::Retreat | MovementVerb::Shield));
            }
        }
    }
}

#[cfg(test)]
mod tests;
