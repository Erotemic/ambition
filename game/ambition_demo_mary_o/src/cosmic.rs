//! Eternal Cosmic Mary-O: her ultimate form.
//!
//! The form is a catalog row (`mary_o_cosmic`), as each of her forms is. The
//! row states what the form is: she is not hurt and she harms what she
//! touches (`empowered`), she has 255 jumps in the air (the `cosmic` tuning),
//! and she is faster. This module is the way in and the way out: one
//! technique on the Utility control, which changes the character she wears.
//!
//! Each way has a row of the form's sheet. Her way in changes her character
//! and then plays `transform`. Her way out plays `shrink` and then changes
//! her character ([`come_back_from_the_form`]): the picture is of the form she
//! leaves, so she wears that form while it plays.
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

use ambition_platformer2d::actors::body::transform_beat::TransformBeat;
use ambition_platformer2d::actors::body::transform_beat::TransformBeatPolicy;
use ambition_platformer2d::actors::body::transform_beat::TransformBeatRequested;
use ambition_platformer2d::characters::action_scheme::ResolvedTechniqueEdges;
use ambition_platformer2d::characters::actor::{RecharacterizeBody, WornCharacter};
use ambition_platformer2d::characters::equipment::WornEquipment;
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::platformer::markers::ControlledSubject;
use ambition_platformer2d::sprite_sheet::character::CharacterAnim;

/// The catalog id of the form.
pub const COSMIC_CHARACTER_ID: &str = "mary_o_cosmic";

/// The id of the form's control in the action scheme, and so the word on the
/// control: `ActionSpec::display` title-cases it to "Transform".
pub const TRANSFORM_TECHNIQUE_ID: &str = "transform";

/// The clip of her way out of the form: the row `shrink` of the form's sheet
/// (`mary_o_v2_cosmic.LEAVE_ROW` of the sprite renderer). No other thing plays
/// that clip on the form: she is not hurt in it.
const LEAVE_ANIM: CharacterAnim = CharacterAnim::Shrink;
/// The beat of the form she comes back to: `cosmic.arrive_beat_seconds` long,
/// and with no clip. A clip of that form is the clip of a hit, and she was not
/// hit.
fn arrive_beat() -> TransformBeatPolicy {
    TransformBeatPolicy {
        duration: crate::rules::mary_o_rules().cosmic.arrive_beat_seconds,
        anim: CharacterAnim::Idle,
        clock_scale: 1.0,
        untouchable: true,
    }
}

/// The owner of the form's claim on the music.
const COSMIC_MUSIC_OWNER: &str = "mary_o_cosmic";
/// The track the form plays: its own theme (the cue `eternal_cosmic` of the
/// music renderer), whose intro is as long as the transformation.
pub const COSMIC_MUSIC_TRACK: &str = "eternal_cosmic";

/// The key of the form's entry in her ability contributions.
const COSMIC_ABILITY_SOURCE: &str = "mary_o_cosmic";

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
fn enter_beat(sprite_content: &ambition_platformer2d::sprite_sheet::SpriteContent,
    catalog: Option<&ambition_platformer2d::character::CharacterCatalog>,
    authored: Option<&ambition_platformer2d::character::AuthoredSheets>,
) -> TransformBeatPolicy {
    TransformBeatPolicy {
        duration: crate::powerups::clip_seconds(&sprite_content, catalog, authored, COSMIC_CHARACTER_ID, CharacterAnim::Transform),
        anim: CharacterAnim::Transform,
        clock_scale: 1.0,
        untouchable: true,
    }
}

