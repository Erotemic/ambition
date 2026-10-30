//! The Hall's giant (`npc_giant_gnu_whole`) is not a mount and nobody rides
//! it, but its body is the giant of the fight: where its art has the trim, a
//! body stands on it (the level part of each horn, each shoulder, and the
//! hump of its back).

use ambition_platformer2d::characters::actor::WornCharacter;
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::mount::BackLedge;
use ambition_platformer2d::platformer::markers::PrimaryPlayerOnly;
use bevy::prelude::*;

use crate::common::{base, fixed_60hz_room_sim};

const GIANT: &str = "npc_giant_gnu_whole";

fn giant(sim: &mut ambition_app::Platformer2dSimHarness) -> (ae::BodyKinematics, BackLedge, bool) {
    let world = sim.world_mut();
    let mut q = world.query::<(&WornCharacter, &ae::BodyKinematics, Option<&BackLedge>, Has<ae::Unmirrored>)>();
    let (_, kin, back, unmirrored) =
        q.iter(world).find(|(worn, ..)| worn.id() == GIANT).unwrap_or_else(|| panic!("the hall stages `{GIANT}`"));
    let back = back.unwrap_or_else(|| panic!("`{GIANT}` has the ledges of the giant's body"));
    (kin.clone(), back.clone(), unmirrored)
}

fn place_player(sim: &mut ambition_app::Platformer2dSimHarness, at: ae::Vec2) {
    let world = sim.world_mut();
    let mut q = world.query_filtered::<(ae::BodyClusterQueryData, &mut ambition_platformer2d::actor::MotionModel), PrimaryPlayerOnly>();
    let (mut clusters, mut model) = q.iter_mut(world).next().expect("the player");
    let mut clusters = clusters.as_clusters_mut();
    ae::movement::transit_body(&mut model, &mut clusters, at, ae::movement::TransitVelocity::Zero);
}

fn player(sim: &mut ambition_app::Platformer2dSimHarness) -> ae::BodyKinematics {
    let world = sim.world_mut();
    let mut q = world.query_filtered::<&ae::BodyKinematics, PrimaryPlayerOnly>();
    q.iter(world).next().expect("the player").clone()
}

/// The highest point of `line` under the span `left..right`.
fn top_under(line: &[ae::Vec2], left: f32, right: f32) -> Option<f32> {
    line.windows(2)
        .filter_map(|w| {
            let (a, b) = (w[0], w[1]);
            let (u0, u1) = (a.x.max(left), b.x.min(right));
            (u1 > u0).then(|| {
                let at = |x: f32| a.y + (b.y - a.y) * (x - a.x) / (b.x - a.x).max(1e-3);
                at(u0).min(at(u1))
            })
        })
        .reduce(f32::min)
}

/// Where a body stands on the giant now: its horns, its shoulders and the
/// hump of its back, each as it is drawn this tick (they bob with the art).
fn standing_lines(sim: &mut ambition_app::Platformer2dSimHarness) -> Vec<Vec<ae::Vec2>> {
    let (kin, back, unmirrored) = giant(sim);
    let ledges: Vec<ae::Aabb> = back.ground(&kin, unmirrored).collect();
    assert_eq!(ledges.len(), 2, "the giant has two horns");
    let shoulders: Vec<[ae::Vec2; 2]> = back.drawn_lines(&kin, unmirrored).map(|(ends, ..)| ends).collect();
    assert_eq!(shoulders.len(), 2, "and two shoulders, drawn in the row it rests in");
    let slope = back.slope_line(&kin, unmirrored).expect("and the hump of its back");
    let mut lines: Vec<Vec<ae::Vec2>> =
        ledges.iter().map(|l| vec![ae::Vec2::new(l.min.x, l.min.y), ae::Vec2::new(l.max.x, l.min.y)]).collect();
    lines.extend(shoulders.iter().map(|s| s.to_vec()));
    lines.push(slope);
    lines
}

#[test]
fn a_body_stands_on_the_halls_giant_where_its_art_has_the_trim() {
    let mut sim = fixed_60hz_room_sim("hall_of_characters");
    sim.step_n(base(), 30);
    let lines = standing_lines(&mut sim);
    let slope = lines[4].clone();
    let spots = lines[..4]
        .iter()
        .map(|line| (line[0] + line[1]) * 0.5)
        .chain(slope.windows(2).map(|w| (w[0] + w[1]) * 0.5))
        // Dropped from a little over it: from higher, a body put on the
        // hump under the end of a shoulder is in reach of that end, and hangs
        // from it (`ledge_grab`) as the hump bobs a pixel under it.
        .map(|p| p - ae::Vec2::new(0.0, 30.0));
    for spot in spots {
        place_player(&mut sim, spot);
        sim.step_n(base(), 20);
        let body = player(&mut sim);
        let feet = body.pos.y + body.size.y * 0.5;
        let (left, right) = (body.pos.x - body.size.x * 0.5, body.pos.x + body.size.x * 0.5);
        let lines = standing_lines(&mut sim);
        assert!(
            lines.iter().any(|line| top_under(line, left, right).is_some_and(|top| (feet - top).abs() < 2.0)),
            "a body put at {spot:?} stands on the Hall's giant: feet at {feet}, at x {}, moving {:?}",
            body.pos.x,
            body.vel,
        );
    }
}
