//! A file saved on disk while the game runs reaches the bodies the game
//! constructed, through the content watch, the reload request and the shell's
//! own re-preparation and activation. No Cargo, no linker, no direct
//! publication API.
//!
//! The crate-level arm `a_host_plays_a_move_edited_on_disk_after_it_was_built`
//! stops at `PreparedCharacterRegistry`. These read the BUILT BODIES
//! (`ActorMoveset`, `BodyHealth`, `CombatTuning`) and the session's frozen
//! cast, which only a real preparation produces.
//!
//! Why `proving_grounds`: the default `ambition_gameplay` room's player wears
//! the derived kit moves, which no authored move table names, and the smash
//! composition runs a live rollback timeline that refuses publication by
//! design. `proving_grounds` routes `ambition_gameplay` through the shell with
//! no `ActiveRollbackAuthority` and builds three goblins whose 34 moves are
//! `goblin.ron`'s own.
//!
//! The move edit is a timing, never an extent: for most of the roster a
//! rendered sprite stage derives the strike geometry from a bone segment, so an
//! extents edit would not reach the actor. Timing is not overridden.
//!
//! The provider-ordering edge (`.before(GameplaySessionSet::Providers)`) is NOT
//! witnessed here: this room's goblins exist before the re-preparation, so
//! nothing is constructed for the edge to order against. Its witness is
//! `an_edit_reaches_the_shipped_game::an_edited_pack_reaches_the_cast_the_shipped_composition_plays`.
use crate::common;

/// An AUTHORED move id — `goblin.ron`'s, not one of the derived kit ids.
#[cfg(not(feature = "static_content"))]
const SUBJECT: &str = "jab";
/// The authored source it lives in, as `pack.ron` spells it.
#[cfg(not(feature = "static_content"))]
const SUBJECT_SOURCE: &str = "data/movesets/goblin.ron";
/// What the edit adds. Large enough that no rounding could explain the result.
#[cfg(not(feature = "static_content"))]
const BUMP: f32 = 0.5;

/// Every duration a CONSTRUCTED body is currently playing for [`SUBJECT`],
/// entity order removed.
///
/// ⛔ Re-found by moveset content every call, never cached as an `Entity`: a
/// re-preparation may rebuild the room's bodies, and a stale entity id would
/// read a despawned actor and look like "the edit never arrived".
///
/// ⛔⛤ **IT USED TO BE `.next()`, AND `.next()` OF A QUERY IS STORAGE ORDER.**
/// `SUBJECT` is `"jab"` — a move id `goblin.ron` authors and **so does the
/// player's own character**, at a different duration. The arm therefore sampled
/// whichever of the two happened to sort first, and it sorted a goblin only by
/// luck. MEASURED 2026-09-13: adding ONE resource to the App shifted every entity
/// id by one, moved `Player Robot v3` to the head of the archetype iteration, and
/// reddened this test **with every goblin carrying the edited value correctly**:
///
/// ```text
/// before: [(515, "Goblin", 0.71), (516, "Goblin", 0.71), (517, "Goblin", 0.71), (523, "Player Robot v3", 0.25)]
/// after:  [(524, "Player Robot v3", 0.25), (516, "Goblin", 0.71), (517, "Goblin", 0.71), (518, "Goblin", 0.71)]
/// ```
///
/// ⇒ A red that says "the edit did not arrive" when the edit arrived on every
/// body it was authored for is a false red, and a false red is obeyed faster
/// than a false green is questioned. The population is the measurement.
#[cfg(not(feature = "static_content"))]
fn subject_durations_on_bodies(
    sim: &mut ambition_sim_harness::Platformer2dSimHarness,
) -> Vec<f32> {
    use ambition_platformer2d::combat::moveset::ActorMoveset;
    let world = sim.world_mut();
    let mut q = world.query::<&ActorMoveset>();
    let mut found: Vec<f32> = q
        .iter(world)
        .filter_map(|ms| ms.0.moves.iter().find(|m| m.id == SUBJECT))
        .map(|m| m.duration_s)
        .collect();
    // ⚠ SORTED so a comparison of two samples cannot depend on iteration order
    // either — the defect above, one level up.
    found.sort_by(f32::total_cmp);
    found
}

/// The duration every body that AUTHORS [`SUBJECT`] at `expected` is playing,
/// counted — and `None` when the population does not agree on one value.
///
/// ⛔ THE QUESTION THIS ARM IS ABOUT IS *"did the edit reach the bodies built
/// from the edited source"*, so the answer is a COUNT over a population, not one
/// body's reading.
#[cfg(not(feature = "static_content"))]
fn subject_body_count_at(
    sim: &mut ambition_sim_harness::Platformer2dSimHarness,
    expected: f32,
) -> usize {
    subject_durations_on_bodies(sim)
        .into_iter()
        .filter(|found| (found - expected).abs() < 1e-4)
        .count()
}

