//! The Bevy gameplay system that ticks projectiles, samples motion
//! input, and routes hits through ECS-native feature damage messages.

use crate::engine_core as ae;
use crate::engine_core::AabbExt;
use bevy::prelude::*;

use super::diagnostics::log_press_diagnostics;
use super::entity::{
    LiveProjectile, PlayerProjectile, ProjectileOwner, ProjectileOwnerId, ProjectileSeq,
    ProjectileSeqCounter,
};
use super::spawn_message::{ProjectilePool, SpawnProjectile};
use super::state::{PlayerProjectileState, ProjectileTraceEvent};
use super::{resolve_world_collision, ProjectileFaction, WorldHitOutcome, WorldHitPolicy};
use crate::audio::SfxMessage;
use crate::features::{
    ActorCombatState, ActorDisposition, BossClusterRef, BossConfig, BreakableFeature, CenteredAabb,
    FeatureId, FeatureSimEntity, HitEvent, HitKnockback, HitMode, HitSource, HitTarget,
};
use crate::player::BodyKinematics;
use crate::projectile::ProjectileGameplay;
use crate::trace::GameplayTraceBuffer;
use crate::GameWorld;
use ambition_vfx::vfx::VfxMessage;

/// Speed multiplier applied to a parried shot as it reverses — a timed parry
/// sends the bolt back a little faster than it arrived.
const PROJECTILE_REFLECT_SPEED_SCALE: f32 = 1.3;
/// Health a successful parry restores (a reason to parry rather than dodge).
const PARRY_HEAL: i32 = 1;

/// A timed-out or wall-killed **lasersword** detonates with a rendered
/// explosion (keyed on the `lasersword:`-prefixed owner id). Returns `None` for
/// any other projectile. VFX-only — replay-neutral.
fn lasersword_detonation(owner_id: &str, pos: ae::Vec2) -> Option<VfxMessage> {
    owner_id
        .starts_with("lasersword")
        .then_some(VfxMessage::Explosion {
            pos,
            kind: ambition_vfx::vfx::ExplosionKind::ClassicBurst,
            scale: 0.7,
        })
}

/// The portal-carved collision world a projectile collides against. Bundled as a
/// [`SystemParam`] so [`update_projectiles`] can build the carved world without
/// adding two more top-level params (it is already at Bevy's 16-param ceiling).
///
/// A portal punched through a wall leaves the opening non-solid, so a shot fired
/// into a wall portal flies THROUGH the opening instead of detonating on the wall
/// — and `portal_transit` (which already moves the projectile body) carries it
/// out the far portal. Without this the projectile collided against the raw world
/// and could never transit a wall portal.
#[derive(bevy::ecs::system::SystemParam)]
pub struct ProjectileCollisionWorld<'w, 's> {
    world: Res<'w, GameWorld>,
    overlay: Res<'w, crate::features::FeatureEcsWorldOverlay>,
    // Folded in here (rather than as its own top-level param) because
    // `update_projectiles` is already at Bevy's 16-param ceiling.
    portals: Query<'w, 's, &'static crate::portal::PlacedPortal>,
}

impl ProjectileCollisionWorld<'_, '_> {
    /// The room world with ONLY the portal apertures carved out — preserves the
    /// projectile's historical raw-world collision (it passes through moving
    /// platforms) while letting a shot sink into a portal opening and transit.
    /// Borrowed (no clone) in the common no-carve case.
    fn solids(&self) -> std::borrow::Cow<'_, ae::World> {
        crate::features::world_with_portal_carves(&self.world.0, &self.overlay.portal_carves)
    }

    /// Snapshot the placed portals for the per-projectile transit test.
    fn portal_list(&self) -> Vec<crate::portal::PlacedPortal> {
        self.portals.iter().copied().collect()
    }
}

