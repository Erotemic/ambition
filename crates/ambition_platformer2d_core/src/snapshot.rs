//! Backend-neutral deterministic snapshot vocabulary.
//!
//! Domain crates implement these traits for their own state without depending
//! on the rollback backend. Backend-specific storage strategies remain above
//! this foundation.

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

/// ⭐⭐ **THE ONE WAY TO BUILD A PEER CHECKSUM PROJECTION.**
///
/// A rollback checksum registered through a `*_checksum` registrar compares only
/// what its projection hashes. That is the mechanism the whole peer-identity
/// campaign runs on, and it was spelled SIX different ways across three crates —
/// a raw field returned unhashed, a `Vec` plus `extend_from_slice`, a
/// wrapping-multiply by a constant, a byte-writer, and two variants of a local
/// `peer_stable_digest` helper. Three different hashing strategies for one job.
///
/// ⛔⛤ THAT IS NOT A TIDINESS COMPLAINT; IT IS WHERE THE BUGS CAME FROM.
/// `CheckpointOperationKey` had THREE spellings of one projection, two folding an
/// optional scope as `scope.0 | 1 << 63` and one writing a tagged value, so the
/// same key hashed differently depending on which resource held it — and the
/// bit-or spelling silently collided a scope whose top bit was set with the
/// absent case. Every such defect found in this campaign has been a projection
/// that encoded something ALMOST the same way as its neighbour.
///
/// ⇒ Two rules are structural here rather than remembered:
///
/// 1. **A DOMAIN IS REQUIRED.** [`Self::in_domain`] takes the name of what is
///    being compared, so two projections cannot produce the same digest from the
///    same numbers. Four of the six old spellings carried no domain at all.
/// 2. **ABSENT IS NEVER ZERO.** [`Self::opt_u64`] writes a presence tag, so "no
///    ordinal authority" and "ordinal 0" stay different answers. This rule was
///    written out longhand at seven call sites and violated at one.
///
/// ⚠ WHAT IT DELIBERATELY DOES NOT DO is decide WHICH fields go in. That is the
/// peer/local judgement and it belongs to the type that owns the state, beside
/// the comment explaining why — a helper that chose for you would move that
/// judgement somewhere nobody reviews it.
#[derive(Clone, Debug)]
pub struct PeerDigest {
    hasher: StateHasher,
}

impl PeerDigest {
    /// Start a projection for a named domain.
    ///
    /// ⭐ THE DOMAIN IS A STRING, NOT A MAGIC NUMBER. The two older tags were
    /// `0x5700_0000_0000_0001` and `0x5D00_0000_0000_0002`, which are unreadable
    /// and give no hint whether a third would collide. A name says what is being
    /// compared and reads the way this repo's stable schema names read.
    pub fn in_domain(domain: &str) -> Self {
        let mut hasher = StateHasher::default();
        hasher.write(&(domain.len() as u64).to_le_bytes());
        hasher.write(domain.as_bytes());
        Self { hasher }
    }

    pub fn u64(mut self, value: u64) -> Self {
        self.hasher.write(&value.to_le_bytes());
        self
    }

    pub fn bool(mut self, value: bool) -> Self {
        self.hasher.write(&[u8::from(value)]);
        self
    }

    /// An optional count, with a PRESENCE TAG so absent and zero differ.
    pub fn opt_u64(mut self, value: Option<u64>) -> Self {
        match value {
            None => self.hasher.write(&[0]),
            Some(value) => {
                self.hasher.write(&[1]);
                self.hasher.write(&value.to_le_bytes());
            }
        }
        self
    }

    /// Length-prefixed bytes, so two adjacent fields cannot be re-split.
    pub fn bytes(mut self, value: &[u8]) -> Self {
        self.hasher.write(&(value.len() as u64).to_le_bytes());
        self.hasher.write(value);
        self
    }

