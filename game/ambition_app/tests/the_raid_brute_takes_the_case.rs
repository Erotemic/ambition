#![cfg(feature = "rl_sim")]
//! Scout fights the workshop raiders. The beaten brute wounds Dennis with a
//! bomb and leaves with the case from the bench.

use ambition_app::rl_sim::AmbitionSim as _;
use ambition_platformer2d::characters::actor::CharacterForm;
use ambition_platformer2d::held_items::ItemCustody;
use ambition_platformer2d::persistence::save::AmbitionGameSave;
use ambition_platformer2d::platformer::sim_id::SimId;
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
    world.resource::<AmbitionGameSave>().data().flag(id)
}

fn a_bomb_flies(world: &mut World) -> bool {
    use ambition_platformer2d::held_items::ReleasedAs;
    let mut items = world.query::<(&SimId, &ReleasedAs)>();
    items.iter(world).any(|(id, _)| *id == SimId::placement("intro_brute_bomb"))
}

fn cases(world: &mut World) -> Vec<(ItemCustody, ambition_platformer2d::engine_core::Vec2)> {
    use ambition_platformer2d::engine_core::BodyKinematics;
    let case = SimId::placement("intro_bench_case");
    world.query::<(&SimId, &ItemCustody, &BodyKinematics)>()
        .iter(world).filter(|(id, _, _)| **id == case)
        .map(|(_, custody, body)| (*custody, body.pos)).collect()
}

struct Watching<'a, 'b> {
    pad: &'a mut Pad<'b>,
    skip: bool,
    skipped: bool,
    thrown_at: Option<i32>,
    after: i32,
    carried: bool,
    warning_ticks: u32,
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
        if !self.skip && self.warning_ticks < 180 {
            use ambition_platformer2d::cutscene::{ActiveCutscene, CutsceneBeat};
            let warning = self.pad.world().resource::<ActiveCutscene>().iter().any(|scene| {
                scene.script_id() == "intro_brute_takes_the_case"
                    && matches!(scene.runtime.current_beat(), Some(CutsceneBeat::Dialogue { speaker, .. }) if speaker == "Dennis")
            });
            if warning {
                self.warning_ticks += 1;
                press.confirm = false;
            }
        }
        self.pad.step(press);
        use ambition_platformer2d::platformer::authored_logic::{AuthoredVerdictLog, CommandId};
        if let Some(verdict) = self.pad.world().get_resource::<AuthoredVerdictLog>()
            .and_then(|log| log.latest_run_of_id(&CommandId::new("workshop", "ship_case")))
        {
            assert!(verdict.refusal().is_none(), "case transfer refused: {verdict}");
        }
        let health = creator_health(self.pad.world_mut());
        if self.thrown_at.is_none() && a_bomb_flies(self.pad.world_mut()) {
            self.thrown_at = health;
        }
        if self.thrown_at.is_some() {
            self.after = self.after.min(health.unwrap_or(i32::MAX));
        }
        if scene.is_some() {
            let all = cases(self.pad.world_mut());
            assert!(all.len() <= 1, "the case has one physical identity: {all:?}");
            if let [(ItemCustody::Held { holder }, _)] = all.as_slice() {
                assert!(self.pad.world().get::<CharacterForm>(*holder).is_some_and(|form| form.id() == "npc_raid_brute"));
                self.carried = true;
            }
        }
    }
}

#[test]
fn the_raid_brute_takes_the_case_and_bombs_the_creator() {
    the_brute_scene_ends_the_raid(false);
}

/// Cancel on the first tick. The shove, wound and case transfer still occur.
#[test]
fn a_skipped_brute_scene_ends_the_raid_as_a_played_one_does() {
    the_brute_scene_ends_the_raid(true);
}

