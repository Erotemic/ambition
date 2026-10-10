//! Named looks for a room's architecture.
//!
//! A room asks for a look with its `palette` level field. A look is
//! presentation only. It reads the room's blocks and draws over them;
//! collision does not change.
//!
//! Each quad of a look is a world-space window into one scene, so a pan of
//! the camera does not move the art on the architecture. The quad's ROLE
//! selects the layer:
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
//! | `clean_corrupted` | One architecture in two states. The corrupted state is DERIVED from the same construction as the clean state, and a scalar field with a blocky front decides which state a point shows. |
//! | `debug_beautiful` | [`RoomBlueprintMaterial`]: the collision truth of the room, drawn as a drawing. A solid is a closed outline, a one-way platform has an open underside. |
//!
//! The architecture of the two-state look is not drawn by a shader each
//! frame. It is drawn one time for each room into textures
//! ([`architecture`] says what it looks like, [`plates`] makes the textures),
//! and [`RoomPlateMaterial`] shows them: surfaces, undersides and door frames.
//! [`RoomStateMaterial`] draws what is left, which moves or is not a thing of
//! the room: the sky, a blink wall's veil and the overlay.
//!
//! The front of the two-state look moves. The state of the room is a fact in
//! the save (`crate::room_look_state`: pure, balanced or corrupt), and
//! [`spread_room_states`] moves the front toward it, so a change of state
//! spreads across the room.
//!
//! A look also chooses the door of the room ([`RoomLook::door_art`]), with the
//! renderer's `EntityArt` seam.
//!
//! The original block sprites stay under the surface quads. If a material does
//! not draw, the room looks as it did before. That is also the fallback for a
//! device with no budget for a shader that fills the screen
//! ([`looks_are_in_budget`]).

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
use ambition_persistence::save::AmbitionGameSave;
use ambition_persistence::settings::ResolvedVisualQuality;
use ambition_platformer2d_world::rooms::{LiveRoomSpecs, LoadingZoneActivation, RoomSpec};
use ambition_render::rendering::label_layout::{MirroredWorldLabel, StaticWorldLabel, WorldLabel};
use ambition_render::rendering::{
    BlockVisual, EntityArt, LoadingZoneVisual, RoomVisual, DOOR_SPRITE_ASPECT,
};
use ambition_sprite_sheet::game_assets::{loading_zone_sprite, EntitySprite};

use crate::room_look_state::RoomLookState;

mod architecture;
mod plates;

pub use plates::RoomPlateMaterial;

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

/// How fast the front of the two-state look moves when the state of the room
/// changes, in world px per second. The hub is about 2400 px across its front,
/// so a change of state spreads across it in about four seconds.
const SPREAD_SPEED: f32 = 620.0;
/// How far past the last corner of the room the front goes for a state that
/// fills the room. The shader's front is ragged by about this much: its
/// wobble, its wisps, and the depth at which its largest blocks start.
const SPREAD_MARGIN: f32 = 640.0;

const ROLE_BACKDROP: f32 = 0.0;
const ROLE_SURFACE: f32 = 1.0;
const ROLE_OVERLAY: f32 = 2.0;
const ROLE_UNDERSIDE: f32 = 3.0;
const ROLE_PORTAL: f32 = 4.0;

/// The block kinds a look draws, as the shader reads them.
const KIND_SOLID: f32 = 0.0;
const KIND_ONE_WAY: f32 = 1.0;
const KIND_BLINK_SOFT: f32 = 2.0;
const KIND_BLINK_HARD: f32 = 3.0;

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

    /// Whether the architecture of this look (terrain surfaces, undersides,
    /// door frames) is drawn from plates ([`plates`]) and not by this
    /// material.
    const ARCHITECTURE_IS_PLATES: bool = false;

    /// Whether the sky of this look is authored parallax layers
    /// ([`RoomSkyMaterial`]) and not drawn by this material.
    const SKY_IS_AUTHORED_LAYERS: bool = false;

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

    /// The door this look gives to a door at `at` in `world`, while the
    /// front of the room has gone `advance` px into its clean side.
    fn door_art(world: &ae::World, at: ae::Vec2, advance: f32) -> EntitySprite;
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

/// The corrupted sky of the two-state look, over the clean one.
///
/// The sky is authored: two parallax themes of four layers each
/// (`ParallaxTheme::HubClean`, `ParallaxTheme::HubCorrupt`), made by the
/// parallax renderer (`room_look_sky.py`). A room with the look names
/// `hub_clean` as its `parallax_theme`, and the parallax system draws it.
/// This material is one quad over that: it lays the layers of `hub_corrupt`
/// where the air of the room is corrupted, and the fog between the sky and
/// the play. Its shader draws no art.
#[derive(Asset, AsBindGroup, TypePath, Debug, Clone)]
pub struct RoomSkyMaterial {
    /// `x, y` = the room size.
    #[uniform(0)]
    pub room: Vec4,
    /// A point on the room's front (`x, y`) and its normal (`z, w`), which
    /// points into the corrupted side.
    #[uniform(1)]
    pub front: Vec4,
    /// The fog ([`RoomLookDepth::uniform`]).
    #[uniform(2)]
    pub depth: Vec4,
    #[texture(3)]
    #[sampler(4)]
    pub sky: Handle<Image>,
    #[texture(5)]
    pub far: Handle<Image>,
    #[texture(6)]
    pub near: Handle<Image>,
    #[texture(7)]
    pub atmosphere: Handle<Image>,
}

