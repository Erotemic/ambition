//! The Bevy gameplay system that ticks projectiles, samples motion
//! input, and routes hits through ECS-native feature damage messages.

use ambition_breakables::BreakableFeature;
use ambition_platformer2d_core as ae;
use ambition_platformer2d_core::AabbExt;
use bevy::prelude::*;

use ambition_boss_encounter::BossConfig;
use ambition_characters::actor::ActorFaction;
use ambition_combat::allegiance::AttackAllegiance;
use ambition_combat::components::{
    ActorAggression, CenteredAabb,
    FeatureId};
use ambition_combat::events::{
    HitEvent, HitKnockback, HitKnockbackMagnitude, HitMode, HitSource, HitTarget,
};
use ambition_gameplay_trace::GameplayTraceBuffer;
use ambition_platformer2d_core::BodyKinematics;
use ambition_platformer2d_shared_tangle::lifecycle::FeatureSimEntity;
use ambition_projectiles::ProjectileGameplay;
use ambition_projectiles::diagnostics::log_press_diagnostics;
use ambition_projectiles::entity::{LiveProjectile, ProjectileOwner, ProjectileSeq};
use ambition_projectiles::state::{PlayerProjectileState, ProjectileTraceEvent};
use ambition_projectiles::{
    ProjectileSpawnRequest, ProjectileStart, WorldHitOutcome, query_world_collision,
};
use ambition_sfx::{SfxMessage, SfxWriter};
use ambition_vfx::vfx::VfxMessage;
use ambition_vfx::vfx::VfxWriter;

/// Speed multiplier applied to a parried shot as it reverses — a timed parry
/// sends the bolt back a little faster than it arrived.
const PROJECTILE_REFLECT_SPEED_SCALE: f32 = 1.3;
/// Health a successful parry restores (a reason to parry rather than dodge).
const PARRY_HEAL: i32 = 1;

/// Reflect a parried projectile and transfer combat ownership to the parrier.
/// `ProjectileOwner` and `AttackAllegiance` change together so damage uses
/// the parrier's authority. Presentation source stays with the original shot,
/// while the parry clang uses the parrier's source: combat ownership and a
/// projectile's voice are separate facts.
fn commit_parried_shot(
    commands: &mut Commands,
    proj_entity: Entity,
    kin: &mut BodyKinematics,
    response: &super::intercept::PreparedInterception,
    impact: ae::Vec2,
    parrier: Entity,
    parrier_allegiance: AttackAllegiance,
    parrier_source: Option<&ambition_sfx::PresentationSourceId>,
    // The live room the shot flies in: the parry impact is drawn there.
    room: Option<ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance>,
    sfx: &mut SfxWriter,
    vfx: &mut VfxWriter,
) -> bool {
    let survived=super::intercept::intercept_projectile(
        commands,proj_entity,kin,parrier,parrier_allegiance,response,
    );
    if !survived { return false; }
    sfx.in_space(ambition_vfx::sfx_space(room)).write_for_body(
        parrier_source,
        SfxMessage::Play {
            id: ambition_sfx::ids::WORLD_ROCK_HIT,
            emitter: impact.into(),
        },
    );
    vfx.write_in(room, VfxMessage::Impact { pos: impact, blow: ambition_vfx::ImpactBlow::PLAIN });
    true
}
/// Per-body charge/motion-recognition/fire input for charge-capable bodies.
/// Emits [`ProjectileSpawnRequest`]; [`step_projectiles`] owns flight.
#[allow(clippy::too_many_arguments)]
pub fn charge_projectile_input(
    births: ambition_projectiles::ProjectileBirth,
    world_time: Res<ambition_time::WorldTime>,
    // Per-BODY projectile state lives on the charge-capable body itself. Iterates
    // every such body so co-op / possession builds get one independent charge timer.
    mut charge_body_q: Query<
        (
            Entity,
            &ambition_platformer2d_core::BodyKinematics,
            &ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame,
            &mut ambition_projectiles::PlayerProjectileState,
            Option<&mut ambition_characters::actor::BodyAnimFacts>,
        ),
        With<ambition_characters::brain::ChargesProjectiles>,
    >,
    mut brain_actions: MessageReader<ambition_characters::brain::ActorActionMessage>,
    // The projected policy, not `Res<UserSettings>` — see `PlayerDamagePolicy`.
    damage_policy: Res<ambition_damage::PlayerDamagePolicy>,
    // The open, content-owned motion-technique registry. The named gesture
    // patterns (qcf / qcf_grace / hcf) live in content, not this crate; the fire
    // policy below asks the catalog whether each fired.
    technique_catalog: Res<ambition_projectiles::MotionTechniqueCatalog>,
    mut trace: ResMut<GameplayTraceBuffer>,
    // Where the firing hand is: see `ambition_combat::shot_birth`.
    landmarks: ambition_combat::body_landmarks::BodyLandmarks,
    // Firing emits a next-tick `ProjectileSpawnRequest`; its materializer runs after this
    // system so newly-fired projectiles first tick next frame.
    mut spawn_projectiles: MessageWriter<ProjectileSpawnRequest>,
    // A refused press says why (`Q33`), as the action road's refusal does.
    mut refused: MessageWriter<ambition_combat::RangedFireRefused>,
    // How the fireball flies in the firer's room: each game states its own.
    fireball_flight: ambition_platformer2d_world::governing_rules::RulesOf<ambition_projectiles::FireballFlight,
    >,
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
                intent,
            } => Some((
                msg.actor,
                PlayerProjectileTickInfo {
                    axis,
                    aim,
                    press,
                    held,
                    released,
                    intent,
                },
            )),
            _ => None,
        })
        .collect();

    let damage_mult = damage_policy.outgoing;
    for (body_entity, kin, resolved_frame, mut state, mut anim) in &mut charge_body_q {
        let tick_info = tick_infos.get(&body_entity).copied().unwrap_or_default();
        state.clock += dt;
        state.spawner.tick(dt);

        // Motion input uses the same +Y-down convention as `MotionDirection::from_axis`.
        let dir = ambition_projectiles::MotionDirection::from_axis(
            tick_info.axis.x,
            tick_info.axis.y,
            0.55,
        );
        let now = state.clock;
        state.motion_buffer.push(dir, now);

        let mut events: Vec<ProjectileTraceEvent> = Vec::new();

        let frame = resolved_frame.basis();
        let direction = match ambition_combat::shot_birth::world_fire_dir(kin, frame.to_world(tick_info.aim)) {
            Ok(direction) => direction,
            Err(fault) => { warn!(?fault, "charge aim refused"); continue; }
        };
        let local_dir = frame.to_local(direction);
        // The shot is born at the hand that fires, in the row the body fires
        // in. Asked only on a tick that fires. The shot's size is of its kind
        // and charge, so the place is too.
        let origin_of = |kind: ambition_projectiles::ProjectileKind, charge_tier: u8| {
            let half_extent = kind
                .charged_spec(kind.spec(ae::Vec2::ZERO, direction, 1.0), charge_tier)
                .half_extent;
            ambition_combat::shot_birth::charge_shot_birth(
                &landmarks,
                body_entity,
                kin,
                direction,
                half_extent,
            )
        };

        let to_world = |local: ae::Vec2| frame.to_world(local);
        let mut firing = Firing {
            owner: body_entity,
            origin_of: &origin_of,
            admit: &|body| births.prepare(body_entity, kin.pos, body),
            flight: fireball_flight.of(body_entity).copied(),
            local_dir,
            to_world: &to_world,
            direction,
            damage_mult,
            spawn: &mut spawn_projectiles,
            refused: &mut refused,
            fired: 0,
        };

        // A shot asked for in the cooldown fires on the first tick the
        // cooldown is over, or is refused when it has waited too long.
        if let Some(mut shot) = state.buffered.take() {
            match firing.fire(&mut state, &mut events, shot.kind, shot.charge_tier) {
                FireOutcome::Fired | FireOutcome::Geometry(_) => {}
                FireOutcome::Cooldown => {
                    shot.remaining -= dt;
                    if shot.remaining > 0.0 {
                        state.buffered = Some(shot);
                    } else {
                        firing.refuse(&state, shot.kind);
                    }
                }
                FireOutcome::OutOfResource => firing.refuse(&state, shot.kind),
            }
        }

        // Press edge: try Hadouken tiers first (most-specific motion gate
        // wins), else start charging a Fireball. Order matters — the
        // grace shape is a SUBSEQUENCE of the full QCF, so check Super
        // first; otherwise a 3-step input would fire a weak Hadouken.
        //
        // The press, the hold and the release are each read on their own: a
        // tap shorter than one tick arrives as a press and a release in the
        // same tick, and it is a shot.
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
            // `RUST_LOG=crate::projectile=info` (or
            // `RUST_LOG=info` more broadly) to surface these.
            log_press_diagnostics(
                &state.motion_buffer,
                super_qcf,
                half_circle,
                grace_qcf,
                motion_kind,
            );

            // A motion gesture fires (or waits for the cooldown) at once and
            // starts no charge. A gesture the meter cannot pay for is not a
            // lost press: the press charges the fireball, as a press with no
            // gesture does.
            let gesture_taken = motion_kind.is_some_and(|kind| {
                state.motion_buffer.clear();
                firing.fire_or_wait(&mut state, &mut events, kind, 0) != FireOutcome::OutOfResource
            });
            state.charging = (!gesture_taken && state.unlocked.fireball).then_some(0.0);
        } else if tick_info.held {
            if let Some(t) = state.charging.as_mut() {
                *t += dt;
            }
        }
        if tick_info.released {
            if let Some(hold) = state.charging.take() {
                let tier = state.charge_tuning.tier_for_hold(hold);
                firing.ask(&mut state, &mut events, ambition_projectiles::ProjectileKind::Fireball, tier,
                );
            }
        } else if tick_info.intent && state.charging.is_none() && state.unlocked.fireball {
            // An autonomous driver's "shoot" is a tap: an uncharged Fireball.
            // The spawner's refire cooldown paces a driver that asks every
            // tick, so it does not buffer.
            let kind = ambition_projectiles::ProjectileKind::Fireball;
            if firing.fire(&mut state, &mut events, kind, 0) == FireOutcome::OutOfResource {
                firing.refuse(&state, kind);
            }
        }
        let fired_this_frame = firing.fired;

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

