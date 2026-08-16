//! Procedural visual effects for the sandbox.
//!
//! Particles are CPU-side Bevy sprite entities for now. Keeping this behind a
//! compact module gives us a later migration seam to GPU particles or Hanabi.

use ambition_platformer2d_core as ae;
use bevy::math::Vec2 as BVec2;
use bevy::prelude::*;
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

/// **What an [`FxId`] names**: the authored row, the sheet holding it, and the
/// packed cue that ships with it.
///
/// ⭐⭐ **this index IS the engine's effect vocabulary, and nothing declares it.**
/// It is built by walking the FX sheets' own baked records and hashing each row
/// name — so the set of drawable effects is the set of shipped rows, by
/// construction. That is what replaced `move_vfx_kind` (name→enum),
/// `explosion_anim` (enum→pose) and `explosion_sfx` (enum→cue): three tables
/// whose whole job was getting back to the string the content already had.
///
/// ⚠ `SfxId` is precomputed here rather than at every spawn: the cue name is
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

/// **The sound `fx` makes.** A property of the NAME, not of the call site: the
/// bank ships one `vfx.<family>.<row>` cue for every authored row, so an
/// emitter that says which effect has already said which sound.
pub fn effect_cue(fx: FxId) -> Option<SfxId> {
    effect_index().by_id.get(&fx).map(|(_, cue)| *cue)
}

/// **Say once, per id, that an effect named nothing.**
///
/// SFX's policy, for SFX's reason: the vocabulary is open (a game may author
/// effects the engine never heard of), so a miss is a report rather than a
/// refusal — but a per-frame warning for a move that fires sixty times a second
/// trains everyone to filter the channel. ⚠ the id is a one-way hash and the
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
            target: "ambition_render::fx",
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

#[derive(Component)]
pub struct SpeechBubbleVisual {
    pos: ae::Vec2,
    age: f32,
    duration: f32,
    stack_offset: f32,
    target_stack_offset: f32,
}

#[derive(Component)]
pub struct SpeechBubbleOutline;

struct PendingSpeechBubble {
    pos: ae::Vec2,
    text: String,
    age: f32,
    stack_offset: f32,
    target_stack_offset: f32,
}

const SPEECH_BUBBLE_DURATION: f32 = 2.2;
const SPEECH_BUBBLE_BASE_RISE: f32 = 14.0;
const SPEECH_BUBBLE_STACK_STEP: f32 = 28.0;
const SPEECH_BUBBLE_STACK_MAX: f32 = 84.0;
const SPEECH_BUBBLE_STACK_INITIAL_NUDGE: f32 = 12.0;
const SPEECH_BUBBLE_STACK_SPEED: f32 = 80.0;
const SPEECH_BUBBLE_STACK_X_RANGE: f32 = 160.0;
const SPEECH_BUBBLE_STACK_Y_RANGE: f32 = 96.0;
const SPEECH_BUBBLE_PUSH_FADE_AFTER: f32 = 0.85;

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
        });
        // The override if there is one, otherwise the cue the effect's own name
        // already addresses. A caller has nothing to remember.
        if let Some(id) = request.sfx.or_else(|| effect_cue(request.fx)) {
            sfx.write(SfxMessage::Play {
                id,
                pos: request.pos,
            });
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
    mut speech_bubbles: Query<(&mut SpeechBubbleVisual, &mut Transform, &mut TextColor)>,
    // Speech bubbles quote their text with real typographic quotes, so they
    // need a real face. `None` falls back to Bevy's ASCII-only subset, which is
    // the honest outcome when a composition loads no fonts at all.
    ui_fonts: Option<Res<crate::ui_fonts::UiFonts>>,
) {
    let spawn_scope = SessionSpawnScope::for_optional_active_session(active_session.as_deref());
    let world = &world.0;
    let mut pending_speech_bubbles = Vec::new();
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
            VfxMessage::Impact { pos } => spawn_impact(&mut commands, spawn_scope, world, pos),
            VfxMessage::CoinPop { pos } => spawn_coin_pop(&mut commands, spawn_scope, world, pos),
            VfxMessage::Effect { pos, fx, scale } => {
                spawn_effect(
                    &mut commands,
                    spawn_scope,
                    world,
                    assets.as_deref(),
                    pos,
                    fx,
                    scale,
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
                make_room_for_speech_bubble(pos, world, &mut speech_bubbles);
                make_room_for_pending_speech_bubble(pos, &mut pending_speech_bubbles);
                pending_speech_bubbles.push(PendingSpeechBubble {
                    pos,
                    text,
                    age: 0.0,
                    stack_offset: 0.0,
                    target_stack_offset: 0.0,
                });
            }
        }
    }
    let bubble_font = ui_fonts
        .as_deref()
        .map(|fonts| fonts.text_font(18.0, crate::ui_fonts::UiFontWeight::Regular))
        .unwrap_or_default();
    for bubble in pending_speech_bubbles {
        spawn_speech_bubble(
            &mut commands,
            spawn_scope,
            world,
            bubble.pos,
            &bubble.text,
            bubble.age,
            bubble.stack_offset,
            bubble.target_stack_offset,
            &bubble_font,
        );
    }
}

