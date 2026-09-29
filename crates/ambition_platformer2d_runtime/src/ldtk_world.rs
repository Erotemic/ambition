//! Opt-in host composition for games that use an LDtk world. Installs the LDtk
//! runtime spine; it is not part of the default engine plugin group.

use bevy::app::{App, Plugin};

/// A game with an LDtk world adds this; nothing else does.
///
/// Installs the LDtk runtime spine (index rebuild chain + its index resources).
/// Deliberately NOT part of [`crate::PlatformerEnginePlugins`] — see the module
/// doc.
///
/// The format adds no rollback row: `LdtkRuntimeIndex` is prepared content
/// that no session writes, and the active area is `RoomSet`'s.
pub struct LdtkWorldPlugin;

impl Plugin for LdtkWorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ambition_platformer2d_ldtk::LdtkRuntimeSpinePlugin);
    }
}