/// One charge body's firing on one tick: where its shots are born, which way
/// they fly, and the two messages a press can end in.
struct Firing<'a, 's, 'r> {
    owner: Entity,
    origin_of: &'a dyn Fn(ambition_projectiles::ProjectileKind, u8) -> Result<ae::Vec2, ae::frame::NumericFault>,
    admit: &'a dyn Fn(ambition_projectiles::ProjectileBody) -> Result<ambition_projectiles::ProjectileBody, ae::frame::thread::Refusal>,
    /// How the fireball flies in the firer's room; `None` flies the kind's own.
    flight: Option<ambition_projectiles::FireballFlight>,
    /// The aim in the firer's frame, and that frame's way to the world.
    local_dir: ae::Vec2,
    to_world: &'a dyn Fn(ae::Vec2) -> ae::Vec2,
    direction: ae::Vec2,
    damage_mult: f32,
    spawn: &'a mut MessageWriter<'s, ProjectileSpawnRequest>,
    refused: &'a mut MessageWriter<'r, ambition_combat::RangedFireRefused>,
    /// Shots fired this tick: the shoot pose pulses on the firing tick.
    fired: u32,
}

impl Firing<'_, '_, '_> {
    /// Run spawn checks for `kind`, apply the Fireball charge tier, and emit
    /// a next-tick [`ProjectileSpawnRequest`] on success.
    fn fire(
        &mut self,
        state: &mut PlayerProjectileState,
        events: &mut Vec<ProjectileTraceEvent>,
        kind: ambition_projectiles::ProjectileKind,
        charge_tier: u8,
    ) -> FireOutcome {
        let origin = match (self.origin_of)(kind, charge_tier) {
            Ok(origin) => origin,
            Err(fault) => {
                let refusal = ae::frame::thread::Refusal { path: Vec::new(), link: None,
                    reason: ae::frame::thread::RefusalReason::Numeric(fault) };
                warn!(?refusal, "charge birth refused"); return FireOutcome::Geometry(refusal);
            }
        };
        let mut proposed_spawner = state.spawner;
        match proposed_spawner.try_spawn(kind, origin, self.direction, self.damage_mult) {
            Ok(spec) => {
                let flight = self.flight.filter(|_| kind == ambition_projectiles::ProjectileKind::Fireball);
                let spec = match flight {
                    Some(flight) => flight.fly(spec, self.local_dir, self.to_world),
                    None => spec,
                };
                let spec = kind.charged_spec(spec, charge_tier);
                let mut body = ambition_projectiles::ProjectileBody::from_spec(spec);
                if let Some(flight) = flight {
                    body.game.rebound = flight.rebound();
                }
                let body = match (self.admit)(body) {
                    Ok(body) => body,
                    Err(refusal) => { warn!(?refusal, "projectile materialization refused"); return FireOutcome::Geometry(refusal); }
                };
                state.spawner = proposed_spawner;
                self.spawn.write(
                    ProjectileSpawnRequest::named(
                        self.owner,
                        ambition_projectiles::InFlightProjectile { body },
                        kind,
                        ProjectileStart::StepNextTick,
                    )
                    .charged(charge_tier),
                );
                events.push(ProjectileTraceEvent::Fired { kind });
                self.fired += 1;
                FireOutcome::Fired
            }
            Err(ambition_projectiles::SpawnFailure::OutOfResource) => {
                events.push(ProjectileTraceEvent::BlockedByResource { kind });
                FireOutcome::OutOfResource
            }
            Err(ambition_projectiles::SpawnFailure::Cooldown) => FireOutcome::Cooldown,
        }
    }

    /// [`Self::fire`], and a shot the cooldown refuses waits in the buffer.
    fn fire_or_wait(
        &mut self,
        state: &mut PlayerProjectileState,
        events: &mut Vec<ProjectileTraceEvent>,
        kind: ambition_projectiles::ProjectileKind,
        charge_tier: u8,
    ) -> FireOutcome {
        let outcome = self.fire(state, events, kind, charge_tier);
        if outcome == FireOutcome::Cooldown {
            state.buffered = Some(ambition_projectiles::state::BufferedShot {
                kind,
                charge_tier,
                remaining: ambition_projectiles::state::FIRE_BUFFER_S,
            });
        }
        outcome
    }

    /// A player's ask for a shot: it fires, or waits in the buffer for the
    /// cooldown, or is refused because the meter cannot pay for it.
    fn ask(
        &mut self,
        state: &mut PlayerProjectileState,
        events: &mut Vec<ProjectileTraceEvent>,
        kind: ambition_projectiles::ProjectileKind,
        charge_tier: u8,
    ) {
        if self.fire_or_wait(state, events, kind, charge_tier) == FireOutcome::OutOfResource {
            self.refuse(state, kind);
        }
    }

    /// ⭐ THE REFUSAL IS A FACT (`Q33`): a refused press and no press must
    /// not look the same downstream. A failed `try_spawn` changes nothing, so
    /// the readiness read here is the readiness the press was refused on.
    fn refuse(&mut self, state: &PlayerProjectileState, kind: ambition_projectiles::ProjectileKind,
    ) {
        self.refused.write(ambition_combat::RangedFireRefused {
            actor: self.owner,
            readiness: ambition_combat::WeaponReadiness::of_spawner(&state.spawner, kind),
        });
    }
}

/// `request`, said from the shot's own presentation source when it has one.
fn said_by(
    request: ambition_vfx::vfx::FxRequest,
    source: Option<&ambition_sfx::PresentationSourceId>,
) -> ambition_vfx::vfx::FxRequest {
    match source {
        Some(source) => request.from_source(source.clone()),
        None => request,
    }
}

/// What one attempt to fire did.
#[derive(Clone, Debug, PartialEq, Eq)]
enum FireOutcome {
    Fired,
    Geometry(ae::frame::thread::Refusal),
    /// The spawner is cooling down. The caller decides whether the shot waits.
    Cooldown,
    /// The meter cannot pay for the shot.
    OutOfResource,
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
    intent: bool,
}

/// Set containing the unified in-flight projectile step for all allegiances.
/// Shots are processed in global `ProjectileSeq` order; allegiance determines damage,
/// parry, and solid-contact policy.
#[allow(clippy::too_many_arguments)]
/// Ordering anchor for the in-flight projectile step only.
#[derive(bevy::prelude::SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ProjectileStepSet;

/// A blocking surface wins when it reaches the leg before or with a hurt target.
/// Both times use the original source parameter.
pub(crate) fn wall_reaches_first(wall: Option<f64>, contact_time: f64) -> bool {
    wall.is_some_and(|wall| wall.total_cmp(&contact_time).is_le())
}

/// Is the surface that stopped this shot the TARGET'S OWN?
///
/// ⭐⭐ **A CONTRIBUTOR THAT SUPPLIES BOTH A SURFACE AND A DAMAGEABLE VOLUME IS
/// ONE THING, AND Q96 RULES THAT IT IS.** A solid destructible publishes a
/// collision block AND a hurt volume at the same place. Once its surface is
/// admitted into the shot's world, `wall_reaches_first` sees a wall at or before
/// the crate's own contact and refuses the hit — *"wall wins, therefore the crate
/// is invulnerable"*, which the ruling rejects by name. The contact must instead
/// COALESCE: damage once AND apply the surface response.
///
/// ⛔⛔ **THE COMPARISON IS THE OCCURRENCE, NOT A NAME STRING.** `world/overlay.rs`
/// publishes a breakable's block as
/// `GeoId::placement(PlacementId(its FeatureId), ordinal)` precisely so this can
/// be asked of identity. Matching `Block.name` would have worked well enough to
/// look right — the overlay writes `"ecs-breakable {display name}"` there — and
/// the protocol forbids exactly that: a display name is not an identity, and two
/// breakables may share one.
///
/// ⚠ **`index` IS DELIBERATELY IGNORED.** One occurrence may contribute several
/// blocks (ordinals 0..n); every one of them is still ITSELF.
pub(crate) fn wall_is_the_targets_own_surface(
    wall: Option<&ae::Block>,
    target_id: Option<&str>,
) -> bool {
    let (Some(wall), Some(target_id)) = (wall, target_id) else {
        return false;
    };
    match &wall.id.source {
        ae::GeoSource::Placement(placement) => placement.0 == target_id,
        _ => false,
    }
}

#[cfg(test)]
mod birth_tests {
    use super::*;

