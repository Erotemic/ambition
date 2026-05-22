//! Falling-sand prototype room integration.
//!
//! The LDtk-authored `falling_sand_room` contains four switches whose save
//! flags gate particle spouts. This module bridges the particle sim back into
//! Ambition's movement world by projecting dense sand/liquid tiles into one-way
//! platforms and temporary water regions before the player simulation runs.

use std::collections::{HashMap, HashSet};

use ambition_engine as ae;
use bevy::prelude::*;
use bevy_falling_sand::prelude::*;

const ROOM_ID: &str = "falling_sand_room";

const TYPE_SAND: &str = "AmbitionSand";
const TYPE_WATER: &str = "AmbitionWater";
const TYPE_OIL: &str = "AmbitionOil";
const TYPE_WALL: &str = "AmbitionWall";

const SAND_SWITCH: &str = "falling_sand_sand_switch";
const WATER_SWITCH: &str = "falling_sand_water_switch";
const OIL_SWITCH: &str = "falling_sand_oil_switch";
const MIXED_SWITCH: &str = "falling_sand_mixed_switch";

const TILE_SIZE: i32 = 16;
/// Floor / side-wall thickness in particle cells. Needs to be deep
/// enough that high-density material can't tunnel through during a
/// single sim step; 16 has held up in practice where 2 did not.
const FLOOR_WALL_THICKNESS: i32 = 16;
const SIDE_WALL_THICKNESS: i32 = 8;
const SAND_THRESHOLD: usize = 14;
const LIQUID_THRESHOLD: usize = 10;
const MATERIAL_VISUAL_THRESHOLD: usize = 6;
const MAX_DYNAMIC_SAND_TILES: usize = 220;
const MAX_DYNAMIC_LIQUID_TILES: usize = 180;

#[derive(Resource, Default)]
struct FallingSandRoomState {
    active_room: bool,
    last_room_id: Option<String>,
    base_blocks: Vec<ae::Block>,
    base_water_regions: Vec<ae::WaterRegion>,
    /// Snapshot of the active player's swim ability at the moment the
    /// room was entered. Restored on exit so the room's forced-swim
    /// effect doesn't leak into other rooms.
    ///
    /// Stored as a single value (not keyed by `Entity`) because the
    /// sandbox is single-player; an Entity-keyed map would leak
    /// entries every time the player respawned with a new Entity id
    /// while still inside the room.
    swim_snapshot: Option<SwimSnapshot>,
    seeded_boundaries: bool,
    spouts: FallingSandSpoutState,
    visual_emit_counter: u32,
}

/// Stored player swim state plus a marker so we can tell whether the
/// snapshot belongs to the currently spawned player entity. If the
/// player respawns inside the room, the previous snapshot becomes
/// stale and we re-capture from the new entity's current swim state.
#[derive(Clone, Copy, Debug)]
struct SwimSnapshot {
    player_entity: Entity,
    previous_swim: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct FallingSandSpoutState {
    sand: bool,
    water: bool,
    oil: bool,
    mixed: bool,
}

impl FallingSandSpoutState {
    fn from_save(save: &ae::SandboxSaveData) -> Self {
        Self {
            sand: save.switch(SAND_SWITCH),
            water: save.switch(WATER_SWITCH),
            oil: save.switch(OIL_SWITCH),
            mixed: save.switch(MIXED_SWITCH),
        }
    }

    fn toggle(&mut self, id: &str) -> bool {
        match id {
            SAND_SWITCH => {
                self.sand = !self.sand;
                true
            }
            WATER_SWITCH => {
                self.water = !self.water;
                true
            }
            OIL_SWITCH => {
                self.oil = !self.oil;
                true
            }
            MIXED_SWITCH => {
                self.mixed = !self.mixed;
                true
            }
            _ => false,
        }
    }
}

#[derive(Component)]
struct FallingSandMaterialVisual {
    tile: (i32, i32),
    kind: MaterialKind,
}

#[derive(Component)]
struct FallingSandStreamParticle {
    world_pos: ae::Vec2,
    vel: ae::Vec2,
    age: f32,
    lifetime: f32,
}

#[derive(Component)]
struct FallingSandSpoutNozzle {
    id: &'static str,
}

pub struct FallingSandRoomPlugin;

impl Plugin for FallingSandRoomPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FallingSandRoomState>()
            .add_plugins(
                FallingSandPlugin::default()
                    .with_chunk_size(64)
                    .with_map_size(32),
            )
            .add_systems(Startup, setup_particle_types)
            .add_systems(
                Update,
                (
                    sync_falling_sand_room_state,
                    seed_falling_sand_room_boundaries,
                    sync_falling_sand_spout_nozzles,
                    emit_falling_sand_spouts,
                    animate_falling_sand_stream_particles,
                    project_particles_to_movement_world,
                    grant_room_swim_controls,
                )
                    .chain()
                    .in_set(crate::app::SandboxSet::WorldPrep),
            )
            .add_systems(
                Update,
                (
                    capture_falling_sand_switch_interactions,
                    // Visual sync must run *after* the toggle handler so
                    // SwitchOn reflects the spout state we just set —
                    // otherwise the engine's "switch is latching" semantics
                    // leave the sprite stuck green while the spout flips
                    // back off, which inverts the player's mental model.
                    sync_falling_sand_switch_visuals,
                )
                    .chain()
                    .in_set(crate::app::SandboxSet::GameplayEffects),
            )
            // Diagnostic: once per second while in the falling-sand room,
            // dump per-type particle counts and Y-distribution. Lets us
            // see at a glance whether particles are being spawned,
            // whether they're reaching the floor wall band, and where
            // they're going if they vanish.
            .add_systems(Update, log_falling_sand_diagnostics);
    }
}

