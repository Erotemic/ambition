//! The character-body overlay: damage flash and intangibility blink.
//!
//! ONE sibling `Material2d` mesh per character sprite carries both cues — the
//! same world-space sibling pattern content-owned overlays use (the
//! [`super::ActorOverlaySet`] seam) — with a tiny shader that outputs a flat
//! tint masked by the source sprite's alpha. When neither cue is showing the
//! shader discards every fragment (no GPU work) and the source sprite renders
//! normally.
//!
//! Five cues, one overlay, and the priority between them is decided in
//! [`overlay_look`] rather than by whichever pass wrote last:
//!
//! - **impact flash**, a hot strength-scaled pop for exactly the hitlag a
//!   connect bought. A jab gets almost nothing and a smash gets a wall of
//!   light, off one resolved number and with no threshold in between;
//! - **parry flash**, a hard white-gold snap for a perfect shield that
//!   actually CAUGHT a strike. It reads `parry_flash_secs`, never
//!   `parrying()` — the window standing open is true of every raised guard for
//!   a few ticks, and a cue driven off that fires on every shield raise;
//! - **damage flash**, pure white, held then faded over its `hit_flash` timer;
//! - **intangibility blink**, a pale pulse selected by the ACTIVE ROUTE from
//!   the sim-published semantic causes of untouchability. Games opt causes into
//!   this shared cue; character-owned effects remain independent, so Mary-O's
//!   empowerment can draw a quasar while a simultaneous dodge still blinks.
//! - **smash-charge pulse**, a hot amber that pulses FASTER and brighter as the
//!   held charge fills, so latched / building / loaded are three readings of
//!   one number.
//!
//! The order is impact, then parry, then damage flash, then blink, then
//! charge, and each step of it is a decision about what an opponent needs to
//! know first. A landed hit is the loudest fact there is and it is over in a
//! few frames; its tail is the damage flash. A parry sits just under it and
//! the two almost never collide — a parry is now a full negation, so the body
//! that caught the strike took no hit to flash for. Where they DO collide is a
//! second strike arriving inside the parry's beat, and there being struck is
//! the more urgent correction. Intangibility outranks the charge because
//! misreading it wastes a whole attack, where misreading a charge costs
//! spacing — and a body that is both is telling you the same thing either way:
//! do not go in.
//!
//! The parry flash is the ONLY thing that tells a spectator a parry happened.
//! Because a caught strike is negated outright — no hit event, no landed-hit
//! fact, no cost to the guard — there is no impact, no damage flash and no
//! shield-stress change to infer it from.
//!
//! The two cues an impact interrupts are STATES, and they RESUME rather than
//! restart: both are pure functions of the sim tick, so when the flash ends
//! the blink and the pulse are exactly where they would have been. That is
//! what lets the impact be brief without costing the state it covered.
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

/// The impact flash's colour: hotter and yellower than the damage flash's
/// white, so a heavy connect and its own fading tail are two readings rather
/// than one long one.
const IMPACT_TINT: Vec3 = Vec3::new(1.0, 0.93, 0.74);

/// Overlay intensity of an impact at the weakest connect and at the ceiling.
///
/// The floor is deliberately non-zero: every connect that produces hitlag at
/// all is worth a frame of light, and the strength read is what separates a
/// jab from a smash — not a threshold. Playtesting can add one; nothing here
/// assumes it.
const IMPACT_MIN_INTENSITY: f32 = 0.30;
const IMPACT_MAX_INTENSITY: f32 = 1.0;

/// The parry flash: a hard white-gold snap, brighter than the guard's own
/// window colour so the catch is unmistakably a different event from holding
/// the shield up.
const PARRY_TINT: Vec3 = Vec3::new(1.0, 0.97, 0.72);