/// Player projectile INPUT: per-player charge / Hadouken-motion recognition /
/// fire. Emits [`SpawnProjectile`] into the shared pool; the actual flight is
/// stepped by [`step_projectiles`] (the unified faction-general stepper). Split
/// out of the former `update_projectiles` when the player + enemy step loops
/// were merged.
#[allow(clippy::too_many_arguments)]
pub fn player_projectile_input(
    world_time: Res<crate::WorldTime>,
    // Per-player projectile state lives on the player entity itself. Iterates
    // every player so co-op / possession builds get one independent charge timer.
    mut player_q: Query<
        (
            Entity,
            &crate::player::BodyKinematics,
            &mut crate::projectile::PlayerProjectileState,
            &mut crate::player::PlayerAnimState,
        ),
        With<crate::player::PlayerEntity>,
    >,
    mut brain_actions: MessageReader<crate::brain::ActorActionMessage>,
    user_settings: Res<crate::persistence::settings::UserSettings>,
    gravity: crate::physics::GravityCtx,
    mut trace: ResMut<GameplayTraceBuffer>,
    // Firing emits `SpawnProjectile`; the player-pool consumer runs after this
    // system so newly-fired projectiles first tick next frame.
    mut spawn_projectiles: MessageWriter<SpawnProjectile>,
) {
    // Sim clock: spawner pacing freezes in bullet-time alongside the world.
    let dt = world_time.sim_dt();

    // Build a per-actor map of PlayerProjectileTick infos so the
    // per-player loop below can look up each player's tick info by
    // entity without re-iterating the message stream. The brain-side
    // emitter (`emit_player_projectile_tick_messages`) produces
    // exactly one per player-brain actor per tick.
    let tick_infos: std::collections::HashMap<Entity, PlayerProjectileTickInfo> = brain_actions
        .read()
        .filter_map(|msg| match msg.request {
            crate::brain::ActionRequest::PlayerProjectileTick {
                axis,
                aim,
                press,
                held,
                released,
            } => Some((
                msg.actor,
                PlayerProjectileTickInfo {
                    axis,
                    aim,
                    press,
                    held,
                    released,
                },
            )),
            _ => None,
        })
        .collect();

    let damage_mult = user_settings.gameplay.player_damage_multiplier;
    for (player_entity, kin, mut state, mut anim) in &mut player_q {
        let tick_info = tick_infos.get(&player_entity).copied().unwrap_or_default();
        state.clock += dt;
        state.spawner.tick(dt);

        // Sample motion for Hadouken recognition. Both the action message
        // axis and `MotionDirection::from_axis` use the +Y-DOWN convention
        // (the engine matcher returns `Down` for y > 0; pinned by the
        // `motion_direction_quantization` engine test). Pass axis through
        // unchanged — an earlier negation here was inverting the sign
        // and silently mapping every "press Down" sample to `Up`, which
        // made every QCF detection fail forever.
        let dir =
            crate::projectile::MotionDirection::from_axis(tick_info.axis.x, tick_info.axis.y, 0.55);
        let now = state.clock;
        state.motion_buffer.push(dir, now);

        let mut events: Vec<ProjectileTraceEvent> = Vec::new();

        let facing = if kin.facing.abs() < f32::EPSILON {
            1.0
        } else {
            kin.facing.signum()
        };
        let frame = ae::AccelerationFrame::new(gravity.dir_at(kin.pos));
        let origin = kin.pos
            + frame.to_world(ae::Vec2::new(facing * (kin.size.x * 0.5 + 4.0), -kin.size.y * 0.20));
        let direction = frame.to_world(ae::Vec2::new(facing, 0.0));

        // Count fires locally because spawn messages are consumed after this
        // system, but shoot animation still pulses on the firing frame.
        let mut fired_this_frame = 0u32;

        // Press edge: try Hadouken tiers first (most-specific motion gate
        // wins), else start charging a Fireball. Order matters — the
        // grace shape is a SUBSEQUENCE of the full QCF, so check Super
        // first; otherwise a 3-step input would fire a weak Hadouken.
        if tick_info.press {
            let super_qcf = state.motion_buffer.detect_quarter_circle();
            let half_circle = state.motion_buffer.detect_half_circle();
            let grace_qcf = state.motion_buffer.detect_quarter_circle_grace();

            let motion_kind = if (super_qcf.is_some() || half_circle.is_some())
                && state.unlocked.hadouken_super
            {
                Some(crate::projectile::ProjectileKind::HadoukenSuper)
            } else if grace_qcf.is_some() && state.unlocked.hadouken {
                Some(crate::projectile::ProjectileKind::Hadouken)
            } else {
                None
            };

            // Debug log on every fire-press so the player can see
            // exactly what the motion recognizer saw and why a given
            // press did or didn't upgrade to a Hadouken. Run with
            // `RUST_LOG=ambition_gameplay_core::projectile=info` (or
            // `RUST_LOG=info` more broadly) to surface these.
            log_press_diagnostics(
                &state.motion_buffer,
                super_qcf,
                half_circle,
                grace_qcf,
                motion_kind,
            );

            if let Some(kind) = motion_kind {
                // Motion gesture committed — fire immediately, do not
                // start a charge for this press.
                fired_this_frame += try_fire_projectile(
                    &mut state,
                    player_entity,
                    kind,
                    origin,
                    direction,
                    damage_mult,
                    0,
                    &mut events,
                    &mut spawn_projectiles,
                ) as u32;
                state.motion_buffer.clear();
                state.charging = None;
            } else if state.unlocked.fireball {
                // Begin charging the Fireball. Release-edge below
                // commits the charged shot.
                state.charging = Some(0.0);
            }
        } else if tick_info.held {
            if let Some(t) = state.charging.as_mut() {
                *t += dt;
            }
        } else if tick_info.released {
            if let Some(hold) = state.charging.take() {
                let tier = state.charge_tuning.tier_for_hold(hold);
                fired_this_frame += try_fire_projectile(
                    &mut state,
                    player_entity,
                    crate::projectile::ProjectileKind::Fireball,
                    origin,
                    direction,
                    damage_mult,
                    tier,
                    &mut events,
                    &mut spawn_projectiles,
                ) as u32;
            }
        }

        // Mirror projectile state onto the player's animation flags. `aim`
        // tracks the held-charge pose every frame; `shoot` is a short
        // post-fire pulse triggered only on the frame the body count grew.
        // SHOOT_ANIM_HOLD_SECS is short enough that a rapid-fire stream
        // visibly stutters between Shoot and Idle/Walk rather than locking
        // out the locomotion read.
        const SHOOT_ANIM_HOLD_SECS: f32 = 0.18;
        let charging = state.charging.is_some();
        if anim.aim_anim_active != charging {
            anim.aim_anim_active = charging;
        }
        if fired_this_frame > 0 {
            anim.shoot_anim_timer = SHOOT_ANIM_HOLD_SECS;
        }

        let tick = trace.current_tick();
        for event in events {
            trace.push_event(event.into_trace_event(tick));
        }
    } // end per-player loop
}

