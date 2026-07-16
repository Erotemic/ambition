//! Projectile visual identity — an **open, content-owned** art registry.
//!
//! This is the single art-selection seam shared by every projectile, player and
//! enemy alike. A projectile entity carries a [`ProjectileVisualId`] component (an
//! open string set at spawn); the renderer resolves it through the
//! [`ProjectileVisualCatalog`] to the data [`ProjectileArt`] descriptor and draws
//! the generic `source` / `size` / `rotation` axes it reports. The renderer never
//! matches on a named identity — so adding a new projectile look is *one
//! registration in a content crate* with no render-system edit (the
//! engine-for-other-games test).
//!
//! The reusable crate owns the generic art VOCABULARY and an **empty-by-default**
//! catalog; it names no projectile. A game registers each named look
//! (`"fireball"`, `"apple"`, `"lasersword"`, …) from its content crate via
//! [`ProjectileVisualAppExt::register_projectile_visual`]. An unregistered id
//! resolves to the engine's generic hostile shot ([`ProjectileArt::generic`]) so
//! the model still draws something before any content registers — the same
//! "generic fallback stays in-engine" shape as the dialogue voice catalog.

use std::collections::BTreeMap;

use bevy::prelude::{App, Component, Resource};

/// Open visual identity carried by every projectile entity (player + enemy),
/// stamped at the fire site. Resolved to a [`ProjectileArt`] through the
/// [`ProjectileVisualCatalog`]. An empty id (or any unregistered id) reads as the
/// generic hostile shot.
#[derive(Component, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct ProjectileVisualId(pub String);

impl ProjectileVisualId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// How to draw the pixels for a projectile. Generic render primitives — the
/// renderer matches on THESE, not on any named identity.
#[derive(Clone, Debug, PartialEq)]
pub enum ProjectileArtSource {
    /// The shared "energy ball" texture, tinted `rgba`. Falls back to a flat
    /// colored quad of the same tint when the texture is missing. (Player kit.)
    EnergyTinted { rgba: [f32; 4] },
    /// A flat colored quad — no texture.
    SolidColor { rgba: [f32; 4] },
    /// A single standalone image at `path` (relative to the asset root, e.g.
    /// `"sprites/.../foo.png"`). Not a spritesheet.
    Image { path: String },
    /// One row of a registered spritesheet (looked up by `target` in the
    /// `SheetRegistry`; the image path comes from the sheet's own manifest).
    /// `animate` cycles the row's frames on the row's authored cadence;
    /// otherwise the renderer clips the first frame statically.
    Sheet {
        target: String,
        animation: String,
        animate: bool,
    },
}

/// How big to render the projectile sprite.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ProjectileRenderSize {
    /// Scale to the projectile body hitbox (each axis clamped to `min` px),
    /// times `scale`.
    Body { min: f32, scale: f32 },
    /// Fixed render width in px; height follows the source frame's aspect.
    FixedWidth(f32),
}

/// How the sprite is oriented each frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProjectileRotation {
    /// Mirror horizontally to face travel direction (energy balls, rocks).
    FlipToTravel,
    /// Rotate to align the sprite with its velocity vector (thrown blade).
    /// A `"pommel"` anchor in the sheet frame, if present, becomes the pivot.
    VelocityAligned,
    /// Stay upright relative to local gravity (apple, glider) — identity under
    /// normal gravity, rotates under sideways / inverted gravity.
    GravityUpright,
}

/// Position-free description of the VFX a projectile emits when it expires on a
/// solid / times out (e.g. the pirate lasersword detonation). The sim stepper
/// stamps the live position at expiry time via [`Self::to_message`]. `None` on a
/// kind's art means the stepper's generic impact fallback is used.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ProjectileExpiryBurst {
    pub kind: ambition_vfx::vfx::ExplosionKind,
    pub scale: f32,
}

impl ProjectileExpiryBurst {
    pub fn to_message(self, pos: ambition_engine_core::Vec2) -> ambition_vfx::vfx::VfxMessage {
        ambition_vfx::vfx::VfxMessage::Explosion {
            pos,
            kind: self.kind,
            scale: self.scale,
        }
    }
}

/// The complete authored description of a projectile look — the SINGLE source of
/// truth per registered id. Everything the presentation + sim layers need (live
/// art + debug placeholder color + trace label + custom expiry FX) lives in one
/// record, built from orthogonal, reusable axes: a `source` (how to get pixels) ×
/// `size` × `rotation`. The renderer matches on each AXIS independently, so a new
/// look that reuses existing axis values needs zero render-system changes.
#[derive(Clone, Debug, PartialEq)]
pub struct ProjectileArt {
    pub source: ProjectileArtSource,
    pub size: ProjectileRenderSize,
    pub rotation: ProjectileRotation,
    /// Representative flat color for the debug placeholder overlay (keyed by the
    /// visual id, frame-agnostic — never by who fired the shot).
    pub debug_tint: [f32; 4],
    /// Short stable identifier for traces / debug entity names.
    pub label: String,
    /// Custom VFX emitted when this projectile expires. `None` uses the sim
    /// stepper's generic impact fallback.
    pub expiry_vfx: Option<ProjectileExpiryBurst>,
}

