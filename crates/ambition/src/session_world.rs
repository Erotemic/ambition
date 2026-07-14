//! Canonical live session-world surface.
//!
//! One exact [`SessionRoot`] entity owns the live world components for a
//! gameplay activation. Consumers read and mutate those components directly
//! through [`SessionWorldRef`] and [`SessionWorldMut`]. At frontend routes the
//! root does not exist, so gameplay-world access is structurally unavailable.
//! No process-resident projection or synchronization bridge exists.

pub use ambition_platformer_primitives::lifecycle::{
    session_world_exists, SessionRoot, SessionWorldMut, SessionWorldRef,
};
pub use ambition_runtime::{
    PlatformerSessionCatalogs, PlatformerSessionRequests, PlatformerSessionWorld,
};