impl Material2d for RoomSkyMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://ambition_content/presentation/shaders/room_sky.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }
}

/// How far behind the play the sky of the two-state look is drawn.
///
/// The play is sharp and clear. The sky is behind it, and it reads so when it
/// is a little out of focus and has fog in front of it. The blur is in the
/// art (`BLUR` in the parallax renderer's `room_look_sky.py`: change it there
/// and regenerate). The fog is drawn over the sky, and these are its numbers,
/// to tune by eye: the developer inspector shows them (`ambition_app::dev`),
/// `capture_scene --look-depth` photographs them, and each change is drawn in
/// the next frame.
#[derive(Resource, Reflect, Debug, Clone, Copy, PartialEq)]
#[reflect(Resource)]
pub struct RoomLookDepth {
    /// How much fog is in front of the sky, from 0 (none) to 1.
    pub fog: f32,
    /// How much the fog is in patches, from 0 (even) to 1.
    pub fog_patches: f32,
}

impl Default for RoomLookDepth {
    fn default() -> Self {
        Self { fog: 0.14, fog_patches: 0.5 }
    }
}

impl RoomLookDepth {
    /// The numbers as the shader reads them (`depth` in `room_sky.wgsl`).
    pub fn uniform(&self) -> Vec4 {
        Vec4::new(self.fog.clamp(0.0, 1.0), self.fog_patches.clamp(0.0, 1.0), 0.0, 0.0)
    }
}

