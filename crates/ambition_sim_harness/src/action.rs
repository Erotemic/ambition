//! Agent-facing action vocabulary and conversion into the engine-owned `ControlFrame`.

use ambition_platformer2d::sim::ControlFrame;

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
    /// Edge-triggered "just pressed left this frame" in the raw input/screen
    /// frame. Most agents can leave this false; it exists for gesture tests
    /// under rotated control mappings.
    pub left_pressed: bool,
    /// Edge-triggered "just pressed right this frame" in the raw input/screen
    /// frame.
    pub right_pressed: bool,
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
    pub attack_held: bool,
    pub attack_released: bool,
    /// Device-independent strong-attack hint (dedicated smash key / C-stick).
    pub attack_strong: bool,
    /// Signature special (`Platformer2dInputActionMonolith::Special`). A dedicated slot since the
    /// `special_pressed = blink_pressed` alias was retired: pressing `blink` no
    /// longer fires a body's special, so an agent/scripted driver sets THIS to
    /// command the special (the folded player bubble_shield, a boss's authored
    /// content special via `dispatch_boss_special`, …).
    pub special: bool,
    /// The special button STAYS DOWN this frame. The sustain beside
    /// [`Self::special`]'s edge, and the only way a scripted driver or a policy
    /// can charge a held neutral special: the edge starts the move and this is
    /// what keeps its timeline frozen.
    pub special_held: bool,
    pub blink: bool,
    pub blink_held: bool,
    pub blink_released: bool,
    pub pogo: bool,
    pub interact: bool,
    /// Interact button HELD this frame (sustain), distinct from the `interact`
    /// rising edge. Hold gestures (e.g. possession's ~2s Down+Interact) read the
    /// held state; a real held button reports both (edge on frame one, held
    /// throughout). RL/scripted agents that only tap leave this false.
    pub interact_held: bool,
    pub projectile: bool,
    pub projectile_held: bool,
    pub projectile_released: bool,
    pub fly_toggle: bool,
    pub reset: bool,
    pub start: bool,
    /// Rising edge of the modifier slot — the sustained-technique control slot.
    pub modifier: bool,
    /// Modifier slot held this frame. A body's own rules decide what sustaining
    /// it does, so a headless driver can exercise a hold-driven technique
    /// (a locomotion mode, a stance) exactly as a device would.
    pub modifier_held: bool,
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
            burst_pressed: a.dash,
            // up_pressed / down_pressed are edge-triggered (just- pressed) on the desktop input
            // pipeline. Auto-deriving them from move_y > 0.5 every frame breaks gestures that
            // depend on the edge: register_down_tap reads down_pressed each tick and treats
            // every consecutive true as a fresh tap, which fires double-tap-down → MorphBall on
            // the second held frame.
            //
            // Fix: leave these fields neutral (false) by default in
            // the AgentAction → ControlFrame conversion. Agents that
            // genuinely want to fire an Up / Down edge can set the
            // explicit `up_pressed` / `down_pressed` fields on
            // AgentAction (added below) once-per-edge and the
            // converter forwards them. The continuous axis still
            // drives `axis_y` so gameplay reads (crouch, fast-fall,
            // ladder-climb) keep working.
            left_pressed: a.left_pressed,
            right_pressed: a.right_pressed,
            up_pressed: a.up_pressed,
            down_pressed: a.down_pressed,
            fast_fall_pressed: false,
            blink_pressed: a.blink,
            blink_held: a.blink_held,
            blink_released: a.blink_released,
            // Dedicated special slot (the alias `special_pressed = blink_pressed`
            // is retired): agents fire the special through `a.special`, not blink.
            special_pressed: a.special,
            special_held: a.special_held,
            attack_pressed: a.attack,
            attack_held: a.attack_held,
            attack_released: a.attack_released,
            // A scripted action says strong-or-not; the interpreter's three-valued
            // hint reads that as `Smash` or "you decide". Nothing in the harness
            // vocabulary asks to force a TILT yet — a right-stick mode is what
            // would (parity inventory §9).
            attack_strength_hint: if a.attack_strong {
                ambition_platformer2d::sim::AttackStrengthHint::Smash
            } else {
                ambition_platformer2d::sim::AttackStrengthHint::Auto
            },
            // ⛔ A SCRIPTED ACTION HAS NO SECOND STICK. It says which
            // direction it wants through the movement axis, so an attack from it
            // is aimed by that — which is what `false` means here.
            attack_from_aim_stick: false,
            pogo_pressed: a.pogo,
            fly_toggle_pressed: a.fly_toggle,
            interact_pressed: a.interact,
            interact_held: a.interact_held,
            reset_pressed: a.reset,
            start_pressed: a.start,
            projectile_pressed: a.projectile,
            projectile_held: a.projectile_held,
            projectile_released: a.projectile_released,
            shield_held: false,
            // false like `shield_held` beside it, and for the same reason: the
            // RL action space does not carry this verb yet. Adding it means
            // widening `AgentAction` and retraining against a larger space, which
            // is a decision about the harness rather than a line in a converter.
            grab_pressed: false,
            // Same reason as `grab_pressed` above, and one more: an agent has
            // nothing to gain from a taunt, which is what a taunt is.
            taunt_pressed: false,
            modifier_held: a.modifier_held,
            modifier_pressed: a.modifier,
            aim_x: a.aim_x,
            aim_y: a.aim_y,
        }
    }
}

#[cfg(test)]
mod action_tests;
