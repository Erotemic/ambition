//! Map / minimap state AND the Map tab that renders it.
//!
//! `MapMenuState` holds the visited-room set, per-room geometry
//! (`MapRoomNode`), open/minimap toggles, and the clamped zoom level
//! (`MAP_ZOOM_MIN`..`MAP_ZOOM_MAX`). `summary_lines` produces the text the HUD
//! shows; a host-owned UI adapter can render it as a full map, minimap, or menu tab.

use std::collections::BTreeSet;

use bevy::prelude::*;

#[derive(Clone, Debug)]
pub struct MapRoomNode {
    pub id: String,
    pub world_min: Vec2,
    pub world_size: Vec2,
}

#[derive(Resource)]
pub struct MapMenuState {
    pub open: bool,
    pub minimap_enabled: bool,
    pub visited: BTreeSet<String>,
    pub rooms: Vec<MapRoomNode>,
    pub zoom: f32,
}

impl Default for MapMenuState {
    fn default() -> Self {
        Self {
            open: false,
            minimap_enabled: false,
            visited: BTreeSet::new(),
            rooms: Vec::new(),
            zoom: 1.0,
        }
    }
}

pub const MAP_ZOOM_STEP: f32 = 1.25;
pub const MAP_ZOOM_MIN: f32 = 0.5;
pub const MAP_ZOOM_MAX: f32 = 4.0;

impl MapMenuState {
    pub fn toggle_open(&mut self) {
        self.open = !self.open;
    }

    pub fn toggle_minimap(&mut self) {
        self.minimap_enabled = !self.minimap_enabled;
    }

    pub fn zoom_in(&mut self) {
        self.zoom = (self.zoom * MAP_ZOOM_STEP).clamp(MAP_ZOOM_MIN, MAP_ZOOM_MAX);
    }

    pub fn zoom_out(&mut self) {
        self.zoom = (self.zoom / MAP_ZOOM_STEP).clamp(MAP_ZOOM_MIN, MAP_ZOOM_MAX);
    }

    pub fn zoom_reset(&mut self) {
        self.zoom = 1.0;
    }

    pub fn record_visit(&mut self, room_id: &str) {
        self.visited.insert(room_id.to_string());
    }

    pub fn summary_lines(&self, current_room: &str) -> Vec<String> {
        if !self.open {
            if self.minimap_enabled {
                return vec![format!(
                    "minimap: {} visited / current = {}",
                    self.visited.len(),
                    current_room
                )];
            }
            return Vec::new();
        }
        let mut lines = vec![format!("MAP — {} visited", self.visited.len())];
        for id in &self.visited {
            let marker = if id == current_room { "→" } else { " " };
            lines.push(format!("{marker} {id}"));
        }
        lines
    }
}

/// The map-menu DOMAIN's sim-state plugin (track 6, decision #9): the crate
/// owns its own visited-rooms/map state; the sim assembly only adds the
/// plugin. Deliberately a bare resource init — the menu-host reusable/product
/// line is drawn by the second consumer (decision #7), not in advance.
/// The set [`populate_map_rooms`] runs in — **published so a composition can
/// bracket it by PHASE instead of by naming the function.**
///
/// ⭐ THE PATTERN IS THE ONE `ambition_app` ALREADY USES NEXT DOOR: *"the plugin
/// publishes `AudioInitSet` and the host brackets it here, beside every other
/// `phase_mark`."* The app's startup profile has a mark named
/// `after_map_menu_spawn` whose whole job is to time this system, so the mark
/// needs something to order against once the system stops being written inline.
#[cfg(feature = "ldtk")]
#[derive(bevy::prelude::SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MapMenuSpawnSet;

pub struct MapStatePlugin;

