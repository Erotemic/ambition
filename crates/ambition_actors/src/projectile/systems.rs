//! The Bevy gameplay system that ticks projectiles, samples motion
//! input, and routes hits through ECS-native feature damage messages.

use ambition_engine_core as ae;
use ambition_engine_core::AabbExt;
use bevy::prelude::*;

use super::diagnostics::log_press_diagnostics;
use super::entity::{LiveProjectile, ProjectileOwner, ProjectileOwnerId, ProjectileSeq};
use super::spawn_message::{ProjectilePool, SpawnProjectile};
use super::state::{PlayerProjectileState, ProjectileTraceEvent};
use super::{resolve_world_collision, WorldHitOutcome};
use crate::actor::BodyKinematics;
use crate::features::{
    can_damage, damage_lands, ActorAggression, ActorDisposition, ActorFaction, BossClusterRef,
    BossConfig, BreakableFeature, CenteredAabb, FeatureId, FeatureSimEntity, HitEvent,
    HitKnockback, HitMode, HitSource, HitTarget,
};
use crate::projectile::ProjectileGameplay;
use crate::trace::GameplayTraceBuffer;
use ambition_characters::actor::BodyCombat;
use ambition_sfx::SfxMessage;
use ambition_vfx::vfx::VfxMessage;

/// Speed multiplier applied to a parried shot as it reverses — a timed parry
/// sends the bolt back a little faster than it arrived.
const PROJECTILE_REFLECT_SPEED_SCALE: f32 = 1.3;
/// Health a successful parry restores (a reason to parry rather than dodge).
const PARRY_HEAL: i32 = 1;

/// Body-generic projectile PARRY reflect: a timed shield RE-OWNS the shot to the
/// parrying body (so its firer faction becomes the parrier's next tick → damage
/// routes off the parrier, back at whoever it feuds with) and reverses+boosts the
/// velocity. Re-owning — not flipping a faction label — is how a reflected shot
/// becomes the parrier's attack now that damage is owner-driven. The SAME mechanic
/// for the player and any shielding actor (a possessed body, a mixed-faction
/// duelist); the player's parry HEAL stays a player-facing reward at the call site
/// (fable review 2026-07-02 §A10).
fn reflect_parried_shot(
    commands: &mut Commands,
    proj_entity: Entity,
    kin: &mut BodyKinematics,
    parrier: Entity,
    sfx: &mut MessageWriter<SfxMessage>,
    vfx: &mut MessageWriter<VfxMessage>,
) {
    commands
        .entity(proj_entity)
        .insert(ProjectileOwner(parrier));
    kin.vel = -kin.vel * PROJECTILE_REFLECT_SPEED_SCALE;
    sfx.write(SfxMessage::Play {
        id: ambition_sfx::ids::WORLD_ROCK_HIT,
        pos: kin.pos,
    });
    vfx.write(VfxMessage::Impact { pos: kin.pos });
}
const PLAYER_PROJECTILE_MUZZLE_CLEARANCE: f32 = 4.0;

fn player_projectile_local_fire_dir(aim_local: ae::Vec2, facing: f32) -> ae::Vec2 {
    if aim_local.length() > 0.1 {
        aim_local.normalize_or_zero()
    } else {
        ae::Vec2::new(facing, 0.0)
    }
}

fn player_projectile_muzzle_local_offset(
    local_dir: ae::Vec2,
    facing: f32,
    size: ae::Vec2,
) -> ae::Vec2 {
    let half = size * 0.5;
    if local_dir.x.abs() >= local_dir.y.abs() {
        let side = if local_dir.x.abs() > 0.001 {
            local_dir.x.signum()
        } else {
            facing.signum()
        };
        ae::Vec2::new(
            side * (half.x + PLAYER_PROJECTILE_MUZZLE_CLEARANCE),
            -size.y * 0.20,
        )
    } else {
        let feet_axis = local_dir.y.signum();
        ae::Vec2::new(
            facing.signum() * half.x * 0.4,
            feet_axis * (half.y + PLAYER_PROJECTILE_MUZZLE_CLEARANCE),
        )
    }
}

