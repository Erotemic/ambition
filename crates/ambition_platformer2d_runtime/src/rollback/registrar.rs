//! **The GGRS side of the domain-owned registration seam.**
//!
//! [`AmbitionRollbackApp`](super::AmbitionRollbackApp) is a typed façade over
//! `bevy_ggrs`, and it works — but it is an extension trait on `App`, declared
//! in THIS crate, so only crates that may depend on this crate can speak it. Every
//! domain that sits BELOW the runtime therefore had its registration line hoisted
//! up here, and the runtime accumulated a census of gameplay types it otherwise
//! knows nothing about.
//!
//! ⛔ **the reason given for that was that `bevy_ggrs` 0.21 registration is
//! generic over the concrete type, with no `TypeId`-keyed path — so something
//! must monomorphize it, and only this crate may name `bevy_ggrs`.** Both halves
//! are still true. Neither implies the list of types lives here: the
//! monomorphizing call site can be a TRAIT METHOD the domain invokes.
//!
//! ⭐ so the vocabulary moved to the floor
//! ([`ambition_platformer2d_core::snapshot::RollbackRegistrar`]) where a domain
//! can name it, and this file is the half that could not move — the implementor
//! that names `bevy_ggrs`. `bevy_ggrs` stays sequestered in this crate and
//! `ambition_app`; the domain gains no dependency at all.
//!
//! ⚠ **it is a WRAPPER around `&mut App`, and it has to be.** The trait is
//! foreign (it lives in the floor) and `bevy_app::App` is foreign, so
//! `impl RollbackRegistrar for App` in this crate is an orphan-rule violation:
//! `error[E0117]: only traits defined in the current crate can be implemented for
//! types defined outside of the crate`. A local newtype is the fix, and it is not
//! a workaround — it is a place to put the host's own registration policy later.
//!
//! ⛔ **this must never grow a list.** A registrar that carried a table of the
//! domains it registers would be the same census in a new file. The host calls
//! each domain's `register_*_rollback_state`, and the domain names its own types.

use bevy::prelude::App;
use bevy::prelude::Resource;

use ambition_platformer2d_core::snapshot::RollbackRegistrar;

/// A `bevy_ggrs`-backed [`RollbackRegistrar`], borrowed from the host's `App` for
/// the duration of one registration pass.
///
/// ⚠ borrowed rather than owned so the host can hand it to a domain mid-build
/// and keep using the `App` afterwards; the borrow ends with the pass.
pub struct GgrsRollbackRegistrar<'a> {
    app: &'a mut App,
}

impl<'a> GgrsRollbackRegistrar<'a> {
    /// Borrow `app` as a registrar a domain can register itself against.
    pub fn new(app: &'a mut App) -> Self {
        Self { app }
    }
}

impl RollbackRegistrar for GgrsRollbackRegistrar<'_> {
    fn rollback_resource_clone_checksum<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
        projection: &'static str,
        checksum: for<'b> fn(&'b T) -> u64,
    ) -> &mut Self
    where
        T: Resource + Clone,
    {
        // ⚠ **the recorded `detail` is composed, not quoted.** The domain says
        // what the checksum SEES; this side says how the value is STORED, which
        // is the only half that names a backend. Joined, it is byte-identical to
        // what the schema baseline already records — the seam moves the caller,
        // not the wire form.
        super::registry::install_resource_clone_checksum::<T>(
            self.app,
            owner,
            name,
            format!("bevy_ggrs clone snapshot + {projection}"),
            checksum,
        );
        self
    }
}
