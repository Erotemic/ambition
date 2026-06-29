//! The gravity-relative reference frame and the transforms between Ambition's
//! three frames.
//!
//! Ambition reasons about three reference frames; the bugs in this area
//! (attack direction, pogo, sprite orientation, facing flip) were all "someone
//! forgot to transform between two of them". [`AccelerationFrame`] makes the frames
//! and the transforms explicit so being gravity-aware is "you hold a
//! `AccelerationFrame`", not "you remembered to multiply by `gravity_dir`".
//!
//! - **Input frame** — the controller: `axis_x` right-positive, `axis_y`
//!   screen-down-positive. Raw, never rotated.
//! - **Local body frame** — relative to the controlled body: `+x` is the run / side
//!   axis, `+y` is *toward the feet* (the body's own "down"). Combat geometry,
//!   impulses, and gates are authored here, in the upright (normal-gravity) pose.
//! - **World frame** — engine coordinates (`+y` screen-down).
//!
//! Under normal gravity the local body frame *equals* the world frame, so every
//! transform below is the identity and play is byte-identical.

use crate::Vec2;

/// Stick magnitude above which a source counts as "engaged" for
/// [`AccelerationFrame::resolve_aim_local`]'s aim → movement → facing priority.
const STICK_SELECT_DEADZONE: f32 = 0.3;

/// How the raw INPUT frame maps onto the controlled body's local frame — "which
/// way is right when gravity is sideways or upside-down". A human-control
/// preference (see [`AccelerationFrame::control_frame`]).
///
/// Deliberately NOT `Default`: there is no source-agnostic default frame mode.
/// The default depends on the INPUT SOURCE — see [`Self::DEFAULT_MOVEMENT`] /
/// [`Self::DEFAULT_AIM`], which are the single source of truth that
/// [`ControlFrameModes::default`] and every settings/tuning fallback resolve to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum InputFrameMode {
    /// Input is always SCREEN-aligned: right is screen-right regardless of
    /// gravity (the human mentally tracks the controlled body).
    ScreenRelative,
    /// Input always follows the controlled body's local frame: right is the
    /// body's own right, fully rotated with gravity (no accommodation).
    BodyRelativeStrict,
    /// HYBRID / body-relative assist: follow the controlled body frame
    /// up to ±90° from screen-down — gravity down / left / right, where a human
    /// tracks the rotation fine — then revert to screen-aligned past 90° (gravity
    /// up-ish), where the flip is hard to map. The vertical "descend" gate
    /// (pogo / crouch) is independent and always flips with the body frame
    /// ([`Self::descend`]).
    BodyRelativeAssist,
}

impl InputFrameMode {
    /// THE default for LOCOMOTION input. Single source of truth — every
    /// settings/tuning/fallback default for the movement stick resolves here
    /// (directly, or via [`ControlFrameModes::default`]). A `const` so `const`
    /// contexts like [`crate::movement::DEFAULT_TUNING`] can reference it too.
    pub const DEFAULT_MOVEMENT: Self = Self::ScreenRelative;
    /// THE default for PRECISION-AIM input (blink steer, ranged/held aim) — point
    /// where the stick points on screen at any gravity. Single source of truth.
    pub const DEFAULT_AIM: Self = Self::ScreenRelative;
}

/// The pair of [`InputFrameMode`] policies a control authority maps raw input
/// through, split by INPUT SOURCE rather than by actor.
///
/// The locomotion stick (left stick / movement keys) and the precision-aim stick
/// (right stick / aim) are physically different sources and a human tracks them
/// differently under rotated gravity, so they each carry their own mapping
/// policy. Both default to screen-directed ([`InputFrameMode::ScreenRelative`]) —
/// press / point a screen direction and the controlled body moves / aims that way
/// on screen at any gravity. See [`InputFrameMode::DEFAULT_MOVEMENT`] /
/// [`InputFrameMode::DEFAULT_AIM`] for the single source of truth.
///
/// This is frame-agnostic and actor-agnostic: it is a control-authority preference,
/// not a property of any one (privileged) actor. [`AccelerationFrame::resolve_aim_local`]
/// consumes it for the verbs that pick a direction by source priority (aim → move
/// → facing).
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ControlFrameModes {
    /// How the locomotion stick maps onto the body's local frame.
    pub movement: InputFrameMode,
    /// How the precision-aim stick maps onto the body's local frame.
    pub aim: InputFrameMode,
}

