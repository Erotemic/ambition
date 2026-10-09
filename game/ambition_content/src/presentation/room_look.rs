//! Named looks for a room's architecture.
//!
//! A room asks for a look with its `palette` level field. A look is
//! presentation only. It reads the room's blocks and draws over them;
//! collision does not change.
//!
//! One material draws a whole look. Each quad is a world-space window into the
//! same procedural scene, so a pan of the camera does not move the art on the
//! architecture. The quad's ROLE selects the layer:
//!
//! | role | quad | what it is for |
//! |---|---|---|
//! | backdrop | the room | what is behind the architecture |
//! | surface | one block | the architecture |
//! | underside | below one platform | what hangs from a platform |
//! | portal | around one door | what frames a door |
//! | overlay | the room | what floats in front of the architecture |
//!
//! Each look gives the roles its own meaning, in its own shader:
//!
//! | palette | look |
//! |---|---|
//! | `clean_corrupted` | [`RoomStateMaterial`]: one architecture in two states. The corrupted state is DERIVED in the shader from the same construction as the clean state, and a scalar field with a blocky front decides which state a point shows. |
//! | `debug_beautiful` | [`RoomBlueprintMaterial`]: the collision truth of the room, drawn as a drawing. A solid is a closed outline, a one-way platform has an open underside. |
//!
//! A look also chooses the door of the room ([`RoomLook::door_art`]), with the
//! renderer's `EntityArt` seam.
//!
//! The original block sprites stay under the surface quads. If a material does
//! not draw, the room looks as it did before.

use bevy::{
    asset::embedded_asset,
    prelude::*,
    reflect::TypePath,
    render::render_resource::AsBindGroup,
    shader::ShaderRef,
    sprite_render::{AlphaMode2d, Material2d, Material2dPlugin, MeshMaterial2d},
};

use ambition_platformer2d_core as ae;
use ambition_platformer2d_core::config::{world_to_bevy, WORLD_Z_BLOCK};
use ambition_platformer2d_core::AabbExt;
use ambition_platformer2d_shared_tangle::lifecycle::{
    ActiveSessionScope, InRoomInstance, SessionScopeSet, SessionSpawnScope, SpawnSessionScopedExt,
};
use ambition_platformer2d_world::rooms::{LiveRoomSpecs, LoadingZoneActivation, RoomSpec};
use ambition_render::rendering::label_layout::{MirroredWorldLabel, StaticWorldLabel, WorldLabel};
use ambition_render::rendering::{
    BlockVisual, EntityArt, LoadingZoneVisual, RoomVisual, DOOR_SPRITE_ASPECT,
};
use ambition_sprite_sheet::game_assets::EntitySprite;

/// Above the parallax panels (`-18.0..=-15.0`), below the blocks.
const BACKDROP_Z: f32 = -14.5;
/// Behind the surfaces: ornament below a platform is not a place to stand.
const UNDERSIDE_Z: f32 = WORLD_Z_BLOCK + 0.1;
/// How far an underside reaches below its platform.
const UNDERSIDE_REACH: f32 = 230.0;
/// A block at most this tall is a platform, and gets an underside.
const PLATFORM_MAX_HEIGHT: f32 = 40.0;
/// Between the undersides and the surfaces, so a floor covers the foot of a
/// door frame. The door sprite is far above (`WORLD_Z_BLOCK + 6.0`).
const PORTAL_Z: f32 = WORLD_Z_BLOCK + 0.15;
/// How far a portal quad reaches past its door's trigger box.
const PORTAL_PAD: f32 = 56.0;
/// Directly above the block sprite it replaces.
const SURFACE_Z: f32 = WORLD_Z_BLOCK + 0.2;
/// Above the surfaces, below climbables, water, doors and bodies.
const OVERLAY_Z: f32 = WORLD_Z_BLOCK + 3.0;
/// How far a surface quad reaches past its block, for art that grows past the
/// block edge.
const SURFACE_PAD: f32 = 16.0;
/// How far the backdrop reaches past the room, for a camera at the room edge.
const BACKDROP_PAD: f32 = 1200.0;

