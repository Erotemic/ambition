//! A mite's death row leads into what it becomes (Jon, 2026-11-12): the
//! exploding mite's row is its fuse and the blast comes at the row's end, not
//! at the kill; the dividing mite's row is its division and its two halves
//! stand where its lobes are when the row ends. In each case the mite is not
//! drawn after its row.

use crate::common::{base, fixed_60hz_room_options};
use ambition_app::{AmbitionSim as _, Platformer2dSimHarness};
use ambition_platformer2d::characters::actor::{BodyHealth, CharacterForm};
use ambition_platformer2d::combat::components::DeathBeat;
use ambition_platformer2d::engine_core::BodyKinematics;
use bevy::prelude::*;

/// The sandbox room with two exploding mites and a dividing mite.
const ROOM: &str = "volatile_cache";

fn mites(sim: &mut Platformer2dSimHarness, id: &str) -> Vec<(Entity, BodyKinematics, bool, Option<DeathBeat>)> {
    let world = sim.world_mut();
    world
        .query::<(Entity, &CharacterForm, &BodyHealth, &BodyKinematics, Option<&DeathBeat>)>()
        .iter(world)
        .filter(|(_, worn, ..)| worn.id() == id)
        .map(|(entity, _, health, kin, beat)| (entity, kin.clone(), health.alive(), beat.copied()))
        .collect()
}

fn blasts(sim: &mut Platformer2dSimHarness) -> usize {
    let world = sim.world_mut();
    world.query::<&Name>().iter(world).filter(|name| name.as_str() == "Exploding mite blast").count()
}

/// A lethal hit on `body`, as a strike delivers one.
fn kill(sim: &mut Platformer2dSimHarness, body: Entity, kin: &BodyKinematics) {
    use ambition_platformer2d::combat::events::{HitEvent, HitMode, HitSource, HitTarget};
    sim.world_mut().write_message(HitEvent {
        contact: None,
        pieces: Vec::new(),
        strike_sfx: None,
        volume: kin.aabb().into(),
        damage: 99,
        source: HitSource::Contact,
        attacker: None,
        room: None,
        target: HitTarget::Body(body),
        mode: HitMode::Knockback,
        knockback: None,
        ignored_targets: Vec::new(),
        attacker_move_instance: None,
    });
}

fn room() -> Platformer2dSimHarness {
    let mut sim = Platformer2dSimHarness::new_with_options(fixed_60hz_room_options(ROOM))
        .unwrap_or_else(|e| panic!("`{ROOM}` builds: {e}"));
    sim.step(base());
    sim
}

#[test]
fn an_exploding_mite_goes_off_when_its_fuse_row_ends() {
    let mut sim = room();
    let (mite, kin, _, _) = mites(&mut sim, "npc_exploding_mite").into_iter().next().expect("an exploding mite");
    kill(&mut sim, mite, &kin);
    let mut blew_at = None;
    for tick in 0..120 {
        sim.step(base());
        let beat = mites(&mut sim, "npc_exploding_mite")
            .into_iter()
            .find(|(entity, ..)| *entity == mite)
            .and_then(|(.., beat)| beat)
            .expect("the dead mite keeps its beat");
        if blasts(&mut sim) > 0 {
            blew_at = Some(tick);
            assert!(beat.row_ended(), "the blast came before the fuse row ended: {beat:?}");
            assert!(!beat.visible(false), "the mite is still drawn after it went off: {beat:?}");
            break;
        }
        assert!(beat.visible(false), "the mite left the view before it went off: {beat:?}");
    }
    let blew_at = blew_at.expect("the mite went off");
    assert!(blew_at > 30, "the mite went off at the kill (tick {blew_at}), not after its fuse row");
}

#[test]
fn a_living_exploding_mite_does_not_go_off() {
    let mut sim = room();
    for _ in 0..400 {
        sim.step(base());
        assert_eq!(blasts(&mut sim), 0, "a mite nobody killed went off");
    }
}

#[test]
fn a_dividing_mite_becomes_its_two_halves_when_its_row_ends() {
    let mut sim = room();
    let (mite, kin, _, _) = mites(&mut sim, "npc_dividing_mite").into_iter().next().expect("a dividing mite");
    kill(&mut sim, mite, &kin);
    for _ in 0..120 {
        sim.step(base());
        let halves = mites(&mut sim, "npc_dividing_mite_half");
        let (_, parent, _, beat) = mites(&mut sim, "npc_dividing_mite")
            .into_iter()
            .find(|(entity, ..)| *entity == mite)
            .expect("the dead mite stays until its room goes");
        let beat = beat.expect("a dividing mite has a death beat");
        if halves.is_empty() {
            assert!(beat.visible(false), "the mite left the view before it divided: {beat:?}");
            continue;
        }
        assert!(beat.row_ended(), "the halves came before the division row ended: {beat:?}");
        assert!(!beat.visible(false), "the mite is still drawn beside its halves: {beat:?}");
        assert_eq!(halves.len(), 2);
        let quarter = parent.size().x * 0.25;
        let mut offsets: Vec<f32> = halves.iter().map(|(_, half, ..)| half.pos.x - parent.pos.x).collect();
        offsets.sort_by(f32::total_cmp);
        for (offset, side) in offsets.iter().zip([-1.0, 1.0]) {
            assert!((offset - side * quarter).abs() < 2.0, "a half is not where its lobe was: {offsets:?}, quarter {quarter}");
        }
        for (_, half, alive, _) in &halves {
            assert!(*alive);
            let feet = |kin: &BodyKinematics| kin.pos.y + kin.size().y * 0.5;
            assert!((feet(half) - feet(&parent)).abs() < 1.0, "a half does not stand where the mite stood");
        }
        return;
    }
    panic!("the dividing mite never divided");
}
