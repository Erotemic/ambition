//! The message the movement kernel writes for causal instrumentation. Its
//! recorders are `ambition_platformer2d_runtime::causal`'s.

/// Movement operations emitted by the kernel for optional causal instrumentation.
///
/// The core movement crate does not depend on `ambition_causal`, so the host forwards
/// `FrameEvents::operations` through this message. Compositions without a causal
/// recorder may omit the writer entirely.
#[cfg(feature = "causal")]
#[derive(bevy::prelude::Message, Clone, Debug)]
pub struct BodyMovementOps {
    pub body: bevy::prelude::Entity,
    pub ops: Vec<ambition_platformer2d_core::MovementOp>,
}
