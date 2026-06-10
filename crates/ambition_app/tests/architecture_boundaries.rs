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

/// The machinery lib's source root (most boundaries guard the
/// `ambition_sandbox` lib; this test crate lives in `ambition_app`).
fn crate_src() -> PathBuf {
    repo_root().join("crates/ambition_sandbox/src")
}

/// The app-assembly crate's source root.
fn app_src() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
}

/// The named-content crate's source root.
fn content_src() -> PathBuf {
    repo_root().join("crates/ambition_content/src")
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
    // Scans SANDBOX-LIB code: vocabulary needles are `crate::…` (in-lib
    // paths). (An earlier path-rewrite sed briefly flipped these to
    // `ambition_sandbox::…`, which can never occur inside the lib —
    // restored 2026-06-10.)
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
    // `ambition_sandbox::portal` (the portal mechanic, including its `pieces` Core math
    // submodule) is forbidden — match the mechanic path with explicit boundaries.
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
    //     `ambition_sandbox::physics` and `ambition_sandbox::platformer_runtime::orientation` are
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
        "ambition_sandbox::physics should be a facade re-exporting the extracted crate's gravity module"
    );
}

#[test]
fn architecture_boundaries_menu_crate_stays_content_free() {
    // Phase D (unified menu refactor): the reusable engine menu crate
    // `ambition_menu` (the page-model vocabulary + the bevy_ui grid + 3D cube
    // RENDERERS) must NOT depend on the sandbox or its game content. The ONE menu
    // content model + settings IR + dispatcher live in the SANDBOX
    // (`ambition_sandbox::menu::*`); the crate only provides the renderer-agnostic model
    // types and the two presentations. If the crate grew a sandbox dependency,
    // the "reusable renderer, game owns the content" boundary would be broken.
    //
    // This replaces no prior guard: the deleted `pause_menu`, legacy
    // `inventory::ui`, and `bevy_ui_grid_menu` modules were never named by any
    // architecture guard (so there was nothing to remove/repoint), and the menu
    // is now unconditional. This adds the meaningful new invariant the refactor
    // creates.
    let root = repo_root();
    let crate_root = root.join("crates/ambition_menu");

    assert!(
        crate_root.join("Cargo.toml").exists(),
        "ambition_menu crate should exist at crates/ambition_menu"
    );

    // (a) The manifest must not depend on the sandbox.
    let crate_manifest =
        fs::read_to_string(crate_root.join("Cargo.toml")).expect("read ambition_menu manifest");
    let depends_on_sandbox = crate_manifest.lines().any(|line| {
        let line = line.trim();
        line.starts_with("ambition_sandbox =") || line.starts_with("ambition_sandbox.")
    });
    assert!(
        !depends_on_sandbox,
        "ambition_menu must not depend on ambition_sandbox (it is the reusable renderer; \
         the game owns the menu CONTENT in ambition_sandbox::menu::*)"
    );

    // (b) No source file may reach back into the sandbox or its game content.
    //     The crate is content-agnostic: it is generic over the host's page-id /
    //     action types, so it never names `ambition_sandbox::items`, the sandbox's settings
    //     IR, etc. A `use ambition_sandbox` (or a stray game-content path) would
    //     mean the boundary leaked.
    let mut violations = Vec::new();
    for file in collect_rs_files(&crate_root.join("src")) {
        let text = fs::read_to_string(&file).expect("read ambition_menu source");
        for (idx, raw) in text.lines().enumerate() {
            let line = raw.trim();
            if line.starts_with("//") || line.starts_with("/*") || line.starts_with('*') {
                continue;
            }
            if line.contains("ambition_sandbox") {
                violations.push(format!(
                    "{}:{} ambition_menu reaches into the sandbox: {line}",
                    file.display(),
                    idx + 1
                ));
            }
        }
    }
    assert!(
        violations.is_empty(),
        "ambition_menu must stay content-free (no ambition_sandbox references):\n{}",
        violations.join("\n")
    );
}

