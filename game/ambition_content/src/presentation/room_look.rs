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
//! The architecture of the two-state look is published art: two terrain
//! skins, `hub_clean` and `hub_corrupt` (`terrain/two_state.py` in the sprite
//! renderer), which the room's terrain lays on the same blocks
//! (`ambition_render::rendering::terrain_skin`). [`RoomSkinMaterial`] shows
//! each skin on its side of the front; with no budget for the look, each skin
//! is a plain textured mesh and shows where the room's state is its own
//! ([`show_two_state_skins`]). So the room keeps its look with each shader
//! off, and comes with it on its first frame. [`RoomStateMaterial`] draws
//! what moves or is not a thing of the room: a blink wall's veil and the
//! overlay. The sky is authored parallax layers ([`RoomSkyMaterial`]).
//!
//! The front of the two-state look moves. The state of the room is a fact in
//! the save (`crate::room_look_state`: pure, balanced or corrupt), and
//! [`spread_room_states`] moves the front toward it, so a change of state
//! spreads across the room.
//!
//! A look also chooses the door of the room ([`RoomLook::door_art`]), with the
//! renderer's `EntityArt` seam.
//!
//! A device with no budget for a shader that fills the screen
//! ([`looks_are_in_budget`]) draws no look quad: its room is the skins.