fn setup_particle_types(mut commands: Commands) {
    commands.spawn((
        Name::new("particle type: ambition sand"),
        ParticleType::new(TYPE_SAND),
        ColorProfile::palette(vec![
            Color::Srgba(Srgba::hex("#E2C16A").expect("valid sand color")),
            Color::Srgba(Srgba::hex("#B99045").expect("valid sand color")),
            Color::Srgba(Srgba::hex("#F1D98A").expect("valid sand color")),
        ]),
        Movement::from(vec![
            vec![IVec2::new(0, -1)],
            vec![IVec2::new(-1, -1), IVec2::new(1, -1)],
        ]),
        Density(1250),
        // Speed range chosen for visual continuity. Wide ranges
        // (e.g. 4..8 like the original) make consecutive frames'
        // emissions land 4–8 cells apart vertically, so the stream
        // reads as discrete clumps. Keeping the range tight gives a
        // continuous-looking column.
        Speed::new(3, 4),
    ));

    commands.spawn((
        Name::new("particle type: ambition water"),
        ParticleType::new(TYPE_WATER),
        ColorProfile::palette(vec![
            Color::Srgba(Srgba::hex("#4DA3FF").expect("valid water color")),
            Color::Srgba(Srgba::hex("#2E6FBF").expect("valid water color")),
            Color::Srgba(Srgba::hex("#86C7FF").expect("valid water color")),
        ]),
        Movement::from(vec![
            vec![IVec2::new(0, -1)],
            vec![IVec2::new(-1, -1), IVec2::new(1, -1)],
            vec![IVec2::new(-1, 0), IVec2::new(1, 0)],
        ]),
        Density(1000),
        Speed::new(4, 5),
    ));

    // Static wall: matches the basic.rs demo's "Dirt Wall" pattern —
    // ParticleType + ColorProfile only, no Density/Movement/Speed. In
    // bevy_falling_sand's displacement logic, a particle with no
    // Density component is unconditionally "obstructed" by any moving
    // particle that hits it, which is exactly what we want for an
    // immovable wall. Adding Density(3000) ALSO blocks sand by virtue
    // of "moving particle density < wall density → obstructed", but
    // the demo specifically uses the no-Density form, so mirror it.
    commands.spawn((
        Name::new("particle type: ambition static wall"),
        ParticleType::new(TYPE_WALL),
        ColorProfile::palette(vec![
            Color::Srgba(Srgba::hex("#253040").expect("valid wall color")),
            Color::Srgba(Srgba::hex("#34445A").expect("valid wall color")),
        ]),
    ));

    commands.spawn((
        Name::new("particle type: ambition oil"),
        ParticleType::new(TYPE_OIL),
        ColorProfile::palette(vec![
            Color::Srgba(Srgba::hex("#4C3520").expect("valid oil color")),
            Color::Srgba(Srgba::hex("#2A1E14").expect("valid oil color")),
            Color::Srgba(Srgba::hex("#80613C").expect("valid oil color")),
        ]),
        Movement::from(vec![
            vec![IVec2::new(0, -1)],
            vec![IVec2::new(-1, -1), IVec2::new(1, -1)],
            vec![IVec2::new(-1, 0), IVec2::new(1, 0)],
        ]),
        Density(850),
        // Oil falls slower than sand or water — viscous.
        Speed::new(2, 3),
    ));
}

fn sync_falling_sand_room_state(
    mut commands: Commands,
    room_set: Res<crate::rooms::RoomSet>,
    world: Res<crate::GameWorld>,
    save: Res<crate::persistence::save::SandboxSave>,
    mut state: ResMut<FallingSandRoomState>,
    particles: Query<Entity, With<Particle>>,
) {
    let active_id = room_set.active_spec().id.as_str();
    let active_room = active_id == ROOM_ID;

    if state.last_room_id.as_deref() == Some(active_id) {
        state.active_room = active_room;
        return;
    }

    state.last_room_id = Some(active_id.to_owned());
    state.active_room = active_room;

    for particle in &particles {
        commands.entity(particle).despawn();
    }

    if active_room {
        state.base_blocks = world.0.blocks.clone();
        state.base_water_regions = world.0.water_regions.clone();
        state.seeded_boundaries = false;
        state.spouts = FallingSandSpoutState::from_save(save.data());
        state.visual_emit_counter = 0;
    } else {
        state.base_blocks.clear();
        state.base_water_regions.clear();
        state.seeded_boundaries = false;
        state.spouts = FallingSandSpoutState::default();
        state.visual_emit_counter = 0;
    }
}

