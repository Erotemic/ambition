//! The robot's blade in Ambition (Jon, 2026-10-12): the base attacks feel as
//! the nail of Hollow Knight does.
//!
//! - A swing roots no one: the robot walks through it at its speed.
//! - The jab and the forward attack are one 1-2 string (the blade goes up,
//!   then comes down). At rest a third strike follows, a thrust; in a walk
//!   the string is the two swings again.
//! - A press after a pause is the first swing again.
//! - The air has the same swings. The strike behind, in the air, turns the
//!   robot about (as Marth's), so the same stick is forward after it.
//!
//! The authority: the move on the player's body (`MovePlayback`), its
//! velocity and its facing, in the game the player plays.

use ambition_app::AmbitionSim;
use ambition_app::{AgentAction, Platformer2dSimHarness, Platformer2dSimHarnessOptions, TimestepMode};
use ambition_platformer2d::combat::moveset::MovePlayback;
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::platformer::markers::PrimaryPlayerOnly;

fn home() -> Platformer2dSimHarness {
    let opts = Platformer2dSimHarnessOptions::default().with_timestep(TimestepMode::fixed_60hz());
    let mut sim = Platformer2dSimHarness::new_with_options(opts).expect("the start room builds headlessly");
    for _ in 0..40 {
        sim.step(AgentAction::default());
    }
    let world = sim.world_mut();
    let mut worn = world.query_filtered::<&ambition_platformer2d::characters::actor::WornCharacter, PrimaryPlayerOnly>();
    let id = worn.iter(world).next().expect("the player wears a character").id().to_string();
    assert_eq!(id, "player_robot_v3", "the premise: the player is robot v3");
    sim
}

fn body(sim: &mut Platformer2dSimHarness) -> ae::BodyKinematics {
    let world = sim.world_mut();
    let mut q = world.query_filtered::<&ae::BodyKinematics, PrimaryPlayerOnly>();
    q.iter(world).next().expect("the player").clone()
}

fn on_ground(sim: &mut Platformer2dSimHarness) -> bool {
    let world = sim.world_mut();
    let mut q = world.query_filtered::<&ae::BodyGroundState, PrimaryPlayerOnly>();
    q.iter(world).next().expect("the player").on_ground
}

/// The move on the player's body: its id, and its instance (a new use of the
/// same move has a new instance).
fn swing(sim: &mut Platformer2dSimHarness) -> Option<(String, u64)> {
    let world = sim.world_mut();
    let mut q = world.query_filtered::<Option<&MovePlayback>, PrimaryPlayerOnly>();
    q.iter(world).next().flatten().map(|pb| (pb.spec.id.to_string(), pb.instance as u64))
}

/// Press attack once each `gap` frames, `presses` times, with `held` as the
/// stick, and say each move that started.
fn string(sim: &mut Platformer2dSimHarness, held: AgentAction, presses: usize, gap: usize) -> Vec<String> {
    let mut started: Vec<(String, u64)> = Vec::new();
    for frame in 0..presses * gap + 6 {
        let press = frame % gap == 0 && frame / gap < presses;
        sim.step(AgentAction { attack: press, attack_held: press, ..held.clone() });
        if let Some(now) = swing(sim) {
            if started.last() != Some(&now) {
                started.push(now);
            }
        }
    }
    started.into_iter().map(|(id, _)| id).collect()
}

fn rest(sim: &mut Platformer2dSimHarness, frames: usize) {
    for _ in 0..frames {
        sim.step(AgentAction::default());
    }
}

#[test]
fn a_swing_does_not_slow_a_walk() {
    let mut sim = home();
    let right = AgentAction { move_x: 1.0, ..Default::default() };
    for _ in 0..45 {
        sim.step(right.clone());
    }
    let speed = body(&mut sim).vel.x;
    assert!(speed > 100.0, "the premise: the robot runs: {speed}");
    sim.step(AgentAction { attack: true, attack_held: true, ..right.clone() });
    // From the frame of the press: a kick back on the press is a slower run.
    let mut slowest = body(&mut sim).vel.x;
    let mut swung = false;
    let mut second = false;
    for frame in 0..34 {
        // A second press: the down swing does not slow him either.
        sim.step(AgentAction { attack: frame == 15, attack_held: frame == 15, ..right.clone() });
        swung |= swing(&mut sim).is_some_and(|(id, _)| id == "blade_rise");
        second |= swing(&mut sim).is_some_and(|(id, _)| id == "blade_fall");
        slowest = slowest.min(body(&mut sim).vel.x);
    }
    assert!(swung, "a press in a run started no swing of the blade");
    assert!(second, "the second press in a run started no down swing");
    assert!(slowest >= speed * 0.999, "the swing slowed the run from {speed} to {slowest}");
}

