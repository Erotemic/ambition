#![allow(dead_code)]

//! Shared fixtures for `ambition_app` integration tests.
//!
//! Keep this intentionally small: integration tests should still read like
//! end-to-end scripts, but the neutral `AgentAction` and fixed-60Hz sim setup are
//! common enough that copying them into every test obscures the scenario logic.

use ambition_app::rl_sim::TimestepMode;
use ambition_app::AmbitionSim;
use ambition_app::{AgentAction, Platformer2dSimHarness, Platformer2dSimHarnessOptions};
use ambition_platformer2d::platformer::sim_id::SimId;
use bevy::prelude::Entity;

/// A fully-neutral action; build scenario inputs with struct update:
/// `AgentAction { move_x: 1.0, ..base() }`.
pub fn base() -> AgentAction {
    AgentAction {
        move_x: 0.0,
        move_y: 0.0,
        left_pressed: false,
        right_pressed: false,
        up_pressed: false,
        down_pressed: false,
        jump: false,
        jump_held: false,
        jump_released: false,
        dash: false,
        attack: false,
        attack_held: false,
        attack_released: false,
        attack_strength: Default::default(),
        // A scripted action steers its attack with the movement axis; only a
        // C-stick replay says otherwise.
        attack_from_aim_stick: false,
        attack_aim: (0.0, 0.0),
        special: false,
        special_held: false,
        blink: false,
        blink_held: false,
        blink_released: false,
        pogo: false,
        interact: false,
        interact_held: false,
        projectile: false,
        projectile_held: false,
        projectile_released: false,
        fly_toggle: false,
        reset: false,
        start: false,
        modifier: false,
        modifier_held: false,
        aim_x: 0.0,
        aim_y: 0.0,
    }
}

/// Hold full right for tests that only need a simple locomotion input.
pub fn hold_right() -> AgentAction {
    AgentAction {
        move_x: 1.0,
        ..base()
    }
}

pub fn fixed_60hz_options() -> Platformer2dSimHarnessOptions {
    Platformer2dSimHarnessOptions::default().with_timestep(TimestepMode::fixed_60hz())
}

pub fn fixed_60hz_room_options(room: &str) -> Platformer2dSimHarnessOptions {
    fixed_60hz_options().with_required_start_room(room)
}

pub fn fixed_60hz_sim() -> Platformer2dSimHarness {
    Platformer2dSimHarness::new_with_options(fixed_60hz_options())
        .expect("Platformer2dSimHarness::new")
}

pub fn fixed_60hz_room_sim(room: &str) -> Platformer2dSimHarness {
    Platformer2dSimHarness::new_with_options(fixed_60hz_room_options(room))
        .expect("Platformer2dSimHarness::new")
}

#[cfg(feature = "portal")]
use ambition_platformer2d::portal::PlacedPortal;

/// Return all currently-live authored portal pairs, after any link resolution.
///
/// `portal_lab` now authors explicit `link` ids. After the app steps once,
/// linked portals are assigned generated `Indexed` channels, so tests should not
/// assume the old Purple/Yellow channels remain on the live `PlacedPortal`s.
#[cfg(feature = "portal")]
pub fn authored_portal_pairs(
    sim: &mut Platformer2dSimHarness,
) -> Vec<(PlacedPortal, PlacedPortal)> {
    let mut q = sim.world_mut().query::<&PlacedPortal>();
    let world = sim.world();
    let mut portals: Vec<PlacedPortal> = q
        .iter(world)
        .filter(|p| !p.channel.is_gun_pair())
        .cloned()
        .collect();
    portals.sort_by(|a, b| {
        a.pos
            .x
            .total_cmp(&b.pos.x)
            .then(a.pos.y.total_cmp(&b.pos.y))
            .then(a.channel.name().cmp(&b.channel.name()))
    });

    let mut pairs = Vec::new();
    for entry in &portals {
        if let Some(exit) = portals
            .iter()
            .find(|candidate| candidate.channel == entry.channel.partner())
        {
            pairs.push((entry.clone(), exit.clone()));
        }
    }
    pairs
}

/// First live authored pair in deterministic left-to-right/top-to-bottom order.
#[cfg(feature = "portal")]
pub fn first_authored_portal_pair(
    sim: &mut Platformer2dSimHarness,
) -> (PlacedPortal, PlacedPortal) {
    authored_portal_pairs(sim)
        .into_iter()
        .next()
        .expect("room has a linked authored portal pair")
}

