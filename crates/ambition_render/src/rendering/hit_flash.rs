//! The character-body overlay: damage flash and intangibility blink.
//!
//! ONE sibling `Material2d` mesh per character sprite carries both cues — the
//! same world-space sibling pattern content-owned overlays use (the
//! [`super::ActorOverlaySet`] seam) — with a tiny shader that outputs a flat
//! tint masked by the source sprite's alpha. When neither cue is showing the
//! shader discards every fragment (no GPU work) and the source sprite renders
//! normally.
//!
//! Three cues, one overlay, and the priority between them is decided in
//! [`overlay_look`] rather than by whichever pass wrote last:
//!
//! - **damage flash**, pure white, held then faded over its `hit_flash` timer;
//! - **intangibility blink**, a pale pulse for exactly as long as the body
//!   cannot be struck. It reads the sim's resolved `unhittable` fact, never a
//!   pose, a move name or an animation row, so it covers every grant the
//!   damage rule honours — dodge, tech/getup, ledge, respawn protection, the
//!   i-frames a hit leaves — and gains a new one the day the damage rule does.
//! - **smash-charge pulse**, a hot amber that pulses FASTER and brighter as the
//!   held charge fills, so latched / building / loaded are three readings of
//!   one number.
//!
//! The order is damage flash, then blink, then charge, and each step of it is
//! a decision about what an opponent needs to know first. Being struck is the
//! loudest fact and is over in a fifth of a second. Intangibility outranks the
//! charge because misreading it wastes a whole attack, where misreading a
//! charge costs spacing — and a body that is both is telling you the same
//! thing either way: do not go in.
//!
//! Every cue is drawn by sampling the SOURCE sprite's own atlas frame and flip
//! flag, so silhouette and facing are preserved by construction rather than by
//! a rule somebody has to keep.
//!
//! Source-of-truth per body kind:
//!
//! - Actor (NPC / enemy / seated fighter): the `FeatureView` row, by feature id.
//! - Boss: the boss encounter fields exposed through the same read-model seam.
//! - Player-bodied: the `BodyPoseView` component on the sprite's own entity.

use bevy::{
    image::TextureAtlasLayout,
    prelude::*,
    reflect::TypePath,
    render::render_resource::AsBindGroup,
    shader::ShaderRef,
    sprite::Anchor,
    sprite_render::{AlphaMode2d, Material2d, Material2dPlugin, MeshMaterial2d},
};

use super::primitives::{FeatureVisual, PlayerVisual, PropVisual};
use ambition_platformer2d_shared_tangle::lifecycle::{
    SessionScopedEntity, SessionSpawnScope, SpawnSessionScopedExt,
};

const SHADER_ASSET_PATH: &str = "shaders/hit_flash.wgsl";

/// Hold the flash at full intensity for the first 80% of the timer,
/// then fade smoothly to zero. Without this the flash ends in a
/// sudden cut that reads as a missing frame; the fade keeps the
/// transition readable at the cost of one extra frame of bright
/// pixels.
const FLASH_HOLD_FRACTION: f32 = 0.80;

const REFERENCE_FLASH_SECONDS: f32 = 0.24;

/// The intangibility blink, in SIM TICKS per cycle. Sim-derived rather than
/// wall-clock so the pulse is the same in a capture, a replay and on screen,
/// and the same at any refresh rate. At the shipped 60Hz step this is a ~6Hz
/// pulse — fast enough to read as "cannot be hit", slow enough not to strobe.
const BLINK_PERIOD_TICKS: u64 = 10;

/// Peak overlay intensity of the blink. Well under the damage flash's 1.0: the
/// tell has to be legible without erasing the silhouette it is drawn over.
const BLINK_PEAK_INTENSITY: f32 = 0.55;

/// The three cue colours. White is the strike, pale blue is "you cannot touch
/// this right now", amber is a held smash gathering.
const FLASH_TINT: Vec3 = Vec3::new(1.0, 1.0, 1.0);
const BLINK_TINT: Vec3 = Vec3::new(0.62, 0.86, 1.0);
const CHARGE_TINT: Vec3 = Vec3::new(1.0, 0.71, 0.28);

/// The smash-charge pulse, in cycles per SIM TICK at zero and full charge.
/// At the shipped 60Hz step that is a lazy ~2Hz throb when the hold latches
/// and a hard ~10Hz strobe when it is loaded — the rate IS the readout.
const CHARGE_RATE_LATCHED: f32 = 2.0 / 60.0;
const CHARGE_RATE_LOADED: f32 = 10.0 / 60.0;