    #[test]
    fn a_refused_birth_spends_no_meter_or_cooldown_and_emits_no_shot() {
        use bevy::ecs::system::SystemState;
        let mut app = App::new();
        app.add_message::<ProjectileSpawnRequest>();
        app.add_message::<ambition_combat::RangedFireRefused>();
        let mut params = SystemState::<(MessageWriter<ProjectileSpawnRequest>, MessageWriter<ambition_combat::RangedFireRefused>)>::new(app.world_mut());
        let mut state = PlayerProjectileState::default();
        let before = state.spawner;
        let mut events = Vec::new();
        {
            let (mut spawn, mut refused) = params.get_mut(app.world_mut()).unwrap();
            let origin = |_, _| Err(ae::frame::NumericFault::Precision);
            let to_world = |vector| vector;
            let mut firing = Firing { owner: Entity::PLACEHOLDER, origin_of: &origin,
                admit: &Ok, flight: None, local_dir: ae::Vec2::X, to_world: &to_world,
                direction: ae::Vec2::X, damage_mult: 1.0, spawn: &mut spawn,
                refused: &mut refused, fired: 0 };
            assert_eq!(firing.fire(&mut state, &mut events, ambition_projectiles::ProjectileKind::Fireball, 0),
                FireOutcome::Geometry(ae::frame::thread::Refusal { path: Vec::new(), link: None,
                    reason: ae::frame::thread::RefusalReason::Numeric(ae::frame::NumericFault::Precision) }));
            assert_eq!(firing.fired, 0);
            let partition_refusal = ae::frame::thread::Refusal { path: vec![7], link: Some(7),
                reason: ae::frame::thread::RefusalReason::PieceLimit };
            let admit = |_| Err(partition_refusal.clone());
            let origin = |_, _| Ok(ae::Vec2::ZERO);
            firing.origin_of = &origin;
            firing.admit = &admit;
            assert_eq!(firing.fire(&mut state, &mut events, ambition_projectiles::ProjectileKind::Fireball, 0),
                FireOutcome::Geometry(partition_refusal.clone()));
            assert_eq!(firing.fired, 0);
        }
        assert_eq!(state.spawner, before);
        assert!(events.is_empty());
        assert!(app.world().resource::<Messages<ProjectileSpawnRequest>>().is_empty());
    }
}

#[cfg(test)]
mod coalesce_tests {
    use super::*;

    fn contributed_block(placement: &str, index: u16) -> ae::Block {
        ae::Block {
            id: ae::GeoId::placement(ae::PlacementId::new(placement), index),
            name: "ecs-breakable A Crate".to_string(),
            aabb: ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::splat(8.0)),
            kind: ae::BlockKind::Solid,
            velocity: ae::Vec2::ZERO,
            art_color: None,
        }
    }

    #[test]
    fn a_crates_own_surface_is_recognised_as_its_own() {
        let block = contributed_block("crate_17", 0);
        assert!(wall_is_the_targets_own_surface(
            Some(&block),
            Some("crate_17")
        ));
    }

    /// ⚠ ONE OCCURRENCE MAY CONTRIBUTE SEVERAL BLOCKS and every one is itself.
    #[test]
    fn a_later_ordinal_of_the_same_occurrence_is_still_its_own() {
        let block = contributed_block("crate_17", 3);
        assert!(wall_is_the_targets_own_surface(
            Some(&block),
            Some("crate_17")
        ));
    }

    /// ⛔ THE ARM THAT MATTERS: an UNRELATED wall still blocks. Without this the
    /// coalescing rule would make every shot reach every breakable through every
    /// wall, which is the defect the ordering filter exists to prevent.
    #[test]
    fn somebody_elses_wall_is_not_the_targets_own() {
        let block = contributed_block("a_different_crate", 0);
        assert!(!wall_is_the_targets_own_surface(
            Some(&block),
            Some("crate_17")
        ));
    }

    /// ⛔⛔ TILE TERRAIN IS NEVER ANYBODY'S OWN SURFACE, and it is the common
    /// case: a `TileLayer` block has no occurrence to be confused with.
    #[test]
    fn authored_terrain_is_nobodys_own_surface() {
        let mut block = contributed_block("crate_17", 0);
        block.id = ae::GeoId {
            source: ae::GeoSource::TileLayer {
                layer: "solids".to_string(),
            },
            index: 4,
        };
        assert!(!wall_is_the_targets_own_surface(
            Some(&block),
            Some("crate_17")
        ));
    }

    /// ⭐ AND A NAME MATCH IS NOT AN IDENTITY MATCH. The overlay puts the
    /// breakable's DISPLAY name in `Block.name`; a predicate reading that would
    /// pass this fixture and fail the moment two crates share a label.
    #[test]
    fn a_matching_display_name_with_a_different_occurrence_is_not_its_own() {
        let mut block = contributed_block("some_other_id", 0);
        block.name = "ecs-breakable crate_17".to_string();
        assert!(!wall_is_the_targets_own_surface(
            Some(&block),
            Some("crate_17")
        ));
    }

    #[test]
    fn no_wall_and_no_target_are_both_answers_of_no() {
        let block = contributed_block("crate_17", 0);
        assert!(!wall_is_the_targets_own_surface(None, Some("crate_17")));
        assert!(!wall_is_the_targets_own_surface(Some(&block), None));
    }
}

/// Resolve a splash through the common strike path. The shot supplies its
/// emission scope, captured allegiance, cause, and move credit before retirement.
pub(crate) fn emit_landing_splash(
    projectile: Entity,
    allegiance: Option<&AttackAllegiance>,
    pos: ae::Vec2,
    damage: i32,
    half: f32,
    attacker: Option<Entity>,
    // The live room the shot lands in. A shot with no owner has no attacker
    // to give the splash a room.
    room: Option<ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance>,
    // The use of the move of the attacker that fired this shot, if a move
    // fired it. The caller supplies the value. See `FiredByMoveInstance`.
    move_instance: Option<u32>,
    blasts: &mut ambition_combat::damage_box::DamageBoxSpawner,
    sfx: &mut SfxWriter,
    vfx: &mut VfxWriter,
) {
    let source = allegiance.map_or_else(
        || if attacker.is_none() { ambition_combat::strike::HitSide::Environment } else { ambition_combat::strike::HitSide::Neutral },
        |side| ambition_combat::hit_side_from_actor_faction(side.effective_faction()),
    );
    blasts.spawn(projectile, attacker.unwrap_or(Entity::PLACEHOLDER), source, pos,
        ambition_combat::strike::DamageBox {
            cause: HitSource::Projectile,
            move_instance,
            half_extent: ae::Vec2::splat(half),
            shape: None,
            damage,
            knockback: ambition_combat::strike::HitboxKnockback::FeelScale(1.0),
            lifetime_s: 0.0,
            name: Some("projectile splash"),
        });
    sfx.in_space(ambition_vfx::sfx_space(room)).write(SfxMessage::Play {
        id: ambition_sfx::ids::WORLD_ROCK_HIT,
        emitter: (pos).into(),
    });
    vfx.for_room(room).write(VfxMessage::Effect {
        pos,
        fx: ambition_vfx::fx::ids::CLASSIC_BURST,
        scale: 1.0,
        pose: ambition_vfx::FxPose::UPRIGHT,
    });
}

