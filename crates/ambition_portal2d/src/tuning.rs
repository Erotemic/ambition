//! Runtime-tunable portal feel and convention policy.
//!
//! The portal map convention is a field of `PortalTuning`, so dev tools edit
//! it as ordinary Bevy state. Consumers pass `map_convention()` to the pure
//! helpers, which also work without Bevy.

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

    /// The geometry layer's convention this names. Consumers take it as a
    /// parameter; do not copy it into a global. A global cannot differ between
    /// two sessions in one process and is not rollback state.
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
    /// (the facing-flip write in [`portal_transit`](crate::portal_transit)).
    /// It is ANDed with whether the body reorients (a body in the player
    /// population), so it can only suppress the flip. On by default; a host can
    /// mirror its own gameplay setting into it.
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


/// The inspector's mirror of [`PortalTuning`].
///
/// The sim reads `PortalTuning` inside the rollback window (`GgrsSchedule`),
/// so the inspector panel must not write it directly (`Q120`). The panel edits
/// this mirror; the edit is proposed, and `PortalTuning` changes only after
/// the rollback timeline's owner admits it. Same pattern as
/// `EditableMovementTuning` → `ActiveMovementTuning`.
///
/// `Deref`/`DerefMut` let the panel keep its `&mut tuning.field` code.
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

/// This domain's key in `PendingMechanicalEdits`. The type is the identity,
/// not the label; see `MechanicalDomain`.
pub struct PortalTuningDomain;

pub fn portal_tuning_domain() -> ambition_platformer2d_core::MechanicalDomain {
    ambition_platformer2d_core::MechanicalDomain::of::<PortalTuningDomain>("portal_tuning")
}

/// Raise a changed portal mechanic as a proposal.
///
/// `is_added` is excluded: Bevy counts insertion as a change, and a proposal
/// on the first frame would stop the session that just started.
pub fn propose_editable_portal_tuning(
    editable: bevy::prelude::Res<EditablePortalTuning>,
    mut pending: bevy::prelude::ResMut<ambition_platformer2d_core::PendingMechanicalEdits>,
) {
    if !editable.is_changed() || editable.is_added() {
        return;
    }
    pending.propose(portal_tuning_domain());
}

/// Copy an admitted portal-mechanic edit into the value the simulation reads.
///
/// Not change-guarded: the guard is on the proposal. An `is_changed` guard
/// here would drop edits that were staged for a frame.
pub fn publish_editable_portal_tuning(
    editable: bevy::prelude::Res<EditablePortalTuning>,
    admission: Option<bevy::prelude::Res<ambition_platformer2d_core::MechanicalEditAdmission>>,
    mut pending: bevy::prelude::ResMut<ambition_platformer2d_core::PendingMechanicalEdits>,
    mut active: bevy::prelude::ResMut<PortalTuning>,
) {
    if !pending.is_pending(portal_tuning_domain()) {
        return;
    }
    // Absent means publish: without a rollback host there is no history to
    // contradict.
    if matches!(
        admission.as_deref(),
        Some(ambition_platformer2d_core::MechanicalEditAdmission::Refuse)
    ) {
        return;
    }
    *active = editable.0;
    pending.take(portal_tuning_domain());
}