/// Peak overlay intensity at zero and full charge. The pulse gets brighter as
/// well as faster so "loaded" is unmistakable at a glance.
const CHARGE_PEAK_LATCHED: f32 = 0.34;
const CHARGE_PEAK_LOADED: f32 = 0.72;

/// The tick count the pulse's phase wraps on. Large enough that the seam is
/// once a minute at 60Hz, small enough that `tick as f32` keeps its precision.
const CHARGE_PHASE_WRAP: u64 = 3600;

/// Z bias for the overlay mesh — must sit IN FRONT of every other
/// per-character overlay so the white silhouette is never covered.
/// Content-owned overlay siblings (the [`super::ActorOverlaySet`]
/// seam — e.g. Ambition's puppy-slug deep-dream material) sit at a
/// z bias of ~0.9, so a flash bias below that gets the white blanked
/// out. 1.5 gives a comfortable margin over those (0.9) and the
/// HazardColumn telegraph quad (+1.0 of boss z) without colliding
/// with HUD layers, which live in the hundreds.
const FLASH_OVERLAY_Z_BIAS: f32 = 1.5;

/// Install the material plugin behind the hit-flash overlay.
pub fn add_hit_flash_material_plugin(app: &mut App) {
    app.add_plugins(Material2dPlugin::<HitFlashMaterial>::default());
}

/// Material2d backing the white-silhouette overlay.
///
/// Bindings mirror the puppy-slug deep-dream material so the shader
/// driver can re-use the same WebGL2-friendly layout (vec4 uniforms,
/// no struct UBOs).
#[derive(Asset, AsBindGroup, TypePath, Debug, Clone)]
pub struct HitFlashMaterial {
    /// Current atlas frame as a UV rect on the loaded spritesheet.
    /// `(min.x, min.y, max.x, max.y)` normalized.
    #[uniform(0)]
    pub uv_rect: Vec4,
    /// `(intensity, flip_x, _, _)`. `intensity` is the shader's
    /// gate: 0.0 → discard everything; 1.0 → full white silhouette.
    #[uniform(1)]
    pub control: Vec4,
    #[texture(2)]
    #[sampler(3)]
    pub color_texture: Handle<Image>,
    /// `rgb` is the silhouette colour; `a` is unused. A uniform rather than a
    /// shader constant because ONE overlay draws every cue.
    #[uniform(4)]
    pub tint: Vec4,
}

impl Material2d for HitFlashMaterial {
    fn fragment_shader() -> ShaderRef {
        SHADER_ASSET_PATH.into()
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }
}

/// Marker placed on the source sprite entity once an overlay sibling
/// has been spawned for it. Stores the overlay's entity id so we can
/// despawn / re-sync it without scanning.
#[derive(Component, Debug, Clone, Copy)]
pub struct HitFlashSource {
    overlay: Entity,
}

/// Marker on the sibling mesh that runs the hit-flash material.
#[derive(Component, Debug, Clone, Copy)]
pub struct HitFlashOverlay {
    source: Entity,
}

/// Attach a flash overlay to every textured character sprite that
/// doesn't already have one. Gates on `FeatureVisual` / `PlayerVisual`
/// presence so prop visuals and one-shot VFX don't pick up the
/// overlay accidentally.
#[cfg(target_os = "android")]
pub fn attach_hit_flash_overlays() {}

