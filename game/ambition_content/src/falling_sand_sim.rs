//! Falling-sand room SIMULATION — the deterministic, headless-safe half.
//!
//! This module owns everything about the falling-sand room that belongs to the
//! sim tick: the room/switch/spout state, the sand grid CA ([`sand_grid`]),
//! the FS3 settled-sand ledger, and the persistent collision projection. It is
//! deliberately **not** behind the `falling_sand` cargo feature and has no
//! `bevy_falling_sand` dependency, so:
//!
//! - its conservation and settling proofs run in every
//!   `cargo test -p ambition_content` (the F13 lesson: a feature-gated test is
//!   a test that silently stops running), and
//! - the headless `SandboxSim` harness can drive the room end-to-end, which is
//!   what the authored-room regression in `ambition_app` does.
//!
//! The feature-gated sibling [`crate::falling_sand`] keeps the
//! `bevy_falling_sand` bridge for WATER and OIL (their slice has not landed)
//! plus all presentation: particle rendering, nozzle sprites, the sand-grid
//! texture, diagnostics. Registration: [`FallingSandSimPlugin`] is added by
//! `AmbitionContentPlugin` under the `falling_sand` feature so feature-bundle
//! semantics are unchanged; the presentation plugin remains visible-binary
//! only.
//!
//! # One solver step per simulation tick
//!
//! [`step_sand_grid`] runs in the SIM schedule, so the CA advances exactly
//! when the simulation does — never on render frames while the game is paused,
//! never zero-or-twice under fixed-tick catch-up. (This is the property
//! `bevy_falling_sand` structurally cannot provide; `falling-sand.md` §4
//! records the adapt-vs-replace ruling.) Under a rollback host, emission and
//! stepping are additionally gated on `simulation_pass_is_authoritative`: the
//! grid is not rollback-registered state, so a re-simulated frame must not
//! double-advance it — while [`project_settled_sand`] runs on EVERY pass, so
//! replayed player physics still sees the same settled ground.

pub mod sand_grid;

use ambition_engine_core as ae;
use ambition_platformer_primitives::schedule::{
    simulation_pass_is_authoritative, SandboxSet, SimScheduleExt,
};
use bevy::prelude::*;

pub use sand_grid::{SandCell, SandGrid, SettledSandLedger};

pub const ROOM_ID: &str = "falling_sand_room";

/// Particle-type names, shared with the `bevy_falling_sand` bridge module.
/// Sand's name survives only to mark its mouths in the spout table — no sand
/// particle is ever spawned into the external crate anymore.
pub const TYPE_SAND: &str = "AmbitionSand";
pub const TYPE_WATER: &str = "AmbitionWater";
pub const TYPE_OIL: &str = "AmbitionOil";
pub const TYPE_WALL: &str = "AmbitionWall";

pub const SAND_SWITCH: &str = "falling_sand_sand_switch";
pub const WATER_SWITCH: &str = "falling_sand_water_switch";
pub const OIL_SWITCH: &str = "falling_sand_oil_switch";
pub const MIXED_SWITCH: &str = "falling_sand_mixed_switch";

pub const TILE_SIZE: i32 = 16;
/// Floor / side-wall thickness in particle cells. Needs to be deep
/// enough that high-density material can't tunnel through during a
/// single sim step; 16 has held up in practice where 2 did not.
pub const FLOOR_WALL_THICKNESS: i32 = 16;
pub const SIDE_WALL_THICKNESS: i32 = 8;

/// Emission budget for the sand grid, in grains. The spout stops when the
/// total ever emitted reaches this (nothing drains in this slice, so emitted
/// == live mass). NOT silent: the emitter warns once when the budget closes
/// the spout.
pub const MAX_SAND_EMISSION: u64 = 120_000;

#[derive(Resource, Default)]
pub struct FallingSandRoomState {
    pub active_room: bool,
    pub last_room_id: Option<String>,
    /// Snapshot of the active player's swim ability at the moment the
    /// room was entered. Restored on exit so the room's forced-swim
    /// effect doesn't leak into other rooms.
    ///
    /// Stored as a single value (not keyed by `Entity`) because the
    /// sandbox is single-player; an Entity-keyed map would leak
    /// entries every time the player respawned with a new Entity id
    /// while still inside the room.
    pub swim_snapshot: Option<SwimSnapshot>,
    pub seeded_boundaries: bool,
    pub spouts: FallingSandSpoutState,
}

