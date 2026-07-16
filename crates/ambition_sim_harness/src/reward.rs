//! Example reward-shaping functions for the headless RL sim (TODO #198).
//!
//! These are pure functions over a `(previous, current)` observation
//! transition — deterministic and dependency-free, so research code
//! (Python via a future PyO3 binding, or a Rust policy harness) can call
//! them directly or copy them as a template for a task-specific reward.
//! They intentionally express *separable* shaping terms (survival,
//! exploration, health) plus one composite example; a real objective
//! should tune the weights or swap in its own terms rather than treat
//! `default_shaped` as canonical.
//!
//! All terms read only fields already on [`AgentObservation`], so adding
//! a reward costs nothing at the sim layer.

use crate::AgentObservation;

/// Survival term: a small positive tick reward for staying alive, a large
/// penalty when a death/reset happened between the two observations
/// (`resets` incremented), and a one-shot penalty on the frame the player
/// *newly* takes damage (the `recently_damaged` rising edge).
pub fn survival(prev: &AgentObservation, cur: &AgentObservation) -> f32 {
    let mut r = 0.0;
    if cur.alive() {
        r += 0.01;
    }
    if cur.resets > prev.resets {
        r -= 1.0;
    }
    if cur.recently_damaged && !prev.recently_damaged {
        r -= 0.25;
    }
    r
}

/// Exploration term: a bonus for reaching a new room plus a small,
/// capped shaped reward for distance travelled this step. The cap keeps a
/// blink/teleport from spiking the reward (and keeps the term bounded for
/// stable learning).
pub fn exploration(prev: &AgentObservation, cur: &AgentObservation) -> f32 {
    let mut r = 0.0;
    if cur.active_room != prev.active_room {
        r += 1.0;
    }
    let dx = cur.player_pos.0 - prev.player_pos.0;
    let dy = cur.player_pos.1 - prev.player_pos.1;
    let dist = (dx * dx + dy * dy).sqrt();
    r += (dist * 0.001).min(0.05);
    r
}

/// Health-preservation term: a small standing reward proportional to the
/// current HP fraction, so the policy is nudged to avoid attrition even
/// without a discrete damage event this frame.
pub fn health_preservation(cur: &AgentObservation) -> f32 {
    cur.hp_fraction() * 0.02
}

/// Composite example reward combining [`survival`], [`exploration`], and
/// [`health_preservation`] with documented weights baked into each term.
/// A starting point for "stay alive, keep exploring, don't get hit" —
/// real tasks should tune or replace.
pub fn default_shaped(prev: &AgentObservation, cur: &AgentObservation) -> f32 {
    survival(prev, cur) + exploration(prev, cur) + health_preservation(cur)
}

#[cfg(test)]
mod tests;
