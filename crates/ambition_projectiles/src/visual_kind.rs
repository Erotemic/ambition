//! The **visual identity** of a projectile — a content-owned named kind plus its
//! authored art descriptor.
//!
//! This is the single art-selection seam shared by every projectile, player and
//! enemy alike. A projectile entity carries a [`ProjectileVisualKind`] component
//! (set at spawn); the renderer reads only the data [`ProjectileArt`] descriptor
//! that [`ProjectileVisualKind::art`] returns. The renderer never matches on the
//! named kind — it consumes the generic `source` / `size` / `rotation` the
//! descriptor reports, so adding a new kind that reuses existing render
//! capabilities is *one data arm here* with no render-system edit (the
//! engine-for-other-games test).
//!
//! Why this lives here and not on the foundation spec
//! (`ambition_platformer_primitives::projectile::ProjectileSpec`): that spec is
//! deliberately content-free ("carries no named projectile vocabulary"). Named
//! art is content, so the identity travels as a content-layer component —
//! mirroring how the player side already carries [`ProjectileKind`]. `owner_id`
//! stays for self/friendly-fire filtering and traces ONLY; it is never read for
//! art.

use bevy::prelude::Component;
use serde::{Deserialize, Serialize};

use super::ProjectileKind;

/// What a projectile looks like. Carried as an ECS component by every projectile
/// (player + enemy); the firing site stamps it at spawn.
///
/// Player kinds map 1:1 from [`ProjectileKind`] (gameplay tier) via [`From`];
/// enemy kinds are authored in archetype / item / boss-special data. New kinds
/// are added here + a single arm in [`ProjectileVisualKind::art`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Component)]
pub enum ProjectileVisualKind {
    /// Player fireball — warm energy ball, mild arc, flips to travel.
    Fireball,
    /// Player Hadouken — cool blue energy ball.
    Hadouken,
    /// Player super Hadouken — stronger blue tint.
    HadoukenSuper,
    /// GNU-ton's apple-rain fruit — generated apple sprite, gravity-upright.
    Apple,
    /// Pirate gun-sword discharge — spinning blade aligned to its velocity.
    Lasersword,
    /// The Perfect Cell-ular Automaton's zoning shot — an animated Conway glider.
    Glider,
    /// Generic hostile shot — orange-red quad. Default for any enemy projectile
    /// whose firing data does not author a distinct look.
    #[default]
    EnemyDefault,
}

