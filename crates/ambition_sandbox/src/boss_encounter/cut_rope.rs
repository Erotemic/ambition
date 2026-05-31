//! Cut-rope boss arena rules.
//!
//! The arena is authored in LDtk as ordinary `Prop` entities named/kinded
//! `cut_rope_rope` and `cut_rope_anvil`, plus a `BossSpawn` whose behavior id
//! is `smirking_behemoth_boss`. This system keeps the one-off mechanic tied to
//! authored level data rather than hard-coded coordinates: cutting the rope prop
//! starts the anvil prop falling; the anvil impact forces the boss encounter
//! through the normal death pipeline.

use bevy::prelude::*;

use crate::brain::ActorControl;
use crate::audio::SfxMessage;
use crate::boss_encounter::{force_boss_death, BossEncounterRegistry};
use crate::brain::BossAttackState;
use crate::config::world_to_bevy;
use crate::engine_core::{self as ae, AabbExt};
use crate::features::{
    BossFeature, BossRuntime, FeatureAabb, FeatureName, FeatureSimEntity, GameplayBanner, HitEvent,
    HitSource, ResetRoomFeaturesEvent,
};
use crate::presentation::fx::{
    ExplosionKind, ExplosionRequest, FireworksRequest, ParticleKind, VfxMessage,
};
use crate::presentation::rendering::PropVisual;
use crate::rooms::{PropSpec, RoomSet};
use crate::world::physics::{DebrisBurstMessage, PhysicsDebrisCue};

pub const CUT_ROPE_BOSS_ID: &str = "smirking_behemoth_boss";
const CUT_ROPE_ROOM_ID: &str = "you_have_to_cut_the_rope";
const ROPE_KIND: &str = "cut_rope_rope";
const ANVIL_KIND: &str = "cut_rope_anvil";
const ANVIL_GRAVITY: f32 = 1400.0;
const ANVIL_TERMINAL_SPEED: f32 = 920.0;
const ANVIL_Z_OFFSET: f32 = 0.75;
const ROPE_ALIGNMENT_TOLERANCE: f32 = 42.0;
const ROPE_LURE_SPEED: f32 = 150.0;
const ROPE_SPARK_INTERVAL: f32 = 0.22;

pub fn is_cut_rope_boss(id: &str) -> bool {
    id == CUT_ROPE_BOSS_ID
}

#[derive(Resource, Default)]
pub struct CutRopeBossArenaState {
    active_room: String,
    rope_cut: bool,
    awaiting_alignment: bool,
    anvil_center: Option<ae::Vec2>,
    anvil_velocity_y: f32,
    kill_sent: bool,
    anvil_exploded: bool,
    rope_fx_timer: f32,
    rope_fx_pulse: u32,
    death_fireworks_sent: bool,
}

