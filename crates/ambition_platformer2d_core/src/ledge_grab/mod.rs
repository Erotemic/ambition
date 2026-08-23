//! Ledge grab probe, state, and movement-pipeline tick helpers.
//!
//! The probe answers: "is there a ledge corner I can snap onto, and
//! where is the hang / pull-up path?" The state machine is engine-owned so
//! ledge grab participates in the same movement tick as gravity, collision,
//! water, and wall state instead of running as a post-update sandbox mutator.

#![allow(unused_imports)]
use crate::geometry::{Aabb, AabbExt};
use crate::movement::{AxisSweptParams, InputState, MovementOp, MovementTuning};
use crate::world::{BlockKind, World};
use crate::Vec2;

/// Duration of the standard ledge pull-up transition.
pub const LEDGE_CLIMB_TIME: f32 = 0.24;

/// Duration of a Smash-Bros-style ledge ROLL: shorter than the climb
/// because the player commits horizontally and lands further inboard.
/// The whole window grants invulnerability via the existing dodge-roll
/// timer.
pub const LEDGE_ROLL_TIME: f32 = 0.30;

/// Duration of a getup-attack (Smash-style "ledge attack"). The
/// player lifts to the platform on the same curve as the climb but
/// swings during the lift; the active hitbox fires at the start and
/// the player has invuln frames via `AxisManeuverState::ledge_invuln_timer` for
/// the duration. Tuned slightly longer than a plain climb to give the
/// swing time to read.
pub const LEDGE_GETUP_ATTACK_TIME: f32 = 0.30;

/// How much of a ledge getup ATTACK is intangible.
///
/// ⚠ THE WHOLE ATTACK, which is a design choice this constant exists to make
/// VISIBLE rather than to settle. It was inherited rather than decided: the
/// getup attack borrowed the dodge roll's timer for its full duration, so an
/// attack thrown from the edge was untouchable through its own recovery — free,
/// and the one ledge option with no punish. Ultimate covers roughly the first
/// half of its ledge attack and leaves the tail exposed, which is what makes
/// contesting the edge a read.
///
/// Left at the full duration so this split changes NOTHING about what refuses
/// damage. Shortening it is a one-line tuning decision now that there is a line
/// to shorten.
pub const LEDGE_GETUP_ATTACK_INVULN: f32 = LEDGE_GETUP_ATTACK_TIME;

/// How much further inboard the roll lands than the climb. The roll
/// target is `climb_target + into_axis * LEDGE_ROLL_OVERSHOOT`,
/// chosen so the roll covers ~1 player width past the platform edge
/// — enough that the player visibly tumbles past the lip, like the
/// "ledge roll" option in Smash Bros.
pub const LEDGE_ROLL_OVERSHOOT: f32 = 36.0;

/// Require a tiny hang beat before held horizontal input into the platform
/// auto-starts the climb.
pub const LEDGE_TOWARD_CLIMB_DELAY: f32 = 0.045;

/// Minimum hang time before any climb/roll input can fire. Tightened
/// from 0.16 to 0.06 (≈ one frame at 60Hz of debounce) for Smash-Bros
/// snap-and-act feel — the original 160 ms felt mushy because most of
/// the time the player is grabbing INTENTIONALLY and wants to act
/// immediately.
pub const LEDGE_MIN_CLIMB_DELAY: f32 = 0.06;

/// Intangibility window granted at the moment the player grabs a
/// ledge. Mirrors Smash's "ledge intangibility" so a grab can't be
/// punished by edge-guards on contact. Plumbed through
/// `AxisManeuverState::ledge_invuln_timer`, which is the ledge's own window and
/// engine's "invuln while rolling" gate; reusing it keeps the damage
/// pipeline single-source.
pub const LEDGE_GRAB_INVULN_TIME: f32 = 0.50;

/// How long a body must spend OFF a ledge to earn the full window.
///
/// A counter would punish the recovery as hard as the stall; the clock only punishes the stall.
///
/// the CURVE is rough and the numbers are placeholders. What is confirmed
/// about the genre is the shape — airtime buys the window — not the constants.
pub const LEDGE_INVULN_FULL_AIRTIME: f32 = 1.20;

/// The window a body earns however fast it re-caught the edge. Not zero: a grab
/// that granted nothing at all would make the ledge itself a punish, and the
/// re-grab cooldown ([`LEDGE_REGRAB_COOLDOWN`]) is what actually forbids the
/// instant stall.
pub const LEDGE_INVULN_MIN_TIME: f32 = 0.10;