/// **Can `fx` be drawn as ART right now, and from what?**
///
/// The decision `spawn_effect` makes, factored out so a test can ask the engine
/// rather than re-derive it: `None` here IS the particle fallback. Two ways to
/// get it — the id names no shipped row, or the sheet holding that row is not
/// decoded in these assets.
///
/// ⛔ the slot is re-resolved against the LOADED spec rather than trusted from
/// the baked index. They are the same sheet today; a quality variant with a
/// different row layout would make them disagree, and drawing the wrong row is
/// exactly the failure `first_bound_row` was built to refuse.
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

/// **How big an unscaled authored effect is drawn, in world units.**
///
/// ⛔ **this was `132.0` written inline, and it was every effect in the
/// project.** Jon, 2026-08-16: *"right now we are seeing crazy upscaled vfx"*.
/// Measured against a two-CPU Smash capture, a fighter's body stands about 60
/// world units; a 132-unit square is more than twice her height, so a jab's
/// spark and a screen-clearing super drew at the same size and both of them
/// covered the fighter throwing them.
///
/// ⭐ **the number is now the DEFAULT, not the answer.** A move authors
/// `Vfx { scale }` (see `MoveEventKind::Vfx`), so a flourish asks for less and a
/// super asks for more — which is the expressive range the constant took away.
/// This value is a little under a fighter's height on purpose: an effect the
/// same size as the body reads as the body's own, which is what a move's burst
/// is.
pub const FX_DEFAULT_WORLD_SIZE: f32 = 56.0;

/// **Draw the authored effect `fx`, or say why not.**
///
/// Three ways this ends, and they are different facts: the id names no shipped
/// row (a counted miss — the authored id is wrong or the art was never made);
/// the sheet holding the row is not decoded (`--no-assets`, or a build whose
/// FX manifests were not baked); or it draws. Only the last one is art, and the
/// other two share the particle fallback that used to be the ONLY outcome
/// outside Ambition.
fn spawn_effect(
    commands: &mut Commands,
    session_scope: Option<SessionSpawnScope>,
    world: &ae::World,
    assets: Option<&ambition_sprite_sheet::game_assets::GameAssets>,
    pos: ae::Vec2,
    fx: FxId,
    scale: f32,
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
    let scale = scale.max(0.1);
    let render_size = BVec2::splat(FX_DEFAULT_WORLD_SIZE * scale);
    // ⛔ NOT `build_character_sprite_with_render_size`: that opens on
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
            Transform::from_translation(world_to_bevy(world, pos, WORLD_Z_FX + 6.0)),
            animator,
            EffectVisual {
                pos,
                age: 0.0,
                // ⭐ the AUTHORED length, not a magic 0.72. The row knows how
                // many frames it has and how long each one holds; a fixed
                // duration either cut a long effect off or held a short one on
                // its last frame while it faded.
                duration: effect.clip_secs().max(0.05),
            },
        ),
    );
}

fn make_room_for_speech_bubble(
    pos: ae::Vec2,
    world: &ae::World,
    speech_bubbles: &mut Query<(&mut SpeechBubbleVisual, &mut Transform, &mut TextColor)>,
) {
    for (mut bubble, mut transform, mut color) in speech_bubbles.iter_mut() {
        if !speech_bubbles_should_stack(bubble.pos, pos) {
            continue;
        }
        let (age, stack_offset, target_stack_offset) = pushed_speech_bubble(
            bubble.age,
            bubble.stack_offset,
            bubble.target_stack_offset,
            bubble.duration,
        );
        bubble.age = age;
        bubble.stack_offset = stack_offset;
        bubble.target_stack_offset = target_stack_offset;
        apply_speech_bubble_visual(
            world,
            bubble.pos,
            bubble.age,
            bubble.duration,
            bubble.stack_offset,
            &mut transform,
            &mut color,
        );
    }
}

