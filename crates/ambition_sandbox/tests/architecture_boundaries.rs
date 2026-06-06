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
