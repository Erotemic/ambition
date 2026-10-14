//! A census of the ECS while Mary O is played: how many archetypes there are,
//! and how long a frame takes, minute by minute. A measurement, not a test.
//!
//!     cargo test -j 6 -p ambition_app --test app_it -- mary_o_archetype_census --ignored --nocapture
//!
//! Why (2026-10-13): on Android, Mary O starts at 40-50 FPS and falls to ~20
//! after a few minutes. Three CPU profiles of one session put 2-4% of all CPU
//! in Bevy's `QueryState::new_archetype`, and its share rose through the
//! session (2.2% -> 2.9% -> 3.8%), with the bit set matching beside it. A
//! query state matches each archetype once, so that cost in steady play means
//! archetypes keep being made, or query states keep being made fresh and
//! match every archetype there is. This runs the game and counts.
//!
//! First result (2026-10-13, this host): 852 archetypes and 1024 entities
//! from frame 600 to frame 7200, ms/frame flat. So the cost is fresh query
//! states, each matched against ~850 archetypes, not archetypes being made.
//!
//! The player runs right and jumps, and each death respawns her, so rooms are
//! built again as in play. The host is the HUD test's (`mary_o_hud_surround`):
//! the simulation, the shell and the presentation that does not draw.

#![cfg(feature = "rl_sim")]

use std::collections::BTreeSet;
use std::time::Instant;

use ambition_platformer2d::input::ControlFrame;

/// Frames in all, and frames between two lines of the census.
const FRAMES: usize = 7200;
const EVERY: usize = 600;

fn archetype_names(world: &bevy::prelude::World, archetype: &bevy::ecs::archetype::Archetype) -> String {
    let mut names: Vec<String> = archetype
        .components()
        .iter()
        .filter_map(|&id| world.components().get_info(id).map(|info| info.name().to_string()))
        .map(|name| name.rsplit("::").next().unwrap_or(&name).to_string())
        .collect();
    names.sort();
    names.join(", ")
}

#[test]
#[ignore = "a measurement; run it by name with --ignored --nocapture"]
fn mary_o_archetype_census() {
    let mut app = crate::mary_o_hud_surround::mary_o_hud_app(bevy::math::Vec2::new(1600.0, 900.0));
    // One simulation step in each update, and the script's controls for the player.
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(std::time::Duration::from_secs_f64(1.0 / 60.0)));
    ambition_platformer2d::scripted_input::drive_the_local_participant(&mut app);
    crate::mary_o_hud_surround::enter_mary_o(&mut app);
    let start = app.world().archetypes().len();
    let mut seen: BTreeSet<usize> = (0..start).collect();
    let mut window = Instant::now();
    println!("archetypes at the start of play: {start}");
    println!("{:>6} {:>10} {:>9} {:>12}", "frame", "archetypes", "entities", "ms/frame");
    let mut late: Vec<String> = Vec::new();
    for frame in 0..FRAMES {
        let mut press = ControlFrame::default();
        press.axis_x = 1.0;
        let jumping = frame % 90 < 30;
        press.jump_pressed = frame % 90 == 0;
        press.jump_held = jumping;
        app.world_mut().resource_mut::<ambition_platformer2d::scripted_input::ScriptedControls>().0 = press;
        app.update();
        if (frame + 1) % EVERY == 0 {
            let world = app.world();
            let ms = window.elapsed().as_secs_f64() * 1000.0 / EVERY as f64;
            println!("{:>6} {:>10} {:>9} {:>12.2}", frame + 1, world.archetypes().len(), world.entities().len(), ms);
            // The archetypes made since the last line, past the first minute.
            for archetype in world.archetypes().iter() {
                let index = archetype.id().index();
                if seen.insert(index) && frame + 1 > 3600 && late.len() < 40 {
                    late.push(format!("  #{index} ({} entities): {}", archetype.len(), archetype_names(world, archetype)));
                }
            }
            window = Instant::now();
        }
    }
    println!("archetypes made after the first minute of play (first 40):");
    for line in &late {
        println!("{line}");
    }
}