/// Export this build's sources to `root` and move [`SUBJECT`]'s timing by
/// [`BUMP`]. Returns (sources written, moves edited, authored duration before).
///
/// ⛔ NO `tempfile` DEV-DEPENDENCY IS ADDED FOR THIS — a pid+nanos directory
/// under the system temp dir, removed on the way out.
///
/// ⛔⛤ **EDITED THROUGH THE TYPED DOCUMENT, AND THE EDIT IS COUNTED.** A fixture
/// edit that matches nothing and a passing test look identical; this repository
/// has been bitten by a zero-match substitution twice in one day.
#[cfg(not(feature = "static_content"))]
fn export_with_the_subject_retimed(root: &std::path::Path) -> (usize, usize, f32) {
    let written = ambition_content::pack::export_sources_to(root).expect("sources export");
    let path = root.join(SUBJECT_SOURCE);
    let text = std::fs::read_to_string(&path).expect("the exported subject source reads");
    let mut doc = ambition_platformer2d::entity_catalog::EntityCatalogDoc::parse(&text)
        .expect("a shipped move table parses as its typed document");
    let (mut edited, mut before) = (0usize, f32::NAN);
    for entity in &mut doc.entities {
        let Some(moveset) = entity.contracts.moveset.as_mut() else {
            continue;
        };
        for spec in &mut moveset.moves {
            if spec.id != SUBJECT {
                continue;
            }
            before = spec.duration_s;
            let after = spec.duration_s + BUMP;
            // ⚠ THE LAST WINDOW FOLLOWS THE DURATION when it ended with the
            // move, so the edit cannot produce a timeline shorter than its own
            // length and be refused for a reason that is not the subject.
            for window in &mut spec.windows {
                if (window.end_s - spec.duration_s).abs() < 1e-6 {
                    window.end_s = after;
                }
            }
            spec.duration_s = after;
            edited += 1;
        }
    }
    std::fs::write(&path, doc.to_ron().expect("the edited document serializes"))
        .expect("the edited source writes");
    (written, edited, before)
}

/// The shell's current activation id, or `None` before any route is active.
#[cfg(not(feature = "static_content"))]
fn activation_id(
    sim: &mut ambition_sim_harness::Platformer2dSimHarness,
) -> Option<ambition_platformer2d::game_shell::ShellActivationId> {
    sim.world()
        .get_resource::<ambition_platformer2d::game_shell::ShellRouter>()
        .and_then(|r| r.active.as_ref())
        .map(|a| a.activation_id)
}

/// The live cast's generation.
#[cfg(not(feature = "static_content"))]
fn cast_generation(sim: &mut ambition_sim_harness::Platformer2dSimHarness) -> Option<u64> {
    sim.world()
        .get_resource::<ambition_platformer2d::characters::prepared::PreparedCharacterRegistry>()
        .map(|r| r.generation().get())
}

