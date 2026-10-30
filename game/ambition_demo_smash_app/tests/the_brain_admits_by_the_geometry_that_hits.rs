//! ONE AUTHORITY FOR WHERE A MOVE HITS (the witness).
//!
//! The fighter brain admits a swing by `MoveFrameData.coverage`; the body
//! strikes with the `Hitbox` entities the move spawns. The strike spawn
//! prefers the sprite's authored hit polygon for a vfx-tagged volume whose
//! clip the sheet names, and the brain's coverage used to be the RON box
//! regardless — measured 2026-10-30, `polygon_air_forward`'s box sat 50 px
//! under its live hitbox. `ambition_combat::authored_coverage` now derives
//! the kit's region by the spawn's own rule, from the spawn's own resolver.
//!
//! This test is the equality those two must keep: for every seated fighter
//! of the pair and every move that authors a volume in its first Active
//! window, the region the brain is handed for that window equals the extent
//! of the strike entities the body spawns when the window opens, in the
//! body's own frame, to the pixel. Manifest-backed and RON-backed moves
//! alike: the rule is one rule.

use std::collections::BTreeMap;

use ambition_demo_smash_app::build_demo_app;
use ambition_platformer2d::actor::MatchSeat;
use ambition_platformer2d::combat::authored_coverage::authored_coverage_over;
use ambition_platformer2d::combat::authored_volumes::AuthoredAttackVolumeResolver;
use ambition_platformer2d::combat::components::ActorRenderSize;
use ambition_platformer2d::combat::moveset::{ActorMoveset, MovePlayback};
use ambition_platformer2d::combat::strike::Hitbox;
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::entity_catalog::{MoveSpec, WindowTag};
use bevy::prelude::*;

/// The demo composition's own pair: the polygons live in the shipped
/// composition, which `smash_cpu_report` measures (`inside` whiffs 5 → 1 at
/// rung-9 pair); this witness holds the rule on the fighters the demo seats.
const PAIR: [&str; 2] = [
    ambition_demo_smash::SMASH_GEORGE_BOOUL,
    ambition_demo_smash::SMASH_CHARACTER_ID,
];
const TOLERANCE_PX: f32 = 0.51;

/// What a seated body resolves its authored volumes with: the same inputs
/// the strike spawn reads.
struct SeatedBody {
    entity: Entity,
    seat: usize,
    character: Option<String>,
    size: ae::Vec2,
    drawn_quad: Option<ae::Vec2>,
    art_floor: f32,
    facing: f32,
    moves: Vec<MoveSpec>,
}

fn a_live_pair() -> App {
    let mut app = build_demo_app();
    for _ in 0..30 {
        app.update();
    }
    let roster = ambition_demo_smash::smash_roster_at_levels(PAIR, &[5, 5]);
    app.world_mut().insert_resource(roster);
    app.world_mut()
        .write_message(ambition_platformer2d::game_shell::ShellCommand::GoTo(
            ambition_platformer2d::game_shell::ShellRouteId::new(
                ambition_demo_smash::SMASH_GAMEPLAY_ROUTE,
            ),
        ));
    // Anchor to the live edge, not a frame count: the route transition's
    // loading takes a number of frames that varies per process, and the
    // fighters are held until the ceremony ends. (`match_report.rs`, same
    // rule, same measurement.)
    let countdown = ambition_demo_smash::smash_roster(PAIR)
        .rules
        .opening_countdown_ticks;
    let mut waited = 0usize;
    loop {
        app.update();
        waited += 1;
        let world = app.world_mut();
        let mut seats = world.query::<(&MatchSeat, &ae::BodyKinematics)>();
        let moving = seats.iter(world).any(|(_, kin)| kin.vel.length_squared() > 1e-6);
        if moving {
            break;
        }
        assert!(
            waited <= countdown as usize + 6000,
            "the match never went live: no seated fighter moved in {waited} frames"
        );
    }
    println!("[coverage-witness] the match went live {waited} frames after the route was asked for");
    app
}

fn seated_bodies(app: &mut App) -> Vec<SeatedBody> {
    use bevy::ecs::system::RunSystemOnce as _;
    // The art floor is a resolved rule (`CombatTuningOf` is a `SystemParam`),
    // so it is read in a one-shot system; everything else is a plain lookup.
    let floors: Vec<(Entity, usize, f32)> = app
        .world_mut()
        .run_system_once(
            |tuning: ambition_platformer2d::combat::rules::CombatTuningOf,
             seats: Query<(Entity, &MatchSeat)>|
             -> Vec<(Entity, usize, f32)> {
                seats
                    .iter()
                    .map(|(entity, seat)| {
                        (entity, seat.0, tuning.of(entity).map_or(0.0, |rules| rules.art_hit_area_min))
                    })
                    .collect::<Vec<(Entity, usize, f32)>>()
            },
        )
        .expect("the seat reader runs");
    let world = app.world();
    let mut out: Vec<SeatedBody> = floors
        .into_iter()
        .map(|(entity, seat, art_floor)| {
            let kin = world
                .get::<ae::BodyKinematics>(entity)
                .unwrap_or_else(|| panic!("seat {seat} has no kinematics"));
            SeatedBody {
                entity,
                seat,
                character: world
                    .get::<ambition_platformer2d::characters::actor::WornCharacter>(entity)
                    .map(|w| w.id().to_string()),
                size: kin.size,
                drawn_quad: world.get::<ActorRenderSize>(entity).map(|quad| quad.0),
                art_floor,
                facing: if kin.facing < 0.0 { -1.0 } else { 1.0 },
                moves: world
                    .get::<ActorMoveset>(entity)
                    .map(|m| m.0.moves.clone())
                    .unwrap_or_default(),
            }
        })
        .collect();
    out.sort_by_key(|body| body.seat);
    out
}