/// Run spawn checks for `kind`, apply the Fireball charge tier, and emit a
/// player-pool [`SpawnProjectile`] on success. Shared by press and release
/// paths; returns whether the shoot animation should pulse.
#[allow(clippy::too_many_arguments)]
fn try_fire_projectile(
    state: &mut PlayerProjectileState,
    owner: Entity,
    kind: crate::projectile::ProjectileKind,
    origin: ae::Vec2,
    direction: ae::Vec2,
    damage_mult: f32,
    charge_tier: u8,
    events: &mut Vec<ProjectileTraceEvent>,
    spawn_projectiles: &mut MessageWriter<SpawnProjectile>,
) -> bool {
    match state
        .spawner
        .try_spawn(kind, origin, direction, damage_mult)
    {
        Ok(spec) => {
            let spec = spec.with_charge_tier(charge_tier);
            spawn_projectiles.write(SpawnProjectile {
                pool: ProjectilePool::Player { owner },
                projectile: crate::projectile::InFlightProjectile {
                    body: crate::projectile::ProjectileBody::from_spec(spec),
                    owner_id: String::new(),
                },
            });
            events.push(ProjectileTraceEvent::Fired { kind });
            true
        }
        Err(crate::projectile::SpawnFailure::OutOfResource) => {
            events.push(ProjectileTraceEvent::BlockedByResource { kind });
            false
        }
        Err(crate::projectile::SpawnFailure::Cooldown) => false,
    }
}