impl Default for ControlFrameModes {
    /// Both locomotion and precision aiming default to screen-directed, resolved
    /// from the per-source single source of truth on [`InputFrameMode`].
    fn default() -> Self {
        Self {
            movement: InputFrameMode::DEFAULT_MOVEMENT,
            aim: InputFrameMode::DEFAULT_AIM,
        }
    }
}

/// Raw digital direction edges in the input/screen frame.
///
/// These are intentionally separate from the analog axis: an axis can be held
/// for many frames, while double-tap / interact gestures need the single frame
/// on which a cardinal direction became newly active.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RawDirectionEdges {
    pub left: bool,
    pub right: bool,
    pub up: bool,
    pub down: bool,
}

impl RawDirectionEdges {
    pub const fn new(left: bool, right: bool, up: bool, down: bool) -> Self {
        Self {
            left,
            right,
            up,
            down,
        }
    }

    fn pressed_for_raw_axis(self, raw_axis: Vec2) -> bool {
        if raw_axis.length_squared() <= 1e-6 {
            return false;
        }
        let axis = raw_axis.normalize();
        let candidates = [
            (self.right, Vec2::new(1.0, 0.0)),
            (self.down, Vec2::new(0.0, 1.0)),
            (self.left, Vec2::new(-1.0, 0.0)),
            (self.up, Vec2::new(0.0, -1.0)),
        ];
        let mut best = candidates[0];
        let mut best_dot = axis.dot(candidates[0].1);
        for candidate in candidates.iter().copied().skip(1) {
            let dot = axis.dot(candidate.1);
            if dot > best_dot {
                best = candidate;
                best_dot = dot;
            }
        }
        best.0
    }
}

/// The controlled body's local interpretation of one raw input frame.
///
/// This is the reference-frame seam: presentation/input systems supply raw axes
/// in screen/input coordinates; gameplay verbs should consume `local_axis` when
/// they mean unqualified left/right/up/down for the controlled body.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ResolvedControlFrame {
    /// Raw input/screen-frame stick: `+x` screen-right, `+y` screen-down.
    pub raw_axis: Vec2,
    /// Controlled-body-local stick: `+x` local side/right, `+y` local down
    /// / toward-feet.
    pub local_axis: Vec2,
    pub mode: InputFrameMode,
    pub frame: AccelerationFrame,
}

impl ResolvedControlFrame {
    pub fn local_down_pressed(self, edges: RawDirectionEdges) -> bool {
        self.frame
            .local_direction_pressed(self.mode, Vec2::new(0.0, 1.0), edges)
    }

    pub fn local_up_pressed(self, edges: RawDirectionEdges) -> bool {
        self.frame
            .local_direction_pressed(self.mode, Vec2::new(0.0, -1.0), edges)
    }

    pub fn local_right_pressed(self, edges: RawDirectionEdges) -> bool {
        self.frame
            .local_direction_pressed(self.mode, Vec2::new(1.0, 0.0), edges)
    }

    pub fn local_left_pressed(self, edges: RawDirectionEdges) -> bool {
        self.frame
            .local_direction_pressed(self.mode, Vec2::new(-1.0, 0.0), edges)
    }
}

/// The controlled body's local reference frame under a net "down"-defining acceleration.
///
/// The common source of `down` is gravity, but the frame is deliberately not
/// gravity-specific (hence the name): any net proper acceleration — a force
/// field, thrust, a spinning room — defines the body's local "down", and the
/// frame transforms the same way. The direction is also NOT snapped to a
/// cardinal, so an off-axis / rotating "down" works (the transforms are general
/// rotations); the gravity system happens to feed cardinal directions today.
///
/// `down` (toward the feet, a unit vector) and `side` (the perpendicular run
/// axis) are the local body frame's basis expressed in world coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AccelerationFrame {
    /// Toward the feet (unit) — the player's own "down". `(0,1)` under normal
    /// gravity.
    pub down: Vec2,
    /// The run / side axis (perpendicular to `down`). `(1,0)` under normal gravity.
    pub side: Vec2,
}

