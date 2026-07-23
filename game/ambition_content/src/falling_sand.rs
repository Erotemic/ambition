//! Falling-sand room PRESENTATION + `bevy_falling_sand` bridge for water/oil —
//! CONTENT (a self-gating room plugin: feature-gated, visible-binary only,
//! active only while its authored room is; R3.3 room-mechanics-by-kind).
//!
//! The room's SIMULATION half — spout/switch state, the deterministic sand
//! grid, the FS3 settled-sand ledger, and the persistent collision projection
//! — lives ungated in [`crate::falling_sand_sim`], driven one solver step per
//! sim tick and testable headless. THIS module is what remains on the external
//! `bevy_falling_sand` crate and the render side:
//!
//! - the particle CA bridge for **water and oil** (their FS slice has not
//!   landed; sand no longer touches the external crate),
//! - the liquid projection into temporary water regions (excluding tiles the
//!   settled-sand ledger owns — single owner per tile),
//! - presentation: `bevy_falling_sand`'s particle rendering, the sand-grid
//!   texture, nozzle sprites, switch visuals, and diagnostics.
//!
//! # Status after the FS2/FS3 sand slice (2026-07-20)
//!
//! Sand is CORRECT (deterministic, conserving, settling — see
//! `falling_sand_sim::sand_grid`); water and oil remain on the old path with
//! its known defects (frame-locked stepping, no level-finding). The bfs-side
//! sand plumbing below (`MaterialKind::Sand` arms, `project_sand`) is
//! vestigial — it sees zero sand particles — and dies with the water/oil
//! slice rather than churning that path now.

use std::collections::{HashMap, HashSet};

use ambition_engine_core as ae;
use ambition_platformer_primitives::schedule::SimScheduleExt;
use bevy::prelude::*;
use bevy_falling_sand::prelude::*;

use crate::falling_sand_sim::{
    open_spouts, FallingSandRoomState, FallingSandSimSet, FallingSandSpoutState, FallingSandWorld,
    MIXED_SWITCH, OIL_SWITCH, ROOM_ID, SAND_SWITCH, SIDE_WALL_THICKNESS, TILE_SIZE, TYPE_OIL,
    TYPE_SAND, TYPE_WALL, TYPE_WATER, WATER_SWITCH,
};
use crate::falling_sand_sim::{SandCell, FLOOR_WALL_THICKNESS};

/// Per-tile minimum particle counts before we promote the tile into
/// Ambition's collision/visual world. Tuned for "flood the room"
/// — the room is 64×40 = 2560 tiles, so the previous 14/10
/// thresholds left huge swaths of low-density spread invisible.
/// Each tile holds up to TILE_SIZE² = 256 particle cells, so a
/// count of 4 means ~1.5% fill — enough for a thin liquid film to
/// register, low enough not to flicker on stray particles. Sand
/// settles densely so its threshold can stay a bit higher than
/// liquid's.
const SAND_THRESHOLD: usize = 6;
const LIQUID_THRESHOLD: usize = 4;
const MATERIAL_VISUAL_THRESHOLD: usize = 3;
/// Maximum number of tile-sized collision blocks / water regions
/// projected per material per frame. Sized to "the whole room
/// flooded" — the room is 2560 tiles, so 2500 lets either material
/// cover essentially every cell. The earlier ~200 cap was a perf
/// safety net from when the prototype was first standing up; the
/// movement world tolerates a few-thousand-block scan fine on
/// desktop, and the alternative ("liquid invisible past the first
/// pool") is worse than the collision-iter cost.
const MAX_DYNAMIC_SAND_TILES: usize = 2500;
const MAX_DYNAMIC_LIQUID_TILES: usize = 2500;

#[derive(Component)]
struct FallingSandMaterialVisual {
    tile: (i32, i32),
    kind: MaterialKind,
}

/// The room-sized texture that draws the sand grid: loose grains and settled
/// ground, at cell (world-pixel) resolution. One sprite, one image, rewritten
/// only on sim ticks that actually advanced the grid.
#[derive(Component)]
struct FallingSandGridVisual {
    image: Handle<Image>,
    drawn_tick: Option<u64>,
}

#[derive(Component)]
struct FallingSandSpoutNozzle {
    id: &'static str,
}

pub struct FallingSandRoomPlugin;

