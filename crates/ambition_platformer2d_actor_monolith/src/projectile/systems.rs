//! The Bevy gameplay system that ticks projectiles, samples motion
//! input, and routes hits through ECS-native feature damage messages.

use ambition_platformer2d_core as ae;
use ambition_platformer2d_core::AabbExt;
use bevy::prelude::*;

use super::allegiance::ProjectileAllegiance;
use super::diagnostics::log_press_diagnostics;
use super::entity::{LiveProjectile, ProjectileOwner, ProjectileOwnerId, ProjectileSeq};
use super::spawn_message::{ProjectilePool, SpawnProjectile};
use super::state::{PlayerProjectileState, ProjectileTraceEvent};
use super::{resolve_world_collision, WorldHitOutcome};
use crate::actor::BodyKinematics;
use ambition_boss_encounter::{BossClusterRef, BossConfig};
use crate::features::{
    ActorAggression, ActorFaction, BreakableFeature, CenteredAabb, FeatureId, FeatureSimEntity,
    HitEvent, HitKnockback, HitKnockbackMagnitude, HitMode, HitSource, HitTarget,
};
use crate::projectile::ProjectileGameplay;
use crate::trace::GameplayTraceBuffer;
use ambition_sfx::{SfxMessage, SfxWriter};
use ambition_vfx::vfx::VfxMessage;

/// Speed multiplier applied to a parried shot as it reverses — a timed parry
/// sends the bolt back a little faster than it arrived.
const PROJECTILE_REFLECT_SPEED_SCALE: f32 = 1.3;
/// Health a successful parry restores (a reason to parry rather than dodge).
const PARRY_HEAL: i32 = 1;

