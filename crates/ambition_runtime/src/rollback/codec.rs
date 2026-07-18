//! Deterministic canonical strategies and checksum projections for GGRS.
//!
//! `bevy_ggrs` owns frame history, save/load ordering, entity recreation, and
//! resimulation. Ambition supplies explicit field-order encoders where a plain
//! clone or Rust `Hash` would not provide a meaningful deterministic contract.
//! The bytes live only inside GGRS snapshot storage and checksum plugins; there
//! is no independent Ambition snapshot, restore, or history authority.

/// A deterministic, process-stable FNV-1a 64-bit hash.
#[derive(Clone, Copy, Debug)]
pub struct StateHasher(u64);

impl Default for StateHasher {
    fn default() -> Self {
        Self(0xcbf2_9ce4_8422_2325)
    }
}

impl StateHasher {
    pub fn write(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.0 ^= *byte as u64;
            self.0 = self.0.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }

    pub fn finish(self) -> u64 {
        self.0
    }
}

/// Canonical full-value encoding used by the GGRS storage strategy and checksum projection.
pub trait SnapshotState: Send + Sync + 'static {
    fn encode(&self, out: &mut Vec<u8>);
    fn decode(reader: &mut Reader<'_>) -> Option<Self>
    where
        Self: Sized;
}

/// Canonical mutable-cursor projection for values whose complete authored half
/// is stored by `bevy_ggrs` using clone snapshots.
pub trait SnapshotCursor: Send + Sync + 'static {
    fn encode_cursor(&self, out: &mut Vec<u8>);
}

/// Canonical reference projection for values that contain authored definitions.
/// GGRS stores the complete value; this projection exists solely for checksums.
pub trait SnapshotResolve: Send + Sync + 'static {
    fn encode_ref(&self, out: &mut Vec<u8>);
}

pub fn encode_state<T: SnapshotState>(value: &T) -> Vec<u8> {
    let mut bytes = Vec::new();
    value.encode(&mut bytes);
    bytes
}

pub fn decode_state<T: SnapshotState>(bytes: &[u8]) -> Option<T> {
    let mut reader = Reader::new(bytes);
    let value = T::decode(&mut reader)?;
    reader.finish()?;
    Some(value)
}

pub fn state_checksum<T: SnapshotState>(value: &T) -> u64 {
    checksum_bytes(&encode_state(value))
}

pub fn cursor_checksum<T: SnapshotCursor>(value: &T) -> u64 {
    let mut bytes = Vec::new();
    value.encode_cursor(&mut bytes);
    checksum_bytes(&bytes)
}

pub fn resolved_checksum<T: SnapshotResolve>(value: &T) -> u64 {
    let mut bytes = Vec::new();
    value.encode_ref(&mut bytes);
    checksum_bytes(&bytes)
}

pub fn checksum_bytes(bytes: &[u8]) -> u64 {
    let mut hasher = StateHasher::default();
    hasher.write(bytes);
    hasher.finish()
}

pub struct CanonicalCodecStrategy<T>(std::marker::PhantomData<T>);

impl<T> bevy_ggrs::Strategy for CanonicalCodecStrategy<T>
where
    T: SnapshotState,
{
    type Target = T;
    type Stored = Vec<u8>;

    fn store(target: &Self::Target) -> Self::Stored {
        encode_state(target)
    }

    fn load(stored: &Self::Stored) -> Self::Target {
        decode_state(stored).unwrap_or_else(|| {
            panic!(
                "canonical rollback codec for {} rejected bytes it previously encoded",
                std::any::type_name::<T>()
            )
        })
    }

    fn update(target: &mut Self::Target, stored: &Self::Stored) {
        *target = Self::load(stored);
    }
}

pub fn put_opt_str(out: &mut Vec<u8>, value: Option<&str>) {
    match value {
        None => put_bool(out, false),
        Some(value) => {
            put_bool(out, true);
            put_str(out, value);
        }
    }
}

pub fn put_str(out: &mut Vec<u8>, value: &str) {
    put_u32(out, value.len() as u32);
    out.extend_from_slice(value.as_bytes());
}

pub fn put_f32(out: &mut Vec<u8>, value: f32) {
    out.extend_from_slice(&canonical_f32_bits(value).to_le_bytes());
}

pub fn put_i32(out: &mut Vec<u8>, value: i32) {
    out.extend_from_slice(&value.to_le_bytes());
}

pub fn put_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_le_bytes());
}

pub fn put_bool(out: &mut Vec<u8>, value: bool) {
    out.push(value as u8);
}

pub fn put_u8(out: &mut Vec<u8>, value: u8) {
    out.push(value);
}

pub fn put_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

pub fn put_vec2(out: &mut Vec<u8>, value: bevy::math::Vec2) {
    put_f32(out, value.x);
    put_f32(out, value.y);
}

pub struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, at: 0 }
    }

    fn take(&mut self, len: usize) -> Option<&'a [u8]> {
        let end = self.at.checked_add(len)?;
        let result = self.bytes.get(self.at..end)?;
        self.at = end;
        Some(result)
    }

    pub fn f32(&mut self) -> Option<f32> {
        Some(f32::from_bits(u32::from_le_bytes(
            self.take(4)?.try_into().ok()?,
        )))
    }

    pub fn i32(&mut self) -> Option<i32> {
        Some(i32::from_le_bytes(self.take(4)?.try_into().ok()?))
    }

    pub fn u64(&mut self) -> Option<u64> {
        Some(u64::from_le_bytes(self.take(8)?.try_into().ok()?))
    }

    pub fn bool(&mut self) -> Option<bool> {
        match self.u8()? {
            0 => Some(false),
            1 => Some(true),
            _ => None,
        }
    }

    pub fn u8(&mut self) -> Option<u8> {
        Some(*self.take(1)?.first()?)
    }

    pub fn u32(&mut self) -> Option<u32> {
        Some(u32::from_le_bytes(self.take(4)?.try_into().ok()?))
    }

    pub fn vec2(&mut self) -> Option<bevy::math::Vec2> {
        Some(bevy::math::Vec2::new(self.f32()?, self.f32()?))
    }

    pub fn str(&mut self) -> Option<&'a str> {
        let len = self.u32()? as usize;
        std::str::from_utf8(self.take(len)?).ok()
    }

    #[allow(clippy::option_option)]
    pub fn opt_str(&mut self) -> Option<Option<&'a str>> {
        Some(if self.bool()? {
            Some(self.str()?)
        } else {
            None
        })
    }

    pub fn finish(self) -> Option<()> {
        (self.at == self.bytes.len()).then_some(())
    }
}

fn canonical_f32_bits(value: f32) -> u32 {
    if value.is_nan() {
        f32::NAN.to_bits()
    } else {
        value.to_bits()
    }
}