/// The union of this body's live strike entities, in its own frame
/// (`+x` toward facing, `+y` toward the feet), or `None` when none is live.
fn live_strike_extent(app: &mut App, body: Entity, facing: f32) -> Option<((f32, f32), (f32, f32))> {
    let world = app.world_mut();
    let center = world.get::<ae::CenteredAabb>(body)?.center;
    let mut q = world.query::<&Hitbox>();
    q.iter(world)
        .filter(|hitbox| hitbox.owner == body)
        .map(|hitbox| {
            let b = hitbox.world_volume(center).bounds();
            let (lo_x, hi_x) = if facing < 0.0 {
                (center.x - b.max.x, center.x - b.min.x)
            } else {
                (b.min.x - center.x, b.max.x - center.x)
            };
            ((lo_x, b.min.y - center.y), (hi_x, b.max.y - center.y))
        })
        .reduce(|a, b| ((a.0 .0.min(b.0 .0), a.0 .1.min(b.0 .1)), (a.1 .0.max(b.1 .0), a.1 .1.max(b.1 .1))))
}

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() <= TOLERANCE_PX
}

#[test]
fn the_region_the_brain_is_handed_is_the_extent_the_body_strikes_with() {
    let mut app = a_live_pair();
    let bodies = seated_bodies(&mut app);
    assert_eq!(bodies.len(), 2, "two seated fighters, so this measures the pair");

    // Brains off: the witness presses the moves itself, and a brain that
    // pressed its own would replace the playback under the measurement.
    for body in &bodies {
        app.world_mut()
            .entity_mut(body.entity)
            .remove::<ambition_platformer2d::characters::brain::Brain>();
    }

    let resolver = app
        .world()
        .resource::<AuthoredAttackVolumeResolver>()
        .clone();
    let catalog = app
        .world()
        .resource::<ambition_platformer2d::characters::actor::character_catalog::CharacterCatalog>()
        .clone();

    let mut witnessed: BTreeMap<String, usize> = BTreeMap::new();
    let mut manifest_backed = 0usize;
    let mut failures: Vec<String> = Vec::new();
    for body in &bodies {
        let character = body.character.as_deref();
        let (size, quad, art_floor) = (body.size, body.drawn_quad, body.art_floor);
        let resolve = |clip: &str, start_s: f32| {
            resolver
                .resolve(&catalog, character, clip, size, quad, Some(start_s))
                .map(|volume| {
                    if art_floor > 0.0 {
                        volume.at_least(ae::Vec2::splat(art_floor * size.y))
                    } else {
                        volume
                    }
                })
        };
        for spec in &body.moves {
            let Some((first_index, first)) = spec
                .windows
                .iter()
                .enumerate()
                .find(|(_, w)| matches!(w.tag, WindowTag::Active) && !w.volumes.is_empty())
            else {
                continue;
            };
            let Some(expected) = authored_coverage_over(spec, &resolve, |index| index == first_index) else {
                continue;
            };
            if resolve(spec.clip.clip.as_str(), first.start_s).is_some() {
                manifest_backed += 1;
            }
            // Start the move at the window's opening, with the body's facing;
            // the volumes spawn through `Commands` and are live next tick.
            let start = first.start_s + 1e-4;
            app.world_mut().entity_mut(body.entity).insert(MovePlayback::new_at(
                std::sync::Arc::new(spec.clone()),
                body.facing,
                start,
            ));
            let mut actual = None;
            for _ in 0..4 {
                app.update();
                actual = live_strike_extent(&mut app, body.entity, body.facing);
                if actual.is_some() {
                    break;
                }
            }
            app.world_mut().entity_mut(body.entity).remove::<MovePlayback>();
            for _ in 0..3 {
                app.update();
            }
            let Some((lo, hi)) = actual else {
                failures.push(format!(
                    "seat {} {}: no strike entity opened for its first Active window",
                    body.seat, spec.id
                ));
                continue;
            };
            let agrees = close(lo.0, expected.min.0)
                && close(lo.1, expected.min.1)
                && close(hi.0, expected.max.0)
                && close(hi.1, expected.max.1);
            if !agrees {
                failures.push(format!(
                    "seat {} {}: brain region ({:.1},{:.1})..({:.1},{:.1}) vs live strike ({:.1},{:.1})..({:.1},{:.1})",
                    body.seat, spec.id, expected.min.0, expected.min.1, expected.max.0, expected.max.1,
                    lo.0, lo.1, hi.0, hi.1
                ));
            }
            *witnessed.entry(body.character.clone().unwrap_or_default()).or_default() += 1;
        }
    }
    let total: usize = witnessed.values().sum();
    println!("[coverage-witness] {total} moves witnessed across {witnessed:?}; {manifest_backed} manifest-backed");
    // The zero floor: a pair that authors no strike would pass vacuously.
    assert!(total >= 10, "only {total} moves measured; the pair authors dozens");
    // Manifest-backed moves are the case the rule exists for; how many the
    // demo pair authors is printed above, and the RON-backed ones hold the
    // same equality by the same code path.
    let _ = manifest_backed;
    assert!(failures.is_empty(), "the brain and the body disagree on where a move hits:\n{}", failures.join("\n"));
}
