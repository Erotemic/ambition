//! The NATIVE apple rain, kept as the reference trace of its procedural
//! module (`ambition_content_modules::apple_rain`). Test-only: the game runs
//! the module. `module_parity_tests` holds the module to this system.

use bevy::prelude::*;

use ambition_boss_encounter::BossClusterRef;
use ambition_characters::brain::{
    ActorActionMessage,
};
use ambition_platformer2d::actor::FeatureSimEntity;
use ambition_platformer2d_core as ae;
use ambition_projectiles::{ProjectileSpawn, ProjectileSpawnRequest, ProjectileStart};
use ambition_time::WorldTime;

const APPLE_RAIN_KEY: &str = "apple_rain";
const APPLE_RAIN_INTERVAL: f32 = 0.35;
const APPLE_RAIN_SPAWN_SPEED: f32 = 35.0;
const APPLE_RAIN_DAMAGE: i32 = 1;


/// Defaulted-attached to every boss; only a boss whose `ActionSet.special` is
/// `SpecialActionSpec::Special("apple_rain")` advances it (only such a boss generates the Special
/// messages the consumer reads) — the `gnu_ton_rider` encounter is the current wielder. Per the
/// actor/brain follow-up plan Task B: components hold state, consumers spawn effects.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct AppleRainSpawnState {
    /// Seconds carried over from the previous tick's apple-spawn.
    /// Drained while >= `interval_s` and refilled by `dt` each tick
    /// the Special message arrives.
    pub spawn_accum: f32,
    /// Monotonic spawn counter; used as the golden-ratio sequence
    /// index for deterministic per-spawn x distribution.
    pub spawn_index: u32,
}

/// Apple cosmetic / collision constants. These are the only definitions; a
/// second boss that adopts the consumer pattern shares them.
const APPLE_RAIN_HALF_EXTENT: ae::Vec2 = ae::Vec2::new(14.0, 16.0);
const APPLE_RAIN_GRAVITY: f32 = 540.0;
const APPLE_RAIN_LIFETIME: f32 = 6.0;
const APPLE_RAIN_SPAWN_HEIGHT_ABOVE_PLAYER: f32 = 320.0;
/// Apple art is data-driven via the `"apple"` visual id registered in the
/// projectile visual catalog; ownership and friendly-fire come from the firing
/// body and its frozen projectile allegiance instead of an id convention.
const PHI_FRAC: f32 = 0.618_033_99;

/// Horizontal spawn lane (world x) for the `spawn_index`-th apple.
/// Apples spread across the playable width by a golden-ratio
/// sequence — even coverage without an obvious left-to-right sweep —
/// then slide out from under the boss body so an apple never spawns
/// already overlapping the boss head; it picks the nearer boss edge to
/// keep the dodge small. Pure so the distribution + dodge are
/// unit-testable independently of the message/projectile plumbing.
fn apple_rain_spawn_x(spawn_index: u32, world_width: f32, boss_aabb: ae::Aabb) -> f32 {
    let margin = APPLE_RAIN_HALF_EXTENT.x + 8.0;
    let max_x = (world_width - margin).max(margin);
    let spawnable_width = (max_x - margin).max(0.0);
    let frac = ((spawn_index as f32) * PHI_FRAC).fract();
    let mut spawn_x = margin + frac * spawnable_width;
    let self_left = boss_aabb.min.x - APPLE_RAIN_HALF_EXTENT.x;
    let self_right = boss_aabb.max.x + APPLE_RAIN_HALF_EXTENT.x;
    if spawn_x > self_left && spawn_x < self_right {
        spawn_x = if spawn_x - self_left < self_right - spawn_x {
            self_left
        } else {
            self_right
        };
        spawn_x = spawn_x.clamp(margin, max_x);
    }
    spawn_x
}

