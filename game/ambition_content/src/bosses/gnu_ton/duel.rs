//! The duel on the giant's back: the scholar fights with the moves of the
//! fighter `npc_gnu_ton`.
//!
//! His conductor (the module) says in its record that he fights, and which
//! move of the table he chose. This plays that move ON HIS BODY: the same
//! `MoveSpec` the Smash roster's GNU-ton plays (`assets/data/movesets/
//! gnu_ton.ron`), so a move is tuned in one place and is felt in both games.
//!
//! How: a boss body plays a move when its `BossAttackIntent` names one of its
//! `ActorMoveset` (`trigger_boss_attack_moves`). The pattern's brain writes
//! that intent each tick; while he fights, this writes it after the brain.
//! The pattern's own move (the giant's punch) is not played then: the giant
//! stands, and he is the fight.
//!
//! A move of the table is made for the main game here: the table's damage and
//! launch are those of a platform fighter (percent, and a launch that grows
//! with it), and a boss's strike is a few hearts and a fixed launch.
//!
//! A module has no port that starts a move; this is the same road as
//! `portals.rs`. The debt is in `docs/planning/game/gnu-ton-fight-debts.md`.

use bevy::prelude::*;

use ambition_boss_encounter::BossConfig;
use ambition_characters::actor::BodyHealth;
use ambition_characters::brain::{BossAttackIntent, BossAttackProfile};
use ambition_combat::moveset::{cancel_move_playback, ActorMoveset, MoveEnd, MovePlayback};
use ambition_content_modules::gnu_ton::choreography::arms;
use ambition_content_modules::gnu_ton::conductor::{Conductor, GNU_TON_ID};
use ambition_entity_catalog::{MoveEventKind, MoveSpec, VolumeShape, WindowTag};
use ambition_platformer2d::sprite_sheet::character::PinnedRow;

/// The fighter whose table he fights with.
pub const FIGHTER: &str = "npc_gnu_ton";
/// A duel move's id on his body: the table's id with this before it, so that
/// it is none of the pattern's moves.
const PREFIX: &str = "gnu_duel_";
/// The table's damage at and over which a move is a heavy one here.
const HEAVY_DAMAGE: i32 = 12;
/// What a duel move does in the main game: hearts, and a fixed launch (a
/// boss's strike is 1.25).
const LIGHT: (i32, f32) = (1, 1.0);
const HEAVY: (i32, f32) = (2, 1.7);
/// The table is of a fighter as tall as the player. On the giant he is drawn
/// larger (his body is about twice the player's height), so a move's reach is
/// larger by this, and a move that is not over his head is put DOWN, at the
/// height of a player beside him (from the middle of his body), and is at
/// least this tall. Generous on purpose for a first cut.
const REACH: f32 = 1.3;
const OVERHEAD: f32 = -30.0;
const AT_THE_PLAYER: f32 = 24.0;
const LEAST_HALF_HEIGHT: f32 = 30.0;

/// His state in the duel, on his body. Not rollback state: it is found again
/// from the record and from the body's move.
#[derive(Component, Clone, Debug, Default, PartialEq)]
pub struct GnuTonDuel {
    /// The count of the choice that was last started.
    turn: u32,
    /// The move that plays, or is to start: its id on his body.
    playing: Option<String>,
    /// That move was seen on the body: when it is gone, it is over.
    started: bool,
    /// The duel is over, and the pattern's move that was cut is still asked
    /// for: it is not played from its middle.
    cooling: bool,
}

/// The effects of the fighter that call the fist of his giant out of a
/// portal (`gnu_ton_vfx`). They are of the platform-fighter stage. In his
/// boss fight the giant is under him and has its own punch, so a move that
/// calls the fist is a blow of his book here.
pub(super) const CALLED_FIST: &str = "gnu_fist_";
/// The reach of his book, about his body, in the fighter's units: where the
/// blow is, and its half size.
const BOOK: ((f32, f32), (f32, f32)) = ((42.0, -4.0), (30.0, 24.0));

/// A move of the fighter's table, made for the main game and for his body.
fn duel_spec(spec: &MoveSpec) -> MoveSpec {
    let mut spec = spec.clone();
    spec.id = format!("{PREFIX}{}", spec.id);
    let events = spec.events.len();
    spec.events
        .retain(|event| !matches!(&event.kind, MoveEventKind::Vfx { effect, .. } if effect.starts_with(CALLED_FIST)));
    if spec.events.len() != events {
        for volume in spec.windows.iter_mut().flat_map(|window| &mut window.volumes) {
            if let VolumeShape::Rect { offset, half_extents } = &mut volume.shape {
                (*offset, *half_extents) = BOOK;
            }
            // The fist was the picture of the blow; the book's is his stroke.
            volume.vfx = Some(ambition_entity_catalog::authoring::SLASH_ARC_VFX.to_string());
        }
    }
    for window in &mut spec.windows {
        for volume in &mut window.volumes {
            let (damage, launch) = if volume.damage >= HEAVY_DAMAGE { HEAVY } else { LIGHT };
            volume.damage = damage;
            volume.knockback = launch;
            volume.knockback_growth = None;
            if let VolumeShape::Rect { offset, half_extents } = &mut volume.shape {
                offset.0 *= REACH;
                half_extents.0 *= REACH;
                if offset.1 <= OVERHEAD {
                    offset.1 *= 1.6;
                    half_extents.1 *= 1.5;
                } else {
                    offset.1 = AT_THE_PLAYER;
                    half_extents.1 = (half_extents.1 * REACH).max(LEAST_HALF_HEIGHT);
                }
            }
        }
    }
    spec
}

