//! Named Ambition item-roster / default-inventory registration.
//!
//! Owns inserting the starter [`OwnedItems`] roster (the 24-item catalog's
//! default ownership set). The item enum + metadata still live in
//! `ambition_sandbox::items`; this module only owns the decision to seed the default
//! inventory as a sandbox resource.
//!
//! NOTE: this is installed from the *presentation* assembly
//! (`app::plugins::install_menu_setup_and_hotkeys`), not from
//! `add_simulation_plugins`, to preserve the exact original insertion
//! point. Headless / RL builds that skip presentation therefore do not
//! seed the starter roster — matching pre-Stage-11 behavior. Moving the
//! insert earlier would change which builds carry `OwnedItems`, so it
//! stays presentation-scoped until that is intentionally revisited.

use bevy::prelude::*;

/// Installs the starter Ambition item-ownership roster.
pub struct AmbitionItemRosterPlugin;

impl Plugin for AmbitionItemRosterPlugin {
    fn build(&self, app: &mut App) {
        // The 24-item catalog ownership model is always-on core state (pickups
        // and dialogue read/write it regardless of which menu renders it).
        app.insert_resource(ambition_sandbox::items::OwnedItems::starter());
    }
}
