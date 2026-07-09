use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("crate lives under <repo>/crates/ambition_actors")
        .to_path_buf()
}

/// The machinery lib's source root (most boundaries guard the
/// `ambition_actors` lib; this test crate lives in `ambition_app`).
fn crate_src() -> PathBuf {
    repo_root().join("crates/ambition_actors/src")
}

/// The app-assembly crate's source root.
fn app_src() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
}

/// The named-content crate's source root.
fn content_src() -> PathBuf {
    repo_root().join("game/ambition_content/src")
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

fn is_comment_line(line: &str) -> bool {
    let line = line.trim();
    line.starts_with("//") || line.starts_with("/*") || line.starts_with('*')
}

fn manifest_depends_on(manifest: &str, crate_name: &str) -> bool {
    manifest.lines().any(|line| {
        let line = line.trim();
        line.starts_with(&format!("{crate_name} =")) || line.starts_with(&format!("{crate_name}."))
    })
}

fn assert_manifest_has_no_deps(crate_root: &Path, forbidden: &[&str], context: &str) {
    let manifest = fs::read_to_string(crate_root.join("Cargo.toml")).expect("read crate manifest");
    let violations = forbidden
        .iter()
        .copied()
        .filter(|name| manifest_depends_on(&manifest, name))
        .collect::<Vec<_>>();
    assert!(
        violations.is_empty(),
        "{context} must not depend on: {:?}",
        violations
    );
}

fn scan_code_refs<F>(roots: &[PathBuf], forbidden: &[&str], mut allow: F) -> Vec<String>
where
    F: FnMut(&Path, &str) -> bool,
{
    let mut violations = Vec::new();
    for file in roots.iter().flat_map(|root| collect_rs_files(root)) {
        let text = fs::read_to_string(&file).expect("read rust source");
        for (idx, raw) in text.lines().enumerate() {
            let line = raw.trim();
            if is_comment_line(line) || allow(&file, line) {
                continue;
            }
            for needle in forbidden {
                if line.contains(needle) {
                    violations.push(format!(
                        "{}:{} references `{needle}`: {line}",
                        file.display(),
                        idx + 1
                    ));
                }
            }
        }
    }
    violations
}

fn scan_text_refs(roots: &[PathBuf], forbidden: &[&str]) -> Vec<String> {
    let mut violations = Vec::new();
    for file in roots.iter().flat_map(|root| collect_rs_files(root)) {
        let text = fs::read_to_string(&file).expect("read rust source");
        for needle in forbidden {
            if text.contains(needle) {
                violations.push(format!("{} mentions `{needle}`", file.display()));
            }
        }
    }
    violations
}

fn assert_source_tree_has_no_code_refs(root: PathBuf, forbidden: &[&str], context: &str) {
    let violations = scan_code_refs(&[root], forbidden, |_, _| false);
    assert!(
        violations.is_empty(),
        "{context}:\n{}",
        violations.join("\n")
    );
}

fn assert_paths_exist(root: &Path, rels: &[&str], context: &str) {
    let missing = rels
        .iter()
        .copied()
        .filter(|rel| !root.join(rel).exists())
        .collect::<Vec<_>>();
    assert!(
        missing.is_empty(),
        "{context} missing expected paths: {:?}",
        missing
    );
}

fn assert_paths_absent(root: &Path, rels: &[&str], context: &str) {
    let present = rels
        .iter()
        .copied()
        .filter(|rel| root.join(rel).exists())
        .collect::<Vec<_>>();
    assert!(
        present.is_empty(),
        "{context} should not contain paths: {:?}",
        present
    );
}

fn assert_workspace_contains_crate(crate_name: &str) {
    let workspace_manifest =
        fs::read_to_string(repo_root().join("Cargo.toml")).expect("read workspace manifest");
    assert!(
        workspace_manifest.contains(&format!("crates/{crate_name}")),
        "{crate_name} must be a registered workspace member"
    );
}

fn manifest_path_deps(manifest: &str) -> Vec<String> {
    manifest
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
        .collect()
}


fn manifest_ambition_deps_including_facade(manifest: &str) -> Vec<String> {
    manifest
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.starts_with('#') || line.starts_with("//") {
                return None;
            }
            let name = line.split([' ', '=', '.']).next().unwrap_or("");
            if name == "ambition" || name.starts_with("ambition_") {
                Some(name.to_string())
            } else {
                None
            }
        })
        .collect()
}

fn assert_manifest_ambition_deps_only(crate_root: &Path, allowed: &[&str], context: &str) {
    let manifest = fs::read_to_string(crate_root.join("Cargo.toml")).expect("read crate manifest");
    let violations = manifest_ambition_deps_including_facade(&manifest)
        .into_iter()
        .filter(|dep| !allowed.contains(&dep.as_str()))
        .collect::<Vec<_>>();
    assert!(
        violations.is_empty(),
        "{context} may only depend on {:?}, found {:?}",
        allowed,
        violations
    );
}

fn assert_manifest_path_deps_only(crate_root: &Path, allowed: &[&str], context: &str) {
    let manifest = fs::read_to_string(crate_root.join("Cargo.toml")).expect("read crate manifest");
    let violations = manifest_path_deps(&manifest)
        .into_iter()
        .filter(|dep| !allowed.contains(&dep.as_str()))
        .collect::<Vec<_>>();
    assert!(
        violations.is_empty(),
        "{context} may only depend on {:?}, found {:?}",
        allowed,
        violations
    );
}

fn is_test_file(path: &Path) -> bool {
    let rel = path.display().to_string();
    rel.ends_with("tests.rs") || rel.contains("/tests/")
}

fn scan_code_refs_filtered<F, A>(
    roots: &[PathBuf],
    forbidden: &[&str],
    mut include: F,
    mut allow: A,
) -> Vec<String>
where
    F: FnMut(&Path) -> bool,
    A: FnMut(&Path, &str) -> bool,
{
    let mut violations = Vec::new();
    for file in roots.iter().flat_map(|root| collect_rs_files(root)) {
        if !include(&file) {
            continue;
        }
        let text = fs::read_to_string(&file).expect("read rust source");
        for (idx, raw) in text.lines().enumerate() {
            let line = raw.trim();
            if is_comment_line(line) || allow(&file, line) {
                continue;
            }
            for needle in forbidden {
                if line.contains(needle) {
                    violations.push(format!(
                        "{}:{} references `{needle}`: {line}",
                        file.display(),
                        idx + 1
                    ));
                }
            }
        }
    }
    violations
}

fn assert_code_refs_filtered<F, A>(
    roots: &[PathBuf],
    forbidden: &[&str],
    include: F,
    allow: A,
    context: &str,
) where
    F: FnMut(&Path) -> bool,
    A: FnMut(&Path, &str) -> bool,
{
    let violations = scan_code_refs_filtered(roots, forbidden, include, allow);
    assert!(
        violations.is_empty(),
        "{context}:\n{}",
        violations.join("\n")
    );
}

fn assert_code_refs_absent(roots: &[PathBuf], forbidden: &[&str], context: &str) {
    assert_code_refs_filtered(roots, forbidden, |_| true, |_, _| false, context);
}

fn assert_production_lines_have_no_refs(files: &[PathBuf], forbidden: &[&str], context: &str) {
    let mut violations = Vec::new();
    for file in files {
        let text = fs::read_to_string(file).expect("read rust source");
        let prod = text
            .split("#[cfg(test)]")
            .next()
            .expect("split always yields at least one piece");
        for (idx, raw) in prod.lines().enumerate() {
            let line = raw.trim();
            if is_comment_line(line) {
                continue;
            }
            for needle in forbidden {
                if line.contains(needle) {
                    violations.push(format!(
                        "{}:{} references `{needle}`: {line}",
                        file.display(),
                        idx + 1
                    ));
                }
            }
        }
    }
    assert!(
        violations.is_empty(),
        "{context}:\n{}",
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
        let (rel, count) = line
            .split_once('=')
            .unwrap_or_else(|| panic!("{}:{} expected path=count", path.display(), idx + 1));
        let count = count
            .trim()
            .parse::<usize>()
            .unwrap_or_else(|_| panic!("{}:{} expected integer count", path.display(), idx + 1));
        allowlist.insert(rel.trim().to_string(), count);
    }
    allowlist
}


/// E9: downstream game/content crates should depend on one engine facade instead
/// of copying the app shell's direct dependency wall. The Sanic/SMB1 demo homes
/// are intentionally empty at first; their value is the manifest oracle.
#[test]
fn architecture_boundaries_umbrella_crate_and_demo_homes_exist() {
    let root = repo_root();
    let workspace_manifest = fs::read_to_string(root.join("Cargo.toml"))
        .expect("read workspace manifest");
    for member in [
        "crates/ambition",
        "game/ambition_demo_sanic",
        "game/ambition_demo_smb1",
    ] {
        assert!(
            workspace_manifest.contains(&format!("\"{member}\"")),
            "workspace must register {member}"
        );
    }

    let umbrella_root = root.join("crates/ambition");
    assert_manifest_has_no_deps(
        &umbrella_root,
        &["ambition_app", "ambition_content", "ambition_menu_kaleidoscope"],
        "the ambition facade is an engine surface, not an app/content shell",
    );
    let umbrella_lib = fs::read_to_string(umbrella_root.join("src/lib.rs"))
        .expect("read ambition facade lib.rs");
    for required in [
        "PlatformerEnginePlugins",
        "PlatformerHostPlugins",
        "pub use ambition_runtime as runtime",
        "pub use ambition_render as render",
        "pub use ambition_world as world",
    ] {
        assert!(
            umbrella_lib.contains(required),
            "ambition facade should expose `{required}`"
        );
    }

    for (rel, label) in [
        ("game/ambition_demo_sanic", "Sanic demo home"),
        ("game/ambition_demo_smb1", "SMB1 demo home"),
    ] {
        let crate_root = root.join(rel);
        assert_manifest_ambition_deps_only(
            &crate_root,
            &["ambition"],
            &format!("{label} should depend on the umbrella plus its own content only"),
        );
        assert_source_tree_has_no_code_refs(
            crate_root.join("src"),
            &["ambition_actors::", "ambition_runtime::", "ambition_render::", "ambition_app::"],
            &format!("{label} should reach the engine through the ambition facade"),
        );
    }
}

/// The `ambition_render` crate is the sandbox's renderer. The sim machinery
/// (`ambition_actors`) must not depend on it, and after F1.5 render must not
/// depend back on the machinery either. Both crates meet only through lower
/// vocabulary/read-model crates selected by the app composition root.
#[test]
fn architecture_boundaries_render_and_actor_crates_are_decoupled() {
    assert_workspace_contains_crate("ambition_render");
    let sandbox_root = repo_root().join("crates/ambition_actors");
    let render_root = repo_root().join("crates/ambition_render");
    assert_manifest_has_no_deps(
        &sandbox_root,
        &["ambition_render"],
        "the sim machinery must not depend on its renderer",
    );
    assert_manifest_has_no_deps(
        &render_root,
        &["ambition_actors"],
        "F1.5 complete: render must read lower view/vocabulary crates, not actor machinery",
    );
    // And no source file smuggles either crate in past the manifest.
    assert_source_tree_has_no_code_refs(
        sandbox_root.join("src"),
        &["ambition_render"],
        "ambition_actors must not reference the render crate",
    );
    assert_source_tree_has_no_code_refs(
        render_root.join("src"),
        &["ambition_actors"],
        "ambition_render must not reference the actor machinery crate after F1.5",
    );
}

