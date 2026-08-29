//! Every art path declared by the assembled host must resolve to a mounted file.
//! This checks declaration-to-file resolution across the real provider composition.

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
fn resolves(declared: &str, roots: &[PathBuf]) -> bool {
    let relative = declared.rsplit("://").next().unwrap_or(declared);
    roots.iter().any(|root| root.join(relative).is_file())
}

/// Non-vacuity control: a path that does not exist must fail resolution.
#[test]
fn the_resolver_can_actually_report_a_missing_file() {
    let roots = asset_roots();
    assert!(!roots.is_empty(), "no asset root resolved");
    assert!(
        !resolves("sprites/a_file_nobody_ever_drew.png", &roots),
        "the resolver answers YES for a path that does not exist, so every          emptiness assertion in this file is green by construction"
    );
    assert!(
        !resolves("game://sprites/a_file_nobody_ever_drew.png", &roots),
        "…and the same through the `game://` road, which is the one content          actually declares"
    );
    // …and it says yes to something real, so it is not simply always false.
    assert!(
        resolves("sprites/alice_spritesheet.png", &roots),
        "the resolver cannot find a sheet that is definitely there — the roots          are wrong and these checks are measuring the wrong tree"
    );
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
         Either generate the target (scripts/regen/sprites.sh) or stop declaring the id.",
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
    use ambition_platformer2d::projectiles::visual::{
        ProjectileArtSource, ProjectileVisualCatalog,
    };

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

/// A character's FACE, which is the fourth member of this family and the one
/// a select screen found.
///
/// `CharacterCatalog::portrait_ref` derives `sprites/<stem>_portraits.png` from
/// the gameplay spritesheet's own name. That convention is good — a character
/// gets a portrait for free — and it has the exact failure this file exists for:
/// a derived path is not a promise that the art was generated. The path
/// resolves, the asset server fails the load asynchronously, the `ImageNode`
/// draws nothing, and every layer below is silent.
///
/// This asks the ASSEMBLED catalog, which is the same object the UI asks.
#[test]
fn every_catalog_character_that_derives_a_portrait_has_the_art() {
    use ambition_platformer2d::character::CharacterCatalog;

    /// Characters whose portrait art was never generated.
    ///
    /// One missing generator target, six blank faces, no error anywhere.
    ///
    /// It closed in one command per form —
    /// `sprite2d_renderer portraits mary_o_v2{,_fire,_tall}`. The renderer
    /// declared both products for those targets all along (`portrait-files`
    /// answers with them) and had simply never been RUN for them, while
    /// `super_mary_o_portraits.png` sat next door looking like coverage. The
    /// sheet name diverged; the pipeline never broke.
    ///
    /// asserted as a SET, so it holds in both directions. A new character
    /// with no art fails here, and so does an entry left behind after its art
    /// arrives — the same staleness the rollback resource ratchet had to grow a
    /// second assert to prevent.
    ///
    /// Asking the ASSEMBLED resource is why this one is right; the regex was reading the places
    /// I already knew to look.
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
        missing, known,
        "the set of characters with no portrait art has CHANGED.\n\
         If it grew: a character now derives a portrait path with no file behind \
         it, and any UI that draws a face — the dialogue box, the smash select \
         grid — will show a blank where that character should be, silently.\n\
         If it shrank: the art arrived. Update KNOWN_MISSING, or this list \
         becomes a confident description of a shape that has moved."
    );
}

/// The SHEET, which is the stronger form of the question above.
///
/// A portrait is a face; the spritesheet is the character. Generated art is gitignored, so a
/// sheet nobody's batch produces exists only on the machine that once rendered it, and the
/// failure shows up as a clone with no character in it.
///
/// and it CANNOT answer the fresh-clone question, which is the one that bites. Generated art is
/// gitignored, so this test sees whatever the machine running it happens to have rendered — a sheet
/// that no batch publishes but that was made by hand a year ago passes here and is absent on a
/// clone. What it does catch is a catalog row naming art that is nowhere at all, which is the typo
/// case.
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
         whose sheet no `scripts/regen/sprites.sh` batch publishes exists only on a \
         machine that once rendered it by hand — generated art is gitignored, \
         so a fresh clone gets a character with no body."
    );
}

/// AND THE MANIFEST, WHICH IS THE HALF THAT SAYS WHERE THE FRAMES ARE.
///
/// ⭐⭐ A ROW DECLARES TWO FILES AND ONLY ONE OF THEM WAS CHECKED. The sibling
/// above pins `spritesheet` — the pixels — and a missing one is a character with
/// no body. `manifest` is the `.ron` beside it that carries every frame rect,
/// anchor and clip: a row naming a manifest that is nowhere at all has no
/// geometry to draw the pixels WITH, which is the worse of the two failures and
/// the one nothing was asking about.
///
/// ⛔ SAME LIMIT AS ITS SIBLING, stated so it is not mistaken for more: generated
/// art is gitignored, so this sees whatever the machine running it has rendered.
/// It catches the TYPO — a row naming a file that exists nowhere — and cannot
/// answer the fresh-clone question.
#[test]
fn every_catalog_character_names_a_manifest_that_exists() {
    use ambition_platformer2d::character::CharacterCatalog;

    /// Characters whose sheet manifest no regen batch produces.
    const KNOWN_MISSING: &[&str] = &[];

    let app = build_visible_app(VisibleRenderMode::NoWindow, true);
    let roots = asset_roots();
    assert!(!roots.is_empty(), "no asset root resolved");

    let catalog = app
        .world()
        .get_resource::<CharacterCatalog>()
        .expect("the composed host has an assembled character catalog");
    // The premise: this measures nothing if the catalog is empty, and an empty
    // catalog is exactly what a composition failure looks like from here.
    assert!(
        catalog.iter().count() > 1,
        "the assembled catalog carries {} rows, so this census cannot fail",
        catalog.iter().count()
    );
    let mut missing: Vec<String> = catalog
        .iter()
        .filter(|(_, entry)| !resolves(&entry.manifest, &roots))
        .map(|(id, _)| id.clone())
        .collect();
    missing.sort();
    missing.dedup();

    let known: Vec<String> = KNOWN_MISSING.iter().map(|id| id.to_string()).collect();
    assert_eq!(
        missing, known,
        "the set of characters with no sheet MANIFEST has CHANGED. A row whose \
         `manifest` names nothing has no frame rects, so the character draws \
         from a sheet nobody can index — the pixels being present does not help."
    );
}