/// How to draw the pixels for a [`ProjectileVisualKind`]. Generic render
/// primitives — the renderer matches on THESE, not on the named kind.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ProjectileArtSource {
    /// The shared "energy ball" texture, tinted `rgba`. Falls back to a flat
    /// colored quad of the same tint when the texture is missing. (Player kit.)
    EnergyTinted { rgba: [f32; 4] },
    /// A flat colored quad — no texture.
    SolidColor { rgba: [f32; 4] },
    /// A single standalone image at `path` (relative to the asset root, e.g.
    /// `"sprites/.../foo.png"`). Not a spritesheet.
    Image { path: &'static str },
    /// One row of a registered spritesheet (looked up by `target` in the
    /// `SheetRegistry`; the image path comes from the sheet's own manifest).
    /// `animate` cycles the row's frames on the row's authored cadence;
    /// otherwise the renderer clips the first frame statically.
    Sheet {
        target: &'static str,
        animation: &'static str,
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

/// The complete authored description of a projectile kind's appearance — the
/// SINGLE source of truth per kind. Everything the presentation layer needs
/// (live art + debug placeholder color + trace label) lives in one record, so
/// adding a kind is one entry here, not edits scattered across several methods.
///
/// It is built from orthogonal, reusable axes: a `source` (how to get pixels) ×
/// `size` × `rotation`. The renderer matches on each AXIS independently, so a
/// new kind that reuses existing axis values needs zero render-system changes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ProjectileArt {
    pub source: ProjectileArtSource,
    pub size: ProjectileRenderSize,
    pub rotation: ProjectileRotation,
    /// Representative flat color for the debug placeholder overlay (keyed by
    /// KIND, frame-agnostic — never by who fired the shot).
    pub debug_tint: [f32; 4],
    /// Short stable identifier for traces / debug entity names.
    pub label: &'static str,
}

impl ProjectileVisualKind {
    /// The complete art descriptor for this kind — the entire kind→appearance
    /// registry, one data record per kind, content-owned. The renderer consumes
    /// the generic `source` / `size` / `rotation` axes; `debug_tint` / `label`
    /// serve the overlay + traces. This is the ONLY place a kind's look is
    /// defined; `label()` / `debug_tint()` just read fields off it.
    pub fn art(self) -> ProjectileArt {
        use ProjectileArtSource::{EnergyTinted, Image, Sheet, SolidColor};
        use ProjectileRenderSize::{Body, FixedWidth};
        use ProjectileRotation::{FlipToTravel, GravityUpright, VelocityAligned};
        match self {
            // Player kit: the shared energy ball, tinted per tier. Warm orange
            // fireball; cool blue Hadouken; stronger blue super.
            Self::Fireball => ProjectileArt {
                source: EnergyTinted {
                    rgba: [1.0, 0.74, 0.30, 0.95],
                },
                size: Body {
                    min: 8.0,
                    scale: 1.0,
                },
                rotation: FlipToTravel,
                debug_tint: [1.0, 0.74, 0.30, 1.0],
                label: "fireball",
            },
            Self::Hadouken => ProjectileArt {
                source: EnergyTinted {
                    rgba: [0.45, 0.78, 1.0, 0.96],
                },
                size: Body {
                    min: 8.0,
                    scale: 1.0,
                },
                rotation: FlipToTravel,
                debug_tint: [0.45, 0.78, 1.0, 1.0],
                label: "hadouken",
            },
            Self::HadoukenSuper => ProjectileArt {
                source: EnergyTinted {
                    rgba: [0.30, 0.55, 1.0, 1.0],
                },
                size: Body {
                    min: 8.0,
                    scale: 1.0,
                },
                rotation: FlipToTravel,
                debug_tint: [0.30, 0.55, 1.0, 1.0],
                label: "hadouken_super",
            },
            // GNU-ton apple rain: generated apple, a touch over the body box,
            // upright relative to local gravity.
            Self::Apple => ProjectileArt {
                source: Image {
                    path: "sprites/gnu_ton_boss/gnu_ton_apple.png",
                },
                size: Body {
                    min: 8.0,
                    scale: 1.12,
                },
                rotation: GravityUpright,
                debug_tint: [0.90, 0.20, 0.20, 1.0],
                label: "apple",
            },
            // Pirate gun-sword: the first idle frame of the lasersword sheet,
            // rotated along the velocity (pommel pivot read from the manifest).
            Self::Lasersword => ProjectileArt {
                source: Sheet {
                    target: "lasersword",
                    animation: "idle",
                    animate: false,
                },
                size: FixedWidth(56.0),
                rotation: VelocityAligned,
                debug_tint: [0.45, 1.0, 0.85, 1.0],
                label: "lasersword",
            },
            // PCA zoning shot: the animated Conway glider, upright vs gravity,
            // sized for arena readability rather than to the small hitbox.
            Self::Glider => ProjectileArt {
                source: Sheet {
                    target: "glider",
                    animation: "fly",
                    animate: true,
                },
                size: FixedWidth(38.0),
                rotation: GravityUpright,
                debug_tint: [0.40, 0.95, 0.45, 1.0],
                label: "glider",
            },
            // Generic hostile shot: orange-red quad, readable against the sky and
            // distinct from the warm-yellow player fireball.
            Self::EnemyDefault => ProjectileArt {
                source: SolidColor {
                    rgba: [1.0, 0.45, 0.18, 0.95],
                },
                size: Body {
                    min: 8.0,
                    scale: 1.0,
                },
                rotation: FlipToTravel,
                debug_tint: [1.0, 0.45, 0.18, 1.0],
                label: "enemy_default",
            },
        }
    }

    /// Encode this kind as the opaque `visual_tag` carried on the foundation
    /// [`EnemyProjectileSpawn`](ambition_platformer_primitives::projectile::EnemyProjectileSpawn).
    /// The foundation never interprets the tag (like `charge_tier`); only the
    /// content layer round-trips it through [`Self::from_tag`]. `EnemyDefault`
    /// maps to `0` so an unset / zero tag reads as the generic hostile shot.
    pub fn to_tag(self) -> u16 {
        match self {
            Self::EnemyDefault => 0,
            Self::Fireball => 1,
            Self::Hadouken => 2,
            Self::HadoukenSuper => 3,
            Self::Apple => 4,
            Self::Lasersword => 5,
            Self::Glider => 6,
        }
    }

    /// Decode a `visual_tag` (see [`Self::to_tag`]). Unknown / zero tags fall
    /// back to [`Self::EnemyDefault`].
    pub fn from_tag(tag: u16) -> Self {
        match tag {
            1 => Self::Fireball,
            2 => Self::Hadouken,
            3 => Self::HadoukenSuper,
            4 => Self::Apple,
            5 => Self::Lasersword,
            6 => Self::Glider,
            _ => Self::EnemyDefault,
        }
    }

    /// Representative flat color for the debug placeholder overlay (keyed by
    /// KIND, frame-agnostic — never by who fired the shot). Reads the single
    /// [`Self::art`] descriptor.
    pub fn debug_tint(self) -> [f32; 4] {
        self.art().debug_tint
    }

    /// A short stable label for traces / debug names. Reads the single
    /// [`Self::art`] descriptor.
    pub fn label(self) -> &'static str {
        self.art().label
    }

    /// VFX emitted when this projectile expires because it timed out or hit a
    /// solid. Most projectiles use the generic impact supplied by the sim
    /// stepper; special projectile visual identities own their special death
    /// presentation here, next to the rest of the projectile-kind art policy.
    ///
    /// This remains replay-neutral presentation vocabulary: the gameplay stepper
    /// decides *when* a projectile expires, then asks the visual kind whether that
    /// expiry has a custom VFX cue.
    pub fn expiry_vfx(
        self,
        pos: ambition_engine_core::Vec2,
    ) -> Option<ambition_vfx::vfx::VfxMessage> {
        (self == Self::Lasersword).then_some(ambition_vfx::vfx::VfxMessage::Explosion {
            pos,
            kind: ambition_vfx::vfx::ExplosionKind::ClassicBurst,
            scale: 0.7,
        })
    }
}

