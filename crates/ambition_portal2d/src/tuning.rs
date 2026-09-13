//! Runtime-tunable portal feel and convention policy.
//!
//! The portal map convention is intentionally a resource-facing enum here, even
//! though the pure math layer stores the live convention in a tiny global. That
//! lets dev tools edit it as ordinary Bevy state while the pure helpers stay
//! usable from tests and non-Bevy callers.

use bevy::prelude::*;

use crate::types::{MIN_EXIT_SPEED, TELEPORT_COOLDOWN_S};

/// Which isometry glues a portal pair together.
#[derive(Clone, Copy, Debug, Default, Reflect, PartialEq, Eq)]
pub enum PortalConvention {
    /// Historical det -1 portal map: tangents are preserved and normals flip.
    #[default]
    Reflection,
    /// Proper det +1 portal map: the entry-facing chart rotates into the exit.
    Rotation,
}

impl PortalConvention {
    /// The boolean convention used by the pure geometry layer.
    pub const fn is_rotation(self) -> bool {
        matches!(self, Self::Rotation)
    }

    /// The geometry layer's convention this names.
    ///
    /// ⭐ ONE VALUE, THREADED. It used to be pushed into a process-global
    /// `AtomicBool` by a system that ran once per change; every consumer then
    /// read the global instead of the tuning that owns it. Two providers in one
    /// process could not disagree, and a static is not rollback state, so a
    /// convention the inspector changed mid-session did not rewind with the
    /// world.
    pub const fn map_convention(self) -> ambition_platformer2d_core::frame::MapConvention {
        match self {
            Self::Reflection => ambition_platformer2d_core::frame::MapConvention::Reflection,
            Self::Rotation => ambition_platformer2d_core::frame::MapConvention::Rotation,
        }
    }

    /// Convert from the pure geometry layer's boolean convention.
    pub const fn from_rotation(rotation: bool) -> Self {
        if rotation {
            Self::Rotation
        } else {
            Self::Reflection
        }
    }
}

/// Portal mechanics and adapter feel knobs surfaced in the F3 dev inspector.
#[derive(Resource, Clone, Copy, Debug, Reflect)]
#[reflect(Resource)]
pub struct PortalTuning {
    /// Active portal map convention. Exposed as a combo box by the inspector.
    pub convention: PortalConvention,
    /// Budget for portal-aware logic raycasts. Current production fire traces do
    /// not recurse yet, but tests/tools can use this instead of hard-coding.
    pub raycast_recursion_depth: u32,
    /// Minimum exit speed along the exit normal after a body transfers.
    pub min_exit_speed: f32,
    /// Per-body anti-ping-pong latch after a transfer.
    pub teleport_cooldown_s: f32,
    /// Duration of the input guard that prevents immediate pushback into the
    /// exit wall.
    pub emission_time_s: f32,
    /// Stick/axis magnitude above which movement counts as held.
    pub input_held_epsilon: f32,
    /// Cosine threshold before a changed held direction drops the input warp.
    pub input_warp_keep_cos: f32,
    /// While an actor is in a portal aperture, disable wall movement abilities
    /// so carved aperture edges cannot catch them.
    pub suppress_wall_abilities: bool,
    /// Whether a same-wall turn-around transit re-orients the body's `facing`
    /// (the `policy.reorient && facing_flip` write in [`transit`](crate::transit)).
    /// This is a global gate ANDed with the per-body [`PortalPolicy`]'s `reorient`
    /// flag, so it only ever suppresses the flip — bodies whose policy already
    /// keeps facing (bosses, projectiles) are unaffected either way. The portal
    /// crate defaults it ON to preserve standalone behavior; a host can mirror
    /// its own gameplay setting into it.
    ///
    /// [`PortalPolicy`]: crate::transit::PortalPolicy
    pub reorient_facing: bool,
}

impl Default for PortalTuning {
    fn default() -> Self {
        Self {
            convention: PortalConvention::Rotation,
            raycast_recursion_depth: 4,
            min_exit_speed: MIN_EXIT_SPEED,
            teleport_cooldown_s: TELEPORT_COOLDOWN_S,
            emission_time_s: 0.18,
            input_held_epsilon: 0.25,
            input_warp_keep_cos: 0.5,
            suppress_wall_abilities: true,
            reorient_facing: true,
        }
    }
}

