//! The Flying Spaghetti Monster: a false god of noodles and meatballs.
//!
//! [`conductor`] flies it and performs its moves; [`appendages`] are the
//! noodlings it sends after you once wounded. Its sauce — more of it the more
//! damage it has taken — is presentation, in `presentation::fsm_sauce`.

pub mod appendages;
pub mod conductor;

pub use conductor::{adopt_fsm, conduct_fsm, FsmConductor, Move};