/// Consume player-pool [`SpawnProjectile`] messages and spawn one projectile
/// entity per message. Scheduled after `update_projectiles` so new player
/// projectiles exist this frame but first tick next frame. Enemy-pool messages
/// are ignored here and consumed by the enemy-projectile system.
pub fn apply_player_spawn_projectile_messages(
    mut commands: Commands,
    mut seq: ResMut<ProjectileSeqCounter>,
    mut spawn_projectiles: MessageReader<SpawnProjectile>,
) {
    for msg in spawn_projectiles.read() {
        let ProjectilePool::Player { owner } = msg.pool else {
            continue;
        };
        let body = &msg.projectile.body;
        commands.spawn((
            body.kin,
            body.game,
            ProjectileOwner(owner),
            seq.next(),
            ProjectileOwnerId(msg.projectile.owner_id.clone()),
            crate::projectile::LiveProjectile,
            PlayerProjectile,
            Name::new("Player projectile (sim)"),
        ));
    }
}

/// Flattened view of the `PlayerProjectileTick` request — used inside
/// `update_projectiles` after destructuring the matched
/// `ActorActionMessage`. A separate type so the "no message arrived
/// this tick" fallback can rely on `Default`.
#[derive(Clone, Copy, Debug, Default)]
struct PlayerProjectileTickInfo {
    axis: ae::Vec2,
    #[allow(dead_code, reason = "carried for future aim-driven fire direction")]
    aim: ae::Vec2,
    press: bool,
    held: bool,
    released: bool,
}

