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
//! Slice 0 re-homed [`BodyKinematics`] (the single position / velocity / size /
//! facing component the player, enemies, NPCs, and bosses all share). Slice 0b
//! re-homes the entity markers [`PlayerEntity`] / [`PrimaryPlayer`] (already
//! foundation types) + the [`PrimaryPlayerOnly`] filter. Subsequent slices move
//! the combat/economy sim-state here.

use bevy::prelude::With;

pub use crate::platformer_runtime::body::BodyKinematics;
pub use ambition_platformer_primitives::markers::{PlayerEntity, PrimaryPlayer};

/// The shared **movement-cluster components** every body carries — the 18
/// ancillary clusters (ground contact, wall, jump, dash, flight, blink, ledge,
/// dodge, shield, body-mode, environment contact, mana, offense, action buffer,
/// lifetime, combo trace, base size, ability mask) that, together with
/// [`BodyKinematics`], form the authoritative movement aggregate the shared
/// pipeline (`ae::update_body_with_tuning_clusters`) reads and writes.
///
/// These were historically named `Player*` and surfaced through `crate::player`,
/// which made every non-player module that names a body component import the
/// player. They are not player-specific — enemies, NPCs, and bosses all carry
/// them — so they are re-homed here on the neutral actor vocabulary under the
/// `Body*` convention (matching [`BodyKinematics`] / `BodyHealth` /
/// `BodyCombat`, which live on `ambition_characters::actor::body`). The types
/// `#[derive(Component)]` in `ambition_engine_core`; this is the single import
/// surface for them.
pub use ambition_engine_core::{
    BodyAbilities, BodyActionBuffer, BodyBaseSize, BodyBlinkState, BodyComboTrace, BodyDashState,
    BodyDodgeState, BodyEnvironmentContact, BodyFlightState, BodyGroundState, BodyJumpState,
    BodyLedgeState, BodyLifetime, BodyMana, BodyModeState, BodyOffense, BodyShieldState,
    BodyWallState,
};

/// The 18 **ancillary movement clusters** every body spawns with as real ECS
/// components — everything in the movement aggregate EXCEPT [`BodyKinematics`]
/// (the shared kinematic truth, spawned as its own component so rendering /
/// gravity / targeting can read one position without the movement set).
///
/// This is the single spawn surface for the ancillary clusters, nested by BOTH
/// the player (`PlayerSimulationBundle`) and every actor
/// (`ActorClusterSeed::into_components`). Carrying the identical real components
/// on both is what lets one query ([`ambition_engine_core::BodyClusterQueryData`])
/// — and ultimately one movement driver — serve the player and the actors alike,
/// instead of the actor wrapping them in a non-ECS scratch blob.
#[derive(bevy::prelude::Bundle)]
pub struct AncillaryMovementBundle {
    pub abilities: BodyAbilities,
    pub base_size: BodyBaseSize,
    pub ground: BodyGroundState,
    pub wall: BodyWallState,
    pub jump: BodyJumpState,
    pub dash: BodyDashState,
    pub flight: BodyFlightState,
    pub blink: BodyBlinkState,
    pub ledge: BodyLedgeState,
    pub dodge: BodyDodgeState,
    pub shield: BodyShieldState,
    pub body_mode: BodyModeState,
    pub env_contact: BodyEnvironmentContact,
    pub mana: BodyMana,
    pub offense: BodyOffense,
    pub action_buffer: BodyActionBuffer,
    pub lifetime: BodyLifetime,
    pub combo_trace: BodyComboTrace,
}

impl AncillaryMovementBundle {
    /// Split the 18 ancillary clusters out of a [`BodyClusterScratch`],
    /// dropping its vestigial `kinematics` field (the body's authoritative
    /// [`BodyKinematics`] is spawned separately).
    pub fn from_scratch(scratch: ambition_engine_core::BodyClusterScratch) -> Self {
        let ambition_engine_core::BodyClusterScratch {
            abilities,
            kinematics: _,
            base_size,
            ground,
            wall,
            jump,
            dash,
            flight,
            blink,
            ledge,
            dodge,
            shield,
            body_mode,
            env_contact,
            mana,
            offense,
            action_buffer,
            lifetime,
            combo_trace,
        } = scratch;
        Self {
            abilities,
            base_size,
            ground,
            wall,
            jump,
            dash,
            flight,
            blink,
            ledge,
            dodge,
            shield,
            body_mode,
            env_contact,
            mana,
            offense,
            action_buffer,
            lifetime,
            combo_trace,
        }
    }
}

/// Query filter for the **home avatar** — `With<PlayerEntity>` + `With<PrimaryPlayer>`.
///
/// Use this ONLY for genuine home-body concerns (respawn, save sync, sandbox
/// reset, HUD/debug subject). It does NOT identify the currently CONTROLLED body:
/// during possession the controlled body is a different entity (the one carrying
/// `Brain::Player(PlayerSlot::PRIMARY)`). Systems that act on "whoever the player
/// is driving" — camera, portal viewer, abilities, melee — read the
/// `ControlledSubject` resource instead of this filter.
pub type PrimaryPlayerOnly = (With<PlayerEntity>, With<PrimaryPlayer>);
