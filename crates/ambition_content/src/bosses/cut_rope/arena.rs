//! Cut-rope boss arena: the cut-rope-SPECIFIC bits — rope-cut detection (the one
//! bespoke trigger), the heavy-object (anvil/piano) prop visuals + cycle, and the
//! death flavor (sparks / explosion / fireworks).
//!
//! The actual FIGHT is the generic encounter machinery (R5): cutting the rope
//! fires `Gate("rope_cut")`, the cut-rope `EncounterScript` lures the behemoth
//! (`CommandMoveTo` → generic `CommandedMove`) + drops the anvil (`DropHazard` →
//! generic `FallingHazard`), the hazard fires `Gate("cut_rope_impact")` on
//! contact, and the script `ForceKill`s the behemoth. This file no longer
//! contains any anvil physics or boss steering — those are reusable mechanics.
//!
//! Split out of the former 793-line `cut_rope.rs` (2026-06-15); the bespoke
//! physics was lifted into the generic encounter mechanic (2026-06-23).

use super::*;

use ambition_gameplay_core::boss_encounter::{EncounterGate, FallingHazard};

/// Cut-rope VISUAL/FLAVOR state. The anvil's PHYSICS lives on the generic
/// `FallingHazard` entity now; `anvil_center` / `awaiting_alignment` are mirrored
/// from it each frame so the existing prop-visual + spark code is unchanged.
#[derive(Resource, Default)]
pub struct CutRopeBossArenaState {
    active_room: String,
    rope_cut: bool,
    /// Mirror of "the hazard is still waiting for the boss to align" (drives the
    /// waiting rope sparks).
    awaiting_alignment: bool,
    /// Mirror of the falling hazard's center (drives the anvil prop visual).
    anvil_center: Option<ae::Vec2>,
    anvil_exploded: bool,
    rope_fx_timer: f32,
    rope_fx_pulse: u32,
    death_fireworks_sent: bool,
}

/// Detect the player slashing the authored rope prop → fire `Gate("rope_cut")`
/// (the cut-rope `EncounterScript` turns that into the lure + anvil drop). The
/// rope-cut is the ONLY cut-rope-specific trigger; everything after it is the
/// generic encounter script + falling-hazard mechanic. Also owns the cut-rope
/// state reset on room enter/exit + room-feature reset.
pub fn detect_cut_rope_rope_cut(
    room_set: Res<RoomSet>,
    mut state: ResMut<CutRopeBossArenaState>,
    mut hit_events: MessageReader<HitEvent>,
    mut reset_events: MessageReader<ResetRoomFeaturesEvent>,
    mut sfx: MessageWriter<SfxMessage>,
    mut vfx: MessageWriter<VfxMessage>,
    mut gate_writer: MessageWriter<EncounterGate>,
) {
    let room = room_set.active_spec();
    if room.id != CUT_ROPE_ROOM_ID {
        if state.active_room != room.id {
            reset_cut_rope_arena_state_for_room(&mut state, &room.id);
        }
        // Drain readers so stale slash/reset messages don't fire on room entry.
        for _ in hit_events.read() {}
        for _ in reset_events.read() {}
        return;
    }
    if state.active_room != room.id {
        reset_cut_rope_arena_state_for_room(&mut state, &room.id);
    }
    if reset_events.read().next().is_some() {
        reset_cut_rope_arena_state_for_room(&mut state, &room.id);
    }

    let Some(rope) = authored_prop(room_set.active_props(), ROPE_KIND) else {
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
        if !event.volume.intersects_aabb(rope_aabb) {
            continue;
        }
        state.rope_cut = true;
        state.awaiting_alignment = true;
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
        // Hand off to the generic encounter script (lure + drop the anvil).
        gate_writer.write(EncounterGate::new("rope_cut"));
    }
}

