//! **An authoring format's state is INSTALLED, not owed.**
//!
//! Until 2026-08-16 `PlatformerSessionWorld` — the engine's canonical live
//! session bundle — carried `runtime_rooms: LdtkRuntimeIndex` as a mandatory
//! field. Every RON-authored game therefore constructed an
//! `LdtkRuntimeIndex::default()`, a value whose own doc calls it *"the 'no LDtk
//! world installed' index"*, for a world it would never install. The index is
//! optional session state a FORMAT installs now, so a RON-authored session root
//! carries no such component at all.
//!
//! ⛔ **the two tests below are ONE claim and neither half means anything
//! alone.** "Sanic has no LDtk index" is trivially satisfiable by never
//! inserting the component anywhere — which is precisely what a bad
//! implementation of this change looks like, and it would delete LDtk streaming
//! from the shipped game while turning this file green. So the absence is only
//! asserted beside a POSITIVE observation that the LDtk-authored game does
//! install a real, non-empty index onto the same kind of root, through the same
//! activation road. Both terms are observed, or the pair proves nothing.

use ambition_platformer2d::app::prelude::*;
use ambition_platformer2d::ldtk_map::LdtkRuntimeIndex;
use ambition_platformer2d::platformer::lifecycle::{
    session_world_component, session_world_entity, settle_until_session_world,
    SESSION_SETTLE_FRAMES,
};
use ambition_platformer2d::runtime::demo_fixture::RoomSet;

/// The versus stage — a RON-authored experience, mounted through the public API.
/// It is the same module the Smash consumer-matrix row drives, chosen because it
/// is a RON game that lives in THIS crate: no demo-app dependency, and it
/// activates through the identical provider road the LDtk game uses.
struct VersusModule;

impl GameModule for VersusModule {
    fn manifest(&self) -> ModuleManifest {
        ModuleManifest::new(ambition_app::app::versus::VERSUS_EXPERIENCE)
    }

    fn define(&self, module: &mut ModuleDraft) {
        module
            .experience(ambition_app::app::versus::VERSUS_EXPERIENCE)
            .launcher_route(ambition_app::app::shell_host::AMBITION_LAUNCHER_ROUTE)
            .gameplay_route(ambition_app::app::versus::VERSUS_GAMEPLAY_ROUTE)
            .capability(VersusCapability);
    }
}

#[derive(Clone)]
struct VersusCapability;

impl ambition_platformer2d::bevy::prelude::Plugin for VersusCapability {
    fn build(&self, app: &mut ambition_platformer2d::bevy::prelude::App) {
        ambition_app::app::versus::compose_versus_experience(app);
    }
}

/// **THE POSITIVE TERM: the LDtk-authored game installs a real index.**
///
/// Without this, its sibling below would pass in a build where nothing ever
/// inserts the component — the failure mode that matters most, because it is
/// silent and it removes level streaming from the shipped game.
#[test]
fn the_ldtk_authored_game_installs_a_real_index_onto_its_session_root() {
    let mut app = ambition_platformer2d::bevy::prelude::App::new();
    ambition_platformer2d::runtime::add_headless_foundation(&mut app);
    ambition_app::app::shell_host::compose_ambition_gameplay_host(&mut app);

    settle_until_session_world(&mut app, SESSION_SETTLE_FRAMES)
        .expect("the gameplay host must reach a live session world");

    let index = session_world_component::<LdtkRuntimeIndex>(app.world()).expect(
        "the LDtk-authored game installed no LdtkRuntimeIndex onto its session root. \
         The index is optional session state now, and this is the road that is \
         supposed to install it — an absence here is LDtk streaming silently gone, \
         not a boundary cleanly drawn",
    );

    // ⚠ presence is not enough: a `default()` index inserted unconditionally
    // would satisfy `is_some()` while being exactly the empty value this whole
    // change exists to delete. The active area is the field `from_project` fills
    // and `Default` leaves blank, so it separates a real installation from the
    // placeholder.
    assert!(
        !index.active_area().is_empty(),
        "the installed index names no active area, so it is the empty \
         'no LDtk world installed' placeholder wearing an installation's clothes"
    );
}

/// **THE INVARIANT: a RON-authored session carries no LDtk index at all.**
#[test]
fn a_ron_authored_session_root_carries_no_ldtk_index() {
    let mut app = PlatformerApp::headless()
        .mount(VersusModule)
        .try_build()
        .expect("the versus stage must compose through the public API");

    let mut settled = None;
    for _ in 0..SESSION_SETTLE_FRAMES {
        if let Some(entity) = session_world_entity(app.world()) {
            settled = Some(entity);
            break;
        }
        app.update();
    }
    let root = settled.expect(
        "the RON-authored versus stage reached no session world, so this test \
         never got to the state where the wrong implementation could be caught",
    );

    // ⚠ **the root must be REAL before its emptiness means anything.** A handle
    // to an entity that is not a live session world would report "no LDtk index"
    // for the uninteresting reason that it has no components at all. `RoomSet`
    // is a canonical session-world component every platformer session owns, so
    // observing it is what turns the assertion below into a statement about the
    // boundary rather than about a stale entity id.
    let rooms = session_world_component::<RoomSet>(app.world())
        .expect("the settled session root carries no RoomSet, so it is not a live session world");
    assert!(
        !rooms.active_spec().id.is_empty(),
        "the RON-authored session names no active room"
    );

    assert!(
        app.world().get::<LdtkRuntimeIndex>(root).is_none(),
        "a RON-authored session root carries an LdtkRuntimeIndex. Nothing in this \
         game installs an LDtk world, so the only value it can hold is the empty \
         'no LDtk world installed' placeholder — a format adapter's type back \
         inside the canonical session world, which is the defect this file pins"
    );
}
