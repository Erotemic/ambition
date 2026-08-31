//! Mary-O's invincible rainbow-quasar presentation pass.
//!
//! The effect is presentation-only and content-owned. It activates for a
//! Mary-O player body while the body cannot be hurt, so the Cosmic Quasar pickup
//! does not need to know anything about Bevy materials. The source sprite
//! remains visible underneath a synced sibling `Material2d` mesh; a shader
//! failure therefore degrades to ordinary Mary-O rather than making the player
//! disappear.
//!
//! ## It reads the AUTHORITY, not a mirror of it
//!
//! "Cannot be hurt" has exactly one authoritative answer: the `Invulnerability` reason set on
//! `Health`, which is what `Health::damage` itself consults. The effect read the loser of that race
//! and stayed dark for the whole ten seconds.
//!
//! Reading the fact that DECIDES whether hits land means there is no race to
//! win, because there is no second copy.
//!
//! It reads ONE REASON out of that set, not `any()`: `EMPOWERED`.

use std::collections::HashMap;

use bevy::{
    asset::embedded_asset,
    image::TextureAtlasLayout,
    prelude::*,
    reflect::TypePath,
    render::render_resource::AsBindGroup,
    shader::ShaderRef,
    sprite::Anchor,
    sprite_render::{AlphaMode2d, Material2d, Material2dPlugin, MeshMaterial2d},
};

use ambition_platformer2d::characters::actor::BodyHealth;
use ambition_platformer2d::characters::actor::WornCharacter;
use ambition_platformer2d::platformer::lifecycle::{
    SessionScopedEntity, SessionSpawnScope, SpawnSessionScopedExt,
};
use ambition_platformer2d::render::rendering::{ActorOverlaySet, PlayerVisual, RoomVisual};

const EFFECT_STRENGTH: f32 = 1.0;
const OVERLAY_ALPHA: f32 = 0.96;
const OVERLAY_Z_BIAS: f32 = 1.0;

#[derive(Resource, Default)]
struct QuasarShaderInstalled;

/// Runtime tuning for development and capture tooling.
#[derive(Resource, Reflect, Debug, Clone, Copy)]
#[reflect(Resource)]
pub struct MaryOQuasarShaderSettings {
    /// Disable the overlay without changing the authoritative invincible fact.
    pub disabled: bool,
    /// Multiplier applied to the shader's authored effect strength.
    pub strength: f32,
}

impl Default for MaryOQuasarShaderSettings {
    fn default() -> Self {
        Self {
            disabled: false,
            strength: 1.0,
        }
    }
}