/// The beat of her way out of the form: the row of [`LEAVE_ANIM`] of the
/// form's sheet, and `cosmic.leave_hold_seconds` (`mary_o_rules.ron`) on its last frame. She wears the form
/// while it plays.
fn leave_beat(sprite_content: &ambition_platformer2d::sprite_sheet::SpriteContent,
    catalog: Option<&ambition_platformer2d::character::CharacterCatalog>,
    authored: Option<&ambition_platformer2d::character::AuthoredSheets>,
) -> TransformBeatPolicy {
    TransformBeatPolicy {
        duration: crate::powerups::clip_seconds(&sprite_content, catalog, authored, COSMIC_CHARACTER_ID, LEAVE_ANIM) + crate::rules::mary_o_rules().cosmic.leave_hold_seconds,
        anim: LEAVE_ANIM,
        clock_scale: 1.0,
        untouchable: true,
    }
}

/// Whether `beat` is her way out of the form.
fn is_leaving(beat: &TransformBeat) -> bool {
    beat.policy.anim == LEAVE_ANIM
}

/// Change the body the player drives between the form and the form of her
/// ladder, on a press of the technique. A press while she changes into the
/// form ends the change: she is in the form at once. A press while she
/// changes out of it ends that change: she is out of it on the next tick.
pub fn toggle_cosmic_form(sprite_content: bevy::prelude::Res<ambition_platformer2d::sprite_sheet::SpriteContent>,
    subject: Option<Res<ControlledSubject>>,
    mut bodies: Query<(
        &ResolvedTechniqueEdges,
        &mut WornCharacter,
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
    let Ok((edges, mut worn, kin, beat)) = bodies.get_mut(body) else {
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
        if in_form && is_leaving(&beat) {
            // No time is left of it, so `come_back_from_the_form` takes her
            // out of the form on this tick.
            beat.remaining = 0.0;
            return;
        }
    }
    let cue = if in_form {
        // Her way out: the row plays on the form, and she wears the form
        // until it ends.
        commands
            .entity(body)
            .try_insert(TransformBeatRequested(leave_beat(&sprite_content, catalog.as_deref(), authored.as_deref())));
        crate::forms::mary_o_forms().top_cue_down()
    } else if crate::powerups::is_her_form(worn.id()) {
        worn.0 = COSMIC_CHARACTER_ID.into();
        commands.entity(body).try_insert((
            TransformBeatRequested(enter_beat(&sprite_content, catalog.as_deref(), authored.as_deref())),
            RecharacterizeBody,
        ));
        crate::forms::mary_o_forms().top_cue_up()
    } else {
        // A body that is not Mary-O has no such form.
        return;
    };
    // The form borrows the sounds of the top rung of her ladder: arriving at
    // it, and leaving it for the rung below.
    let Some(cue) = cue else {
        return;
    };
    sfx.write_for(
        body,
        ambition_platformer2d::sfx::SfxMessage::Play {
            id: ambition_platformer2d::sfx::SfxId::from_static(cue),
            pos: kin.pos,
        },
    );
}

/// Take her out of the form when the row of her way out has played: she
/// wears the form of her ladder that her equipment gives her, with a short
/// beat of its own.
///
/// The beat of her way out is the only record that she is on her way out.
/// The engine takes a beat away when its time is up, so she comes back while
/// it still has the time of its hold, or the time of two ticks when a tick is
/// longer than the hold: a beat is not gone before this sees its end.
pub fn come_back_from_the_form(
    time: Res<ambition_platformer2d::time::WorldTime>,
    mut bodies: Query<(Entity, &mut WornCharacter, Option<&WornEquipment>, &TransformBeat)>,
    mut commands: Commands,
) {
    for (body, mut worn, equipment, beat) in &mut bodies {
        if !is_cosmic(worn.id()) || !is_leaving(beat) {
            continue;
        }
        if beat.remaining > crate::rules::mary_o_rules().cosmic.leave_hold_seconds.max(2.0 * time.wall_dt()) {
            continue;
        }
        worn.0 = crate::powerups::ladder_character(equipment).into();
        commands
            .entity(body)
            .try_insert((TransformBeatRequested(arrive_beat()), RecharacterizeBody));
    }
}

/// The sky of a level has a top: a body of the player that goes up in the
/// air stops at the top of its room.
///
/// The form has a jump for each press, so she went up with no end: 4400 px
/// over the top of 1-1 in ten seconds (measured, 2026-10-11). The view stays
/// in the room, so she was out of it, and the view went up and down as it
/// looked for her (Jon: "a camera bug when you are above the intended play
/// zone, it keeps snapping up and down"). A body on the ground is not moved:
/// a ledge near the top of a room is a place she can stand, with her head
/// over the top.
///
/// A level with a sky zone has a higher sky: she goes up out of the view as
/// far as the zone (`crate::sky_top`), and the zone takes her.
pub fn the_sky_has_a_top(
    rooms: Option<ambition_platformer2d::world::rooms::LiveRoomSpecs>,
    mut bodies: Query<
        (Entity, &mut ae::BodyKinematics, Option<&mut ae::SweepSample>, &ae::BodyGroundState),
        With<ambition_platformer2d::platformer::markers::PlayerEntity>,
    >,
) {
    for (entity, mut kin, mut sweep, ground) in &mut bodies {
        let half = kin.size.y * 0.5;
        // The top of a room is the line y = 0 of its own space. With no live
        // room, that line is the top.
        let top = rooms.as_ref().and_then(|rooms| rooms.spec_of(entity)).map_or(0.0, crate::sky_top);
        if ground.on_ground || kin.pos.y >= top + half {
            continue;
        }
        // The top holds her as a ceiling does: at the line, with no speed up.
        // The sweep moves with her, so the next step does not see a jump.
        let at = ae::Vec2::new(kin.pos.x, top + half);
        let vel = ae::Vec2::new(kin.vel.x, kin.vel.y.max(0.0));
        ae::movement::constrain_body_pose(&mut kin, sweep.as_deref_mut(), at, vel);
    }
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
pub fn lend_the_jump_in_the_air(mut bodies: Query<(&WornCharacter, &mut ae::AbilityContributions)>) {
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

/// Cut short the stop of her own body for a hit she lands in the form. It
/// runs after the hits of the tick are resolved, and the clock reads the
/// timer on the next tick. Each body in the form, not only the first player.
pub fn shorten_her_hit_stop(
    mut bodies: Query<(&WornCharacter, &mut ambition_platformer2d::characters::actor::BodyCombat)>,
) {
    for (worn, mut combat) in &mut bodies {
        if is_cosmic(worn.id()) && combat.hitstop_timer > crate::rules::mary_o_rules().cosmic.hit_stop_max_seconds {
            combat.hitstop_timer = crate::rules::mary_o_rules().cosmic.hit_stop_max_seconds;
        }
    }
}

/// Run condition of [`play_cosmic_music`]: a body is in the form, or a cosmic
/// claim stands that may be owed a release.
pub fn cosmic_music_is_owed(
    bodies: Query<&WornCharacter>,
    music: Option<
        ambition_platformer2d::platformer::lifecycle::SessionWorldRef<
            ambition_platformer2d::encounter::EncounterMusicRequest,
        >,
    >,
) -> bool {
    bodies.iter().any(|worn| is_cosmic(worn.id()))
        || music.is_some_and(|music| music.holds_claims_of_kind(COSMIC_MUSIC_OWNER))
}

/// The theme of the form, on the priority-music seam the quasar uses: claimed
/// in the live room of each body in the form, and released when no body in
/// that room is in it, so the theme of the level comes back with no record
/// here of what it was.
pub fn play_cosmic_music(
    bodies: Query<(Entity, &WornCharacter)>,
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

    /// The beat of the form she comes back to is the rules'
    /// `cosmic.arrive_beat_seconds`, with no clip and no slow-down.
    #[test]
    fn the_arrive_beat_is_the_rules_file() {
        let beat = arrive_beat();
        assert_eq!(beat.duration, crate::rules::mary_o_rules().cosmic.arrive_beat_seconds, "the arrive beat is not the rules'");
        assert_eq!((beat.anim, beat.clock_scale), (CharacterAnim::Idle, 1.0));
    }

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
}
