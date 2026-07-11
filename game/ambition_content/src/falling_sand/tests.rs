//! Unit tests for the parent module, extracted from an inline
//! `#[cfg(test)] mod tests` (test-organization campaign, 2026-07-10). Pure move:
//! same test names + logic, now an adjacent child module with private access via
//! `use super::*;`.

use super::*;

fn room(w: f32, h: f32) -> ae::World {
    ae::World::new("fs-test", ae::Vec2::new(w, h), ae::Vec2::ZERO, Vec::new())
}

fn at(world: &ae::World, x: f32, y: f32) -> IVec2 {
    world_to_particle_grid(world, ae::Vec2::new(x, y))
}

/// **FS1's conservation law.** The projection is a read-model over the grid:
/// it may neither create matter nor lose it. Every particle the pass walks
/// lands in exactly one ledger column, and every counted particle lands in
/// exactly one tile bucket. `tally_particles` `debug_assert`s the second half
/// on every real frame; this pins the first.
#[test]
fn every_particle_lands_in_exactly_one_ledger_column() {
    let w = room(256.0, 256.0);
    let mut scratch = ProjectionScratch::default();

    let particles = vec![
        (at(&w, 10.0, 10.0), TYPE_SAND),
        (at(&w, 11.0, 10.0), TYPE_SAND),
        (at(&w, 40.0, 40.0), TYPE_WATER),
        (at(&w, 200.0, 200.0), TYPE_OIL),
        // A wall particle: geometry, not matter. Named, not silently dropped.
        (at(&w, 12.0, 10.0), TYPE_WALL),
        // Off the map entirely.
        (IVec2::new(100_000, 100_000), TYPE_SAND),
    ];
    let n = particles.len();

    let ledger = tally_particles(&w, particles.into_iter(), &mut scratch);
    assert_eq!(ledger.total(), n, "no particle escaped the ledger");
    assert_eq!(ledger.sand, 2);
    assert_eq!(ledger.water, 1);
    assert_eq!(ledger.oil, 1);
    assert_eq!(
        ledger.unmodelled, 1,
        "the wall is geometry, and is named so"
    );
    assert_eq!(ledger.outside_world, 1);

    for kind in [MaterialKind::Sand, MaterialKind::Water, MaterialKind::Oil] {
        assert_eq!(
            scratch.bucketed(kind),
            ledger.counted(kind),
            "{kind:?}: buckets and ledger disagree — a particle was counted twice \
             or lost between the query and the tile map"
        );
    }
}

/// Two particles of the same material in the same tile are two particles, not
/// one tile. The bucket is a COUNT, and the thresholds read it.
#[test]
fn particles_accumulate_within_a_tile_rather_than_collapsing() {
    let w = room(256.0, 256.0);
    let mut scratch = ProjectionScratch::default();
    let same_tile: Vec<(IVec2, &str)> = (0..7)
        .map(|i| (at(&w, 20.0 + i as f32, 20.0), TYPE_SAND))
        .collect();
    let ledger = tally_particles(&w, same_tile.into_iter(), &mut scratch);
    assert_eq!(ledger.sand, 7);
    assert_eq!(scratch.sand_tiles.len(), 1, "one tile");
    assert_eq!(scratch.bucketed(MaterialKind::Sand), 7, "seven particles");
}

/// **Single owner, per tile.** A tile dense enough to be sand is a solid; it
/// must not ALSO become a water region, or the player would swim inside a
/// block. Sand claims first and liquid yields — and the visual agrees with the
/// collision, so what you see is what you stand on.
#[test]
fn a_tile_dense_in_both_sand_and_water_is_owned_by_sand_alone() {
    let mut scratch = ProjectionScratch::default();
    scratch.sand_tiles.insert((3, 3), SAND_THRESHOLD + 4);
    scratch.water_tiles.insert((3, 3), LIQUID_THRESHOLD + 4);
    scratch.water_tiles.insert((9, 9), LIQUID_THRESHOLD + 4);

    let mut blocks = Vec::new();
    project_sand(&mut blocks, &mut scratch);
    assert_eq!(blocks.len(), 1, "the shared tile is a solid");
    assert!(scratch.dense_sand.contains(&(3, 3)));

    let mut water = Vec::new();
    let mut added = 0;
    project_liquid(
        &mut water,
        &mut scratch,
        &mut added,
        MaterialKind::Water,
        ae::WaterKind::Clear,
        falling_water_spec(),
    );
    assert_eq!(water.len(), 1, "only the tile sand did NOT claim");
    assert_eq!(
        scratch.desired_visuals.get(&(3, 3)),
        Some(&MaterialKind::Sand),
        "the visual agrees with the collision: you stand on what you see"
    );
    assert_eq!(
        scratch.desired_visuals.get(&(9, 9)),
        Some(&MaterialKind::Water)
    );
}