/// Install the embedded WGSL material and its player-overlay lifecycle.
///
/// Idempotent, so both Mary-O composition paths may call it safely.
///
/// So a headless Mary-O installed the pass and then panicked on the first frame ("Resource does not
/// exist"), taking 18 of the demo app's 20 integration tests with it. A proxy answers a question
/// next to the one being asked; the run condition asks for exactly the collections the systems
/// take, and cannot drift from them as long as it names the same types.
pub fn install(app: &mut App) {
    if app.world().contains_resource::<QuasarShaderInstalled>() {
        return;
    }
    // The asset half still needs its own install-time guard, for its own reason:
    // `embedded_asset!` and `init_asset` both reach into `AssetPlugin`'s
    // registries and panic without them. That is a real precondition of these
    // two lines — not a stand-in for a question about rendering.
    if app
        .world()
        .get_resource::<bevy::asset::io::embedded::EmbeddedAssetRegistry>()
        .is_none()
    {
        return;
    }

    app.insert_resource(QuasarShaderInstalled);
    embedded_asset!(app, "shaders/invincible_rainbow_quasar.wgsl");
    // the MATERIAL plugin asks whether this app RENDERS, which the asset
    // guard above does not. `Material2dPlugin` installs render-world state —
    // `PreparedMaterial2d`, `EntitiesNeedingSpecialization` — into any app that
    // merely has an `AssetPlugin`, which every headless composition has. That is
    // the same "a proxy answers the question next door" mistake this module's
    // own doc records, one line further down: the asset registry is a real
    // precondition of `embedded_asset!`, and it is not evidence of a renderer.
    if app.get_sub_app(bevy::render::RenderApp).is_some() {
        app.add_plugins(Material2dPlugin::<MaryOQuasarMaterial>::default());
    }
    app.init_resource::<MaryOQuasarShaderSettings>();
    app.register_type::<MaryOQuasarShaderSettings>();
    app.add_systems(
        Update,
        (
            attach_quasar_overlays,
            sync_quasar_overlays,
            cleanup_quasar_overlays,
        )
            .chain()
            .in_set(ActorOverlaySet)
            // The render-world asset collections `attach_quasar_overlays` writes
            // into. Absent = this app does not draw, so there is nothing for an
            // overlay to be drawn on.
            .run_if(resource_exists::<Assets<Mesh>>)
            .run_if(resource_exists::<Assets<Image>>)
            .run_if(resource_exists::<Assets<TextureAtlasLayout>>)
            // the MATERIAL collection too, and leaving it out was a real
            // panic. `Material2dPlugin` is what creates
            // `Assets<MaryOQuasarMaterial>`, and gating that plugin on a render
            // app made the collection genuinely absent in headless
            // compositions — where `attach_quasar_overlays` then failed
            // parameter validation in the shipped-composition resource sweep.
            // The doc above says this condition names *"exactly the collections
            // the systems take"*; it named three of the four.
            .run_if(resource_exists::<Assets<MaryOQuasarMaterial>>),
    );
}

/// Atlas-aware material for Mary-O's invincible spectral overlay.
///
/// Bindings:
/// - `uv_rect`: normalized atlas-frame bounds.
/// - `control`: elapsed seconds, x-flip, strength, deterministic seed.
/// - `detail`: local-frame texel size, reserved pulse channel, overlay alpha.
#[derive(Asset, AsBindGroup, TypePath, Debug, Clone)]
pub struct MaryOQuasarMaterial {
    #[uniform(0)]
    pub uv_rect: Vec4,
    #[uniform(1)]
    pub control: Vec4,
    #[uniform(2)]
    pub detail: Vec4,
    #[texture(3)]
    #[sampler(4)]
    pub color_texture: Handle<Image>,
}

impl Material2d for MaryOQuasarMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://ambition_demo_mary_o/shaders/invincible_rainbow_quasar.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }
}

/// How long a Mary-O overlay may be "not yet" before it is a report.
///
/// The two blocking conditions — no `custom_size`, no resolvable sprite frame —
/// are both ordinary while the texture decodes, and a boot prints them for a
/// frame or two. One second at 60fps is far past any decode, so a candidate
/// still waiting here is one whose overlay is never going to appear.
///
/// counted per candidate and reported ONCE, not re-warned every frame
/// after the threshold: a diagnostic that fires sixty times a second for a
/// standing condition is the same noise problem one frame earlier.
const QUASAR_ATTACH_GRACE_FRAMES: u32 = 60;

#[derive(Component, Debug, Clone, Copy)]
struct MaryOQuasarSource {
    overlay: Entity,
    seed: f32,
}

#[derive(Component, Debug, Clone, Copy)]
struct MaryOQuasarOverlay {
    source: Entity,
}

