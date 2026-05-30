//! Cut-rope boss arena rules.
//!
//! The arena is authored in LDtk as ordinary `Prop` entities named/kinded
//! `cut_rope_rope` and `cut_rope_anvil`, plus a `BossSpawn` whose behavior id
//! is `smirking_behemoth_boss`. This system keeps the one-off mechanic tied to
//! authored level data rather than hard-coded coordinates: cutting the rope prop
//! starts the anvil prop falling; the anvil impact forces the boss encounter
//! through the normal death pipeline.

use bevy::prelude::*;

use crate::audio::SfxMessage;
use crate::boss_encounter::{force_boss_death, BossEncounterRegistry};
use crate::config::world_to_bevy;
use crate::engine_core::{self as ae, AabbExt};
use crate::features::{
    BossFeature, FeatureAabb, FeatureName, FeatureSimEntity, GameplayBanner, HitEvent, HitSource,
};
use crate::presentation::fx::{ParticleKind, VfxMessage};
use crate::presentation::rendering::PropVisual;
use crate::world::physics::{DebrisBurstMessage, PhysicsDebrisCue};
use crate::rooms::{PropSpec, RoomSet};

pub const CUT_ROPE_BOSS_ID: &str = "smirking_behemoth_boss";
const CUT_ROPE_ROOM_ID: &str = "you_have_to_cut_the_rope";
const ROPE_KIND: &str = "cut_rope_rope";
const ANVIL_KIND: &str = "cut_rope_anvil";
const ANVIL_GRAVITY: f32 = 1400.0;
const ANVIL_TERMINAL_SPEED: f32 = 920.0;
const ANVIL_Z_OFFSET: f32 = 0.75;

pub fn is_cut_rope_boss(id: &str) -> bool {
    id == CUT_ROPE_BOSS_ID
}

#[derive(Default)]
pub struct CutRopeBossArenaState {
    active_room: String,
    rope_cut: bool,
    anvil_center: Option<ae::Vec2>,
    anvil_velocity_y: f32,
    kill_sent: bool,
}

/// Drive the Smirking Behemoth's environmental win condition.
pub fn tick_cut_rope_boss_arena(
    world_time: Res<crate::WorldTime>,
    room_set: Res<RoomSet>,
    mut state: Local<CutRopeBossArenaState>,
    mut hit_events: MessageReader<HitEvent>,
    mut bosses: Query<(&FeatureAabb, &mut BossFeature), With<FeatureSimEntity>>,
    mut prop_visuals: Query<(&FeatureName, &PropVisual, &mut Transform, Option<&mut Visibility>)>,
    mut boss_registry: Option<ResMut<BossEncounterRegistry>>,
    mut music_request: Option<ResMut<crate::encounter::BossEncounterMusicRequest>>,
    mut cutscene_queue: Option<ResMut<crate::presentation::cutscene::CutsceneTriggerQueue>>,
    mut banner: ResMut<GameplayBanner>,
    mut sfx: MessageWriter<SfxMessage>,
    mut vfx: MessageWriter<VfxMessage>,
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
        // Advance the reader so old slash messages do not get interpreted if
        // the player warps into the cut-rope room on the next frame.
        for _ in hit_events.read() {}
        return;
    }
    if state.active_room != room.id {
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
        state.anvil_center = Some(anvil.pos);
        state.anvil_velocity_y = 0.0;
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
    state.anvil_velocity_y = (state.anvil_velocity_y + ANVIL_GRAVITY * dt).min(ANVIL_TERMINAL_SPEED);
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
        boss.alive = false;
        boss.health.current = 0;
        boss.hit_flash = boss.hit_flash.max(0.35);

        if let (Some(registry), Some(music), Some(cutscene)) = (
            boss_registry.as_deref_mut(),
            music_request.as_deref_mut(),
            cutscene_queue.as_deref_mut(),
        ) {
            let _ = force_boss_death(registry, music, cutscene, &mut banner, boss.id.as_str());
        }

        banner.show("Smirking Behemoth was flattened".to_string(), 2.8);
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
        sfx.write(SfxMessage::Death { pos: boss.pos });
        break;
    }
}

fn authored_prop<'a>(props: &'a [PropSpec], kind: &str) -> Option<&'a PropSpec> {
    props.iter().find(|prop| prop.kind == kind || prop.name == kind)
}

fn prop_aabb(prop: &PropSpec) -> ae::Aabb {
    ae::Aabb::new(prop.pos, prop.size * 0.5)
}

fn sync_cut_rope_prop_visuals(
    prop_visuals: &mut Query<(&FeatureName, &PropVisual, &mut Transform, Option<&mut Visibility>)>,
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
