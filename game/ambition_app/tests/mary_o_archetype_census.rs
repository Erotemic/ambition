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
