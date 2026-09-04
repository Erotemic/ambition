//! ONE SPRITE, ONE SIMPLE VISUAL MANIPULATION.
//!
//! The engine had four unrelated ways to alter a sprite's pixels and no name
//! for the idea: `Sprite.color` (the built-in multiply), the projectile
//! catalog's `EnergyTinted` art source, the hit-flash silhouette overlay, and
//! the portal-clip material's `tint` uniform. Each one is correct in its place;
//! none of them is reusable, and a fifth caller wanting "this sprite, but a
//! different colour" had to pick one and copy it. [`SpriteEffect`] is that
//! missing concept.
//!
//! ⭐ **THE OPERATION MATTERS MORE THAN THE COLOUR, and it is the reason this
//! is not just a component wrapping `Sprite.color`.** A multiply cannot
//! recolour art that already has a colour: an orange sprite multiplied by green
//! is dark mud, not a green sprite. Only [`SpriteEffect::HueShift`] turns a blue
//! gun into a red one while keeping its shading, highlights and antialiased
//! edges. Callers reach for "tint" and mean one of four different things:
//!
//! | effect | what it can do | cost |
//! |---|---|---|
//! | [`Tint`](SpriteEffect::Tint) | darken / warm / flash; recolour WHITE art | free — writes `Sprite.color`, batches with every other sprite |
//! | [`HueShift`](SpriteEffect::HueShift) | recolour COLOURED art, keeping shading | one material |
//! | [`Saturate`](SpriteEffect::Saturate) | greyscale ... punchier | one material |
//! | [`Silhouette`](SpriteEffect::Silhouette) | shape only, flat colour | one material |
//!
//! ⛔ **`Tint` deliberately does NOT go through the material.** It is the one
//! operation the built-in sprite pipeline already performs per instance, and
//! routing it through a mesh would cost a pipeline switch to compute something
//! the hardware was already doing for free. An effect that CAN be free is free;
//! [`SpriteEffect::needs_material`] is where that split is decided, once.

use bevy::asset::embedded_asset;
use bevy::platform::collections::HashMap;
use bevy::image::TextureAtlasLayout;
use bevy::prelude::*;
use bevy::reflect::TypePath;
use bevy::render::render_resource::AsBindGroup;
use bevy::shader::ShaderRef;
use bevy::sprite_render::{AlphaMode2d, Material2d, Material2dPlugin};

/// One simple visual manipulation of a sprite's own pixels.
///
/// Attach beside a `Sprite`. [`apply_free_sprite_effects`] applies the ones the
/// built-in pipeline can do; the rest are drawn through [`SpriteFxMaterial`] by
/// whoever spawns the quad (see [`SpriteFxMaterial::for_effect`]).
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub enum SpriteEffect {
    /// Multiply the sprite by a colour — exactly `Sprite.color`, and free.
    ///
    /// ⚠ Multiply can only ever DARKEN toward the given colour. It recolours
    /// white or greyscale art perfectly and coloured art not at all; reach for
    /// [`HueShift`](Self::HueShift) when the art already has a hue.
    Tint(Color),
    /// Rotate the hue by `degrees`, keeping luminance and saturation.
    ///
    /// Grey pixels lie on the rotation axis and are unchanged, so white
    /// highlights stay white and black outlines stay black — the sprite reads
    /// as the same drawing in a different colour rather than as a colour wash.
    HueShift { degrees: f32 },
    /// Scale saturation about luminance: `0.0` greyscale, `1.0` unchanged,
    /// `> 1.0` more vivid.
    Saturate { factor: f32 },
    /// A flat colour masked by the sprite's alpha: shape, no interior detail.
    Silhouette(Color),
}

impl SpriteEffect {
    /// Does drawing this effect require [`SpriteFxMaterial`], or can the
    /// built-in sprite pipeline express it?
    ///
    /// The one place the free/shader split is decided, so a new effect cannot
    /// disagree with itself between the spawn path and the apply system.
    pub fn needs_material(self) -> bool {
        !matches!(self, SpriteEffect::Tint(_))
    }