const ROLE_BACKDROP: f32 = 0.0;
const ROLE_SURFACE: f32 = 1.0;
const ROLE_OVERLAY: f32 = 2.0;
const ROLE_UNDERSIDE: f32 = 3.0;
const ROLE_PORTAL: f32 = 4.0;

/// The block kinds a look draws, as the shader reads them.
const KIND_SOLID: f32 = 0.0;
const KIND_ONE_WAY: f32 = 1.0;

/// Text on the pale side of the two-state look: ink, with a paper halo so that
/// a sign that crosses the front stays readable.
const INK_TEXT: Color = Color::srgba(0.165, 0.140, 0.250, 0.96);
const INK_HALO: Color = Color::srgba(0.965, 0.945, 0.900, 0.80);

/// One named look: a material whose shader draws every role.
///
/// All positions are in engine world coordinates (y down), which is where the
/// shaders do their work.
pub trait RoomLook: Material2d {
    /// The `palette` level-field value that asks for this look.
    const PALETTE: &'static str;

    /// One window into the look's scene.
    ///
    /// - `piece`: what this quad draws, `min.x, min.y, size.x, size.y`.
    /// - `room`: `x, y` = the room size, `z` = the role. `w` = the block kind
    ///   for a surface or an underside, and the width of the door sprite as
    ///   a fraction of its height for a portal, whose `piece` is the trigger
    ///   box of the door.
    /// - `front`: a point on the room's front (`x, y`) and its normal
    ///   (`z, w`).
    fn window(piece: Vec4, room: Vec4, front: Vec4) -> Self;

    /// The door this look gives to a door at `at` in `world`.
    fn door_art(world: &ae::World, at: ae::Vec2) -> EntitySprite;
}

/// One architecture in two states: clean and corrupted.
#[derive(Asset, AsBindGroup, TypePath, Debug, Clone)]
pub struct RoomStateMaterial {
    #[uniform(0)]
    pub piece: Vec4,
    #[uniform(1)]
    pub room: Vec4,
    /// The normal points into the corrupted side.
    #[uniform(2)]
    pub front: Vec4,
}

impl Material2d for RoomStateMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://ambition_content/presentation/shaders/room_state.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }
}

impl RoomLook for RoomStateMaterial {
    const PALETTE: &'static str = "clean_corrupted";

    fn window(piece: Vec4, room: Vec4, front: Vec4) -> Self {
        Self { piece, room, front }
    }

    fn door_art(world: &ae::World, at: ae::Vec2) -> EntitySprite {
        if behind_front(world, at) > 0.0 {
            EntitySprite::DoorVoxel
        } else {
            EntitySprite::DoorStone
        }
    }
}

/// The collision truth of a room, drawn as a drawing.
#[derive(Asset, AsBindGroup, TypePath, Debug, Clone)]
pub struct RoomBlueprintMaterial {
    #[uniform(0)]
    pub piece: Vec4,
    #[uniform(1)]
    pub room: Vec4,
    /// This look has one state. The front is not read.
    #[uniform(2)]
    pub front: Vec4,
}

impl Material2d for RoomBlueprintMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://ambition_content/presentation/shaders/room_blueprint.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }
}

impl RoomLook for RoomBlueprintMaterial {
    const PALETTE: &'static str = "debug_beautiful";

    fn window(piece: Vec4, room: Vec4, front: Vec4) -> Self {
        Self { piece, room, front }
    }

    fn door_art(_world: &ae::World, _at: ae::Vec2) -> EntitySprite {
        EntitySprite::DoorBlueprint
    }
}

#[derive(Resource, Default)]
struct RoomLookInstalled;

/// Marks a live room whose look is spawned. It is stamped with the room, so it
/// leaves with the room, and the look is spawned again on a replay.
#[derive(Component)]
struct PresentedRoomLook;

/// The unit quad every piece of a look scales.
#[derive(Resource)]
struct RoomLookQuad(Handle<Mesh>);

