//! The character-body overlay: damage flash and intangibility blink.
//!
//! One sibling `Material2d` mesh per character sprite carries all cues. It uses
//! the same world-space sibling pattern as content-owned overlays (the
//! [`super::ActorOverlaySet`] seam), with a small shader that outputs a flat
//! tint masked by the source sprite's alpha. When no cue shows, the shader
//! discards every fragment and the source sprite renders normally.
//!
//! [`overlay_look`] decides the priority between the five cues:
//!
//! - **impact flash**: a hot, strength-scaled pop for exactly the hitlag of
//!   a connect. A jab gets little light and a smash a lot, with no threshold.
//! - **parry flash**: a hard white-gold snap for a perfect shield that caught
//!   a strike. It reads `parry_flash_secs`, not `parrying()`: the parry window
//!   is open on every guard raise.
//! - **damage flash**: pure white, held, then faded over its `hit_flash` timer.
//! - **intangibility blink**: a pale pulse, selected by the active route from
//!   the sim-published causes of untouchability. Character-owned effects stay
//!   independent, so Mary-O's empowerment can draw a quasar while a dodge
//!   still blinks.
//! - **smash-charge pulse**: amber that pulses faster and brighter as the
//!   held charge fills.
//!
//! Priority: impact, parry, damage flash, blink, charge. A landed hit is the
//! loudest fact and lasts a few frames; the damage flash is its tail. A parry
//! negates the strike, so it rarely collides with an impact; if a second strike
//! lands inside the parry beat, being struck wins. Intangibility outranks the
//! charge because misreading it wastes a whole attack.
//!
//! The parry flash is the only sign of a parry. A caught strike has no hit
//! event, no damage, and no shield-stress change.
//!
//! The blink and the charge pulse are pure functions of the sim tick, so they
//! resume at the correct phase after an impact.
//!
//! Every cue samples the source sprite's atlas frame and flip flag, so
//! silhouette and facing always match.
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

/// Hold the flash at full intensity for the first 80% of the timer, then fade
/// to zero. A sudden cut reads as a missing frame.
const FLASH_HOLD_FRACTION: f32 = 0.80;

const REFERENCE_FLASH_SECONDS: f32 = 0.24;

/// The intangibility blink, in sim ticks per cycle. Sim-derived so the pulse
/// is the same in a capture, a replay, and on screen, at any refresh rate. At
/// 60 Hz this is about 6 Hz: readable, but not a strobe.
const BLINK_PERIOD_TICKS: u64 = 10;

/// Peak blink intensity. Below the damage flash's 1.0, so the silhouette
/// stays visible.
const BLINK_PEAK_INTENSITY: f32 = 0.55;

/// The cue colours. White is a strike, pale blue is "cannot be touched now",
/// amber is a held smash charging.
const FLASH_TINT: Vec3 = Vec3::new(1.0, 1.0, 1.0);
const BLINK_TINT: Vec3 = Vec3::new(0.62, 0.86, 1.0);
const CHARGE_TINT: Vec3 = Vec3::new(1.0, 0.71, 0.28);

/// The smash-charge pulse, in cycles per sim tick at zero and full charge.
/// At 60 Hz: about 2 Hz when the hold latches, about 10 Hz when loaded.
const CHARGE_RATE_LATCHED: f32 = 2.0 / 60.0;
const CHARGE_RATE_LOADED: f32 = 10.0 / 60.0;

/// Peak intensity at zero and full charge. The pulse also brightens, so
/// "loaded" is clear at a glance.
const CHARGE_PEAK_LATCHED: f32 = 0.34;
const CHARGE_PEAK_LOADED: f32 = 0.72;

/// The tick count where the pulse phase wraps. The seam is once a minute at
/// 60 Hz, and `tick as f32` keeps its precision.
const CHARGE_PHASE_WRAP: u64 = 3600;

/// The impact flash colour: hotter and yellower than the damage flash's white,
/// so the connect and its fading tail read as two events.
const IMPACT_TINT: Vec3 = Vec3::new(1.0, 0.93, 0.74);

/// Impact intensity at the weakest connect and at the ceiling.
///
/// The floor is non-zero: every connect with hitlag gets a frame of light.
/// Strength, not a threshold, separates a jab from a smash.
const IMPACT_MIN_INTENSITY: f32 = 0.30;
const IMPACT_MAX_INTENSITY: f32 = 1.0;

/// The parry flash: a hard white-gold snap, brighter than the guard's window
/// colour, so a catch is clearly different from holding the shield.
const PARRY_TINT: Vec3 = Vec3::new(1.0, 0.97, 0.72);

