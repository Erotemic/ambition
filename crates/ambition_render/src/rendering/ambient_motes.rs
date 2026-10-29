//! The motes of a room: small things that drift in its air.
//!
//! A theme has a picture of motes (`RoomDressingPart::Motes`: dust, embers,
//! fireflies, bubbles) and a style ([`MoteStyle`]) that says how many there
//! are and how they move. A room of that theme gets that many small sprites.
//! Each one has a home in a field a little larger than the view. The field
//! goes with the camera and a mote that leaves one side of it comes in at the
//! other, so the motes are in the world (a mote does not move with the
//! camera) and there are always the same number in view.
//!
//! This is sprites that move, and no shader: a room has its motes with each
//! shader off. It is presentation only. Nothing reads a mote.
//!
//! Limits:
//!
//! - The field goes with the first main camera. In a split view, the motes
//!   of a room are around that camera only.
//! - A mote does not know the terrain: it goes through a wall. A mote that
//!   is "behind" is drawn behind the blocks, so most of this is not seen.

use bevy::prelude::*;

use ambition_platformer2d_core::config::{WORLD_Z_BLOCK, WORLD_Z_FX};
use ambition_platformer2d_shared_tangle::camera_layers::MainCamera;
use ambition_platformer2d_shared_tangle::lifecycle::{
    ActiveSessionScope, InRoomInstance, SessionSpawnScope, SpawnSessionScopedExt,
};
use ambition_platformer2d_world::rooms::LiveRoomSpecs;
use ambition_sprite_sheet::game_assets::{
    GameAssets, ParallaxTheme, RoomDressingPart, MOTE_CELL_PX, MOTE_VARIANTS,
};

use super::primitives::RoomVisual;

pub use ambition_sprite_sheet::game_assets::MoteStyle;

/// One mote. Each value is made one time, when the mote is.
#[derive(Component, Clone, Copy, Debug)]
pub struct AmbientMote {
    /// Where the mote is in the field at time 0, in parts of the field.
    pub home: Vec2,
    /// Where the mote is in its wander and in its twinkle at time 0.
    pub phase: f32,
    /// How fast this mote is, in parts of the speed of its style.
    pub pace: f32,
    pub style: MoteStyle,
    pub z: f32,
}

/// The field of the motes, in parts of what the camera shows of the world.
const FIELD: f32 = 1.3;
/// What a camera shows when it does not say.
const DEFAULT_VISIBLE: Vec2 = Vec2::new(640.0, 360.0);

/// A number from 0 to 1 for `n`, the same each time.
fn dice(n: u32) -> f32 {
    let mut x = n.wrapping_mul(0x9E37_79B9) ^ 0x85EB_CA6B;
    x ^= x >> 15;
    x = x.wrapping_mul(0x2C1B_3C6D);
    x ^= x >> 12;
    x = x.wrapping_mul(0x297A_2D39);
    x ^= x >> 15;
    (x >> 8) as f32 / (1u32 << 24) as f32
}

/// Where a mote is at time `t`, for a camera at `camera` that shows
/// `visible` of the world, and its alpha in parts of the alpha of its style.
pub fn mote_at(mote: &AmbientMote, t: f32, camera: Vec2, visible: Vec2) -> (Vec2, f32) {
    let field = visible * FIELD;
    let turn = t * mote.style.wander_rate * std::f32::consts::TAU + mote.phase;
    let free = mote.home * field
        + mote.style.velocity * mote.pace * t
        + Vec2::new(turn.sin(), (turn * 0.8 + mote.phase).cos()) * mote.style.wander;
    let origin = camera - field * 0.5;
    let inside = Vec2::new((free.x - origin.x).rem_euclid(field.x), (free.y - origin.y).rem_euclid(field.y));
    let twinkle = 0.5 + 0.5 * (t * mote.style.twinkle_rate * std::f32::consts::TAU + mote.phase * 3.0).sin();
    (origin + inside, 1.0 - mote.style.twinkle * twinkle)
}

/// The room of this marker has its motes.
#[derive(Component, Clone, Copy, Debug)]
pub struct PresentedRoomMotes;

