//! The authored key of a move that ends its own mover in a detonation.
//!
//! A move authors it on its timeline (`Effect((key: "detonate"))`); the
//! handler is `ambition_combat::detonate`. It takes no params: the blast is
//! the mover's own death trait (`explodes_on_death`), so a body that detonates
//! and a body that is killed leave the same blast.

/// The authored effect key.
pub const DETONATE: &str = "detonate";
