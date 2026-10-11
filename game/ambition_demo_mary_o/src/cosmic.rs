//! Eternal Cosmic Mary-O: her ultimate form.
//!
//! The form is a catalog row (`mary_o_cosmic`), as each of her forms is. The
//! row states what the form is: she is not hurt and she harms what she
//! touches (`empowered`), she has 255 jumps in the air (the `cosmic` tuning),
//! and she is faster. This module is the way in and the way out: one
//! technique on the Utility control, which changes the character she wears.
//!
//! The jump in the air is a verb, and a body gets its verbs when the session
//! builds it: to wear a row later changes its feel and not its verbs. So the
//! form lends her the verb while she wears it ([`lend_the_jump_in_the_air`]),
//! through the seam each live source of a verb uses (`AbilityContributions`).
//!
//! She is in the form while she wears its row and for no other reason, so the
//! form has no state of its own. The form she comes back to is the rung of
//! her ladder that her equipment puts her on (`powerups::ladder_character`):
//! what she picks up while she is in the form, she has when she leaves it.
//!
//! Today each player has the technique at all times: it is the tool a tester
//! moves through a level with (Jon, 2026-10-11). The form is to be a thing of
//! the game later, and what gives it to her then is not decided.

use bevy::prelude::*;

use ambition_platformer2d::actors::features::transform_beat::{TransformBeat, TransformBeatPolicy, TransformBeatRequested};
use ambition_platformer2d::characters::action_scheme::ResolvedTechniqueEdges;
use ambition_platformer2d::characters::actor::{RecharacterizeBody, WornCharacter};
use ambition_platformer2d::characters::equipment::WornEquipment;
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::platformer::markers::{ControlledSubject, PrimaryPlayer};
use ambition_platformer2d::sprite_sheet::character::CharacterAnim;

/// The catalog id of the form.
pub const COSMIC_CHARACTER_ID: &str = "mary_o_cosmic";
/// The display name of its row.
pub const COSMIC_DISPLAY_NAME: &str = "Eternal Cosmic Mary-O";

/// The id of the form's control in the action scheme, and so the word on the
/// control: `ActionSpec::display` title-cases it to "Transform".
pub const TRANSFORM_TECHNIQUE_ID: &str = "transform";

/// The beat of her way out of the form: short, and with no clip. A clip of
/// the form she comes back to is the clip of a hit, and she was not hit.
const LEAVE_BEAT: TransformBeatPolicy = TransformBeatPolicy {
    duration: 0.3,
    anim: CharacterAnim::Idle,
    clock_scale: 1.0,
    untouchable: true,
};

/// The owner of the form's claim on the music.
const COSMIC_MUSIC_OWNER: &str = "mary_o_cosmic";
/// The track the form plays. It is the theme of the quasar until the form has
/// a theme of its own.
pub const COSMIC_MUSIC_TRACK: &str = crate::provider::MARY_O_STAR_MUSIC_TRACK;

/// The key of the form's entry in her ability contributions.
const COSMIC_ABILITY_SOURCE: &str = "mary_o_cosmic";

/// Ticks from one sparkle of the form to the next.
const SPARKLE_EVERY_TICKS: u64 = 7;
/// How far from the middle of her body a sparkle starts, in px.
const SPARKLE_RADIUS: f32 = 26.0;

/// Whether `character_id` is the form.
pub fn is_cosmic(character_id: &str) -> bool {
    character_id == COSMIC_CHARACTER_ID
}

/// The transformation's identity in the action scheme: a technique on the
/// Utility control.
pub(crate) fn transform_technique() -> ambition_platformer2d::entity_catalog::action_scheme::ActionSpec {
    use ambition_platformer2d::entity_catalog::action_scheme as sch;
    sch::ActionSpec {
        id: sch::ActionId::new(TRANSFORM_TECHNIQUE_ID),
        slot: sch::ControlSlot::Utility,
        display_name: None,
        visual: None,
        gate: sch::ActionGate::Technique(TRANSFORM_TECHNIQUE_ID.to_string()),
    }
}

