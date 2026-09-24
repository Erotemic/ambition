//! The gravity-relative reference frame and the transforms between Ambition's
//! three frames.
//!
//! [`AccelerationFrame`] makes the frames and transforms explicit: being
//! gravity-aware means holding an `AccelerationFrame`, not remembering to
//! multiply by `gravity_dir`.
//!
//! - Input frame — the controller: `axis_x` right-positive, `axis_y`
//!   screen-down-positive. Raw, never rotated.
//! - Local body frame — relative to the controlled body: `+x` is the run / side
//!   axis, `+y` is *toward the feet* (the body's own "down"). Combat geometry,
//!   impulses, and gates are authored here, in the upright (normal-gravity) pose.
//! - World frame — engine coordinates (`+y` screen-down).
//!
//! Under normal gravity the local body frame equals the world frame, so every
//! transform below is the identity.

use crate::Vec2;

/// Stick magnitude above which a source counts as "engaged" for
/// [`AccelerationFrame::resolve_aim_local`]'s aim → movement → facing priority.
const STICK_SELECT_DEADZONE: f32 = 0.3;

/// Raw device/screen-frame axes: `+x` screen-right, `+y` screen-down.
///
/// The only directional form input devices may produce. It has no gameplay
/// meaning until [`AccelerationFrame::resolve_input`] (or an equivalent typed
/// seam) resolves it into [`LocalAxes`]. The movement kernel never sees one.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ScreenAxes {
    pub x: f32,
    pub y: f32,
}

impl ScreenAxes {
    pub const ZERO: Self = Self { x: 0.0, y: 0.0 };

    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub const fn from_vec(v: Vec2) -> Self {
        Self { x: v.x, y: v.y }
    }

    pub const fn vec(self) -> Vec2 {
        Vec2::new(self.x, self.y)
    }
}

/// Controlled-body-local axes: `+x` local side/right, `+y` toward the feet.
///
/// The frame of every unqualified movement verb. Produced once per controller
/// tick from raw [`ScreenAxes`] and the body's [`AccelerationFrame`]; consumed
/// by the movement kernel and by verbs that mean the body's own
/// left/right/up/down.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LocalAxes {
    pub x: f32,
    pub y: f32,
}

impl LocalAxes {
    pub const ZERO: Self = Self { x: 0.0, y: 0.0 };
    /// Unit vector along the body's own side/right axis.
    pub const X: Self = Self { x: 1.0, y: 0.0 };
    /// Unit vector toward the body's feet.
    pub const Y: Self = Self { x: 0.0, y: 1.0 };

    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub const fn from_vec(v: Vec2) -> Self {
        Self { x: v.x, y: v.y }
    }

    pub const fn vec(self) -> Vec2 {
        Vec2::new(self.x, self.y)
    }

    /// Magnitude: a throttle, when this is a locomotion intent.
    pub fn length(self) -> f32 {
        self.vec().length()
    }

    pub fn normalize_or_zero(self) -> Self {
        Self::from_vec(self.vec().normalize_or_zero())
    }

    pub fn clamp_length_max(self, max: f32) -> Self {
        Self::from_vec(self.vec().clamp_length_max(max))
    }
}

// Scaling, adding, and negating cannot change a vector's frame, so these
// operators are on the typed value. A frame change must go through
// `to_world` / `to_local`. There is no `From<Vec2>` / `Into<Vec2>` in either
// direction.
impl std::ops::Mul<f32> for LocalAxes {
    type Output = Self;
    fn mul(self, scale: f32) -> Self {
        Self::new(self.x * scale, self.y * scale)
    }
}

impl std::ops::MulAssign<f32> for LocalAxes {
    fn mul_assign(&mut self, scale: f32) {
        self.x *= scale;
        self.y *= scale;
    }
}

impl std::ops::Div<f32> for LocalAxes {
    type Output = Self;
    fn div(self, scale: f32) -> Self {
        Self::new(self.x / scale, self.y / scale)
    }
}

impl std::ops::Neg for LocalAxes {
    type Output = Self;
    fn neg(self) -> Self {
        Self::new(-self.x, -self.y)
    }
}

impl std::ops::Add for LocalAxes {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self::new(self.x + rhs.x, self.y + rhs.y)
    }
}