/// How long a parry flash stays at full, as a fraction of the published beat,
/// and the beat it normalizes against.
///
/// Separate from the damage flash: a damage flash fades like a wound, a parry
/// snaps. This one holds almost the whole beat, then cuts.
const PARRY_HOLD_FRACTION: f32 = 0.70;
const REFERENCE_PARRY_SECONDS: f32 = 0.18;

/// Z bias for the overlay mesh. It must be in front of every other
/// per-character overlay. Content-owned overlays (the
/// [`super::ActorOverlaySet`] seam, for example the puppy-slug deep-dream
/// material) use ~0.9, and the HazardColumn telegraph quad uses +1.0 of boss
/// z. HUD layers are in the hundreds.
const FLASH_OVERLAY_Z_BIAS: f32 = 1.5;

/// Install the material plugin behind the hit-flash overlay.
pub fn add_hit_flash_material_plugin(app: &mut App) {
    // Standalone harnesses may not install the provider lifecycle. The default
    // has no shared defense effect; a route opts in through its policy.
    app.init_resource::<
        ambition_platformer2d_shared_tangle::gameplay_presentation::ActiveDefensePresentationPolicy,
    >();
    app.add_plugins(Material2dPlugin::<HitFlashMaterial>::default());
}

/// Material2d backing the white-silhouette overlay.
///
/// Bindings mirror the puppy-slug deep-dream material, so they use the same
/// WebGL2-friendly layout (vec4 uniforms, no struct UBOs).
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
    /// `rgb` is the silhouette colour; `a` is unused. A uniform because one
    /// overlay draws every cue.
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

/// Marker on the source sprite entity after its overlay sibling spawns.
/// Stores the overlay entity so it can be synced or despawned without a scan.
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
        // Eligible: a textured sprite (atlas or plain image) of a character.
        // `FeatureVisual` covers enemies, NPCs, and bosses; `PlayerVisual`
        // covers the player. The query filter excludes props.
        if feature.is_none() && player.is_none() {
            continue;
        }
        let Some(render_size) = sprite.custom_size else {
            // Not sized yet (first spawn frame). The upgrade systems set
            // `custom_size` next tick; try again then.
            continue;
        };
        let Some(uv_rect) = current_sprite_uv_rect(sprite, &texture_layouts) else {
            // Texture / atlas not loaded yet; try again next frame.
            continue;
        };

        let material = materials.add(HitFlashMaterial {
            uv_rect,
            // Start hidden: `intensity = 0.0` makes the shader discard every
            // fragment. The sync system raises it when a cue is active.
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
                    // Always spawn `Visible`. The shader discards fragments at zero
                    // intensity. Spawning `Hidden` can leave the auto-inserted
                    // `InheritedVisibility` false for that tick (same issue as the
                    // deep-dream overlay).
                    Visibility::Visible,
                    HitFlashOverlay {
                        source: source_entity,
                    },
                    // Declares what this mesh paints, so the portal compositor can
                    // clip it like a sprite instead of hiding it when its body is
                    // partly behind a pane. A unit quad scaled to the drawn size, with
                    // the anchor folded into the translation: `size` ONE, `anchor`
                    // ZERO. `sync_hit_flash_overlays` keeps it current.
                    ambition_sprite_fx::DeclaredFrame {
                        color_texture: sprite.image.clone(),
                        uv_rect,
                        flip_x: sprite.flip_x,
                        tint: FLASH_TINT.extend(0.0),
                        silhouette: true,
                        size: Vec2::ONE,
                        anchor: Vec2::ZERO,
                    },
                    // Ownership: which body this overlay draws. `source` above is for
                    // mirroring the sprite; `PresentationOf` is the shared spelling
                    // that consumers such as portal composition read.
                    ambition_platformer2d_shared_tangle::lifecycle::PresentationOf(
                        source_entity,
                    ),
                    // Not `RoomVisual`: that requires `RoomScopedEntity`, which the
                    // room transition despawns. The player is not room-scoped, so
                    // the overlay would die at every loading zone while the
                    // `HitFlashSource` stayed, and the attach gate would not
                    // re-create it. `cleanup_hit_flash_overlays` despawns orphans
                    // by checking the source's `HitFlashSource` marker instead.
                    Name::new("HitFlash Overlay"),
                ),
            )
            .id();
        // `try_insert`: enemy sources are room-scoped, and a body can take its
        // last hit on the frame a room transition despawns it. A deferred
        // presentation write must tolerate its target going away (L23).
        commands.entity(source_entity).try_insert(HitFlashSource {
            overlay: overlay_entity,
        });
    }
}

