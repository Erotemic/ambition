use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("crate lives under <repo>/crates/ambition_sandbox")
        .to_path_buf()
}

fn crate_src() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
}

fn collect_rs_files(root: &Path) -> Vec<PathBuf> {
    let mut stack = vec![root.to_path_buf()];
    let mut files = Vec::new();
    while let Some(path) = stack.pop() {
        let Ok(meta) = fs::metadata(&path) else {
            continue;
        };
        if meta.is_dir() {
            let mut entries = fs::read_dir(&path)
                .expect("read directory")
                .map(|entry| entry.expect("directory entry").path())
                .collect::<Vec<_>>();
            entries.sort();
            stack.extend(entries.into_iter().rev());
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            files.push(path);
        }
    }
    files.sort();
    files
}

#[test]
fn architecture_boundaries_platformer_runtime_stays_content_free() {
    let root = crate_src().join("platformer_runtime");
    let forbidden = [
        "crate::content",
        "crate::ambition_content",
        "crate::intro",
        "crate::boss_encounter",
        "crate::quest",
        "crate::assets::sandbox_assets",
        "crate::music",
        "crate::items",
        "crate::app",
        "crate::dev",
        "crate::presentation",
    ];
    // `crate::portal` (the portal mechanic) is forbidden, but `crate::portal_pieces`
    // (the reusable Core portal-map math) is allowed — match the mechanic path with
    // an explicit boundary so the prefix does not false-positive on portal_pieces.
    let forbidden_boundary = ["crate::portal::", "crate::portal;", "crate::portal}"];

    let mut violations = Vec::new();
    for file in collect_rs_files(&root) {
        let text = fs::read_to_string(&file).expect("read rust source");
        for needle in forbidden {
            if text.contains(needle) {
                violations.push(format!("{} imports or mentions {needle}", file.display()));
            }
        }
        for needle in forbidden_boundary {
            if text.contains(needle) {
                violations.push(format!("{} imports or mentions {needle}", file.display()));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "platformer_runtime must remain reusable and content-free:\n{}",
        violations.join("\n")
    );
}

#[test]
fn architecture_boundaries_platformer_runtime_crate_is_extracted() {
    // Stage 13 / Task K: the import-clean proto-runtime seams (lifecycle
    // vocabulary + schedule sets) were extracted into the standalone
    // `ambition_platformer_runtime` crate, which compiles without
    // `ambition_sandbox`. Assert (a) the crate exists and is registered, (b) it
    // does NOT depend on the sandbox, (c) the moved modules now live in the new
    // crate (and not in the sandbox), and (d) the still-local remainder
    // (collision / orientation / transit) is documented as not-yet-extracted
    // because each still reaches back into the sandbox.
    let root = repo_root();
    let crate_root = root.join("crates/ambition_platformer_runtime");

    // (a) The crate exists with the extracted modules.
    assert!(
        crate_root.join("Cargo.toml").exists(),
        "ambition_platformer_runtime crate should exist at crates/ambition_platformer_runtime"
    );
    for rel in [
        "src/lib.rs",
        "src/prelude.rs",
        "src/schedule.rs",
        "src/lifecycle/mod.rs",
        "src/lifecycle/markers.rs",
        "src/lifecycle/spawn_ext.rs",
        "src/lifecycle/cleanup.rs",
    ] {
        assert!(
            crate_root.join(rel).exists(),
            "extracted platformer-runtime crate should include {rel}"
        );
    }

    // (b) The extracted crate must not depend on the sandbox (the whole point
    //     of the extraction: it is reusable and content-free).
    let crate_manifest =
        fs::read_to_string(crate_root.join("Cargo.toml")).expect("read crate manifest");
    // Scan dependency-bearing lines only (a `description` may name the sandbox
    // as the crate it was extracted from). A real dep line would contain
    // `ambition_sandbox =` or `ambition_sandbox.`.
    let depends_on_sandbox = crate_manifest.lines().any(|line| {
        let line = line.trim();
        line.starts_with("ambition_sandbox =") || line.starts_with("ambition_sandbox.")
    });
    assert!(
        !depends_on_sandbox,
        "ambition_platformer_runtime must not depend on ambition_sandbox"
    );

    // (c) The moved modules no longer live in the sandbox; the sandbox's
    //     platformer_runtime is a facade re-exporting the crate.
    let sandbox_runtime = crate_src().join("platformer_runtime");
    assert!(
        !sandbox_runtime.join("schedule.rs").exists(),
        "schedule.rs should have moved into the extracted crate"
    );
    assert!(
        !sandbox_runtime.join("lifecycle").exists(),
        "lifecycle/ should have moved into the extracted crate"
    );
    // Stage M1: pure portal-map math + the body-transit velocity helper moved
    // into the crate; transit.rs is gone from the sandbox and math/transit live
    // in the crate.
    assert!(
        !sandbox_runtime.join("transit.rs").exists(),
        "transit.rs should have moved into the extracted crate (Stage M1)"
    );
    assert!(
        crate_root.join("src/math.rs").exists(),
        "pure portal-map math should live in the extracted crate (Stage M1)"
    );
    assert!(
        crate_root.join("src/transit.rs").exists(),
        "transit.rs should live in the extracted crate (Stage M1)"
    );
    // Stage 16 / S1–S2: the generic solid-world raycast (SolidWorldQuery /
    // raycast_solids / ray_aabb + the engine_core::World adapter) and the
    // unified BodyKinematics re-export moved into the crate; the sandbox keeps
    // only thin facades.
    assert!(
        crate_root.join("src/world_query.rs").exists(),
        "solid-world raycast should live in the extracted crate (Stage 16 / S1)"
    );
    assert!(
        crate_root.join("src/body.rs").exists(),
        "unified BodyKinematics should live in the extracted crate (Stage 16 / S2)"
    );
    let facade = fs::read_to_string(sandbox_runtime.join("mod.rs")).expect("read facade mod.rs");
    assert!(
        facade.contains("ambition_platformer_runtime::{gravity, lifecycle, math, schedule, transit}"),
        "sandbox platformer_runtime facade should re-export the extracted crate's gravity + lifecycle + math + schedule + transit"
    );

    // (d) Stage 16 extracted the rest of the generic ECS runtime layer:
    //     gravity (S4) and orientation (S5) now live in the crate, so
    //     `crate::physics` and `crate::platformer_runtime::orientation` are
    //     facades. There is no not-yet-extracted remainder left under
    //     `platformer_runtime/` — every module there is a facade or adapter.
    assert!(
        crate_root.join("src/gravity.rs").exists(),
        "the gravity runtime should live in the extracted crate (Stage 16 / S4)"
    );
    assert!(
        crate_root.join("src/orientation.rs").exists(),
        "actor orientation should live in the extracted crate (Stage 16 / S5)"
    );
    // The sandbox-side modules are facades re-exporting the extracted crate.
    let orientation_facade = fs::read_to_string(sandbox_runtime.join("orientation.rs"))
        .expect("read orientation facade");
    assert!(
        orientation_facade.contains("ambition_platformer_runtime::orientation"),
        "sandbox orientation should re-export the extracted crate's orientation module"
    );
    let physics_facade = fs::read_to_string(crate_src().join("physics.rs")).expect("read physics");
    assert!(
        physics_facade.contains("ambition_platformer_runtime::gravity"),
        "crate::physics should be a facade re-exporting the extracted crate's gravity module"
    );
}

#[test]
fn architecture_boundaries_input_crate_is_extracted() {
    // ADR 0019: the device -> ControlFrame input layer (SandboxAction /
    // ControlFrame / MenuControlFrame / keyboard+gamepad presets + the
    // input-domain settings) was extracted into the standalone `ambition_input`
    // crate, which compiles without `ambition_sandbox`. Assert (a) the crate
    // exists with the moved modules, (b) it does NOT depend on the sandbox,
    // (c) the modules no longer live in the sandbox and `crate::input` /
    // `persistence::settings::controls` are facades over the crate.
    let root = repo_root();
    let crate_root = root.join("crates/ambition_input");

    // (a) The crate exists with the extracted modules.
    assert!(
        crate_root.join("Cargo.toml").exists(),
        "ambition_input crate should exist at crates/ambition_input"
    );
    for rel in [
        "src/lib.rs",
        "src/actions.rs",
        "src/control.rs",
        "src/menu.rs",
        "src/presets.rs",
        "src/settings.rs",
    ] {
        assert!(
            crate_root.join(rel).exists(),
            "extracted input crate should include {rel}"
        );
    }

    // (b) The extracted crate must not depend on the sandbox (the whole point:
    //     input is an upper sibling, decoupled from sandbox content).
    let crate_manifest =
        fs::read_to_string(crate_root.join("Cargo.toml")).expect("read input crate manifest");
    let depends_on_sandbox = crate_manifest.lines().any(|line| {
        let line = line.trim();
        line.starts_with("ambition_sandbox =") || line.starts_with("ambition_sandbox.")
    });
    assert!(
        !depends_on_sandbox,
        "ambition_input must not depend on ambition_sandbox"
    );

    // (c) The moved modules no longer live in the sandbox; `crate::input` and
    //     `persistence::settings::controls` are facades over the crate.
    assert!(
        !crate_src().join("input.rs").exists() && !crate_src().join("input").exists(),
        "the input module should have moved into the ambition_input crate"
    );
    assert!(
        !crate_src()
            .join("persistence/settings/controls.rs")
            .exists(),
        "input-domain controls settings should have moved into the ambition_input crate"
    );
    let lib = fs::read_to_string(crate_src().join("lib.rs")).expect("read sandbox lib.rs");
    assert!(
        lib.contains("pub use ambition_input as input"),
        "crate::input should be a facade re-exporting the ambition_input crate"
    );
    let settings_mod = fs::read_to_string(crate_src().join("persistence/settings/mod.rs"))
        .expect("read persistence settings mod.rs");
    assert!(
        settings_mod.contains("pub use ambition_input::settings as controls"),
        "persistence::settings::controls should re-export ambition_input::settings"
    );
}

fn raw_commands_spawn_count(text: &str) -> usize {
    text.match_indices("commands.spawn(").count()
}

fn read_spawn_allowlist() -> BTreeMap<String, usize> {
    let path = repo_root().join("docs/architecture/architecture-boundary-allowlist.txt");
    let text = fs::read_to_string(&path).expect("read architecture boundary allowlist");
    let mut allowlist = BTreeMap::new();
    for (idx, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((path, count)) = line.split_once('=') else {
            panic!("{}:{} expected path=count", path.display(), idx + 1);
        };
        let count = count
            .parse::<usize>()
            .unwrap_or_else(|_| panic!("{}:{} invalid count", path, idx + 1));
        allowlist.insert(path.to_string(), count);
    }
    allowlist
}

#[test]
fn architecture_boundaries_room_feature_spawns_do_not_add_raw_spawns() {
    let src_root = crate_src();
    let spawn_dir = src_root.join("content/features/ecs");
    let allowlist = read_spawn_allowlist();
    let mut violations = Vec::new();

    for file in collect_rs_files(&spawn_dir) {
        let Some(name) = file.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if !name.starts_with("spawn") {
            continue;
        }
        let text = fs::read_to_string(&file).expect("read spawn source");
        let rel = file
            .strip_prefix(&src_root)
            .expect("source file under src")
            .to_string_lossy()
            .replace('\\', "/");
        let actual = raw_commands_spawn_count(&text);
        let allowed = *allowlist.get(&rel).unwrap_or(&0);
        if actual > allowed {
            violations.push(format!(
                "{rel} has {actual} raw commands.spawn calls; allowed {allowed}. Use SpawnScopedExt lifecycle helpers or update docs/architecture/architecture-boundary-allowlist.txt with justification."
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "room-authored spawn modules gained raw spawn sites:\n{}",
        violations.join("\n")
    );
}

#[test]
fn architecture_boundaries_app_plugins_does_not_reown_moved_subsystems() {
    let path = crate_src().join("app/plugins.rs");
    let text = fs::read_to_string(&path).expect("read app/plugins.rs");
    let forbidden = [
        "fn register_portal_systems",
        "fn register_item_pickup_systems",
        "crate::portal::portal_fire_system",
        "crate::portal::portal_projectile_step",
        "crate::portal::portal_transit_system",
        "crate::item_pickup::pickup_held_item_system",
        "crate::item_pickup::throw_held_item_system",
        "crate::item_pickup::ground_item_physics",
    ];

    let violations = forbidden
        .into_iter()
        .filter(|needle| text.contains(needle))
        .collect::<Vec<_>>();

    assert!(
        violations.is_empty(),
        "app/plugins.rs reintroduced subsystem-owned registrations: {:?}",
        violations
    );
}

#[test]
fn architecture_boundaries_non_portal_mechanics_use_runtime_raycast_seam() {
    let src_root = crate_src();
    let checked_files = ["blink.rs", "dive.rs", "grapple.rs", "item_pickup.rs"];
    let mut violations = Vec::new();

    for rel in checked_files {
        let path = src_root.join(rel);
        let text = fs::read_to_string(&path).expect("read source file");
        if text.contains("crate::portal::raycast_solids") {
            violations.push(format!(
                "{rel} still reaches into portal for a generic solid raycast; use crate::platformer_runtime::collision::raycast_solids"
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "generic raycasts should live behind the proto-runtime seam:\n{}",
        violations.join("\n")
    );
}

#[test]
fn architecture_boundaries_portal_orders_against_item_set_not_function() {
    // Phase 2b (ADR 0019): the cross-set edges that tie portal to the host app
    // schedule moved OUT of portal/plugin.rs and INTO the sandbox wiring fn
    // `wire_portal_schedule` (crate::app::plugins) so `crate::portal` can become
    // a standalone crate. The portal plugin must no longer name the item set at
    // all; the sandbox declares the CoreHeldItems ordering for PortalSet::Transit.
    let portal_text = fs::read_to_string(crate_src().join("portal/plugin.rs"))
        .expect("read portal plugin source");
    let names_item_set_in_code = portal_text.lines().any(|raw| {
        let line = raw.trim();
        if line.starts_with("//") || line.starts_with("/*") || line.starts_with('*') {
            return false;
        }
        line.contains("crate::item_pickup")
    });
    assert!(
        !names_item_set_in_code,
        "portal/plugin.rs must not name the item subsystem in code; the host wires PortalSet::Transit against ItemPickupSet"
    );

    let wiring = fs::read_to_string(crate_src().join("app/plugins.rs"))
        .expect("read sandbox plugins source");
    assert!(
        !wiring.contains("after(crate::item_pickup::ground_item_physics)"),
        "the host should order PortalSet::Transit against crate::item_pickup::ItemPickupSet, not the concrete ground_item_physics function"
    );
    assert!(
        wiring.contains("crate::item_pickup::ItemPickupSet::CoreHeldItems"),
        "the sandbox portal wiring should order PortalSet::Transit on the held-item/ground-item simulation set"
    );
}

#[test]
fn architecture_boundaries_portal_core_does_not_import_ambition_content_roster() {
    // Reusable portal core (Stage 9 / Task H) must not depend on Ambition
    // content concepts: the item roster (`crate::items` / `Item::PortalGun` /
    // `OwnedItems`), the inventory menu, held-item equip glue
    // (`StashedActionSet`), quests, save schema, or the LDtk world schema. The
    // Ambition-specific input/inventory bindings live in
    // `crate::ambition_content::portal` adapters, which translate `ControlFrame`
    // and item state into the reusable portal intent/outcome messages.
    let root = crate_src().join("portal");

    // Hard-forbidden content roster: portal core must never name these. (No
    // allowlist — these were fully moved into the content adapter.)
    let forbidden = [
        "crate::items",
        "Item::PortalGun",
        "OwnedItems",
        "crate::inventory",
        "crate::oot_menu",
        "StashedActionSet",
        "crate::content",
        "crate::quest",
        "crate::ldtk_world",
        "crate::world::ldtk_world",
        "crate::persistence",
        // Stage Q / Task H2: portal transit + presentation no longer import the
        // Ambition input frame or the ground-item body. Transit reads a
        // content-agnostic `PlayerMovementIntent` + `PortalTransitable`, and the
        // held-gun presentation reads a `PortalAimHint`; the content adapter
        // (`crate::ambition_content::portal`) owns the `ControlFrame` / `GroundItem`
        // glue. (The `crate::item_pickup::ItemPickupSet` schedule label remains
        // allowlisted below — it is an ordering label, not a content concept.)
        "crate::input::ControlFrame",
        "crate::item_pickup::GroundItem",
    ];

    // ALLOWLIST — genuinely-shared low-level couplings that remain in portal
    // core for now, each with a tracked reason. These are NOT the item roster:
    //
    //   crate::item_pickup::ItemPickupSet
    //     - plugin.rs: a *schedule ordering label* (not a content concept);
    //       already guarded by `architecture_boundaries_portal_orders_against_item_set_not_function`.
    //
    // (Stage Q / Task H2 removed the former `crate::input::ControlFrame` and
    // `crate::item_pickup::GroundItem` entries: transit + presentation now read
    // the content-agnostic `PlayerMovementIntent` / `PortalTransitable` /
    // `PortalAimHint`, and those names are now hard-forbidden above.)
    let allow = |line: &str, is_test: bool| -> bool {
        line.contains("crate::item_pickup::ItemPickupSet")
            || line.contains("crate::item_pickup::axe_spec") // test fixture only
            // Portal-core tests (tests.rs) legitimately build the Ambition
            // `GroundItem` fixture and drive the full content+core transit chain
            // through the `crate::ambition_content::portal` adapters; those names
            // are NOT used in non-test core code (asserted by the absence of
            // hits in every other file).
            || (is_test && line.contains("crate::item_pickup::GroundItem"))
            // tests.rs drives the warp through the full content+core chain on the
            // `ControlFrame` surface (the content adapter mirrors it to/from the
            // movement intent); core (non-test) code reads `PlayerMovementIntent`.
            || (is_test && line.contains("crate::input::ControlFrame"))
    };

    let mut violations = Vec::new();
    for file in collect_rs_files(&root) {
        let is_test = file
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n == "tests.rs");
        let text = fs::read_to_string(&file).expect("read portal source");
        for (idx, raw) in text.lines().enumerate() {
            let line = raw.trim();
            // Skip comments / doc-comments: they legitimately reference the
            // adapter module path by name.
            if line.starts_with("//") || line.starts_with("/*") || line.starts_with('*') {
                continue;
            }
            if allow(line, is_test) {
                continue;
            }
            for needle in forbidden {
                if line.contains(needle) {
                    // Non-test core code triggers any forbidden name; tests may
                    // build the documented fixtures (axe_spec / GroundItem),
                    // which are allowlisted above, but anything else in tests is
                    // still a genuine violation.
                    violations.push(format!(
                        "{}:{} portal core references Ambition content `{needle}`: {line}",
                        file.display(),
                        idx + 1
                    ));
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "portal core must consume reusable messages/components, not the Ambition content roster. \
Move the binding into crate::ambition_content::portal (or extend the documented allowlist with a reason):\n{}",
        violations.join("\n")
    );
}

#[test]
fn architecture_boundaries_named_content_registers_through_content_plugin() {
    // Stage 11 / Task J: named Ambition content registration is owned by
    // `crate::ambition_content::AmbitionContentPlugin`. Assert (a) the
    // content boundary exists with the composer + per-content submodules,
    // (b) the app assembly installs the content plugin, and (c) the named
    // rosters are no longer constructed inline in `app/sim_resources.rs`.
    let src_root = crate_src();

    // (a) The content boundary owns the registration files.
    let expected = [
        "ambition_content/mod.rs",
        "ambition_content/plugin.rs",
        "ambition_content/quests/mod.rs",
        "ambition_content/bosses/mod.rs",
        "ambition_content/dialogue/mod.rs",
        "ambition_content/items/mod.rs",
    ];
    for rel in expected {
        assert!(
            src_root.join(rel).exists(),
            "named Ambition content boundary should include {rel}"
        );
    }

    let plugin_text = fs::read_to_string(src_root.join("ambition_content/plugin.rs"))
        .expect("read ambition_content/plugin.rs");
    assert!(
        plugin_text.contains("pub struct AmbitionContentPlugin"),
        "ambition_content/plugin.rs should define AmbitionContentPlugin"
    );
    // The composer must actually compose the named-content registrations.
    for needle in [
        "AmbitionQuestContentPlugin",
        "AmbitionBossContentPlugin",
        "AmbitionDialogueContentPlugin",
        "crate::intro::IntroPlugin",
    ] {
        assert!(
            plugin_text.contains(needle),
            "AmbitionContentPlugin should compose {needle}"
        );
    }

    // (b) App assembly installs the content plugin instead of registering
    //     named content directly.
    let plugins_text =
        fs::read_to_string(src_root.join("app/plugins.rs")).expect("read app/plugins.rs");
    assert!(
        plugins_text.contains("crate::ambition_content::AmbitionContentPlugin"),
        "app/plugins.rs should install AmbitionContentPlugin"
    );

    // (c) The moved named-content rosters no longer get constructed inline in
    //     the simulation-resources plugin; they flow through the content
    //     boundary now. (Runtime music channels / empty registries populated
    //     from LDtk are mechanic state, not named content, and stay put.)
    let sim_resources_text = fs::read_to_string(src_root.join("app/sim_resources.rs"))
        .expect("read app/sim_resources.rs");
    let forbidden_inline = [
        "QuestRegistry::default()",
        "BossEncounterRegistry::default()",
        "default_cutscene_library()",
        "RoomCutsceneBindings::defaults()",
        "install_boss_banter",
        "install_pirate_banter",
        "crate::intro::IntroPlugin",
    ];
    let violations = forbidden_inline
        .into_iter()
        .filter(|needle| sim_resources_text.contains(needle))
        .collect::<Vec<_>>();
    assert!(
        violations.is_empty(),
        "app/sim_resources.rs still constructs named content inline (should move behind \
AmbitionContentPlugin): {violations:?}"
    );

    // The starter item roster moved into the content boundary; app/plugins.rs
    // should no longer construct it inline.
    assert!(
        !plugins_text.contains("crate::items::OwnedItems::starter()"),
        "app/plugins.rs should install the item roster via \
crate::ambition_content::items::AmbitionItemRosterPlugin, not inline OwnedItems::starter()"
    );
}

#[test]
fn architecture_boundaries_gravity_zone_mechanic_left_portal() {
    // Stage 6 follow-up (ADR 0019): the gravity-zone MECHANIC (the zones /
    // switches that flip ambient gravity + their visuals) was extracted out of
    // `crate::portal` into `crate::mechanics::gravity`, which owns its own
    // `GravityPlugin`. Assert (a) the new module exists with its files, and
    // (b) the portal module no longer names the gravity-zone-mechanic symbols.
    let src_root = crate_src();

    // (a) The gravity mechanic owns its files.
    for rel in [
        "mechanics/mod.rs",
        "mechanics/gravity/mod.rs",
        "mechanics/gravity/plugin.rs",
        "mechanics/gravity/lifecycle.rs",
        "mechanics/gravity/presentation.rs",
    ] {
        assert!(
            src_root.join(rel).exists(),
            "extracted gravity mechanic should include {rel}"
        );
    }
    let gravity_plugin = fs::read_to_string(src_root.join("mechanics/gravity/plugin.rs"))
        .expect("read gravity plugin");
    assert!(
        gravity_plugin.contains("pub struct GravityPlugin"),
        "gravity mechanic should own a GravityPlugin"
    );
    // The gravity mechanic must not depend on portal. Match the mechanic path
    // with explicit boundaries (so `crate::portal_pieces` does not false-
    // positive) and skip comment lines (the module doc legitimately names the
    // `crate::portal` it was extracted from).
    let portal_boundaries = [
        "crate::portal::",
        "crate::portal;",
        "crate::portal}",
        "crate::portal,",
        "crate::portal ",
    ];
    for file in collect_rs_files(&src_root.join("mechanics/gravity")) {
        let text = fs::read_to_string(&file).expect("read gravity source");
        for (idx, raw) in text.lines().enumerate() {
            let line = raw.trim();
            if line.starts_with("//") || line.starts_with("/*") || line.starts_with('*') {
                continue;
            }
            for needle in portal_boundaries {
                assert!(
                    !line.contains(needle),
                    "{}:{} references {needle} — the gravity mechanic must be portal-independent: {line}",
                    file.display(),
                    idx + 1
                );
            }
        }
    }

    // (b) Portal no longer owns the gravity-zone mechanic symbols.
    let forbidden_in_portal = [
        "GravityFlipSwitch",
        "gravity_flip_switch_system",
        "GravityZoneVisual",
        "GravitySwitchVisual",
        "sync_gravity_zone_visual",
        "sync_gravity_switch_visual",
        "reset_gravity_on_room_reset",
    ];
    let mut violations = Vec::new();
    for file in collect_rs_files(&src_root.join("portal")) {
        // tests.rs legitimately exercises the moved system through its new
        // `crate::mechanics::gravity` path (a gravity-mechanic unit test that
        // still lives beside the portal test helpers).
        let is_test = file
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n == "tests.rs");
        let text = fs::read_to_string(&file).expect("read portal source");
        for (idx, raw) in text.lines().enumerate() {
            let line = raw.trim();
            if line.starts_with("//") || line.starts_with("/*") || line.starts_with('*') {
                continue;
            }
            // Allow the test file to reference the moved symbols via the new
            // `crate::mechanics::gravity` path only.
            if is_test && line.contains("crate::mechanics::gravity") {
                continue;
            }
            for needle in forbidden_in_portal {
                if line.contains(needle) {
                    violations.push(format!(
                        "{}:{} portal still references gravity-zone-mechanic symbol `{needle}`: {line}",
                        file.display(),
                        idx + 1
                    ));
                }
            }
        }
    }
    assert!(
        violations.is_empty(),
        "the gravity-zone mechanic must fully leave crate::portal (it lives in crate::mechanics::gravity now):\n{}",
        violations.join("\n")
    );
}

#[test]
fn architecture_boundaries_portal_has_facade_plugin_and_schedule_files() {
    let src_root = crate_src();
    let expected = [
        "portal/mod.rs",
        "portal/plugin.rs",
        "portal/schedule.rs",
        // Stage 7 split the old monolithic `portal/implementation.rs` into
        // responsibility submodules behind the facade.
        "portal/color.rs",
        "portal/types.rs",
        "portal/gun.rs",
        "portal/pickup.rs",
        "portal/shot.rs",
        "portal/placement.rs",
        "portal/transit.rs",
        "portal/lifecycle.rs",
        "portal/presentation.rs",
    ];

    for rel in expected {
        assert!(
            src_root.join(rel).exists(),
            "portal module split should include {rel}"
        );
    }

    assert!(
        !src_root.join("portal.rs").exists(),
        "remove crates/ambition_sandbox/src/portal.rs after applying the overlay so Rust does not see both portal.rs and portal/mod.rs"
    );

    let mod_text = fs::read_to_string(src_root.join("portal/mod.rs")).expect("read portal facade");
    assert!(
        mod_text.contains("pub use plugin::{PortalPlugin, PortalSimulationPlugin}"),
        "portal facade should re-export the top-level and simulation portal plugins"
    );
    assert!(
        mod_text.contains("pub use schedule::PortalSet"),
        "portal facade should expose portal-owned schedule vocabulary"
    );

    let plugin_text =
        fs::read_to_string(src_root.join("portal/plugin.rs")).expect("read portal plugin");
    assert!(
        plugin_text.contains("PortalSet::Transit"),
        "portal plugin should label transit systems with PortalSet"
    );
}
