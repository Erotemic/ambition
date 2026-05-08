//! `.sfxbank` binary file format reader.
//!
//! See `tools/ambition_sfx_pack/pack.py` for the canonical format spec
//! and the producer. This crate is intentionally pure-data: no audio
//! decoders, no Bevy, no async. Higher layers (`ambition_sfx`) wrap
//! these byte slices into playable clips.
//!
//! # Layout (little-endian)
//!
//! ```text
//! Header (40 bytes):
//!   magic           [u8; 8]   = b"AMBNDSFX"
//!   version         u32       = 1
//!   entry_count     u32
//!   entries_offset  u64
//!   payloads_offset u64
//!   names_offset    u64
//!
//! Entry table (entry_count * 64 bytes, sorted ascending by id_hash):
//!   id_hash         u64
//!   offset          u64
//!   length          u32
//!   codec           u8 (0=Wav, 1=Ogg, 2=Flac)
//!   channels        u8
//!   _pad0           u16
//!   sample_rate     u32
//!   duration_ms     u32
//!   default_gain_db f32
//!   peak_db         f32
//!   rms_db          f32
//!   flags           u32 (bit0=streamable_hint, bit1=looping)
//!   _reserved       [u8; 16]
//!
//! Payloads: concatenated, in entry order.
//!
//! Names section (debug; runtime may skip):
//!   per entry, in id_hash order: { len: u16, bytes: [u8; len] }
//! ```

use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::Path;

pub const MAGIC: [u8; 8] = *b"AMBNDSFX";
pub const VERSION: u32 = 1;
pub const HEADER_SIZE: usize = 40;
pub const ENTRY_SIZE: usize = 64;

pub const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
pub const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// FNV-1a 64-bit hash of a byte slice. `const fn` so call sites get
/// compile-time hashes for `SfxId::from_static` etc.
#[inline]
pub const fn fnv1a_64(bytes: &[u8]) -> u64 {
    let mut hash: u64 = FNV_OFFSET;
    let mut i = 0;
    while i < bytes.len() {
        hash ^= bytes[i] as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
        i += 1;
    }
    hash
}