fn seed_falling_sand_room_boundaries(
    room_set: Res<crate::rooms::RoomSet>,
    mut state: ResMut<FallingSandRoomState>,
    mut writer: MessageWriter<SpawnParticleSignal>,
) {
    let room = room_set.active_spec();
    if room.id != ROOM_ID || state.seeded_boundaries {
        return;
    }

    let world = &room.world;

    // Side walls keep falling material inside the room.
    emit_wall_rect(
        &mut writer,
        world,
        0.0,
        0.0,
        SIDE_WALL_THICKNESS,
        world.size.y as i32,
    );
    emit_wall_rect(
        &mut writer,
        world,
        world.size.x - SIDE_WALL_THICKNESS as f32,
        0.0,
        SIDE_WALL_THICKNESS,
        world.size.y as i32,
    );
    // Cap the bottom of the world so anything that slips through a gap
    // in the LDtk floor doesn't tunnel out of the bevy_falling_sand map.
    emit_wall_rect(
        &mut writer,
        world,
        0.0,
        world.size.y - SIDE_WALL_THICKNESS as f32,
        world.size.x as i32,
        SIDE_WALL_THICKNESS,
    );

    // Mirror the LDtk room's collision blocks as wall particles so
    // material piles ON TOP of the surfaces the player actually walks
    // on. The previous version used an artificial `world.size.y - 64`
    // floor that sat below the visible LDtk floor — sand was piling
    // there, just out of view and below the player's collision plane.
    // Only the top FLOOR_WALL_THICKNESS rows of each block need walls
    // (material rests at the surface), which keeps the particle count
    // bounded even for big floor blocks.
    let mut block_wall_emits = 0usize;
    for block in &world.blocks {
        let min = block.aabb.min;
        let width = (block.aabb.max.x - min.x).round() as i32;
        let block_height = (block.aabb.max.y - min.y).round() as i32;
        let strip_height = block_height.min(FLOOR_WALL_THICKNESS);
        if width <= 0 || strip_height <= 0 {
            continue;
        }
        emit_wall_rect(&mut writer, world, min.x, min.y, width, strip_height);
        block_wall_emits += (width as usize) * (strip_height as usize);
    }

    // Low retaining lips help split the spout streams into visually
    // separate columns. Particles, not Ambition collision.
    let retain_top = (world.size.y - 200.0).max(0.0);
    for x in [256.0, 512.0, 704.0] {
        emit_wall_rect(&mut writer, world, x, retain_top, 2, 190);
    }

    bevy::log::info!(
        "falling_sand_room: seeded boundaries — {} block-top wall particles across {} LDtk blocks",
        block_wall_emits,
        world.blocks.len()
    );

    state.seeded_boundaries = true;
}

fn emit_wall_rect(
    writer: &mut MessageWriter<SpawnParticleSignal>,
    world: &ae::World,
    x: f32,
    y: f32,
    width: i32,
    height: i32,
) {
    emit_particle_rect(writer, TYPE_WALL, world, x, y, width, height);
}

fn emit_particle_rect(
    writer: &mut MessageWriter<SpawnParticleSignal>,
    particle_type: &'static str,
    world: &ae::World,
    x: f32,
    y: f32,
    width: i32,
    height: i32,
) {
    let start_x = x.round() as i32;
    let start_y = y.round() as i32;
    for dx in 0..width.max(0) {
        for dy in 0..height.max(0) {
            let world_pos = ae::Vec2::new((start_x + dx) as f32, (start_y + dy) as f32);
            // `overwrite_existing` so overlapping seed regions (e.g. the
            // side walls + the floor share corners; multiple LDtk blocks
            // may touch) don't leave silent-fail holes that material can
            // tunnel through. `new` would silently skip any cell that's
            // already occupied by an earlier seed call this frame.
            writer.write(SpawnParticleSignal::overwrite_existing(
                Particle::new(particle_type),
                world_to_particle_grid(world, world_pos),
            ));
        }
    }
}

fn capture_falling_sand_switch_interactions(
    room_set: Res<crate::rooms::RoomSet>,
    mut state: ResMut<FallingSandRoomState>,
    mut save: ResMut<crate::persistence::save::SandboxSave>,
    mut effects: MessageReader<crate::features::GameplayEffect>,
) {
    if room_set.active_spec().id != ROOM_ID {
        return;
    }

    for effect in effects.read() {
        let crate::features::GameplayEffect::ActivateSwitch { activation, .. } = effect else {
            continue;
        };
        if state.spouts.toggle(activation.id.as_str()) {
            // Mirror the in-memory toggle into the save so the spout
            // state survives a reset / room re-entry. Without this
            // write the save's switch flag stays whatever the
            // encounter pipeline set it to (which is "true on first
            // activation" only when the switch's `action` is
            // `ResetEncounter`).
            let on = match activation.id.as_str() {
                SAND_SWITCH => state.spouts.sand,
                WATER_SWITCH => state.spouts.water,
                OIL_SWITCH => state.spouts.oil,
                MIXED_SWITCH => state.spouts.mixed,
                _ => continue,
            };
            save.data_mut().set_switch(&activation.id, on);
            bevy::log::info!(
                "falling_sand_room: spout {} -> {} (state {:?})",
                activation.id,
                on,
                state.spouts
            );
        } else {
            bevy::log::debug!(
                "falling_sand_room: ignoring switch activation id={:?} (not a spout switch)",
                activation.id
            );
        }
    }
}

