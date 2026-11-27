//! The dash's afterimages: while a body dashes it leaves copies of the
//! picture it draws, tinted and fading where they were left, so the burst
//! reads as speed and not as a teleport.
//!
//! A copy is what the body draws on that tick: its visible part slots (a
//! rigged body), or its root sprite (a baked one, or a rigged body drawn
//! composited). Each copy is a plain sprite in the same render layers, a hair
//! behind the body, and fades on presentation time.
//!
//! Presentation only, and keyed to the sim tick like the locomotion dust: a
//! copy is left on a tick the body dashes, so the trail has the same spacing
//! at any refresh rate, and a rollback that replays the tick leaves nothing
//! twice (the tick is remembered).

use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;

use ambition_platformer2d_core::BodyMotionFacts;
use ambition_time::SimTick;

use super::actors::rigged::{RiggedPresentation, RiggedPresentedBy};

/// The afterimage's tint: the dash's purple (the concept sheet's), multiplied
/// into the copy, so the white shell goes lavender and the outlines stay dark.
const AFTERIMAGE_TINT: (f32, f32, f32) = (0.74, 0.5, 1.0);
/// How opaque a copy is when it is left.
const AFTERIMAGE_ALPHA: f32 = 0.55;
/// How long a copy takes to fade out (seconds).
const AFTERIMAGE_LIFE: f32 = 0.22;
/// Sim ticks between two copies of one body.
const AFTERIMAGE_EVERY: u64 = 2;
/// How far behind the body a copy is drawn: past the whole stack of its part
/// slots (`SLOT_DEPTH_STEP` apart), not past the world behind it.
const AFTERIMAGE_DEPTH: f32 = 0.01;

/// One copy of a dashing body's picture, fading.
#[derive(Component, Debug)]
pub struct DashAfterimage {
    age: f32,
}

/// The ticks each body last left a copy on.
#[derive(Default)]
pub struct AfterimageMemory {
    tick: Option<u64>,
    left: std::collections::HashMap<Entity, u64>,
}

type SpriteSource<'a> = (&'a Sprite, &'a GlobalTransform, &'a InheritedVisibility, Option<&'a RenderLayers>);

/// Leave a copy of each dashing body's picture.
pub fn leave_dash_afterimages(
    tick: Res<SimTick>,
    mut memory: Local<AfterimageMemory>,
    mut commands: Commands,
    bodies: Query<(Entity, &BodyMotionFacts, Option<&RiggedPresentedBy>)>,
    owners: Query<&RiggedPresentation>,
    sprites: Query<SpriteSource<'_>>,
) {
    if memory.tick == Some(tick.0) {
        return;
    }
    memory.tick = Some(tick.0);
    let now = tick.0;
    memory.left.retain(|_, at| now.saturating_sub(*at) < AFTERIMAGE_EVERY * 8);
    for (entity, facts, presented) in &bodies {
        if !facts.dashing {
            continue;
        }
        if memory.left.get(&entity).is_some_and(|at| now.saturating_sub(*at) < AFTERIMAGE_EVERY) {
            continue;
        }
        memory.left.insert(entity, now);
        let slots = presented
            .and_then(|by| owners.get(by.owner()).ok())
            .map(|presentation| presentation.slots.as_slice())
            .unwrap_or_default();
        let mut copied = 0;
        for slot in slots {
            if let Ok(source) = sprites.get(*slot) {
                copied += usize::from(copy(&mut commands, source));
            }
        }
        // Nothing drawn from parts: the root draws the body (a baked sheet,
        // or the composited cell).
        if copied == 0 {
            if let Ok(source) = sprites.get(entity) {
                copy(&mut commands, source);
            }
        }
    }
}

/// Spawn a fading copy of one visible sprite. False when it is not drawn.
fn copy(commands: &mut Commands, (sprite, at, visible, layers): SpriteSource<'_>) -> bool {
    if !visible.get() || sprite.image == Handle::default() {
        return false;
    }
    let mut transform = at.compute_transform();
    transform.translation.z -= AFTERIMAGE_DEPTH;
    let (r, g, b) = AFTERIMAGE_TINT;
    let mut ghost = sprite.clone();
    ghost.color = Color::srgba(r, g, b, AFTERIMAGE_ALPHA);
    let mut spawned = commands.spawn((Name::new("dash afterimage"), ghost, transform, DashAfterimage { age: 0.0 }));
    if let Some(layers) = layers {
        spawned.insert(layers.clone());
    }
    true
}

/// Fade the copies out, and take them away once they have.
pub fn fade_dash_afterimages(
    time: Res<Time>,
    mut commands: Commands,
    mut ghosts: Query<(Entity, &mut DashAfterimage, &mut Sprite)>,
) {
    let dt = time.delta_secs();
    for (entity, mut ghost, mut sprite) in &mut ghosts {
        ghost.age += dt;
        let left = 1.0 - ghost.age / AFTERIMAGE_LIFE;
        if left <= 0.0 {
            commands.entity(entity).despawn();
            continue;
        }
        sprite.color.set_alpha(AFTERIMAGE_ALPHA * left * left);
    }
}