impl std::ops::Sub for LocalAxes {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self::new(self.x - rhs.x, self.y - rhs.y)
    }
}

/// A world-space direction or step whose frame resolution already happened.
///
/// Scripted directions, impulses, and seam-resolved aim vectors cross the
/// movement boundary in this form, so a world quantity cannot be mistaken for
/// a screen or body-local one.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct WorldVec2(pub Vec2);

impl WorldVec2 {
    pub const ZERO: Self = Self(Vec2::ZERO);

    pub const fn new(x: f32, y: f32) -> Self {
        Self(Vec2::new(x, y))
    }

    pub const fn vec(self) -> Vec2 {
        self.0
    }
}

// The same frame-preserving set as [`LocalAxes`]. `Deref<Target = Vec2>`
// gives the read-only geometry, so only operators that must return a
// `WorldVec2` are written here.
impl std::ops::Mul<f32> for WorldVec2 {
    type Output = Self;
    fn mul(self, scale: f32) -> Self {
        Self(self.0 * scale)
    }
}

impl std::ops::MulAssign<f32> for WorldVec2 {
    fn mul_assign(&mut self, scale: f32) {
        self.0 *= scale;
    }
}

impl std::ops::Neg for WorldVec2 {
    type Output = Self;
    fn neg(self) -> Self {
        Self(-self.0)
    }
}

impl std::ops::Add for WorldVec2 {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self(self.0 + rhs.0)
    }
}

impl std::ops::Sub for WorldVec2 {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self(self.0 - rhs.0)
    }
}

impl std::ops::Deref for WorldVec2 {
    type Target = Vec2;
    fn deref(&self) -> &Vec2 {
        &self.0
    }
}

/// How the raw input frame maps onto the controlled body's local frame: which
/// way is right when gravity is sideways or inverted. A human-control
/// preference (see [`AccelerationFrame::control_frame`]).
///
/// Not `Default`: the default depends on the input source. See
/// [`Self::DEFAULT_MOVEMENT`] / [`Self::DEFAULT_AIM`], which
/// [`ControlFrameModes::default`] and every settings fallback use.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum InputFrameMode {
    /// Input is always screen-aligned: right is screen-right at any gravity.
    ScreenRelative,
    /// Input follows the controlled body's local frame: right is the body's
    /// own right, fully rotated with gravity.
    BodyRelativeStrict,
    /// Hybrid: follow the body frame up to ±90° from screen-down (gravity
    /// down, left, right), then revert to screen-aligned past 90° (gravity
    /// up-ish), where the flip is hard to track. The descend gate (pogo,
    /// crouch) always flips with the body frame ([`Self::descend`]).
    BodyRelativeAssist,
}

impl InputFrameMode {
    /// The default for locomotion input. Every movement-stick default resolves
    /// here, directly or via [`ControlFrameModes::default`]. A `const` so
    /// [`crate::movement::DEFAULT_TUNING`] can use it.
    pub const DEFAULT_MOVEMENT: Self = Self::ScreenRelative;
    /// The default for precision-aim input (blink steer, ranged aim): point
    /// where the stick points on screen at any gravity.
    pub const DEFAULT_AIM: Self = Self::ScreenRelative;

    /// The mode that actually applies once the observing view's frame is known.
    ///
    /// Under [`CameraReferenceFrame::SubjectFrame`] every mode becomes
    /// [`Self::BodyRelativeStrict`]. A subject-frame view rolls until
    /// screen-down is the body's `down` and screen-right is its `side`, so
    /// `ScreenRelative` through [`AccelerationFrame::resolve_input`] gives
    /// `(sx, sy)` back (the basis is orthonormal), the same as
    /// `BodyRelativeStrict`. `BodyRelativeAssist` collapses because a body that
    /// never appears flipped needs no accommodation.
    ///
    /// So do not read `ScreenRelative` raw when a view can roll: `side`/`down`
    /// are in world coordinates, and "screen" would silently mean "world".
    ///
    /// Takes the view's policy, never its rotation. The presented roll is
    /// eased and not rollback state; the policy is a settings-derived enum.
    pub fn under_camera(self, camera: CameraReferenceFrame) -> Self {
        match camera {
            CameraReferenceFrame::WorldFixed => self,
            CameraReferenceFrame::SubjectFrame => Self::BodyRelativeStrict,
        }
    }
}

