//! The NATIVE saddle point, kept as the reference trace of its procedural
//! module (`ambition_content_modules::saddle_point`). Test-only: the game runs
//! the module. `module_parity_tests` holds the module to this system.

use bevy::prelude::*;

use ambition_boss_encounter::BossClusterRef;
use ambition_characters::brain::ActorActionMessage;
use ambition_platformer2d::actor::FeatureSimEntity;
use ambition_platformer2d_core as ae;
use ambition_time::WorldTime;

// ===================================================================
// Migrated boss-special Techniques (from ambition_platformer2d_actor_monolith brain_effects).
// Each owns its key + per-boss state + params + behavior; the engine
// names none of them.
// ===================================================================

const SADDLE_POINT_KEY: &str = "saddle_point";

// Values are tuned for the gradient-sentinel arena.
const SADDLE_POINT_ARM_LENGTH: f32 = 220.0;
const SADDLE_POINT_ARM_THICKNESS: f32 = 36.0;
const SADDLE_POINT_AXIS_PERIOD_S: f32 = 1.2;
const SADDLE_POINT_DAMAGE: i32 = 2;

/// Per-boss state for `saddle_point`. Tracks which axis (horizontal arm
/// or vertical arm) is currently the damaging one + how much time
/// is left in this axis before the toggle.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct SaddlePointState {
    /// Whether the strike is currently active. Reset on no-message
    /// ticks so we re-spawn the hitbox entities on the next strike.
    pub strike_active: bool,
    /// Which axis is "live" — true = horizontal arm, false = vertical.
    pub axis_horizontal: bool,
    /// Seconds left in the current axis before toggling.
    pub axis_remaining_s: f32,
    /// Hitbox entities for each arm; tracked so the rotation can
    /// despawn/replace them on toggle. `None` between strikes.
    pub horizontal_hitbox: Option<Entity>,
    pub vertical_hitbox: Option<Entity>,
}

const SADDLE_POINT_KNOCKBACK: f32 = 1.6;

