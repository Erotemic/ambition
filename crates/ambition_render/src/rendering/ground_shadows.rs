//! The shadow a body throws on the ground under it.
//!
//! A character with no shadow is a picture on top of a room. A soft dark
//! ellipse under its feet puts it in the room, and says how high a body in
//! the air is: the shadow is smaller and fainter the higher the body is, and
//! it is gone at [`REACH`].
//!
//! The shadow is a published picture of the room's theme
//! (`RoomDressingPart::Shadow`) drawn as one sprite for each body: no shader.
//! Each shadow uses the presented body position and the geometry of its
//! live room.
//!
//! Limits:
//!
//! - The ground is the top of the nearest solid block or one-way platform
//!   under the middle of the body. A body that stands on a moving platform or
//!   another body has its shadow on the ground under that.
//! - Down is down the screen: a room with another gravity has shadows that
//!   are not under the feet.
//! - A room of a game that did not ask for shadows has none
//!   ([`GroundShadowRooms`]). No game asks now: Jon, 2026-10-10, a shadow
//!   under each body does not suit the style of the game. The system stays for
//!   a game that wants one.
//! - A room that names no theme, or a theme with no shadow picture, has none.

use std::collections::HashMap;

use bevy::prelude::*;

use ambition_platformer2d_core as ae;
use ambition_platformer2d_core::config::{world_to_bevy, WORLD_Z_BLOCK};
use ambition_platformer2d_core::AabbExt;
use ambition_platformer2d_shared_tangle::lifecycle::{
    ActiveSessionScope, SessionSpawnScope, SpawnSessionScopedExt,
};
use ambition_platformer2d_world::rooms::LiveRoomSpecs;
use ambition_sprite_sheet::game_assets::{GameAssets, ParallaxTheme, RoomDressingPart};

use super::presented_bodies::{PresentedBodies, PresentedBodyKey};
use super::primitives::RoomVisual;
use super::retained_entities::RetainedEntities;

/// How far over the ground a body still has a shadow, in world units.
pub const REACH: f32 = 150.0;
/// The alpha of the shadow of a body that stands on the ground.
const FULL_ALPHA: f32 = 0.78;
/// The height of the shadow of a body that stands, in world units.
const THICKNESS: f32 = 12.0;
/// The least width of the shadow of a body that stands, in world units.
const LEAST_WIDTH: f32 = 30.0;
/// How far under the top of the ground the middle of the shadow is, in world
/// units: the larger part of it is on the ground, not on the air over it.
const SINK: f32 = 1.5;
/// In front of the terrain and its trims and of a door, behind each actor.
const SHADOW_Z: f32 = WORLD_Z_BLOCK + 7.0;

/// A shadow on the ground of the body's live room.
#[derive(Component)]
#[require(crate::rendering::AnimatedWorldSpriteTint)]
pub struct GroundShadow;

/// The rooms that draw ground shadows: the rooms of each game that asked.
///
/// A ground shadow is a look that one game wants and another does not, and
/// one binary hosts more than one game. Thus the answer is for each room, by
/// the game mode of the room. A room with no mode is a room of the host, and
/// the host does not ask.
#[derive(Resource, Default, Clone, Debug)]
pub struct GroundShadowRooms {
    modes: Vec<&'static str>,
}

impl GroundShadowRooms {
    /// Draw ground shadows in each room that has this game mode.
    pub fn allow_mode(&mut self, mode: &'static str) {
        if !self.modes.contains(&mode) {
            self.modes.push(mode);
        }
    }

    /// Whether a room with this game mode draws ground shadows.
    pub fn draws_in(&self, mode: Option<&str>) -> bool {
        mode.is_some_and(|mode| self.modes.contains(&mode))
    }
}

/// The top of the ground under a body whose feet are at `feet`: the nearest
/// top of a solid block, a blink wall or a one-way platform that is at the
/// feet or below them and under the middle of the body. `half_width` is half
/// the width of the body. `None` when there is none within [`REACH`].
pub fn ground_below(world: &ae::World, feet: ae::Vec2, half_width: f32) -> Option<f32> {
    // The feet of a body that stands are on the top, to a part of a unit.
    const STANDS: f32 = 3.0;
    let reach = half_width * 0.5;
    world
        .blocks
        .iter()
        .filter(|block| {
            matches!(
                block.kind,
                ae::BlockKind::Solid | ae::BlockKind::OneWay | ae::BlockKind::BlinkWall { .. }
            )
        })
        .filter_map(|block| {
            let (centre, half) = (block.aabb.center(), block.aabb.half_size());
            let top = centre.y - half.y;
            let over = centre.x - half.x < feet.x + reach && centre.x + half.x > feet.x - reach;
            (over && top >= feet.y - STANDS && top - feet.y <= REACH).then_some(top)
        })
        .min_by(|a, b| a.total_cmp(b))
}

