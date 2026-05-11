use crate::input::ControlFrame;

/// Action emitted by an RL agent / scripted driver every tick.
///
/// Fields mirror the engine-relevant subset of `ControlFrame` — held vs
/// pressed flags are kept because the sandbox uses both edges (a held
/// jump glides; a pressed jump kicks off the buffered jump path). The
/// `aim_x` / `aim_y` knobs feed precision-blink aim when blink is held.
///
/// Defaults are all-zero / all-false: a `do nothing` action. Constructed
/// fields can be set individually since most agent policies emit a
/// sparse per-frame intent (e.g. just `move_x = 1.0` for "walk right").
#[derive(Clone, Copy, Debug, Default)]
pub struct AgentAction {
    pub move_x: f32,
    pub move_y: f32,
    /// Edge-triggered "just pressed up this frame". The desktop
    /// input pipeline sets this from `actions.just_pressed(MoveUp)`;
    /// agents that want to fire an Up gesture (door tap, ladder
    /// entry) set this to true on a single frame and back to false
    /// on subsequent frames. The continuous `move_y` axis still
    /// drives gameplay reads that need held-state.
    pub up_pressed: bool,
    /// Edge-triggered "just pressed down this frame". Same shape as
    /// `up_pressed`; setting it true every frame would re-trigger
    /// the double-tap-down → MorphBall gesture incorrectly.
    pub down_pressed: bool,
    pub jump: bool,
    pub jump_held: bool,
    pub jump_released: bool,
    pub dash: bool,
    pub attack: bool,
    pub blink: bool,
    pub blink_held: bool,
    pub blink_released: bool,
    pub pogo: bool,
    pub interact: bool,
    pub projectile: bool,
    pub projectile_held: bool,
    pub projectile_released: bool,
    pub fly_toggle: bool,
    pub reset: bool,
    pub start: bool,
    pub aim_x: f32,
    pub aim_y: f32,
}

impl AgentAction {
    /// Convenience constructor for tests / agent policies that only set
    /// the horizontal axis.
    pub fn move_x(value: f32) -> Self {
        Self {
            move_x: value,
            ..Self::default()
        }
    }

    /// Convenience: a pressed-this-frame jump with held kept on.
    pub fn jump() -> Self {
        Self {
            jump: true,
            jump_held: true,
            ..Self::default()
        }
    }

    /// Convenience: pressed-this-frame reset.
    pub fn reset() -> Self {
        Self {
            reset: true,
            ..Self::default()
        }
    }
}

impl From<AgentAction> for ControlFrame {
    fn from(a: AgentAction) -> Self {
        ControlFrame {
            axis_x: a.move_x,
            axis_y: a.move_y,
            jump_pressed: a.jump,
            jump_held: a.jump_held,
            jump_released: a.jump_released,
            dash_pressed: a.dash,
            // up_pressed / down_pressed are edge-triggered (just-
            // pressed) on the desktop input pipeline. Auto-deriving
            // them from move_y > 0.5 every frame breaks gestures
            // that depend on the edge: register_down_tap reads
            // down_pressed each tick and treats every consecutive
            // true as a fresh tap, which fires double-tap-down →
            // MorphBall on the second held frame. Crouch is the
            // visible symptom: holding Down should crouch
            // continuously, not curl into MorphBall after one frame.
            //
            // Fix: leave these fields neutral (false) by default in
            // the AgentAction → ControlFrame conversion. Agents that
            // genuinely want to fire an Up / Down edge can set the
            // explicit `up_pressed` / `down_pressed` fields on
            // AgentAction (added below) once-per-edge and the
            // converter forwards them. The continuous axis still
            // drives `axis_y` so gameplay reads (crouch, fast-fall,
            // ladder-climb) keep working.
            up_pressed: a.up_pressed,
            down_pressed: a.down_pressed,
            fast_fall_pressed: false,
            blink_pressed: a.blink,
            blink_held: a.blink_held,
            blink_released: a.blink_released,
            attack_pressed: a.attack,
            pogo_pressed: a.pogo,
            fly_toggle_pressed: a.fly_toggle,
            interact_pressed: a.interact,
            reset_pressed: a.reset,
            start_pressed: a.start,
            projectile_pressed: a.projectile,
            projectile_held: a.projectile_held,
            projectile_released: a.projectile_released,
            aim_x: a.aim_x,
            aim_y: a.aim_y,
        }
    }
}