/// ⭐ THE RUNNING GAME PLAYS SAVED FILES, with no call from the test. The
/// content watch (`ambition_content::content_watch`) sees a source change,
/// compiles the pack from disk and requests the reload; the shell re-prepares
/// and activates on its own. The watch is pointed at an exported copy, so the
/// test edits no file another test reads. One sim carries every save:
///
/// 1. A file no `pack.ron` line declares is not played and requests nothing.
/// 2. One save edits five families: a move timing (`goblin.ron`), a character
///    row's HP and tags (`character_catalog.ron`), a fighter facet and an item
///    row. Until the activation the bodies play the old move; on the frame the
///    shell activates they play the new one and the cast generation has moved.
///    Then every goblin is rebuilt with the saved HP and as a robot (its hurt
///    material comes from the catalog row's tags, which the prepared cast does
///    not carry, so this is the candidate CATALOG reaching construction), the
///    session's frozen cast holds the edits, and one save made one reload.
/// 3. A save that adds a character row and one that removes it: the row joins
///    and leaves both the published and the frozen cast, and the others stay.
/// 4. A line added to `pack.ron` brings the file from step 1 into the library.
/// 5. The movement defaults take their own road (`MovementDefaultsWatch`, the
///    mirror the F3 inspector writes): a save that does not parse is refused,
///    a valid one changes how high the player jumps, a misspelt feel field
///    refuses the whole save, and a feel-only save writes no movement tuning.
#[cfg(not(feature = "static_content"))]
#[test]
fn content_files_saved_while_the_game_runs_are_played() {
    use ambition_app::app::movement_defaults_watch::MovementDefaultsWatch;
    use ambition_content::content_watch::ContentSourceWatch;
    use ambition_content::gameplay_defaults::PLATFORMER_DEFAULTS_FILE;
    use ambition_platformer2d::characters::prepared::{ActiveSessionCast, PreparedCharacterRegistry};
    use ambition_platformer2d::characters::smash_fighter::FIGHTER_DAMAGE;
    use ambition_platformer2d::cutscene::CutsceneLibrary;
    use ambition_platformer2d::engine_core::ActiveMovementTuning;
    use ambition_platformer2d::items::{item_catalog, Item};
    use ambition_platformer2d::vfx::vfx::ImpactMaterial;
    type Sim = ambition_sim_harness::Platformer2dSimHarness;
    const EDITED_HP: i32 = 9;
    const ADDED: &str = "goblin_scout";

    let frozen_has = |sim: &Sim, id: &str| {
        sim.world()
            .get_resource::<ActiveSessionCast>()
            .and_then(ActiveSessionCast::cast)
            .is_some_and(|cast| cast.get(id).is_some())
    };
    let published_has = |sim: &Sim, id: &str| {
        sim.world().get_resource::<PreparedCharacterRegistry>().is_some_and(|cast| cast.get(id).is_some())
    };
    let jab = |sim: &Sim| {
        sim.world()
            .get_resource::<ActiveSessionCast>()
            .and_then(ActiveSessionCast::cast)
            .and_then(|cast| cast.get("player_robot_v3"))
            .and_then(|definition| definition.scaled_move_damage.get(&FIGHTER_DAMAGE))
            .and_then(|damage| damage.get("jab").cloned())
    };
    let gun_sword = |sim: &Sim| {
        item_catalog(sim.world()).display_name(Item::GunSword).expect("the pack names the gun-sword").to_string()
    };
    let has_new_script = |sim: &Sim| {
        sim.world()
            .get_resource::<CutsceneLibrary>()
            .is_some_and(|library| library.get("a_script_added_on_disk").is_some())
    };
    let robots = |sim: &mut Sim| {
        let materials = goblin_materials(sim);
        !materials.is_empty() && materials.iter().all(|material| *material == ImpactMaterial::Robot)
    };
    let jump = |sim: &Sim| sim.world().resource::<ActiveMovementTuning>().0.jump_speed;

    let mut sim = common::fixed_60hz_room_sim("proving_grounds");
    for _ in 0..40 {
        sim.step(common::base());
    }

    // ── the premises: what the room plays before any save ───────────────────
    // The population and its size: the room builds three goblins from
    // `goblin.ron`, and every later count is compared against this one.
    let before = subject_durations_on_bodies(&mut sim).first().copied().unwrap_or_else(|| {
        panic!("no constructed body plays `{SUBJECT}`, so this room cannot witness an authored edit")
    });
    let subjects_before = subject_body_count_at(&mut sim, before);
    assert!(
        subjects_before >= 3,
        "this room builds three bodies from `{SUBJECT_SOURCE}` and {subjects_before} carry `{SUBJECT}` at {before}s"
    );
    // A live rollback timeline refuses publication by design.
    assert!(
        sim.world()
            .get_resource::<ambition_platformer2d::runtime::rollback::ActiveRollbackAuthority>()
            .is_none(),
        "this room has a live rollback authority, so the reload would be refused at the publication boundary"
    );
    let hps_before = goblin_max_hps(&mut sim);
    assert!(
        hps_before.len() >= 3 && hps_before.iter().all(|&hp| hp != EDITED_HP),
        "the premise: three goblins whose HP the edit changes, got {hps_before:?}"
    );
    let materials_before = goblin_materials(&mut sim);
    assert!(
        materials_before.len() == hps_before.len()
            && materials_before.iter().all(|material| *material == ImpactMaterial::Flesh),
        "the premise: the goblins are flesh, got {materials_before:?}"
    );
    assert_eq!(jab(&sim), Some(vec![3]), "the premise: the shipped facet's jab deals 3");
    assert_eq!(gun_sword(&sim), "Gun-Sword", "the premise: the shipped catalog names the slot `Gun-Sword`");
    assert!(frozen_has(&sim, "goblin") && !frozen_has(&sim, ADDED), "the premise: the goblin is in the cast and `{ADDED}` is not");
    assert!(
        sim.world().get_resource::<ContentSourceWatch>().is_some()
            && sim.world().get_resource::<MovementDefaultsWatch>().is_some(),
        "the premise: a build that reads content off disk watches it"
    );
    let before_activation = activation_id(&mut sim);
    let before_generation = cast_generation(&mut sim);
    assert!(
        before_activation.is_some() && before_generation.is_some(),
        "the room has no active shell route or no published cast, so neither clock this test watches exists yet"
    );

    let root = std::env::temp_dir().join(format!(
        "ambition_content_watch_{}_{}",
        std::process::id(),
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).expect("clock").as_nanos()
    ));
    let dir = root.with_extension("movement");
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let written = ambition_content::pack::export_sources_to(&root).expect("sources export");
        assert!(written > 10, "the export wrote {written} source(s), so the watch would read a directory that is not this build's content");
        sim.world_mut().insert_resource(ContentSourceWatch::new(root.clone()));
        let requested = |sim: &Sim| sim.world().resource::<ContentSourceWatch>().requested;
        let step_until = |sim: &mut Sim, done: &dyn Fn(&mut Sim) -> bool, what: &str| {
            let mut frames = 0;
            while !done(sim) {
                sim.step(common::base());
                frames += 1;
                assert!(frames < 600, "600 frames after the save, {what}; reloads requested: {}", requested(&*sim));
            }
        };

        // ── 1. a file no manifest line declares ──────────────────────────────
        // A file system's clock can be coarse: each save must look newer.
        std::thread::sleep(std::time::Duration::from_millis(20));
        std::fs::write(
            root.join("data/cutscenes/added_on_disk.ron"),
            r#"[(id: "a_script_added_on_disk", beats: [Wait(seconds: 0.1)], seen_flag: None)]"#,
        )
        .unwrap();
        for _ in 0..120 {
            sim.step(common::base());
        }
        assert_eq!(requested(&sim), 0, "an unchanged copy, or a file no manifest line declares, asked for a reload");
        assert!(!has_new_script(&sim), "a file no manifest line declares was played");

        // ── 2. one save, five families ───────────────────────────────────────
        std::thread::sleep(std::time::Duration::from_millis(20));
        let (_, edited, authored_before) = export_with_the_subject_retimed(&root);
        assert_eq!(edited, 1, "the retime matched {edited} authored move(s) named `{SUBJECT}` in {SUBJECT_SOURCE}");
        assert!(
            (authored_before - before).abs() < 1e-4,
            "the body plays `{SUBJECT}` at {before} but the authored source says {authored_before}: the body does not play the file this edits"
        );
        let catalog_path = root.join("data/character_catalog.ron");
        let shipped_catalog = std::fs::read_to_string(&catalog_path).unwrap();
        let edited_catalog = {
            let start = shipped_catalog.find("\"goblin\": (").expect("the catalog has a goblin row");
            let at = start + shipped_catalog[start..].find("max_health: Some(").expect("the row states its HP");
            let end = at + shipped_catalog[at..].find(')').unwrap() + 1;
            let text = format!("{}max_health: Some({EDITED_HP}){}", &shipped_catalog[..at], &shipped_catalog[end..]);
            let at = start + text[start..].find("tags: [").expect("the row states its tags") + "tags: [".len();
            format!("{}\"robot\", {}", &text[..at], &text[at..])
        };
        std::fs::write(&catalog_path, &edited_catalog).unwrap();
        let facet = root.join("data/fighters/player_robot_v3.ron");
        let text = std::fs::read_to_string(&facet).unwrap();
        assert_eq!(text.matches("\"jab\": [3],").count(), 1, "the premise: the facet states the jab once");
        std::fs::write(&facet, text.replacen("\"jab\": [3],", "\"jab\": [4],", 1)).unwrap();
        let items = root.join("data/items.ron");
        let text = std::fs::read_to_string(&items).unwrap();
        let from = "display_name: \"Gun-Sword\",";
        assert_eq!(text.matches(from).count(), 1, "the premise: items.ron names the gun-sword once");
        std::fs::write(&items, text.replacen(from, "display_name: \"Sword-Gun\",", 1)).unwrap();

        // Assert at the activating frame, not at a frame budget's end: a
        // publication that lands a frame after the world was built from it
        // passes a wait for the value.
        let mut activated_at = None;
        for frame in 0..600 {
            sim.step(common::base());
            if activation_id(&mut sim) != before_activation {
                activated_at = Some(frame);
                break;
            }
            assert_eq!(
                subject_body_count_at(&mut sim, before),
                subjects_before,
                "frame {frame}: the bodies changed what they play before any activation, so there is no boundary to be atomic at"
            );
        }
        let activated_at = activated_at.unwrap_or_else(|| {
            panic!(
                "600 frames after the save the shell never activated a new generation (reloads requested: {})",
                requested(&sim)
            )
        });
        assert_eq!(
            subject_body_count_at(&mut sim, before + BUMP),
            subjects_before,
            "the shell activated on frame {activated_at} and the constructed bodies play `{SUBJECT}` at {:?}, not {}",
            subject_durations_on_bodies(&mut sim),
            before + BUMP
        );
        // This candidate edits a move table, so it stages a cast: a generation
        // that did not advance means the edit reached the actor by another road.
        assert_ne!(cast_generation(&mut sim), before_generation, "the cast generation did not advance for a candidate that edits `{SUBJECT}`");

        step_until(
            &mut sim,
            &|sim| {
                let hps = goblin_max_hps(sim);
                !hps.is_empty() && hps.iter().all(|&hp| hp == EDITED_HP) && robots(sim) && jab(&*sim) == Some(vec![4]) && gun_sword(&*sim) == "Sword-Gun"
            },
            "the goblins, the facet or the item catalog do not play the save",
        );
        assert_eq!(requested(&sim), 1, "one save, one reload");
        assert_eq!(goblin_max_hps(&mut sim).len(), hps_before.len(), "the same goblins, rebuilt");
        let catalog = sim.world().resource::<ambition_platformer2d::characters::actor::character_catalog::CharacterCatalog>();
        let row = catalog.get("goblin").expect("the published catalog has the goblin");
        assert_eq!(row.max_health, Some(EDITED_HP), "the App's catalog is not the published one");
        assert!(row.tags.iter().any(|tag| tag == "robot"), "the App's catalog does not carry the saved tag");
        // The cast the session FROZE, not only the App's registry: bodies are
        // derived from the generation's cast (`SessionCast`).
        let frozen_hp = sim
            .world()
            .resource::<ActiveSessionCast>()
            .cast()
            .and_then(|cast| cast.get("goblin"))
            .and_then(|definition| definition.vitals.max_health);
        assert_eq!(frozen_hp, Some(EDITED_HP), "the session did not freeze the cast it was prepared from");

        // ── 3. a row added, then removed ─────────────────────────────────────
        let with_scout = {
            let start = edited_catalog.find("\"goblin\": (").expect("the catalog has a goblin row");
            let end = start + edited_catalog[start..].find("\"sandbag\": (").expect("the sandbag row follows it");
            let row = edited_catalog[start..end]
                .replacen("\"goblin\": (", &format!("\"{ADDED}\": ("), 1)
                .replacen("display_name: \"Goblin\"", "display_name: \"Goblin Scout\"", 1);
            format!("{}{row}{}", &edited_catalog[..end], &edited_catalog[end..])
        };
        std::thread::sleep(std::time::Duration::from_millis(20));
        std::fs::write(&catalog_path, &with_scout).unwrap();
        step_until(&mut sim, &|sim| frozen_has(&*sim, ADDED), "the added character is not in the frozen cast");
        assert!(published_has(&sim, ADDED), "the added character is not in the published cast");
        assert!(frozen_has(&sim, "goblin"), "adding a character dropped the others");
        std::thread::sleep(std::time::Duration::from_millis(20));
        std::fs::write(&catalog_path, &edited_catalog).unwrap();
        step_until(&mut sim, &|sim| !frozen_has(&*sim, ADDED), "the removed character is still in the frozen cast");
        assert!(!published_has(&sim, ADDED), "the removed character is still in the published cast");
        assert!(frozen_has(&sim, "goblin"), "removing a character dropped the others");
        assert_eq!(requested(&sim), 3, "three saves, three reloads");

        // ── 4. the manifest line ─────────────────────────────────────────────
        let manifest = root.join("pack.ron");
        let text = std::fs::read_to_string(&manifest).unwrap();
        let anchor = r#"(path: "data/cutscenes/intro.ron", schema: "cutscene_library", version: 1),"#;
        assert_eq!(text.matches(anchor).count(), 1, "the premise: the manifest declares the intro cutscenes once");
        std::thread::sleep(std::time::Duration::from_millis(20));
        std::fs::write(
            &manifest,
            text.replacen(
                anchor,
                &format!(r#"{anchor} (path: "data/cutscenes/added_on_disk.ron", schema: "cutscene_library", version: 1),"#),
                1,
            ),
        )
        .unwrap();
        step_until(&mut sim, &|sim| has_new_script(&*sim), "the library has no script from the file the manifest now declares");

        // ── 5. the movement defaults ─────────────────────────────────────────
        let shipped = std::fs::read_to_string(PLATFORMER_DEFAULTS_FILE).expect("the defaults file");
        let shipped_jump = jump(&sim);
        let stated = format!("jump_speed: {shipped_jump:.1},");
        assert_eq!(shipped.matches(&stated).count(), 1, "the premise: the defaults file states the live jump speed ({stated}) once");
        let before_launch = launch_speed(&mut sim);
        assert!(before_launch > 0.0, "the premise: the player leaves the floor when it jumps");
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("platformer_defaults.ron");
        std::fs::write(&file, &shipped).unwrap();
        sim.world_mut().insert_resource(MovementDefaultsWatch::new(file.clone()));
        let applied = |sim: &Sim| sim.world().resource::<MovementDefaultsWatch>().applied;
        let raised = shipped_jump + 70.0;
        std::thread::sleep(std::time::Duration::from_millis(20));
        std::fs::write(&file, "( this is not the defaults").unwrap();
        for _ in 0..45 {
            sim.step(common::base());
        }
        assert_eq!((applied(&sim), jump(&sim)), (0, shipped_jump), "a save that does not parse is refused");
        std::thread::sleep(std::time::Duration::from_millis(20));
        std::fs::write(&file, shipped.replace(&stated, &format!("jump_speed: {raised:.1},"))).unwrap();
        let mut frames = 0;
        while jump(&sim) != raised {
            sim.step(common::base());
            frames += 1;
            assert!(frames < 120, "120 frames after the save the jump speed is {}", jump(&sim));
        }
        assert_eq!(applied(&sim), 1, "one save, one write");
        // The resource alone would pass if every body read an authored tuning
        // of its own.
        let after_launch = launch_speed(&mut sim);
        assert!(
            after_launch > before_launch * 1.05,
            "the player's jump launch did not follow the saved tuning: {before_launch} before, {after_launch} after"
        );
        use ambition_platformer2d::combat::feel::Platformer2dFeelTuningMonolith;
        let hitlag = |sim: &Sim| sim.world().resource::<Platformer2dFeelTuningMonolith>().hitlag_time;
        assert_eq!(hitlag(&sim), Platformer2dFeelTuningMonolith::default().hitlag_time, "the premise: the shipped hitlag");
        let with_feel = |jump_speed: f32, feel: &str| {
            let moved = shipped.replace(&stated, &format!("jump_speed: {jump_speed:.1},"));
            let close = moved.rfind(')').expect("the file is one tuple");
            format!("{}    feel: ({feel}),\n{}", &moved[..close], &moved[close..])
        };
        let feel_written = |sim: &Sim| sim.world().resource::<MovementDefaultsWatch>().applied_feel;
        // A field the feel does not have refuses the whole save: the jump speed
        // in the same save is not taken either.
        std::thread::sleep(std::time::Duration::from_millis(20));
        std::fs::write(&file, with_feel(raised + 10.0, "hitlag_tme: 0.25")).unwrap();
        for _ in 0..45 {
            sim.step(common::base());
        }
        assert_eq!((feel_written(&sim), jump(&sim)), (0, raised), "a misspelt feel field did not refuse the save");
        std::thread::sleep(std::time::Duration::from_millis(20));
        std::fs::write(&file, with_feel(raised, "hitlag_time: 0.25")).unwrap();
        let mut frames = 0;
        while hitlag(&sim) != 0.25 {
            sim.step(common::base());
            frames += 1;
            assert!(frames < 120, "120 frames after the save the hitlag is {}", hitlag(&sim));
        }
        assert_eq!((applied(&sim), feel_written(&sim)), (1, 1), "the feel save wrote more than the feel");
        assert_eq!(jump(&sim), raised, "a feel-only save changed the movement tuning");
    }));
    let _ = std::fs::remove_dir_all(&root);
    let _ = std::fs::remove_dir_all(&dir);
    if let Err(payload) = outcome {
        std::panic::resume_unwind(payload);
    }
}

