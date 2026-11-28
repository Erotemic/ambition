//! Interpolate rigid part transforms in parent frames.
//!
//! Published keyframes remain in sheet space. Calculate the local transforms
//! from both endpoints and compose them in parent-first order. Keep the draw
//! order and the selected raster. The texture scale does not scale a joint.

use super::{draw_overhang, shorter_turn, tween_draw, PartDraw, RigPart};
use bevy::math::Vec2;

#[derive(Debug, Clone, Default, PartialEq)]
pub(super) struct Hierarchy {
    order: Vec<(u16, Option<u16>)>,
    track_count: usize,
}

#[derive(Clone, Copy)]
struct Motion {
    index: usize,
    parent: Option<usize>,
    at: Vec2,
    end: Vec2,
    rotation: f32,
    turn: f32,
    reflected_scale: Option<Vec2>,
}

fn rotate(v: Vec2, angle: f32) -> Vec2 {
    let (s, c) = angle.sin_cos();
    Vec2::new(c * v.x - s * v.y, s * v.x + c * v.y)
}

fn local(draw: &PartDraw, parent: Option<&PartDraw>) -> (Vec2, f32) {
    match parent {
        Some(parent) => (
            rotate(draw.at - parent.at, -parent.rotation),
            draw.rotation - parent.rotation,
        ),
        None => (draw.at, draw.rotation),
    }
}

impl Hierarchy {
    pub(super) fn new(order: Vec<(u16, Option<u16>)>, track_count: usize) -> Self {
        Self { order, track_count }
    }

    pub(super) fn is_empty(&self) -> bool {
        self.order.is_empty()
    }

    /// The output is in the source frame's order for a tween, and in the
    /// destination frame's order for a clip blend. A track present in only
    /// one endpoint keeps that endpoint's local transform. A composition
    /// that has removed its parent uses the world transform instead.
    fn motions(&self, from: &[PartDraw], to: &[PartDraw], destination: bool) -> Vec<Motion> {
        let mut indices = vec![[None; 3]; self.track_count];
        for (side, draws) in [from, to].into_iter().enumerate() {
            for (index, draw) in draws.iter().enumerate() {
                if let Some(track) = draw
                    .track
                    .and_then(|track| indices.get_mut(usize::from(track)))
                {
                    track[side] = Some(index);
                }
            }
        }
        let mut motions = Vec::with_capacity(self.order.len());
        for &(track, parent) in &self.order {
            let track = usize::from(track);
            let [a, b, _] = indices[track];
            let Some(index) = (if destination { b } else { a }) else {
                continue;
            };
            let p = parent.map(|p| indices[usize::from(p)]);
            let parent = p.filter(|p| {
                p[2].is_some() && (a.is_none() || p[0].is_some()) && (b.is_none() || p[1].is_some())
            });
            let x = a.map(|i| local(&from[i], parent.and_then(|p| p[0]).map(|i| &from[i])));
            let y = b.map(|i| local(&to[i], parent.and_then(|p| p[1]).map(|i| &to[i])));
            let (at, rotation) = x.or(y).expect("selected track is present");
            let (end, angle) = y.unwrap_or((at, rotation));
            let reflected_scale = match (destination, a, b) {
                // Keep the destination scale when the blend changes handedness.
                (true, Some(a), Some(b))
                    if !(from[a].scale * to[b].scale).cmpgt(Vec2::ZERO).all() =>
                {
                    Some(to[b].scale)
                }
                _ => None,
            };
            indices[track][2] = Some(motions.len());
            motions.push(Motion {
                index,
                parent: parent.and_then(|p| p[2]),
                at,
                end,
                rotation,
                turn: shorter_turn(rotation, angle),
                reflected_scale,
            });
        }
        motions
    }

    fn apply_motions(motions: &[Motion], t: f32, out: &mut [PartDraw]) {
        for motion in motions {
            let local_at = motion.at.lerp(motion.end, t);
            let local_angle = motion.rotation + motion.turn * t;
            let (at, rotation) = if let Some(parent) = motion.parent {
                let parent = out[motions[parent].index];
                (
                    parent.at + rotate(local_at, parent.rotation),
                    parent.rotation + local_angle,
                )
            } else {
                (local_at, local_angle)
            };
            let draw = &mut out[motion.index];
            draw.at = at;
            draw.rotation = rotation;
            if let Some(scale) = motion.reflected_scale {
                draw.scale = scale;
            }
        }
    }

    pub(super) fn tween(&self, from: &[PartDraw], to: &[PartDraw], t: f32, out: &mut [PartDraw]) {
        Self::apply_motions(&self.motions(from, to, false), t, out);
    }

    pub(super) fn blend(&self, from: &[PartDraw], t: f32, onto: &mut [PartDraw]) {
        let motions = self.motions(from, onto, true);
        super::blend_from(from, t, onto);
        Self::apply_motions(&motions, t, onto);
    }

    /// Bound the arc between keyframes. For p(t)=q(t)+R(a(t))*v(t),
    /// |p''| <= |q''| + a'^2 max(|v0|,|v1|) + 2 |a'| |v1-v0|.
    /// The angles are linear, also after composition. Apply the same bound
    /// to each corner of the selected raster. A chord of length h in time
    /// is within acceleration*h*h/8 of the curve.
    pub(super) fn overhang(
        &self,
        parts: &[RigPart],
        from: &[PartDraw],
        to: &[PartDraw],
        targets: &[Option<std::num::NonZeroU32>],
        all: &[PartDraw],
        frame_size: Vec2,
        feet: Vec2,
    ) -> f32 {
        if self.is_empty() {
            return 0.0;
        }
        let motions = self.motions(from, to, false);
        let mut bounds: Vec<(f32, f32)> = Vec::with_capacity(motions.len());
        let mut acceleration = 0.0_f32;
        for motion in &motions {
            let (parent_speed, parent_accel) = motion.parent.map_or((0.0, 0.0), |p| bounds[p]);
            let speed = parent_speed + motion.turn;
            let accel = parent_accel
                + parent_speed * parent_speed * motion.at.length().max(motion.end.length())
                + 2.0 * parent_speed.abs() * (motion.end - motion.at).length();
            bounds.push((speed, accel));
            let draw = &from[motion.index];
            let end_scale =
                targets[motion.index].map_or(draw.scale, |i| all[i.get() as usize - 1].scale);
            let part = &parts[usize::from(draw.part)];
            for corner in super::part_corners(part) {
                let (a, b) = (corner * draw.scale, corner * end_scale);
                acceleration = acceleration.max(
                    accel
                        + speed * speed * a.length().max(b.length())
                        + 2.0 * speed.abs() * (b - a).length(),
                );
            }
        }
        let error = acceleration / 8.0;
        let segments = ((error / super::TWEEN_ENVELOPE_SLACK_PX).sqrt().ceil() as usize)
            .clamp(1, super::TWEEN_ENVELOPE_MOST_SEGMENTS);
        let mut out = from.to_vec();
        let mut reach = 0.0_f32;
        for step in 0..=segments {
            let t = step as f32 / segments as f32;
            for (i, (draw, target)) in from.iter().zip(targets).enumerate() {
                out[i] = target.map_or(*draw, |index| {
                    tween_draw(draw, &all[index.get() as usize - 1], t)
                });
            }
            Self::apply_motions(&motions, t, &mut out);
            for motion in &motions {
                let draw = &out[motion.index];
                reach = reach.max(draw_overhang(
                    &parts[usize::from(draw.part)],
                    draw,
                    frame_size,
                    feet,
                ));
            }
        }
        reach + error / (segments as f32 * segments as f32)
    }
}
