//! The Flying Spaghetti Monster: a false god of noodles and meatballs.
//!
//! Its conductor — it flies the god and performs its moves — is the
//! procedural module `ambition_content_modules::fsm`, a conducted boss on the
//! extension host (`ambition.boss.conduct`). [`conductor`] is what the game
//! keeps here: the god's id, its birth, and a view of the module's record for
//! tests and inspectors. Its sauce — more of it the more damage it has taken —
//! is presentation, in `presentation::fsm_sauce`.

pub mod conductor;

#[cfg(test)]
pub(crate) mod appendages_reference_tests;
#[cfg(test)]
pub(crate) mod conductor_reference_tests;
#[cfg(test)]
mod fsm_parity_tests;

pub use conductor::{birth, conductor_of, FsmView, Move, FSM_ID};
