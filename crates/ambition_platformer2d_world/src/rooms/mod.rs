//! Room graph and authored room IR.

use ambition_platformer2d_core as ae;
use bevy_ecs::prelude::{Component, Message};
use petgraph::graph::{Graph, NodeIndex};

mod camera;
mod gate_portal;
mod graph;
mod loading_zone;
mod metadata;
mod room_graph;
mod rollback;
mod spawn;
mod specs;

pub use camera::*;
pub use gate_portal::*;
pub use ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance;
pub use rollback::register_rollback_state;
pub use loading_zone::*;
pub use metadata::*;
pub use room_graph::*;
pub use spawn::validated_spawn;
pub use specs::*;
