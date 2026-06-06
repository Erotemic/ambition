//! Convenience imports for proto-runtime call sites.
//!
//! This re-exports the extracted crate's content-free prelude (lifecycle +
//! schedule vocabulary) plus the sandbox-local `raycast_solids` seam, so the
//! pre-extraction `crate::platformer_runtime::prelude::*` API is unchanged.

pub use super::collision::raycast_solids;
pub use ambition_platformer_runtime::prelude::*;