/// Tell the sky of each two-state room how much fog is in front of it: when
/// the numbers change, and when a sky comes.
fn apply_look_depth(
    depth: Res<RoomLookDepth>,
    new_quads: Query<(), Added<MeshMaterial2d<RoomSkyMaterial>>>,
    quads: Query<&MeshMaterial2d<RoomSkyMaterial>>,
    mut materials: ResMut<Assets<RoomSkyMaterial>>,
) {
    if !depth.is_changed() && new_quads.is_empty() {
        return;
    }
    let wanted = depth.uniform();
    for quad in &quads {
        // Read first: a write marks the material for the renderer again.
        if materials.get(&quad.0).is_some_and(|material| material.depth != wanted) {
            if let Some(mut material) = materials.get_mut(&quad.0) {
                material.depth = wanted;
            }
        }
    }
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
    const ARCHITECTURE_IS_PLATES: bool = true;
    const SKY_IS_AUTHORED_LAYERS: bool = true;

    fn window(piece: Vec4, room: Vec4, front: Vec4) -> Self {
        Self { piece, room, front }
    }

    fn door_art(world: &ae::World, at: ae::Vec2, advance: f32) -> EntitySprite {
        if behind_front(world, at, advance) > 0.0 {
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

    fn door_art(_world: &ae::World, _at: ae::Vec2, _advance: f32) -> EntitySprite {
        EntitySprite::DoorBlueprint
    }
}

#[derive(Resource, Default)]
struct RoomLookInstalled;

/// Where the front of a two-state room is now: how far it has gone into the
/// clean side from the centre of the room, in world px. Negative is into the
/// corrupted side. One for each live room with a look, on an entity of its
/// own; a look with one state does not read it.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
struct RoomLookSpread {
    advance: f32,
}

/// The ink of a static sign on the pale side: what the sign looked like
/// before, and the halo that the ink added.
#[derive(Component)]
struct LabelInk {
    text_color: Color,
    outline_color: Option<Color>,
    halo: [Entity; 4],
}

/// Marks each entity of a spawned look. It is stamped with the room, so it
/// leaves with the room, and the look is spawned again on a replay. A live
/// room with no such entity has no look.
#[derive(Component)]
struct PresentedRoomLook;

/// Whether this device draws a room look.
///
/// A look is a shader that fills the screen, two times. The quality budget
/// says if a device draws such shaders (the Potato tier does not), and the
/// screen filter reads the same number.
fn looks_are_in_budget(quality: Option<Res<ResolvedVisualQuality>>) -> bool {
    draws_looks(quality.as_deref())
}

/// [`looks_are_in_budget`], for a system that runs in each budget.
fn draws_looks(quality: Option<&ResolvedVisualQuality>) -> bool {
    quality.is_none_or(|quality| quality.budget.shaders.screen_shader_scale > 0.0)
}

/// Take each look away when the budget for it goes. The block sprites below
/// are then the room.
///
/// A look also dresses things that are not its own: a door's art and a sign's
/// ink. Those are undressed by the systems that dress them ([`dress_doors`],
/// [`ink_labels_on_the_clean_side`]), which run in each budget for that
/// reason. With them gated on the budget, a room that was presented when the
/// tier changed kept its look's doors and its inked signs over plain blocks.
fn retire_looks_out_of_budget(
    mut commands: Commands,
    pieces: Query<Entity, With<PresentedRoomLook>>,
) {
    for piece in &pieces {
        commands.entity(piece).try_despawn();
    }
}

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
    embedded_asset!(app, "shaders/room_plate.wgsl");
    embedded_asset!(app, "shaders/room_sky.wgsl");
    embedded_asset!(app, "shaders/room_blueprint.wgsl");
    // `ambition_content::room_look`: what the looks share. A composition with
    // no shader assets draws no look to import it.
    if app.world().contains_resource::<Assets<bevy::shader::Shader>>() {
        bevy::shader::load_shader_library!(app, "shaders/room_look_common.wgsl");
    }
    if app.get_sub_app(bevy::render::RenderApp).is_some() {
        app.add_plugins(Material2dPlugin::<RoomPlateMaterial>::default());
        app.add_plugins(Material2dPlugin::<RoomSkyMaterial>::default());
    }
    // The unit quad is made now and not with the first room. A mesh that goes
    // to the GPU in a frame whose upload budget is spent arrives late, and
    // Bevy does not try again to draw a quad whose MESH was late (it does for
    // a late material). Each quad of the look would stay undrawn.
    if let Some(mut meshes) = app.world_mut().get_resource_mut::<Assets<Mesh>>() {
        let quad = meshes.add(Rectangle::new(1.0, 1.0));
        app.insert_resource(RoomLookQuad(quad));
    }
    app.init_resource::<RoomPlates>().add_systems(
        Update,
        prepare_room_plates
            .before(SessionScopeSet::Presentation)
            .run_if(resource_exists::<Assets<Image>>)
            .run_if(resource_exists::<Assets<RoomPlateMaterial>>)
            .run_if(looks_are_in_budget),
    );
    install_look::<RoomStateMaterial>(app);
    install_look::<RoomBlueprintMaterial>(app);
    app.init_resource::<RoomLookDepth>()
        .register_type::<RoomLookDepth>()
        .add_systems(Update, apply_look_depth.run_if(resource_exists::<Assets<RoomSkyMaterial>>));
    app.add_systems(
        Update,
        (
            (
                spread_room_states.run_if(looks_are_in_budget),
                // In each budget: out of budget it gives each sign its own
                // colours back.
                ink_labels_on_the_clean_side,
            )
                .chain()
                .run_if(resource_exists::<Assets<RoomStateMaterial>>),
            retire_looks_out_of_budget.run_if(not(looks_are_in_budget)),
        ),
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
            .run_if(resource_exists::<Assets<M>>)
            .run_if(looks_are_in_budget),
    );
    // In each budget: out of budget it gives each door its plain art back.
    app.add_systems(Update, dress_doors::<M>.run_if(resource_exists::<Assets<M>>));
}

/// Whether a room asks for the look `M`.
fn asks_for<M: RoomLook>(spec: &RoomSpec) -> bool {
    spec.metadata.visual_profile.palette.as_deref() == Some(M::PALETTE)
}

/// The normal of a room's front: it points into the corrupted side.
fn front_normal() -> Vec2 {
    Vec2::new(1.0, 0.32).normalize()
}

/// The front of a room: a line with a lean, `advance` px into the clean side
/// from the centre of the room. A room has no authored front yet, so each
/// room that asks for a look gets the same one.
fn room_front(world: &ae::World, advance: f32) -> Vec4 {
    let normal = front_normal();
    let at = Vec2::new(world.size.x, world.size.y) * 0.5 - normal * advance;
    Vec4::new(at.x, at.y, normal.x, normal.y)
}

/// How far the front goes from the centre of `world` for a state that fills
/// the room: past its last corner, and past the ragged edge of the front.
fn full_advance(world: &ae::World) -> f32 {
    let normal = front_normal();
    0.5 * (normal.x.abs() * world.size.x + normal.y.abs() * world.size.y) + SPREAD_MARGIN
}

/// Where the front of the room `spec` wants to be, by the state the save
/// records for it. A composition with no save has balanced rooms.
fn wanted_advance(spec: &RoomSpec, save: Option<&AmbitionGameSave>) -> f32 {
    let state = save.map_or(RoomLookState::Balanced, |save| RoomLookState::of(save, &spec.id));
    state.level() * full_advance(&spec.world)
}

/// How far `at` is behind the front of `world`, in world px, while the front
/// has gone `advance` px into the clean side. Positive is the corrupted side
/// of the two-state look.
///
/// The shader is the authority on the front (`field` in `room_state.wgsl`).
/// This is its part that does not move by itself: the plane and the large
/// wobble. The part that moves is at most 65 px, so a sign or a door that
/// close to the front can be dressed for the other side.
fn behind_front(world: &ae::World, at: ae::Vec2, advance: f32) -> f32 {
    let front = room_front(world, advance);
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
/// The door of a two-state room follows the front, so this asks again each
/// frame and writes only a door that changes.
///
/// With no budget for the look, a door this dressed gets the art of its kind
/// back: the look is all or nothing, and its door on plain blocks is half.
fn dress_doors<M: RoomLook>(
    mut commands: Commands,
    quality: Option<Res<ResolvedVisualQuality>>,
    rooms: LiveRoomSpecs,
    spreads: Query<(&InRoomInstance, &RoomLookSpread)>,
    doors: Query<(Entity, &LoadingZoneVisual, &InRoomInstance, Option<&EntityArt>)>,
) {
    for (entity, visual, stamp, dressed) in &doors {
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
        let art = if draws_looks(quality.as_deref()) {
            EntityArt(M::door_art(&spec.world, zone.aabb.center(), advance_of(&spreads, stamp)))
        } else if dressed.is_some() {
            EntityArt(loading_zone_sprite(zone.activation))
        } else {
            // Never dressed: the door has the art of its kind already.
            continue;
        };
        if dressed != Some(&art) {
            // `try_insert`: a room visual can leave before the command flush.
            commands.entity(entity).try_insert(art);
        }
    }
}

/// Where the front of the live room `stamp` is now. A room with no spread
/// record (its look is not spawned) has its front at the centre.
fn advance_of(spreads: &Query<(&InRoomInstance, &RoomLookSpread)>, stamp: &InRoomInstance) -> f32 {
    spreads
        .iter()
        .find(|(room, _)| room.0 == stamp.0)
        .map_or(0.0, |(_, spread)| spread.advance)
}

/// Move the front of each two-state room toward the state the save records
/// for it, and tell the room's quads where it is.
fn spread_room_states(
    time: Res<Time>,
    save: Option<Res<AmbitionGameSave>>,
    rooms: LiveRoomSpecs,
    mut spreads: Query<(&InRoomInstance, &mut RoomLookSpread)>,
    quads: Query<(&InRoomInstance, &MeshMaterial2d<RoomStateMaterial>)>,
    mut materials: ResMut<Assets<RoomStateMaterial>>,
    plate_quads: Query<(&InRoomInstance, &MeshMaterial2d<RoomPlateMaterial>)>,
    mut plate_materials: Option<ResMut<Assets<RoomPlateMaterial>>>,
    sky_quads: Query<(&InRoomInstance, &MeshMaterial2d<RoomSkyMaterial>)>,
    mut sky_materials: Option<ResMut<Assets<RoomSkyMaterial>>>,
) {
    for (stamp, mut spread) in &mut spreads {
        let Some((_, definition)) = rooms.live_rooms().find(|(room, _)| *room == stamp.0) else {
            continue;
        };
        let spec = rooms.rooms().spec(definition);
        if !asks_for::<RoomStateMaterial>(spec) {
            continue;
        }
        let wanted = wanted_advance(spec, save.as_deref());
        if spread.advance == wanted {
            continue;
        }
        let step = SPREAD_SPEED * time.delta_secs();
        spread.advance += (wanted - spread.advance).clamp(-step, step);
        let front = room_front(&spec.world, spread.advance);
        for (quad_stamp, material) in &quads {
            if quad_stamp.0 != stamp.0 {
                continue;
            }
            if let Some(mut material) = materials.get_mut(&material.0) {
                material.front = front;
            }
        }
        if let Some(sky_materials) = sky_materials.as_deref_mut() {
            for (quad_stamp, material) in &sky_quads {
                if quad_stamp.0 != stamp.0 {
                    continue;
                }
                if let Some(mut material) = sky_materials.get_mut(&material.0) {
                    material.front = front;
                }
            }
        }
        let Some(plate_materials) = plate_materials.as_deref_mut() else {
            continue;
        };
        for (quad_stamp, material) in &plate_quads {
            if quad_stamp.0 != stamp.0 {
                continue;
            }
            if let Some(mut material) = plate_materials.get_mut(&material.0) {
                material.front = front;
            }
        }
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
    save: Option<Res<AmbitionGameSave>>,
    active_session: Option<Res<ActiveSessionScope>>,
    spreads: Query<(&InRoomInstance, &RoomLookSpread)>,
    plate_quads: Query<&InRoomInstance, With<MeshMaterial2d<RoomPlateMaterial>>>,
    plates: Option<Res<RoomPlates>>,
    mut plate_materials: Option<ResMut<Assets<RoomPlateMaterial>>>,
    mut sky: SkyLayers,
) {
    let Some(session_scope) =
        SessionSpawnScope::for_optional_active_session(active_session.as_deref())
    else {
        return;
    };
    for (room, definition) in rooms.live_rooms() {
        let spec = rooms.rooms().spec(definition);
        if !asks_for::<M>(spec) {
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
        // The plates of the room come when they are drawn, which can be after
        // the rest of its look.
        if M::ARCHITECTURE_IS_PLATES && !plate_quads.iter().any(|stamp| stamp.0 == room) {
            let ready = plates.as_deref().and_then(|plates| plates.ready(spec));
            if let (Some(ready), Some(materials)) = (ready, plate_materials.as_deref_mut()) {
                // Where the front is now, or where a room that comes this
                // frame starts.
                let advance = spreads
                    .iter()
                    .find(|(stamp, _)| stamp.0 == room)
                    .map_or_else(|| wanted_advance(spec, save.as_deref()), |(_, spread)| spread.advance);
                spawn_plates(&mut commands, scope, spec, room_front(world, advance), &quad, ready, materials);
            }
        }
        if presented.iter().any(|stamp| stamp.0 == room) {
            continue;
        }
        // A room starts in the state the save records: the front moves only
        // when the state changes while the room is live.
        let advance = wanted_advance(spec, save.as_deref());
        let front = room_front(world, advance);
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
                    PresentedRoomLook,
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
        if !M::SKY_IS_AUTHORED_LAYERS {
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
        }
        for block in &world.blocks {
            let Some(kind) = look_kind(block) else {
                continue;
            };
            let (min, size) = block_rect(block);
            let is_terrain = kind == KIND_SOLID || kind == KIND_ONE_WAY;
            if M::ARCHITECTURE_IS_PLATES && is_terrain {
                // Drawn from a plate ([`architecture_of`]).
                continue;
            }
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
            if is_terrain && is_platform(size) {
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
            if M::ARCHITECTURE_IS_PLATES {
                // Drawn from a plate ([`architecture_of`]).
                continue;
            }
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
        if M::SKY_IS_AUTHORED_LAYERS {
            if let Some(material) = sky.corrupted_over_clean(room_size, front) {
                let size = room_size + Vec2::splat(BACKDROP_PAD * 2.0);
                commands.spawn_session_scoped(
                    scope,
                    (
                        Mesh2d(quad.clone()),
                        MeshMaterial2d(material),
                        Transform::from_translation(world_to_bevy(
                            world,
                            ae::Vec2::new(room_size.x * 0.5, room_size.y * 0.5),
                            BACKDROP_Z,
                        ))
                        .with_scale(Vec3::new(size.x, size.y, 1.0)),
                        Name::new("room look sky"),
                        RoomVisual,
                        PresentedRoomLook,
                    ),
                );
            }
        }
        commands.spawn_session_scoped(
            scope,
            (
                RoomLookSpread { advance },
                PresentedRoomLook,
                RoomVisual,
                Name::new("room look spread"),
            ),
        );
    }
}

/// What a look needs to lay its authored corrupted sky over the clean one. A
/// composition that does not render, or that has no art on disk, has none of
/// it and draws no sky of its own: the room's parallax theme is its sky.
#[derive(bevy::ecs::system::SystemParam)]
struct SkyLayers<'w> {
    assets: Option<Res<'w, ambition_sprite_sheet::game_assets::GameAssets>>,
    materials: Option<ResMut<'w, Assets<RoomSkyMaterial>>>,
    depth: Option<Res<'w, RoomLookDepth>>,
}

impl SkyLayers<'_> {
    /// The material of the sky quad of a room of `room_size`: the four layers
    /// of the corrupted theme. `None` when a layer is not to be had.
    fn corrupted_over_clean(&mut self, room_size: Vec2, front: Vec4) -> Option<Handle<RoomSkyMaterial>> {
        use ambition_sprite_sheet::game_assets::{ParallaxLayerAsset, ParallaxTheme};
        let (assets, materials) = (self.assets.as_deref()?, self.materials.as_deref_mut()?);
        // The room's theme names this one as its corrupted state, and the
        // loader of the room's theme loads it and keeps it
        // (`ParallaxTheme::corrupted`). This only reads it.
        let layer = |asset| assets.parallax_layers.get(ParallaxTheme::HubCorrupt, asset).cloned();
        Some(materials.add(RoomSkyMaterial {
            room: Vec4::new(room_size.x, room_size.y, 0.0, 0.0),
            front,
            depth: self.depth.as_deref().copied().unwrap_or_default().uniform(),
            sky: layer(ParallaxLayerAsset::Sky)?,
            far: layer(ParallaxLayerAsset::FarBackplate)?,
            near: layer(ParallaxLayerAsset::NearBackground)?,
            atmosphere: layer(ParallaxLayerAsset::ForegroundAtmosphere)?,
        }))
    }
}

/// The block kind a look draws `block` as, if it draws it. Terrain and blink
/// walls. A look must keep a blink wall a thing of its own, not stone: a
/// blink goes through it. A hazard or a pad says what it is with its own art,
/// and that art stays.
fn look_kind(block: &ae::Block) -> Option<f32> {
    Some(match block.kind {
        ae::BlockKind::Solid => KIND_SOLID,
        ae::BlockKind::OneWay => KIND_ONE_WAY,
        ae::BlockKind::BlinkWall { tier: ae::BlinkWallTier::Soft } => KIND_BLINK_SOFT,
        ae::BlockKind::BlinkWall { tier: ae::BlinkWallTier::Hard } => KIND_BLINK_HARD,
        _ => return None,
    })
}

/// The upper left corner and the size of `block`.
fn block_rect(block: &ae::Block) -> (Vec2, Vec2) {
    let half = block.aabb.half_size();
    let center = block.aabb.center();
    (Vec2::new(center.x - half.x, center.y - half.y), Vec2::new(half.x * 2.0, half.y * 2.0))
}

/// Whether a terrain block of `size` is a platform, which has an underside.
fn is_platform(size: Vec2) -> bool {
    size.y <= PLATFORM_MAX_HEIGHT && size.x >= 96.0
}

/// One piece of the architecture of a room: the block it is the look of (its
/// index in the room's blocks), if it is one, and a name.
struct ArchitecturePiece {
    piece: architecture::Piece,
    block: Option<usize>,
    name: String,
}

/// The architecture of `spec` that the two-state look draws from plates: the
/// surface of each terrain block, the underside of each platform, and the
/// frame of each door.
fn architecture_of(spec: &RoomSpec) -> Vec<ArchitecturePiece> {
    use architecture::{Part, Piece};
    let mut pieces = Vec::new();
    for (index, block) in spec.world.blocks.iter().enumerate() {
        if !matches!(look_kind(block), Some(kind) if kind == KIND_SOLID || kind == KIND_ONE_WAY) {
            continue;
        }
        let (min, size) = block_rect(block);
        pieces.push(ArchitecturePiece {
            piece: Piece { part: Part::Surface, min, size },
            block: Some(index),
            name: format!("room look surface: {}", block.name),
        });
        if is_platform(size) {
            pieces.push(ArchitecturePiece {
                piece: Piece {
                    part: Part::Underside,
                    min: Vec2::new(min.x, min.y + size.y),
                    size: Vec2::new(size.x, UNDERSIDE_REACH),
                },
                block: Some(index),
                name: format!("room look underside: {}", block.name),
            });
        }
    }
    for zone in &spec.loading_zones {
        if !matches!(zone.activation, LoadingZoneActivation::Door) {
            continue;
        }
        let half = zone.aabb.half_size();
        let center = zone.aabb.center();
        pieces.push(ArchitecturePiece {
            piece: Piece {
                part: Part::Door { aspect: DOOR_SPRITE_ASPECT },
                min: Vec2::new(center.x - half.x, center.y - half.y),
                size: Vec2::new(half.x * 2.0, half.y * 2.0),
            },
            block: None,
            name: format!("room look portal: {}", zone.name),
        });
    }
    pieces
}

/// The pieces of [`architecture_of`], with no names: what a plate is drawn
/// from, and what says that a plate is of the room as it is now.
fn architecture_pieces(spec: &RoomSpec) -> Vec<architecture::Piece> {
    architecture_of(spec).into_iter().map(|piece| piece.piece).collect()
}

/// The pages of a room, drawn and not yet textures.
struct DrawnPlates {
    placed: Vec<plates::Placed>,
    pages: Vec<(UVec2, Vec<u8>)>,
}

/// The plates of a room that are drawn.
struct ReadyPlates {
    placed: Vec<plates::Placed>,
    pages: Vec<Handle<Image>>,
}

enum PlateWork {
    Drawing(bevy::tasks::Task<DrawnPlates>),
    Ready(ReadyPlates),
}

/// The plates of one room, and the pieces they were drawn from.
struct RoomPlateSet {
    pieces: Vec<architecture::Piece>,
    work: PlateWork,
}

/// The plates of the two-state rooms that are live and of the two-state rooms
/// next to them, by room id.
///
/// A room's plates are drawn before the room comes, while the room next to it
/// is played, so that the room comes with its look and with no frame that
/// pays for the drawing. The drawing is a task off the main thread. A room
/// that comes with no plate (the first room of a session, a room that a
/// warp reaches) shows its block sprites until its plates are drawn.
///
/// The set is of the session's rooms as they are: a room that is not live and
/// not next to a live room has no entry, and an entry whose room has other
/// pieces now (a level that was loaded again) is drawn again.
#[derive(Resource, Default)]
struct RoomPlates {
    by_room: std::collections::HashMap<String, RoomPlateSet>,
}

impl RoomPlates {
    /// The plates of `spec`, if they are drawn.
    fn ready(&self, spec: &RoomSpec) -> Option<&ReadyPlates> {
        match &self.by_room.get(&spec.id)?.work {
            PlateWork::Ready(ready) => Some(ready),
            PlateWork::Drawing(_) => None,
        }
    }
}

/// Draw the plates of each two-state room that is live or next to a live
/// room, and forget the plates of each other room. See [`RoomPlates`].
fn prepare_room_plates(
    rooms: LiveRoomSpecs,
    mut plates: ResMut<RoomPlates>,
    mut images: ResMut<Assets<Image>>,
    device: Option<Res<bevy::render::renderer::RenderDevice>>,
) {
    let set = rooms.rooms();
    let mut wanted: Vec<(&RoomSpec, Vec<architecture::Piece>)> = Vec::new();
    for (_, definition) in rooms.live_rooms() {
        let near = set.neighboring_room_indices_of(definition.index());
        for index in std::iter::once(definition.index()).chain(near) {
            let Some(spec) = set.definition(index).map(|definition| set.spec(definition)) else {
                continue;
            };
            if asks_for::<RoomStateMaterial>(spec) && !wanted.iter().any(|(known, _)| known.id == spec.id) {
                wanted.push((spec, architecture_pieces(spec)));
            }
        }
    }
    plates
        .by_room
        .retain(|id, set| wanted.iter().any(|(spec, pieces)| &spec.id == id && *pieces == set.pieces));
    let page = device
        .as_deref()
        .map_or(plates::PAGE_MAX, |device| device.limits().max_texture_dimension_2d.min(plates::PAGE_MAX));
    for (spec, pieces) in wanted {
        if plates.by_room.contains_key(&spec.id) || pieces.is_empty() {
            continue;
        }
        let art = pieces.clone();
        let task = bevy::tasks::AsyncComputeTaskPool::get_or_init(bevy::tasks::TaskPool::default).spawn(async move {
            let windows = plates::windows_of(&art, page);
            let (placed, sizes) = plates::pack(&windows, page);
            let data = plates::draw_pages(&art, &placed, &sizes);
            DrawnPlates {
                placed,
                pages: sizes.into_iter().zip(data).collect(),
            }
        });
        plates.by_room.insert(spec.id.clone(), RoomPlateSet { pieces, work: PlateWork::Drawing(task) });
    }
    for (id, set) in &mut plates.by_room {
        let PlateWork::Drawing(task) = &mut set.work else {
            continue;
        };
        let Some(drawn) = bevy::tasks::futures::check_ready(task) else {
            continue;
        };
        let texels: u32 = drawn.pages.iter().map(|(size, _)| size.x * size.y).sum();
        info!(
            "room look plates of '{id}': {} pieces in {} quads on {} pages, {:.1} Mpx, drawn off the main thread",
            set.pieces.len(),
            drawn.placed.len(),
            drawn.pages.len(),
            texels as f32 / 1.0e6,
        );
        let pages = drawn.pages.into_iter().map(|(size, data)| images.add(plates::page_image(size, data))).collect();
        set.work = PlateWork::Ready(ReadyPlates { placed: drawn.placed, pages });
    }
}

/// The most that [`plate_lifts`] adds to the depth of a piece. The roles of a
/// look are 0.05 apart.
const PLATE_LIFT_MAX: f32 = 0.04;

/// A small depth for each piece, all different: a piece that is higher in the
/// room is nearer to the eye.
///
/// The art of a piece goes past its block (ivy hangs from a corner), so two
/// pieces that are near each other overlap. With one depth for both, the
/// renderer drew them in an order that was not the same in each frame, and
/// the ivy flickered. Ivy hangs down, so the piece above is the one in front.
fn plate_lifts(pieces: &[ArchitecturePiece]) -> Vec<f32> {
    let mut order: Vec<usize> = (0..pieces.len()).collect();
    // The lowest piece first (y is down), and from the left in one row.
    order.sort_by(|&a, &b| {
        let (a, b) = (pieces[a].piece.min, pieces[b].piece.min);
        b.y.total_cmp(&a.y).then(a.x.total_cmp(&b.x))
    });
    let mut lift = vec![0.0; pieces.len()];
    for (rank, &index) in order.iter().enumerate() {
        lift[index] = PLATE_LIFT_MAX * rank as f32 / pieces.len().max(1) as f32;
    }
    lift
}

/// Give the room `spec` one quad for each window of its plates.
fn spawn_plates(
    commands: &mut Commands,
    scope: SessionSpawnScope,
    spec: &RoomSpec,
    front: Vec4,
    quad: &Handle<Mesh>,
    ready: &ReadyPlates,
    materials: &mut Assets<RoomPlateMaterial>,
) {
    let pieces = architecture_of(spec);
    let lift = plate_lifts(&pieces);
    for cell in &ready.placed {
        let ArchitecturePiece { piece, block, name } = &pieces[cell.window.piece];
        let (role, z) = match piece.part {
            architecture::Part::Surface => (ROLE_SURFACE, SURFACE_Z),
            architecture::Part::Underside => (ROLE_UNDERSIDE, UNDERSIDE_Z),
            architecture::Part::Door { .. } => (ROLE_PORTAL, PORTAL_Z),
        };
        let z = z + lift[cell.window.piece];
        let held = cell.window.size.as_vec2();
        let extent = cell.window.extent;
        let centre = cell.window.min + extent * 0.5;
        let corrupt = cell.corrupt();
        let mut quad = commands.spawn_session_scoped(
            scope,
            (
                Mesh2d(quad.clone()),
                MeshMaterial2d(materials.add(RoomPlateMaterial {
                    piece: Vec4::new(piece.min.x, piece.min.y, piece.size.x, piece.size.y),
                    window: Vec4::new(cell.window.min.x, cell.window.min.y, extent.x, extent.y),
                    kind: Vec4::new(role, held.x, held.y, 0.0),
                    front,
                    cells: Vec4::new(cell.clean.x as f32, cell.clean.y as f32, corrupt.x as f32, corrupt.y as f32),
                    plate: ready.pages[cell.page].clone(),
                })),
                Transform::from_translation(world_to_bevy(&spec.world, ae::Vec2::new(centre.x, centre.y), z))
                    .with_scale(Vec3::new(extent.x, extent.y, 1.0)),
                Name::new(name.clone()),
                RoomVisual,
                PresentedRoomLook,
            ),
        );
        // As a quad that draws a block with a shader: it leaves when the
        // block is removed, and it flinches when the block is struck.
        if let Some(block) = block.map(|index| &spec.world.blocks[index]) {
            quad.insert(BlockVisual {
                block_name: block.name.clone(),
                geo_id: block.id.clone(),
            });
        }
    }
}

/// Write the static signs of the pale side of the two-state look in ink.
///
/// A static label is white, which the clean sky hides. The placement pass
/// reads the colour from [`WorldLabel`], so this writes there. The front
/// moves, so each sign is asked again each frame: a sign the front passes
/// gets its ink, or gets back the colours it had ([`LabelInk`]). The copy of
/// a label for a second view is a label also, with its own ink and halo.
fn ink_labels_on_the_clean_side(
    mut commands: Commands,
    quality: Option<Res<ResolvedVisualQuality>>,
    rooms: LiveRoomSpecs,
    spreads: Query<(&InRoomInstance, &RoomLookSpread)>,
    mut labels: Query<
        (
            Entity,
            &mut WorldLabel,
            &Text2d,
            &TextFont,
            Option<&InRoomInstance>,
            Option<&MirroredWorldLabel>,
            Option<&LabelInk>,
        ),
        With<StaticWorldLabel>,
    >,
    stamps: Query<&InRoomInstance>,
) {
    // With no budget no look is drawn and no air is pale: each sign that has
    // ink gets its own colours back, by the arm that takes ink off below.
    let in_budget = draws_looks(quality.as_deref());
    for (entity, mut label, text, font, stamp, copy, ink) in &mut labels {
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
        let pale = in_budget && behind_front(world, at, advance_of(&spreads, stamp)) <= -60.0;
        match (pale, ink) {
            (true, None) => {
                let halo = [Vec2::X, -Vec2::X, Vec2::Y, -Vec2::Y].map(|offset| {
                    commands
                        .spawn((
                            text.clone(),
                            font.clone(),
                            TextColor(INK_HALO),
                            Transform::from_xyz(offset.x * 1.2, offset.y * 1.2, -0.1),
                            ChildOf(entity),
                        ))
                        .id()
                });
                commands.entity(entity).try_insert(LabelInk {
                    text_color: label.text_color,
                    outline_color: label.outline_color,
                    halo,
                });
                label.text_color = INK_TEXT;
                label.outline_color = Some(INK_HALO);
            }
            (false, Some(ink)) => {
                label.text_color = ink.text_color;
                label.outline_color = ink.outline_color;
                for halo in ink.halo {
                    commands.entity(halo).try_despawn();
                }
                commands.entity(entity).try_remove::<LabelInk>();
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// No two pieces have one depth, and a piece that is higher in the room
    /// is in front, in each role: the order two pieces that overlap are drawn
    /// in does not change from frame to frame.
    #[test]
    fn each_plate_has_a_depth_of_its_own_and_a_higher_piece_is_in_front() {
        let piece = |x: f32, y: f32| ArchitecturePiece {
            piece: architecture::Piece {
                part: architecture::Part::Surface,
                min: Vec2::new(x, y),
                size: Vec2::new(128.0, 32.0),
            },
            block: None,
            name: String::new(),
        };
        let pieces = vec![piece(0.0, 400.0), piece(64.0, 380.0), piece(300.0, 400.0), piece(0.0, 100.0)];
        let lift = plate_lifts(&pieces);
        let mut sorted = lift.clone();
        sorted.sort_by(f32::total_cmp);
        sorted.dedup();
        assert_eq!(sorted.len(), pieces.len(), "two pieces share a depth: {lift:?}");
        assert!(lift.iter().all(|lift| (0.0..PLATE_LIFT_MAX).contains(lift)), "{lift:?}");
        assert!(lift[1] > lift[0] && lift[3] > lift[1], "a higher piece is not in front: {lift:?}");
    }

    /// The numbers of the fog reach the sky of the look: a sky that comes
    /// gets the numbers of the session, and a change of the numbers reaches
    /// the sky that is there. This is the road a developer tunes by.
    #[test]
    fn the_fog_numbers_reach_a_sky_that_comes_and_a_sky_that_is_there() {
        let mut app = App::new();
        app.init_resource::<Assets<RoomSkyMaterial>>()
            .insert_resource(RoomLookDepth { fog: 0.4, fog_patches: 0.1 })
            .add_systems(Update, apply_look_depth);
        let layer = Handle::<Image>::default();
        let material = app.world_mut().resource_mut::<Assets<RoomSkyMaterial>>().add(RoomSkyMaterial {
            room: Vec4::ZERO,
            front: Vec4::ZERO,
            depth: RoomLookDepth::default().uniform(),
            sky: layer.clone(),
            far: layer.clone(),
            near: layer.clone(),
            atmosphere: layer,
        });
        app.world_mut().spawn(MeshMaterial2d(material.clone()));
        let depth_of = |app: &App| app.world().resource::<Assets<RoomSkyMaterial>>().get(&material).unwrap().depth;
        assert_ne!(depth_of(&app), Vec4::new(0.4, 0.1, 0.0, 0.0), "premise: a new sky has the default numbers");
        app.update();
        assert_eq!(depth_of(&app), Vec4::new(0.4, 0.1, 0.0, 0.0));

        // Out of range: the shader gets a fog it can use.
        *app.world_mut().resource_mut::<RoomLookDepth>() = RoomLookDepth { fog: 7.0, fog_patches: -1.0 };
        app.update();
        assert_eq!(depth_of(&app), Vec4::new(1.0, 0.0, 0.0, 0.0));
    }
}