#[inline]
pub const fn fnv1a_64_str(s: &str) -> u64 {
    fnv1a_64(s.as_bytes())
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Codec {
    Wav = 0,
    Ogg = 1,
    Flac = 2,
}

impl Codec {
    pub fn from_byte(byte: u8) -> Result<Self, BankError> {
        match byte {
            0 => Ok(Codec::Wav),
            1 => Ok(Codec::Ogg),
            2 => Ok(Codec::Flac),
            other => Err(BankError::UnknownCodec(other)),
        }
    }
}

/// Bit flags packed into [`EntryRecord::flags`].
pub mod flag {
    pub const STREAMABLE_HINT: u32 = 1 << 0;
    pub const LOOPING: u32 = 1 << 1;
}

/// Decoded form of a 64-byte entry record. Kept as an owned struct
/// (rather than a raw slice projection) so the reader can drop the
/// names section / verify alignment up-front and hand back ergonomic
/// `Copy` records to callers.
#[derive(Clone, Copy, Debug)]
pub struct EntryRecord {
    pub id_hash: u64,
    pub offset: u64,
    pub length: u32,
    pub codec: Codec,
    pub channels: u8,
    pub sample_rate: u32,
    pub duration_ms: u32,
    pub default_gain_db: f32,
    pub peak_db: f32,
    pub rms_db: f32,
    pub flags: u32,
}

impl EntryRecord {
    pub fn streamable_hint(&self) -> bool {
        self.flags & flag::STREAMABLE_HINT != 0
    }

    pub fn looping(&self) -> bool {
        self.flags & flag::LOOPING != 0
    }
}

#[derive(Debug, thiserror::Error)]
pub enum BankError {
    #[error("io error: {0}")]
    Io(#[from] io::Error),
    #[error("file too short to be a sfxbank ({0} bytes)")]
    TooShort(usize),
    #[error("bad magic: expected {expected:?}, got {got:?}")]
    BadMagic { expected: [u8; 8], got: [u8; 8] },
    #[error("unsupported version: {0} (this build supports {VERSION})")]
    UnsupportedVersion(u32),
    #[error("entries region out of bounds: offset {offset} + {bytes} > file len {file_len}")]
    EntriesOutOfBounds {
        offset: u64,
        bytes: u64,
        file_len: u64,
    },
    #[error("payload out of bounds for entry hash 0x{id_hash:016x}: offset {offset} + len {length} > file len {file_len}")]
    PayloadOutOfBounds {
        id_hash: u64,
        offset: u64,
        length: u32,
        file_len: u64,
    },
    #[error("entry table not sorted by id_hash (entry {index} hash 0x{this:016x} < prior 0x{prior:016x})")]
    EntriesUnsorted {
        index: usize,
        prior: u64,
        this: u64,
    },
    #[error("duplicate id_hash 0x{0:016x}")]
    DuplicateHash(u64),
    #[error("unknown codec byte: {0}")]
    UnknownCodec(u8),
    #[error("names section truncated at entry {index}")]
    NamesTruncated { index: usize },
    #[error("names section claims length {claimed} but only {available} bytes left")]
    NamesOverflow { claimed: usize, available: usize },
}

#[derive(Clone, Copy, Debug)]
pub struct Header {
    pub version: u32,
    pub entry_count: u32,
    pub entries_offset: u64,
    pub payloads_offset: u64,
    pub names_offset: u64,
}

/// Parsed `.sfxbank` ready for lookups. Holds the full file in memory;
/// at the catalog sizes we target (~76 entries → ~1.6 MB today) this
/// is fine. Swap to mmap if/when load size or RAM justifies it.
pub struct SfxBank {
    bytes: Vec<u8>,
    header: Header,
    entries: Vec<EntryRecord>,
    by_hash: HashMap<u64, usize>,
    names: HashMap<u64, String>,
}

impl SfxBank {
    pub fn from_path(path: &Path) -> Result<Self, BankError> {
        let bytes = fs::read(path)?;
        Self::from_bytes(bytes)
    }

    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self, BankError> {
        if bytes.len() < HEADER_SIZE {
            return Err(BankError::TooShort(bytes.len()));
        }

        let mut magic = [0u8; 8];
        magic.copy_from_slice(&bytes[0..8]);
        if magic != MAGIC {
            return Err(BankError::BadMagic {
                expected: MAGIC,
                got: magic,
            });
        }

        let version = read_u32(&bytes, 8);
        if version != VERSION {
            return Err(BankError::UnsupportedVersion(version));
        }

        let entry_count = read_u32(&bytes, 12);
        let entries_offset = read_u64(&bytes, 16);
        let payloads_offset = read_u64(&bytes, 24);
        let names_offset = read_u64(&bytes, 32);

        let header = Header {
            version,
            entry_count,
            entries_offset,
            payloads_offset,
            names_offset,
        };

        let entries_bytes_needed = entry_count as u64 * ENTRY_SIZE as u64;
        let file_len = bytes.len() as u64;
        if entries_offset.saturating_add(entries_bytes_needed) > file_len {
            return Err(BankError::EntriesOutOfBounds {
                offset: entries_offset,
                bytes: entries_bytes_needed,
                file_len,
            });
        }

        let mut entries = Vec::with_capacity(entry_count as usize);
        let mut by_hash = HashMap::with_capacity(entry_count as usize);
        let mut prior_hash: Option<u64> = None;
        for index in 0..entry_count as usize {
            let base = entries_offset as usize + index * ENTRY_SIZE;
            let record = parse_entry(&bytes, base)?;
            if let Some(prior) = prior_hash {
                if record.id_hash < prior {
                    return Err(BankError::EntriesUnsorted {
                        index,
                        prior,
                        this: record.id_hash,
                    });
                }
                if record.id_hash == prior {
                    return Err(BankError::DuplicateHash(record.id_hash));
                }
            }
            prior_hash = Some(record.id_hash);

            if record.offset.saturating_add(record.length as u64) > file_len {
                return Err(BankError::PayloadOutOfBounds {
                    id_hash: record.id_hash,
                    offset: record.offset,
                    length: record.length,
                    file_len,
                });
            }

            by_hash.insert(record.id_hash, index);
            entries.push(record);
        }

        let names = parse_names(&bytes, &entries, names_offset as usize)?;

        Ok(Self {
            bytes,
            header,
            entries,
            by_hash,
            names,
        })
    }

    pub fn header(&self) -> &Header {
        &self.header
    }

    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    pub fn lookup(&self, id_hash: u64) -> Option<EntryRef<'_>> {
        self.by_hash.get(&id_hash).map(|&index| EntryRef {
            record: &self.entries[index],
            payload: payload_slice(&self.bytes, &self.entries[index]),
            name: self.names.get(&id_hash).map(String::as_str),
        })
    }

    pub fn contains(&self, id_hash: u64) -> bool {
        self.by_hash.contains_key(&id_hash)
    }

    pub fn iter(&self) -> impl Iterator<Item = EntryRef<'_>> {
        self.entries.iter().map(|record| EntryRef {
            record,
            payload: payload_slice(&self.bytes, record),
            name: self.names.get(&record.id_hash).map(String::as_str),
        })
    }

    /// All known names, hashed lookup, mainly for debug/dump tooling.
    pub fn name_for(&self, id_hash: u64) -> Option<&str> {
        self.names.get(&id_hash).map(String::as_str)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct EntryRef<'a> {
    pub record: &'a EntryRecord,
    pub payload: &'a [u8],
    pub name: Option<&'a str>,
}

fn payload_slice<'a>(bytes: &'a [u8], record: &EntryRecord) -> &'a [u8] {
    let start = record.offset as usize;
    let end = start + record.length as usize;
    &bytes[start..end]
}