/// First floor-to-floor authored pair, used by tests that must exercise a floor
/// carve instead of a wall/ceiling portal.
#[cfg(feature = "portal")]
pub fn first_floor_authored_portal_pair(
    sim: &mut Platformer2dSimHarness,
) -> (PlacedPortal, PlacedPortal) {
    authored_portal_pairs(sim)
        .into_iter()
        .find(|(entry, exit)| entry.normal.y < -0.5 && exit.normal.y < -0.5)
        .expect("room has a linked floor-to-floor authored portal pair")
}

/// Drive `vertical_shaft`'s authored enemy, and return the body and the
/// identity its room minted it under.
///
/// It lives here rather than in one of them because `carried_item_crosses_rooms` and
/// `a_save_remembers_where_you_left_things` are siblings, not a hierarchy.
///
/// the possession is asserted here, once. A test whose setup silently
/// failed to possess anything measures a body nobody is driving, and every
/// assertion about custody below would pass for the wrong reason.
pub fn possess_the_authored_enemy(sim: &mut Platformer2dSimHarness) -> (Entity, SimId) {
    use ambition_platformer2d::actors::control::possession::PossessionState;
    use ambition_platformer2d::characters::brain::Brain;
    use ambition_platformer2d::engine_core::BodyKinematics;

    for _ in 0..30 {
        sim.step(base());
    }
    let (actor, id) = {
        let world = sim.world_mut();
        let mut q = world.query::<(Entity, &SimId, &Brain, &BodyKinematics)>();
        q.iter(world)
            .find(|(_, id, _, _)| id.as_str().starts_with("placement:EnemySpawn"))
            .map(|(e, id, _, _)| (e, id.clone()))
            .expect("'vertical_shaft' authors an enemy with a placement identity")
    };
    for i in 0..900 {
        if let Some(here) = sim
            .world()
            .get::<BodyKinematics>(actor)
            .map(|k| (k.pos.x, k.pos.y))
        {
            sim.teleport_player(here);
        }
        sim.step(AgentAction {
            move_y: 1.0,
            interact: i == 0,
            interact_held: true,
            ..base()
        });
        if sim.world_mut().resource::<PossessionState>().possessed == Some(actor) {
            break;
        }
    }
    assert_eq!(
        sim.world_mut().resource::<PossessionState>().possessed,
        Some(actor),
        "setup: nothing below is about a driven body unless one is being driven"
    );
    (actor, id)
}

/// THIS App's resident character pages, judged per App.
///
/// The image-stage ledger is a process-global static and `app_it` runs its
/// tests as threads of one process, so reading the ledger's resident rows
/// directly counts every sibling test's pages too — a "leak" that tracks
/// whoever else happens to be running (measured 2026-09-02: 15→14 on one
/// schedule, 20→19 on another, always a page some other App owned). Asset ids
/// are per-arena indices and collide across Apps by construction, so the id
/// alone cannot attribute a row either.
///
/// ⭐ What IS per App: `Assets<Image>` and the asset server's path for each
/// id. So residency is read from this App's own assets and the ledger is
/// consulted only as a CLASSIFIER — "was this PATH demanded on the
/// `character-sheet` road". A sibling's row can share an index; it cannot make
/// a page resident here that is not.
///
/// ⛔⛤ **AND THE CLASSIFIER WAS KEYED ON THE ID, WHICH IS THE ONE THING THE
/// PARAGRAPH ABOVE SAYS COLLIDES — MEASURED 2026-09-16.** It read
/// `ledger.get(id)` and then dropped the page unless the row's path matched
/// this App's path for that id. A sibling that inserts the same index with a
/// different path OVERWRITES the row, so the mismatch branch discarded THIS
/// App's page: the guard written to keep siblings out was throwing this App's
/// own rows away.
///
/// Instrumented over one 6-arm `hall_transition_cover` run, counting why each
/// image was rejected:
///
/// ```text
/// no-path=22 no-row=0 other-source=76 PATH-MISMATCH=120 kept=29
/// no-path=22 no-row=8 other-source=70 PATH-MISMATCH=9   kept=138
/// ```
///
/// ⇒ One arm kept 29 of 149 and its neighbour kept 138, decided by whoever else
/// was running. `two_round_trips_through_the_gallery_return_the_same_working_set`
/// compares two samples of that number, which is how it failed a full lane on
/// *"70 → 71 pages"* while passing alone at 149 three times byte-identically.
///
/// ⭐ A path is a property of the ASSET, not of the App that loaded it, so the
/// classifier is built path-keyed: every row the ledger holds, by path. Any App
/// that loaded the path answers the same question about it.
///
/// Returns `(path, megapixels)` for every such page.
pub fn resident_character_pages(app: &bevy::prelude::App) -> Vec<(String, f64)> {
    resident_character_pages_classified_by(app, &character_sheet_paths())
}