/// Which frame a view presents the world in.
///
/// A presentation policy only: gravity, collision, and body integration are
/// the same whichever frame observes them. It belongs to a view, so indexed
/// views will each carry one.
///
/// It sits beside [`InputFrameMode`] because both answer "which frame is this
/// human operating in?", and [`InputFrameMode::under_camera`] keeps the
/// answers consistent.
#[derive(
    bevy_ecs::prelude::Component,
    Clone,
    Copy,
    Debug,
    Default,
    PartialEq,
    Eq,
    serde::Serialize,
    serde::Deserialize,
)]
pub enum CameraReferenceFrame {
    /// Screen orientation stays tied to the world frame, even under sideways
    /// or inverted gravity.
    #[default]
    WorldFixed,
    /// Screen orientation follows the view subject's resolved body frame, so a
    /// gravity change presents as the world rotating around an upright body.
    ///
    /// The subject is the view's subject, not a protagonist. The resolver
    /// takes a direction, not an entity, so a spectator, a replay, or a second
    /// local view can orient on any body.
    SubjectFrame,
}

/// The pair of [`InputFrameMode`] policies a control authority maps raw input
/// through, split by input source, not by actor.
///
/// The locomotion stick and the precision-aim stick are different sources,
/// and a human tracks them differently under rotated gravity, so each has its
/// own policy. Both default to [`InputFrameMode::ScreenRelative`]. See
/// [`InputFrameMode::DEFAULT_MOVEMENT`] / [`InputFrameMode::DEFAULT_AIM`].
///
/// A control-authority preference, not a property of one actor.
/// [`AccelerationFrame::resolve_aim_local`] uses it for verbs that pick a
/// direction by source priority (aim, then move, then facing).
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ControlFrameModes {
    /// How the locomotion stick maps onto the body's local frame.
    pub movement: InputFrameMode,
    /// How the precision-aim stick maps onto the body's local frame.
    pub aim: InputFrameMode,
}

impl Default for ControlFrameModes {
    /// Both default to screen-directed, from [`InputFrameMode`]'s per-source
    /// defaults.
    fn default() -> Self {
        Self {
            movement: InputFrameMode::DEFAULT_MOVEMENT,
            aim: InputFrameMode::DEFAULT_AIM,
        }
    }
}

/// Raw digital direction edges in the input/screen frame.
///
/// Separate from the analog axis: an axis can be held for many frames, but
/// double-tap and interact gestures need the frame a cardinal became active.
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
/// The reference-frame seam: input systems supply raw screen axes, and
/// gameplay verbs use `local_axis` for the body's unqualified
/// left/right/up/down.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ResolvedControlFrame {
    /// Raw input/screen-frame stick.
    pub raw_axes: ScreenAxes,
    /// Controlled-body-local stick.
    pub local_axes: LocalAxes,
    pub mode: InputFrameMode,
    pub frame: AccelerationFrame,
}

impl ResolvedControlFrame {
    pub fn local_down_pressed(self, edges: RawDirectionEdges) -> bool {
        self.frame
            .local_direction_pressed(self.mode, LocalAxes::new(0.0, 1.0), edges)
    }

    pub fn local_up_pressed(self, edges: RawDirectionEdges) -> bool {
        self.frame
            .local_direction_pressed(self.mode, LocalAxes::new(0.0, -1.0), edges)
    }

    pub fn local_right_pressed(self, edges: RawDirectionEdges) -> bool {
        self.frame
            .local_direction_pressed(self.mode, LocalAxes::new(1.0, 0.0), edges)
    }

    pub fn local_left_pressed(self, edges: RawDirectionEdges) -> bool {
        self.frame
            .local_direction_pressed(self.mode, LocalAxes::new(-1.0, 0.0), edges)
    }
}