    pub fn finish(self) -> u64 {
        self.hasher.finish()
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

    /// Read a nested [`SnapshotState`] value.
    ///
    /// ⭐⭐ **GENERIC ON PURPOSE, SO THIS CRATE LEARNS NOTHING ABOUT ITS
    /// CALLERS.** `snapshot_pod!` decodes each field with `r.<accessor>()`, and
    /// before this the accessors were a fixed list of primitives — so a struct
    /// with one non-primitive field had to hand-roll its WHOLE codec, which for a
    /// twenty-field rollback component is where a field-order bug hides and a
    /// field-order bug in a rollback codec is a desync.
    ///
    /// ⇒ `armor: state` in a `snapshot_pod!` list now works for any type that
    /// implements [`SnapshotState`], resolved by inference from the field.
    pub fn state<T: SnapshotState>(&mut self) -> Option<T> {
        T::decode(self)
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
    // ⚠ THE OBSERVATION IS EXCLUSIVE: an infinity is non-finite, so it must not
    // ALSO land in the finite tally that a caller uses as its anti-vacuity floor.
    // A floor inflated by the very values it is meant to make visible is a floor
    // that rises when the defect spreads.
    if value.is_finite() {
        non_finite::count_finite();
    } else {
        non_finite::observe(value);
    }
    if value.is_nan() {
        f32::NAN.to_bits()
    } else {
        value.to_bits()
    }
}

/// ⛔⛔ **A NON-FINITE VALUE IN CANONICAL STATE IS NORMALISED SO THE DESYNC CHECK
/// CANNOT SEE IT EITHER — and that is not a bug, it is why nobody caught this.**
///
/// `canonical_f32_bits` already asks `is_nan()`, and it asks in order to
/// collapse every NaN to ONE bit pattern **so that two peers' checksums agree**.
/// The mechanism that exists to notice two peers diverging has been made blind to
/// this specific poison, deliberately, for a good reason. ⇒ Two peers can hold a
/// NaN in the same canonical field and agree perfectly about it forever, while
/// `f32::clamp` returns NaN for a NaN input, so one such value poisons a meter
/// permanently and every later comparison against it is false. The failure and
/// the healthy state are indistinguishable to everything that looks.
///
/// ⭐ SO THIS IS NOT A NEW GUARD — it is the smallest possible correction to a
/// function that already computes the answer and throws it away. And it is
/// EXHAUSTIVE BY CONSTRUCTION rather than by discipline: passing through here is
/// what MAKES a value canonical, so a type that gains a canonical float is
/// observed automatically. There is no walk to keep in sync and nothing to
/// forget.
///
/// ⚠ **WHAT IT DOES NOT COVER, and the text must not claim otherwise: values
/// that are never ENCODED.** MEASURED 2026-09-10 against
/// `game/ambition_app/tests/rollback_schema_baseline.txt` (490 rows): 166 are
/// `component-clone` and call `encode` on nothing. 107 of those name another
/// authoritative projection that does cover them; **59 say in as many words that
/// they are "not in the session checksum"**, and those are the rows nothing here
/// or anywhere else observes. This sees the canonical-checksum half — the half
/// that must agree between peers — and is blind to the other.
///
/// ⚠ AND `inf` / `-inf` ARE COUNTED HERE BUT NOT CANONICALISED BY THE ENCODER.
/// Counting without changing the encoding is deliberate: whether an infinity
/// should collapse the way a NaN does changes what two peers agree about, which
/// is a maintainer's decision and not this observer's.
pub mod non_finite {
    use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
    use std::sync::{Mutex, MutexGuard, OnceLock};

    // ⛔⛔ PROCESS-GLOBAL, NOT THREAD-LOCAL, AND THE FIRST DESIGN WAS THE OTHER ONE.
    // A thread-local was the obvious way to keep one test's numbers out of
    // another's, and it MEASURED ZERO: `cargo test` runs the harness step from
    // the caller's thread, but bevy's multi-threaded executor runs the encoding
    // systems on TASK-POOL threads, so a thread-local armed by the test observes
    // nothing at all. The anti-vacuity floor below is the only reason that was
    // visible instead of reading as a clean bill of health -- MEASURED
    // 2026-09-10: thread-local 0 finite, process-global 116,280 finite over the
    // same 30 sync-test frames.
    static ARMED: AtomicBool = AtomicBool::new(false);
    static NON_FINITE: AtomicU64 = AtomicU64::new(0);
    static FINITE: AtomicU64 = AtomicU64::new(0);

    fn arming_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    /// Held for as long as the counters are armed.
    ///
    /// ⚠ THE LOCK SERIALISES OBSERVERS, NOT THE PROCESS. Two tests that both arm
    /// cannot interleave -- which matters, because one of them deliberately
    /// injects a NaN and would otherwise redden the other. Every OTHER test in
    /// the binary keeps encoding while this one is armed, so the counts are
    /// PROCESS-WIDE for the armed window and a caller must not describe them as
    /// its own. That is a weaker claim and it is the true one: what a green run
    /// certifies is "no canonical float encoded ANYWHERE in this process during
    /// the window was non-finite".
    pub struct Armed {
        _guard: MutexGuard<'static, ()>,
    }

    impl Drop for Armed {
        fn drop(&mut self) {
            ARMED.store(false, Ordering::Relaxed);
        }
    }

    impl Armed {
        /// `(non-finite seen, finite seen)` since arming.
        ///
        /// ⚠ THE SECOND NUMBER IS THE LOAD-BEARING ONE. A window that encoded NO
        /// floats reports zero non-finite and reads exactly like a healthy one,
        /// so a caller must floor the population rather than only check the
        /// offenders.
        pub fn observed(&self) -> (u64, u64) {
            (
                NON_FINITE.load(Ordering::Relaxed),
                FINITE.load(Ordering::Relaxed),
            )
        }
    }

    /// ⭐ DISARMED BY DEFAULT, so the shipped encoder pays one relaxed atomic
    /// load per float and nothing else. A counter that is always on is a cost
    /// paid every frame of every build for a question only a test asks.
    pub fn arm() -> Armed {
        let guard = arming_lock().lock().unwrap_or_else(|e| e.into_inner());
        NON_FINITE.store(0, Ordering::Relaxed);
        FINITE.store(0, Ordering::Relaxed);
        ARMED.store(true, Ordering::Relaxed);
        Armed { _guard: guard }
    }

    pub(super) fn observe(_value: f32) {
        if ARMED.load(Ordering::Relaxed) {
            NON_FINITE.fetch_add(1, Ordering::Relaxed);
        }
    }

    pub(super) fn count_finite() {
        if ARMED.load(Ordering::Relaxed) {
            FINITE.fetch_add(1, Ordering::Relaxed);
        }
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
// These are `#[macro_export]`, so every path inside them is `$crate::`- qualified.

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
/// The codes are AUTHORED, never derived from declaration order: a variant
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

/// What a capability REQUIRES rewound, declared where it can be read without
/// linking a rollback host.
///
/// A capability offers its rollback state and a composition installs it (see
/// `capability_demo` for the worked example). That split keeps a mechanic's
/// dependency closure to foundations — and leaves a hole: nothing makes the
/// composition actually install the offer, and omitting one is a DESYNC, not
/// a missing feature. A cooldown that is not rewound lets its action fire
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
    /// What breaks if it is missing. Not decoration: a host that hits this
    /// needs to know whether it is looking at a desync or at an optional extra,
    /// and only the capability knows.
    pub why: &'static str,
}

/// Backend-neutral registrar for domain-owned rollback state.
///
/// Domains supply the concrete component type and checksum projection; the host
/// implements storage mechanics and may depend on the rollback backend. Generic
/// methods are intentionally not object-safe. The runtime uses an `App` wrapper
/// to satisfy dependency and orphan-rule boundaries without moving backend
/// dependencies into domain crates.
pub trait RollbackRegistrar {
    /// Defaults fail closed so a partial test registrar cannot silently omit newly
    /// requested rollback state.
    fn rollback_component_canonical<T>(
        &mut self,
        _owner: &'static str,
        name: &'static str,
    ) -> &mut Self
    where
        T: bevy_ecs::component::Component<Mutability = bevy_ecs::component::Mutable>
            + SnapshotState,
    {
        panic!("RollbackRegistrar does not support rollback_component_canonical for {name}")
    }

    /// Canonical snapshot whose CHECKSUM is a stated projection of the value.
    ///
    /// ⛔⛤ **THE COMPONENT HALF OF A FAMILY THAT WAS ASYMMETRIC.** Resources had
    /// `rollback_resource_canonical_checksum` and
    /// `rollback_resource_optional_canonical_checksum`; components had only the
    /// CLONE-strategy `rollback_component_clone_checksum`. So a component that
    /// snapshots through its own canonical codec had no way to state a
    /// projection at all, and the peer/local split was simply unavailable for
    /// it — which is not a decision anybody made, it is a gap. Measured
    /// 2026-09-15 while looking for the registrar `TransactionId` would need:
    /// its `{epoch}\t{room}\t{session}` string is `component-canonical`, two of
    /// its three terms are per-App counts, and there was no mechanism to compare
    /// less than all of it.
    ///
    /// ⚠ `SimId` is in the same position, and it is the type both provenance
    /// defects in this campaign travelled through.
    fn rollback_component_canonical_checksum<T>(
        &mut self,
        _owner: &'static str,
        name: &'static str,
        _detail: &'static str,
        _projection: fn(&T) -> u64,
    ) -> &mut Self
    where
        T: bevy_ecs::component::Component<Mutability = bevy_ecs::component::Mutable>
            + SnapshotState,
    {
        panic!(
            "RollbackRegistrar does not support rollback_component_canonical_checksum for {name}"
        )
    }

    fn rollback_component_cursor<T>(
        &mut self,
        _owner: &'static str,
        name: &'static str,
    ) -> &mut Self
    where
        T: bevy_ecs::component::Component<Mutability = bevy_ecs::component::Mutable>
            + Clone
            + SnapshotCursor,
    {
        panic!("RollbackRegistrar does not support rollback_component_cursor for {name}")
    }

    fn rollback_component_resolved<T>(
        &mut self,
        _owner: &'static str,
        name: &'static str,
    ) -> &mut Self
    where
        T: bevy_ecs::component::Component<Mutability = bevy_ecs::component::Mutable>
            + Clone
            + SnapshotResolve,
    {
        panic!("RollbackRegistrar does not support rollback_component_resolved for {name}")
    }

    fn rollback_component_clone<T>(&mut self, _owner: &'static str, name: &'static str) -> &mut Self
    where
        T: bevy_ecs::component::Component<Mutability = bevy_ecs::component::Mutable> + Clone,
    {
        panic!("RollbackRegistrar does not support rollback_component_clone for {name}")
    }

    fn rollback_component_clone_entity_ref<T>(
        &mut self,
        _owner: &'static str,
        name: &'static str,
        _referenced: fn(&T) -> bevy_ecs::entity::Entity,
    ) -> &mut Self
    where
        T: bevy_ecs::component::Component<Mutability = bevy_ecs::component::Mutable> + Clone,
    {
        panic!("RollbackRegistrar does not support rollback_component_clone_entity_ref for {name}")
    }

    fn rollback_component_clone_entity_set<T>(
        &mut self,
        _owner: &'static str,
        name: &'static str,
        _referenced: fn(&T) -> Vec<bevy_ecs::entity::Entity>,
    ) -> &mut Self
    where
        T: bevy_ecs::component::Component<Mutability = bevy_ecs::component::Mutable> + Clone,
    {
        panic!("RollbackRegistrar does not support rollback_component_clone_entity_set for {name}")
    }

    fn rollback_component_clone_entity_map<T>(
        &mut self,
        _owner: &'static str,
        name: &'static str,
        _referenced: fn(&T) -> Vec<(u64, bevy_ecs::entity::Entity)>,
    ) -> &mut Self
    where
        T: bevy_ecs::component::Component<Mutability = bevy_ecs::component::Mutable> + Clone,
    {
        panic!("RollbackRegistrar does not support rollback_component_clone_entity_map for {name}")
    }

    fn rollback_component_clone_probed<T>(
        &mut self,
        _owner: &'static str,
        name: &'static str,
        _projection: fn(&T) -> u64,
    ) -> &mut Self
    where
        T: bevy_ecs::component::Component<Mutability = bevy_ecs::component::Mutable> + Clone,
    {
        panic!("RollbackRegistrar does not support rollback_component_clone_probed for {name}")
    }

    fn rollback_component_clone_state<T>(
        &mut self,
        _owner: &'static str,
        name: &'static str,
    ) -> &mut Self
    where
        T: bevy_ecs::component::Component<Mutability = bevy_ecs::component::Mutable>
            + Clone
            + SnapshotState,
    {
        panic!("RollbackRegistrar does not support rollback_component_clone_state for {name}")
    }

    /// Clone-snapshot a component and checksum the domain projection.
    ///
    /// `projection` describes only what the checksum sees. The backend owns the
    /// storage half of the schema detail, so a domain never has to name GGRS.
    fn rollback_component_clone_checksum<T>(
        &mut self,
        _owner: &'static str,
        name: &'static str,
        _projection: &'static str,
        _checksum: for<'a> fn(&'a T) -> u64,
    ) -> &mut Self
    where
        T: bevy_ecs::component::Component<Mutability = bevy_ecs::component::Mutable> + Clone,
    {
        panic!("RollbackRegistrar does not support rollback_component_clone_checksum for {name}")
    }

    /// Clone-snapshot a component and checksum a domain projection while preserving
    /// an exact, domain-owned schema description.
    ///
    /// Unlike [`Self::rollback_component_clone_checksum`], `detail` is already the
    /// complete schema detail and is recorded verbatim. Use this when the stable
    /// schema identity intentionally owns its prose rather than composing a
    /// backend storage description with a projection description.
    fn rollback_component_clone_checksum_with_schema_detail<T>(
        &mut self,
        _owner: &'static str,
        name: &'static str,
        _detail: &'static str,
        _checksum: for<'a> fn(&'a T) -> u64,
    ) -> &mut Self
    where
        T: bevy_ecs::component::Component<Mutability = bevy_ecs::component::Mutable> + Clone,
    {
        panic!(
            "RollbackRegistrar does not support rollback_component_clone_checksum_with_schema_detail for {name}"
        )
    }

    fn rollback_resource_canonical<T>(
        &mut self,
        _owner: &'static str,
        name: &'static str,
    ) -> &mut Self
    where
        T: bevy_ecs::resource::Resource<Mutability = bevy_ecs::component::Mutable> + SnapshotState,
    {
        panic!("RollbackRegistrar does not support rollback_resource_canonical for {name}")
    }

    fn rollback_resource_optional_canonical<T>(
        &mut self,
        _owner: &'static str,
        name: &'static str,
    ) -> &mut Self
    where
        T: bevy_ecs::resource::Resource<Mutability = bevy_ecs::component::Mutable> + SnapshotState,
    {
        panic!("RollbackRegistrar does not support rollback_resource_optional_canonical for {name}")
    }

    /// Canonical snapshot whose CHECKSUM is a stated projection of the value.
    ///
    /// For an always-present resource that must rewind intact while holding a
    /// field two peers cannot agree on.
    fn rollback_resource_canonical_checksum<T>(
        &mut self,
        _owner: &'static str,
        name: &'static str,
        _detail: &'static str,
        _projection: fn(&T) -> u64,
    ) -> &mut Self
    where
        T: bevy_ecs::resource::Resource<Mutability = bevy_ecs::component::Mutable> + SnapshotState,
    {
        panic!("RollbackRegistrar does not support rollback_resource_canonical_checksum for {name}")
    }

    /// Presence-aware canonical snapshot whose CHECKSUM is a stated projection
    /// of the value rather than the whole encoding — for a resource that must
    /// rewind intact while holding a field two peers cannot agree on.
    fn rollback_resource_optional_canonical_checksum<T>(
        &mut self,
        _owner: &'static str,
        name: &'static str,
        _detail: &'static str,
        _projection: fn(&T) -> u64,
    ) -> &mut Self
    where
        T: bevy_ecs::resource::Resource<Mutability = bevy_ecs::component::Mutable> + SnapshotState,
    {
        panic!(
            "RollbackRegistrar does not support \
             rollback_resource_optional_canonical_checksum for {name}"
        )
    }

    fn rollback_resource_clone<T>(&mut self, _owner: &'static str, name: &'static str) -> &mut Self
    where
        T: bevy_ecs::resource::Resource<Mutability = bevy_ecs::component::Mutable> + Clone,
    {
        panic!("RollbackRegistrar does not support rollback_resource_clone for {name}")
    }

    fn rollback_resource_clone_entity_set<T>(
        &mut self,
        _owner: &'static str,
        name: &'static str,
        _referenced: fn(&T) -> Vec<bevy_ecs::entity::Entity>,
    ) -> &mut Self
    where
        T: bevy_ecs::resource::Resource<Mutability = bevy_ecs::component::Mutable> + Clone,
    {
        panic!("RollbackRegistrar does not support rollback_resource_clone_entity_set for {name}")
    }

    fn rollback_resource_clone_entity_set_probed<T>(
        &mut self,
        _owner: &'static str,
        name: &'static str,
        _referenced: fn(&T) -> Vec<bevy_ecs::entity::Entity>,
        _facts: fn(&T) -> u64,
    ) -> &mut Self
    where
        T: bevy_ecs::resource::Resource<Mutability = bevy_ecs::component::Mutable> + Clone,
    {
        panic!("RollbackRegistrar does not support rollback_resource_clone_entity_set_probed for {name}")
    }

    /// Clone-snapshot a resource and checksum the domain projection.
    ///
    /// `projection` describes only what the checksum sees. The backend owns the
    /// storage half of the schema detail, so a domain never has to name GGRS.
    fn rollback_resource_clone_checksum<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
        projection: &'static str,
        checksum: for<'a> fn(&'a T) -> u64,
    ) -> &mut Self
    where
        T: bevy_ecs::resource::Resource<Mutability = bevy_ecs::component::Mutable> + Clone;

    /// Resource twin of
    /// [`Self::rollback_component_clone_checksum_with_schema_detail`].
    fn rollback_resource_clone_checksum_with_schema_detail<T>(
        &mut self,
        _owner: &'static str,
        name: &'static str,
        _detail: &'static str,
        _checksum: for<'a> fn(&'a T) -> u64,
    ) -> &mut Self
    where
        T: bevy_ecs::resource::Resource<Mutability = bevy_ecs::component::Mutable> + Clone,
    {
        panic!(
            "RollbackRegistrar does not support rollback_resource_clone_checksum_with_schema_detail for {name}"
        )
    }

    fn rollback_map_entities<T>(&mut self, _owner: &'static str, name: &'static str) -> &mut Self
    where
        T: bevy_ecs::component::Component<Mutability = bevy_ecs::component::Mutable>
            + bevy_ecs::entity::MapEntities,
    {
        panic!("RollbackRegistrar does not support rollback_map_entities for {name}")
    }

    fn rollback_resource_map_entities<T>(
        &mut self,
        _owner: &'static str,
        name: &'static str,
    ) -> &mut Self
    where
        T: bevy_ecs::resource::Resource<Mutability = bevy_ecs::component::Mutable>
            + bevy_ecs::entity::MapEntities,
    {
        panic!("RollbackRegistrar does not support rollback_resource_map_entities for {name}")
    }

    fn require_rollback<T>(&mut self, _owner: &'static str, name: &'static str) -> &mut Self
    where
        T: bevy_ecs::component::Component,
    {
        panic!("RollbackRegistrar does not support require_rollback for {name}")
    }

    fn clear_message_on_rollback<T>(
        &mut self,
        _owner: &'static str,
        name: &'static str,
    ) -> &mut Self
    where
        T: bevy_ecs::message::Message,
    {
        panic!("RollbackRegistrar does not support clear_message_on_rollback for {name}")
    }

    fn declare_rollback_derived_component<T>(
        &mut self,
        _owner: &'static str,
        name: &'static str,
        _reason: &'static str,
    ) -> &mut Self
    where
        T: bevy_ecs::component::Component,
    {
        panic!("RollbackRegistrar does not support declare_rollback_derived_component for {name}")
    }

    /// Declare derived component state with a canonical value projection for
    /// restore localization. The backend may use the projection for diagnostics
    /// without snapshotting the derived value itself.
    fn declare_rollback_derived_component_state<T>(
        &mut self,
        _owner: &'static str,
        name: &'static str,
        _reason: &'static str,
    ) -> &mut Self
    where
        T: bevy_ecs::component::Component + SnapshotState,
    {
        panic!("RollbackRegistrar does not support declare_rollback_derived_component_state for {name}")
    }

    fn declare_rollback_derived_resource<T>(
        &mut self,
        _owner: &'static str,
        name: &'static str,
        _reason: &'static str,
    ) -> &mut Self
    where
        T: bevy_ecs::resource::Resource,
    {
        panic!("RollbackRegistrar does not support declare_rollback_derived_resource for {name}")
    }

    /// Resource twin of [`Self::declare_rollback_derived_component_state`].
    fn declare_rollback_derived_resource_state<T>(
        &mut self,
        _owner: &'static str,
        name: &'static str,
        _reason: &'static str,
    ) -> &mut Self
    where
        T: bevy_ecs::resource::Resource + SnapshotState,
    {
        panic!(
            "RollbackRegistrar does not support declare_rollback_derived_resource_state for {name}"
        )
    }

    fn declare_dynamic_anchor<T>(
        &mut self,
        _owner: &'static str,
        name: &'static str,
        _detail: &'static str,
    ) -> &mut Self
    where
        T: 'static,
    {
        panic!("RollbackRegistrar does not support declare_dynamic_anchor for {name}")
    }
}

