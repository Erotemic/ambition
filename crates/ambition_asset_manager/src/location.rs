//! [`AssetLocation`] — where the bytes for a logical [`crate::AssetId`]
//! live for a given [`crate::profile::AssetProfile`].
//!
//! The resolver picks one variant per `(id, profile)` pair. Bevy
//! consumers only ever see the string returned by [`AssetLocation::bevy_asset_path`];
//! non-Bevy byte adapters (SFX bank, manifest preload) consult the
//! variant directly via [`AssetLocation::as_local_path`] / [`AssetLocation::http_url`].
//!
//! ## Location vs Source
//!
//! [`AssetLocation`] is the resolved per-asset endpoint.
//! [`crate::profile::AssetSourceProfile`] is the *policy* a profile uses
//! to pick a location — they live in different layers so a profile can
//! preview each variant without committing to one.
//!
//! ## Bevy `AssetPath` mapping
//!
//! Bevy's `AssetPath` accepts source-qualified strings like
//! `embedded://sprites/foo.png` or `remote://world/sandbox.ldtk` (see
//! Bevy docs on [`AssetPath`](https://docs.rs/bevy/0.18/bevy/asset/struct.AssetPath.html)).
//! `AssetLocation::bevy_asset_path` formats those strings; the actual
//! Bevy `AssetSource` registrations (e.g. `embedded`, `http`, `https`)
//! must be wired by the consuming app via Bevy's
//! `AssetPlugin::source` / `AssetServer::register_source` APIs.
//! See `docs/systems/asset-manager.md`.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Resolved location for one asset under one profile.
///
/// `Disabled` is the "the profile intentionally doesn't ship this asset"
/// sentinel; callers honor it via [`crate::policy::MissingAssetPolicy`].
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AssetLocation {
    /// Default Bevy `AssetPath` — relative to the registered default
    /// `AssetSource` root (typically `assets/`). String form: `"sprites/foo.png"`.
    BevyPath(String),

    /// Source-qualified Bevy `AssetPath`. The `source` segment must be
    /// registered with Bevy (`embedded`, `http`, `remote`, etc.). The
    /// `path` segment is relative within that source. String form:
    /// `"embedded://sprites/foo.png"`.
    BevySourcePath { source: String, path: String },

    /// Bevy `embedded://` shortcut — equivalent to
    /// `BevySourcePath { source: "embedded", path }`. Kept distinct so
    /// the catalog can advertise "this lives in the binary" without
    /// callers having to spell the source name.
    Embedded(String),

    /// Host-OS path. Always absolute or relative-to-CWD as authored;
    /// the resolver does not normalize. Used by non-Bevy adapters
    /// (e.g. `load_sfx_bank_bytes`) and as a hint for sources that
    /// expose filesystem hot-reload.
    LocalPath(PathBuf),

    /// Plain HTTP(S) URL — for the `WebHttp` profile or any
    /// `BevySourcePath` of `http://` / `https://`. Bevy's `http` /
    /// `https` `AssetSource` features wrap these for runtime loads.
    HttpUrl(String),

    /// IPFS gateway URL builder — `gateway` is the HTTP base
    /// (e.g. `https://w3s.link`), `cid` is the content id, `path` the
    /// trailing path inside the CID directory. Renders as
    /// `https://<gateway>/ipfs/<cid>/<path>`. First-pass placeholder;
    /// no native IPFS client is pulled in.
    IpfsGateway {
        gateway: String,
        cid: String,
        path: String,
    },

    /// The asset is intentionally unavailable under this profile.
    /// Callers consult [`crate::policy::MissingAssetPolicy`] to decide
    /// between Error / Warn / Silent. Useful for headless / `--no-assets`.
    Disabled,
}

impl AssetLocation {
    /// Convenience constructor for `BevyPath`.
    pub fn bevy(path: impl Into<String>) -> Self {
        Self::BevyPath(path.into())
    }

    /// Convenience constructor for `Embedded`.
    pub fn embedded(path: impl Into<String>) -> Self {
        Self::Embedded(path.into())
    }

    /// Render the Bevy `AssetPath` string for this location, or `None`
    /// if the location is not addressable through Bevy's `AssetServer`
    /// (LocalPath, IpfsGateway, Disabled).
    ///
    /// Returned form:
    ///
    /// - `BevyPath("sprites/foo.png")`      → `"sprites/foo.png"`
    /// - `BevySourcePath { src, path }`     → `"src://path"`
    /// - `Embedded("sprites/foo.png")`      → `"embedded://sprites/foo.png"`
    /// - `HttpUrl("https://.../foo.png")`   → the URL verbatim (caller
    ///   must have an `http` / `https` `AssetSource` registered)
    pub fn bevy_asset_path(&self) -> Option<String> {
        match self {
            Self::BevyPath(path) => Some(path.clone()),
            Self::BevySourcePath { source, path } => Some(format!("{source}://{path}")),
            Self::Embedded(path) => Some(format!("embedded://{path}")),
            Self::HttpUrl(url) => Some(url.clone()),
            Self::LocalPath(_) | Self::IpfsGateway { .. } | Self::Disabled => None,
        }
    }

    /// Host-OS path, if any. Used by:
    /// - filesystem hot-reload to know which path to `inotify` on.
    /// - SFX bank byte adapter to `std::fs::read` the bank.
    pub fn as_local_path(&self) -> Option<&std::path::Path> {
        match self {
            Self::LocalPath(p) => Some(p.as_path()),
            _ => None,
        }
    }