impl bevy::prelude::Plugin for MapStatePlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.init_resource::<MapMenuState>();
        // ⭐⭐ AND THE TWO SYSTEMS THIS DOMAIN OWNS, because the SECOND consumer
        // the note above waits for has arrived: the app's shell host and the
        // runtime's progression schedule both reach into this crate and name
        // these functions themselves.
        //
        // ⛔ `handle_map_menu_hotkeys` WAS A PASSENGER IN SOMEBODY ELSE'S CHAIN.
        // The host registered it `.chain()`ed behind `handle_ldtk_hot_reload` and
        // `handle_trace_hotkey` — three crates in one sequence — and that chain
        // exists for a reason that is not this system's: the two ahead of it were
        // ordered because `handle_debug_hotkeys` and `handle_ldtk_hot_reload` both
        // write `DeveloperRuntimeState`. ⇒ MEASURED: the three write DISJOINT
        // resources (`DeveloperRuntimeState`, `GameplayTraceBuffer`,
        // `MapMenuState`), so the order was grouping, not a dependency.
        //
        // ⭐ WHAT IT ACTUALLY NEEDS is expressible in vocabulary this crate
        // already depends on: after the simulation phase, while a session world
        // exists. No new dependency edge — `Platformer2dSimulationPhaseMonolith`
        // and `session_world_exists` both live in `shared_tangle`.

        // ⛔⛔ VOCABULARY ONLY. The systems live in
        // [`install_map_menu_systems`], and that is a FUNCTION on purpose:
        // a `Plugin` inside a plugin group answers "does this composition
        // run these?" for everybody. When this plugin added them, the
        // runtime group carried them into EVERY composition and
        // `handle_map_menu_hotkeys` panicked in headless apps with no
        // `ButtonInput` -- six unrelated tests, from a population nobody
        // enumerated.
    }
}