#[cfg(test)]
mod rollback_registrar_default_method_tests {
    use super::RollbackRegistrar;

    // Bevy 0.19: `Resource: Component`, and only the derive can emit both.
    #[derive(Clone, bevy_ecs::resource::Resource)]
    struct DummyResource;

    struct CapturingRegistrar {
        called: bool,
    }

    impl RollbackRegistrar for CapturingRegistrar {
        fn rollback_resource_clone_checksum<T>(
            &mut self,
            _owner: &'static str,
            _name: &'static str,
            _projection: &'static str,
            _checksum: for<'a> fn(&'a T) -> u64,
        ) -> &mut Self
        where
            T: bevy_ecs::resource::Resource<Mutability = bevy_ecs::component::Mutable> + Clone,
        {
            self.called = true;
            self
        }
    }

    fn dummy_checksum(_: &DummyResource) -> u64 {
        0
    }

    #[test]
    fn a_narrow_registrar_only_implements_the_operation_it_captures() {
        let mut registrar = CapturingRegistrar { called: false };
        registrar.rollback_resource_clone_checksum::<DummyResource>(
            "test",
            "resource.dummy",
            "dummy projection",
            dummy_checksum,
        );
        assert!(registrar.called);
    }
}

#[cfg(test)]
mod peer_digest_tests {
    use super::PeerDigest;