#[test]
fn architecture_boundaries_platformer_runtime_stays_content_free() {
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
        "ambition_portal::",
        "ambition_portal;",
        "ambition_portal}",
    ];
    let violations = scan_text_refs(&[crate_src().join("platformer_runtime")], &forbidden);
    assert!(
        violations.is_empty(),
        "platformer_runtime must remain reusable and content-free:\n{}",
        violations.join("\n")
    );
}

#[test]
fn architecture_boundaries_platformer_runtime_crate_is_extracted() {
    let root = repo_root();
    let crate_root = root.join("crates/ambition_platformer_primitives");
    let sandbox_runtime = crate_src().join("platformer_runtime");

    assert_paths_exist(
        &crate_root,
        &[
            "Cargo.toml",
            "src/lib.rs",
            "src/prelude.rs",
            "src/schedule.rs",
            "src/lifecycle/mod.rs",
            "src/lifecycle/markers.rs",
            "src/lifecycle/spawn_ext.rs",
            "src/lifecycle/cleanup.rs",
            "src/math.rs",
            "src/transit.rs",
            "src/body.rs",
            "src/gravity.rs",
            "src/orientation.rs",
        ],
        "ambition_platformer_primitives extracted crate",
    );
    assert_manifest_has_no_deps(
        &crate_root,
        &["ambition_actors"],
        "ambition_platformer_primitives must remain reusable and content-free",
    );

    // The generic projectile primitive must NOT name a game's projectile kinds.
    // The named Fireball/Hadouken vocabulary + stat tables live in
    // `ambition_projectiles::kind`; the engine carries only generic
    // `ProjectileSpec` data (bounces/gravity/half_extent/…). Guard against the
    // named-content leak creeping back into the foundation crate.
    let projectile_dir = crate_root.join("src/projectile");
    for file in ["spec.rs", "body.rs", "collision.rs", "mod.rs"] {
        let text = fs::read_to_string(projectile_dir.join(file))
            .unwrap_or_else(|e| panic!("read projectile/{file}: {e}"));
        for needle in ["ProjectileKind", "Fireball", "Hadouken"] {
            assert!(
                !text.contains(needle),
                "ambition_platformer_primitives/src/projectile/{file} must stay content-free, \
                 but names `{needle}` — named projectile kinds belong in \
                 ambition_projectiles::kind"
            );
        }
    }
    assert_paths_absent(
        &sandbox_runtime,
        &["schedule.rs", "lifecycle", "transit.rs"],
        "sandbox platformer_runtime facade",
    );

    let facade = fs::read_to_string(sandbox_runtime.join("mod.rs")).expect("read facade mod.rs");
    assert!(
        facade.contains("ambition_platformer_primitives::{gravity, lifecycle, math, schedule, transit}"),
        "sandbox platformer_runtime facade should re-export extracted gravity/lifecycle/math/schedule/transit"
    );
    let orientation_facade = fs::read_to_string(sandbox_runtime.join("orientation.rs"))
        .expect("read orientation facade");
    assert!(
        orientation_facade.contains("ambition_platformer_primitives::orientation"),
        "sandbox orientation should re-export the extracted orientation module"
    );
    let physics_facade = fs::read_to_string(crate_src().join("physics.rs")).expect("read physics");
    assert!(
        physics_facade.contains("ambition_platformer_primitives::gravity"),
        "ambition_actors::physics should re-export the extracted gravity module"
    );
}

