//! An authoring format's state is INSTALLED, not owed.
//!
//! Every RON-authored game therefore constructed an `LdtkRuntimeIndex::default()`, a value whose
//! own doc calls it *"the 'no LDtk world installed' index"*, for a world it would never install.
//! The index is optional session state a FORMAT installs now, so a RON-authored session root
//! carries no such component at all.
//!
//! the two tests below are ONE claim and neither half means anything
//! alone. "Sanic has no LDtk index" is trivially satisfiable by never
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

/// THE POSITIVE TERM: the LDtk-authored game installs a real index.
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

    // presence is not enough: a `default()` index inserted unconditionally
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

/// THE INVARIANT: a RON-authored session carries no LDtk index at all.
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

    // the root must be REAL before its emptiness means anything. A handle
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

/// The registry row the LDtk runtime index is registered under.
const LDTK_ROLLBACK_ROW: &str = "root.ldtk_runtime_index";

/// Does this composition's snapshot schema contain the LDtk world's row?
fn schema_names_the_ldtk_row(world: &ambition_platformer2d::bevy::prelude::World) -> bool {
    world
        .get_resource::<ambition_platformer2d::rollback::RollbackRegistry>()
        .expect(
            "no RollbackRegistry in this composition, so the schema question was never \
             actually asked — the engine group installs one in every game",
        )
        .descriptors()
        .any(|entry| entry.name == LDTK_ROLLBACK_ROW)
}

/// THE POSITIVE TERM for the half: the LDtk-authored game installs the spine and carries the
/// format's row in its wire format.
///
/// without this, its sibling below passes in a build where
/// `LdtkWorldPlugin` is added by nobody — which deletes level streaming AND the
/// index's rollback participation from the shipped game while turning the pair
/// green.
#[test]
fn the_ldtk_authored_game_installs_the_spine_and_registers_its_rollback_row() {
    let mut app = ambition_platformer2d::bevy::prelude::App::new();
    ambition_platformer2d::runtime::add_headless_foundation(&mut app);
    ambition_app::app::shell_host::compose_ambition_gameplay_host(&mut app);

    assert!(
        app.world()
            .get_resource::<ambition_platformer2d::ldtk_map::LdtkRuntimeSpineIndex>()
            .is_some(),
        "the LDtk-authored game installed no LDtk runtime spine. This is the road \
         that is supposed to add `LdtkWorldPlugin`, and an absence here is the \
         format's index-rebuild chain silently gone, not a boundary cleanly drawn"
    );
    assert!(
        schema_names_the_ldtk_row(app.world()),
        "the LDtk-authored game's snapshot schema does not name '{LDTK_ROLLBACK_ROW}'. \
         The index is rewound state in THIS game — a missing registration is a \
         desync, not a tidier boundary"
    );
}

/// THE INVARIANT: a RON-authored composition never mentions LDtk.
///
/// a plugin that is added and then declines to run is still added: its six index resources are
/// still initialized, its systems are still in the schedule graph, and its component is still a
/// row in the wire format. This pins the other half — the engine group does not install an
/// authoring format at all.
#[test]
fn a_ron_authored_composition_installs_no_ldtk_spine_and_no_ldtk_rollback_row() {
    let app = PlatformerApp::headless()
        .mount(VersusModule)
        .try_build()
        .expect("the versus stage must compose through the public API");

    // the composition must be REAL before its emptiness means anything. An
    // App that failed to assemble the engine would report "no LDtk spine" for
    // the uninteresting reason that it has nothing in it. The registry is
    // installed by `AmbitionRollbackSchemaPlugin`, the FIRST plugin in the
    // engine group, and a populated one is proof the group ran.
    let registry_rows = app
        .world()
        .get_resource::<ambition_platformer2d::rollback::RollbackRegistry>()
        .expect("the RON-authored composition installed no RollbackRegistry, so it never \
                 reached the state where a stray LDtk registration could be observed")
        .descriptors()
        .count();
    assert!(
        registry_rows > 100,
        "the RON-authored composition recorded only {registry_rows} rollback rows, so the \
         engine group did not assemble and this test proves nothing about LDtk"
    );

    assert!(
        app.world()
            .get_resource::<ambition_platformer2d::ldtk_map::LdtkRuntimeSpineIndex>()
            .is_none(),
        "a RON-authored composition initialized the LDtk runtime spine's index. The spine \
         is a format's, and this game has no LDtk world — the resource can only ever hold \
         the empty value the rebuild chain declines to fill"
    );
    assert!(
        !schema_names_the_ldtk_row(app.world()),
        "a RON-authored composition's snapshot schema names '{LDTK_ROLLBACK_ROW}'. Nothing \
         in this game installs an LDtk world, so the format's component is a row in a \
         wire format that can never contain it — and it changes the fingerprint two \
         peers must agree on"
    );
}