/// Install the looks and their per-room spawners. Idempotent.
pub fn install(app: &mut App) {
    if app.world().contains_resource::<RoomLookInstalled>() {
        return;
    }
    // `embedded_asset!` needs the asset registries.
    if app
        .world()
        .get_resource::<bevy::asset::io::embedded::EmbeddedAssetRegistry>()
        .is_none()
    {
        return;
    }
    app.insert_resource(RoomLookInstalled);
    embedded_asset!(app, "shaders/room_state.wgsl");
    embedded_asset!(app, "shaders/room_blueprint.wgsl");
    // `ambition_content::room_look`: what the looks share. A composition with
    // no shader assets draws no look to import it.
    if app.world().contains_resource::<Assets<bevy::shader::Shader>>() {
        bevy::shader::load_shader_library!(app, "shaders/room_look_common.wgsl");
    }
    install_look::<RoomStateMaterial>(app);
    install_look::<RoomBlueprintMaterial>(app);
    app.add_systems(
        Update,
        ink_labels_on_the_clean_side.run_if(resource_exists::<Assets<RoomStateMaterial>>),
    );
}

fn install_look<M: RoomLook>(app: &mut App)
where
    M::Data: PartialEq + Eq + std::hash::Hash + Clone,
{
    // The material plugin installs render-world state, so it needs an app
    // that renders. An asset registry is not evidence of a renderer.
    if app.get_sub_app(bevy::render::RenderApp).is_some() {
        app.add_plugins(Material2dPlugin::<M>::default());
    }
    app.add_systems(
        Update,
        present_room_look::<M>
            // With the room's own visuals, which a look draws over.
            .in_set(SessionScopeSet::Presentation)
            // Absent collections mean that this app does not draw.
            .run_if(resource_exists::<Assets<Mesh>>)
            .run_if(resource_exists::<Assets<M>>),
    );
    app.add_systems(Update, dress_doors::<M>.run_if(resource_exists::<Assets<M>>));
}

/// Whether a room asks for the look `M`.
fn asks_for<M: RoomLook>(spec: &RoomSpec) -> bool {
    spec.metadata.visual_profile.palette.as_deref() == Some(M::PALETTE)
}

/// The front of a room: through the room centre, with a lean. A room has no
/// authored front yet, so each room that asks for a look gets the same one.
fn room_front(world: &ae::World) -> Vec4 {
    let normal = Vec2::new(1.0, 0.32).normalize();
    Vec4::new(world.size.x * 0.5, world.size.y * 0.5, normal.x, normal.y)
}

/// How far `at` is behind the front of `world`, in world px. Positive is the
/// corrupted side of the two-state look.
///
/// The shader is the authority on the front (`field` in `room_state.wgsl`).
/// This is its part that does not move: the plane and the large wobble. The
/// part that moves is at most 65 px, so a sign or a door that close to the
/// front can be dressed for the other side.
fn behind_front(world: &ae::World, at: ae::Vec2) -> f32 {
    let front = room_front(world);
    let wobble = (value_noise(Vec2::new(at.x, at.y), 260.0, 7) - 0.5) * 260.0;
    (at.x - front.x) * front.z + (at.y - front.y) * front.w + wobble
}

/// `rand_cell` of `room_look_common.wgsl`: a value in `[0, 1)` for one
/// integer cell. The two must agree bit for bit.
fn rand_cell(cell: Vec2, salt: u32) -> f32 {
    let x = cell.x.floor() as i32 as u32;
    let y = cell.y.floor() as i32 as u32;
    let mut h = x.wrapping_mul(1_597_334_677)
        ^ y.wrapping_mul(3_812_015_801)
        ^ salt.wrapping_mul(668_265_263);
    h ^= h >> 16;
    h = h.wrapping_mul(0x7feb_352d);
    h ^= h >> 15;
    h = h.wrapping_mul(0x846c_a68b);
    h ^= h >> 16;
    (h >> 8) as f32 * (1.0 / 16_777_216.0)
}

