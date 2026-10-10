//! The mark of the mark/recall item: a place in one live room.
//!
//! The item is a procedural module (`ambition_content_modules::mark_recall`):
//! Attack sets the mark (`ambition.items.set_mark`), Blink recalls to it
//! (`ambition.items.mark`, then a transit). The mark is the world's fact: a
//! [`PlayerMark`] component on the body, so each body has its own. The beacon
//! visual (`ambition_render::rendering::mark_beacon`), the session reset and
//! the simulation view read it.

use bevy::prelude::*;

use ambition_platformer2d_core as ae;

/// The teleport mark a player dropped with the Mark/Recall item, if any. A
/// component, not a resource, so each player's mark is independent.
///
/// ⭐ A MARK IS A PLACE IN ONE LIVE ROOM (customers 1 and 2). A position names
/// a place only together with the room it is in. The mark keeps the live room
/// it was dropped in, and a recall from any other room does nothing. Before,
/// a recall after a crossing moved the body to the old coordinates in the new
/// room's geometry, with one player too.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct PlayerMark {
    /// World position of the dropped mark, or `None` until one is set.
    pub pos: Option<ae::Vec2>,
    /// The live room the mark was dropped in: the body's stamp then (`None`
    /// for a body with no stamp).
    pub room: Option<ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance>,
}