    /// `(operation, scalar argument, colour argument)` as the shader reads them.
    /// Kept next to the variants so the WGSL opcodes have exactly one author.
    fn shader_args(self) -> (f32, f32, Color) {
        match self {
            SpriteEffect::Tint(color) => (0.0, 0.0, color),
            SpriteEffect::HueShift { degrees } => (1.0, degrees, Color::WHITE),
            SpriteEffect::Saturate { factor } => (2.0, factor, Color::WHITE),
            SpriteEffect::Silhouette(color) => (3.0, 0.0, color),
        }
    }
}

/// Apply the effects the built-in sprite pipeline can express, in place.
///
/// Only [`SpriteEffect::Tint`] qualifies; the others need a material and are
/// left to the caller that draws the quad, so this system never silently
/// half-applies a hue shift by writing its (white) colour argument.
pub fn apply_free_sprite_effects(mut sprites: Query<(&SpriteEffect, &mut Sprite)>) {
    for (effect, mut sprite) in &mut sprites {
        if let SpriteEffect::Tint(color) = *effect {
            if sprite.color != color {
                sprite.color = color;
            }
        }
    }
}

/// Identity of one distinct material draw, so identical ones share a handle.
///
/// ⚠ Floats are keyed by their BIT PATTERN, not their value. That is exact and
/// deliberate: two hue angles that differ in the last bit are two different
/// draws and must not collide, and `f32` has no `Eq` for the good reason that
/// `NaN != NaN`. A `NaN` angle keys consistently here and the shader renders
/// whatever it renders — a wrong colour, never a wrong entity's material.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct MaterialKey {
    control: [u32; 4],
    colour: [u32; 4],
    uv_rect: [u32; 4],
    image: AssetId<Image>,
}

impl MaterialKey {
    fn new(effect: SpriteEffect, basis: &SpriteFrameBasis, image: &Handle<Image>, flip_x: bool) -> Self {
        let (op, scalar, colour) = effect.shader_args();
        let c = colour.to_linear();
        let bits = |v: Vec4| [v.x.to_bits(), v.y.to_bits(), v.z.to_bits(), v.w.to_bits()];
        Self {
            control: bits(Vec4::new(op, if flip_x { 1.0 } else { 0.0 }, scalar, 0.0)),
            colour: bits(Vec4::new(c.red, c.green, c.blue, c.alpha)),
            uv_rect: bits(basis.uv_rect),
            image: image.id(),
        }
    }
}

/// Marks a sprite this crate has taken over and is drawing as a mesh, holding
/// the `Sprite` it replaced so the entity can be handed back unchanged.
///
/// ⭐ The original is STORED rather than rebuilt. A sprite carries flip, anchor,
/// custom size, atlas index and image; reconstructing one from the mesh draw
/// would quietly drop whichever of those the reconstruction forgot, and the
/// entity would come back subtly different from the one that went in.
#[derive(Component, Clone, Debug)]
pub struct SpriteFxDrawn {
    /// The sprite as it was before this crate replaced its draw.
    pub original: Sprite,
    /// The effect the current mesh draw was built for, so a CHANGED effect
    /// rebuilds and an unchanged one costs nothing.
    pub drawn_for: SpriteEffect,
}