// ⛔⛔ `sync_portal_tuning_convention` WAS HERE, AND IT WAS THE WHOLE DEFECT. It
// mirrored `PortalTuning::convention` into a process-global `AtomicBool` once
// per frame so that every consumer could read the global instead of the tuning
// that owns it. A resource copied into a static is a resource with two
// authorities: two providers in one process could not disagree, load order
// decided who won, and a static is not rollback state — so a convention the
// inspector changed mid-session did not rewind with the world. Consumers take
// `PortalTuning::map_convention()` as a parameter now, and there is nothing to
// mirror.

/// The inspector's mirror of [`PortalTuning`].
///
/// ⛔⛤ **`Q120`, 2026-09-13: THE F-KEY PANEL USED TO WRITE THE AUTHORITATIVE
/// VALUE DIRECTLY, AND THE SIMULATION READS IT INSIDE THE ROLLBACK WINDOW.**
/// `game/ambition_app/src/dev/portal_inspector.rs` did
/// `world.get_resource_mut::<PortalTuning>()` from an egui pass, while
/// `transit.rs`'s portal systems take `Res<PortalTuning>` in the sim schedule —
/// which under the rollback host is `GgrsSchedule`. ⇒ *"What value will a replay
/// of frame N observe?"* was answered *"whatever the panel holds now"*, and the
/// rollback waiver that called this *"forward-only"* is the category `Q119`
/// already ruled is not one.
///
/// ⭐ **SAME SHAPE AS `EditableMovementTuning` → `ActiveMovementTuning`**, and
/// deliberately so: the panel edits a mirror, the mirror is PROPOSED, and the
/// authoritative value moves only once the rollback timeline's owner has
/// admitted it. Every existing reader — two simulation systems and five
/// presentation ones — keeps reading `PortalTuning` and is untouched.
///
/// ⚠ `Deref`/`DerefMut`, so the panel's several hundred `&mut tuning.field` rows
/// did not have to change. The only edit at the call site is which resource it
/// asks for, which is exactly the amount of change this repair should cost.
#[derive(
    bevy::prelude::Resource,
    Clone,
    Copy,
    Debug,
    Default,
    bevy::prelude::Deref,
    bevy::prelude::DerefMut,
)]
pub struct EditablePortalTuning(pub PortalTuning);

/// This domain's key in `PendingMechanicalEdits`, declared beside the value.
pub const PORTAL_TUNING: ambition_platformer2d_core::MechanicalDomain =
    ambition_platformer2d_core::MechanicalDomain("portal_tuning");

/// Raise a changed portal mechanic as a PROPOSAL.
///
/// ⚠ **`is_added` IS EXCLUDED** for the reason every proposer excludes it: Bevy
/// counts INSERTION as a change, and the mirror is installed before content
/// finishes seeding — proposing that would stop the session the composition had
/// just started, on frame one, every time.
pub fn propose_editable_portal_tuning(
    editable: bevy::prelude::Res<EditablePortalTuning>,
    mut pending: bevy::prelude::ResMut<ambition_platformer2d_core::PendingMechanicalEdits>,
) {
    if !editable.is_changed() || editable.is_added() {
        return;
    }
    pending.propose(PORTAL_TUNING);
}

/// Copy an ADMITTED portal-mechanic edit into the value the simulation reads.
///
/// ⛔ Deliberately NOT change-guarded: the guard lives on the PROPOSAL, so an
/// untouched panel raises nothing and this never runs its write. Re-adding
/// `is_changed` here would drop every edit that had to be staged behind a
/// foreign rollback timeline for a frame, which is the whole point of staging it.
pub fn publish_editable_portal_tuning(
    editable: bevy::prelude::Res<EditablePortalTuning>,
    admission: Option<bevy::prelude::Res<ambition_platformer2d_core::MechanicalEditAdmission>>,
    mut pending: bevy::prelude::ResMut<ambition_platformer2d_core::PendingMechanicalEdits>,
    mut active: bevy::prelude::ResMut<PortalTuning>,
) {
    if !pending.is_pending(PORTAL_TUNING) {
        return;
    }
    // ⛔ ABSENT ⇒ PUBLISH, matching the resource's own default: a composition
    // with no rollback host has no history an edit could contradict.
    if matches!(
        admission.as_deref(),
        Some(ambition_platformer2d_core::MechanicalEditAdmission::Refuse)
    ) {
        return;
    }
    *active = editable.0;
    pending.take(PORTAL_TUNING);
}
