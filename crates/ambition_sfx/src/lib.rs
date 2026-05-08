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
//!   often. Add entries here when the IDE help is worth a const; otherwise
//!   `SfxId::from_static("foo.bar")` at the call site is fine.
//!
//! No Bevy dependency; this crate stays usable from headless / RL /
//! benchmarking contexts.

pub use ambition_sfx_bank::{Codec, fnv1a_64, fnv1a_64_str};
use ambition_sfx_bank::{EntryRecord, SfxBank};

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

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

/// A single SFX clip, ready to be handed to an audio backend (kira, rodio, …).
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
// BankProvider — backed by an `ambition_sfx_bank::SfxBank`.
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
// FilesystemProvider — reads loose files from the renderer output dir.
// Useful for dev (skip the pack step) and as a fallback. Maintains an
// in-memory id_hash → file path map populated at construction time.
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
            for (ext, codec) in [("wav", Codec::Wav), ("ogg", Codec::Ogg), ("flac", Codec::Flac)] {
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
// SilentProvider — never has anything. Sentinel for headless / CI.
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
// LayeredProvider — try children in order; first hit wins.
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
    ids.iter().copied().filter(|id| !provider.has(*id)).collect()
}

// =====================================================================
// `ids` module: hand-maintained const list for the SFX gameplay code
// references often. Adding to this list is purely an ergonomics call —
// the bank stores everything regardless. Use literals at the call site
// for one-off / rare SFX.
//
// IDs match the catalog produced by `ambition_sfx_renderer` (see
// `tools/ambition_sfx_renderer/output/`). When in doubt, run:
//   python3 tools/ambition_sfx_pack/pack.py --dump
// and grep `crates/ambition_sandbox/assets/audio/sfx.bank.txt`.
// =====================================================================

pub mod ids {
    use super::SfxId;

    // Player movement
    pub const PLAYER_JUMP: SfxId = SfxId::from_static("player.jump");
    pub const PLAYER_DOUBLE_JUMP: SfxId = SfxId::from_static("player.double_jump");
    pub const PLAYER_DASH: SfxId = SfxId::from_static("player.dash");
    pub const PLAYER_BLINK: SfxId = SfxId::from_static("player.blink");
    pub const PLAYER_PRECISION_BLINK: SfxId = SfxId::from_static("player.precision_blink");
    pub const PLAYER_POGO: SfxId = SfxId::from_static("player.pogo");
    pub const PLAYER_LAND: SfxId = SfxId::from_static("player.land");
    pub const PLAYER_FAST_FALL: SfxId = SfxId::from_static("player.fast_fall");
    pub const PLAYER_WALL_JUMP: SfxId = SfxId::from_static("player.wall_jump");
    pub const PLAYER_WALL_SLIDE: SfxId = SfxId::from_static("player.wall_slide");
    pub const PLAYER_WALL_CLING: SfxId = SfxId::from_static("player.wall_cling");
    pub const PLAYER_LEDGE_GRAB: SfxId = SfxId::from_static("player.ledge_grab");
    pub const PLAYER_REBOUND: SfxId = SfxId::from_static("player.rebound");

    // Player combat / vitals
    pub const PLAYER_SLASH: SfxId = SfxId::from_static("player.slash");
    pub const PLAYER_HIT: SfxId = SfxId::from_static("player.hit");
    pub const PLAYER_DAMAGE: SfxId = SfxId::from_static("player.damage");
    pub const PLAYER_HEAL: SfxId = SfxId::from_static("player.heal");
    pub const PLAYER_DEATH: SfxId = SfxId::from_static("player.death");
    pub const PLAYER_RESPAWN: SfxId = SfxId::from_static("player.respawn");
    pub const PLAYER_RESET: SfxId = SfxId::from_static("player.reset");
    pub const PLAYER_LOW_HEALTH_PULSE: SfxId = SfxId::from_static("player.low_health.pulse");
    pub const PLAYER_STAMINA_EMPTY: SfxId = SfxId::from_static("player.stamina_empty");
    pub const PLAYER_ABILITY_UNLOCK: SfxId = SfxId::from_static("player.ability_unlock");