/// Attach a flash overlay to every textured character sprite that
/// doesn't already have one. Gates on `FeatureVisual` / `PlayerVisual`
/// presence so prop visuals and one-shot VFX don't pick up the
/// overlay accidentally.
#[cfg(not(target_os = "android"))]
pub fn attach_hit_flash_overlays(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<HitFlashMaterial>>,
    texture_layouts: Res<Assets<TextureAtlasLayout>>,
    candidates: Query<
        (
            Entity,
            &Transform,
            &Sprite,
            Option<&Anchor>,
            Option<&FeatureVisual>,
            Option<&PlayerVisual>,
            Option<&SessionScopedEntity>,
        ),
        (Without<HitFlashSource>, Without<PropVisual>),
    >,
) {
    for (source_entity, transform, sprite, anchor, feature, player, session_owner) in &candidates {
        // Eligibility: a textured sprite (atlas OR plain image) that
        // belongs to a character — FeatureVisual covers
        // enemies/NPCs/bosses, PlayerVisual covers the player. Props
        // are excluded by the query filter.
        if feature.is_none() && player.is_none() {
            continue;
        }
        let Some(render_size) = sprite.custom_size else {
            // Sprites without `custom_size` haven't been sized by
            // the render pipeline yet (initial spawn frame). Skip
            // and try again next frame — the upgrade systems set
            // custom_size on the next tick.
            continue;
        };
        let Some(uv_rect) = current_sprite_uv_rect(sprite, &texture_layouts) else {
            // Texture / atlas not loaded yet; try again next frame.
            continue;
        };

        let material = materials.add(HitFlashMaterial {
            uv_rect,
            // Start hidden — `intensity = 0.0` causes the shader to
            // discard every fragment. The sync system bumps this
            // up whenever the source's hit_flash timer is positive.
            control: Vec4::new(0.0, flip_flag(sprite), 0.0, 0.0),
            color_texture: sprite.image.clone(),
            tint: FLASH_TINT.extend(1.0),
        });
        let mesh = meshes.add(Rectangle::default());
        let overlay_transform = overlay_transform_from_source(transform, anchor, render_size);
        let session_scope = session_owner.map_or(SessionSpawnScope::UNSCOPED, |owner| {
            SessionSpawnScope::scoped(owner.0)
        });
        let overlay_entity = commands
            .spawn_session_scoped(
                session_scope,
                (
                    Mesh2d(mesh),
                    MeshMaterial2d(material),
                    overlay_transform,
                    // Stay `Visible` always — the shader's `discard`
                    // arm zero-cost-culls fragments when `intensity == 0`,
                    // and starting with `Hidden` can stick the auto-inserted
                    // `InheritedVisibility` at false in a way that
                    // PostUpdate's propagator can't fix on the same tick
                    // (see the deep-dream comment for the same gotcha).
                    Visibility::Visible,
                    HitFlashOverlay {
                        source: source_entity,
                    },
                    // NOT `RoomVisual` — that requires `RoomScopedEntity`,
                    // and the room-transition pass despawns every
                    // RoomScopedEntity. The player isn't room-scoped, so
                    // adding RoomVisual here would orphan the player's
                    // HitFlashSource against a dead overlay every time
                    // the player crossed a loading zone, and the
                    // `Without<HitFlashSource>` attach gate would
                    // refuse to re-create it. Instead,
                    // `cleanup_hit_flash_overlays` despawns orphans by
                    // checking whether the source entity still has its
                    // `HitFlashSource` marker — that handles enemies'
                    // room-scoped sources cleanly without depending on
                    // RoomScopedEntity for the overlay itself.
                    Name::new("HitFlash Overlay"),
                ),
            )
            .id();
        // `try_insert`: this pass's own comment above notes that enemy sources
        // are ROOM-SCOPED, so a room transition despawns them — and a body can
        // take its last hit on the frame it is torn down. Marking a corpse as a
        // flash source has no meaning, and the same L23 reasoning applies: a
        // deferred presentation write must tolerate its target going away.
        commands.entity(source_entity).try_insert(HitFlashSource {
            overlay: overlay_entity,
        });
    }
}

/// Mirror the source sprite's atlas frame / facing / world transform
/// into the overlay material and toggle visibility based on the
/// source's current `hit_flash` timer.
#[cfg(target_os = "android")]
pub fn sync_hit_flash_overlays() {}

