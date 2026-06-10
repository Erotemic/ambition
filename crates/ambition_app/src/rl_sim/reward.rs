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

use super::AgentObservation;

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
mod tests {
    use super::*;

    fn obs() -> AgentObservation {
        AgentObservation {
            tick: 0,
            player_pos: (0.0, 0.0),
            player_vel: (0.0, 0.0),
            player_size: (16.0, 32.0),
            on_ground: true,
            on_wall: false,
            wall_clinging: false,
            wall_climbing: false,
            facing: 1.0,
            fast_falling: false,
            fly_enabled: false,
            gliding: false,
            dash_charges: 1,
            air_jumps: 1,
            blink_aiming: false,
            hp: 10,
            hp_max: 10,
            mana: 0,
            mana_max: 0,
            time_alive: 0.0,
            resets: 0,
            body_mode: "Stand".into(),
            active_room: "room_a".into(),
            world_size: (1000.0, 1000.0),
            world_spawn: (0.0, 0.0),
            last_safe_pos: (0.0, 0.0),
            recently_damaged: false,
            in_hitstun: false,
            invincible: false,
            in_water: false,
            water_kind: None,
            water_submersion: 0.0,
            on_climbable: false,
            climbable_kind: None,
        }
    }

    #[test]
    fn surviving_is_positive_dying_is_a_large_penalty() {
        let prev = obs();
        let mut cur = obs();
        cur.tick = 1;
        assert!(
            survival(&prev, &cur) > 0.0,
            "alive should earn a tick reward"
        );

        // A reset between observations (death) dominates with a big penalty.
        cur.resets = 1;
        cur.hp = 0;
        assert!(
            survival(&prev, &cur) < -0.5,
            "a death/reset should be a large negative reward"
        );
    }

    #[test]
    fn newly_taking_damage_is_penalized_on_the_rising_edge_only() {
        let prev = obs();
        let mut cur = obs();
        cur.recently_damaged = true;
        let edge = survival(&prev, &cur);

        let mut prev_dmg = obs();
        prev_dmg.recently_damaged = true;
        let mut still_dmg = obs();
        still_dmg.recently_damaged = true;
        // Already-damaged → no fresh edge penalty, so the held state scores
        // higher than the rising edge.
        assert!(
            survival(&prev_dmg, &still_dmg) > edge,
            "the penalty should fire once on the damage edge, not every frame"
        );
    }

    #[test]
    fn reaching_a_new_room_rewards_exploration() {
        let prev = obs();
        let mut cur = obs();
        cur.active_room = "room_b".into();
        assert!(
            exploration(&prev, &cur) >= 1.0,
            "entering a new room should grant the room bonus"
        );
    }

    #[test]
    fn distance_term_is_capped_so_teleports_do_not_spike() {
        let prev = obs();
        let mut cur = obs();
        cur.player_pos = (100000.0, 0.0); // huge blink
                                          // Same room, so only the (capped) distance term applies.
        assert!(
            (exploration(&prev, &cur) - 0.05).abs() < 1e-6,
            "the distance term must saturate at its cap"
        );
    }

    #[test]
    fn health_term_scales_with_hp_fraction() {
        let mut full = obs();
        full.hp = 10;
        let mut half = obs();
        half.hp = 5;
        assert!(health_preservation(&full) > health_preservation(&half));
        assert_eq!(health_preservation(&full), 0.02);
    }

    #[test]
    fn composite_is_the_sum_of_its_terms() {
        let prev = obs();
        let mut cur = obs();
        cur.player_pos = (50.0, 0.0);
        let expected = survival(&prev, &cur) + exploration(&prev, &cur) + health_preservation(&cur);
        assert_eq!(default_shaped(&prev, &cur), expected);
    }
}