fn make_room_for_pending_speech_bubble(pos: ae::Vec2, speech_bubbles: &mut [PendingSpeechBubble]) {
    for bubble in speech_bubbles.iter_mut() {
        if !speech_bubbles_should_stack(bubble.pos, pos) {
            continue;
        }
        let (age, stack_offset, target_stack_offset) = pushed_speech_bubble(
            bubble.age,
            bubble.stack_offset,
            bubble.target_stack_offset,
            SPEECH_BUBBLE_DURATION,
        );
        bubble.age = age;
        bubble.stack_offset = stack_offset;
        bubble.target_stack_offset = target_stack_offset;
    }
}

fn speech_bubbles_should_stack(existing: ae::Vec2, incoming: ae::Vec2) -> bool {
    (existing.x - incoming.x).abs() <= SPEECH_BUBBLE_STACK_X_RANGE
        && (existing.y - incoming.y).abs() <= SPEECH_BUBBLE_STACK_Y_RANGE
}

fn pushed_speech_bubble(
    age: f32,
    stack_offset: f32,
    target_stack_offset: f32,
    duration: f32,
) -> (f32, f32, f32) {
    let next_target_stack_offset =
        (target_stack_offset + SPEECH_BUBBLE_STACK_STEP).min(SPEECH_BUBBLE_STACK_MAX);
    let next_stack_offset = stack_offset
        .max(next_target_stack_offset.min(stack_offset + SPEECH_BUBBLE_STACK_INITIAL_NUDGE));
    let next_age = age.max((duration - SPEECH_BUBBLE_PUSH_FADE_AFTER).max(0.0));
    (next_age, next_stack_offset, next_target_stack_offset)
}

fn advance_speech_bubble_stack_offset(stack_offset: f32, target_stack_offset: f32, dt: f32) -> f32 {
    let remaining = target_stack_offset - stack_offset;
    if remaining <= 0.0 {
        return stack_offset;
    }
    stack_offset + remaining.min(SPEECH_BUBBLE_STACK_SPEED * dt)
}

fn speech_bubble_progress(age: f32, duration: f32) -> f32 {
    if duration <= 0.0 {
        return 1.0;
    }
    (age / duration).clamp(0.0, 1.0)
}

fn speech_bubble_rise(age: f32, duration: f32, stack_offset: f32) -> f32 {
    SPEECH_BUBBLE_BASE_RISE * speech_bubble_progress(age, duration) + stack_offset
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

fn apply_speech_bubble_visual(
    world: &ae::World,
    pos: ae::Vec2,
    age: f32,
    duration: f32,
    stack_offset: f32,
    transform: &mut Transform,
    color: &mut TextColor,
) {
    let rise = speech_bubble_rise(age, duration, stack_offset);
    let alpha = speech_bubble_alpha(age, duration);
    transform.translation = world_to_bevy(world, pos + ae::Vec2::new(0.0, -rise), WORLD_Z_FX + 8.0);
    *color = TextColor(Color::srgba(1.0, 1.0, 1.0, 0.95 * alpha));
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
        &mut Transform,
        &mut TextColor,
    )>,
) {
    let dt = time.delta_secs();
    for (entity, mut bubble, mut transform, mut color) in &mut query {
        bubble.age += dt;
        bubble.stack_offset =
            advance_speech_bubble_stack_offset(bubble.stack_offset, bubble.target_stack_offset, dt);
        if bubble.age >= bubble.duration {
            commands.entity(entity).despawn();
            continue;
        }
        apply_speech_bubble_visual(
            &world.0,
            bubble.pos,
            bubble.age,
            bubble.duration,
            bubble.stack_offset,
            &mut transform,
            &mut color,
        );
    }
}

