//! How many allocations a partition makes, per case of the doc's benchmark
//! list (`docs/planning/engine/portals-a-body-in-two-places.md`, Benchmarks).
//! A measurement, not a check: `cargo test --release -p ambition_platformer2d_core
//! --test partition_allocations -- --ignored --nocapture`. Its own test binary,
//! for the counting allocator.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

use ambition_platformer2d_core::frame::thread::{partition, PieceBudget, PortalLink};
use ambition_platformer2d_core::frame::{MapConvention, PortalAperture, PortalFrame};
use ambition_platformer2d_core::{Aabb, CombatVolume, Vec2};

struct Counting;
static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static GLOBAL: Counting = Counting;

fn ap(x: f32, y: f32, ny: f32) -> PortalAperture {
    PortalAperture { frame: PortalFrame::fixed(Vec2::new(x, y), Vec2::new(0.0, ny)), half_length: 40.0 }
}

fn links(pairs: &[(PortalAperture, PortalAperture)]) -> Vec<PortalLink> {
    pairs.iter().enumerate().flat_map(|(i, pair)| PortalLink::both_ways(i, *pair)).collect()
}

#[test]
#[ignore]
fn partition_allocations() {
    let boxed = |c: Vec2, h: Vec2| CombatVolume::aabb(Aabb::new(c, h));
    let one = links(&[(ap(0.0, 0.0, -1.0), ap(500.0, -200.0, 1.0))]);
    let four: Vec<_> =
        (0..4).map(|i| (ap(i as f32 * 100.0, 0.0, -1.0), ap(i as f32 * 100.0 + 1000.0, -300.0, 1.0))).collect();
    let chain = links(&[(ap(0.0, 0.0, -1.0), ap(500.0, -200.0, 1.0)), (ap(500.0, -100.0, -1.0), ap(900.0, -200.0, 1.0))]);
    let cycle = links(&[(ap(0.0, 0.0, -1.0), ap(0.0, -200.0, 1.0))]);
    let cases: Vec<(&str, CombatVolume, Vec2, Vec<PortalLink>, Option<u32>)> = vec![
        ("no portals", boxed(Vec2::new(0.0, -20.0), Vec2::new(12.0, 18.0)), Vec2::new(0.0, -20.0), Vec::new(), None),
        ("one pair, not reached", boxed(Vec2::new(0.0, -100.0), Vec2::splat(12.0)), Vec2::new(0.0, -100.0), one.clone(), None),
        ("straddling one pair", boxed(Vec2::ZERO, Vec2::splat(12.0)), Vec2::new(0.0, -1.0), one, Some(0)),
        ("a giant across four pairs", boxed(Vec2::new(150.0, 10.0), Vec2::new(200.0, 20.0)), Vec2::new(150.0, -50.0), links(&four), None),
        ("an arm through two pairs", boxed(Vec2::new(0.0, 75.0), Vec2::new(10.0, 75.0)), Vec2::new(0.0, -10.0), chain, None),
        ("a pair facing itself", boxed(Vec2::new(0.0, 1990.0), Vec2::new(5.0, 2000.0)), Vec2::new(0.0, -10.0), cycle, None),
    ];
    for (name, volume, root, links, first) in cases {
        let before = ALLOCATIONS.load(Ordering::Relaxed);
        let p = std::hint::black_box(partition(&volume, root, &links, first, PieceBudget::COMPOSED, MapConvention::Reflection));
        let made = ALLOCATIONS.load(Ordering::Relaxed) - before;
        println!("{name:<28} {made:>4} allocations ({} pieces)", p.pieces.len());
    }
}