/// What this grab's intangibility is worth, given how long the body has been
/// off a ledge. Linear between [`LEDGE_INVULN_MIN_TIME`] and
/// [`LEDGE_GRAB_INVULN_TIME`].
pub fn ledge_grab_invuln_earned(time_off_ledge: f32) -> f32 {
    let t = (time_off_ledge / LEDGE_INVULN_FULL_AIRTIME).clamp(0.0, 1.0);
    LEDGE_INVULN_MIN_TIME + (LEDGE_GRAB_INVULN_TIME - LEDGE_INVULN_MIN_TIME) * t
}

/// Cooldown blocking a fresh ledge grab right after the player
/// voluntarily released a ledge (drop / ledge-jump / ledge-release).
/// At typical gravity (~1500 px/s²) a player accelerating from rest
/// clears the chin-band in about 200 ms; pad to 250 ms so the same
/// lip can't re-snap on the very next fall sample, and also so the
/// player gets a clear "I'm dropping" beat before any auto-snap can
/// re-engage. Tune up for stickier feel, down for snappier recovery.
pub const LEDGE_REGRAB_COOLDOWN: f32 = 0.25;

/// How far above the player's head a ledge top can be and still count
/// as reachable. This is intentionally more generous than the old
/// chin-band so a slightly low jump can still catch the lip.
pub const LEDGE_REACH_UP: f32 = 28.0;

/// How far below the player's head a ledge top can be and still count
/// as reachable. This covers fast descents and frame-to-frame motion
/// where the head has already dipped past the lip before the probe runs.
pub const LEDGE_REACH_DOWN: f32 = 30.0;

/// Horizontal magnet distance from the player's reaching side to the
/// ledge face. The old 4px tolerance only worked after exact wall
/// contact; 10px catches near-misses without pulling from across gaps.
pub const LEDGE_HORIZONTAL_REACH: f32 = 10.0;

/// Horizontal input threshold used only to request an airborne ledge
/// probe. Hanging/getup choices keep their stronger 0.4 dead-zone below
/// so climbing/dropping from a ledge does not become accidental.
pub const LEDGE_GRAB_INTENT_DEADZONE: f32 = 0.25;

/// The original, tighter vertical reach above the player's head. A grab
/// inside this old window is considered "precise" and keeps the
/// momentum-carry getup reward.
pub const LEDGE_PRECISE_REACH_UP: f32 = 12.0;

/// The original, tighter vertical reach below the player's head. The
/// outer forgiving band still catches the player, but does not earn the
/// boost unless the lip was also inside this precise band.
pub const LEDGE_PRECISE_REACH_DOWN: f32 = 18.0;

/// The original wall-face tolerance. Grabs within this horizontal band
/// are precise; the wider `LEDGE_HORIZONTAL_REACH` is only a safety net.
pub const LEDGE_PRECISE_HORIZONTAL_REACH: f32 = 4.0;

/// What surface, and where, does the probe accept a ledge grab?
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LedgeContact {
    /// X-direction the wall pushes the player toward (+1 = wall on
    /// player's left, ‑1 = wall on player's right).
    pub wall_normal_x: f32,
    /// World position the player should snap to (their center while
    /// hanging on the ledge). The vertical pos is the top of the
    /// ledge; the horizontal pos hugs the wall edge.
    pub anchor: Vec2,
    /// Top-left of the platform the player would climb up onto. Used
    /// by the climb animation so the player can ease toward this
    /// point on Up + Jump.
    pub climb_target: Vec2,
}

/// Which getup the player chose when leaving the hang. The state
/// machine interpolates position differently for each variant, and
/// the sandbox HUD reads this to label the action ("Climb" / "Roll")
/// at the bottom of the screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LedgeGetupKind {
    /// Standard pull-up: short arc from anchor to `climb_target`.
    Climb,
    /// Smash-Bros style ledge roll: faster, covers more ground past
    /// the platform edge, and grants invulnerability for the whole
    /// duration via `AxisManeuverState::ledge_invuln_timer`.
    Roll,
    /// Smash-Bros style ledge getup attack: the player swings onto the
    /// platform, attacking on the way up. Movement follows the same
    /// curve as `Climb`; the slash hitbox is fired at the start and
    /// the player is invulnerable for the duration. Reuses the
    /// regular player attack animation for now — TODO: author a
    /// dedicated getup-attack sprite/animation so the swing reads
    /// distinctly from a normal slash.
    Attack,
}

