//! Where a limb that goes through a portal is cut, and the lines of light
//! that are drawn on those cuts.
//!
//! A limb in a portal is drawn in two pictures: its body's picture is cut at
//! the near line (the stump), and the end of the limb comes out of the far
//! line in a picture of its own (a fist with the end of its fore arm). Each
//! picture publishes where it is cut, each frame (a [`LimbSeam`]): the middle
//! of the cut, the way along it, and how wide the limb is there. The seam is
//! measured on the drawn picture, so it is where the limb is drawn, and not
//! where a constant of a game says it is.
//!
//! A line of light that a module asks to be ON a limb
//! (`ambition_portal_port::LineOnLimb`) is put on its seam by
//! [`lay_light_lines_on_limb_seams`]: at the middle of the cut, rotated to
//! it, never shorter than the limb is wide there, and drawn over the limb
//! (so it reads as the seam, not as a line behind the arm). The line eases
//! onto the seam and off it: a portal's line never jumps.
//!
//! The renderer publishes the seams (`ambition_render`'s
//! `publish_limb_seams`, from the anchors `seam:<slot>:a` / `:b` of each
//! frame of a sheet). This crate reads them. Both are presentation: no
//! simulation system reads a seam, and a rewind does not need one.

use bevy::prelude::*;

use crate::PortalLightLine;

/// The extra length of a line on a limb, past the limb's two edges together.
pub const LIMB_LINE_MARGIN: f32 = 6.0;
/// How long a line takes to ease onto a seam, or off one, in seconds.
pub const LAY_S: f32 = 0.12;
/// How far over its limb's picture a line on the limb is drawn (render z).
pub const OVER_LIMB_Z: f32 = 0.3;
/// How far from where the module put a line on a limb's own body the cut of
/// that body may be, in px. A cut farther away is another limb's.
pub const LAY_REACH: f32 = 160.0;

/// Where a drawn limb is cut at a portal's line. Engine world coordinates
/// (+y down).
#[derive(Clone, Debug, PartialEq)]
pub struct LimbSeam {
    /// The slot of the limb that is cut (`hand_left`), on the body it is a
    /// limb of (the stump). `None`: the body IS the limb, and this is the cut
    /// of its own picture (the end of the limb that came out of the far line).
    /// A picture of a limb alone does not know which limb of its host it is:
    /// a line on such a cut is laid on the nearest one ([`LAY_REACH`]).
    pub limb: Option<String>,
    /// The middle of the cut.
    pub middle: Vec2,
    /// The way along the cut (unit length).
    pub along: Vec2,
    /// Half the length of the cut: half the limb's width there, its outline
    /// included.
    pub half: f32,
    /// The render z of the picture that is cut.
    pub depth: f32,
}

/// The seams of a drawn body this frame (see the module). Empty when no
/// limb of it is cut.
#[derive(Component, Clone, Debug, Default, PartialEq)]
pub struct LimbSeams(pub Vec<LimbSeam>);

/// On a [`PortalLightLine`] that is on a limb: which limb, where the module
/// put the line (it is drawn there while no seam is published), and how far
/// it is laid on the seam.
#[derive(Component, Clone, Debug, PartialEq)]
pub struct LightLineOnLimb {
    pub limb: String,
    pub on_its_body: bool,
    /// Where the module put the line: its middle, the room's side, its length.
    pub nominal: (Vec2, Vec2, f32),
    /// 0 at the module's place, 1 on the seam.
    pub laid: f32,
    /// The last place on a seam, and the z over its limb: a line eases off a
    /// seam that is gone from where it was.
    pub last: Option<((Vec2, Vec2, f32), f32)>,
}

impl LightLineOnLimb {
    pub fn new(limb: String, on_its_body: bool, nominal: (Vec2, Vec2, f32)) -> Self {
        Self {
            limb,
            on_its_body,
            nominal,
            laid: 0.0,
            last: None,
        }
    }
}

/// A line on `seam`: at its middle, rotated to it (the room's side kept
/// toward `normal`), and never shorter than the limb is wide plus
/// [`LIMB_LINE_MARGIN`].
pub fn laid_on(seam: &LimbSeam, normal: Vec2, length: f32) -> (Vec2, Vec2, f32) {
    let across = Vec2::new(-seam.along.y, seam.along.x);
    let across = if across.dot(normal) < 0.0 {
        -across
    } else {
        across
    };
    (
        seam.middle,
        across,
        length.max(seam.half * 2.0 + LIMB_LINE_MARGIN),
    )
}