/// Draw sprites carrying a shader [`SpriteEffect`] as a textured quad.
///
/// ⭐ **THIS IS WHAT MAKES THE EFFECT A COMPONENT RATHER THAN A DRAW PROTOCOL.**
/// A caller adds [`SpriteEffect`] beside its `Sprite` and is done; it does not
/// need to know that three of the four effects cannot be expressed by the
/// sprite pipeline, build a mesh, resolve an atlas frame, or own a material
/// handle. Without this system every caller wanting a hue shift would reimplement
/// the portal-clip crate's mesh path, which is the duplication this crate exists
/// to end.
///
/// ⚠ **An unloaded texture leaves the sprite alone rather than blanking it.**
/// `sprite_frame_basis` answers `None` until the image or atlas layout arrives,
/// and a quad sampling a texture that is not there draws nothing at all — so the
/// untouched sprite is the correct output for those frames, and the effect
/// applies on a later one.
pub fn draw_sprite_effects(
    mut commands: Commands,
    mut materials: ResMut<Assets<SpriteFxMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    layouts: Res<Assets<TextureAtlasLayout>>,
    images: Res<Assets<Image>>,
    mut quad: Local<Option<Handle<Mesh>>>,
    mut cache: Local<HashMap<MaterialKey, Handle<SpriteFxMaterial>>>,
    fresh: Query<(Entity, &SpriteEffect, &Sprite, &Transform), Without<SpriteFxDrawn>>,
    mut drawn: Query<(Entity, &SpriteEffect, &mut SpriteFxDrawn)>,
) {
    for (entity, effect, sprite, transform) in &fresh {
        if !effect.needs_material() {
            continue;
        }
        let Some(basis) = sprite_frame_basis(sprite, &layouts, &images) else {
            continue;
        };
        let mesh = quad
            .get_or_insert_with(|| meshes.add(Rectangle::default()))
            .clone();
        // ⛔ CACHED, because the callers that most want an effect are the ones
        // that respawn their sprite every frame. The portal gun's visual is
        // despawned and rebuilt each tick; minting a material per entity per
        // tick would push a new asset through `Assets` sixty times a second for
        // a gun whose colour never changes. Identical draws share one handle,
        // so a hue-shifted gun costs ONE material for the life of the process.
        let key = MaterialKey::new(*effect, &basis, &sprite.image, sprite.flip_x);
        let material = cache
            .entry(key)
            .or_insert_with(|| {
                materials.add(SpriteFxMaterial::for_effect(
                    *effect,
                    basis,
                    sprite.image.clone(),
                    sprite.flip_x,
                ))
            })
            .clone();
        let mut entity_commands = commands.entity(entity);
        entity_commands.insert((
            SpriteFxDrawn {
                original: sprite.clone(),
                drawn_for: *effect,
            },
            Mesh2d(mesh),
            MeshMaterial2d(material),
            Transform {
                scale: (basis.size * transform.scale.truncate()).extend(1.0),
                ..*transform
            },
        ));
        entity_commands.remove::<Sprite>();
    }

    // An effect CHANGED on an entity already drawn as a mesh: put the sprite
    // back and let the pass above rebuild it. Restoring first is what keeps the
    // stored original authoritative rather than accumulating edits.
    for (entity, effect, mut state) in &mut drawn {
        if state.drawn_for == *effect {
            continue;
        }
        state.drawn_for = *effect;
        let mut entity_commands = commands.entity(entity);
        entity_commands.insert(state.original.clone());
        entity_commands
            .remove::<(Mesh2d, MeshMaterial2d<SpriteFxMaterial>, SpriteFxDrawn)>();
    }
}

/// Hand back a sprite whose [`SpriteEffect`] was removed.
///
/// Without this, removing the effect would leave the entity drawn as a mesh
/// forever — the effect would be un-cancellable, which is a worse bug than it
/// never having applied.
pub fn restore_sprites_without_effects(
    mut commands: Commands,
    orphaned: Query<(Entity, &SpriteFxDrawn), Without<SpriteEffect>>,
) {
    for (entity, state) in &orphaned {
        let mut entity_commands = commands.entity(entity);
        entity_commands.insert(state.original.clone());
        entity_commands
            .remove::<(Mesh2d, MeshMaterial2d<SpriteFxMaterial>, SpriteFxDrawn)>();
    }
}

/// `Material2d` for one sprite drawn through a [`SpriteEffect`].
///
/// Bindings follow the WebGL2-friendly convention the other overlays in this
/// workspace use: plain `vec4` uniforms, no struct UBOs, no arrays.
#[derive(Asset, AsBindGroup, TypePath, Debug, Clone)]
pub struct SpriteFxMaterial {
    /// Current atlas frame as a normalized UV rect `(min.x, min.y, max.x, max.y)`.
    #[uniform(0)]
    pub uv_rect: Vec4,
    /// `(operation, flip_x, scalar, _)`.
    #[uniform(1)]
    pub control: Vec4,
    /// Colour argument (linear RGBA) for the operations that take one.
    #[uniform(2)]
    pub colour: Vec4,
    #[texture(3)]
    #[sampler(4)]
    pub color_texture: Handle<Image>,
}

