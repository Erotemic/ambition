#![cfg(feature = "rl_sim")]
//! The end of level 1's raid (story sequences in content,
//! `data/cutscenes/intro.ron`): when the three raiders are down, a brute comes
//! in at the barn door. He cannot be killed there (`beaten_at`), and when he
//! is beaten a cutscene that starts on that condition (`starts_when:
//! actor.beaten npc_raid_brute`) throws everyone back (`actor.shove`), walks
//! him to the bench, throws a bomb at the Creator (`actor.throw`), puts the
//! case in his hand (`actor.hold`) and sends him out through the lane edge
//! (`actor.leave_through`). His going wins the raid. The player fights with
//! its own inputs.

use ambition_app::rl_sim::AmbitionSim as _;
use bevy::prelude::World;

use crate::common::walk::{through_the_raid, walk_through, Pad, Press, RaidStart, Walker as _};
use crate::common::{base, fixed_60hz_room_options};

/// Live and fallen bodies of `character`.
fn bodies_of(world: &mut World, character: &str) -> (usize, usize) {
    let mut bodies = world.query::<(
        &ambition_platformer2d::characters::actor::CharacterForm,
        Option<&ambition_platformer2d::characters::actor::BodyHealth>,
    )>();
    let mut live = 0;
    let mut fallen = 0;
    for (form, health) in bodies.iter(world) {
        if form.id() != character {
            continue;
        }
        if health.is_none_or(|health| health.health.current > 0) {
            live += 1;
        } else {
            fallen += 1;
        }
    }
    (live, fallen)
}

fn flag(world: &World, id: &str) -> bool {
    world
        .get_resource::<ambition_platformer2d::persistence::save::AmbitionGameSave>()
        .is_some_and(|save| save.data().flag(id))
}

/// Is a thrown bomb in the world?
fn a_bomb_flies(world: &mut World) -> bool {
    let mut items = world.query::<&ambition_platformer2d::held_items::GroundItem>();
    items.iter(world).any(|item| item.spec.id == "bomb")
}

/// A walker that watches the Creator's health from the tick a bomb is first
/// in the world: the scene plays inside the raid's walk. With `skip`, it holds
/// cancel while the brute's scene plays, as a player skipping it does.
struct Watching<'a, 'b> {
    pad: &'a mut Pad<'b>,
    skip: bool,
    skipped: bool,
    thrown_at: Option<i32>,
    after: i32,
}

/// The brute's scene, if it plays: whether its presentation is skipped.
fn brute_scene(world: &World) -> Option<bool> {
    world
        .resource::<ambition_platformer2d::cutscene::ActiveCutscene>()
        .iter()
        .find(|playing| playing.runtime.script.id == "intro_brute_takes_the_case")
        .map(|playing| playing.runtime.skipping)
}

impl crate::common::walk::Walker for Watching<'_, '_> {
    fn world(&self) -> &World {
        self.pad.world()
    }
    fn world_mut(&mut self) -> &mut World {
        self.pad.world_mut()
    }
    fn step(&mut self, mut press: Press) {
        let scene = brute_scene(self.pad.world());
        self.skipped |= scene == Some(true);
        press.cancel = self.skip && scene == Some(false);
        self.pad.step(press);
        let health = creator_health(self.pad.world_mut());
        if self.thrown_at.is_none() && a_bomb_flies(self.pad.world_mut()) {
            self.thrown_at = health;
        }
        if self.thrown_at.is_some() {
            self.after = self.after.min(health.unwrap_or(i32::MAX));
        }
    }
}

#[test]
fn the_raid_brute_takes_the_case_and_bombs_the_creator() {
    the_brute_scene_ends_the_raid(false);
}

/// STORY SEQUENCES SS8: skipping the brute's scene skips its show, not its
/// story. The skip holds cancel from the scene's first tick, before the
/// shove, the walk, the throw and the departure; the scene still waits on
/// the Creator reaching the bench, the bomb bursting and the brute going,
/// and its outcome is normal play's.
#[test]
fn a_skipped_brute_scene_ends_the_raid_as_a_played_one_does() {
    the_brute_scene_ends_the_raid(true);
}

