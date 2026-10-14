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

/// A row of the plain-shot sheet (the renderer's `shot_kit`): drawn nose
/// along its flight, the head of the shot on the body. `width` is how wide the
/// art is drawn, in body widths (the art has a tail the body does not).
fn plain_shot(row: &str, width: f32, debug_tint: [f32; 4], burst: Option<f32>) -> ProjectileArt {
    ProjectileArt {
        source: ProjectileArtSource::Sheet {
            target: "shot_kit".to_string(),
            animation: row.to_string(),
            animate: true,
        },
        size: ProjectileRenderSize::BodyWidth { scale: width },
        rotation: ProjectileRotation::VelocityAligned,
        debug_tint,
        label: row.to_string(),
        expiry_vfx: burst.map(|scale| ProjectileExpiryBurst {
            fx: ambition_vfx::fx::ids::CLASSIC_BURST,
            scale,
        }),
    }
}

/// Register every named Ambition projectile look into the App-local catalog.
pub(super) fn register(app: &mut App) {
    // Player kit: a comet of fire; a ball of blue force behind a crescent
    // front; the same, larger, with a second crescent.
    app.register_projectile_visual("fireball", plain_shot("fireball", 1.9, [1.0, 0.74, 0.30, 1.0], None));
    app.register_projectile_visual("hadouken", plain_shot("hadouken", 2.1, [0.45, 0.78, 1.0, 1.0], None));
    app.register_projectile_visual(
        "hadouken_super",
        plain_shot("hadouken_super", 1.8, [0.30, 0.55, 1.0, 1.0], None),
    );

    // The looks of the plain ranged styles (`RangedStyle::default_visual`): a
    // thrown rock, which tumbles in its row and is not turned to its flight,
    // and an arrow.
    app.register_projectile_visual(
        ambition_platformer2d::characters::brain::action_set::ROCK_VISUAL,
        ProjectileArt {
            rotation: ProjectileRotation::FlipToTravel,
            ..plain_shot("rock", 1.45, [0.55, 0.50, 0.45, 1.0], None)
        },
    );
    app.register_projectile_visual(
        ambition_platformer2d::characters::brain::action_set::ARROW_VISUAL,
        ProjectileArt {
            source: ProjectileArtSource::Sheet {
                target: "bow_arrow".to_string(),
                animation: "idle".to_string(),
                animate: false,
            },
            size: ProjectileRenderSize::FixedWidth(44.0),
            rotation: ProjectileRotation::VelocityAligned,
            debug_tint: [0.62, 0.45, 0.25, 1.0],
            label: "arrow".to_string(),
            expiry_vfx: None,
        },
    );
    // The dilophosaurus's spit: a glob of venom, with drops behind it.
    app.register_projectile_visual("spit", plain_shot("spit", 2.05, [0.62, 0.84, 0.20, 1.0], Some(0.4)));
    // A shot that names no look (the empty id): a bolt of magic. A boss's
    // volley and a `Bolt` ranged action are this, not the engine's orange box.
    app.register_projectile_visual("", plain_shot("bolt", 1.75, [0.74, 0.40, 0.95, 1.0], Some(0.4)));

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

    // The Flying Spaghetti Monster's thrown meatball: the meatball it wears,
    // sauced, a touch over the body box, upright relative to local gravity.
    app.register_projectile_visual(
        "meatball",
        ProjectileArt {
            source: ProjectileArtSource::Image {
                path: "sprites/fsm_meatball.png".to_string(),
            },
            size: ProjectileRenderSize::Body {
                min: 8.0,
                scale: 1.15,
            },
            rotation: ProjectileRotation::GravityUpright,
            debug_tint: [0.55, 0.30, 0.20, 1.0],
            label: "meatball".to_string(),
            expiry_vfx: None,
        },
    );

    // The Tyrant King's falling rock: a chunk of his arena's ceiling, shaken
    // loose by a stomp, a landing or a crash. A touch over the body box,
    // upright: it falls, it does not fly.
    app.register_projectile_visual(
        "trex_rock",
        ProjectileArt {
            source: ProjectileArtSource::Image {
                path: "sprites/trex_rock.png".to_string(),
            },
            size: ProjectileRenderSize::Body {
                min: 10.0,
                scale: 1.2,
            },
            rotation: ProjectileRotation::GravityUpright,
            debug_tint: [0.50, 0.44, 0.40, 1.0],
            label: "trex_rock".to_string(),
            expiry_vfx: None,
        },
    );

    // The Hunny Horror's honey: the globs he spits and the drops his roar
    // shakes from the ceiling. A touch over the body box, upright: it falls.
    app.register_projectile_visual(
        "honey_glob",
        ProjectileArt {
            source: ProjectileArtSource::Image {
                path: "sprites/honey_glob.png".to_string(),
            },
            size: ProjectileRenderSize::Body {
                min: 10.0,
                scale: 1.3,
            },
            rotation: ProjectileRotation::GravityUpright,
            debug_tint: [0.96, 0.66, 0.12, 1.0],
            label: "honey_glob".to_string(),
            expiry_vfx: None,
        },
    );

    // The Hunny Horror's red balloon: he rises on it, lets go, and it floats
    // on to pop on the ceiling. The picture is the balloon over its string,
    // which ends at the bottom of the picture, in his paw; upright. From its
    // sheet, which keeps the picture's shape (an `Image` is drawn square).
    app.register_projectile_visual(
        "red_balloon",
        ProjectileArt {
            source: ProjectileArtSource::Sheet {
                target: "red_balloon".to_string(),
                animation: "idle".to_string(),
                animate: false,
            },
            size: ProjectileRenderSize::FixedWidth(56.0),
            rotation: ProjectileRotation::GravityUpright,
            debug_tint: [0.84, 0.12, 0.16, 1.0],
            label: "red_balloon".to_string(),
            expiry_vfx: None,
        },
    );

    // The Smirking Behemoth's eye beam: a line of bolts of light from its
    // eye, nose along their flight. Each is drawn longer than the space
    // between two of them, so the line reads as one beam.
    app.register_projectile_visual(
        "eye_beam",
        ProjectileArt {
            source: ProjectileArtSource::Sheet {
                target: "behemoth_eye_bolt".to_string(),
                animation: "fly".to_string(),
                animate: true,
            },
            size: ProjectileRenderSize::FixedWidth(62.0),
            rotation: ProjectileRotation::VelocityAligned,
            debug_tint: [1.0, 0.93, 0.55, 1.0],
            label: "eye_beam".to_string(),
            expiry_vfx: Some(ProjectileExpiryBurst {
                fx: ambition_vfx::fx::ids::CLASSIC_BURST,
                scale: 0.45,
            }),
        },
    );
    // The Mockingbird's wingtip missile: its own sheet (the missile its
    // wing carries, motor lit, the flame flickering), nose along its flight,
    // bursting where it ends.
    app.register_projectile_visual(
        "mockingbird_missile",
        ProjectileArt {
            source: ProjectileArtSource::Sheet {
                target: "mockingbird_missile".to_string(),
                animation: "fly".to_string(),
                animate: true,
            },
            size: ProjectileRenderSize::FixedWidth(78.0),
            rotation: ProjectileRotation::VelocityAligned,
            debug_tint: [0.82, 0.84, 0.88, 1.0],
            label: "mockingbird_missile".to_string(),
            expiry_vfx: Some(ProjectileExpiryBurst {
                fx: ambition_vfx::fx::ids::CLASSIC_BURST,
                scale: 0.9,
            }),
        },
    );
    // Its fireball: the energy ball, hotter and redder than the player's,
    // bursting where it ends.
    app.register_projectile_visual(
        "mockingbird_fireball",
        ProjectileArt {
            expiry_vfx: Some(ProjectileExpiryBurst {
                fx: ambition_vfx::fx::ids::CLASSIC_BURST,
                scale: 0.6,
            }),
            label: "mockingbird_fireball".to_string(),
            ..plain_shot("fireball", 1.9, [1.0, 0.44, 0.14, 1.0], None)
        },
    );
    // Its fireball once its fire is cold (phase 3): the same ball, blue.
    app.register_projectile_visual(
        "mockingbird_coldfire",
        ProjectileArt {
            expiry_vfx: Some(ProjectileExpiryBurst {
                fx: ambition_vfx::fx::ids::CLASSIC_BURST,
                scale: 0.6,
            }),
            label: "mockingbird_coldfire".to_string(),
            ..plain_shot("coldfire", 1.9, [0.36, 0.72, 1.0, 1.0], None)
        },
    );
    // The burning lightsabers it spits from its mouth: its own sheet, one row
    // for each fire it burns with. The saber's turn is in the row's frames
    // (the flames trail the spin), so the sprite is not turned to its flight.
    for (id, animation) in [("mockingbird_lightsaber", "fly"), ("mockingbird_lightsaber_cold", "fly_cold")] {
        app.register_projectile_visual(
            id,
            ProjectileArt {
                source: ProjectileArtSource::Sheet {
                    target: "mockingbird_lightsaber".to_string(),
                    animation: animation.to_string(),
                    animate: true,
                },
                size: ProjectileRenderSize::FixedWidth(150.0),
                rotation: ProjectileRotation::GravityUpright,
                debug_tint: if animation == "fly" { [1.0, 0.3, 0.15, 1.0] } else { [0.3, 0.6, 1.0, 1.0] },
                label: id.to_string(),
                expiry_vfx: Some(ProjectileExpiryBurst {
                    fx: ambition_vfx::fx::ids::CLASSIC_BURST,
                    scale: 0.7,
                }),
            },
        );
    }

    // The Projectile Polygon's charge shot, in five tiers.
    //
    // Five looks, not one scaled. A held shot must read as different from a tap
    // as it leaves the barrel, and scale alone fails when small on a busy stage.
    // So the sheet tells tiers apart by shape: tier 1 is a bare pellet, tier 3
    // adds an orbiting arc, tier 5 adds a compression ring. `RangedCharge::visuals`
    // steps between them by charge fraction while damage and speed climb
    // smoothly.
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

    // The Officer's service round: his sidearm's shot, with his own art.
    //
    // `RangedActionSpec::pistol` authors no `visual` and the Officer sets no
    // `ranged_vfx`, so without this his shot would resolve an empty id to
    // `ProjectileArt::generic()`, the engine's placeholder quad.
    //
    // `FlipToTravel` mirrors the sprite with `flip_x = vel.x < 0.0`, which
    // assumes art drawn pointing +x. A solid quad is symmetric, so the flip is
    // invisible on it. This sheet is authored nose-+x
    // (`targets/projectiles/pistol_round.py` states it).
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
            "meatball",
            "trex_rock",
            "honey_glob",
            "red_balloon",
            "thrown_rock",
            "arrow",
            "spit",
            // the look of a shot that names none
            "",
            "eye_beam",
            "mockingbird_missile",
            "mockingbird_fireball",
            "mockingbird_coldfire",
            "mockingbird_lightsaber",
            "mockingbird_lightsaber_cold",
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
