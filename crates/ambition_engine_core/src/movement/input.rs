use std::marker::PhantomData;

use crate::reference_frame::{LocalAxes, WorldVec2};
use crate::Vec2;

/// An enum whose variants index a compact fixed [`ActionEdges`] store. `COUNT`
/// must fit in the `u32` bitset (≤ 32), and `index` must be a stable, dense
/// `0..COUNT`. Implemented by [`MovementAction`] (and, later, any other
/// action family that wants the same edge storage).
pub trait ActionKey: Copy {
    const COUNT: usize;
    fn index(self) -> usize;
}

/// The closed set of movement-kernel actions — the gate through which input
/// reaches locomotion. The kernel dispatches on THIS, not on named booleans:
/// adding a movement verb is one variant here plus its handling, never a new
/// field threaded through every input struct.
///
/// Combat / interaction / reset are deliberately NOT here — they are not
/// movement-kernel primitives and live at their own seams (see the input
/// ownership split).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MovementAction {
    Jump,
    Dash,
    Blink,
    FlyToggle,
    FastFall,
}

impl MovementAction {
    /// All variants, in index order — for iteration and exhaustive folds.
    pub const ALL: [MovementAction; 5] = [
        Self::Jump,
        Self::Dash,
        Self::Blink,
        Self::FlyToggle,
        Self::FastFall,
    ];
}

impl ActionKey for MovementAction {
    const COUNT: usize = 5;
    fn index(self) -> usize {
        self as usize
    }
}

/// The three edge states of one action this frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Edge {
    pub pressed: bool,
    pub held: bool,
    pub released: bool,
}

impl Edge {
    pub const NONE: Edge = Edge {
        pressed: false,
        held: false,
        released: false,
    };
}

/// Compact per-action edge storage: three `u32` bitsets (pressed / held /
/// released) indexed by an [`ActionKey`]. `Copy`, `Default`, and cheap to pass
/// by value; the kernel reads it through the typed accessors below rather than
/// indexing raw.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActionEdges<A> {
    pressed: u32,
    held: u32,
    released: u32,
    _marker: PhantomData<A>,
}

impl<A> Default for ActionEdges<A> {
    fn default() -> Self {
        Self {
            pressed: 0,
            held: 0,
            released: 0,
            _marker: PhantomData,
        }
    }
}

impl<A: ActionKey> ActionEdges<A> {
    #[inline]
    pub fn pressed(&self, action: A) -> bool {
        self.pressed & bit(action) != 0
    }
    #[inline]
    pub fn held(&self, action: A) -> bool {
        self.held & bit(action) != 0
    }
    #[inline]
    pub fn released(&self, action: A) -> bool {
        self.released & bit(action) != 0
    }
    /// All three edges of one action at once.
    pub fn get(&self, action: A) -> Edge {
        Edge {
            pressed: self.pressed(action),
            held: self.held(action),
            released: self.released(action),
        }
    }
    /// Overwrite one action's edges.
    pub fn set(&mut self, action: A, edge: Edge) {
        let b = bit(action);
        set_bit(&mut self.pressed, b, edge.pressed);
        set_bit(&mut self.held, b, edge.held);
        set_bit(&mut self.released, b, edge.released);
    }
    /// Builder form of [`set`](Self::set).
    #[must_use]
    pub fn with(mut self, action: A, edge: Edge) -> Self {
        self.set(action, edge);
        self
    }
}

#[inline]
fn bit<A: ActionKey>(action: A) -> u32 {
    debug_assert!(action.index() < A::COUNT && A::COUNT <= 32);
    1u32 << action.index()
}

#[inline]
fn set_bit(mask: &mut u32, bit: u32, on: bool) {
    if on {
        *mask |= bit;
    } else {
        *mask &= !bit;
    }
}

#[cfg(test)]
mod action_edge_tests {
    use super::*;

    #[test]
    fn edges_are_independent_per_action() {
        let mut edges = ActionEdges::<MovementAction>::default();
        edges.set(
            MovementAction::Jump,
            Edge {
                pressed: true,
                held: true,
                released: false,
            },
        );
        edges.set(
            MovementAction::Blink,
            Edge {
                pressed: false,
                held: false,
                released: true,
            },
        );

        assert!(edges.pressed(MovementAction::Jump));
        assert!(edges.held(MovementAction::Jump));
        assert!(!edges.released(MovementAction::Jump));

        assert!(edges.released(MovementAction::Blink));
        assert!(!edges.pressed(MovementAction::Blink));

        // Untouched actions read as empty — no bleed from the bitset.
        assert_eq!(edges.get(MovementAction::Dash), Edge::NONE);
        assert_eq!(edges.get(MovementAction::FastFall), Edge::NONE);
    }