/// Install the map menu's systems.
///
/// ⭐⭐ A FUNCTION, NOT A PLUGIN, AND THE DISTINCTION IS THE POINT. The
/// capability declares its own vocabulary unconditionally ([`MapStatePlugin`]
/// owns [`MapMenuState`]); WHETHER a composition runs these systems is the
/// composition's to answer. ⇒ The caller names ONE function instead of three
/// private systems -- the whole benefit of the carve -- without deciding for
/// hosts that never wanted a map.
///
/// ⛔⛔ **THE CONTRACT: CALL THIS FROM A COMPOSITION THAT HAS INPUT.** These systems
/// take `Res<ButtonInput<KeyCode>>` and `Res<MenuControlFrame>` STRICTLY, and both
/// come from the windowed host's input plugins. ⇒ There is deliberately NO
/// `resource_exists` guard: one was added after this carve panicked headless apps,
/// and a review named it correctly as runtime feature-detection standing in for a
/// composition contract. ⚠ **It was also only half a guard** — it tested
/// `ButtonInput` while `MenuControlFrame` comes from `HostInputBindingsPlugin`, so
/// a composition with Bevy's `InputPlugin` and without the host would have passed
/// the condition and still failed the parameter.
///
/// ⇒ Calling this from a host with no input is a COMPOSITION ERROR and should
/// crash loudly at startup rather than be silently skipped for the process's life.
pub fn install_map_menu_systems(app: &mut bevy::prelude::App) {
    // ⭐ THE STARTUP HALF. `populate_map_rooms` reads `Res<ActiveLdtkProject>` and
    // writes the room list once; the app had it inline in a Startup chain ordered
    // `.after(setup_simulation_system)` — a system that is never registered, so
    // that edge was a no-op (removed 2026-09-06). ⇒ ITS REAL PREREQUISITE IS THE
    // RESOURCE, which is a `run_if` and needs no host anchor at all.
    #[cfg(feature = "ldtk")]
    app.add_systems(
        bevy::prelude::Startup,
        systems::populate_map_rooms.in_set(MapMenuSpawnSet).run_if(
            bevy::prelude::resource_exists::<ambition_platformer2d_ldtk::ActiveLdtkProject>,
        ),
    );
    // ⛔ ONE SYSTEM OF THIS DOMAIN IS STILL INSTALLED BY THE HOST, and the
    // reason is a real prerequisite rather than an oversight.
    // `populate_map_rooms` sits in the app's STARTUP chain, bracketed by
    // profiling marks (`after_map_menu_spawn` exists to time it) and ordered
    // `.after(setup_simulation_system)` — a host FUNCTION, which this crate
    // cannot name and should not.
    //
    // ⭐ THE PATTERN TO FOLLOW IS THREE LINES ABOVE IT IN THAT FILE: *"the
    // plugin publishes `AudioInitSet` and the host brackets it here"*. ⇒ The
    // carve wants this plugin to publish a `MapMenuSpawnSet`, install
    // `populate_map_rooms` into it, and the host to order its phase mark
    // `.after(MapMenuSpawnSet)` — plus a published set standing where
    // `setup_simulation_system` stands today, which is the host's to make.
    // ⚠ It is also `#[cfg(feature = "ldtk")]`, so the install carries the gate.
    app.add_systems(
            bevy::prelude::Update,
            (
                // ⛔⛔ GATED ON THE INPUT RESOURCE, and the carve is why. While the
                // SHELL registered this, its population was "compositions with a
                // shell", which always carry `bevy_input`. `MapStatePlugin` is
                // installed by the runtime plugin group, so moving the install
                // here widened the population to EVERY composition -- including
                // headless test apps with no `ButtonInput<KeyCode>`, where a
                // missing `Res` is a validation PANIC rather than a skip. Six
                // unrelated app tests died on it at once.
                //
                // ⇒ MOVING AN INSTALL MOVES ITS POPULATION, and a system's params
                // are a claim about the composition it runs in.
                //
                // ✔ THE OTHER TWO WERE CHECKED THE SAME WAY rather than trusted to
                // a green suite: `map_menu_pointer_dismiss` and `sync_map_menu`
                // need only `MapMenuState` -- which this plugin supplies itself --
                // plus queries, and a query that matches nothing is an empty
                // iteration rather than a panic. The hotkey was the only one
                // reaching outside the plugin's own resources.
                // ⛔⛔ NO `run_if` HERE, and the absence is the contract. A
                // `resource_exists::<ButtonInput>` guard stood on this line while the
                // docstring above said it had been removed -- the doc described the
                // decision and the code kept the sniff, so the two disagreed and a
                // review had to find it. ⚠ It was also only HALF a guard: it tested
                // `ButtonInput` while this system equally requires `MenuControlFrame`
                // from `HostInputBindingsPlugin`, so a composition with Bevy's
                // `InputPlugin` and no host passed the condition and then failed the
                // parameter anyway. ⇒ Calling this installer without input is a
                // COMPOSITION ERROR and crashes loudly, which is the point.
                input::handle_map_menu_hotkeys,
                pointer::map_menu_pointer_dismiss,
                // ⭐ THE VIEW JOINS ITS OWN DOMAIN. The host registered this with
                // the IDENTICAL ordering the hotkey needed -- after the simulation
                // phase, while a session world exists -- in a separate
                // `add_systems` call thirty lines away. One group says once what
                // three registrations said three times.
                ui::sync_map_menu,
            )
                .after(
                    ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhaseMonolith::CoreSimulation,
                )
                .run_if(ambition_platformer2d_shared_tangle::lifecycle::session_world_exists),
        );
}

// Nothing in the simulation ever called them: their only consumers are the runtime's progression
// schedule and the app's shell host, both of which reach this crate directly.
mod input;
mod pointer;
mod systems;
mod ui;

#[cfg(test)]
mod tests;

pub use input::handle_map_menu_hotkeys;
pub use pointer::map_menu_pointer_dismiss;
#[cfg(feature = "ldtk")]
pub use systems::populate_map_rooms;
pub use systems::{sync_map_from_save, track_room_visits};
pub use ui::{spawn_map_menu_with_scope, sync_map_menu, MapMenuRoot};

#[cfg(test)]
use ui::short_room_label;