    /// ⛔⛔ **THE TWO RULES THE TYPE EXISTS TO MAKE STRUCTURAL.**
    ///
    /// Both were previously written longhand at every call site, which is why
    /// one site got each of them wrong.
    #[test]
    fn a_domain_separates_identical_payloads_and_absent_is_not_zero() {
        // RULE 1: the same numbers in two domains are two digests. Four of the
        // six spellings this replaced carried no domain, so a verdict digest and
        // a seat-count digest over the same integer were the same value.
        assert_ne!(
            PeerDigest::in_domain("match.verdict").u64(7).finish(),
            PeerDigest::in_domain("match.seats").u64(7).finish(),
            "the domain does not reach the digest, so two projections over the \
             same numbers collide"
        );
        // ⚠ AND A DOMAIN THAT IS A PREFIX OF ANOTHER MUST STILL SEPARATE. A
        // bare concatenation would make domain "ab" + field 1 collide with
        // domain "a" + field b1; the length prefix is what prevents it.
        assert_ne!(
            PeerDigest::in_domain("match").u64(1).finish(),
            PeerDigest::in_domain("match.a").u64(1).finish(),
            "a domain that is a prefix of another collides with it"
        );

        // RULE 2: absent is not zero. `SessionMatchOrdinal` returned a bare
        // `next`, so an unclaimed mint and a session with zero matches were the
        // same answer.
        assert_ne!(
            PeerDigest::in_domain("d").opt_u64(None).finish(),
            PeerDigest::in_domain("d").opt_u64(Some(0)).finish(),
            "an absent optional projects as zero"
        );
        assert_ne!(
            PeerDigest::in_domain("d").opt_u64(Some(0)).finish(),
            PeerDigest::in_domain("d").opt_u64(Some(1)).finish(),
            "the optional's VALUE does not reach the digest"
        );
    }

