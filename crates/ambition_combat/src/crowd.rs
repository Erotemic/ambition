//! Crowding classification used by fighter spacing logic.
//!
//! Ground and aerial bodies contest space within their own crowd class.

/// Which crowd class contributes to this body's spacing pressure.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CrowdKind {
    /// Fights from the ground.
    Ground,
    /// Fights from the air, and spaces itself against other flyers.
    Aerial,
}
