//! The ethereal trail: a limb that flies free of its body stays visibly its own.
//!
//! GNU-ton's fists are separate bodies — they fly, fall and stick in the floor
//! far from the giant they belong to — and nothing on screen said whose they
//! were. A limb's bond to its host is already sim state (`Limb::of`), projected
//! as `FeatureView::limb_host`, so this draws every limb's bond: a chain of soft
//! glowing wisps from the host's body out to the limb, drifting outward along
//! it, faint where it leaves the body and brightest at the hand.
//!
//! Presentation only, and procedural like the flyline: one small glow texture
//! built once, a fixed number of wisps per limb, placed every frame. Drawn
//! behind every actor, so the trail comes out from behind the body.

use bevy::asset::RenderAssetUsages;
use bevy::image::Image;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use ambition_platformer2d_shared_tangle::lifecycle::{
    ActiveSessionScope, SessionSpawnScope, SpawnSessionScopedExt,
};

/// Wisps along one trail.
const WISPS: usize = 14;
/// Glow texture side, texels.
const GLOW: u32 = 32;
/// How far the trail bows sideways at its middle, world px.
const SAG: f32 = 26.0;
/// Trips a wisp makes from body to hand, per second.
const FLOW_HZ: f32 = 0.55;
/// Behind every actor (`WORLD_Z_DUMMY + 1`), in front of the level.
pub(crate) const TRAIL_Z: f32 = ambition_platformer2d_core::config::WORLD_Z_DUMMY + 0.5;

/// One wisp of one limb's trail.
#[derive(Component)]
pub struct LimbTrailWisp {
    pub body: Entity,
    pub index: usize,
}

/// The glow's texture handle, built once.
#[derive(Resource, Clone, Default)]
pub struct LimbTrailSprite {
    pub handle: Handle<Image>,
}

/// A soft round glow: bright core, long falloff, no edge to alias.
pub fn build_limb_trail_image() -> Image {
    let mut data = vec![0u8; (GLOW * GLOW * 4) as usize];
    let c = (GLOW as f32 - 1.0) * 0.5;
    for y in 0..GLOW {
        for x in 0..GLOW {
            let d = ((x as f32 - c).powi(2) + (y as f32 - c).powi(2)).sqrt() / c;
            let a = (1.0 - d).max(0.0).powf(1.8);
            let core = (1.0 - d * 2.2).max(0.0);
            let i = ((y * GLOW + x) * 4) as usize;
            data[i] = (220.0 + 35.0 * core) as u8;
            data[i + 1] = (236.0 + 19.0 * core) as u8;
            data[i + 2] = 255;
            data[i + 3] = (a * 255.0) as u8;
        }
    }
    Image::new(
        Extent3d { width: GLOW, height: GLOW, depth_or_array_layers: 1 },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    )
}

pub fn build_limb_trail_sprite(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let handle = images.add(build_limb_trail_image());
    commands.insert_resource(LimbTrailSprite { handle });
}

/// Where wisp `index` of a trail from `host` to `limb` is at `time`, its size
/// and its alpha. Pure, so the shape is testable without an App.
pub fn wisp(host: Vec2, limb: Vec2, index: usize, time: f32) -> (Vec2, f32, f32) {
    let span = limb - host;
    let across = Vec2::new(-span.y, span.x).normalize_or_zero();
    // Evenly spaced, all drifting outward together, wrapping at the hand.
    let u = ((index as f32 + (time * FLOW_HZ * WISPS as f32).fract()) / WISPS as f32).fract();
    let bow = (u * std::f32::consts::PI).sin();
    let shimmer = ((time * 3.1 + index as f32 * 1.7).sin()) * 4.0 * bow;
    let at = host + span * u + across * (SAG * bow + shimmer);
    // Faint leaving the body, brightest near the hand, gone at both ends.
    let fade = (u * std::f32::consts::PI).sin().powf(0.6);
    let alpha = (0.18 + 0.5 * u) * fade;
    let size = 9.0 + 12.0 * u;
    (at, size, alpha)
}

