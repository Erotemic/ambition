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
use ambition_content_schemas::rules::GnuTonDuelRules;
use ambition_entity_catalog::{MoveEventKind, MoveSpec, VolumeShape, WindowTag};
use ambition_platformer2d::sprite_sheet::character::PinnedRow;

/// The fighter whose table he fights with.
pub const FIGHTER: &str = "npc_gnu_ton";
/// A duel move's id on his body: the table's id with this before it, so that
/// it is none of the pattern's moves.
const PREFIX: &str = "gnu_duel_";

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

/// A move of the fighter's table, made for the main game and for his body, by
/// the pack's `gnu_ton_duel` (`data/ambition_rules.ron`).
fn duel_spec(spec: &MoveSpec, tuning: &GnuTonDuelRules) -> MoveSpec {
    let mut spec = spec.clone();
    spec.id = format!("{PREFIX}{}", spec.id);
    let events = spec.events.len();
    spec.events
        .retain(|event| !matches!(&event.kind, MoveEventKind::Vfx { effect, .. } if effect.starts_with(CALLED_FIST)));
    if spec.events.len() != events {
        for volume in spec.windows.iter_mut().flat_map(|window| &mut window.volumes) {
            if let VolumeShape::Rect { offset, half_extents } = &mut volume.shape {
                (*offset, *half_extents) = (tuning.book.offset, tuning.book.half_extents);
            }
            // The fist was the picture of the blow; the book's is his stroke.
            volume.vfx = Some(ambition_entity_catalog::authoring::SLASH_ARC_VFX.to_string());
        }
    }
    for window in &mut spec.windows {
        for volume in &mut window.volumes {
            let strike = if volume.damage >= tuning.heavy_damage { tuning.heavy } else { tuning.light };
            volume.damage = strike.hearts;
            volume.knockback = strike.launch;
            volume.knockback_growth = None;
            if let VolumeShape::Rect { offset, half_extents } = &mut volume.shape {
                offset.0 *= tuning.reach;
                half_extents.0 *= tuning.reach;
                if offset.1 <= tuning.overhead {
                    offset.1 *= tuning.overhead_rise;
                    half_extents.1 *= tuning.overhead_grow;
                } else {
                    offset.1 = tuning.at_the_player;
                    half_extents.1 = (half_extents.1 * tuning.reach).max(tuning.least_half_height);
                }
            }
        }
    }
    spec
}

/// The duel moves, made from the fighter's table in the pack this App plays.
fn duel_specs(pack: &crate::pack::SelectedContentPack, tuning: &GnuTonDuelRules) -> Vec<MoveSpec> {
    let table = ambition_characters::moveset_content_schema::lowered_movesets(pack.get())
        .and_then(|tables| tables.get(FIGHTER))
        .unwrap_or_else(|| panic!("the pack has no move table for `{FIGHTER}`"));
    arms::DUEL_MOVES
        .iter()
        .map(|(id, ..)| duel_spec(table.move_by_id(id).unwrap_or_else(|| panic!("the table of `{FIGHTER}` has no `{id}`")), tuning))
        .collect()
}

