//! Procedural visual effects for the sandbox.
//!
//! Particles are CPU-side Bevy sprite entities for now. Keeping this behind a
//! compact module gives us a later migration seam to GPU particles or Hanabi.

use ambition_platformer2d_core as ae;
use bevy::math::Vec2 as BVec2;
use bevy::prelude::*;
use bevy::text::TextBounds;
use std::f32::consts::TAU;

use ambition_platformer2d_core::config::{world_to_bevy, WORLD_Z_FX};
use ambition_platformer2d_shared_tangle::lifecycle::{
    ActiveSessionScope, SessionSpawnScope, SpawnSessionScopedExt,
};
use ambition_sfx::{SfxId, SfxMessage, SfxWriter};
use ambition_sprite_sheet::character::CharacterAnimator;
use ambition_sprite_sheet::fx::{authored_effects, AuthoredEffect};
use ambition_vfx::FxId;

// The VFX MESSAGE vocabulary now lives in the foundation crate `ambition_vfx`
// (presentation-neutral data, so a sim system can emit a cue without depending on
// this render module). Re-exported here so existing `crate::fx::*`
// paths keep resolving.
pub use ambition_vfx::vfx::{FireworksRequest, FxRequest, ParticleKind, SlashKind, VfxMessage};

/// What an [`FxId`] names: the authored row, the sheet holding it, and the
/// packed cue that ships with it.
///
/// this index IS the engine's effect vocabulary, and nothing declares it.
/// It is built by walking the FX sheets' own baked records and hashing each row
/// name — so the set of drawable effects is the set of shipped rows, by
/// construction. That is what replaced `move_vfx_kind` (name→enum),
/// `explosion_anim` (enum→pose) and `explosion_sfx` (enum→cue): three tables
/// whose whole job was getting back to the string the content already had.
///
/// `SfxId` is precomputed here rather than at every spawn: the cue name is
/// derived from the row (`vfx.<family>.<row>`), and hashing it once at first
/// use keeps the draw path allocation-free.
struct EffectIndex {
    by_id: std::collections::HashMap<FxId, (&'static AuthoredEffect, SfxId)>,
}

fn effect_index() -> &'static EffectIndex {
    static INDEX: std::sync::OnceLock<EffectIndex> = std::sync::OnceLock::new();
    INDEX.get_or_init(|| EffectIndex {
        by_id: authored_effects()
            .values()
            .map(|effect| (FxId::new(effect.name), (effect, SfxId::new(&effect.cue))))
            .collect(),
    })
}

/// The authored effect `fx` names, if the shipped art has a row for it.
pub fn authored_effect_for(fx: FxId) -> Option<&'static AuthoredEffect> {
    effect_index().by_id.get(&fx).map(|(effect, _)| *effect)
}

/// The sound `fx` makes. A property of the NAME, not of the call site: the
/// bank ships one `vfx.<family>.<row>` cue for every authored row, so an
/// emitter that says which effect has already said which sound.
pub fn effect_cue(fx: FxId) -> Option<SfxId> {
    effect_index().by_id.get(&fx).map(|(_, cue)| *cue)
}

/// Say once, per id, that an effect named nothing.
///
/// SFX's policy, for SFX's reason: the vocabulary is open (a game may author
/// effects the engine never heard of), so a miss is a report rather than a
/// refusal — but a per-frame warning for a move that fires sixty times a second
/// trains everyone to filter the channel. the id is a one-way hash and the
/// name is not among the shipped rows *by definition of being a miss*, so the
/// report can only print the hash. That is the honest amount of information.
fn note_effect_miss(fx: FxId) {
    use std::collections::HashSet;
    use std::sync::Mutex;
    static SEEN: std::sync::OnceLock<Mutex<HashSet<u64>>> = std::sync::OnceLock::new();
    let seen = SEEN.get_or_init(|| Mutex::new(HashSet::new()));
    let first = seen
        .lock()
        .map(|mut set| set.insert(fx.hash()))
        .unwrap_or(false);
    if first {
        bevy::log::warn!(
            target: "crate::fx",
            "{fx} is not a row on any of the {} shipped FX sheets, so it draws a generic \
             particle burst instead of art; check the authored `Vfx {{ effect }}` id",
            ambition_sprite_sheet::fx::FX_SHEETS.len(),
        );
    }
}

fn rgba(r: f32, g: f32, b: f32, a: f32) -> Color {
    Color::srgba(r, g, b, a.clamp(0.0, 1.0))
}

#[derive(Component)]
pub struct ParticleVisual {
    kind: ParticleKind,
    pos: ae::Vec2,
    vel: ae::Vec2,
    age: f32,
    lifetime: f32,
    radius: f32,
    rgba: [f32; 4],
    gravity: f32,
    drag: f32,
}

#[derive(Component)]
pub struct ImpactVisual {
    pos: ae::Vec2,
    age: f32,
    duration: f32,
    radius: f32,
}

#[derive(Component)]
pub struct EffectVisual {
    pos: ae::Vec2,
    age: f32,
    duration: f32,
}

#[derive(Component)]
pub struct FireworkSequence {
    origin: ae::Vec2,
    age: f32,
    next_index: usize,
    schedule: Vec<FireworkBurstSpec>,
}

#[derive(Clone, Copy, Debug)]
pub struct FireworkBurstSpec {
    at: f32,
    offset: ae::Vec2,
    fx: FxId,
    scale: f32,
}

/// One floating line of speech.
///
/// This struct is only the line's clock.
#[derive(Component)]
pub struct SpeechBubbleVisual {
    pos: ae::Vec2,
    age: f32,
    duration: f32,
}

/// Marker on the shadow copies drawn behind a bubble's text. Their colour is
/// painted by the placement pass along with the line they shadow
/// (`paint_outlines`), so this exists to say which children are the shadow
/// pass rather than to drive one.
#[derive(Component)]
pub struct SpeechBubbleOutline;

const SPEECH_BUBBLE_DURATION: f32 = 2.2;
const SPEECH_BUBBLE_BASE_RISE: f32 = 14.0;
/// The alpha a bubble's text and its shadow are drawn at when the line is at
/// full strength. The placement pass multiplies the line's fade into these.
const SPEECH_BUBBLE_TEXT_ALPHA: f32 = 0.95;
const SPEECH_BUBBLE_OUTLINE_ALPHA: f32 = 0.88;

/// One ember of the live blink-destination indicator. Spawned in a small
/// rotating ring at the predicted teleport landing while the blink button is
/// held, despawned when the player releases or the blink ability is gated.
#[derive(Component)]
pub struct BlinkPreviewVisual {
    /// Phase offset around the ring, in radians. Each ember has a distinct
    /// constant so the ring keeps its shape while the ring as a whole spins.
    /// Read by `update_blink_preview` (the ring spin) in render's own builds;
    /// `allow(dead_code)` because a feature-stripped dep-build (e.g. content's
    /// `--all-features` pulling render) compiles that reader out, and the
    /// -D-warnings CI must stay clean across configs.
    #[allow(dead_code)]
    angle_offset: f32,
}