/// Give each live room whose theme has motes its motes.
pub fn present_room_motes(
    mut commands: Commands,
    rooms: Option<LiveRoomSpecs>,
    assets: Option<Res<GameAssets>>,
    quality: Option<Res<crate::quality::ResolvedVisualQuality>>,
    active_session: Option<Res<ActiveSessionScope>>,
    presented: Query<&InRoomInstance, With<PresentedRoomMotes>>,
) {
    let (Some(rooms), Some(assets)) = (rooms, assets) else {
        return;
    };
    // A tier that draws no parallax draws no motes: they are the same kind
    // of thing, the air of the room.
    if quality.is_some_and(|quality| !quality.budget.parallax.enabled) {
        return;
    }
    let Some(session_scope) = SessionSpawnScope::for_optional_active_session(active_session.as_deref()) else {
        return;
    };
    for (room, definition) in rooms.live_rooms() {
        if presented.iter().any(|stamp| stamp.0 == room) {
            continue;
        }
        let spec = rooms.rooms().spec(definition);
        let Some(theme) = ParallaxTheme::named_by_room_metadata(&spec.metadata) else {
            continue;
        };
        let (Some(style), Some(image)) = (assets.room_themes.motes.get(theme.key()), assets.room_dressing.get(theme, RoomDressingPart::Motes)) else {
            // The picture can come later than the room: ask again then.
            continue;
        };
        let scope = session_scope.in_room(Some(room));
        commands.spawn_session_scoped(scope, (PresentedRoomMotes, RoomVisual, Name::new("Room motes")));
        let cell = MOTE_CELL_PX as f32;
        for index in 0..style.count as u32 {
            let roll = |salt: u32| dice(index.wrapping_mul(7919).wrapping_add(salt).wrapping_add(theme.key().bytes().fold(0x811c9dc5u32, |hash, byte| (hash ^ byte as u32).wrapping_mul(0x01000193))));
            let variant = (roll(1) * MOTE_VARIANTS as f32).min(MOTE_VARIANTS as f32 - 1.0).floor();
            let size = style.size.0 + (style.size.1 - style.size.0) * roll(2);
            let front = roll(3) < style.front;
            let mote = AmbientMote {
                home: Vec2::new(roll(4), roll(5)),
                phase: roll(6) * std::f32::consts::TAU,
                pace: 0.6 + 0.8 * roll(7),
                style: *style,
                // In front of the effects and behind the foreground layer, or
                // behind the blocks.
                z: if front { WORLD_Z_FX + 10.0 + roll(8) } else { WORLD_Z_BLOCK - 1.0 - roll(8) },
            };
            commands.spawn_session_scoped(
                scope,
                (
                    Sprite {
                        image: image.clone(),
                        rect: Some(Rect::new(variant * cell, 0.0, (variant + 1.0) * cell, cell)),
                        custom_size: Some(Vec2::splat(size)),
                        color: Color::srgba(1.0, 1.0, 1.0, 0.0),
                        ..Default::default()
                    },
                    Transform::from_xyz(0.0, 0.0, mote.z),
                    mote,
                    RoomVisual,
                    Name::new("Ambient mote"),
                ),
            );
        }
    }
}

