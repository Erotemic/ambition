//! The scenery of a Sanic act: the goal post.
//!
//! The goal of an act was a place and a word over it (a debug label). It is a
//! post now: a prop of the `sanic_goal_post` sheet that stands on the ground
//! at the goal and turns its sign over when the act is cleared.

use bevy::prelude::*;

use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::world::rooms::{PropDraw, PropSpec, RoomSpec};

use crate::{SanicActPhase, SanicActState};

/// The sheet of the goal post, and the name of its prop.
pub const GOAL_POST_SPRITE: &str = "sanic_goal_post";

/// The rows of the sheet: it turns over once, then shows its other side.
const SPIN_ROW: &str = "spin";
const CLEARED_ROW: &str = "cleared";

/// The ground at `x`: the lowest surface there that a body stands on, from
/// the chains and the solid blocks of `world`. `None` over a place with none.
pub fn ground_at(world: &ae::World, x: f32) -> Option<f32> {
    let on_chains = world.chains.iter().flat_map(|chain| {
        (0..chain.segment_count()).filter_map(move |segment| {
            // Ground: its ride side is up (in a room y is down).
            if chain.normal(segment).y > -0.5 {
                return None;
            }
            let (a, b) = chain.segment(segment);
            let (lo, hi) = if a.x <= b.x { (a, b) } else { (b, a) };
            (lo.x <= x && x <= hi.x && hi.x > lo.x)
                .then(|| lo.y + (hi.y - lo.y) * (x - lo.x) / (hi.x - lo.x))
        })
    });
    let on_blocks = world
        .blocks
        .iter()
        .filter(|block| matches!(block.kind, ae::BlockKind::Solid))
        .filter(|block| block.aabb.min.x <= x && x <= block.aabb.max.x)
        .map(|block| block.aabb.min.y);
    on_chains
        .chain(on_blocks)
        .fold(None, |lowest: Option<f32>, y| {
            Some(lowest.map_or(y, |low| low.max(y)))
        })
}

/// Stand the goal post of `room` on the ground at its goal.
///
/// A build that baked no sheet for it (sprites not generated) or a goal over
/// no ground leaves the room as it was.
pub fn dress_goal(room: &mut RoomSpec) {
    let goal_x = crate::goal_x_of(&room.world);
    let Some(ground) = ground_at(&room.world, goal_x) else {
        return;
    };
    let Some(record) = ambition_platformer2d::sprite_sheet::character::sheets::record_for_sheet_key(
        GOAL_POST_SPRITE,
    ) else {
        return;
    };
    let Some(body) = record
        .body_metrics
        .as_ref()
        .and_then(|metrics| metrics.body_pixel_bbox)
    else {
        return;
    };
    let frame = ae::Vec2::new(record.frame_width as f32, record.frame_height as f32);
    let scale = crate::rules::sanic_rules().act.goal_post_height / body.h as f32;
    let (foot, centre_x) = (
        (body.y + body.h) as f32,
        body.x as f32 + body.w as f32 * 0.5,
    );
    room.props.push(PropSpec {
        id: format!("{}_goal_post", room.id),
        name: GOAL_POST_SPRITE.to_string(),
        kind: GOAL_POST_SPRITE.to_string(),
        // The middle of the frame, carried into the world through the post's
        // foot and its middle column.
        pos: ae::Vec2::new(
            goal_x + (frame.x * 0.5 - centre_x) * scale,
            ground + (frame.y * 0.5 - foot) * scale,
        ),
        size: frame * scale,
        flip_y: false,
        // Built world, behind the runner.
        draw: PropDraw::Structure,
    });
}

/// The row the post shows (`None`: its idle), from whether the act is
/// cleared, the row it shows now, and whether that row has played out.
fn post_row(cleared: bool, showing: Option<&str>, played_out: bool) -> Option<&'static str> {
    match (cleared, showing) {
        (false, _) => None,
        (true, Some(CLEARED_ROW)) => Some(CLEARED_ROW),
        (true, Some(SPIN_ROW)) if played_out => Some(CLEARED_ROW),
        (true, _) => Some(SPIN_ROW),
    }
}

/// Show each goal post as the act says: its badnik side while the act runs,
/// its spin when the act is cleared, then its star.
///
/// Derived from [`SanicActState`] every frame, not from the clear: that state
/// is rollback state, so a clear on a frame a rewind throws away turns the
/// sign back.
pub fn dress_goal_posts(
    mut commands: Commands,
    acts: Query<&SanicActState>,
    posts: Query<(
        Entity,
        &ambition_platformer2d::render::rendering::PropVisual,
        &ambition_platformer2d::sprite_sheet::character::CharacterAnimator,
        Option<&ambition_platformer2d::render::rendering::PropClip>,
    )>,
) {
    use ambition_platformer2d::render::rendering::PropClip;
    let cleared = acts
        .iter()
        .any(|act| matches!(act.phase, SanicActPhase::Cleared { .. }));
    for (entity, prop, animator, clip) in &posts {
        if prop.kind != GOAL_POST_SPRITE {
            continue;
        }
        let showing = clip.map(|clip| clip.0.clip.as_str());
        let want = post_row(cleared, showing, animator.clip_finished());
        if want == showing {
            continue;
        }
        match want {
            Some(row) => {
                let request = ambition_platformer2d::sim_view::ClipRequest::from_chain(&[row])
                    .expect("one row");
                commands.entity(entity).insert(PropClip(request));
            }
            None => {
                commands.entity(entity).remove::<PropClip>();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The post stands on the ground of each act, at its goal. The control is
    /// the ground itself: an act whose goal were over no ground would get no
    /// post and no error.
    #[test]
    fn each_act_has_a_goal_post_that_stands_on_its_ground() {
        for room in [
            crate::sanic_speedway(),
            crate::sanic_highway(),
            crate::sanic_darkness(),
        ] {
            let goal_x = crate::goal_x_of(&room.world);
            let ground = ground_at(&room.world, goal_x)
                .unwrap_or_else(|| panic!("{}: no ground at its goal ({goal_x})", room.id));
            let post = room
                .props
                .iter()
                .find(|prop| prop.kind == GOAL_POST_SPRITE)
                .unwrap_or_else(|| {
                    panic!(
                        "{}: no goal post (is the `{GOAL_POST_SPRITE}` sheet baked?)",
                        room.id
                    )
                });
            let foot = post.pos.y + post.size.y * 0.5;
            assert!(
                (foot - ground).abs() < post.size.y * 0.08,
                "{}: the foot of the frame is at {foot:.1}, the ground at {ground:.1}",
                room.id
            );
            assert!(
                (post.pos.x - goal_x).abs() < 4.0,
                "{}: the post is at {:.1}, the goal at {goal_x}",
                room.id,
                post.pos.x
            );
        }
    }

    #[test]
    fn the_post_idles_spins_once_then_shows_its_star_and_a_rewind_turns_it_back() {
        assert_eq!(post_row(false, None, false), None);
        assert_eq!(post_row(true, None, false), Some(SPIN_ROW));
        assert_eq!(post_row(true, Some(SPIN_ROW), false), Some(SPIN_ROW));
        assert_eq!(post_row(true, Some(SPIN_ROW), true), Some(CLEARED_ROW));
        assert_eq!(post_row(true, Some(CLEARED_ROW), true), Some(CLEARED_ROW));
        assert_eq!(post_row(false, Some(CLEARED_ROW), true), None);
    }
}