fn attach_quasar_overlays(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<MaryOQuasarMaterial>>,
    texture_layouts: Res<Assets<TextureAtlasLayout>>,
    images: Res<Assets<Image>>,
    candidates: Query<
        (
            Entity,
            &WornCharacter,
            &Transform,
            &Sprite,
            Option<&Anchor>,
            Option<&SessionScopedEntity>,
        ),
        (With<PlayerVisual>, Without<MaryOQuasarSource>),
    >,
    // How long each candidate has been un-attachable, and whether it has already
    // been reported. See [`QUASAR_ATTACH_GRACE_FRAMES`].
    mut waiting: Local<HashMap<Entity, (u32, bool)>>,
) {
    waiting.retain(|entity, _| candidates.get(*entity).is_ok());
    for (source_entity, worn, transform, sprite, anchor, session_owner) in &candidates {
        if !is_mary_o_form(worn.id()) {
            continue;
        }
        // The two ways attaching can silently do nothing. Both are "not yet"
        // conditions that normally clear within a frame or two of the sprite
        // loading, so they are only worth a word if they PERSIST — which is
        // exactly the case where the overlay never appears and nothing says so.
        let mut report = |reason: std::fmt::Arguments| {
            let (frames, reported) = waiting.entry(source_entity).or_insert((0, false));
            *frames += 1;
            if *frames >= QUASAR_ATTACH_GRACE_FRAMES && !*reported {
                *reported = true;
                warn!(
                    target: "mary_o::quasar",
                    "overlay STILL not attached after {frames} frames: {reason}"
                );
            }
        };
        let Some(render_size) = sprite.custom_size else {
            report(format_args!("the Mary-O sprite has no custom_size"));
            continue;
        };
        let Some((uv_rect, frame_texel)) = current_sprite_frame(sprite, &texture_layouts, &images)
        else {
            report(format_args!(
                "no current sprite frame (atlas {:?}, image loaded = {})",
                sprite.texture_atlas.is_some(),
                images.get(&sprite.image).is_some(),
            ));
            continue;
        };
        waiting.remove(&source_entity);
        info!(
            target: "mary_o::quasar",
            "overlay attached to {source_entity} ({})",
            worn.id()
        );

        let seed = seed_from_id(worn.id());
        let material = materials.add(MaryOQuasarMaterial {
            uv_rect,
            control: Vec4::new(0.0, flip_flag(sprite), EFFECT_STRENGTH, seed),
            detail: Vec4::new(frame_texel.x, frame_texel.y, 0.0, OVERLAY_ALPHA),
            color_texture: sprite.image.clone(),
        });
        let mesh = meshes.add(Rectangle::default());
        let overlay_transform = overlay_transform_from_source(transform, anchor, render_size);
        let session_scope = SessionSpawnScope::new(session_owner.map(|owner| owner.0));
        let overlay = commands
            .spawn_session_scoped(
                session_scope,
                (
                    Mesh2d(mesh),
                    MeshMaterial2d(material),
                    overlay_transform,
                    Visibility::Hidden,
                    MaryOQuasarOverlay {
                        source: source_entity,
                    },
                    RoomVisual,
                    Name::new("Mary-O Invincible Rainbow Quasar Overlay"),
                ),
            )
            .id();
        commands
            .entity(source_entity)
            .insert(MaryOQuasarSource { overlay, seed });
    }
}

