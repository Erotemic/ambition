//! Cut-rope boss arena: the cut-rope-specific parts. Rope-cut detection (the
//! one bespoke trigger), the heavy-object (anvil/piano) prop visuals and cycle,
//! and the death flavor (sparks, explosion, fireworks).
//!
//! The fight itself is generic encounter machinery: cutting the rope fires
//! `Gate("rope_cut")`, the cut-rope `EncounterScript` lures the behemoth
//! (`CommandMoveTo` → generic `CommandedMove`) and drops the anvil
//! (`DropHazard` → generic `FallingHazard`), the hazard fires
//! `Gate("cut_rope_impact")` on contact, and the script `ForceKill`s the
//! behemoth. No anvil physics or boss steering lives here.

use super::*;

use ambition_boss_encounter::{EncounterGate, FallingHazard};
use ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance;
use ambition_sfx::SfxWriter;

/// Cut-rope visual and flavor state, one arena per live cut-rope room. It is
/// presentation: no simulation decision reads it (the rope's gate fires on
/// every hit, and the script decides what a gate does). The
/// anvil's physics lives on the generic `FallingHazard` entity;
/// `anvil_center` / `awaiting_alignment` are mirrored from it each frame for
/// the prop-visual and spark code.
///
/// Keyed by live room, not by room id: two live rooms of the arena are two
/// fights, and a rope cut in one leaves the other's rope hanging. A live room
/// that is not live any more, or is not the arena, has no entry.
#[derive(Resource, Default)]
pub struct CutRopeBossArenaState {
    arenas: std::collections::BTreeMap<LiveRoomInstance, CutRopeArena>,
}

/// One live cut-rope room's arena.
#[derive(Clone, Default)]
pub(super) struct CutRopeArena {
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

/// The live cut-rope rooms, each with its spec.
fn live_arenas<'a>(
    rooms: &'a ambition_platformer2d::world::rooms::LiveRoomSpecs,
) -> Vec<(LiveRoomInstance, &'a ambition_platformer2d::world::rooms::RoomSpec)> {
    rooms
        .live_rooms()
        .map(|(room, definition)| (room, rooms.rooms().spec(definition)))
        .filter(|(_, spec)| spec.id == CUT_ROPE_ROOM_ID)
        .collect()
}

