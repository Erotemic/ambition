//! Player input intent: what the player is trying to do, independent
//! of where they are in the world.
//!
//! "Intent" here means *only* the player-driven slice — directional aim
//! and (eventually) modifier holds like "charging a smash" or "running
//! a motion input." World state (aerial / on a ledge / nearby
//! interactable) is NOT intent; it's player or world state, consumed
//! separately by resolvers. Keeping the line clear avoids the
//! growing-god-struct anti-pattern the previous design (`PlayerActionContext`)
//! was sliding into.

use bevy::prelude::*;

/// Nine-way directional intent derived from the stick + facing.
///
/// Closed set on purpose — impossible combinations like "up AND down"
/// don't exist, and resolvers `match` exhaustively. The diagonals are
/// retained so future smash-style "forward-down strong" reads have a
/// home, even though grounded resolvers today only branch on the
/// cardinal directions.
///
/// `Forward` / `Back` are relative to the player's `facing`, not a
/// world-space sign, so a player facing left who pushes the stick left
/// reads as `Forward`. This is the same convention Smash uses for
/// f-tilt / b-air etc.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Hash)]
pub enum Aim {
    #[default]
    Neutral,
    Forward,
    Back,
    Up,
    Down,
    ForwardUp,
    ForwardDown,
    BackUp,
    BackDown,
}

impl Aim {
    /// True when the aim has any downward component (cardinal Down or
    /// either down diagonal). Convenient for resolvers that don't
    /// care about left/right.
    pub fn is_down(self) -> bool {
        matches!(self, Aim::Down | Aim::ForwardDown | Aim::BackDown)
    }

    /// True when the aim has any upward component.
    pub fn is_up(self) -> bool {
        matches!(self, Aim::Up | Aim::ForwardUp | Aim::BackUp)
    }

    /// True when the aim points opposite the player's facing on the X
    /// axis (cardinal Back or either back diagonal).
    pub fn is_back(self) -> bool {
        matches!(self, Aim::Back | Aim::BackUp | Aim::BackDown)
    }

    /// True when the aim points along the player's facing on the X
    /// axis.
    pub fn is_forward(self) -> bool {
        matches!(self, Aim::Forward | Aim::ForwardUp | Aim::ForwardDown)
    }
}

/// Build an [`Aim`] from controlled-body-local stick axes + the actor's facing sign.
///
/// `facing` follows the engine convention in the controlled body frame: `+1.0`
/// = local right, `-1.0` = local left. `axis_x` / `axis_y` are local side/down
/// intent. The threshold matches the existing drop-through trigger (`axis_y >
/// 0.35`) so the HUD switches at the same moment the gameplay behavior does.
pub fn compute_aim(axis_x: f32, axis_y: f32, facing: f32) -> Aim {
    /// Matches the existing drop-through threshold so the HUD and
    /// gameplay flip simultaneously. Tuning this knob affects HUD
    /// jitter near the deadzone; consider hysteresis if the label
    /// ever flickers in practice.
    const T: f32 = 0.35;
    // X axis: resolved relative to facing. `along_facing = facing
    // >= 0.0` treats `facing = 0` (rare cold-start) as right-facing
    // so the HUD doesn't read everything as backwards before the
    // controller updates facing.
    let along_facing = facing >= 0.0;
    let forward_axis = if along_facing { axis_x } else { -axis_x };
    let x_dir: i8 = if forward_axis > T {
        1 // forward
    } else if forward_axis < -T {
        -1 // back
    } else {
        0
    };
    // Local Y axis: positive `axis_y` is controlled-body down, negative is up.
    let y_dir: i8 = if axis_y > T {
        1 // down
    } else if axis_y < -T {
        -1 // up
    } else {
        0
    };
    match (x_dir, y_dir) {
        (0, 0) => Aim::Neutral,
        (1, 0) => Aim::Forward,
        (-1, 0) => Aim::Back,
        (0, -1) => Aim::Up,
        (0, 1) => Aim::Down,
        (1, -1) => Aim::ForwardUp,
        (1, 1) => Aim::ForwardDown,
        (-1, -1) => Aim::BackUp,
        (-1, 1) => Aim::BackDown,
        // Above arms are exhaustive over the {-1, 0, 1}² product;
        // the catch-all keeps the compiler happy in case future
        // changes broaden the discriminant range.
        _ => Aim::Neutral,
    }
}

/// Resource: per-frame snapshot of player-driven intent. Refreshed by
/// [`compute_controlled_actor_intent`] once per frame, after the input
/// pipeline has folded keyboard + gamepad + touch into the slot frame
/// (`SlotControls`).
///
/// The compute system runs only when a controlled body exists; in
/// menu / startup states with no player yet, the resource keeps its
/// previous (or default) value rather than panicking on a missing
/// query.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct PlayerIntent {
    pub aim: Aim,
}