impl Plugin for FallingSandRoomPlugin {
    fn build(&self, app: &mut App) {
        let sim = app.sim_schedule();
        // Room/spout state, the sand grid, and the switch capture are owned by
        // the ungated `FallingSandSimPlugin` (registered by
        // `AmbitionContentPlugin` in every composition). This plugin is the
        // water/oil bridge + presentation, ordered after the sim half.
        app.init_resource::<FallingSandProjectionReport>()
            .add_plugins(
                FallingSandPlugin::default()
                    .with_chunk_size(64)
                    .with_map_size(32),
            )
            .add_systems(Startup, setup_particle_types)
            .add_systems(
                sim,
                (
                    despawn_bfs_particles_when_the_room_changes,
                    seed_falling_sand_room_boundaries,
                    sync_falling_sand_spout_nozzles,
                    emit_falling_sand_spouts,
                    project_particles_to_movement_world,
                )
                    .chain()
                    // The projection contributes liquid to the collision
                    // overlay, which the rebuild clears each frame — run after
                    // it, and after the sim half so spout state is synced and
                    // the settled ledger is current for the tile exclusion.
                    .after(ambition_actors::features::rebuild_feature_ecs_world_overlay)
                    .after(FallingSandSimSet)
                    .in_set(ambition_platformer_primitives::schedule::SandboxSet::WorldPrep),
            )
            .add_systems(
                sim,
                // Visual sync must run *after* the toggle handler so
                // SwitchOn reflects the spout state we just set —
                // otherwise the engine's "switch is latching" semantics
                // leave the sprite stuck green while the spout flips
                // back off, which inverts the player's mental model.
                sync_falling_sand_switch_visuals
                    .after(crate::falling_sand_sim::capture_falling_sand_switch_interactions)
                    .in_set(ambition_platformer_primitives::schedule::SandboxSet::GameplayEffects),
            )
            // `bevy_falling_sand` inits `ParticleSimulationRun` unconditionally,
            // so its chunk scan (`par_handle_movement_by_chunks` over the full
            // 32x32 map) burns CPU in every room of every game, particles or
            // not. Hold the upstream gate open only while this room is active
            // or bfs particles still exist (a frame of flip latency is fine —
            // room transitions despawn the particles anyway).
            .add_systems(Update, gate_bfs_simulation_to_room_presence)
            // The sand grid's presentation: a room-sized texture redrawn on
            // grid ticks ("always draw blind" — the sim half ships with its
            // visual, even though the author can't see it).
            .add_systems(Update, sync_sand_grid_texture)
            // Diagnostic: once per second while in the falling-sand room,
            // dump per-type particle counts and Y-distribution. Lets us
            // see at a glance whether particles are being spawned,
            // whether they're reaching the floor wall band, and where
            // they're going if they vanish.
            .add_systems(Update, log_falling_sand_diagnostics);
    }
}

/// Mirror room presence into `bevy_falling_sand`'s own on/off switch: its
/// `PreUpdate`/`PostUpdate` sets all `run_if(resource_exists::<
/// ParticleSimulationRun>)`, so removing the resource idles the whole external
/// sim. Presence keeps it running until the last particle is gone, so a room
/// exit never strands live particles mid-drain.
fn gate_bfs_simulation_to_room_presence(
    mut commands: Commands,
    state: Res<FallingSandRoomState>,
    particles: Query<(), With<Particle>>,
    gate: Option<Res<ParticleSimulationRun>>,
) {
    let should_run = state.active_room || !particles.is_empty();
    if should_run && gate.is_none() {
        commands.init_resource::<ParticleSimulationRun>();
    } else if !should_run && gate.is_some() {
        commands.remove_resource::<ParticleSimulationRun>();
    }
}

