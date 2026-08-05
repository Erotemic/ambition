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
        manifest.join("../../crates/ambition_platformer2d_actor_monolith/assets"),
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
    use ambition_platformer2d::platformer::world_item_art::WorldItemArtManifest;

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
    use ambition_platformer2d::projectiles::visual::{ProjectileArtSource, ProjectileVisualCatalog};

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

/// The AUDIO half of the same question, and the row from the triage table that
/// says `mary_o_you_died` — "a cue requested successfully, into silence".
///
/// `every_live_music_track_resolves_under_web_served_assets` already asserts the
/// catalog produces a PATH for every track. That is the target → catalog
/// direction. This is the other one: the path names a FILE. A track whose OGG
/// was never rendered resolves, loads nothing, and plays nothing — the radio
/// simply has a silent station, which is indistinguishable from a quiet moment.
#[test]
fn every_declared_music_track_path_names_a_file_that_exists() {
    use ambition_platformer2d::audio::spec::MusicRegistry;

    let app = build_visible_app(VisibleRenderMode::NoWindow, true);
    let roots = asset_roots();
    assert!(!roots.is_empty(), "no asset root resolved");

    let Some(registry) = app.world().get_resource::<MusicRegistry>() else {
        panic!("the composed host has no music registry to check");
    };
    assert!(
        !registry.tracks.is_empty(),
        "the music registry is empty — this would pass about nothing"
    );

    // `asset_path: None` is a legitimate declaration: the track exists as an id
    // with no file yet, and the catalog already drops it loudly on the web
    // profile (see the sibling test). This checks the ones that DO name a file.
    let missing: Vec<String> = registry
        .tracks
        .iter()
        .filter_map(|track| track.asset_path.as_ref().map(|path| (track, path)))
        .filter(|(_, path)| !resolves(path, &roots))
        .map(|(track, path)| format!("  {} -> {path}", track.id))
        .collect();

    assert!(
        missing.is_empty(),
        "{} declared music track path(s) name no file:\n{}\n\n\
         The id resolves, the load finds nothing, and the station is silent — \
         which sounds exactly like a quiet moment. `scripts/regen_music_registry.py` \
         generates this registry; a path in it with no OGG behind it means the \
         render step was skipped.",
        missing.len(),
        missing.join("\n"),
    );
}