#[test]
fn at_rest_the_string_is_rise_fall_thrust_and_in_a_walk_it_is_rise_fall_rise() {
    let mut sim = home();
    // 16 frames between presses: 0.27 s, an easy pace.
    assert_eq!(string(&mut sim, AgentAction::default(), 3, 16), ["blade_rise", "blade_fall", "blade_thrust"]);
    rest(&mut sim, 60);
    let right = AgentAction { move_x: 1.0, ..Default::default() };
    for _ in 0..10 {
        sim.step(right.clone());
    }
    assert_eq!(string(&mut sim, right, 4, 16), ["blade_rise", "blade_fall", "blade_rise", "blade_fall"]);
}

#[test]
fn a_slow_second_press_is_still_the_fall_and_a_press_after_a_pause_is_the_rise() {
    let mut sim = home();
    // 27 frames: 0.45 s. The picture of the first swing is over (0.28 s); the
    // time to press again is not (0.5 s).
    assert_eq!(string(&mut sim, AgentAction::default(), 2, 27), ["blade_rise", "blade_fall"]);
    rest(&mut sim, 60);
    // 36 frames: 0.6 s. Each press is the first swing.
    assert_eq!(string(&mut sim, AgentAction::default(), 3, 36), ["blade_rise", "blade_rise", "blade_rise"]);
}

#[test]
fn the_air_has_the_same_swings_and_the_strike_behind_turns_him_about() {
    let mut sim = home();
    sim.step(AgentAction { jump: true, jump_held: true, ..Default::default() });
    for _ in 0..6 {
        sim.step(AgentAction { jump_held: true, ..Default::default() });
    }
    assert!(!on_ground(&mut sim), "the premise: the robot is in the air");
    assert_eq!(string(&mut sim, AgentAction { jump_held: true, ..Default::default() }, 2, 16), ["blade_rise", "blade_fall"]);
    // Down, and up again: the strike behind.
    for _ in 0..240 {
        sim.step(AgentAction::default());
        if on_ground(&mut sim) && swing(&mut sim).is_none() {
            break;
        }
    }
    rest(&mut sim, 10);
    let facing = body(&mut sim).facing;
    let back = AgentAction { move_x: -facing, jump_held: true, ..Default::default() };
    sim.step(AgentAction { jump: true, jump_held: true, ..Default::default() });
    for _ in 0..6 {
        sim.step(AgentAction { jump_held: true, ..Default::default() });
    }
    assert_eq!(body(&mut sim).facing, facing, "the premise: he faces as he did");
    sim.step(AgentAction { attack: true, attack_held: true, ..back.clone() });
    let mut moves: Vec<String> = Vec::new();
    let mut turned_at = None;
    for frame in 0..30 {
        // The same stick, and a second press when the strike is over.
        sim.step(AgentAction { attack: frame == 22, attack_held: frame == 22, ..back.clone() });
        if let Some((id, _)) = swing(&mut sim) {
            if moves.last() != Some(&id) {
                moves.push(id);
            }
        }
        if turned_at.is_none() && body(&mut sim).facing == -facing {
            turned_at = Some(frame);
        }
    }
    assert_eq!(moves.first().map(String::as_str), Some("blade_behind"), "the stick behind him in the air: {moves:?}");
    assert!(turned_at.is_some(), "the strike behind did not turn him about");
    assert_eq!(moves.get(1).map(String::as_str), Some("blade_rise"), "the same stick after the turn is forward: {moves:?}");
}

