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
    // Each trail is placed by the live room of the body it hangs from.
    world: ambition_platformer2d_shared_tangle::lifecycle::LiveRoomOf<
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
        if let Some(world) = world.of(entity) {
            place(&world.0, &mut transform, &mut art, wisp(host, limb, wisp_of.index, now));
        }
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
        let Some(room) = world.room_of(body) else {
            continue;
        };
        let Some(room_world) = world.in_room(room) else {
            continue;
        };
        for index in 0..WISPS {
            let mut transform = Transform::default();
            let mut art = Sprite::from_image(sprite.handle.clone());
            place(&room_world.0, &mut transform, &mut art, wisp(host, limb, index, now));
            commands.spawn_session_scoped(
                scope.in_room(Some(room)),
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

    /// A limb's row, with its hand on its host at `at`, so every wisp is at `at`.
    fn limb_at(at: ambition_platformer2d_core::Vec2) -> ambition_sim_view::FeatureView {
        ambition_sim_view::FeatureView {
            pos: at,
            size: ambition_platformer2d_core::Vec2::new(20.0, 20.0),
            kind: ambition_platformer2d_shared_tangle::feature_kind::FeatureVisualKind::Actor,
            visible: true,
            submerged: false,
            wire_anchor: None,
            grab_reach: None,
            line_anchor: None,
            limb_host: Some(at),
            depth_plane: ambition_platformer2d_core::DepthPlane::PLAYABLE,
            flash: false,
            breakable_state: None,
            chest_opened: false,
            fighting: false,
            switch_on: false,
            rotation_rad: 0.0,
            alive: true,
            hit_flash_secs: 0.0,
            parry_flash_secs: 0.0,
            hp_current: 1,
            hp_max: 1,
            training_dummy: false,
            hit_strength: 0.0,
            unhittable: false,
            defense_cues: ambition_sim_view::DefenseCueCauses::NONE,
            sprite_offset: None,
        }
    }

    /// Each limb's trail is drawn in the live room of the body it hangs from
    /// (view half, cut V2f). Two live rooms of different sizes, a limb in each
    /// at one simulation position: each trail's wisps are stamped with their
    /// body's room and placed by its flip, at spawn and on the next frame.
    #[test]
    fn each_limb_trail_is_drawn_in_its_body_s_own_live_room() {
        use ambition_platformer2d_core as ae;
        use ambition_platformer2d_shared_tangle::lifecycle::{
            insert_live_room_component, spawn_live_room, InRoomInstance, LiveRoomInstance,
        };
        const AT: ae::Vec2 = ae::Vec2::new(100.0, 200.0);
        let room = |size: ae::Vec2| {
            ae::RoomGeometry(ae::World::new("limb room", size, ae::Vec2::new(40.0, 40.0), Vec::new()))
        };
        let (big, small) = (ae::Vec2::new(800.0, 600.0), ae::Vec2::new(400.0, 300.0));
        let mut app = App::new();
        app.init_resource::<Time>();
        app.init_resource::<LimbTrailSprite>();
        insert_live_room_component(app.world_mut(), room(big));
        let second = LiveRoomInstance::ACTIVATION.next();
        spawn_live_room(app.world_mut(), second, room(small));
        app.insert_resource(ambition_sim_view::FeatureViewIndex::from_rows([
            ("limb_in_big".to_string(), limb_at(AT)),
            ("limb_in_small".to_string(), limb_at(AT)),
        ]));
        app.add_systems(Update, sync_limb_trails);
        for (id, room) in [("limb_in_big", LiveRoomInstance::ACTIVATION), ("limb_in_small", second)] {
            app.world_mut()
                .spawn((super::super::FeatureVisual { id: id.to_string() }, InRoomInstance(room)));
        }
        let flipped = |size: ae::Vec2| Vec2::new(AT.x - size.x * 0.5, size.y * 0.5 - AT.y);
        let expected = [
            (Some(LiveRoomInstance::ACTIVATION.ordinal()), flipped(big)),
            (Some(second.ordinal()), flipped(small)),
        ];
        // The first frame spawns the wisps; the second places the standing ones.
        for frame in ["spawn", "follow"] {
            app.update();
            let mut q = app.world_mut().query::<(&LimbTrailWisp, &Transform, Option<&InRoomInstance>)>();
            let mut drawn: Vec<(Option<u32>, Vec2)> = q
                .iter(app.world())
                .map(|(_, transform, stamp)| (stamp.map(|stamp| stamp.0.ordinal()), transform.translation.truncate()))
                .collect();
            drawn.sort_by(|a, b| a.0.cmp(&b.0));
            drawn.dedup();
            assert_eq!(
                drawn, expected,
                "(room, position) of the wisps after the {frame} frame: each trail must be stamped with and \
                 placed by its body's live room"
            );
        }
    }
}
