//! The GGRS bridge over the floor's snapshot vocabulary.
//!
//! What is left is the part that could NOT move: `CanonicalCodecStrategy` implements
//! `bevy_ggrs::Strategy`, and `bevy_ggrs` is a patched fork. Pulling it into
//! `ambition_platformer2d_core` would put the fork underneath every domain crate — the floor's
//! whole value is that it depends on no workspace crate and only two Bevy subcrates, and that is
//! what makes it a place a domain can reach.
//!
//! The vocabulary is re-exported here so `rollback::*` consumers inside this
//! crate keep working; the trait is defined in the floor.

pub use ambition_platformer2d_core::snapshot::{
    checksum_bytes, cursor_checksum, decode_state, encode_state, put_bool, put_f32, put_i32,
    put_opt_str, put_str, put_u32, put_u64, put_u8, put_vec2, resolved_checksum, state_checksum,
    Reader, SnapshotCursor, SnapshotResolve, SnapshotState, StateHasher,
};

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
