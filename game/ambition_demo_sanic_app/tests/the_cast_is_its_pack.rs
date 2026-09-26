//! Sanic's cast is prepared from his pack as the Rust registration built it.
//!
//! His two forms and the badnik were a catalog in a Rust string plus a
//! registration loop and a badnik function that restated each character's
//! body, gait, contact damage and policy. They are rows in his pack now. The
//! expected values below are what that Rust stated, so this checks each
//! prepared character against the authority it replaced.

use ambition_platformer2d::characters::actor::definition::BodySource;
use ambition_platformer2d::characters::actor::{CharacterLocomotion, ContactDamage};
use ambition_platformer2d::characters::brain::{BrainProfile, CharacterBrainTemplate, MoveStyleSpec};
use ambition_platformer2d::characters::prepared::PreparedCharacterRegistry;
use ambition_platformer2d::engine_core as ae;

/// The height the body stands at the scale it was built with, from its sheet.
fn standing_height(sheet: &str, body: Option<&BodySource>) -> f32 {
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
    .y
}

#[test]
fn the_sanic_cast_is_prepared_from_its_pack_as_the_rust_registration_built_it() {
    let mut app = ambition_demo_sanic_app::build_demo_app();
    for _ in 0..30 {
        app.update();
    }
    let registry = app.world().resource::<PreparedCharacterRegistry>();
    let get = |id: &str| {
        registry
            .get(id)
            .unwrap_or_else(|| panic!("`{id}` is not in the prepared cast"))
    };

    // Both forms: his sheet, at one scale, standing 48 units tall.
    let sanic = get("sanic");
    let super_sanic = get("super_sanic");
    assert_eq!(sanic.sheet.as_deref(), Some("sanic"));
    assert_eq!(super_sanic.sheet.as_deref(), Some("super_sanic"));
    assert!(
        (standing_height("sanic", sanic.body.as_ref()) - 48.0).abs() < 1e-3,
        "Sanic stands 48 units tall"
    );
    assert_eq!(
        sanic.body, super_sanic.body,
        "the two forms share one scale; their sizes differ only where their art does"
    );
    for form in [sanic, super_sanic] {
        assert_eq!(form.vitals.max_health, Some(1), "{}", form.id);
    }

    // The badnik: every fact its registration function stated.
    let badnik = get(ambition_demo_sanic::badnik::BADNIK_BRAIN_KEY);
    assert_eq!(badnik.sheet.as_deref(), Some("ai_slop"));
    assert!(
        (standing_height("ai_slop", badnik.body.as_ref()) - 48.0 * 0.6).abs() < 1e-3,
        "a badnik stands three fifths of Sanic"
    );
    assert_eq!(
        badnik.locomotion,
        Some(CharacterLocomotion {
            run_speed: 60.0,
            move_style: MoveStyleSpec::Walk,
            baseline_free_flight: Some(false),
            ..Default::default()
        })
    );
    assert_eq!(
        badnik.motion_model,
        ae::MotionModelSpec::SurfaceMomentum(ae::MomentumParams {
            ground_accel: 600.0,
            brake: 1800.0,
            friction: 1800.0,
            slope_factor: 0.0,
            top_speed: 60.0,
            air_accel: 0.0,
            jump_speed: 0.0,
            stick_factor: 1.5,
            ..ae::MomentumParams::default()
        })
    );
    assert_eq!(
        badnik.contact_damage,
        Some(ContactDamage {
            strength: 0.5,
            amount: 1,
        })
    );
    assert_eq!(
        badnik.autonomous_profile,
        Some(BrainProfile {
            template: CharacterBrainTemplate::Wanderer,
            aggro_radius: 0.0,
            attack_range: 0.0,
            patrol_effort: 1.0,
            turns_at_ledges: true,
            ..Default::default()
        })
    );
    assert_eq!(badnik.vitals.max_health, Some(1));
}