/// Each live mockingbird's maximum HP.
#[cfg(not(feature = "static_content"))]
fn mockingbird_max_hps(sim: &mut ambition_sim_harness::Platformer2dSimHarness) -> Vec<i32> {
    let world = sim.world_mut();
    let mut q = world.query::<(
        &ambition_platformer2d::boss_encounter::BossConfig,
        &ambition_platformer2d::characters::actor::BodyHealth,
    )>();
    q.iter(world)
        .filter(|(config, _)| config.behavior.id == "mockingbird")
        .map(|(_, health)| health.max())
        .collect()
}

/// The live mockingbird's `strike_speed_scale`, and how many mockingbirds.
#[cfg(not(feature = "static_content"))]
fn mockingbird_strike_speed_scales(sim: &mut ambition_sim_harness::Platformer2dSimHarness) -> Vec<f32> {
    let world = sim.world_mut();
    let mut q = world.query::<&ambition_platformer2d::boss_encounter::BossConfig>();
    q.iter(world)
        .filter(|config| config.behavior.id == "mockingbird")
        .map(|config| config.behavior.strike_speed_scale)
        .collect()
}

/// ⭐ A BOSS TUNING EDIT IS PLAYED WITHOUT A RESTART. `boss_profiles.ron` is
/// a participating domain of the reload (2026-10-01): the candidate catalog is
/// admitted at request time, frozen by the preparation, and published at the
/// activation, so the boss the new session builds plays the saved value.
#[cfg(not(feature = "static_content"))]
#[test]
fn a_boss_tuning_saved_while_the_game_runs_is_played() {
    use ambition_content::content_watch::ContentSourceWatch;
    const EDITED: f32 = 0.37;
    let mut sim = common::fixed_60hz_room_sim("mockingbird_sky");
    for _ in 0..40 {
        sim.step(common::base());
    }
    let before = mockingbird_strike_speed_scales(&mut sim);
    assert_eq!(before.len(), 1, "the premise: the arena builds one mockingbird");
    assert!((before[0] - EDITED).abs() > 0.1, "the premise: the edit changes the value");
    // And its encounter: the HP is seeded from the encounter file.
    const EDITED_HP: i32 = 41;
    assert_eq!(mockingbird_max_hps(&mut sim).len(), 1);
    assert_ne!(mockingbird_max_hps(&mut sim)[0], EDITED_HP, "the premise: the edit changes the HP");

    let root = std::env::temp_dir().join(format!("ambition_boss_watch_{}", std::process::id()));
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        ambition_content::pack::export_sources_to(&root).expect("sources export");
        sim.world_mut().insert_resource(ContentSourceWatch::new(root.clone()));
        std::thread::sleep(std::time::Duration::from_millis(20));
        let path = root.join("data/boss_profiles.ron");
        let text = std::fs::read_to_string(&path).unwrap();
        let start = text.find("\"mockingbird\": (").expect("the roster has a mockingbird row");
        let at = start + text[start..].find("strike_speed_scale: ").expect("the row has a strike speed scale");
        let end = at + text[at..].find(',').unwrap();
        let edited = format!("{}strike_speed_scale: {EDITED}{}", &text[..at], &text[end..]);
        std::fs::write(&path, edited).unwrap();
        let encounter = root.join("data/boss_encounters/mockingbird.ron");
        let text = std::fs::read_to_string(&encounter).unwrap();
        let at = text.find("max_hp: ").expect("the encounter states its HP");
        let end = at + text[at..].find(',').unwrap();
        std::fs::write(&encounter, format!("{}max_hp: {EDITED_HP}{}", &text[..at], &text[end..])).unwrap();

        let mut frames = 0;
        while mockingbird_strike_speed_scales(&mut sim).first().is_none_or(|v| (v - EDITED).abs() > 1e-6) {
            sim.step(common::base());
            frames += 1;
            assert!(
                frames < 600,
                "600 frames after the save the mockingbird plays {:?}; reloads requested: {}",
                mockingbird_strike_speed_scales(&mut sim),
                sim.world().resource::<ContentSourceWatch>().requested
            );
        }
        assert_eq!(mockingbird_strike_speed_scales(&mut sim).len(), 1, "one mockingbird, rebuilt");
        let catalog = sim.world().resource::<ambition_platformer2d::boss_encounter::BossCatalog>();
        assert_eq!(
            catalog.behavior("mockingbird").map(|b| b.strike_speed_scale),
            Some(EDITED),
            "the App's catalog is the published one"
        );
        // ⛔ AND THE GENERATION THE SESSION FROZE. Until 2026-10-01
        // `update_boss_encounters` re-seeded the behaviour from the App's
        // catalog after construction, so a session that froze N's catalog
        // under N+1's identity still showed N+1 on the boss (poison "the claim
        // carries no catalog" stayed green). The live value witnesses the
        // freeze now; this asserts the frozen record directly as well.
        let frozen = sim
            .world()
            .resource::<ambition_platformer2d::construction::mechanics::SessionMechanics>()
            .bosses
            .behavior("mockingbird")
            .map(|b| b.strike_speed_scale);
        assert_eq!(frozen, Some(EDITED), "the session froze the catalog it was prepared from");
        // ⛔ The encounter is seeded from the catalog the boss was built with
        // (`BossConfig::seed`), not from the App's on the boss's first tick.
        assert_eq!(mockingbird_max_hps(&mut sim), [EDITED_HP], "the rebuilt boss has the saved HP");
        eprintln!("a saved boss tuning was played {frames} frames after the save");
    }));
    let _ = std::fs::remove_dir_all(&root);
    if let Err(payload) = outcome {
        std::panic::resume_unwind(payload);
    }
}