/// The classifier: every path any arm in this process has demanded on the
/// `character-sheet` road.
///
/// ⛔⛤ **IT ONLY GROWS, AND AN ARM THAT SAMPLES IT TWICE IS USING TWO
/// INSTRUMENTS.** Keying it on paths instead of asset ids fixed a census that
/// read 29 of 149 pages; it did NOT make the set stable during one arm's
/// window, because a sibling arm demanding a new character-sheet path mid-window
/// adds a row, and any page of that path already resident in THIS App starts
/// counting. `two_round_trips_through_the_gallery_return_the_same_working_set`
/// failed a full workspace run on *"126 → 127 pages"* with its REALIZATION count
/// identical at 258 on both laps and megapixels up by exactly one page — the
/// signature of the instrument moving, not the App retaining.
///
/// ⇒ An arm that COMPARES two samples takes this once and passes it to both
/// readings. `resident_character_pages` re-reads it, which is right for a
/// single-sample caller.
pub fn character_sheet_paths() -> std::collections::BTreeSet<String> {
    let ledger = ambition_platformer2d::sprite_sheet::game_assets::image_stages::ledger();
    ledger
        .rows()
        .filter(|row| row.source == Some("character-sheet"))
        .filter_map(|row| row.path.clone())
        .collect()
}

