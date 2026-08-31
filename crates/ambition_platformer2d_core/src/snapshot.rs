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
