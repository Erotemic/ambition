//! Room graph and authored room IR.

use ambition_engine_core as ae;
use bevy_ecs::prelude::{Component, Message};
use petgraph::graph::{Graph, NodeIndex};

mod camera;
mod gate_portal;
mod graph;
mod loading_zone;
mod metadata;
mod room_graph;
mod spawn;
mod specs;

pub use camera::*;
pub use gate_portal::*;
pub use loading_zone::*;
pub use metadata::*;
pub use room_graph::*;
pub use spawn::validated_spawn;
pub use specs::*;

/// Request to (re)spawn the active room's static visuals + parallax layers.
///
/// Simulation emits this after it has flipped the active room; presentation
/// consumes it and rebuilds visual entities from [`RoomSet`]. The message lives
/// with room vocabulary so render does not depend on the actor-domain crate just
/// to hear that room visuals are stale.
#[derive(Message, Clone, Copy, Debug, Default)]
pub struct RespawnRoomVisualsRequested;