/// Step every live projectile in deterministic spawn order.
/// Player/enemy routing shares this body-general path; bosses and breakables use the feature-hit path.
pub fn step_projectiles(
    mut commands: Commands,
    world_time: Res<ambition_time::WorldTime>,
    collision: ambition_projectiles::collision_world::ProjectileCollisionWorld,
    gravity: ambition_platformer2d_shared_tangle::gravity::GravityCtx,
    mut projectiles: Query<
        (
            Entity,
            &mut BodyKinematics,
            &mut ProjectileGameplay,
            &mut ambition_platformer2d_shared_tangle::frame_env::CarriedAcceleration,
            Option<&ProjectileOwner>,
            // `None` is reserved for a genuinely ownerless/environmental volley.
            Option<&AttackAllegiance>,
            &ProjectileSeq,
            Option<&ambition_projectiles::ProjectileKind>,
            Option<&ambition_projectiles::ProjectileVisualId>,
            // G1: the firer's presentation source, stamped on the bolt at spawn.
            // Read from the PROJECTILE rather than chased back through the owner,
            // because a shot outlives its firer and must still sound like it.
            Option<&ambition_sfx::BodyPresentationSource>,
            // ⭐ THE USE OF THE MOVE THAT FIRED THIS SHOT. The reason is the
            // same as the reason on the line above. A shot can land after its
            // move stops, and also after its sound stops. The damage result
            // must name the move that made the shot. It must not name the move
            // that the owner plays at the time of the hit (A12). The value is
            // empty for each shot that no move fired.
            Option<&ambition_projectiles::FiredByMoveInstance>,
            // The growing launch this shot deals, if its weapon authors one.
            Option<&ambition_projectiles::ProjectileLaunch>,
            // Whom this shot has already hit on the leg it is flying.
            &mut ambition_platformer2d_shared_tangle::projectile::ProjectileHits,
        ),
        (
            With<LiveProjectile>,
            Without<ambition_platformer2d_shared_tangle::markers::PlayerEntity>,
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
    ecs_breakables: Query<
        (Entity, &FeatureId, &CenteredAabb, &BreakableFeature),
        With<FeatureSimEntity>,
    >,
    // ⛔ A2a: THE CATALOG, THE ATTACK STATE AND THE ANIMATION SAMPLE LEFT WITH
    // THE DERIVATION. The stepper asked "does this shot reach a boss" by
    // rebuilding the boss's hurt geometry from those three; it reads the
    // published `DamageableVolumes` now, which is the same value the damage
    // applier uses and the only one the publisher's authored override reaches.
    ecs_bosses: Query<
        (
            Entity,
            &FeatureId,
            &CenteredAabb,
            &ambition_characters::actor::BodyHealth,
            &ambition_combat::components::DamageableVolumes,
            Option<&ambition_platformer2d_core::BodyKinematics>,
        ),
        (With<FeatureSimEntity>, With<BossConfig>),
    >,
    mut sfx: SfxWriter,
    mut vfx: VfxWriter,
    mut heals: MessageWriter<ambition_damage::healing::BodyHealRequested>,
    mut trace: ResMut<GameplayTraceBuffer>,
    // Damage authority comes from the firer's faction/grudge/team. Match team outranks
    // faction for whether a shot may land; bosses use the authored catalog below.
    // The rules of the shot's own live room.
    tuning: ambition_combat::rules::CombatTuningOf,
    // Bundled into one SystemParam slot to stay under Bevy's parameter ceiling.
    // ⭐ A2a DELETED THE BOSS CATALOG FROM THIS SYSTEM. It was here for exactly
    // one reason — rebuilding a boss's hurt geometry to answer "does this shot
    // reach it" — and the answer comes from the published volumes now. The
    // projectile stepper no longer needs to know what a boss IS.
    (owner_combat, visual_catalog, mut outcome_fx, boss_guards, mut blasts, freeze): (
        Query<(
            &ActorFaction,
            Option<&ActorAggression>,
            Option<&ambition_combat::targeting::MatchTeam>,
            Option<&ambition_characters::control::DrivingParticipant>,
        )>,
        Res<ambition_projectiles::ProjectileVisualCatalog>,
        // The effect a shot's look shows where it lands, is turned, or goes out.
        MessageWriter<ambition_vfx::vfx::FxRequest>,
        // Whether a boss turns blows. `apply_boss_hit` reads the same two
        // fields and decides the damage; the shot reads them for its look.
        Query<(&BossConfig, &ambition_boss_encounter::BossEncounter)>,
        ambition_combat::damage_box::DamageBoxSpawner,
        // A connect's room holds its shots too (ISL-1b).
        ambition_combat::impact_hitstop::RoomFreeze,
    ),
) {
    let dt = world_time.sim_dt();
    // Each live room's projectile world, composed once per tick the first
    // time a shot in it asks.
    let mut composed: Vec<(Option<ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance>, Option<std::borrow::Cow<'_, ae::World>>,
    )> = Vec::new();
    // The pairs a shot's leg can enter, by live room.
    #[cfg(feature = "portal")]
    let portals_by_room = collision.portals_by_room();
    #[cfg(feature = "portal")]
    let portal_tuning = collision.portal_tuning();
    let tick = trace.current_tick();

    // Collect + sort by the GLOBAL spawn sequence (a single deterministic order
    // across both factions; the seq counter is shared at spawn).
    let mut ordered: Vec<(Entity, ProjectileSeq)> = projectiles
        .iter()
        .map(|(entity, _, _, _, _, _, seq, _, _, _, _, _, _)| (entity, *seq))
        .collect();
    ordered.sort_by_key(|(_, seq)| *seq);

    'projectiles: for (proj_entity, _) in ordered {
        let Ok((
            _,
            mut kin,
            mut game,
            acceleration,
            owner,
            stamped_allegiance,
            _,
            kind,
            visual_id,
            bolt_source,
            fired_by_move,
            launch,
            mut already_hit,
        )) = projectiles.get_mut(proj_entity)
        else {
            continue;
        };
        let stamped_allegiance = stamped_allegiance.cloned();
        let bolt_source = bolt_source.map(|source| source.id().clone());
        // The code copies this value out of the borrow with the other stamped
        // facts. The reason is the same: the code writes the damage results
        // below after it releases `projectiles`.
        let fired_by_move = fired_by_move.map(|stamp| stamp.0);
        let launch = launch.copied();
        // Named kind when the shot uses the named vocabulary; open-visual
        // volleys deliberately remain kind-less.
        #[cfg(feature="portal")]
        let mut acceleration=acceleration;
        let kind = kind.copied();
        // Open visual id (every spawned shot carries one; empty reads as the
        // generic look). Drives the detonation FX pick via the content-owned
        // catalog; ownership and allegiance are separate facts.
        let look = visual_id.map(|v| v.0.as_str()).and_then(|id| visual_catalog.get(id));
        let expiry_burst = look.and_then(|art| art.expiry_vfx);
        let hit_burst = look.and_then(|art| art.hit_vfx);
        let deflect_burst = look.and_then(|art| art.deflect_vfx);
        let owner_entity = owner.map(|o| o.0);
        // The shot flies in one live room, collides with that room's walls
        // and reaches only that room's bodies (OW1 cut 4). A shot whose room
        // is not live is not stepped.
        let shot_room = collision.shot_room(proj_entity, owner_entity);
        let friendly_fire = tuning.in_room(shot_room).map(|t| t.damage_policy()).unwrap_or_default();
        let dt = dt * freeze.scale_in(shot_room);
        let room_index = match composed.iter().position(|(room, _)| *room == shot_room) {
            Some(index) => index,
            None => {
                composed.push((shot_room, collision.solids_in(shot_room)));
                composed.len() - 1
            }
        };
        let Some(room_world) = composed[room_index].1.as_deref() else {
            continue;
        };
        let collision_world: &ambition_platformer2d_core::World = room_world;
        let owner_combat_data = owner_entity.and_then(|e| owner_combat.get(e).ok());
        // Projectile allegiance is frozen launch authority and rollback state. Parry may
        // deliberately rewrite it; it must not be re-derived from a firer that may be gone.
        let allegiance: Option<AttackAllegiance> = match stamped_allegiance {
            Some(stamped) => Some(stamped),
            // First sight. A shot always takes flight while its firer lives, so
            // this is the tick the fact is still there to be frozen.
            None => {
                let fresh = owner_combat_data.map(|(faction, _, team, driver)| AttackAllegiance::from_body(*faction, team, driver));
                if let Some(fresh) = fresh.clone() {
                    commands.entity(proj_entity).insert(fresh);
                }
                fresh
            }
        };
        // A grudge is live firer state, not launch provenance, so resolve it
        // from the current owner. If grudges gain a lifetime beyond the firer,
        // stamp the target's `SimId` on the projectile rather than an `Entity`.
        let firer_grudge: Option<ambition_combat::components::Grudge> = owner_combat_data
            .and_then(|(_, agg, _, _)| agg)
            .and_then(|a| a.grudge);
        // Only a projectile with no named owner is indiscriminate. A temporarily
        // unresolved named owner must not turn a team-owned shot into a hazard.
        let indiscriminate = allegiance.is_none() && owner_entity.is_none();

        // Tick + lifetime. A dead lasersword detonates; everything else logs an
        // Expired trace event.
        // A projectile is a FREE body (not a kernel body): resolve its gravity
        // inline by the body-overlap rule, not the center point (ADR 0024).
        let gravity_dir = gravity.dir_for(shot_room, kin.aabb());
        // ⛔⛔ **A2b: THE LEG IS CAPTURED, NOT RECONSTRUCTED.** Two places below
        // used to derive this shot's travel segment as `kin.pos - kin.vel * dt`.
        // That is EXACT for today's integrator — `tick` accelerates and then
        // integrates with the NEW velocity — and it is exact only because of
        // three unrelated facts nothing states together: that integration order,
        // an interception short-circuit that stops a re-owned shot before either
        // site, and a portal transit that `continue`s past both. Change any one
        // and the segment silently describes space the shot never crossed, on the
        // road that decides who it damages. The pre-integration position is right
        // here; taking it costs a copy and removes the coincidence.
        let mut integrated=*kin;
        let mut aged_game=*game;
        if !aged_game.tick(&mut integrated,*acceleration,dt,gravity_dir) {
            if let Some(burst)=expiry_burst {
                outcome_fx.write(said_by(burst.request(shot_room,kin.pos),bolt_source.as_ref()));
            } else {trace.push_event(ProjectileTraceEvent::Expired {kind}.into_trace_event(tick));}
            commands.entity(proj_entity).despawn();
            continue 'projectiles;
        }
        #[cfg(feature="portal")]
        let (mut leg_time,mut root_count,mut first_override)=(0.0,0,None);
        #[cfg(not(feature="portal"))]
        let leg_time=0.0;
        #[cfg(feature="portal")]
        let mut leg_candidate=integrated;
        #[cfg(not(feature="portal"))]
        let leg_candidate=integrated;
        #[cfg(feature="portal")]
        let mut hit_this_tick=false;
        #[cfg(not(feature="portal"))]
        let hit_this_tick=false;
        loop {
        let before=*kin;
        let leg_start=before.pos;
        let mut candidate=leg_candidate;
        let mut candidate_game=*game;
        candidate_game.age=aged_game.age;
        let gravity_dir=gravity.dir_for(shot_room,before.aabb());
        let admission=collision.motion_for(proj_entity,shot_room);
        #[cfg(feature="portal")]
        let admission=admission.map(|mut policy| {
            if let Some(first)=first_override {policy.first=Some(first);}
            policy
        });
        let attempted_delta = candidate.pos - leg_start;

        // A root event ends this leg. Keep the original line and its closed
        // prefix so transit does not reconstruct time from a projected endpoint.
        #[cfg(feature = "portal")]
        let (contact_end, travel_end, travel, root_crossing) = {
            let domain = ae::CombatVolume::aabb(ae::Aabb::new(ae::Vec2::ZERO, before.canonical_size * 0.5));
            let map = match before.material_map() {
                Ok(map) => map,
                Err(reason) => {
                    *game = candidate_game;
                    commands.entity(proj_entity).insert(ae::SweepSample::at_tick_end(before, ae::Vec2::ZERO));
                    bevy::log::debug!(target: "ambition_portal::refusal", body = ?proj_entity, ?reason,
                        "projectile root map refused before commit");
                    continue 'projectiles;
                }
            };
            let cast = ae::frame::material_cast::MaterialCast::new(
                ae::frame::thread::MaterialPrimitive { domain: &domain, to_world: map },
                leg_start,
                attempted_delta,
                leg_time..1.0,
            );
            match ambition_portal2d::first_portal_crossing_through(&cast,
                portals_by_room.in_room(shot_room), if root_count==0 {collision.backside_of(proj_entity)} else {None}, 1.0) {
                Ok(Some(hit)) => (hit.root.point, hit.root.toi,
                    ae::LinearTravel::Prefix { delta: attempted_delta, until: hit.root.toi },Some(hit)),
                Ok(None) => (candidate.pos, 1.0, ae::LinearTravel::Complete,None),
                Err(refusal) => {
                    *game = candidate_game;
                    commands.entity(proj_entity).insert(ae::SweepSample::at_tick_end(before, ae::Vec2::ZERO));
                    bevy::log::debug!(target: "ambition_portal::refusal", body = ?proj_entity, ?refusal,
                        "projectile root crossing refused before commit");
                    continue 'projectiles;
                }
            }
        };
        #[cfg(not(feature = "portal"))]
        let (contact_end,travel_end,travel) = (candidate.pos,1.0,ae::LinearTravel::Complete);

        // Wall and recipient contacts use the same admitted source material.
        // Build it once for this leg, before any recipient can commit a hit.
        let source_domain = before.canonical_box();
        let source_cells = source_domain.as_ref().map_err(|fault| ae::frame::thread::Refusal {
            path: vec![], link: None, reason: ae::frame::thread::RefusalReason::Numeric(*fault),
        }).and_then(|domain| {
            let map=before.material_map().map_err(|fault| ae::frame::thread::Refusal {
                path: vec![], link: None, reason: ae::frame::thread::RefusalReason::Numeric(fault),
            })?;
            ae::frame::material_cast::MaterialCast::new(ae::frame::thread::MaterialPrimitive {
                domain,to_world:map },before.pos,attempted_delta,leg_time..1.0)
                .with_initial_overlap(ae::frame::material_cast::InitialOverlap::Report)
                .prefix(travel_end)?.cells(admission)
        }).and_then(|cells| {cells.admit_payloads(candidate.vel,&[acceleration.0])?;Ok(cells)});
        let source_cells=match source_cells {
            Ok(cells)=>cells,
            Err(refusal)=>{
                *game=candidate_game;
                commands.entity(proj_entity).insert(ae::SweepSample::at_tick_end(before,ae::Vec2::ZERO));
                bevy::log::debug!(target:"ambition_portal::refusal",body=?proj_entity,?refusal,
                    "projectile source contact refused before commit");
                continue 'projectiles;
            }
        };

        candidate.pos = contact_end;
        let world_contact = match query_world_collision(
            before, candidate, candidate_game, collision_world, gravity_dir, dt,leg_time..1.0, travel,
            admission,
        ) {
            Ok(response) => response,
            Err(refusal) => {
                *game = candidate_game;
                commands.entity(proj_entity).insert(ae::SweepSample::at_tick_end(before, ae::Vec2::ZERO));
                bevy::log::debug!(target: "ambition_portal::refusal", body = ?proj_entity, ?refusal,
                    "projectile contact refused before commit");
                continue 'projectiles;
            }
        };
        let blocked_at = world_contact.contact.as_ref();
        let world_response;
        #[cfg(feature="portal")]
        let prepared_crossing;
        // `true` when an independent blocker reaches this leg no later than the
        // candidate contact — strictly earlier, or at an exact tie, which the
        // protocol awards to the wall.
        let wall_reaches_first = move |contact_time: f64| {
            wall_reaches_first(blocked_at.map(|hit| hit.contact.toi), contact_time)
        };
        // ⭐⭐ THE SAME QUESTION, ASKED ABOUT A TARGET THAT MAY OWN THE WALL.
        // Now that a live object's contributed surface is in this shot's world,
        // the wall standing at a crate's own contact IS the crate — and refusing
        // the hit on that ground is *"wall wins, therefore the crate is
        // invulnerable"*, which Q96 rejects by name. One contact: it damages
        // once AND takes the surface response below.
        let wall_blocks_reaching = move |contact_time: f64, target_id: &str| {
            wall_reaches_first(contact_time)
                && !wall_is_the_targets_own_surface(
                    blocked_at.and_then(|hit| match hit.collider { ae::world::MaterialCollider::Block(block) => Some(block), _ => None }),
                    Some(target_id),
                )
        };

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
            // ⭐⭐ A RETURNING SHOT IS TWO CHANCES, AND EACH VICTIM GETS ONE OF
            // EACH. The ponytail despawned on its first body contact, which made
            // every bit of the landed return-flight work unobservable in
            // combat — she threw it, it hit somebody, and it never came back
            //. ⛔ AND SIMPLY DELETING THE DESPAWN IS THE
            // WRONG FIX: a shot that survives contact overlaps its victim for as
            // many ticks as it takes to pass through, and damages on every one.
            //
            // ⭐ THE RULE IS THE PULSE RULE, which this codebase already states
            // for multi-hit moves: one continuous stretch of contact owns ONE
            // per-victim answer, and a gap starts a new one. A boomerang's legs
            // are its two stretches — out, and home — so each victim is hit at
            // most once per leg, the turnaround re-arms everybody, and hitting A
            // never protects B.
            //
            // ⛔ THE LEG IS DERIVED AND THE LEDGER IS STORED, which is the split
            // that matters: `ProjectileGameplay::leg` reads the trajectory the
            // shot is already carrying and so cannot disagree with it, while
            // `ProjectileHits` is authoritative rollback state because a
            // resimulated frame that lost it re-hits everybody the shot had
            // already passed through.
            let next_leg = candidate_game.leg(candidate.vel, *acceleration);
            let rearm_hits = next_leg != candidate_game.hits_cleared_on_leg;
            let may_touch = |target| !hit_this_tick && Some(target) != owner_entity
                && (rearm_hits || !already_hit.hit.contains(&target));
            // ⛔⛔ **WHERE IT WAS STRUCK, NOT WHERE THE TICK ENDED.** This was a
            // bare `bool`, and the splash below then used `kin.pos` — the
            // INTEGRATED ENDPOINT, which for a fast shot is far past a thin
            // target contacted at TOI 0.2. The protocol defines the landing
            // splash as an area attack at the SELECTED IMPACT LOCATION, and
            // this is gameplay, not presentation: the area `HitEvent` is
            // centred there, so a wrong centre hits different recipients —
            // possibly on the far side of geometry that lies after the contact.
            let mut struck_at: Option<ae::Vec2> = None;
            // ⛔⛔ NEAREST FIRST, AND IT USED TO BE QUERY ORDER. This loop `break`s
            // on the first row that qualifies, and `&victims` is a Bevy query —
            // archetype order, which is not a promise and is not reproduced by a
            // rollback resimulation. So a shot arriving on two overlapping
            // bodies in one tick damaged whichever the archetype happened to
            // list first, and a resim could pick the other one. Damage is
            // rollback-authoritative state; deciding it by iteration order is
            // deterministically wrong.
            //
            // ⭐ ORDERED BY GEOMETRY, which resimulation DOES reproduce exactly:
            // distance from where the shot started this leg, so the body it
            // reached first is the body it hits. ⚠ ties break on the victim's
            // own position rather than on its entity — an entity index is not
            // stable across a rewind and a position is, for the same reason the
            // ordering itself is trustworthy — and then on `SimId`, because two
            // bodies CAN share a position (a spawn point, a stack) and a stable
            // sort over an equal key hands the decision back to query order
            // (S3, 2026-09-02).
            //
            // ⭐⭐ AND IT IS TIME OF IMPACT NOW, NOT CENTRE DISTANCE. Contact
            // itself is swept: the shot's box travels the captured leg, so a
            // fast bolt reaches a thin body it flies clean through between
            // samples — the endpoint overlap this used to ask says no, and the
            // shot sailed on with the target untouched. Once contact has a
            // finite time, ordering by "whose centre is nearer the muzzle" can
            // disagree with "whom it reached first"; a big body slightly further
            // out is touched before a small one dead ahead.
            // ⛔⛔ **THE FEATURE CANDIDATE IS RESOLVED HERE, BESIDE THE BODIES,
            // BECAUSE THEY ARE COMPETING FOR THE SAME CONTACT.** It used to be
            // computed after this whole block had `continue`d away on a body
            // hit, which made "body" beat "boss or breakable" by being the
            // earlier CODE branch: a crate at 0.2 of the leg lost to a body at
            // 0.8. That is the family knowledge A2 exists to remove — the
            // protocol orders contacts within a projectile by finite time of
            // impact, and a family name is not a time.
            // Every recipient query must succeed before motion or contact is committed.
            let prepared = (|| -> Result<_, ae::frame::thread::Refusal> {
                let breakable_contact = super::feature_contact::projectile_reaches_breakable(
                    &source_cells,
                    collision.body_geometry(),
                    shot_room,
                    |target| {
                        may_touch(target)
                            && collision.room_of(target) == shot_room
                    },
                    &ecs_breakables,
                )?;
                let boss_contact = super::feature_contact::projectile_reaches_boss(
                    &source_cells,
                    collision.body_geometry(),
                    shot_room,
                    |target| {
                        may_touch(target)
                            && collision.room_of(target) == shot_room
                    },
                    &ecs_bosses,
                )?;
                let feature_contact = breakable_contact.into_iter().chain(boss_contact)
                // ⛔⛔ **A WALL EARLIER ON THE LEG STOPS THE SHOT BEFORE IT REACHES A
                // CRATE OR A BOSS**, and this branch asked nothing at all. It emitted
                // the targeted hit and the splash and `continue`d, so the world sweep
                // below only ever ran when NO feature was reached — which made the
                // ordering *"feature contact, else world collision"* rather than
                // *"whichever came first"*. A breakable standing behind an unrelated
                // solid was broken through it, and a boss behind one was damaged
                // through it. Same rule as the bodies below, same single sweep, same
                // strict comparison.
                .filter(|contact| contact.contact.hit.toi <= travel_end)
                .filter(|contact| !wall_blocks_reaching(contact.contact.hit.toi, &contact.target_id))
                .min_by(super::feature_contact::FeatureContact::order_for_caller);
                let mut ordered = Vec::new();
                for victim in victims.iter().filter(|victim| collision.room_of(victim.entity) == shot_room
                    && may_touch(victim.entity)) {
                    // Only an eligible recipient can refuse this contact query.
                let can_hit = indiscriminate
                    || allegiance.as_ref().is_some_and(|side| {
                        ambition_combat::targeting::damage_lands_between(
                            tuning.relations(),
                            ambition_combat::targeting::Combatant {
                                faction: side.faction,
                                driver: side.driver.as_ref(),
                                team: side.team(),
                            },
                            ambition_combat::targeting::Combatant {
                                faction: *victim.faction,
                                driver: victim.driver,
                                team: victim.team,
                            },
                            friendly_fire,
                            firer_grudge,
                            victim.entity,
                        )
                    });
                if !can_hit || victim.is_corpse() {
                    continue;
                }
                    if let Some(hit) = victim.contact_along(&source_cells, collision.body_geometry(), shot_room)?
                        .filter(|hit| hit.hit.toi <= travel_end) {
                        let magnitude=if let Some(launch)=launch {
                            let numeric=|fault| ae::frame::thread::Refusal {
                                path:hit.hit.source_path.clone(),link:None,
                                reason:ae::frame::thread::RefusalReason::Numeric(fault),
                            };
                            let into_victim=hit.hit.source_map.then(hit.hit.target_map.inverse().map_err(numeric)?)
                                .map_err(numeric)?;
                            let (damage_taken,weight)=victim.knockback_growth_inputs();
                            ambition_combat::hitbox::projectile_launch_magnitude(launch.base,launch.growth,
                                damage_taken,weight,&tuning.in_room(shot_room).unwrap_or_default())
                                .in_chart(into_victim).map_err(numeric)?
                        } else { HitKnockbackMagnitude::FeelScale(0.85) };
                        ordered.push((hit, victim, magnitude));
                    }
                }
                Ok((feature_contact, ordered))
            })();
            let (feature_contact, mut ordered) = match prepared {
                Ok(contacts) => contacts,
                Err(refusal) => {
                    *game = candidate_game;
                    commands.entity(proj_entity).insert(ae::SweepSample::at_tick_end(before, ae::Vec2::ZERO));
                    bevy::log::debug!(target: "ambition_portal::refusal", body = ?proj_entity, ?refusal,
                        "projectile recipient contact refused before commit");
                    continue 'projectiles;
                }
            };
            ordered.sort_by(|(at, a, _), (bt, b, _)| {
                let (ax, ay) = (a.aabb.aabb().center().x, a.aabb.aabb().center().y);
                let (bx, by) = (b.aabb.aabb().center().x, b.aabb.aabb().center().y);
                at.hit.toi.total_cmp(&bt.hit.toi)
                    .then(ax.total_cmp(&bx))
                    .then(ay.total_cmp(&by))
                    // ⚠ THE KEY, NOT THE `Option`. `a.sim_id.cmp(&b.sim_id)`
                    // put an UNIDENTIFIED body first, which is the reverse of
                    // the rule the field documents.
                    .then_with(|| {
                        ambition_combat::hitbox::victim_identity_key(
                            a.sim_id.map(|id| id.as_str()))
                        .cmp(&ambition_combat::hitbox::victim_identity_key(
                            b.sim_id.map(|id| id.as_str()),
                        ))
                    })
            });

            // ⛔⛔ **TWO COINCIDENT VICTIMS THAT ARE BOTH UNIDENTIFIED ARE
            // UNDECIDABLE HERE, AND THE SORT ABOVE WOULD HIDE IT.** Ordering
            // absent identity LAST fixes the case where one body has a `SimId`
            // and the other does not; it cannot fix the case where NEITHER
            // does. Those two compare equal on every key, `sort_by` is stable,
            // and stability over an equal key is Bevy query order — which a
            // rollback resimulation does not promise to reproduce. The protocol
            // calls missing required target identity a CONSTRUCTION failure, not
            // a sort fallback, so there is nothing for this seam to invent.
            //
            // ⭐ AND THE ASSERT IS ALSO THE CENSUS. A static scan cannot see this
            // population — bodies receive `CenteredAabb` and `ActorFaction` from
            // separate inserts, so a bundle-shaped scanner reports 2 of 2 and is
            // describing itself. An assertion that never fires across the whole
            // suite answers the question that matters — *does this happen on any
            // road we exercise?* — and if it fires it hands over the exact
            // fixture, which is worth more than a count.
            //
            // ⚠ DEBUG ONLY, DELIBERATELY. This is a per-shot hot loop and a case
            // that should never happen does not earn a per-frame log.
            #[cfg(debug_assertions)]
            {
                let ambiguous = ordered
                    .windows(2)
                    .find(|pair| {
                        let ((at, a, _), (bt, b, _)) = (&pair[0], &pair[1]);
                        a.sim_id.is_none()
                            && b.sim_id.is_none()
                            && at.hit.toi == bt.hit.toi
                            && a.aabb.aabb().center() == b.aabb.aabb().center()
                    })
                    .map(|pair| (pair[0].1.entity, pair[1].1.entity));
                debug_assert!(
                    ambiguous.is_none(),
                    "two victims coincide in time AND position and NEITHER carries \
                     a `SimId`, so which one this shot strikes is Bevy query order: \
                     {ambiguous:?}. Nothing on `StrikeVictim` can separate them — \
                     `Entity` does not survive a rewind and every geometric key is \
                     already spent. Give the bodies identity where they are BUILT",
                );
            }

            let selected_body=ordered.into_iter()
                .find(|(contact,_,_)|!wall_reaches_first(contact.hit.toi))
                .filter(|(contact,victim,_)|!(feature_contact.as_ref().is_some_and(|feature| {
                    let victim_center = victim.aabb.aabb().center();
                    feature
                        .contact.hit.toi
                        .total_cmp(&contact.hit.toi)
                        .then(feature.target_center.x.total_cmp(&victim_center.x))
                        .then(feature.target_center.y.total_cmp(&victim_center.y))
                        .then_with(|| {
                            // The feature is always identified; the body may not
                            // be, and an unidentified body does not win the tie.
                            ambition_combat::hitbox::victim_identity_key(Some(
                                feature.target_id.as_str(),
                            ))
                            .cmp(&ambition_combat::hitbox::victim_identity_key(
                                victim.sim_id.map(|id| id.as_str()),
                            ),
                            )
                        })
                        .is_lt()
                })));
            // Admit the response before movement, guard rewards or ownership change.
            let interception=(|| -> Result<Option<super::intercept::PreparedInterception>,ae::frame::thread::Refusal> {
                let Some((contact,victim,_))=selected_body.as_ref() else {return Ok(None);};
                let Ok(guard)=guards.get(victim.entity) else {return Ok(None);};
                if !guard.parrying() {return Ok(None);}
                let absorbs=guard.absorbs_projectiles();
                let request=if absorbs {super::intercept::ProjectileInterception::Consume}
                    else {super::intercept::ProjectileInterception::Reflect {speed_scale:PROJECTILE_REFLECT_SPEED_SCALE}};
                let frame=if absorbs {ae::frame::material_cast::ContactVelocityFrame::IDENTITY}
                    else {source_cells.contact_velocity_frame(&contact.hit)?};
                let numeric=|fault| ae::frame::thread::Refusal {path:contact.hit.source_path.clone(),link:None,
                    reason:ae::frame::thread::RefusalReason::Numeric(fault)};
                let mut stopped=candidate;
                stopped.pos=ae::frame::Sim2::IDENTITY.point_from_wide(
                    leg_start.as_dvec2()+attempted_delta.as_dvec2()*contact.hit.toi).map_err(numeric)?;
                let response=request.prepare(stopped,frame).map_err(numeric)?;
                if !absorbs {source_cells.admit_payloads_at(contact.hit.toi,response.body().vel,&[acceleration.0])?;}
                Ok(Some(response))
            })();
            let interception=match interception {
                Ok(response)=>response,
                Err(refusal)=>{
                    *game=candidate_game;
                    commands.entity(proj_entity).insert(ae::SweepSample::at_tick_end(before,ae::Vec2::ZERO));
                    bevy::log::debug!(target:"ambition_portal::refusal",body=?proj_entity,?refusal,
                        "projectile interception refused before commit");
                    continue 'projectiles;
                }
            };

            #[cfg(feature="portal")]
            {
                let reaches_root=interception.is_none() && world_contact.contact.is_none()
                    && (candidate_game.returns(*acceleration) || selected_body.is_none() && feature_contact.is_none());
                let prepared=(|| -> Result<_,ae::frame::thread::Refusal> {
                    let Some(crossing)=root_crossing.filter(|_|reaches_root) else {return Ok(None);};
                    let numeric=|fault|ae::frame::thread::Refusal {path:vec![],link:None,
                        reason:ae::frame::thread::RefusalReason::Numeric(fault)};
                    let policy=admission.ok_or_else(||numeric(ae::frame::NumericFault::InvalidFrame))?;
                    let portals=portals_by_room.in_room(shot_room);
                    let enter=ambition_portal2d::find_portal(portals,crossing.entry_channel)
                        .ok_or_else(||numeric(ae::frame::NumericFault::InvalidFrame))?;
                    let exit=ambition_portal2d::find_portal(portals,crossing.exit_channel)
                        .ok_or_else(||numeric(ae::frame::NumericFault::InvalidFrame))?;
                    let forward=policy.links.iter().find(|link|link.enter==enter.aperture() && link.exit==exit.aperture())
                        .ok_or_else(||numeric(ae::frame::NumericFault::InvalidFrame))?;
                    let reverse=policy.links.iter().find(|link|link.enter==exit.aperture() && link.exit==enter.aperture())
                        .ok_or_else(||numeric(ae::frame::NumericFault::InvalidFrame))?;
                    if root_count>=policy.budget.max_hops {
                        return Err(ae::frame::thread::Refusal {path:vec![],link:Some(forward.key),
                            reason:ae::frame::thread::RefusalReason::HopLimit});
                    }
                    let transfer=ambition_portal2d::PortalTransfer::at_crossing(crossing,candidate.pos,candidate.vel,&portal_tuning)
                        .map_err(numeric)?;
                    let input_warp=ambition_portal2d::portal_input_warp_flips_horizontal_for_convention(
                        policy.convention,transfer.enter.frame.normal,transfer.exit.frame.normal);
                    let prepared=ambition_portal2d::PreparedPortalTransfer::new(candidate,Some(acceleration.0),transfer,input_warp)
                        .map_err(numeric)?;
                    let body=*prepared.body();
                    let domain=body.canonical_box().map_err(numeric)?;
                    let cast=ae::frame::material_cast::MaterialCast::new(ae::frame::thread::MaterialPrimitive {
                        domain:&domain,to_world:body.material_map().map_err(numeric)?,
                    },body.pos,ae::Vec2::ZERO,crossing.root.world_time..crossing.root.world_time);
                    ae::frame::thread::MotionAdmission {first:Some(reverse.key),..policy}.admit(cast,body.vel,
                        &[prepared.acceleration().unwrap()])?;
                    Ok(Some((prepared,crossing.root.world_time,reverse.key)))
                })();
                prepared_crossing=match prepared {
                    Ok(prepared)=>prepared,
                    Err(refusal)=>{
                        *game=candidate_game;
                        commands.entity(proj_entity).insert(ae::SweepSample::at_tick_end(before,ae::Vec2::ZERO));
                        bevy::log::debug!(target:"ambition_portal::refusal",body=?proj_entity,?refusal,
                            "projectile root transfer refused before commit");
                        continue 'projectiles;
                    }
                };
            }

            // A terminal target response does not reach the later world response.
            world_response=if interception.is_none() && (candidate_game.returns(*acceleration)
                || selected_body.is_none() && feature_contact.is_none()) {
                match world_contact.response() {
                    Ok(response)=>Some(response),
                    Err(refusal)=>{
                        *game=candidate_game;
                        commands.entity(proj_entity).insert(ae::SweepSample::at_tick_end(before,ae::Vec2::ZERO));
                        bevy::log::debug!(target:"ambition_portal::refusal",body=?proj_entity,?refusal,
                            "projectile world response refused before commit");
                        continue 'projectiles;
                    }
                }
            } else {None};

            if rearm_hits {
                already_hit.hit.clear();
                candidate_game.hits_cleared_on_leg = next_leg;
            }
            *game = candidate_game;
            if let Some(response)=interception {
                let (contact,victim,_)=selected_body.as_ref().unwrap();
                guards.get_mut(victim.entity).unwrap().catch_parry();
                heals.write(ambition_damage::healing::BodyHealRequested::new(PARRY_HEAL,victim.entity));
                if commit_parried_shot(&mut commands,proj_entity,&mut kin,&response,contact.hit.point,
                    victim.entity,AttackAllegiance::from_body(*victim.faction,victim.team,victim.driver),
                    victim.voice.map(ambition_sfx::BodyPresentationSource::id),shot_room,&mut sfx,&mut vfx) {
                    commands.entity(proj_entity).insert(ae::SweepSample {turn:None,material:before.material,
                        travel:ae::LinearTravel::Prefix {delta:attempted_delta,until:contact.hit.toi},time:[leg_time,1.0],
                        prev:leg_start,curr:kin.pos,vel:kin.vel,half:kin.half_extent(),down:ae::Vec2::ZERO});
                }
                continue 'projectiles;
            }
            *kin = candidate;

            // A shot crosses a portal as every body does (`portal_transit`): it
            // straddles the opening and the pair takes it when its centre
            // crosses. Its leg is its sweep, so a fast shot that crossed a plane
            // this tick is taken through it, not stopped on the wall behind.
            commands.entity(proj_entity).insert(ae::SweepSample { turn: None, material: before.material, travel, time: [leg_time, 1.0],
                prev: leg_start,
                curr: kin.pos,
                vel: kin.vel,
                half: kin.size() * 0.5,
                down: ae::Vec2::ZERO,
            });

            if let Some((contact,victim,magnitude))=&selected_body {
                let victim_body=victim.aabb.aabb();
                // CM8: vulnerability is no longer read here to MUTE feedback — the ONE
                // victim-side reaction fires only on a landed hit, so a dodged / parried /
                // i-framed hit is muted for free at consume time (`resolve_body_hit`).
                let contact_volume = contact.source_volume.clone();
                let source_in_victim = contact.source_in_target;
                let side = victim.knockback_side();
                let knock_dir = (victim_body.center() - source_in_victim).dot(side).signum();
                let knock_dir = if knock_dir == 0.0 { 1.0 } else { knock_dir };
                let impact_pos = contact.hit.point;
                feature_damage.write(HitEvent {
                    contact: Some(ambition_combat::events::HitContact::from(&contact.hit)),
                    pieces: Vec::new(),
                    strike_sfx: None,
                    volume: contact_volume,
                    damage: game.damage.max(1),
                    source: HitSource::Projectile,
                    // The firing actor (enemy / boss), when the shot was spawned
                    // with a real owner — `None` for ownerless shots.
                    attacker: owner_entity,
                    room: None,
                    // The victim, named — no producer-side classification.
                    target: HitTarget::Body(victim.entity),
                    mode: HitMode::Knockback,
                    knockback: Some(HitKnockback {
                        // An ordinary hit: it stuns.
                        reaction: ambition_platformer2d_core::hit_response::HitReaction::Strike,
                        dir: knock_dir,
                        magnitude: *magnitude,
                        // WHERE THE SHOT WAS, not where it ended: the endpoint of
                        // a fast bolt is past the body it hit, and a knockback
                        // sourced there pushes the victim back toward the muzzle.
                        source_pos: source_in_victim,
                        impact_pos,
                        launch_dir: None,
                        follow: None,
                        guard: Default::default(),
                    }),
                    ignored_targets: Vec::new(),
                    attacker_move_instance: fired_by_move,
                });
                // CM8: the struck body's feedback (sound + spray) is emitted by
                // the ONE victim-side reaction now — a player victim through
                // `apply_player_hit_events`, an actor through `apply_actor_hit`.
                // Emitting here too would double the cue.
                //
                // The shot's own look of landing is the shot's: its hit burst.
                if let Some(burst) = hit_burst {
                    outcome_fx.write(said_by(burst.request(shot_room, impact_pos), bolt_source.as_ref(),
                    ));
                }
                already_hit.hit.insert(victim.entity);
                struck_at = Some(contact.hit.point);
                #[cfg(feature="portal")]
                {hit_this_tick=true;}
            }
            if let Some(impact) = struck_at {
                trace.push_event(
                    ProjectileTraceEvent::Hit {
                        kind,
                        damage: game.damage,
                    }
                    .into_trace_event(tick),
                );
                // ⭐ A SHOT THAT COMES BACK OUTLIVES WHAT IT HIT. Everything else
                // is spent on contact, which is what a bolt IS; the ponytail is
                // thrown and caught, and a throw that ends in the first body it
                // touches is not the move Jon asked for.
                if !game.returns(*acceleration) {
                    if game.splash_half_extent > 0.0 {
                        emit_landing_splash(
                            proj_entity,
                            allegiance.as_ref(),
                            impact,
                            game.damage.max(1),
                            game.splash_half_extent,
                            owner_entity,
                            shot_room,
                            fired_by_move,
                            &mut blasts,
                            &mut sfx,
                            &mut vfx,
                        );
                    }
                    commands.entity(proj_entity).despawn();
                    continue 'projectiles;
                }
            } else if let Some(contact) = feature_contact.as_ref() {
                let direct = HitEvent {
                    contact: Some(ambition_combat::events::HitContact::from(&contact.contact.hit)),
                    pieces: Vec::new(),
                    strike_sfx: None,
                    volume: contact.contact.source_volume.clone(),
                    damage: game.damage.max(1),
                    source: HitSource::Projectile,
                    attacker: owner_entity,
                    room: None,
                    // ⭐ NAMED, and the sweep above is what names it. Sent as
                    // `UnresolvedFeatures` the applier took the VOLUME and damaged
                    // every breakable it overlapped, so one shot whose contact box
                    // spanned two adjacent crates broke both — and which part of a
                    // multi-part boss took the hit was a query-order answer a rewind
                    // need not reproduce. A direct contact fixes its recipient
                    // before damage.
                    target: HitTarget::Feature(contact.target),
                    mode: HitMode::Knockback,
                    knockback: None,
                    ignored_targets: Vec::new(),
                    attacker_move_instance: fired_by_move,
                };
                // ⛔⛔ **DIRECT FIRST, THEN ITS LANDING AREA — and this branch had
                // it backwards.** The ordinary body road writes the targeted
                // request and then the splash; this one wrote the splash first,
                // so for a boss or a breakable the AREA event was applied before
                // the hit that caused it. It matters wherever the first
                // application changes what the second finds: a hit that grants
                // invulnerability, or one that breaks the target, was being
                // credited to the splash and refused to the shot.
                //
                // ⚠ THIS IS A BEHAVIOUR CHANGE, not a tidy-up, and the protocol
                // asks for it as its own patch for that reason. One splash per
                // landing and the recipient's inclusion in it are unchanged.
                feature_damage.write(direct);
                // A boss whose hull turns blows turns the shot: the look of a
                // deflection, not of a hit.
                let turned = boss_guards.get(contact.target).is_ok_and(|(config, status)| {
                    config.behavior.environmental_kill_only || status.guarded
                });
                if let Some(burst) = if turned { deflect_burst } else { hit_burst } {
                    let at = contact.contact.hit.point;
                    outcome_fx.write(said_by(burst.request(shot_room, at), bolt_source.as_ref()));
                }
                if game.splash_half_extent > 0.0 {
                    emit_landing_splash(
                        proj_entity,
                        allegiance.as_ref(),
                        // ⛔ THE SELECTED CONTACT, NOT THE TICK ENDPOINT — same
                        // defect as the body road above, same reason it is
                        // gameplay rather than presentation. This branch has the
                        // contact's own time in hand, so there is nothing to
                        // reconstruct.
                        contact.contact.hit.point,
                        game.damage.max(1),
                        game.splash_half_extent,
                        owner_entity,
                        shot_room,
                        fired_by_move,
                        &mut blasts,
                        &mut sfx,
                        &mut vfx,
                    );
                }
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
                // ⛔⛔ **LIFETIME IS THE SHOT'S POLICY, NOT THE RECEIVER'S FAMILY,
                // and this branch used to despawn UNCONDITIONALLY.** A boomerang
                // that clipped a crate simply never came back; the same throw
                // into a body survived, because the ordinary branch asks
                // `game.returns(*acceleration)`. One projectile, two lifetimes, decided by
                // what it happened to touch. The protocol calls this an
                // intentional correction rather than a preserved behaviour.
                //
                // ⭐ IT NEEDED THE TARGET'S IDENTITY, which is why the swept
                // contact carries one. A surviving shot overlaps what it hit for
                // as many ticks as it takes to pass through, so it must remember
                // WHOM on this leg — exactly the ledger the body branch keeps.
                already_hit.hit.insert(contact.target);
                #[cfg(feature="portal")]
                {hit_this_tick=true;}
                if !game.returns(*acceleration) {
                    commands.entity(proj_entity).despawn();
                    continue 'projectiles;
                }
            }
        }

        let world_response=world_response.expect("surviving flight prepares its world response before commit");
        *kin = world_response.body;
        let recorded_travel = world_contact.contact.as_ref().map_or(travel, |hit| {
            ae::LinearTravel::Prefix { delta: attempted_delta, until: hit.contact.toi }
        });
        game.bounces_remaining = world_response.bounces_remaining;
        commands.entity(proj_entity).insert(ae::SweepSample { turn: None, material: before.material, travel: recorded_travel, time: [leg_time, 1.0],
            prev: leg_start, curr: kin.pos, vel: candidate.vel,
            half: kin.half_extent(), down: ae::Vec2::ZERO,
        });
        match world_response.outcome {
            WorldHitOutcome::Bounced { pos } => {
                sfx.in_space(ambition_vfx::sfx_space(shot_room)).write_for_body(bolt_source.as_ref(), SfxMessage::Hit { pos });
                continue 'projectiles;
            }
            WorldHitOutcome::Expired { pos } => {
                if game.splash_half_extent > 0.0 {
                    emit_landing_splash(
                        proj_entity,
                        allegiance.as_ref(),
                        pos,
                        game.damage.max(1),
                        game.splash_half_extent,
                        owner_entity,
                        shot_room,
                        fired_by_move,
                        &mut blasts,
                        &mut sfx,
                        &mut vfx,
                    );
                }
                match expiry_burst {
                    Some(burst) => {
                        outcome_fx.write(said_by(burst.request(shot_room, pos), bolt_source.as_ref()));
                    }
                    None => {
                        trace.push_event(
                            ProjectileTraceEvent::Hit {
                                kind,
                                damage: game.damage,
                            }
                            .into_trace_event(tick),
                        );
                        vfx.write_in(shot_room, VfxMessage::Impact { pos, blow: ambition_vfx::ImpactBlow::PLAIN });
                    }
                }
                commands.entity(proj_entity).despawn();
                continue 'projectiles;
            }
            WorldHitOutcome::Continue => {
                #[cfg(feature="portal")]
                if let Some((prepared,time,first))=prepared_crossing {
                    ambition_portal2d::commit_body_transfer(&mut commands,proj_entity,&mut kin,
                        Some(&mut acceleration),None,prepared,portal_tuning.teleport_cooldown_s);
                    commands.entity(proj_entity).insert(ae::SweepSample::at_tick_end(*kin,ae::Vec2::ZERO));
                    root_count+=1;
                    first_override=Some(first);
                    leg_time=time;
                    if leg_time<1.0 {
                        leg_candidate=*kin;
                        match ae::frame::Sim2::IDENTITY.point_from_wide(kin.pos.as_dvec2()
                            +kin.vel.as_dvec2()*(f64::from(dt)*(1.0-leg_time))) {
                            Ok(point)=>leg_candidate.pos=point,
                            Err(reason)=>{
                                bevy::log::debug!(target:"ambition_portal::refusal",body=?proj_entity,?reason,
                                    "projectile remaining translation refused");
                                continue 'projectiles;
                            }
                        }
                        continue;
                    }
                }
            }
        }
        break;
        }
    }
}

