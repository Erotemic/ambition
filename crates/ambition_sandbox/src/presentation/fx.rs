//! Procedural visual effects for the sandbox.
//!
//! Particles are CPU-side Bevy sprite entities for now. Keeping this behind a
//! compact module gives us a later migration seam to GPU particles or Hanabi.

use crate::engine_core as ae;
use crate::engine_core::AabbExt;
use bevy::math::Vec2 as BVec2;
use bevy::prelude::*;
use std::f32::consts::TAU;

use crate::config::{rgba, world_to_bevy, WORLD_Z_FX};

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
pub struct SlashPreviewVisual {
    age: f32,
    duration: f32,
}

#[derive(Component)]
pub struct SpeechBubbleVisual {
    pos: ae::Vec2,
    age: f32,
    duration: f32,
    stack_offset: f32,
    target_stack_offset: f32,
}

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
    angle_offset: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParticleKind {
    Spark,
    Dust,
    Shard,
}

/// Typed sandbox-side visual-effects message (Bevy 0.18 buffered Message
/// API; the pre-0.18 `Event`/`EventReader` names moved to observer-style
/// one-shots). Emitted by simulation systems via the Vec collector and
/// drained into `Messages<VfxMessage>` by the player tick. The
/// presentation-side `vfx_spawn_messages` subscriber spawns the actual
/// particle/impact/slash entities.
///
/// Headless builds omit the subscriber; messages accumulate and drain
/// without entity spawns.
#[derive(Message, Clone, Debug)]
pub enum VfxMessage {
    Burst {
        pos: ae::Vec2,
        count: u32,
        speed: f32,
        color: [f32; 4],
        kind: ParticleKind,
    },
    Dust {
        pos: ae::Vec2,
        facing: f32,
    },
    Impact {
        pos: ae::Vec2,
    },
    BlinkEffects {
        from: ae::Vec2,
        to: ae::Vec2,
        precision: bool,
    },
    SlashPreview {
        hitbox: ae::Aabb,
    },
    ResetEffects {
        from: ae::Vec2,
        to: ae::Vec2,
    },
    SpeechBubble {
        pos: ae::Vec2,
        text: String,
    },
}

/// Presentation-side subscriber. Reads `VfxMessage`s and spawns particle /
/// impact / slash entities. Skipped in headless builds.
pub fn vfx_spawn_messages(
    mut commands: Commands,
    mut messages: MessageReader<VfxMessage>,
    world: Res<crate::GameWorld>,
    mut speech_bubbles: Query<(
        &mut SpeechBubbleVisual,
        &mut Transform,
        &mut TextColor,
    )>,
) {
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
                    world,
                    pos,
                    count as usize,
                    speed,
                    color,
                    kind,
                );
            }
            VfxMessage::Dust { pos, facing } => spawn_dust(&mut commands, world, pos, facing),
            VfxMessage::Impact { pos } => spawn_impact(&mut commands, world, pos),
            VfxMessage::BlinkEffects {
                from,
                to,
                precision,
            } => {
                spawn_blink_effects(&mut commands, world, from, to, precision);
            }
            VfxMessage::SlashPreview { hitbox } => {
                spawn_slash_preview(&mut commands, world, hitbox);
            }
            VfxMessage::ResetEffects { from, to } => {
                spawn_reset_effects(&mut commands, world, from, to);
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
    for bubble in pending_speech_bubbles {
        spawn_speech_bubble(
            &mut commands,
            world,
            bubble.pos,
            &bubble.text,
            bubble.age,
            bubble.stack_offset,
            bubble.target_stack_offset,
        );
    }
}