pub fn update_speech_bubble_outlines(
    bubbles: Query<(&SpeechBubbleVisual, &Children)>,
    mut outline_colors: Query<&mut TextColor, With<SpeechBubbleOutline>>,
) {
    for (bubble, children) in &bubbles {
        let outline_alpha = 0.88 * speech_bubble_alpha(bubble.age, bubble.duration);
        for child in children.iter() {
            if let Ok(mut outline_color) = outline_colors.get_mut(child) {
                *outline_color = TextColor(Color::srgba(0.0, 0.0, 0.0, outline_alpha));
            }
        }
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

pub fn spawn_speech_bubble(
    commands: &mut Commands,
    session_scope: Option<SessionSpawnScope>,
    world: &ae::World,
    pos: ae::Vec2,
    text: &str,
    age: f32,
    stack_offset: f32,
    target_stack_offset: f32,
    // The face the bubble draws in. Threaded rather than repaired a frame later
    // (the way `label_layout` patches world labels) because a bubble lives about
    // a second and fades the whole time — one frame in the wrong font is a
    // meaningful fraction of the thing.
    font: &TextFont,
) {
    // ⚠ this line is why the font MATTERS here more than anywhere else: the
    // bubble supplies its own non-ASCII. Left at `TextFont::default()` the
    // curly quotes resolved Bevy's built-in `FiraMono-subset.ttf` — the same
    // handle the menu tofu came down to. ⚠ NOT yet photographed here: the
    // menu's box does not reproduce everywhere that handle is used, so treat
    // this as the same defect SHAPE rather than a confirmed sighting. See
    // `ambition_menu::render::bevy_ui::MenuFont`.
    let bubble_text = format!("\u{201c}{text}\u{201d}");
    let Some(session_scope) = session_scope else {
        return;
    };
    let duration = SPEECH_BUBBLE_DURATION;
    let mut transform = Transform::default();
    let mut color = TextColor(Color::srgba(1.0, 1.0, 1.0, 0.95));
    apply_speech_bubble_visual(
        world,
        pos,
        age,
        duration,
        stack_offset,
        &mut transform,
        &mut color,
    );
    let outline_color = TextColor(Color::srgba(
        0.0,
        0.0,
        0.0,
        0.88 * speech_bubble_alpha(age, duration),
    ));
    commands
        .spawn_session_scoped(
            session_scope,
            (
                Text2d::new(bubble_text.clone()),
                TextFont {
                    font_size: 18.0,
                    ..font.clone()
                },
                color,
                transform,
                SpeechBubbleVisual {
                    pos,
                    age,
                    duration,
                    stack_offset,
                    target_stack_offset,
                },
                Name::new(format!("Speech bubble: {text}")),
            ),
        )
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
                        font_size: 18.0,
                        ..font.clone()
                    },
                    outline_color,
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
    // Reset is a teleport-like state transition. Showing both endpoints avoids
    // the ambiguity where a burst at spawn can look like a coordinate bug when
    // the player reset from somewhere else.
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
        let lifetime = 0.22 + 0.16 * ((i * 7 + 3) % 9) as f32 / 8.0;
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
                    gravity: match kind {
                        ParticleKind::Spark => 300.0,
                        ParticleKind::Dust => 120.0,
                        ParticleKind::Shard => 650.0,
                    },
                    drag: match kind {
                        ParticleKind::Spark => 3.4,
                        ParticleKind::Dust => 4.7,
                        ParticleKind::Shard => 1.8,
                    },
                },
            ),
        );
    }
}

/// **One coin, up and back down** — the acknowledgement a struck coin block owes.
///
/// A single ballistic particle rather than a burst: the coin leaves straight up,
/// gravity brings it back, and it is gone inside a third of a second. It is
/// presentation only — no collider, no pickup, no session state — because the
/// block already credited the purse before this was written.
///
/// ⭐ **the four numbers are the whole feel and they are together on purpose.**
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
                // ⚠ NEGATIVE y is up: world y is down-positive here.
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
/// Pure consumer of the sim-resolved
/// [`ambition_sim_view::BlinkPreviewFact`] (E4 slice 18):
/// the destination is computed sim-side with the SAME resolution the actual
/// blink uses, so the preview can never disagree with the eventual teleport
/// endpoint. This system only draws the ember ring.
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

    /// **Every shipped effect row is addressable by its hashed name, and the
    /// sound comes with it.**
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

    /// **The old five are ordinary rows now.**
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
        // ⭐ and one from outside it, resolved by the same call — the property
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
}
