//! Falling-sand room SIMULATION — the deterministic, headless-safe half.
//!
//! This module owns everything about the falling-sand room that belongs to the
//! sim tick: the room/switch/spout state, the sand grid CA ([`sand_grid`]),
//! the FS3 settled-sand ledger, and the persistent collision projection. It is
//! deliberately not behind the `falling_sand` cargo feature and has no
//! `bevy_falling_sand` dependency, so:
//!
//! Registration: [`FallingSandSimPlugin`] is added by `AmbitionContentPlugin` under the
//! `falling_sand` feature so feature-bundle semantics are unchanged; the presentation plugin
//! remains visible-binary only.
//!
//! # One solver step per simulation tick
//!
//! (This is the property `bevy_falling_sand` structurally cannot provide; `falling-sand.md` §4
//! records the adapt-vs-replace ruling.)
//!
//! `SandGrid` and `SettledSandLedger` are not in the GGRS registry; the authoritative-pass gate
//! below prevents a replay from double-advancing them, but a historical replay still observes
//! the grid's present/future state. The falling-sand room therefore remains outside the netcode
//! acceptance surface until the explicitly blocked fork/rewrite work in
//! `docs/planning/engine/falling-sand.md` is authorized.

pub mod sand_grid;

use ambition_platformer2d_core as ae;
use ambition_platformer2d_shared_tangle::schedule::{
    simulation_pass_is_authoritative, Platformer2dSimulationPhaseMonolith, SimScheduleExt,
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
    pub seeded_boundaries: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FallingSandSpoutState {
    pub sand: bool,
    pub water: bool,
    pub oil: bool,
    pub mixed: bool,
}

impl FallingSandSpoutState {
    /// ⛔ THE SAVE IS THE ONE ANSWER to "is this spout open". The room kept its
    /// own toggled copy beside it and re-wrote the save from that copy after the
    /// switch drain had already toggled it — two writers of one durable fact.
    /// The drain (`drain_switch_activations`, the spouts are authored
    /// `ResetEncounter`) owns the toggle; every reader derives from the save.
    pub fn from_save(save: &ambition_persistence::save_data::AmbitionGameSaveData) -> Self {
        Self {
            sand: save.switch(SAND_SWITCH),
            water: save.switch(WATER_SWITCH),
            oil: save.switch(OIL_SWITCH),
            mixed: save.switch(MIXED_SWITCH),
        }
    }
}

/// One spout mouth: what it emits, where, and how wide. A table, because
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

/// Pure, so the wiring from four switches to five streams is testable without a room.
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

/// `grid` is `Some` only while the falling-sand room is active; leaving the room clears both
/// (matching the old particle-despawn semantics on room change).
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
                    // double-step. This is only a duplicate-advance guard, NOT
                    // rollback correctness: the unsnapshotted grid/ledger keep
                    // their present state while historical frames replay (see
                    // the module-level warning and falling-sand.md).
                    emit_sand_into_grid.run_if(simulation_pass_is_authoritative),
                    step_sand_grid.run_if(simulation_pass_is_authoritative),
                    // The overlay itself is rebuilt on every pass from the
                    // current ledger. That keeps ordinary composition intact;
                    // it does not reconstruct historical sand state.
                    project_settled_sand,
                )
                    .chain()
                    // The projection contributes settled sand to the collision
                    // overlay, which the rebuild clears each frame — run after
                    // it (the same WorldPrep contract the gates use).
                    .after(ambition_platformer2d_shared_tangle::schedule::FeatureWorldOverlaySet)
                    .in_set(Platformer2dSimulationPhaseMonolith::WorldPrep)
                    .in_set(FallingSandSimSet),
            )
            .add_systems(
                sim,
                lend_room_swim
                    .in_set(ambition_platformer2d_shared_tangle::schedule::WorldPrepSet::BeforeIntegrate),
            );
    }
}

pub fn sync_falling_sand_room_state(
    room_set: ambition_platformer2d::platformer::lifecycle::SessionWorldRef<
        ambition_platformer2d::world::rooms::RoomSet,
    >,
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
}

/// Build the sand grid on room entry (walls seeded from the SAME authored
/// blocks the player collides with), clear it on exit.
pub fn prepare_sand_world(
    room_set: ambition_platformer2d::platformer::lifecycle::SessionWorldRef<
        ambition_platformer2d::world::rooms::RoomSet,
    >,
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
    save: Res<ambition_persistence::save::AmbitionGameSave>,
    mut sand: ResMut<FallingSandWorld>,
    mut budget_warned: Local<bool>,
) {
    if !state.active_room {
        return;
    }
    let Some(grid) = sand.grid.as_mut() else {
        return;
    };
    for mouth in open_spouts(&FallingSandSpoutState::from_save(save.data())) {
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

/// One solver step + the FS3 transfer, once per authoritative sim tick.
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
pub fn project_settled_sand(
    state: Res<FallingSandRoomState>,
    sand: Res<FallingSandWorld>,
    mut overlay: ResMut<
        ambition_platformer2d::world::FeatureEcsWorldOverlay,
    >,
) {
    if !state.active_room {
        return;
    }
    overlay.gate_solids.extend(sand.ledger.blocks());
}

/// This room's key in a body's [`AbilityContributions`].
///
/// [`AbilityContributions`]: ambition_platformer2d_core::AbilityContributions
pub const ROOM_SWIM: &str = "falling_sand.room_swim";

/// Players in the falling-sand room can swim. The room lends the verb while it
/// is the active room and withdraws its loan anywhere else, so whatever else
/// grants or withholds swim is untouched.
pub fn lend_room_swim(
    room_set: ambition_platformer2d::platformer::lifecycle::SessionWorldRef<
        ambition_platformer2d::world::rooms::RoomSet,
    >,
    mut players: Query<
        &mut ambition_platformer2d_core::AbilityContributions,
        With<ambition_platformer2d_shared_tangle::markers::PlayerEntity>,
    >,
) {
    let in_room = room_set.active_spec().id == ROOM_ID;
    let swim = ambition_platformer2d_core::AbilityContribution::Lend(
        ambition_platformer2d_core::AbilitySet {
            swim: true,
            ..ambition_platformer2d_core::AbilitySet::NONE
        },
    );
    for mut contributions in &mut players {
        match (in_room, contributions.get(ROOM_SWIM).is_some()) {
            (true, false) => contributions.set(ROOM_SWIM, swim),
            (false, true) => contributions.clear(ROOM_SWIM),
            _ => {}
        }
    }
}