/// Below the density threshold a tile is neither solid nor swimmable — the
/// matter is still IN the grid, it is simply too thin to project. Conservation
/// lives in the grid, not in the overlay.
#[test]
fn thin_matter_projects_nothing_but_is_not_lost() {
    let mut scratch = ProjectionScratch::default();
    scratch.sand_tiles.insert((1, 1), SAND_THRESHOLD - 1);
    scratch.water_tiles.insert((2, 2), LIQUID_THRESHOLD - 1);

    let mut blocks = Vec::new();
    project_sand(&mut blocks, &mut scratch);
    assert!(blocks.is_empty());

    let mut water = Vec::new();
    let mut added = 0;
    project_liquid(
        &mut water,
        &mut scratch,
        &mut added,
        MaterialKind::Water,
        ae::WaterKind::Clear,
        falling_water_spec(),
    );
    assert!(water.is_empty());
    assert_eq!(scratch.bucketed(MaterialKind::Sand), SAND_THRESHOLD - 1);
}

/// The switch→spout wiring, as a table. `mixed` opens three mouths, and the
/// order is fixed so the emit pass is deterministic (ADR 0023).
#[test]
fn the_switch_state_selects_a_deterministic_set_of_spout_mouths() {
    let none = FallingSandSpoutState::default();
    assert!(open_spouts(&none).is_empty());

    let sand_only = FallingSandSpoutState {
        sand: true,
        ..Default::default()
    };
    let mouths = open_spouts(&sand_only);
    assert_eq!(mouths.len(), 1);
    assert_eq!(mouths[0].particle_type, TYPE_SAND);
    assert_eq!(mouths[0].width, SOLO_SPOUT_WIDTH);

    let all = FallingSandSpoutState {
        sand: true,
        water: true,
        oil: true,
        mixed: true,
    };
    let types: Vec<&str> = open_spouts(&all).iter().map(|m| m.particle_type).collect();
    assert_eq!(
        types,
        [
            TYPE_SAND, TYPE_WATER, TYPE_OIL, // the three solo mouths
            TYPE_SAND, TYPE_WATER, TYPE_OIL, // then mixed's three, narrower
        ]
    );
    assert!(open_spouts(&all)[3..]
        .iter()
        .all(|m| m.width == MIXED_SPOUT_WIDTH));
}

/// **The bug FS1 exists to kill, pinned at the definition.** Matter had a
/// second home: `FallingSandStreamParticle`, an Ambition-side sprite that fell
/// on its own gravity, ignored every block in the room, and despawned at an
/// invented `world.size.y - 64` floor — so it poured straight through the
/// platforms the real particles were pooling on. Its absence is the invariant.
///
/// The check is on the DEFINITIONS, not on mentions of the names: the doc
/// comments above deliberately say those names out loud so the next reader
/// knows what was removed and why, and an occurrence-counting lint would fight
/// its own explanation. A lint that cannot survive its own documentation is
/// the wrong lint.
#[test]
fn the_grid_is_the_only_owner_of_matter() {
    let source = include_str!("../falling_sand.rs");
    assert!(
        source.contains("SpawnParticleSignal"),
        "the one entry point for matter"
    );
    for banned_definition in banned_definitions() {
        assert!(
            !source.contains(&banned_definition),
            "`{banned_definition}` is back — a second, geometry-ignoring \
             representation of matter. falling-sand.md §1: a particle exists in \
             exactly one place, the grid. If this room needs more visual \
             feedback, draw the GRID harder; do not spawn a parallel fleet that \
             falls on its own physics."
        );
    }
}

/// The needles, ASSEMBLED AT RUNTIME so they never appear as literals in this
/// file. Spelling them out would make the guard find its own test and fail
/// forever — the same self-reference trap the `ControlFrame` lint's near-miss
/// tests were written for.
fn banned_definitions() -> Vec<String> {
    let stream = format!("{}{}", "FallingSandStream", "Particle");
    vec![
        format!("struct {stream}"),
        format!("fn {}{}", "spawn_stream_", "particles"),
        format!("fn {}{}", "animate_falling_sand_stream_", "particles"),
    ]
}

/// **Poison test.** The guard must be able to go red; a lint that cannot fail
/// is worse than none (ADR 0023's rule, applied here). Feeds it a source that
/// DOES contain a reintroduced definition and checks it is seen.
#[test]
fn the_single_owner_guard_can_detect_a_reintroduced_representation() {
    let reintroduced = format!(
        "{} {{ world_pos: Vec2, vel: Vec2 }}",
        banned_definitions()[0]
    );
    let hits = banned_definitions()
        .iter()
        .filter(|needle| reintroduced.contains(needle.as_str()))
        .count();
    assert_eq!(hits, 1, "the guard sees a real reintroduction");

    // ...and it does NOT fire on the module as it stands, which is the other
    // half of "can fail": a lint that always fires is also useless.
    let source = include_str!("../falling_sand.rs");
    assert!(banned_definitions()
        .iter()
        .all(|needle| !source.contains(needle.as_str())));
}
