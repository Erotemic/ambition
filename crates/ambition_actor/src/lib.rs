//! The unified actor system (Stage 22 extraction).
//!
//! One crate for everything that makes an entity a controllable actor:
//!
//! - [`actor`] — the control-frame vocabulary (`ActorControl`,
//!   `ActorControlFrame`), AI state components, pose/faction
//!   vocabulary, and the character catalog (id → display name, sprite
//!   tuning, default brain/action-set presets).
//! - [`boss_encounter`] — the boss-encounter phase state machine +
//!   spec schema (the game's named roster stays game-side).
//! - [`brain`] — the universal brain: every controllable entity
//!   (player, NPC, enemy, boss) carries `Brain` + `ActionSet` and is
//!   driven through one dispatch. Includes the boss attack-pattern
//!   schedule + `BossEncounterPhase` (bosses ARE actors, ADR 0016).
//!
//! Content-free by construction: attack profiles are behavior
//! vocabulary (`HandSlam`, `DebrisRain`, ...), never boss names; the
//! named world lives in `ambition_content` / the machinery lib above.
//! `ambition_sandbox` re-exports both modules at their historical
//! `crate::actor` / `crate::brain` paths.

pub mod actor;
pub mod boss_encounter;
pub mod brain;