/// The controlled body's local reference basis (named for acceleration).
///
/// Gravity usually supplies `down`, but orientation is a separate fact: zero
/// acceleration can keep this basis, and lateral acceleration need not rotate
/// it. The direction is not snapped to a cardinal, so off-axis `down` works;
/// the gravity system feeds cardinals today.
///
/// `down` (toward the feet, unit) and `side` (the perpendicular run axis) are
/// the local basis in world coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AccelerationFrame {
    /// Toward the feet (unit): the player's own "down". `(0,1)` under normal
    /// gravity.
    pub down: Vec2,
    /// The run / side axis (perpendicular to `down`). `(1,0)` under normal gravity.
    pub side: Vec2,
}

/// The complete per-body acceleration frame used by the movement kernel.
///
/// Pairs the [`AccelerationFrame`] basis with separate gravity/orienting and
/// external world-space acceleration, which stay independent even when
/// ordinary gravity aligns them. Build it once per body tick and pass the
/// same value to input interpretation and the active movement policy.
///
/// Runtime environment state, never movement-model configuration, so
/// swapping physics policies cannot reset the body's current frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MotionFrame {
    gravity_acceleration: Vec2,
    external_acceleration: Vec2,
    basis: AccelerationFrame,
}

impl MotionFrame {
    /// Build a frame from an explicit basis and one orienting acceleration.
    ///
    /// The acceleration is the gravity/orienting part, with no external part.
    /// Environment resolution should use [`Self::with_accelerations`], so jump
    /// laws can scale gravity without scaling wind, tractor fields, or
    /// inertial acceleration.
    pub const fn new(basis: AccelerationFrame, gravity_acceleration: Vec2) -> Self {
        Self::with_accelerations(basis, gravity_acceleration, Vec2::ZERO)
    }

    /// Build a frame while preserving the environment's acceleration
    /// decomposition.
    ///
    /// `gravity_acceleration` defines the body-relative gravity response and is
    /// the only contribution a phased jump may scale. `external_acceleration`
    /// remains world-space and unscaled. The basis stays authoritative even when
    /// either contribution is zero.
    pub const fn with_accelerations(
        basis: AccelerationFrame,
        gravity_acceleration: Vec2,
        external_acceleration: Vec2,
    ) -> Self {
        Self {
            gravity_acceleration,
            external_acceleration,
            basis,
        }
    }

    /// Derive both orientation and force from a non-zero world-space
    /// acceleration. Returns `None` for zero because a zero vector cannot define
    /// a reference-frame orientation.
    pub fn from_acceleration(acceleration: Vec2) -> Option<Self> {
        let down = acceleration.try_normalize()?;
        Some(Self::new(AccelerationFrame::new(down), acceleration))
    }

    /// Compose an environment-supplied down direction with a per-body response
    /// magnitude. This remains well-defined at zero magnitude because the
    /// direction explicitly carries the reference-frame orientation.
    pub fn from_direction(direction: Vec2, magnitude: f32) -> Self {
        let down = direction.try_normalize().unwrap_or(Vec2::new(0.0, 1.0));
        Self::new(AccelerationFrame::new(down), down * magnitude.max(0.0))
    }

    /// Full world-space acceleration applied this tick.
    pub fn acceleration(self) -> Vec2 {
        self.gravity_acceleration + self.external_acceleration
    }

    /// The gravity/orienting contribution. Jump-arc laws may scale this term.
    pub const fn gravity_acceleration(self) -> Vec2 {
        self.gravity_acceleration
    }

    /// Non-orienting world-space acceleration. Jump-arc laws must not scale it.
    pub const fn external_acceleration(self) -> Vec2 {
        self.external_acceleration
    }

    /// Magnitude of the complete acceleration in world units per second squared.
    pub fn magnitude(self) -> f32 {
        self.acceleration().length()
    }

    /// The acceleration-relative body basis.
    pub const fn basis(self) -> AccelerationFrame {
        self.basis
    }

    /// Toward the feet in world coordinates.
    pub const fn down(self) -> Vec2 {
        self.basis.down
    }

    /// Local side/right in world coordinates.
    pub const fn side(self) -> Vec2 {
        self.basis.side
    }

    /// Convert a body-local vector into world coordinates.
    pub fn to_world(self, local: Vec2) -> Vec2 {
        self.basis.to_world(local)
    }

    /// Convert a world vector into body-local coordinates.
    pub fn to_local(self, world: Vec2) -> Vec2 {
        self.basis.to_local(world)
    }
}