fn sync_quasar_overlays(
    presentation_time: ambition_platformer2d::time::PresentationTime,
    mut elapsed: Local<f32>,
    settings: Res<MaryOQuasarShaderSettings>,
    texture_layouts: Res<Assets<TextureAtlasLayout>>,
    images: Res<Assets<Image>>,
    sources: Query<
        (
            Entity,
            &WornCharacter,
            &BodyHealth,
            &Transform,
            &Sprite,
            Option<&Anchor>,
            &MaryOQuasarSource,
            Option<&Visibility>,
        ),
        Without<MaryOQuasarOverlay>,
    >,
    mut overlays: Query<(
        &mut Transform,
        &mut Visibility,
        &MeshMaterial2d<MaryOQuasarMaterial>,
        &MaryOQuasarOverlay,
    )>,
    mut materials: ResMut<Assets<MaryOQuasarMaterial>>,
    mut was_enabled: Local<Option<bool>>,
) {
    *elapsed += presentation_time.wall_dt();

    for (
        source_entity,
        worn,
        health,
        source_transform,
        source_sprite,
        anchor,
        source,
        source_visibility,
    ) in &sources
    {
        let Ok((mut overlay_transform, mut overlay_visibility, material_handle, overlay)) =
            overlays.get_mut(source.overlay)
        else {
            continue;
        };
        if overlay.source != source_entity {
            continue;
        }

        let source_visible = !matches!(source_visibility, Some(v) if *v == Visibility::Hidden);
        let enabled = is_mary_o_form(worn.id())
            && health
                .health
                .invulnerable
                .holds(ambition_platformer2d::characters::actor::Invulnerability::EMPOWERED)
            && source_visible
            && !settings.disabled
            && settings.strength > 0.0;
        // Say it ONCE per transition, naming every condition. "I got the quasar
        // and saw nothing" has five possible causes and no way to tell them
        // apart from the outside; this line distinguishes them.
        // Log the FIRST observation too, not only transitions. Logging only on
        // change meant the failing case — never enabled — printed nothing at
        // all, which is the one case the diagnostic existed for.
        if *was_enabled != Some(enabled) {
            *was_enabled = Some(enabled);
            info!(
                target: "mary_o::quasar",
                "overlay enabled = {enabled} (form '{}' ok = {}, invincible = {}, \
                 source_visible = {source_visible}, disabled = {}, strength = {})",
                worn.id(),
                is_mary_o_form(worn.id()),
                health.health.invulnerable.holds(
                ambition_platformer2d::characters::actor::Invulnerability::EMPOWERED,
            ),
                settings.disabled,
                settings.strength,
            );
        }
        *overlay_visibility = if enabled {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        if !enabled {
            continue;
        }

        let Some(render_size) = source_sprite.custom_size else {
            continue;
        };
        let Some((uv_rect, frame_texel)) =
            current_sprite_frame(source_sprite, &texture_layouts, &images)
        else {
            continue;
        };

        *overlay_transform = overlay_transform_from_source(source_transform, anchor, render_size);
        if let Some(mut material) = materials.get_mut(&material_handle.0) {
            material.uv_rect = uv_rect;
            material.control = Vec4::new(
                *elapsed,
                flip_flag(source_sprite),
                (EFFECT_STRENGTH * settings.strength).clamp(0.0, 2.0),
                source.seed,
            );
            material.detail = Vec4::new(frame_texel.x, frame_texel.y, 0.0, OVERLAY_ALPHA);
            material.color_texture = source_sprite.image.clone();
        }
    }
}

/// Keep the source/overlay pair honest in BOTH directions.
///
/// It only ever swept one way — despawn an overlay whose source is gone — and the other way is the
/// one that actually happened. The overlay carries [`RoomVisual`], so a room rebuild or a sprite
/// rebind ("assets changed") takes it, while the SOURCE survives holding a `MaryOQuasarSource` that
/// points at a dead entity. From that moment the effect is permanently dark and silent: `attach`
/// skips the source forever (it filters `Without<MaryOQuasarSource>`), `sync` fails its overlay
/// lookup and `continue`s before it can even log, and this system finds no overlay to complain
/// about.
///
/// A back-reference to an entity that can die independently is a CACHE, and a
/// cache whose target dies has to be invalidated. Dropping the component is the
/// invalidation: `attach` rebuilds the pair on the next frame.
fn cleanup_quasar_overlays(
    mut commands: Commands,
    sources: Query<(Entity, &MaryOQuasarSource)>,
    overlays: Query<(Entity, &MaryOQuasarOverlay)>,
) {
    for (overlay_entity, overlay) in &overlays {
        if sources.get(overlay.source).is_err() {
            commands.entity(overlay_entity).despawn();
        }
    }
    for (source_entity, source) in &sources {
        if overlays.get(source.overlay).is_err() {
            commands.entity(source_entity).remove::<MaryOQuasarSource>();
        }
    }
}

fn is_mary_o_form(id: &str) -> bool {
    matches!(id, "mary_o" | "mary_o_tall" | "mary_o_fire")
}

fn current_sprite_frame(
    sprite: &Sprite,
    texture_layouts: &Assets<TextureAtlasLayout>,
    images: &Assets<Image>,
) -> Option<(Vec4, Vec2)> {
    let atlas = sprite.texture_atlas.as_ref()?;
    let layout = texture_layouts.get(&atlas.layout)?;
    let rect = layout.textures.get(atlas.index)?;
    let image = images.get(&sprite.image)?;
    let texture_size = image.texture_descriptor.size;
    let texture_size = Vec2::new(
        texture_size.width.max(1) as f32,
        texture_size.height.max(1) as f32,
    );
    let frame_size = Vec2::new(
        (rect.max.x - rect.min.x).max(1) as f32,
        (rect.max.y - rect.min.y).max(1) as f32,
    );
    Some((
        Vec4::new(
            rect.min.x as f32 / texture_size.x,
            rect.min.y as f32 / texture_size.y,
            rect.max.x as f32 / texture_size.x,
            rect.max.y as f32 / texture_size.y,
        ),
        Vec2::ONE / frame_size,
    ))
}

fn overlay_transform_from_source(
    source: &Transform,
    anchor: Option<&Anchor>,
    render_size: Vec2,
) -> Transform {
    let anchor_offset = anchor_to_mesh_offset(anchor, render_size);
    let world_offset = source.rotation.mul_vec3(anchor_offset.extend(0.0));
    let mut transform = *source;
    transform.translation += world_offset;
    transform.translation.z += OVERLAY_Z_BIAS;
    transform.scale = render_size.extend(1.0);
    transform
}

fn anchor_to_mesh_offset(anchor: Option<&Anchor>, render_size: Vec2) -> Vec2 {
    let anchor = anchor.map(|a| a.0).unwrap_or(Vec2::ZERO);
    -anchor * render_size
}

fn flip_flag(sprite: &Sprite) -> f32 {
    if sprite.flip_x {
        1.0
    } else {
        0.0
    }
}

fn seed_from_id(id: &str) -> f32 {
    let mut hash: u32 = 0x811c9dc5;
    for byte in id.bytes() {
        hash ^= byte as u32;
        hash = hash.wrapping_mul(0x01000193);
    }
    (hash & 0x00ff_ffff) as f32 / 16_777_216.0
}

#[cfg(test)]
mod tests {
    use super::is_mary_o_form;

    #[test]
    fn quasar_shader_accepts_all_mary_o_power_forms_only() {
        assert!(is_mary_o_form("mary_o"));
        assert!(is_mary_o_form("mary_o_tall"));
        assert!(is_mary_o_form("mary_o_fire"));
        assert!(!is_mary_o_form("sanic"));
    }

    use super::*;

    /// A dead overlay must release its source, or the effect is dark forever.
    ///
    /// `attach_quasar_overlays` filters `Without<MaryOQuasarSource>`, so a source still holding a
    /// back-reference to a despawned overlay is never rebuilt.
    #[test]
    fn a_source_whose_overlay_died_lets_go_so_the_pair_can_be_rebuilt() {
        let mut app = App::new();
        app.add_systems(Update, cleanup_quasar_overlays);

        let overlay = app.world_mut().spawn_empty().id();
        let source = app
            .world_mut()
            .spawn(MaryOQuasarSource { overlay, seed: 0.0 })
            .id();
        app.world_mut()
            .entity_mut(overlay)
            .insert(MaryOQuasarOverlay { source });

        app.update();
        assert!(
            app.world().get::<MaryOQuasarSource>(source).is_some(),
            "an intact pair is left alone"
        );

        // The room rebuild takes the overlay out from under the source.
        app.world_mut().entity_mut(overlay).despawn();
        app.update();

        assert!(
            app.world().get::<MaryOQuasarSource>(source).is_none(),
            "the source released its dead overlay, so `attach` will rebuild the pair"
        );
    }

    /// The direction that already worked, kept honest.
    #[test]
    fn an_overlay_whose_source_died_is_despawned() {
        let mut app = App::new();
        app.add_systems(Update, cleanup_quasar_overlays);

        let source = app.world_mut().spawn_empty().id();
        let overlay = app.world_mut().spawn(MaryOQuasarOverlay { source }).id();
        app.world_mut()
            .entity_mut(source)
            .insert(MaryOQuasarSource { overlay, seed: 0.0 });

        app.world_mut().entity_mut(source).despawn();
        app.update();

        assert!(
            app.world().get_entity(overlay).is_err(),
            "an orphaned overlay is cleaned up"
        );
    }
}
