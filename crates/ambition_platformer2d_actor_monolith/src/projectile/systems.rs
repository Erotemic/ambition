//! The Bevy gameplay system that ticks projectiles, samples motion
//! input, and routes hits through ECS-native feature damage messages.

use ambition_platformer2d_core as ae;
use ambition_platformer2d_core::AabbExt;
use bevy::prelude::*;

use super::allegiance::ProjectileAllegiance;
use ambition_projectiles::diagnostics::log_press_diagnostics;
use ambition_projectiles::entity::{LiveProjectile, ProjectileOwner, ProjectileSeq};
use ambition_projectiles::state::{PlayerProjectileState, ProjectileTraceEvent};
use ambition_projectiles::{resolve_world_collision, WorldHitOutcome};
use ambition_projectiles::{ProjectileSpawnRequest, ProjectileStart};
use crate::actor::BodyKinematics;
use crate::features::{ActorAggression, ActorFaction, BreakableFeature, CenteredAabb, FeatureId, FeatureSimEntity};
use ambition_combat::events::{HitEvent, HitKnockback, HitKnockbackMagnitude, HitMode, HitSource, HitTarget};
use ambition_projectiles::ProjectileGameplay;
use crate::trace::GameplayTraceBuffer;
use ambition_boss_encounter::{BossClusterRef, BossConfig};
use ambition_sfx::{SfxMessage, SfxWriter};
use ambition_vfx::vfx::VfxMessage;

/// Speed multiplier applied to a parried shot as it reverses — a timed parry
/// sends the bolt back a little faster than it arrived.
const PROJECTILE_REFLECT_SPEED_SCALE: f32 = 1.3;
/// Health a successful parry restores (a reason to parry rather than dodge).
const PARRY_HEAL: i32 = 1;

