//! Ambition asset catalog + source/profile policy.
//!
//! This crate owns **Ambition's logical asset layer**: stable [`AssetId`]s,
//! a [`manifest`] of [`AssetEntry`] records, [`profile::AssetProfile`]
//! personas, [`policy::MissingAssetPolicy`] / [`policy::CachePolicy`]
//! rules, [`preload::PreloadGroup`] tagging, and a [`resolver`] that
//! maps `(id, profile)` to a concrete [`location::AssetLocation`].
//!
//! It deliberately does NOT re-implement async loading, handles,
//! dependencies, or hot reload — Bevy's `AssetServer` /
//! `AssetReader` / `AssetPath` already cover those. The optional
//! [`bevy_integration`] module exposes a thin Bevy plugin/resource pair
//! that turns resolved locations into `AssetServer::load` calls.
//!
//! For non-Bevy byte consumers (the SFX bank, manifest preload, raw
//! shader bytes) [`sfx_integration`] and friends provide small synchronous
//! adapters that consult the resolver and return bytes ready for the
//! consuming subsystem.
//!
//! # End-state architecture
//!
//! See [`docs/asset_manager.md`](https://github.com/anthropics/claude-code/tree/main/docs)
//! (the in-repo design doc) for the full per-profile contract — what
//! sources each profile understands, hot-reload availability, and the
//! recommended Bevy `AssetSource` wiring per platform.

pub mod id;
pub mod kind;
pub mod location;
pub mod manifest;
pub mod policy;
pub mod preload;
pub mod profile;
pub mod resolver;

#[cfg(feature = "bevy")]
pub mod bevy_integration;

#[cfg(feature = "sfx")]
pub mod sfx_integration;

pub use id::AssetId;
pub use kind::AssetKind;
pub use location::{ipfs_gateway_url, AssetLocation};
pub use manifest::{AssetEntry, AssetManifest, AssetManifestBuilder, LocationCandidate};
pub use policy::{CachePolicy, MissingAssetPolicy};
pub use preload::PreloadGroup;
pub use profile::{AssetProfile, AssetSourceProfile};
pub use resolver::{resolve, resolve_all, AssetResolutionError, ResolvedAsset};

#[cfg(feature = "bevy")]
pub use bevy_integration::{
    path_for_active, AmbitionAssetCatalog, AmbitionAssetManagerPlugin, AmbitionAssetProfile,
};

#[cfg(feature = "sfx")]
pub use sfx_integration::{build_provider_from_path, build_provider_from_resolved, SfxBankResolveError};