impl From<ProjectileKind> for ProjectileVisualKind {
    /// Player gameplay tier → its visual identity (1:1).
    fn from(kind: ProjectileKind) -> Self {
        match kind {
            ProjectileKind::Fireball => Self::Fireball,
            ProjectileKind::Hadouken => Self::Hadouken,
            ProjectileKind::HadoukenSuper => Self::HadoukenSuper,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn player_kind_maps_to_matching_visual_kind() {
        assert_eq!(
            ProjectileVisualKind::from(ProjectileKind::Fireball),
            ProjectileVisualKind::Fireball
        );
        assert_eq!(
            ProjectileVisualKind::from(ProjectileKind::Hadouken),
            ProjectileVisualKind::Hadouken
        );
        assert_eq!(
            ProjectileVisualKind::from(ProjectileKind::HadoukenSuper),
            ProjectileVisualKind::HadoukenSuper
        );
    }

    #[test]
    fn enemy_default_is_the_default() {
        assert_eq!(
            ProjectileVisualKind::default(),
            ProjectileVisualKind::EnemyDefault
        );
    }

    #[test]
    fn glider_is_an_animated_sheet_kept_upright() {
        let art = ProjectileVisualKind::Glider.art();
        assert_eq!(art.rotation, ProjectileRotation::GravityUpright);
        match art.source {
            ProjectileArtSource::Sheet {
                target,
                animation,
                animate,
            } => {
                assert_eq!(target, "glider");
                assert_eq!(animation, "fly");
                assert!(animate, "the glider must cycle its fly frames");
            }
            other => panic!("glider should be a Sheet source, got {other:?}"),
        }
    }

    #[test]
    fn lasersword_is_velocity_aligned_static_frame() {
        let art = ProjectileVisualKind::Lasersword.art();
        assert_eq!(art.rotation, ProjectileRotation::VelocityAligned);
        assert!(
            matches!(
                art.source,
                ProjectileArtSource::Sheet { animate: false, .. }
            ),
            "lasersword projectile clips a single frame; it does not cycle"
        );
    }

    #[test]
    fn lasersword_owns_its_custom_expiry_vfx() {
        let pos = ambition_engine_core::Vec2::new(12.0, -3.0);
        let Some(ambition_vfx::vfx::VfxMessage::Explosion {
            pos: got,
            kind,
            scale,
        }) = ProjectileVisualKind::Lasersword.expiry_vfx(pos)
        else {
            panic!("lasersword expiry should emit the authored explosion cue");
        };
        assert_eq!(got, pos);
        assert_eq!(kind, ambition_vfx::vfx::ExplosionKind::ClassicBurst);
        assert_eq!(scale, 0.7);
    }

    #[test]
    fn ordinary_projectiles_have_no_custom_expiry_vfx() {
        let pos = ambition_engine_core::Vec2::new(1.0, 2.0);
        for kind in [
            ProjectileVisualKind::Fireball,
            ProjectileVisualKind::Hadouken,
            ProjectileVisualKind::HadoukenSuper,
            ProjectileVisualKind::Apple,
            ProjectileVisualKind::Glider,
            ProjectileVisualKind::EnemyDefault,
        ] {
            assert!(
                kind.expiry_vfx(pos).is_none(),
                "{kind:?} should use the sim stepper's generic impact fallback"
            );
        }
    }

    #[test]
    fn apple_is_gravity_upright_image() {
        let art = ProjectileVisualKind::Apple.art();
        assert_eq!(art.rotation, ProjectileRotation::GravityUpright);
        assert!(matches!(art.source, ProjectileArtSource::Image { .. }));
    }

    #[test]
    fn player_kits_use_the_tinted_energy_texture() {
        for kind in [
            ProjectileVisualKind::Fireball,
            ProjectileVisualKind::Hadouken,
            ProjectileVisualKind::HadoukenSuper,
        ] {
            assert!(
                matches!(kind.art().source, ProjectileArtSource::EnergyTinted { .. }),
                "{kind:?} should render as the tinted energy ball"
            );
            assert_eq!(kind.art().rotation, ProjectileRotation::FlipToTravel);
        }
    }

    #[test]
    fn visual_tag_round_trips_every_kind() {
        for kind in [
            ProjectileVisualKind::Fireball,
            ProjectileVisualKind::Hadouken,
            ProjectileVisualKind::HadoukenSuper,
            ProjectileVisualKind::Apple,
            ProjectileVisualKind::Lasersword,
            ProjectileVisualKind::Glider,
            ProjectileVisualKind::EnemyDefault,
        ] {
            assert_eq!(
                ProjectileVisualKind::from_tag(kind.to_tag()),
                kind,
                "{kind:?} must survive the foundation visual_tag round-trip"
            );
        }
        // Zero / unknown tags read as the generic hostile shot.
        assert_eq!(
            ProjectileVisualKind::from_tag(0),
            ProjectileVisualKind::EnemyDefault
        );
        assert_eq!(
            ProjectileVisualKind::from_tag(9999),
            ProjectileVisualKind::EnemyDefault
        );
    }

    #[test]
    fn enemy_default_is_a_solid_quad() {
        assert!(matches!(
            ProjectileVisualKind::EnemyDefault.art().source,
            ProjectileArtSource::SolidColor { .. }
        ));
    }
}