/// Drive the Smirking Behemoth's environmental win condition.
pub fn tick_cut_rope_boss_arena(
    world_time: Res<crate::WorldTime>,
    room_set: Res<RoomSet>,
    mut state: ResMut<CutRopeBossArenaState>,
    mut hit_events: MessageReader<HitEvent>,
    mut reset_events: MessageReader<ResetRoomFeaturesEvent>,
    mut bosses: Query<(&FeatureAabb, &mut BossFeature), With<FeatureSimEntity>>,
    mut prop_visuals: Query<(
        &FeatureName,
        &PropVisual,
        &mut Transform,
        Option<&mut Visibility>,
    )>,
    mut boss_registry: Option<ResMut<BossEncounterRegistry>>,
    mut music_request: Option<ResMut<crate::encounter::BossEncounterMusicRequest>>,
    mut cutscene_queue: Option<ResMut<crate::presentation::cutscene::CutsceneTriggerQueue>>,
    mut banner: ResMut<GameplayBanner>,
    mut sfx: MessageWriter<SfxMessage>,
    mut vfx: MessageWriter<VfxMessage>,
    mut explosions: MessageWriter<ExplosionRequest>,
    mut fireworks: MessageWriter<FireworksRequest>,
    mut debris: MessageWriter<DebrisBurstMessage>,
) {
    let room = room_set.active_spec();
    if room.id != CUT_ROPE_ROOM_ID {
        if state.active_room != room.id {
            *state = CutRopeBossArenaState {
                active_room: room.id.clone(),
                ..Default::default()
            };
        }
        // Advance the readers so old slash/reset messages do not get interpreted if
        // the player warps into the cut-rope room on the next frame.
        for _ in hit_events.read() {}
        for _ in reset_events.read() {}
        return;
    }
    if state.active_room != room.id {
        *state = CutRopeBossArenaState {
            active_room: room.id.clone(),
            ..Default::default()
        };
    }

    let reset_requested = reset_events.read().next().is_some();
    if reset_requested {
        *state = CutRopeBossArenaState {
            active_room: room.id.clone(),
            ..Default::default()
        };
    }

    let Some(rope) = authored_prop(room_set.active_props(), ROPE_KIND) else {
        for _ in hit_events.read() {}
        return;
    };
    let Some(anvil) = authored_prop(room_set.active_props(), ANVIL_KIND) else {
        for _ in hit_events.read() {}
        return;
    };

    let rope_aabb = prop_aabb(rope);
    for event in hit_events.read() {
        if state.rope_cut {
            continue;
        }
        if !matches!(&event.source, HitSource::PlayerSlash { .. }) {
            continue;
        }
        if !event.volume.strict_intersects(rope_aabb) {
            continue;
        }
        state.rope_cut = true;
        state.awaiting_alignment = true;
        state.anvil_center = Some(anvil.pos);
        state.anvil_velocity_y = 0.0;
        state.rope_fx_timer = 0.0;
        state.rope_fx_pulse = 0;
        vfx.write(VfxMessage::Impact {
            pos: event.volume.center(),
        });
        vfx.write(VfxMessage::Burst {
            pos: rope.pos,
            count: 14,
            speed: 160.0,
            color: [0.90, 0.82, 0.58, 0.78],
            kind: ParticleKind::Shard,
        });
        sfx.write(SfxMessage::Slash { pos: rope.pos });
    }

    sync_cut_rope_prop_visuals(&mut prop_visuals, room_set.active_world(), &state, anvil);

    if state.kill_sent || !state.rope_cut {
        return;
    }

    let dt = world_time.sim_dt().max(0.0);
    let Some(mut center) = state.anvil_center else {
        return;
    };

    let mut boss_under_anvil = false;
    let mut live_boss_pos = None;
    for (_aabb, feature) in bosses.iter_mut() {
        let boss = &feature.boss;
        if !is_cut_rope_boss(&boss.behavior.id) || !boss.alive {
            continue;
        }
        live_boss_pos = Some(boss.pos);
        if boss_is_under_anvil(boss, center.x) {
            boss_under_anvil = true;
            break;
        }
    }

    if !boss_under_anvil {
        state.awaiting_alignment = true;
        state.anvil_velocity_y = 0.0;
        pulse_waiting_rope_explosions(
            &mut state,
            dt,
            rope.pos,
            live_boss_pos.unwrap_or(rope.pos),
            &mut explosions,
        );
        return;
    }

    state.awaiting_alignment = false;
    state.anvil_velocity_y =
        (state.anvil_velocity_y + ANVIL_GRAVITY * dt).min(ANVIL_TERMINAL_SPEED);
    center.y += state.anvil_velocity_y * dt;
    state.anvil_center = Some(center);

    let anvil_aabb = ae::Aabb::new(center, anvil.size * 0.5);
    let floor_y = room.world.size.y - anvil.size.y * 0.5;
    if center.y > floor_y {
        state.anvil_center = Some(ae::Vec2::new(center.x, floor_y));
        state.anvil_velocity_y = 0.0;
    }

    for (_aabb, mut feature) in &mut bosses {
        let boss = &mut feature.boss;
        if !is_cut_rope_boss(&boss.behavior.id) || !boss.alive {
            continue;
        }
        if !anvil_aabb.strict_intersects(boss.aabb()) {
            continue;
        }
        state.kill_sent = true;
        state.anvil_exploded = true;
        boss.alive = false;
        boss.health.current = 0;
        // The death animation should render as-authored. A lingering
        // hit-flash overlay reads as a white silhouette stuck over the body.
        boss.hit_flash = 0.0;

        if let (Some(registry), Some(music), Some(cutscene)) = (
            boss_registry.as_deref_mut(),
            music_request.as_deref_mut(),
            cutscene_queue.as_deref_mut(),
        ) {
            let _ = force_boss_death(registry, music, cutscene, &mut banner, boss.id.as_str());
        }

        banner.show("Smirking Behemoth was flattened".to_string(), 2.8);
        explosions.write(ExplosionRequest::classic(center).with_scale(1.25));
        if !state.death_fireworks_sent {
            let mut death_show = FireworksRequest::around(boss.pos);
            death_show.count = 18;
            death_show.spread = ae::Vec2::new(420.0, 280.0);
            death_show.duration = 2.75;
            fireworks.write(death_show);
            state.death_fireworks_sent = true;
        }
        vfx.write(VfxMessage::Burst {
            pos: boss.pos,
            count: 28,
            speed: 260.0,
            color: [0.84, 0.95, 1.0, 0.86],
            kind: ParticleKind::Spark,
        });
        debris.write(DebrisBurstMessage {
            pos: boss.pos,
            cue: PhysicsDebrisCue::BossRagdoll,
        });
        break;
    }
}

