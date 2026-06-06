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
    let facade = fs::read_to_string(sandbox_runtime.join("mod.rs")).expect("read facade mod.rs");
    assert!(
        facade.contains("ambition_platformer_runtime::{lifecycle, schedule}"),
        "sandbox platformer_runtime facade should re-export the extracted crate's lifecycle + schedule"
    );

    // (d) The still-local remainder stays in the sandbox because it is not
    //     import-clean; document the blocking dependency for each.
    for (rel, blocking_dep) in [
        ("collision.rs", "crate::engine_core"),
        ("orientation.rs", "crate::physics"),
        ("transit.rs", "crate::portal_pieces"),
    ] {
        let path = sandbox_runtime.join(rel);
        assert!(
            path.exists(),
            "{rel} stays in the sandbox facade until its sandbox coupling is decoupled"
        );
        let text = fs::read_to_string(&path).expect("read remainder module");
        assert!(
            text.contains(blocking_dep),
            "{rel} is documented as not-yet-extracted because it depends on {blocking_dep}; \
if that dependency is gone, extract it into ambition_platformer_runtime and update this guardrail"
        );
    }
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
    let text = fs::read_to_string(crate_src().join("portal/plugin.rs"))
        .expect("read portal plugin source");
    assert!(
        !text.contains("after(crate::item_pickup::ground_item_physics)"),
        "portal should order against crate::item_pickup::ItemPickupSet, not the concrete ground_item_physics function"
    );
    assert!(
        text.contains("crate::item_pickup::ItemPickupSet::CoreHeldItems"),
        "portal transit should document its dependency on the held-item/ground-item simulation set"
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
    ];

    // ALLOWLIST — genuinely-shared low-level couplings that remain in portal
    // core for now, each with a tracked reason. These are NOT the item roster;
    // they are deeper input/physics-body seams a later pass (or Task J) can
    // finish extracting behind a generic abstraction:
    //
    //   crate::input::ControlFrame
    //     - transit.rs `warp_portal_input` / `portal_transit_system`: the
    //       same-wall held-input warp + emergence guard read AND mutate the
    //       Ambition input frame. Extracting this needs a generic movement-input
    //       abstraction; deferred to keep replay timing identical.
    //     - presentation.rs `sync_portal_mode_indicator`: visible-build aim
    //       indicator resolves aim from the control frame. Presentation glue.
    //   crate::item_pickup::GroundItem
    //     - transit.rs `portal_teleport_ground_items`: thrown-item transit
    //       queries the content body component. Needs a generic transit-body
    //       marker to decouple.
    //   crate::item_pickup::ItemPickupSet
    //     - plugin.rs: a *schedule ordering label* (not a content concept);
    //       already guarded by `architecture_boundaries_portal_orders_against_item_set_not_function`.
    let allow = |line: &str| -> bool {
        line.contains("crate::input::ControlFrame")
            || line.contains("crate::item_pickup::GroundItem")
            || line.contains("crate::item_pickup::ItemPickupSet")
            || line.contains("crate::item_pickup::axe_spec") // test fixture only
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
            if allow(line) {
                continue;
            }
            for needle in forbidden {
                if line.contains(needle) {
                    // Tests may construct content fixtures (axe_spec/GroundItem),
                    // which are already allowlisted; anything else in tests is a
                    // genuine violation too, so do not blanket-skip tests.
                    let _ = is_test;
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