impl SpriteFxMaterial {
    /// Build the material that draws `sprite`'s current frame under `effect`.
    ///
    /// `basis` comes from [`sprite_frame_basis`]; `None` there means the
    /// texture has not loaded and the caller should draw the plain sprite this
    /// frame rather than a quad sampling nothing.
    pub fn for_effect(
        effect: SpriteEffect,
        basis: SpriteFrameBasis,
        image: Handle<Image>,
        flip_x: bool,
    ) -> Self {
        let (op, scalar, colour) = effect.shader_args();
        let c = colour.to_linear();
        Self {
            uv_rect: basis.uv_rect,
            control: Vec4::new(op, if flip_x { 1.0 } else { 0.0 }, scalar, 0.0),
            colour: Vec4::new(c.red, c.green, c.blue, c.alpha),
            color_texture: image,
        }
    }
}

impl Material2d for SpriteFxMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://ambition_sprite_fx/shaders/sprite_fx.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }
}

/// The sprite-effect concept: the free-path system plus, where the host has a
/// real asset/render stack, the material pipeline the shader effects draw with.
///
/// A headless host (tests, servers) still gets [`SpriteEffect`] and the free
/// tint path; only the material registration is skipped, exactly as the
/// portal-clip and hit-flash materials do.
pub struct SpriteFxPlugin;

impl Plugin for SpriteFxPlugin {
    fn build(&self, app: &mut App) {
        // The free path always runs; the mesh path needs the render stack and
        // is added below with the material it draws through.
        app.add_systems(PostUpdate, apply_free_sprite_effects);
        // `embedded_asset!` needs the AssetPlugin's registry.
        if app
            .world()
            .get_resource::<bevy::asset::io::embedded::EmbeddedAssetRegistry>()
            .is_none()
        {
            return;
        }
        embedded_asset!(app, "shaders/sprite_fx.wgsl");
        app.add_plugins(Material2dPlugin::<SpriteFxMaterial>::default());
        app.add_systems(
            PostUpdate,
            (restore_sprites_without_effects, draw_sprite_effects)
                .chain()
                .after(apply_free_sprite_effects),
        );
    }
}

/// The render basis of a sprite's CURRENT frame: the normalized UV rect of the
/// frame on its texture, and the world-space quad size the sprite draws at.
/// `None` while the texture / atlas layout hasn't loaded.
///
/// ⭐ Lives here, at the render floor, because it is not specific to any one
/// effect: it answers "which pixels would the sprite renderer draw right now",
/// which every mesh-drawn sprite manipulation needs. It was written for the
/// portal-clip material and the hit-flash overlay independently before that.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpriteFrameBasis {
    /// `(min.x, min.y, max.x, max.y)` normalized on the sprite's texture.
    pub uv_rect: Vec4,
    /// Drawn quad size: `custom_size` when set (trimmed sheets update it per
    /// frame), else the frame's native pixel size.
    pub size: Vec2,
}

/// Resolve the [`SpriteFrameBasis`] for `sprite` (atlas frame or whole image),
/// sampling exactly the pixels the sprite renderer would.
pub fn sprite_frame_basis(
    sprite: &Sprite,
    layouts: &Assets<TextureAtlasLayout>,
    images: &Assets<Image>,
) -> Option<SpriteFrameBasis> {
    // Atlased sprites derive dimensions from the atlas layout so they do not
    // require main-world image access. Whole-image sprites still need the image.
    let (uv_rect, frame_px) = if let Some(atlas) = sprite.texture_atlas.as_ref() {
        let layout = layouts.get(&atlas.layout)?;
        let rect = layout.textures.get(atlas.index)?;
        let tex = Vec2::new(layout.size.x.max(1) as f32, layout.size.y.max(1) as f32);
        let min = Vec2::new(rect.min.x as f32, rect.min.y as f32);
        let max = Vec2::new(rect.max.x as f32, rect.max.y as f32);
        (
            Vec4::new(min.x / tex.x, min.y / tex.y, max.x / tex.x, max.y / tex.y),
            max - min,
        )
    } else {
        let image = images.get(&sprite.image)?;
        let texture_size = image.texture_descriptor.size;
        let tex = Vec2::new(
            texture_size.width.max(1) as f32,
            texture_size.height.max(1) as f32,
        );
        (Vec4::new(0.0, 0.0, 1.0, 1.0), tex)
    };
    Some(SpriteFrameBasis {
        uv_rect,
        size: sprite.custom_size.unwrap_or(frame_px),
    })
}

#[cfg(test)]
mod tests;
