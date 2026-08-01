//! **The deterministic snapshot vocabulary.**
//!
//! ⚠ Lives in the FLOOR — `ambition_platformer2d_core` depends on no workspace crate,
//! which is what lets every domain implement [`SnapshotState`] for its OWN
//! types. It was `ambition_platformer2d_runtime::rollback::codec` until 2026-07-30, and
//! `ambition_platformer2d_runtime` depends on twenty domain crates while none depends on it,
//! so no domain could: the trait sat above them. The tree recorded that cost as
//! ~100 foreign impls in one 2688-line file, there because it was the only
//! place they could compile.
//!
//! Carved under api-growth-method.md §4, which authorises an internal carve
//! when a leak cannot be closed without moving code between crates — and
//! authorises exactly the boundary the leak names.
//!
//! ⚠ `CanonicalCodecStrategy` deliberately did NOT come along: it implements
//! `bevy_ggrs::Strategy`, so it is a GGRS bridge, and pulling `bevy_ggrs` into
//! the floor would make every domain depend on the patched fork that is this
//! engine's highest-cost consumer leak.

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

pub fn put_vec2(out: &mut Vec<u8>, value: bevy_math::Vec2) {
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

    pub fn vec2(&mut self) -> Option<bevy_math::Vec2> {
        Some(bevy_math::Vec2::new(self.f32()?, self.f32()?))
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

/// One overload per encodable primitive, so `snapshot_pod!` does not have to
/// name the writer twice. The reader cannot do this — `Option<T>` inference
/// would need the field type, which the macro does not have.
pub trait PasteEncode: Copy {
    fn put(self, out: &mut Vec<u8>);
}

macro_rules! paste_encode {
    ($ty:ty, $writer:ident) => {
        impl PasteEncode for $ty {
            fn put(self, out: &mut Vec<u8>) {
                $writer(out, self);
            }
        }
    };
}

paste_encode!(f32, put_f32);
paste_encode!(bool, put_bool);
paste_encode!(u8, put_u8);
paste_encode!(u32, put_u32);
paste_encode!(i32, put_i32);
paste_encode!(u64, put_u64);
paste_encode!(bevy_math::Vec2, put_vec2);

#[doc(hidden)]
pub fn paste_put<T: PasteEncode>(out: &mut Vec<u8>, value: T) {
    value.put(out);
}

// ── The three authoring macros ──────────────────────────────────────────────
//
// ⚠ These are `#[macro_export]`, so every path inside them is `$crate::`-
// qualified. They expand to `impl SnapshotState for …`, which is why they had
// to move WITH the trait: the orphan rule binds an impl to the crate that owns
// either the trait or the type, and after the carve that is the domain crate at
// the call site — not `ambition_platformer2d_runtime`, where they used to live.

/// A struct whose every field is a `PasteEncode` primitive, read back in the
/// same order.
#[macro_export]
macro_rules! snapshot_pod {
    ($ty:path { $($field:ident : $get:ident),+ $(,)? }) => {
        impl $crate::snapshot::SnapshotState for $ty {
            fn encode(&self, out: &mut ::std::vec::Vec<u8>) {
                $( $crate::snapshot::paste_put(out, self.$field); )+
            }
            fn decode(r: &mut $crate::snapshot::Reader<'_>) -> ::core::option::Option<Self> {
                ::core::option::Option::Some(Self { $( $field: r.$get()? ),+ })
            }
        }
    };
}

/// A fieldless enum, encoded as one explicitly authored byte per variant.
///
/// ⚠ The codes are AUTHORED, never derived from declaration order: a variant
/// inserted in the middle would silently renumber every one after it, and a
/// snapshot decoded across that change would be wrong rather than absent.
#[macro_export]
macro_rules! snapshot_unit_enum {
    ($ty:path { $($variant:ident = $code:literal),+ $(,)? }) => {
        impl $crate::snapshot::SnapshotState for $ty {
            fn encode(&self, out: &mut ::std::vec::Vec<u8>) {
                #[allow(unused_imports)]
                use $ty as E;
                $crate::snapshot::put_u8(
                    out,
                    match self {
                        $( E::$variant => $code ),+
                    },
                );
            }
            fn decode(r: &mut $crate::snapshot::Reader<'_>) -> ::core::option::Option<Self> {
                #[allow(unused_imports)]
                use $ty as E;
                match r.u8()? {
                    $( $code => ::core::option::Option::Some(E::$variant), )+
                    _ => ::core::option::Option::None,
                }
            }
        }
    };
}

/// A unit struct: presence is the whole state, so the encoding is empty.
#[macro_export]
macro_rules! snapshot_marker {
    ($ty:path) => {
        impl $crate::snapshot::SnapshotState for $ty {
            fn encode(&self, _out: &mut ::std::vec::Vec<u8>) {}
            fn decode(_r: &mut $crate::snapshot::Reader<'_>) -> ::core::option::Option<Self> {
                ::core::option::Option::Some(Self)
            }
        }
    };
}


/// **What a capability REQUIRES rewound**, declared where it can be read without
/// linking a rollback host.
///
/// A capability offers its rollback state and a composition installs it (see
/// `capability_demo` for the worked example). That split keeps a mechanic's
/// dependency closure to foundations — and leaves a hole: nothing makes the
/// composition actually install the offer, and **omitting one is a DESYNC, not
/// a missing feature**. A cooldown that is not rewound lets its action fire
/// twice from one charge on a resimulated frame.
///
/// So a capability also declares what it needs, and a host can check.
/// `ambition_platformer2d_runtime::rollback::missing_required_state` is the check; this is
/// the vocabulary, and it lives here — a foundation with no Bevy app and no
/// GGRS — precisely so the declaring end costs nothing.
///
/// It is the same shape the content compiler already uses for
/// `RuntimeDisposition::Runtime`: declare the obligation next to the thing that
/// has it, and let the assembler refuse when it is unmet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RequiredRollbackState {
    /// The owner label the registration must carry — the capability's own name.
    pub owner: &'static str,
    /// The registration name, e.g. `"pulse.cooldown"`.
    pub name: &'static str,
    /// **What breaks if it is missing.** Not decoration: a host that hits this
    /// needs to know whether it is looking at a desync or at an optional extra,
    /// and only the capability knows.
    pub why: &'static str,
}