fn the_brute_scene_ends_the_raid(skip: bool) {
    let options = fixed_60hz_room_options("intro_wake_room");
    let options = if skip { options.with_sync_test_rollback_settings(2, 12) } else { options };
    let mut sim = ambition_app::Platformer2dSimHarness::new_with_options(options).expect("the workshop boots");
    sim.step_n(base(), 30);
    assert!(matches!(cases(sim.world_mut()).as_slice(), [(ItemCustody::InWorld, at)] if at.x > 1000.0 && at.y < 320.0), "one case rests on the bench before the raid");
    let refusals = sim.world().resource::<ambition_platformer2d::runtime::cutscene::CutsceneLineRefusals>().0.clone();
    assert!(refusals.is_empty(), "every shipped cutscene line prepares: {refusals:?}");
    let mut pad = Pad::new(&mut sim);
    // The Creator's health as the bomb leaves the brute's hand, and the
    // least of it after.
    let mut watching = Watching { pad: &mut pad, skip, skipped: false, thrown_at: None, after: i32::MAX, carried: false, warning_ticks: 0 };
    if let Err(why) = through_the_raid(&mut watching, RaidStart::Door) {
        let world = watching.world_mut();
        let live = bodies_of(world, "npc_lab_raider");
        let brute = bodies_of(world, "npc_raid_brute");
        let health = creator_health(world);
        panic!("the raid: {why}; raiders {live:?}; brute {brute:?}; Dennis {health:?}; wounded {}, case taken {}, case in transit {}",
            flag(world, "intro_dennis_wounded"), flag(world, "intro_case_taken"), flag(world, "intro_case_in_transit"));
    }
    // The raid is won the tick he goes; the scene sees him gone on its next
    // tick, and the Creator's last words follow.
    for frame in 0..1200 {
        if flag(watching.world(), "intro_brute_scene_seen") {
            break;
        }
        watching.step(Press { confirm: frame % 10 == 0, ..Press::default() });
    }
    let (skipped, thrown_at, after, carried, warning) = (watching.skipped, watching.thrown_at, watching.after, watching.carried, watching.warning_ticks);
    assert_eq!(skipped, skip, "the premise: the scene was skipped exactly when the walker held cancel");
    assert!(flag(sim.world(), "intro_brute_scene_seen"), "the brute's scene played; brutes live and fallen: {:?}", bodies_of(sim.world_mut(), "npc_raid_brute"));
    assert!(carried && flag(sim.world(), "intro_case_taken"), "the brute took the bench case");
    assert_eq!(warning, if skip { 0 } else { 180 }, "normal play waits on the warning");
    assert!(flag(sim.world(), "intro_dennis_wounded"), "the exact bomb damaged Dennis");
    assert!(cases(sim.world_mut()).is_empty(), "the case left the workshop");
    assert_eq!(bodies_of(sim.world_mut(), "npc_lab_raider"), (0, 3), "the three raiders fell");
    assert_eq!(bodies_of(sim.world_mut(), "npc_raid_brute"), (0, 0), "the brute went out with the case rather than fell");
    let thrown_at = thrown_at.expect("the brute threw a bomb");
    assert!(after < thrown_at, "the bomb hurt the Creator: {thrown_at} when it was thrown, {after} after");
    assert!(after > 0, "the bomb wounded him and did not kill him: he falls after his last words");
    sim.rollback_health().expect("the raid remains consistent under rewind");
    if !skip {
        let save = sim.world().resource::<AmbitionGameSave>().data().clone();
        assert!(!save.flag("intro_creator_fragment_seen"), "the save precedes the final words");
        let mut resumed = ambition_app::Platformer2dSimHarness::new_with_options(
            fixed_60hz_room_options("intro_wake_room").with_save(save.clone()),
        ).expect("the workshop boots after departure");
        let mut pad = Pad::new(&mut resumed);
        for frame in 0..240 {
            if flag(pad.world(), "intro_creator_fragment_seen") {
                break;
            }
            pad.step(Press { confirm: frame % 10 == 0, ..Press::default() });
        }
        assert!(flag(pad.world(), "intro_creator_fragment_seen"), "the final words play after loading");
        assert_eq!(creator_health(pad.world_mut()), Some(0));
        assert_eq!(bodies_of(pad.world_mut(), "npc_raid_brute"), (0, 0));
        assert!(cases(pad.world_mut()).is_empty(), "the bench stays empty after loading");

        let siding = "intro_freight_yard";
        let mut restored = ambition_app::Platformer2dSimHarness::new_with_options(fixed_60hz_room_options(siding).with_save(save))
            .expect("the siding boots from the raid save");
        restored.step_n(base(), 35);
        let all = cases(restored.world_mut());
        let [(ItemCustody::InWorld, at)] = all.as_slice() else {
            panic!("the raid save restores one case at the siding: {all:?}");
        };
        restored.teleport_player((at.x, at.y));
        for frame in 0..120 {
            if flag(restored.world(), "intro_creator_notes_recovered") {
                break;
            }
            Pad::new(&mut restored).step(Press { attack: frame % 12 == 0, confirm: frame % 12 == 6, ..Press::default() });
        }
        assert!(flag(restored.world(), "intro_creator_notes_recovered"), "picking up that case starts the recovery");
        assert!(matches!(cases(restored.world_mut()).as_slice(), [(ItemCustody::Held { .. }, _)]));
    }
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