/// Declares the frame a gameplay quantity is authored or interpreted in.
///
/// A code-level contract, not content metadata. It lets tests and call sites
/// tell body-local verbs from screen/HUD input and world geometry without
/// authored surface labels.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GameplayFramePolicy {
    /// Local to the controlled body: `x` is side/right and `y` is toward feet.
    ControlledBodyLocal,
    /// Relative to the current acceleration frame, usually equivalent to
    /// controlled-body-local for movement/contact mechanics.
    AccelerationFrame,
    /// Room geometry, scripted world hazards, and other effects that do not
    /// rotate with a body.
    WorldSpace,
    /// Raw display/input space. Stays at the input seam and is converted
    /// before gameplay resolution.
    ScreenSpace,
}

impl AccelerationFrame {
    /// Build the frame from the net down-defining acceleration (usually
    /// gravity). Normalized but not cardinal-snapped. The side axis is `down`
    /// rotated −90°, so normal gravity gives `down=(0,1)`, `side=(1,0)`, and
    /// identity transforms. Zero acceleration gives normal-gravity down.
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
    /// Digital controls and glyph labels snap to the four screen directions.
    /// Keeping the snap here makes the four-cones rule a shared policy, not a
    /// per-mechanic special case.
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

    /// The frame the input stick maps through, per [`InputFrameMode`].
    /// `to_world(stick)` on the result turns raw `(axis_x, axis_y)` into a
    /// world movement direction. Screen gives identity; body-strict gives this
    /// frame; assist gives this frame up to ±90° (down.y ≥ 0), else
    /// screen-aligned. This drives free movement (run, flight); the
    /// toward-feet gate uses [`Self::descend`] so pogo and crouch always flip.
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

    /// Input to body: screen-vertical input (`axis_y`, +Y down) to the descend
    /// intent that gates crouch, pogo, drop-through, and fast-fall. The gate
    /// stays on the up/down keys and flips sign only past ±90° from
    /// screen-down. Identity under normal gravity.
    ///
    /// Equal to the `y` of [`Self::resolve_input`] in
    /// [`InputFrameMode::BodyRelativeAssist`]. Prefer `resolve_input` at the
    /// input seam, so the run axis and descend gate use the same mode.
    pub fn descend(self, input_axis_y: f32) -> f32 {
        input_axis_y * if self.down.y < 0.0 { -1.0 } else { 1.0 }
    }

    /// Input to body, both axes. Resolve the raw stick `(axis_x, axis_y)`
    /// into a local stick (`x` = run along [`Self::side`], `y` = descend along
    /// [`Self::down`]) per [`InputFrameMode`]. [`Self::to_world`] on the result
    /// gives the world direction.
    ///
    /// - [`InputFrameMode::BodyRelativeStrict`]: the stick is already the
    ///   local frame, fully rotated with gravity.
    /// - [`InputFrameMode::ScreenRelative`]: project the screen stick onto the
    ///   body basis, so the body moves the way the stick points on screen.
    ///   Under sideways gravity the run and descend roles swap.
    /// - [`InputFrameMode::BodyRelativeAssist`]: equals `BodyRelativeStrict`
    ///   up to ±90° from screen-down, then inverts both axes past 90°. Matches
    ///   the `axis_x` run plus [`Self::descend`] gate at every orientation.
    pub fn resolve_input(self, mode: InputFrameMode, axes: ScreenAxes) -> LocalAxes {
        match mode {
            InputFrameMode::BodyRelativeStrict => LocalAxes::new(axes.x, axes.y),
            InputFrameMode::ScreenRelative => {
                let input = axes.vec();
                LocalAxes::new(input.dot(self.side), input.dot(self.down))
            }
            InputFrameMode::BodyRelativeAssist => {
                let s = if self.down.y < 0.0 { -1.0 } else { 1.0 };
                LocalAxes::new(axes.x * s, axes.y * s)
            }
        }
    }

