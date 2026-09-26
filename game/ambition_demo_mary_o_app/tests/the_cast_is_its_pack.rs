//! Mary-O's cast is prepared from her pack as the Rust registration built it.
//!
//! Her three forms, her two walkers and the two plane swarms were a catalog in
//! a Rust string with a movement feel spliced into it three times, a
//! registration loop, and three functions that restated each enemy's body,
//! gait, contact damage and policy. They are rows in her pack now. The expected
//! values below are what that Rust stated, so this checks each prepared
//! character against the authority it replaced.

use ambition_platformer2d::characters::actor::definition::BodySource;
use ambition_platformer2d::characters::actor::{CharacterLocomotion, ContactDamage};
use ambition_platformer2d::characters::brain::{BrainProfile, CharacterBrainTemplate, MoveStyleSpec};
use ambition_platformer2d::characters::prepared::PreparedCharacterRegistry;

/// The idle body, in world units, at the scale the body was built with.
fn idle_body(sheet: &str, body: Option<&BodySource>) -> bevy::prelude::Vec2 {
    let Some(BodySource::SpriteAuthored { world_per_pixel }) = body else {
        panic!("`{sheet}` must be a body its sheet authors per pose, got {body:?}");
    };
    ambition_platformer2d::character_sprites::posed_body_geometry(
        sheet,
        ambition_platformer2d::sprite_sheet::character::CharacterAnim::Idle,
        *world_per_pixel,
    )
    .unwrap_or_else(|| panic!("`{sheet}` has no baked art to measure"))
    .collision
}

#[test]
fn the_mary_o_cast_is_prepared_from_its_pack_as_the_rust_registration_built_it() {
    let mut app = ambition_demo_mary_o_app::build_demo_app();
    for _ in 0..30 {
        app.update();
    }
    let registry = app.world().resource::<PreparedCharacterRegistry>();
    let get = |id: &str| {
        registry
            .get(id)
            .unwrap_or_else(|| panic!("`{id}` is not in the prepared cast"))
    };

    // Her forms: her sheets, at one scale, the small one standing one tile.
    let small = get("mary_o");
    let tall = get("mary_o_tall");
    let fire = get("mary_o_fire");
    assert_eq!(small.sheet.as_deref(), Some("mary_o_v2"));
    assert_eq!(tall.sheet.as_deref(), Some("mary_o_v2_tall"));
    assert_eq!(fire.sheet.as_deref(), Some("mary_o_v2_fire"));
    let her = idle_body("mary_o_v2", small.body.as_ref());
    assert!((her.y - 32.0).abs() < 1e-3, "small Mary-O stands one tile: {her:?}");
    for form in [tall, fire] {
        assert_eq!(
            form.body, small.body,
            "`{}` shares her one scale; the forms differ only where their art does",
            form.id
        );
    }
    // Mary-O Classic, the feel the Rust spliced into each row.
    let classic = small.movement_tuning.expect("she authors her feel");
    assert_eq!(
        (classic.jump_speed, classic.gravity, classic.max_run_speed, classic.max_fall_speed),
        (450.0, 2250.0, 300.0, 480.0)
    );
    for form in [small, tall, fire] {
        assert_eq!(form.movement_tuning, Some(classic), "{}", form.id);
        assert_eq!(form.vitals.max_health, Some(1), "{}", form.id);
        assert!(
            form.authored_moveset.is_some(),
            "`{}` wears her move table",
            form.id
        );
    }

    // The walkers: one hit point, a forward walk at full effort that reverses
    // at walls, and contact damage as their whole offense.
    let walker = |run_speed: f32| {
        (
            Some(CharacterLocomotion {
                run_speed,
                move_style: MoveStyleSpec::Walk,
                baseline_free_flight: Some(false),
                ..Default::default()
            }),
            Some(ContactDamage {
                strength: 0.5,
                amount: 1,
            }),
            Some(BrainProfile {
                template: CharacterBrainTemplate::Wanderer,
                aggro_radius: 0.0,
                attack_range: 0.0,
                patrol_effort: 1.0,
                ..Default::default()
            }),
            Some(1),
        )
    };
    for (id, run_speed) in [("solid_snake", 46.0), ("ai_slop", 42.0)] {
        let enemy = get(id);
        assert_eq!(enemy.sheet.as_deref(), Some(id));
        assert_eq!(
            (
                enemy.locomotion,
                enemy.contact_damage,
                enemy.autonomous_profile,
                enemy.vitals.max_health
            ),
            walker(run_speed),
            "{id}"
        );
    }
    // The snake is as wide as she is; the slop is 28 wide.
    let snake = idle_body("solid_snake", get("solid_snake").body.as_ref());
    assert!((snake.x - her.x).abs() < 1e-3, "snake {snake:?}, Mary-O {her:?}");
    let slop = idle_body("ai_slop", get("ai_slop").body.as_ref());
    assert!((slop.x - 28.0).abs() < 1e-3, "slop {slop:?}");

    // The plane swarms: free flight at full effort, which a body that cannot
    // see it would lose and fall.
    for (id, sheet, run_speed, max_health) in [
        ("npc_snakes_on_a_paper_plane", "snakes_on_a_paper_plane", 58.0, 1),
        ("npc_snakes_on_a_cartesian_plane", "snakes_on_a_cartesian_plane", 38.0, 2),
    ] {
        let plane = get(id);
        assert_eq!(plane.sheet.as_deref(), Some(sheet));
        assert_eq!(plane.body, None, "{id} is sized by its placement, not per pose");
        assert_eq!(
            (
                plane.locomotion,
                plane.contact_damage,
                plane.autonomous_profile,
                plane.vitals.max_health
            ),
            (
                Some(CharacterLocomotion {
                    run_speed,
                    move_style: MoveStyleSpec::Float,
                    baseline_free_flight: Some(true),
                    ..Default::default()
                }),
                Some(ContactDamage {
                    strength: 0.5,
                    amount: 1,
                }),
                Some(BrainProfile {
                    template: CharacterBrainTemplate::Aerial,
                    aggro_radius: 0.0,
                    attack_range: 0.0,
                    patrol_effort: 1.0,
                    chase_effort: 1.0,
                    ..Default::default()
                }),
                Some(max_health),
            ),
            "{id}"
        );
    }
}