/// Declares the frame a gameplay quantity is authored or interpreted in.
///
/// This is intentionally a code-level contract, not content metadata. It lets
/// tests and call sites distinguish body-local verbs from screen/HUD input and
/// true world/environment geometry without inventing authored floors, walls, or
/// other surface labels.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GameplayFramePolicy {
    /// Local to the controlled body: `x` is side/right and `y` is toward feet.
    ControlledBodyLocal,
    /// Relative to the current acceleration frame, usually equivalent to
    /// controlled-body-local for movement/contact mechanics.
    AccelerationFrame,
    /// Fixed world/environment space. Use for room geometry, scripted world
    /// hazards, and other effects that deliberately do not rotate with a body.
    WorldSpace,
    /// Raw display/input space. This should live at the input seam and be
    /// converted before gameplay resolution.
    ScreenSpace,
}

impl AccelerationFrame {
    /// Build the frame from the net "down"-defining acceleration (gravity is the
    /// usual source). The direction is normalized but NOT cardinal-snapped, so an
    /// arbitrary-angle `down` is supported. The side axis is `down` rotated −90°,
    /// so under normal gravity `down=(0,1)`, `side=(1,0)` and every transform is
    /// the identity. A zero acceleration defaults to normal-gravity down.
    pub fn new(acceleration: Vec2) -> Self {
        let down = acceleration.try_normalize().unwrap_or(Vec2::new(0.0, 1.0));
        Self {
            down,
            side: Vec2::new(down.y, -down.x),
        }
    }

    /// Build the frame from the nearest cardinal down direction to an arbitrary
    /// acceleration vector.
    ///
    /// Physics may eventually keep arbitrary-angle acceleration, but digital
    /// controls and glyph labels intentionally snap into the four principal
    /// screen directions. Keeping that snap here makes the “four cones” rule a
    /// shared reference-frame policy instead of a per-mechanic special case.
    pub fn cardinalized(acceleration: Vec2) -> Self {
        Self::new(Self::nearest_cardinal_down(acceleration))
    }