/// Fan out reusable effect requests into the visual and audio message channels.
/// Simulation code writes an [`FxRequest`] instead of remembering to pair a
/// visual `VfxMessage::Effect` with the matching packed-bank cue; headless
/// tests can still ignore the render/audio backends while gameplay stays
/// ECS-native.
pub fn process_fx_requests(
    mut requests: MessageReader<FxRequest>,
    mut vfx: MessageWriter<VfxMessage>,
    mut sfx: SfxWriter,
) {
    for request in requests.read() {
        vfx.write(VfxMessage::Effect {
            pos: request.pos,
            fx: request.fx,
            scale: request.scale,
            // the REQUESTER's pose, not identity — this is the whole route a move's committed
            // facing takes to the artwork.
            pose: request.pose,
        });
        // The override if there is one, otherwise the cue the effect's own name
        // already addresses. A caller has nothing to remember.
        //
        // `unscoped` is the default and means what it always did — the active context's primary
        // source decides — so every existing caller is byte-identical.
        if let Some(id) = request.sfx.or_else(|| effect_cue(request.fx)) {
            let play = SfxMessage::Play {
                id,
                pos: request.pos,
            };
            if request.source.is_unscoped() {
                sfx.write(play);
            } else {
                sfx.write_from(request.source.clone(), play);
            }
        }
    }
}

/// Own the temporal spread of a [`FireworksRequest`]: callers say "fireworks
/// here" and this system fans it into a short, spatially distributed sequence of
/// explosion VFX/SFX over the request's duration.
pub fn process_fireworks_requests(
    mut commands: Commands,
    mut requests: MessageReader<FireworksRequest>,
    active_session: Option<Res<ActiveSessionScope>>,
) {
    let spawn_scope = SessionSpawnScope::for_optional_active_session(active_session.as_deref());
    for request in requests.read() {
        let Some(spawn_scope) = spawn_scope else {
            continue;
        };
        let count = request.count.max(5).min(24) as usize;
        let duration = request.duration.max(0.35);
        let mut schedule = Vec::with_capacity(count);
        for i in 0..count {
            let f = if count <= 1 {
                0.0
            } else {
                i as f32 / (count - 1) as f32
            };
            let jitter_t = (((i * 17 + 11) % 9) as f32 - 4.0) * 0.018;
            let at = (0.08 + 0.84 * f) * duration + jitter_t;
            let x_jitter = ((i * 37 + 13) % 101) as f32 / 100.0 - 0.5;
            let y_jitter = ((i * 53 + 29) % 101) as f32 / 100.0;
            let wave = (f * TAU * 1.65).sin() * request.spread.x * 0.18;
            let offset = ae::Vec2::new(
                x_jitter * request.spread.x + wave,
                -request.spread.y * (0.22 + 0.78 * y_jitter),
            );
            let fx = match i % 5 {
                0 => ambition_vfx::fx::ids::STARBURST,
                1 => ambition_vfx::fx::ids::CLASSIC_BURST,
                2 => ambition_vfx::fx::ids::BURST_ROUND,
                3 => ambition_vfx::fx::ids::SHOCKWAVE,
                _ => ambition_vfx::fx::ids::SMOKE_BURST,
            };
            let scale = 0.82 + (((i * 19 + 7) % 9) as f32) * 0.055;
            schedule.push(FireworkBurstSpec {
                at: at.max(0.0),
                offset,
                fx,
                scale,
            });
        }
        schedule.sort_by(|a, b| a.at.partial_cmp(&b.at).unwrap_or(std::cmp::Ordering::Equal));
        commands.spawn_session_scoped(
            spawn_scope,
            (
                Name::new("Firework explosion sequence"),
                FireworkSequence {
                    origin: request.origin,
                    age: 0.0,
                    next_index: 0,
                    schedule,
                },
            ),
        );
    }
}

pub fn tick_firework_sequences(
    mut commands: Commands,
    presentation_time: ambition_time::PresentationTime,
    mut sequences: Query<(Entity, &mut FireworkSequence)>,
    mut effects: MessageWriter<FxRequest>,
) {
    let dt = presentation_time.wall_dt().max(0.0);
    for (entity, mut sequence) in &mut sequences {
        sequence.age += dt;
        while sequence.next_index < sequence.schedule.len()
            && sequence.schedule[sequence.next_index].at <= sequence.age
        {
            let burst = sequence.schedule[sequence.next_index];
            effects.write(
                FxRequest::new(sequence.origin + burst.offset, burst.fx).with_scale(burst.scale),
            );
            sequence.next_index += 1;
        }
        let done = sequence.next_index >= sequence.schedule.len()
            && sequence
                .schedule
                .last()
                .map(|last| sequence.age > last.at + 0.75)
                .unwrap_or(true);
        if done {
            commands.entity(entity).despawn();
        }
    }
}

/// Presentation-side subscriber. Reads `VfxMessage`s and spawns particle /
/// impact / slash entities. Skipped in headless builds.
pub fn vfx_spawn_messages(
    mut commands: Commands,
    mut messages: MessageReader<VfxMessage>,
    world: ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<
        ambition_platformer2d_core::RoomGeometry,
    >,
    assets: Option<Res<ambition_sprite_sheet::game_assets::GameAssets>>,
    active_session: Option<Res<ActiveSessionScope>>,
    // Speech bubbles quote their text with real typographic quotes, so they
    // need a real face. `None` falls back to Bevy's ASCII-only subset, which is
    // the honest outcome when a composition loads no fonts at all.
    ui_fonts: Option<Res<crate::ui_fonts::UiFonts>>,
) {
    let spawn_scope = SessionSpawnScope::for_optional_active_session(active_session.as_deref());
    let world = &world.0;
    let bubble_font = ui_fonts
        .as_deref()
        .map(|fonts| fonts.text_font(18.0, crate::ui_fonts::UiFontWeight::Regular))
        .unwrap_or_default();
    for message in messages.read() {
        match message.clone() {
            VfxMessage::Burst {
                pos,
                count,
                speed,
                color,
                kind,
            } => {
                spawn_burst(
                    &mut commands,
                    spawn_scope,
                    world,
                    pos,
                    count as usize,
                    speed,
                    color,
                    kind,
                );
            }
            VfxMessage::Dust { pos, facing } => {
                spawn_dust(&mut commands, spawn_scope, world, pos, facing)
            }
            VfxMessage::Impact { pos } => {
                spawn_hit_marker(&mut commands, spawn_scope, world, assets.as_deref(), pos)
            }
            VfxMessage::CoinPop { pos } => spawn_coin_pop(&mut commands, spawn_scope, world, pos),
            VfxMessage::Effect {
                pos,
                fx,
                scale,
                pose,
            } => {
                spawn_effect(
                    &mut commands,
                    spawn_scope,
                    world,
                    assets.as_deref(),
                    pos,
                    fx,
                    scale,
                    pose,
                );
            }
            VfxMessage::BlinkEffects {
                from,
                to,
                precision,
            } => {
                spawn_blink_effects(&mut commands, spawn_scope, world, from, to, precision);
            }
            // The melee slash effect is a sheet-driven visual handled by its own
            // self-contained system, `rendering::slash_visuals::spawn_slash_effects`
            // (co-located with the shrine visual). No-op here so this particle
            // dispatcher's match stays exhaustive.
            VfxMessage::Slash { .. } => {}
            VfxMessage::ResetEffects { from, to } => {
                spawn_reset_effects(&mut commands, spawn_scope, world, from, to);
            }
            VfxMessage::SpeechBubble { pos, text } => {
                spawn_speech_bubble(&mut commands, spawn_scope, world, pos, &text, &bubble_font);
            }
        }
    }
}

