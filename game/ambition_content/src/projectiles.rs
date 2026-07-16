//! Ambition-owned projectile visual registrations.
//!
//! Every named projectile look Ambition ships — the player fireball/Hadouken kit,
//! the GNU-ton apple rain, the pirate gun-sword lasersword discharge, the Perfect
//! Cell-ular Automaton's Conway glider — is registered here into the reusable,
//! empty-by-default [`ProjectileVisualCatalog`](ambition_projectiles::ProjectileVisualCatalog).
//! The engine crate names none of them; a projectile carries an open
//! `ProjectileVisualId` and render + sim resolve it through this registry.
//!
//! Adding a projectile look is one registration here, with no edit to the
//! reusable projectile or render crates (the engine-for-other-games test).

use ambition_projectiles::{
    ProjectileArt, ProjectileArtSource, ProjectileExpiryBurst, ProjectileRenderSize,
    ProjectileRotation, ProjectileVisualAppExt,
};
use bevy::prelude::App;

/// The shared "energy ball" look, tinted per gameplay tier — the player kit.
fn energy_ball(rgba: [f32; 4], label: &str) -> ProjectileArt {
    ProjectileArt {
        source: ProjectileArtSource::EnergyTinted { rgba },
        size: ProjectileRenderSize::Body {
            min: 8.0,
            scale: 1.0,
        },
        rotation: ProjectileRotation::FlipToTravel,
        debug_tint: [rgba[0], rgba[1], rgba[2], 1.0],
        label: label.to_string(),
        expiry_vfx: None,
    }
}

/// Register every named Ambition projectile look into the App-local catalog.
pub(super) fn register(app: &mut App) {
    // Player kit: the shared energy ball, tinted per tier. Warm orange fireball;
    // cool blue Hadouken; stronger blue super.
    app.register_projectile_visual("fireball", energy_ball([1.0, 0.74, 0.30, 0.95], "fireball"));
    app.register_projectile_visual("hadouken", energy_ball([0.45, 0.78, 1.0, 0.96], "hadouken"));
    app.register_projectile_visual(
        "hadouken_super",
        energy_ball([0.30, 0.55, 1.0, 1.0], "hadouken_super"),
    );

    // GNU-ton apple rain: a generated apple sprite, a touch over the body box,
    // upright relative to local gravity.
    app.register_projectile_visual(
        "apple",
        ProjectileArt {
            source: ProjectileArtSource::Image {
                path: "sprites/gnu_ton_boss/gnu_ton_apple.png".to_string(),
            },
            size: ProjectileRenderSize::Body {
                min: 8.0,
                scale: 1.12,
            },
            rotation: ProjectileRotation::GravityUpright,
            debug_tint: [0.90, 0.20, 0.20, 1.0],
            label: "apple".to_string(),
            expiry_vfx: None,
        },
    );

    // Pirate gun-sword: the first idle frame of the lasersword sheet, rotated
    // along the velocity (pommel pivot read from the manifest), detonating on
    // expiry.
    app.register_projectile_visual(
        "lasersword",
        ProjectileArt {
            source: ProjectileArtSource::Sheet {
                target: "lasersword".to_string(),
                animation: "idle".to_string(),
                animate: false,
            },
            size: ProjectileRenderSize::FixedWidth(56.0),
            rotation: ProjectileRotation::VelocityAligned,
            debug_tint: [0.45, 1.0, 0.85, 1.0],
            label: "lasersword".to_string(),
            expiry_vfx: Some(ProjectileExpiryBurst {
                kind: ambition_vfx::vfx::ExplosionKind::ClassicBurst,
                scale: 0.7,
            }),
        },
    );

    // PCA zoning shot: the animated Conway glider, upright vs gravity, sized for
    // arena readability rather than to the small hitbox.
    app.register_projectile_visual(
        "glider",
        ProjectileArt {
            source: ProjectileArtSource::Sheet {
                target: "glider".to_string(),
                animation: "fly".to_string(),
                animate: true,
            },
            size: ProjectileRenderSize::FixedWidth(38.0),
            rotation: ProjectileRotation::GravityUpright,
            debug_tint: [0.40, 0.95, 0.45, 1.0],
            label: "glider".to_string(),
            expiry_vfx: None,
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_projectiles::{ProjectileRotation, ProjectileVisualCatalog};

    #[test]
    fn ambition_registers_every_named_projectile_look() {
        let mut app = App::new();
        register(&mut app);
        let catalog = app.world().resource::<ProjectileVisualCatalog>();
        for id in [
            "fireball",
            "hadouken",
            "hadouken_super",
            "apple",
            "lasersword",
            "glider",
        ] {
            assert!(catalog.get(id).is_some(), "{id} must be registered");
        }
        // An unregistered id resolves to the engine's generic hostile shot.
        assert!(catalog.get("unregistered").is_none());
    }

    #[test]
    fn lasersword_owns_its_detonation_and_the_glider_animates_upright() {
        let mut app = App::new();
        register(&mut app);
        let catalog = app.world().resource::<ProjectileVisualCatalog>();
        assert!(
            catalog.get("lasersword").unwrap().expiry_vfx.is_some(),
            "the pirate lasersword detonates on expiry"
        );
        let glider = catalog.get("glider").unwrap();
        assert_eq!(glider.rotation, ProjectileRotation::GravityUpright);
    }
}