/// Mirror the source sprite's atlas frame / facing / world transform
/// into the overlay material and toggle visibility based on the
/// source's current `hit_flash` timer.
#[cfg(not(target_os = "android"))]
pub fn sync_hit_flash_overlays(
    mut commands: Commands,
    texture_layouts: Res<Assets<TextureAtlasLayout>>,
    // The blink's phase. Sim-derived: see `BLINK_PERIOD_TICKS`.
    tick: Res<ambition_time::SimTick>,
    // Sim-built read-models (E4 slices 2+5): a feature's flash timer rides
    // its `FeatureView` row; the player-bodied timer rides `BodyPoseView`
    // on the SAME entity that carries the sprite.
    feature_views: Res<ambition_sim_view::FeatureViewIndex>,
    anim_frames: Res<ambition_sim_view::ActorAnimIndex>,
    poses: Query<&ambition_sim_view::BodyPoseView>,
    sources: Query<
        (
            Entity,
            &Transform,
            &Sprite,
            Option<&Anchor>,
            Option<&FeatureVisual>,
            Option<&PlayerVisual>,
            &HitFlashSource,
            // The source's OWN visibility. The overlay is a separate root entity
            // that stays `Visible` forever (see the spawn site), so it does not
            // inherit a hidden source — see `overlay_look`.
            Option<&Visibility>,
        ),
        Without<HitFlashOverlay>,
    >,
    mut overlays: Query<(
        &mut Transform,
        &MeshMaterial2d<HitFlashMaterial>,
        &HitFlashOverlay,
    )>,
    mut materials: ResMut<Assets<HitFlashMaterial>>,
) {
    for (
        source_entity,
        source_transform,
        source_sprite,
        anchor,
        feature,
        player,
        source,
        source_visibility,
    ) in &sources
    {
        let Some(render_size) = source_sprite.custom_size else {
            continue;
        };
        let Some(uv_rect) = current_sprite_uv_rect(source_sprite, &texture_layouts) else {
            continue;
        };
        let flip = flip_flag(source_sprite);

        // Single dispatch covers every character type the universal
        // Brain/ActorControl architecture knows about — player, NPC,
        // enemy, boss. Each routes through a different per-entity
        // storage today (BodyCombat vs the actor cluster vs
        // the boss cluster components) but they all converge on one shader uniform
        // through this lookup. A future refactor that unifies them
        // into a single `HitFlash` component can collapse this to
        // one query without changing the overlay sync.
        let facts = overlay_facts_for_source(
            source_entity,
            feature,
            player,
            &feature_views,
            &anim_frames,
            &poses,
        );
        let (intensity, tint) = overlay_look(facts, tick.0, source_visibility.copied());

        let Ok((mut overlay_transform, material_handle, overlay)) =
            overlays.get_mut(source.overlay)
        else {
            // Overlay despawned underneath us (could happen if a
            // cleanup pass beat us this tick on a source that's
            // about to die). Drop the stale `HitFlashSource` so the
            // attach gate spawns a fresh overlay next frame instead
            // of letting the source flash silently forever.
            commands
                .entity(source_entity)
                .try_remove::<HitFlashSource>();
            continue;
        };
        if overlay.source != source_entity {
            continue;
        }
        // Visibility stays `Visible` permanently; the shader's
        // `discard` arm makes the overlay free when intensity == 0,
        // and we sidestep the InheritedVisibility-propagation gotcha
        // documented at the spawn site.
        *overlay_transform = overlay_transform_from_source(source_transform, anchor, render_size);
        if let Some(material) = materials.get_mut(&material_handle.0) {
            material.uv_rect = uv_rect;
            material.control = Vec4::new(intensity, flip, 0.0, 0.0);
            material.color_texture = source_sprite.image.clone();
            material.tint = tint.extend(1.0);
        }
    }
}

/// Remove orphan overlays whose source entity despawned. Mirrors the
/// deep-dream cleanup pass — without it a despawn between
/// FeatureViewSync and PresentationVisualAnimationPlugin can leave
/// the white silhouette frozen mid-air for one frame on the next
/// scene load.
#[cfg(target_os = "android")]
pub fn cleanup_hit_flash_overlays() {}

/// Remove orphan overlays whose source entity despawned. Mirrors the
/// deep-dream cleanup pass — without it a despawn between
/// FeatureViewSync and PresentationVisualAnimationPlugin can leave
/// the white silhouette frozen mid-air for one frame on the next
/// scene load.
#[cfg(not(target_os = "android"))]
pub fn cleanup_hit_flash_overlays(
    mut commands: Commands,
    sources: Query<(), With<HitFlashSource>>,
    overlays: Query<(Entity, &HitFlashOverlay)>,
) {
    for (overlay_entity, overlay) in &overlays {
        if sources.get(overlay.source).is_err() {
            commands.entity(overlay_entity).despawn();
        }
    }
}

/// What the overlay must show for one source this frame.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct OverlayFacts {
    /// Seconds left on the damage flash, if this source has that timer at all.
    pub hit_flash_secs: Option<f32>,
    /// The body cannot be struck right now. Resolved sim-side from the damage
    /// rule; presentation never re-derives it.
    pub unhittable: bool,
    /// A smash charge is being HELD, normalized `0..=1`. Resolved sim-side by
    /// `MovePlayback::smash_charge_fraction`; `None` the instant it releases.
    ///
    /// ⛔ never re-derived here from a move name or Startup progress — a tapped
    /// smash and a fully held one share both.
    pub smash_charge: Option<f32>,
}