/// Can `fx` be drawn as ART right now, and from what?
///
/// The decision `spawn_effect` makes, factored out so a test can ask the engine
/// rather than re-derive it: `None` here IS the particle fallback. Two ways to
/// get it — the id names no shipped row, or the sheet holding that row is not
/// decoded in these assets.
pub fn resolve_drawable(
    assets: Option<&ambition_sprite_sheet::game_assets::GameAssets>,
    fx: FxId,
) -> Option<(
    &'static AuthoredEffect,
    &ambition_sprite_sheet::character::CharacterSpriteAsset,
    usize,
)> {
    let effect = authored_effect_for(fx)?;
    let asset = assets?.fx.get(effect.sheet)?;
    let slot = asset.spec.clip_slot([effect.name])?;
    Some((effect, asset, slot))
}

/// How big an unscaled authored effect is drawn, in world units.
///
/// the number is now the DEFAULT, not the answer. A move authors
/// `Vfx { scale }` (see `MoveEventKind::Vfx`), so a flourish asks for less and a
/// super asks for more — which is the expressive range the constant took away.
/// This value is a little under a fighter's height on purpose: an effect the
/// same size as the body reads as the body's own, which is what a move's burst
/// is.
pub const FX_DEFAULT_WORLD_SIZE: f32 = 56.0;

/// Draw the authored effect `fx`, or say why not.
///
/// Three ways this ends, and they are different facts: the id names no shipped row (a counted miss
/// — the authored id is wrong or the art was never made); the sheet holding the row is not decoded
/// (`--no-assets`, or a build whose FX manifests were not baked); or it draws.
fn spawn_effect(
    commands: &mut Commands,
    session_scope: Option<SessionSpawnScope>,
    world: &ae::World,
    assets: Option<&ambition_sprite_sheet::game_assets::GameAssets>,
    pos: ae::Vec2,
    fx: FxId,
    scale: f32,
    pose: ambition_vfx::FxPose,
) {
    let Some(session_scope) = session_scope else {
        return;
    };
    if authored_effect_for(fx).is_none() {
        note_effect_miss(fx);
    }
    let Some((effect, asset, slot)) = resolve_drawable(assets, fx) else {
        // Fallback keeps the call site useful in headless/no-asset profiles.
        spawn_burst(
            commands,
            Some(session_scope),
            world,
            pos,
            24,
            260.0,
            [0.95, 0.74, 0.28, 0.85],
            ParticleKind::Spark,
        );
        spawn_impact(commands, Some(session_scope), world, pos);
        return;
    };
    draw_effect_clip(
        commands,
        session_scope,
        world,
        pos,
        effect,
        asset,
        slot,
        scale,
        pose,
    );
}

/// The generic hit marker: what an ordinary impact looks like.
///
/// the art was already shipped and nothing asked for it. The engine's
/// own `generic_action_fx` sheet carries `hit_soft`, `hit_hard`, `hit_metal` and
/// `hit_energy`; the marker is simply a consumer that never joined — the same
/// shape as the 189-rows-on-disk / 5-reachable-from-Rust finding that
/// `ambition_sprite_sheet::fx` was built to close.
///
/// `hit_soft` for every impact, deliberately. [`ambition_vfx:ImpactMaterial`] already
/// distinguishes flesh / robot / metal, and the sheet already draws all three — but the
/// material lives on the VICTIM's `HurtFeedback` and `VfxMessage:Impact` carries a position and
/// nothing else, so joining those two vocabularies is a message change and a taste call, not
/// part of giving the marker art.
pub const GENERIC_HIT_FX: FxId = FxId::from_static("hit_soft");

/// How big a generic hit draws, as a multiple of [`FX_DEFAULT_WORLD_SIZE`].
///
/// MEASURED, not reasoned. The first attempt read the sheet's
/// `body_pixel_bbox` (48 of a 128px frame) and predicted that `0.9` would draw a
/// 19-unit spark. Photographed, its solid core came out 51 x 56 world units:
/// the bbox describes ONE rect of the opening frame, and the clip's later frames
/// fill the square. So the drawn size is the frame size, `0.9 x 56 = 50` — as
/// tall as the 46-unit fighter being hit.
///
/// `0.6` puts the spark at about 34 units, two-thirds of a fighter: unmistakable at the contact
/// point without becoming the thing you look at.
const GENERIC_HIT_FX_SCALE: f32 = 0.6;

/// Draw the shipped hit art at `pos`, or fall back to the bare marker.
///
/// the fallback is [`spawn_impact`] and NOT [`spawn_effect`]'s particle burst:
/// a composition with no decoded sheets should look exactly as it did before,
/// and a burst of 24 sparks per hit is not "exactly as before".
fn spawn_hit_marker(
    commands: &mut Commands,
    session_scope: Option<SessionSpawnScope>,
    world: &ae::World,
    assets: Option<&ambition_sprite_sheet::game_assets::GameAssets>,
    pos: ae::Vec2,
) {
    let Some(scope) = session_scope else {
        return;
    };
    let Some((effect, asset, slot)) = resolve_drawable(assets, GENERIC_HIT_FX) else {
        spawn_impact(commands, Some(scope), world, pos);
        return;
    };
    draw_effect_clip(
        commands,
        scope,
        world,
        pos,
        effect,
        asset,
        slot,
        GENERIC_HIT_FX_SCALE,
        ambition_vfx::FxPose::UPRIGHT,
    );
}

/// Spawn one resolved effect clip. The half of [`spawn_effect`] that runs
/// once the art is in hand, shared with [`spawn_hit_marker`] so the two cannot
/// drift in how an effect is sized, posed, animated or scoped.
#[allow(clippy::too_many_arguments)]
fn draw_effect_clip(
    commands: &mut Commands,
    session_scope: SessionSpawnScope,
    world: &ae::World,
    pos: ae::Vec2,
    effect: &'static AuthoredEffect,
    asset: &ambition_sprite_sheet::character::CharacterSpriteAsset,
    slot: usize,
    scale: f32,
    pose: ambition_vfx::FxPose,
) {
    let scale = scale.max(0.1);
    let render_size = BVec2::splat(FX_DEFAULT_WORLD_SIZE * scale);
    // NOT `build_character_sprite_with_render_size`: that opens on
    // `CharacterAnim::Idle`, and an effect sheet has no idle row — asking for
    // one panics. The first frame of the clip is the right opening frame anyway.
    let mut sprite = Sprite::from_atlas_image(
        asset.texture.clone(),
        bevy::image::TextureAtlas {
            layout: asset.layout.clone(),
            index: asset.spec.flat_index_at(slot, 0),
        },
    );
    sprite.custom_size = Some(render_size);
    // `FxPose:UPRIGHT` is the identity, so every emitter that never had an opinion draws
    // exactly as before.
    sprite.flip_x = pose.mirror;
    let mut animator = CharacterAnimator::new(asset);
    animator.request_clip(
        [effect.name],
        ambition_sprite_sheet::character::CharacterAnim::Idle,
    );
    commands.spawn_session_scoped(
        session_scope,
        (
            Name::new(format!("VFX effect: {}", effect.name)),
            sprite,
            Transform::from_translation(world_to_bevy(world, pos, WORLD_Z_FX + 6.0))
                .with_rotation(Quat::from_rotation_z(pose.angle)),
            animator,
            EffectVisual {
                pos,
                age: 0.0,
                // the AUTHORED length, not a magic 0.72.
                duration: effect.clip_secs().max(0.05),
            },
        ),
    );
}

impl SpeechBubbleVisual {
    fn new(pos: ae::Vec2, duration: f32) -> Self {
        Self {
            pos,
            age: 0.0,
            duration,
        }
    }
}

fn speech_bubble_progress(age: f32, duration: f32) -> f32 {
    if duration <= 0.0 {
        return 1.0;
    }
    (age / duration).clamp(0.0, 1.0)
}