/// The SHAPE of the archetypes after a minute of Mary O: how many hold no
/// entity, one, a few, many; how many tables back them; and which components
/// are in the most archetypes (the ones that split the others). A
/// measurement, not a test. For the counts in any game, in play, use the
/// `[census] ecs` row (`AMBITION_PROFILE_CENSUS=1`, `docs/recipes/profiling.md`).
///
/// First result (2026-10-14, this host): 852 archetypes, of which 602 are
/// resources (one per resource type since Bevy 0.19), 210 empty, 620 hold one
/// entity; the UI nodes are the largest scene family (129 archetypes, 80
/// entities).
///
///     cargo test -j 6 -p ambition_app --test app_it -- mary_o_archetype_shape --ignored --nocapture
#[test]
#[ignore = "a measurement; run it by name with --ignored --nocapture"]
fn mary_o_archetype_shape() {
    use std::collections::BTreeMap;
    let mut app = crate::mary_o_hud_surround::mary_o_hud_app(bevy::math::Vec2::new(1600.0, 900.0));
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(std::time::Duration::from_secs_f64(1.0 / 60.0)));
    ambition_platformer2d::scripted_input::drive_the_local_participant(&mut app);
    crate::mary_o_hud_surround::enter_mary_o(&mut app);
    for frame in 0..3600 {
        let mut press = ControlFrame::default();
        press.axis_x = 1.0;
        press.jump_pressed = frame % 90 == 0;
        press.jump_held = frame % 90 < 30;
        app.world_mut().resource_mut::<ambition_platformer2d::scripted_input::ScriptedControls>().0 = press;
        app.update();
    }
    let world = app.world();
    let archetypes: Vec<_> = world.archetypes().iter().collect();
    let mut buckets: BTreeMap<&str, usize> = BTreeMap::new();
    for a in &archetypes {
        let key = match a.len() {
            0 => "0 entities",
            1 => "1 entity",
            2..=9 => "2-9 entities",
            10..=99 => "10-99 entities",
            _ => "100+ entities",
        };
        *buckets.entry(key).or_default() += 1;
    }
    println!("archetypes: {}, tables: {}, entities: {}", archetypes.len(), world.storages().tables.len(), world.entities().len());
    for (k, v) in &buckets {
        println!("  {k:>15}: {v}");
    }
    // Components by the number of archetypes they are in.
    let mut per_component: BTreeMap<String, (usize, usize)> = BTreeMap::new();
    for a in &archetypes {
        for &id in a.components().iter() {
            if let Some(info) = world.components().get_info(id) {
                let name = info.name().to_string();
                let short = name.rsplit("::").next().unwrap_or(&name).to_string();
                let e = per_component.entry(short).or_default();
                e.0 += 1;
                e.1 += a.len() as usize;
            }
        }
    }
    let mut ranked: Vec<_> = per_component.into_iter().collect();
    ranked.sort_by(|a, b| b.1 .0.cmp(&a.1 .0));
    println!("components in the most archetypes (archetypes, entities):");
    for (name, (n, e)) in ranked.iter().take(25) {
        println!("  {n:>4} {e:>6}  {name}");
    }
    // Archetypes by their most telling component: the rarest one in them.
    let rarity: BTreeMap<&str, usize> = ranked.iter().map(|(n, (c, _))| (n.as_str(), *c)).collect();
    let mut families: BTreeMap<String, (usize, usize)> = BTreeMap::new();
    for a in &archetypes {
        let rarest = a
            .components()
            .iter()
            .filter_map(|&id| world.components().get_info(id))
            .map(|info| {
                let name = info.name().to_string();
                name.rsplit("::").next().unwrap_or(&name).to_string()
            })
            .min_by_key(|n| rarity.get(n.as_str()).copied().unwrap_or(0));
        let e = families.entry(rarest.unwrap_or_else(|| "(no components)".into())).or_default();
        e.0 += 1;
        e.1 += a.len() as usize;
    }
    let mut fam: Vec<_> = families.into_iter().collect();
    fam.sort_by(|a, b| b.1 .0.cmp(&a.1 .0));
    println!("archetype families by their rarest component (archetypes, entities), top 25:");
    for (name, (n, e)) in fam.iter().take(25) {
        println!("  {n:>4} {e:>6}  {name}");
    }
}