    /// Resolve a direction-picking verb (blink target, grapple/dive, held-shot
    /// aim) into the body's local frame, picking the policy by input source
    /// ([`ControlFrameModes`]):
    ///
    /// - aim stick engaged: precision aim through `modes.aim`;
    /// - else movement stick engaged: through `modes.movement`;
    /// - else body-local facing (`+x`).
    ///
    /// `aim` and `movement` are raw input sticks (`+x` right, `+y` down);
    /// `facing` is the body's screen facing sign. The result is unit length
    /// (or the facing fallback). [`Self::to_world`] lifts it to world space.
    pub fn resolve_aim_local(
        self,
        modes: ControlFrameModes,
        aim: ScreenAxes,
        movement: ScreenAxes,
        facing: f32,
    ) -> LocalAxes {
        if aim.vec().length() > STICK_SELECT_DEADZONE {
            let local = self.resolve_input(modes.aim, aim);
            return LocalAxes::from_vec(local.vec().normalize_or_zero());
        }
        if movement.vec().length() > STICK_SELECT_DEADZONE {
            let local = self.resolve_input(modes.movement, movement);
            return LocalAxes::from_vec(local.vec().normalize_or_zero());
        }
        LocalAxes::new(if facing >= 0.0 { 1.0 } else { -1.0 }, 0.0)
    }

    /// Resolve a raw stick into the body's local frame and keep both forms
    /// together, for consumers that must name their frame.
    pub fn resolve_control(self, mode: InputFrameMode, axes: ScreenAxes) -> ResolvedControlFrame {
        ResolvedControlFrame {
            raw_axes: axes,
            local_axes: self.resolve_input(mode, axes),
            mode,
            frame: self,
        }
    }

    /// Inverse of [`Self::resolve_input`] for a local/body-frame axis.
    ///
    /// Used for touch-glyph placement: for a local command (`D`, `U`, `L`,
    /// `R`), find the raw joystick direction to label with it.
    pub fn raw_axis_for_resolved_input(self, mode: InputFrameMode, local: LocalAxes) -> ScreenAxes {
        match mode {
            InputFrameMode::BodyRelativeStrict => ScreenAxes::new(local.x, local.y),
            InputFrameMode::ScreenRelative => ScreenAxes::from_vec(self.to_world(local.vec())),
            InputFrameMode::BodyRelativeAssist => {
                let s = if self.down.y < 0.0 { -1.0 } else { 1.0 };
                ScreenAxes::new(local.x * s, local.y * s)
            }
        }
    }

    /// Test whether a raw cardinal edge corresponds to the given local/body
    /// direction under this input mapping.
    pub fn local_direction_pressed(
        self,
        mode: InputFrameMode,
        local: LocalAxes,
        edges: RawDirectionEdges,
    ) -> bool {
        edges.pressed_for_raw_axis(self.raw_axis_for_resolved_input(mode, local).vec())
    }

    /// Local body to world. Rotate a local vector (`+y` toward the feet) into
    /// world coordinates. Identity under normal gravity.
    pub fn to_world(self, player: Vec2) -> Vec2 {
        self.side * player.x + self.down * player.y
    }

    /// World to local body: `x` is side/right, `y` is toward feet.
    pub fn to_local(self, world: Vec2) -> Vec2 {
        Vec2::new(world.dot(self.side), world.dot(self.down))
    }

    /// Local body to world for an axis-aligned half-extent. Returns the world
    /// AABB half-extent that bounds the rotated box: exact for cardinal frames
    /// (90° swaps width and height), a bound for off-axis frames. Identity
    /// under normal and inverted gravity.
    pub fn to_world_half(self, half: Vec2) -> Vec2 {
        Vec2::new(
            (self.side.x * half.x).abs() + (self.down.x * half.y).abs(),
            (self.side.y * half.x).abs() + (self.down.y * half.y).abs(),
        )
    }

    /// Set `vel` to a launch of `speed` away from the feet (jump, pogo bounce),
    /// keeping the component perpendicular to gravity.
    pub fn launch(self, vel: &mut Vec2, speed: f32) {
        let perp = *vel - vel.dot(self.down) * self.down;
        *vel = perp - speed * self.down;
    }

    /// The component of `vel` directed toward the feet (its descent speed).
    pub fn descend_speed(self, vel: Vec2) -> f32 {
        vel.dot(self.down)
    }

    pub fn ensure_descend_speed(self, vel: &mut Vec2, speed: f32) {
        let cur = vel.dot(self.down);
        if cur < speed {
            *vel += self.down * (speed - cur);
        }
    }
}

#[cfg(test)]
mod tests;