/// Charge-projectile INPUT: per-BODY charge / Hadouken-motion recognition / fire.
/// Emits [`SpawnProjectile`] into the shared pool; the actual flight is stepped by
/// [`step_projectiles`] (the unified faction-general stepper).
///
/// Body/ability-subject, NOT player-marker: it iterates any body carrying the
/// chargeable-projectile CAPABILITY ([`ambition_characters::brain::ChargesProjectiles`])
/// plus its charge state — the SAME capability gate the emitter
/// (`emit_player_projectile_tick_messages`) uses, so the two sides are symmetric.
/// The projectile origin is the EMITTING body's own muzzle (`kin.pos`), so a
/// possessed body that adopts the player's kit fires from ITSELF, not the home
/// avatar. Only the home body carries the charge state today; the player-flavoured
/// anim pulse is therefore OPTIONAL (a non-home charge body has no `BodyAnimFacts`).
#[allow(clippy::too_many_arguments)]
pub fn charge_projectile_input(
    world_time: Res<ambition_time::WorldTime>,
    // Per-BODY projectile state lives on the charge-capable body itself. Iterates
    // every such body so co-op / possession builds get one independent charge timer.
    mut charge_body_q: Query<
        (
            Entity,
            &crate::actor::BodyKinematics,
            &mut crate::projectile::PlayerProjectileState,
            Option<&mut crate::actor::BodyAnimFacts>,
        ),
        With<ambition_characters::brain::ChargesProjectiles>,
    >,
    mut brain_actions: MessageReader<ambition_characters::brain::ActorActionMessage>,
    user_settings: Res<ambition_persistence::settings::UserSettings>,
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
            ambition_characters::brain::ActionRequest::PlayerProjectileTick {
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
    for (body_entity, kin, mut state, mut anim) in &mut charge_body_q {
        let tick_info = tick_infos.get(&body_entity).copied().unwrap_or_default();
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
        let local_dir = player_projectile_local_fire_dir(tick_info.aim, facing);
        let local_muzzle = player_projectile_muzzle_local_offset(local_dir, facing, kin.size);
        let origin = kin.pos + frame.to_world(local_muzzle);
        let direction = frame.to_world(local_dir).normalize_or_zero();

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
            // `RUST_LOG=ambition_actors::projectile=info` (or
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
                    body_entity,
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
                    body_entity,
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
        // The player-flavoured anim pulse only exists on a home body; a possessed
        // charge body drives its own actor anim, so this is optional.
        if let Some(anim) = anim.as_mut() {
            if anim.aim_anim_active != charging {
                anim.aim_anim_active = charging;
            }
            if fired_this_frame > 0 {
                anim.shoot_anim_timer = SHOOT_ANIM_HOLD_SECS;
            }
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
            let spec = kind.charged_spec(spec, charge_tier);
            spawn_projectiles.write(SpawnProjectile {
                pool: ProjectilePool::Player { owner },
                projectile: crate::projectile::InFlightProjectile {
                    body: crate::projectile::ProjectileBody::from_spec(spec),
                    owner_id: String::new(),
                },
                kind: Some(kind),
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

/// Flattened view of the `PlayerProjectileTick` request — used inside
/// `update_projectiles` after destructuring the matched
/// `ActorActionMessage`. A separate type so the "no message arrived
/// this tick" fallback can rely on `Default`.
#[derive(Clone, Copy, Debug, Default)]
struct PlayerProjectileTickInfo {
    axis: ae::Vec2,
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
///   one despawn) and bounce on solids per `WorldHitPolicy::Bouncing`.
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
    world_time: Res<ambition_time::WorldTime>,
    carved: ambition_projectiles::collision_world::ProjectileCollisionWorld,
    gravity: crate::physics::GravityCtx,
    mut projectiles: Query<
        (
            Entity,
            &mut BodyKinematics,
            &mut ProjectileGameplay,
            Option<&ProjectileOwner>,
            Option<&ProjectileOwnerId>,
            &ProjectileSeq,
            Option<&crate::projectile::ProjectileKind>,
            Option<&crate::projectile::ProjectileVisualKind>,
        ),
        (
            With<LiveProjectile>,
            Without<crate::actor::PlayerEntity>,
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
            &crate::features::CenteredAabb,
            &crate::actor::BodyOffense,
            &crate::actor::BodyDodgeState,
            &crate::actor::BodyShieldState,
            &ambition_characters::actor::BodyCombat,
        ),
        (With<crate::actor::PlayerEntity>, Without<LiveProjectile>),
    >,
    mut feature_damage: MessageWriter<HitEvent>,
    ecs_breakables: Query<(&FeatureId, &CenteredAabb, &BreakableFeature), With<FeatureSimEntity>>,
    ecs_actors: Query<
        (&FeatureId, &CenteredAabb, &ActorDisposition, &BodyCombat),
        (With<FeatureSimEntity>, Without<BossConfig>),
    >,
    ecs_bosses: Query<
        (
            &FeatureId,
            &CenteredAabb,
            BossClusterRef,
            &ambition_characters::actor::BodyHealth,
            &ambition_characters::brain::BossAttackState,
            Option<&crate::features::BossAnimationFrameSample>,
        ),
        With<FeatureSimEntity>,
    >,
    mut sfx: MessageWriter<SfxMessage>,
    mut vfx: MessageWriter<VfxMessage>,
    mut heals: MessageWriter<crate::player::PlayerHealRequested>,
    mut trace: ResMut<GameplayTraceBuffer>,
    // Relational damage authority + non-player actor victims for actor-vs-actor
    // projectile damage. A shot damages any DIFFERENT-faction body it hits, routed
    // off the FIRER's faction (looked up from its owner entity): a PCA (Enemy)
    // glider hits a robot (Boss) and vice versa, and a stray hits a different-faction
    // bystander (the observer). Same-faction allies are spared unless friendly fire
    // is on — so a pirate's shot can't hit another pirate. (Targeting is separate.)
    friendly_fire: Option<Res<crate::features::FriendlyFire>>,
    // Bundled into ONE tuple slot to stay under Bevy's 16-param ceiling:
    // - `actor_victims` — non-boss actors a hostile shot can damage (actor-vs-actor).
    // - `owner_combat` — the firer's REAL faction + optional grudge, looked up from
    //   the projectile's owner entity (player / enemy / boss / player-robot). The
    //   faction RETIRES the binary `ProjectileGameplay.faction` (damage routes off the
    //   owner, not a side label); the grudge is the per-entity DAMAGE override that
    //   lets a shot hit a same-faction body its firer feuds with (an `Npc` duelist's
    //   bolt). Read-only, so it may overlap `actor_victims`.
    (actor_victims, owner_combat): (
        Query<
            (
                Entity,
                &CenteredAabb,
                &ActorFaction,
                Option<&crate::actor::BodyShieldState>,
            ),
            (With<FeatureSimEntity>, Without<BossConfig>),
        >,
        Query<(&ActorFaction, Option<&ActorAggression>)>,
    ),
) {
    let dt = world_time.sim_dt();
    let friendly_fire = friendly_fire.map(|r| *r).unwrap_or_default();
    let collision_world = carved.solids();
    let portal_list = carved.portal_list();
    let tick = trace.current_tick();

    // Collect + sort by the GLOBAL spawn sequence (a single deterministic order
    // across both factions; the seq counter is shared at spawn).
    let mut ordered: Vec<(Entity, ProjectileSeq)> = projectiles
        .iter()
        .map(|(entity, _, _, _, _, seq, _, _)| (entity, *seq))
        .collect();
    ordered.sort_by_key(|(_, seq)| *seq);

    for (proj_entity, _) in ordered {
        let Ok((_, mut kin, mut game, owner, _owner_id, _, kind, visual_kind)) =
            projectiles.get_mut(proj_entity)
        else {
            continue;
        };
        // Named kind for player shots (None for kind-less enemy volleys).
        let kind = kind.copied();
        // Visual identity (every spawned shot carries one; default to the
        // generic hostile look if somehow absent). Drives the detonation FX
        // pick — by kind, not by sniffing the owner-id string.
        let visual_kind = visual_kind.copied().unwrap_or_default();
        let owner_entity = owner.map(|o| o.0);
        // The firer's real faction — the OWNER's, not the shot's stored label.
        // `None` = OWNERLESS (a truly ownerless volley, or a shot whose firer
        // despawned mid-flight): there is no one to be friendly to, so it becomes
        // INDISCRIMINATE — environmental damage that hurts every body it overlaps,
        // friend or foe. A Player-faction firer's shot is the player's universal
        // attack (hits breakables/actors/bosses); any other firer's shot is hostile
        // (damages the player + relational foes).
        let owner_combat_data = owner_entity.and_then(|e| owner_combat.get(e).ok());
        let firer_faction: Option<ActorFaction> = owner_combat_data.map(|(f, _)| *f);
        // The firer's personal grudge — the per-entity damage override (a duelist's
        // shot lands on the rival it feuds with even at the same faction).
        let firer_grudge: Option<Entity> = owner_combat_data
            .and_then(|(_, agg)| agg)
            .and_then(|a| a.grudge);
        let indiscriminate = firer_faction.is_none();

        // Tick + lifetime. A dead lasersword detonates; everything else logs an
        // Expired trace event.
        let gravity_dir = gravity.dir_at(kin.pos);
        if !game.tick(&mut kin, dt, gravity_dir) {
            if let Some(boom) = visual_kind.expiry_vfx(kin.pos) {
                vfx.write(boom);
                sfx.write(SfxMessage::Play {
                    id: ambition_sfx::ids::WORLD_EXPLOSION,
                    pos: kin.pos,
                });
            } else {
                trace.push_event(ProjectileTraceEvent::Expired { kind }.into_trace_event(tick));
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

        // Damage routed by the FIRER's real faction (the owner's), not a label on
        // the shot. A Player-faction firer's shot is the player's universal attack;
        // any other firer's shot (or an OWNERLESS, indiscriminate one) damages the
        // player + actors.
        if firer_faction == Some(ActorFaction::Player) {
            let hit_event = HitEvent {
                volume: kin.aabb().into(),
                damage: game.damage.max(1),
                // Player shots always carry the kind component; default is
                // unreachable (kept total for the engine-generic body).
                source: HitSource::PlayerProjectile,
                attacker: owner_entity,
                target: HitTarget::Volume,
                mode: HitMode::Knockback,
                knockback: None,
                ignored_targets: Vec::new(),
            };
            let hit = crate::features::ecs_hit_event_hits_breakable(&hit_event, &ecs_breakables)
                || crate::features::ecs_hit_event_hits_actor(&hit_event, &ecs_actors)
                || crate::features::ecs_hit_event_hits_boss(&hit_event, &ecs_bosses);
            if hit {
                feature_damage.write(hit_event);
                sfx.write(SfxMessage::Hit { pos: kin.pos });
                trace.push_event(
                    ProjectileTraceEvent::Hit {
                        kind,
                        damage: game.damage,
                    }
                    .into_trace_event(tick),
                );
                commands.entity(proj_entity).despawn();
                continue;
            }
        } else {
            // A hostile shot (any non-Player firer) OR an OWNERLESS indiscriminate
            // shot. It damages the player it overlaps + actors. With a firer, the
            // `can_damage` (different-faction) rule decides who; an indiscriminate
            // shot bypasses that — it hurts everyone, there is no ally to spare.
            let mut hit_any_player = false;
            let mut reflected = false;
            // Damage is physical: a shot hits the player if it's a different
            // faction from the firer (so a duel's stray catches the observer);
            // only a same-faction firer (co-op) passes them by, unless friendly
            // fire is on. An ownerless shot always can (indiscriminate).
            let can_hit_player = indiscriminate
                || firer_faction
                    .is_some_and(|f| can_damage(f, ActorFaction::Player, friendly_fire));
            for (player_entity, player_kin, hurtbox, offense, dodge, shield, combat) in
                &player_body_q
            {
                if !can_hit_player {
                    break;
                }
                // The PUBLISHED gravity-oriented hurtbox (§A6), not a raw
                // kinematics box.
                if !kin.aabb().strict_intersects(hurtbox.aabb()) {
                    continue;
                }
                // Parry: a timed shield RE-OWNS the shot to the player (so its
                // firer faction becomes Player next tick → it routes as the
                // player's own shot, back at the enemies) and reverses (+boosts)
                // its velocity. Re-owning, not flipping a faction label, is how a
                // reflected shot becomes the player's attack now that damage is
                // owner-driven.
                if shield.parrying() {
                    reflect_parried_shot(
                        &mut commands,
                        proj_entity,
                        &mut kin,
                        player_entity,
                        &mut sfx,
                        &mut vfx,
                    );
                    // Player-facing reward policy — the reflect mechanic above is
                    // shared with actors; only the player heals on parry.
                    heals.write(crate::player::PlayerHealRequested::new(PARRY_HEAL));
                    reflected = true;
                    break;
                }
                // The ONE vulnerability rule (§A5). This site had drifted (it
                // dropped the parry term); behavior is unchanged because a
                // parrying shield reflects + breaks above before reaching here.
                if !crate::combat::util::body_vulnerable(offense, dodge, shield, combat) {
                    continue;
                }
                // Knockback side in the victim's LOCAL frame (fable review
                // 2026-07-02 §B11): a screen-X difference degenerates exactly
                // when sideways gravity separates the pair along world-Y.
                let side = ae::AccelerationFrame::new(gravity.dir_at(player_kin.pos)).side;
                let knock_dir = (player_kin.pos - kin.pos).dot(side).signum();
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
                    volume: kin.aabb().into(),
                    damage: game.damage.max(1),
                    source: HitSource::EnemyProjectile,
                    // The firing actor (enemy / boss), when the shot was
                    // spawned with a real owner — `None` for ownerless shots.
                    attacker: owner_entity,
                    target: HitTarget::Player(player_entity),
                    mode: HitMode::Knockback,
                    knockback: Some(HitKnockback {
                        dir: knock_dir,
                        strength: 0.85,
                        source_pos: kin.pos,
                        impact_pos,
                        launch_dir: None,
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
            // Relational actor-vs-actor (S3e): the shot damages the first
            // overlapping actor its firer is hostile to (e.g. a Boss-faction
            // body in a spectator arena), pre-resolved to that exact entity.
            let mut hit_any_actor = false;
            let mut reflected_by_actor = false;
            for (victim_entity, victim_aabb, victim_faction, victim_shield) in &actor_victims {
                if Some(victim_entity) == owner_entity {
                    continue;
                }
                // An owned shot damages a faction-foe (any different faction) OR a
                // same-faction body its firer holds a grudge against (an `Npc`
                // duelist's bolt); an indiscriminate (ownerless) shot damages every
                // actor it overlaps.
                let can_hit = indiscriminate
                    || firer_faction.is_some_and(|f| {
                        damage_lands(
                            f,
                            *victim_faction,
                            friendly_fire,
                            firer_grudge,
                            victim_entity,
                        )
                    });
                if !can_hit {
                    continue;
                }
                if !kin.aabb().strict_intersects(victim_aabb.aabb()) {
                    continue;
                }
                // Parry: a shielding actor (a possessed body, a mixed-faction
                // duelist) reflects the shot through the SAME re-own mechanic the
                // player uses — the shot survives as the parrier's bolt, back at its
                // foes (§A10). No heal: the parry-heal is player reward policy.
                if victim_shield.is_some_and(|s| s.parrying()) {
                    reflect_parried_shot(
                        &mut commands,
                        proj_entity,
                        &mut kin,
                        victim_entity,
                        &mut sfx,
                        &mut vfx,
                    );
                    reflected_by_actor = true;
                    break;
                }
                feature_damage.write(HitEvent {
                    volume: kin.aabb().into(),
                    damage: game.damage.max(1),
                    source: HitSource::EnemyProjectile,
                    attacker: owner_entity,
                    target: HitTarget::Actor(victim_entity),
                    mode: HitMode::Knockback,
                    knockback: None,
                    ignored_targets: Vec::new(),
                });
                sfx.write(SfxMessage::Hit { pos: kin.pos });
                vfx.write(VfxMessage::Impact { pos: kin.pos });
                hit_any_actor = true;
                break;
            }
            // A shot an actor parried survives as that actor's bolt (keep flying).
            if reflected_by_actor {
                continue;
            }
            if hit_any_actor {
                commands.entity(proj_entity).despawn();
                continue;
            }
        }

        // World collision: the policy is the projectile's own (authored on its
        // spec/ability, firer-agnostic) — NOT a function of who fired it. A
        // bouncing fireball arcs whoever throws it; a lasersword detonates on
        // the wall. (B2: retires the faction→policy fork.)
        let world_hit = game.world_hit;
        match resolve_world_collision(
            &mut kin,
            &mut game,
            &collision_world,
            world_hit,
            gravity_dir,
        ) {
            WorldHitOutcome::Bounced { pos } => {
                sfx.write(SfxMessage::Hit { pos });
            }
            WorldHitOutcome::Expired { pos } => {
                match visual_kind.expiry_vfx(pos) {
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
                                kind,
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

#[cfg(test)]
mod parry_tests {
    use super::*;

    #[derive(Resource)]
    struct Parrier(Entity);

    fn reflect_the_shot(
        mut commands: Commands,
        parrier: Res<Parrier>,
        mut sfx: MessageWriter<SfxMessage>,
        mut vfx: MessageWriter<VfxMessage>,
        mut shots: Query<(Entity, &mut BodyKinematics)>,
    ) {
        for (proj, mut kin) in &mut shots {
            reflect_parried_shot(&mut commands, proj, &mut kin, parrier.0, &mut sfx, &mut vfx);
        }
    }

    /// The body-generic parry reflect — the ONE mechanic the player parry and the
    /// new actor parry both call (§A10) — re-owns the shot to the parrying body and
    /// reverses + boosts its velocity, so a reflected shot becomes the parrier's own
    /// bolt (damage routes off the parrier's faction next tick) whether a player or
    /// a shielding actor caught it. Pins that a future edit can't make the reflect
    /// re-own to a hardcoded player again.
    #[test]
    fn reflect_re_owns_the_shot_to_the_parrier_and_reverses_velocity() {
        let mut app = App::new();
        app.add_message::<SfxMessage>();
        app.add_message::<VfxMessage>();
        let parrier = app.world_mut().spawn_empty().id();
        let proj = app
            .world_mut()
            .spawn(BodyKinematics {
                pos: ae::Vec2::ZERO,
                vel: ae::Vec2::new(100.0, -40.0),
                size: ae::Vec2::new(8.0, 8.0),
                facing: 1.0,
            })
            .id();
        app.insert_resource(Parrier(parrier));
        app.add_systems(Update, reflect_the_shot);
        app.update();

        let world = app.world();
        let owner = world
            .get::<ProjectileOwner>(proj)
            .expect("the parried shot is re-owned to the parrier");
        assert_eq!(owner.0, parrier, "re-owned to the body that parried it");
        let kin = world.get::<BodyKinematics>(proj).unwrap();
        assert_eq!(
            kin.vel,
            ae::Vec2::new(-100.0, 40.0) * PROJECTILE_REFLECT_SPEED_SCALE,
            "velocity reversed and boosted by the reflect scale"
        );
    }
}