/// The slow float every line does while it fades. Shared, bounded by 14, and
/// applied to the ANCHOR the line asks for — the placement pass adds whatever
/// further lift the frame turns out to need.
fn speech_bubble_rise(age: f32, duration: f32) -> f32 {
    SPEECH_BUBBLE_BASE_RISE * speech_bubble_progress(age, duration)
}

fn speech_bubble_alpha(age: f32, duration: f32) -> f32 {
    let t = speech_bubble_progress(age, duration);
    let alpha = if t < 0.75 {
        1.0
    } else {
        1.0 - (t - 0.75) / 0.25
    };
    alpha.clamp(0.0, 1.0)
}

/// Say where the line wants to be and how strongly it wants to be seen — and
/// stop there.
///
/// it writes neither the `Transform` nor the `TextColor`. The placement pass
/// is the single writer of both for every [`WorldLabel`], and two writers
/// sharing one placement is how a label drifts: a pass that reads back the
/// transform it moved last frame accumulates its own correction.
fn publish_speech_bubble_label(
    world: &ae::World,
    bubble: &SpeechBubbleVisual,
    label: &mut crate::rendering::label_layout::WorldLabel,
) {
    let rise = speech_bubble_rise(bubble.age, bubble.duration);
    label.anchor = world_to_bevy(
        world,
        bubble.pos + ae::Vec2::new(0.0, -rise),
        WORLD_Z_FX + 8.0,
    );
    label.owner_opacity = speech_bubble_alpha(bubble.age, bubble.duration);
}

pub fn update_speech_bubbles(
    mut commands: Commands,
    time: Res<Time>,
    world: ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<
        ambition_platformer2d_core::RoomGeometry,
    >,
    mut query: Query<(
        Entity,
        &mut SpeechBubbleVisual,
        &mut crate::rendering::label_layout::WorldLabel,
    )>,
) {
    let dt = time.delta_secs();
    for (entity, mut bubble, mut label) in &mut query {
        bubble.age += dt;
        if bubble.age >= bubble.duration {
            commands.entity(entity).despawn();
            continue;
        }
        publish_speech_bubble_label(&world.0, &bubble, &mut label);
    }
}

pub fn update_effects(
    mut commands: Commands,
    time: Res<Time>,
    world: ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<
        ambition_platformer2d_core::RoomGeometry,
    >,
    mut query: Query<(
        Entity,
        &mut EffectVisual,
        &mut Transform,
        &mut Sprite,
        &mut CharacterAnimator,
    )>,
) {
    let dt = time.delta_secs();
    for (entity, mut fx, mut transform, mut sprite, mut animator) in &mut query {
        fx.age += dt;
        if fx.age >= fx.duration {
            commands.entity(entity).despawn();
            continue;
        }
        let index = animator.tick(dt);
        if let Some(atlas) = sprite.texture_atlas.as_mut() {
            atlas.index = index;
        }
        let t = (fx.age / fx.duration).clamp(0.0, 1.0);
        let alpha = 1.0 - t;
        transform.translation = world_to_bevy(&world.0, fx.pos, WORLD_Z_FX + 6.0);
        sprite.color = Color::srgba(1.0, 1.0, 1.0, alpha);
    }
}

pub fn update_particles(
    mut commands: Commands,
    time: Res<Time>,
    world: ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<
        ambition_platformer2d_core::RoomGeometry,
    >,
    mut query: Query<(Entity, &mut ParticleVisual, &mut Transform, &mut Sprite)>,
) {
    let dt = time.delta_secs();
    for (entity, mut p, mut transform, mut sprite) in &mut query {
        p.age += dt;
        if p.age >= p.lifetime {
            commands.entity(entity).despawn();
            continue;
        }
        p.vel.y += p.gravity * dt;
        let drag = (1.0 - p.drag * dt).clamp(0.0, 1.0);
        p.vel *= drag;
        let velocity = p.vel;
        p.pos += velocity * dt;
        let t = (p.age / p.lifetime).clamp(0.0, 1.0);
        let alpha = p.rgba[3] * (1.0 - t);
        let size = match p.kind {
            ParticleKind::Spark => p.radius * (1.0 - 0.35 * t),
            ParticleKind::Dust => p.radius * (1.0 + 0.70 * t),
            ParticleKind::Shard => p.radius * (1.0 - 0.15 * t),
        };
        transform.translation = world_to_bevy(&world.0, p.pos, WORLD_Z_FX);
        sprite.custom_size = Some(BVec2::splat(size.max(0.5)));
        sprite.color = rgba(p.rgba[0], p.rgba[1], p.rgba[2], alpha);
    }
}

pub fn update_impacts(
    mut commands: Commands,
    time: Res<Time>,
    world: ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<
        ambition_platformer2d_core::RoomGeometry,
    >,
    mut query: Query<(Entity, &mut ImpactVisual, &mut Transform, &mut Sprite)>,
) {
    let dt = time.delta_secs();
    for (entity, mut fx, mut transform, mut sprite) in &mut query {
        fx.age += dt;
        if fx.age >= fx.duration {
            commands.entity(entity).despawn();
            continue;
        }
        let t = (fx.age / fx.duration).clamp(0.0, 1.0);
        let radius = fx.radius + 46.0 * t;
        let alpha = 0.82 * (1.0 - t);
        transform.translation = world_to_bevy(&world.0, fx.pos, WORLD_Z_FX + 1.0);
        sprite.custom_size = Some(BVec2::splat(radius));
        sprite.color = Color::srgba(1.0, 1.0, 0.35, alpha);
    }
}

/// How wide a bark may be, in world units.
const SPEECH_BUBBLE_MAX_WIDTH: f32 = 120.0;