/// Every image resident in THIS App, by path, with its megapixels — UNCLASSIFIED.
///
/// ⭐ **RESIDENCY IS A PROPERTY OF THIS APP AND CLASSIFICATION IS NOT**, so an
/// arm that compares two moments records this at each moment and classifies ONCE
/// at the end. Then a page classified between the two moments lands in BOTH
/// sets and cancels, while a page that genuinely arrived between them does not —
/// which is the separation a frozen classifier alone cannot make, because the
/// App's own new load is exactly what puts a path in the ledger late.
pub fn resident_image_paths(app: &bevy::prelude::App) -> Vec<(String, f64)> {
    use bevy::prelude::{Assets, Image};
    let world = app.world();
    let images = world.resource::<Assets<Image>>();
    let server = world.resource::<bevy::asset::AssetServer>();
    let mut out = Vec::new();
    for (id, image) in images.iter() {
        let Some(path) = server.get_path(id).map(|path| path.to_string()) else {
            continue;
        };
        let megapixels = f64::from(image.width()) * f64::from(image.height()) / 1.0e6;
        out.push((path, megapixels));
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

/// [`resident_character_pages`] against a classifier the caller holds still.
pub fn resident_character_pages_classified_by(
    app: &bevy::prelude::App,
    character_sheet_paths: &std::collections::BTreeSet<String>,
) -> Vec<(String, f64)> {
    use bevy::prelude::{Assets, Image};
    let world = app.world();
    let images = world.resource::<Assets<Image>>();
    let server = world.resource::<bevy::asset::AssetServer>();
    let mut out = Vec::new();
    for (id, image) in images.iter() {
        let Some(path) = server.get_path(id).map(|path| path.to_string()) else {
            continue;
        };
        if !character_sheet_paths.contains(&path) {
            continue;
        }
        let megapixels = f64::from(image.width()) * f64::from(image.height()) / 1.0e6;
        out.push((path, megapixels));
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

/// The resident character pages of THIS App that no realization in its
/// tables (character sheets or props) owns. A retired realization drops its
/// pages' last handles, so a page still resident with no owner is held by
/// something else — the leak the residency rule exists to catch.
pub fn orphan_character_pages(app: &bevy::prelude::App) -> Vec<String> {
    let assets = app
        .world()
        .resource::<ambition_platformer2d::sprite_sheet::game_assets::GameAssets>();
    let owned: std::collections::BTreeSet<String> = assets
        .characters
        .resident_sheets()
        .map(|(_, sheet)| sheet)
        .chain(assets.characters.props.values())
        .flat_map(|sheet| sheet.pages.iter())
        .filter_map(|page| page.texture.path().map(|path| path.to_string()))
        .collect();
    resident_character_pages(app)
        .into_iter()
        .map(|(path, _)| path)
        .filter(|path| !owned.contains(path))
        .collect()
}

/// A sync-test rollback sim owned the way the SHIPPED GAME owns it.
///
/// ⛔⛔ **THE DEFAULT ROLLBACK FIXTURE IS NOT THE SHIPPED OWNERSHIP MODE, AND
/// UNTIL 2026-09-18 NOTHING IN THIS WORKSPACE EXERCISED THE ONE THAT IS.**
/// `with_sync_test_rollback_settings` installs the session through
/// `start_sync_test_session`, which stamps `SyncTestOwner::Caller`;
/// `locally_rebasable_timeline` answers only for `SyncTestOwner::LocalMaintainer`.
/// So on the ordinary fixture `mechanical_mutation_boundary` reports
/// `ForeignTimeline` and `decide_mechanical_edit_admission` REFUSES every
/// developer edit — correctly, because a harness that installed its own timeline
/// did not ask for it to be rebased. The shipped game arms `LocalSessionPolicy`
/// (the rollback observatory does it) and `maintain_local_session` installs the
/// session as `LocalMaintainer`, where the same edit is ADMITTED and the baseline
/// is stood down and rebased.
///
/// ⇒ Measured when this was written: `LocalSessionPolicy` appeared NOWHERE in
/// `game/ambition_app/tests/` or `crates/ambition_sim_harness/src/`, so every
/// rollback arm in the suite sat on the refusing side of the admission road
/// without saying so. Reach for this whenever an arm's subject involves a
/// mechanical edit, a session rebase, or anything that asks whether this host may
/// stop its own timeline.
///
/// ⚠ **IT STOPS THE CALLER-OWNED SESSION AND LETS THE MAINTAINER BUILD ITS OWN**,
/// rather than editing `RollbackSessionOwnership` in place: the owner stamp and
/// the installed session are ONE FACT, and writing half of it is how a fixture
/// comes to describe a world that cannot exist.
pub fn hand_the_timeline_to_the_local_maintainer(sim: &mut Platformer2dSimHarness) {
    use ambition_platformer2d::rollback::local_session::LocalSessionPolicy;

    let world = sim.world_mut();
    ambition_platformer2d::rollback::stop_session(world);
    // ⚠ THE ORDER IS `(check_distance, max_prediction_window)` AND GGRS REQUIRES
    // THE FIRST TO BE SMALLER — the same 4 and 10 every rollback arm here uses.
    // Inverting them does not fail loudly: `maintain_local_session` catches
    // `Invalid Request: Check distance too big`, records it in
    // `LocalSessionOwnership::last_error` and carries on with NO session, so the
    // arm silently becomes a no-rollback arm that PASSES.
    world.insert_resource(LocalSessionPolicy {
        check_distance: 4,
        max_prediction_window: 10,
        autostart: true,
    });
}

/// A rollback sim whose timeline this host owns, settled for `frames`.
///
/// The settle matters: `maintain_local_session` starts GGRS only once gameplay is
/// active (`session_world_entity(world).is_some()`), so a fixture that asserts
/// immediately after construction is asserting about a world with no timeline.
pub fn maintainer_owned_rollback_sim(frames: usize) -> Platformer2dSimHarness {
    let mut sim = Platformer2dSimHarness::new_with_options(
        fixed_60hz_options().with_sync_test_rollback_settings(4, 10),
    )
    .expect("the sandbox builds headlessly under a sync-test session");
    hand_the_timeline_to_the_local_maintainer(&mut sim);
    for _ in 0..frames {
        sim.step(base());
    }
    // ⛔⛤ **THE FIXTURE ANSWERS FOR ITSELF, BECAUSE THE ARMS COULD NOT.**
    // Inverting the settings pair above leaves GGRS refusing the session —
    // `maintain_local_session` records `Invalid Request: Check distance too big`
    // and carries on with NONE — and a rollback arm handed that world passes
    // while measuring nothing: the admission reads `NoTimeline` and publishes
    // anything, and `rollback_health()` on a sessionless world reports fine.
    // Three arms of `a_dev_clone_survives_a_rewind` stayed green under exactly
    // that poison. So every caller that asked for a settled session gets the
    // check here instead of remembering to write it.
    //
    // ⚠ ONLY WHEN `frames > 0`: `maintain_local_session` installs GGRS on the
    // first step, once gameplay is active, so at zero frames there is nothing
    // to read yet and two arms deliberately want that moment.
    if frames > 0 {
        let boundary = format!(
            "{:?}",
            ambition_platformer2d::rollback::mechanical_mutation_boundary(sim.world())
        );
        assert_eq!(
            boundary, "LocallyRebasable",
            "this fixture promises a live timeline THIS host owns after {frames} frame(s) \
             and the boundary reports `{boundary}`. `NoTimeline` means no session was \
             installed — check the `(check_distance, max_prediction_window)` order above; \
             `ForeignTimeline` means it is caller-owned and every mechanical edit is refused"
        );
        sim.rollback_health().unwrap_or_else(|error| {
            panic!("this fixture promises a healthy timeline after {frames} frame(s): {error}")
        });
    }
    sim
}

/// ⛔⛤ **THE HUB'S BOOT CUTSCENE IS LIVE SINCE 2026-09-18, AND IT BLOCKS ON A
/// DIALOGUE BEAT.** `default_room_cutscene_bindings()` bound `test_intro` to
/// `central_hub_main` — an LDtk LEVEL id — so the row could never match the
/// runtime room and the hub's intro had never played. `479d5a028` repointed it
/// at `central_hub_complex`, which was the right fix and made the cutscene real:
/// on first entry the hub now runs `// boot sequence` (1.4 s), a fade (0.8 s),
/// and then a `CutsceneBeat::Dialogue` with NO duration — it waits for a
/// dismiss. While it plays, `declare_in_session_input_contexts` gives the seat a
/// CAPTURING `CUTSCENE_CONTEXT` claim, so `gameplay_owned()` is false and no
/// gameplay input routes anywhere.
///
/// ⇒ Four arms written before the binding was fixed hung on it, and their
/// failure messages sent the reader after the input road instead. A test whose
/// subject is not the intro says so by calling this, which sets the same
/// `seen_flag` a returning player's save carries — the ordinary state of every
/// visit after the first, not a special test mode.
///
/// ⚠ THIS IS AN OPT-OUT, SO SOMETHING ELSE HAS TO HOLD THE FACT: the intro
/// playing and capturing input on first entry is witnessed by
/// `the_hub_intro_plays_on_first_entry_and_holds_input.rs`. Without that arm,
/// calling this everywhere would quietly restore the world in which the
/// binding was still broken.
/// ⚠ **TWO ROADS, BECAUSE THE FLAG ONLY STOPS A START.**
/// `start_queued_cutscene` consults `seen_flag` when it STARTS a script, so a
/// flag set after the cutscene is already running changes nothing — measured:
/// `Platformer2dSimHarness::new_with_options` has the hub's intro playing by
/// the time it returns, and setting the flag on the harness left it playing
/// through every subsequent step. A harness caller therefore seeds the SAVE
/// before construction with [`a_save_that_has_seen_the_hub_intro`]; a
/// shell-host `App` can use this function, because its room is not loaded
/// until the gameplay route is entered some frames later.
pub fn the_hub_intro_has_already_played(world: &mut bevy::prelude::World) {
    let mut save = world
        .get_resource_mut::<ambition_platformer2d::persistence::save::AmbitionGameSave>()
        .expect("this composition carries a save; the flag has nowhere else to live");
    save.data_mut().set_flag("test_intro_seen".to_string(), true);
}

/// A save whose player has already seen the hub's intro — the `with_save`
/// form of [`the_hub_intro_has_already_played`], for the harness road where
/// the room is loaded before the caller ever holds a `World`.
pub fn a_save_that_has_seen_the_hub_intro(
) -> ambition_platformer2d::session::AmbitionGameSaveData {
    let mut data = ambition_platformer2d::session::AmbitionGameSaveData::default();
    data.set_flag("test_intro_seen".to_string(), true);
    data
}