/// `value_noise` of `room_look_common.wgsl`.
fn value_noise(p: Vec2, scale: f32, salt: u32) -> f32 {
    let g = p / scale;
    let i = g.floor();
    let f = g - i;
    let f = f * f * (Vec2::splat(3.0) - 2.0 * f);
    let a = rand_cell(i, salt);
    let b = rand_cell(i + Vec2::X, salt);
    let c = rand_cell(i + Vec2::Y, salt);
    let d = rand_cell(i + Vec2::ONE, salt);
    (a + (b - a) * f.x) * (1.0 - f.y) + (c + (d - c) * f.x) * f.y
}

/// Give each door of a room that asks for the look `M` the door of that look.
fn dress_doors<M: RoomLook>(
    mut commands: Commands,
    rooms: LiveRoomSpecs,
    doors: Query<(Entity, &LoadingZoneVisual, &InRoomInstance), Added<LoadingZoneVisual>>,
) {
    for (entity, visual, stamp) in &doors {
        let Some((_, definition)) = rooms.live_rooms().find(|(room, _)| *room == stamp.0) else {
            continue;
        };
        let spec = rooms.rooms().spec(definition);
        if !asks_for::<M>(spec) {
            continue;
        }
        let Some(zone) = spec.loading_zones.iter().find(|zone| {
            zone.id == visual.id && matches!(zone.activation, LoadingZoneActivation::Door)
        }) else {
            continue;
        };
        // `try_insert`: a room visual can leave before the command flush.
        commands
            .entity(entity)
            .try_insert(EntityArt(M::door_art(&spec.world, zone.aabb.center())));
    }
}

/// Give each live room that asks for the look `M` its quads, stamped with the
/// room.
fn present_room_look<M: RoomLook>(
    mut commands: Commands,
    rooms: LiveRoomSpecs,
    presented: Query<&InRoomInstance, With<PresentedRoomLook>>,
    quad: Option<Res<RoomLookQuad>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<M>>,
    active_session: Option<Res<ActiveSessionScope>>,
) {
    let Some(session_scope) =
        SessionSpawnScope::for_optional_active_session(active_session.as_deref())
    else {
        return;
    };
    for (room, definition) in rooms.live_rooms() {
        let spec = rooms.rooms().spec(definition);
        if !asks_for::<M>(spec) || presented.iter().any(|stamp| stamp.0 == room) {
            continue;
        }
        let quad = match quad.as_deref() {
            Some(quad) => quad.0.clone(),
            None => {
                let handle = meshes.add(Rectangle::new(1.0, 1.0));
                commands.insert_resource(RoomLookQuad(handle.clone()));
                handle
            }
        };
        let scope = session_scope.in_room(Some(room));
        let world = &spec.world;
        let front = room_front(world);
        let mut spawn = |name: String,
                         min: Vec2,
                         size: Vec2,
                         pad: f32,
                         role: f32,
                         kind: f32,
                         z: f32,
                         block: Option<&ae::Block>| {
            let center = ae::Vec2::new(min.x + size.x * 0.5, min.y + size.y * 0.5);
            let mut quad = commands.spawn_session_scoped(
                scope,
                (
                    Mesh2d(quad.clone()),
                    MeshMaterial2d(materials.add(M::window(
                        Vec4::new(min.x, min.y, size.x, size.y),
                        Vec4::new(world.size.x, world.size.y, role, kind),
                        front,
                    ))),
                    Transform::from_translation(world_to_bevy(world, center, z))
                        .with_scale(Vec3::new(size.x + pad * 2.0, size.y + pad * 2.0, 1.0)),
                    Name::new(name),
                    RoomVisual,
                ),
            );
            // A quad that draws a block is a visual of that block: it leaves
            // when the block is removed, and it flinches when the block is
            // struck, as the block sprite does.
            if let Some(block) = block {
                quad.insert(BlockVisual {
                    block_name: block.name.clone(),
                    geo_id: block.id.clone(),
                });
            }
        };
        let room_size = Vec2::new(world.size.x, world.size.y);
        spawn(
            "room look backdrop".to_string(),
            Vec2::ZERO,
            room_size,
            BACKDROP_PAD,
            ROLE_BACKDROP,
            KIND_SOLID,
            BACKDROP_Z,
            None,
        );
        for block in &world.blocks {
            // Terrain only. A blink wall, a hazard or a pad says what it is
            // with its own art, and that art stays.
            let kind = match block.kind {
                ae::BlockKind::Solid => KIND_SOLID,
                ae::BlockKind::OneWay => KIND_ONE_WAY,
                _ => continue,
            };
            let half = block.aabb.half_size();
            let center = block.aabb.center();
            let min = Vec2::new(center.x - half.x, center.y - half.y);
            let size = Vec2::new(half.x * 2.0, half.y * 2.0);
            spawn(
                format!("room look surface: {}", block.name),
                min,
                size,
                SURFACE_PAD,
                ROLE_SURFACE,
                kind,
                SURFACE_Z,
                Some(block),
            );
            if size.y <= PLATFORM_MAX_HEIGHT && size.x >= 96.0 {
                spawn(
                    format!("room look underside: {}", block.name),
                    Vec2::new(min.x, min.y + size.y),
                    Vec2::new(size.x, UNDERSIDE_REACH),
                    0.0,
                    ROLE_UNDERSIDE,
                    kind,
                    UNDERSIDE_Z,
                    Some(block),
                );
            }
        }
        for zone in &spec.loading_zones {
            if !matches!(zone.activation, LoadingZoneActivation::Door) {
                continue;
            }
            let half = zone.aabb.half_size();
            let center = zone.aabb.center();
            spawn(
                format!("room look portal: {}", zone.name),
                Vec2::new(center.x - half.x, center.y - half.y),
                Vec2::new(half.x * 2.0, half.y * 2.0),
                PORTAL_PAD,
                ROLE_PORTAL,
                DOOR_SPRITE_ASPECT,
                PORTAL_Z,
                None,
            );
        }
        spawn(
            "room look overlay".to_string(),
            Vec2::ZERO,
            room_size,
            BACKDROP_PAD,
            ROLE_OVERLAY,
            KIND_SOLID,
            OVERLAY_Z,
            None,
        );
        commands.spawn_session_scoped(
            scope,
            (PresentedRoomLook, RoomVisual, Name::new("presented room look")),
        );
    }
}