/// Pure simulation/gameplay code in `ambition_actors` must not import the
/// presentation layer. The render layer depends on the sim — never the reverse —
/// so that `presentation/` can be lifted into a standalone render crate without
/// dragging gameplay logic along.
///
/// The allowlist names the only modules permitted to import `crate::presentation`,
/// and each is legitimately at-or-above the render boundary:
///   - `presentation/` itself (the render layer).
///   - `dialog/ui.rs` — IS UI rendering (draws the dialog box + fonts).
///   - `runtime/setup.rs` + `runtime/reset/` — composition-root orchestration that
///     wires sim and render together (spawns the scene, respawns room visuals on
///     reset). These sit above presentation by construction.
///
/// To extend the allowlist you must justify that the module genuinely belongs at
/// or above the presentation layer — not merely that it currently compiles.
#[test]
fn architecture_boundaries_sim_does_not_import_presentation() {
    let src = crate_src();
    let allowed_prefixes = [
        "presentation/",
        "dialog/ui.rs",
        "runtime/setup.rs",
        "runtime/reset/",
    ];
    let is_allowed = |file: &Path| {
        let rel = file.strip_prefix(&src).unwrap_or(file);
        let rel_str = rel.to_string_lossy().replace('\\', "/");
        allowed_prefixes
            .iter()
            .any(|p| rel_str.starts_with(p) || rel_str.contains(&format!("/{p}")))
    };
    let violations = scan_code_refs(&[src.clone()], &["crate::presentation"], |file, _line| {
        is_allowed(file)
    });
    assert!(
        violations.is_empty(),
        "pure gameplay/sim code must not import `crate::presentation` (render depends on sim, \
         not the reverse). Move the imported type DOWN to a foundation/runtime module, or invert \
         the call with an event. Violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn architecture_boundaries_menu_crate_stays_content_free() {
    let crate_root = repo_root().join("crates/ambition_menu");
    assert!(
        crate_root.join("Cargo.toml").exists(),
        "ambition_menu crate should exist at crates/ambition_menu"
    );
    assert_manifest_has_no_deps(
        &crate_root,
        &["ambition_actors"],
        "ambition_menu is the reusable renderer; the game owns menu content",
    );
    assert_source_tree_has_no_code_refs(
        crate_root.join("src"),
        &["ambition_actors"],
        "ambition_menu must stay content-free",
    );
}

#[test]
fn architecture_boundaries_persistence_crate_owns_stored_shapes_only() {
    let crate_root = repo_root().join("crates/ambition_persistence");
    assert_workspace_contains_crate("ambition_persistence");
    assert!(
        crate_root.join("Cargo.toml").exists(),
        "ambition_persistence crate should exist at crates/ambition_persistence"
    );
    let forbidden = [
        "ambition_actors",
        "ambition_menu",
        "ambition_render",
        "ambition_content",
        "ambition_app",
    ];
    assert_manifest_has_no_deps(
        &crate_root,
        &forbidden,
        "ambition_persistence owns stored shapes, not menu/UI/game machinery",
    );
    assert_source_tree_has_no_code_refs(
        crate_root.join("src"),
        &forbidden,
        "ambition_persistence must stay free of menu/UI/game machinery imports",
    );
}

/// W3/W4: `ambition_world` owns backend-agnostic room/placement IR, while
/// `ambition_ldtk_map` owns the LDtk backend. The world crate must never import
/// LDtk/runtime/app machinery; the backend converts downward into the world IR
/// and must not depend back on gameplay-core.
#[test]
fn architecture_boundaries_world_ir_and_ldtk_backend_are_split() {
    let world_root = repo_root().join("crates/ambition_world");
    let ldtk_root = repo_root().join("crates/ambition_ldtk_map");
    assert_workspace_contains_crate("ambition_world");
    assert_workspace_contains_crate("ambition_ldtk_map");
    assert!(
        world_root.join("Cargo.toml").exists(),
        "ambition_world crate should exist at crates/ambition_world"
    );
    assert!(
        ldtk_root.join("Cargo.toml").exists(),
        "ambition_ldtk_map crate should exist at crates/ambition_ldtk_map"
    );

    assert_manifest_has_no_deps(
        &world_root,
        &[
            "ambition_ldtk_map",
            "ambition_actors",
            "ambition_runtime",
            "ambition_render",
            "ambition_content",
            "ambition_app",
            "ambition_combat",
            "ambition_interaction",
            "ambition_portal",
            "bevy_ecs_ldtk",
        ],
        "ambition_world is the backend-agnostic world IR",
    );
    assert_source_tree_has_no_code_refs(
        world_root.join("src"),
        &[
            "ambition_ldtk_map",
            "ambition_combat",
            "ambition_interaction",
            "ambition_portal",
            "bevy_ecs_ldtk",
        ],
        "ambition_world source must stay free of backend and runtime-family code imports",
    );

    assert_manifest_has_no_deps(
        &ldtk_root,
        &[
            "ambition_actors",
            "ambition_runtime",
            "ambition_render",
            "ambition_content",
            "ambition_app",
        ],
        "ambition_ldtk_map converts into world IR without depending on the sim heart",
    );
    // The cfg(test) fixture manifest reads the GAME'S real world FILES via a
    // filesystem path (the sanctioned cross-crate fixture pattern — see
    // ldtk_map::manifest::test_fixture_manifest). A path string into
    // game/…/assets is DATA access for tests, not a code dependency; the
    // Cargo-manifest assertion above still forbids the real dep.
    let violations = scan_code_refs(
        &[ldtk_root.join("src")],
        &[
            "ambition_actors",
            "ambition_runtime",
            "ambition_render",
            "ambition_content",
            "ambition_app",
        ],
        |_, line| line.contains("../../game/ambition_content/assets"),
    );
    assert!(
        violations.is_empty(),
        "ambition_ldtk_map source must not reach upward into sim/app/render/content:\n{}",
        violations.join("\n")
    );
}

/// `ambition_projectiles` (E2) owns the reusable projectile MODEL — shot
/// vocabulary, ECS components, the spawn pool + player-pool spawner, and pure
/// portal transit. It is a FOUNDATIONAL crate: it may name only the geometry /
/// primitive / portal / trace / input foundations, never the sim heart, the
/// combat kit, character brains, or any host/content/render machinery. The
/// victim-side hit routing + charge-input steppers that DO need those stay in
/// `ambition_actors` and consume this crate (the legal sim → model arrow).
#[test]
fn architecture_boundaries_projectiles_crate_is_model_only() {
    let crate_root = repo_root().join("crates/ambition_projectiles");
    assert_workspace_contains_crate("ambition_projectiles");
    assert!(
        crate_root.join("Cargo.toml").exists(),
        "ambition_projectiles crate should exist at crates/ambition_projectiles"
    );
    let forbidden = [
        "ambition_actors",
        "ambition_combat",
        "ambition_characters",
        "ambition_sim_view",
        "ambition_runtime",
        "ambition_render",
        "ambition_content",
        "ambition_app",
    ];
    assert_manifest_has_no_deps(
        &crate_root,
        &forbidden,
        "ambition_projectiles is the projectile MODEL — no sim-heart / combat / \
         brain / host / content coupling (the woven steppers stay in ambition_actors)",
    );
    assert_source_tree_has_no_code_refs(
        crate_root.join("src"),
        &forbidden,
        "ambition_projectiles must stay a reusable model free of sim/combat/host imports",
    );
}

/// `ambition_encounter` (E-enc) owns the reusable encounter wave/lockdown
/// vocabulary and headless state machine. The LDtk loader, ECS mob spawning,
/// feature overlay, banners, save/quest plumbing, and schedule adapters stay in
/// `ambition_actors` until their owning domains move, so this crate must remain free
/// of sim-heart, content, render, runtime, host, and app dependencies.
#[test]
fn architecture_boundaries_encounter_crate_is_state_only() {
    let crate_root = repo_root().join("crates/ambition_encounter");
    assert_workspace_contains_crate("ambition_encounter");
    assert!(
        crate_root.join("Cargo.toml").exists(),
        "ambition_encounter crate should exist at crates/ambition_encounter"
    );
    let forbidden = [
        "ambition_actors",
        "ambition_characters",
        "ambition_ldtk_map",
        "ambition_sim_view",
        "ambition_runtime",
        "ambition_render",
        "ambition_content",
        "ambition_app",
    ];
    assert_manifest_has_no_deps(
        &crate_root,
        &forbidden,
        "ambition_encounter is reusable encounter state/vocabulary; adapters stay above it",
    );
    assert_source_tree_has_no_code_refs(
        crate_root.join("src"),
        &forbidden,
        "ambition_encounter source must not reach into sim/content/render/host crates",
    );
}

/// `ambition_items` (E8/F1.6) owns the reusable item catalog and shop
/// primitives only. Live pickup/throw/projectile systems stay in
/// `ambition_actors::items::pickup` because they mutate actor bodies, gravity,
/// portals, abilities, and hit events; menu-navigation state lives in
/// `ambition_inventory_ui` so the item model stays below UI navigation.
#[test]
fn architecture_boundaries_items_crate_is_catalog_and_shop_only() {
    let crate_root = repo_root().join("crates/ambition_items");
    assert_workspace_contains_crate("ambition_items");
    assert!(
        crate_root.join("Cargo.toml").exists(),
        "ambition_items crate should exist at crates/ambition_items"
    );
    assert_manifest_has_no_deps(
        &crate_root,
        &[
            "ambition_actors",
            "ambition_render",
            "ambition_content",
            "ambition_app",
            "ambition_ui_nav",
            "ambition_inventory_ui",
        ],
        "ambition_items owns reusable catalog/shop data; UI state lives in ambition_inventory_ui",
    );
    assert_source_tree_has_no_code_refs(
        crate_root.join("src"),
        &[
            "ambition_actors",
            "ambition_render",
            "ambition_content",
            "ambition_app",
            "ambition_ui_nav",
            "ambition_inventory_ui",
        ],
        "ambition_items source must not reach into actor/content/render/app/UI crates",
    );
    assert_paths_absent(
        &crate_root.join("src"),
        &[
            "inventory_ui",
            "inventory_ui/mod.rs",
            "inventory_ui/model.rs",
        ],
        "ambition_items after F1.6 inventory UI split",
    );
    assert_paths_absent(
        &crate_src(),
        &[
            "inventory_ui",
            "inventory_ui/mod.rs",
            "inventory_ui/model.rs",
        ],
        "actor-sim inventory UI module after E8",
    );
}

/// `ambition_inventory_ui` (F1.6) owns only inventory menu-navigation state:
/// selected slot, tab, scroll, pointer confirm/armed state, and
/// `MenuFocusState`. It may depend on UI navigation vocabulary, but not on the
/// item catalog or any app/render/sim/content tier.
#[test]
fn architecture_boundaries_inventory_ui_crate_is_menu_state_only() {
    let crate_root = repo_root().join("crates/ambition_inventory_ui");
    assert_workspace_contains_crate("ambition_inventory_ui");
    assert!(
        crate_root.join("Cargo.toml").exists(),
        "ambition_inventory_ui crate should exist at crates/ambition_inventory_ui"
    );
    assert_manifest_path_deps_only(
        &crate_root,
        &["ambition_ui_nav"],
        "ambition_inventory_ui is reusable menu-navigation state, not item catalog/sim/presentation",
    );
    assert_source_tree_has_no_code_refs(
        crate_root.join("src"),
        &[
            "ambition_items::",
            "ambition_actors",
            "ambition_render",
            "ambition_content",
            "ambition_app",
        ],
        "ambition_inventory_ui source must stay a small UI-state leaf over ambition_ui_nav",
    );
}


/// `ambition_asset_manager` (F1.8) owns logical asset catalog/profile
/// resolution only. Concrete backend providers — including SFX-bank
/// `BankProvider` construction — live in the owning audio/app layer after a
/// caller resolves an `AssetId` to a location.
#[test]
fn architecture_boundaries_asset_manager_is_backend_generic() {
    let crate_root = repo_root().join("crates/ambition_asset_manager");
    assert_workspace_contains_crate("ambition_asset_manager");
    assert!(
        crate_root.join("Cargo.toml").exists(),
        "ambition_asset_manager crate should exist at crates/ambition_asset_manager"
    );

    assert_manifest_has_no_deps(
        &crate_root,
        &["ambition_sfx", "ambition_audio"],
        "asset manager resolves logical locations; audio/SFX providers stay in their owning layer",
    );
    assert_code_refs_absent(
        &[crate_root.join("src")],
        &["ambition_sfx::", "BankProvider", "SfxBankResolveError"],
        "asset manager source must not construct audio/SFX backend providers",
    );
    assert_paths_absent(
        &crate_root.join("src"),
        &["sfx_integration.rs"],
        "F1.8 removes the asset-manager SFX adapter instead of feature-gating an upward dependency",
    );

    let manifest = fs::read_to_string(crate_root.join("Cargo.toml"))
        .expect("read ambition_asset_manager manifest");
    assert!(
        !manifest.contains("dep:ambition_sfx")
            && !manifest
                .lines()
                .any(|line| line.trim().starts_with("sfx =")),
        "asset manager should not expose an `sfx` feature that reintroduces an audio backend edge"
    );
}


#[test]
fn architecture_boundaries_projectile_schedule_wiring_lives_in_runtime() {
    let runtime_root = repo_root().join("crates/ambition_runtime");
    assert_paths_exist(
        &runtime_root.join("src"),
        &["projectile_schedule.rs"],
        "F2 projectile residual scheduling facade",
    );

    let facade = fs::read_to_string(runtime_root.join("src/projectile_schedule.rs"))
        .expect("read runtime projectile schedule facade");
    assert!(
        facade.contains("ambition_actors::projectile")
            && facade.contains("ambition_actors::enemy_projectile")
            && facade.contains("ambition_projectiles::apply_player_spawn_projectile_messages"),
        "runtime should own the explicit projectile schedule facade over the remaining actor-side steppers"
    );

    assert_code_refs_absent(
        &[app_src()],
        &[
            "ambition_actors::projectile::",
            "ambition_actors::enemy_projectile::",
        ],
        "F2: app composition should schedule projectile steppers through ambition_runtime::projectile_schedule",
    );

    assert_code_refs_filtered(
        &[content_src()],
        &[
            "ambition_actors::projectile::",
            "ambition_actors::enemy_projectile::",
        ],
        |path| !is_test_file(path),
        |_, line| line.contains("use ambition_actors::enemy_projectile::apply_projectile_effects"),
        "F2: production content should emit projectile effects/model messages, not reach actor projectile steppers directly;          the one public-API content test may still drive the actor stepper fixture directly",
    );

    assert_code_refs_filtered(
        &[runtime_root.join("src")],
        &[
            "ambition_actors::projectile::",
            "ambition_actors::enemy_projectile::",
        ],
        |path| {
            path.file_name()
                .and_then(|name| name.to_str())
                != Some("projectile_schedule.rs")
        },
        |_, _| false,
        "F2: the remaining actor-side projectile stepper edge should be centralized in runtime::projectile_schedule",
    );
}

#[test]
fn architecture_boundaries_runtime_is_headless_composition_tier() {
    let crate_root = repo_root().join("crates/ambition_runtime");
    assert_workspace_contains_crate("ambition_runtime");
    assert!(
        crate_root.join("Cargo.toml").exists(),
        "ambition_runtime crate should exist at crates/ambition_runtime"
    );

    let manifest = fs::read_to_string(crate_root.join("Cargo.toml"))
        .expect("read ambition_runtime manifest");
    for required in [
        "ambition_actors",
        "ambition_combat",
        "ambition_projectiles",
        "ambition_encounter",
        "ambition_dialog",
        "ambition_menu",
    ] {
        assert!(
            manifest_depends_on(&manifest, required),
            "F1.9: ambition_runtime is the headless sim composition tier, so its              direct dependency on {required} is intentional and should be              documented rather than chased as an inversion bug"
        );
    }

    assert_manifest_path_deps_only(
        &crate_root,
        &[
            "ambition_actors",
            "ambition_projectiles",
            "ambition_combat",
            "ambition_encounter",
            "ambition_dialog",
            "ambition_characters",
            "ambition_portal",
            "ambition_sim_view",
            "ambition_vfx",
            "ambition_time",
            "ambition_input",
            "ambition_engine_core",
            "ambition_sfx",
            "ambition_persistence",
            "ambition_menu",
            "ambition_platformer_primitives",
            "ambition_dev_tools",
        ],
        "F1.9: ambition_runtime composes the headless engine/sim surface; it may          name sim/mechanic/model/dev-state crates but not app, content, host, or renderer crates",
    );

    assert_manifest_has_no_deps(
        &crate_root,
        &[
            "ambition_app",
            "ambition_content",
            "ambition_host",
            "ambition_render",
            "ambition_touch_input",
            "ambition_inventory_ui",
            "ambition_ldtk_map",
            "bevy_ecs_ldtk",
        ],
        "ambition_runtime is headless composition, not app/content/host/render/backend ownership",
    );
    assert_code_refs_absent(
        &[crate_root.join("src")],
        &[
            "ambition_render::",
            "ambition_host::",
            "ambition_content::",
            "ambition_app::",
            "ambition_touch_input::",
            "ambition_inventory_ui::",
            "ambition_ldtk_map::",
            "bevy_ecs_ldtk::",
        ],
        "runtime source should compose headless sim/mechanic/menu-model crates without reaching into          app/content/host/render tiers",
    );
}


#[test]
fn architecture_boundaries_host_does_not_depend_on_actors() {
    let crate_root = repo_root().join("crates/ambition_host");
    assert_workspace_contains_crate("ambition_host");
    assert!(
        crate_root.join("Cargo.toml").exists(),
        "ambition_host crate should exist at crates/ambition_host"
    );

    assert_manifest_has_no_deps(
        &crate_root,
        &["ambition_actors"],
        "F1.10: the windowed host wires presentation/input/runtime seams, not the actor-systems crate",
    );
    assert_code_refs_absent(
        &[crate_root.join("src"), crate_root.join("tests")],
        &["ambition_actors::"],
        "F1.10: host source and smoke fixtures should reach actor-owned setup only through runtime facades",
    );
}

/// `ambition_menu_kaleidoscope` is the FIRST engine extension crate (E1e): the
/// bevy_lunex 3D cube renderer for the `ambition_menu` page model. It is
/// optional for any game — a host installs it to draw the same backend-agnostic
/// model as a cube — so it must name only engine deps (ambition_menu + bevy +
/// bevy_lunex) and no game/app/content machinery.
#[test]
fn architecture_boundaries_kaleidoscope_is_an_engine_extension() {
    let crate_root = repo_root().join("game/ambition_menu_kaleidoscope");
    let workspace_manifest =
        fs::read_to_string(repo_root().join("Cargo.toml")).expect("read workspace manifest");
    assert!(
        workspace_manifest.contains("game/ambition_menu_kaleidoscope"),
        "ambition_menu_kaleidoscope must be a registered workspace member (in game/)"
    );
    assert!(
        crate_root.join("Cargo.toml").exists(),
        "ambition_menu_kaleidoscope should exist at game/ambition_menu_kaleidoscope"
    );
    // Only the reusable menu model — no game/app/content/gameplay deps.
    assert_manifest_path_deps_only(
        &crate_root,
        &["ambition_menu"],
        "ambition_menu_kaleidoscope is an engine extension over the menu model",
    );
    assert_source_tree_has_no_code_refs(
        crate_root.join("src"),
        &[
            "ambition_actors",
            "ambition_render",
            "ambition_content",
            "ambition_app",
            "ambition_settings_menu",
        ],
        "the kaleidoscope extension must name no game/app/content machinery",
    );
    // The base menu crate must stay bevy_lunex-free now that the cube left.
    let menu_manifest = fs::read_to_string(repo_root().join("crates/ambition_menu/Cargo.toml"))
        .expect("read ambition_menu manifest");
    assert!(
        !menu_manifest.contains("bevy_lunex ="),
        "ambition_menu should be bevy_lunex-free; the cube renderer is the extension crate"
    );
}

/// `ambition_settings_menu` is the renderer-agnostic settings + system menu IR
/// (E1e). It is pure logic over `ambition_persistence::settings` + the keyboard
/// presets — no bevy, no renderer, no game state — so both menu backends render
/// the same model and the settings IR stops being the god-dep that forced menu
/// presentation to reach back into gameplay-core.
#[test]
fn architecture_boundaries_settings_menu_ir_is_foundation_only() {
    let crate_root = repo_root().join("crates/ambition_settings_menu");
    assert_workspace_contains_crate("ambition_settings_menu");
    assert!(
        crate_root.join("Cargo.toml").exists(),
        "ambition_settings_menu crate should exist at crates/ambition_settings_menu"
    );
    assert_manifest_path_deps_only(
        &crate_root,
        &["ambition_persistence", "ambition_input"],
        "ambition_settings_menu is the pure settings IR; no renderer/game deps",
    );
    assert_source_tree_has_no_code_refs(
        crate_root.join("src"),
        &[
            "ambition_actors",
            "ambition_menu",
            "ambition_render",
            "ambition_content",
            "ambition_app",
            "bevy_lunex",
            "bevy::",
        ],
        "ambition_settings_menu must stay a pure, renderer-agnostic model",
    );
}

/// `ambition_dev_tools` is the reusable developer-tooling STATE + logic (E1d):
/// `DeveloperTools`, the reflected editable player-tuning / ability / stats
/// resources, the profile enums, the startup profiler, `DeveloperTools` disk
/// persistence, and the live-edit sync systems. It names only the foundational
/// body/marker/health vocabulary, so another platformer can wire an inspector
/// against it. The egui overlay UI (`DevToolsPlugin`) stays app-side and the
/// sim `trace` recorder stays sim-side — neither may leak into this crate.
#[test]
fn architecture_boundaries_dev_tools_crate_is_foundation_only() {
    let crate_root = repo_root().join("crates/ambition_dev_tools");
    assert_workspace_contains_crate("ambition_dev_tools");
    assert!(
        crate_root.join("Cargo.toml").exists(),
        "ambition_dev_tools crate should exist at crates/ambition_dev_tools"
    );
    assert_manifest_path_deps_only(
        &crate_root,
        &[
            "ambition_engine_core",
            "ambition_characters",
            "ambition_platformer_primitives",
            "ambition_persistence",
        ],
        "ambition_dev_tools is foundational dev-tool state; overlays/sim stay out",
    );
    assert_source_tree_has_no_code_refs(
        crate_root.join("src"),
        &[
            "ambition_actors",
            "ambition_menu",
            "ambition_render",
            "ambition_content",
            "ambition_app",
            "bevy_inspector_egui",
        ],
        "ambition_dev_tools must stay free of game/menu/render/app/egui machinery",
    );
}

/// `ambition_dialog` is the reusable dialogue runtime (E1c): the `DialogState`
/// view model, typewriter reveal + input systems, and the `bevy_yarnspinner`
/// bridge + binding-installer seam. It must name no game/actor/menu/UI content —
/// a host's game-specific Yarn bindings register through the installer seam and
/// the host maps `DialogState.active` onto its own session mode. So another
/// platformer reuses the dialogue runtime by depending on it and installing its
/// own vocabulary.
#[test]
fn architecture_boundaries_dialog_crate_is_runtime_only() {
    let crate_root = repo_root().join("crates/ambition_dialog");
    assert_workspace_contains_crate("ambition_dialog");
    assert!(
        crate_root.join("Cargo.toml").exists(),
        "ambition_dialog crate should exist at crates/ambition_dialog"
    );
    // Only the foundational tiers: geometry, ui-nav, input, sfx, persistence.
    assert_manifest_path_deps_only(
        &crate_root,
        &[
            "ambition_engine_core",
            "ambition_ui_nav",
            "ambition_input",
            "ambition_sfx",
            "ambition_persistence",
        ],
        "ambition_dialog is the reusable dialogue runtime; game bindings stay host-side",
    );
    assert_source_tree_has_no_code_refs(
        crate_root.join("src"),
        &[
            "ambition_actors",
            "ambition_menu",
            "ambition_render",
            "ambition_content",
            "ambition_app",
            "ambition_characters",
        ],
        "ambition_dialog must stay free of game/actor/menu/UI machinery imports",
    );
}

/// `ambition_interaction` is a reusable, content-free foundation crate: the
/// interactive-world-object MODEL (Interactable / InteractionKind / Pickup / Chest
/// / Breakable + state enums) over the actor + geometry foundations. It must not
/// depend on the game machinery (`ambition_actors`) or name any game content, so
/// another platformer reuses the interaction vocabulary by depending on it.
#[test]
fn architecture_boundaries_interaction_crate_is_foundation_only() {
    let crate_root = repo_root().join("crates/ambition_interaction");
    assert!(
        crate_root.join("Cargo.toml").exists(),
        "ambition_interaction crate should exist at crates/ambition_interaction"
    );
    assert_manifest_has_no_deps(
        &crate_root,
        &[
            "ambition_actors",
            "ambition_content",
            "ambition_render",
            "bevy",
        ],
        "ambition_interaction is a content-free data model over the actor/geometry foundations",
    );
    assert_source_tree_has_no_code_refs(
        crate_root.join("src"),
        &[
            "ambition_actors",
            "ambition_content",
            "gnu_ton",
            "gradient_sentinel",
        ],
        "ambition_interaction must stay content-free + machinery-free",
    );
}

#[test]
fn architecture_boundaries_effects_crate_is_foundation_only() {
    let crate_root = repo_root().join("crates/ambition_vfx");
    assert!(
        crate_root.join("Cargo.toml").exists(),
        "ambition_vfx crate should exist at crates/ambition_vfx"
    );
    assert_manifest_has_no_deps(
        &crate_root,
        &["ambition_actors", "ambition_characters"],
        "ambition_vfx is the reusable effect substrate (Effect vocabulary + \
         Hitbox + executor); it must never depend on the game lib or cast crate",
    );
    assert_source_tree_has_no_code_refs(
        crate_root.join("src"),
        &["ambition_actors", "ambition_characters"],
        "ambition_vfx must stay content-free / foundation-only",
    );
}

#[test]
fn architecture_boundaries_input_crate_is_extracted() {
    let crate_root = repo_root().join("crates/ambition_input");

    assert_paths_exist(
        &crate_root,
        &[
            "Cargo.toml",
            "src/lib.rs",
            "src/actions.rs",
            "src/control.rs",
            "src/menu.rs",
            "src/presets.rs",
            "src/settings.rs",
        ],
        "ambition_input extracted crate",
    );
    assert_manifest_has_no_deps(
        &crate_root,
        &["ambition_actors", "ambition_characters"],
        "ambition_input must stay decoupled from sandbox content and reusable character brains",
    );
    assert_paths_absent(
        &crate_src(),
        &["input.rs", "input", "persistence/settings/controls.rs"],
        "sandbox input facades",
    );

    let lib = fs::read_to_string(crate_src().join("lib.rs")).expect("read sandbox lib.rs");
    assert!(
        !lib.contains("pub use ambition_input as input"),
        "the ambition_actors::input compat shim was removed; \
         import ambition_input by its canonical path, not via ambition_actors"
    );
    // The canonical `controls` re-export moved into `ambition_persistence`
    // during E1a; ambition_actors' settings mod now surfaces it transitively.
    // Assert both links so the input-settings vocabulary stays single-sourced.
    let persistence_settings =
        fs::read_to_string(repo_root().join("crates/ambition_persistence/src/settings/mod.rs"))
            .expect("read ambition_persistence settings mod.rs");
    assert!(
        persistence_settings.contains("pub use ambition_input::settings as controls"),
        "ambition_persistence::settings::controls should re-export ambition_input::settings"
    );
    let settings_mod = fs::read_to_string(crate_src().join("persistence/settings/mod.rs"))
        .expect("read persistence settings mod.rs");
    assert!(
        settings_mod.contains("controls"),
        "ambition_actors persistence::settings should re-surface `controls` \
         from ambition_persistence (the E1a layering)"
    );
}


#[test]
fn architecture_boundaries_control_frame_lives_with_engine_body_contract() {
    let engine_root = repo_root().join("crates/ambition_engine_core");
    let characters_root = repo_root().join("crates/ambition_characters");
    let input_root = repo_root().join("crates/ambition_input");

    assert_paths_exist(
        &engine_root,
        &["src/control_frame.rs"],
        "engine-owned ControlFrame vocabulary",
    );
    let engine_lib = fs::read_to_string(engine_root.join("src/lib.rs"))
        .expect("read ambition_engine_core lib.rs");
    assert!(
        engine_lib.contains("pub mod control_frame")
            && engine_lib.contains("pub use control_frame::ControlFrame"),
        "ControlFrame should be exported from ambition_engine_core beside InputState/reference-frame vocabulary"
    );

    assert_manifest_has_no_deps(
        &characters_root,
        &["ambition_input"],
        "ambition_characters brains consume engine-owned control vocabulary; input is only a device adapter",
    );
    assert_source_tree_has_no_code_refs(
        characters_root.join("src"),
        &["ambition_input", "ambition_input::ControlFrame"],
        "ambition_characters should not depend on the input adapter for brain-facing ControlFrame",
    );

    let input_lib = fs::read_to_string(input_root.join("src/lib.rs"))
        .expect("read ambition_input lib.rs");
    assert!(
        input_lib.contains("pub use ambition_engine_core::ControlFrame"),
        "ambition_input may keep the old import path as a compatibility re-export over engine_core::ControlFrame"
    );
}

#[test]
fn architecture_boundaries_game_mode_lives_with_schedule_vocabulary() {
    let schedule = fs::read_to_string(
        repo_root().join("crates/ambition_platformer_primitives/src/schedule.rs"),
    )
    .expect("read platformer primitives schedule");
    assert!(
        schedule.contains("pub enum GameMode")
            && schedule.contains("pub fn gameplay_allowed")
            && schedule.contains("pub fn gameplay_suspended"),
        "GameMode and its gameplay run conditions should live beside the primitive schedule labels"
    );

    assert_paths_absent(
        &crate_src(),
        &["session/game_mode.rs"],
        "F2.1 deletes the actor-side GameMode compatibility facade once consumers name the lower schedule vocabulary",
    );
    let actors_lib = fs::read_to_string(crate_src().join("lib.rs")).expect("read actors lib");
    assert!(
        !actors_lib.contains("pub use session::game_mode")
            && !actors_lib.contains("pub use game_mode"),
        "ambition_actors must not re-export GameMode or its run conditions after the F2.1 facade burn-down"
    );

    assert_code_refs_absent(
        &[
            crate_src(),
            repo_root().join("crates/ambition_runtime/src"),
            repo_root().join("crates/ambition_sim_view/src"),
            repo_root().join("crates/ambition_touch_input/src"),
            repo_root().join("crates/ambition_render/src"),
            content_src(),
            app_src(),
        ],
        &[
            "ambition_actors::GameMode",
            "ambition_actors::game_mode",
            "ambition_actors::session::game_mode",
            "ambition_actors::gameplay_allowed",
            "ambition_actors::gameplay_suspended",
            "crate::GameMode",
            "crate::gameplay_allowed",
            "crate::game_mode",
            "crate::session::game_mode",
        ],
        "all code should name GameMode through ambition_platformer_primitives::schedule",
    );
}

#[test]
fn architecture_boundaries_f2_actor_facades_burned_down() {
    assert_paths_absent(
        &crate_src(),
        &[
            "session/camera_layers.rs",
            "session/game_mode.rs",
            "time/camera_ease.rs",
        ],
        "F2.1 removes F1-era actor compatibility facades once consumers name the canonical lower crates",
    );

    let actors_lib = fs::read_to_string(crate_src().join("lib.rs")).expect("read actors lib");
    for needle in [
        "pub use session::game_mode",
        "pub use game_mode",
        "CameraEaseState",
        "CameraEaseTuning",
        "SandboxDevState",
    ] {
        assert!(
            !actors_lib.contains(needle),
            "ambition_actors crate root should not keep F1 compatibility facade `{needle}`"
        );
    }

    let possession = fs::read_to_string(
        crate_src().join("abilities/traversal/possession.rs"),
    )
    .expect("read possession module");
    assert!(
        !possession.contains("pub use ambition_platformer_primitives::markers::ControlledSubject"),
        "possession keeps possession behavior; ControlledSubject is named from ambition_platformer_primitives::markers"
    );

    assert_code_refs_absent(
        &[
            repo_root().join("crates/ambition_runtime/src"),
            repo_root().join("crates/ambition_sim_view/src"),
            repo_root().join("crates/ambition_render/src"),
            content_src(),
            app_src(),
        ],
        &[
            "ambition_actors::SandboxDevState",
            "ambition_actors::CameraEaseState",
            "ambition_actors::CameraEaseTuning",
            "ambition_actors::time::camera_ease",
            "ambition_actors::session::camera_layers",
            "ambition_actors::abilities::traversal::possession::ControlledSubject",
            "ambition_actors::features::FeatureEcsWorldOverlay",
        ],
        "F2.1 consumers should use the canonical lower crates instead of actor compatibility facades",
    );
}

#[test]
fn architecture_boundaries_f2_schedule_labels_use_primitive_home() {
    assert_code_refs_absent(
        &[
            repo_root().join("crates/ambition_runtime/src"),
            repo_root().join("crates/ambition_sim_view/src"),
            repo_root().join("crates/ambition_sim_view/tests"),
            content_src(),
            app_src(),
        ],
        &[
            "ambition_actors::schedule::SandboxSet",
            "ambition_actors::schedule::CombatSet",
            "ambition_actors::schedule::BossSteerSlot",
            "ambition_actors::schedule::PresentationSetupSet",
            "ambition_actors::schedule::SimulationSetupSet",
        ],
        "F2 schedule-label pass: pure schedule labels live in ambition_platformer_primitives::schedule; ambition_actors::schedule keeps only the concrete schedule installer and input bridge systems",
    );
}

#[test]
fn architecture_boundaries_f2_asset_vocab_consumers_use_lower_crates() {
    assert_code_refs_absent(
        &[
            repo_root().join("crates/ambition_sim_view/src"),
            content_src(),
            repo_root().join("game/ambition_content/tests"),
            app_src(),
        ],
        &[
            "ambition_actors::assets::game_assets::GameAssetConfig",
            "ambition_actors::assets::game_assets::GameAssets",
            "ambition_actors::assets::game_assets::EntitySprite",
            "ambition_actors::assets::sandbox_assets::SandboxAssetCatalog",
            "ambition_actors::assets::sandbox_assets::ids",
        ],
        "F2 assets pass: pure asset vocabulary is owned by ambition_sprite_sheet / ambition_asset_manager; actor assets may only remain as game-specific loader/catalog adapter seams",
    );
}

#[test]
fn architecture_boundaries_f2_character_anim_vocab_consumers_use_sprite_sheet() {
    assert_code_refs_absent(
        &[
            repo_root().join("crates/ambition_sim_view/src"),
            repo_root().join("crates/ambition_render/src"),
            content_src(),
            app_src(),
        ],
        &["ambition_actors::character_sprites::CharacterAnim"],
        "F2 character-sprites pass: pure character animation row vocabulary lives in ambition_sprite_sheet; actor character_sprites keeps only adapters that read actor facts or content catalogs",
    );
}

#[test]
fn architecture_boundaries_f2_encounter_vocab_consumers_use_encounter_crate() {
    assert_code_refs_absent(
        &[
            repo_root().join("crates/ambition_runtime/src"),
            repo_root().join("crates/ambition_sim_view/src"),
            content_src(),
            app_src(),
            repo_root().join("game/ambition_app/tests/boss_lifecycle.rs"),
        ],
        &[
            "ambition_actors::encounter::EncounterEvent",
            "ambition_actors::encounter::EncounterMobSpec",
            "ambition_actors::encounter::EncounterMusicRequest",
            "ambition_actors::encounter::BossEncounterMusicRequest",
            "ambition_actors::encounter::EncounterPhase",
            "ambition_actors::encounter::EncounterRegistry",
            "ambition_actors::encounter::EncounterRun",
            "ambition_actors::encounter::EncounterSpec",
            "ambition_actors::encounter::EncounterState",
            "ambition_actors::encounter::EncounterWaveSpec",
            "ambition_actors::encounter::LockWallSpec",
            "ambition_actors::encounter::encounter_reward_chest_pos",
            "ambition_actors::encounter::encounter_reward_looted_flag",
            "ambition_actors::encounter::install_encounter_waves",
        ],
        "F2 encounter vocabulary pass: pure encounter state/spec/music/reward vocabulary lives in ambition_encounter; actor encounter keeps LDtk/ECS/schedule adapters",
    );
}


#[test]
fn architecture_boundaries_menu_backend_vocab_consumers_use_menu_crate() {
    assert_code_refs_absent(
        &[app_src(), repo_root().join("crates/ambition_actors/src")],
        &[
            "ambition_actors::menu::backend",
            "crate::menu::backend::InventoryUiBackend",
            "crate::menu::backend::BEVY_UI_MENU_BACKEND_ENABLED",
            "crate::menu::backend::KALEIDOSCOPE_MENU_BACKEND_ENABLED",
        ],
        "F2 menu-backend pass: InventoryUiBackend and backend availability constants live in ambition_menu::backend; actor menu keeps only map/settings adapter residue",
    );
}


#[test]
fn architecture_boundaries_app_menu_settings_vocab_consumers_use_lower_crates() {
    assert_paths_absent(
        &crate_src().join("menu"),
        &["ir"],
        "F2 settings/menu IR closeout deletes the actor-side IR facade once actor persistence names ambition_settings_menu directly",
    );
    assert_code_refs_absent(
        &[app_src(), crate_src()],
        &[
            "ambition_actors::persistence::settings",
            "ambition_actors::menu::ir",
        ],
        "F2 settings/menu IR facade pass: app menu hosts import stored settings from ambition_persistence and renderer-agnostic menu IR from ambition_settings_menu, not through ambition_actors",
    );
}



#[test]
fn architecture_boundaries_map_state_consumers_use_menu_crate() {
    let menu_map = repo_root().join("crates/ambition_menu/src/map.rs");
    assert!(
        menu_map.exists(),
        "F2 map-state pass: renderer-agnostic MapMenuState should live in ambition_menu::map"
    );
    assert_paths_absent(
        &crate_src().join("menu/map"),
        &["model.rs"],
        "F2 map-state closeout deletes the actor-side model facade once runtime/app consumers name ambition_menu::map directly",
    );
    assert_code_refs_absent(
        &[
            app_src(),
            repo_root().join("crates/ambition_sim_view/src"),
            repo_root().join("crates/ambition_runtime/src"),
        ],
        &["ambition_actors::menu::map::MapMenuState"],
        "F2 map-state pass: app/presentation/read-model consumers name ambition_menu::map::MapMenuState; actor menu keeps only the room/save hydration + Bevy-UI adapter systems",
    );
}

#[test]
fn architecture_boundaries_actor_dialog_keeps_only_game_bindings() {
    let dialog = fs::read_to_string(crate_src().join("dialog.rs")).expect("read actor dialog.rs");
    for forbidden in [
        "pub use ambition_dialog",
        "DialogChoice",
        "DialogChoiceSlot",
        "dialog_input",
        "dialog_pointer_input",
        "dialog_reveal_tick",
        "DialogueRunnerEntity",
    ] {
        assert!(
            !dialog.contains(forbidden),
            "actor dialog should not re-export reusable dialog runtime vocabulary after F2 closeout: {forbidden}"
        );
    }
    let yarn = fs::read_to_string(crate_src().join("dialog/yarn_bindings.rs"))
        .expect("read actor dialog/yarn_bindings.rs");
    assert!(
        !yarn.contains("pub use ambition_dialog"),
        "actor Yarn bindings should not re-export generic Yarn binding vocabulary; content names ambition_dialog directly"
    );
}

#[test]
fn architecture_boundaries_actor_persistence_keeps_no_dev_tools_alias() {
    let persistence = fs::read_to_string(crate_src().join("persistence/mod.rs"))
        .expect("read actor persistence mod.rs");
    assert!(
        !persistence.contains("DeveloperPersistenceSchedulePlugin"),
        "actor persistence should not keep the dev-tools persistence compatibility alias after consumers use ambition_dev_tools directly"
    );
}

#[test]
fn architecture_boundaries_dialog_vocab_consumers_use_dialog_crate() {
    assert_code_refs_absent(
        &[
            repo_root().join("crates/ambition_runtime/src"),
            content_src(),
            app_src(),
        ],
        &[
            "ambition_actors::dialog::DialogState",
            "ambition_actors::dialog::DialogChoice",
            "ambition_actors::dialog::DialogChoiceSlot",
            "ambition_actors::dialog::dialog_input",
            "ambition_actors::dialog::dialog_pointer_input",
            "ambition_actors::dialog::dialog_reveal_tick",
            "ambition_actors::dialog::yarn_bindings::YarnContentBindings",
            "ambition_actors::dialog::yarn_bindings::YarnStateMirror",
            "ambition_actors::dialog::yarn_bindings::YarnStateMirrorData",
            "ambition_actors::dialog::yarn_bindings::YarnPresentationCue",
            "ambition_actors::dialog::yarn_bindings::clear_yarn_presentation_cue",
        ],
        "F2 dialog facade pass: reusable dialog state/input/Yarn-binding vocabulary lives in ambition_dialog; actor dialog keeps only Ambition game bindings and GameMode sync plugins",
    );
}

#[test]
fn architecture_boundaries_developer_persistence_uses_dev_tools_crate() {
    assert_code_refs_absent(
        &[app_src()],
        &["ambition_actors::persistence::DeveloperPersistenceSchedulePlugin"],
        "F2 developer-persistence pass: DeveloperTools disk persistence is scheduled through ambition_dev_tools, not the actor persistence facade",
    );
}

#[test]
fn architecture_boundaries_audio_facade_consumers_use_audio_crate() {
    assert_code_refs_absent(
        &[app_src(), repo_root().join("crates/ambition_runtime/src")],
        &[
            "ambition_actors::audio::AudioLibrary",
            "ambition_actors::audio::MusicPlaybackState",
            "ambition_actors::audio::RadioStationState",
            "ambition_actors::audio::MusicChannel",
            "ambition_actors::audio::SfxChannel",
            "ambition_actors::audio::set_radio_track",
        ],
        "F2 audio pass: pure playback vocabulary is owned by ambition_audio::library; \
         ambition_actors::audio keeps only the sandbox audio plugin/environment adapter",
    );
}

/// App-thinness (ADR 0019): the mobile / touch input adapter is a sibling ENGINE
/// crate (`ambition_touch_input`), not host code inside the app binary. It carries
/// no app-only coupling (only the `ambition_input`/`ambition_platformer_primitives`/
/// `ambition_actors`/`render`/`ui_nav`/`cutscene` library seams), so a second
/// platformer host can reuse touch controls by adding the crate — the "second game"
/// oracle. This guards the extraction: the app must WIRE the plugin from the crate,
/// never re-own the adapter under `src/host/`.
#[test]
fn architecture_boundaries_touch_input_crate_is_extracted() {
    let crate_root = repo_root().join("crates/ambition_touch_input");
    assert_paths_exist(
        &crate_root,
        &[
            "Cargo.toml",
            "src/lib.rs",
            "src/state.rs",
            "src/bevy_plugin.rs",
            "src/menu_bridge.rs",
            "src/layout.rs",
            "src/exclusion.rs",
        ],
        "ambition_touch_input extracted crate",
    );
    // The old in-app location is gone — the app no longer OWNS the touch adapter.
    let app_src = repo_root().join("game/ambition_app/src");
    assert_paths_absent(
        &app_src,
        &["host/mobile_input", "host/mobile_input/mod.rs"],
        "in-app touch adapter (moved to ambition_touch_input)",
    );
    // The app WIRES the plugin from the crate, not from a local module path.
    let plugins = fs::read_to_string(app_src.join("app/plugins.rs")).expect("read plugins.rs");
    assert!(
        plugins.contains("ambition_touch_input::TouchControlsPlugin"),
        "the app adds the touch plugin from the ambition_touch_input crate"
    );
    assert!(
        !plugins.contains("crate::host::mobile_input"),
        "the app must not reference the removed in-app mobile_input module path"
    );
}


/// F1.11 ruling: the touch crate is a small presentation/input adapter, not a
/// pure input-model crate. Its direct render dependency is intentional because
/// it owns the on-screen joystick/action-button HUD and keeps the rendered
/// overlay aligned with the raw-touch hit regions. The ratchet is not "remove
/// render"; it is "do not let this adapter grow upward into app/content/host or
/// backend ownership". A future rename/re-home under a presentation grouping can
/// update this test, but the current dependency is accepted.
#[test]
fn architecture_boundaries_touch_input_render_edge_is_intentional() {
    let crate_root = repo_root().join("crates/ambition_touch_input");
    assert_workspace_contains_crate("ambition_touch_input");

    let manifest = fs::read_to_string(crate_root.join("Cargo.toml"))
        .expect("read ambition_touch_input manifest");
    assert!(
        manifest_depends_on(&manifest, "ambition_render"),
        "F1.11: ambition_touch_input owns a visible touch HUD, so its render edge is an intentional presentation-adapter dependency"
    );

    assert_manifest_path_deps_only(
        &crate_root,
        &[
            "ambition_input",
            "ambition_engine_core",
            "ambition_platformer_primitives",
            "ambition_actors",
            "ambition_render",
            "ambition_cutscene",
            "ambition_ui_nav",
            "ambition_persistence",
        ],
        "F1.11: ambition_touch_input is a reusable touch presentation/input adapter; it may name input/render/nav/cutscene/sim seams but not app/content/host/backend crates",
    );

    assert_manifest_has_no_deps(
        &crate_root,
        &[
            "ambition_app",
            "ambition_content",
            "ambition_host",
            "ambition_ldtk_map",
            "ambition_asset_manager",
            "bevy_ecs_ldtk",
        ],
        "ambition_touch_input should remain a reusable presentation/input adapter rather than app/content/host/backend ownership",
    );

    assert_code_refs_absent(
        &[crate_root.join("src")],
        &[
            "ambition_app::",
            "ambition_content::",
            "ambition_host::",
            "ambition_ldtk_map::",
            "bevy_ecs_ldtk::",
        ],
        "touch presentation adapter source should stay out of app/content/host/backend implementation details",
    );
}

#[test]
fn architecture_boundaries_room_feature_spawns_do_not_add_raw_spawns() {
    let src_root = crate_src();
    let spawn_dir = src_root.join("features/ecs");
    assert!(
        spawn_dir.exists(),
        "spawn guardrail path does not exist: {}",
        spawn_dir.display()
    );
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
        "ambition_portal::portal_fire_system",
        "ambition_portal::portal_projectile_step",
        "ambition_portal::portal_transit",
        "ambition_actors::item_pickup::pickup_held_item_system",
        "ambition_actors::item_pickup::throw_held_item_system",
        "ambition_actors::item_pickup::ground_item_physics",
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
fn architecture_boundaries_input_timer_systems_moved_to_actors() {
    // C4 app-thinness: the body-generic input/timer/dev systems that used to live
    // in `app/sim_systems.rs` now live in their owning `ambition_actors`
    // modules. The app must NOT re-DEFINE them (it only references the moved
    // `pub fn`s from the library), so the schedule ordering stays app-owned while
    // the gameplay logic lives down in the library.
    let sim_systems =
        fs::read_to_string(app_src().join("app/sim_systems.rs")).expect("read app/sim_systems.rs");
    let moved = [
        "fn sync_live_player_dev_edits_system",
        "fn apply_suspended_time_scale_system",
        "fn input_timer_system",
        "fn interaction_input_system",
        "fn cleanup_timers_system",
    ];
    let redefined = moved
        .into_iter()
        .filter(|needle| sim_systems.contains(needle))
        .collect::<Vec<_>>();
    assert!(
        redefined.is_empty(),
        "app/sim_systems.rs re-defines library-owned systems: {redefined:?}"
    );

    // The engine schedule (E5 step 5: the shared player-frame wiring lives in
    // `ambition_runtime::PlayerSchedulePlugin`) references the moved systems
    // by their ambition_actors paths.
    let engine_schedule =
        fs::read_to_string(repo_root().join("crates/ambition_runtime/src/player_schedule.rs"))
            .expect("read ambition_runtime/src/player_schedule.rs");
    for needle in [
        "ambition_dev_tools::sync_live_player_dev_edits_system",
        "ambition_actors::time::time_control::apply_suspended_time_scale_system",
        "ambition_actors::player::input_timer_system",
        "ambition_actors::player::interaction_input_system",
        "ambition_actors::player::cleanup_timers_system",
    ] {
        assert!(
            engine_schedule.contains(needle),
            "the engine player schedule must reference the moved system via its library path: {needle}"
        );
    }

    // The two genuinely host/reset-bound systems (they call the app-only
    // `world_flow::reset_sandbox`) DO stay in the app. The replay consumer is
    // the GENERIC one — it drains the engine's `RoomReplayRequested`; the
    // cut-rope emitter lives content-side on `ContentDialogueFollowupSet`
    // (E5-finish de-weave).
    for needle in [
        "fn apply_player_reset_input_system",
        "fn apply_room_replay_request_system",
    ] {
        assert!(
            sim_systems.contains(needle),
            "host/reset-bound system must remain defined in the app: {needle}"
        );
    }
}

#[test]
fn architecture_boundaries_non_portal_mechanics_use_runtime_raycast_seam() {
    let src_root = crate_src();
    let checked_files = [
        "abilities/traversal/blink.rs",
        "abilities/traversal/dive.rs",
        "abilities/traversal/grapple.rs",
        // pickup.rs was split into a `pickup/` dir (Refactor 6); the production
        // raycast usage lives in mod.rs.
        "items/pickup/mod.rs",
    ];
    let mut violations = Vec::new();

    for rel in checked_files {
        let path = src_root.join(rel);
        let text = fs::read_to_string(&path).expect("read source file");
        if text.contains("ambition_portal::raycast_solids") {
            violations.push(format!(
                "{rel} still reaches into portal for a generic solid raycast; use ambition_actors::platformer_runtime::collision::raycast_solids"
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
    let portal_text = fs::read_to_string(repo_root().join("crates/ambition_portal/src/plugin.rs"))
        .expect("read portal plugin source");
    let names_item_set_in_code = portal_text.lines().any(|raw| {
        let line = raw.trim();
        !is_comment_line(line) && line.contains("ambition_actors::items::pickup")
    });
    assert!(
        !names_item_set_in_code,
        "portal/plugin.rs must not name the host item subsystem in code"
    );

    // The portal-set placement lives in the engine group now (E5 step 5:
    // `ambition_runtime::PortalSchedulePlugin`).
    let wiring =
        fs::read_to_string(repo_root().join("crates/ambition_runtime/src/portal_schedule.rs"))
            .expect("read ambition_runtime/src/portal_schedule.rs");
    assert!(
        !wiring.contains("after(ambition_actors::items::pickup::ground_item_physics)"),
        "portal wiring should order PortalSet::Transit against ItemPickupSet, not a concrete function"
    );
    assert!(
        wiring.contains("ambition_actors::items::pickup::ItemPickupSet::CoreHeldItems"),
        "portal wiring should order PortalSet::Transit on the held-item/ground-item simulation set"
    );
}

#[test]
fn architecture_boundaries_portal_core_does_not_import_ambition_content_roster() {
    let roots = [
        repo_root().join("crates/ambition_portal/src"),
        repo_root().join("crates/ambition_portal_presentation/src"),
        crate_src().join("portal"),
    ];
    let forbidden = [
        "ambition_actors::items",
        "Item::PortalGun",
        "OwnedItems",
        "ambition_inventory_ui",
        "ambition_actors::menu::effects",
        "StashedActionSet",
        "ambition_actors::content",
        "ambition_actors::quest",
        "ambition_actors::ldtk_world",
        "ambition_actors::world::ldtk_world",
        "ambition_actors::persistence",
        "ambition_input::ControlFrame",
        "ambition_actors::items::pickup::GroundItem",
    ];
    let violations = scan_code_refs_filtered(
        &roots,
        &forbidden,
        |_| true,
        |file, line| {
            let is_test = file.file_name().and_then(|n| n.to_str()) == Some("tests.rs");
            line.contains("ambition_actors::items::pickup::ItemPickupSet")
                || line.contains("ambition_actors::items::pickup::axe_spec")
                || (is_test && line.contains("ambition_actors::items::pickup::GroundItem"))
                || (is_test && line.contains("ambition_input::ControlFrame"))
        },
    );
    assert!(
        violations.is_empty(),
        "portal core/presentation should consume reusable seams, not Ambition content roster names:
{}",
        violations.join(
            "
"
        )
    );
}

#[test]
fn architecture_boundaries_portal_core_does_not_name_host_world_or_reset() {
    let root = repo_root().join("crates/ambition_portal/src");
    assert_code_refs_filtered(
        &[root],
        &[
            "ambition_actors::features",
            "ambition_engine_core::RoomGeometry",
            "Res<RoomGeometry>",
            "FeatureEcsWorldOverlay",
            "ResetRoomFeaturesEvent",
            "ambition_input::ControlFrame",
            "ambition_actors::player",
        ],
        |file| {
            let name = file.file_name().and_then(|n| n.to_str()).unwrap_or("");
            name != "tests.rs" && name != "presentation.rs"
        },
        |_, _| false,
        "portal core must reach host collision/reset/input through portal-owned resources and content adapters",
    );
}

#[test]
fn architecture_boundaries_music_director_is_content_agnostic() {
    assert_code_refs_filtered(
        &[crate_src().join("music")],
        &["crate::encounter", "crate::rooms", "crate::content"],
        |file| {
            let name = file.file_name().and_then(|n| n.to_str()).unwrap_or("");
            name != "intent.rs" && name != "tests.rs"
        },
        |_, _| false,
        "music director must consume neutral MusicIntent; content mapping belongs in music::intent",
    );
}

#[test]
fn architecture_boundaries_named_content_registers_through_content_plugin() {
    assert_paths_exist(
        &content_src(),
        &[
            "lib.rs",
            "plugin.rs",
            "quests/mod.rs",
            "bosses/mod.rs",
            "dialogue/mod.rs",
            "items/mod.rs",
        ],
        "named Ambition content crate",
    );

    let plugin_text =
        fs::read_to_string(content_src().join("plugin.rs")).expect("read content plugin.rs");
    assert!(plugin_text.contains("pub struct AmbitionContentPlugin"));
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

    let plugins_text =
        fs::read_to_string(app_src().join("app/plugins.rs")).expect("read app/plugins.rs");
    assert!(
        plugins_text.contains("ambition_content::AmbitionContentPlugin"),
        "app/plugins.rs should install AmbitionContentPlugin"
    );

    let sim_resources_text = fs::read_to_string(app_src().join("app/sim_resources.rs"))
        .expect("read app/sim_resources.rs");
    let forbidden_inline = [
        "QuestRegistry::default()",
        "BossEncounterRegistry::default()",
        "default_cutscene_library()",
        "default_room_cutscene_bindings()",
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
        "app/sim_resources.rs still constructs named content inline: {violations:?}"
    );
    assert!(
        !plugins_text.contains("ambition_items::OwnedItems::starter()"),
        "app/plugins.rs should install the item roster through the content plugin"
    );
}

#[test]
fn architecture_boundaries_gravity_zone_mechanic_left_portal() {
    let src_root = crate_src();
    assert_paths_exist(
        &src_root,
        &[
            "gravity/mod.rs",
            "gravity/plugin.rs",
            "gravity/lifecycle.rs",
        ],
        "extracted gravity mechanic",
    );
    // The gravity visuals (`presentation.rs`) moved to the ambition_render crate
    // (sim/render seam completion, 2026-06-15) — gravity is now sim-only.
    assert_paths_exist(
        &repo_root().join("crates/ambition_render/src/rendering"),
        &["gravity_visuals.rs"],
        "gravity visuals live in the render crate",
    );
    let gravity_plugin =
        fs::read_to_string(src_root.join("gravity/plugin.rs")).expect("read gravity plugin");
    assert!(
        gravity_plugin.contains("pub struct GravityPlugin"),
        "gravity mechanic should own a GravityPlugin"
    );
    assert_code_refs_absent(
        &[src_root.join("gravity")],
        &[
            "ambition_portal::",
            "ambition_portal;",
            "ambition_portal}",
            "ambition_actors::portal,",
            "ambition_portal ",
        ],
        "gravity mechanic must be portal-independent",
    );

    let forbidden_in_portal = [
        "GravityFlipSwitch",
        "gravity_flip_switch_system",
        "GravityZoneVisual",
        "GravitySwitchVisual",
        "sync_gravity_zone_visual",
        "sync_gravity_switch_visual",
        "reset_gravity_on_room_reset",
    ];
    let violations = scan_code_refs_filtered(
        &[src_root.join("portal")],
        &forbidden_in_portal,
        |_| true,
        |file, line| {
            file.file_name().and_then(|n| n.to_str()) == Some("tests.rs")
                && line.contains("ambition_actors::gravity")
        },
    );
    assert!(
        violations.is_empty(),
        "portal must not own gravity-zone mechanic symbols:
{}",
        violations.join(
            "
"
        )
    );
}

#[test]
fn architecture_boundaries_abilities_live_under_abilities_layer() {
    let src_root = crate_src();
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
        if src_root.join(format!("{name}.rs")).exists() {
            violations.push(format!(
                "ability {name} must live under src/abilities/, not crate root"
            ));
        }
        if !src_root
            .join(format!("abilities/{subdir}/{name}.rs"))
            .exists()
        {
            violations.push(format!("missing src/abilities/{subdir}/{name}.rs"));
        }
    }
    assert!(
        violations.is_empty(),
        "player abilities should stay under src/abilities/:
{}",
        violations.join(
            "
"
        )
    );

    // The app composes the engine simulation — abilities included — through the
    // `ambition_runtime::PlatformerEnginePlugins` group (E5, the demo gate). The
    // abilities are still assembled via `AmbitionAbilitiesPlugin`, now INSIDE
    // that group, so a demo app gets the same ability kit without touching the
    // app.
    let plugins_text =
        fs::read_to_string(app_src().join("app/plugins.rs")).expect("read app/plugins.rs");
    assert!(
        plugins_text.contains("ambition_runtime::PlatformerEnginePlugins"),
        "app/plugins.rs should compose the engine sim through PlatformerEnginePlugins"
    );
    let runtime_text = fs::read_to_string(repo_root().join("crates/ambition_runtime/src/lib.rs"))
        .expect("read ambition_runtime/src/lib.rs");
    assert!(
        runtime_text.contains("ambition_actors::abilities::AmbitionAbilitiesPlugin"),
        "PlatformerEnginePlugins should compose abilities through AmbitionAbilitiesPlugin"
    );
}

#[test]
fn architecture_boundaries_portal_has_plugin_and_schedule_files_without_actor_facade() {
    let src_root = crate_src();
    let portal_crate_src = repo_root().join("crates/ambition_portal/src");
    assert_paths_exist(
        &portal_crate_src,
        &[
            "lib.rs",
            "plugin.rs",
            "schedule.rs",
            "color.rs",
            "types.rs",
            "gun.rs",
            // Renamed from the flat `pickup.rs` / `shot.rs` to the `gun_`-scoped
            // files as the gun mechanic grew its own family (gun_lifecycle /
            // gun_pickup / gun_projectile). The test tracks the real filenames.
            "gun_pickup.rs",
            "gun_projectile.rs",
            "placement.rs",
            "transit.rs",
            "lifecycle.rs",
            "messages.rs",
            "pieces.rs",
        ],
        "portal mechanic crate",
    );
    assert_paths_absent(
        &src_root,
        &[
            "portal/mod.rs",
            "portal/plugin.rs",
            "portal/transit.rs",
            "portal/gun.rs",
            "portal/presentation.rs",
            "portal.rs",
        ],
        "actor crate after portal facade deletion",
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
    let crate_root = repo_root().join("crates/ambition_portal");
    assert_paths_exist(
        &crate_root,
        &["Cargo.toml", "src/lib.rs"],
        "ambition_portal crate",
    );
    assert_workspace_contains_crate("ambition_portal");
    assert_manifest_has_no_deps(
        &crate_root,
        &[
            "ambition_actors",
            "ambition_content",
            "ambition_input",
            "ambition_sfx",
            "ambition_menu",
        ],
        "ambition_portal must stay host-free",
    );
    assert_manifest_path_deps_only(
        &crate_root,
        &["ambition_engine_core", "ambition_platformer_primitives"],
        "ambition_portal path deps",
    );
    assert_code_refs_absent(
        &[crate_root.join("src")],
        &["ambition_actors"],
        "ambition_portal source must stay content-free",
    );

    let lib_text = fs::read_to_string(crate_root.join("src/lib.rs")).expect("read portal lib.rs");
    let plugin_text =
        fs::read_to_string(crate_root.join("src/plugin.rs")).expect("read portal plugin.rs");
    assert!(
        lib_text.contains("pub use plugin::{PortalPlugin")
            && plugin_text.contains("impl Plugin for PortalPlugin"),
        "ambition_portal should expose a drop-in PortalPlugin"
    );
    assert!(
        !fs::read_to_string(crate_src().join("lib.rs"))
            .expect("read ambition_actors lib.rs")
            .contains("pub mod portal"),
        "ambition_actors must not reintroduce a portal facade; consumers import ambition_portal directly"
    );
}

#[test]
fn architecture_boundaries_portal_presentation_crate_is_extracted() {
    let root = repo_root();
    let crate_root = root.join("crates/ambition_portal_presentation");
    assert_paths_exist(
        &crate_root,
        &["Cargo.toml"],
        "ambition_portal_presentation crate",
    );
    assert_workspace_contains_crate("ambition_portal_presentation");
    assert_manifest_has_no_deps(
        &crate_root,
        &[
            "ambition_actors",
            "ambition_content",
            "ambition_input",
            "ambition_sfx",
            "ambition_menu",
        ],
        "ambition_portal_presentation must stay host-free",
    );
    assert_manifest_path_deps_only(
        &crate_root,
        &[
            "ambition_engine_core",
            "ambition_platformer_primitives",
            "ambition_portal",
        ],
        "ambition_portal_presentation path deps",
    );

    let mechanic_manifest = fs::read_to_string(root.join("crates/ambition_portal/Cargo.toml"))
        .expect("read ambition_portal manifest");
    assert!(
        !mechanic_manifest.contains("ambition_portal_presentation"),
        "headless portal mechanic must not depend on its renderer"
    );
    assert_code_refs_absent(
        &[crate_root.join("src")],
        &["ambition_actors", "ambition_content"],
        "ambition_portal_presentation source must stay host-free",
    );

    let lib_text =
        fs::read_to_string(crate_root.join("src/lib.rs")).expect("read presentation lib.rs");
    assert!(
        lib_text.contains("PortalPresentationPlugin"),
        "ambition_portal_presentation should expose a PortalPresentationPlugin"
    );
    let actors_manifest = fs::read_to_string(root.join("crates/ambition_actors/Cargo.toml"))
        .expect("read ambition_actors manifest");
    assert!(
        !actors_manifest.contains("ambition_portal_presentation"),
        "ambition_actors must not depend on portal presentation now that the facade is gone"
    );
}

#[test]
fn architecture_boundaries_time_crate_is_extracted() {
    let crate_root = repo_root().join("crates/ambition_time");
    assert_paths_exist(
        &crate_root,
        &["Cargo.toml", "src/lib.rs"],
        "ambition_time crate",
    );
    assert_workspace_contains_crate("ambition_time");
    assert_manifest_has_no_deps(
        &crate_root,
        &["ambition_actors"],
        "ambition_time is the reusable time layer",
    );
    assert_code_refs_absent(
        &[crate_root.join("src")],
        &["ambition_actors"],
        "ambition_time source must stay content-free",
    );

    let lib_text = fs::read_to_string(crate_root.join("src/lib.rs")).expect("read time lib.rs");
    assert!(
        lib_text.contains("pub struct TimePlugin")
            && lib_text.contains("impl Plugin for TimePlugin"),
        "ambition_time should expose a TimePlugin"
    );

    let sandbox_time = crate_src().join("time");
    // §D1 (02088cba) removed the `crate::time::{world_time,clock_state,time_control}`
    // re-export facades: callers name `ambition_time::` DIRECTLY. `clock_state.rs`
    // is gone (ClockState is named from the crate); `world_time.rs` is no longer a
    // facade but the sandbox-only sim-dt BRIDGE (`mirror_sim_dt_into_runtime`) that
    // couples ambition_time to the runtime crate's neutral `SimDt`.
    assert_paths_absent(
        &sandbox_time,
        &["clock_state.rs"],
        "clock_state facade removed — ClockState is named from ambition_time directly",
    );
    let world_time = fs::read_to_string(sandbox_time.join("world_time.rs"))
        .expect("read sandbox time/world_time.rs bridge");
    assert!(
        world_time.contains("mirror_sim_dt_into_runtime") && world_time.contains("ambition_time::"),
        "world_time.rs is the sandbox sim-dt bridge that names ambition_time directly, \
         not a re-export facade"
    );
    assert_paths_exist(
        &sandbox_time,
        &["time_control", "feel.rs"],
        "sandbox game-specific time policy",
    );
    assert_paths_absent(
        &sandbox_time,
        &["camera_ease.rs"],
        "camera ease/shake is primitive presentation vocabulary after F2.1 facade burn-down",
    );
}

#[test]
fn architecture_boundaries_f3_2_swept_movers_use_kernel_sweep_sample() {
    let actor_clusters = fs::read_to_string(
        repo_root().join("crates/ambition_actors/src/features/ecs/actor_clusters.rs"),
    )
    .expect("read actor_clusters.rs");
    assert!(
        actor_clusters.contains("pub sweep: &'static mut ae::SweepSample"),
        "F3.2: ECS ActorClusterQueryData must require the shared SweepSample component; \
         boss integration uses this query data, so an optional sweep would silently \
         leave boss bodies on the old fallback path"
    );
    assert!(
        !actor_clusters.contains("pub sweep: Option<&'static mut ae::SweepSample>"),
        "F3.2: runtime actor/boss queries should not make SweepSample optional"
    );

    let portal_transit = fs::read_to_string(
        repo_root().join("crates/ambition_portal/src/transit.rs"),
    )
    .expect("read portal transit.rs");
    assert!(
        portal_transit.contains("Option<&ae::SweepSample>")
            && portal_transit.contains("portal_sweep_sample"),
        "F3.2: portal transit CCD should consume the canonical movement-kernel SweepSample"
    );
    assert!(
        !portal_transit.contains("PortalSweepAnchor"),
        "F3.2: portal-local sweep anchors were retired by the shared SweepSample path"
    );

    let portal_lib = fs::read_to_string(repo_root().join("crates/ambition_portal/src/lib.rs"))
        .expect("read portal lib.rs");
    assert!(
        !portal_lib.contains("PortalSweepAnchor"),
        "F3.2: PortalSweepAnchor should not remain part of the portal public surface"
    );
}

#[test]
fn architecture_boundaries_clock_resets_use_time_control_owner() {
    let roots = [crate_src(), app_src()];
    let violations = scan_code_refs(&roots, &["time_scale = 1.0", "time_scale=1.0"], |file, _| {
        let rel = file.strip_prefix(repo_root()).unwrap_or(file);
        rel == Path::new("crates/ambition_actors/src/time/time_control/mod.rs")
            || rel == Path::new("crates/ambition_actors/src/time/time_control/tests.rs")
    });
    assert!(
        violations.is_empty(),
        "F4.3: reset/respawn/transition code must emit ClockResetRequest; \
         only the time-control owner snaps ClockState.time_scale to 1.0:\n{}",
        violations.join("\n")
    );

    let runtime_schedule = fs::read_to_string(repo_root().join("crates/ambition_runtime/src/player_schedule.rs"))
        .expect("read player_schedule.rs");
    assert!(
        runtime_schedule.contains("apply_clock_reset_requests")
            && runtime_schedule.contains("run_if(gameplay_allowed)"),
        "F4.3: the runtime time-control chain must apply queued ClockResetRequest messages"
    );
}

#[test]
fn architecture_boundaries_player_fallbacks_are_slot_ordered() {
    let roots = [
        repo_root().join("crates/ambition_actors/src/features/ecs/save_sync.rs"),
        repo_root().join("crates/ambition_actors/src/features/ecs/actors/update.rs"),
    ];
    let violations = scan_code_refs(&roots, &["primary_player.iter().next()", "player_query.iter().next()"], |_, _| false);
    assert!(
        violations.is_empty(),
        "F4.4: player fallbacks must be deterministic by PlayerSlot, not raw Bevy query order:
{}",
        violations.join("
")
    );

    for path in roots {
        let text = fs::read_to_string(&path).expect("read player fallback site");
        assert!(
            text.contains("PlayerSlot") && text.contains("AMBITION_REVIEW(determinism)"),
            "F4.4: {} must document the deterministic PlayerSlot fallback",
            path.display()
        );
    }
}

#[test]
fn architecture_boundaries_machinery_does_not_import_content() {
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
        "inventory_ui",
        "items",
        "gravity",
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
        "session",
        "shrine.rs",
        "time",
        "ui_nav",
        "world",
    ];
    let roots = machinery_dirs
        .into_iter()
        .map(|dir| crate_src().join(dir))
        .collect::<Vec<_>>();
    assert_code_refs_filtered(
        &roots,
        &["crate::content::", "ambition_content::"],
        |file| !is_test_file(file),
        |_, _| false,
        "machinery modules must not import the content module",
    );
}

#[test]
fn architecture_boundaries_combat_kit_stays_content_free() {
    assert_code_refs_absent(
        &[crate_src().join("combat")],
        &[
            "crate::content",
            "crate::ambition_content",
            "CharacterArchetype",
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
        ],
        "generic combat kit must not name Ambition content",
    );
}

#[test]
fn architecture_boundaries_presentation_does_not_use_the_archetype_enum() {
    assert_code_refs_filtered(
        &[crate_src().join("presentation")],
        &["CharacterArchetype", "is_composite_spawn", "sandbag_"],
        |file| !is_test_file(file),
        |_, _| false,
        "presentation should consume authored visual data, not the archetype enum",
    );
}

#[test]
fn architecture_boundaries_enemy_sim_reads_data_not_the_archetype_enum() {
    // The pure per-frame surfaces (the damage hook + the presentation
    // resolver) must branch on projected tuning / capabilities, never the
    // named roster enum. `features/ecs/actors.rs` is deliberately NOT here:
    // it now mixes per-frame tick helpers (which read `caps`) with
    // spawn-time NPC→enemy conversion (which legitimately names the roster
    // to resolve a spawn archetype). The structural invariant that the
    // PERSISTED component never carries the enum is enforced separately by
    // `architecture_boundaries_enemy_config_is_archetype_free`.
    let files = [
        crate_src().join("features/ecs/damage/mod.rs"),
        // The presentation feature-resolver moved to the ambition_render crate.
        repo_root().join("crates/ambition_render/src/rendering/features.rs"),
    ];
    assert_production_lines_have_no_refs(
        &files,
        &["CharacterArchetype", ".archetype"],
        "per-frame enemy sim should read projected tuning/capabilities, not named archetypes",
    );
}

#[test]
fn architecture_boundaries_enemy_config_is_archetype_free() {
    // The spawn-seam milestone: the DURABLE enemy component (`ActorConfig`)
    // and the per-frame mutable view (`ActorMut`) carry projected generic
    // kit data — `tuning`, `brain_spec`, and the `CombatCapabilities`
    // component — so neither the per-frame integration nor the runtime
    // brain rebuilds (provoke, dismount) call back into the named roster.
    // That is what lets the roster (`CharacterArchetype` + specs + RON) leave
    // the machinery lib for `ambition_content`. The spawn-time
    // `ActorClusterSeed` is allowed to carry the enum (it is consumed
    // before the entity exists), so this guards only the durable structs.
    let text = fs::read_to_string(crate_src().join("features/ecs/actor_clusters.rs"))
        .expect("read actor_clusters.rs");
    for struct_name in ["pub struct ActorConfig {", "pub struct ActorMut<'a> {"] {
        let start = text
            .find(struct_name)
            .unwrap_or_else(|| panic!("{struct_name} not found in actor_clusters.rs"));
        let body = &text[start..];
        let end = body
            .find("\n}")
            .expect("struct should have a closing brace");
        // Skip doc/comment lines — a field's prose may legitimately mention
        // "projected from the archetype" while the field itself is generic.
        let violations: Vec<&str> = body[..end]
            .lines()
            .map(str::trim)
            .filter(|line| !is_comment_line(line))
            .filter(|line| line.contains("CharacterArchetype") || line.contains("archetype:"))
            .collect();
        assert!(
            violations.is_empty(),
            "{struct_name} must stay archetype-free — project generic kit data \
             (tuning / brain_spec / caps) at spawn instead of storing the roster \
             enum; offending field(s): {violations:?}",
        );
    }
}

#[test]
fn architecture_boundaries_lib_menu_keeps_only_the_coupled_pieces() {
    let menu_dir = crate_src().join("menu");
    assert_paths_absent(
        &menu_dir,
        &[
            "model.rs",
            "dispatch.rs",
            "effects.rs",
            "grid_backend.rs",
            "kaleidoscope_app.rs",
            "backend.rs",
        ],
        "lib menu should not regain app-host/backend menu files",
    );
    assert_paths_exist(
        &menu_dir,
        &["map"],
        "lib menu keeps only map hydration/UI adapters after settings IR moved to ambition_settings_menu",
    );
    assert_paths_exist(
        &app_src().join("menu"),
        &["kaleidoscope_app.rs", "grid_backend.rs", "model.rs"],
        "app menu host stack",
    );
}

#[test]
fn architecture_boundaries_dev_overlays_live_in_app() {
    let lib_dev = crate_src().join("dev");
    assert_paths_absent(
        &lib_dev,
        &["debug_overlay.rs", "fps_overlay.rs"],
        "presentation-only dev overlays should live in ambition_app::dev",
    );
    // E1d: the dev-tool STATE + startup profiler moved into the foundational
    // `ambition_dev_tools` crate. Only the sim-coupled trace recorder stays.
    assert_paths_absent(
        &lib_dev,
        &["dev_tools", "profiling.rs"],
        "dev-tool state + profiling moved to ambition_dev_tools (E1d)",
    );
    assert_paths_exist(
        &lib_dev,
        &["trace.rs", "trace"],
        "sim-coupled trace recorder stays sim-side",
    );
    assert_paths_exist(
        &repo_root().join("crates/ambition_dev_tools/src"),
        &["dev_tools", "profiling.rs", "persistence.rs"],
        "ambition_dev_tools owns the dev-tool state, profiler, and developer persistence",
    );
    assert_paths_exist(
        &app_src().join("dev"),
        &["debug_overlay.rs", "fps_overlay.rs"],
        "app dev overlay files",
    );

    assert_code_refs_absent(
        &[
            app_src(),
            repo_root().join("crates/ambition_runtime/src"),
            repo_root().join("crates/ambition_sim_view/src"),
        ],
        &[
            "ambition_actors::dev::dev_tools",
            "ambition_actors::dev::profiling",
            "ambition_actors::dev::sync_live_player_dev_edits_system",
        ],
        "external consumers must name ambition_dev_tools directly; ambition_actors::dev keeps only trace",
    );
}

#[test]
fn architecture_boundaries_actor_crate_is_content_free_and_foundation_clean() {
    assert_code_refs_absent(
        &[repo_root().join("crates/ambition_characters/src")],
        &[
            "ambition_actors",
            "ambition_content",
            "ambition_app",
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
            "CharacterArchetype",
        ],
        "ambition_characters must stay content-free with no upward imports",
    );
}