impl ProjectileArt {
    /// The engine's generic hostile-shot look — an orange-red quad. Used for any
    /// projectile whose visual id is unregistered (the content-free fallback: the
    /// reusable crate never names a projectile, but the model still draws
    /// something and stays playable before any content registers).
    pub fn generic() -> Self {
        Self {
            source: ProjectileArtSource::SolidColor {
                rgba: [1.0, 0.45, 0.18, 0.95],
            },
            size: ProjectileRenderSize::Body {
                min: 8.0,
                scale: 1.0,
            },
            rotation: ProjectileRotation::FlipToTravel,
            debug_tint: [1.0, 0.45, 0.18, 1.0],
            label: "projectile".to_string(),
            expiry_vfx: None,
        }
    }
}

/// Open, content-populated registry: projectile visual id → authored art.
///
/// Empty by default; a game's content crate fills it via
/// [`ProjectileVisualAppExt::register_projectile_visual`]. Render + sim resolve a
/// [`ProjectileVisualId`] through [`Self::resolve`], which falls back to
/// [`ProjectileArt::generic`] for unregistered ids.
#[derive(Resource, Clone, Debug, Default)]
pub struct ProjectileVisualCatalog {
    arts: BTreeMap<String, ProjectileArt>,
}

impl ProjectileVisualCatalog {
    /// The registered art for `id`, or `None` if unregistered.
    pub fn get(&self, id: &str) -> Option<&ProjectileArt> {
        self.arts.get(id)
    }

    /// Resolve `id` → art, falling back to the generic hostile shot when the id
    /// is unregistered (or empty). Returns an owned art so callers needn't hold
    /// the resource borrow across the frame.
    pub fn resolve(&self, id: &str) -> ProjectileArt {
        self.arts
            .get(id)
            .cloned()
            .unwrap_or_else(ProjectileArt::generic)
    }

    /// Register a named look. Idempotent for identical (id, art); panics on a
    /// conflicting re-registration so a content authoring mistake is loud at
    /// startup, matching the dialogue/wielded-item catalogs.
    fn register(&mut self, id: impl Into<String>, art: ProjectileArt) {
        let id = id.into();
        match self.arts.get(&id) {
            Some(existing) if *existing == art => {}
            Some(_) => panic!(
                "projectile visual id {id:?} already registered with a different art descriptor"
            ),
            None => {
                self.arts.insert(id, art);
            }
        }
    }
}

/// Composition-time sugar for registering a named projectile look from a content
/// crate, mirroring `register_dialogue_voiceprint` / `register_wielded_item_visual`.
pub trait ProjectileVisualAppExt {
    fn register_projectile_visual(
        &mut self,
        id: impl Into<String>,
        art: ProjectileArt,
    ) -> &mut Self;
}

impl ProjectileVisualAppExt for App {
    fn register_projectile_visual(
        &mut self,
        id: impl Into<String>,
        art: ProjectileArt,
    ) -> &mut Self {
        self.init_resource::<ProjectileVisualCatalog>();
        self.world_mut()
            .resource_mut::<ProjectileVisualCatalog>()
            .register(id, art);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_art(label: &str) -> ProjectileArt {
        ProjectileArt {
            source: ProjectileArtSource::SolidColor {
                rgba: [0.1, 0.2, 0.3, 1.0],
            },
            size: ProjectileRenderSize::FixedWidth(20.0),
            rotation: ProjectileRotation::GravityUpright,
            debug_tint: [0.1, 0.2, 0.3, 1.0],
            label: label.to_string(),
            expiry_vfx: None,
        }
    }

    #[test]
    fn unregistered_id_resolves_to_the_generic_shot() {
        let catalog = ProjectileVisualCatalog::default();
        assert!(catalog.get("nope").is_none());
        assert_eq!(catalog.resolve("nope"), ProjectileArt::generic());
        // Empty id (the enemy default channel) also reads as generic.
        assert_eq!(catalog.resolve(""), ProjectileArt::generic());
    }

    #[test]
    fn registered_id_resolves_to_its_art() {
        let mut catalog = ProjectileVisualCatalog::default();
        catalog.register("apple", sample_art("apple"));
        assert_eq!(catalog.resolve("apple"), sample_art("apple"));
    }

    #[test]
    fn identical_reregistration_is_idempotent() {
        let mut catalog = ProjectileVisualCatalog::default();
        catalog.register("apple", sample_art("apple"));
        catalog.register("apple", sample_art("apple"));
        assert_eq!(catalog.resolve("apple"), sample_art("apple"));
    }

    #[test]
    #[should_panic(expected = "already registered")]
    fn conflicting_reregistration_panics() {
        let mut catalog = ProjectileVisualCatalog::default();
        catalog.register("apple", sample_art("apple"));
        catalog.register("apple", sample_art("different"));
    }
}