/// Unified overlay-fact dispatch.
///
/// One entry point for every character type the universal-Brain unification
/// covers — caller doesn't need to know whether the source is a player, enemy,
/// NPC, or boss. Both roads publish the same two facts on their own read-model
/// row, so adding overlay feedback to a new body kind is "publish the row".
///
/// | type | read-model row |
/// |------|----------------|
/// | player-bodied | `BodyPoseView` on the sprite's own entity |
/// | enemy / NPC / seated fighter | the `FeatureView` row, by feature id |
/// | boss | the `FeatureView` row, by feature id |
fn overlay_facts_for_source(
    source_entity: Entity,
    feature: Option<&FeatureVisual>,
    player: Option<&PlayerVisual>,
    feature_views: &ambition_sim_view::FeatureViewIndex,
    // The charge rides the per-frame POSE row on the actor road, not the
    // feature row, so the two indexes are joined on the same feature id here.
    anim_frames: &ambition_sim_view::ActorAnimIndex,
    poses: &Query<&ambition_sim_view::BodyPoseView>,
) -> OverlayFacts {
    // Player path: the entity that carries `PlayerVisual` is the same one
    // that carries the sim-built `BodyPoseView`, so read ITS row —
    // per-entity, so player clones flash independently.
    if player.is_some() {
        return poses
            .get(source_entity)
            .map(|p| OverlayFacts {
                hit_flash_secs: Some(p.hit_flash_secs),
                unhittable: p.unhittable,
                smash_charge: p.smash_charge,
            })
            .unwrap_or_default();
    }
    // Feature path: the facts ride the `FeatureView` row (actors, seated
    // fighters and bosses alike; the "no silhouette over a boss corpse" rule
    // is applied at the rebuild site). Kinds with no body carry the defaults.
    let Some(feature) = feature else {
        return OverlayFacts::default();
    };
    let mut facts = feature_views
        .get(feature.id.as_str())
        .map(|view| OverlayFacts {
            hit_flash_secs: Some(view.hit_flash_secs),
            unhittable: view.unhittable,
            smash_charge: None,
        })
        .unwrap_or_default();
    facts.smash_charge = anim_frames
        .get(feature.id.as_str())
        .and_then(|frame| frame.smash_charge);
    facts
}

/// The overlay's shader intensity AND colour for one source this frame.
///
/// This is where the two cues are ARBITRATED, once, instead of each pass
/// writing the material and the last one winning. The damage flash outranks
/// the blink: a body struck out of its own dodge should read as struck.
///
/// A hidden body shows nothing. The overlay is a separate ROOT entity that
/// stays `Visibility::Visible` permanently — a deliberate workaround for the
/// `InheritedVisibility`-propagation gotcha documented at its spawn site — and it
/// is textured with the SOURCE sprite's own image. So it does not inherit a hidden
/// source: while the player is balled up (body `Hidden`, morph-ball sprite drawn),
/// taking a hit would have painted the robot's silhouette right over the ball.
///
/// Hiding a body must hide everything that draws it. `Visibility::Inherited` is
/// treated as visible here, which is correct at the top level and conservative
/// under a hidden ancestor: a fully-hidden hierarchy has an ancestor whose
/// overlay is likewise suppressed, and the shader's `discard` arm makes a
/// zero-intensity overlay free either way.
fn overlay_look(
    facts: OverlayFacts,
    tick: u64,
    source_visibility: Option<Visibility>,
) -> (f32, Vec3) {
    if matches!(source_visibility, Some(Visibility::Hidden)) {
        return (0.0, FLASH_TINT);
    }
    let flash = facts.hit_flash_secs.map_or(0.0, normalize_hit_flash);
    if flash > 0.0 {
        return (flash, FLASH_TINT);
    }
    if facts.unhittable {
        return (blink_intensity(tick), BLINK_TINT);
    }
    if let Some(charge) = facts.smash_charge {
        return (charge_pulse_intensity(charge, tick), CHARGE_TINT);
    }
    (0.0, FLASH_TINT)
}