    // Player damage-type variants (when source is typed)
    pub const PLAYER_HIT_FIRE: SfxId = SfxId::from_static("player.hit.fire");
    pub const PLAYER_HIT_ICE: SfxId = SfxId::from_static("player.hit.ice");
    pub const PLAYER_HIT_LIGHTNING: SfxId = SfxId::from_static("player.hit.lightning");
    pub const PLAYER_HIT_POISON: SfxId = SfxId::from_static("player.hit.poison");

    // Hazards (single-shot contacts)
    pub const HAZARD_LAVA_SPLASH: SfxId = SfxId::from_static("hazard.lava.splash");
    pub const HAZARD_ACID_SPLASH: SfxId = SfxId::from_static("hazard.acid.splash");
    pub const HAZARD_SPIKE_HIT: SfxId = SfxId::from_static("hazard.spike.hit");
    pub const HAZARD_ELECTRIC_ARC: SfxId = SfxId::from_static("hazard.electric.arc");
    pub const HAZARD_SAW_HIT: SfxId = SfxId::from_static("hazard.saw.hit");
    // Looped hazard ambients (start/stop on volume entry/exit) —
    // wiring lives in TODO until the loop-lifecycle subsystem lands.
    pub const HAZARD_WIND_GUST_LOOP: SfxId = SfxId::from_static("hazard.wind.gust_loop");
    pub const HAZARD_POISON_CLOUD_LOOP: SfxId =
        SfxId::from_static("hazard.poison.cloud_loop");
    pub const HAZARD_ELECTRIC_LOOP: SfxId = SfxId::from_static("hazard.electric.loop");
    pub const HAZARD_SAW_LOOP: SfxId = SfxId::from_static("hazard.saw.loop");

    // UI
    pub const UI_MENU_MOVE: SfxId = SfxId::from_static("ui.menu.move");
    pub const UI_MENU_ACCEPT: SfxId = SfxId::from_static("ui.menu.accept");
    pub const UI_MENU_BACK: SfxId = SfxId::from_static("ui.menu.back");
    pub const UI_TAB_CHANGE: SfxId = SfxId::from_static("ui.tab.change");
    pub const UI_PAUSE_OPEN: SfxId = SfxId::from_static("ui.pause.open");
    pub const UI_PAUSE_CLOSE: SfxId = SfxId::from_static("ui.pause.close");
    pub const UI_SAVE_COMPLETE: SfxId = SfxId::from_static("ui.save.complete");
    pub const UI_ERROR: SfxId = SfxId::from_static("ui.error");

    // Footsteps (variants are sibling ids; gameplay picks among them)
    pub const PLAYER_FOOTSTEP_STONE_01: SfxId = SfxId::from_static("player.footstep.stone.01");
    pub const PLAYER_FOOTSTEP_STONE_02: SfxId = SfxId::from_static("player.footstep.stone.02");
    pub const PLAYER_FOOTSTEP_METAL_01: SfxId = SfxId::from_static("player.footstep.metal.01");
    pub const PLAYER_FOOTSTEP_METAL_02: SfxId = SfxId::from_static("player.footstep.metal.02");
    pub const PLAYER_FOOTSTEP_SOFT_01: SfxId = SfxId::from_static("player.footstep.soft.01");
    pub const PLAYER_FOOTSTEP_SOFT_02: SfxId = SfxId::from_static("player.footstep.soft.02");