#[cfg(test)]
mod parry_tests {
    use super::*;
    use ambition_vfx::vfx::VfxInRoom;

    #[derive(Resource)]
    struct Parrier(Entity);

    fn reflect_the_shot(
        mut commands: Commands,
        parrier: Res<Parrier>,
        mut sfx: SfxWriter,
        mut vfx: VfxWriter,
        sources: Query<&ambition_sfx::BodyPresentationSource>,
        mut shots: Query<(Entity, &mut BodyKinematics)>,
    ) {
        let parrier_source = sources.get(parrier.0).ok().map(|s| s.id().clone());
        for (proj, mut kin) in &mut shots {
            let response=super::super::intercept::ProjectileInterception::Reflect {speed_scale:PROJECTILE_REFLECT_SPEED_SCALE}
                .prepare(*kin,ae::frame::material_cast::ContactVelocityFrame::IDENTITY).unwrap();
            let impact=kin.pos;
            commit_parried_shot(
                &mut commands,
                proj,
                &mut kin,
                &response,
                impact,
                parrier.0,
                AttackAllegiance {
                    driver: None,
                    faction: ActorFaction::Player,
                    team: None,
                },
                parrier_source.as_ref(),
                None,
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
        app.add_message::<VfxInRoom>();
        let parrier = app
            .world_mut()
            .spawn(ambition_sfx::BodyPresentationSource(
                ambition_sfx::PresentationSourceId::new("mary_o_demo"),
            ))
            .id();
        app.world_mut().spawn((
            BodyKinematics { material: Default::default(),
                pos: ae::Vec2::ZERO,
                vel: ae::Vec2::new(100.0, 0.0),
                canonical_size: ae::Vec2::new(8.0, 8.0),
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
        app.add_message::<VfxInRoom>();
        let parrier = app.world_mut().spawn_empty().id();
        let proj = app
            .world_mut()
            .spawn(BodyKinematics { material: Default::default(),
                pos: ae::Vec2::ZERO,
                vel: ae::Vec2::new(100.0, -40.0),
                canonical_size: ae::Vec2::new(8.0, 8.0),
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
            world.get::<AttackAllegiance>(proj),
            Some(&AttackAllegiance {
                driver: None,
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

#[cfg(test)]
mod wall_order_tests {
/// An exact tie and adjacent clock values retain their strict order.
#[test]
fn an_independent_wall_wins_an_exact_tie_against_a_hurt_target() {
    use super::*;

    assert!(
        wall_reaches_first(Some(0.5), 0.5),
        "a wall at exactly the target's contact time must win — a tie may not \
         grant damage through it"
    );

    assert!(
        wall_reaches_first(Some(0.5f64.next_down()), 0.5),
        "a wall that genuinely reaches the leg first was ordered behind the \
         target by the epsilon slack"
    );
    assert!(wall_reaches_first(Some(0.1), 0.5), "a plainly earlier wall is first");

    assert!(
        !wall_reaches_first(Some(0.5f64.next_up()), 0.5),
        "a wall reached AFTER the target must not block it"
    );
    assert!(!wall_reaches_first(Some(0.9), 0.5), "a plainly later wall is second");
    assert!(
        !wall_reaches_first(None, 0.5),
        "no wall on the leg cannot block anything"
    );
}

}