    #[test]
    fn clearing_an_edge_does_not_touch_neighbors() {
        let held = Edge {
            held: true,
            ..Edge::NONE
        };
        let mut edges = ActionEdges::<MovementAction>::default()
            .with(MovementAction::Jump, held)
            .with(MovementAction::Dash, held);
        // Clear Jump; Dash must survive.
        edges.set(MovementAction::Jump, Edge::NONE);
        assert!(!edges.held(MovementAction::Jump));
        assert!(edges.held(MovementAction::Dash));
    }

    #[test]
    fn index_is_dense_and_within_the_bitset() {
        assert_eq!(MovementAction::COUNT, MovementAction::ALL.len());
        for (i, action) in MovementAction::ALL.iter().enumerate() {
            assert_eq!(action.index(), i);
            assert!(action.index() < 32);
        }
    }
}

/// Game-action input for one simulation frame — the resolved motion intent.
///
/// Keyboard/gamepad remapping belongs in the presentation layer, and
/// screen-vs-body input-frame accommodation belongs at the controller seam.
/// Every directional field here carries its frame in its TYPE: by the time an
/// `InputState` reaches the movement kernel, all frame resolution has already
/// happened against the same [`crate::MotionFrame`] the kernel will step with.
///
/// - [`LocalAxes`] — controlled-body-local (`+x` side/right, `+y` toward-feet);
/// - [`WorldVec2`] — world-space, resolved through a controller frame policy at
///   the seam.
///
/// Raw [`crate::ScreenAxes`] never appear below the seam.
#[derive(Clone, Copy, Debug, Default)]
pub struct InputState {
    /// Locomotion stick in the controlled body's local frame.
    pub axes: LocalAxes,
    pub jump_pressed: bool,
    pub jump_held: bool,
    pub jump_released: bool,
    pub dash_pressed: bool,
    /// Toggle free-flight mode when the ability is enabled.
    pub fly_toggle_pressed: bool,
    /// Blink/special button pressed this frame.
    pub blink_pressed: bool,
    /// Blink/special button held this frame.
    pub blink_held: bool,
    /// Blink/special button released this frame.
    pub blink_released: bool,
    /// WORLD-space quick-blink direction, already resolved through the movement
    /// frame mode at the input seam. The engine consumes this directly (it does
    /// NOT re-derive blink direction from the local `axes`), so quick blink is
    /// locomotion-framed and gravity-correct without the engine knowing the
    /// gravity frame. Zero → fall back to facing.
    pub blink_quick_dir: WorldVec2,
    /// WORLD-space precision-blink steer vector for the current frame, resolved
    /// through the *aim* frame mode at the seam (screen-directed by default).
    /// Magnitude carries the stick deflection; the engine integrates it into the
    /// precision aim offset. Decoupled from `blink_quick_dir` so quick blink and
    /// precision blink can use different frame policies on the same stick.
    pub blink_aim_step: WorldVec2,
    /// Double-tap-down gesture recognized by the input layer. This is separate
    /// from the local descend axis so down+attack can mean pogo without forcing
    /// fast-fall.
    pub fast_fall_pressed: bool,
    pub attack_pressed: bool,
    /// Dedicated downward/pogo slash action. This is separate from
    /// `attack_pressed` so layouts can expose four main face-button verbs.
    pub pogo_pressed: bool,
    /// Generic context/confirm input. The engine only uses this for mechanics
    /// that are already movement-owned (currently ledge pull-up confirm); room
    /// interactions remain sandbox-owned.
    pub interact_pressed: bool,
    pub reset_pressed: bool,
    /// Shield button is currently held. When the `shield` ability is active,
    /// holding this deploys the bubble; releasing drops it. The first
    /// `parry_window_time` seconds after activation are the parry window (full
    /// invulnerability).
    pub shield_held: bool,
    /// Real, unscaled frame duration supplied by the presentation layer.
    ///
    /// Most simulation uses the scaled `raw_dt`, but precision-blink aiming is
    /// a control/UI gesture: the cursor should stay responsive even when game
    /// time is nearly frozen. If zero, the engine falls back to scaled dt.
    pub control_dt: f32,
}

impl InputState {
    /// The locomotion stick in the controlled body's local acceleration frame,
    /// as a bare vector for kernel-internal math.
    pub const fn local_axis(self) -> Vec2 {
        self.axes.vec()
    }

    /// Convenience constructor for a locomotion-only intent.
    pub const fn with_axes(x: f32, y: f32) -> Self {
        let mut input = Self::const_default();
        input.axes = LocalAxes::new(x, y);
        input
    }

    const fn const_default() -> Self {
        Self {
            axes: LocalAxes::ZERO,
            jump_pressed: false,
            jump_held: false,
            jump_released: false,
            dash_pressed: false,
            fly_toggle_pressed: false,
            blink_pressed: false,
            blink_held: false,
            blink_released: false,
            blink_quick_dir: WorldVec2::ZERO,
            blink_aim_step: WorldVec2::ZERO,
            fast_fall_pressed: false,
            attack_pressed: false,
            pogo_pressed: false,
            interact_pressed: false,
            reset_pressed: false,
            shield_held: false,
            control_dt: 0.0,
        }
    }
}
