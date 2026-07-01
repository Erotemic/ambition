//! Publish/install boundary for generated sprite assets.
//!
//! Generators (the `ambition_sprite2d_renderer` toolchain and the quality-
//! variant scripts) emit many kinds of file: runtime sheet records and page
//! images, transitional actor sidecars, throwaway YAML/JSON intermediates, and
//! human-only diagnostics (canonical poses, labeled previews, debug overlays).
//! Only a subset belongs under the runtime asset roots the game loads.
//!
//! This module is the first slice of
//! `docs/planning/engine/data-driven-sprites-and-characters.md`: it makes the
//! install boundary explicit without rewriting any runtime consumer.
//!
//! - [`classify`] — decide what a generated file *is* from its path shape.
//! - [`manifest`] — the [`PublishManifest`], the record of what got installed.
//! - [`publish`] — the small install step: staging in, runtime artifacts out.
//! - [`hygiene`] — scan a runtime root for diagnostics that leaked in.
//!
//! The [`hygiene`] scan is the piece with immediate teeth: a real-data test
//! (see `tests`) fails if a `*_canonical.png`, `*_preview_labeled.png`, or
//! `*_debug.png` ever reappears under a runtime sprite root.

pub mod classify;
pub mod hygiene;
pub mod manifest;
pub mod publish;
mod walk;

pub use classify::{classify, ArtifactClass};
pub use hygiene::{scan_runtime_root, scan_runtime_roots, HygieneFinding, HygieneReport};
pub use manifest::{DiagnosticEntry, InstalledEntry, ManifestError, PublishManifest, Quality};
pub use publish::{install, PublishOptions};

/// The runtime sprite roots this crate ships, relative to the crate manifest
/// dir. The base `sprites/` root is tracked; the scaled variants are gitignored
/// and only present after a quality-variant regen, so a hygiene scan of a
/// missing variant is simply clean.
pub const RUNTIME_SPRITE_ROOTS: &[&str] = &[
    "assets/sprites",
    "assets/sprites_0_5x",
    "assets/sprites_0_25x",
    "assets/sprites_potato",
];

#[cfg(test)]
mod tests;