    // World interactions
    pub const WORLD_TREASURE_CHEST_OPEN: SfxId =
        SfxId::from_static("world.treasure_chest.open");
    pub const WORLD_DOOR_OPEN: SfxId = SfxId::from_static("world.door.open");
    pub const WORLD_DOOR_CLOSE: SfxId = SfxId::from_static("world.door.close");
    pub const WORLD_DOOR_HEAVY_OPEN: SfxId = SfxId::from_static("world.door.heavy_open");
    pub const WORLD_DOOR_HEAVY_CLOSE: SfxId = SfxId::from_static("world.door.heavy_close");
    pub const WORLD_DOOR_LOCKED_RATTLE: SfxId =
        SfxId::from_static("world.door.locked.rattle");
    pub const WORLD_GATE_RISE: SfxId = SfxId::from_static("world.gate.rise");
    pub const WORLD_GATE_FALL: SfxId = SfxId::from_static("world.gate.fall");
    pub const WORLD_LEVER_ENGAGE: SfxId = SfxId::from_static("world.lever.engage");
    pub const WORLD_LEVER_DISENGAGE: SfxId = SfxId::from_static("world.lever.disengage");
    pub const WORLD_LOCK_OPEN: SfxId = SfxId::from_static("world.lock.open");
    pub const WORLD_PRESSURE_PLATE_CLICK_ON: SfxId =
        SfxId::from_static("world.pressure_plate.click_on");
    pub const WORLD_PRESSURE_PLATE_CLICK_OFF: SfxId =
        SfxId::from_static("world.pressure_plate.click_off");
    pub const WORLD_SWITCH_TOGGLE: SfxId = SfxId::from_static("world.switch.toggle");
    pub const WORLD_CRATE_BREAK: SfxId = SfxId::from_static("world.crate.break");
    pub const WORLD_ROCK_BREAK: SfxId = SfxId::from_static("world.rock.break");
    pub const WORLD_ROCK_HIT: SfxId = SfxId::from_static("world.rock.hit");
    pub const WORLD_PORTAL_ENTER: SfxId = SfxId::from_static("world.portal.enter");
    pub const WORLD_CHECKPOINT_ACTIVATE: SfxId =
        SfxId::from_static("world.checkpoint.activate");
    pub const WORLD_SAVE_POINT_ACTIVATE: SfxId =
        SfxId::from_static("world.save_point.activate");
    pub const WORLD_SAVE_POINT_IDLE_LOOP: SfxId =
        SfxId::from_static("world.save_point.idle_loop");
    pub const WORLD_TELEPORTER_LOOP: SfxId = SfxId::from_static("world.teleporter.loop");
    pub const WORLD_SECRET_REVEAL: SfxId = SfxId::from_static("world.secret.reveal");
    pub const WORLD_ABILITY_UNLOCK: SfxId = SfxId::from_static("world.ability.unlock");
    pub const WORLD_UPGRADE_PERMANENT: SfxId =
        SfxId::from_static("world.upgrade.permanent");
    pub const WORLD_PLATFORM_START: SfxId = SfxId::from_static("world.platform.start");
    pub const WORLD_PLATFORM_LOOP: SfxId = SfxId::from_static("world.platform.loop");
    pub const WORLD_PLATFORM_STOP: SfxId = SfxId::from_static("world.platform.stop");

    // Pickups
    pub const WORLD_PICKUP_GENERIC: SfxId = SfxId::from_static("world.pickup.generic");
    pub const WORLD_HEALTH_COLLECT: SfxId = SfxId::from_static("world.health.collect");
    pub const WORLD_HEART_CONTAINER_COLLECT: SfxId =
        SfxId::from_static("world.heart_container.collect");
    pub const WORLD_COIN_PICKUP: SfxId = SfxId::from_static("world.coin.pickup");
    pub const WORLD_COIN_COLLECT: SfxId = SfxId::from_static("world.coin.collect");
    pub const WORLD_COIN_LARGE: SfxId = SfxId::from_static("world.coin.large");
    pub const WORLD_COIN_HUGE: SfxId = SfxId::from_static("world.coin.huge");
    pub const WORLD_KEY_PICKUP: SfxId = SfxId::from_static("world.key.pickup");
    pub const WORLD_LORE_PICKUP: SfxId = SfxId::from_static("world.lore.pickup");
    pub const PLAYER_COLLECT_COIN: SfxId = SfxId::from_static("player.collect.coin");
    pub const PLAYER_COLLECT_HEALTH: SfxId = SfxId::from_static("player.collect.health");
    pub const PLAYER_PICKUP_HEALTH: SfxId = SfxId::from_static("player.pickup.health");

