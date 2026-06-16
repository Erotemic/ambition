//! Cut-rope boss arena runtime: the heavy-object (anvil/piano) drop cycle
//! state, its per-tick simulation, prop visuals, and under-anvil steering.
//!
//! Split out of the former 793-line `cut_rope.rs` (2026-06-15).

use super::*;

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
    world_time: Res<ambition_sandbox::WorldTime>,
    room_set: Res<RoomSet>,
    mut state: ResMut<CutRopeBossArenaState>,
    heavy_object: Res<CutRopeHeavyObjectCycle>,
    mut hit_events: MessageReader<HitEvent>,
    mut reset_events: MessageReader<ResetRoomFeaturesEvent>,
    mut bosses: Query<(&CenteredAabb, BossClusterQueryData), With<FeatureSimEntity>>,
    mut boss_registry: Option<ResMut<BossEncounterRegistry>>,
    mut music_request: Option<ResMut<ambition_sandbox::encounter::BossEncounterMusicRequest>>,
    mut cutscene_queue: Option<ResMut<ambition_render::cutscene::CutsceneTriggerQueue>>,
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
            reset_cut_rope_arena_state_for_room(&mut *state, &room.id);
        }
        // Advance the readers so old slash/reset messages do not get interpreted if
        // the player warps into the cut-rope room on the next frame.
        for _ in hit_events.read() {}
        for _ in reset_events.read() {}
        return;
    }
    if state.active_room != room.id {
        reset_cut_rope_arena_state_for_room(&mut *state, &room.id);
    }

    let reset_requested = reset_events.read().next().is_some();
    if reset_requested {
        reset_cut_rope_arena_state_for_room(&mut *state, &room.id);
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
        let boss = feature.as_boss_ref();
        if !is_cut_rope_boss(&boss.config.behavior.id) || !boss.status.alive {
            continue;
        }
        live_boss_pos = Some(boss.kin.pos);
        if boss_is_under_anvil(&boss, center.x) {
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
        if !is_cut_rope_boss(&feature.config.behavior.id) || !feature.status.alive {
            continue;
        }
        if !anvil_aabb.strict_intersects(feature.as_boss_ref().aabb()) {
            continue;
        }
        state.kill_sent = true;
        state.anvil_exploded = true;
        feature.status.alive = false;
        feature.status.health.current = 0;
        // The death animation should render as-authored. A lingering
        // hit-flash overlay reads as a white silhouette stuck over the body.
        feature.status.hit_flash = 0.0;

        if let (Some(registry), Some(music), Some(cutscene)) = (
            boss_registry.as_deref_mut(),
            music_request.as_deref_mut(),
            cutscene_queue.as_deref_mut(),
        ) {
            let _ = force_boss_death(
                registry,
                music,
                cutscene,
                &mut banner,
                feature.config.id.as_str(),
            );
        }

        banner.show(
            format!(
                "Smirking Behemoth was flattened by a {}",
                heavy_object.current().display_name()
            ),
            2.8,
        );
        explosions.write(ExplosionRequest::classic(center).with_scale(1.25));
        if !state.death_fireworks_sent {
            let mut death_show = FireworksRequest::around(feature.kin.pos);
            death_show.count = 18;
            death_show.spread = ae::Vec2::new(420.0, 280.0);
            death_show.duration = 2.75;
            fireworks.write(death_show);
            state.death_fireworks_sent = true;
        }
        vfx.write(VfxMessage::Burst {
            pos: feature.kin.pos,
            count: 28,
            speed: 260.0,
            color: [0.84, 0.95, 1.0, 0.86],
            kind: ParticleKind::Spark,
        });
        debris.write(DebrisBurstMessage {
            pos: feature.kin.pos,
            cue: PhysicsDebrisCue::BossRagdoll,
        });
        break;
    }
}

/// Keep the authored rope/heavy-object prop visuals in sync with the cut-rope
/// arena state. This is intentionally separate from `tick_cut_rope_boss_arena`:
/// the gameplay tick already uses Bevy's maximum practical system-parameter
/// arity after boss death/music/VFX plumbing, and adding the rendering query
/// there makes it stop satisfying `IntoSystem`/`.run_if(...)`.
pub fn sync_cut_rope_boss_arena_prop_visuals(
    room_set: Res<RoomSet>,
    state: Res<CutRopeBossArenaState>,
    heavy_object: Res<CutRopeHeavyObjectCycle>,
    mut prop_visuals: Query<(
        &FeatureName,
        &mut PropVisual,
        &mut Transform,
        &mut Sprite,
        Option<&mut CharacterAnimator>,
        Option<&mut Anchor>,
        Option<&mut Visibility>,
    )>,
    assets: Option<Res<GameAssets>>,
) {
    if state.active_room != CUT_ROPE_ROOM_ID || room_set.active_spec().id != CUT_ROPE_ROOM_ID {
        return;
    }
    let Some(anvil) = authored_prop(room_set.active_props(), ANVIL_KIND) else {
        return;
    };
    sync_cut_rope_prop_visuals(
        &mut prop_visuals,
        room_set.active_world(),
        &state,
        anvil,
        heavy_object.current(),
        assets.as_deref(),
    );
}