/// Force the falling-sand switch sprites' `SwitchOn` flag to track the
/// spout state. The engine's default switch behaviour is one-way
/// latching (`on.0 = true` on activation, never reset), which inverts
/// the player's mental model once they toggle a spout closed: the
/// sprite stays "on" (green) while the spout is actually off.
fn sync_falling_sand_switch_visuals(
    state: Res<FallingSandRoomState>,
    room_set: Res<crate::rooms::RoomSet>,
    mut switches: Query<(
        &crate::features::SwitchFeature,
        &mut crate::features::SwitchOn,
    )>,
) {
    if !state.active_room || room_set.active_spec().id != ROOM_ID {
        return;
    }
    for (switch, mut on) in &mut switches {
        let desired = match switch.activation.id.as_str() {
            SAND_SWITCH => state.spouts.sand,
            WATER_SWITCH => state.spouts.water,
            OIL_SWITCH => state.spouts.oil,
            MIXED_SWITCH => state.spouts.mixed,
            _ => continue,
        };
        if on.0 != desired {
            on.0 = desired;
        }
    }
}

fn sync_falling_sand_spout_nozzles(
    mut commands: Commands,
    room_set: Res<crate::rooms::RoomSet>,
    state: Res<FallingSandRoomState>,
    existing: Query<(Entity, &FallingSandSpoutNozzle)>,
) {
    let room = room_set.active_spec();
    if !state.active_room || room.id != ROOM_ID {
        for (entity, _) in &existing {
            commands.entity(entity).despawn();
        }
        return;
    }

    let mut desired = HashSet::<&'static str>::new();
    if state.spouts.sand {
        desired.insert(SAND_SWITCH);
    }
    if state.spouts.water {
        desired.insert(WATER_SWITCH);
    }
    if state.spouts.oil {
        desired.insert(OIL_SWITCH);
    }
    if state.spouts.mixed {
        desired.insert(MIXED_SWITCH);
    }

    let mut present = HashSet::<&'static str>::new();
    for (entity, nozzle) in &existing {
        if desired.contains(nozzle.id) {
            present.insert(nozzle.id);
        } else {
            commands.entity(entity).despawn();
        }
    }

    for &id in &desired {
        if present.contains(id) {
            continue;
        }
        let (x, y, width, color) = match id {
            SAND_SWITCH => (176.0, 90.0, 38.0, MaterialKind::Sand.visual_color()),
            WATER_SWITCH => (384.0, 90.0, 38.0, MaterialKind::Water.visual_color()),
            OIL_SWITCH => (592.0, 90.0, 38.0, MaterialKind::Oil.visual_color()),
            MIXED_SWITCH => (792.0, 90.0, 86.0, crate::config::rgba(0.92, 0.80, 0.38, 0.90)),
            _ => continue,
        };
        commands.spawn((
            Name::new(format!("falling sand open spout {id}")),
            Sprite::from_color(color, Vec2::new(width, 8.0)),
            Transform::from_translation(crate::config::world_to_bevy(
                &room.world,
                ae::Vec2::new(x, y - 8.0),
                crate::config::WORLD_Z_FX + 1.0,
            )),
            FallingSandSpoutNozzle { id },
            crate::presentation::rendering::RoomVisual,
        ));
    }
}

fn emit_falling_sand_spouts(
    mut commands: Commands,
    room_set: Res<crate::rooms::RoomSet>,
    mut state: ResMut<FallingSandRoomState>,
    mut writer: MessageWriter<SpawnParticleSignal>,
    mut last_logged: Local<Option<FallingSandSpoutState>>,
) {
    let room = room_set.active_spec();
    if !state.active_room || room.id != ROOM_ID {
        return;
    }

    // One info-log per state transition (open/close) so the user can
    // verify in the console that the toggle reached this system. Sampled
    // on edges only — no per-frame spam.
    if last_logged.as_ref() != Some(&state.spouts) {
        let spout_grid_sand = world_to_particle_grid(&room.world, ae::Vec2::new(176.0, 90.0));
        bevy::log::info!(
            "falling_sand_room: emit pass — spouts={:?}, sand-spout-grid={:?}",
            state.spouts,
            spout_grid_sand
        );
        *last_logged = Some(state.spouts);
    }

    let world = &room.world;

    // Feed the crate's simulation for material accumulation and collision/water
    // projection, and also spawn lightweight Ambition-side falling pixels so
    // the player gets immediate visual feedback that the spout opened.
    //
    // emit_spout uses `overwrite_existing` (not `new`) so the spout mouth
    // is guaranteed to produce a particle every frame even if the cell
    // was still occupied by the previous frame's emission. The 8×1 mouth
    // gives a visibly thick stream at ~480 particles/sec per solo spout
    // (one per frame per X-column). Mixed splits its budget across three
    // streams.
    if state.spouts.sand {
        emit_spout(&mut writer, TYPE_SAND, world, 176.0, 90.0, 8, 1);
        spawn_stream_particles(
            &mut commands,
            world,
            MaterialKind::Sand,
            176.0,
            90.0,
            36.0,
            4,
            &mut state.visual_emit_counter,
        );
    }
    if state.spouts.water {
        emit_spout(&mut writer, TYPE_WATER, world, 384.0, 90.0, 8, 1);
        spawn_stream_particles(
            &mut commands,
            world,
            MaterialKind::Water,
            384.0,
            90.0,
            36.0,
            4,
            &mut state.visual_emit_counter,
        );
    }
    if state.spouts.oil {
        emit_spout(&mut writer, TYPE_OIL, world, 592.0, 90.0, 8, 1);
        spawn_stream_particles(
            &mut commands,
            world,
            MaterialKind::Oil,
            592.0,
            90.0,
            36.0,
            3,
            &mut state.visual_emit_counter,
        );
    }
    if state.spouts.mixed {
        // Mixed spreads three thinner streams so the combined throughput
        // stays in the same per-frame budget as one solo spout.
        emit_spout(&mut writer, TYPE_SAND, world, 760.0, 90.0, 3, 1);
        emit_spout(&mut writer, TYPE_WATER, world, 792.0, 90.0, 3, 1);
        emit_spout(&mut writer, TYPE_OIL, world, 824.0, 90.0, 3, 1);
        spawn_stream_particles(
            &mut commands,
            world,
            MaterialKind::Sand,
            760.0,
            90.0,
            20.0,
            2,
            &mut state.visual_emit_counter,
        );
        spawn_stream_particles(
            &mut commands,
            world,
            MaterialKind::Water,
            792.0,
            90.0,
            20.0,
            2,
            &mut state.visual_emit_counter,
        );
        spawn_stream_particles(
            &mut commands,
            world,
            MaterialKind::Oil,
            824.0,
            90.0,
            20.0,
            2,
            &mut state.visual_emit_counter,
        );
    }
}