use bevy::{
    prelude::*,
    reflect::TypePath,
    render::render_resource::AsBindGroup,
    shader::ShaderRef,
    sprite_render::{AlphaMode2d, ColorMaterial, Material2d, Material2dPlugin, MeshMaterial2d},
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
use ambition_render::rendering::terrain_skin::TerrainSkinMesh;
use ambition_render::rendering::{
    BlockVisual, EntityArt, LoadingZoneVisual, RoomVisual, DOOR_SPRITE_ASPECT,
};
use ambition_sprite_sheet::game_assets::ParallaxTheme;
use ambition_sprite_sheet::game_assets::EntitySprite;

use crate::room_look_state::RoomLookState;

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

/// What the looks' shaders share (`ambition_content::room_look`).
const ROOM_LOOK_LIBRARY: &str = "embedded://ambition_content/presentation/shaders/room_look_common.wgsl";

/// One named look: a material whose shader draws every role.
///
/// All positions are in engine world coordinates (y down), which is where the
/// shaders do their work.
pub trait RoomLook: Material2d {
    /// The `palette` level-field value that asks for this look.
    const PALETTE: &'static str;

    /// Whether the terrain of this look (its solid and one-way blocks, and
    /// their door frames) is the room's terrain skins and not drawn by this
    /// material.
    const TERRAIN_IS_SKINS: bool = false;

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

/// One terrain skin of a two-state room, shown on its side of the front.
///
/// The skins are published art: `ParallaxTheme::HubClean` and
/// `ParallaxTheme::HubCorrupt` (`terrain/two_state.py` in the sprite
/// renderer), laid on the same blocks by the room's terrain. This material is
/// one mesh of a skin: it shows the skin where the room is in the skin's
/// state, and adds what the front does to the stone near it. Its shader
/// (`room_skin.wgsl`) draws no art.
#[derive(Asset, AsBindGroup, TypePath, Debug, Clone)]
pub struct RoomSkinMaterial {
    /// `x, y` = the room size; `z` = 1 for the corrupted skin, 0 for the
    /// clean one.
    #[uniform(0)]
    pub room: Vec4,
    /// A point on the room's front (`x, y`) and its normal (`z, w`), which
    /// points into the corrupted side.
    #[uniform(1)]
    pub front: Vec4,
    #[texture(2)]
    #[sampler(3)]
    pub skin: Handle<Image>,
}

impl Material2d for RoomSkinMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://ambition_content/presentation/shaders/room_skin.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }
}

/// A mesh of a skin of a two-state room: the room it is of, and the plain
/// material its terrain made for it, which it shows with no budget for the
/// look.
#[derive(Component, Clone, Debug)]
struct TwoStateSkin {
    room: ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance,
    plain: Handle<ColorMaterial>,
}

/// Show each skin of each two-state room where the room is in its state.
///
/// In budget, a mesh of a skin draws with [`RoomSkinMaterial`], which follows
/// the moving front. Out of budget it is the plain textured mesh its terrain
/// made, and a square of the room ([`TerrainSkinMesh::centre`]) shows the skin
/// of the side of the front it is on: a room that is all one state shows that
/// skin, with no shader.
fn show_two_state_skins(
    mut commands: Commands,
    quality: Option<Res<ResolvedVisualQuality>>,
    save: Option<Res<AmbitionGameSave>>,
    rooms: LiveRoomSpecs,
    spreads: Query<(&InRoomInstance, &RoomLookSpread)>,
    terrains: Query<&InRoomInstance>,
    mut meshes: Query<(
        Entity,
        &TerrainSkinMesh,
        &ChildOf,
        &mut Visibility,
        Option<&MeshMaterial2d<ColorMaterial>>,
        Option<&MeshMaterial2d<RoomSkinMaterial>>,
        Option<&TwoStateSkin>,
    )>,
    plain: Res<Assets<ColorMaterial>>,
    mut skins: ResMut<Assets<RoomSkinMaterial>>,
) {
    let in_budget = draws_looks(quality.as_deref());
    for (entity, mesh, parent, mut visibility, colour, shaded, two) in &mut meshes {
        let corrupt = match mesh.theme {
            ParallaxTheme::HubClean => false,
            ParallaxTheme::HubCorrupt => true,
            _ => continue,
        };
        let Ok(stamp) = terrains.get(parent.parent()) else {
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
        let advance = advance_of(&spreads, stamp, spec, save.as_deref());
        let shown = if in_budget {
            match (colour, shaded) {
                (Some(colour), _) => {
                    let Some(texture) = plain.get(&colour.0).and_then(|material| material.texture.clone()) else {
                        continue;
                    };
                    let material = skins.add(RoomSkinMaterial {
                        room: Vec4::new(world.size.x, world.size.y, if corrupt { 1.0 } else { 0.0 }, 0.0),
                        front: room_front(world, advance),
                        skin: texture,
                    });
                    commands
                        .entity(entity)
                        .try_remove::<MeshMaterial2d<ColorMaterial>>()
                        .try_insert((MeshMaterial2d(material), TwoStateSkin { room: stamp.0, plain: colour.0.clone() }));
                }
                (None, Some(shaded)) => {
                    // A picture of the skin that is another handle now (a
                    // reload) is the one the skin shows.
                    let wanted = two.and_then(|two| plain.get(&two.plain)).and_then(|material| material.texture.clone());
                    if let Some(wanted) = wanted {
                        if skins.get(&shaded.0).is_some_and(|material| material.skin != wanted) {
                            if let Some(mut material) = skins.get_mut(&shaded.0) {
                                material.skin = wanted;
                            }
                        }
                    }
                }
                (None, None) => {}
            }
            true
        } else {
            if let (Some(_), Some(two)) = (shaded, two) {
                commands
                    .entity(entity)
                    .try_remove::<(MeshMaterial2d<RoomSkinMaterial>, TwoStateSkin)>()
                    .try_insert(MeshMaterial2d(two.plain.clone()));
            }
            (behind_front(world, ae::Vec2::new(mesh.centre.x, mesh.centre.y), advance) > 0.0) == corrupt
        };
        let wanted = if shown { Visibility::Inherited } else { Visibility::Hidden };
        if *visibility != wanted {
            *visibility = wanted;
        }
    }
}

/// Out of budget the sky is the flat backdrop of the room's theme
/// (`ParallaxLayerAsset::Flat`), one picture fixed to the view. A two-state
/// room shows the flat backdrop of the state the middle of the view is in, so
/// the sky changes with the room on a tier with no shader too.
fn show_two_state_backdrop(
    quality: Option<Res<ResolvedVisualQuality>>,
    assets: Option<Res<ambition_sprite_sheet::game_assets::GameAssets>>,
    save: Option<Res<AmbitionGameSave>>,
    rooms: LiveRoomSpecs,
    spreads: Query<(&InRoomInstance, &RoomLookSpread)>,
    mut panels: Query<(&InRoomInstance, &Transform, &mut Sprite), With<ambition_render::rendering::ParallaxLayerVisual>>,
) {
    use ambition_sprite_sheet::game_assets::ParallaxLayerAsset;
    if draws_looks(quality.as_deref()) {
        return;
    }
    let Some(assets) = assets else {
        return;
    };
    let flat = |theme| assets.parallax_layers.get(theme, ParallaxLayerAsset::Flat);
    let (Some(clean), Some(corrupted)) = (flat(ParallaxTheme::HubClean), flat(ParallaxTheme::HubCorrupt)) else {
        return;
    };
    for (stamp, transform, mut sprite) in &mut panels {
        if sprite.image != *clean && sprite.image != *corrupted {
            continue;
        }
        let Some((_, definition)) = rooms.live_rooms().find(|(room, _)| *room == stamp.0) else {
            continue;
        };
        let spec = rooms.rooms().spec(definition);
        let world = &spec.world;
        let advance = advance_of(&spreads, stamp, spec, save.as_deref());
        // The panel is over the camera; the room is drawn centred on the origin, y up.
        let at = ae::Vec2::new(transform.translation.x + world.size.x * 0.5, world.size.y * 0.5 - transform.translation.y);
        let wanted = if behind_front(world, at, advance) > 0.0 { corrupted } else { clean };
        if sprite.image != *wanted {
            sprite.image = wanted.clone();
        }
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
    const TERRAIN_IS_SKINS: bool = true;
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
    quality.is_none_or(|quality| quality.budget.shaders.draws_screen_shaders())
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
    // `content_asset!` needs the asset registries.
    if app
        .world()
        .get_resource::<bevy::asset::io::embedded::EmbeddedAssetRegistry>()
        .is_none()
    {
        return;
    }
    app.insert_resource(RoomLookInstalled);
    ambition_platformer2d::content_asset!(app, "src/presentation/shaders/room_state.wgsl");
    ambition_platformer2d::content_asset!(app, "src/presentation/shaders/room_skin.wgsl");
    ambition_platformer2d::content_asset!(app, "src/presentation/shaders/room_sky.wgsl");
    ambition_platformer2d::content_asset!(app, "src/presentation/shaders/room_blueprint.wgsl");
    // `ambition_content::room_look`: what the looks share. A composition with
    // no shader assets draws no look to import it.
    if app.world().contains_resource::<Assets<bevy::shader::Shader>>() {
        ambition_platformer2d::content_asset!(app, "src/presentation/shaders/room_look_common.wgsl");
        // A library is imported, never named by a material, so a held handle
        // keeps it loaded (as `load_shader_library!` does).
        let library: Handle<bevy::shader::Shader> =
            app.world().resource::<AssetServer>().load(ROOM_LOOK_LIBRARY);
        core::mem::forget(library);
    }
    if app.get_sub_app(bevy::render::RenderApp).is_some() {
        app.add_plugins(Material2dPlugin::<RoomSkinMaterial>::default());
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
    // In each budget: out of budget it gives each skin its plain mesh back.
    app.add_systems(
        Update,
        show_two_state_skins
            .run_if(resource_exists::<Assets<ColorMaterial>>)
            .run_if(resource_exists::<Assets<RoomSkinMaterial>>),
    );
    app.add_systems(Update, show_two_state_backdrop);
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
    // In each budget: the door is the look's with each shader off.
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
/// The door is authored art, so it is the look's in each budget: a room keeps
/// its look with each shader off.
fn dress_doors<M: RoomLook>(
    mut commands: Commands,
    save: Option<Res<AmbitionGameSave>>,
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
        let advance = advance_of(&spreads, stamp, spec, save.as_deref());
        let art = EntityArt(M::door_art(&spec.world, zone.aabb.center(), advance));
        if dressed != Some(&art) {
            // `try_insert`: a room visual can leave before the command flush.
            commands.entity(entity).try_insert(art);
        }
    }
}

/// Where the front of the live room `stamp` (of `spec`) is now. A room with no
/// spread record (its look is not spawned: out of budget, or not yet) has its
/// front where the state the save records puts it.
fn advance_of(
    spreads: &Query<(&InRoomInstance, &RoomLookSpread)>,
    stamp: &InRoomInstance,
    spec: &RoomSpec,
    save: Option<&AmbitionGameSave>,
) -> f32 {
    spreads
        .iter()
        .find(|(room, _)| room.0 == stamp.0)
        .map_or_else(|| wanted_advance(spec, save), |(_, spread)| spread.advance)
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
    skins: Query<(&TwoStateSkin, &MeshMaterial2d<RoomSkinMaterial>)>,
    mut skin_materials: Option<ResMut<Assets<RoomSkinMaterial>>>,
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
        let Some(skin_materials) = skin_materials.as_deref_mut() else {
            continue;
        };
        for (skin, material) in &skins {
            if skin.room != stamp.0 {
                continue;
            }
            if let Some(mut material) = skin_materials.get_mut(&material.0) {
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
            if M::TERRAIN_IS_SKINS && is_terrain {
                // The room's terrain skins draw it ([`show_two_state_skins`]).
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
            if M::TERRAIN_IS_SKINS {
                // The door's own art is the look's ([`dress_doors`]).
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
    save: Option<Res<AmbitionGameSave>>,
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
        let pale = in_budget && behind_front(world, at, advance_of(&spreads, stamp, spec, save.as_deref())) <= -60.0;
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
