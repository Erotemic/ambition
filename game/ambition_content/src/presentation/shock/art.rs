//! The skeleton and electrical arcs of a shock.

use ambition_characters::actor::Landmark;
use ambition_render::{
    folded_mesh::Tris,
    lines::strands::{jagged, ribbon, Rng},
};
use bevy::prelude::Vec2;

pub(super) const DURATION: f32 = 0.22;
const ARC_SEGMENTS: usize = 8;

pub(super) struct Skeleton {
    head: Vec2,
    hands: [Option<Vec2>; 2],
    feet: [Option<Vec2>; 2],
}

impl Skeleton {
    pub(super) fn from_landmarks(joint: impl Fn(Landmark) -> Option<Vec2>) -> Option<Self> {
        let head = joint(Landmark::Head)?;
        let hands = [joint(Landmark::HandNear), joint(Landmark::HandFar)];
        let feet = [joint(Landmark::FootNear), joint(Landmark::FootFar)];
        (hands.iter().any(Option::is_some) && feet.iter().any(Option::is_some)).then_some(Self {
            head,
            hands,
            feet,
        })
    }

    pub(super) fn standing(center: Vec2, size: Vec2, side: Vec2, down: Vec2) -> Self {
        let at = |x, y| Some(center + side * (x * size.x) + down * (y * size.y));
        Self {
            head: center - down * (0.30 * size.y),
            hands: [at(-0.35, 0.12), at(0.35, 0.12)],
            feet: [at(-0.19, 0.50), at(0.19, 0.50)],
        }
    }

    fn draw(self, tris: &mut Tris, center: Vec2, down: Vec2, side: Vec2, height: f32, alpha: f32) {
        let bone = [0.78, 0.91, 1.0, alpha];
        let dark = [0.025, 0.028, 0.10, alpha];
        let width = (height * 0.045).clamp(0.8, 4.5);
        let chest = self.head.lerp(center, 0.75);
        let hip = center + down * height * 0.10;
        stroke(tris, &[self.head, chest, hip], width, bone);
        for i in 0..3 {
            let center = chest.lerp(hip, 0.22 + i as f32 * 0.20);
            let rib = side * (0.12 - 0.018 * i as f32) * height;
            stroke(
                tris,
                &[center + rib, center + down * width, center - rib],
                width * 0.62,
                bone,
            );
        }
        for (end, sign) in self.hands.into_iter().zip([-1.0, 1.0]) {
            if let Some(hand) = end {
                let shoulder = chest + side * (sign * height * 0.09);
                let elbow = shoulder.lerp(hand, 0.55) + down * height * 0.045;
                stroke(tris, &[shoulder, elbow, hand], width, bone);
            }
        }
        for (end, sign) in self.feet.into_iter().zip([-1.0, 1.0]) {
            if let Some(foot) = end {
                let socket = hip + side * (sign * height * 0.055);
                let knee = socket.lerp(foot, 0.54);
                stroke(tris, &[socket, knee, foot], width, bone);
            }
        }
        let skull = self.head - down * height * 0.046;
        let radius = height * 0.125;
        ellipse(tris, skull, side * radius, down * radius * 0.78, bone);
        stroke(
            tris,
            &[
                skull + down * radius * 0.64 - side * radius * 0.55,
                skull + down * radius * 0.64 + side * radius * 0.55,
            ],
            radius * 0.52,
            bone,
        );
        for sign in [-1.0, 1.0] {
            ellipse(
                tris,
                skull + side * radius * 0.37 * sign,
                side * radius * 0.23,
                down * radius * 0.2,
                dark,
            );
        }
    }
}

fn stroke(tris: &mut Tris, points: &[Vec2], width: f32, color: [f32; 4]) {
    let widths = [width; 3];
    ribbon(
        tris,
        points,
        &widths[..points.len()],
        color,
        [color[0], color[1], color[2], color[3] * 0.50],
    );
}

fn ellipse(tris: &mut Tris, at: Vec2, side: Vec2, down: Vec2, color: [f32; 4]) {
    const SIDES: u32 = 12;
    let center = tris.vertex(at, color);
    for i in 0..SIDES {
        let (y, x) = (std::f32::consts::TAU * i as f32 / SIDES as f32).sin_cos();
        tris.vertex(at + side * x + down * y, color);
    }
    for i in 0..SIDES {
        tris.indices
            .extend_from_slice(&[center, center + 1 + i, center + 1 + (i + 1) % SIDES]);
    }
}

pub(super) fn shock_art(
    seed: u64,
    age: f32,
    center: Vec2,
    size: Vec2,
    side: Vec2,
    down: Vec2,
    bones: Option<Skeleton>,
) -> Tris {
    let mut tris = Tris::default();
    let remaining = (1.0 - age / DURATION).clamp(0.0, 1.0);
    if let Some(bones) = bones {
        bones.draw(&mut tris, center, down, side, size.y, remaining.sqrt());
    }

    let mut rng = Rng::new(seed ^ ((age / 0.044) as u64));
    let half = size * Vec2::new(0.58, 0.52);
    let width = (size.y * 0.043).clamp(1.0, 3.5);
    let at = |angle: f32, reach: f32| {
        center + reach * (side * half.x * angle.cos() + down * half.y * angle.sin())
    };
    for i in 0..3 {
        let angle = std::f32::consts::TAU * i as f32 / 3.0 + rng.range(-0.6, 0.6);
        let from = at(angle, 0.76);
        let to = at(angle + rng.range(0.6, 1.65), rng.range(1.02, 1.4));
        let points = jagged(&mut rng, from, to, ARC_SEGMENTS, 0.19);
        ribbon(
            &mut tris,
            &points,
            &[width * 3.0; ARC_SEGMENTS + 1],
            [0.34, 0.24, 1.0, remaining * 0.70],
            [0.32, 0.18, 1.0, 0.0],
        );
        ribbon(
            &mut tris,
            &points,
            &[width; ARC_SEGMENTS + 1],
            [0.91, 0.97, 1.0, remaining.sqrt()],
            [0.54, 0.65, 1.0, 0.0],
        );
    }
    tris
}