/// Write the static signs of the pale side of the two-state look in ink.
///
/// A static label is white, which the clean sky hides. The placement pass
/// reads the colour from [`WorldLabel`], so this writes there, one time, when
/// the label appears. The copy of a label for a second view is a label that
/// appears also, so it gets its own ink and its own halo.
fn ink_labels_on_the_clean_side(
    mut commands: Commands,
    rooms: LiveRoomSpecs,
    mut labels: Query<
        (
            Entity,
            &mut WorldLabel,
            &Text2d,
            &TextFont,
            Option<&InRoomInstance>,
            Option<&MirroredWorldLabel>,
        ),
        (With<StaticWorldLabel>, Added<WorldLabel>),
    >,
    stamps: Query<&InRoomInstance>,
) {
    for (entity, mut label, text, font, stamp, copy) in &mut labels {
        let stamp = stamp.or_else(|| copy.and_then(|copy| stamps.get(copy.root).ok()));
        let Some(stamp) = stamp else {
            continue;
        };
        let Some((_, definition)) = rooms.live_rooms().find(|(room, _)| *room == stamp.0) else {
            continue;
        };
        let spec = rooms.rooms().spec(definition);
        if !asks_for::<RoomStateMaterial>(spec) {
            continue;
        }
        let world = &spec.world;
        let at = ae::config::bevy_size_to_world(world.size, ae::Vec2::new(label.anchor.x, label.anchor.y));
        // The air is pale until about here (`air_state` in the shader).
        if behind_front(world, at) > -60.0 {
            continue;
        }
        label.text_color = INK_TEXT;
        label.outline_color = Some(INK_HALO);
        commands.entity(entity).with_children(|parent| {
            for offset in [Vec2::X, -Vec2::X, Vec2::Y, -Vec2::Y] {
                parent.spawn((
                    text.clone(),
                    font.clone(),
                    TextColor(INK_HALO),
                    Transform::from_xyz(offset.x * 1.2, offset.y * 1.2, -0.1),
                ));
            }
        });
    }
}
