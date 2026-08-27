//! WHAT A FIGHTER'S TABLE SOUNDS LIKE, ASKED OF THE REAL SEAM.
//!
//! the one claim here is that an authored burst is heard EXACTLY ONCE, and it is a claim no
//! per-table test could make. Those tests could only ever check for the MISSING half.
//!
//! it is not a data test. The oracle is the running pair of systems the
//! game installs: `ambition_combat::moveset::dispatch_move_events` writing onto
//! the request channel, and `ambition_render::fx::process_fx_requests` fanning
//! it into the effect plus the cue the effect's own name addresses. Modelling
//! that fan-out here would pin the model, not the engine.

use ambition_combat::moveset::{dispatch_move_events, MoveEventMessage};
use ambition_platformer2d::entity_catalog::{MoveEventKind, MoveSpec, MovesetContract};
use ambition_platformer2d_core as ae;
use ambition_render::fx::process_fx_requests;
use ambition_sfx::{OwnedSfxMessage, PresentationSourceId, SfxId, SfxMessage};
use bevy::prelude::*;

use crate::authored_movesets::tables;


/// Drive one authored INSTANT of one move through the real seam and report
/// every cue that reached the SFX channel.
fn cues_at_one_instant(
    spec: &MoveSpec,
    at_s: f32,
    which: fn(&MoveEventKind) -> bool,
) -> Vec<SfxId> {
    let mut app = App::new();
    app.add_message::<MoveEventMessage>();
    app.add_message::<OwnedSfxMessage>();
    app.add_message::<ambition_vfx::VfxMessage>();
    app.add_message::<ambition_vfx::FxRequest>();
    app.add_message::<ambition_characters::brain::ActorActionMessage>();
    // `.chain()` inserts the `ApplyDeferred` that makes the fan-out see what
    // the dispatcher just wrote — the two really do run in this order in the
    // host (`ambition_platformer2d_host`).
    app.add_systems(Update, (dispatch_move_events, process_fx_requests).chain());
    let owner = app
        .world_mut()
        .spawn(ae::BodyKinematics {
            pos: ae::Vec2::ZERO,
            vel: ae::Vec2::ZERO,
            size: ae::Vec2::new(16.0, 24.0),
            facing: 1.0,
        })
        .id();
    for ev in spec
        .events
        .iter()
        .filter(|e| e.at_s == at_s && which(&e.kind))
    {
        app.world_mut()
            .resource_mut::<Messages<MoveEventMessage>>()
            .write(MoveEventMessage {
                world_offset: ae::Vec2::ZERO,
                owner,
                move_id: spec.id.clone(),
                presentation_source: PresentationSourceId::unscoped(),
                kind: ev.kind.clone(),
                world_pose: ambition_vfx::FxPose::UPRIGHT,
            });
    }
    app.update();
    let messages = app.world().resource::<Messages<OwnedSfxMessage>>();
    let mut cursor = messages.get_cursor();
    cursor
        .read(messages)
        .filter_map(|m| match m.request {
            SfxMessage::Play { id, .. } => Some(id),
            _ => None,
        })
        .collect()
}

/// The name behind a hashed cue, for a failure message that can be acted on.
///
/// [`SfxId`] is a one-way hash, so the report reverses it against the names this
/// build could possibly have played: every authored effect's paired cue, plus
/// every cue any table names by hand.
fn name_of(id: SfxId) -> String {
    for effect in ambition_platformer2d::sprite_sheet::fx::authored_effects().values() {
        if SfxId::new(&effect.cue) == id {
            return effect.cue.clone();
        }
        let looped = format!("{}.loop", effect.cue);
        if SfxId::new(&looped) == id {
            return looped;
        }
    }
    for (_, set) in tables() {
        for m in &set.moves {
            for ev in &m.events {
                let named = match &ev.kind {
                    MoveEventKind::Sfx { cue } => Some(cue.clone()),
                    MoveEventKind::Vfx { sfx: Some(cue), .. } => Some(cue.clone()),
                    _ => None,
                };
                if let Some(cue) = named {
                    if SfxId::new(&cue) == id {
                        return cue;
                    }
                }
            }
        }
    }
    format!("{id}")
}