/// The smash-charge pulse's intensity at one sim tick.
///
/// Both the RATE and the peak rise with the held fraction, monotonically, so
/// the same three beats are readable two ways at once: a slow dim throb when
/// the hold latches, a hard bright strobe when it is loaded. The exact curve is
/// a presentation tuning constant; that it never falls as the charge rises is
/// not.
fn charge_pulse_intensity(charge: f32, tick: u64) -> f32 {
    let charge = charge.clamp(0.0, 1.0);
    let rate = CHARGE_RATE_LATCHED + (CHARGE_RATE_LOADED - CHARGE_RATE_LATCHED) * charge;
    let peak = CHARGE_PEAK_LATCHED + (CHARGE_PEAK_LOADED - CHARGE_PEAK_LATCHED) * charge;
    // Phase as a fraction of a cycle. Wrapped before the float conversion so a
    // long match cannot grind the precision away.
    let phase = (((tick % CHARGE_PHASE_WRAP) as f32) * rate).fract();
    // Triangle, like the blink: a square wave reads as a dropped frame.
    peak * (1.0 - (2.0 * phase - 1.0).abs())
}

/// The blink's intensity at one sim tick: a triangle wave over
/// [`BLINK_PERIOD_TICKS`], peaking at [`BLINK_PEAK_INTENSITY`].
///
/// A triangle rather than an on/off square because a hard square at 6Hz reads
/// as a dropped frame; the ramp reads as a pulse.
fn blink_intensity(tick: u64) -> f32 {
    let phase = (tick % BLINK_PERIOD_TICKS) as f32 / BLINK_PERIOD_TICKS as f32;
    BLINK_PEAK_INTENSITY * (1.0 - (2.0 * phase - 1.0).abs())
}

/// Map raw seconds-remaining into a [0, 1] intensity. Holds at 1.0
/// for the first 80% of `REFERENCE_FLASH_SECONDS`, then ramps
/// linearly to 0 over the last 20%. Above `REFERENCE_FLASH_SECONDS`
/// stays clamped at 1.0; at or below zero stays at 0.0.
fn normalize_hit_flash(seconds: f32) -> f32 {
    if seconds <= 0.0 {
        return 0.0;
    }
    let fade_end = REFERENCE_FLASH_SECONDS * (1.0 - FLASH_HOLD_FRACTION);
    if seconds >= fade_end {
        1.0
    } else {
        (seconds / fade_end).clamp(0.0, 1.0)
    }
}