/// Detect a melee hit on the authored rope prop of a live cut-rope room and
/// fire `Gate("rope_cut")` in that room (the cut-rope `EncounterScript` of
/// that room turns it into the lure and anvil drop). The rope cut is the only
/// cut-rope-specific trigger. The hit's room is the room it names, else its
/// attacker's ([`HitEvent::live_room`]).
pub fn detect_cut_rope_rope_cut(
    rooms: ambition_platformer2d::world::rooms::LiveRoomSpecs,
    mut state: ResMut<CutRopeBossArenaState>,
    mut hit_events: MessageReader<HitEvent>,
    mut reset_events: MessageReader<RoomReplayAdmitted>,
    mut sfx: SfxWriter,
    mut vfx: VfxWriter,
    mut gate_writer: MessageWriter<EncounterGate>,
) {
    // An arena whose live room is gone is forgotten; a new live room of the
    // arena starts with its rope hanging.
    let arenas: Vec<LiveRoomInstance> = live_arenas(&rooms).into_iter().map(|(room, _)| room).collect();
    state.arenas.retain(|room, _| arenas.contains(room));
    // Drained, not acted on. `reset_cut_rope_boss_arena_on_room_reset`
    // (registered in `ContentRoomResetSet`) clears the arena on this message.
    // A second retractor here would hide the other's absence: deleting either
    // would leave the end-to-end replay test green.
    //
    // The read stays: a `MessageReader` that skips a frame carries the backlog
    // into the next one.
    for _ in reset_events.read() {}

    for event in hit_events.read() {
        if !matches!(&event.source, HitSource::Melee) {
            continue;
        }
        let Some(room) = event.live_room(rooms.live()) else {
            continue;
        };
        let Some(spec) = rooms
            .definition_in(room)
            .map(|definition| rooms.rooms().spec(definition))
            .filter(|spec| spec.id == CUT_ROPE_ROOM_ID)
        else {
            continue;
        };
        let Some(rope) = authored_prop(&spec.props, ROPE_KIND) else {
            continue;
        };
        if !event.volume.intersects_aabb(prop_aabb(rope)) {
            continue;
        }
        // Every hit on the rope fires the gate. Whether the rope is cut, as a
        // fact of the fight, is the script's: only the beat that waits on
        // `rope_cut` takes it, and the script is rollback state. The arena
        // below is not, so it gates only the effects. A rewind across the cut
        // resimulates the hit into a gate again.
        gate_writer.write(EncounterGate::new("rope_cut").in_room(Some(room)));
        let arena = state.arenas.entry(room).or_default();
        if arena.rope_cut {
            continue;
        }
        arena.rope_cut = true;
        arena.awaiting_alignment = true;
        arena.rope_fx_timer = 0.0;
        arena.rope_fx_pulse = 0;
        // The cut is drawn in the arena's own live room.
        let mut vfx = vfx.for_room(Some(room));
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
}

/// Cut-rope flavor, per live cut-rope room: mirror that room's falling hazard
/// onto its arena, pulse the waiting rope sparks, and react to that room's
/// `cut_rope_impact` gate with the explosion, fireworks and banner. The kill
/// is the EncounterScript's `ForceKill`; the anvil physics is the generic
/// `FallingHazard`.
pub fn tick_cut_rope_flavor(
    world_time: Res<ambition_time::WorldTime>,
    rooms: ambition_platformer2d::world::rooms::LiveRoomSpecs,
    mut state: ResMut<CutRopeBossArenaState>,
    heavy_object: Res<CutRopeHeavyObjectCycle>,
    mut gates: MessageReader<EncounterGate>,
    hazards: Query<(Entity, &CenteredAabb, &FallingHazard)>,
    bosses: Query<(Entity, BossClusterRef), With<FeatureSimEntity>>,
    mut banner: ResMut<GameplayBanner>,
    mut explosions: MessageWriter<FxRequest>,
    mut fireworks: MessageWriter<FireworksRequest>,
    mut debris: MessageWriter<DebrisBurstMessage>,
    mut vfx: VfxWriter,
) {
    // Fully drain the gate reader (cursor hygiene) + note each room's anvil
    // impact.
    let impacted: Vec<Option<LiveRoomInstance>> = gates
        .read()
        .filter(|gate| gate.gate == "cut_rope_impact")
        .map(|gate| ambition_encounter::occurrence::message_room(rooms.live(), gate.room))
        .collect();
    let dt = world_time.sim_dt().max(0.0);
    for (room, spec) in live_arenas(&rooms) {
        let arena = state.arenas.entry(room).or_default();
        let in_room = |entity: Entity| rooms.live().of(entity) == Some(room);

        // Mirror this room's falling hazard (the anvil) onto its arena.
        if let Some((_, aabb, hazard)) = hazards.iter().find(|(hazard, _, _)| in_room(*hazard)) {
            arena.anvil_center = Some(aabb.center);
            arena.awaiting_alignment = !hazard.dropping;
        }

        let boss_pos = bosses.iter().find_map(|(entity, feature)| {
            let boss = feature.as_boss_ref();
            (in_room(entity) && is_cut_rope_boss(&boss.config.behavior.id)).then_some(boss.kin.pos)
        });

        // Waiting rope sparks while the anvil hangs unaligned.
        if arena.rope_cut && !arena.anvil_exploded && arena.awaiting_alignment {
            if let Some(rope) = authored_prop(&spec.props, ROPE_KIND) {
                let rope_pos = rope.pos;
                pulse_waiting_rope_explosions(
                    arena,
                    room,
                    dt,
                    rope_pos,
                    boss_pos.unwrap_or(rope_pos),
                    &mut explosions,
                );
            }
        }

        // The anvil hit → death flavor (the EncounterScript does the actual kill).
        if impacted.contains(&Some(room)) && !arena.anvil_exploded {
            arena.anvil_exploded = true;
            let center = arena.anvil_center.or(boss_pos).unwrap_or(ae::Vec2::ZERO);
            let burst_pos = boss_pos.unwrap_or(center);
            banner.show(
                format!(
                    "Smirking Behemoth was flattened by a {}",
                    heavy_object.current().display_name()
                ),
                2.8,
            );
            explosions.write(FxRequest::classic(Some(room), center).with_scale(1.25));
            if !arena.death_fireworks_sent {
                let mut death_show = FireworksRequest::around(Some(room), burst_pos);
                death_show.count = 18;
                death_show.spread = ae::Vec2::new(420.0, 280.0);
                death_show.duration = 2.75;
                fireworks.write(death_show);
                arena.death_fireworks_sent = true;
            }
            vfx.for_room(Some(room)).write(VfxMessage::Burst {
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
}

/// Keep the authored rope and heavy-object prop visuals in sync with the arena
/// state. Separate from the gameplay systems so the rendering query does not
/// grow their parameter count. Each live cut-rope room draws its own arena:
/// its props are the prop visuals in that live room.
pub fn sync_cut_rope_boss_arena_prop_visuals(
    rooms: ambition_platformer2d::world::rooms::LiveRoomSpecs,
    state: Res<CutRopeBossArenaState>,
    heavy_object: Res<CutRopeHeavyObjectCycle>,
    mut prop_visuals: Query<(
        Entity,
        &mut PropVisual,
        &mut Transform,
        &mut Sprite,
        Option<&mut CharacterAnimator>,
        Option<&mut Anchor>,
        Option<&mut Visibility>,
    )>,
    assets: Option<Res<GameAssets>>,
) {
    for (room, spec) in live_arenas(&rooms) {
        let Some(arena) = state.arenas.get(&room) else {
            continue;
        };
        let Some(anvil) = authored_prop(&spec.props, ANVIL_KIND) else {
            continue;
        };
        sync_cut_rope_prop_visuals(
            &mut prop_visuals,
            |prop| rooms.live().of(prop) == Some(room),
            &spec.world,
            arena,
            anvil,
            heavy_object.current(),
            assets.as_deref(),
        );
    }
}

fn pulse_waiting_rope_explosions(
    state: &mut CutRopeArena,
    // The arena's live room: the sparks are drawn there.
    room: LiveRoomInstance,
    dt: f32,
    rope_pos: ae::Vec2,
    boss_pos: ae::Vec2,
    explosions: &mut MessageWriter<FxRequest>,
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
    let fx = match i % 5 {
        0 => ambition_vfx::fx::ids::STARBURST,
        1 => ambition_vfx::fx::ids::CLASSIC_BURST,
        2 => ambition_vfx::fx::ids::BURST_ROUND,
        3 => ambition_vfx::fx::ids::SHOCKWAVE,
        _ => ambition_vfx::fx::ids::SMOKE_BURST,
    };
    explosions.write(
        FxRequest::new(Some(room), rope_pos + ae::Vec2::new(horizontal_pull + x, y), fx)
            .with_scale(0.48),
    );
}

/// Is this visual the trap's heavy object: the thing that hangs, drops and
/// squashes the boss?
///
/// By the authored iid, never by `kind`. `apply_cut_rope_heavy_object_sprite`
/// writes `PropVisual.kind` when the trap cycles anvil → piano, so `kind`
/// answers "what is it drawn as now", not "which prop is it". The caption
/// (`prop.name`) is not an identity either. A third heavy object needs no new
/// arm here.
pub(super) fn is_heavy_object(prop: &PropVisual, anvil: &PropSpec) -> bool {
    prop.id == anvil.id
}

/// The authored prop of a given kind.
///
/// `PropSpec.kind` is the identity; `PropSpec.name` is not. `PropSpec`'s doc
/// calls `name` the "LDtk display name — authors edit this; the renderer uses
/// it only for entity naming / debug overlay", so matching it would let a
/// caption name a prop the fight depends on.
pub(super) fn authored_prop<'a>(props: &'a [PropSpec], kind: &str) -> Option<&'a PropSpec> {
    props.iter().find(|prop| prop.kind == kind)
}

fn prop_aabb(prop: &PropSpec) -> ae::Aabb {
    ae::Aabb::new(prop.pos, prop.size * 0.5)
}

fn sync_cut_rope_prop_visuals(
    prop_visuals: &mut Query<(
        Entity,
        &mut PropVisual,
        &mut Transform,
        &mut Sprite,
        Option<&mut CharacterAnimator>,
        Option<&mut Anchor>,
        Option<&mut Visibility>,
    )>,
    // Whether a prop visual is in the arena's live room. Two live rooms of
    // the arena hold the same props.
    in_arena: impl Fn(Entity) -> bool,
    world: &ae::World,
    state: &CutRopeArena,
    anvil: &PropSpec,
    object_kind: CutRopeHeavyObjectKind,
    assets: Option<&GameAssets>,
) {
    for (entity, mut prop, mut transform, mut sprite, animator, anchor, visibility) in
        prop_visuals.iter_mut()
    {
        if !in_arena(entity) {
            continue;
        }
        // The heavy object is matched by its authored id, not by `kind`:
        // `apply_cut_rope_heavy_object_sprite` writes `prop.kind` when the trap
        // cycles anvil -> piano, so `kind` is what it is drawn as now, not an
        // identity.
        //
        // `PropVisual.id` is the LDtk iid the renderer copies from the authored
        // `PropSpec`. It is the one field here that is both stable and unique.
        //
        // The rope keeps a `kind` test: nothing re-skins the rope.
        if prop.kind == ROPE_KIND {
            if let Some(mut visibility) = visibility {
                *visibility = if state.rope_cut {
                    Visibility::Hidden
                } else {
                    Visibility::Visible
                };
            }
        } else if is_heavy_object(&prop, anvil) {
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

/// Reset cut-rope-specific prop state immediately when a same-room reset is requested.
///
/// The main flavor tick is gameplay-gated. Dialogue commands can request a
/// room replay while gameplay is suspended, so this runs in the ungated
/// room-reset chain and restores rope/anvil visuals on the reset frame.
///
/// The room reset is the live room of the replay's subject (OW1 Cut A); a
/// replay with no subject, or of an unstamped subject, is of the sole live
/// room.
pub fn reset_cut_rope_boss_arena_on_room_reset(
    rooms: ambition_platformer2d::world::rooms::LiveRoomSpecs,
    live: ambition_platformer2d_shared_tangle::lifecycle::LiveRooms,
    mut state: ResMut<CutRopeBossArenaState>,
    mut heavy_object: ResMut<CutRopeHeavyObjectCycle>,
    mut reset_events: MessageReader<RoomReplayAdmitted>,
    mut prop_visuals: Query<(
        Entity,
        &mut PropVisual,
        &mut Transform,
        &mut Sprite,
        Option<&mut CharacterAnimator>,
        Option<&mut Anchor>,
        Option<&mut Visibility>,
    )>,
    assets: Option<Res<GameAssets>>,
) {
    let Some(subject_room) = reset_events
        .read()
        .next()
        .map(|admitted| admitted.subject.as_ref().and_then(|subject| subject.room))
    else {
        return;
    };
    let Some(replayed) = subject_room.or_else(|| live.sole()) else {
        return;
    };
    let Some(definition) = rooms.definition_in(replayed) else {
        return;
    };
    let spec = rooms.rooms().spec(definition);
    if spec.id != CUT_ROPE_ROOM_ID {
        state.arenas.remove(&replayed);
        return;
    }
    heavy_object.advance();
    let arena = CutRopeArena::default();
    state.arenas.insert(replayed, arena.clone());
    if let Some(anvil) = authored_prop(&spec.props, ANVIL_KIND) {
        sync_cut_rope_prop_visuals(
            &mut prop_visuals,
            |prop| live.of(prop) == Some(replayed),
            &spec.world,
            &arena,
            anvil,
            heavy_object.current(),
            assets.as_deref(),
        );
    }
}
