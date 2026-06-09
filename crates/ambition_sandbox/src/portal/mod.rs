//! Portal mechanic facade.
//!
//! The portal **mechanic** (the portal gun place/replace/channel, the one
//! generic aperture transit over `PortalBody` + `PortalPolicy`, placement +
//! transit math, carve publishing, pieces geometry, lifecycle, the pure shot
//! helper over `SolidWorldQuery`, the portal events + `PortalPlugin`) now lives
//! in the standalone, content-free crate
//! [`ambition_portal`](https://docs.rs/ambition_portal) (Stage 19; ADR 0019).
//! This module is a thin FACADE that re-exports the whole crate so every inbound
//! `crate::portal::…` path keeps resolving with zero churn, and it keeps the two
//! things that are NOT the mechanic on top:
//!
//! - **`presentation`** (render-gated) — portal quads + labels, the gun sprite,
//!   the body-piece decomposition mid-transit, the disorientation indicator, and
//!   the `F7` dev off-switch. Rendering is a HOST concern, so it stays in the
//!   sandbox behind the `portal_render` feature; the portal *simulation* builds
//!   without it.
//! - **`tests`** — the integration tests that drive portal core THROUGH the
//!   Ambition adapters (input / inventory / carve bridge / world seam), so they
//!   necessarily reference sandbox types and stay sandbox-side.
//!
//! The Ambition adapters that bridge the crate's seams to game concepts (input →
//! fire intent, carve → collision overlay, room-reset → clear, sfx, player input
//! / ability shaping, identity → policy tagging) live in
//! [`crate::ambition_content::portal`].

// The whole reusable mechanic, surfaced at the historic `crate::portal::…` paths.
pub use ambition_portal::*;

/// Portal presentation (sprites, rings, body pieces, disorientation FX, the dev
/// toggle). Render only — compiled and re-exported exclusively behind the
/// `portal_render` feature so the portal *simulation* (the `ambition_portal`
/// crate) builds without any render-facing systems or components.
#[cfg(feature = "portal_render")]
mod presentation;

#[cfg(test)]
mod tests;

#[cfg(feature = "portal_render")]
pub use presentation::{
    load_portal_gun_art, portal_dev_toggle_system, sync_portal_body_pieces,
    sync_portal_disorientation_indicator, sync_portal_mode_indicator, sync_portal_visuals,
    PortalAimHint, PortalBodyPiece, PortalDisorientIndicator, PortalGunArt, PortalModeIndicator,
    PortalVisual,
};
