//! **Every declared art id must name a file that exists.**
//!
//! [`docs/planning/triage/declared-id-resolution-checks.md`] opened this in
//! 2026-07-25 after a Mary-O playtest round, with a table of five instances and
//! one sentence that is the whole reason it matters:
//!
//! > Every row in the table above was found by a player noticing something
//! > missing, not by a test.
//!
//! The failure is that `Option` does two jobs at once — *"this build
//! legitimately has no assets"* and *"this content named something that does not
//! exist"* — and they are indistinguishable at the call site, so the second
//! silently inherits the first's tolerance. Content declares a `sprite` id, the
//! resolution returns `None`, the caller treats it as an art-free build, and the
//! pickup **simulates perfectly while producing nothing at all**. No warning, no
//! log line, no failing test.
//!
//! Jon ruled out the obvious fix — a boot-time validation pass — on startup cost:
//! *"it puts the cost on every launch forever to catch a class of mistake that is
//! made at authoring time."* The triage's recommendation is a TEST as the gate,
//! which costs the shipped binary nothing, and it named the direction that was
//! missing: the existing `every_*` tests check target → catalog, and every one of
//! those five bugs was the other way round, a declared id → a target that does
//! not exist.
//!
//! ⚠ This asserts against the composed SHIPPED host, not against a fixture. A
//! provider that declares art nobody generated is exactly the case, and only the
//! real composition knows which providers are in the build.

use std::path::{Path, PathBuf};

use ambition_app::app::{build_visible_app, VisibleRenderMode};

/// The two asset roots the desktop host mounts: the engine's own tree and the
/// `game://` source a provider addresses its content through.
///
/// Both, because a declared path may legitimately live in either — and a check
/// that knew only one would report a provider's real art as missing, which is
/// the noise that gets a guard waived.
fn asset_roots() -> Vec<PathBuf> {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    [
        manifest.join("../../crates/ambition_actors/assets"),
        manifest.join("../ambition_content/assets"),
    ]
    .into_iter()
    .filter_map(|path| path.canonicalize().ok())
    .filter(|path| path.is_dir())
    .collect()
}

/// Whether `declared` names a real file under any mounted root.
///
/// A `game://`-qualified path is stripped to its tail: source qualification says
/// WHOSE tree to look in, and this check is about whether the file is anywhere at
/// all. Being permissive here is deliberate — the failure being caught is "no
/// such file", not "wrong source".
fn resolves(declared: &str, roots: &[PathBuf]) -> bool {
    let relative = declared.rsplit("://").next().unwrap_or(declared);
    roots.iter().any(|root| root.join(relative).is_file())
}

#[test]
fn every_declared_world_item_art_path_names_a_file_that_exists() {
    use ambition::platformer::world_item_art::WorldItemArtManifest;

    let app = build_visible_app(VisibleRenderMode::NoWindow, true);
    let roots = asset_roots();
    assert!(
        !roots.is_empty(),
        "no asset root resolved, so this test cannot see any file and would \
         pass or fail for the wrong reason"
    );

    let manifest = app
        .world()
        .get_resource::<WorldItemArtManifest>()
        .expect("the composed host declares world-item art");
    let entries = manifest.effective();
    assert!(
        !entries.is_empty(),
        "the composed host declares no world-item art at all — this is asserting \
         over an empty list, which is how a check reports success about nothing"
    );

    let missing: Vec<String> = entries
        .iter()
        .filter(|entry| !resolves(&entry.asset_path, &roots))
        .map(|entry| format!("  {} -> {}", entry.sprite_id, entry.asset_path))
        .collect();

    assert!(
        missing.is_empty(),
        "{} declared world-item art path(s) name no file:\n{}\n\n\
         The id is registered and the lookup returns `None`, which the render \
         path cannot distinguish from an art-free build — so the pickup spawns, \
         magnetizes, credits, and is INVISIBLE. That is a fire-flower you collect \
         and never see, and the only thing that found the last one was a player.\n\
         Either generate the target (regen_sprites.sh) or stop declaring the id.",
        missing.len(),
        missing.join("\n"),
    );
}

/// The same question for PROJECTILE art, which resolves through a catalog whose
/// miss behaviour is even more forgiving: `ProjectileVisualCatalog::resolve`
/// falls back to the generic hostile shot for an unregistered id. That is right
/// at runtime — a bolt nobody skinned should still be visible — and it means a
/// declared image naming no file looks exactly like a shot nobody skinned.
///
/// Only `Image` and `Sheet` sources are checkable: the tinted and solid-colour
/// ones name no file by construction.
#[test]
fn every_declared_projectile_image_names_a_file_that_exists() {
    use ambition::projectiles::visual::{ProjectileArtSource, ProjectileVisualCatalog};

    let app = build_visible_app(VisibleRenderMode::NoWindow, true);
    let roots = asset_roots();
    assert!(!roots.is_empty(), "no asset root resolved");

    let catalog = app
        .world()
        .get_resource::<ProjectileVisualCatalog>()
        .expect("the composed host has a projectile visual catalog");

    let declared: Vec<(&str, &str)> = catalog
        .iter()
        .filter_map(|(id, art)| match &art.source {
            ProjectileArtSource::Image { path } => Some((id, path.as_str())),
            _ => None,
        })
        .collect();

    // No `Image` sources at all is a legitimate state (every shot tinted or
    // sheet-backed), so this reports rather than asserts — an empty list here is
    // not the "checking nothing" failure it is for world items, where the shipped
    // host definitely declares some.
    if declared.is_empty() {
        eprintln!("[declared-art] no projectile Image sources declared");
        return;
    }

    let missing: Vec<String> = declared
        .iter()
        .filter(|(_, path)| !resolves(path, &roots))
        .map(|(id, path)| format!("  {id} -> {path}"))
        .collect();

    assert!(
        missing.is_empty(),
        "{} declared projectile image(s) name no file:\n{}\n\n\
         `resolve` falls back to the generic shot, so this does not crash and \
         does not log — the projectile flies with the wrong look and nothing \
         says which one was meant.",
        missing.len(),
        missing.join("\n"),
    );
}
