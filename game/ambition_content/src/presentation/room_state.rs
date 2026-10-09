//! A room drawn in two states of one architecture: clean and corrupted.
//!
//! A room asks for this look with its `palette` level field
//! ([`CLEAN_CORRUPTED_PALETTE`]). The look is presentation only. It reads the
//! room's blocks and draws over them; collision does not change.
//!
//! One material draws all of it. Each quad is a world-space window into the
//! same procedural scene, so a pan of the camera does not move the art on the
//! architecture. The quad's ROLE selects the layer:
//!
//! | role | quad | what it draws |
//! |---|---|---|
//! | backdrop | the room | sky, far towers, construction lines |
//! | surface | one block | masonry, cap, gold trim, inlay |
//! | underside | below one platform | brackets, arches, a banner, light leaks |
//! | overlay | the room | loose blocks that float off the corrupted mass |
//!
//! The corrupted state is DERIVED in the shader from the same construction as
//! the clean state. An arch keeps its outline because the two states are one
//! drawing. A scalar field in world space decides which state a point shows:
//! built things break into blocks at the front, open air changes as haze.
//!
//! The original block sprites stay under the surface quads. If the material
//! does not draw, the room looks as it did before.

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
use ambition_platformer2d_world::rooms::LiveRoomSpecs;
use ambition_render::rendering::label_layout::{MirroredWorldLabel, StaticWorldLabel, WorldLabel};
use ambition_render::rendering::RoomVisual;

/// The `palette` level-field value that asks for this look.
pub const CLEAN_CORRUPTED_PALETTE: &str = "clean_corrupted";

/// Above the parallax panels (`-18.0..=-15.0`), below the blocks.
const BACKDROP_Z: f32 = -14.5;
/// Behind the surfaces: ornament below a platform is not a place to stand.
const UNDERSIDE_Z: f32 = WORLD_Z_BLOCK + 0.1;
/// How far the ornament and the light leaks reach below a platform.
const UNDERSIDE_REACH: f32 = 230.0;
/// A block at most this tall is a platform, and gets an underside.
const PLATFORM_MAX_HEIGHT: f32 = 40.0;
/// Directly above the block sprite it replaces.
const SURFACE_Z: f32 = WORLD_Z_BLOCK + 0.2;
/// Above the surfaces, below climbables, water, doors and bodies.
const OVERLAY_Z: f32 = WORLD_Z_BLOCK + 3.0;
/// How far a surface quad reaches past its block, so that a corrupted block
/// can grow a cell past its edge.
const SURFACE_PAD: f32 = 16.0;
/// How far the backdrop reaches past the room, for a camera at the room edge.
const BACKDROP_PAD: f32 = 1200.0;

const ROLE_BACKDROP: f32 = 0.0;
const ROLE_SURFACE: f32 = 1.0;
const ROLE_OVERLAY: f32 = 2.0;
const ROLE_UNDERSIDE: f32 = 3.0;

/// Text on the pale side: ink, with a paper halo so that a sign that crosses
/// the front stays readable.
const INK_TEXT: Color = Color::srgba(0.165, 0.140, 0.250, 0.96);
const INK_HALO: Color = Color::srgba(0.965, 0.945, 0.900, 0.80);

#[derive(Resource, Default)]
struct RoomStateInstalled;

/// Marks a live room whose state look is spawned. It is stamped with the room,
/// so it leaves with the room, and the look is spawned again on a replay.
#[derive(Component)]
struct PresentedRoomState;

/// The unit quad every piece of the look scales.
#[derive(Resource)]
struct RoomStateQuad(Handle<Mesh>);

