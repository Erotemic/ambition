//! **The items domain's rollback schema** (Campaign 2, R3).
//!
//! Ground items and the pickup bookkeeping a rewind has to put back — including who was reaching for what.
//!
//! ⚠ **relocation only.** The registrations were extracted mechanically and the
//! schema baseline verifies the result is byte-identical — a retyped call is
//! exactly the mistake that would slip through review and not through the
//! baseline.
//!
//! ⚠ the owner label stays `ambition_platformer2d_runtime` because this module is in it, and
//! must be: `ambition_items` sits below the runtime in the crate graph. R1's
//! recorded decision is that this is the right shape for every domain below the
//! runtime; crates above it own their schemas directly.

use bevy::prelude::App;

use super::super::AmbitionRollbackApp;

const OWNER: &str = "ambition_platformer2d_runtime";

/// Register everything the items domain needs rewound.
pub(in crate::rollback) fn register(app: &mut App) {
    app.rollback_resource_clone::<ambition_items::OwnedItems>(OWNER, "resource.owned_items");
}