/// Mirror the source sprite's atlas frame, facing, and transform into the
/// overlay material, and set the cue intensity.
#[cfg(target_os = "android")]
pub fn sync_hit_flash_overlays() {}

/// Mirror the source sprite's atlas frame, facing, and transform into the
/// overlay material, and set the cue intensity.
#[cfg(not(target_os = "android"))]
pub fn sync_hit_flash_overlays(
    mut commands: Commands,
    texture_layouts: Res<Assets<TextureAtlasLayout>>,
    // The blink's phase. Sim-derived: see `BLINK_PERIOD_TICKS`.
    tick: Res<ambition_time::SimTick>,
    defense_policy: Res<
        ambition_platformer2d_shared_tangle::gameplay_presentation::ActiveDefensePresentationPolicy,
    >,
    // Sim-built read models: a feature's flash timer is on its
    // `FeatureView` row; a player body's is on `BodyPoseView` on the
    // sprite's own entity.
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
            // The source's own visibility. The overlay is a separate root, so it
            // does not inherit a hidden source. See `overlay_look`.
            Option<&Visibility>,
            // Except when the portal hides the source: then clipped pieces draw
            // the body, and the overlay composites into its own pieces, so it
            // must keep its intensity.
            PortalHidIt,
        ),
        Without<HitFlashOverlay>,
    >,
    mut overlays: Query<(
        &mut Transform,
        &MeshMaterial2d<HitFlashMaterial>,
        &HitFlashOverlay,
        &mut ambition_sprite_fx::DeclaredFrame,
        &mut Visibility,
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
        portal_hid_source,
    ) in &sources
    {
        let Some(render_size) = source_sprite.custom_size else {
            continue;
        };
        let Some(uv_rect) = current_sprite_uv_rect(source_sprite, &texture_layouts) else {
            continue;
        };
        let flip = flip_flag(source_sprite);

        // One dispatch for player, NPC, enemy, and boss. Each has different
        // storage, but all reach the same shader uniform through this lookup.
        let facts = overlay_facts_for_source(
            source_entity,
            feature,
            player,
            &feature_views,
            &anim_frames,
            &poses,
            defense_policy.0,
        );
        let source_visibility = if portal_hid_it(portal_hid_source) {
            None
        } else {
            source_visibility.copied()
        };
        let (intensity, tint) = overlay_look(facts, tick.0, source_visibility);

        let Ok((
            mut overlay_transform,
            material_handle,
            overlay,
            mut declared,
            mut overlay_visibility,
        )) = overlays.get_mut(source.overlay)
        else {
            // The overlay was despawned (a cleanup pass ran first on a dying
            // source). Remove the stale `HitFlashSource` so the attach gate
            // spawns a new overlay next frame.
            commands
                .entity(source_entity)
                .try_remove::<HitFlashSource>();
            continue;
        };
        if overlay.source != source_entity {
            continue;
        }
        // This system owns the overlay's visibility every frame. The overlay
        // is a compositing candidate: the portal resolver hides it while a pane
        // covers it and releases without asserting when the pane moves. So
        // this system asserts `Visible` every frame, and the resolver (later)
        // reasserts `Hidden` while it has a reason, like `sync_visuals` with
        // bodies. Do not skip the write when `PortalSourceHidden` is present:
        // on the frame a body crosses to the near side, the flash would vanish
        // for one frame.
        if *overlay_visibility != Visibility::Visible {
            *overlay_visibility = Visibility::Visible;
        }
        *overlay_transform = overlay_transform_from_source(source_transform, anchor, render_size);
        // Read before writing, and write only a change. `Assets::get_mut` marks
        // the asset modified, so the material is re-uploaded to the GPU that
        // frame. Overlays live forever, so an unconditional `get_mut` would
        // re-upload every idle overlay every frame, which is costly in
        // `prepare_assets<PreparedMaterial2d<HitFlashMaterial>>`. Same rule as
        // `converge_character_residency_to_active_quality`.
        let control = Vec4::new(intensity, flip, 0.0, 0.0);
        let tint = tint.extend(1.0);
        let unchanged = materials.get(&material_handle.0).is_some_and(|material| {
            material.uv_rect == uv_rect
                && material.control == control
                && material.tint == tint
                && material.color_texture == source_sprite.image
        });
        if !unchanged {
            if let Some(mut material) = materials.get_mut(&material_handle.0) {
                material.uv_rect = uv_rect;
                material.control = control;
                material.color_texture = source_sprite.image.clone();
                material.tint = tint;
            }
        }
        // Keep the declaration current, with the same read-before-write rule
        // (`Changed` still fans out).
        let declared_now = ambition_sprite_fx::DeclaredFrame {
            color_texture: source_sprite.image.clone(),
            uv_rect,
            flip_x: flip > 0.5,
            tint: Vec4::new(tint.x, tint.y, tint.z, intensity),
            silhouette: true,
            size: Vec2::ONE,
            anchor: Vec2::ZERO,
        };
        if *declared != declared_now {
            *declared = declared_now;
        }
    }
}