/// NO `Assets<Image>`, and the plain-image branch shows why it was never
/// needed. This fetched the image for one value — `texture_descriptor.size`,
/// to normalise the frame rect — which `TextureAtlasLayout::size` already
/// carries; and in the whole-image branch it computed that size and then
/// returned the constant `(0, 0, 1, 1)` without using it. There, the lookup was
/// doing nothing but gating on "has the texture decoded", a question the frame
/// rect does not depend on.
///
/// it mattered because of what the dependency BLOCKED. Bevy loads images as
/// `MAIN_WORLD | RENDER_WORLD`, so every decoded sheet keeps its full RGBA in
/// main-world RAM — 1803 MB entering Hall of Characters. Each main-world reader
/// of a loaded sheet is one more thing standing between the game and dropping
/// `MAIN_WORLD`, and this one wanted two integers.
///
/// there are THREE implementations of this computation — here,
/// `ambition_content::presentation::deep_dream`, and
/// `ambition_portal2d_presentation::clip_material::sprite_frame_basis` (whose
/// doc says it "mirrors the hit-flash overlay's UV resolution", which is a
/// citation, not a mechanism). All three now agree; converging them needs a home
/// both `ambition_render` and `ambition_portal2d_presentation` can reach, and
/// the dependency runs render → portal, so it is not either of them.
fn current_sprite_uv_rect(
    sprite: &Sprite,
    texture_layouts: &Assets<TextureAtlasLayout>,
) -> Option<Vec4> {
    let Some(atlas) = sprite.texture_atlas.as_ref() else {
        // Plain-image sprite: the whole texture is the "frame".
        return Some(Vec4::new(0.0, 0.0, 1.0, 1.0));
    };
    let layout = texture_layouts.get(&atlas.layout)?;
    let rect = layout.textures.get(atlas.index)?;
    let size = Vec2::new(layout.size.x.max(1) as f32, layout.size.y.max(1) as f32);
    Some(Vec4::new(
        rect.min.x as f32 / size.x,
        rect.min.y as f32 / size.y,
        rect.max.x as f32 / size.x,
        rect.max.y as f32 / size.y,
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
    transform.translation.z += FLASH_OVERLAY_Z_BIAS;
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Above the reference duration: full intensity.
    #[test]
    fn normalize_above_reference_saturates() {
        assert_eq!(normalize_hit_flash(0.5), 1.0);
        assert_eq!(normalize_hit_flash(REFERENCE_FLASH_SECONDS), 1.0);
    }

    /// At and below zero: dark.
    #[test]
    fn normalize_at_or_below_zero_is_dark() {
        assert_eq!(normalize_hit_flash(0.0), 0.0);
        assert_eq!(normalize_hit_flash(-0.1), 0.0);
    }

    /// Inside the fade window the value scales linearly.
    #[test]
    fn normalize_fades_in_final_window() {
        let fade_end = REFERENCE_FLASH_SECONDS * (1.0 - FLASH_HOLD_FRACTION);
        let mid = fade_end * 0.5;
        let intensity = normalize_hit_flash(mid);
        assert!(
            (intensity - 0.5).abs() < 1e-3,
            "expected ~0.5 at fade midpoint; got {intensity}",
        );
    }

    /// Above the fade window but below the reference: full white.
    #[test]
    fn normalize_in_hold_window_full_intensity() {
        let fade_end = REFERENCE_FLASH_SECONDS * (1.0 - FLASH_HOLD_FRACTION);
        let between = (fade_end + REFERENCE_FLASH_SECONDS) * 0.5;
        assert_eq!(normalize_hit_flash(between), 1.0);
    }

    /// A hidden body flashes nothing. The overlay is a separate root entity,
    /// permanently `Visible`, textured with the SOURCE's own sprite image. Nothing
    /// made it follow the source's visibility, so taking a hit while balled up
    /// (body `Hidden`, morph-ball sprite drawn) painted the robot's silhouette
    /// right over the ball. That is a live suspect for tracks.md's "morph ball
    /// still draws the robot".
    #[test]
    fn a_hidden_source_shows_nothing_however_hard_it_was_hit() {
        assert_eq!(look(flash(10.0), 0).0, 0.0);
        assert_eq!(look(flash(0.2), 0).0, 0.0);
        // And the blink is hidden by the same rule, at its own peak tick.
        assert_eq!(look(intangible(), BLINK_PERIOD_TICKS / 2).0, 0.0);

        fn look(facts: OverlayFacts, tick: u64) -> (f32, Vec3) {
            overlay_look(facts, tick, Some(Visibility::Hidden))
        }
    }

    /// The guard is narrow: a visible, inherited, or unknown source flashes
    /// exactly as it did before. `Inherited` reads as visible, which is right at
    /// the top level and harmless below one — a hidden ancestor suppresses its own
    /// overlay, and the shader discards a zero-intensity fragment for free.
    #[test]
    fn a_visible_source_still_flashes_exactly_as_before() {
        for vis in [Some(Visibility::Visible), Some(Visibility::Inherited), None] {
            let (intensity, tint) = overlay_look(flash(10.0), 0, vis);
            assert_eq!(intensity, normalize_hit_flash(10.0), "{vis:?}");
            assert_eq!(tint, FLASH_TINT);
            assert_eq!(overlay_look(OverlayFacts::default(), 0, vis).0, 0.0);
        }
    }

    /// THE PRIORITY, stated once: a body struck out of its own dodge reads as
    /// struck. Without this the blink would overwrite the flash on the exact
    /// frames the flash exists to mark.
    #[test]
    fn the_damage_flash_outranks_the_intangibility_blink() {
        let both = OverlayFacts {
            hit_flash_secs: Some(0.2),
            unhittable: true,
            smash_charge: None,
        };
        // Every tick of the blink cycle, including its peak.
        for tick in 0..BLINK_PERIOD_TICKS * 2 {
            let (intensity, tint) = overlay_look(both, tick, None);
            assert_eq!(tint, FLASH_TINT, "tick {tick}");
            assert_eq!(intensity, normalize_hit_flash(0.2), "tick {tick}");
        }
        // And once the flash drains, the blink takes over rather than nothing.
        let after = OverlayFacts {
            hit_flash_secs: Some(0.0),
            unhittable: true,
            smash_charge: None,
        };
        let (intensity, tint) = overlay_look(after, BLINK_PERIOD_TICKS / 2, None);
        assert_eq!(tint, BLINK_TINT);
        assert!(intensity > 0.0);
    }

    /// The blink runs for exactly as long as the body is unhittable, pulses
    /// rather than holding, and never reaches the flash's full white.
    #[test]
    fn the_blink_pulses_only_while_the_body_cannot_be_struck() {
        let mut peak: f32 = 0.0;
        let mut trough = f32::MAX;
        for tick in 0..BLINK_PERIOD_TICKS {
            let (intensity, tint) = overlay_look(intangible(), tick, None);
            assert_eq!(tint, BLINK_TINT);
            peak = peak.max(intensity);
            trough = trough.min(intensity);
        }
        assert!(peak > trough, "a blink that never varies is a tint");
        assert!(peak <= BLINK_PEAK_INTENSITY);
        assert!(peak < 1.0, "the blink must not erase the silhouette");
        // Hittable again: the overlay goes dark on the very next tick.
        assert_eq!(overlay_look(OverlayFacts::default(), 0, None).0, 0.0);
    }

    /// A merely raised shield is not intangibility. The predicate lives in the
    /// simulation, so all this side owes is: no fact, no blink.
    #[test]
    fn nothing_blinks_without_the_resolved_fact() {
        for tick in 0..BLINK_PERIOD_TICKS * 3 {
            assert_eq!(overlay_look(OverlayFacts::default(), tick, None).0, 0.0);
        }
    }

    /// THE THREE BEATS, from one number: the pulse gets both FASTER and
    /// brighter as the hold fills, and never dimmer.
    #[test]
    fn the_charge_pulse_quickens_and_brightens_monotonically() {
        let peak_over_a_cycle = |charge: f32| {
            // Long enough to contain a whole cycle at the slowest rate.
            (0..CHARGE_PHASE_WRAP)
                .map(|tick| charge_pulse_intensity(charge, tick))
                .fold(0.0_f32, f32::max)
        };
        let crossings = |charge: f32| {
            // How often the pulse returns to its bright half — the readable
            // proxy for "rate", measured rather than asserted from the constant.
            (1..600)
                .filter(|tick| {
                    let previous = charge_pulse_intensity(charge, tick - 1);
                    let now = charge_pulse_intensity(charge, *tick);
                    previous < now && previous == 0.0
                })
                .count()
        };

        let latched = peak_over_a_cycle(0.0);
        let half = peak_over_a_cycle(0.5);
        let loaded = peak_over_a_cycle(1.0);
        assert!(latched < half && half < loaded, "{latched} {half} {loaded}");
        assert!(loaded < 1.0, "the charge must not erase the silhouette");

        assert!(
            crossings(0.0) < crossings(1.0),
            "a loaded charge must pulse faster than a fresh one: {} vs {}",
            crossings(0.0),
            crossings(1.0)
        );
    }

    /// The charge is the LOWEST-priority cue, and the order is the point:
    /// being struck outranks everything, and intangibility outranks a charge
    /// because misreading it wastes a whole attack.
    #[test]
    fn a_charge_yields_to_both_louder_cues() {
        let charging = OverlayFacts {
            hit_flash_secs: Some(0.0),
            unhittable: false,
            smash_charge: Some(1.0),
        };
        let mid = (CHARGE_PHASE_WRAP / 7) as u64;
        assert_eq!(overlay_look(charging, mid, None).1, CHARGE_TINT);

        // Intangible while charging — an armoured smash — reads as intangible.
        let intangible_and_charging = OverlayFacts {
            unhittable: true,
            ..charging
        };
        assert_eq!(
            overlay_look(intangible_and_charging, mid, None).1,
            BLINK_TINT
        );

        // Struck while charging reads as struck, whatever else is true.
        let struck_while_charging = OverlayFacts {
            hit_flash_secs: Some(0.2),
            unhittable: true,
            smash_charge: Some(1.0),
        };
        assert_eq!(overlay_look(struck_while_charging, mid, None).1, FLASH_TINT);
    }

    /// No charge, no pulse — at every tick. The fact is the whole gate.
    #[test]
    fn nothing_pulses_without_a_held_charge() {
        for tick in 0..CHARGE_PHASE_WRAP {
            assert_eq!(overlay_look(OverlayFacts::default(), tick, None).0, 0.0);
        }
    }

    fn flash(secs: f32) -> OverlayFacts {
        OverlayFacts {
            hit_flash_secs: Some(secs),
            unhittable: false,
            smash_charge: None,
        }
    }

    fn intangible() -> OverlayFacts {
        OverlayFacts {
            hit_flash_secs: Some(0.0),
            unhittable: true,
            smash_charge: None,
        }
    }
}