/// Whether a grab was earned inside the original tight ledge probe or
/// recovered through the wider forgiveness band.
///
/// The catch itself is valid either way, but only [`LedgeGrabQuality::Precise`]
/// gets momentum-carry and fast-getup rewards. This keeps accessibility from
/// erasing the skill reward for clean, on-window grabs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LedgeGrabQuality {
    /// The ledge face and top lip were both inside the original tight
    /// chin/face probe window.
    Precise,
    Forgiving,
}

impl LedgeGrabQuality {
    pub fn is_precise(self) -> bool {
        matches!(self, Self::Precise)
    }
}

/// Engine-owned ledge-grab state for the player.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LedgeGrabState {
    pub contact: LedgeContact,
    /// Seconds since the cling-snap fired. Used for input affordances such as
    /// giving held-into-wall input a tiny beat before it auto-starts the climb.
    pub elapsed: f32,
    /// True once a getup (climb or roll) has been requested. While
    /// true, the movement tick interpolates the player along the
    /// chosen getup curve.
    pub climbing: bool,
    /// Which getup is in progress. Only meaningful while
    /// ``climbing``; ignored while hanging.
    pub getup_kind: LedgeGetupKind,
    /// Seconds spent in the pull-up / roll transition.
    pub climb_elapsed: f32,
    pub momentum_at_grab: Vec2,
    pub grab_quality: LedgeGrabQuality,
}

impl LedgeGrabState {
    pub fn hanging(contact: LedgeContact) -> Self {
        Self {
            contact,
            elapsed: 0.0,
            climbing: false,
            getup_kind: LedgeGetupKind::Climb,
            climb_elapsed: 0.0,
            momentum_at_grab: Vec2::ZERO,
            // Directly constructed/test states model the old precise grab
            // unless the real latch path overwrites this from geometry.
            grab_quality: LedgeGrabQuality::Precise,
        }
    }

    /// Hanging state with an explicit grab-quality classification.
    pub fn hanging_with_quality(contact: LedgeContact, grab_quality: LedgeGrabQuality) -> Self {
        Self {
            grab_quality,
            ..Self::hanging(contact)
        }
    }

    /// Compatibility helper for call sites/tests that classify the grab as a
    /// boolean. Prefer [`LedgeGrabQuality`] in new code so the precision reward
    /// remains visible in type signatures.
    pub fn hanging_with_precision(contact: LedgeContact, precise_grab: bool) -> Self {
        Self::hanging_with_quality(
            contact,
            if precise_grab {
                LedgeGrabQuality::Precise
            } else {
                LedgeGrabQuality::Forgiving
            },
        )
    }

    /// Convenience constructor for tests: hanging-on-ledge state with
    /// a specific incoming-momentum vector, so boost-eligible getup
    /// paths can be exercised without spelling out every field.
    #[cfg(test)]
    pub fn hanging_with_momentum(contact: LedgeContact, momentum: Vec2) -> Self {
        Self {
            momentum_at_grab: momentum,
            ..Self::hanging(contact)
        }
    }

    /// Duration of the active getup at this state's `getup_kind`.
    /// Returns 0 if not currently in a getup.
    pub fn getup_duration(self) -> f32 {
        match self.getup_kind {
            LedgeGetupKind::Climb => LEDGE_CLIMB_TIME,
            LedgeGetupKind::Roll => LEDGE_ROLL_TIME,
            LedgeGetupKind::Attack => LEDGE_GETUP_ATTACK_TIME,
        }
    }
}

fn smoothstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn quadratic_bezier(start: Vec2, control: Vec2, end: Vec2, t: f32) -> Vec2 {
    let t = t.clamp(0.0, 1.0);
    let one_t = 1.0 - t;
    start * (one_t * one_t) + control * (2.0 * one_t * t) + end * (t * t)
}

fn into_platform_world_axis_in_frame(contact: LedgeContact, gravity_dir: Vec2) -> Vec2 {
    let frame = crate::AccelerationFrame::new(gravity_dir);
    frame.side * into_platform_axis(contact)
}

fn climb_control_point_in_frame(contact: LedgeContact, target: Vec2, gravity_dir: Vec2) -> Vec2 {
    // Smash-style curved climb: first move away from the feet along the ledge
    // wall, then arc inboard onto the platform. This must be authored in the
    // controlled body's acceleration frame: the inboard component is local side,
    // while the lift component is local up/away-from-feet. Do not infer the
    // inboard axis from the larger world-space delta; for a normal ledge the
    // vertical lift can be longer than the horizontal step, and for sideways
    // gravity either component may dominate.
    let into = into_platform_world_axis_in_frame(contact, gravity_dir);
    let delta = target - contact.anchor;
    let inboard = into * delta.dot(into);
    contact.anchor + (delta - inboard)
}