/// Stored player swim state plus a marker so we can tell whether the
/// snapshot belongs to the currently spawned player entity. If the
/// player respawns inside the room, the previous snapshot becomes
/// stale and we re-capture from the new entity's current swim state.
#[derive(Clone, Copy, Debug)]
pub struct SwimSnapshot {
    pub player_entity: Entity,
    pub previous_swim: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FallingSandSpoutState {
    pub sand: bool,
    pub water: bool,
    pub oil: bool,
    pub mixed: bool,
}

impl FallingSandSpoutState {
    pub fn from_save(save: &ambition_persistence::save_data::SandboxSaveData) -> Self {
        Self {
            sand: save.switch(SAND_SWITCH),
            water: save.switch(WATER_SWITCH),
            oil: save.switch(OIL_SWITCH),
            mixed: save.switch(MIXED_SWITCH),
        }
    }

    pub fn toggle(&mut self, id: &str) -> bool {
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

/// One spout mouth: what it emits, where, and how wide. A **table**, because
/// `falling-sand.md` §1 rules that a spout is an authored PLACEMENT
/// (`PlacementSchema::Spout { material, rate, direction }`) lowered by content
/// — not a hardcoded runtime spawn. Until [W-a]/[W-b] land, this is the same
/// data in the same shape, one `const` away from being read off the map.
pub struct SpoutMouth {
    pub particle_type: &'static str,
    pub x: f32,
    pub y: f32,
    /// Mouth width in particle cells.
    pub width: i32,
}

pub const SOLO_SPOUT_WIDTH: i32 = 8;
/// Mixed splits the same per-frame budget across three streams.
pub const MIXED_SPOUT_WIDTH: i32 = 3;

pub const SAND_SPOUT: SpoutMouth = SpoutMouth {
    particle_type: TYPE_SAND,
    x: 176.0,
    y: 90.0,
    width: SOLO_SPOUT_WIDTH,
};
pub const WATER_SPOUT: SpoutMouth = SpoutMouth {
    particle_type: TYPE_WATER,
    x: 384.0,
    y: 90.0,
    width: SOLO_SPOUT_WIDTH,
};
pub const OIL_SPOUT: SpoutMouth = SpoutMouth {
    particle_type: TYPE_OIL,
    x: 592.0,
    y: 90.0,
    width: SOLO_SPOUT_WIDTH,
};
pub const MIXED_SPOUTS: [SpoutMouth; 3] = [
    SpoutMouth {
        particle_type: TYPE_SAND,
        x: 760.0,
        y: 90.0,
        width: MIXED_SPOUT_WIDTH,
    },
    SpoutMouth {
        particle_type: TYPE_WATER,
        x: 792.0,
        y: 90.0,
        width: MIXED_SPOUT_WIDTH,
    },
    SpoutMouth {
        particle_type: TYPE_OIL,
        x: 824.0,
        y: 90.0,
        width: MIXED_SPOUT_WIDTH,
    },
];

/// The mouths a switch state opens, in a fixed order. Pure, so the wiring from
/// four switches to five streams is testable without a room.
pub fn open_spouts(spouts: &FallingSandSpoutState) -> Vec<&'static SpoutMouth> {
    let mut open: Vec<&'static SpoutMouth> = Vec::new();
    if spouts.sand {
        open.push(&SAND_SPOUT);
    }
    if spouts.water {
        open.push(&WATER_SPOUT);
    }
    if spouts.oil {
        open.push(&OIL_SPOUT);
    }
    if spouts.mixed {
        open.extend(MIXED_SPOUTS.iter());
    }
    open
}

/// The room's sand matter, both owners in one resource: the loose grid and
/// the settled ledger. `grid` is `Some` only while the falling-sand room is
/// active; leaving the room clears both (matching the old particle-despawn
/// semantics on room change).
#[derive(Resource, Default)]
pub struct FallingSandWorld {
    pub grid: Option<SandGrid>,
    pub ledger: SettledSandLedger,
}

/// All falling-sand SIM systems live in this set so the feature-gated
/// presentation module can order its `bevy_falling_sand` bridge after it
/// (spout state must be synced before water/oil emission; the settled ledger
/// must be current before the liquid projection excludes its tiles).
#[derive(SystemSet, Clone, Debug, Hash, PartialEq, Eq)]
pub struct FallingSandSimSet;

pub struct FallingSandSimPlugin;

impl Plugin for FallingSandSimPlugin {
    fn build(&self, app: &mut App) {
        let sim = app.sim_schedule();
        app.init_resource::<FallingSandRoomState>()
            .init_resource::<FallingSandWorld>()
            .add_systems(
                sim,
                (
                    sync_falling_sand_room_state,
                    prepare_sand_world,
                    // A re-simulated rollback frame must not re-emit or
                    // double-step: the grid is not rollback-registered, so
                    // "advance once per authoritative tick" is the contract.
                    emit_sand_into_grid.run_if(simulation_pass_is_authoritative),
                    step_sand_grid.run_if(simulation_pass_is_authoritative),
                    // …but the projection runs EVERY pass: the overlay is
                    // rebuilt each pass, and replayed player physics must
                    // stand on the same settled ground as the original.
                    project_settled_sand,
                    grant_room_swim_controls,
                )
                    .chain()
                    // The projection contributes settled sand to the collision
                    // overlay, which the rebuild clears each frame — run after
                    // it (the same WorldPrep contract the gates use).
                    .after(ambition_actors::features::rebuild_feature_ecs_world_overlay)
                    .in_set(SandboxSet::WorldPrep)
                    .in_set(FallingSandSimSet),
            )
            .add_systems(
                sim,
                capture_falling_sand_switch_interactions.in_set(SandboxSet::GameplayEffects),
            );
    }
}

pub fn sync_falling_sand_room_state(
    room_set: ambition::platformer::lifecycle::SessionWorldRef<ambition_actors::rooms::RoomSet>,
    save: Res<ambition_persistence::save::SandboxSave>,
    mut state: ResMut<FallingSandRoomState>,
) {
    let active_id = room_set.active_spec().id.as_str();
    let active_room = active_id == ROOM_ID;

    if state.last_room_id.as_deref() == Some(active_id) {
        state.active_room = active_room;
        return;
    }

    state.last_room_id = Some(active_id.to_owned());
    state.active_room = active_room;
    state.seeded_boundaries = false;

    if active_room {
        state.spouts = FallingSandSpoutState::from_save(save.data());
    } else {
        state.spouts = FallingSandSpoutState::default();
    }
}

/// Build the sand grid on room entry (walls seeded from the SAME authored
/// blocks the player collides with), clear it on exit.
pub fn prepare_sand_world(
    room_set: ambition::platformer::lifecycle::SessionWorldRef<ambition_actors::rooms::RoomSet>,
    state: Res<FallingSandRoomState>,
    mut sand: ResMut<FallingSandWorld>,
) {
    if !state.active_room {
        if sand.grid.is_some() || !sand.ledger.is_empty() {
            *sand = FallingSandWorld::default();
        }
        return;
    }
    if sand.grid.is_some() {
        return;
    }

    let room = room_set.active_spec();
    let world = &room.world;
    let mut grid = SandGrid::new(world.size.x as i32, world.size.y as i32);

    // Side walls keep falling material inside the room; the bottom cap stops
    // anything that slips through a gap in the LDtk floor.
    grid.fill_wall_rect(0, 0, SIDE_WALL_THICKNESS, grid.height());
    grid.fill_wall_rect(
        grid.width() - SIDE_WALL_THICKNESS,
        0,
        SIDE_WALL_THICKNESS,
        grid.height(),
    );
    grid.fill_wall_rect(
        0,
        grid.height() - SIDE_WALL_THICKNESS,
        grid.width(),
        SIDE_WALL_THICKNESS,
    );

    // Mirror the LDtk room's collision blocks so sand piles ON TOP of the
    // surfaces the player actually walks on. Only the top strip of each block
    // is needed (material rests at the surface). One-way platforms are
    // deliberately skipped — falling material passes through them the way the
    // player drops through with a down-press.
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
        grid.fill_wall_rect(
            min.x.round() as i32,
            min.y.round() as i32,
            width,
            strip_height,
        );
    }

