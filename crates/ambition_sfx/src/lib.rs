//! SFX runtime contract for Ambition.
//!
//! - [`SfxId`]: hashed string-id newtype. Construct via `SfxId::from_static`
//!   for compile-time hashing of literal ids; `SfxId::new` for runtime
//!   strings (e.g. read from data files).
//! - [`SfxClip`]: encoded-bytes clip with metadata; the unit a provider hands
//!   back. Caller owns decode (kira's `StaticSoundData::from_cursor` handles
//!   both WAV and OGG bytes uniformly).
//! - [`SfxProvider`]: trait for fetching clips. Impls in this crate:
//!   [`BankProvider`], [`FilesystemProvider`], [`SilentProvider`],
//!   [`LayeredProvider`].
//! - [`ids`]: hand-maintained const ids for the cues gameplay code references
//!   often. Add entries there when the IDE help is worth a const; otherwise
//!   `SfxId::from_static("foo.bar")` at the call site is fine.
//!
//! The default `bevy` feature supplies the request/message adapter. Consumers
//! that only need ids, clips, and providers may disable default features to keep
//! this crate independent of Bevy for headless, RL, and benchmarking contexts.

pub use ambition_sfx_bank::{fnv1a_64, fnv1a_64_str, Codec};
use ambition_sfx_bank::{EntryRecord, SfxBank};

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub mod ids;

#[cfg(feature = "bevy")]
mod message;
#[cfg(feature = "bevy")]
pub use message::{AudioContextOwner, OwnedSfxMessage, SfxEmissionContext, SfxMessage, SfxWriter};

/// Stable, hashed identifier for an SFX entry. Construct via
/// [`SfxId::from_static`] for compile-time hashing of literal ids
/// (zero runtime cost) or [`SfxId::new`] for ids that come from data
/// files at runtime.
///
/// The hash is FNV-1a 64 of the UTF-8 bytes of the id string.
/// Cross-language compatible with the Python packer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SfxId(u64);

impl SfxId {
    /// Construct from a `'static` string literal. The hash is computed
    /// at compile time when called from `const` context.
    pub const fn from_static(s: &'static str) -> Self {
        Self(fnv1a_64_str(s))
    }

    /// Construct from a runtime string (e.g. from RON/JSON config).
    pub fn new(s: &str) -> Self {
        Self(fnv1a_64_str(s))
    }

    /// Construct directly from a precomputed hash. Mostly useful for
    /// tests and for ids loaded from a bank's name section.
    pub const fn from_hash(hash: u64) -> Self {
        Self(hash)
    }

    pub const fn hash(self) -> u64 {
        self.0
    }
}

impl std::fmt::Display for SfxId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SfxId(0x{:016x})", self.0)
    }
}

/// A single SFX clip, ready to be handed to an audio backend (kira, rodio, ...).
///
/// `bytes` are the *encoded* container payload (WAV or OGG); decoders
/// downstream sniff the container. Keeping it encoded means the bank
/// reader doesn't need an audio decoder, and tests/headless code can
/// inspect or count without dragging audio deps in.
#[derive(Clone, Debug)]
pub struct SfxClip {
    /// Encoded bytes, owned via `Arc` so cloning is cheap and the same
    /// underlying buffer can back many concurrent plays.
    pub bytes: Arc<[u8]>,
    pub codec: Codec,
    pub channels: u8,
    pub sample_rate: u32,
    pub duration_ms: u32,
    pub default_gain_db: f32,
    pub peak_db: f32,
    pub rms_db: f32,
    pub flags: u32,
}

impl SfxClip {
    pub fn streamable_hint(&self) -> bool {
        self.flags & ambition_sfx_bank::flag::STREAMABLE_HINT != 0
    }

    pub fn looping(&self) -> bool {
        self.flags & ambition_sfx_bank::flag::LOOPING != 0
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SfxError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("bank error: {0}")]
    Bank(#[from] ambition_sfx_bank::BankError),
}

/// Source of [`SfxClip`]s. All providers in this crate are `Send + Sync`
/// so they can live behind a `Resource` or `Arc`.
pub trait SfxProvider: Send + Sync {
    fn provide_clip(&self, id: SfxId) -> Option<SfxClip>;

    /// Whether this provider has the given id. Defaults to attempting
    /// `provide_clip`. Backends with cheaper presence checks (e.g.
    /// the bank's HashMap lookup) should override.
    fn has(&self, id: SfxId) -> bool {
        self.provide_clip(id).is_some()
    }
}

// =====================================================================
// BankProvider - backed by an `ambition_sfx_bank::SfxBank`.
// =====================================================================

pub struct BankProvider {
    bank: SfxBank,
}

impl BankProvider {
    pub fn from_path(path: &Path) -> Result<Self, SfxError> {
        let bank = SfxBank::from_path(path)?;
        Ok(Self { bank })
    }

    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self, SfxError> {
        let bank = SfxBank::from_bytes(bytes)?;
        Ok(Self { bank })
    }

