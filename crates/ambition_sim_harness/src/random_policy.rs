//! Small deterministic policies used by harness examples and stress tests.

use crate::AgentAction;

/// Tiny deterministic LCG used by the RL demo binaries and fuzz tests.
/// It is not a quality RNG; it is stable, cheap, and seed-reproducible.
#[derive(Clone, Copy, Debug)]
pub struct Lcg(u64);

impl Lcg {
    pub fn new(seed: u64) -> Self {
        Self(seed.max(1))
    }

    pub fn next_u32(&mut self) -> u32 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.0 >> 32) as u32
    }

    pub fn unit(&mut self) -> f32 {
        (self.next_u32() as f32) / (u32::MAX as f32 + 1.0)
    }

    pub fn signed_unit(&mut self) -> f32 {
        2.0 * self.unit() - 1.0
    }

    pub fn chance(&mut self, p: f32) -> bool {
        self.unit() < p
    }
}

/// Per-step action probabilities for deterministic random-walk policies.
#[derive(Clone, Copy, Debug)]
pub struct RandomWalkTuning {
    pub axis_change_chance: f32,
    pub pinned_axis_chance: f32,
    pub jump_chance: f32,
    pub jump_hold_chance: f32,
    pub dash_chance: f32,
    pub attack_chance: f32,
    pub blink_chance: f32,
    pub pogo_chance: f32,
    pub interact_chance: f32,
    pub fly_toggle_chance: f32,
    pub reset_chance: f32,
    pub up_pressed_chance: f32,
    pub down_pressed_chance: f32,
    pub vertical_drift: f32,
    pub idle_aim_drift: f32,
    pub blink_uses_random_aim: bool,
}

impl RandomWalkTuning {
    /// The default demo policy: lots of lateral motion, occasional jump/dash/
    /// blink/attack/interact, and very rare resets.
    pub fn demo() -> Self {
        Self {
            axis_change_chance: 0.06,
            pinned_axis_chance: 0.5,
            jump_chance: 0.05,
            jump_hold_chance: 0.5,
            dash_chance: 0.02,
            attack_chance: 0.01,
            blink_chance: 0.005,
            pogo_chance: 0.0,
            interact_chance: 0.01,
            fly_toggle_chance: 0.0,
            reset_chance: 0.0005,
            up_pressed_chance: 0.0,
            down_pressed_chance: 0.0,
            vertical_drift: 0.0,
            idle_aim_drift: 0.05,
            blink_uses_random_aim: true,
        }
    }

    /// Fast test policy: same basic movement as the demo, but no random resets.
    pub fn fuzz() -> Self {
        Self {
            reset_chance: 0.0,
            idle_aim_drift: 0.0,
            blink_uses_random_aim: false,
            ..Self::demo()
        }
    }

    /// Traversal-stress policy for collision diagnostics: more vertical input,
    /// jumping, flying, pogo, and blink pressure into ceilings/corners.
    pub fn traversal_stress() -> Self {
        Self {
            axis_change_chance: 0.08,
            pinned_axis_chance: 0.6,
            jump_chance: 0.10,
            jump_hold_chance: 0.6,
            dash_chance: 0.04,
            attack_chance: 0.01,
            blink_chance: 0.02,
            pogo_chance: 0.02,
            interact_chance: 0.0,
            fly_toggle_chance: 0.03,
            reset_chance: 0.0,
            up_pressed_chance: 0.20,
            down_pressed_chance: 0.08,
            vertical_drift: 0.3,
            idle_aim_drift: 1.0,
            blink_uses_random_aim: true,
        }
    }
}

/// Deterministic sticky-axis random action policy.
pub struct RandomWalkPolicy {
    rng: Lcg,
    axis_x: f32,
    tuning: RandomWalkTuning,
}

impl RandomWalkPolicy {
    pub fn new(seed: u64, tuning: RandomWalkTuning) -> Self {
        Self {
            rng: Lcg::new(seed),
            axis_x: 0.0,
            tuning,
        }
    }

    pub fn demo(seed: u64) -> Self {
        Self::new(seed, RandomWalkTuning::demo())
    }

    pub fn fuzz(seed: u64) -> Self {
        Self::new(seed, RandomWalkTuning::fuzz())
    }

    pub fn traversal_stress(seed: u64) -> Self {
        Self::new(seed, RandomWalkTuning::traversal_stress())
    }

    pub fn rng_mut(&mut self) -> &mut Lcg {
        &mut self.rng
    }

    pub fn act(&mut self) -> AgentAction {
        if self.rng.chance(self.tuning.axis_change_chance) {
            self.axis_x = if self.rng.chance(self.tuning.pinned_axis_chance) {
                if self.rng.chance(0.5) {
                    1.0
                } else {
                    -1.0
                }
            } else {
                self.rng.signed_unit()
            };
        }
        let jump = self.rng.chance(self.tuning.jump_chance);
        let blink = self.rng.chance(self.tuning.blink_chance);
        let up_pressed = self.rng.chance(self.tuning.up_pressed_chance);
        let (aim_x, aim_y) = if blink && self.tuning.blink_uses_random_aim {
            (self.rng.signed_unit(), self.rng.signed_unit())
        } else {
            (
                self.rng.signed_unit() * self.tuning.idle_aim_drift,
                self.rng.signed_unit() * self.tuning.idle_aim_drift,
            )
        };
        AgentAction {
            move_x: self.axis_x,
            move_y: if up_pressed {
                -1.0
            } else {
                self.rng.signed_unit() * self.tuning.vertical_drift
            },
            up_pressed,
            down_pressed: self.rng.chance(self.tuning.down_pressed_chance),
            jump,
            jump_held: jump || self.rng.chance(self.tuning.jump_hold_chance),
            dash: self.rng.chance(self.tuning.dash_chance),
            attack: self.rng.chance(self.tuning.attack_chance),
            blink,
            pogo: self.rng.chance(self.tuning.pogo_chance),
            interact: self.rng.chance(self.tuning.interact_chance),
            fly_toggle: self.rng.chance(self.tuning.fly_toggle_chance),
            reset: self.rng.chance(self.tuning.reset_chance),
            aim_x,
            aim_y,
            ..AgentAction::default()
        }
    }
}