/// **A character's FACE, which is the fourth member of this family and the one
/// a select screen found.**
///
/// `CharacterCatalog::portrait_ref` derives `sprites/<stem>_portraits.png` from
/// the gameplay spritesheet's own name. That convention is good — a character
/// gets a portrait for free — and it has the exact failure this file exists for:
/// **a derived path is not a promise that the art was generated.** The path
/// resolves, the asset server fails the load asynchronously, the `ImageNode`
/// draws nothing, and every layer below is silent.
///
/// ⛔ found on 2026-08-05 by LOOKING at the new smash character-select screen:
/// Mary-O's cell was a hole. `inspect_hall_portraits.py` existed the whole time
/// and could not have caught it — it reads ONE catalog file and filters to rows
/// with a `hall_dialogue_id`, so a character outside the Hall, or declared by a
/// demo's own Rust fragment, was never in its population. This asks the
/// ASSEMBLED catalog, which is the same object the UI asks.
#[test]
fn every_catalog_character_that_derives_a_portrait_has_the_art() {
    use ambition_platformer2d::character::CharacterCatalog;

    /// **Characters whose portrait art was never generated.**
    ///
    /// ⭐ **EMPTY, and it was six on the morning of 2026-08-05.** Every one of
    /// them wore the `mary_o_v2` sheet family, and FOUR separate surfaces drew
    /// from it: the Mary-O demo's three forms, Pocket's runner, TwinTrack's
    /// traveller, and Ambition's own versus arena. One missing generator target,
    /// six blank faces, no error anywhere.
    ///
    /// It closed in one command per form —
    /// `sprite2d_renderer portraits mary_o_v2{,_fire,_tall}`. The renderer
    /// declared both products for those targets all along (`portrait-files`
    /// answers with them) and had simply never been RUN for them, while
    /// `super_mary_o_portraits.png` sat next door looking like coverage. The
    /// sheet name diverged; the pipeline never broke.
    ///
    /// ⚠ **asserted as a SET, so it holds in both directions.** A new character
    /// with no art fails here, and so does an entry left behind after its art
    /// arrives — the same staleness the rollback resource ratchet had to grow a
    /// second assert to prevent.
    ///
    /// ⚠ **a hand-rolled census over the catalog SOURCE found five of the six
    /// and missed `arena_duelist_close`**, which is declared in Rust inside
    /// `ambition_app` rather than in any catalog RON. Asking the ASSEMBLED
    /// resource is why this one is right; the regex was reading the places I
    /// already knew to look.
    const KNOWN_MISSING: &[&str] = &[];

    let app = build_visible_app(VisibleRenderMode::NoWindow, true);
    let roots = asset_roots();
    assert!(!roots.is_empty(), "no asset root resolved");

    let catalog = app
        .world()
        .get_resource::<CharacterCatalog>()
        .expect("the composed host has an assembled character catalog");
    assert!(
        catalog.len() > 100,
        "the assembled catalog holds only {} characters — this composition is \
         not the shipped one and the check would be about almost nothing",
        catalog.len()
    );

    let ids: Vec<String> = catalog.iter().map(|(id, _)| id.clone()).collect();
    let mut missing: Vec<String> = ids
        .iter()
        .filter_map(|id| catalog.portrait_ref(id).map(|portrait| (id, portrait)))
        .filter(|(_, portrait)| !resolves(&portrait.image, &roots))
        .map(|(id, _)| id.clone())
        .collect();
    missing.sort();

    let known: Vec<String> = KNOWN_MISSING.iter().map(|id| id.to_string()).collect();
    assert_eq!(
        missing,
        known,
        "the set of characters with no portrait art has CHANGED.\n\
         If it grew: a character now derives a portrait path with no file behind \
         it, and any UI that draws a face — the dialogue box, the smash select \
         grid — will show a blank where that character should be, silently.\n\
         If it shrank: the art arrived. Update KNOWN_MISSING, or this list \
         becomes a confident description of a shape that has moved."
    );
}

/// **The SHEET, which is the stronger form of the question above.**
///
/// A portrait is a face; the spritesheet is the character. `regen_sprites.sh`
/// publishes an EXPLICIT list of targets, and its own comments record the last
/// time this went wrong — five Hall characters *"that previously depended on
/// manually generated local assets and therefore disappeared on a fresh
/// clone"*. Generated art is gitignored, so a sheet nobody's batch produces
/// exists only on the machine that once rendered it, and the failure shows up
/// as a clone with no character in it.
///
/// ⚠ **and it CANNOT answer the fresh-clone question, which is the one that
/// bites.** Generated art is gitignored, so this test sees whatever the machine
/// running it happens to have rendered — a sheet that no batch publishes but
/// that was made by hand a year ago passes here and is absent on a clone. What
/// it does catch is a catalog row naming art that is nowhere at all, which is
/// the typo case. The fresh-clone question is answered by `regen_sprites.sh`'s
/// own `expected_files` postcondition; the fix for `mary_o_v2` on 2026-08-05
/// went THERE, and this is its cheap companion.
#[test]
fn every_catalog_character_names_a_spritesheet_that_exists() {
    use ambition_platformer2d::character::CharacterCatalog;

    /// Characters whose spritesheet no regen batch produces.
    const KNOWN_MISSING: &[&str] = &[];

    let app = build_visible_app(VisibleRenderMode::NoWindow, true);
    let roots = asset_roots();
    assert!(!roots.is_empty(), "no asset root resolved");

    let catalog = app
        .world()
        .get_resource::<CharacterCatalog>()
        .expect("the composed host has an assembled character catalog");
    let mut missing: Vec<String> = catalog
        .iter()
        .filter(|(_, entry)| !resolves(&entry.spritesheet, &roots))
        .map(|(id, _)| id.clone())
        .collect();
    missing.sort();
    missing.dedup();

    let known: Vec<String> = KNOWN_MISSING.iter().map(|id| id.to_string()).collect();
    assert_eq!(
        missing, known,
        "the set of characters with no spritesheet has CHANGED. A character \
         whose sheet no `regen_sprites.sh` batch publishes exists only on a \
         machine that once rendered it by hand — generated art is gitignored, \
         so a fresh clone gets a character with no body."
    );
}