    pub fn entry_count(&self) -> usize {
        self.bank.entry_count()
    }

    pub fn name_for(&self, id: SfxId) -> Option<&str> {
        self.bank.name_for(id.hash())
    }

    /// Iterate (id_hash, name) pairs for everything the bank knows about.
    /// Useful for `validate_known_ids` reverse maps and for diagnostics.
    pub fn iter_ids(&self) -> impl Iterator<Item = (SfxId, Option<&str>)> {
        self.bank
            .iter()
            .map(|entry| (SfxId::from_hash(entry.record.id_hash), entry.name))
    }

    /// A content fingerprint per bank id: an FNV-1a-64 hash of the encoded
    /// payload bytes. The audio layer records these fingerprints beside the
    /// provider-qualified id for cache invalidation, diagnostics, and stable
    /// source-identity tests.
    pub fn content_fingerprints(&self) -> std::collections::BTreeMap<SfxId, u64> {
        self.bank
            .iter()
            .map(|entry| {
                (
                    SfxId::from_hash(entry.record.id_hash),
                    fnv1a_64(entry.payload),
                )
            })
            .collect()
    }
}

impl SfxProvider for BankProvider {
    fn provide_clip(&self, id: SfxId) -> Option<SfxClip> {
        let entry = self.bank.lookup(id.hash())?;
        Some(clip_from_entry(entry.record, entry.payload))
    }

    fn has(&self, id: SfxId) -> bool {
        self.bank.contains(id.hash())
    }
}

fn clip_from_entry(record: &EntryRecord, payload: &[u8]) -> SfxClip {
    SfxClip {
        bytes: Arc::from(payload.to_vec().into_boxed_slice()),
        codec: record.codec,
        channels: record.channels,
        sample_rate: record.sample_rate,
        duration_ms: record.duration_ms,
        default_gain_db: record.default_gain_db,
        peak_db: record.peak_db,
        rms_db: record.rms_db,
        flags: record.flags,
    }
}

// =====================================================================
// FilesystemProvider - reads loose files from the renderer output dir.
// Useful for dev (skip the pack step) and as a fallback. Maintains an
// in-memory id_hash -> file path map populated at construction time.
// =====================================================================

pub struct FilesystemProvider {
    by_hash: HashMap<u64, FilesystemEntry>,
}

struct FilesystemEntry {
    path: PathBuf,
    codec: Codec,
}

impl FilesystemProvider {
    /// Walk `output/<id>/<id>.{wav,ogg}` (the layout produced by the
    /// SFX renderer). Each subdirectory whose name contains a matching
    /// audio file becomes an entry; the id is the directory name.
    pub fn from_renderer_output(root: &Path) -> Result<Self, SfxError> {
        let mut by_hash = HashMap::new();
        if !root.is_dir() {
            return Ok(Self { by_hash });
        }
        for child in fs::read_dir(root)? {
            let child = child?;
            if !child.file_type()?.is_dir() {
                continue;
            }
            let name = match child.file_name().into_string() {
                Ok(n) => n,
                Err(_) => continue,
            };
            for (ext, codec) in [
                ("wav", Codec::Wav),
                ("ogg", Codec::Ogg),
                ("flac", Codec::Flac),
            ] {
                let candidate = child.path().join(format!("{name}.{ext}"));
                if candidate.exists() {
                    by_hash.insert(
                        fnv1a_64_str(&name),
                        FilesystemEntry {
                            path: candidate,
                            codec,
                        },
                    );
                    break;
                }
            }
        }
        Ok(Self { by_hash })
    }

    pub fn entry_count(&self) -> usize {
        self.by_hash.len()
    }
}

impl SfxProvider for FilesystemProvider {
    fn provide_clip(&self, id: SfxId) -> Option<SfxClip> {
        let entry = self.by_hash.get(&id.hash())?;
        let bytes = fs::read(&entry.path).ok()?;
        Some(SfxClip {
            bytes: Arc::from(bytes.into_boxed_slice()),
            codec: entry.codec,
            channels: 2,
            sample_rate: 48_000,
            duration_ms: 0,
            default_gain_db: 0.0,
            peak_db: 0.0,
            rms_db: 0.0,
            flags: 0,
        })
    }

    fn has(&self, id: SfxId) -> bool {
        self.by_hash.contains_key(&id.hash())
    }
}

// =====================================================================
// SilentProvider - never has anything. Sentinel for headless / CI.
// =====================================================================

#[derive(Default)]
pub struct SilentProvider;

impl SfxProvider for SilentProvider {
    fn provide_clip(&self, _id: SfxId) -> Option<SfxClip> {
        None
    }