pub fn spawn_speech_bubble(
    commands: &mut Commands,
    session_scope: Option<SessionSpawnScope>,
    world: &ae::World,
    // The speaker's head, in world space. Where the line is DRAWN is not
    // decided here and cannot be: the placement pass separates it from every
    // other world label on the next pass over the frame.
    pos: ae::Vec2,
    text: &str,
    // The face the bubble draws in. Threaded rather than repaired a frame later
    // (the way `label_layout` patches world labels) because a bubble lives about
    // a second and fades the whole time — one frame in the wrong font is a
    // meaningful fraction of the thing.
    font: &TextFont,
) {
    // this line is why the font MATTERS here more than anywhere else: the bubble supplies its
    // own non-ASCII. Left at `TextFont::default()` the curly quotes resolved Bevy's built-in
    // `FiraMono-subset.ttf` — the same handle the menu tofu came down to. See
    // `ambition_menu::render::bevy_ui::MenuFont`.
    let bubble_text = format!("\u{201c}{text}\u{201d}");
    let Some(session_scope) = session_scope else {
        return;
    };
    use crate::rendering::label_layout::{WorldLabel, WorldLabelFamily};

    let visual = SpeechBubbleVisual::new(pos, SPEECH_BUBBLE_DURATION);
    let text_color = Color::srgba(1.0, 1.0, 1.0, SPEECH_BUBBLE_TEXT_ALPHA);
    let outline_color = Color::srgba(0.0, 0.0, 0.0, SPEECH_BUBBLE_OUTLINE_ALPHA);
    let mut bubble = commands.spawn_session_scoped(
        session_scope,
        (
            Text2d::new(bubble_text.clone()),
            TextFont {
                font_size: FontSize::Px(18.0),
                ..font.clone()
            },
            // width only. `TextBounds`' own doc says characters outside
            // the bounds after wrapping are TRUNCATED, so a height bound would
            // silently eat the end of a long bark — the one thing worse than a
            // wide one.
            TextBounds {
                width: Some(SPEECH_BUBBLE_MAX_WIDTH),
                height: None,
            },
            // Wrapped lines centre under each other, so the bubble stays a
            // block over its speaker rather than a left-aligned ladder.
            TextLayout::justify(Justify::Center),
            TextColor(text_color),
            Name::new(format!("Speech bubble: {text}")),
        ),
    );
    // Two speakers can say the same words from the same spot; only the entity is theirs alone.
    let owner_id = format!("speech:{}", bubble.id().index());
    let mut label = WorldLabel::new(owner_id, WorldLabelFamily::Speech, Vec3::ZERO)
        .with_colors(text_color, Some(outline_color));
    publish_speech_bubble_label(world, &visual, &mut label);
    bubble
        .insert((
            Transform::from_translation(label.anchor),
            Visibility::Visible,
            label,
            visual,
        ))
        .with_children(|parent| {
            for offset in [
                BVec2::new(-1.25, 0.0),
                BVec2::new(1.25, 0.0),
                BVec2::new(0.0, -1.25),
                BVec2::new(0.0, 1.25),
            ] {
                parent.spawn((
                    Text2d::new(bubble_text.clone()),
                    TextFont {
                        font_size: FontSize::Px(18.0),
                        ..font.clone()
                    },
                    // the shadow must wrap EXACTLY as the line it shadows;
                    // a different bound here is four ghosts at four offsets.
                    TextBounds {
                        width: Some(SPEECH_BUBBLE_MAX_WIDTH),
                        height: None,
                    },
                    TextLayout::justify(Justify::Center),
                    // Painted every frame by the placement pass along with the
                    // line it shadows; this is only the first frame's value.
                    TextColor(outline_color),
                    Transform::from_xyz(offset.x, offset.y, -0.1),
                    SpeechBubbleOutline,
                    Name::new("Speech bubble outline"),
                ));
            }
        });
}

pub fn spawn_impact(
    commands: &mut Commands,
    session_scope: Option<SessionSpawnScope>,
    world: &ae::World,
    pos: ae::Vec2,
) {
    let Some(session_scope) = session_scope else {
        return;
    };
    commands.spawn_session_scoped(
        session_scope,
        (
            Sprite::from_color(Color::srgba(1.0, 1.0, 0.35, 0.82), BVec2::splat(12.0)),
            Transform::from_translation(world_to_bevy(world, pos, WORLD_Z_FX + 1.0)),
            ImpactVisual {
                pos,
                age: 0.0,
                duration: 0.24,
                radius: 12.0,
            },
        ),
    );
}

pub fn spawn_reset_effects(
    commands: &mut Commands,
    session_scope: Option<SessionSpawnScope>,
    world: &ae::World,
    from: ae::Vec2,
    to: ae::Vec2,
) {
    // Reset is a teleport-like state transition.
    if (from - to).length() > 8.0 {
        spawn_burst(
            commands,
            session_scope,
            world,
            from,
            10,
            180.0,
            [0.32, 0.48, 0.70, 0.52],
            ParticleKind::Dust,
        );
    }
    spawn_burst(
        commands,
        session_scope,
        world,
        to,
        24,
        280.0,
        [0.55, 0.85, 1.0, 0.90],
        ParticleKind::Spark,
    );
    spawn_impact(commands, session_scope, world, to);
}

/// The shortest a burst particle lives, and how much longer the longest-lived
/// one gets. `spawn_burst` spreads every particle across this window.
const BURST_MIN_LIFETIME: f32 = 0.22;
const BURST_LIFETIME_SPREAD: f32 = 0.16;

/// Velocity damping per second, by particle flavour.
const fn particle_drag(kind: ParticleKind) -> f32 {
    match kind {
        ParticleKind::Spark => 3.4,
        ParticleKind::Dust => 4.7,
        ParticleKind::Shard => 1.8,
    }
}

/// Downward acceleration, by particle flavour.
const fn particle_gravity(kind: ParticleKind) -> f32 {
    match kind {
        ParticleKind::Spark => 300.0,
        ParticleKind::Dust => 120.0,
        ParticleKind::Shard => 650.0,
    }
}

/// How far the FURTHEST particle of a burst travels, in world units.
///
/// [`update_particles`] damps velocity by [`particle_drag`] every frame, which
/// integrates to `v0/drag * (1 - e^(-drag * t))`; the furthest particle leaves
/// at the full `speed` and lives [`BURST_MIN_LIFETIME`] +
/// [`BURST_LIFETIME_SPREAD`].
///
/// ⭐ Exposed because a caller that has to keep a burst ON SCREEN needs the
/// burst's ENVELOPE, not its centre, and a caller that re-derives the envelope
/// beside the spawner drifts from it the first time a drag constant moves.
pub fn burst_reach(speed: f32, kind: ParticleKind) -> f32 {
    let drag = particle_drag(kind);
    let lifetime = BURST_MIN_LIFETIME + BURST_LIFETIME_SPREAD;
    speed / drag * (1.0 - (-drag * lifetime).exp())
}

pub fn spawn_burst(
    commands: &mut Commands,
    session_scope: Option<SessionSpawnScope>,
    world: &ae::World,
    pos: ae::Vec2,
    count: usize,
    speed: f32,
    color_rgba: [f32; 4],
    kind: ParticleKind,
) {
    let Some(session_scope) = session_scope else {
        return;
    };
    // TODO(quality): thread `ResolvedVisualQuality.budget.particles` into the
    // central VFX spawn API, then clamp `count` and spawn-rate here instead of
    // letting individual gameplay emitters interpret quality profiles.
    let count = count.max(1);
    for i in 0..count {
        let t = i as f32 / count as f32;
        let wobble = ((i * 37 + 17) as f32).sin() * 0.22;
        let angle = TAU * t + wobble;
        let strength = speed * (0.45 + 0.55 * ((i * 13 + 5) % 11) as f32 / 10.0);
        let vel = ae::Vec2::new(angle.cos() * strength, angle.sin() * strength);
        let radius = 2.0 + 2.5 * ((i * 5 + 1) % 7) as f32 / 6.0;
        let lifetime = BURST_MIN_LIFETIME + BURST_LIFETIME_SPREAD * ((i * 7 + 3) % 9) as f32 / 8.0;
        commands.spawn_session_scoped(
            session_scope,
            (
                Sprite::from_color(
                    rgba(color_rgba[0], color_rgba[1], color_rgba[2], color_rgba[3]),
                    BVec2::splat(radius),
                ),
                Transform::from_translation(world_to_bevy(world, pos, WORLD_Z_FX)),
                ParticleVisual {
                    kind,
                    pos,
                    vel,
                    age: 0.0,
                    lifetime,
                    radius,
                    rgba: color_rgba,
                    gravity: particle_gravity(kind),
                    drag: particle_drag(kind),
                },
            ),
        );
    }
}