/// The duel moves, made from the fighter's table in the pack this App plays.
fn duel_specs(pack: &crate::pack::SelectedContentPack) -> Vec<MoveSpec> {
    let table = ambition_characters::moveset_content_schema::lowered_movesets(pack.get())
        .and_then(|tables| tables.get(FIGHTER))
        .unwrap_or_else(|| panic!("the pack has no move table for `{FIGHTER}`"));
    arms::DUEL_MOVES
        .iter()
        .map(|(id, ..)| duel_spec(table.move_by_id(id).unwrap_or_else(|| panic!("the table of `{FIGHTER}` has no `{id}`"))))
        .collect()
}

/// While the scholar fights on the giant's back, play the move he chose on
/// his body, and draw it.
#[allow(clippy::type_complexity)]
pub fn duel_gnu_ton(
    mut commands: Commands,
    pack: Res<crate::pack::SelectedContentPack>,
    mut scholars: Query<(
        Entity,
        &BossConfig,
        &BodyHealth,
        &ambition_extension_host::BodyRecords,
        &mut ActorMoveset,
        &mut BossAttackIntent,
        Option<&mut MovePlayback>,
        Option<&mut GnuTonDuel>,
        Option<&mut PinnedRow>,
    )>,
) {
    for (entity, config, health, records, mut moveset, mut intent, mut playback, duel, row) in &mut scholars {
        if config.behavior.id != GNU_TON_ID {
            continue;
        }
        let Some(mut duel) = duel else {
            // His strikes are drawn with the fighter's art (his stroke of ink).
            commands.entity(entity).insert((GnuTonDuel::default(), ambition_characters::actor::StrikesAs(FIGHTER.into())));
            continue;
        };
        // His body has the duel moves beside the pattern's.
        // Made when a body has none of them, from the pack, and kept by no
        // one but the body: this system is in the schedule that rewinds.
        if moveset.0.moves.iter().all(|spec| !spec.id.starts_with(PREFIX)) {
            moveset.0.moves.extend(duel_specs(&pack));
        }
        let conductor = records.get(&Conductor::KEY).and_then(|record| Conductor::from_record(record).ok()).filter(|_| health.alive());
        let (fights, chosen) = conductor.as_ref().map_or((false, None), Conductor::duel);
        // A blow launched him: his move is stopped, whatever part of it plays.
        if conductor.as_ref().is_some_and(Conductor::reels) {
            if let Some(pb) = playback.as_mut().filter(|pb| pb.spec.id.starts_with(PREFIX)) {
                cancel_move_playback(&mut commands, entity, pb, MoveEnd::Interrupted);
                playback = None;
            }
        }

        if !fights {
            // A move of the duel that still plays is stopped.
            if let Some(pb) = playback.as_mut().filter(|pb| pb.spec.id.starts_with(PREFIX)) {
                cancel_move_playback(&mut commands, entity, pb, MoveEnd::Interrupted);
            }
            if duel.playing.take().is_some() || duel.started {
                duel.started = false;
                duel.cooling = true;
            }
            // The pattern's move that the duel cut is not played from its
            // middle: wait for its next tell, or for no move.
            if duel.cooling {
                if intent.active_profile.is_some() && intent.telegraph_profile.is_none() {
                    intent.clear();
                } else {
                    duel.cooling = false;
                }
            }
            continue;
        }

        // He fights: the pattern's move is not played.
        duel.cooling = true;
        if let Some(pb) = playback.as_mut().filter(|pb| !pb.spec.id.starts_with(PREFIX)) {
            cancel_move_playback(&mut commands, entity, pb, MoveEnd::Interrupted);
            playback = None;
        }
        // The move that plays is over when it is gone from the body.
        let on_body = playback.as_ref().map(|pb| pb.spec.id.clone());
        if duel.started && on_body.is_none() {
            duel.playing = None;
            duel.started = false;
        }
        match chosen {
            // A new choice, and no move plays: it starts.
            Some((id, turn)) if turn != duel.turn && duel.playing.is_none() => {
                duel.turn = turn;
                duel.playing = Some(format!("{PREFIX}{id}"));
                duel.started = false;
            }
            // He chose no move (a blow landed on him): his move stops if it
            // has not struck yet.
            None => {
                if let Some(pb) = playback.as_mut() {
                    let strikes_at = pb.spec.windows.iter().find(|w| matches!(w.tag, WindowTag::Active)).map_or(0.0, |w| w.start_s);
                    if pb.t < strikes_at {
                        cancel_move_playback(&mut commands, entity, pb, MoveEnd::Interrupted);
                        playback = None;
                    }
                }
                if playback.is_none() {
                    duel.playing = None;
                    duel.started = false;
                }
            }
            _ => {}
        }
        match duel.playing.clone() {
            Some(id) => {
                duel.started |= on_body.as_deref() == Some(id.as_str());
                intent.telegraph_profile = Some(BossAttackProfile::Special(id));
                intent.active_profile = None;
            }
            None => intent.clear(),
        }
        // He is drawn in the move: its row, at its time.
        if let (Some(pb), Some(mut row)) = (playback.as_ref(), row) {
            let mut names: Vec<&str> = vec![pb.spec.clip.clip.as_str()];
            names.extend(pb.spec.clip.fallbacks.iter().map(String::as_str));
            // The rows of the sheet he wears now, until it has the fighter's.
            names.push("point");
            row.pin(&names, pb.t, false);
        }
    }
}