/// EFFECTS consumer: `saddle_point` rotating cross hazard.
///
/// On the first Special message of a strike, spawns two World-anchored
/// hitbox entities centered on the boss — one horizontal arm, one
/// vertical arm. Only the "live" axis carries non-zero damage; the
/// inactive axis is despawned. Every `axis_period_s` seconds the
/// active axis toggles: the live hitbox is despawned and the other
/// arm is spawned in its place. This creates a readable "stand on
/// the safe axis" puzzle for the player.
///
/// The boss may move during the strike (AnchorSway profile), so the
/// hitboxes use the boss entity as their anchor base by being
/// re-spawned at the boss position each toggle. (A future
/// `HitboxAnchor::FollowOwner` with a per-arm long offset would
/// move the cross with the boss in real time — out of scope here.)
pub fn spawn_saddle_point_from_special_messages(
    mut commands: Commands,
    world_time: Res<WorldTime>,
    mut messages: MessageReader<ActorActionMessage>,
    mut bosses: Query<
        (
            Entity,
            BossClusterRef,
            &ambition_characters::actor::BodyHealth,
            &mut SaddlePointState,
        ),
        With<FeatureSimEntity>,
    >,
) {
    let dt = world_time.sim_dt();

    let firing = super::actors_firing(&mut messages, SADDLE_POINT_KEY);

    for (entity, boss_feature, health, mut state) in &mut bosses {
        let boss = boss_feature.as_boss_ref();
        if !firing.contains_key(&entity) {
            // Strike closed — despawn any lingering hitboxes and
            // reset state so the next strike starts clean.
            if let Some(h) = state.horizontal_hitbox.take() {
                commands.entity(h).despawn();
            }
            if let Some(h) = state.vertical_hitbox.take() {
                commands.entity(h).despawn();
            }
            state.strike_active = false;
            state.axis_remaining_s = 0.0;
            continue;
        };
        if !health.alive() {
            if let Some(h) = state.horizontal_hitbox.take() {
                commands.entity(h).despawn();
            }
            if let Some(h) = state.vertical_hitbox.take() {
                commands.entity(h).despawn();
            }
            continue;
        }
        let (arm_length, arm_thickness, axis_period_s, damage) = (
            SADDLE_POINT_ARM_LENGTH,
            SADDLE_POINT_ARM_THICKNESS,
            SADDLE_POINT_AXIS_PERIOD_S,
            SADDLE_POINT_DAMAGE,
        );
        let period = axis_period_s.max(0.05);

        // Strike start (or re-start after a between-strike gap):
        // spawn the initial active hitbox + reset rotation timer.
        // The boss may move during the strike (AnchorSway), so each
        // toggle re-spawns at the *current* boss center.
        let spawn_axis_hitbox = |commands: &mut Commands, axis_horizontal: bool| -> Entity {
            let (he_x, he_y) = if axis_horizontal {
                (arm_length, arm_thickness)
            } else {
                (arm_thickness, arm_length)
            };
            // Lifetime > axis_period_s so the hitbox doesn't expire
            // mid-axis. We despawn it on toggle or strike end.
            //
            // This one calls the executor DIRECTLY (not via `Effect::DamageBox`)
            // on purpose: the rotating cross tracks each arm's `Entity` to
            // despawn it on toggle, and the fire-and-forget `EffectRequest` seam
            // can't hand the spawned entity back. Effects you need a handle to
            // use the spawn helper directly; fire-and-forget ones emit a request.
            ambition_combat::strike::spawn_damage_box(
                commands,
                entity,
                ambition_vfx::HitSide::Boss,
                boss.kin.pos,
                ambition_combat::strike::DamageBox {
                    half_extent: ae::Vec2::new(he_x, he_y),
                    shape: None,
                    damage,
                    knockback: SADDLE_POINT_KNOCKBACK,
                    lifetime_s: period * 2.0,
                    name: None,
                },
            )
        };

        if !state.strike_active {
            // First tick of the strike — clear any leftovers and
            // spawn the first axis.
            if let Some(h) = state.horizontal_hitbox.take() {
                commands.entity(h).despawn();
            }
            if let Some(h) = state.vertical_hitbox.take() {
                commands.entity(h).despawn();
            }
            // Start on the horizontal axis (matches the visual
            // expectation: cross forms, horizontal arm lights up
            // first, then alternates).
            state.axis_horizontal = true;
            state.horizontal_hitbox = Some(spawn_axis_hitbox(&mut commands, true));
            state.vertical_hitbox = None;
            state.axis_remaining_s = period;
            state.strike_active = true;
            continue;
        }

        // Continuing strike — advance axis timer; toggle on expiry.
        state.axis_remaining_s = (state.axis_remaining_s - dt).max(0.0);
        if state.axis_remaining_s <= 0.0 {
            state.axis_horizontal = !state.axis_horizontal;
            // Despawn previous axis, spawn the new one.
            if let Some(h) = state.horizontal_hitbox.take() {
                commands.entity(h).despawn();
            }
            if let Some(h) = state.vertical_hitbox.take() {
                commands.entity(h).despawn();
            }
            if state.axis_horizontal {
                state.horizontal_hitbox = Some(spawn_axis_hitbox(&mut commands, true));
            } else {
                state.vertical_hitbox = Some(spawn_axis_hitbox(&mut commands, false));
            }
            state.axis_remaining_s = period;
        }
    }
}

impl bevy::ecs::entity::MapEntities for SaddlePointState {
    fn map_entities<M: bevy::ecs::entity::EntityMapper>(&mut self, mapper: &mut M) {
        if let Some(entity) = self.horizontal_hitbox.as_mut() {
            *entity = mapper.get_mapped(*entity);
        }
        if let Some(entity) = self.vertical_hitbox.as_mut() {
            *entity = mapper.get_mapped(*entity);
        }
    }
}
