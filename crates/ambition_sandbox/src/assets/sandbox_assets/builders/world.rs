//! World / level data builders: LDtk projects (`sandbox.ldtk`,
//! `intro.ldtk`, `you_have_to_cut_the_rope.ldtk`) and the sandbox tuning RON.

use ambition_asset_manager::{
    AssetEntry, AssetKind, AssetLocation, AssetManifest, AssetSourceProfile, MissingAssetPolicy,
    PreloadGroup,
};

use super::super::ids;
use super::super::{
    EMBEDDED_CUT_ROPE_LDTK_ASSET_PATH, EMBEDDED_INTRO_LDTK_ASSET_PATH,
    EMBEDDED_SANDBOX_LDTK_ASSET_PATH,
};

/// LDtk world entries. The primary `world.sandbox_ldtk` is required —
/// the game cannot run without it. Secondary worlds (`world.intro_ldtk`
/// and `world.cut_rope_ldtk` today) are optional: the merge loader skips them silently if the
/// catalog reports them disabled, matching the prior "tolerate missing
/// secondary file" behavior.
///
/// Explicit `LooseFilesystem` candidates carry the absolute
/// `CARGO_MANIFEST_DIR/assets/...` path so the desktop hot-reload
/// watcher (primary world only) can find a `LocalPath` to inotify.
pub(in super::super) fn extend_with_world_entries(manifest: &mut AssetManifest) {
    let loose_sandbox = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("assets")
        .join(crate::ldtk_world::SANDBOX_LDTK_ASSET);
    manifest.insert(
        AssetEntry::new(
            ids::sandbox_ldtk(),
            AssetKind::LdtkProject,
            crate::ldtk_world::SANDBOX_LDTK_ASSET,
        )
        .with_missing_policy(MissingAssetPolicy::Error)
        .with_preload_group(PreloadGroup::Bootstrap)
        .with_location(
            AssetSourceProfile::LooseFilesystem,
            AssetLocation::LocalPath(loose_sandbox),
        )
        .with_location(
            AssetSourceProfile::EmbeddedBinary,
            AssetLocation::embedded(EMBEDDED_SANDBOX_LDTK_ASSET_PATH),
        ),
    );

    // intro.ldtk lives next to sandbox.ldtk and is loaded by the
    // secondary-worlds merge step. Optional today because a fresh
    // checkout without the intro file should still boot the sandbox.
    let loose_intro = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("assets")
        .join("ambition/worlds/intro.ldtk");
    manifest.insert(
        AssetEntry::new(
            ids::intro_ldtk(),
            AssetKind::LdtkProject,
            "ambition/worlds/intro.ldtk",
        )
        .with_missing_policy(MissingAssetPolicy::WarnAndPlaceholder)
        .with_preload_group(PreloadGroup::Bootstrap)
        .with_location(
            AssetSourceProfile::LooseFilesystem,
            AssetLocation::LocalPath(loose_intro),
        )
        .with_location(
            AssetSourceProfile::EmbeddedBinary,
            AssetLocation::embedded(EMBEDDED_INTRO_LDTK_ASSET_PATH),
        ),
    );

    // Cut-rope boss arena lives in its own LDtk file while the Hall of Bosses
    // remains in sandbox.ldtk. Optional for the same reason as intro.ldtk:
    // a partial checkout can still boot the sandbox without the side world.
    let loose_cut_rope = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("assets")
        .join("ambition/worlds/you_have_to_cut_the_rope.ldtk");
    manifest.insert(
        AssetEntry::new(
            ids::cut_rope_ldtk(),
            AssetKind::LdtkProject,
            "ambition/worlds/you_have_to_cut_the_rope.ldtk",
        )
        .with_missing_policy(MissingAssetPolicy::WarnAndPlaceholder)
        .with_preload_group(PreloadGroup::Bootstrap)
        .with_location(
            AssetSourceProfile::LooseFilesystem,
            AssetLocation::LocalPath(loose_cut_rope),
        )
        .with_location(
            AssetSourceProfile::EmbeddedBinary,
            AssetLocation::embedded(EMBEDDED_CUT_ROPE_LDTK_ASSET_PATH),
        ),
    );
}

/// Sandbox tuning RON entry. Required — the game refuses to run
/// without it. Today the live consumer is
/// [`crate::content::data::SandboxDataSpec::load_embedded`] (always via
/// `include_str!`); the catalog entry exists so future code that asks
/// for the Bevy path under a non-static profile gets a real answer.
pub(in super::super) fn extend_with_data_entries(manifest: &mut AssetManifest) {
    manifest.insert(
        AssetEntry::new(
            ids::sandbox_data(),
            AssetKind::RonData,
            "ambition/sandbox.ron",
        )
        .with_missing_policy(MissingAssetPolicy::Error)
        .with_preload_group(PreloadGroup::Bootstrap),
    );
}
