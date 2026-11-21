//! Frame data, live: every move in play as a training display prints it.
//!
//! The combat overlay's phase bar (F1, `Combat` preset) shows where a move's
//! clock is against its windows. This panel prints the numbers: the frame the
//! move is on, the frame it hits on, its active frames and its recovery
//! (`MoveFrameReadout`). With the frame-step panel beside it, a move can be
//! walked one frame at a time and read.
//!
//! Host developer state: it reads the simulation and writes nothing.

use ambition_platformer2d::actor::MatchSeat;
use ambition_platformer2d::combat::moveset::frame_readout::MoveFrameReadout;
use ambition_platformer2d::combat::moveset::MovePlayback;
use ambition_platformer2d::dev_tools::dev_tools::DeveloperTools;
use bevy::prelude::*;
use bevy_inspector_egui::bevy_egui::{egui, EguiContext, EguiPrimaryContextPass, PrimaryEguiContext};

pub(crate) struct FrameDataPanelPlugin;

impl Plugin for FrameDataPanelPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(EguiPrimaryContextPass, frame_data_ui);
    }
}

/// Every move in play, one line each, seats first in seat order.
fn readouts(world: &mut World) -> Vec<String> {
    let hz = ambition_platformer2d::runtime::SIM_TICK_HZ as f32;
    let mut rows: Vec<(Option<usize>, String)> = world
        .query::<(Entity, &MovePlayback, Option<&MatchSeat>, Option<&Name>)>()
        .iter(world)
        .map(|(body, playback, seat, name)| {
            let who = match (seat, name) {
                (Some(seat), _) => format!("P{}", seat.0 + 1),
                (None, Some(name)) => name.to_string(),
                (None, None) => format!("{body}"),
            };
            (seat.map(|seat| seat.0), format!("{who}: {}", MoveFrameReadout::of(playback, hz).line()))
        })
        .collect();
    rows.sort_by(|a, b| (a.0.is_none(), a.0, &a.1).cmp(&(b.0.is_none(), b.0, &b.1)));
    rows.into_iter().map(|(_, line)| line).collect()
}

fn frame_data_ui(world: &mut World) {
    if !world
        .get_resource::<DeveloperTools>()
        .is_some_and(|tools| tools.inspector_visible)
    {
        return;
    }
    let Ok(context) = world
        .query_filtered::<&mut EguiContext, With<PrimaryEguiContext>>()
        .single(world)
    else {
        return;
    };
    let mut context = context.clone();
    let tick = world
        .get_resource::<ambition_platformer2d::sim::SimTick>()
        .map(|tick| tick.get());
    let lines = readouts(world);
    egui::Window::new("Frame data")
        .default_width(520.0)
        .resizable(true)
        .show(context.get_mut(), |ui| {
            if let Some(tick) = tick {
                ui.label(format!("sim tick {tick}"));
            }
            if lines.is_empty() {
                ui.label("no move in play");
            }
            for line in &lines {
                ui.monospace(line);
            }
        });
}