/// Move each mote to where it is now.
pub fn drift_room_motes(
    time: Res<Time>,
    cameras: Query<(&Transform, Option<&Projection>), (With<MainCamera>, Without<AmbientMote>)>,
    mut motes: Query<(&AmbientMote, &mut Transform, &mut Sprite)>,
) {
    if motes.is_empty() {
        return;
    }
    let Some((camera, projection)) = cameras.iter().next() else {
        return;
    };
    let visible = match projection {
        Some(Projection::Orthographic(orthographic)) => {
            let area = orthographic.area.size();
            // The area is 2 by 2 before the camera system has run one time.
            if area.x > 2.0 && area.y > 2.0 { area } else { DEFAULT_VISIBLE }
        }
        _ => DEFAULT_VISIBLE,
    };
    let t = time.elapsed_secs();
    for (mote, mut transform, mut sprite) in &mut motes {
        let (at, alpha) = mote_at(mote, t, camera.translation.truncate(), visible);
        transform.translation = at.extend(mote.z);
        sprite.color = Color::srgba(1.0, 1.0, 1.0, mote.style.alpha * alpha);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mote(style: MoteStyle) -> AmbientMote {
        AmbientMote { home: Vec2::new(0.25, 0.75), phase: 1.0, pace: 1.0, style, z: 0.0 }
    }

    const STILL: MoteStyle = MoteStyle { count: 1, velocity: Vec2::ZERO, wander: 0.0, wander_rate: 0.0, size: (1.0, 1.0), alpha: 1.0, twinkle: 0.0, twinkle_rate: 0.0, front: 0.0 };
    const VISIBLE: Vec2 = Vec2::new(640.0, 360.0);

    /// A mote is in the world: when the camera moves less than the field, a
    /// mote that does not move stays where it is. And it is always in the
    /// field of the camera: a camera that goes far finds it at the same
    /// place in the field as a camera one field back.
    #[test]
    fn a_mote_stays_in_the_world_and_in_the_field_of_the_camera() {
        let mote = mote(STILL);
        let field = VISIBLE * FIELD;
        let (at, _) = mote_at(&mote, 0.0, field * 0.5, VISIBLE);
        assert!((at - Vec2::new(0.25, 0.75) * field).length() < 1e-3);
        let (moved, _) = mote_at(&mote, 0.0, field * 0.5 + Vec2::new(40.0, -25.0), VISIBLE);
        assert!((moved - at).length() < 1e-3, "the camera moved, the mote did not");
        let far = field * 0.5 + field * Vec2::new(7.0, -3.0);
        let (wrapped, _) = mote_at(&mote, 0.0, far, VISIBLE);
        assert!((wrapped - (at + field * Vec2::new(7.0, -3.0))).length() < 1e-2);
        for camera in [Vec2::ZERO, Vec2::new(12_345.0, -6_789.0)] {
            let (at, _) = mote_at(&mote, 3.0, camera, VISIBLE);
            let rel = at - (camera - field * 0.5);
            assert!(rel.x >= 0.0 && rel.x < field.x && rel.y >= 0.0 && rel.y < field.y);
        }
    }

    /// A mote with a velocity is that far along after a second, and one that
    /// twinkles loses no more than its style says.
    #[test]
    fn a_mote_moves_at_its_velocity_and_twinkles_within_its_style() {
        let rising = mote(MoteStyle { velocity: Vec2::new(0.0, 20.0), ..STILL });
        let centre = VISIBLE * FIELD * 0.5;
        let (a, _) = mote_at(&rising, 0.0, centre, VISIBLE);
        let (b, _) = mote_at(&rising, 1.0, centre, VISIBLE);
        assert!((b - a - Vec2::new(0.0, 20.0)).length() < 1e-3);
        let twinkling = mote(MoteStyle { twinkle: 0.6, twinkle_rate: 1.0, ..STILL });
        let alphas: Vec<f32> = (0..40).map(|i| mote_at(&twinkling, i as f32 * 0.025, centre, VISIBLE).1).collect();
        let (low, high) = alphas.iter().fold((1.0f32, 0.0f32), |(lo, hi), a| (lo.min(*a), hi.max(*a)));
        assert!(low >= 0.4 - 1e-4 && low < 0.45, "the low point is 1 - twinkle: {low}");
        assert!(high > 0.95 && high <= 1.0, "{high}");
        assert_eq!(mote_at(&mote(STILL), 0.37, centre, VISIBLE).1, 1.0, "control: a steady mote");
    }

    #[test]
    fn the_dice_fill_their_range() {
        let rolls: Vec<f32> = (0..2000).map(dice).collect();
        assert!(rolls.iter().all(|r| (0.0..1.0).contains(r)));
        let mean = rolls.iter().sum::<f32>() / rolls.len() as f32;
        assert!((mean - 0.5).abs() < 0.03, "{mean}");
        assert!(rolls.iter().any(|r| *r < 0.02) && rolls.iter().any(|r| *r > 0.98));
    }
}