/// One coin, up and back down — the acknowledgement a struck coin block owes.
///
/// A single ballistic particle rather than a burst: the coin leaves straight up, gravity brings
/// it back, and it is gone inside a third of a second.
///
/// the four numbers are the whole feel and they are together on purpose.
/// Rise, gravity, size and colour are the dials worth turning; everything else
/// about the effect follows from them.
pub fn spawn_coin_pop(
    commands: &mut Commands,
    session_scope: Option<SessionSpawnScope>,
    world: &ae::World,
    pos: ae::Vec2,
) {
    const RISE_SPEED: f32 = 260.0;
    const FALL: f32 = 900.0;
    const RADIUS: f32 = 5.0;
    const GOLD: [f32; 4] = [1.0, 0.84, 0.22, 1.0];

    let Some(session_scope) = session_scope else {
        return;
    };
    commands.spawn_session_scoped(
        session_scope,
        (
            Sprite::from_color(
                rgba(GOLD[0], GOLD[1], GOLD[2], GOLD[3]),
                BVec2::splat(RADIUS * 2.0),
            ),
            Transform::from_translation(world_to_bevy(world, pos, WORLD_Z_FX)),
            ParticleVisual {
                kind: ParticleKind::Shard,
                pos,
                // NEGATIVE y is up: world y is down-positive here.
                vel: ae::Vec2::new(0.0, -RISE_SPEED),
                age: 0.0,
                // Long enough to rise and fall back past where it started.
                lifetime: 2.0 * RISE_SPEED / FALL,
                radius: RADIUS,
                rgba: GOLD,
                gravity: FALL,
                // No drag: a coin arcs, it does not drift to a halt in the air.
                drag: 0.0,
            },
        ),
    );
}

pub fn spawn_dust(
    commands: &mut Commands,
    session_scope: Option<SessionSpawnScope>,
    world: &ae::World,
    pos: ae::Vec2,
    facing: f32,
) {
    let Some(session_scope) = session_scope else {
        return;
    };
    for i in 0..6 {
        let lateral = -facing * (75.0 + i as f32 * 18.0);
        let upward = -35.0 - i as f32 * 8.0;
        let radius = 3.5 + i as f32 * 0.35;
        commands.spawn_session_scoped(
            session_scope,
            (
                Sprite::from_color(Color::srgba(0.58, 0.62, 0.72, 0.75), BVec2::splat(radius)),
                Transform::from_translation(world_to_bevy(world, pos, WORLD_Z_FX)),
                ParticleVisual {
                    kind: ParticleKind::Dust,
                    pos,
                    vel: ae::Vec2::new(lateral, upward),
                    age: 0.0,
                    lifetime: 0.28 + 0.03 * i as f32,
                    radius,
                    rgba: [0.58, 0.62, 0.72, 0.75],
                    gravity: 80.0,
                    drag: 4.4,
                },
            ),
        );
    }
}

pub fn spawn_blink_effects(
    commands: &mut Commands,
    session_scope: Option<SessionSpawnScope>,
    world: &ae::World,
    from: ae::Vec2,
    to: ae::Vec2,
    precision: bool,
) {
    let exit_color = if precision {
        [0.40, 0.34, 1.00, 0.78]
    } else {
        [0.24, 0.74, 1.00, 0.68]
    };
    let entry_color = if precision {
        [0.92, 0.42, 1.00, 0.92]
    } else {
        [0.42, 1.00, 0.92, 0.90]
    };
    spawn_burst(
        commands,
        session_scope,
        world,
        from,
        if precision { 18 } else { 12 },
        250.0,
        exit_color,
        ParticleKind::Spark,
    );
    spawn_burst(
        commands,
        session_scope,
        world,
        to,
        if precision { 28 } else { 18 },
        360.0,
        entry_color,
        ParticleKind::Spark,
    );
    spawn_impact(commands, session_scope, world, to);
}