/// The beat of her way into the form: the `transform` row of the form's
/// sheet, for as long as the row takes to draw. The world keeps its time: the
/// row is long, and she is not hurt while it plays.
fn enter_beat(
    catalog: Option<&ambition_platformer2d::character::CharacterCatalog>,
    authored: Option<&ambition_platformer2d::character::AuthoredSheets>,
) -> TransformBeatPolicy {
    TransformBeatPolicy {
        duration: crate::powerups::clip_seconds(catalog, authored, COSMIC_CHARACTER_ID, CharacterAnim::Transform),
        anim: CharacterAnim::Transform,
        clock_scale: 1.0,
        untouchable: true,
    }
}

/// Change the body the player drives between the form and the form of her
/// ladder, on a press of the technique. A press while she changes into the
/// form ends the change: she is in the form at once.
pub fn toggle_cosmic_form(
    subject: Option<Res<ControlledSubject>>,
    mut bodies: Query<(
        &ResolvedTechniqueEdges,
        &mut WornCharacter,
        Option<&WornEquipment>,
        &ae::BodyKinematics,
        Option<&mut TransformBeat>,
    )>,
    catalog: Option<Res<ambition_platformer2d::character::CharacterCatalog>>,
    authored: Option<Res<ambition_platformer2d::character::AuthoredSheets>>,
    mut sfx: ambition_platformer2d::sfx::BodySfxWriter,
    mut commands: Commands,
) {
    let Some(body) = subject.and_then(|subject| subject.0) else {
        return;
    };
    let Ok((edges, mut worn, equipment, kin, beat)) = bodies.get_mut(body) else {
        return;
    };
    if !edges.pressed(TRANSFORM_TECHNIQUE_ID) {
        return;
    }
    let in_form = is_cosmic(worn.id());
    if let Some(mut beat) = beat {
        if in_form && beat.policy.anim == CharacterAnim::Transform {
            beat.remaining = 0.0;
            return;
        }
    }
    let (next, policy, cue) = if in_form {
        (crate::powerups::ladder_character(equipment), LEAVE_BEAT, crate::powerups::SFX_FIRE_TO_BIG)
    } else if crate::powerups::is_her_form(worn.id()) {
        (
            COSMIC_CHARACTER_ID,
            enter_beat(catalog.as_deref(), authored.as_deref()),
            crate::powerups::SFX_BIG_TO_FIRE,
        )
    } else {
        // A body that is not Mary-O has no such form.
        return;
    };
    sfx.write_for(
        body,
        ambition_platformer2d::sfx::SfxMessage::Play {
            id: ambition_platformer2d::sfx::SfxId::from_static(cue),
            pos: kin.pos,
        },
    );
    worn.0 = next.into();
    commands
        .entity(body)
        .try_insert((TransformBeatRequested(policy), RecharacterizeBody));
}

/// The verbs the form lends: the jump in the air.
fn lent_verbs() -> ae::AbilitySet {
    ae::AbilitySet {
        double_jump: true,
        ..ae::AbilitySet::NONE
    }
}

/// Lend her the jump in the air while she wears the form, and take it back
/// after. The entry is read from the character she wears on each tick, so it
/// is no state of its own. How many jumps she has is the form's tuning.
pub fn lend_the_jump_in_the_air(mut bodies: Query<(&WornCharacter, &mut ae::AbilityContributions), With<PrimaryPlayer>>) {
    for (worn, mut contributions) in &mut bodies {
        let lent = contributions.get(COSMIC_ABILITY_SOURCE).is_some();
        let wanted = is_cosmic(worn.id());
        if wanted && !lent {
            contributions.set(COSMIC_ABILITY_SOURCE, ae::AbilityContribution::Lend(lent_verbs()));
        } else if !wanted && lent {
            contributions.clear(COSMIC_ABILITY_SOURCE);
        }
    }
}

