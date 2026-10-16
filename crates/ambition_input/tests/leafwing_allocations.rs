//! MEASUREMENT (findings page `docs/planning/engine/performance-and-composition-findings.md`,
//! F13, D13): how many allocations one `InputMap::process_actions` call makes
//! for a seat built as the game builds it, and how many of them are clash
//! handling (`InputMap::handle_clashes`), which resolves nothing under
//! `ClashStrategy::PressAll`.
//!
//! Run: `cargo test -p ambition_input --features input --test leafwing_allocations -- --ignored --nocapture`.
//! Delete with the leafwing fork decision.
#![cfg(feature = "input")]

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::sync::atomic::{AtomicU64, Ordering};

use ambition_input::Platformer2dInputActionMonolith as Action;
use ambition_input::{BindingRecipe, PresetId};
use bevy::prelude::*;
use leafwing_input_manager::input_map::UpdatedActions;
use leafwing_input_manager::plugin::InputManagerPlugin;
use leafwing_input_manager::prelude::ClashStrategy;
use leafwing_input_manager::user_input::updating::CentralInputStore;

struct Counting;

static ALLOCS: AtomicU64 = AtomicU64::new(0);
thread_local! {
    static COUNTING: Cell<bool> = const { Cell::new(false) };
}

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if COUNTING.with(Cell::get) {
            ALLOCS.fetch_add(1, Ordering::Relaxed);
        }
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        if COUNTING.with(Cell::get) {
            ALLOCS.fetch_add(1, Ordering::Relaxed);
        }
        unsafe { System.realloc(ptr, layout, size) }
    }
}

#[global_allocator]
static GLOBAL: Counting = Counting;

fn count<R>(f: impl FnOnce() -> R) -> (u64, R) {
    ALLOCS.store(0, Ordering::Relaxed);
    COUNTING.with(|c| c.set(true));
    let result = f();
    COUNTING.with(|c| c.set(false));
    (ALLOCS.load(Ordering::Relaxed), result)
}

#[test]
#[ignore = "a measurement; run with --ignored --nocapture"]
fn allocations_of_one_seat_update() {
    // The input store as a running game has it: every input kind registered.
    let mut app = App::new();
    app.add_plugins((bevy::input::InputPlugin, InputManagerPlugin::<Action>::default()));
    app.add_plugins(bevy::time::TimePlugin);
    app.update();
    app.update();
    let store = app.world().resource::<CentralInputStore>();

    for preset in [PresetId::ArrowsZxc, PresetId::WasdJkl] {
        let map = BindingRecipe::preset(preset).build();
        let (whole, _) = count(|| map.process_actions(None, store, ClashStrategy::PressAll));
        let (clashes, _) = count(|| {
            let mut updated = UpdatedActions::<Action>::default();
            map.handle_clashes(&mut updated, store, ClashStrategy::PressAll, Entity::PLACEHOLDER);
        });
        let buttons = map.iter_buttonlike().map(|(_, bindings)| bindings.len()).sum::<usize>();
        println!(
            "LEAFWING-ALLOCS preset={preset:?} button_actions={} button_bindings={buttons} \
             process_actions={whole} of_which_handle_clashes={clashes}",
            map.iter_buttonlike().count(),
        );
    }
}