/// ⭐ THE SAVED EDIT UNDER THE SHIPPED OWNERSHIP MODE: a timeline the local
/// maintainer owns is rebased onto the reloaded generation, not crossed by it.
#[cfg(not(feature = "static_content"))]
#[test]
fn a_content_file_saved_under_a_local_timeline_rebases_it() {
    use ambition_app::rl_sim::AmbitionSim as _;
    use ambition_content::content_watch::ContentSourceWatch;
    let mut sim = ambition_sim_harness::Platformer2dSimHarness::new_with_options(
        common::fixed_60hz_room_options("proving_grounds").with_sync_test_rollback_settings(4, 10),
    )
    .expect("the room builds under a sync-test session");
    common::hand_the_timeline_to_the_local_maintainer(&mut sim);
    for _ in 0..40 {
        sim.step(common::base());
    }
    let boundary = |sim: &ambition_sim_harness::Platformer2dSimHarness| {
        format!("{:?}", ambition_platformer2d::rollback::mechanical_mutation_boundary(sim.world()))
    };
    let bound_and_live = |sim: &mut ambition_sim_harness::Platformer2dSimHarness| {
        let bound = sim
            .world()
            .get_resource::<ambition_platformer2d::runtime::rollback::ActiveRollbackAuthority>()
            .and_then(|authority| authority.contract().content);
        let world = sim.world_mut();
        let mut q = world.query::<&ambition_platformer2d::runtime::PreparedContentIdentity>();
        (bound, q.single(world).ok().copied())
    };
    assert_eq!(boundary(&sim), "LocallyRebasable", "the premise: this host owns the timeline");
    let (bound_before, live) = bound_and_live(&mut sim);
    assert!(bound_before.is_some() && bound_before == live, "the premise: the timeline binds the session's content");
    let before = subject_durations_on_bodies(&mut sim).first().copied().expect("a body plays the subject");
    let subjects = subject_body_count_at(&mut sim, before);

    let root = std::env::temp_dir().join(format!("ambition_content_watch_local_{}", std::process::id()));
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        ambition_content::pack::export_sources_to(&root).expect("sources export");
        sim.world_mut().insert_resource(ContentSourceWatch::new(root.clone()));
        std::thread::sleep(std::time::Duration::from_millis(20));
        export_with_the_subject_retimed(&root);
        let mut frames = 0;
        while subject_body_count_at(&mut sim, before + BUMP) < subjects {
            sim.step(common::base());
            frames += 1;
            assert!(frames < 600, "600 frames after the save the bodies still play the old move");
        }
        for _ in 0..10 {
            sim.step(common::base());
        }
        assert_eq!(boundary(&sim), "LocallyRebasable", "the maintainer started the timeline again");
        let (bound, live) = bound_and_live(&mut sim);
        assert_eq!(bound, live, "the new timeline binds the reloaded content");
        assert_ne!(bound, bound_before, "and that content is a new generation");
        sim.rollback_health().expect("the rebased timeline is healthy");
    }));
    let _ = std::fs::remove_dir_all(&root);
    if let Err(payload) = outcome {
        std::panic::resume_unwind(payload);
    }
}