/// The unified projectile step pipeline. Processes EVERY in-flight projectile —
/// player- and enemy-spawned alike (one `LiveProjectile` query) — sorted by
/// the global [`ProjectileSeq`], routing behavior by
/// [`ProjectileGameplay::faction`]:
///
/// - **Player-faction** shots damage enemies / bosses / breakables (one hit =
///   one despawn) and bounce on solids per `WorldHitPolicy::PlayerBouncing`.
/// - **Enemy-faction** shots can be parried (flip to Player-faction + reflect),
///   else damage the first vulnerable overlapping player, and expire on any
///   solid contact.
///
/// Lasersword shots detonate (rendered explosion) on death / wall-hit either
/// way. This replaces the former separate `update_projectiles` step loop and
/// `update_enemy_projectiles`; the player INPUT / charge / fire stays in
/// [`update_projectiles`], which now only spawns into this shared pool.
#[allow(clippy::too_many_arguments)]
pub fn step_projectiles(
    mut commands: Commands,
    world_time: Res<crate::WorldTime>,
    carved: ProjectileCollisionWorld,
    gravity: crate::physics::GravityCtx,
    mut projectiles: Query<
        (
            Entity,
            &mut BodyKinematics,
            &mut ProjectileGameplay,
            Option<&ProjectileOwner>,
            Option<&ProjectileOwnerId>,
            &ProjectileSeq,
        ),
        (
            With<LiveProjectile>,
            Without<crate::player::PlayerEntity>,
            Without<FeatureSimEntity>,
        ),
    >,
    // Read-only player bodies for enemy-faction damage + parry. Disjoint from the
    // mutable projectile query above (both touch `BodyKinematics`; B0001) via the
    // `LiveProjectile` / `PlayerEntity` marker split.
    player_body_q: Query<
        (
            Entity,
            &BodyKinematics,
            &crate::player::PlayerOffense,
            &crate::player::PlayerDodgeState,
            &crate::player::PlayerShieldState,
            &crate::player::PlayerCombatState,
        ),
        (With<crate::player::PlayerEntity>, Without<LiveProjectile>),
    >,
    mut feature_damage: MessageWriter<HitEvent>,
    ecs_breakables: Query<(&FeatureId, &CenteredAabb, &BreakableFeature), With<FeatureSimEntity>>,
    ecs_actors: Query<
        (
            &FeatureId,
            &CenteredAabb,
            &ActorDisposition,
            &ActorCombatState,
        ),
        (With<FeatureSimEntity>, Without<BossConfig>),
    >,
    ecs_bosses: Query<
        (
            &FeatureId,
            &CenteredAabb,
            BossClusterRef,
            &crate::brain::BossAttackState,
            Option<&crate::features::BossAnimationFrameSample>,
        ),
        With<FeatureSimEntity>,
    >,
    mut sfx: MessageWriter<SfxMessage>,
    mut vfx: MessageWriter<VfxMessage>,
    mut heals: MessageWriter<crate::player::PlayerHealRequested>,
    mut trace: ResMut<GameplayTraceBuffer>,
) {
    let dt = world_time.sim_dt();
    let collision_world = carved.solids();
    let portal_list = carved.portal_list();
    let tick = trace.current_tick();

    // Collect + sort by the GLOBAL spawn sequence (a single deterministic order
    // across both factions; the seq counter is shared at spawn).
    let mut ordered: Vec<(Entity, ProjectileSeq)> = projectiles
        .iter()
        .map(|(entity, _, _, _, _, seq)| (entity, *seq))
        .collect();
    ordered.sort_by_key(|(_, seq)| *seq);

    for (proj_entity, _) in ordered {
        let Ok((_, mut kin, mut game, owner, owner_id, _)) = projectiles.get_mut(proj_entity)
        else {
            continue;
        };
        let owner_entity = owner.map(|o| o.0);
        let owner_id_str = owner_id.map(|o| o.0.clone()).unwrap_or_default();

        // Tick + lifetime. A dead lasersword detonates; everything else logs an
        // Expired trace event.
        let gravity_dir = gravity.dir_at(kin.pos);
        if !game.tick(&mut kin, dt, gravity_dir) {
            if let Some(boom) = lasersword_detonation(&owner_id_str, kin.pos) {
                vfx.write(boom);
                sfx.write(SfxMessage::Play {
                    id: ambition_sfx::ids::WORLD_EXPLOSION,
                    pos: kin.pos,
                });
            } else {
                trace.push_event(
                    ProjectileTraceEvent::Expired { kind: game.kind }.into_trace_event(tick),
                );
            }
            commands.entity(proj_entity).despawn();
            continue;
        }

        // Portal transit: thread the aperture instead of hitting the wall.
        if !portal_list.is_empty()
            && crate::projectile::try_projectile_portal_transit(&mut kin, &portal_list)
        {
            continue;
        }

        // Faction-routed damage.
        match game.faction {
            ProjectileFaction::Player => {
                let hit_event = HitEvent {
                    volume: kin.aabb(),
                    damage: game.damage.max(1),
                    source: HitSource::PlayerProjectile { kind: game.kind },
                    attacker: owner_entity,
                    target: HitTarget::Volume,
                    mode: HitMode::Knockback,
                    knockback: None,
                    ignored_targets: Vec::new(),
                };
                let hit =
                    crate::features::ecs_hit_event_hits_breakable(&hit_event, &ecs_breakables)
                        || crate::features::ecs_hit_event_hits_actor(&hit_event, &ecs_actors)
                        || crate::features::ecs_hit_event_hits_boss(&hit_event, &ecs_bosses);
                if hit {
                    feature_damage.write(hit_event);
                    sfx.write(SfxMessage::Hit { pos: kin.pos });
                    trace.push_event(
                        ProjectileTraceEvent::Hit {
                            kind: game.kind,
                            damage: game.damage,
                        }
                        .into_trace_event(tick),
                    );
                    commands.entity(proj_entity).despawn();
                    continue;
                }
            }
            ProjectileFaction::Enemy => {
                let mut hit_any_player = false;
                let mut reflected = false;
                for (player_entity, player_kin, offense, dodge, shield, combat) in &player_body_q {
                    if !kin.aabb().strict_intersects(player_kin.aabb()) {
                        continue;
                    }
                    // Parry: a timed shield flips the shot to the player's faction
                    // and reverses (+boosts) its velocity, so next tick's routing
                    // sends it back at the enemies.
                    if shield.parrying() {
                        game.faction = ProjectileFaction::Player;
                        kin.vel = -kin.vel * PROJECTILE_REFLECT_SPEED_SCALE;
                        sfx.write(SfxMessage::Play {
                            id: ambition_sfx::ids::WORLD_ROCK_HIT,
                            pos: kin.pos,
                        });
                        vfx.write(VfxMessage::Impact { pos: kin.pos });
                        heals.write(crate::player::PlayerHealRequested::new(PARRY_HEAL));
                        reflected = true;
                        break;
                    }
                    let dodge_rolling = dodge.roll_timer > 0.0;
                    let vulnerable = !offense.invincible && !dodge_rolling && combat.vulnerable();
                    if !vulnerable {
                        continue;
                    }
                    let knock_dir = (player_kin.pos.x - kin.pos.x).signum();
                    let knock_dir = if knock_dir.abs() < 0.001 {
                        1.0
                    } else {
                        knock_dir
                    };
                    let impact_pos = ae::Vec2::new(
                        (player_kin.pos.x + kin.pos.x) * 0.5,
                        (player_kin.pos.y + kin.pos.y) * 0.5,
                    );
                    feature_damage.write(HitEvent {
                        volume: kin.aabb(),
                        damage: game.damage.max(1),
                        source: HitSource::EnemyProjectile,
                        attacker: None,
                        target: HitTarget::Player(player_entity),
                        mode: HitMode::Knockback,
                        knockback: Some(HitKnockback {
                            dir: knock_dir,
                            strength: 0.85,
                            source_pos: kin.pos,
                            impact_pos,
                        }),
                        ignored_targets: Vec::new(),
                    });
                    sfx.write(SfxMessage::Hit { pos: kin.pos });
                    vfx.write(VfxMessage::Impact { pos: kin.pos });
                    hit_any_player = true;
                    break;
                }
                // A parried shot survives as a player-faction bolt (keep in flight).
                if reflected {
                    continue;
                }
                if hit_any_player {
                    commands.entity(proj_entity).despawn();
                    continue;
                }
            }
        }

        // World collision: faction picks the policy. Player fireballs bounce;
        // enemy shots expire on any contact. A lasersword detonates on the wall.
        let policy = match game.faction {
            ProjectileFaction::Player => WorldHitPolicy::PlayerBouncing,
            ProjectileFaction::Enemy => WorldHitPolicy::EnemyExpireOnAnyContact,
        };
        match resolve_world_collision(&mut kin, &mut game, &collision_world, policy, gravity_dir) {
            WorldHitOutcome::Bounced { pos } => {
                sfx.write(SfxMessage::Hit { pos });
            }
            WorldHitOutcome::Expired { pos } => {
                match lasersword_detonation(&owner_id_str, pos) {
                    Some(boom) => {
                        vfx.write(boom);
                        sfx.write(SfxMessage::Play {
                            id: ambition_sfx::ids::WORLD_EXPLOSION,
                            pos,
                        });
                    }
                    None => {
                        trace.push_event(
                            ProjectileTraceEvent::Hit {
                                kind: game.kind,
                                damage: game.damage,
                            }
                            .into_trace_event(tick),
                        );
                        vfx.write(VfxMessage::Impact { pos });
                    }
                }
                commands.entity(proj_entity).despawn();
            }
            WorldHitOutcome::Continue => {}
        }
    }
}