    /// Nearest principal `down` direction to `acceleration`.
    pub fn nearest_cardinal_down(acceleration: Vec2) -> Vec2 {
        let down = acceleration.try_normalize().unwrap_or(Vec2::new(0.0, 1.0));
        let candidates = [
            Vec2::new(0.0, 1.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(0.0, -1.0),
            Vec2::new(-1.0, 0.0),
        ];
        let mut best = candidates[0];
        let mut best_dot = down.dot(best);
        for candidate in candidates.iter().copied().skip(1) {
            let dot = down.dot(candidate);
            if dot > best_dot {
                best = candidate;
                best_dot = dot;
            }
        }
        best
    }

    /// The frame the INPUT stick maps through, per the player's [`InputFrameMode`].
    /// `to_world(stick)` on the result turns raw `(axis_x, axis_y)` into a
    /// world-space movement direction. `Screen` → identity; `Player` → this frame;
    /// `Hybrid` → this frame up to ±90° (down.y ≥ 0), else screen-aligned. NOTE:
    /// this drives free MOVEMENT (run / flight); the toward-feet GATE uses
    /// [`Self::descend`] directly so pogo/crouch always flip with the player.
    pub fn control_frame(self, mode: InputFrameMode) -> AccelerationFrame {
        let screen = AccelerationFrame::new(Vec2::new(0.0, 1.0));
        match mode {
            InputFrameMode::ScreenRelative => screen,
            InputFrameMode::BodyRelativeStrict => self,
            InputFrameMode::BodyRelativeAssist => {
                if self.down.y >= 0.0 {
                    self
                } else {
                    screen
                }
            }
        }
    }

    /// INPUT → PLAYER. Screen-vertical input (`axis_y`, +Y screen-down) → the
    /// "descend" (toward-feet) intent that gates crouch / pogo / drop-through /
    /// fast-fall. The accommodation: the gate stays on the up/down keys and only
    /// flips sign once gravity rotates PAST ±90° from screen-down (i.e. gravity
    /// points up-ish). Identity under normal gravity.
    ///
    /// This is exactly the `y` of [`Self::resolve_input`] in [`InputFrameMode::BodyRelativeAssist`];
    /// prefer `resolve_input` at the input seam so the run axis and the descend
    /// gate honor the SAME mode together.
    pub fn descend(self, input_axis_y: f32) -> f32 {
        input_axis_y * if self.down.y < 0.0 { -1.0 } else { 1.0 }
    }

    /// INPUT → PLAYER, both axes. Resolve the raw INPUT-frame stick
    /// `(axis_x, axis_y)` (right-positive / screen-down-positive) into a
    /// local-body-frame stick — `x` = run (along [`Self::side`]), `y` = descend
    /// (toward the feet, along [`Self::down`]) — per the player's
    /// [`InputFrameMode`]. [`Self::to_world`] on the result gives the world-space
    /// movement direction; the `x`/`y` scalars drive the run axis and the
    /// descend gates respectively.
    ///
    /// - [`InputFrameMode::BodyRelativeStrict`] — the stick already IS the local body frame:
    ///   `(axis_x, axis_y)`, fully rotated with gravity.
    /// - [`InputFrameMode::ScreenRelative`] — the stick is screen-aligned; project it onto
    ///   the player basis so the body moves the way the stick points ON SCREEN at
    ///   any gravity (push screen-right → move screen-right). Under sideways
    ///   gravity the run/descend roles swap, exactly as screen-directed control
    ///   expects.
    /// - [`InputFrameMode::BodyRelativeAssist`] — BYTE-IDENTICAL at every
    ///   orientation to the old `axis_x` run + [`Self::descend`] gate: it equals
    ///   `BodyRelativeStrict` up to ±90° from screen-down, then inverts BOTH axes
    ///   past 90° (gravity up-ish) so the hard-to-track flip reverts to a
    ///   screen-like feel.
    pub fn resolve_input(self, mode: InputFrameMode, axis_x: f32, axis_y: f32) -> Vec2 {
        match mode {
            InputFrameMode::BodyRelativeStrict => Vec2::new(axis_x, axis_y),
            InputFrameMode::ScreenRelative => {
                let input = Vec2::new(axis_x, axis_y);
                Vec2::new(input.dot(self.side), input.dot(self.down))
            }
            InputFrameMode::BodyRelativeAssist => {
                let s = if self.down.y < 0.0 { -1.0 } else { 1.0 };
                Vec2::new(axis_x * s, axis_y * s)
            }
        }
    }

    /// Resolve a direction-picking verb (blink target, grapple/dive direction,
    /// held-shot aim) into the controlled body's LOCAL frame, choosing the frame
    /// policy by INPUT SOURCE per [`ControlFrameModes`]:
    ///
    /// - **aim stick engaged** → precision aiming, resolved through `modes.aim`
    ///   (the "precision blink");
    /// - **else movement stick engaged** → locomotion, resolved through
    ///   `modes.movement` (the "quick blink");
    /// - **else** → body-local facing (`+x`), no mode needed.
    ///
    /// `aim` and `movement` are raw INPUT-frame sticks (`+x` screen-right, `+y`
    /// screen-down); `facing` is the body's screen-space facing sign. The result
    /// is unit-length (or the facing fallback). [`Self::to_world`] lifts it to
    /// world space for the raycast / spawn.
    pub fn resolve_aim_local(
        self,
        modes: ControlFrameModes,
        aim: Vec2,
        movement: Vec2,
        facing: f32,
    ) -> Vec2 {
        if aim.length() > STICK_SELECT_DEADZONE {
            return self
                .resolve_input(modes.aim, aim.x, aim.y)
                .normalize_or_zero();
        }
        if movement.length() > STICK_SELECT_DEADZONE {
            return self
                .resolve_input(modes.movement, movement.x, movement.y)
                .normalize_or_zero();
        }
        Vec2::new(if facing >= 0.0 { 1.0 } else { -1.0 }, 0.0)
    }

    /// Resolve a raw input/screen-frame stick into the controlled body's local
    /// frame and keep both representations together for consumers that need to
    /// be explicit about which frame they are using.
    pub fn resolve_control(
        self,
        mode: InputFrameMode,
        axis_x: f32,
        axis_y: f32,
    ) -> ResolvedControlFrame {
        ResolvedControlFrame {
            raw_axis: Vec2::new(axis_x, axis_y),
            local_axis: self.resolve_input(mode, axis_x, axis_y),
            mode,
            frame: self,
        }
    }

    /// Inverse of [`Self::resolve_input`] for a local/body-frame axis.
    ///
    /// This is primarily used for touch-glyph placement: given the semantic
    /// local command (`D`, `U`, `L`, `R`), find the raw joystick direction that
    /// should be labeled with that command under the active mapping policy.
    pub fn raw_axis_for_resolved_input(self, mode: InputFrameMode, local_axis: Vec2) -> Vec2 {
        match mode {
            InputFrameMode::BodyRelativeStrict => local_axis,
            InputFrameMode::ScreenRelative => self.to_world(local_axis),
            InputFrameMode::BodyRelativeAssist => {
                let s = if self.down.y < 0.0 { -1.0 } else { 1.0 };
                local_axis * s
            }
        }
    }

    /// Test whether a raw cardinal edge corresponds to the given local/body
    /// direction under this input mapping.
    pub fn local_direction_pressed(
        self,
        mode: InputFrameMode,
        local_axis: Vec2,
        edges: RawDirectionEdges,
    ) -> bool {
        edges.pressed_for_raw_axis(self.raw_axis_for_resolved_input(mode, local_axis))
    }

    /// LOCAL BODY → WORLD. Rotate a local-body vector (authored with `+y` toward the
    /// feet) into world coordinates. Identity under normal gravity.
    pub fn to_world(self, player: Vec2) -> Vec2 {
        self.side * player.x + self.down * player.y
    }

    /// WORLD → LOCAL BODY. Project a world vector into this acceleration frame:
    /// `x` is side/right, `y` is toward feet.
    pub fn to_local(self, world: Vec2) -> Vec2 {
        Vec2::new(world.dot(self.side), world.dot(self.down))
    }

    /// LOCAL BODY → WORLD for an axis-aligned half-extent. Returns the world-space
    /// AABB half-extent that BOUNDS the rotated box: exact for cardinal frames
    /// (90° just swaps width/height), an over-approximation for off-axis frames
    /// (the bound of the tilted rectangle). Identity under normal / inverted
    /// gravity.
    pub fn to_world_half(self, half: Vec2) -> Vec2 {
        Vec2::new(
            (self.side.x * half.x).abs() + (self.down.x * half.y).abs(),
            (self.side.y * half.x).abs() + (self.down.y * half.y).abs(),
        )
    }

    /// Set `vel` to a launch of `speed` AWAY from the feet (jump / pogo bounce),
    /// preserving the component perpendicular to gravity.
    pub fn launch(self, vel: &mut Vec2, speed: f32) {
        let perp = *vel - vel.dot(self.down) * self.down;
        *vel = perp - speed * self.down;
    }

    /// The component of `vel` directed toward the feet (its descent speed).
    pub fn descend_speed(self, vel: Vec2) -> f32 {
        vel.dot(self.down)
    }

    /// Force `vel`'s toward-feet component to at least `speed` (used to "commit"
    /// a down-attack), leaving the perpendicular component alone.
    pub fn ensure_descend_speed(self, vel: &mut Vec2, speed: f32) {
        let cur = vel.dot(self.down);
        if cur < speed {
            *vel += self.down * (speed - cur);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normal_gravity_is_identity() {
        let f = AccelerationFrame::new(Vec2::new(0.0, 1.0));
        assert_eq!(f.down, Vec2::new(0.0, 1.0));
        assert_eq!(f.side, Vec2::new(1.0, 0.0));
        assert_eq!(f.descend(0.7), 0.7);
        assert_eq!(f.to_world(Vec2::new(3.0, 5.0)), Vec2::new(3.0, 5.0));
        assert_eq!(
            f.to_world_half(Vec2::new(26.0, 34.0)),
            Vec2::new(26.0, 34.0)
        );
    }

    #[test]
    fn inverted_gravity_flips_descend_and_vertical() {
        let f = AccelerationFrame::new(Vec2::new(0.0, -1.0));
        // Holding screen-up (axis_y = -1) is "descend" (toward the up-feet).
        assert_eq!(f.descend(-1.0), 1.0);
        // A local-body "toward feet" offset (+y) maps to screen-up.
        assert_eq!(f.to_world(Vec2::new(0.0, 32.0)), Vec2::new(0.0, -32.0));
        // Vertical half-extent unchanged (still a 180° frame, no axis swap).
        assert_eq!(
            f.to_world_half(Vec2::new(26.0, 34.0)),
            Vec2::new(26.0, 34.0)
        );
    }

    #[test]
    fn sideways_gravity_swaps_axes() {
        let f = AccelerationFrame::new(Vec2::new(1.0, 0.0)); // gravity points screen-right
        assert_eq!(f.down, Vec2::new(1.0, 0.0));
        // Toward-feet (+y player) maps to screen-right.
        assert_eq!(f.to_world(Vec2::new(0.0, 32.0)), Vec2::new(32.0, 0.0));
        // A wide-thin down-attack box becomes thin-wide in world.
        assert_eq!(
            f.to_world_half(Vec2::new(26.0, 34.0)),
            Vec2::new(34.0, 26.0)
        );
    }

    #[test]
    fn off_axis_down_is_a_general_rotation() {
        // A 45° "down" (toward screen down-right) is not snapped — the frame is a
        // real rotation, so toward-feet maps along the diagonal.
        let f = AccelerationFrame::new(Vec2::new(1.0, 1.0));
        let inv_sqrt2 = 1.0 / 2.0_f32.sqrt();
        assert!((f.down - Vec2::new(inv_sqrt2, inv_sqrt2)).length() < 1e-6);
        let feet = f.to_world(Vec2::new(0.0, 10.0));
        assert!((feet - Vec2::new(10.0 * inv_sqrt2, 10.0 * inv_sqrt2)).length() < 1e-5);
    }

    #[test]
    fn hybrid_control_frame_rotates_to_90_then_reverts() {
        // Right gravity (≤90°): the control frame follows the player, so "right"
        // on the stick maps to screen-up (the player's right).
        let right = AccelerationFrame::new(Vec2::new(1.0, 0.0));
        let cf = right.control_frame(InputFrameMode::BodyRelativeAssist);
        let world = cf.to_world(Vec2::new(1.0, 0.0));
        assert!((world - Vec2::new(0.0, -1.0)).length() < 1e-6, "{world:?}");
        // Up gravity (>90°): the control frame reverts to screen, so "right" maps
        // to screen-right (= the player's left — the accommodation).
        let up = AccelerationFrame::new(Vec2::new(0.0, -1.0));
        let cf = up.control_frame(InputFrameMode::BodyRelativeAssist);
        assert_eq!(cf.to_world(Vec2::new(1.0, 0.0)), Vec2::new(1.0, 0.0));
        // Player mode never reverts; Screen mode never rotates.
        assert_eq!(up.control_frame(InputFrameMode::BodyRelativeStrict), up);
        assert_eq!(
            up.control_frame(InputFrameMode::ScreenRelative).down,
            Vec2::new(0.0, 1.0)
        );
    }

    // The four cardinal gravities, as (name, down) pairs.
    const CARDINALS: [(&str, Vec2); 4] = [
        ("down", Vec2::new(0.0, 1.0)),
        ("right", Vec2::new(1.0, 0.0)),
        ("up", Vec2::new(0.0, -1.0)),
        ("left", Vec2::new(-1.0, 0.0)),
    ];

    #[test]
    fn hybrid_resolve_input_matches_the_legacy_run_and_descend_at_every_orientation() {
        // Hybrid MUST stay byte-identical to the old seam (the replay guard only
        // covers normal gravity, so pin all four here). Old run: drive
        // `control_frame(Hybrid).side` by `axis_x`. Old descend: `descend(axis_y)`.
        for (name, down) in CARDINALS {
            let f = AccelerationFrame::new(down);
            for &(ax, ay) in &[
                (1.0, 0.0),
                (-0.4, 0.0),
                (0.0, 0.7),
                (0.0, -1.0),
                (0.6, -0.3),
            ] {
                let r = f.resolve_input(InputFrameMode::BodyRelativeAssist, ax, ay);
                // Run: world velocity direction must match the legacy basis * axis_x.
                let legacy_run = f.control_frame(InputFrameMode::BodyRelativeAssist).side * ax;
                let new_run = f.side * r.x;
                assert!(
                    (legacy_run - new_run).length() < 1e-6,
                    "{name}: run mismatch ax={ax} -> legacy {legacy_run:?} new {new_run:?}"
                );
                // Descend gate scalar must match `descend(axis_y)` exactly.
                assert!(
                    (f.descend(ay) - r.y).abs() < 1e-6,
                    "{name}: descend mismatch ay={ay}"
                );
            }
        }
    }

    #[test]
    fn screen_mode_is_screen_relative_at_every_orientation() {
        // In Screen mode the body moves the way the stick points ON SCREEN: the
        // world movement direction equals the raw input vector regardless of
        // gravity (to_world ∘ resolve_input == identity on the screen vector).
        for (name, down) in CARDINALS {
            let f = AccelerationFrame::new(down);
            for &(ax, ay) in &[
                (1.0, 0.0),
                (0.0, 1.0),
                (-1.0, 0.0),
                (0.0, -1.0),
                (0.5, -0.5),
            ] {
                let world = f.to_world(f.resolve_input(InputFrameMode::ScreenRelative, ax, ay));
                assert!(
                    (world - Vec2::new(ax, ay)).length() < 1e-6,
                    "{name}: screen input ({ax},{ay}) should move screen-relative, got {world:?}"
                );
            }
        }
    }

    #[test]
    fn screen_mode_matches_the_authored_quadrant_spec() {
        // The exact mapping Jon specified. Gravity RIGHT (player's feet point
        // screen-right): run = +side (screen-up), descend = +down (screen-right).
        let right = AccelerationFrame::new(Vec2::new(1.0, 0.0));
        let r = |ax, ay| right.resolve_input(InputFrameMode::ScreenRelative, ax, ay);
        assert_eq!(
            r(1.0, 0.0),
            Vec2::new(0.0, 1.0),
            "input-right -> player-down"
        );
        assert_eq!(
            r(0.0, -1.0),
            Vec2::new(1.0, 0.0),
            "input-up -> player-right"
        );
        assert_eq!(
            r(-1.0, 0.0),
            Vec2::new(0.0, -1.0),
            "input-left -> player-up"
        );
        assert_eq!(
            r(0.0, 1.0),
            Vec2::new(-1.0, 0.0),
            "input-down -> player-left"
        );

        // Gravity LEFT (feet point screen-left).
        let left = AccelerationFrame::new(Vec2::new(-1.0, 0.0));
        let l = |ax, ay| left.resolve_input(InputFrameMode::ScreenRelative, ax, ay);
        assert_eq!(
            l(-1.0, 0.0),
            Vec2::new(0.0, 1.0),
            "input-left -> player-down"
        );
        assert_eq!(
            l(0.0, 1.0),
            Vec2::new(1.0, 0.0),
            "input-down -> player-right"
        );
        assert_eq!(
            l(0.0, -1.0),
            Vec2::new(-1.0, 0.0),
            "input-up -> player-left"
        );
        assert_eq!(
            l(1.0, 0.0),
            Vec2::new(0.0, -1.0),
            "input-right -> player-up"
        );
    }

    #[test]
    fn inverse_mapping_places_local_labels_on_raw_joystick_directions() {
        let right = AccelerationFrame::new(Vec2::new(1.0, 0.0));
        assert_eq!(
            right.raw_axis_for_resolved_input(InputFrameMode::ScreenRelative, Vec2::new(0.0, 1.0)),
            Vec2::new(1.0, 0.0),
            "screen-directed: local down labels raw right when feet point screen-right"
        );
        assert_eq!(
            right.raw_axis_for_resolved_input(
                InputFrameMode::BodyRelativeAssist,
                Vec2::new(0.0, 1.0)
            ),
            Vec2::new(0.0, 1.0),
            "body-relative assist: local down stays on raw down for side gravity"
        );

        let up = AccelerationFrame::new(Vec2::new(0.0, -1.0));
        assert_eq!(
            up.raw_axis_for_resolved_input(InputFrameMode::BodyRelativeAssist, Vec2::new(0.0, 1.0)),
            Vec2::new(0.0, -1.0),
            "body-relative assist flips only when inverted"
        );
    }

    #[test]
    fn local_edge_mapping_uses_the_same_inverse_mapping() {
        let right = AccelerationFrame::new(Vec2::new(1.0, 0.0));
        let edges = RawDirectionEdges::new(false, true, false, false); // raw right edge
        let resolved = right.resolve_control(InputFrameMode::ScreenRelative, 1.0, 0.0);
        assert!(resolved.local_down_pressed(edges));
        assert!(!resolved.local_up_pressed(edges));

        let hybrid = right.resolve_control(InputFrameMode::BodyRelativeAssist, 1.0, 0.0);
        assert!(!hybrid.local_down_pressed(edges));
    }

    #[test]
    fn cardinalized_acceleration_uses_four_control_cones() {
        assert_eq!(
            AccelerationFrame::nearest_cardinal_down(Vec2::new(0.2, 0.9)),
            Vec2::new(0.0, 1.0)
        );
        assert_eq!(
            AccelerationFrame::nearest_cardinal_down(Vec2::new(0.9, 0.2)),
            Vec2::new(1.0, 0.0)
        );
        assert_eq!(
            AccelerationFrame::nearest_cardinal_down(Vec2::new(-0.8, 0.1)),
            Vec2::new(-1.0, 0.0)
        );
    }

    #[test]
    fn player_mode_is_the_raw_stick_in_the_player_frame() {
        // Player mode never accommodates: the stick IS the local body frame.
        let up = AccelerationFrame::new(Vec2::new(0.0, -1.0));
        assert_eq!(
            up.resolve_input(InputFrameMode::BodyRelativeStrict, 0.3, -0.7),
            Vec2::new(0.3, -0.7)
        );
    }

    #[test]
    fn resolve_aim_local_picks_frame_by_source_under_flipped_gravity() {
        // Upside-down gravity: feet point up (screen). The aim stick uses the AIM
        // policy, the movement stick uses the MOVEMENT policy, independently.
        let up = AccelerationFrame::new(Vec2::new(0.0, -1.0));
        let modes = ControlFrameModes {
            movement: InputFrameMode::BodyRelativeStrict, // strict body-relative locomotion
            aim: InputFrameMode::ScreenRelative,          // screen-directed precision aim
        };

        // Aim stick pushed screen-up (-y). Screen aim → world stays screen-up
        // regardless of gravity: to_world(resolve) == (0,-1).
        let aim_local = up.resolve_aim_local(modes, Vec2::new(0.0, -1.0), Vec2::ZERO, 1.0);
        assert_eq!(up.to_world(aim_local), Vec2::new(0.0, -1.0));

        // No aim, movement stick pushed screen-up (-y). Player movement → the
        // stick IS the body frame, so world = side*0 + down*(-1) = -down = (0,1).
        let move_local = up.resolve_aim_local(modes, Vec2::ZERO, Vec2::new(0.0, -1.0), 1.0);
        assert_eq!(up.to_world(move_local), Vec2::new(0.0, 1.0));

        // Neither stick → body-local facing (+x), gravity-independent in local frame.
        let facing_local = up.resolve_aim_local(modes, Vec2::ZERO, Vec2::ZERO, -1.0);
        assert_eq!(facing_local, Vec2::new(-1.0, 0.0));
    }

    #[test]
    fn control_frame_modes_default_is_screen_relative_both() {
        let d = ControlFrameModes::default();
        assert_eq!(d.movement, InputFrameMode::ScreenRelative);
        assert_eq!(d.aim, InputFrameMode::ScreenRelative);
    }

    #[test]
    fn launch_is_away_from_feet() {
        let f = AccelerationFrame::new(Vec2::new(0.0, -1.0)); // up gravity
        let mut v = Vec2::new(5.0, 0.0);
        f.launch(&mut v, 600.0);
        // Away from up-feet = screen-down (+y); perpendicular x preserved.
        assert_eq!(v, Vec2::new(5.0, 600.0));
    }
}