/// Each live goblin's maximum HP.
#[cfg(not(feature = "static_content"))]
fn goblin_max_hps(sim: &mut ambition_sim_harness::Platformer2dSimHarness) -> Vec<i32> {
    let world = sim.world_mut();
    let mut q = world.query::<(
        &ambition_platformer2d::characters::actor::WornCharacter,
        &ambition_platformer2d::characters::actor::BodyHealth,
    )>();
    q.iter(world)
        .filter(|(worn, _)| worn.0.to_string() == "goblin")
        .map(|(_, health)| health.max())
        .collect()
}

/// The hurt material of every goblin the room built: a raw catalog fact (the
/// row's tags), not a prepared-cast one.
#[cfg(not(feature = "static_content"))]
fn goblin_materials(
    sim: &mut ambition_sim_harness::Platformer2dSimHarness,
) -> Vec<ambition_platformer2d::vfx::vfx::ImpactMaterial> {
    let world = sim.world_mut();
    let mut q = world.query::<(
        &ambition_platformer2d::characters::actor::WornCharacter,
        &ambition_platformer2d::combat::components::CombatTuning,
    )>();
    q.iter(world)
        .filter(|(worn, _)| worn.0.to_string() == "goblin")
        .map(|(_, tuning)| tuning.hurt_feedback.material)
        .collect()
}

