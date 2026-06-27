//! The neutral **actor vocabulary** home for shared sim-state — the components
//! every actor carries, the player included.
//!
//! Establishing this module is step 4 (the keystone) of the unified-actors plan
//! (`docs/planning/engine/unified-actors.md` / `engine/architecture.md`): the
//! shared body/sim-state was historically surfaced through `crate::player`, which
//! made `crate::player` a universal dependency sink — ~20 of the non-player
//! modules imported it just to name a body component. Re-homing the shared types
//! here dissolves those back-edges so the runtime domains can extract into leaf
//! crates.
//!
//! **Rule:** new *shared* sim-state (state every actor has) lands here on the
//! actor vocabulary, never on a `Player*`-named component. Genuinely player-only
//! state (camera, HUD, device input, wallet) stays in `crate::player`.
//!
//! Slice 0 re-homes [`BodyKinematics`] (the single position / velocity / size /
//! facing component the player, enemies, NPCs, and bosses all share). Subsequent
//! slices move the entity markers and the combat/economy sim-state here.

pub use crate::platformer_runtime::body::BodyKinematics;
