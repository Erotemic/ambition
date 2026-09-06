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

    // THE PROJECTILE POLYGON'S CHARGE SHOT, in five tiers.
    //
    // ⭐ FIVE LOOKS AND NOT ONE SCALED. A shot the player held for a second has
    // to be readable as different from a tap the instant it leaves the barrel,
    // and scale alone does not survive being small on a busy stage — so the
    // sheet tells the tiers apart by SHAPE: tier 1 is a bare pellet, tier 3
    // gains an orbiting arc, tier 5 gains a compression ring and reads as a
    // different object. `RangedCharge::visuals` steps between them by charge
    // fraction while damage and speed climb smoothly underneath.
    //
    // `Body` sizing, so the drawn ball follows the half-extent the charge
    // scales: a shot that hits in a bigger box than it is drawn in is a lie,
    // and the reverse is a shot that eats you from nowhere.
    for (tier, animation) in [
        (1, "travel_tier1"),
        (2, "travel_tier2"),
        (3, "travel_tier3"),
        (4, "travel_tier4"),
        (5, "travel_tier5"),
    ] {
        app.register_projectile_visual(
            format!("polygon_charge_shot_tier{tier}"),
            ProjectileArt {
                source: ProjectileArtSource::Sheet {
                    target: "polygon_charge_shot".to_string(),
                    animation: animation.to_string(),
                    animate: true,
                },
                size: ProjectileRenderSize::Body {
                    min: 10.0,
                    scale: 2.4,
                },
                // A ball has no forward face to align, and spinning one that is
                // drawn with its own orbit would fight the animation.
                rotation: ProjectileRotation::FlipToTravel,
                debug_tint: [0.38, 0.84, 0.99, 1.0],
                label: format!("polygon_charge_shot_tier{tier}"),
                expiry_vfx: None,
            },
        );
    }

    // The gauntlet fireball a player throws: its own glowing sprite, radial,
    // drawn a touch over the 24 x 18 contact box so the fire visibly fills the
    // space that hits. This is the look the deleted held-shot renderer gave it
    // (a 30 px `gauntlet_fireball.png` sprite); the catalog's "fireball" id is
    // the tinted energy ball of the player kit and is not this.
    app.register_projectile_visual(
        ambition_platformer2d::characters::brain::action_set::GAUNTLET_FIREBALL_VISUAL,
        ProjectileArt {
            source: ProjectileArtSource::Image {
                path: "sprites/props/gauntlet_fireball.png".to_string(),
            },
            size: ProjectileRenderSize::FixedWidth(30.0),
            rotation: ProjectileRotation::GravityUpright,
            debug_tint: [1.0, 0.55, 0.20, 1.0],
            label: "gauntlet_fireball".to_string(),
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
                fx: ambition_vfx::fx::ids::CLASSIC_BURST,
                scale: 0.7,
            }),
        },
    );

    // THE OFFICER'S SERVICE ROUND — his sidearm's shot, with his own art.
    //
    // ⭐⭐ REGISTERED BECAUSE IT WAS NOT. `RangedActionSpec::pistol` authored no
    // `visual` and the Officer's `CharacterDefinition` sets no `ranged_vfx`, so
    // his shot resolved through an EMPTY id to `ProjectileArt::generic()` — the
    // engine's content-free orange-red quad. His sheet draws a drawn pistol and
    // a muzzle flare, and what came out of it was the debug placeholder.
    //
    // ⛔ AND THE PLACEHOLDER IS WHY THE FLIP LOOKED INNOCENT. `FlipToTravel`
    // mirrors the sprite with `flip_x = vel.x < 0.0`, which assumes art drawn
    // pointing +x. A SOLID-COLOUR QUAD IS SYMMETRIC, so that flip was a no-op
    // and the axis could not be seen to be right or wrong from the picture. The
    // sheet is authored nose-+x (`targets/projectiles/pistol_round.py` states
    // it), which is what makes the rotation axis meaningful here.
    //
    // `FixedWidth`, not `Body`: the round's hitbox is the shared pistol
    // envelope and a bullet drawn to it would be a fat lozenge. A slug reads by
    // its silhouette at a fixed size.
    app.register_projectile_visual(
        "pistol_round",
        ProjectileArt {
            source: ProjectileArtSource::Sheet {
                target: "pistol_round".to_string(),
                animation: "travel".to_string(),
                animate: true,
            },
            size: ProjectileRenderSize::FixedWidth(22.0),
            rotation: ProjectileRotation::FlipToTravel,
            debug_tint: [0.97, 0.81, 0.42, 1.0],
            label: "pistol_round".to_string(),
            expiry_vfx: None,
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