    /// ⛔ FIELD ORDER AND FIELD BOUNDARIES BOTH MATTER, or a projection that
    /// added a field would silently agree with one that reordered two.
    #[test]
    fn two_fields_cannot_be_re_split_or_reordered() {
        assert_ne!(
            PeerDigest::in_domain("d").u64(1).u64(2).finish(),
            PeerDigest::in_domain("d").u64(2).u64(1).finish(),
            "field order does not reach the digest"
        );
        // ⚠ THE LENGTH PREFIX ON `bytes` IS LOAD-BEARING. Without it "ab" then
        // "c" and "a" then "bc" are the same byte stream — which is exactly how
        // an `escape_segment`-free identity string collides.
        assert_ne!(
            PeerDigest::in_domain("d").bytes(b"ab").bytes(b"c").finish(),
            PeerDigest::in_domain("d").bytes(b"a").bytes(b"bc").finish(),
            "adjacent byte fields can be re-split, so two different pairs agree"
        );
    }

    /// ⚠ AND IT MUST BE DETERMINISTIC ACROSS CALLS, since two peers compute it
    /// in separate processes. `StateHasher` is a fixed-seed FNV-1a, not
    /// `DefaultHasher`, whose seed is randomised per process — a projection built
    /// on that would desync every pair of peers and pass every single-process
    /// test.
    #[test]
    fn the_same_projection_twice_is_the_same_digest() {
        let build = || {
            PeerDigest::in_domain("match.clock")
                .opt_u64(Some(3))
                .u64(50_000)
                .bool(true)
                .finish()
        };
        assert_eq!(build(), build());
    }
}