fn the_brute_scene_ends_the_raid(skip: bool) {
    let mut sim = ambition_app::Platformer2dSimHarness::new_with_options(fixed_60hz_room_options("intro_wake_room"))
        .expect("the workshop boots");
    sim.step_n(base(), 30);
    let refusals = sim.world().resource::<ambition_platformer2d::runtime::cutscene::CutsceneLineRefusals>().0.clone();
    assert!(refusals.is_empty(), "every shipped cutscene line prepares: {refusals:?}");
    let mut pad = Pad::new(&mut sim);
    // The Creator's health as the bomb leaves the brute's hand, and the
    // least of it after.
    let mut watching = Watching { pad: &mut pad, skip, skipped: false, thrown_at: None, after: i32::MAX };
    through_the_raid(&mut watching, RaidStart::Door).unwrap_or_else(|why| panic!("the raid: {why}"));
    // The raid is won the tick he goes; the scene sees him gone on its next
    // tick, and the Creator's last words follow.
    for frame in 0..1200 {
        if flag(watching.world(), "intro_brute_scene_seen") {
            break;
        }
        watching.step(Press { confirm: frame % 10 == 0, ..Press::default() });
    }
    let (skipped, thrown_at, after) = (watching.skipped, watching.thrown_at, watching.after);
    assert_eq!(skipped, skip, "the premise: the scene was skipped exactly when the walker held cancel");
    assert!(flag(sim.world(), "intro_brute_scene_seen"), "the brute's scene played; brutes live and fallen: {:?}", bodies_of(sim.world_mut(), "npc_raid_brute"));
    assert!(flag(sim.world(), "intro_case_taken"), "the yard gate's flag is set");
    assert_eq!(bodies_of(sim.world_mut(), "npc_lab_raider"), (0, 3), "the three raiders fell");
    assert_eq!(bodies_of(sim.world_mut(), "npc_raid_brute"), (0, 0), "the brute went out with the case rather than fell");
    let thrown_at = thrown_at.expect("the brute threw a bomb");
    assert!(after < thrown_at, "the bomb hurt the Creator: {thrown_at} when it was thrown, {after} after");
    assert!(after > 0, "the bomb wounded him and did not kill him: he falls after his last words");
}

/// The Creator's health, in the live workshop.
fn creator_health(world: &mut World) -> Option<i32> {
    let mut bodies = world.query::<(
        &ambition_platformer2d::characters::actor::CharacterForm,
        &ambition_platformer2d::characters::actor::BodyHealth,
    )>();
    bodies.iter(world).find(|(form, _)| form.id() == "npc_creator").map(|(_, health)| health.current())
}

/// STORY SEQUENCES SS3, lasting presence: the Creator dies in a cutscene
/// (its `Fall` beat), and when Scout comes back into the workshop he lies
/// where he fell. A `Fall` records the death as a hit's would.
#[test]
fn the_creator_lies_where_he_fell_when_scout_comes_back() {
    let mut sim = ambition_app::Platformer2dSimHarness::new_with_options(fixed_60hz_room_options("intro_wake_room"))
        .expect("the workshop boots");
    sim.step_n(base(), 30);
    let mut pad = Pad::new(&mut sim);
    through_the_raid(&mut pad, RaidStart::Door).unwrap_or_else(|why| panic!("the raid: {why}"));
    for frame in 0..2400 {
        if flag(pad.world(), "intro_creator_fragment_seen") {
            break;
        }
        pad.step(Press { confirm: frame % 10 == 0, ..Press::default() });
    }
    assert!(flag(pad.world(), "intro_creator_fragment_seen"), "his last words played");
    assert_eq!(creator_health(pad.world_mut()), Some(0), "he fell in the scene");
    walk_through(&mut pad, "intro_hobford_lane").unwrap_or_else(|why| panic!("out to the lane: {why}"));
    walk_through(&mut pad, "intro_wake_room").unwrap_or_else(|why| panic!("back into the workshop: {why}"));
    for _ in 0..30 {
        pad.step(Press::default());
    }
    assert_eq!(creator_health(pad.world_mut()), Some(0), "the workshop is built again with him where he fell");
}
