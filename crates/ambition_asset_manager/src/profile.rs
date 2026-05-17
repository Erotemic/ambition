//! [`AssetProfile`] — the active platform/runtime persona that drives
//! which [`crate::location::AssetLocation`] the resolver returns for a
//! given [`crate::AssetId`].
//!
//! The profile is set once per app session (typically from a CLI flag or
//! compile-time `cfg`) and lives behind the
//! [`crate::bevy_integration::AmbitionAssetProfile`] resource on Bevy
//! builds.
//!
//! ## Profile vs Source
//!
//! [`AssetProfile`] is the *high-level persona* (Desktop, Web,
//! Android, ...). Each profile picks an ordered list of
//! [`AssetSourceProfile`] kinds it understands; the resolver walks an
//! [`crate::manifest::AssetEntry`]'s authored locations and returns the
//! first one whose source kind is enabled for the active profile.

use serde::{Deserialize, Serialize};

/// Active platform/runtime persona. One value per app session.
///
/// See `docs/asset_manager.md` for the per-profile contract (preferred
/// sources, hot-reload availability, embedding policy, fallback behavior).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AssetProfile {
    /// `cargo run` from the workspace with the loose `assets/` tree on
    /// disk. Supports filesystem hot-reload.
    DesktopDevLoose,
    /// Installed/packaged desktop build. Asset tree lives next to the
    /// binary; no `CARGO_MANIFEST_DIR` assumptions, no hot-reload.
    DesktopInstalled,
    /// Steam Deck install layout. Same as `DesktopInstalled` for now
    /// (kept distinct so a future Deck-specific input policy or
    /// low-power asset profile doesn't have to fork `DesktopInstalled`).
    SteamDeckInstalled,
    /// Android APK / app bundle. Assets read through Bevy's Android
    /// AssetReader (or embedded fallback for the LDtk bootstrap, see
    /// `feature_static_map` in the sandbox).
    AndroidBundle,
    /// iOS app bundle. Same shape as `AndroidBundle` modulo the platform
    /// AssetReader. No iOS build runs yet; here so the manifest schema
    /// can be authored ahead of time.
    IosBundle,
    /// Browser build — assets fetched over HTTP through Bevy's
    /// `http` / `https` `AssetSource`. Hot-reload requires a server-side
    /// polling/ETag flow; not in the first slice.
    WebHttp,
    /// Browser build with critical assets embedded in the wasm module
    /// (LDtk bootstrap, default font) and optional assets fetched over
    /// HTTP. Today's wasm first-pass build sits here.
    WebStatic,
    /// Browser build that serves the regular `assets/` tree alongside
    /// the wasm/JS via the page origin. Bevy's wasm default asset
    /// reader is an `HttpAssetReader` that fetches `/assets/<path>`
    /// over HTTP — the resolver synthesizes plain `BevyPath` defaults
    /// from each entry's `logical_path` and Bevy handles the rest.
    /// Same authored `EmbeddedBinary` candidates still take priority
    /// (so LDtk JSON loads from `embedded://` while sprites/fonts/
    /// music load from `/assets/...`). This is the "same game in the
    /// browser" mode — no per-asset packaging of optional art.
    WebServedAssets,
    /// Single-binary bundle (cross-platform). Every authored asset is
    /// embedded; no filesystem reads. Useful for itch.io demo binaries.
    BundledStatic,
    /// Programmatic / RL / smoke driver. No display, no audio. Every
    /// non-required asset resolves to [`crate::location::AssetLocation::Disabled`].
    NoAssets,
    /// Same as `NoAssets` but slightly stricter — required assets are
    /// also allowed to be missing because the headless driver may not
    /// own the asset tree (e.g. CI test runner).
    Headless,
    /// IPFS gateway placeholder. Builds URLs via
    /// [`crate::location::ipfs_gateway_url`]. No native IPFS client.
    IpfsGatewayPlaceholder,
}

impl AssetProfile {
    /// Stable lower-snake-case label (for diagnostics / logs).
    pub fn label(self) -> &'static str {
        match self {
            Self::DesktopDevLoose => "desktop_dev_loose",
            Self::DesktopInstalled => "desktop_installed",
            Self::SteamDeckInstalled => "steam_deck_installed",
            Self::AndroidBundle => "android_bundle",
            Self::IosBundle => "ios_bundle",
            Self::WebHttp => "web_http",
            Self::WebStatic => "web_static",
            Self::WebServedAssets => "web_served_assets",
            Self::BundledStatic => "bundled_static",
            Self::NoAssets => "no_assets",
            Self::Headless => "headless",
            Self::IpfsGatewayPlaceholder => "ipfs_gateway_placeholder",
        }
    }

    /// Ordered list of source kinds this profile prefers, in
    /// resolution order. The resolver walks the manifest's locations
    /// and returns the first match.
    ///
    /// Authored locations with no matching source for the active
    /// profile are skipped; if nothing matches the resolver falls back
    /// to [`crate::location::AssetLocation::Disabled`] and the
    /// [`crate::policy::MissingAssetPolicy`] decides what to do.
    pub fn preferred_sources(self) -> &'static [AssetSourceProfile] {
        use AssetSourceProfile::*;
        match self {
            Self::DesktopDevLoose => &[LooseFilesystem, EmbeddedBinary, HttpRemote],
            Self::DesktopInstalled | Self::SteamDeckInstalled => {
                &[InstalledFilesystem, EmbeddedBinary, HttpRemote]
            }
            Self::AndroidBundle => &[AndroidApk, EmbeddedBinary],
            Self::IosBundle => &[IosBundle, EmbeddedBinary],
            Self::WebHttp => &[HttpRemote, EmbeddedBinary],
            Self::WebStatic => &[EmbeddedBinary, HttpRemote],
            // WebServedAssets prefers `InstalledFilesystem` so the
            // resolver's pass-2 synthesizer emits a plain `BevyPath`
            // (Bevy's wasm default asset reader fetches it from
            // `/assets/<path>` over HTTP). `EmbeddedBinary` is the
            // pass-1 escape valve: an entry that authors an
            // explicit `Embedded` candidate (e.g. the LDtk world
            // under `static_map`) still loads from
            // `EmbeddedAssetRegistry`, because pass 1 walks every
            // preferred source for an authored candidate before
            // pass 2's synthesizer runs. The relative order between
            // these two sources matters only when an entry has no
            // authored candidate — and in that case we want the
            // BevyPath synthesis.
            Self::WebServedAssets => &[InstalledFilesystem, EmbeddedBinary],
            Self::BundledStatic => &[EmbeddedBinary],
            Self::NoAssets | Self::Headless => &[],
            Self::IpfsGatewayPlaceholder => &[IpfsGateway, HttpRemote, EmbeddedBinary],
        }
    }

    /// Whether the profile can offer filesystem hot-reload. Local
    /// filesystem profiles can; bundled / web / no-assets cannot.
    pub fn supports_hot_reload(self) -> bool {
        matches!(self, Self::DesktopDevLoose)
    }

    /// Whether required assets are allowed to be missing under this
    /// profile. `Headless` is the only profile that returns true — CI
    /// runners often don't ship the full asset tree, and the test
    /// harness doesn't need them to.
    pub fn tolerates_missing_required(self) -> bool {
        matches!(self, Self::Headless)
    }
}