fn parse_entry(bytes: &[u8], base: usize) -> Result<EntryRecord, BankError> {
    let id_hash = read_u64(bytes, base);
    let offset = read_u64(bytes, base + 8);
    let length = read_u32(bytes, base + 16);
    let codec = Codec::from_byte(bytes[base + 20])?;
    let channels = bytes[base + 21];
    // 22..24: _pad0
    let sample_rate = read_u32(bytes, base + 24);
    let duration_ms = read_u32(bytes, base + 28);
    let default_gain_db = read_f32(bytes, base + 32);
    let peak_db = read_f32(bytes, base + 36);
    let rms_db = read_f32(bytes, base + 40);
    let flags = read_u32(bytes, base + 44);
    // 48..64: _reserved
    Ok(EntryRecord {
        id_hash,
        offset,
        length,
        codec,
        channels,
        sample_rate,
        duration_ms,
        default_gain_db,
        peak_db,
        rms_db,
        flags,
    })
}

fn parse_names(
    bytes: &[u8],
    entries: &[EntryRecord],
    mut cursor: usize,
) -> Result<HashMap<u64, String>, BankError> {
    let mut names = HashMap::with_capacity(entries.len());
    for (index, record) in entries.iter().enumerate() {
        if cursor + 2 > bytes.len() {
            // Names section is optional — if it doesn't exist or is
            // truncated past the headers, return whatever we managed.
            // A truncation in the middle of the section is a real
            // error though.
            if index == 0 {
                return Ok(names);
            }
            return Err(BankError::NamesTruncated { index });
        }
        let len = read_u16(bytes, cursor) as usize;
        cursor += 2;
        if cursor + len > bytes.len() {
            return Err(BankError::NamesOverflow {
                claimed: len,
                available: bytes.len() - cursor,
            });
        }
        if let Ok(s) = std::str::from_utf8(&bytes[cursor..cursor + len]) {
            names.insert(record.id_hash, s.to_owned());
        }
        cursor += len;
    }
    Ok(names)
}

#[inline]
fn read_u16(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([bytes[offset], bytes[offset + 1]])
}

#[inline]
fn read_u32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ])
}

#[inline]
fn read_u64(bytes: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
        bytes[offset + 4],
        bytes[offset + 5],
        bytes[offset + 6],
        bytes[offset + 7],
    ])
}