/// How long a parry flash stays at full before falling away, as a fraction of
/// the published beat, and the beat this normalizes against.
///
/// A separate reference from the damage flash's because the two are different
/// KINDS of event: a damage flash is a wound fading, a parry is a snap. This
/// one holds almost the whole beat and then cuts.
const PARRY_HOLD_FRACTION: f32 = 0.70;
const REFERENCE_PARRY_SECONDS: f32 = 0.18;

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
    // Standalone renderer harnesses may not install the provider lifecycle. The
    // default is deliberately no shared defense effect; a gameplay route opts
    // into one through its authored presentation policy.
    app.init_resource::<
        ambition_platformer2d_shared_tangle::gameplay_presentation::ActiveDefensePresentationPolicy,
    >();
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
    defense_policy: Res<
        ambition_platformer2d_shared_tangle::gameplay_presentation::ActiveDefensePresentationPolicy,
    >,
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
            defense_policy.0,
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
        // ⛔⛔ READ BEFORE WRITING, AND ONLY WRITE A CHANGE. `Assets::get_mut`
        // MARKS THE ASSET MODIFIED, and a modified material is re-uploaded to the
        // GPU that frame. These overlays are deliberately kept alive forever (see
        // the visibility note above — the shader's `discard` makes an idle one
        // free), so an unconditional `get_mut` re-uploaded EVERY overlay EVERY
        // frame including the idle ones.
        //
        // Measured on hardware 2026-08-29:
        // `prepare_assets<PreparedMaterial2d<HitFlashMaterial>>` cost **312.8us
        // mean over 28,353 frames — 8.87s of the session**, the largest recurring
        // cost in the trace, for an effect that is invisible most of the time.
        //
        // ⭐ The rule is already written down in this repo, in
        // `converge_character_residency_to_active_quality`: *"it is READ first,
        // because a `ResMut` deref-mut marks it changed for every reader
        // downstream, every frame, forever."* Same defect, different asset.
        let control = Vec4::new(intensity, flip, 0.0, 0.0);
        let tint = tint.extend(1.0);
        let unchanged = materials.get(&material_handle.0).is_some_and(|material| {
            material.uv_rect == uv_rect
                && material.control == control
                && material.tint == tint
                && material.color_texture == source_sprite.image
        });
        if !unchanged {
            if let Some(material) = materials.get_mut(&material_handle.0) {
                material.uv_rect = uv_rect;
                material.control = control;
                material.color_texture = source_sprite.image.clone();
                material.tint = tint;
            }
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
    /// The active route opted at least one of this body's semantic defense
    /// causes into the shared i-frame blink. Resolved from the sim-published
    /// cause mask plus route policy; the renderer does not special-case games,
    /// characters, or individual invulnerability reasons.
    pub iframe_blink: bool,
    /// Seconds left on a parry that actually CAUGHT a strike; `0.0` almost
    /// always. Resolved sim-side from `BodyShieldState::parry_caught_timer`.
    ///
    /// ⛔ not the parry WINDOW. See the module docs.
    pub parry_flash_secs: f32,
    /// How hard the hit currently freezing this body was, `0..=1`; `0.0` when
    /// no hitlag is running. Resolved sim-side from the hitlag the hit already
    /// set, so nothing here touches hit resolution.
    pub hit_strength: f32,
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
    defense_policy: ambition_platformer2d_shared_tangle::gameplay_presentation::DefensePresentationPolicy,
) -> OverlayFacts {
    // Player path: the entity that carries `PlayerVisual` is the same one
    // that carries the sim-built `BodyPoseView`, so read ITS row —
    // per-entity, so player clones flash independently.
    if player.is_some() {
        return poses
            .get(source_entity)
            .map(|p| overlay_facts_from_pose(p, defense_policy))
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
    // The IMPACT, first and briefest: it lasts exactly the hitlag the connect
    // bought, which is a few frames for a jab and a real beat for a smash.
    let impact = impact_intensity(facts.hit_strength);
    if impact > 0.0 {
        return (impact, IMPACT_TINT);
    }
    // The PARRY, and it is the only evidence there is: a caught strike is
    // negated outright, so nothing else on this body changed to imply it.
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
/// `0.0` means no hitlag is running, and only that: a body IN hitlag always
/// flashes, because the weakest connect the hitlag law admits is still a
/// connect. Between the floor and the ceiling the read is proportional, so a
/// jab and a smash differ by how much light rather than by whether there is
/// any — no threshold, and none needed unless playtesting asks.
fn impact_intensity(strength: f32) -> f32 {
    if strength <= 0.0 {
        return 0.0;
    }
    let strength = strength.clamp(0.0, 1.0);
    IMPACT_MIN_INTENSITY + (IMPACT_MAX_INTENSITY - IMPACT_MIN_INTENSITY) * strength
}

/// Map a parry beat's seconds-remaining into a `0..=1` intensity.
///
/// Holds near full for most of the beat and then cuts away, which is what makes
/// it read as a SNAP rather than as the damage flash's slower bloom-and-fade.
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

    /// The renderer receives semantic causes plus the ACTIVE ROUTE policy, not
    /// a Mary-O-shaped exception boolean. Ordinary iframe policy leaves
    /// content-owned empowerment alone, while another route may opt that cause
    /// into the shared blink explicitly.
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

        // Add an independent defensive grant on the SAME body. The character-owned
        // empowerment remains independent while the move iframe opts the shared
        // blink in.
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

    /// THE PRIORITY, stated once: a body struck out of its own dodge reads as
    /// struck. Without this the blink would overwrite the flash on the exact
    /// frames the flash exists to mark.
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
        // And once the flash drains, the blink takes over rather than nothing.
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
            parry_flash_secs: 0.0,
            hit_strength: 0.0,
            iframe_blink: false,
            smash_charge: Some(1.0),
        };
        let mid = (CHARGE_PHASE_WRAP / 7) as u64;
        assert_eq!(overlay_look(charging, mid, None).1, CHARGE_TINT);

        // Intangible while charging — an armoured smash — reads as intangible.
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

    /// A connect flashes in proportion to how hard it was, with no threshold
    /// in between — and a body that is not in hitlag does not flash at all.
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

    /// The impact is the LOUDEST cue, and the states it interrupts RESUME
    /// where they would have been rather than restarting. That is what lets it
    /// be brief without costing the state it covered.
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

        // The moment the hitlag ends, the states are exactly where the tick
        // says they should be — not restarted from zero.
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

    /// A caught parry snaps, and it is the ONLY evidence: the body that
    /// caught the strike took no hit, so nothing else on it changed.
    #[test]
    fn a_caught_parry_snaps_and_is_the_only_evidence() {
        // A parried body is unhittable while its window is open and is holding
        // no charge and no wound — exactly the state a real parry leaves.
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

        // It SNAPS: near full for most of the beat, then cuts.
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

    /// Where a parry and an impact DO collide — a second strike arriving
    /// inside the parry's beat — being struck is the more urgent correction.
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
            // A raised guard inside its parry WINDOW is unhittable, which is
            // exactly the state a cue driven off `parrying()` would fire on.
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