/// One window into the two-state scene.
///
/// All positions are in engine world coordinates (y down), which is where the
/// shader does its work.
#[derive(Asset, AsBindGroup, TypePath, Debug, Clone)]
pub struct RoomStateMaterial {
    /// The piece this quad draws: `min.x, min.y, size.x, size.y`.
    #[uniform(0)]
    pub piece: Vec4,
    /// `room.x, room.y` = the room size. `z` = the role. `w` = a seed.
    #[uniform(1)]
    pub room: Vec4,
    /// The front: a point on it (`x, y`) and its normal (`z, w`). The normal
    /// points into the corrupted side.
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

/// Install the material and the per-room spawner. Idempotent.
pub fn install(app: &mut App) {
    if app.world().contains_resource::<RoomStateInstalled>() {
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
    app.insert_resource(RoomStateInstalled);
    embedded_asset!(app, "shaders/room_state.wgsl");
    // The material plugin installs render-world state, so it needs an app
    // that renders. An asset registry is not evidence of a renderer.
    if app.get_sub_app(bevy::render::RenderApp).is_some() {
        app.add_plugins(Material2dPlugin::<RoomStateMaterial>::default());
    }
    app.add_systems(
        Update,
        present_room_states
            // With the room's own visuals, which this look draws over.
            .in_set(SessionScopeSet::Presentation)
            // Absent collections mean that this app does not draw.
            .run_if(resource_exists::<Assets<Mesh>>)
            .run_if(resource_exists::<Assets<RoomStateMaterial>>),
    );
    app.add_systems(
        Update,
        ink_labels_on_the_clean_side
            .run_if(resource_exists::<Assets<RoomStateMaterial>>),
    );
}

/// Whether a room asks for the clean/corrupted look.
fn asks_for_state_look(spec: &ambition_platformer2d_world::rooms::RoomSpec) -> bool {
    spec.metadata.visual_profile.palette.as_deref() == Some(CLEAN_CORRUPTED_PALETTE)
}

/// The front of a room: through the room centre, with a lean. The corrupted
/// side is on the right. A room has no authored front yet, so each room that
/// asks for the look gets the same one.
fn room_front(world: &ae::World) -> Vec4 {
    let normal = Vec2::new(1.0, 0.32).normalize();
    Vec4::new(world.size.x * 0.5, world.size.y * 0.5, normal.x, normal.y)
}

/// Give each live room that asks for the look its quads, stamped with the room.
fn present_room_states(
    mut commands: Commands,
    rooms: LiveRoomSpecs,
    presented: Query<&InRoomInstance, With<PresentedRoomState>>,
    quad: Option<Res<RoomStateQuad>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<RoomStateMaterial>>,
    active_session: Option<Res<ActiveSessionScope>>,
) {
    let Some(session_scope) =
        SessionSpawnScope::for_optional_active_session(active_session.as_deref())
    else {
        return;
    };
    for (room, definition) in rooms.live_rooms() {
        let spec = rooms.rooms().spec(definition);
        if !asks_for_state_look(spec) || presented.iter().any(|stamp| stamp.0 == room) {
            continue;
        }
        let quad = match quad.as_deref() {
            Some(quad) => quad.0.clone(),
            None => {
                let handle = meshes.add(Rectangle::new(1.0, 1.0));
                commands.insert_resource(RoomStateQuad(handle.clone()));
                handle
            }
        };
        let scope = session_scope.in_room(Some(room));
        let world = &spec.world;
        let front = room_front(world);
        let mut spawn = |name: String, min: Vec2, size: Vec2, pad: f32, role: f32, z: f32| {
            let center = ae::Vec2::new(min.x + size.x * 0.5, min.y + size.y * 0.5);
            commands.spawn_session_scoped(
                scope,
                (
                    Mesh2d(quad.clone()),
                    MeshMaterial2d(materials.add(RoomStateMaterial {
                        piece: Vec4::new(min.x, min.y, size.x, size.y),
                        room: Vec4::new(world.size.x, world.size.y, role, 0.0),
                        front,
                    })),
                    Transform::from_translation(world_to_bevy(world, center, z))
                        .with_scale(Vec3::new(size.x + pad * 2.0, size.y + pad * 2.0, 1.0)),
                    Name::new(name),
                    RoomVisual,
                ),
            );
        };
        let room_size = Vec2::new(world.size.x, world.size.y);
        spawn(
            "room state backdrop".to_string(),
            Vec2::ZERO,
            room_size,
            BACKDROP_PAD,
            ROLE_BACKDROP,
            BACKDROP_Z,
        );
        for block in &world.blocks {
            // Terrain only. A blink wall, a hazard or a pad says what it is
            // with its own art, and that art stays.
            if !matches!(block.kind, ae::BlockKind::Solid | ae::BlockKind::OneWay) {
                continue;
            }
            let half = block.aabb.half_size();
            let center = block.aabb.center();
            let min = Vec2::new(center.x - half.x, center.y - half.y);
            let size = Vec2::new(half.x * 2.0, half.y * 2.0);
            spawn(
                format!("room state surface: {}", block.name),
                min,
                size,
                SURFACE_PAD,
                ROLE_SURFACE,
                SURFACE_Z,
            );
            if size.y <= PLATFORM_MAX_HEIGHT && size.x >= 96.0 {
                spawn(
                    format!("room state underside: {}", block.name),
                    Vec2::new(min.x, min.y + size.y),
                    Vec2::new(size.x, UNDERSIDE_REACH),
                    0.0,
                    ROLE_UNDERSIDE,
                    UNDERSIDE_Z,
                );
            }
        }
        spawn(
            "room state overlay".to_string(),
            Vec2::ZERO,
            room_size,
            BACKDROP_PAD,
            ROLE_OVERLAY,
            OVERLAY_Z,
        );
        commands.spawn_session_scoped(
            scope,
            (PresentedRoomState, RoomVisual, Name::new("presented room state")),
        );
    }
}

/// Write the static signs of the pale side in ink.
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
        if !asks_for_state_look(spec) {
            continue;
        }
        let world = &spec.world;
        let front = room_front(world);
        let at = ae::config::bevy_size_to_world(world.size, ae::Vec2::new(label.anchor.x, label.anchor.y));
        let behind_front = (at.x - front.x) * front.z + (at.y - front.y) * front.w;
        // The air is pale until about here (`air_state` in the shader).
        if behind_front > -60.0 {
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