/// The line `laid` of the way from `from` to `to` (eased: it starts and
/// ends slowly).
pub fn eased_between(
    from: (Vec2, Vec2, f32),
    to: (Vec2, Vec2, f32),
    laid: f32,
) -> (Vec2, Vec2, f32) {
    let u = laid.clamp(0.0, 1.0);
    let u = u * u * (3.0 - 2.0 * u);
    let normal = from.1.lerp(to.1, u).normalize_or(to.1);
    (from.0.lerp(to.0, u), normal, from.2 + (to.2 - from.2) * u)
}

/// Put each line on a limb on its limb's seam in its room: the stump of that
/// limb on its host, or the cut of a limb's own picture, whichever the line
/// names, nearest to where the module put it. Before the lines are drawn
/// (`sync_portal_light_lines`).
pub fn lay_light_lines_on_limb_seams(
    time: Res<Time>,
    live: ambition_platformer2d_shared_tangle::lifecycle::LiveRooms,
    bodies: Query<(Entity, &LimbSeams)>,
    mut lines: Query<(Entity, &mut LightLineOnLimb, &mut PortalLightLine)>,
) {
    let step = time.delta_secs().min(0.1) / LAY_S;
    for (line_entity, mut on, mut light) in &mut lines {
        let room = live.of(line_entity);
        let (at, normal, length) = on.nominal;
        let seam = bodies
            .iter()
            .filter(|(body, _)| room.is_some() && live.of(*body) == room)
            .flat_map(|(_, seams)| seams.0.iter())
            .filter(|seam| match &seam.limb {
                Some(limb) => !on.on_its_body && *limb == on.limb,
                None => on.on_its_body && seam.middle.distance(at) <= LAY_REACH,
            })
            .min_by(|a, b| {
                a.middle
                    .distance_squared(at)
                    .total_cmp(&b.middle.distance_squared(at))
            });
        let target = seam.map(|seam| (laid_on(seam, normal, length), seam.depth + OVER_LIMB_Z));
        if target.is_some() {
            on.last = target;
        }
        let laid = (on.laid + if target.is_some() { step } else { -step }).clamp(0.0, 1.0);
        if laid != on.laid {
            on.laid = laid;
        }
        let (place, depth) = match on.last {
            Some((on_seam, depth)) if laid > 0.0 => {
                (eased_between(on.nominal, on_seam, laid), Some(depth))
            }
            _ => (on.nominal, None),
        };
        let wanted = PortalLightLine {
            pos: place.0,
            normal: place.1,
            length: place.2,
            depth,
            ..*light
        };
        if *light != wanted {
            *light = wanted;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seam(middle: Vec2, along: Vec2, half: f32) -> LimbSeam {
        LimbSeam {
            limb: Some("hand_left".into()),
            middle,
            along,
            half,
            depth: 12.0,
        }
    }

    /// ⭐ A line on a limb is at the middle of the limb's cut, rotated to it,
    /// as wide as the limb there plus its margin, and its room's side is the
    /// side the module asked for.
    #[test]
    fn a_line_on_a_limb_is_on_its_cut_and_as_wide_as_the_limb() {
        let cut = seam(Vec2::new(300.0, 420.0), Vec2::new(0.0, 1.0), 28.0);
        let (at, normal, length) = laid_on(&cut, Vec2::new(-1.0, 0.1), 10.0);
        assert!(at.distance(cut.middle) < 1e-4);
        assert!((normal - Vec2::new(-1.0, 0.0)).length() < 1e-4, "{normal}");
        assert!((length - (56.0 + LIMB_LINE_MARGIN)).abs() < 1e-4);
        // A line that the module opens wider than the limb stays as open.
        assert_eq!(laid_on(&cut, Vec2::X, 150.0).2, 150.0);
    }

    /// A line eases onto its seam: it does not jump.
    #[test]
    fn a_line_eases_onto_its_seam() {
        let from = (Vec2::ZERO, Vec2::X, 60.0);
        let to = (Vec2::new(40.0, 0.0), Vec2::Y, 80.0);
        assert_eq!(eased_between(from, to, 0.0), from);
        let end = eased_between(from, to, 1.0);
        assert!(end.0.distance(to.0) < 1e-4 && end.1.distance(to.1) < 1e-4);
        let mut last = from.0;
        for k in 1..=10 {
            let now = eased_between(from, to, k as f32 / 10.0).0;
            assert!(
                now.distance(last) <= 40.0 * 0.16,
                "a step of {}",
                now.distance(last)
            );
            last = now;
        }
    }
}
