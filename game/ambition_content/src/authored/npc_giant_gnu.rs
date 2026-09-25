//! The carried giant (ADR 0020). A brainless, stationary MOUNT whose
//! RIDER is the threat — GNU-ton, who stays a boss and is not touched
//! here.
//!
//! the first migrated body that authors `is_hostile: false`,
//! and it could not have migrated a day earlier: the character-first
//! constructor wrote that flag as the literal `true`, so a migrated giant
//! would have started hunting the player it exists to carry. The row's
//! hostility half is controller policy and now says so.
//!
//! Its size is its sprite's: see `with_sprite_authored_body` below. Its
//! `respawn: OnRoomReenter` lives on the placement, where a respawn policy
//! belongs.

use ambition_characters::actor::CharacterLocomotion;
use ambition_characters::brain::{BrainProfile, CharacterBrainTemplate, MoveStyleSpec};
use ambition_platformer2d::character::CharacterDefinition;

/// World units per sprite pixel for the giant and its fists: one scale, so a
/// fist is drawn the size of the hand that throws it.
pub(crate) const GNU_WORLD_PER_PIXEL: f32 = 1.3;

/// See the module doc. Reached through [`super::AUTHORED_CAST`], which is also
/// what makes this character buildable — there is no second list to remember.
pub(crate) fn author(_id: &str, definition: CharacterDefinition) -> CharacterDefinition {
    let mut definition = definition
        .with_locomotion(CharacterLocomotion {
            // Grounded heavy locomotion, inert while StandStill — the
            // correct gait for a lumbering giant if ever steered.
            run_speed: 0.0,
            move_style: MoveStyleSpec::WalkHeavy,
            ..Default::default()
        })
        // THE ART IS THE BODY. The giant was a 220x220 placement box with a
        // 768x576 frame scaled x4.5 over it, so it drew three times its own
        // collision and stood on nothing it appeared to stand on. Now the box is
        // the sheet's authored body (338x319 px) at one scale, and its feet are
        // on the floor.
        .with_sheet("giant_gnu")
        .with_sprite_authored_body(GNU_WORLD_PER_PIXEL)
        .with_mount(ambition_characters::actor::CharacterMount {
            class: Some("giant".to_string()),
            // The scholar sits at the base of the gnu's neck: the sheet's
            // shoulder point (388, 313) px against its body-box centre
            // (350, 329.5), at the same scale, lifted half his height.
            saddle: Some((50.0, -67.0)),
            ..Default::default()
        })
        .with_autonomous_profile(BrainProfile {
            template: CharacterBrainTemplate::StandStill,
            aggro_radius: 0.0,
            attack_range: 0.0,
            // It never seeks and never strikes — and `StandStill` with
            // a zero aggro radius already SAYS that. The relationship
            // half ("this creature is not your enemy") is the
            // PLACEMENT's: the sandbox giant authors `Peaceful`.
            ..Default::default()
        });
    definition.vitals.max_health = Some(42);
    // Far heavier than the scholar riding it, so the mount pair's centre
    // of gravity sits on the giant and the lighter rider orbits it under
    // a gravity flip.
    definition.vitals.mass = Some(8.0);
    // No `contact_damage`: a prop-like mount does no damage by being
    // stood next to, which is what `body_contact_damage: false` said.
    definition
}