    // Ladder / climbing
    pub const PLAYER_LADDER_GRAB: SfxId = SfxId::from_static("player.ladder.grab");
    pub const PLAYER_LADDER_CLIMB: SfxId = SfxId::from_static("player.ladder.climb");
    pub const PLAYER_LADDER_CLIMB_LOOP: SfxId =
        SfxId::from_static("player.ladder.climb_loop");

    // Footstep variants by surface (variant numbers chosen per surface)
    pub const PLAYER_FOOTSTEP_GRASS_01: SfxId = SfxId::from_static("player.footstep.grass.01");
    pub const PLAYER_FOOTSTEP_GRASS_02: SfxId = SfxId::from_static("player.footstep.grass.02");
    pub const PLAYER_FOOTSTEP_GRASS_03: SfxId = SfxId::from_static("player.footstep.grass.03");
    pub const PLAYER_FOOTSTEP_WOOD_01: SfxId = SfxId::from_static("player.footstep.wood.01");
    pub const PLAYER_FOOTSTEP_WOOD_02: SfxId = SfxId::from_static("player.footstep.wood.02");
    pub const PLAYER_FOOTSTEP_WOOD_03: SfxId = SfxId::from_static("player.footstep.wood.03");
    pub const PLAYER_FOOTSTEP_WATER_01: SfxId = SfxId::from_static("player.footstep.water.01");
    pub const PLAYER_FOOTSTEP_WATER_02: SfxId = SfxId::from_static("player.footstep.water.02");
    pub const PLAYER_FOOTSTEP_WATER_03: SfxId = SfxId::from_static("player.footstep.water.03");
    pub const PLAYER_FOOTSTEP_ICE_01: SfxId = SfxId::from_static("player.footstep.ice.01");
    pub const PLAYER_FOOTSTEP_ICE_02: SfxId = SfxId::from_static("player.footstep.ice.02");
    pub const PLAYER_FOOTSTEP_ICE_03: SfxId = SfxId::from_static("player.footstep.ice.03");
    pub const PLAYER_FOOTSTEP_SAND_01: SfxId = SfxId::from_static("player.footstep.sand.01");
    pub const PLAYER_FOOTSTEP_SAND_02: SfxId = SfxId::from_static("player.footstep.sand.02");
    pub const PLAYER_FOOTSTEP_SAND_03: SfxId = SfxId::from_static("player.footstep.sand.03");
    pub const PLAYER_FOOTSTEP_SNOW_01: SfxId = SfxId::from_static("player.footstep.snow.01");
    pub const PLAYER_FOOTSTEP_SNOW_02: SfxId = SfxId::from_static("player.footstep.snow.02");
    pub const PLAYER_FOOTSTEP_SNOW_03: SfxId = SfxId::from_static("player.footstep.snow.03");
    pub const PLAYER_FOOTSTEP_GLASS_01: SfxId = SfxId::from_static("player.footstep.glass.01");
    pub const PLAYER_FOOTSTEP_GLASS_02: SfxId = SfxId::from_static("player.footstep.glass.02");

    // UI (additional)
    pub const UI_ACCEPT: SfxId = SfxId::from_static("ui.accept");
    pub const UI_BACK: SfxId = SfxId::from_static("ui.back");
    pub const UI_CONFIRM_WARNING: SfxId = SfxId::from_static("ui.confirm.warning");
    pub const UI_SLIDER_TICK: SfxId = SfxId::from_static("ui.slider.tick");
    pub const UI_TOGGLE_ON: SfxId = SfxId::from_static("ui.toggle.on");
    pub const UI_TOGGLE_OFF: SfxId = SfxId::from_static("ui.toggle.off");
    pub const UI_TOOLTIP_APPEAR: SfxId = SfxId::from_static("ui.tooltip.appear");
    pub const UI_NOTIFICATION_DISCOVERY: SfxId =
        SfxId::from_static("ui.notification.discovery");
    pub const UI_NOTIFICATION_QUEST_COMPLETE: SfxId =
        SfxId::from_static("ui.notification.quest_complete");
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
            .join("ambition_sandbox")
            .join("assets")
            .join("audio")
            .join("sfx.bank");
        if !path.exists() {
            eprintln!("(skipped) {} missing — run pack.py", path.display());
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