/// The size and the alpha of the shadow of a body `width` wide that is
/// `height` over the ground: full when it stands, nothing at [`REACH`].
pub fn shadow_shape(width: f32, height: f32) -> (Vec2, f32) {
    let t = (height / REACH).clamp(0.0, 1.0);
    let scale = 1.0 - 0.5 * t;
    // The box of a body is narrower than its picture: the shadow is wider
    // than the box, and not less than [`LEAST_WIDTH`].
    let full = (width * 1.5).max(LEAST_WIDTH);
    (Vec2::new(full * scale, THICKNESS * scale), FULL_ALPHA * (1.0 - t).powf(1.3))
}

/// Run condition of [`sync_ground_shadows`]: a game asked for shadows, or a
/// shadow is still drawn and is taken away on this run.
pub fn shadows_are_asked_for_or_drawn(allowed: Option<Res<GroundShadowRooms>>, drawn: Query<(), With<GroundShadow>>) -> bool {
    allowed.is_some_and(|rooms| !rooms.modes.is_empty()) || !drawn.is_empty()
}

/// Draw the shadow of each body that has ground under it.
pub(crate) fn sync_ground_shadows(
    mut commands: Commands,
    bodies: PresentedBodies,
    specs: Option<LiveRoomSpecs>,
    assets: Option<Res<GameAssets>>,
    allowed: Option<Res<GroundShadowRooms>>,
    quality: Option<Res<crate::quality::ResolvedVisualQuality>>,
    active_session: Option<Res<ActiveSessionScope>>,
    mut shadows: Query<(&mut Transform, &mut Sprite), With<GroundShadow>>,
    mut retained: Local<RetainedEntities<PresentedBodyKey>>,
) {
    let scope = SessionSpawnScope::for_optional_active_session(active_session.as_deref());
    let mut wanted = HashMap::new();
    if let (Some(specs), Some(assets), Some(allowed), Some(_)) =
        (specs, assets, allowed, scope.filter(|_| quality.is_none_or(|quality| quality.budget.parallax.enabled)))
    {
        let pictures: HashMap<_, _> = specs.live_rooms()
            .filter_map(|(room, definition)| {
                let metadata = &specs.rooms().spec(definition).metadata;
                if !allowed.draws_in(metadata.mode.as_deref()) {
                    return None;
                }
                let theme = ParallaxTheme::named_by_room_metadata(metadata)?;
                Some((room, assets.room_dressing.get(theme, RoomDressingPart::Shadow)?))
            })
            .collect();
        if !pictures.is_empty() {
            for body in bodies.iter().filter(|body| body.visible && !body.submerged) {
                let Some((world, image)) = body.geometry.zip(body.room.and_then(|room| pictures.get(&room))) else {
                    continue;
                };
                let feet = body.pos + Vec2::Y * (0.5 * body.size.y);
                let Some(top) = ground_below(world, feet, body.size.x * 0.5) else {
                    continue;
                };
                let (shape, alpha) = shadow_shape(body.size.x, (top - feet.y).max(0.0));
                if alpha > 0.01 {
                    let at = world_to_bevy(world, Vec2::new(body.pos.x, top + SINK), SHADOW_Z);
                    wanted.insert(body.key(), (at, shape, Color::srgba(1.0, 1.0, 1.0, alpha), (*image).clone()));
                }
            }
        }
    }

    retained.reconcile(&mut commands, |key| wanted.contains_key(key));
    let Some(scope) = scope else {
        return;
    };
    for (key, (at, shape, color, image)) in wanted {
        if let Some((mut transform, mut sprite)) = retained.get(&key).and_then(|entity| shadows.get_mut(*entity).ok()) {
            if transform.translation != at {
                transform.translation = at;
            }
            if sprite.image != image {
                sprite.image = image;
            }
            if sprite.custom_size != Some(shape) {
                sprite.custom_size = Some(shape);
            }
            if sprite.color != color {
                sprite.color = color;
            }
            continue;
        }
        let entity = commands.spawn_session_scoped(
            scope.in_room(key.room),
            (
                Sprite { image, custom_size: Some(shape), color, ..Default::default() },
                Transform::from_translation(at),
                GroundShadow,
                RoomVisual,
                Name::new("Ground shadow"),
            ),
        ).id();
        retained.insert(key, entity);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn room() -> ae::World {
        let (at, size) = (ae::Vec2::new, ae::Vec2::new);
        ae::World::new(
            "room",
            ae::Vec2::new(800.0, 600.0),
            ae::Vec2::ZERO,
            vec![
                ae::Block::solid("floor", at(0.0, 500.0), size(800.0, 100.0)),
                ae::Block::one_way("platform", at(300.0, 400.0), size(100.0, 16.0)),
                ae::Block::hazard("spikes", at(600.0, 480.0), size(100.0, 20.0)),
                ae::Block::solid("roof", at(0.0, 0.0), size(800.0, 40.0)),
            ],
        )
    }

    /// The ground under a body is the nearest top at its feet or below them.
    /// A platform under the body is nearer than the floor. A hazard is not
    /// ground, the roof over the body is not under it, and a body too high
    /// has no ground.
    #[test]
    fn the_ground_is_the_nearest_top_under_the_feet() {
        let world = room();
        assert_eq!(ground_below(&world, ae::Vec2::new(100.0, 500.0), 12.0), Some(500.0), "it stands on the floor");
        assert_eq!(ground_below(&world, ae::Vec2::new(100.0, 440.0), 12.0), Some(500.0), "60 over the floor");
        assert_eq!(ground_below(&world, ae::Vec2::new(350.0, 380.0), 12.0), Some(400.0), "over the platform");
        assert_eq!(ground_below(&world, ae::Vec2::new(350.0, 430.0), 12.0), Some(500.0), "under the platform");
        assert_eq!(ground_below(&world, ae::Vec2::new(650.0, 460.0), 12.0), Some(500.0), "a hazard is not ground");
        assert_eq!(ground_below(&world, ae::Vec2::new(100.0, 500.0 - REACH - 10.0), 12.0), None, "too high");
    }

    /// A body that stands has the full shadow, and a body in the air a
    /// smaller and fainter one, down to nothing at the reach.
    #[test]
    fn a_shadow_is_smaller_and_fainter_as_the_body_goes_up() {
        let (stand, stand_alpha) = shadow_shape(30.0, 0.0);
        let (mid, mid_alpha) = shadow_shape(30.0, REACH * 0.5);
        let (_, gone) = shadow_shape(30.0, REACH);
        assert_eq!(stand, Vec2::new(45.0, THICKNESS));
        assert_eq!(shadow_shape(10.0, 0.0).0.x, LEAST_WIDTH, "a thin body has a shadow that shows");
        assert_eq!(stand_alpha, FULL_ALPHA);
        assert!(mid.x < stand.x && mid.y < stand.y && mid_alpha < stand_alpha && mid_alpha > 0.0);
        assert_eq!(gone, 0.0);
    }

    /// Only the rooms of a game that asked draw shadows. A room with no mode
    /// is a room of the host, and it draws none.
    #[test]
    fn only_the_rooms_of_a_game_that_asked_draw_shadows() {
        let mut rooms = GroundShadowRooms::default();
        assert!(!rooms.draws_in(Some("mary_o")));
        rooms.allow_mode("mary_o");
        assert!(rooms.draws_in(Some("mary_o")));
        assert!(!rooms.draws_in(Some("sanic")));
        assert!(!rooms.draws_in(None));
    }

    #[test]
    fn equal_actor_ids_keep_separate_ground_shadows() {
        use ambition_platformer2d_shared_tangle::feature_kind::FeatureVisualKind;
        use ambition_platformer2d_shared_tangle::lifecycle::{
            insert_live_room_component, spawn_live_room, InRoomInstance, LiveRoomInstance,
        };
        use ambition_platformer2d_world::rooms::{insert_room_set, LiveRoomDefinition, RoomSet, RoomSpec};
        use ambition_sim_view::{FeatureView, FeatureViewIndex};
        use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

        let mut app = App::new();
        let mut allowed = GroundShadowRooms::default();
        allowed.allow_mode("test");
        app.insert_resource(allowed)
            .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(std::time::Duration::ZERO))
            .init_resource::<Assets<Image>>()
            .add_systems(Update, sync_ground_shadows);

        #[cfg(feature = "card_view")]
        {
            use super::super::card_view::{realize_card_scenery, CardMeshes, CardRootDepths};
            use bevy::{camera::visibility::VisibilitySystems, transform::TransformSystems};
            app.add_plugins((bevy::transform::TransformPlugin, bevy::camera::visibility::VisibilityPlugin))
                .init_resource::<Assets<Mesh>>()
                .init_resource::<Assets<bevy::mesh::skinning::SkinnedMeshInverseBindposes>>()
                .init_resource::<Assets<StandardMaterial>>()
                .init_resource::<CardMeshes>()
                .init_resource::<CardRootDepths>()
                .add_systems(PostUpdate, realize_card_scenery
                    .after(TransformSystems::Propagate)
                    .after(VisibilitySystems::VisibilityPropagate));
        }

        let floor = |y| ae::RoomGeometry(ae::World::new(
            "arena", Vec2::new(800.0, 600.0), Vec2::ZERO,
            vec![ae::Block::solid("floor", Vec2::new(0.0, y), Vec2::new(800.0, 100.0))],
        ));
        let (near, far) = (floor(500.0), floor(420.0));
        let mut spec = RoomSpec::new("arena", near.0.clone());
        spec.metadata.mode = Some("test".into());
        spec.metadata.visual_profile.parallax_theme = Some("cave".into());
        let east = LiveRoomInstance::ACTIVATION;
        let west = east.next();
        let world = app.world_mut();
        insert_room_set(world, RoomSet::from_parts_or_panic("arena", vec![spec], Vec::new()));
        insert_live_room_component(world, near);
        let west_root = spawn_live_room(world, west, LiveRoomDefinition::from_index(0));
        world.entity_mut(west_root).insert(far);
        let image = world.resource_mut::<Assets<Image>>().add(Image::new_fill(
            Extent3d { width: 8, height: 4, depth_or_array_layers: 1 },
            TextureDimension::D2, &[0, 0, 0, 255], TextureFormat::Rgba8UnormSrgb,
            bevy::asset::RenderAssetUsages::default(),
        ));
        let mut assets = GameAssets::default();
        assets.room_dressing.insert(ParallaxTheme::new("cave"), RoomDressingPart::Shadow, image.clone());
        world.insert_resource(assets);
        let id = "same_actor";
        let mirrors = [east, west].map(|room| world.spawn((super::super::FeatureVisual { id: id.to_owned() }, InRoomInstance(room))).id());
        let view = |room, y| FeatureView {
            room: Some(room),
            pos: Vec2::new(100.0, y),
            size: Vec2::new(30.0, 40.0),
            kind: FeatureVisualKind::Actor,
            visible: true,
            submerged: false,
            wire_anchor: None,
            grab_reach: None,
            line_anchor: None,
            limb_host: None,
            depth_plane: Default::default(),
            flash: false,
            breakable_state: None,
            chest_opened: false,
            fighting: false,
            switch_on: false,
            material: Default::default(),
            alive: true,
            hit_flash_secs: 0.0,
            parry_flash_secs: 0.0,
            hp_current: 1,
            hp_max: 1,
            training_dummy: false,
            armored: false,
            hit_strength: 0.0,
            unhittable: false,
            defense_cues: ambition_sim_view::DefenseCueCauses::NONE,
            sprite_offset: None,
            order: Default::default(),
        };
        let rows = |y| FeatureViewIndex::from_rows([(id.to_owned(), view(east, y)), (id.to_owned(), view(west, 400.0))]);
        world.insert_resource(rows(400.0));

        let shadows = |app: &mut App| {
            let world = app.world_mut();
            world.query_filtered::<(Entity, &InRoomInstance, &Transform, &Sprite), With<GroundShadow>>()
                .iter(world).map(|(id, room, t, sprite)| (room.0, (id, t.translation, sprite.color.alpha(), sprite.image.id())))
                .collect::<HashMap<_, _>>()
        };
        app.update();
        let first = shadows(&mut app);
        assert_eq!(first.len(), 2);
        assert_eq!(first[&east].1, Vec3::new(-300.0, -201.5, SHADOW_Z));
        assert_eq!(first[&west].1, Vec3::new(-300.0, -121.5, SHADOW_Z));
        assert!(first[&east].2 < first[&west].2);
        assert_eq!(first[&east].3, image.id());

        #[cfg(feature = "card_view")]
        {
            use super::super::card_view::CardScenery;
            let world = app.world_mut();
            let mut cards = world.query_filtered::<(&Transform, &Visibility), With<CardScenery>>();
            let mut positions = cards.iter(world)
                .filter(|(_, visible)| **visible != Visibility::Hidden)
                .map(|(transform, _)| transform.translation.truncate()).collect::<Vec<_>>();
            positions.sort_by(|a, b| a.y.total_cmp(&b.y));
            assert_eq!(positions, [first[&east].1.truncate(), first[&west].1.truncate()]);
        }

        app.insert_resource(rows(460.0));
        app.update();
        let moved = shadows(&mut app);
        assert_eq!(moved[&east].0, first[&east].0);
        assert!(moved[&east].2 > first[&east].2);
        assert_eq!(moved[&west], first[&west]);
        app.world_mut().despawn(mirrors[0]);
        app.update();
        assert_eq!(shadows(&mut app), HashMap::from([(west, first[&west])]));
    }
}