/// Where the sparkle of `tick` starts, from the middle of her body: each one
/// a golden angle on from the one before it, so they go round her evenly.
fn sparkle_offset(tick: u64) -> ae::Vec2 {
    let turn = (tick / SPARKLE_EVERY_TICKS) as f32 * 2.399_963;
    ae::Vec2::new(turn.cos(), turn.sin()) * SPARKLE_RADIUS
}

/// Sparkles about her while she is in the form. They are the engine's own
/// particles, so they are no picture of hers and no state: the tick says when
/// and where.
pub fn sparkle_while_cosmic(
    tick: Option<Res<ambition_platformer2d::time::SimTick>>,
    rooms: ambition_platformer2d::platformer::lifecycle::LiveRooms,
    mut vfx: ambition_platformer2d::vfx::VfxWriter,
    players: Query<(Entity, &WornCharacter, &ae::BodyKinematics), With<PrimaryPlayer>>,
) {
    let Some(tick) = tick.map(|tick| tick.get()) else {
        return;
    };
    if tick % SPARKLE_EVERY_TICKS != 0 {
        return;
    }
    for (body, worn, kin) in &players {
        if !is_cosmic(worn.id()) {
            continue;
        }
        // Gold, then pink, then cyan: the colours of her wings.
        let color = [[1.0, 0.86, 0.42, 1.0], [1.0, 0.56, 0.80, 1.0], [0.55, 0.90, 1.0, 1.0]][((tick / SPARKLE_EVERY_TICKS) % 3) as usize];
        vfx.for_room(rooms.of(body)).write(ambition_platformer2d::vfx::VfxMessage::Burst {
            pos: kin.pos + sparkle_offset(tick),
            count: 2,
            speed: 46.0,
            color,
            kind: ambition_platformer2d::vfx::ParticleKind::Spark,
        });
    }
}

/// The theme of the form, on the priority-music seam the quasar uses: claimed
/// in the live room of each body in the form, and released when no body in
/// that room is in it, so the theme of the level comes back with no record
/// here of what it was.
pub fn play_cosmic_music(
    bodies: Query<(Entity, &WornCharacter), With<PrimaryPlayer>>,
    live: ambition_platformer2d::platformer::lifecycle::LiveRooms,
    music: Option<
        ambition_platformer2d::platformer::lifecycle::SessionWorldMut<
            ambition_platformer2d::encounter::EncounterMusicRequest,
        >,
    >,
    // The tick, which dates a music claim.
    sim_tick: Option<Res<ambition_platformer2d::sim::SimTick>>,
) {
    let Some(mut music) = music else {
        return;
    };
    let now = sim_tick.as_ref().map_or(0, |tick| tick.0);
    let rooms: Vec<_> = bodies
        .iter()
        .filter(|(_, worn)| is_cosmic(worn.id()))
        .map(|(body, _)| live.of(body))
        .collect();
    music.release_priority_where(COSMIC_MUSIC_OWNER, |_, room| !rooms.contains(&room));
    for room in rooms {
        music.claim_priority(room, COSMIC_MUSIC_OWNER, COSMIC_MUSIC_TRACK, now);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The form lends one verb and no other: a form that lent `fly` would
    /// take the Utility control from its own way out.
    #[test]
    fn the_form_lends_the_jump_in_the_air_and_no_other_verb() {
        let lent = lent_verbs();
        assert!(lent.double_jump);
        assert_eq!(
            lent,
            ae::AbilitySet {
                double_jump: true,
                ..ae::AbilitySet::NONE
            }
        );
        assert_eq!(lent.air_jump_count(255), 255);
    }

    /// The sparkles go round her: no two in a row start at one place.
    #[test]
    fn two_sparkles_in_a_row_start_at_two_places() {
        let (a, b) = (sparkle_offset(0), sparkle_offset(SPARKLE_EVERY_TICKS));
        assert!((a - b).length() > 1.0, "{a:?} and {b:?}");
        assert!((a.length() - SPARKLE_RADIUS).abs() < 1e-3);
    }
}