/// A BURST'S OWN CUE IS NOT ALSO WRITTEN BY HAND — IT IS HEARD ONCE.
///
/// Every jab, tilt and smash in those tables played its sound twice, 412 app tests stayed green,
/// and the only instrument that could have caught it was a fighter's ears.
///
/// the two halves of one instant are each run through the real dispatcher and
/// the real fan-out and then INTERSECTED, so what a burst "already says" is
/// answered by the engine rather than by a table transcribed from it.
///
/// two mirrored pairs deliberately fall outside this, Alice's `side_channel` and the
/// cellular automaton's `garden_growth` each throw the SAME effect at `±x` on the same frame,
/// so each is heard twice — two spatialised bursts, two spatialised sounds.
#[test]
fn a_paired_burst_is_heard_exactly_once() {
    fn is_burst(k: &MoveEventKind) -> bool {
        matches!(k, MoveEventKind::Vfx { .. })
    }
    fn is_hand_written(k: &MoveEventKind) -> bool {
        matches!(k, MoveEventKind::Sfx { .. })
    }

    let mut from_bursts = 0usize;
    let mut by_hand = 0usize;
    let mut instants = 0usize;
    for (who, set) in tables() {
        for m in &set.moves {
            let mut moments: Vec<f32> = m.events.iter().map(|e| e.at_s).collect();
            moments.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            moments.dedup();
            for at_s in moments {
                instants += 1;
                let bursts = cues_at_one_instant(m, at_s, is_burst);
                let hand = cues_at_one_instant(m, at_s, is_hand_written);
                from_bursts += bursts.len();
                by_hand += hand.len();
                for id in &hand {
                    assert!(
                        !bursts.contains(id),
                        "{who}'s `{}` writes `{}` by hand at {at_s}s, and the \
                         burst beside it already addresses that cue — so it is \
                         heard TWICE. Delete the `Sfx` event; the request derives \
                         the sound. If the sound is genuinely not the row's \
                         default, say so ON the burst with `vfx_cued`.",
                        m.id,
                        name_of(*id),
                    );
                }
            }
        }
    }
    // A fixture that dispatched nothing satisfies the loop above perfectly, and so does one
    // where the fan-out never ran.
    assert!(
        instants > 150,
        "only {instants} authored instants swept — the tables did not load"
    );
    assert!(
        from_bursts > 100,
        "only {from_bursts} cues came from bursts alone — the pairing is not \
         running, so this test cannot see a doubling either"
    );
    assert!(
        by_hand > 20,
        "only {by_hand} hand-written cues fired — the other half of the \
         intersection is empty, which would make this pass for free"
    );
}

/// A SUSTAINED BURST STILL PLAYS ITS LOOPING VARIANT, NOT THE PLAIN ROW CUE.
///
/// ten of the shipped effect rows pack their sound as `vfx.<family>.<row>.loop`
/// and ship no plain `vfx.<family>.<row>` at all — a stream, an orbit, a held
/// field. They are the reason the override arm is not speculative, and the
/// reason deleting the restatements wholesale would have been wrong: for these
/// the authored cue was never the default, and it now rides on the burst itself
/// (`vfx_cued`) instead of on a second event.
#[test]
fn a_sustained_burst_keeps_its_looping_cue() {
    fn is_burst(k: &MoveEventKind) -> bool {
        matches!(k, MoveEventKind::Vfx { .. })
    }
    let mut overrides = 0usize;
    for (who, set) in tables() {
        for m in &set.moves {
            for ev in &m.events {
                let MoveEventKind::Vfx {
                    effect,
                    sfx: Some(cue),
                    ..
                } = &ev.kind
                else {
                    continue;
                };
                overrides += 1;
                let played = cues_at_one_instant(m, ev.at_s, is_burst);
                assert!(
                    played.contains(&SfxId::new(cue)),
                    "{who}'s `{}` names `{cue}` on its `{effect}` burst and the \
                     channel never heard it",
                    m.id,
                );
                let derived = ambition_platformer2d::sprite_sheet::fx::authored_effect(effect)
                    .map(|e| SfxId::new(&e.cue))
                    .expect("an override sits on an effect the art ships");
                assert!(
                    !played.contains(&derived),
                    "{who}'s `{}` asked for `{cue}` and the plain row cue played \
                     anyway — the override did not replace the default",
                    m.id,
                );
            }
        }
    }
    assert!(
        overrides >= 20,
        "only {overrides} sustained bursts carry their loop cue — the override \
         arm has lost adopters, which means somebody's held field went silent"
    );
}

/// A MOVE'S INDEPENDENT SOUNDS STILL FIRE.
///
/// 50 of the 145 authored cues were never a restatement of anything — a grunt, a charge whine,
/// a metal chink. A future sweep that mistakes one for ceremony trips here rather than in a
/// match.
#[test]
fn a_moves_own_voice_is_not_ceremony() {
    fn is_hand_written(k: &MoveEventKind) -> bool {
        matches!(k, MoveEventKind::Sfx { .. })
    }
    let mut independent = 0usize;
    for (who, set) in tables() {
        for m in &set.moves {
            for ev in &m.events {
                let MoveEventKind::Sfx { cue } = &ev.kind else {
                    continue;
                };
                independent += 1;
                assert!(
                    !cue.starts_with("vfx."),
                    "{who}'s `{}` still hand-writes `{cue}`, which is an effect's \
                     own paired cue — the burst beside it already says that",
                    m.id,
                );
                assert!(
                    cues_at_one_instant(m, ev.at_s, is_hand_written).contains(&SfxId::new(cue)),
                    "{who}'s `{}` names `{cue}` and the channel never heard it",
                    m.id,
                );
            }
        }
    }
    assert!(
        independent >= 40,
        "only {independent} sounds of a move's own survive — losing one is a \
         fighter going quiet"
    );
}
