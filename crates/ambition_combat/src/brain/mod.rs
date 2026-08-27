//! THINKING THAT IS NOT THE FLOOR'S BUSINESS.
//!
//! ⭐⭐ D168's admission test, applied to brains: `ambition_characters` owns what
//! a character IS — its body, its catalog identity, the DATA every brain arm is
//! made of — and the layer above owns how it THINKS. The nine ordinary NPC arms
//! (patrol, wander, skirmish…) stay down there because a game with no fighters
//! still wants them; the platform-fighter stages do not.
//!
//! ⛔ WHAT COULD NOT COME: every type the `Brain` snapshot encoder reads is
//! pinned to `ambition_characters` by the orphan rule, since that crate owns
//! `Brain` and this one depends on it. `SmashCfg`, `SmashState` and `BroadMode`
//! are therefore still down there and named from here — the legal direction.
pub mod smash;