/// Live ring of orbiting embers showing where the next blink will land.
///
/// This system only draws the ember ring.
#[cfg(feature = "input")]
pub fn update_blink_preview(
    mut commands: Commands,
    time: Res<Time>,
    world: ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<
        ambition_platformer2d_core::RoomGeometry,
    >,
    fact: Res<ambition_sim_view::BlinkPreviewFact>,
    active_session: Option<Res<ActiveSessionScope>>,
    mut existing: Query<(Entity, &BlinkPreviewVisual, &mut Transform, &mut Sprite)>,
) {
    let spawn_scope = SessionSpawnScope::for_optional_active_session(active_session.as_deref());
    if !fact.active || spawn_scope.is_none() {
        for (entity, _, _, _) in &existing {
            commands.entity(entity).despawn();
        }
        return;
    }
    let session_scope = spawn_scope.expect("active preview requires a spawn scope");
    let target = fact.target;
    let precision = fact.precision;
    // Match the post-blink burst palette so the preview reads as
    // "this is what's about to happen here".
    let color = if precision {
        rgba(0.92, 0.42, 1.00, 0.85)
    } else {
        rgba(0.42, 1.00, 0.92, 0.80)
    };

    const RING_EMBERS: usize = 4;
    let radius = fact.body_min_extent * 0.45;
    let spin = time.elapsed_secs() * 2.4;
    let pulse = 1.0 + 0.18 * (time.elapsed_secs() * 5.5).sin();
    let ember_size = (fact.body_min_extent * 0.18) * pulse;

    let mut emitted = 0;
    for (_, ember, mut transform, mut sprite) in &mut existing {
        let angle = spin + ember.angle_offset;
        let offset = ae::Vec2::new(angle.cos(), angle.sin()) * radius;
        transform.translation = world_to_bevy(&world.0, target + offset, WORLD_Z_FX + 1.5);
        sprite.custom_size = Some(BVec2::splat(ember_size.max(1.0)));
        sprite.color = color;
        emitted += 1;
    }

    if emitted == 0 {
        for i in 0..RING_EMBERS {
            let angle_offset = TAU * (i as f32) / RING_EMBERS as f32;
            let angle = spin + angle_offset;
            let offset = ae::Vec2::new(angle.cos(), angle.sin()) * radius;
            commands.spawn_session_scoped(
                session_scope,
                (
                    Sprite::from_color(color, BVec2::splat(ember_size.max(1.0))),
                    Transform::from_translation(world_to_bevy(
                        &world.0,
                        target + offset,
                        WORLD_Z_FX + 1.5,
                    )),
                    BlinkPreviewVisual { angle_offset },
                ),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The generic hit marker names a row the art actually ships.
    ///
    /// The only thing that would notice is somebody photographing a match, which is how it was
    /// found the first time.
    ///
    /// the id is a one-way hash, so this asks the index rather than
    /// comparing strings: `authored_effect_for` answers only for a name the
    /// shipped sheets carry.
    #[test]
    fn the_generic_hit_marker_names_a_shipped_row() {
        let effect = authored_effect_for(GENERIC_HIT_FX).expect(
            "`hit_soft` is not a row on any shipped FX sheet, so every impact in \
             the game draws an untextured rectangle again",
        );
        assert_eq!(effect.name, "hit_soft");
        assert_eq!(effect.sheet, "generic_action_fx");
    }

    /// Every shipped effect row is addressable by its hashed name, and the
    /// sound comes with it.
    ///
    /// The index is the whole vocabulary now, so its size is the number of rows
    /// the art actually ships — not a number anyone typed. Asserting both
    /// directions on a sample keeps the *pairing* honest: the same string that
    /// finds the clip finds the cue.
    #[test]
    fn an_effect_id_resolves_to_its_row_and_its_cue() {
        assert_eq!(
            effect_index().by_id.len(),
            ambition_sprite_sheet::fx::authored_effects().len(),
            "hashing the names must not collide two rows into one id"
        );

        for (name, sheet, cue) in [
            (
                "classic_burst",
                "generic_explosions",
                "vfx.explosion.classic_burst",
            ),
            (
                "sonic_boom",
                "generic_exotic_fx",
                "vfx.generic_exotic.sonic_boom",
            ),
            (
                "reductio_impact",
                "george_booul_vfx",
                "vfx.george_booul.reductio_impact",
            ),
        ] {
            let fx = FxId::new(name);
            let effect = authored_effect_for(fx).unwrap_or_else(|| panic!("`{name}` ships"));
            assert_eq!(effect.sheet, sheet);
            assert_eq!(effect_cue(fx), Some(SfxId::from_static(cue)));
        }
    }

    /// The old five are ordinary rows now.
    ///
    /// `ExplosionKind`'s variants were the five rows of one sheet, reached
    /// through three tables. They resolve through exactly the same path as the
    /// other 184 — which is the claim the deletion rests on, so it is worth
    /// saying out loud rather than inferring from the absence of the enum.
    #[test]
    fn the_five_former_enum_variants_take_the_same_path_as_every_other_row() {
        use ambition_vfx::fx::ids;
        for (fx, name) in [
            (ids::CLASSIC_BURST, "classic_burst"),
            (ids::BURST_ROUND, "burst_round"),
            (ids::SHOCKWAVE, "shockwave"),
            (ids::SMOKE_BURST, "smoke_burst"),
            (ids::STARBURST, "starburst"),
        ] {
            let effect = authored_effect_for(fx).expect("a row of generic_explosions");
            assert_eq!(effect.name, name);
            assert_eq!(effect.sheet, "generic_explosions");
        }
        // and one from outside it, resolved by the same call — the property
        // the enum made impossible.
        assert_eq!(
            authored_effect_for(ids::SONIC_BOOM).map(|e| e.sheet),
            Some("generic_exotic_fx"),
        );
    }

    /// An id no sheet carries resolves to nothing rather than to row 0 of
    /// something — the `unwrap_or(0)` habit `first_bound_row` exists to refuse.
    #[test]
    fn an_unknown_effect_resolves_to_nothing_not_to_row_zero() {
        assert!(authored_effect_for(FxId::new("kaboom")).is_none());
        assert!(effect_cue(FxId::new("kaboom")).is_none());
    }
    /// A request's SOURCE survives the fan-out.
    ///
    /// `dispatch_move_events` scopes its `Sfx` arm by the event's presentation source, and its
    /// `Vfx` arm writes a bare `VfxMessage::Effect` — going around the pairing.
    ///
    /// both arms asserted: the unscoped default must stay on the plain write,
    /// or every existing caller changes behaviour to buy this.
    #[test]
    fn a_requests_presentation_source_reaches_the_cue_it_pairs() {
        use ambition_sfx::{OwnedSfxMessage, PresentationSourceId};
        use bevy::prelude::*;

        fn run(source: PresentationSourceId) -> Vec<OwnedSfxMessage> {
            let mut app = App::new();
            app.add_message::<ambition_vfx::VfxMessage>();
            app.add_message::<OwnedSfxMessage>();
            app.add_message::<FxRequest>();
            app.add_systems(Update, process_fx_requests);
            app.world_mut().write_message(
                FxRequest::new(ae::Vec2::ZERO, ambition_vfx::fx::ids::CLASSIC_BURST)
                    .from_source(source),
            );
            app.update();
            let messages = app.world().resource::<Messages<OwnedSfxMessage>>();
            let mut cursor = messages.get_cursor();
            cursor.read(messages).cloned().collect()
        }

        let seat = run(PresentationSourceId::from("seat_two"));
        assert_eq!(seat.len(), 1, "the paired cue did not reach the channel");
        assert_eq!(
            seat[0].source.as_str(),
            "seat_two",
            "a request attributed to a seat played as somebody else"
        );

        let anyone = run(PresentationSourceId::unscoped());
        assert_eq!(anyone.len(), 1, "the paired cue did not reach the channel");
        assert!(
            anyone[0].source.is_unscoped(),
            "the unscoped default acquired a source, so every existing caller \
             changed behaviour to buy the scoping"
        );
    }

    // ── Speech-bubble placement (, then ) ────────────────────────────
    //
    // The column is gone: a bubble is a `WorldLabel` and the shared ranked pass separates it from
    // every other world label, its own family included.

    use crate::rendering::label_layout::{
        label_size, LabelBox, WorldLabel, WorldLabelFamily, WorldLabelLayoutPlugin,
        WorldLabelLayoutSet, WorldLabelLayoutSettings,
    };

    fn stage() -> ambition_platformer2d_core::World {
        ambition_platformer2d_core::movement::containment::walled_box(
            ae::Vec2::new(1600.0, 900.0),
            16.0,
        )
    }

    /// The composed frame these guards read: the shared placement pass, the
    /// bubble clock and the message subscriber, wired the way
    /// `HostVfxPresentationPlugin` wires them.
    struct SpeechFrame {
        app: App,
        settings: WorldLabelLayoutSettings,
    }

    impl SpeechFrame {
        fn new() -> Self {
            use ambition_platformer2d_shared_tangle::lifecycle::{SessionRoot, SessionScopeId};

            let mut app = App::new();
            app.init_resource::<Time>();
            app.add_message::<ambition_vfx::VfxMessage>();
            app.add_plugins(WorldLabelLayoutPlugin);
            app.add_systems(
                Update,
                (vfx_spawn_messages, update_speech_bubbles)
                    .chain()
                    .before(WorldLabelLayoutSet),
            );
            app.world_mut().spawn((
                SessionRoot(SessionScopeId(0)),
                ambition_platformer2d_core::RoomGeometry(stage()),
            ));
            // The pass ranks per VIEW, so a composition without one places
            // nothing at all — a fixture that forgot this would pass by
            // drawing nobody.
            app.world_mut().spawn((
                ambition_sim_view::LocalView,
                ambition_sim_view::LocalViewId::FIRST,
                ambition_sim_view::CameraViewState::default(),
            ));
            Self {
                app,
                settings: WorldLabelLayoutSettings::default(),
            }
        }

        fn say(&mut self, pos: ae::Vec2, text: &str) {
            self.app
                .world_mut()
                .write_message(ambition_vfx::VfxMessage::SpeechBubble {
                    pos,
                    text: text.into(),
                });
        }

        /// A name plate at a speaker's head, published exactly the way
        /// `sync_actor_nameplates` publishes one.
        fn plate(&mut self, name: &str, pos: ae::Vec2) {
            let anchor = world_to_bevy(&stage(), pos, WORLD_Z_FX + 8.0);
            self.app.world_mut().spawn((
                Text2d::new(name.to_string()),
                TextFont {
                    font_size: FontSize::Px(18.0),
                    ..default()
                },
                TextColor(Color::WHITE),
                Transform::from_translation(anchor),
                Visibility::Visible,
                WorldLabel::new(name, WorldLabelFamily::Actor, anchor),
            ));
        }

        /// `app.update()` is NOT a tick of sim time, so the clock is
        /// advanced explicitly. Ageing is what makes "the older line" mean
        /// anything, and it is what moves a line's anchor as it floats.
        fn tick(&mut self, secs: f32) {
            self.app
                .world_mut()
                .resource_mut::<Time>()
                .advance_by(std::time::Duration::from_secs_f32(secs));
            self.app.update();
        }

        fn drawn(&mut self) -> Vec<(String, LabelBox)> {
            let settings = self.settings.clone();
            self.app
                .world_mut()
                .query_filtered::<(&Text2d, &TextFont, &Transform, &Visibility), With<WorldLabel>>()
                .iter(self.app.world())
                .filter(|(_, _, _, visibility)| **visibility != Visibility::Hidden)
                .map(|(text, font, transform, _)| {
                    (
                        text.as_str().to_string(),
                        LabelBox {
                            center: transform.translation.truncate(),
                            half: label_size(None, text.as_str(), font.font_size, &settings) * 0.5,
                        },
                    )
                })
                .collect()
        }

        /// The drawn speech lines only.
        fn drawn_lines(&mut self) -> Vec<(String, LabelBox)> {
            self.drawn()
                .into_iter()
                .filter(|(text, _)| text.starts_with('\u{201c}'))
                .collect()
        }
    }

    /// No two drawn labels occupy the same pixels — the property, stated where
    /// the reader looks.
    #[track_caller]
    fn assert_no_overlap(drawn: &[(String, LabelBox)]) {
        for (i, (a_text, a)) in drawn.iter().enumerate() {
            for (b_text, b) in drawn.iter().skip(i + 1) {
                assert!(
                    !a.overlaps(b, 0.0),
                    "{a_text:?} and {b_text:?} were drawn through each other: \
                     {a:?} and {b:?}"
                );
            }
        }
    }

    const TAUNT: &str = "Either you are on the stage or you are not.";
    const BELAY: &str = "Belay that, ye barnacle!";

    /// Two lines from two speakers never print through each other.
    ///
    /// The pass has no per-speaker offset to be right about: it compares the BOXES the lines land
    /// in, in one space.
    #[test]
    fn speakers_at_different_heights_do_not_print_through_each_other() {
        let floor = |x: f32| ae::Vec2::new(x, 225.44);
        let airborne = |x: f32| ae::Vec2::new(x, 208.84);
        let higher = |x: f32| ae::Vec2::new(x, 196.62);

        let pairs = [
            [floor(150.7), airborne(180.6)],
            [airborne(180.6), floor(150.7)],
            [floor(150.7), higher(160.0)],
            [higher(160.0), floor(150.7)],
        ];
        for pair in pairs {
            let mut frame = SpeechFrame::new();
            frame.say(pair[0], TAUNT);
            frame.tick(1.0 / 60.0);
            frame.say(pair[1], BELAY);
            frame.tick(1.0 / 60.0);
            let lines = frame.drawn_lines();
            assert_eq!(lines.len(), 2, "a line went missing: {lines:?}");
            assert_no_overlap(&lines);
        }

        // The photographed frame: George from the floor twice, then the pirate
        // from the air — and the same three arriving as ONE burst, which is how
        // two CPUs taunting on the same tick reach the renderer.
        for burst in [false, true] {
            let mut frame = SpeechFrame::new();
            let arrivals = [
                (floor(150.7), TAUNT),
                (floor(158.3), BELAY),
                (airborne(180.6), "Arr."),
            ];
            for (pos, text) in arrivals {
                frame.say(pos, text);
                if !burst {
                    frame.tick(1.0 / 60.0);
                }
            }
            frame.tick(1.0 / 60.0);
            let lines = frame.drawn_lines();
            assert_eq!(lines.len(), 3, "a line went missing: {lines:?}");
            assert_no_overlap(&lines);
        }
    }

    /// A line already on screen and a line born this frame are placed together.
    ///
    /// The behavioural half. What the two deleted make-room routines could not
    /// give was that each swept its own population, so a bubble cleared every
    /// member of the other list and could still land on one of them. The pass
    /// has one population by construction — it iterates the labels that exist.
    #[test]
    fn a_live_line_and_a_line_born_this_frame_are_placed_together() {
        let mut frame = SpeechFrame::new();
        frame.say(ae::Vec2::new(150.7, 225.44), TAUNT);
        frame.tick(1.0 / 60.0);
        // A beat passes, so the second line meets a LIVE entity rather than a
        // queued neighbour, which is the whole point.
        frame.tick(0.2);
        frame.say(ae::Vec2::new(180.6, 196.62), BELAY);
        frame.tick(1.0 / 60.0);

        let lines = frame.drawn_lines();
        assert_eq!(lines.len(), 2, "both taunts should be on screen: {lines:?}");
        assert_no_overlap(&lines);
    }

    /// Four fighters all get a line, and a fifth speaker never costs legibility.
    ///
    /// A four-fighter free-for-all is the widest supported match, so four lines
    /// at four heights is the load the placement budget is sized for. The
    /// fifth is the honest limit: a line the pass cannot place is HIDDEN, never
    /// drawn onto its neighbour.
    #[test]
    fn a_four_fighter_free_for_all_fits_and_a_fifth_never_prints_through() {
        // Four fighters at four heights — the same assorted anchors measured, which is where
        // its per-speaker offsets cancelled.
        let assorted = [
            (ae::Vec2::new(131.1, 225.44), TAUNT),
            (ae::Vec2::new(150.7, 208.84), BELAY),
            (ae::Vec2::new(180.6, 196.62), "Arr."),
            (ae::Vec2::new(196.0, 180.0), "Get off the stage."),
        ];
        let mut frame = SpeechFrame::new();
        for (pos, text) in assorted {
            frame.say(pos, text);
            frame.tick(1.0 / 60.0);
        }
        let lines = frame.drawn_lines();
        assert_eq!(
            lines.len(),
            4,
            "a four-fighter free-for-all lost a line: {lines:?}"
        );
        assert_no_overlap(&lines);

        // A fifth speaker: whatever the budget allows is drawn, and nothing
        // that is drawn is drawn through anything else.
        frame.say(ae::Vec2::new(210.0, 200.0), "Five of us?");
        frame.tick(1.0 / 60.0);
        let lines = frame.drawn_lines();
        assert!(
            lines.len() >= 4,
            "the fifth speaker cost more than one line: {lines:?}"
        );
        assert_no_overlap(&lines);
    }

    /// A name plate and a speech bubble at one anchor do not print through each other.
    #[test]
    fn a_name_plate_and_a_speech_bubble_do_not_print_through_each_other() {
        let speaker = ae::Vec2::new(150.7, 225.44);
        let mut frame = SpeechFrame::new();
        frame.plate("George Booul", speaker);
        frame.say(speaker, TAUNT);
        frame.tick(1.0 / 60.0);

        let drawn = frame.drawn();
        assert_eq!(
            drawn.len(),
            2,
            "the fixture must draw both the plate and the taunt: {drawn:?}"
        );
        assert_no_overlap(&drawn);
    }

    /// The plate holds its ground and the LINE moves.
    ///
    /// The ranking argument, asserted rather than described: a plate is
    /// permanent furniture on a body the eye is tracking, so displacing it
    /// would make it hop once per taunt; a bubble is born rising and gone in
    /// two seconds. `WorldLabelFamily`'s declaration order says so and this is
    /// what says it is still true.
    #[test]
    fn the_bubble_yields_to_the_name_plate_and_not_the_other_way_round() {
        let speaker = ae::Vec2::new(150.7, 225.44);
        let plate_anchor = world_to_bevy(&stage(), speaker, WORLD_Z_FX + 8.0);

        let mut frame = SpeechFrame::new();
        frame.plate("George Booul", speaker);
        frame.say(speaker, TAUNT);
        frame.tick(1.0 / 60.0);

        let drawn = frame.drawn();
        let plate = drawn
            .iter()
            .find(|(text, _)| text == "George Booul")
            .expect("the plate must be drawn");
        assert_eq!(
            plate.1.center,
            plate_anchor.truncate(),
            "the name plate was displaced; it is the family that must NOT move"
        );
        let line = drawn
            .iter()
            .find(|(text, _)| text.starts_with('\u{201c}'))
            .expect("the taunt must be drawn");
        assert_ne!(
            line.1.center,
            plate_anchor.truncate(),
            "the taunt stayed on its raw anchor, so nothing placed it"
        );
    }
}
