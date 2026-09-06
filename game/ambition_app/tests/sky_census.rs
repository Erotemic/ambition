//! Diagnostic census for combat/SFX activity in `pirate_sky_lookout`.
//!
//! The current headless harness leaves the targeted pirates inert, so this test
//! is ignored rather than treating zero SFX as a valid scene result. Run it
//! explicitly when diagnosing whether that harness reaches enemy combat.

use crate::common::fixed_60hz_room_sim;
use ambition_platformer2d::sfx::{OwnedSfxMessage, SfxMessage};
use std::collections::HashMap;

#[test]
#[ignore = "instrument: the harness's pirates never fire, so this measures an inert room — see the module doc"]
fn pirate_sky_sfx_census() {
    let mut sim = fixed_60hz_room_sim("pirate_sky_lookout");
    let mut per_tick_worst = 0usize;
    let mut worst_cue = String::new();
    let mut totals: HashMap<String, usize> = HashMap::new();
    let mut ticks_with_audio = 0usize;
    let mut fx_requests = 0usize;
    let mut attack_press_ticks = 0usize;
    let mut attack_presses = 0usize;
    let mut move_ticks = 0usize;
    let mut moves_seen: HashMap<String, usize> = HashMap::new();
    let mut fx_channel_missing = false;

    for tick in 0..1800 {
        // Jump repeatedly and hold up. GO AT THEM. The riders are dormancy-gated on observers
        // (`AwakeNearObservers`), so an idle or stationary player never wakes them and the room
        // measures as silent.
        let action = ambition_app::AgentAction {
            move_x: if (tick / 240) % 2 == 0 { 1.0 } else { -1.0 },
            jump: tick % 90 == 0,
            ..crate::common::base()
        };
        sim.step(action);
        let cues: Vec<String> = {
            let world = sim.world();
            let Some(messages) = world.get_resource::<bevy::prelude::Messages<OwnedSfxMessage>>()
            else {
                continue;
            };
            let mut cursor = messages.get_cursor();
            cursor
                .read(messages)
                .filter_map(|m| match &m.request {
                    SfxMessage::Play { id, .. } => Some(format!("{id}")),
                    _ => None,
                })
                .collect()
        };
        // IS THE INSTRUMENT DEAF? Count the UPSTREAM channel too. If
        // `FxRequest`s are plentiful while `OwnedSfxMessage`s are scarce, the
        // fan-out that turns one into the other is not installed here and the
        // census is measuring its own composition, not the game.
        {
            let world = sim.world();
            if let Some(m) = world
                .get_resource::<bevy::prelude::Messages<ambition_platformer2d::vfx::FxRequest>>()
            {
                let mut c = m.get_cursor();
                fx_requests += c.read(m).count();
            } else {
                fx_channel_missing = true;
            }
        }
        {
            use ambition_platformer2d::characters::control::ActorControl;
            let w = sim.world_mut();
            let mut cq = w.query::<&ActorControl>();
            let world = sim.world();
            let n = cq
                .iter(world)
                .filter(|c| c.0.melee_pressed || c.0.special_pressed || c.0.projectile_pressed)
                .count();
            if n > 0 {
                attack_press_ticks += 1;
                attack_presses += n;
            }
            let w = sim.world_mut();
            let mut mq = w.query::<&ambition_platformer2d::combat::moveset::MovePlayback>();
            let mut any = false;
            for pb in mq.iter(sim.world()) {
                any = true;
                *moves_seen.entry(pb.spec.id.clone()).or_default() += 1;
            }
            if any {
                move_ticks += 1;
            }
        }
        if !cues.is_empty() {
            ticks_with_audio += 1;
        }
        let mut counts: HashMap<&String, usize> = HashMap::new();
        for c in &cues {
            *counts.entry(c).or_default() += 1;
            *totals.entry(c.clone()).or_default() += 1;
        }
        for (cue, n) in counts {
            if n > per_tick_worst {
                per_tick_worst = n;
                worst_cue = cue.clone();
            }
        }
    }
    // Diagnose the population: are the pirates even here, and did anything fire?
    {
        use ambition_platformer2d::combat::actor_tuning::ActorConfig;
        let world = sim.world_mut();
        let mut q = world.query::<&ActorConfig>();
        let world = sim.world();
        let mut brains: HashMap<String, usize> = HashMap::new();
        for cfg in q.iter(world) {
            *brains.entry(format!("{:?}", cfg.brain)).or_default() += 1;
        }
        println!(
            "[sky-census] actors in the room: {}",
            brains.values().sum::<usize>()
        );
        {
            // PARITY WITH THE REAL HOST. The shell host boots with 780
            // systems across visible schedules (printed by the same census in
            // `smash_in_the_host`). If this composition carries materially
            // fewer, every headless test is measuring a smaller game.
            let world = sim.world();
            let schedules = world.resource::<bevy::ecs::schedule::Schedules>();
            let total: usize = schedules.iter().map(|(_, s)| s.systems_len()).sum();
            let n = schedules
                .iter()
                .filter(|(_, s)| s.systems_len() > 0)
                .count();
            println!("[sky-census] HEADLESS schedules: {n} non-empty, {total} systems total");
        }
        {
            use ambition_platformer2d::actors::features::ecs::dormancy::Dormant;
            let w = sim.world_mut();
            let mut dq = w.query::<&Dormant>();
            let n = dq.iter(sim.world()).count();
            println!("[sky-census] DORMANT actors: {n}");
        }
        {
            use ambition_platformer2d::combat::ActorTarget;
            let w = sim.world_mut();
            let mut tq = w.query::<&ActorTarget>();
            let world = sim.world();
            let total = tq.iter(world).count();
            let with_target = tq.iter(world).filter(|t| t.entity.is_some()).count();
            println!("[sky-census] actors with a combat TARGET: {with_target}/{total}");
        }
        {
            // DO THEY ACT? Separate "inert" from "acting silently".
            use ambition_platformer2d::combat::moveset::{ActorMoveset, MovePlayback};
            let w = sim.world_mut();
            let mut mq = w.query::<&ActorMoveset>();
            let with_moveset = mq.iter(sim.world()).count();
            let w = sim.world_mut();
            let mut pq = w.query::<&MovePlayback>();
            let playing = pq.iter(sim.world()).count();
            println!(
                "[sky-census] actors WITH A MOVESET: {with_moveset}; mid-move right now: {playing}"
            );
        }
        {
            use ambition_platformer2d::characters::control::ActorControl;
            let w = sim.world_mut();
            let mut cq = w.query::<&ActorControl>();
            let world = sim.world();
            let n = cq.iter(world).count();
            let pressing = cq
                .iter(world)
                .filter(|c| c.0.melee_pressed || c.0.special_pressed || c.0.projectile_pressed)
                .count();
            println!(
                "[sky-census] actors with a CONTROL frame: {n}; pressing an attack: {pressing}"
            );
        }

        let mut rows: Vec<(&String, &usize)> = brains.iter().collect();
        rows.sort_by(|a, b| b.1.cmp(a.1));
        for (b, n) in rows.iter().take(10) {
            println!("[sky-census]   {n:3} x {b}");
        }
    }
    let mut rows: Vec<(&String, &usize)> = totals.iter().collect();
    rows.sort_by(|a, b| b.1.cmp(a.1));
    println!("[sky-census] ticks with audio: {ticks_with_audio}/1800");
    println!("[sky-census] upstream FxRequests seen: {fx_requests} (channel missing: {fx_channel_missing})");
    println!("[sky-census] ticks where SOMEBODY pressed an attack: {attack_press_ticks}/1800 ({attack_presses} presses)");
    println!("[sky-census] ticks where SOMEBODY was mid-move: {move_ticks}/1800");
    let mut mrows: Vec<(&String, &usize)> = moves_seen.iter().collect();
    mrows.sort_by(|a, b| b.1.cmp(a.1));
    for (m, n) in mrows.iter().take(10) {
        println!("[sky-census]   move-ticks {n:5} x {m}");
    }
    println!("[sky-census] worst single tick: {per_tick_worst} x {worst_cue}");
    for (cue, n) in rows.iter().take(12) {
        println!("[sky-census]   {n:5} x {cue}");
    }
}