    fn has(&self, _id: SfxId) -> bool {
        false
    }
}

// =====================================================================
// LayeredProvider - try children in order; first hit wins.
// =====================================================================

pub struct LayeredProvider {
    layers: Vec<Box<dyn SfxProvider>>,
}

impl LayeredProvider {
    pub fn new(layers: Vec<Box<dyn SfxProvider>>) -> Self {
        Self { layers }
    }

    pub fn push(&mut self, provider: Box<dyn SfxProvider>) {
        self.layers.push(provider);
    }
}

impl SfxProvider for LayeredProvider {
    fn provide_clip(&self, id: SfxId) -> Option<SfxClip> {
        for layer in &self.layers {
            if let Some(clip) = layer.provide_clip(id) {
                return Some(clip);
            }
        }
        None
    }

    fn has(&self, id: SfxId) -> bool {
        self.layers.iter().any(|layer| layer.has(id))
    }
}

// =====================================================================
// Validation: assert every id in a known list resolves in a provider.
// Cheap startup safety net; panics in dev / warns in release callers'
// preference.
// =====================================================================

/// Returns the subset of `ids` that the provider does NOT have. Empty
/// vec means everything resolved. Caller decides whether to panic, log,
/// or ignore.
pub fn missing_ids<P: SfxProvider + ?Sized>(provider: &P, ids: &[SfxId]) -> Vec<SfxId> {
    ids.iter()
        .copied()
        .filter(|id| !provider.has(*id))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_hash_is_compile_time() {
        const ID: SfxId = SfxId::from_static("player.jump");
        assert_eq!(ID, SfxId::new("player.jump"));
        assert_ne!(ID, SfxId::new("player.jumb"));
    }

    #[test]
    fn silent_provider_never_has_anything() {
        let p = SilentProvider;
        assert!(!p.has(ids::PLAYER_JUMP));
        assert!(p.provide_clip(ids::PLAYER_JUMP).is_none());
    }

    #[test]
    fn layered_provider_first_hit_wins() {
        struct Fake(SfxId);
        impl SfxProvider for Fake {
            fn provide_clip(&self, id: SfxId) -> Option<SfxClip> {
                if id == self.0 {
                    Some(SfxClip {
                        bytes: Arc::from(Vec::<u8>::new().into_boxed_slice()),
                        codec: Codec::Wav,
                        channels: 1,
                        sample_rate: 1,
                        duration_ms: 1,
                        default_gain_db: 0.0,
                        peak_db: 0.0,
                        rms_db: 0.0,
                        flags: 0,
                    })
                } else {
                    None
                }
            }
            fn has(&self, id: SfxId) -> bool {
                id == self.0
            }
        }

        let layered = LayeredProvider::new(vec![
            Box::new(Fake(ids::PLAYER_JUMP)),
            Box::new(Fake(ids::PLAYER_DASH)),
        ]);
        assert!(layered.has(ids::PLAYER_JUMP));
        assert!(layered.has(ids::PLAYER_DASH));
        assert!(!layered.has(ids::PLAYER_HIT));
    }

    /// Validates that the const ids in `ids::*` all resolve against the
    /// real packed bank shipped with the sandbox. Skipped if the bank
    /// file isn't present (e.g. cold checkout before `pack.py` ran).
    #[test]
    fn const_ids_resolve_in_real_bank() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("ambition_actors")
            .join("assets")
            .join("audio")
            .join("sfx.bank");
        if !path.exists() {
            eprintln!("(skipped) {} missing - run pack.py", path.display());
            return;
        }
        let provider = BankProvider::from_path(&path).expect("load bank");
        let known: &[SfxId] = &[
            ids::PLAYER_JUMP,
            ids::PLAYER_DOUBLE_JUMP,
            ids::PLAYER_DASH,
            ids::PLAYER_BLINK,
            ids::PLAYER_PRECISION_BLINK,
            ids::PLAYER_POGO,
            ids::PLAYER_LAND,
            ids::PLAYER_SLASH,
            ids::PLAYER_HIT,
            ids::PLAYER_DEATH,
            ids::PLAYER_RESPAWN,
            ids::PLAYER_RESET,
            ids::UI_MENU_MOVE,
            ids::UI_MENU_ACCEPT,
            ids::UI_MENU_BACK,
            ids::PLAYER_FOOTSTEP_STONE_01,
            ids::PLAYER_FOOTSTEP_STONE_02,
        ];
        let missing = missing_ids(&provider, known);
        assert!(
            missing.is_empty(),
            "ids::* references {} sfx not in bank: {:?}",
            missing.len(),
            missing,
        );
    }
}