    /// HTTP(S) URL form, including the IPFS gateway expansion. None for
    /// purely local / disabled locations.
    pub fn http_url(&self) -> Option<String> {
        match self {
            Self::HttpUrl(url) => Some(url.clone()),
            Self::BevySourcePath { source, path } if source == "http" || source == "https" => {
                Some(format!("{source}://{path}"))
            }
            Self::IpfsGateway { gateway, cid, path } => Some(ipfs_gateway_url(gateway, cid, path)),
            _ => None,
        }
    }

    /// Whether the underlying source can fire file-change notifications
    /// today. Local filesystem variants can; embedded / HTTP / IPFS
    /// cannot. This is the seam LDtk hot-reload checks before arming
    /// the file watcher.
    pub fn supports_hot_reload(&self) -> bool {
        matches!(self, Self::LocalPath(_) | Self::BevyPath(_))
    }

    /// Whether the location is "absent" — the profile chose not to ship
    /// this asset. See [`crate::policy::MissingAssetPolicy`].
    pub fn is_disabled(&self) -> bool {
        matches!(self, Self::Disabled)
    }
}

/// Build the gateway URL for an IPFS `(cid, path)` pair. Used both by
/// `AssetLocation::http_url` and standalone in tests.
///
/// The gateway is normalized to drop a single trailing slash so we don't
/// emit `https://gw//ipfs/...`. Other slashes inside the path are kept.
pub fn ipfs_gateway_url(gateway: &str, cid: &str, path: &str) -> String {
    let gw = gateway.trim_end_matches('/');
    let path = path.trim_start_matches('/');
    if path.is_empty() {
        format!("{gw}/ipfs/{cid}")
    } else {
        format!("{gw}/ipfs/{cid}/{path}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bevy_path_round_trips() {
        let loc = AssetLocation::bevy("sprites/foo.png");
        assert_eq!(loc.bevy_asset_path().as_deref(), Some("sprites/foo.png"));
        assert!(loc.http_url().is_none());
        assert!(loc.supports_hot_reload());
        assert!(!loc.is_disabled());
    }

    #[test]
    fn source_qualified_bevy_path_formats_with_scheme() {
        let loc = AssetLocation::BevySourcePath {
            source: "remote".into(),
            path: "world/sandbox.ldtk".into(),
        };
        assert_eq!(
            loc.bevy_asset_path().as_deref(),
            Some("remote://world/sandbox.ldtk"),
        );
    }

    #[test]
    fn embedded_emits_embedded_scheme() {
        let loc = AssetLocation::embedded("sprites/chest_closed.png");
        assert_eq!(
            loc.bevy_asset_path().as_deref(),
            Some("embedded://sprites/chest_closed.png"),
        );
        // embedded assets can't watch the filesystem.
        assert!(!loc.supports_hot_reload());
    }

    #[test]
    fn http_url_passes_through_and_disables_hot_reload() {
        let loc = AssetLocation::HttpUrl("https://cdn.example.com/foo.png".into());
        assert_eq!(
            loc.bevy_asset_path().as_deref(),
            Some("https://cdn.example.com/foo.png"),
        );
        assert_eq!(
            loc.http_url().as_deref(),
            Some("https://cdn.example.com/foo.png"),
        );
        assert!(!loc.supports_hot_reload());
    }

    #[test]
    fn ipfs_gateway_renders_canonical_url() {
        let loc = AssetLocation::IpfsGateway {
            gateway: "https://w3s.link/".into(),
            cid: "bafybei".into(),
            path: "sprites/foo.png".into(),
        };
        assert_eq!(
            loc.http_url().as_deref(),
            Some("https://w3s.link/ipfs/bafybei/sprites/foo.png"),
        );
        // Not reachable through default Bevy AssetServer without a
        // registered IPFS source.
        assert!(loc.bevy_asset_path().is_none());
    }

    #[test]
    fn ipfs_gateway_with_empty_path_drops_trailing_slash() {
        assert_eq!(
            ipfs_gateway_url("https://gw/", "bafy", ""),
            "https://gw/ipfs/bafy",
        );
    }

    #[test]
    fn disabled_is_disabled() {
        let loc = AssetLocation::Disabled;
        assert!(loc.is_disabled());
        assert!(loc.bevy_asset_path().is_none());
        assert!(loc.http_url().is_none());
        assert!(!loc.supports_hot_reload());
    }

    #[test]
    fn local_path_supports_hot_reload_but_has_no_bevy_path() {
        let loc = AssetLocation::LocalPath(PathBuf::from("/tmp/sandbox.ldtk"));
        assert!(loc.supports_hot_reload());
        assert!(loc.bevy_asset_path().is_none());
        assert_eq!(
            loc.as_local_path().map(|p| p.to_string_lossy().to_string()),
            Some("/tmp/sandbox.ldtk".to_string()),
        );
    }

    #[test]
    fn bevy_source_path_http_round_trips_through_http_url() {
        let loc = AssetLocation::BevySourcePath {
            source: "https".into(),
            path: "cdn.example.com/foo.png".into(),
        };
        assert_eq!(
            loc.http_url().as_deref(),
            Some("https://cdn.example.com/foo.png"),
        );
    }
}