    // Low retaining lips split the spout streams into visually separate
    // columns. Grid geometry, not Ambition collision.
    let retain_top = (grid.height() - 200).max(0);
    for x in [256, 512, 704] {
        grid.fill_wall_rect(x, retain_top, 2, 190);
    }

    bevy::log::info!(
        "falling_sand_room: sand grid ready — {}x{} cells over {} authored blocks",
        grid.width(),
        grid.height(),
        world.blocks.len()
    );
    sand.grid = Some(grid);
    sand.ledger = SettledSandLedger::default();
}

/// Pour open sand mouths into the grid — the ONLY way sand matter enters.
pub fn emit_sand_into_grid(
    state: Res<FallingSandRoomState>,
    mut sand: ResMut<FallingSandWorld>,
    mut budget_warned: Local<bool>,
) {
    if !state.active_room {
        return;
    }
    let Some(grid) = sand.grid.as_mut() else {
        return;
    };
    for mouth in open_spouts(&state.spouts) {
        if mouth.particle_type != TYPE_SAND {
            continue;
        }
        if grid.emitted() >= MAX_SAND_EMISSION {
            if !*budget_warned {
                bevy::log::warn!(
                    "falling_sand_room: sand emission budget reached \
                     ({MAX_SAND_EMISSION} grains) — the spout is closed. A \
                     capped pour looks identical to a settled one from the \
                     outside, so this is said out loud once."
                );
                *budget_warned = true;
            }
            break;
        }
        let start_x = mouth.x.round() as i32 - mouth.width / 2;
        let y = mouth.y.round() as i32;
        for dx in 0..mouth.width {
            // A blocked mouth cell refuses the grain (never overwrites); the
            // effective rate self-regulates against a backed-up stream.
            grid.emit_sand(start_x + dx, y);
        }
    }
}

/// **One solver step + the FS3 transfer, once per authoritative sim tick.**
pub fn step_sand_grid(state: Res<FallingSandRoomState>, mut sand: ResMut<FallingSandWorld>) {
    if !state.active_room {
        return;
    }
    let FallingSandWorld {
        grid: Some(grid),
        ledger,
    } = &mut *sand
    else {
        return;
    };
    grid.step();
    grid.settle_into(ledger);
    debug_assert!(
        grid.conserved_with(ledger),
        "sand conservation broke: loose={} settled={} emitted={}",
        grid.loose(),
        ledger.total(),
        grid.emitted()
    );
}

/// Contribute the settled ledger's blocks to the per-frame collision overlay.
/// The LEDGER is the persistent owner; the overlay is just this frame's
/// composition of it — so the ground survives every rebuild without the grid
/// re-proving density each frame (the transient-projection flicker defect).
pub fn project_settled_sand(
    state: Res<FallingSandRoomState>,
    sand: Res<FallingSandWorld>,
    mut overlay: ResMut<ambition_platformer_primitives::feature_overlay::FeatureEcsWorldOverlay>,
) {
    if !state.active_room {
        return;
    }
    overlay.gate_solids.extend(sand.ledger.blocks());
}

pub fn capture_falling_sand_switch_interactions(
    room_set: ambition::platformer::lifecycle::SessionWorldRef<ambition_actors::rooms::RoomSet>,
    mut state: ResMut<FallingSandRoomState>,
    mut save: ResMut<ambition_persistence::save::SandboxSave>,
    mut effects: MessageReader<ambition_actors::features::SwitchActivated>,
) {
    if room_set.active_spec().id != ROOM_ID {
        return;
    }

    for effect in effects.read() {
        let ambition_actors::features::SwitchActivated { activation, .. } = effect;
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

pub fn grant_room_swim_controls(
    room_set: ambition::platformer::lifecycle::SessionWorldRef<ambition_actors::rooms::RoomSet>,
    mut state: ResMut<FallingSandRoomState>,
    mut players: Query<(Entity, &mut ambition_actors::actor::BodyAbilities)>,
) {
    if room_set.active_spec().id == ROOM_ID {
        for (entity, mut abilities) in &mut players {
            let needs_capture = state
                .swim_snapshot
                .map(|snap| snap.player_entity != entity)
                .unwrap_or(true);
            if needs_capture {
                state.swim_snapshot = Some(SwimSnapshot {
                    player_entity: entity,
                    previous_swim: abilities.abilities.swim,
                });
            }
            abilities.abilities.swim = true;
        }
        return;
    }

    let Some(snapshot) = state.swim_snapshot.take() else {
        return;
    };
    for (entity, mut abilities) in &mut players {
        if entity == snapshot.player_entity {
            abilities.abilities.swim = snapshot.previous_swim;
        }
    }
}