fn emit_spout(
    writer: &mut MessageWriter<SpawnParticleSignal>,
    particle_type: &'static str,
    world: &ae::World,
    x: f32,
    y: f32,
    width: i32,
    height: i32,
) {
    let start_x = x.round() as i32 - width / 2;
    let start_y = y.round() as i32;
    for dx in 0..width {
        for dy in 0..height {
            let world_pos = ae::Vec2::new((start_x + dx) as f32, (start_y + dy) as f32);
            // `overwrite_existing` rather than `new` because the spout
            // mouth is hit again every Bevy frame: with `new` the signal
            // silently fails whenever a previous frame's particle hasn't
            // moved out yet, so the effective emit rate collapses far
            // below what the dimensions suggest. Overwriting guarantees
            // a constant supply at the source.
            writer.write(SpawnParticleSignal::overwrite_existing(
                Particle::new(particle_type),
                world_to_particle_grid(world, world_pos),
            ));
        }
    }
}

fn spawn_stream_particles(
    commands: &mut Commands,
    world: &ae::World,
    kind: MaterialKind,
    x: f32,
    y: f32,
    width: f32,
    count: usize,
    counter: &mut u32,
) {
    for i in 0..count {
        *counter = counter.wrapping_add(1);
        let t = ((*counter + i as u32 * 17) % 100) as f32 / 100.0;
        let offset = (t - 0.5) * width;
        let side_wobble = (((*counter / 3 + i as u32 * 11) % 31) as f32 - 15.0) * 0.28;
        let world_pos = ae::Vec2::new(x + offset, y);
        let vel = match kind {
            MaterialKind::Sand => ae::Vec2::new(side_wobble, 270.0 + t * 40.0),
            MaterialKind::Water => ae::Vec2::new(side_wobble * 1.6, 230.0 + t * 55.0),
            MaterialKind::Oil => ae::Vec2::new(side_wobble * 0.55, 150.0 + t * 30.0),
        };
        let size = match kind {
            MaterialKind::Sand => Vec2::splat(4.0),
            MaterialKind::Water => Vec2::new(4.0, 6.0),
            MaterialKind::Oil => Vec2::new(5.0, 6.0),
        };
        commands.spawn((
            Name::new(format!("falling sand stream {kind:?}")),
            Sprite::from_color(kind.stream_color(), size),
            Transform::from_translation(crate::config::world_to_bevy(
                world,
                world_pos,
                crate::config::WORLD_Z_FX + 2.0,
            )),
            FallingSandStreamParticle {
                world_pos,
                vel,
                age: 0.0,
                lifetime: 2.4,
            },
            crate::presentation::rendering::RoomVisual,
        ));
    }
}

fn animate_falling_sand_stream_particles(
    mut commands: Commands,
    room_set: Res<crate::rooms::RoomSet>,
    time: Res<Time>,
    mut particles: Query<(Entity, &mut FallingSandStreamParticle, &mut Transform)>,
) {
    let room = room_set.active_spec();
    if room.id != ROOM_ID {
        for (entity, _, _) in &mut particles {
            commands.entity(entity).despawn();
        }
        return;
    }

    let dt = time.delta_secs().min(0.05);
    let floor_y = (room.world.size.y - 64.0).max(0.0);
    for (entity, mut particle, mut transform) in &mut particles {
        particle.age += dt;
        particle.vel.y += 90.0 * dt;
        let step = particle.vel * dt;
        particle.world_pos += step;
        if particle.age >= particle.lifetime || particle.world_pos.y >= floor_y + 10.0 {
            commands.entity(entity).despawn();
            continue;
        }
        transform.translation = crate::config::world_to_bevy(
            &room.world,
            particle.world_pos,
            crate::config::WORLD_Z_FX + 2.0,
        );
    }
}