/// After the rope is cut, override the boss brain output with a horizontal
/// lure toward the authored anvil center until impact. The movement still goes
/// through `BossRuntime::integrate_body`, so authored solids and future
/// player-control constraints remain authoritative.
pub fn steer_cut_rope_boss_under_anvil(
    state: Res<CutRopeBossArenaState>,
    mut bosses: Query<
        (&BossFeature, &mut ActorControl, &mut BossAttackState),
        With<FeatureSimEntity>,
    >,
) {
    if state.active_room != CUT_ROPE_ROOM_ID || !state.rope_cut || state.kill_sent {
        return;
    }
    let Some(center) = state.anvil_center else {
        return;
    };
    for (feature, mut control, mut attack_state) in &mut bosses {
        let boss = &feature.boss;
        if !boss.alive || !is_cut_rope_boss(&boss.behavior.id) {
            continue;
        }
        let dx = center.x - boss.pos.x;
        attack_state.clear();
        control.0.melee_pressed = false;
        control.0.special_pressed = false;
        control.0.facing = if dx.abs() > 2.0 { dx.signum() } else { boss.facing };
        control.0.desired_vel = if dx.abs() <= boss_alignment_tolerance(boss) {
            ae::Vec2::ZERO
        } else {
            ae::Vec2::new(dx.signum() * ROPE_LURE_SPEED, 0.0)
        };
    }
}

fn boss_is_under_anvil(boss: &BossRuntime, anvil_x: f32) -> bool {
    (boss.pos.x - anvil_x).abs() <= boss_alignment_tolerance(boss)
}

fn boss_alignment_tolerance(boss: &BossRuntime) -> f32 {
    ROPE_ALIGNMENT_TOLERANCE.max(boss.combat_size().x * 0.18)
}

fn pulse_waiting_rope_explosions(
    state: &mut CutRopeBossArenaState,
    dt: f32,
    rope_pos: ae::Vec2,
    boss_pos: ae::Vec2,
    explosions: &mut MessageWriter<ExplosionRequest>,
) {
    state.rope_fx_timer -= dt;
    if state.rope_fx_timer > 0.0 {
        return;
    }
    state.rope_fx_timer = ROPE_SPARK_INTERVAL;
    let i = state.rope_fx_pulse;
    state.rope_fx_pulse = state.rope_fx_pulse.wrapping_add(1);
    let horizontal_pull = (boss_pos.x - rope_pos.x).clamp(-80.0, 80.0) * 0.18;
    let x = (((i.wrapping_mul(37).wrapping_add(11)) % 101) as f32 / 100.0 - 0.5) * 44.0;
    let y = -16.0 - ((i.wrapping_mul(53).wrapping_add(7)) % 59) as f32;
    let kind = match i % 5 {
        0 => ExplosionKind::Starburst,
        1 => ExplosionKind::ClassicBurst,
        2 => ExplosionKind::BurstRound,
        3 => ExplosionKind::Shockwave,
        _ => ExplosionKind::SmokeBurst,
    };
    explosions.write(
        ExplosionRequest::new(rope_pos + ae::Vec2::new(horizontal_pull + x, y), kind)
            .with_scale(0.48),
    );
}

fn authored_prop<'a>(props: &'a [PropSpec], kind: &str) -> Option<&'a PropSpec> {
    props
        .iter()
        .find(|prop| prop.kind == kind || prop.name == kind)
}

fn prop_aabb(prop: &PropSpec) -> ae::Aabb {
    ae::Aabb::new(prop.pos, prop.size * 0.5)
}

fn sync_cut_rope_prop_visuals(
    prop_visuals: &mut Query<(
        &FeatureName,
        &PropVisual,
        &mut Transform,
        Option<&mut Visibility>,
    )>,
    world: &ae::World,
    state: &CutRopeBossArenaState,
    anvil: &PropSpec,
) {
    for (name, prop, mut transform, visibility) in prop_visuals.iter_mut() {
        let key_matches = |needle: &str| prop.kind == needle || name.0 == needle;
        if key_matches(ROPE_KIND) {
            if let Some(mut visibility) = visibility {
                *visibility = if state.rope_cut {
                    Visibility::Hidden
                } else {
                    Visibility::Visible
                };
            }
        } else if key_matches(ANVIL_KIND) {
            if let Some(mut visibility) = visibility {
                *visibility = if state.anvil_exploded {
                    Visibility::Hidden
                } else {
                    Visibility::Visible
                };
            }
            if state.anvil_exploded {
                continue;
            }
            if let Some(center) = state.anvil_center {
                let mut translation = world_to_bevy(world, center, transform.translation.z);
                translation.z = transform.translation.z + ANVIL_Z_OFFSET;
                transform.translation = translation;
            } else {
                let mut translation = world_to_bevy(world, anvil.pos, transform.translation.z);
                translation.z = transform.translation.z;
                transform.translation = translation;
            }
        }
    }
}