#[test]
fn architecture_boundaries_input_crate_is_extracted() {
    // ADR 0019: the device -> ControlFrame input layer (SandboxAction /
    // ControlFrame / MenuControlFrame / keyboard+gamepad presets + the
    // input-domain settings) was extracted into the standalone `ambition_input`
    // crate, which compiles without `ambition_sandbox`. Assert (a) the crate
    // exists with the moved modules, (b) it does NOT depend on the sandbox,
    // (c) the modules no longer live in the sandbox and `ambition_sandbox::input` /
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

    // (c) The moved modules no longer live in the sandbox; `ambition_sandbox::input` and
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
        "ambition_sandbox::input should be a facade re-exporting the ambition_input crate"
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
    let path = app_src().join("app/plugins.rs");
    let text = fs::read_to_string(&path).expect("read app/plugins.rs");
    let forbidden = [
        "fn register_portal_systems",
        "fn register_item_pickup_systems",
        "ambition_sandbox::portal::portal_fire_system",
        "ambition_sandbox::portal::portal_projectile_step",
        "ambition_sandbox::portal::portal_transit",
        "ambition_sandbox::item_pickup::pickup_held_item_system",
        "ambition_sandbox::item_pickup::throw_held_item_system",
        "ambition_sandbox::item_pickup::ground_item_physics",
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
    let checked_files = [
        "abilities/traversal/blink.rs",
        "abilities/traversal/dive.rs",
        "abilities/traversal/grapple.rs",
        "items/pickup.rs",
    ];
    let mut violations = Vec::new();

    for rel in checked_files {
        let path = src_root.join(rel);
        let text = fs::read_to_string(&path).expect("read source file");
        if text.contains("crate::portal::raycast_solids") {
            violations.push(format!(
                "{rel} still reaches into portal for a generic solid raycast; use ambition_sandbox::platformer_runtime::collision::raycast_solids"
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
    // `wire_portal_schedule` (crate::app::plugins) so `ambition_sandbox::portal` can become
    // a standalone crate. The portal plugin must no longer name the item set at
    // all; the sandbox declares the CoreHeldItems ordering for PortalSet::Transit.
    let portal_text = fs::read_to_string(repo_root().join("crates/ambition_portal/src/plugin.rs"))
        .expect("read portal plugin source");
    let names_item_set_in_code = portal_text.lines().any(|raw| {
        let line = raw.trim();
        if line.starts_with("//") || line.starts_with("/*") || line.starts_with('*') {
            return false;
        }
        line.contains("ambition_sandbox::items::pickup")
    });
    assert!(
        !names_item_set_in_code,
        "portal/plugin.rs must not name the item subsystem in code; the host wires PortalSet::Transit against ItemPickupSet"
    );

    let wiring =
        fs::read_to_string(app_src().join("app/plugins.rs")).expect("read sandbox plugins source");
    assert!(
        !wiring.contains("after(ambition_sandbox::items::pickup::ground_item_physics)"),
        "the host should order PortalSet::Transit against ambition_sandbox::items::pickup::ItemPickupSet, not the concrete ground_item_physics function"
    );
    assert!(
        wiring.contains("ambition_sandbox::items::pickup::ItemPickupSet::CoreHeldItems"),
        "the sandbox portal wiring should order PortalSet::Transit on the held-item/ground-item simulation set"
    );
}

#[test]
fn architecture_boundaries_portal_core_does_not_import_ambition_content_roster() {
    // Reusable portal core (Stage 9 / Task H) must not depend on Ambition
    // content concepts: the item roster (`ambition_sandbox::items` / `Item::PortalGun` /
    // `OwnedItems`), the inventory menu, held-item equip glue
    // (`StashedActionSet`), quests, save schema, or the LDtk world schema. The
    // Ambition-specific input/inventory bindings live in
    // `ambition_content::portal` adapters, which translate `ControlFrame`
    // and item state into the reusable portal intent/outcome messages.
    //
    // Stage 19 Phase 5b: the mechanic moved into `ambition_portal`; the facade +
    // presentation + integration tests stay sandbox-side. Scan BOTH so the
    // content-roster ban covers the crate AND the sandbox-side portal files.
    let roots = [
        repo_root().join("crates/ambition_portal/src"),
        crate_src().join("portal"),
    ];

    // Hard-forbidden content roster: portal core must never name these. (No
    // allowlist — these were fully moved into the content adapter.)
    let forbidden = [
        "ambition_sandbox::items",
        "Item::PortalGun",
        "OwnedItems",
        "ambition_sandbox::inventory",
        "ambition_sandbox::menu::effects",
        "StashedActionSet",
        "ambition_sandbox::content",
        "ambition_sandbox::quest",
        "ambition_sandbox::ldtk_world",
        "ambition_sandbox::world::ldtk_world",
        "ambition_sandbox::persistence",
        // Stage Q / Task H2: portal transit + presentation no longer import the
        // Ambition input frame or the ground-item body. Transit reads a
        // content-agnostic `PlayerMovementIntent` + `PortalTransitable`, and the
        // held-gun presentation reads a `PortalAimHint`; the content adapter
        // (`ambition_content::portal`) owns the `ControlFrame` / `GroundItem`
        // glue. (The `ambition_sandbox::item_pickup::ItemPickupSet` schedule label remains
        // allowlisted below — it is an ordering label, not a content concept.)
        "ambition_sandbox::input::ControlFrame",
        "ambition_sandbox::items::pickup::GroundItem",
    ];

    // ALLOWLIST — genuinely-shared low-level couplings that remain in portal
    // core for now, each with a tracked reason. These are NOT the item roster:
    //
    //   ambition_sandbox::item_pickup::ItemPickupSet
    //     - plugin.rs: a *schedule ordering label* (not a content concept);
    //       already guarded by `architecture_boundaries_portal_orders_against_item_set_not_function`.
    //
    // (Stage Q / Task H2 removed the former `ambition_sandbox::input::ControlFrame` and
    // `ambition_sandbox::item_pickup::GroundItem` entries: transit + presentation now read
    // the content-agnostic `PlayerMovementIntent` / `PortalTransitable` /
    // `PortalAimHint`, and those names are now hard-forbidden above.)
    let allow = |line: &str, is_test: bool| -> bool {
        line.contains("ambition_sandbox::items::pickup::ItemPickupSet")
            || line.contains("ambition_sandbox::items::pickup::axe_spec") // test fixture only
            // Portal-core tests (tests.rs) legitimately build the Ambition
            // `GroundItem` fixture and drive the full content+core transit chain
            // through the `ambition_content::portal` adapters; those names
            // are NOT used in non-test core code (asserted by the absence of
            // hits in every other file).
            || (is_test && line.contains("ambition_sandbox::items::pickup::GroundItem"))
            // tests.rs drives the warp through the full content+core chain on the
            // `ControlFrame` surface (the content adapter mirrors it to/from the
            // movement intent); core (non-test) code reads `PlayerMovementIntent`.
            || (is_test && line.contains("ambition_sandbox::input::ControlFrame"))
    };

    let mut violations = Vec::new();
    for file in roots.iter().flat_map(|r| collect_rs_files(r)) {
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
Move the binding into ambition_content::portal (or extend the documented allowlist with a reason):\n{}",
        violations.join("\n")
    );
}

#[test]
fn architecture_boundaries_portal_core_does_not_name_host_world_or_reset() {
    // Stage 19 Phase 2: the portal API-decoupling seams. Portal CORE
    // (non-test, non-render-gated) must no longer name the host's concrete
    // collision world, its feature-overlay carve sink, the Ambition room-reset
    // event, or the Ambition input frame — those moved behind portal-owned
    // resources/messages bridged by `ambition_content::portal`:
    //
    //   * Seam 1 — carve output is the portal-owned `PortalCarves` resource; the
    //     Ambition `bridge_portal_carves` copies it into `FeatureEcsWorldOverlay`.
    //   * Seam 2 — the shot stepper is a pure `step_portal_shot` over
    //     `SolidWorldQuery`; the `Res<GameWorld>` read lives in the Ambition
    //     `portal_projectile_step` adapter.
    //   * Seam 3 — the core fire system consumes a generic `PortalFireIntent`;
    //     the Ambition `resolve_portal_fire_intent` maps the gesture to it.
    //   * Seam 4 — reset is the portal-owned `ClearPortals` message; the Ambition
    //     `bridge_room_reset_to_clear_portals` emits it from
    //     `ResetRoomFeaturesEvent`.
    //
    // `presentation.rs` is render-gated (Phase 5 territory) and still reads
    // `GameWorld` for its visuals, so it is excluded; `tests.rs` legitimately
    // builds host fixtures and drives the full content+core chain, so it is
    // excluded too.
    //
    // Stage 19 Phase 5a decoupled the last `ambition_sandbox::player` residue (gun
    // color/dev toggle → Ambition adapter; ledge-grab suppression + input-warp →
    // Ambition adapters reacting to portal events), and Phase 5b moved the
    // mechanic into the `ambition_portal` crate — which physically cannot name a
    // host `ambition_sandbox::…` path. The scan now targets the crate source; the sandbox
    // facade/presentation/tests are out of scope (presentation is render-gated;
    // tests drive the full content+core chain). `ambition_sandbox::player` is added to the
    // forbidden set now that the residue is gone.
    let root = repo_root().join("crates/ambition_portal/src");
    let forbidden = [
        "ambition_sandbox::features",
        "ambition_sandbox::GameWorld",
        "Res<GameWorld>",
        "FeatureEcsWorldOverlay",
        "ResetRoomFeaturesEvent",
        "ambition_sandbox::input::ControlFrame",
        "ambition_sandbox::player",
    ];

    let mut violations = Vec::new();
    for file in collect_rs_files(&root) {
        let name = file.file_name().and_then(|n| n.to_str()).unwrap_or("");
        // Render-gated presentation + the test module are out of scope for the
        // Phase 2 core-freedom assertion.
        if name == "tests.rs" || name == "presentation.rs" {
            continue;
        }
        let text = fs::read_to_string(&file).expect("read portal source");
        for (idx, raw) in text.lines().enumerate() {
            let line = raw.trim();
            // Comments legitimately reference these names while documenting the
            // seam (e.g. "core never names FeatureEcsWorldOverlay").
            if line.starts_with("//") || line.starts_with("/*") || line.starts_with('*') {
                continue;
            }
            for needle in forbidden {
                if line.contains(needle) {
                    violations.push(format!(
                        "{}:{} portal core still names host concept `{needle}`: {line}",
                        file.display(),
                        idx + 1
                    ));
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "portal core must reach the host collision world / carve sink / room-reset / input \
through portal-owned resources & messages bridged by ambition_content::portal \
(Stage 19 Phase 2). Move the binding into the content adapter:\n{}",
        violations.join("\n")
    );
}

#[test]
fn architecture_boundaries_music_director_is_content_agnostic() {
    // Stage 18 T3: the music director machinery (track switching, crossfade,
    // adaptive layering) is content-agnostic. It must not reach into Ambition
    // gameplay content: encounters, rooms, or the sandbox content data spec.
    // The Ambition mapping ("which cue/track for which encounter/boss/room")
    // lives in `ambition_sandbox::music::intent` (the content half), which resolves a
    // neutral `MusicIntent` the director consumes. A different game would
    // supply its own `compute_music_intent` and reuse the director unchanged.
    //
    // `ambition_sandbox::audio` (the playback backend) and `ambition_sandbox::persistence::settings`
    // (one music-volume float) are deliberately NOT forbidden: they are the
    // legitimate downward dependency direction the in-place decoupling leaves
    // for a future crate extraction to abstract.
    let music_root = crate_src().join("music");
    let forbidden = ["crate::encounter", "crate::rooms", "crate::content"];
    let mut violations = Vec::new();
    for file in collect_rs_files(&music_root) {
        let name = file.file_name().and_then(|n| n.to_str()).unwrap_or("");
        // `intent.rs` IS the content half (it owns the encounter/room/content
        // mapping); `tests.rs` exercises that mapping. Everything else is the
        // reusable director and must stay content-free.
        if name == "intent.rs" || name == "tests.rs" {
            continue;
        }
        let text = fs::read_to_string(&file).expect("read music source");
        for (idx, raw) in text.lines().enumerate() {
            let line = raw.trim();
            if line.starts_with("//") || line.starts_with("/*") || line.starts_with('*') {
                continue;
            }
            for needle in forbidden {
                if line.contains(needle) {
                    violations.push(format!(
                        "{}:{} music director references Ambition content `{needle}`: {line}",
                        file.display(),
                        idx + 1
                    ));
                }
            }
        }
    }
    assert!(
        violations.is_empty(),
        "music director must consume the neutral `MusicIntent`, not encounter/room/content. \
Move the mapping into ambition_sandbox::music::intent:\n{}",
        violations.join("\n")
    );
}

#[test]
fn architecture_boundaries_named_content_registers_through_content_plugin() {
    // Stage 11 / Task J: named Ambition content registration is owned by
    // `ambition_content::AmbitionContentPlugin`. Assert (a) the
    // content boundary exists with the composer + per-content submodules,
    // (b) the app assembly installs the content plugin, and (c) the named
    // rosters are no longer constructed inline in `app/sim_resources.rs`.
    let src_root = crate_src();

    // (a) The content boundary owns the registration files. (Stage 20 / A1
    //     unified `ambition_content/` into `content/`; `ambition_sandbox::ambition_content`
    //     is now an alias of `ambition_sandbox::content`.)
    let expected = [
        "lib.rs",
        "plugin.rs",
        "quests/mod.rs",
        "bosses/mod.rs",
        "dialogue/mod.rs",
        "items/mod.rs",
    ];
    for rel in expected {
        assert!(
            content_src().join(rel).exists(),
            "named Ambition content crate should include {rel}"
        );
    }

    let plugin_text =
        fs::read_to_string(content_src().join("plugin.rs")).expect("read content plugin.rs");
    assert!(
        plugin_text.contains("pub struct AmbitionContentPlugin"),
        "content/plugin.rs should define AmbitionContentPlugin"
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
        fs::read_to_string(app_src().join("app/plugins.rs")).expect("read app/plugins.rs");
    assert!(
        plugins_text.contains("ambition_content::AmbitionContentPlugin"),
        "app/plugins.rs should install AmbitionContentPlugin"
    );

    // (c) The moved named-content rosters no longer get constructed inline in
    //     the simulation-resources plugin; they flow through the content
    //     boundary now. (Runtime music channels / empty registries populated
    //     from LDtk are mechanic state, not named content, and stay put.)
    let sim_resources_text = fs::read_to_string(app_src().join("app/sim_resources.rs"))
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
        !plugins_text.contains("ambition_sandbox::items::OwnedItems::starter()"),
        "app/plugins.rs should install the item roster via \
ambition_content::items::AmbitionItemRosterPlugin, not inline OwnedItems::starter()"
    );
}

#[test]
fn architecture_boundaries_gravity_zone_mechanic_left_portal() {
    // Stage 6 follow-up (ADR 0019): the gravity-zone MECHANIC (the zones /
    // switches that flip ambient gravity + their visuals) was extracted out of
    // `ambition_sandbox::portal` into `ambition_sandbox::mechanics::gravity`, which owns its own
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
    // with explicit boundaries and skip comment lines (the module doc
    // legitimately names the `ambition_sandbox::portal` it was extracted from).
    let portal_boundaries = [
        "ambition_sandbox::portal::",
        "ambition_sandbox::portal;",
        "ambition_sandbox::portal}",
        "ambition_sandbox::portal,",
        "ambition_sandbox::portal ",
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
        // `ambition_sandbox::mechanics::gravity` path (a gravity-mechanic unit test that
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
            // `ambition_sandbox::mechanics::gravity` path only.
            if is_test && line.contains("ambition_sandbox::mechanics::gravity") {
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
        "the gravity-zone mechanic must fully leave ambition_sandbox::portal (it lives in ambition_sandbox::mechanics::gravity now):\n{}",
        violations.join("\n")
    );
}

#[test]
fn architecture_boundaries_abilities_live_under_abilities_layer() {
    // Stage 17 (content/ability boundary run): the 14 loose player ability /
    // weapon mechanics that used to clutter the crate root each got one clear
    // home under `ambition_sandbox::abilities/{traversal,ranged,thrown}/`, composed behind a
    // single `AmbitionAbilitiesPlugin`. This guard locks in the navigability win:
    //   (1) none of the 14 names may exist as a loose `src/<name>.rs` at the root,
    //       and each must exist under `src/abilities/`; and
    //   (2) the abilities layer is registered as exactly one plugin
    //       (`ambition_sandbox::abilities::AmbitionAbilitiesPlugin`) in `app/plugins.rs`.
    // If anyone re-adds `src/blink.rs` (or a sibling) at the root, or wires the
    // abilities individually instead of through the umbrella plugin, this fails.
    let src_root = crate_src();

    // 14 ability modules, each mapped to its destination subdir under abilities/.
    let abilities: [(&str, &str); 14] = [
        ("blink", "traversal"),
        ("dive", "traversal"),
        ("grapple", "traversal"),
        ("possession", "traversal"),
        ("mark_recall", "traversal"),
        ("beam", "ranged"),
        ("meteor", "ranged"),
        ("shockwave", "ranged"),
        ("vortex", "ranged"),
        ("volley", "ranged"),
        ("bomb", "ranged"),
        ("sentry", "ranged"),
        ("gravity_grenade", "thrown"),
        ("puppy_slug_gun", "thrown"),
    ];

    let mut violations = Vec::new();
    for (name, subdir) in abilities {
        // (1a) No loose ability file at the crate root.
        if src_root.join(format!("{name}.rs")).exists() {
            violations.push(format!(
                "ability {name} must live under src/abilities/, not the crate root (Stage 17)"
            ));
        }
        // (1b) The ability now lives under src/abilities/<subdir>/<name>.rs.
        let home = src_root.join(format!("abilities/{subdir}/{name}.rs"));
        if !home.exists() {
            violations.push(format!(
                "ability {name} must live under src/abilities/{subdir}/{name}.rs (Stage 17)"
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "the 14 player abilities must live under src/abilities/, not the crate root:\n{}",
        violations.join("\n")
    );

    // (2) The abilities layer is registered as exactly one plugin: the umbrella
    //     `AmbitionAbilitiesPlugin`. The 14 abilities compose through it, so
    //     `app/plugins.rs` adds that one plugin (no per-ability re-wiring).
    let plugins_text =
        fs::read_to_string(app_src().join("app/plugins.rs")).expect("read app/plugins.rs");
    assert!(
        plugins_text.contains("ambition_sandbox::abilities::AmbitionAbilitiesPlugin"),
        "app/plugins.rs should compose the ability layer through the single \
ambition_sandbox::abilities::AmbitionAbilitiesPlugin (Stage 17)"
    );
}

#[test]
fn architecture_boundaries_portal_has_facade_plugin_and_schedule_files() {
    // Stage 19 Phase 5b (ADR 0019): the portal MECHANIC moved into the standalone
    // `ambition_portal` crate; only the render-gated facade + presentation +
    // integration tests stay sandbox-side.
    let src_root = crate_src();
    let portal_crate_src = repo_root().join("crates/ambition_portal/src");
    let mechanic_files = [
        "lib.rs",
        "plugin.rs",
        "schedule.rs",
        "color.rs",
        "types.rs",
        "gun.rs",
        "pickup.rs",
        "shot.rs",
        "placement.rs",
        "transit.rs",
        "lifecycle.rs",
        "messages.rs",
        "pieces.rs",
    ];
    for rel in mechanic_files {
        assert!(
            portal_crate_src.join(rel).exists(),
            "portal mechanic should live in ambition_portal: missing {rel}"
        );
    }

    // The facade + the render-gated presentation stay in the sandbox.
    for rel in ["portal/mod.rs", "portal/presentation.rs"] {
        assert!(
            src_root.join(rel).exists(),
            "sandbox should keep the portal facade/presentation: {rel}"
        );
    }
    // The mechanic must NOT linger in the sandbox after the move.
    for rel in ["portal/plugin.rs", "portal/transit.rs", "portal/gun.rs"] {
        assert!(
            !src_root.join(rel).exists(),
            "portal mechanic file {rel} must move into ambition_portal, not stay in the sandbox"
        );
    }

    assert!(
        !src_root.join("portal.rs").exists(),
        "remove crates/ambition_sandbox/src/portal.rs so Rust does not see both portal.rs and portal/mod.rs"
    );

    let mod_text = fs::read_to_string(src_root.join("portal/mod.rs")).expect("read portal facade");
    assert!(
        mod_text.contains("pub use ambition_portal::*"),
        "portal facade should re-export the whole ambition_portal crate for zero inbound churn"
    );

    let plugin_text =
        fs::read_to_string(portal_crate_src.join("plugin.rs")).expect("read portal plugin");
    assert!(
        plugin_text.contains("PortalSet::Transit"),
        "portal plugin should label transit systems with PortalSet"
    );
}

#[test]
fn architecture_boundaries_portal_crate_is_extracted() {
    // Stage 19 Phase 5b (ADR 0019): the portal MECHANIC (transit math, placement,
    // lifecycle, carve publish, pieces, gun mechanics, the pure shot helper,
    // messages, types, schedule) was extracted into the standalone
    // `ambition_portal` crate. Assert (a) the crate exists and is registered,
    // (b) it does NOT depend on the sandbox (it is a reusable, content-free
    // physics/mechanic plugin — a different game must be able to drop it in),
    // (c) its only path deps are the lower crates engine_core + platformer_runtime,
    // (d) it exposes a `PortalPlugin`, and (e) no source line names the sandbox.
    let root = repo_root();
    let crate_root = root.join("crates/ambition_portal");

    // (a) The crate exists and is wired into the workspace.
    assert!(
        crate_root.join("Cargo.toml").exists(),
        "ambition_portal crate should exist at crates/ambition_portal"
    );
    assert!(
        crate_root.join("src/lib.rs").exists(),
        "ambition_portal should have a src/lib.rs"
    );
    let workspace_manifest =
        fs::read_to_string(root.join("Cargo.toml")).expect("read workspace manifest");
    assert!(
        workspace_manifest.contains("crates/ambition_portal"),
        "ambition_portal must be a registered workspace member"
    );

    // (b) + (c) Manifest-level dependency boundary: never the sandbox/content,
    //     and the only host path deps are the two lower crates.
    let crate_manifest =
        fs::read_to_string(crate_root.join("Cargo.toml")).expect("read ambition_portal manifest");
    for forbidden in [
        "ambition_sandbox",
        "ambition_content",
        "ambition_input",
        "ambition_sfx",
        "ambition_menu",
    ] {
        let depends = crate_manifest.lines().any(|line| {
            let line = line.trim();
            line.starts_with(&format!("{forbidden} ="))
                || line.starts_with(&format!("{forbidden}."))
        });
        assert!(
            !depends,
            "ambition_portal must not depend on {forbidden} (the mechanic owns no host concern)"
        );
    }
    let path_deps: Vec<String> = crate_manifest
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            line.strip_prefix("ambition_").map(|rest| {
                format!(
                    "ambition_{}",
                    rest.split([' ', '=', '.']).next().unwrap_or("")
                )
            })
        })
        .collect();
    for dep in &path_deps {
        assert!(
            dep == "ambition_engine_core" || dep == "ambition_platformer_runtime",
            "ambition_portal may only depend on engine_core + platformer_runtime, found `{dep}`"
        );
    }

    // (e) Source-level boundary: no code line may reference the sandbox crate.
    for file in collect_rs_files(&crate_root.join("src")) {
        let text = fs::read_to_string(&file).expect("read ambition_portal source");
        for (lineno, line) in text.lines().enumerate() {
            let trimmed = line.trim_start();
            if trimmed.starts_with("//") || trimmed.starts_with('*') {
                continue;
            }
            assert!(
                !line.contains("ambition_sandbox"),
                "ambition_portal must stay content-free; {}:{} references ambition_sandbox",
                file.display(),
                lineno + 1,
            );
        }
    }

    // (d) The crate exposes a drop-in PortalPlugin.
    let lib_text = fs::read_to_string(crate_root.join("src/lib.rs")).expect("read portal lib.rs");
    let plugin_text =
        fs::read_to_string(crate_root.join("src/plugin.rs")).expect("read portal plugin.rs");
    assert!(
        lib_text.contains("pub use plugin::{PortalPlugin")
            && plugin_text.contains("impl Plugin for PortalPlugin"),
        "ambition_portal should expose a PortalPlugin"
    );

    // The sandbox-side facade re-exports the whole crate (zero inbound churn).
    let facade = fs::read_to_string(crate_src().join("portal/mod.rs")).expect("read portal facade");
    assert!(
        facade.contains("pub use ambition_portal::*"),
        "sandbox portal/mod.rs should re-export ambition_portal::*"
    );
}

#[test]
fn architecture_boundaries_time_crate_is_extracted() {
    // Stage 18 / T1b: the generic time vocabulary + producer (WorldTime /
    // ClockState / ClockDomain / ProperTimeScale / the named-clock dt
    // accessors / TimePlugin) was extracted into the standalone
    // `ambition_time` crate. Assert (a) the crate exists and is registered,
    // (b) it does NOT depend on the sandbox (it is the reusable, content-free
    // time layer — a different game must be able to drop it in), (c) it
    // exposes a `TimePlugin`, and (d) the sandbox's time modules are now
    // facades re-exporting the crate's types while keeping the game-specific
    // time-control policy / camera-ease / feel on top.
    let root = repo_root();
    let crate_root = root.join("crates/ambition_time");

    // (a) The crate exists and is wired into the workspace.
    assert!(
        crate_root.join("Cargo.toml").exists(),
        "ambition_time crate should exist at crates/ambition_time"
    );
    assert!(
        crate_root.join("src/lib.rs").exists(),
        "ambition_time should have a src/lib.rs"
    );
    let workspace_manifest =
        fs::read_to_string(root.join("Cargo.toml")).expect("read workspace manifest");
    assert!(
        workspace_manifest.contains("crates/ambition_time"),
        "ambition_time must be a registered workspace member"
    );

    // (b) The extracted crate must not depend on the sandbox.
    let crate_manifest =
        fs::read_to_string(crate_root.join("Cargo.toml")).expect("read ambition_time manifest");
    let depends_on_sandbox = crate_manifest.lines().any(|line| {
        let line = line.trim();
        line.starts_with("ambition_sandbox =") || line.starts_with("ambition_sandbox.")
    });
    assert!(
        !depends_on_sandbox,
        "ambition_time must not depend on ambition_sandbox (it is the reusable time layer)"
    );

    // (b') The crate must stay content-free: no source line may reference the
    //      sandbox crate by path.
    for file in collect_rs_files(&crate_root.join("src")) {
        let text = fs::read_to_string(&file).expect("read ambition_time source");
        for (lineno, line) in text.lines().enumerate() {
            // Skip doc/comment lines: the crate docs legitimately say it was
            // "extracted from ambition_sandbox". Only real code references
            // (a `use` / path) would violate the boundary.
            let trimmed = line.trim_start();
            if trimmed.starts_with("//") || trimmed.starts_with("*") {
                continue;
            }
            assert!(
                !line.contains("ambition_sandbox"),
                "ambition_time must stay content-free; {}:{} references ambition_sandbox",
                file.display(),
                lineno + 1,
            );
        }
    }

    // (c) The crate exposes a drop-in TimePlugin (the north-star composition
    //     handle for a different game).
    let lib_text = fs::read_to_string(crate_root.join("src/lib.rs")).expect("read time lib.rs");
    assert!(
        lib_text.contains("pub struct TimePlugin")
            && lib_text.contains("impl Plugin for TimePlugin"),
        "ambition_time should expose a TimePlugin"
    );

    // (d) The sandbox-side time modules are facades over the crate.
    let sandbox_time = crate_src().join("time");
    let world_time = fs::read_to_string(sandbox_time.join("world_time.rs"))
        .expect("read sandbox time/world_time.rs facade");
    assert!(
        world_time.contains("pub use ambition_time::"),
        "sandbox time/world_time.rs should re-export ambition_time types"
    );
    let clock_state = fs::read_to_string(sandbox_time.join("clock_state.rs"))
        .expect("read sandbox time/clock_state.rs facade");
    assert!(
        clock_state.contains("pub use ambition_time::ClockState"),
        "sandbox time/clock_state.rs should re-export ambition_time::ClockState"
    );
    // The game-specific policy (Regime / requester table) + presentation stay
    // sandbox-side.
    for stay in ["time_control.rs", "camera_ease.rs", "feel.rs"] {
        assert!(
            sandbox_time.join(stay).exists(),
            "sandbox time/{stay} (game-specific policy / presentation) must stay sandbox-side"
        );
    }
}

#[test]
fn architecture_boundaries_machinery_does_not_import_content() {
    // Stage 20 / A1+A3: the ENTIRE machinery lib must keep the content
    // layer out. Post-bisection the crate graph enforces the hard half
    // (`ambition_content` sits ABOVE the lib, so importing it cannot even
    // compile without a manifest change); this guard keeps the VOCABULARY
    // out — neither the old `crate::content::` paths nor a sneaky
    // `ambition_content::` dependency may reappear. The app crate is the
    // composition layer and MAY name content. `crate::features` (the named
    // actor/boss world still living in the lib until the B3 render
    // inversion) is the one documented named-adjacent region (doc 20 B3/B4).
    // *test* files are excluded — fixtures may exercise content freely.
    let machinery_dirs = [
        "abilities",
        "actor",
        "assets",
        "audio",
        "body_mode",
        "boss_encounter",
        "brain",
        "combat",
        "dev",
        "dialog",
        "encounter",
        "enemy_projectile",
        "features",
        "host",
        "interaction.rs",
        "inventory",
        "items",
        "mechanics",
        "menu",
        "music",
        "persistence",
        "physics.rs",
        "platformer_runtime",
        "player",
        "portal",
        "presentation",
        "projectile",
        "quest",
        "runtime",
        "shrine.rs",
        "time",
        "ui_nav",
        "world",
    ];
    let forbidden = ["crate::content::", "ambition_content::"];

    let mut violations = Vec::new();
    for dir in machinery_dirs {
        let root = crate_src().join(dir);
        for file in collect_rs_files(&root) {
            let rel = file.display().to_string();
            if rel.ends_with("tests.rs") || rel.contains("/tests/") {
                continue;
            }
            let text = fs::read_to_string(&file).expect("read rust source");
            for (idx, raw) in text.lines().enumerate() {
                let line = raw.trim();
                if line.starts_with("//") || line.starts_with("/*") || line.starts_with('*') {
                    continue;
                }
                for needle in forbidden {
                    if line.contains(needle) {
                        violations.push(format!("{rel}:{} {line}", idx + 1));
                    }
                }
            }
        }
    }
    assert!(
        violations.is_empty(),
        "machinery modules must not import the content module — invert via a \
registry/marker/message/set the content layer fills (Stage 20 / A1):\n{}",
        violations.join("\n")
    );
}

#[test]
fn architecture_boundaries_combat_kit_stays_content_free() {
    // Stage 20 / A2: the generic combat kit must never name Ambition
    // content — no archetype enum, no named bosses/enemies, no content
    // imports. Named behavior reaches the kit via components/messages the
    // content layer supplies.
    let root = crate_src().join("mechanics").join("combat");
    let forbidden = [
        "crate::content",
        "crate::ambition_content",
        "EnemyArchetype",
        "CutRope",
        "cut_rope",
        "GnuTon",
        "GNU_TON",
        "gnu_ton",
        "Pirate",
        "Mockingbird",
        "BurningFlyingShark",
        "ExplodingMite",
        "DividingMite",
        "clockwork_warden",
        // The boss attack profiles (HandSlam, DebrisRain, LockOnBeam, ...)
        // were de-named in Stage 22 — they are generic behavior vocabulary
        // now, not content names, so they no longer belong on this list.
    ];
    let mut violations = Vec::new();
    for file in collect_rs_files(&root) {
        let text = fs::read_to_string(&file).expect("read rust source");
        for (idx, raw) in text.lines().enumerate() {
            let line = raw.trim();
            if line.starts_with("//") || line.starts_with("/*") || line.starts_with('*') {
                continue;
            }
            for needle in forbidden {
                if line.contains(needle) {
                    violations.push(format!(
                        "{}:{} names content `{needle}`: {line}",
                        file.display(),
                        idx + 1
                    ));
                }
            }
        }
    }
    assert!(
        violations.is_empty(),
        "the combat kit (mechanics::combat) is reusable machinery and must not \
name Ambition content:\n{}",
        violations.join("\n")
    );
}

#[test]
fn architecture_boundaries_presentation_does_not_use_the_archetype_enum() {
    // Stage 20 / B3: presentation consumes authored visual DATA
    // (catalog sprite tuning, composite_visual rows, dream_seed,
    // FeatureVisualKind) — never the named archetype enum. The boss
    // sprite chain in rendering/actors.rs still names boss ids
    // (tracked: the boss-asset-map slice in code_smells.md); this
    // guard pins the ENEMY side so it cannot regress.
    let root = crate_src().join("presentation");
    let forbidden = ["EnemyArchetype", "is_composite_spawn", "sandbag_"];
    let mut violations = Vec::new();
    for file in collect_rs_files(&root) {
        let rel = file.display().to_string();
        if rel.ends_with("tests.rs") || rel.contains("/tests/") {
            continue;
        }
        let text = fs::read_to_string(&file).expect("read rust source");
        for (idx, raw) in text.lines().enumerate() {
            let line = raw.trim();
            if line.starts_with("//") || line.starts_with("/*") || line.starts_with('*') {
                continue;
            }
            for needle in forbidden {
                if line.contains(needle) {
                    violations.push(format!("{rel}:{} {line}", idx + 1));
                }
            }
        }
    }
    assert!(
        violations.is_empty(),
        "presentation must read authored visual data, not the archetype enum \
(invert via spec fields + features::enemy_visual_kind/composite_visual_plan):\n{}",
        violations.join("\n")
    );
}

#[test]
fn architecture_boundaries_lib_menu_keeps_only_the_coupled_pieces() {
    // Stage 20 menu split: the menu HOST stack (page model, dispatcher,
    // item-confirm effects, the bevy_ui grid + 3D cube hosts) moved up to
    // `ambition_app::menu`. The machinery lib keeps ONLY the genuinely
    // lib-coupled pieces: the settings IR (read by persistence), the Map tab
    // (read by presentation), and the backend selector. This guard pins that
    // the host modules don't creep back into the lib.
    let menu_dir = crate_src().join("menu");
    let forbidden_files = [
        "model.rs",
        "dispatch.rs",
        "effects.rs",
        "grid_backend.rs",
        "kaleidoscope_app.rs",
    ];
    for f in forbidden_files {
        assert!(
            !menu_dir.join(f).exists(),
            "menu host file {f} must live in ambition_app::menu, not the machinery lib"
        );
    }
    for required in ["backend.rs", "ir", "map"] {
        assert!(
            menu_dir.join(required).exists(),
            "lib menu should keep {required} (the persistence/presentation-coupled pieces)"
        );
    }
    // The app crate owns the host stack.
    let app_menu = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/menu");
    for f in ["kaleidoscope_app.rs", "grid_backend.rs", "model.rs"] {
        assert!(
            app_menu.join(f).exists(),
            "ambition_app::menu should own the host file {f}"
        );
    }
}

#[test]
fn architecture_boundaries_dev_overlays_live_in_app() {
    // Stage 20 devtools split: the F1 debug overlay + F3 FPS counter are
    // pure presentation with no lib consumer, so they live in
    // `ambition_app::dev`. The machinery lib keeps the dev STATE
    // (dev_tools), the gameplay `trace` recorder (sim writes it), and
    // `profiling` (audio reads phase_mark) — those are genuinely
    // lib-coupled and must NOT be assumed movable.
    let lib_dev = crate_src().join("dev");
    for moved in ["debug_overlay.rs", "fps_overlay.rs"] {
        assert!(
            !lib_dev.join(moved).exists(),
            "{moved} moved to ambition_app::dev; it must not be in the machinery lib"
        );
    }
    for stays in ["dev_tools.rs", "profiling.rs", "trace.rs"] {
        assert!(
            lib_dev.join(stays).exists(),
            "lib dev must keep {stays} (lib-coupled: persistence/sim/audio read it)"
        );
    }
    let app_dev = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/dev");
    for f in ["debug_overlay.rs", "fps_overlay.rs"] {
        assert!(app_dev.join(f).exists(), "ambition_app::dev should own {f}");
    }
}

#[test]
fn architecture_boundaries_actor_crate_is_content_free_and_foundation_clean() {
    // Stage 22: the unified actor system (actor + brain) is a real crate
    // below the machinery lib. Two invariants:
    // (a) it never imports upward (the machinery lib, content, app);
    // (b) it never names game content — attack profiles were de-named to
    //     behavior vocabulary (HandSlam, DebrisRain, ...) precisely so
    //     this crate stays reusable. snake_case sheet-row keys live in
    //     boss_profiles.ron + the named world, not here.
    let root = repo_root().join("crates/ambition_actor/src");
    let forbidden = [
        "GnuTon",
        "gnu_ton",
        "Mockingbird",
        "mockingbird",
        "cut_rope",
        "CutRope",
        "Pirate",
        "BurningFlyingShark",
        "PuppySlug",
        "clockwork_warden",
        "EnemyArchetype",
    ];
    // No exemptions: the crate owns catalog SCHEMA + parser + resolver;
    // the game's roster DATA (embedded RON + roster-pinning tests) lives
    // in ambition_sandbox::character_roster, so nothing in this crate may
    // name content or import upward.
    let upward = ["ambition_sandbox", "ambition_content", "ambition_app"];
    let mut violations = Vec::new();
    for file in collect_rs_files(&root) {
        let rel = file.display().to_string();
        let text = fs::read_to_string(&file).expect("read ambition_actor source");
        for (idx, raw) in text.lines().enumerate() {
            let line = raw.trim();
            if line.starts_with("//") || line.starts_with("/*") || line.starts_with('*') {
                continue;
            }
            for needle in upward.iter().chain(forbidden.iter()) {
                if line.contains(needle) {
                    violations.push(format!("{rel}:{} {line}", idx + 1));
                }
            }
        }
    }
    assert!(
        violations.is_empty(),
        "ambition_actor must stay content-free with no upward imports:\n{}",
        violations.join("\n")
    );
}