fn setup_particle_types(mut commands: Commands) {
    // bevy_falling_sand v0.7.0 lazy-loads chunk entities in
    // `update_chunk_loading`, which early-returns if there's no
    // `ChunkLoader` entity in the world. With no chunk entities, the
    // movement-by-chunks system finds zero dirty chunks and skips
    // every particle (silently — no warning) — exactly the bug we hit
    // (sand spawned into the ParticleMap but never moved).
    //
    // Spawn one static ChunkLoader at the world origin. The map's
    // initial loaded region is [-1024, 1024) on both axes, which
    // contains the entire falling-sand room, so a static loader at
    // (0, 0) is fine — we don't need or want origin shifts.
    commands.spawn((
        Name::new("falling sand chunk loader (static, origin)"),
        ChunkLoader,
        Transform::from_xyz(0.0, 0.0, 0.0),
        GlobalTransform::default(),
    ));

    // No sand ParticleType: sand runs on the deterministic grid in
    // `falling_sand_sim` and never enters the external crate.

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

/// Room-change cleanup for the EXTERNAL crate's particles (water/oil/walls).
/// The sim half owns the room-state sync; this only mirrors its "matter does
/// not survive a room change" rule onto the bfs world.
fn despawn_bfs_particles_when_the_room_changes(
    mut commands: Commands,
    room_set: ambition::platformer::lifecycle::SessionWorldRef<ambition_actors::rooms::RoomSet>,
    particles: Query<Entity, With<Particle>>,
    mut last_room_id: Local<Option<String>>,
) {
    let active_id = room_set.active_spec().id.as_str();
    if last_room_id.as_deref() == Some(active_id) {
        return;
    }
    *last_room_id = Some(active_id.to_owned());
    for particle in &particles {
        commands.entity(particle).despawn();
    }
}

fn seed_falling_sand_room_boundaries(
    room_set: ambition::platformer::lifecycle::SessionWorldRef<ambition_actors::rooms::RoomSet>,
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
    //
    // Only solid blocks (`Solid` / `BlinkWall`) get seeded as particle
    // walls. **One-way platforms are deliberately skipped** — the
    // user expects falling material to PASS THROUGH platforms (the
    // way the player drops through them with the down-press), so
    // seeding wall particles on top would trap material on every
    // mid-height platform and never let it reach the actual floor.
    // This was the cause of "everything pools on the top platform"
    // — the room's high one-way ledges were acting as impenetrable
    // lids that captured all the water and oil before it could
    // settle on the floor.
    let mut block_wall_emits = 0usize;
    for block in &world.blocks {
        if !matches!(
            block.kind,
            ae::BlockKind::Solid | ae::BlockKind::BlinkWall { .. }
        ) {
            continue;
        }
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

/// Force the falling-sand switch sprites' `SwitchOn` flag to track the
/// spout state. The engine's default switch behaviour is one-way
/// latching (`on.0 = true` on activation, never reset), which inverts
/// the player's mental model once they toggle a spout closed: the
/// sprite stays "on" (green) while the spout is actually off.
fn sync_falling_sand_switch_visuals(
    state: Res<FallingSandRoomState>,
    room_set: ambition::platformer::lifecycle::SessionWorldRef<ambition_actors::rooms::RoomSet>,
    mut switches: Query<(
        &ambition_actors::features::SwitchFeature,
        &mut ambition_actors::features::SwitchOn,
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
    room_set: ambition::platformer::lifecycle::SessionWorldRef<ambition_actors::rooms::RoomSet>,
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

    // Iterate the switches in a FIXED order, not `desired`'s hash order: this spawns
    // entities, and `desired` is a std `HashSet` whose iteration is per-process
    // (ADR 0023 rule 3). `desired` stays a membership filter, which the rule allows.
    for &id in &[SAND_SWITCH, WATER_SWITCH, OIL_SWITCH, MIXED_SWITCH] {
        if !desired.contains(id) || present.contains(id) {
            continue;
        }
        let (x, y, width, color) = match id {
            SAND_SWITCH => (176.0, 90.0, 38.0, MaterialKind::Sand.visual_color()),
            WATER_SWITCH => (384.0, 90.0, 38.0, MaterialKind::Water.visual_color()),
            OIL_SWITCH => (592.0, 90.0, 38.0, MaterialKind::Oil.visual_color()),
            MIXED_SWITCH => (
                792.0,
                90.0,
                86.0,
                ambition_actors::config::rgba(0.92, 0.80, 0.38, 0.90),
            ),
            _ => continue,
        };
        commands.spawn((
            Name::new(format!("falling sand open spout {id}")),
            Sprite::from_color(color, Vec2::new(width, 8.0)),
            Transform::from_translation(ambition_engine_core::config::world_to_bevy(
                &room.world,
                ae::Vec2::new(x, y - 8.0),
                ambition_engine_core::config::WORLD_Z_FX + 1.0,
            )),
            FallingSandSpoutNozzle { id },
            ambition_actors::platformer_runtime::lifecycle::RoomVisual,
        ));
    }
}

/// Emit into **the grid, and only the grid** (FS1's single-owner rule).
///
/// This function used to do two things at once: write `SpawnParticleSignal`s
/// into the cellular automaton *and* spawn a parallel fleet of Ambition-side
/// `FallingSandStreamParticle` sprites, "so the player gets immediate visual
/// feedback that the spout opened." Those sprites were matter's second home.
/// They fell on their own hardcoded gravity, ignored every block in the room,
/// and despawned at an invented `world.size.y - 64` floor — so they poured
/// straight THROUGH the platforms the real particles were pooling on and rained
/// down below. That is Jon's reported defect, verbatim: *"water and oil pool on
/// the top platform yet particles ALSO fall forever below."*
///
/// `falling-sand.md` §1: *"A particle exists in exactly one place: the grid …
/// the fix is structural (single owner), not a patch."* The sprites are gone;
/// `bevy_falling_sand`'s own `render` feature draws the falling matter, and
/// `sync_material_visuals` draws what has settled. One owner, two views of it.
fn emit_falling_sand_spouts(
    room_set: ambition::platformer::lifecycle::SessionWorldRef<ambition_actors::rooms::RoomSet>,
    state: Res<FallingSandRoomState>,
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
    for mouth in open_spouts(&state.spouts) {
        // Sand mouths pour into the deterministic grid (sim half), never
        // into the external crate.
        if mouth.particle_type == TYPE_SAND {
            continue;
        }
        emit_spout(
            &mut writer,
            mouth.particle_type,
            world,
            mouth.x,
            mouth.y,
            mouth.width,
            1,
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

    fn tiles_for(&mut self, kind: MaterialKind) -> &mut HashMap<(i32, i32), usize> {
        match kind {
            MaterialKind::Sand => &mut self.sand_tiles,
            MaterialKind::Water => &mut self.water_tiles,
            MaterialKind::Oil => &mut self.oil_tiles,
        }
    }

    /// Total counted into tile buckets, per material — the right-hand side of
    /// the conservation law.
    fn bucketed(&self, kind: MaterialKind) -> usize {
        let tiles = match kind {
            MaterialKind::Sand => &self.sand_tiles,
            MaterialKind::Water => &self.water_tiles,
            MaterialKind::Oil => &self.oil_tiles,
        };
        tiles.values().sum()
    }
}

/// **The conservation ledger for one projection pass** (`falling-sand.md` §1:
/// *"total matter per material = spawned − despawned, every tick"*).
///
/// The projection is a READ-MODEL over the grid — it must neither create matter
/// nor lose it silently. Every particle the pass sees lands in exactly one of
/// these three columns, and `total()` must equal the number of particles walked.
/// A conservation failure would mean a particle is counted into two tiles, or
/// vanished between the query and the buckets.
///
/// `outside_world` and `unmodelled` are not losses — they are the two legitimate
/// exclusions, named so they cannot hide a third. Wall particles are `unmodelled`
/// (the room seeds them, they are geometry, they never project).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct TallyLedger {
    sand: usize,
    water: usize,
    oil: usize,
    /// The particle's grid cell maps to no tile of this room.
    outside_world: usize,
    /// A particle type this room does not model as matter (walls).
    unmodelled: usize,
}

impl TallyLedger {
    fn counted(&self, kind: MaterialKind) -> usize {
        match kind {
            MaterialKind::Sand => self.sand,
            MaterialKind::Water => self.water,
            MaterialKind::Oil => self.oil,
        }
    }

    fn add(&mut self, kind: MaterialKind) {
        match kind {
            MaterialKind::Sand => self.sand += 1,
            MaterialKind::Water => self.water += 1,
            MaterialKind::Oil => self.oil += 1,
        }
    }

    /// Every particle the pass walked, by whichever column claimed it.
    fn total(&self) -> usize {
        self.sand + self.water + self.oil + self.outside_world + self.unmodelled
    }
}

/// Bucket every live particle into its room tile. Pure over the grid, so the
/// conservation law is a unit test rather than a hope.
///
/// The grid is the ONE owner of matter (FS1). This pass reads it and writes
/// nothing back — the caller turns the buckets into overlay solids and water
/// regions, both of which the overlay rebuild clears every frame.
fn tally_particles<'a>(
    world: &ae::World,
    particles: impl Iterator<Item = (IVec2, &'a str)>,
    scratch: &mut ProjectionScratch,
) -> TallyLedger {
    let mut ledger = TallyLedger::default();
    for (grid_position, particle_type) in particles {
        let Some(kind) = MaterialKind::from_particle_type(particle_type) else {
            ledger.unmodelled += 1;
            continue;
        };
        let Some(tile) = grid_to_world_tile(world, grid_position) else {
            ledger.outside_world += 1;
            continue;
        };
        *scratch.tiles_for(kind).entry(tile).or_default() += 1;
        ledger.add(kind);
    }
    for kind in [MaterialKind::Sand, MaterialKind::Water, MaterialKind::Oil] {
        debug_assert_eq!(
            scratch.bucketed(kind),
            ledger.counted(kind),
            "conservation ({kind:?}): every counted particle lands in exactly one \
             tile bucket. A mismatch means matter was created or lost between the \
             grid query and the tile map."
        );
    }
    ledger
}

fn project_particles_to_movement_world(
    mut commands: Commands,
    room_set: ambition::platformer::lifecycle::SessionWorldRef<ambition_actors::rooms::RoomSet>,
    state: Res<FallingSandRoomState>,
    world: ambition::platformer::lifecycle::SessionWorldRef<ambition_engine_core::RoomGeometry>,
    mut overlay: ResMut<ambition_platformer_primitives::feature_overlay::FeatureEcsWorldOverlay>,
    particles: Query<(&GridPosition, &Particle)>,
    visuals: Query<(Entity, &FallingSandMaterialVisual)>,
    sand: Res<FallingSandWorld>,
    mut scratch: Local<ProjectionScratch>,
    mut cap_warned: Local<bool>,
    mut report: ResMut<FallingSandProjectionReport>,
) {
    if !state.active_room || room_set.active_spec().id != ROOM_ID {
        clear_material_visuals(&mut commands, &visuals);
        *report = FallingSandProjectionReport::default();
        return;
    }

    // The authored `RoomGeometry` base is immutable mid-room: settled sand /
    // liquid is a per-frame derived OVERLAY contribution, not a base edit (the
    // RoomGeometry-decision). Sand rides `gate_solids` (full collision
    // composition, no lock-wall sprite — its name dodges the render reconcile);
    // liquid rides `water_regions`. Both are cleared by the overlay rebuild this
    // frame (we run after it in WorldPrep), so we just push.
    scratch.reset_per_frame();

    let ledger = tally_particles(
        &world.0,
        particles.iter().map(|(g, p)| (g.0, p.name.as_ref())),
        &mut scratch,
    );
    // No silent caps: the projection truncates at MAX_DYNAMIC_* tiles, and a
    // truncated frame looks exactly like a settled one from the outside.
    warn_on_projection_cap(&ledger, &scratch, &mut cap_warned);

    let sand_added = project_sand(&mut overlay.gate_solids, &mut scratch);
    // Single owner per TILE, across BOTH sand representations: tiles the
    // settled-sand ledger owns as collision must not also become water
    // regions (you cannot swim inside ground). The ledger's blocks were
    // pushed by the sim half; here they only veto liquid.
    scratch.dense_sand.extend(sand.ledger.solid_tiles());
    let mut liquid_added: usize = 0;
    project_liquid(
        &mut overlay.water_regions,
        &mut scratch,
        &mut liquid_added,
        MaterialKind::Water,
        ae::WaterKind::Clear,
        falling_water_spec(),
    );
    project_liquid(
        &mut overlay.water_regions,
        &mut scratch,
        &mut liquid_added,
        // Oil currently piggy-backs on Murky water until an engine-side
        // Oil fluid kind exists. The drag/gravity profile is in
        // `viscous_oil_spec()` so behaviour stays oil-shaped.
        MaterialKind::Oil,
        ae::WaterKind::Murky,
        viscous_oil_spec(),
    );

    // Record the funnel BEFORE returning: particles → tiles → over-threshold
    // tiles → appended contributions → overlay totals. A stage that drops to
    // zero while the previous one is non-zero names the regression outright.
    let over = |tiles: &HashMap<(i32, i32), usize>, threshold: usize| {
        tiles.values().filter(|count| **count >= threshold).count()
    };
    *report = FallingSandProjectionReport {
        sand_particles: ledger.sand,
        water_particles: ledger.water,
        oil_particles: ledger.oil,
        outside_world: ledger.outside_world,
        sand_tiles: scratch.sand_tiles.len(),
        water_tiles: scratch.water_tiles.len(),
        oil_tiles: scratch.oil_tiles.len(),
        visual_tiles: over(&scratch.sand_tiles, MATERIAL_VISUAL_THRESHOLD)
            + over(&scratch.water_tiles, MATERIAL_VISUAL_THRESHOLD)
            + over(&scratch.oil_tiles, MATERIAL_VISUAL_THRESHOLD),
        sand_tiles_over_threshold: over(&scratch.sand_tiles, SAND_THRESHOLD),
        liquid_tiles_over_threshold: over(&scratch.water_tiles, LIQUID_THRESHOLD)
            + over(&scratch.oil_tiles, LIQUID_THRESHOLD),
        sand_blocks_appended: sand_added,
        liquid_regions_appended: liquid_added,
        overlay_gate_solids: overlay.gate_solids.len(),
        overlay_water_regions: overlay.water_regions.len(),
        capped: sand_added >= MAX_DYNAMIC_SAND_TILES || liquid_added >= MAX_DYNAMIC_LIQUID_TILES,
    };

    sync_material_visuals(&mut commands, &world.0, &scratch.desired_visuals, &visuals);
}

/// Warn ONCE when the per-material tile cap truncates a projection. A truncated
/// frame is indistinguishable from a settled one from the outside — a pool simply
/// stops growing — so a silent cap reads as a physics bug. (`no silent caps`.)
fn warn_on_projection_cap(ledger: &TallyLedger, scratch: &ProjectionScratch, warned: &mut bool) {
    if *warned {
        return;
    }
    let sand_tiles = scratch.sand_tiles.len();
    let liquid_tiles = scratch.water_tiles.len() + scratch.oil_tiles.len();
    if sand_tiles > MAX_DYNAMIC_SAND_TILES || liquid_tiles > MAX_DYNAMIC_LIQUID_TILES {
        bevy::log::warn!(
            "falling_sand_room: projection cap reached — {sand_tiles} sand tiles \
             (cap {MAX_DYNAMIC_SAND_TILES}), {liquid_tiles} liquid tiles (cap \
             {MAX_DYNAMIC_LIQUID_TILES}). Matter beyond the cap is SIMULATED but not \
             projected into collision; the pool will look like it stopped growing. \
             {} particles this frame; ledger: {ledger:?}",
            ledger.total()
        );
        *warned = true;
    }
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

fn project_sand(out_blocks: &mut Vec<ae::Block>, scratch: &mut ProjectionScratch) -> usize {
    let keys = sorted_tiles_by_count_desc(&scratch.sand_tiles);
    let mut added = 0;
    for (tile_x, tile_y) in keys {
        let count = scratch
            .sand_tiles
            .get(&(tile_x, tile_y))
            .copied()
            .unwrap_or(0);
        if count >= MATERIAL_VISUAL_THRESHOLD {
            scratch
                .desired_visuals
                .insert((tile_x, tile_y), MaterialKind::Sand);
        }
        if count < SAND_THRESHOLD || added >= MAX_DYNAMIC_SAND_TILES {
            continue;
        }
        scratch.dense_sand.insert((tile_x, tile_y));
        out_blocks.push(ae::Block::one_way(
            format!("falling_sand:sand:{tile_x}:{tile_y}"),
            tile_min(tile_x, tile_y),
            tile_size_vec(),
        ));
        added += 1;
    }
    added
}

fn project_liquid(
    out_water: &mut Vec<ae::WaterRegion>,
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
        out_water.push(water_tile_region(tile_x, tile_y, water_kind, spec));
        *added += 1;
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
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn log_falling_sand_diagnostics(
    time: Res<Time>,
    state: Res<FallingSandRoomState>,
    room_set: ambition::platformer::lifecycle::SessionWorldRef<ambition_actors::rooms::RoomSet>,
    particles: Query<(&Particle, &GridPosition)>,
    // Component-presence query: every particle that has Particle should
    // ALSO get Density/Speed/Movement/AirResistance/MovementRng inherited
    // from its ParticleType via the sync propagators. If a particle is
    // missing any of these, the movement query in PostUpdate skips it
    // entirely and it just sits in the spout cell — exactly the
    // "stuck at grid_y=230" symptom from the previous run.
    movement_ready: Query<
        Entity,
        (
            With<Particle>,
            With<Density>,
            With<Speed>,
            With<Movement>,
            With<AirResistance>,
            With<MovementRng>,
        ),
    >,
    no_density: Query<Entity, (With<Particle>, Without<Density>)>,
    no_speed: Query<Entity, (With<Particle>, Without<Speed>)>,
    no_movement: Query<Entity, (With<Particle>, Without<Movement>)>,
    no_air: Query<Entity, (With<Particle>, Without<AirResistance>)>,
    no_rng: Query<Entity, (With<Particle>, Without<MovementRng>)>,
    projection: Res<FallingSandProjectionReport>,
    sand: Res<FallingSandWorld>,
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
    // The "floor band" is the seed-wall strip on top of the lowest
    // visible LDtk block. World Y increases downward in our convention,
    // so the floor is the block with the LARGEST `min.y` (the topmost
    // edge of the lowest block). Previously this used `min(block.min.y)`
    // which finds the topmost CEILING block — making `near_floor` and
    // `below_floor` meaningless for the actual pile location.
    let floor_block_top_world_y = world
        .blocks
        .iter()
        .map(|b| b.aabb.min.y)
        .fold(f32::NEG_INFINITY, f32::max);
    let band_top_world_y = floor_block_top_world_y;
    let band_bottom_world_y = floor_block_top_world_y + FLOOR_WALL_THICKNESS as f32;
    // Convert to grid_y. Recall: grid_y = size.y/2 - world_y, so a
    // SMALLER world_y maps to a LARGER grid_y. The floor band's TOP
    // edge in world is its BOTTOM edge in grid space and vice versa.
    let band_grid_y_high = (world.size.y * 0.5 - band_top_world_y).round() as i32;
    let band_grid_y_low = (world.size.y * 0.5 - band_bottom_world_y).round() as i32;

    let mut counts: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
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

    let total_particles =
        movement_ready.iter().count() + no_density.iter().count().max(no_speed.iter().count());
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
    // The sand grid's conservation ledger: loose + settled must equal emitted,
    // every tick, or the CA itself is losing matter.
    if let Some(grid) = sand.grid.as_ref() {
        bevy::log::info!(
            "fs-diag sand-grid: tick={}  emitted={}  loose={}  settled={}  \
             solid_tiles={}  conserved={}",
            grid.tick(),
            grid.emitted(),
            grid.loose(),
            sand.ledger.total(),
            sand.ledger.solid_tiles().count(),
            grid.conserved_with(&sand.ledger),
        );
    }
    // The particle→environment funnel. Read left to right: the first stage that
    // collapses to zero is where the matter is being lost.
    let p = &*projection;
    bevy::log::info!(
        "fs-diag projection: particles(sand={} water={} oil={} outside_room={}) \
         -> tiles(sand={} water={} oil={}) \
         -> over_env_threshold(sand>={} :{}  liquid>={} :{}) \
         -> appended(sand_blocks={} liquid_regions={}) \
         -> overlay(gate_solids={} water_regions={})  visual_tiles={}  capped={}",
        p.sand_particles,
        p.water_particles,
        p.oil_particles,
        p.outside_world,
        p.sand_tiles,
        p.water_tiles,
        p.oil_tiles,
        SAND_THRESHOLD,
        p.sand_tiles_over_threshold,
        LIQUID_THRESHOLD,
        p.liquid_tiles_over_threshold,
        p.sand_blocks_appended,
        p.liquid_regions_appended,
        p.overlay_gate_solids,
        p.overlay_water_regions,
        p.visual_tiles,
        p.capped,
    );
    bevy::log::info!(
        "fs-diag components: movement_ready={}  no_density={}  no_speed={}  no_movement={}  no_air_resistance={}  no_movement_rng={}  (total {:?})",
        movement_ready.iter().count(),
        no_density.iter().count(),
        no_speed.iter().count(),
        no_movement.iter().count(),
        no_air.iter().count(),
        no_rng.iter().count(),
        total_particles,
    );
}

/// What the particle→environment projection actually did last frame.
///
/// The seam this room's regressions hide in: particles can pile up visibly
/// while contributing nothing to collision or liquid, because tiles sit under
/// the density thresholds, or the cap truncated them, or the contribution was
/// emitted and then dropped by the overlay rebuild. The particle-side
/// diagnostics cannot tell those apart — they only see particles. This records
/// each step of the funnel so one log line says WHICH stage lost the matter.
#[derive(Resource, Clone, Copy, Debug, Default)]
struct FallingSandProjectionReport {
    /// Particles tallied into a tile bucket, by material.
    sand_particles: usize,
    water_particles: usize,
    oil_particles: usize,
    /// Particles whose grid cell mapped to no tile of this room.
    outside_world: usize,
    /// Distinct tiles holding any of that material.
    sand_tiles: usize,
    water_tiles: usize,
    oil_tiles: usize,
    /// Tiles at or above the VISUAL threshold (what the player sees).
    visual_tiles: usize,
    /// Tiles at or above the ENVIRONMENT threshold (what collision/liquid uses).
    sand_tiles_over_threshold: usize,
    liquid_tiles_over_threshold: usize,
    /// What the projection actually appended to the overlay.
    sand_blocks_appended: usize,
    liquid_regions_appended: usize,
    /// Overlay totals after this projection, including other contributors.
    overlay_gate_solids: usize,
    overlay_water_regions: usize,
    /// The per-material cap truncated this frame.
    capped: bool,
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
            Self::Sand => ambition_actors::config::rgba(0.95, 0.74, 0.28, 0.72),
            Self::Water => ambition_actors::config::rgba(0.18, 0.55, 1.0, 0.48),
            Self::Oil => ambition_actors::config::rgba(0.20, 0.13, 0.06, 0.66),
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

    // Spawn in a FIXED tile order, not `desired`'s hash order: `desired` is a std
    // `HashMap` whose iteration is per-process (ADR 0023 rule 3), and this spawns
    // entities. AMBITION_REVIEW(determinism): the `.iter()` here is immediately
    // collected and sorted by the `(i32, i32)` tile key BEFORE any spawn, so no
    // hash-order is ever observed — the sort is the determinism, not the iteration.
    let mut ordered: Vec<(&(i32, i32), &MaterialKind)> = desired.iter().collect();
    ordered.sort_by_key(|(tile, _)| **tile);
    for (&tile, &kind) in ordered {
        if existing.contains(&(tile, kind)) {
            continue;
        }
        let center = tile_min(tile.0, tile.1)
            + ae::Vec2::new(TILE_SIZE as f32 * 0.5, TILE_SIZE as f32 * 0.5);
        commands.spawn((
            Name::new(format!(
                "falling sand projected {kind:?} tile {}:{}",
                tile.0, tile.1
            )),
            Sprite::from_color(kind.visual_color(), Vec2::splat(TILE_SIZE as f32)),
            Transform::from_translation(ambition_engine_core::config::world_to_bevy(
                world,
                center,
                ambition_engine_core::config::WORLD_Z_PLAYER + 4.0,
            )),
            FallingSandMaterialVisual { tile, kind },
            ambition_actors::platformer_runtime::lifecycle::RoomVisual,
        ));
    }
}

/// Draw the sand grid ("always draw blind" — the sim half never ships without
/// its visual): one room-sized RGBA texture at cell resolution, loose grains
/// in the classic three-tone palette, settled ground in a deeper tone. The
/// texture is rewritten only on sim ticks that actually advanced the grid, so
/// a paused game costs nothing.
fn sync_sand_grid_texture(
    mut commands: Commands,
    room_set: ambition::platformer::lifecycle::SessionWorldRef<ambition_actors::rooms::RoomSet>,
    state: Res<FallingSandRoomState>,
    sand: Res<FallingSandWorld>,
    mut images: ResMut<Assets<Image>>,
    mut visuals: Query<(Entity, &mut FallingSandGridVisual)>,
) {
    use bevy::asset::RenderAssetUsages;
    use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

    let room = room_set.active_spec();
    let active = state.active_room && room.id == ROOM_ID;
    let grid = match sand.grid.as_ref() {
        Some(grid) if active => grid,
        _ => {
            for (entity, _) in &visuals {
                commands.entity(entity).despawn();
            }
            return;
        }
    };

    let Some((_, mut visual)) = visuals.iter_mut().next() else {
        let image = Image::new_fill(
            Extent3d {
                width: grid.width() as u32,
                height: grid.height() as u32,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            &[0, 0, 0, 0],
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::default(),
        );
        let handle = images.add(image);
        let center = ae::Vec2::new(room.world.size.x * 0.5, room.world.size.y * 0.5);
        commands.spawn((
            Name::new("falling sand grid texture"),
            Sprite {
                image: handle.clone(),
                custom_size: Some(Vec2::new(room.world.size.x, room.world.size.y)),
                ..default()
            },
            Transform::from_translation(ambition_engine_core::config::world_to_bevy(
                &room.world,
                center,
                // Just under the per-tile water/oil settle sprites so the two
                // presentations never z-fight.
                ambition_engine_core::config::WORLD_Z_PLAYER + 3.5,
            )),
            FallingSandGridVisual {
                image: handle,
                drawn_tick: None,
            },
            ambition_actors::platformer_runtime::lifecycle::RoomVisual,
        ));
        return;
    };

    if visual.drawn_tick == Some(grid.tick()) {
        return;
    }
    let Some(image) = images.get_mut(&visual.image) else {
        return;
    };
    let Some(data) = image.data.as_mut() else {
        return;
    };
    paint_sand_grid(grid, data);
    visual.drawn_tick = Some(grid.tick());
}

/// Cell → pixel. Grid coords ARE world pixels (y down), and texture row 0 is
/// the sprite's top, so the mapping is the identity.
fn paint_sand_grid(grid: &crate::falling_sand_sim::SandGrid, data: &mut [u8]) {
    // The old sand ParticleType's palette (#E2C16A / #B99045 / #F1D98A),
    // chosen per cell by a position hash; settled ground one deeper tone.
    const LOOSE: [[u8; 3]; 3] = [[226, 193, 106], [185, 144, 69], [241, 217, 138]];
    const SETTLED: [u8; 3] = [150, 116, 56];
    for y in 0..grid.height() {
        for x in 0..grid.width() {
            let i = ((y * grid.width() + x) * 4) as usize;
            let (rgb, alpha) = match grid.get(x, y) {
                SandCell::Sand => (LOOSE[((x * 31 + y * 17) % 3) as usize], 255),
                SandCell::Settled => (SETTLED, 255),
                _ => ([0, 0, 0], 0),
            };
            data[i..i + 3].copy_from_slice(&rgb);
            data[i + 3] = alpha;
        }
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

#[cfg(test)]
mod tests;