/// "Is the portal hiding this entity?" The render crate can ask only when
/// the portal presentation crate is composed in.
#[cfg(feature = "portal_render")]
type PortalHidIt = Has<ambition_portal2d_presentation::PortalSourceHidden>;
#[cfg(feature = "portal_render")]
fn portal_hid_it(has: bool) -> bool {
    has
}
/// Without portals nothing hides a body for the compositor's reasons.
#[cfg(not(feature = "portal_render"))]
type PortalHidIt = ();
#[cfg(not(feature = "portal_render"))]
fn portal_hid_it((): ()) -> bool {
    false
}

/// Remove orphan overlays whose source entity despawned, like the
/// deep-dream cleanup. Without it, a despawn between `FeatureViewSync` and
/// `PresentationVisualAnimationPlugin` can leave the silhouette frozen for
/// one frame on the next scene load.
#[cfg(target_os = "android")]
pub fn cleanup_hit_flash_overlays() {}

/// Remove orphan overlays whose source entity despawned, like the
/// deep-dream cleanup. Without it, a despawn between `FeatureViewSync` and
/// `PresentationVisualAnimationPlugin` can leave the silhouette frozen for
/// one frame on the next scene load.
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
    /// The active route opted at least one of this body's defense causes
    /// into the shared i-frame blink. Resolved from the sim cause mask and
    /// route policy; the renderer has no per-game special cases.
    pub iframe_blink: bool,
    /// Seconds left on a parry that caught a strike; usually `0.0`.
    /// Resolved sim-side from `BodyShieldState::parry_caught_timer`. Not the
    /// parry window; see the module docs.
    pub parry_flash_secs: f32,
    /// How hard the hit that freezes this body was, `0..=1`; `0.0` when no
    /// hitlag runs. Resolved sim-side from the hitlag the hit set.
    pub hit_strength: f32,
    /// A held smash charge, normalized `0..=1`, from
    /// `MovePlayback::smash_charge_fraction`; `None` on release. Do not derive
    /// it here from a move name or Startup progress: a tapped smash and a held
    /// one share both.
    pub smash_charge: Option<f32>,
}

/// Unified overlay-fact dispatch.
///
/// One entry point for every character type, so the caller need not know the
/// source kind. Each kind publishes the same facts on its read-model row, so a
/// new body kind only needs to publish the row.
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
    // The charge is on the per-frame pose row, not the feature row, so join
    // the two indexes on the feature id.
    anim_frames: &ambition_sim_view::ActorAnimIndex,
    poses: &Query<&ambition_sim_view::BodyPoseView>,
    defense_policy: ambition_platformer2d_shared_tangle::gameplay_presentation::DefensePresentationPolicy,
) -> OverlayFacts {
    // Player path: the `PlayerVisual` entity also carries `BodyPoseView`.
    // Per-entity, so player clones flash independently.
    if player.is_some() {
        return poses
            .get(source_entity)
            .map(|p| overlay_facts_from_pose(p, defense_policy))
            .unwrap_or_default();
    }
    // Feature path: facts are on the `FeatureView` row (actors, seated
    // fighters, bosses). The rebuild site applies "no silhouette over a boss
    // corpse". Kinds with no body get the defaults.
    let Some(feature) = feature else {
        return OverlayFacts::default();
    };
    let mut facts = feature_views
        .get(feature.id.as_str())
        .map(|view| overlay_facts_from_feature(view, defense_policy))
        .unwrap_or_default();
    facts.smash_charge = anim_frames
        .get(feature.id.as_str())
        .and_then(|frame| frame.smash_charge);
    facts
}

fn shared_iframe_blink(
    unhittable: bool,
    causes: ambition_platformer2d_shared_tangle::gameplay_presentation::DefenseCueCauses,
    policy: ambition_platformer2d_shared_tangle::gameplay_presentation::DefensePresentationPolicy,
) -> bool {
    unhittable && policy.resolve(causes).blink
}