/// Reusable per-frame scratch buffers. Living in a `Local<>` keeps the
/// allocations across frames; we just `.clear()` between runs so we don't
/// hand the allocator three new HashMaps every tick.
///
/// `dense_sand` uses a `HashSet` so the liquid pass's "is this tile
/// already a sand block?" check is O(1) instead of the O(n) `Vec::contains`
/// it used to be — at 220 sand tiles × 360 liquid candidates the previous
/// path was ~80k comparisons per frame.
#[derive(Default)]
struct ProjectionScratch {
    sand_tiles: HashMap<(i32, i32), usize>,
    water_tiles: HashMap<(i32, i32), usize>,
    oil_tiles: HashMap<(i32, i32), usize>,
    dense_sand: HashSet<(i32, i32)>,
    desired_visuals: HashMap<(i32, i32), MaterialKind>,
}

impl ProjectionScratch {
    fn reset_per_frame(&mut self) {
        self.sand_tiles.clear();
        self.water_tiles.clear();
        self.oil_tiles.clear();
        self.dense_sand.clear();
        self.desired_visuals.clear();
    }
}

fn project_particles_to_movement_world(
    mut commands: Commands,
    room_set: Res<crate::rooms::RoomSet>,
    state: Res<FallingSandRoomState>,
    mut world: ResMut<crate::GameWorld>,
    particles: Query<(&GridPosition, &Particle)>,
    visuals: Query<(Entity, &FallingSandMaterialVisual)>,
    mut scratch: Local<ProjectionScratch>,
) {
    if !state.active_room || room_set.active_spec().id != ROOM_ID {
        clear_material_visuals(&mut commands, &visuals);
        return;
    }

    // `clone_from` reuses the existing Vec capacity and the per-Block
    // String allocations, instead of allocating fresh on every tick the
    // way `world.0.blocks = state.base_blocks.clone()` did.
    world.0.blocks.clone_from(&state.base_blocks);
    world.0.water_regions.clone_from(&state.base_water_regions);

    scratch.reset_per_frame();

    for (grid_position, particle) in &particles {
        let Some(kind) = MaterialKind::from_particle_type(particle.name.as_ref()) else {
            continue;
        };
        let Some((tile_x, tile_y)) = grid_to_world_tile(&world.0, grid_position.0) else {
            continue;
        };

        let tile_counts = match kind {
            MaterialKind::Sand => &mut scratch.sand_tiles,
            MaterialKind::Water => &mut scratch.water_tiles,
            MaterialKind::Oil => &mut scratch.oil_tiles,
        };
        *tile_counts.entry((tile_x, tile_y)).or_default() += 1;
    }

    project_sand(&mut world.0, &mut scratch);
    let mut liquid_added: usize = 0;
    project_liquid(
        &mut world.0,
        &mut scratch,
        &mut liquid_added,
        MaterialKind::Water,
        ae::WaterKind::Clear,
        falling_water_spec(),
    );
    project_liquid(
        &mut world.0,
        &mut scratch,
        &mut liquid_added,
        // Oil currently piggy-backs on Murky water until an engine-side
        // Oil fluid kind exists. The drag/gravity profile is in
        // `viscous_oil_spec()` so behaviour stays oil-shaped.
        MaterialKind::Oil,
        ae::WaterKind::Murky,
        viscous_oil_spec(),
    );

    sync_material_visuals(&mut commands, &world.0, &scratch.desired_visuals, &visuals);
}

/// Sort tile keys by `count` desc (stable on tile coords as the tiebreaker)
/// so the dynamic-tile cap chooses deterministically and densely-populated
/// tiles always win over sparse ones.
fn sorted_tiles_by_count_desc(tiles: &HashMap<(i32, i32), usize>) -> Vec<(i32, i32)> {
    let mut keys: Vec<(i32, i32)> = tiles.keys().copied().collect();
    keys.sort_by(|a, b| {
        let count_a = tiles.get(a).copied().unwrap_or(0);
        let count_b = tiles.get(b).copied().unwrap_or(0);
        count_b.cmp(&count_a).then_with(|| a.cmp(b))
    });
    keys
}

fn project_sand(world: &mut ae::World, scratch: &mut ProjectionScratch) {
    let keys = sorted_tiles_by_count_desc(&scratch.sand_tiles);
    let mut added = 0;
    for (tile_x, tile_y) in keys {
        let count = scratch.sand_tiles.get(&(tile_x, tile_y)).copied().unwrap_or(0);
        if count >= MATERIAL_VISUAL_THRESHOLD {
            scratch
                .desired_visuals
                .insert((tile_x, tile_y), MaterialKind::Sand);
        }
        if count < SAND_THRESHOLD || added >= MAX_DYNAMIC_SAND_TILES {
            continue;
        }
        scratch.dense_sand.insert((tile_x, tile_y));
        world.blocks.push(ae::Block::one_way(
            format!("falling_sand:sand:{tile_x}:{tile_y}"),
            tile_min(tile_x, tile_y),
            tile_size_vec(),
        ));
        added += 1;
    }
}

