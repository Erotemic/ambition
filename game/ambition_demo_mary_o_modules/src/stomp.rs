//! The ONE top/side contact rule for Mary-O's enemies.
//!
//! Every enemy rule in this demo turns on the same question: is the player ON
//! this body, or beside it? From the top it is a stomp (the snake shells, the
//! AI Slop squash), and the player is always safe. From a side it is a threat:
//! contact damage, or a running shell. The question is answered here, once,
//! so that a module's rule and the demo's native shell rule can not disagree
//! about the same contact.
//!
//! Mary-O runs under screen gravity, so `+y` is DOWN: the player's feet are
//! its `max[1]`, and a body's head is its `min[1]`.

/// A box by its corners.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bounds {
    pub min: [f32; 2],
    pub max: [f32; 2],
}

impl Bounds {
    /// The box of a body at `position` (its middle) with `size`.
    pub fn of(position: [f32; 2], size: [f32; 2]) -> Self {
        let half = [size[0] * 0.5, size[1] * 0.5];
        Self {
            min: [position[0] - half[0], position[1] - half[1]],
            max: [position[0] + half[0], position[1] + half[1]],
        }
    }
}

/// Which face of a body the player is touching.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlayerTouch {
    /// Feet within the stomp band of the head while not moving upward: coming
    /// down onto it, or already resting on it. Resting counts: a body under
    /// the player's feet must never hurt them.
    Top,
    /// Overlapping anywhere else: beside it, or rising into it from below.
    Side,
}

/// Classify the player's contact with one body, or `None` if they are apart.
/// `band` is the vertical tolerance (px) for "feet on its head": `stomp.band`
/// in her pack ([`crate::rules::StompRules`]).
pub fn player_touch(body: Bounds, player: Bounds, player_vel: [f32; 2], band: f32) -> Option<PlayerTouch> {
    let overlap_x = player.min[0] < body.max[0] && player.max[0] > body.min[0];
    if !overlap_x {
        return None;
    }
    let feet = player.max[1];
    // The band is penetration tolerance, not reach: the player's feet must
    // have reached the head plane. Allowing `head - band` classified a
    // player hovering up to sixteen pixels above an enemy as a stomp.
    //
    // Her feet on the same ground sat 9.48 below its head, inside 16, and
    // every side run-in read as a stomp. Half the body is the rule every game
    // of this kind uses: feet above the enemy's middle is a stomp, below it
    // is a hit. That is independent of height, so a shorter enemy can not
    // reopen this.
    let band = band.min((body.max[1] - body.min[1]) * 0.5);
    let on_head = feet >= body.min[1] && feet <= body.min[1] + band;
    // Falling onto the head, or standing on it (`vel.y == 0`), is a stomp.
    // Rising INTO it from below is not: that is a hit.
    if on_head && player_vel[1] >= 0.0 {
        return Some(PlayerTouch::Top);
    }
    let overlap_y = player.min[1] < body.max[1] && player.max[1] > body.min[1];
    overlap_y.then_some(PlayerTouch::Side)
}