/// Reflect a parried projectile and transfer combat ownership to the parrier.
/// `ProjectileOwner` and `ProjectileAllegiance` change together so damage uses
/// the parrier's authority. Presentation source stays with the original shot,
/// while the parry clang uses the parrier's source: combat ownership and a
/// projectile's voice are separate facts.
fn reflect_parried_shot(
    commands: &mut Commands,
    proj_entity: Entity,
    kin: &mut BodyKinematics,
    parrier: Entity,
    parrier_allegiance: ProjectileAllegiance,
    parrier_source: Option<&ambition_sfx::PresentationSourceId>,
    sfx: &mut SfxWriter,
    vfx: &mut MessageWriter<VfxMessage>,
) {
    commands
        .entity(proj_entity)
        .insert((ProjectileOwner(parrier), parrier_allegiance));
    kin.vel = -kin.vel * PROJECTILE_REFLECT_SPEED_SCALE;
    sfx.write_for_body(
        parrier_source,
        SfxMessage::Play {
            id: ambition_sfx::ids::WORLD_ROCK_HIT,
            pos: kin.pos,
        },
    );
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

/// Per-body charge/motion-recognition/fire input for charge-capable bodies.
/// Emits [`ProjectileSpawnRequest`]; [`step_projectiles`] owns flight.
#[allow(clippy::too_many_arguments)]
pub fn charge_projectile_input(
    world_time: Res<ambition_time::WorldTime>,
    // Per-BODY projectile state lives on the charge-capable body itself. Iterates
    // every such body so co-op / possession builds get one independent charge timer.
    mut charge_body_q: Query<
        (
            Entity,
            &crate::actor::BodyKinematics,
            &ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame,
            &mut ambition_projectiles::PlayerProjectileState,
            Option<&mut crate::actor::BodyAnimFacts>,
        ),
        With<ambition_characters::brain::ChargesProjectiles>,
    >,
    mut brain_actions: MessageReader<ambition_characters::brain::ActorActionMessage>,
    user_settings: Res<ambition_persistence::settings::UserSettings>,
    // The open, content-owned motion-technique registry. The named gesture
    // patterns (qcf / qcf_grace / hcf) live in content, not this crate; the fire
    // policy below asks the catalog whether each fired.
    technique_catalog: Res<ambition_projectiles::MotionTechniqueCatalog>,
    mut trace: ResMut<GameplayTraceBuffer>,
    // Firing emits a next-tick `ProjectileSpawnRequest`; its materializer runs after this
    // system so newly-fired projectiles first tick next frame.
    mut spawn_projectiles: MessageWriter<ProjectileSpawnRequest>,
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
    for (body_entity, kin, resolved_frame, mut state, mut anim) in &mut charge_body_q {
        let tick_info = tick_infos.get(&body_entity).copied().unwrap_or_default();
        state.clock += dt;
        state.spawner.tick(dt);

        // Motion input uses the same +Y-down convention as `MotionDirection::from_axis`.
        let dir =
            ambition_projectiles::MotionDirection::from_axis(tick_info.axis.x, tick_info.axis.y, 0.55);
        let now = state.clock;
        state.motion_buffer.push(dir, now);

        let mut events: Vec<ProjectileTraceEvent> = Vec::new();

        let facing = if kin.facing.abs() < f32::EPSILON {
            1.0
        } else {
            kin.facing.signum()
        };
        // The firing body's per-tick resolved frame (ADR 0024 frame law).
        let frame = resolved_frame.basis();
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
            let super_qcf = technique_catalog.detect("qcf", &state.motion_buffer);
            let half_circle = technique_catalog.detect("hcf", &state.motion_buffer);
            let grace_qcf = technique_catalog.detect("qcf_grace", &state.motion_buffer);

            let motion_kind = if (super_qcf.is_some() || half_circle.is_some())
                && state.unlocked.hadouken_super
            {
                Some(ambition_projectiles::ProjectileKind::HadoukenSuper)
            } else if grace_qcf.is_some() && state.unlocked.hadouken {
                Some(ambition_projectiles::ProjectileKind::Hadouken)
            } else {
                None
            };

            // Debug log on every fire-press so the player can see
            // exactly what the motion recognizer saw and why a given
            // press did or didn't upgrade to a Hadouken. Run with
            // `RUST_LOG=ambition_platformer2d_actor_monolith::projectile=info` (or
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
                    ambition_projectiles::ProjectileKind::Fireball,
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
/// next-tick [`ProjectileSpawnRequest`] on success. Shared by press and release
/// paths; returns whether the shoot animation should pulse.
#[allow(clippy::too_many_arguments)]
fn try_fire_projectile(
    state: &mut PlayerProjectileState,
    owner: Entity,
    kind: ambition_projectiles::ProjectileKind,
    origin: ae::Vec2,
    direction: ae::Vec2,
    damage_mult: f32,
    charge_tier: u8,
    events: &mut Vec<ProjectileTraceEvent>,
    spawn_projectiles: &mut MessageWriter<ProjectileSpawnRequest>,
) -> bool {
    match state
        .spawner
        .try_spawn(kind, origin, direction, damage_mult)
    {
        Ok(spec) => {
            let spec = kind.charged_spec(spec, charge_tier);
            spawn_projectiles.write(ProjectileSpawnRequest::named(
                owner,
                ambition_projectiles::InFlightProjectile {
                    body: ambition_projectiles::ProjectileBody::from_spec(spec),
                },
                kind,
                ProjectileStart::StepNextTick,
            ));
            events.push(ProjectileTraceEvent::Fired { kind });
            true
        }
        Err(ambition_projectiles::SpawnFailure::OutOfResource) => {
            events.push(ProjectileTraceEvent::BlockedByResource { kind });
            false
        }
        Err(ambition_projectiles::SpawnFailure::Cooldown) => false,
    }
}

/// Flattened view of the `PlayerProjectileTick` request — used inside
/// `step_projectiles` after destructuring the matched
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

/// Set containing the unified in-flight projectile step for all allegiances.
/// Shots are processed in global `ProjectileSeq` order; allegiance determines damage,
/// parry, and solid-contact policy.
#[allow(clippy::too_many_arguments)]
/// Ordering anchor for the in-flight projectile step only.
#[derive(bevy::prelude::SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ProjectileStepSet;

/// Step every live projectile in deterministic spawn order.
/// Player/enemy routing shares this body-general path; bosses and breakables use the feature-hit path.
pub fn step_projectiles(
    mut commands: Commands,
    world_time: Res<ambition_time::WorldTime>,
    carved: ambition_projectiles::collision_world::ProjectileCollisionWorld,
    gravity: ambition_platformer2d_shared_tangle::gravity::GravityCtx,
    mut projectiles: Query<
        (
            Entity,
            &mut BodyKinematics,
            &mut ProjectileGameplay,
            Option<&ProjectileOwner>,
            // `None` is reserved for a genuinely ownerless/environmental volley.
            Option<&ProjectileAllegiance>,
            &ProjectileSeq,
            Option<&ambition_projectiles::ProjectileKind>,
            Option<&ambition_projectiles::ProjectileVisualId>,
            // G1: the firer's presentation source, stamped on the bolt at spawn.
            // Read from the PROJECTILE rather than chased back through the owner,
            // because a shot outlives its firer and must still sound like it.
            Option<&ambition_sfx::BodyPresentationSource>,
        ),
        (
            With<LiveProjectile>,
            Without<crate::actor::PlayerEntity>,
            Without<FeatureSimEntity>,
        ),
    >,
    // One shared body-victim query for hostile shots and parry. Player identity affects
    // reward/payload policy only; bosses use the separate encounter hit path below.
    // No vulnerability `With` filter: simple factioned hurtbox bodies are valid victims too.
    victims: Query<
        ambition_combat::hitbox::StrikeVictim,
        (Without<LiveProjectile>, Without<BossConfig>),
    >,
    // The victims' guards, taken mutably by entity: a parried shot CATCHES on
    // the shield, which is a write. `StrikeVictim` deliberately carries no
    // shield for this reason — see its own note — so this is the system's only
    // access to the component.
    mut guards: Query<&mut ae::BodyShieldState, Without<LiveProjectile>>,
    mut feature_damage: MessageWriter<HitEvent>,
    ecs_breakables: Query<(&FeatureId, &CenteredAabb, &BreakableFeature), With<FeatureSimEntity>>,
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
    mut sfx: SfxWriter,
    mut vfx: MessageWriter<VfxMessage>,
    mut heals: MessageWriter<crate::avatar::PlayerHealRequested>,
    mut trace: ResMut<GameplayTraceBuffer>,
    // Damage authority comes from the firer's faction/grudge/team. Match team outranks
    // faction for whether a shot may land; bosses use the authored catalog below.
    tuning: Option<Res<ambition_combat::rules::ResolvedCombatTuning>>,
    // Bundled into one SystemParam slot to stay under Bevy's parameter ceiling.
    (owner_combat, boss_catalog, visual_catalog): (
        Query<(
            &ActorFaction,
            Option<&ActorAggression>,
            Option<&ambition_combat::targeting::MatchTeam>,
        )>,
        Res<ambition_boss_encounter::BossCatalog>,
        Res<ambition_projectiles::ProjectileVisualCatalog>,
    ),
) {
    let dt = world_time.sim_dt();
    let friendly_fire = tuning.map(|t| t.friendly_fire()).unwrap_or_default();
    let collision_world = carved.solids();
    // Without the portal capability there are no apertures to thread, so the
    // list is empty by construction and the transit check below is skipped.
    #[cfg(feature = "portal")]
    let portal_list = carved.portal_list();
    let tick = trace.current_tick();

    // Collect + sort by the GLOBAL spawn sequence (a single deterministic order
    // across both factions; the seq counter is shared at spawn).
    let mut ordered: Vec<(Entity, ProjectileSeq)> = projectiles
        .iter()
        .map(|(entity, _, _, _, _, seq, _, _, _)| (entity, *seq))
        .collect();
    ordered.sort_by_key(|(_, seq)| *seq);

    for (proj_entity, _) in ordered {
        let Ok((_, mut kin, mut game, owner, stamped_allegiance, _, kind, visual_id, bolt_source)) =
            projectiles.get_mut(proj_entity)
        else {
            continue;
        };
        let stamped_allegiance = stamped_allegiance.cloned();
        let bolt_source = bolt_source.map(|source| source.id().clone());
        // Named kind when the shot uses the named vocabulary; open-visual
        // volleys deliberately remain kind-less.
        let kind = kind.copied();
        // Open visual id (every spawned shot carries one; empty reads as the
        // generic look). Drives the detonation FX pick via the content-owned
        // catalog; ownership and allegiance are separate facts.
        let expiry_burst = visual_id
            .map(|v| v.0.as_str())
            .and_then(|id| visual_catalog.get(id))
            .and_then(|art| art.expiry_vfx);
        let owner_entity = owner.map(|o| o.0);
        let owner_combat_data = owner_entity.and_then(|e| owner_combat.get(e).ok());
        // Projectile allegiance is frozen launch authority and rollback state. Parry may
        // deliberately rewrite it; it must not be re-derived from a firer that may be gone.
        let allegiance: Option<ProjectileAllegiance> = match stamped_allegiance {
            Some(stamped) => Some(stamped),
            // First sight. A shot always takes flight while its firer lives, so
            // this is the tick the fact is still there to be frozen.
            None => {
                let fresh = owner_combat_data.map(|(faction, _, team)| ProjectileAllegiance {
                    faction: *faction,
                    team: team.cloned(),
                });
                if let Some(fresh) = fresh.clone() {
                    commands.entity(proj_entity).insert(fresh);
                }
                fresh
            }
        };
        // A grudge is live firer state, not launch provenance, so resolve it
        // from the current owner. If grudges gain a lifetime beyond the firer,
        // stamp the target's `SimId` on the projectile rather than an `Entity`.
        let firer_grudge: Option<Entity> = owner_combat_data
            .and_then(|(_, agg, _)| agg)
            .and_then(|a| a.grudge);
        // Only a projectile with no named owner is indiscriminate. A temporarily
        // unresolved named owner must not turn a team-owned shot into a hazard.
        let indiscriminate = allegiance.is_none() && owner_entity.is_none();

        // Tick + lifetime. A dead lasersword detonates; everything else logs an
        // Expired trace event.
        // A projectile is a FREE body (not a kernel body): resolve its gravity
        // inline by the body-overlap rule, not the center point (ADR 0024).
        let gravity_dir = gravity.dir_for(kin.aabb());
        if !game.tick(&mut kin, dt, gravity_dir) {
            if let Some(boom) = expiry_burst.map(|b| b.to_message(kin.pos)) {
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
        #[cfg(feature = "portal")]
        if !portal_list.is_empty()
            && ambition_projectiles::try_projectile_portal_transit(&mut kin, &portal_list)
        {
            continue;
        }

        // Damage routed by the FIRER's real faction (the owner's), not a label on
        // the shot: a shot lands on a faction-foe, on a same-faction body its
        // firer holds a grudge against, or — if it is OWNERLESS and therefore
        // indiscriminate — on everyone, because there is no ally to spare.
        //  ONE victim loop, whoever fired. The player is not a special
        // case; it is a body that happens to carry `PlayerEntity`. This mirrors
        // the unified melee victim loop (`ambition_combat::hitbox`, §A2): the
        // SAME relational rule (`damage_lands`), the SAME published hurtbox,
        // i-frames resolved at CONSUME time, and victim KIND picking only
        // payload policy (the player's parry-heal reward).
        //
        // So the same bolt, fired by an enemy, named its victim and carried knockback; fired by the
        // player it named nobody and carried none, skipped the published silhouette, could not be
        // PARRIED, and never asked about a grudge. Four rules that only existed on one side of a
        // fork whose whole content was who pulled the trigger.
        {
            let mut struck = false;
            let mut reflected = false;
            for victim in &victims {
                if Some(victim.entity) == owner_entity {
                    continue;
                }
                // An owned shot lands on a faction-foe OR a same-faction body its firer holds a
                // grudge against; an indiscriminate (ownerless) shot lands on everyone — there is
                // no ally to spare. `damage_lands_BETWEEN`, so the TEAMS decide when both bodies
                // are seated and the factions decide when they are not.
                let can_hit = indiscriminate
                    || allegiance.as_ref().is_some_and(|side| {
                        ambition_combat::targeting::damage_lands_between(
                            side.faction,
                            victim.effective_faction(),
                            side.team(),
                            victim.team,
                            friendly_fire,
                            firer_grudge,
                            victim.entity,
                        )
                    });
                if !can_hit {
                    continue;
                }
                let victim_body = victim.aabb.aabb();
                // Projectiles use the same published victim geometry as melee.
                // An empty `DamageableVolumes` is intangible; absence falls back to
                // the coarse body box inside `reached_by`.
                if !victim.reached_by(&kin.aabb().into()) {
                    continue;
                }
                // Parry: a timed shield RE-OWNS the shot to the parrier (its firer
                // faction becomes the parrier's next tick → it routes as that
                // body's own shot, back at its foes) and reverses + boosts its
                // velocity. Shared by every body; only the HEAL is player reward
                // policy. A body with no shield state simply cannot parry.
                // The SAME catch the melee strike seam resolves, from the
                // other route a strike arrives on: one fact, both roads.
                let parried = guards
                    .get_mut(victim.entity)
                    .map(|mut shield| {
                        let caught = shield.parrying();
                        if caught {
                            shield.catch_parry();
                        }
                        caught
                    })
                    .unwrap_or(false);
                if parried {
                    reflect_parried_shot(
                        &mut commands,
                        proj_entity,
                        &mut kin,
                        victim.entity,
                        // The parrier's side, written OVER the firer's: this is
                        // the parrier's bolt now. Authored faction, the same one
                        // the firer's stamp freezes, so the two sides of the
                        // re-own are the same question asked the same way.
                        ProjectileAllegiance {
                            faction: *victim.faction,
                            team: victim.team.cloned(),
                        },
                        victim.voice.map(ambition_sfx::BodyPresentationSource::id),
                        &mut sfx,
                        &mut vfx,
                    );
                    if victim.is_player {
                        heals.write(crate::avatar::PlayerHealRequested::new(PARRY_HEAL));
                    }
                    reflected = true;
                    break;
                }
                // CM8: vulnerability is no longer read here to MUTE feedback — the ONE
                // victim-side reaction fires only on a landed hit, so a dodged / parried /
                // i-framed hit is muted for free at consume time (`resolve_body_hit`).
                let side = victim.knockback_side();
                let knock_dir = (victim_body.center() - kin.pos).dot(side).signum();
                let knock_dir = if knock_dir.abs() < 0.001 {
                    1.0
                } else {
                    knock_dir
                };
                let impact_pos = ae::Vec2::new(
                    (victim_body.center().x + kin.pos.x) * 0.5,
                    (victim_body.center().y + kin.pos.y) * 0.5,
                );
                feature_damage.write(HitEvent {
                    strike_sfx: None,
                    volume: kin.aabb().into(),
                    damage: game.damage.max(1),
                    source: HitSource::Projectile,
                    // The firing actor (enemy / boss), when the shot was spawned
                    // with a real owner — `None` for ownerless shots.
                    attacker: owner_entity,
                    // The victim, named — no producer-side classification.
                    target: HitTarget::Body(victim.entity),
                    mode: HitMode::Knockback,
                    knockback: Some(HitKnockback {
                        // An ordinary hit: it stuns.
                        reaction: ambition_platformer2d_core::hit_response::HitReaction::Strike,
                        dir: knock_dir,
                        magnitude: HitKnockbackMagnitude::FeelScale(0.85),
                        source_pos: kin.pos,
                        impact_pos,
                        launch_dir: None,
                        follow: None,
                    }),
                    ignored_targets: Vec::new(),
                });
                // CM8: the struck body's feedback (sound + spray) is emitted by
                // the ONE victim-side reaction now — a player victim through
                // `apply_player_hit_events`, an actor through `apply_actor_hit`.
                // Emitting here too would double the cue.
                struck = true;
                break;
            }
            // A parried shot survives as the parrier's bolt (keep it in flight).
            if reflected {
                continue;
            }
            if struck {
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

            // Bodies were resolved directly above. `UnresolvedFeatures` sends only the remaining
            // boss/breakable portion through feature resolution, avoiding a second body hit.
            let unresolved = HitEvent {
                strike_sfx: None,
                volume: kin.aabb().into(),
                damage: game.damage.max(1),
                source: HitSource::Projectile,
                attacker: owner_entity,
                target: HitTarget::UnresolvedFeatures,
                mode: HitMode::Knockback,
                knockback: None,
                ignored_targets: Vec::new(),
            };
            let reaches_feature =
                crate::features::ecs_hit_event_hits_breakable(&unresolved, &ecs_breakables)
                    || crate::features::ecs_hit_event_hits_boss(
                        &boss_catalog,
                        &unresolved,
                        &ecs_bosses,
                    );
            if reaches_feature {
                feature_damage.write(unresolved);
                // CM8: no attacker-side hit sound here — the struck feature's own
                // victim consumer (the boss reaction, or the breakable's Impact /
                // shatter) owns the cue, so a projectile plink is consistent with
                // a melee plink instead of playing an extra Hit.
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
                sfx.write_for_body(bolt_source.as_ref(), SfxMessage::Hit { pos });
            }
            WorldHitOutcome::Expired { pos } => {
                match expiry_burst.map(|b| b.to_message(pos)) {
                    Some(boom) => {
                        vfx.write(boom);
                        sfx.write_for_body(
                            bolt_source.as_ref(),
                            SfxMessage::Play {
                                id: ambition_sfx::ids::WORLD_EXPLOSION,
                                pos,
                            },
                        );
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
        mut sfx: SfxWriter,
        mut vfx: MessageWriter<VfxMessage>,
        sources: Query<&ambition_sfx::BodyPresentationSource>,
        mut shots: Query<(Entity, &mut BodyKinematics)>,
    ) {
        let parrier_source = sources.get(parrier.0).ok().map(|s| s.id().clone());
        for (proj, mut kin) in &mut shots {
            reflect_parried_shot(
                &mut commands,
                proj,
                &mut kin,
                parrier.0,
                ProjectileAllegiance {
                    faction: ActorFaction::Player,
                    team: None,
                },
                parrier_source.as_ref(),
                &mut sfx,
                &mut vfx,
            );
        }
    }

    /// H7: a reflected shot keeps its firer's voice; the clang is the parrier's.
    ///
    /// The re-own is a COMBAT fact — damage routes off the parrier's faction from the next
    /// tick.
    ///
    /// The choice: a fireball does not become a different fireball because somebody
    /// swatted it back, so its impact keeps sounding like whoever made it. Parrying,
    /// on the other hand, is the parrier's own technique, so the clang is theirs.
    #[test]
    fn a_reflected_shot_keeps_its_firers_voice_and_the_clang_is_the_parriers() {
        let mut app = App::new();
        app.add_message::<ambition_sfx::OwnedSfxMessage>();
        app.add_message::<VfxMessage>();
        let parrier = app
            .world_mut()
            .spawn(ambition_sfx::BodyPresentationSource(
                ambition_sfx::PresentationSourceId::new("mary_o_demo"),
            ))
            .id();
        app.world_mut().spawn((
            BodyKinematics {
                pos: ae::Vec2::ZERO,
                vel: ae::Vec2::new(100.0, 0.0),
                size: ae::Vec2::new(8.0, 8.0),
                facing: 1.0,
            },
            ambition_sfx::BodyPresentationSource(ambition_sfx::PresentationSourceId::new(
                "sanic_demo",
            )),
        ));
        app.insert_resource(Parrier(parrier));
        app.add_systems(Update, reflect_the_shot);
        app.update();

        let clang_sources: Vec<String> = app
            .world()
            .resource::<bevy::ecs::message::Messages<ambition_sfx::OwnedSfxMessage>>()
            .iter_current_update_messages()
            .map(|message| message.source.as_str().to_string())
            .collect();
        assert_eq!(
            clang_sources,
            vec!["mary_o_demo".to_string()],
            "the parry is the PARRIER's technique, so the clang is their cue"
        );

        let mut shots = app
            .world_mut()
            .query::<(&ProjectileOwner, &ambition_sfx::BodyPresentationSource)>();
        let world = app.world();
        let rows: Vec<(Entity, String)> = shots
            .iter(world)
            .map(|(owner, source)| (owner.0, source.id().as_str().to_string()))
            .collect();
        assert_eq!(
            rows,
            vec![(parrier, "sanic_demo".to_string())],
            "combat ownership moved to the parrier and the bolt's VOICE did not — \
             the shot still impacts in the voice of whoever fired it"
        );
    }

    /// A parry re-owns the projectile to the parrying body and reverses/boosts velocity.
    #[test]
    fn reflect_re_owns_the_shot_to_the_parrier_and_reverses_velocity() {
        let mut app = App::new();
        app.add_message::<ambition_sfx::OwnedSfxMessage>();
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
        // The shot's allegiance is a stamp it carries, so the re-own has to overwrite it
        // deliberately
        assert_eq!(
            world.get::<ProjectileAllegiance>(proj),
            Some(&ProjectileAllegiance {
                faction: ActorFaction::Player,
                team: None,
            }),
            "the parry REWRITES the shot's side; a re-own that only moved the \
             entity handle would leave it fighting for whoever fired it"
        );
        let kin = world.get::<BodyKinematics>(proj).unwrap();
        assert_eq!(
            kin.vel,
            ae::Vec2::new(-100.0, 40.0) * PROJECTILE_REFLECT_SPEED_SCALE,
            "velocity reversed and boosted by the reflect scale"
        );
    }
}