/// While the scholar fights on the giant's back, play the move he chose on
/// his body, and draw it.
#[allow(clippy::type_complexity)]
pub fn duel_gnu_ton(
    mut commands: Commands,
    pack: Res<crate::pack::SelectedContentPack>,
    tuning: Res<crate::tuning::AmbitionTuning>,
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
            moveset.0.moves.extend(duel_specs(&pack, &tuning.0.gnu_ton_duel));
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
                // The duel plays the move's first beat: no authored lengths.
                *intent = BossAttackIntent {
                    telegraph_profile: Some(BossAttackProfile::Special(id)),
                    ..Default::default()
                };
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

#[cfg(test)]
mod tests {
    use super::*;

    /// The duel makes a fighter's move by the pack's `gnu_ton_duel`: an edit
    /// to a heavy move's hearts in `data/ambition_rules.ron` is what his heavy
    /// moves deal, with no Rust build. The light moves are the control.
    #[test]
    fn the_duel_makes_his_moves_by_the_pack_file() {
        const HEAVY: &str = "heavy: (hearts: 2, launch: 1.7),";
        let damages = |edit: Option<&'static str>| {
            let pack = crate::pack::compile_pack_with(|path, text| match edit {
                Some(to) if path == "data/ambition_rules.ron" => {
                    assert_eq!(text.matches(HEAVY).count(), 1, "the edit anchor");
                    text.replace(HEAVY, to)
                }
                _ => text,
            })
            .unwrap_or_else(|failure| panic!("{}", failure.render()));
            let tuning = ambition_content_schemas::rules::lowered_ambition_rules(&pack).expect("the pack states its tuning");
            let specs = duel_specs(&crate::pack::SelectedContentPack(std::sync::Arc::new(pack)), &tuning.gnu_ton_duel);
            let mut damages: Vec<i32> =
                specs.iter().flat_map(|spec| &spec.windows).flat_map(|window| &window.volumes).map(|volume| volume.damage).collect();
            damages.sort();
            damages.dedup();
            damages
        };
        assert_eq!(damages(None), vec![1, 2], "the shipped file: light moves deal 1 heart, heavy ones 2");
        assert_eq!(damages(Some("heavy: (hearts: 5, launch: 1.7),")), vec![1, 5], "the edited file is what his heavy moves deal");
    }

    /// A move that calls the fist of his giant strikes with the pack's `book`
    /// box, at the duel's `reach`: an edit to the box in
    /// `data/ambition_rules.ron` is the blow, with no Rust build.
    #[test]
    fn the_book_is_the_pack_files_box() {
        const BOOK: &str = "book: (offset: (42.0, -4.0), half_extents: (30.0, 24.0)),";
        const RISE: &str = "overhead_rise: 1.6,";
        let widths = |edit: Option<(&'static str, &'static str)>| {
            let pack = crate::pack::compile_pack_with(|path, text| match edit {
                Some((from, to)) if path == "data/ambition_rules.ron" => {
                    assert_eq!(text.matches(from).count(), 1, "the edit anchor");
                    text.replace(from, to)
                }
                _ => text,
            })
            .unwrap_or_else(|failure| panic!("{}", failure.render()));
            let tuning = ambition_content_schemas::rules::lowered_ambition_rules(&pack).expect("the pack states its tuning");
            let specs = duel_specs(&crate::pack::SelectedContentPack(std::sync::Arc::new(pack)), &tuning.gnu_ton_duel);
            let boxes = specs
                .iter()
                .flat_map(|spec| &spec.windows)
                .flat_map(|window| &window.volumes)
                .filter_map(|volume| match volume.shape {
                    VolumeShape::Rect { offset, half_extents } => Some((offset, half_extents.0)),
                    _ => None,
                })
                .collect::<Vec<_>>();
            (boxes, tuning.gnu_ton_duel.reach)
        };
        let near = |(a, b): (f32, f32), (c, d): (f32, f32)| (a - c).abs() < 1e-3 && (b - d).abs() < 1e-3;
        let (shipped, reach) = widths(None);
        let (edited, _) = widths(Some((BOOK, "book: (offset: (50.0, -4.0), half_extents: (36.0, 24.0)),")));
        let flat = |boxes: Vec<((f32, f32), f32)>| boxes.into_iter().map(|(offset, half)| (offset.0, half)).collect::<Vec<_>>();
        let (lifted, _) = widths(Some((RISE, "overhead_rise: 2.0,")));
        // A blow over his head rises by the file's `overhead_rise`.
        let rose = shipped
            .iter()
            .zip(&lifted)
            .filter(|((before, _), (after, _))| (after.1 - before.1 * 2.0 / 1.6).abs() < 1e-3 && before.1 != 0.0)
            .count();
        assert!(rose > 0, "no blow over his head rose by the edited `overhead_rise`");
        let (shipped, edited) = (flat(shipped), flat(edited));
        assert_eq!(shipped.len(), edited.len(), "the edit changed which volumes the duel makes");
        let mut books = 0;
        for (before, after) in shipped.iter().zip(&edited) {
            if near(*before, (42.0 * reach, 30.0 * reach)) {
                books += 1;
                assert!(near(*after, (50.0 * reach, 36.0 * reach)), "a book blow became {after:?}, not the edited box at reach {reach}");
            } else {
                assert!(near(*before, *after), "a volume that is not the book moved from {before:?} to {after:?}");
            }
        }
        assert!(books > 0, "no duel move strikes with the book, so this test proves nothing");
    }
}