fn overlay_facts_from_pose(
    pose: &ambition_sim_view::BodyPoseView,
    policy: ambition_platformer2d_shared_tangle::gameplay_presentation::DefensePresentationPolicy,
) -> OverlayFacts {
    OverlayFacts {
        hit_flash_secs: Some(pose.hit_flash_secs),
        parry_flash_secs: pose.parry_flash_secs,
        hit_strength: pose.hit_strength,
        iframe_blink: shared_iframe_blink(pose.unhittable, pose.defense_cues, policy),
        smash_charge: pose.smash_charge,
    }
}

fn overlay_facts_from_feature(
    view: &ambition_sim_view::FeatureView,
    policy: ambition_platformer2d_shared_tangle::gameplay_presentation::DefensePresentationPolicy,
) -> OverlayFacts {
    OverlayFacts {
        hit_flash_secs: Some(view.hit_flash_secs),
        parry_flash_secs: view.parry_flash_secs,
        hit_strength: view.hit_strength,
        iframe_blink: shared_iframe_blink(view.unhittable, view.defense_cues, policy),
        smash_charge: None,
    }
}

/// The overlay's shader intensity and colour for one source this frame.
///
/// The cues are arbitrated here, once, instead of by the last writer.
///
/// A hidden body shows nothing. The overlay is a separate root that stays
/// `Visible` (see its spawn site) and uses the source sprite's image, so it
/// does not inherit a hidden source. Without this check, a hit while balled
/// up (body `Hidden`, morph-ball sprite drawn) would paint the robot's
/// silhouette over the ball.
///
/// `Visibility::Inherited` counts as visible. That is correct at the top
/// level and safe under a hidden ancestor, whose own overlay is also
/// suppressed.
fn overlay_look(
    facts: OverlayFacts,
    tick: u64,
    source_visibility: Option<Visibility>,
) -> (f32, Vec3) {
    if matches!(source_visibility, Some(Visibility::Hidden)) {
        return (0.0, FLASH_TINT);
    }
    // Impact first: it lasts exactly the hitlag of the connect.
    let impact = impact_intensity(facts.hit_strength);
    if impact > 0.0 {
        return (impact, IMPACT_TINT);
    }
    // Parry: the only evidence, because a caught strike changes nothing else.
    let parry = normalize_parry_flash(facts.parry_flash_secs);
    if parry > 0.0 {
        return (parry, PARRY_TINT);
    }
    let flash = facts.hit_flash_secs.map_or(0.0, normalize_hit_flash);
    if flash > 0.0 {
        return (flash, FLASH_TINT);
    }
    if facts.iframe_blink {
        return (blink_intensity(tick), BLINK_TINT);
    }
    if let Some(charge) = facts.smash_charge {
        return (charge_pulse_intensity(charge, tick), CHARGE_TINT);
    }
    (0.0, FLASH_TINT)
}

/// The impact flash's intensity for a connect of this strength.
///
/// `0.0` only when no hitlag runs: a body in hitlag always flashes. Between
/// the floor and ceiling the intensity is proportional, with no threshold.
fn impact_intensity(strength: f32) -> f32 {
    if strength <= 0.0 {
        return 0.0;
    }
    let strength = strength.clamp(0.0, 1.0);
    IMPACT_MIN_INTENSITY + (IMPACT_MAX_INTENSITY - IMPACT_MIN_INTENSITY) * strength
}

/// Map a parry beat's seconds-remaining into a `0..=1` intensity.
///
/// Holds near full for most of the beat, then cuts, so it reads as a snap,
/// not a slow fade.
fn normalize_parry_flash(seconds: f32) -> f32 {
    if seconds <= 0.0 {
        return 0.0;
    }
    let fade_end = REFERENCE_PARRY_SECONDS * (1.0 - PARRY_HOLD_FRACTION);
    if seconds >= fade_end {
        1.0
    } else {
        (seconds / fade_end).clamp(0.0, 1.0)
    }
}

/// The smash-charge pulse's intensity at one sim tick.
///
/// Rate and peak both rise monotonically with the held fraction: a slow dim
/// throb when the hold latches, a hard bright strobe when loaded. The exact
/// curve is tuning; that it never falls as charge rises is a rule.
fn charge_pulse_intensity(charge: f32, tick: u64) -> f32 {
    let charge = charge.clamp(0.0, 1.0);
    let rate = CHARGE_RATE_LATCHED + (CHARGE_RATE_LOADED - CHARGE_RATE_LATCHED) * charge;
    let peak = CHARGE_PEAK_LATCHED + (CHARGE_PEAK_LOADED - CHARGE_PEAK_LATCHED) * charge;
    // Wrap before the float conversion so a long match keeps precision.
    let phase = (((tick % CHARGE_PHASE_WRAP) as f32) * rate).fract();
    // Triangle, like the blink: a square wave reads as a dropped frame.
    peak * (1.0 - (2.0 * phase - 1.0).abs())
}