/// Spawn the apple-rain barrage in response to `ActorActionMessage::Special { spec:
/// SpecialActionSpec::Special("apple_rain") }`. The boss runtime tags `frame.special_pressed =
/// true` every tick its `BossAttackProfile::Special("apple_rain")` strike window is active; the
/// resolver translates that into one `Special` message per tick.
///
/// Bosses whose Special slot is something other than `apple_rain`
/// emit no messages this consumer cares about; bosses whose
/// `BossPattern` brain doesn't fire `special_pressed` simply pass
/// through. The per-boss `AppleRainSpawnState` resets to zero on
/// any tick the message doesn't arrive, so the next strike window
/// starts on a clean beat instead of inheriting a burst from
/// leftover dt.
pub fn spawn_apple_rain_from_special_messages(
    world_time: Res<WorldTime>,
    // The width of each boss's own live room (OW1 cut 7n).
    world: ambition_platformer2d::platformer::lifecycle::LiveRoomOf<
        ambition_platformer2d_core::RoomGeometry,
    >,
    mut messages: MessageReader<ActorActionMessage>,
    mut projectiles: MessageWriter<ProjectileSpawnRequest>,
    mut bosses: Query<
        (
            Entity,
            &mut AppleRainSpawnState,
            BossClusterRef,
            &ambition_characters::actor::BodyHealth,
        ),
        With<FeatureSimEntity>,
    >,
) {
    let dt = world_time.sim_dt();
    // Apple-rain tuning is content-owned (lib consts for now; move with the
    // technique). The brain fires one `Special("apple_rain")` message per tick
    // the strike window is active.
    let (interval_s, spawn_speed, damage) = (
        APPLE_RAIN_INTERVAL,
        APPLE_RAIN_SPAWN_SPEED,
        APPLE_RAIN_DAMAGE,
    );
    // Bosses with an `apple_rain` Special this tick. Multiple messages from one
    // boss collapse to the same entry — "any message this tick" = "strike
    // window active this tick".
    let firing = super::actors_firing(&mut messages, APPLE_RAIN_KEY);

    for (entity, mut state, boss_feature, health) in &mut bosses {
        if !firing.contains_key(&entity) {
            // No message this tick → reset accumulator so a future
            // strike window starts on a clean beat.
            state.spawn_accum = 0.0;
            continue;
        };
        let boss = boss_feature.as_boss_ref();
        if !health.alive() || interval_s <= 0.0 {
            continue;
        }
        let Some(geometry) = world.of(entity) else {
            continue;
        };
        state.spawn_accum += dt;
        let self_aabb = boss.aabb();
        while state.spawn_accum >= interval_s {
            state.spawn_accum -= interval_s;
            // Golden-ratio spread across the playable width, slid out
            // from under the boss body. See `apple_rain_spawn_x`.
            let spawn_x = apple_rain_spawn_x(state.spawn_index, geometry.0.size.x, self_aabb);
            let spawn_y = (boss.kin.pos.y - APPLE_RAIN_SPAWN_HEIGHT_ABOVE_PLAYER)
                .max(APPLE_RAIN_HALF_EXTENT.y + 8.0);
            projectiles.write(ProjectileSpawnRequest::open(
                entity,
                ProjectileSpawn {
                    origin: ae::Vec2::new(spawn_x, spawn_y),
                    // Downward initial velocity so the apple commits to
                    // its lane immediately instead of hanging at zero
                    // until gravity catches up.
                    dir: ae::Vec2::new(0.0, 1.0),
                    speed: spawn_speed,
                    damage,
                    max_lifetime: APPLE_RAIN_LIFETIME,
                    half_extent: APPLE_RAIN_HALF_EXTENT,
                    gravity: APPLE_RAIN_GRAVITY,
                    // The apple-rain fruit renders as the generated apple
                    // sprite (kept upright vs gravity) — keyed by kind, not
                    // by the retired owner-string convention the visuals layer once read.
                    visual_id: "apple".to_string(),
                    // Straight shot: this ability authors no bounce.
                    bounces: 0,
                    bounce_on_world_contact: false,
                    splash_half_extent: 0.0,
                    boomerang_return_s: None,
                },
                ProjectileStart::StepThisTick,
            )
            .fired_by_move_if_any(firing.get(&entity).copied().flatten()),
            );
            state.spawn_index = state.spawn_index.wrapping_add(1);
        }
    }
}