/// Body-generic projectile PARRY reflect: a timed shield RE-OWNS the shot to the
/// parrying body — both halves of that, the owner HANDLE and the
/// [`ProjectileAllegiance`] the damage rule actually reads — and reverses+boosts
/// the velocity. Re-owning is how a reflected shot becomes the parrier's attack
/// now that damage is attribution-driven. The SAME mechanic
/// for the player and any shielding actor (a possessed body, a mixed-faction
/// duelist); the player's parry HEAL stays a player-facing reward at the call site
/// (fable review 2026-07-02 §A10).
///
/// # Combat ownership moves; the bolt's VOICE does not (H1/H7)
///
/// ⭐ **the allegiance is REWRITTEN, not dropped.** The shot's side is a stamp it
/// carries (D150), so a parry is the one thing entitled to overwrite it: this is
/// the parrier's bolt now. Before the stamp existed the re-own worked by making
/// the next tick's owner LOOKUP find a different body, which is the same mechanism
/// that lost the shot's allegiance entirely when a firer died.
///
/// The re-own above is a combat fact: damage now routes off the parrier's faction.
/// It deliberately does NOT re-stamp `BodyPresentationSource`, so the shot's impact
/// still sounds like the character that FIRED it. Those are two different questions
/// — the same split `AudioContextOwner` and `PresentationSourceId` already draw —
/// and a fireball does not become a different fireball because somebody swatted it
/// back. What changes hands is who it is aimed at.
///
/// The CLANG is the other side of that: parrying is the parrier's technique, so the
/// reflect cue takes the parrier's source. Pinned by
/// `parry_tests::a_reflected_shot_keeps_its_firers_voice_and_the_clang_is_the_parriers`,
/// because the policy is a choice and not a consequence.
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
            &crate::physics::ResolvedMotionFrame,
            &mut crate::projectile::PlayerProjectileState,
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
    for (body_entity, kin, resolved_frame, mut state, mut anim) in &mut charge_body_q {
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
                Some(crate::projectile::ProjectileKind::HadoukenSuper)
            } else if grace_qcf.is_some() && state.unlocked.hadouken {
                Some(crate::projectile::ProjectileKind::Hadouken)
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
/// **The set `step_projectiles` runs in.**
///
/// ⛔ `ambition_platformer2d_host` orders two RENDER passes against this function
/// by name — `sync_projectile_visuals` and `sync_projectile_charge_visuals`, both
/// `.after(projectile_schedule::step_projectiles)` — so presentation reaches
/// through the runtime's re-export into a monolith leaf to place itself. The
/// comment beside them says why the edge is real ("both after the step so a
/// projectile fired this frame is visible this frame rather than one frame
/// late"); what was missing was a name to hang it on.
///
/// ⚠ ONE member. The two systems beside it in the tuple —
/// `charge_projectile_input` and `apply_player_spawn_projectile_messages` — are
/// deliberately AFTER the step ("so the new body first ticks next frame"), so a
/// set spanning them would push presentation past a spawn it is not waiting for.
#[derive(bevy::prelude::SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ProjectileStepSet;

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
            // D150: the shot's OWN side of the fight, stamped the first tick it
            // flies and read every tick after. `None` only for a bolt that has
            // never had a living owner to freeze — a genuinely ownerless volley.
            Option<&ProjectileAllegiance>,
            Option<&ProjectileOwnerId>,
            &ProjectileSeq,
            Option<&crate::projectile::ProjectileKind>,
            Option<&crate::projectile::ProjectileVisualId>,
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
    // Read-only victim bodies for hostile-shot damage + parry. Disjoint from the
    // mutable projectile query above (both touch `BodyKinematics`; B0001) via the
    // `LiveProjectile` marker split.
    //
    // ONE victim set for hostile shots — every body, player included. The player
    // is selected by `is_player` for PAYLOAD POLICY only (routing stamp, parry
    // heal), never by a separate query or a separate loop. Bosses are excluded
    // because the boss-facing hit path is `ecs_bosses` below; including them would
    // double-damage.
    //
    // ⭐ **Now the SAME NAMED ROLE melee uses** — [`StrikeVictim`], owned by
    // `ambition_combat::hitbox` beside the victim-geometry rule. The comment here
    // used to claim this loop shared "the SAME published hurtbox" as melee; it did
    // not, because the tuple that would have carried `DamageableVolumes` had run
    // out of arity and the claim was never anything but prose. Sharing the type
    // makes the claim checkable — and it exposed a bolt landing on a body a sword
    // passes through. The INTANGIBILITY half of that is closed in the loop below;
    // the precision half is still open, and says so there.
    //
    // ⚠ NO `With` filter on the vulnerability cluster, deliberately, and unlike
    // melee: a shot must be able to hit any body with a hurtbox and a faction,
    // including a simple feature body carrying no shield/dodge state at all.
    // Narrowing here would silently drop those bodies (the required-components-skip
    // trap), which is exactly how a "my projectile does nothing" bug is born.
    victims: Query<
        ambition_combat::hitbox::StrikeVictim,
        (Without<LiveProjectile>, Without<BossConfig>),
    >,
    mut feature_damage: MessageWriter<HitEvent>,
    ecs_breakables: Query<(&FeatureId, &CenteredAabb, &BreakableFeature), With<FeatureSimEntity>>,
    // ⛔ **the actor hit PREDICTION is gone with the fork that needed it.** It
    // existed so a Player-faction shot could decide whether its `Volume`
    // broadcast would land on anything before emitting one; the victim loop
    // names those bodies directly now, and re-asking here would be the second
    // rule this file just finished deleting. `ecs_actors` went with it — one
    // fewer system param, and one fewer place for "does this hit an actor" to
    // grow a second answer.
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
    // Relational damage authority + non-player actor victims for actor-vs-actor
    // projectile damage. A shot damages any DIFFERENT-faction body it hits, routed
    // off the FIRER's faction (looked up from its owner entity): a PCA (Enemy)
    // glider hits a robot (Boss) and vice versa, and a stray hits a different-faction
    // bystander (the observer). Same-faction allies are spared unless friendly fire
    // is on — so a pirate's shot can't hit another pirate. (Targeting is separate.)
    // AE6: resolved match rules, not the world's baseline toggle.
    tuning: Option<Res<ambition_combat::rules::ResolvedCombatTuning>>,
    // Still bundled into ONE tuple slot to stay under Bevy's 16-param ceiling, but
    // one member SHORTER: `victim_frames` left, because a per-victim component
    // belongs to the victim view rather than to a lookup query beside it.
    // - `owner_combat` — the firer's REAL faction, optional grudge and MATCH TEAM,
    //   looked up from the projectile's owner entity (player / enemy / boss /
    //   player-robot). The faction RETIRES the binary `ProjectileGameplay.faction`
    //   (damage routes off the owner, not a side label); the grudge is the
    //   per-entity DAMAGE override that lets a shot hit a same-faction body its
    //   firer feuds with (an `Npc` duelist's bolt). Read-only, so it may overlap
    //   `victims`.
    //
    //   ⛔⛔ **THE TEAM JOINED IT 2026-08-16, and its absence was Jon's report**:
    //   *"PCA's glider doesn't do any damage or hit anyone."* Measured on the
    //   shipped stage, both seats come back `ActorFaction::Player` with teams
    //   `seat 1` and `seat 2` — melee asks `team_allows_damage` and lands, this
    //   loop asked `damage_lands` and spared every shot as an ally. It was never
    //   about the glider: NO projectile from ANY fighter could hit anybody on a
    //   crossover grid, because a Hall NPC and a demo protagonist are not
    //   enemies of each other outside the match. `StrikeVictim` has carried the
    //   victim's `team` the whole time — *"Outranks faction for 'may this
    //   land'"* — and this was the one caller that never asked for it.
    // - `boss_catalog` — App-local authored boss geometry used by the hit predicate.
    // - `visual_catalog` — the open, content-owned projectile art registry; the
    //   detonation-FX pick resolves a shot's visual id through it.
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
    let portal_list = carved.portal_list();
    let tick = trace.current_tick();

    // Collect + sort by the GLOBAL spawn sequence (a single deterministic order
    // across both factions; the seq counter is shared at spawn).
    let mut ordered: Vec<(Entity, ProjectileSeq)> = projectiles
        .iter()
        .map(|(entity, _, _, _, _, _, seq, _, _, _)| (entity, *seq))
        .collect();
    ordered.sort_by_key(|(_, seq)| *seq);

    for (proj_entity, _) in ordered {
        let Ok((
            _,
            mut kin,
            mut game,
            owner,
            stamped_allegiance,
            _owner_id,
            _,
            kind,
            visual_id,
            bolt_source,
        )) = projectiles.get_mut(proj_entity)
        else {
            continue;
        };
        let stamped_allegiance = stamped_allegiance.cloned();
        let bolt_source = bolt_source.map(|source| source.id().clone());
        // Named kind for player shots (None for kind-less enemy volleys).
        let kind = kind.copied();
        // Open visual id (every spawned shot carries one; empty reads as the
        // generic hostile look). Drives the detonation FX pick via the
        // content-owned catalog — by id, not by sniffing the owner-id string.
        let expiry_burst = visual_id
            .map(|v| v.0.as_str())
            .and_then(|id| visual_catalog.get(id))
            .and_then(|art| art.expiry_vfx);
        let owner_entity = owner.map(|o| o.0);
        let owner_combat_data = owner_entity.and_then(|e| owner_combat.get(e).ok());
        // **The shot's side, CARRIED BY THE SHOT** (D150). Stamped the first tick
        // this bolt is stepped and read on every tick after, so the answer stops
        // depending on whether the firer is still resident.
        //
        // ⛔⛔ **it used to be re-derived every tick from the owner ENTITY, and a
        // miss was read as "ownerless" — which this loop treats as INDISCRIMINATE,
        // "there is no one to be friendly to".** That is the right reading for an
        // environmental volley and a disaster for a fighter's bolt: a stocks
        // ruleset DESPAWNS a fighter who spends their last stock
        // (`take_eliminated_fighters_out_of_play`), so the tick after they lost,
        // their shot in flight turned on their own team. A shot does not become
        // neutral because the body that fired it stopped being resident.
        //
        // ⚠ **freezing is not memoising a lookup** — the stamp is the AUTHORITY
        // from here on, which is what lets a parry deliberately REWRITE it
        // (`reflect_parried_shot`) instead of reaching for whoever the owner
        // handle now points at. It is registered rollback state for the same
        // reason: after a rewind past the firer's death there is nothing left to
        // re-derive it from.
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
        // The firer's personal grudge — the per-entity damage override (a duelist's
        // shot lands on the rival it feuds with even at the same faction).
        //
        // ⚠ deliberately NOT frozen onto the shot beside the allegiance, and the
        // reasoning was AUDITED 2026-08-18 rather than inherited. A grudge is a
        // feud the firer holds *now*, not a side the shot was launched on, and
        // `dissolve_settled_grudges` already gives it a semantic end (either body
        // reaching zero health) that has nothing to do with residency. So the
        // live read is the right read.
        //
        // ⛔ **but it was only defensible once the line below stopped inverting.**
        // A missing owner made the shot INDISCRIMINATE, so "the firer is gone, so
        // there is no feud" silently became "the firer is gone, so hit everyone
        // including the people the feud was meant to spare" — the grudge's
        // narrowing turned into the broadest possible permission. With
        // `indiscriminate` now requiring that no owner was ever NAMED, losing the
        // grudge costs the shot one same-faction target and nothing else, which
        // is what dropping a feud should cost.
        //
        // ⇒ **not stamped, on purpose, and that is a DECISION with a condition
        // attached**: it holds while the grudge's own lifecycle is health-keyed.
        // If a grudge ever becomes something a body can hold past death, or the
        // launch itself starts meaning "I aimed this AT you", the durable form
        // belongs on the shot — as the target's `SimId`, never an `Entity`
        // (N3.1 forbids entity handles in rollback blobs; see
        // `heal_projectile_owners` for the healed-handle pattern that costs).
        let firer_grudge: Option<Entity> = owner_combat_data
            .and_then(|(_, agg, _)| agg)
            .and_then(|a| a.grudge);
        // A truly ownerless volley — environmental damage that hurts every body
        // it overlaps, friend or foe, because there is no one to be friendly to.
        //
        // ⛔⛔ **THIS USED TO READ `allegiance.is_none()` ALONE, and that is a
        // different question wearing the same words.** The comment says "never
        // had an owner"; the expression said "the owner lookup came back empty",
        // and those diverge exactly when it matters. `owner_combat` requires a
        // non-optional `&ActorFaction`, so it returns `Err` for a NAMED owner
        // that is merely gone — or alive but factionless — and on the shot's
        // FIRST step that also means no stamp is taken, so the bolt stays
        // unstamped and re-asks (and re-fails) every tick for the rest of its
        // life. A named firer that could not be resolved was therefore promoted
        // to environmental hazard: hostile to its own team, permanently. That is
        // the D150 failure surviving inside the one window D150's stamp does not
        // cover — the tick before the stamp exists.
        //
        // ⇒ **an owner NAMED is the disqualifier, not an owner RESOLVED.**
        // `ProjectileOwner` is healed across a rewind from durable provenance,
        // so its presence is the stable fact here; what it points at may be
        // absent for a tick without changing whether this shot was somebody's.
        // A named-but-unresolved shot goes INERT rather than indiscriminate,
        // which is the safe direction: failing to damage is recoverable, hitting
        // your own team because a lookup missed is not.
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
        if !portal_list.is_empty()
            && crate::projectile::try_projectile_portal_transit(&mut kin, &portal_list)
        {
            continue;
        }

        // Damage routed by the FIRER's real faction (the owner's), not a label on
        // the shot: a shot lands on a faction-foe, on a same-faction body its
        // firer holds a grudge against, or — if it is OWNERLESS and therefore
        // indiscriminate — on everyone, because there is no ally to spare.
        // ⭐⭐ **ONE victim loop, whoever fired.** The player is not a special
        // case; it is a body that happens to carry `PlayerEntity`. This mirrors
        // the unified melee victim loop (`ambition_combat::hitbox`, §A2): the
        // SAME relational rule (`damage_lands`), the SAME published hurtbox,
        // i-frames resolved at CONSUME time, and victim KIND picking only
        // payload policy (the player's parry-heal reward).
        //
        // ⛔ **a Player-faction shot used to take a different road entirely** —
        // it broadcast `HitTarget::Volume` and let the legacy
        // "iterate-and-take-primary" consumer work out who it meant. So the same
        // bolt, fired by an enemy, named its victim and carried knockback; fired
        // by the player it named nobody and carried none, skipped the published
        // silhouette, could not be PARRIED, and never asked about a grudge. Four
        // rules that only existed on one side of a fork whose whole content was
        // who pulled the trigger.
        {
            let mut struck = false;
            let mut reflected = false;
            for victim in &victims {
                if Some(victim.entity) == owner_entity {
                    continue;
                }
                // An owned shot lands on a faction-foe OR a same-faction body its
                // firer holds a grudge against; an indiscriminate (ownerless) shot
                // lands on everyone — there is no ally to spare.
                // ⭐ `damage_lands_BETWEEN`, so the TEAMS decide when both
                // bodies are seated and the factions decide when they are not.
                // The plain `damage_lands` this used to call cannot see a match
                // at all — see the note on `owner_combat` above.
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
                // **Nothing published ⇒ nothing to hit.** A body carrying
                // `DamageableVolumes` with an EMPTY list has been spoken for and
                // offers no target — an authored invulnerable window, or a corpse
                // `refresh_body_damageable_volumes` cleared. Melee and feature
                // hits inherit that from `strike_reaches_victim`; this loop tested
                // the coarse box and asked nobody, so a bolt landed on (and was
                // eaten by) a body a sword passes straight through. Absence is
                // NOT emptiness: a body with no component at all has simply never
                // been published for, and keeps the coarse box below.
                if victim.is_intangible() {
                    continue;
                }
                // ⚠ **STILL THE COARSE BOX for a body that published a real
                // silhouette**, while melee and feature hits ask
                // `strike_reaches_victim` for the geometry too. That remaining
                // half of the gap is `victim.reached_by(&kin.aabb().into())`, and
                // it is deliberately NOT taken here: it retires
                // `strict_intersects` for projectiles (which rejects edge-touching
                // where the shared rule accepts it) and changes how every shot
                // connects, so it is a feel call and it is Jon's. Queue row D23.
                if !kin.aabb().strict_intersects(victim_body) {
                    continue;
                }
                // Parry: a timed shield RE-OWNS the shot to the parrier (its firer
                // faction becomes the parrier's next tick → it routes as that
                // body's own shot, back at its foes) and reverses + boosts its
                // velocity. Shared by every body; only the HEAL is player reward
                // policy. A body with no shield state simply cannot parry.
                if victim.shield.is_some_and(|shield| shield.parrying()) {
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
                // CM8: vulnerability is no longer read here to MUTE feedback —
                // the ONE victim-side reaction fires only on a landed hit, so a
                // dodged / parried / i-framed hit is muted for free at consume
                // time (`resolve_body_hit`). (`vuln` is still read for the parry /
                // shield reflect branch above.)
                // Knockback side in the victim's LOCAL frame (fable review
                // 2026-07-02 §B11): a screen-X difference degenerates exactly when
                // sideways gravity separates the pair along world-Y.
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
                    // EVERY victim rides the same resolved knockback (§A2 step 6).
                    // The actor branch used to pass `None`, so an actor struck by
                    // the very bolt that launched the player simply absorbed it.
                    knockback: Some(HitKnockback {
                        dir: knock_dir,
                        magnitude: HitKnockbackMagnitude::FeelScale(0.85),
                        source_pos: kin.pos,
                        impact_pos,
                        launch_dir: None,
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

            // **The strike's UNRESOLVED half**, exactly as a body-owned melee
            // publishes it. The loop above named every combat body this shot
            // reaches; it cannot name a breakable, or a boss whose HP and phase
            // live on an encounter rather than on a body carrying the combat
            // cluster — neither matches `StrikeVictim`, and the query excludes
            // `BossConfig` outright.
            //
            // ⛔ `UnresolvedFeatures`, NOT `Volume`: `Volume` means "scan
            // everything" and would damage every body a second time on top of
            // the identified hit it just took. The consumer skips its actor scan
            // on this target for that reason.
            //
            // ⚠ this is the half that used to be a Player-faction privilege. A
            // hostile bolt could not break a crate at all — not by policy, just
            // because the other side of the fork was the only one that looked.
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

    /// **H7: a reflected shot keeps its firer's voice; the clang is the parrier's.**
    ///
    /// The re-own is a COMBAT fact — damage routes off the parrier's faction from
    /// the next tick. It is not a presentation fact, and the two were easy to
    /// conflate now that a projectile carries a source at all (GPT 5.6 asked for
    /// whichever policy is intended to be pinned, since neither follows from the
    /// other).
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

    /// The body-generic parry reflect — the ONE mechanic the player parry and the
    /// new actor parry both call (§A10) — re-owns the shot to the parrying body and
    /// reverses + boosts its velocity, so a reflected shot becomes the parrier's own
    /// bolt (damage routes off the parrier's faction next tick) whether a player or
    /// a shielding actor caught it. Pins that a future edit can't make the reflect
    /// re-own to a hardcoded player again.
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
        // ⭐ **and the SIDE moves with the handle** (D150). The shot's allegiance
        // is a stamp it carries, so the re-own has to overwrite it deliberately —
        // before the stamp existed, changing the owner handle was the whole
        // mechanism, and that is the same mechanism that lost a shot's side when
        // its firer died.
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