/// The blink's intensity at one sim tick: a triangle wave over
/// [`BLINK_PERIOD_TICKS`], peaking at [`BLINK_PEAK_INTENSITY`].
///
/// A triangle, not a square: a hard square at 6 Hz reads as a dropped frame.
fn blink_intensity(tick: u64) -> f32 {
    let phase = (tick % BLINK_PERIOD_TICKS) as f32 / BLINK_PERIOD_TICKS as f32;
    BLINK_PEAK_INTENSITY * (1.0 - (2.0 * phase - 1.0).abs())
}

/// Map seconds remaining to a [0, 1] intensity. Holds 1.0 for the first 80% of
/// `REFERENCE_FLASH_SECONDS`, then ramps linearly to 0. Clamped to 1.0 above
/// the reference and 0.0 at or below zero.
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

/// Does not read `Assets<Image>`. The frame rect needs only the atlas size,
/// which `TextureAtlasLayout::size` has. Bevy keeps decoded sheets in
/// main-world RAM (`MAIN_WORLD | RENDER_WORLD`), and each main-world reader is
/// a blocker for dropping `MAIN_WORLD`.
///
/// Two other copies of this computation exist:
/// `ambition_content::presentation::deep_dream` and
/// `ambition_portal2d_presentation::clip_material::sprite_frame_basis`. All
/// three agree. Merging them needs a crate that both `ambition_render` and
/// `ambition_portal2d_presentation` can reach; render depends on portal, so it
/// cannot be either one.
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

    /// A hidden body flashes nothing. The overlay is a separate root, always
    /// `Visible`, using the source's image. Without the visibility check, a hit
    /// while balled up painted the robot silhouette over the morph ball.
    #[test]
    fn a_hidden_source_shows_nothing_however_hard_it_was_hit() {
        assert_eq!(look(flash(10.0), 0).0, 0.0);
        assert_eq!(look(flash(0.2), 0).0, 0.0);
        // The blink is hidden by the same rule, at its peak tick.
        assert_eq!(look(intangible(), BLINK_PERIOD_TICKS / 2).0, 0.0);

        fn look(facts: OverlayFacts, tick: u64) -> (f32, Vec3) {
            overlay_look(facts, tick, Some(Visibility::Hidden))
        }
    }

    /// The guard is narrow: a visible, inherited, or unknown source flashes as
    /// before.
    #[test]
    fn a_visible_source_still_flashes_exactly_as_before() {
        for vis in [Some(Visibility::Visible), Some(Visibility::Inherited), None] {
            let (intensity, tint) = overlay_look(flash(10.0), 0, vis);
            assert_eq!(intensity, normalize_hit_flash(10.0), "{vis:?}");
            assert_eq!(tint, FLASH_TINT);
            assert_eq!(overlay_look(OverlayFacts::default(), 0, vis).0, 0.0);
        }
    }

    /// The renderer gets semantic causes plus the active route policy, not a
    /// Mary-O special case. Ordinary iframe policy leaves content-owned
    /// empowerment alone; another route can opt that cause into the blink.
    #[test]
    fn route_policy_composes_character_owned_empowerment_with_shared_iframes() {
        use ambition_platformer2d_shared_tangle::gameplay_presentation::{
            DefenseCueCauses, DefensePresentationPolicy,
        };

        let ordinary_iframes = DefensePresentationPolicy::shared_iframe_blink();
        let empowerment_blinks = ordinary_iframes.with_blink(DefenseCueCauses::EMPOWERED);

        let mut pose = ambition_sim_view::BodyPoseView {
            unhittable: true,
            defense_cues: DefenseCueCauses::EMPOWERED,
            ..Default::default()
        };

        assert!(
            !overlay_facts_from_pose(&pose, ordinary_iframes).iframe_blink,
            "content-owned empowerment was implicitly treated as a shared iframe"
        );
        assert!(
            overlay_facts_from_pose(&pose, empowerment_blinks).iframe_blink,
            "a route cannot explicitly opt empowerment into the shared effect"
        );

        // Add a move iframe on the same body. The empowerment stays independent;
        // the move iframe opts into the shared blink.
        pose.defense_cues = DefenseCueCauses::EMPOWERED.union(DefenseCueCauses::MOVE_IFRAME);
        let facts = overlay_facts_from_pose(&pose, ordinary_iframes);
        assert!(facts.iframe_blink);
        assert!(
            overlay_look(facts, BLINK_PERIOD_TICKS / 2, None).0 > 0.0,
            "a content-owned effect swallowed a simultaneous shared iframe cue"
        );

        // Respawn protection is independently optable as an ordinary iframe.
        pose.defense_cues = DefenseCueCauses::RESPAWN;
        assert!(overlay_facts_from_pose(&pose, ordinary_iframes).iframe_blink);

        // Causes do not override the canonical hit-eligibility fact.
        pose.unhittable = false;
        assert!(!overlay_facts_from_pose(&pose, ordinary_iframes).iframe_blink);
    }

    /// A body struck out of its own dodge reads as struck. Otherwise the blink
    /// would hide the flash on the frames the flash marks.
    #[test]
    fn the_damage_flash_outranks_the_intangibility_blink() {
        let both = OverlayFacts {
            hit_flash_secs: Some(0.2),
            parry_flash_secs: 0.0,
            hit_strength: 0.0,
            iframe_blink: true,
            smash_charge: None,
        };
        // Every tick of the blink cycle, including its peak.
        for tick in 0..BLINK_PERIOD_TICKS * 2 {
            let (intensity, tint) = overlay_look(both, tick, None);
            assert_eq!(tint, FLASH_TINT, "tick {tick}");
            assert_eq!(intensity, normalize_hit_flash(0.2), "tick {tick}");
        }
        // Once the flash drains, the blink takes over.
        let after = OverlayFacts {
            hit_flash_secs: Some(0.0),
            parry_flash_secs: 0.0,
            hit_strength: 0.0,
            iframe_blink: true,
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

    /// The pulse gets faster and brighter as the hold fills, never dimmer.
    #[test]
    fn the_charge_pulse_quickens_and_brightens_monotonically() {
        let peak_over_a_cycle = |charge: f32| {
            // Long enough to contain a whole cycle at the slowest rate.
            (0..CHARGE_PHASE_WRAP)
                .map(|tick| charge_pulse_intensity(charge, tick))
                .fold(0.0_f32, f32::max)
        };
        let crossings = |charge: f32| {
            // How often the pulse returns to its bright half: a measured proxy
            // for rate.
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

    /// The charge is the lowest-priority cue. Being struck outranks everything;
    /// intangibility outranks a charge because misreading it wastes an attack.
    #[test]
    fn a_charge_yields_to_both_louder_cues() {
        let charging = OverlayFacts {
            hit_flash_secs: Some(0.0),
            parry_flash_secs: 0.0,
            hit_strength: 0.0,
            iframe_blink: false,
            smash_charge: Some(1.0),
        };
        let mid = (CHARGE_PHASE_WRAP / 7) as u64;
        assert_eq!(overlay_look(charging, mid, None).1, CHARGE_TINT);

        // Intangible while charging (an armoured smash) reads as intangible.
        let intangible_and_charging = OverlayFacts {
            iframe_blink: true,
            ..charging
        };
        assert_eq!(
            overlay_look(intangible_and_charging, mid, None).1,
            BLINK_TINT
        );

        // Struck while charging reads as struck, whatever else is true.
        let struck_while_charging = OverlayFacts {
            hit_flash_secs: Some(0.2),
            parry_flash_secs: 0.0,
            hit_strength: 0.0,
            iframe_blink: true,
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

    /// A connect flashes in proportion to its strength, with no threshold. A
    /// body not in hitlag does not flash.
    #[test]
    fn the_impact_flash_scales_with_the_connect_rather_than_switching_on() {
        assert_eq!(impact_intensity(0.0), 0.0, "no hitlag, no impact");
        assert_eq!(impact_intensity(-1.0), 0.0);

        // Every connect that produced hitlag is worth light.
        let weakest = impact_intensity(f32::EPSILON);
        assert!(weakest >= IMPACT_MIN_INTENSITY, "{weakest}");

        // Proportional across the band, monotone, and capped.
        let mut previous = 0.0;
        for strength in [0.1, 0.25, 0.5, 0.75, 1.0] {
            let now = impact_intensity(strength);
            assert!(
                now > previous,
                "{strength} did not rise: {now} <= {previous}"
            );
            previous = now;
        }
        assert_eq!(impact_intensity(1.0), IMPACT_MAX_INTENSITY);
        assert_eq!(impact_intensity(9.0), IMPACT_MAX_INTENSITY);
    }

    /// The impact is the loudest cue, and the states it interrupts resume at
    /// their tick phase instead of restarting.
    #[test]
    fn an_impact_interrupts_the_states_without_resetting_them() {
        let struck_mid_charge = OverlayFacts {
            hit_flash_secs: Some(0.2),
            parry_flash_secs: 0.0,
            hit_strength: 0.8,
            iframe_blink: true,
            smash_charge: Some(0.5),
        };
        let tick = 17;
        assert_eq!(
            overlay_look(struck_mid_charge, tick, None).1,
            IMPACT_TINT,
            "a landed hit outranks the flash, the blink and the pulse"
        );

        // When hitlag ends, the states are where the tick puts them.
        let after = OverlayFacts {
            hit_strength: 0.0,
            ..struck_mid_charge
        };
        let (_, tint) = overlay_look(after, tick, None);
        assert_eq!(tint, FLASH_TINT, "the damage tail takes over next");

        let blinking = OverlayFacts {
            hit_flash_secs: Some(0.0),
            parry_flash_secs: 0.0,
            hit_strength: 0.0,
            iframe_blink: true,
            smash_charge: Some(0.5),
        };
        assert_eq!(
            overlay_look(blinking, tick, None).0,
            blink_intensity(tick),
            "the blink resumes at the tick's phase, not from the start"
        );

        let pulsing = OverlayFacts {
            iframe_blink: false,
            ..blinking
        };
        assert_eq!(
            overlay_look(pulsing, tick, None).0,
            charge_pulse_intensity(0.5, tick),
            "and so does the charge pulse"
        );
    }

    /// A caught parry snaps, and it is the only evidence: the body took no hit,
    /// so nothing else changed.
    #[test]
    fn a_caught_parry_snaps_and_is_the_only_evidence() {
        // A parried body: unhittable while the window is open, no charge, no
        // wound. This is the state a real parry leaves.
        let parried = OverlayFacts {
            hit_flash_secs: Some(0.0),
            parry_flash_secs: REFERENCE_PARRY_SECONDS,
            hit_strength: 0.0,
            iframe_blink: true,
            smash_charge: None,
        };
        let (intensity, tint) = overlay_look(parried, 0, None);
        assert_eq!(tint, PARRY_TINT, "the parry outranks the i-frame blink");
        assert_eq!(intensity, 1.0);

        // It snaps: near full for most of the beat, then cuts.
        let fade_end = REFERENCE_PARRY_SECONDS * (1.0 - PARRY_HOLD_FRACTION);
        assert_eq!(normalize_parry_flash(fade_end), 1.0);
        let cutting = normalize_parry_flash(fade_end * 0.5);
        assert!(cutting > 0.0 && cutting < 1.0, "{cutting}");
        assert_eq!(normalize_parry_flash(0.0), 0.0);
        assert_eq!(normalize_parry_flash(-1.0), 0.0);

        // Beat over: the blink it was covering resumes at the tick's phase.
        let after = OverlayFacts {
            parry_flash_secs: 0.0,
            ..parried
        };
        assert_eq!(overlay_look(after, 7, None).0, blink_intensity(7));
    }

    /// If a second strike lands inside the parry beat, being struck wins.
    #[test]
    fn a_strike_landing_inside_the_parry_beat_still_reads_as_a_strike() {
        let struck_mid_parry = OverlayFacts {
            hit_flash_secs: Some(0.2),
            parry_flash_secs: REFERENCE_PARRY_SECONDS,
            hit_strength: 0.6,
            iframe_blink: false,
            smash_charge: None,
        };
        assert_eq!(overlay_look(struck_mid_parry, 0, None).1, IMPACT_TINT);
    }

    /// No caught strike, no snap — at every tick, whatever else is true.
    #[test]
    fn nothing_snaps_without_a_caught_parry() {
        let raised_guard = OverlayFacts {
            hit_flash_secs: Some(0.0),
            parry_flash_secs: 0.0,
            hit_strength: 0.0,
            // A raised guard in its parry window is unhittable: the state a cue
            // driven by `parrying()` would fire on.
            iframe_blink: true,
            smash_charge: None,
        };
        for tick in 0..BLINK_PERIOD_TICKS * 3 {
            assert_eq!(
                overlay_look(raised_guard, tick, None).1,
                BLINK_TINT,
                "tick {tick} must read as the i-frame blink, not a parry"
            );
        }
    }

    fn flash(secs: f32) -> OverlayFacts {
        OverlayFacts {
            hit_flash_secs: Some(secs),
            parry_flash_secs: 0.0,
            hit_strength: 0.0,
            iframe_blink: false,
            smash_charge: None,
        }
    }

    fn intangible() -> OverlayFacts {
        OverlayFacts {
            hit_flash_secs: Some(0.0),
            parry_flash_secs: 0.0,
            hit_strength: 0.0,
            iframe_blink: true,
            smash_charge: None,
        }
    }
}