#[inline]
fn read_f32(bytes: &[u8], offset: usize) -> f32 {
    f32::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fnv1a_matches_known_vectors() {
        // Standard FNV-1a 64 vectors:
        // ""        -> 0xcbf29ce484222325 (offset basis)
        // "a"       -> 0xaf63dc4c8601ec8c
        // "foobar"  -> 0x85944171f73967e8
        assert_eq!(fnv1a_64(b""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(fnv1a_64(b"a"), 0xaf63_dc4c_8601_ec8c);
        assert_eq!(fnv1a_64(b"foobar"), 0x8594_4171_f739_67e8);
    }

    #[test]
    fn fnv1a_const_eval_is_usable() {
        const HASH: u64 = fnv1a_64_str("player.jump");
        // Just assert it's non-zero and matches the runtime call —
        // the const-eval path is the actual point of this test.
        assert_eq!(HASH, fnv1a_64_str("player.jump"));
        assert_ne!(HASH, 0);
    }

    #[test]
    fn rejects_short_buffer() {
        let err = SfxBank::from_bytes(vec![0u8; 8]).err().unwrap();
        assert!(matches!(err, BankError::TooShort(8)));
    }

    #[test]
    fn rejects_bad_magic() {
        let mut bytes = vec![0u8; HEADER_SIZE];
        bytes[0..8].copy_from_slice(b"NOTABANK");
        let err = SfxBank::from_bytes(bytes).err().unwrap();
        assert!(matches!(err, BankError::BadMagic { .. }));
    }

    /// Round-trip: build a tiny bank in-memory mimicking the Python
    /// packer's layout, parse it back, verify content.
    #[test]
    fn round_trip_minimal_bank() {
        let payload_a = b"AAAA".to_vec();
        let payload_b = b"BBBBBBBB".to_vec();
        let id_a = "test.a";
        let id_b = "test.b";
        let mut entries = vec![
            (id_a, fnv1a_64_str(id_a), payload_a.clone()),
            (id_b, fnv1a_64_str(id_b), payload_b.clone()),
        ];
        entries.sort_by_key(|(_, h, _)| *h);

        let entry_count = entries.len() as u32;
        let entries_offset = HEADER_SIZE as u64;
        let payloads_offset = entries_offset + entry_count as u64 * ENTRY_SIZE as u64;

        let mut buf: Vec<u8> = Vec::new();
        // Header
        buf.extend_from_slice(&MAGIC);
        buf.extend_from_slice(&VERSION.to_le_bytes());
        buf.extend_from_slice(&entry_count.to_le_bytes());
        buf.extend_from_slice(&entries_offset.to_le_bytes());
        buf.extend_from_slice(&payloads_offset.to_le_bytes());

        // Compute names_offset later; placeholder for now.
        let names_offset_pos = buf.len();
        buf.extend_from_slice(&0u64.to_le_bytes());

        // Entry records
        let mut cursor = payloads_offset;
        let mut record_offsets = Vec::new();
        for (_id, hash, payload) in &entries {
            record_offsets.push(cursor);
            buf.extend_from_slice(&hash.to_le_bytes());
            buf.extend_from_slice(&cursor.to_le_bytes());
            buf.extend_from_slice(&(payload.len() as u32).to_le_bytes());
            buf.push(0); // codec=Wav
            buf.push(2); // channels
            buf.extend_from_slice(&0u16.to_le_bytes()); // _pad0
            buf.extend_from_slice(&48000u32.to_le_bytes());
            buf.extend_from_slice(&123u32.to_le_bytes());
            buf.extend_from_slice(&0.0f32.to_le_bytes());
            buf.extend_from_slice(&(-3.0f32).to_le_bytes());
            buf.extend_from_slice(&(-12.0f32).to_le_bytes());
            buf.extend_from_slice(&0u32.to_le_bytes()); // flags
            buf.extend_from_slice(&[0u8; 16]); // _reserved
            cursor += payload.len() as u64;
        }
        // Payloads
        for (_, _, payload) in &entries {
            buf.extend_from_slice(payload);
        }

        // Names section
        let names_offset = buf.len() as u64;
        for (id, _, _) in &entries {
            buf.extend_from_slice(&(id.len() as u16).to_le_bytes());
            buf.extend_from_slice(id.as_bytes());
        }
        // Patch in the names offset.
        buf[names_offset_pos..names_offset_pos + 8]
            .copy_from_slice(&names_offset.to_le_bytes());

        let bank = SfxBank::from_bytes(buf).expect("parse");
        assert_eq!(bank.entry_count(), 2);

        let a = bank.lookup(fnv1a_64_str(id_a)).expect("entry a");
        assert_eq!(a.payload, payload_a.as_slice());
        assert_eq!(a.record.codec, Codec::Wav);
        assert_eq!(a.record.channels, 2);
        assert_eq!(a.record.sample_rate, 48000);
        assert_eq!(a.name, Some(id_a));

        let b = bank.lookup(fnv1a_64_str(id_b)).expect("entry b");
        assert_eq!(b.payload, payload_b.as_slice());
        assert_eq!(b.name, Some(id_b));

        assert!(!bank.contains(fnv1a_64_str("does.not.exist")));
    }
}