/// After the rope is cut, override the boss brain output with a horizontal
/// lure toward the authored anvil center until impact. The movement still goes
/// through the boss cluster's `integrate_body`, so authored solids and future
/// player-control constraints remain authoritative.
pub fn steer_cut_rope_boss_under_anvil(
    state: Res<CutRopeBossArenaState>,
    mut bosses: Query<
        (BossClusterRef, &mut ActorControl, &mut BossAttackState),
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
        let boss = feature.as_boss_ref();
        if !boss.status.alive || !is_cut_rope_boss(&boss.config.behavior.id) {
            continue;
        }
        let dx = center.x - boss.kin.pos.x;
        attack_state.clear();
        control.0.melee_pressed = false;
        control.0.special_pressed = false;
        control.0.facing = if dx.abs() > 2.0 {
            dx.signum()
        } else {
            boss.kin.facing
        };
        control.0.desired_vel = if dx.abs() <= boss_alignment_tolerance(&boss) {
            ae::Vec2::ZERO
        } else {
            ae::Vec2::new(dx.signum() * ROPE_LURE_SPEED, 0.0)
        };
    }
}

fn boss_is_under_anvil(boss: &BossRef<'_>, anvil_x: f32) -> bool {
    (boss.kin.pos.x - anvil_x).abs() <= boss_alignment_tolerance(boss)
}

fn boss_alignment_tolerance(boss: &BossRef<'_>) -> f32 {
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
        &mut PropVisual,
        &mut Transform,
        &mut Sprite,
        Option<&mut CharacterAnimator>,
        Option<&mut Anchor>,
        Option<&mut Visibility>,
    )>,
    world: &ae::World,
    state: &CutRopeBossArenaState,
    anvil: &PropSpec,
    object_kind: CutRopeHeavyObjectKind,
    assets: Option<&GameAssets>,
) {
    for (name, mut prop, mut transform, mut sprite, animator, anchor, visibility) in
        prop_visuals.iter_mut()
    {
        let key_matches = |needle: &str, prop: &PropVisual| prop.kind == needle || name.0 == needle;
        if key_matches(ROPE_KIND, &prop) {
            if let Some(mut visibility) = visibility {
                *visibility = if state.rope_cut {
                    Visibility::Hidden
                } else {
                    Visibility::Visible
                };
            }
        } else if key_matches(ANVIL_KIND, &prop) || key_matches(PIANO_KIND, &prop) {
            apply_cut_rope_heavy_object_sprite(
                &mut prop,
                &mut sprite,
                animator,
                anchor,
                anvil.size,
                object_kind,
                assets,
            );
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

fn apply_cut_rope_heavy_object_sprite(
    prop: &mut PropVisual,
    sprite: &mut Sprite,
    animator: Option<Mut<CharacterAnimator>>,
    anchor: Option<Mut<Anchor>>,
    collision: ae::Vec2,
    object_kind: CutRopeHeavyObjectKind,
    assets: Option<&GameAssets>,
) {
    let desired_kind = object_kind.prop_kind();
    if prop.kind == desired_kind {
        return;
    }
    let Some(asset) = assets.and_then(|assets| assets.characters.prop_asset_for_kind(desired_kind))
    else {
        return;
    };
    prop.kind = desired_kind.to_string();
    *sprite = build_character_sprite(asset, Vec2::new(collision.x, collision.y));
    if let Some(mut animator) = animator {
        *animator = CharacterAnimator::new(&asset.spec);
    }
    if let Some(mut anchor) = anchor {
        *anchor = feet_anchor_for(&asset.spec, Vec2::new(collision.x, collision.y));
    }
}

fn reset_cut_rope_arena_state_for_room(state: &mut CutRopeBossArenaState, room_id: &str) {
    *state = CutRopeBossArenaState {
        active_room: room_id.to_string(),
        ..Default::default()
    };
}

/// Reset cut-rope-specific prop state immediately when a same-room reset is requested.
///
/// The main arena tick is gameplay-gated because it advances the falling anvil. Dialogue
/// commands can request a room replay while gameplay is suspended, so this reset bridge runs in
/// the ungated room-reset chain and restores rope/anvil visuals on the reset frame instead of
/// relying on the combat tick to observe a short-lived reset message later.
pub fn reset_cut_rope_boss_arena_on_room_reset(
    room_set: Res<RoomSet>,
    mut state: ResMut<CutRopeBossArenaState>,
    mut heavy_object: ResMut<CutRopeHeavyObjectCycle>,
    mut reset_events: MessageReader<ResetRoomFeaturesEvent>,
    mut prop_visuals: Query<(
        &FeatureName,
        &mut PropVisual,
        &mut Transform,
        &mut Sprite,
        Option<&mut CharacterAnimator>,
        Option<&mut Anchor>,
        Option<&mut Visibility>,
    )>,
    assets: Option<Res<GameAssets>>,
) {
    if reset_events.read().next().is_none() {
        return;
    }
    let room = room_set.active_spec();
    if room.id != CUT_ROPE_ROOM_ID {
        if state.active_room != room.id {
            reset_cut_rope_arena_state_for_room(&mut *state, &room.id);
        }
        return;
    }
    heavy_object.advance();
    reset_cut_rope_arena_state_for_room(&mut *state, &room.id);
    if let Some(anvil) = authored_prop(room_set.active_props(), ANVIL_KIND) {
        sync_cut_rope_prop_visuals(
            &mut prop_visuals,
            room_set.active_world(),
            &state,
            anvil,
            heavy_object.current(),
            assets.as_deref(),
        );
    }
}
