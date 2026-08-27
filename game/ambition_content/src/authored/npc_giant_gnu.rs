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
//! `default_size` does NOT come across, and the placement is why: the
//! sandbox's giant is authored as a 220x220 LDtk box, exactly the
//! envelope the row was restating, so the size survives without a second
//! authority stating it. Its `respawn: OnRoomReenter` moves to the
//! placement, where a respawn policy belongs.

use ambition_characters::actor::CharacterLocomotion;
use ambition_characters::brain::{BrainProfile, CharacterBrainTemplate, MoveStyleSpec};
use ambition_platformer2d::character::CharacterDefinition;

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
        .with_mount(ambition_characters::actor::CharacterMount {
            class: Some("giant".to_string()),
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
