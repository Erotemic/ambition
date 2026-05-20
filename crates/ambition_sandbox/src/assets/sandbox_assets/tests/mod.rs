//! Tests for the sandbox asset catalog builder + source plugin, split
//! by concern. Helpers (`fixture_catalog`, `SFX_BANK_ENV_LOCK`) live in
//! this module so each submodule can reach them via `super::`.
//!
//! - [`identity`] — catalog id presence / kind / uniqueness sanity.
//! - [`profiles`] — per-`AssetProfile` resolution + load gate.
//! - [`static_probes`] — lint-style filesystem-probe / legacy-helper
//!   guardrails (grep the source tree for forbidden patterns).
//! - [`embedded_core`] — `embedded_core::*` URL constants + the
//!   `AmbitionAssetSourcePlugin` registration tests.

use super::*;
use crate::content::data::SandboxDataSpec;
use std::collections::HashSet;
use std::sync::Mutex;

mod embedded_core;
mod identity;
mod profiles;
mod static_probes;

/// Shared lock for tests that mutate `AMBITION_SFX_BANK_PATH`.
/// Env-var mutations are process-global; rust tests run in
/// parallel by default, so the lock keeps the
/// `sfx_bank_env_override_is_authored_local_path_candidate` setter
/// from racing the `sfx_bank_resolves_under_desktop_dev_loose`
/// reader.
static SFX_BANK_ENV_LOCK: Mutex<()> = Mutex::new(());

fn fixture_catalog() -> SandboxAssetCatalog {
    let config = GameAssetConfig::default();
    let spec = SandboxDataSpec::load_embedded();
    build_sandbox_catalog(&config, &spec.audio)
}
