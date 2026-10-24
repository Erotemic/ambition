//! Each drawn body publishes where its limbs are cut at portal lines
//! ([`LimbSeams`], see `ambition_portal2d_presentation::limb_seams`).
//!
//! A sheet says it in the anchors of each frame: `seam:<slot>:a` and
//! `seam:<slot>:b` are the two ends of the cut of the limb `<slot>`, and
//! `seam:self:a` / `:b` the cut of the picture of a limb alone (the end of an
//! arm that came out of a portal). Each is a part of the frame (0 to 1), so a
//! sheet of each quality has the same. The renderer measures them on the
//! drawn picture (`giant_art.limb_seams`).
//!
//! The frame is put in the world by the sheet's authored body: the body box
//! of the frame is drawn on the body's box (`posed_body_geometry`). So the
//! seam is where the picture is drawn, on both roads (parts or a baked
//! frame), and does not need either one.

use ambition_platformer2d_core as ae;
use bevy::prelude::*;

use ambition_portal2d_presentation::{LimbSeam, LimbSeams};
use ambition_sprite_sheet::character::CharacterAnimator;

use crate::rendering::primitives::FeatureVisual;

/// The prefix of a seam's anchors, and the name of the seam of a limb's own
/// picture.
const SEAM: &str = "seam:";
const OWN: &str = "self";

/// The seams of one frame: for each pair of anchors `seam:<name>:a` and
/// `seam:<name>:b`, its name and its two ends in pixels of the frame.
pub fn frame_seams<'a>(
    anchors: &'a std::collections::BTreeMap<String, ambition_sprite_sheet::NormPoint>,
    frame: Vec2,
) -> impl Iterator<Item = (&'a str, Vec2, Vec2)> + 'a {
    anchors.iter().filter_map(move |(key, a)| {
        let name = key.strip_prefix(SEAM)?.strip_suffix(":a")?;
        let b = anchors.get(&format!("{SEAM}{name}:b"))?;
        Some((
            name,
            Vec2::new(a.x, a.y) * frame,
            Vec2::new(b.x, b.y) * frame,
        ))
    })
}

/// A cut of a frame (its two ends, in pixels of the frame) as a seam in the
/// world: the frame's `body` box is on the body's box (`pos`, its middle,
/// and `size`), mirrored when `mirrored`.
pub fn seam_in_world(
    name: &str,
    ends: (Vec2, Vec2),
    body: ambition_sprite_sheet::PixelRect,
    pos: ae::Vec2,
    size: ae::Vec2,
    mirrored: bool,
    depth: f32,
) -> Option<LimbSeam> {
    let (cx, cy) = body.center();
    let scale = size / Vec2::new(body.w as f32, body.h as f32);
    let place = |p: Vec2| {
        let mut from_middle = (p - Vec2::new(cx, cy)) * scale;
        if mirrored {
            from_middle.x = -from_middle.x;
        }
        pos + from_middle
    };
    let (a, b) = (place(ends.0), place(ends.1));
    let along = (b - a).try_normalize()?;
    Some(LimbSeam {
        limb: (name != OWN).then(|| name.to_owned()),
        middle: (a + b) * 0.5,
        along,
        half: a.distance(b) * 0.5,
        depth,
    })
}

/// Write the seams of each drawn body's frame this frame (see the module).
/// After the animators chose the frame; before the portal presentation reads
/// the seams.
#[allow(clippy::type_complexity)]
pub fn publish_limb_seams(
    mut commands: Commands,
    views: Res<ambition_sim_view::FeatureViewIndex>,
    mut bodies: Query<(
        Entity,
        &FeatureVisual,
        &CharacterAnimator,
        &Sprite,
        &GlobalTransform,
        Option<&mut LimbSeams>,
    )>,
) {
    for (entity, visual, animator, sprite, place, published) in &mut bodies {
        let spec = &animator.spec;
        let frame = Vec2::new(spec.frame_width as f32, spec.frame_height as f32);
        let anchors = animator
            .drawn_row()
            .and_then(|row| spec.frame_anchors(row, animator.frame));
        let mut seams = Vec::new();
        if let (Some(anchors), Some(body), Some(view)) = (
            anchors,
            spec.authored_body_box(animator.current),
            views.get(&visual.id),
        ) {
            let depth = place.translation().z;
            for (name, a, b) in frame_seams(anchors, frame) {
                seams.extend(seam_in_world(
                    name,
                    (a, b),
                    body,
                    view.pos,
                    view.size,
                    sprite.flip_x,
                    depth,
                ));
            }
        }
        match published {
            Some(mut published) => {
                if published.0 != seams {
                    published.0 = seams;
                }
            }
            None if !seams.is_empty() => {
                commands.entity(entity).try_insert(LimbSeams(seams));
            }
            None => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ⭐ A seam of a frame is put on the body as the frame's body box is:
    /// its middle by the body's middle, its length by the body's scale, and
    /// mirrored with the picture.
    #[test]
    fn a_seam_is_on_the_body_as_its_frame_is() {
        let body = ambition_sprite_sheet::PixelRect {
            x: 100,
            y: 100,
            w: 200,
            h: 100,
        };
        // The body's box is twice the frame's body box, with its middle at
        // (1000, 500).
        let (pos, size) = (ae::Vec2::new(1000.0, 500.0), ae::Vec2::new(400.0, 200.0));
        // A cut 50 px left of the body's middle, 30 px long.
        let ends = (Vec2::new(150.0, 135.0), Vec2::new(150.0, 165.0));
        let seam = seam_in_world("hand_left", ends, body, pos, size, false, 3.0).unwrap();
        assert_eq!(seam.limb.as_deref(), Some("hand_left"));
        assert!(
            seam.middle.distance(Vec2::new(900.0, 500.0)) < 1e-4,
            "{:?}",
            seam.middle
        );
        assert!((seam.half - 30.0).abs() < 1e-4);
        assert!(seam.along.distance(Vec2::Y) < 1e-4);
        let mirrored = seam_in_world("self", ends, body, pos, size, true, 3.0).unwrap();
        assert_eq!(mirrored.limb, None);
        assert!(mirrored.middle.distance(Vec2::new(1100.0, 500.0)) < 1e-4);
    }

    #[test]
    fn a_frame_names_each_seam_by_its_two_anchors() {
        let point = |x: f32, y: f32| ambition_sprite_sheet::NormPoint { x, y };
        let anchors = std::collections::BTreeMap::from([
            ("seam:hand_left:a".to_owned(), point(0.25, 0.5)),
            ("seam:hand_left:b".to_owned(), point(0.25, 0.75)),
            ("seam:lonely:a".to_owned(), point(0.5, 0.5)),
            ("foot".to_owned(), point(0.5, 1.0)),
        ]);
        let seams: Vec<_> = frame_seams(&anchors, Vec2::new(100.0, 40.0)).collect();
        assert_eq!(
            seams,
            vec![("hand_left", Vec2::new(25.0, 20.0), Vec2::new(25.0, 30.0))]
        );
    }
}
