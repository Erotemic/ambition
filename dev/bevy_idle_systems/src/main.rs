//! What a registered system costs when it has nothing to do. See README.md.
use bevy_ecs::prelude::*;
use bevy_ecs::schedule::{MultiThreadedExecutor, SingleThreadedExecutor};
use std::time::Instant;

#[derive(Component)] struct A;
#[derive(Component)] struct B;
#[derive(Resource, Default)] struct Flag(bool);
#[derive(Component)] struct M0;
#[derive(Component)] struct M1;
#[derive(Component)] struct M2;
#[derive(Component)] struct M3;
#[derive(Component)] struct M4;
#[derive(Component)] struct M5;
#[derive(Component)] struct M6;
#[derive(Component)] struct M7;
#[derive(Component)] struct M8;
#[derive(Component)] struct M9;
#[derive(Component)] struct M10;

fn idle(q: Query<(&A, &mut B)>) { for _ in &q {} }
fn gated(q: Query<(&A, &mut B)>) { for _ in &q {} }
fn reader(q: Query<(&A, &B)>) { for _ in &q {} }
fn off(f: Res<Flag>) -> bool { f.0 }

fn bench(n: usize, archetypes: usize, multi: bool, gate: u8) -> f64 {
    let mut world = World::new();
    world.init_resource::<Flag>();
    // Archetypes that never match A: every subset of eleven markers.
    for i in 0..archetypes {
        let mut e = world.spawn(B);
        macro_rules! bit { ($($n:literal $t:ident),*) => { $( if i & (1 << $n) != 0 { e.insert($t); } )* } }
        bit!(0 M0, 1 M1, 2 M2, 3 M3, 4 M4, 5 M5, 6 M6, 7 M7, 8 M8, 9 M9, 10 M10);
    }
    let mut schedule = Schedule::default();
    if multi { schedule.set_executor(MultiThreadedExecutor::new()); } else { schedule.set_executor(SingleThreadedExecutor::new()); }
    for _ in 0..n {
        match gate { 1 => { schedule.add_systems(gated.run_if(off)); } 2 => { schedule.add_systems(reader); } _ => { schedule.add_systems(idle); } }
    }
    for _ in 0..50 { schedule.run(&mut world); }
    let frames = 2000;
    let t = Instant::now();
    for _ in 0..frames { schedule.run(&mut world); }
    t.elapsed().as_secs_f64() * 1e6 / frames as f64
}

fn main() {
    for (multi, name) in [(false, "single"), (true, "multi")] {
        for n in [100usize, 1000] {
            for gate in [0u8, 1, 2] {
                let us = bench(n, 2000, multi, gate);
                let kind = ["idle-writer", "gated", "idle-reader"][gate as usize];
                println!("{name:6} {kind:12} {n:5} systems: {us:9.1} us/frame = {:.3} us/system", us / n as f64);
            }
        }
    }
}