/// Cut-rope FLAVOR: mirror the generic falling hazard onto the visual state,
/// pulse the waiting rope sparks, and react to the `cut_rope_impact` gate with
/// the explosion / fireworks / banner. The KILL itself is the EncounterScript's
/// `ForceKill`; the anvil PHYSICS is the generic `FallingHazard`.
pub fn tick_cut_rope_flavor(
    world_time: Res<ambition_time::WorldTime>,
    room_set: Res<RoomSet>,
    mut state: ResMut<CutRopeBossArenaState>,
    heavy_object: Res<CutRopeHeavyObjectCycle>,
    mut gates: MessageReader<EncounterGate>,
    hazards: Query<(&CenteredAabb, &FallingHazard)>,
    bosses: Query<BossClusterRef, With<FeatureSimEntity>>,
    mut banner: ResMut<GameplayBanner>,
    mut explosions: MessageWriter<ExplosionRequest>,
    mut fireworks: MessageWriter<FireworksRequest>,
    mut debris: MessageWriter<DebrisBurstMessage>,
    mut vfx: MessageWriter<VfxMessage>,
) {
    // Fully drain the gate reader (cursor hygiene) + note an anvil impact.
    let mut impacted = false;
    for gate in gates.read() {
        if gate.gate == "cut_rope_impact" {
            impacted = true;
        }
    }
    if room_set.active_spec().id != CUT_ROPE_ROOM_ID {
        return;
    }

    // Mirror the falling hazard (the anvil) onto the visual state.
    if let Some((aabb, hazard)) = hazards.iter().next() {
        state.anvil_center = Some(aabb.center);
        state.awaiting_alignment = !hazard.dropping;
    }

    let boss_pos = bosses.iter().find_map(|feature| {
        let boss = feature.as_boss_ref();
        is_cut_rope_boss(&boss.config.behavior.id).then_some(boss.kin.pos)
    });

    // Waiting rope sparks while the anvil hangs unaligned.
    if state.rope_cut && !state.anvil_exploded && state.awaiting_alignment {
        if let Some(rope) = authored_prop(room_set.active_props(), ROPE_KIND) {
            let dt = world_time.sim_dt().max(0.0);
            let rope_pos = rope.pos;
            pulse_waiting_rope_explosions(
                &mut state,
                dt,
                rope_pos,
                boss_pos.unwrap_or(rope_pos),
                &mut explosions,
            );
        }
    }

    // The anvil hit → death flavor (the EncounterScript does the actual kill).
    if impacted && !state.anvil_exploded {
        state.anvil_exploded = true;
        let center = state.anvil_center.or(boss_pos).unwrap_or(ae::Vec2::ZERO);
        let burst_pos = boss_pos.unwrap_or(center);
        banner.show(
            format!(
                "Smirking Behemoth was flattened by a {}",
                heavy_object.current().display_name()
            ),
            2.8,
        );
        explosions.write(ExplosionRequest::classic(center).with_scale(1.25));
        if !state.death_fireworks_sent {
            let mut death_show = FireworksRequest::around(burst_pos);
            death_show.count = 18;
            death_show.spread = ae::Vec2::new(420.0, 280.0);
            death_show.duration = 2.75;
            fireworks.write(death_show);
            state.death_fireworks_sent = true;
        }
        vfx.write(VfxMessage::Burst {
            pos: burst_pos,
            count: 28,
            speed: 260.0,
            color: [0.84, 0.95, 1.0, 0.86],
            kind: ParticleKind::Spark,
        });
        debris.write(DebrisBurstMessage {
            pos: burst_pos,
            cue: PhysicsDebrisCue::BossRagdoll,
        });
    }
}

/// Keep the authored rope/heavy-object prop visuals in sync with the cut-rope
/// arena state. This is intentionally separate from the gameplay systems so the
/// rendering query doesn't bloat their parameter arity.
pub fn sync_cut_rope_boss_arena_prop_visuals(
    room_set: Res<RoomSet>,
    state: Res<CutRopeBossArenaState>,
    heavy_object: Res<CutRopeHeavyObjectCycle>,
    mut prop_visuals: Query<(
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
    for (mut prop, mut transform, mut sprite, animator, anchor, visibility) in
        prop_visuals.iter_mut()
    {
        let key_matches =
            |needle: &str, prop: &PropVisual| prop.kind == needle || prop.name == needle;
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
        *animator = CharacterAnimator::new(asset);
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
/// The main flavor tick is gameplay-gated. Dialogue commands can request a room
/// replay while gameplay is suspended, so this reset bridge runs in the ungated
/// room-reset chain and restores rope/anvil visuals on the reset frame.
pub fn reset_cut_rope_boss_arena_on_room_reset(
    room_set: Res<RoomSet>,
    mut state: ResMut<CutRopeBossArenaState>,
    mut heavy_object: ResMut<CutRopeHeavyObjectCycle>,
    mut reset_events: MessageReader<ResetRoomFeaturesEvent>,
    mut prop_visuals: Query<(
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
            reset_cut_rope_arena_state_for_room(&mut state, &room.id);
        }
        return;
    }
    heavy_object.advance();
    reset_cut_rope_arena_state_for_room(&mut state, &room.id);
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