/// Jon, 2026-10-12: "In the ambition game itself, landing lag should be
/// significantly lowered." The move states its lag; the game charges a share.
#[test]
fn a_landing_in_a_move_charges_a_small_share_of_its_lag() {
    use ambition_platformer2d::actor::BodyCombat;
    let mut sim = home();
    let mut charged: Vec<(f32, f32)> = Vec::new();
    // Start the strike under him at more than one height: in some of them he
    // is in it when he lands.
    for wait in [10, 16, 22, 28, 34] {
        sim.step(AgentAction { jump: true, jump_held: true, ..Default::default() });
        for _ in 0..wait {
            sim.step(AgentAction { jump_held: true, ..Default::default() });
        }
        let down = AgentAction { move_y: 1.0, ..Default::default() };
        sim.step(AgentAction { attack: true, attack_held: true, ..down.clone() });
        let mut authored = None;
        let mut lag = 0.0_f32;
        for _ in 0..240 {
            sim.step(down.clone());
            let world = sim.world_mut();
            let mut q = world.query_filtered::<(Option<&MovePlayback>, &BodyCombat), PrimaryPlayerOnly>();
            let (playback, combat) = q.iter(world).next().expect("the player");
            authored = authored.or(playback.and_then(|pb| pb.spec.landing_lag_s));
            lag = lag.max(combat.landing_lag_timer);
            if on_ground(&mut sim) && swing(&mut sim).is_none() && lag == 0.0 {
                break;
            }
            if lag > 0.0 {
                break;
            }
        }
        if let (Some(authored), true) = (authored, lag > 0.0) {
            charged.push((authored, lag));
        }
        rest(&mut sim, 40);
    }
    assert!(!charged.is_empty(), "no landing was in a move that states a landing lag: the test measured nothing");
    for (authored, lag) in charged {
        assert!(lag <= authored * 0.34, "a landing charged {lag} s of a move's {authored} s");
    }
}

/// Jon, 2026-10-12: "when I'm flying (jets on my feet), they disappear when
/// I slash." The body that flies keeps the row of its flight under a swing,
/// and that row's jets are of the lower body: the swing's row has none.
#[test]
fn a_swing_in_flight_keeps_the_jets() {
    let sprite_content = ambition_platformer2d::sprite_sheet::test_support::shared_sprites();
    use ambition_platformer2d::sim_view::BodyPoseView;
    use ambition_platformer2d::sprite_sheet::character::rigged::RiggedSpriteAsset;
    use ambition_platformer2d::sprite_sheet::character::CharacterAnim;
    fn pose(sim: &mut Platformer2dSimHarness) -> BodyPoseView {
        let world = sim.world_mut();
        let mut q = world
            .query_filtered::<&BodyPoseView, bevy::prelude::With<ambition_platformer2d::platformer::lifecycle::PlayerVisual>>();
        q.iter(world).next().expect("the player's pose view").clone()
    }
    let mut sim = home();
    sim.step(AgentAction { fly_toggle: true, ..Default::default() });
    rest(&mut sim, 20);
    assert_eq!(pose(&mut sim).anim, CharacterAnim::Fly, "the premise: the robot flies");
    sim.step(AgentAction { attack: true, attack_held: true, ..Default::default() });
    rest(&mut sim, 3);
    assert!(swing(&mut sim).is_some_and(|(id, _)| id == "blade_rise"), "the premise: he swings in flight");
    let clip = pose(&mut sim).clip.expect("a swing asks for its row");
    assert_eq!(clip.legs, Some(CharacterAnim::Fly), "the swing in flight asks for no flight under it");

    // The picture: the swing's row with the flight row under it draws jets.
    let flipbook = RiggedSpriteAsset::baked(&sprite_content, "player_robot_v3").expect("the robot's part flipbook is published");
    let jets = |draws: &[ambition_platformer2d::sprite_sheet::character::rigged::PartDraw]| {
        draws.iter().filter(|draw| draw.track.is_some_and(|track| flipbook.tracks[usize::from(track)].contains("jet"))).count()
    };
    let (mut drawn, mut scratch) = (Vec::new(), Vec::new());
    flipbook.tween_into(&clip.clip, 1, 0.0, &mut drawn).expect("the swing's row");
    assert_eq!(jets(&drawn), 0, "the premise: the swing's row draws no jet");
    // The robot's row for `Fly` is `hover` (the sheet's name for it).
    assert!(flipbook.legs_under("hover", 0, 0.0, &mut drawn, &mut scratch), "the flight row did not go under the swing");
    assert!(jets(&drawn) > 0, "the swing in flight draws no jet");
}