/// Derive [`PlayerIntent`] from the controlled body's slot input and its
/// facing direction. Runs after the input pipeline + touch fold has
/// published the finalized frame into `SlotControls`.
///
/// Reads the facing from the CONTROLLED subject's own entity (the
/// [`ControlledSubject`] — a possessed actor while possessing, else the home
/// avatar; the actor-local frame, not the global `Res<ControlFrame>`) and the
/// input from that body's own `Brain::Player(slot)` — the same control seam
/// the universal brain tick uses, so the intent is the driven body's own
/// intent whether or not the body carries the player-specific
/// `PlayerInputFrame` mirror (possessed actors don't). A body whose player
/// brain is mid-transfer reads neutral, the conservative default.
pub fn compute_controlled_actor_intent(
    user_settings: Option<Res<ambition_persistence::settings::UserSettings>>,
    slots: Res<ambition_characters::brain::SlotControls>,
    controlled: Option<Res<ambition_platformer_primitives::markers::ControlledSubject>>,
    primary: Query<
        Entity,
        (
            With<crate::actor::PlayerEntity>,
            With<crate::actor::PrimaryPlayer>,
        ),
    >,
    player_q: Query<(
        Option<&ambition_characters::brain::Brain>,
        &crate::actor::BodyKinematics,
        &crate::physics::ResolvedMotionFrame,
    )>,
    mut intent: ResMut<PlayerIntent>,
) {
    // The driven body: possessed subject if any, else the home avatar.
    let subject = controlled
        .and_then(|subject| subject.0)
        .or_else(|| primary.single().ok());
    let Some((brain, kinematics, resolved_frame)) = subject.and_then(|s| player_q.get(s).ok())
    else {
        // No player yet — leave the resource at its default. Any
        // downstream consumer reads `Aim::Neutral`, which is the
        // correct conservative behavior pre-spawn.
        return;
    };
    // The body's input is its slot's frame, resolved through its own player
    // brain. A possessed actor has no `PlayerInputFrame` mirror, so this is
    // the ONE input read that is correct for every controlled body.
    let frame = brain
        .and_then(ambition_characters::brain::Brain::player_slot)
        .map(|slot| slots.get(slot))
        .unwrap_or_default();
    // The body's own per-tick resolved frame (ADR 0024), not a global field.
    let gravity_dir = resolved_frame.down();
    let movement_mode = user_settings.as_deref().map_or(
        ambition_engine_core::InputFrameMode::DEFAULT_MOVEMENT,
        |s| s.gameplay.movement_frame_mode,
    );
    let local_axis = ambition_engine_core::AccelerationFrame::new(gravity_dir).resolve_input(
        movement_mode,
        ambition_engine_core::ScreenAxes::new(frame.axis_x, frame.axis_y),
    );
    let next = PlayerIntent {
        aim: compute_aim(local_axis.x, local_axis.y, kinematics.facing),
    };
    if *intent != next {
        *intent = next;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn neutral_stick_is_neutral_aim() {
        assert_eq!(compute_aim(0.0, 0.0, 1.0), Aim::Neutral);
        // Below threshold reads as neutral too.
        assert_eq!(compute_aim(0.1, -0.2, 1.0), Aim::Neutral);
    }

    #[test]
    fn forward_relative_to_right_facing() {
        // Facing right (+1): push stick right is Forward.
        assert_eq!(compute_aim(1.0, 0.0, 1.0), Aim::Forward);
        // Facing left (-1): push stick right is Back.
        assert_eq!(compute_aim(1.0, 0.0, -1.0), Aim::Back);
    }

    #[test]
    fn back_relative_to_facing() {
        assert_eq!(compute_aim(-1.0, 0.0, 1.0), Aim::Back);
        assert_eq!(compute_aim(-1.0, 0.0, -1.0), Aim::Forward);
    }

    #[test]
    fn up_and_down_resolve_from_local_axis_y_sign() {
        // Local Y is +down, so axis_y > T is Down.
        assert_eq!(compute_aim(0.0, 1.0, 1.0), Aim::Down);
        assert_eq!(compute_aim(0.0, -1.0, 1.0), Aim::Up);
    }

    #[test]
    fn diagonals_combine_axes() {
        // Right + down, facing right = ForwardDown.
        assert_eq!(compute_aim(1.0, 1.0, 1.0), Aim::ForwardDown);
        // Right + down, facing left = BackDown.
        assert_eq!(compute_aim(1.0, 1.0, -1.0), Aim::BackDown);
        // Right + up, facing right = ForwardUp.
        assert_eq!(compute_aim(1.0, -1.0, 1.0), Aim::ForwardUp);
    }

    #[test]
    fn aim_predicate_helpers_cover_diagonals() {
        assert!(Aim::Down.is_down());
        assert!(Aim::ForwardDown.is_down());
        assert!(Aim::BackDown.is_down());
        assert!(!Aim::Up.is_down());

        assert!(Aim::Back.is_back());
        assert!(Aim::BackDown.is_back());
        assert!(!Aim::Forward.is_back());

        assert!(Aim::ForwardUp.is_forward());
        assert!(Aim::ForwardUp.is_up());
    }
}