fn project_liquid(
    world: &mut ae::World,
    scratch: &mut ProjectionScratch,
    added: &mut usize,
    kind: MaterialKind,
    water_kind: ae::WaterKind,
    spec: ae::WaterVolumeSpec,
) {
    // Pull the source map out into a separate scratch so we can borrow
    // it immutably while we still mutate `scratch.sorted_keys` etc.
    let source = match kind {
        MaterialKind::Water => &scratch.water_tiles,
        MaterialKind::Oil => &scratch.oil_tiles,
        // Sand has its own projection path; this function is liquid-only.
        MaterialKind::Sand => return,
    };

    // Build a sorted key list directly here to keep the borrow on `source`
    // short (avoids reusing `scratch.sorted_keys` while `source` is borrowed).
    let mut keys: Vec<(i32, i32)> = source.keys().copied().collect();
    keys.sort_by(|a, b| {
        let count_a = source.get(a).copied().unwrap_or(0);
        let count_b = source.get(b).copied().unwrap_or(0);
        count_b.cmp(&count_a).then_with(|| a.cmp(b))
    });

    for (tile_x, tile_y) in keys {
        let count = source.get(&(tile_x, tile_y)).copied().unwrap_or(0);
        if count >= MATERIAL_VISUAL_THRESHOLD
            && !scratch.desired_visuals.contains_key(&(tile_x, tile_y))
        {
            scratch.desired_visuals.insert((tile_x, tile_y), kind);
        }
        if count < LIQUID_THRESHOLD || *added >= MAX_DYNAMIC_LIQUID_TILES {
            continue;
        }
        if scratch.dense_sand.contains(&(tile_x, tile_y)) {
            continue;
        }
        world.water_regions.push(water_tile_region(
            tile_x,
            tile_y,
            water_kind,
            spec,
        ));
        *added += 1;
    }
}

fn grant_room_swim_controls(
    room_set: Res<crate::rooms::RoomSet>,
    mut state: ResMut<FallingSandRoomState>,
    mut players: Query<(Entity, &mut crate::player::PlayerMovementAuthority)>,
) {
    if room_set.active_spec().id == ROOM_ID {
        for (entity, mut player) in &mut players {
            let needs_capture = state
                .swim_snapshot
                .map(|snap| snap.player_entity != entity)
                .unwrap_or(true);
            if needs_capture {
                state.swim_snapshot = Some(SwimSnapshot {
                    player_entity: entity,
                    previous_swim: player.player.abilities.swim,
                });
            }
            player.player.abilities.swim = true;
        }
        return;
    }

    let Some(snapshot) = state.swim_snapshot.take() else {
        return;
    };
    for (entity, mut player) in &mut players {
        if entity == snapshot.player_entity {
            player.player.abilities.swim = snapshot.previous_swim;
        }
    }
}