fn climb_position_in_frame(contact: LedgeContact, progress: f32, gravity_dir: Vec2) -> Vec2 {
    let t = smoothstep(progress);
    let control = climb_control_point_in_frame(contact, contact.climb_target, gravity_dir);
    quadratic_bezier(contact.anchor, control, contact.climb_target, t)
}

#[cfg(test)]
fn climb_position(contact: LedgeContact, progress: f32) -> Vec2 {
    climb_position_in_frame(contact, progress, Vec2::new(0.0, 1.0))
}

/// Roll target: ``climb_target`` plus an extra ``LEDGE_ROLL_OVERSHOOT``
/// along the into-platform axis, so the player lands a body-width
/// past the lip rather than right at the edge.
fn roll_target_in_frame(contact: LedgeContact, gravity_dir: Vec2) -> Vec2 {
    contact.climb_target
        + into_platform_world_axis_in_frame(contact, gravity_dir) * LEDGE_ROLL_OVERSHOOT
}

#[cfg(test)]
fn roll_target(contact: LedgeContact) -> Vec2 {
    roll_target_in_frame(contact, Vec2::new(0.0, 1.0))
}

fn roll_position_in_frame(contact: LedgeContact, progress: f32, gravity_dir: Vec2) -> Vec2 {
    // Roll should not read as a diagonal drift. Use the same
    // "rise onto the platform, then sweep inboard" arc as climb, but
    // keep the roll's fast-commit timing by mirroring the easing so
    // the horizontal commitment starts immediately and settles
    // smoothly into the landing.
    let t = 1.0 - smoothstep(1.0 - progress.clamp(0.0, 1.0));
    let target = roll_target_in_frame(contact, gravity_dir);
    let control = climb_control_point_in_frame(contact, target, gravity_dir);
    quadratic_bezier(contact.anchor, control, target, t)
}

#[cfg(test)]
fn roll_position(contact: LedgeContact, progress: f32) -> Vec2 {
    roll_position_in_frame(contact, progress, Vec2::new(0.0, 1.0))
}

fn getup_position(state: LedgeGrabState, progress: f32, gravity_dir: Vec2) -> Vec2 {
    match state.getup_kind {
        LedgeGetupKind::Climb => climb_position_in_frame(state.contact, progress, gravity_dir),
        LedgeGetupKind::Roll => roll_position_in_frame(state.contact, progress, gravity_dir),
        // Attack uses the same arc as Climb — only the timing,
        // invuln, and triggered slash differ.
        LedgeGetupKind::Attack => climb_position_in_frame(state.contact, progress, gravity_dir),
    }
}

fn getup_end_position(state: LedgeGrabState, gravity_dir: Vec2) -> Vec2 {
    match state.getup_kind {
        LedgeGetupKind::Climb => state.contact.climb_target,
        LedgeGetupKind::Roll => roll_target_in_frame(state.contact, gravity_dir),
        LedgeGetupKind::Attack => state.contact.climb_target,
    }
}

pub fn into_platform_axis(contact: LedgeContact) -> f32 {
    -contact.wall_normal_x
}

pub fn away_from_platform_axis(contact: LedgeContact) -> f32 {
    contact.wall_normal_x
}

fn ledge_surface_kind(kind: BlockKind) -> bool {
    matches!(
        kind,
        BlockKind::Solid | BlockKind::BlinkWall { .. } | BlockKind::OneWay
    )
}

fn ledge_clearance_blocker_kind(kind: BlockKind) -> bool {
    matches!(
        kind,
        BlockKind::Solid | BlockKind::BlinkWall { .. } | BlockKind::OneWay
    )
}

/// Probe for a grabbable ledge while the player is wall-clinging.
///
/// Inputs:
/// - `player_pos` — center of the player AABB.
/// - `player_size` — full size of the player AABB.
/// - `wall_normal_x` — what `Player::wall_normal_x` reads (+/-1).
/// - `world` — the active collision world.
///
/// The probe scans for a standable ledge surface (`Solid`, `BlinkWall`, or
mod runtime;
pub use runtime::*;

#[cfg(test)]
mod tests;