fn make_room_for_speech_bubble(
    pos: ae::Vec2,
    world: &ae::World,
    speech_bubbles: &mut Query<(
        &mut SpeechBubbleVisual,
        &mut Transform,
        &mut TextColor,
    )>,
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

fn make_room_for_pending_speech_bubble(
    pos: ae::Vec2,
    speech_bubbles: &mut [PendingSpeechBubble],
) {
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
    let next_stack_offset = stack_offset.max(
        next_target_stack_offset.min(stack_offset + SPEECH_BUBBLE_STACK_INITIAL_NUDGE),
    );
    let next_age = age.max((duration - SPEECH_BUBBLE_PUSH_FADE_AFTER).max(0.0));
    (next_age, next_stack_offset, next_target_stack_offset)
}

fn advance_speech_bubble_stack_offset(
    stack_offset: f32,
    target_stack_offset: f32,
    dt: f32,
) -> f32 {
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
    transform.translation = world_to_bevy(
        world,
        pos + ae::Vec2::new(0.0, -rise),
        WORLD_Z_FX + 8.0,
    );
    *color = TextColor(Color::srgba(1.0, 1.0, 1.0, 0.95 * alpha));
}

pub fn update_speech_bubbles(
    mut commands: Commands,
    time: Res<Time>,
    world: Res<crate::GameWorld>,
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
        bubble.stack_offset = advance_speech_bubble_stack_offset(
            bubble.stack_offset,
            bubble.target_stack_offset,
            dt,
        );
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

pub fn update_particles(
    mut commands: Commands,
    time: Res<Time>,
    world: Res<crate::GameWorld>,
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
    world: Res<crate::GameWorld>,
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

pub fn update_slash_previews(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut SlashPreviewVisual, &mut Sprite)>,
) {
    let dt = time.delta_secs();
    for (entity, mut preview, mut sprite) in &mut query {
        preview.age += dt;
        if preview.age >= preview.duration {
            commands.entity(entity).despawn();
            continue;
        }
        let alpha = 0.80 * (1.0 - preview.age / preview.duration);
        sprite.color = Color::srgba(1.0, 1.0, 0.35, alpha);
    }
}

pub fn spawn_speech_bubble(
    commands: &mut Commands,
    world: &ae::World,
    pos: ae::Vec2,
    text: &str,
    age: f32,
    stack_offset: f32,
    target_stack_offset: f32,
) {
    let bubble_text = format!("\u{201c}{text}\u{201d}");
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
    commands.spawn((
        Text2d::new(bubble_text),
        TextFont {
            font_size: 18.0,
            ..default()
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
    ));
}

pub fn spawn_slash_preview(commands: &mut Commands, world: &ae::World, hitbox: ae::Aabb) {
    let size = hitbox.half_size() * 2.0;
    commands.spawn((
        Sprite::from_color(
            Color::srgba(1.0, 1.0, 0.35, 0.80),
            BVec2::new(size.x, size.y),
        ),
        Transform::from_translation(world_to_bevy(world, hitbox.center(), WORLD_Z_FX + 2.0)),
        SlashPreviewVisual {
            age: 0.0,
            duration: 0.10,
        },
    ));
}

pub fn spawn_impact(commands: &mut Commands, world: &ae::World, pos: ae::Vec2) {
    commands.spawn((
        Sprite::from_color(Color::srgba(1.0, 1.0, 0.35, 0.82), BVec2::splat(12.0)),
        Transform::from_translation(world_to_bevy(world, pos, WORLD_Z_FX + 1.0)),
        ImpactVisual {
            pos,
            age: 0.0,
            duration: 0.24,
            radius: 12.0,
        },
    ));
}

pub fn spawn_reset_effects(
    commands: &mut Commands,
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
        world,
        to,
        24,
        280.0,
        [0.55, 0.85, 1.0, 0.90],
        ParticleKind::Spark,
    );
    spawn_impact(commands, world, to);
}

pub fn spawn_burst(
    commands: &mut Commands,
    world: &ae::World,
    pos: ae::Vec2,
    count: usize,
    speed: f32,
    color_rgba: [f32; 4],
    kind: ParticleKind,
) {
    let count = count.max(1);
    for i in 0..count {
        let t = i as f32 / count as f32;
        let wobble = ((i * 37 + 17) as f32).sin() * 0.22;
        let angle = TAU * t + wobble;
        let strength = speed * (0.45 + 0.55 * ((i * 13 + 5) % 11) as f32 / 10.0);
        let vel = ae::Vec2::new(angle.cos() * strength, angle.sin() * strength);
        let radius = 2.0 + 2.5 * ((i * 5 + 1) % 7) as f32 / 6.0;
        let lifetime = 0.22 + 0.16 * ((i * 7 + 3) % 9) as f32 / 8.0;
        commands.spawn((
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
        ));
    }
}

pub fn spawn_dust(commands: &mut Commands, world: &ae::World, pos: ae::Vec2, facing: f32) {
    for i in 0..6 {
        let lateral = -facing * (75.0 + i as f32 * 18.0);
        let upward = -35.0 - i as f32 * 8.0;
        let radius = 3.5 + i as f32 * 0.35;
        commands.spawn((
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
        ));
    }
}

pub fn spawn_blink_effects(
    commands: &mut Commands,
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
        world,
        from,
        if precision { 18 } else { 12 },
        250.0,
        exit_color,
        ParticleKind::Spark,
    );
    spawn_burst(
        commands,
        world,
        to,
        if precision { 28 } else { 18 },
        360.0,
        entry_color,
        ParticleKind::Spark,
    );
    spawn_impact(commands, world, to);
}

/// Live ring of orbiting embers showing where the next blink will land.
///
/// Runs every frame while the blink button is held (or aim is engaged) and
/// the player has the `blink` ability. Mirrors the destination resolution
/// used by the engine and the `show_blink_preview` debug overlay so the
/// preview can never disagree with the eventual teleport endpoint:
/// precision aim uses `blink_destination_to_point` against the steered
/// offset, quick-tap uses `blink_destination` along input/facing.
///
/// The blink button shares ground with menu input, so this honours the same
/// gameplay-only gate as `draw_player_debug` — paused / dialog states do not
/// light up the ring.
#[cfg(feature = "input")]
pub fn update_blink_preview(
    mut commands: Commands,
    time: Res<Time>,
    world: Res<crate::GameWorld>,
    platform_set: Res<crate::MovingPlatformSet>,
    mode: Res<State<crate::game_mode::GameMode>>,
    scene: Res<crate::presentation::rendering::SceneEntities>,
    action_query: Query<
        &leafwing_input_manager::prelude::ActionState<crate::input::SandboxAction>,
        bevy::prelude::With<crate::presentation::rendering::PlayerVisual>,
    >,
    player_q: Query<
        (
            &crate::player::PlayerKinematics,
            &crate::player::PlayerAbilities,
            &crate::player::PlayerBlinkState,
        ),
        crate::player::PrimaryPlayerOnly,
    >,
    mut existing: Query<(Entity, &BlinkPreviewVisual, &mut Transform, &mut Sprite)>,
) {
    use crate::input::ControlFrame;

    let Ok((kin, abilities, blink_state)) = player_q.single() else {
        for (entity, _, _, _) in &existing {
            commands.entity(entity).despawn();
        }
        return;
    };
    let actions = if mode.get().allows_gameplay() {
        action_query.get(scene.player).ok()
    } else {
        None
    };
    let controls = actions.map(ControlFrame::read_gameplay).unwrap_or_default();

    let active = abilities.abilities.blink && (controls.blink_held || blink_state.aiming);

    if !active {
        for (entity, _, _, _) in &existing {
            commands.entity(entity).despawn();
        }
        return;
    }

    // Match the debug overlay's destination resolution exactly. The
    // moving-platform-aware temporary world is what the actual blink
    // resolves against, so the preview must use it too.
    let blink_world =
        crate::world::platforms::world_with_moving_platforms(&world.0, &platform_set.0);
    let target = if blink_state.aiming {
        ae::blink_destination_to_point_clusters(
            &blink_world,
            kin,
            abilities,
            kin.pos + blink_state.aim_offset,
        )
    } else {
        let aim = ae::Vec2::new(controls.axis_x, controls.axis_y)
            .normalize_or(ae::Vec2::new(kin.facing, 0.0));
        ae::blink_destination_clusters(&blink_world, kin, abilities, aim, ae::BLINK_DISTANCE)
    };

    let precision = blink_state.aiming;
    // Match the post-blink burst palette so the preview reads as
    // "this is what's about to happen here".
    let color = if precision {
        rgba(0.92, 0.42, 1.00, 0.85)
    } else {
        rgba(0.42, 1.00, 0.92, 0.80)
    };

    const RING_EMBERS: usize = 4;
    let radius = kin.size.min_element() * 0.45;
    let spin = time.elapsed_secs() * 2.4;
    let pulse = 1.0 + 0.18 * (time.elapsed_secs() * 5.5).sin();
    let ember_size = (kin.size.min_element() * 0.18) * pulse;

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
            commands.spawn((
                Sprite::from_color(color, BVec2::splat(ember_size.max(1.0))),
                Transform::from_translation(world_to_bevy(
                    &world.0,
                    target + offset,
                    WORLD_Z_FX + 1.5,
                )),
                BlinkPreviewVisual { angle_offset },
            ));
        }
    }
}