/// Once per second while in the falling-sand room, log a diagnostic
/// snapshot:
///   - total particle count, broken down by particle type name
///   - sand / water / oil Y-extent (min/max grid_y observed)
///   - count "near the floor" (within the bevy_falling_sand wall band)
///   - count "below the floor" (would mean particles tunneled through)
///   - count of wall particles in the floor band (proves walls exist)
///
/// All from one ECS query, so it's cheap to leave on while we debug.
#[allow(clippy::too_many_arguments)]
fn log_falling_sand_diagnostics(
    time: Res<Time>,
    state: Res<FallingSandRoomState>,
    room_set: Res<crate::rooms::RoomSet>,
    particles: Query<(&Particle, &GridPosition)>,
    mut next_log_at: Local<f32>,
) {
    if !state.active_room || room_set.active_spec().id != ROOM_ID {
        return;
    }
    let now = time.elapsed_secs();
    if now < *next_log_at {
        return;
    }
    *next_log_at = now + 1.0;

    let world = &room_set.active_spec().world;
    // The expected floor band (where our seed walls live): the band of
    // grid_y values covered by emit_wall_rect at each LDtk block top.
    // We approximate "floor band" as the union of bands across blocks
    // by taking the minimum block min.y, since that's the highest
    // visible floor surface in world coords.
    let block_min_y = world
        .blocks
        .iter()
        .map(|b| b.aabb.min.y)
        .fold(f32::INFINITY, f32::min);
    let band_top_world_y = block_min_y;
    let band_bottom_world_y = block_min_y + FLOOR_WALL_THICKNESS as f32;
    // Convert to grid_y. Recall: grid_y = size.y/2 - world_y, so a
    // SMALLER world_y maps to a LARGER grid_y. The floor band's TOP
    // edge in world is its BOTTOM edge in grid space and vice versa.
    let band_grid_y_high = (world.size.y * 0.5 - band_top_world_y).round() as i32;
    let band_grid_y_low = (world.size.y * 0.5 - band_bottom_world_y).round() as i32;

    let mut counts: std::collections::BTreeMap<String, usize> =
        std::collections::BTreeMap::new();
    let mut sand_y_min = i32::MAX;
    let mut sand_y_max = i32::MIN;
    let mut sand_near_floor = 0usize;
    let mut sand_below_floor = 0usize;
    let mut wall_in_floor_band = 0usize;
    for (particle, grid_pos) in &particles {
        let name: &str = particle.name.as_ref();
        *counts.entry(name.to_owned()).or_default() += 1;
        let gy = grid_pos.0.y;
        if name == TYPE_SAND {
            sand_y_min = sand_y_min.min(gy);
            sand_y_max = sand_y_max.max(gy);
            if gy <= band_grid_y_high && gy >= band_grid_y_low {
                sand_near_floor += 1;
            } else if gy < band_grid_y_low {
                sand_below_floor += 1;
            }
        }
        if name == TYPE_WALL && gy <= band_grid_y_high && gy >= band_grid_y_low {
            wall_in_floor_band += 1;
        }
    }

    let sand_extent = if sand_y_min == i32::MAX {
        "(none)".to_owned()
    } else {
        format!("grid_y∈[{sand_y_min}, {sand_y_max}]")
    };

    bevy::log::info!(
        "fs-diag: counts={:?}  sand:{}  near_floor={}  below_floor={}  walls_in_band={}  band_grid_y∈[{}, {}]",
        counts,
        sand_extent,
        sand_near_floor,
        sand_below_floor,
        wall_in_floor_band,
        band_grid_y_low,
        band_grid_y_high,
    );
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum MaterialKind {
    Sand,
    Water,
    Oil,
}

impl MaterialKind {
    fn from_particle_type(name: &str) -> Option<Self> {
        match name {
            TYPE_SAND => Some(Self::Sand),
            TYPE_WATER => Some(Self::Water),
            TYPE_OIL => Some(Self::Oil),
            _ => None,
        }
    }

    fn visual_color(self) -> Color {
        match self {
            Self::Sand => crate::config::rgba(0.95, 0.74, 0.28, 0.72),
            Self::Water => crate::config::rgba(0.18, 0.55, 1.0, 0.48),
            Self::Oil => crate::config::rgba(0.20, 0.13, 0.06, 0.66),
        }
    }

    fn stream_color(self) -> Color {
        match self {
            Self::Sand => crate::config::rgba(1.0, 0.82, 0.33, 0.95),
            Self::Water => crate::config::rgba(0.28, 0.66, 1.0, 0.82),
            Self::Oil => crate::config::rgba(0.19, 0.12, 0.06, 0.92),
        }
    }
}

fn clear_material_visuals(
    commands: &mut Commands,
    visuals: &Query<(Entity, &FallingSandMaterialVisual)>,
) {
    for (entity, _) in visuals.iter() {
        commands.entity(entity).despawn();
    }
}

fn sync_material_visuals(
    commands: &mut Commands,
    world: &ae::World,
    desired: &HashMap<(i32, i32), MaterialKind>,
    visuals: &Query<(Entity, &FallingSandMaterialVisual)>,
) {
    let mut existing = HashSet::<((i32, i32), MaterialKind)>::new();
    for (entity, visual) in visuals.iter() {
        let desired_kind = desired.get(&visual.tile).copied();
        if desired_kind == Some(visual.kind) {
            existing.insert((visual.tile, visual.kind));
        } else {
            commands.entity(entity).despawn();
        }
    }

    for (&tile, &kind) in desired {
        if existing.contains(&(tile, kind)) {
            continue;
        }
        let center = tile_min(tile.0, tile.1)
            + ae::Vec2::new(TILE_SIZE as f32 * 0.5, TILE_SIZE as f32 * 0.5);
        commands.spawn((
            Name::new(format!("falling sand projected {kind:?} tile {}:{}", tile.0, tile.1)),
            Sprite::from_color(kind.visual_color(), Vec2::splat(TILE_SIZE as f32)),
            Transform::from_translation(crate::config::world_to_bevy(
                world,
                center,
                crate::config::WORLD_Z_PLAYER + 4.0,
            )),
            FallingSandMaterialVisual { tile, kind },
            crate::presentation::rendering::RoomVisual,
        ));
    }
}

fn world_to_particle_grid(world: &ae::World, world_pos: ae::Vec2) -> IVec2 {
    IVec2::new(
        (world_pos.x - world.size.x * 0.5).round() as i32,
        (world.size.y * 0.5 - world_pos.y).round() as i32,
    )
}

fn grid_to_world_tile(world: &ae::World, grid: IVec2) -> Option<(i32, i32)> {
    let world_x = grid.x as f32 + world.size.x * 0.5;
    let world_y = world.size.y * 0.5 - grid.y as f32;
    if !(0.0..world.size.x).contains(&world_x) || !(0.0..world.size.y).contains(&world_y) {
        return None;
    }
    Some((
        (world_x.floor() as i32).div_euclid(TILE_SIZE),
        (world_y.floor() as i32).div_euclid(TILE_SIZE),
    ))
}

fn tile_min(tile_x: i32, tile_y: i32) -> ae::Vec2 {
    ae::Vec2::new((tile_x * TILE_SIZE) as f32, (tile_y * TILE_SIZE) as f32)
}

fn tile_size_vec() -> ae::Vec2 {
    ae::Vec2::new(TILE_SIZE as f32, TILE_SIZE as f32)
}

fn water_tile_region(
    tile_x: i32,
    tile_y: i32,
    kind: ae::WaterKind,
    spec: ae::WaterVolumeSpec,
) -> ae::WaterRegion {
    ae::WaterRegion::new(
        ae::aabb_from_min_size(tile_min(tile_x, tile_y), tile_size_vec()),
        kind,
        spec,
    )
}

fn falling_water_spec() -> ae::WaterVolumeSpec {
    ae::WaterVolumeSpec {
        gravity_scale: 0.18,
        drag: 0.82,
        max_fall_speed: 160.0,
        swim_up_impulse: 520.0,
    }
}

fn viscous_oil_spec() -> ae::WaterVolumeSpec {
    ae::WaterVolumeSpec {
        gravity_scale: 0.32,
        drag: 1.85,
        max_fall_speed: 82.0,
        swim_up_impulse: 330.0,
    }
}