/// Draw each limb's trail to its host.
pub fn sync_limb_trails(
    mut commands: Commands,
    time: Res<Time>,
    world: ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<
        ambition_platformer2d_core::RoomGeometry,
    >,
    sprite: Option<Res<LimbTrailSprite>>,
    active_session: Option<Res<ActiveSessionScope>>,
    limbs: Query<(Entity, &super::FeatureVisual)>,
    feature_views: Option<Res<ambition_sim_view::FeatureViewIndex>>,
    mut wisps: Query<(Entity, &LimbTrailWisp, &mut Transform, &mut Sprite)>,
) {
    let now = time.elapsed_secs();
    let mut trails: Vec<(Entity, Vec2, Vec2)> = Vec::new();
    for (body, visual) in &limbs {
        let Some(view) = feature_views.as_ref().and_then(|index| index.get(&visual.id)) else {
            continue;
        };
        if let (Some(host), true) = (view.limb_host, view.visible) {
            trails.push((body, Vec2::new(host.x, host.y), Vec2::new(view.pos.x, view.pos.y)));
        }
    }

    let mut standing = bevy::platform::collections::HashSet::new();
    for (entity, wisp_of, mut transform, mut art) in &mut wisps {
        let Some(&(_, host, limb)) = trails.iter().find(|(b, _, _)| *b == wisp_of.body) else {
            commands.entity(entity).despawn();
            continue;
        };
        standing.insert(wisp_of.body);
        place(&world.0, &mut transform, &mut art, wisp(host, limb, wisp_of.index, now));
    }

    let Some(sprite) = sprite else {
        return;
    };
    let Some(scope) = SessionSpawnScope::for_optional_active_session(active_session.as_deref()) else {
        return;
    };
    for (body, host, limb) in trails {
        if standing.contains(&body) {
            continue;
        }
        for index in 0..WISPS {
            let mut transform = Transform::default();
            let mut art = Sprite::from_image(sprite.handle.clone());
            place(&world.0, &mut transform, &mut art, wisp(host, limb, index, now));
            commands.spawn_session_scoped(
                scope,
                (
                    art,
                    transform,
                    LimbTrailWisp { body, index },
                    ambition_platformer2d_shared_tangle::lifecycle::PresentationOf(body),
                    Name::new("Limb trail wisp"),
                ),
            );
        }
    }
}

fn place(
    room: &ambition_platformer2d_core::World,
    transform: &mut Transform,
    art: &mut Sprite,
    (at, size, alpha): (Vec2, f32, f32),
) {
    transform.translation = ambition_platformer2d_core::config::world_to_bevy(
        room,
        ambition_platformer2d_core::Vec2::new(at.x, at.y),
        TRAIL_Z,
    );
    art.custom_size = Some(Vec2::splat(size));
    art.color = Color::srgba(0.82, 0.92, 1.0, alpha);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The trail runs from the body to the hand, bowed between them, and is
    /// brighter at the hand than where it leaves the body.
    #[test]
    fn a_trail_runs_from_the_body_to_the_hand_and_brightens_toward_it() {
        let (host, limb) = (Vec2::new(0.0, 0.0), Vec2::new(300.0, 0.0));
        let samples: Vec<_> = (0..WISPS).map(|i| wisp(host, limb, i, 0.0)).collect();
        for (at, _, _) in &samples {
            assert!((-1.0..=301.0).contains(&at.x), "a wisp strays past the ends: {at:?}");
        }
        let near_body = samples.iter().min_by(|a, b| a.0.x.total_cmp(&b.0.x)).unwrap();
        let near_hand = samples.iter().filter(|s| s.0.x < 290.0).max_by(|a, b| a.0.x.total_cmp(&b.0.x)).unwrap();
        assert!(near_hand.2 > near_body.2, "brightest toward the hand: {near_body:?} vs {near_hand:?}");
        let middle = samples.iter().min_by(|a, b| (a.0.x - 150.0).abs().total_cmp(&(b.0.x - 150.0).abs())).unwrap();
        assert!(middle.0.y.abs() > 10.0, "the trail bows between them: {middle:?}");
    }
}