/// Kinds of underlying storage a profile can reach. The manifest
/// declares which kinds each asset is available from; the profile picks
/// an order over the kinds it understands.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AssetSourceProfile {
    /// Loose files on the dev workspace (`assets/` next to the crate).
    LooseFilesystem,
    /// App-root-relative `assets/` next to the installed binary.
    InstalledFilesystem,
    /// Embedded into the binary via `include_bytes!` or
    /// `bevy_embedded_assets`. No filesystem.
    EmbeddedBinary,
    /// Android `assets/` packaged inside the APK. Bevy's Android
    /// AssetReader is the consumer.
    AndroidApk,
    /// iOS app bundle resources.
    IosBundle,
    /// HTTP(S) URL fetched at runtime. Browser HTTP source today;
    /// future native `http_asset_source` for CDN-served desktop builds.
    HttpRemote,
    /// IPFS gateway HTTP URL (still HTTP at the wire; semantics differ).
    IpfsGateway,
}

impl AssetSourceProfile {
    pub fn label(self) -> &'static str {
        match self {
            Self::LooseFilesystem => "loose_filesystem",
            Self::InstalledFilesystem => "installed_filesystem",
            Self::EmbeddedBinary => "embedded_binary",
            Self::AndroidApk => "android_apk",
            Self::IosBundle => "ios_bundle",
            Self::HttpRemote => "http_remote",
            Self::IpfsGateway => "ipfs_gateway",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_are_distinct_per_profile() {
        let profiles = [
            AssetProfile::DesktopDevLoose,
            AssetProfile::DesktopInstalled,
            AssetProfile::SteamDeckInstalled,
            AssetProfile::AndroidBundle,
            AssetProfile::IosBundle,
            AssetProfile::WebHttp,
            AssetProfile::WebStatic,
            AssetProfile::WebServedAssets,
            AssetProfile::BundledStatic,
            AssetProfile::NoAssets,
            AssetProfile::Headless,
            AssetProfile::IpfsGatewayPlaceholder,
        ];
        let labels: std::collections::HashSet<_> = profiles.iter().map(|p| p.label()).collect();
        assert_eq!(labels.len(), profiles.len());
    }

    #[test]
    fn desktop_dev_loose_prefers_loose_then_embedded() {
        let order = AssetProfile::DesktopDevLoose.preferred_sources();
        assert_eq!(order[0], AssetSourceProfile::LooseFilesystem);
        assert!(order.contains(&AssetSourceProfile::EmbeddedBinary));
    }

    #[test]
    fn web_static_prefers_embedded_then_http() {
        let order = AssetProfile::WebStatic.preferred_sources();
        assert_eq!(order[0], AssetSourceProfile::EmbeddedBinary);
        assert_eq!(order[1], AssetSourceProfile::HttpRemote);
    }

    #[test]
    fn web_served_assets_prefers_installed_filesystem_then_embedded() {
        let order = AssetProfile::WebServedAssets.preferred_sources();
        // `InstalledFilesystem` first so pass-2 synthesizes a plain
        // `BevyPath` (Bevy's wasm HTTP reader fetches it).
        // Authored `EmbeddedBinary` candidates still win in pass 1.
        assert_eq!(order[0], AssetSourceProfile::InstalledFilesystem);
        assert_eq!(order[1], AssetSourceProfile::EmbeddedBinary);
    }

    #[test]
    fn no_assets_has_no_sources() {
        assert!(AssetProfile::NoAssets.preferred_sources().is_empty());
        assert!(AssetProfile::Headless.preferred_sources().is_empty());
    }

    #[test]
    fn hot_reload_only_for_dev_loose() {
        assert!(AssetProfile::DesktopDevLoose.supports_hot_reload());
        assert!(!AssetProfile::DesktopInstalled.supports_hot_reload());
        assert!(!AssetProfile::WebStatic.supports_hot_reload());
    }

    #[test]
    fn headless_tolerates_missing_required_assets() {
        assert!(AssetProfile::Headless.tolerates_missing_required());
        assert!(!AssetProfile::NoAssets.tolerates_missing_required());
        assert!(!AssetProfile::DesktopInstalled.tolerates_missing_required());
    }
}