/// The primary player's fastest upward speed in the frames after a jump press,
/// from standing.
#[cfg(not(feature = "static_content"))]
fn launch_speed(sim: &mut ambition_sim_harness::Platformer2dSimHarness) -> f32 {
    use ambition_platformer2d::engine_core::BodyKinematics;
    use ambition_platformer2d::platformer::markers::PrimaryPlayerOnly;
    let vertical = |sim: &mut ambition_sim_harness::Platformer2dSimHarness| {
        let world = sim.world_mut();
        let mut query = world.query_filtered::<&BodyKinematics, PrimaryPlayerOnly>();
        query.single(world).expect("the primary player").vel.y
    };
    // ⚠ A WALK FIRST. MEASURED 2026-10-01: an idle player in this room is hit
    // at frame 122 (60 -> 59 HP), and a press in the hitstun after it does not
    // launch. The walk moves the player out of that reach; the press then
    // launches at -555.
    for _ in 0..10 {
        sim.step(ambition_app::AgentAction { move_x: 1.0, right_pressed: true, ..common::base() });
    }
    for _ in 0..30 {
        sim.step(common::base());
    }
    // ⚠ AND THE PRESS WAITS UNTIL THE PLAYER CAN ACT. The room's hostile still
    // reaches the player now and then, and where it is on a given frame
    // moves with every published body size; a press inside hitstop or
    // hitstun launches nothing and reads as a tuning that did not apply.
    let actionable = |sim: &mut ambition_sim_harness::Platformer2dSimHarness| {
        use ambition_platformer2d::characters::actor::BodyCombat;
        let world = sim.world_mut();
        let mut query = world.query_filtered::<(&BodyCombat, &BodyKinematics), PrimaryPlayerOnly>();
        let (combat, kin) = query.single(world).expect("the primary player");
        combat.hitstop_timer <= 0.0 && combat.hitstun_timer <= 0.0 && kin.vel.y == 0.0
    };
    let mut waited = 0;
    while !actionable(sim) {
        sim.step(common::base());
        waited += 1;
        assert!(waited < 180, "the player was not free to act for 180 frames");
    }
    let resting = vertical(sim);
    sim.step(ambition_app::AgentAction { jump: true, jump_held: true, ..common::base() });
    let mut peak: f32 = 0.0;
    for _ in 0..4 {
        sim.step(ambition_app::AgentAction { jump_held: true, ..common::base() });
        peak = peak.max((vertical(sim) - resting).abs());
    }
    peak
}
