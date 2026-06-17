//! Player ECS spawn bundles.

use crate::engine_core as ae;
use bevy::prelude::*;

use super::components::{
    ActivePlayerAttack, LocalPlayer, PlayerAnimState, PlayerBlinkCameraState, PlayerCombatState,
    PlayerEntity, PlayerHealth, PlayerInputFrame, PlayerInteractionState,
    PlayerSafetyState, PlayerSlot, PlayerWallet, PrimaryPlayer,
};
use super::movement_components::{
    BodyKinematics, PlayerAbilities, PlayerActionBuffer, PlayerBaseSize, PlayerBlinkState,
    PlayerBodyModeState, PlayerComboTrace, PlayerDashState, PlayerDodgeState,
    PlayerEnvironmentContact, PlayerFlightState, PlayerGroundState, PlayerJumpState,
    PlayerLedgeState, PlayerLifetime, PlayerMana, PlayerOffense, PlayerShieldState,
    PlayerWallState,
};
use crate::brain::{ActionSet, ActorControl, Brain};
use crate::features::{ActorFaction, ActorPose};

/// All simulation components required on the player entity.
///
/// Use this bundle in `commands.spawn()` together with presentation-side
/// components (`Transform`, `PlayerVisual`) so the spawn call documents
/// what simulation state the player entity carries. The bundle does not
/// include `Transform` or `Sprite` — those are presentation concerns.
/// Identity tag bundle: every player entity carries exactly these
/// components. Useful as a building block in tests that want to spawn
/// an additional player without rebuilding the full simulation bundle.
#[derive(Bundle)]
pub struct PlayerIdentityBundle {
    pub marker: PlayerEntity,
    pub slot: PlayerSlot,
}

impl PlayerIdentityBundle {
    pub fn new(slot: PlayerSlot) -> Self {
        Self {
            marker: PlayerEntity,
            slot,
        }
    }
}

#[derive(Bundle)]
pub struct PlayerSimulationBundle {
    pub identity: PlayerIdentityBundle,
    pub primary: PrimaryPlayer,
    /// Runtime-side marker (`ambition_platformer_primitives`) tagging this as the
    /// body whose position drives live gravity resolution. The gravity runtime
    /// queries `With<PrimaryBody>` instead of the sandbox's player markers, so
    /// the gravity layer stays content-free.
    pub primary_body: ambition_platformer_primitives::body::PrimaryBody,
    pub local: LocalPlayer,
    pub health: PlayerHealth,
    pub wallet: PlayerWallet,
    pub combat: PlayerCombatState,
    pub interaction: PlayerInteractionState,
    pub anim: PlayerAnimState,
    pub blink_cam: PlayerBlinkCameraState,
    pub attack: ActivePlayerAttack,
    pub safety: PlayerSafetyState,
    pub input: PlayerInputFrame,
    pub faction: ActorFaction,
    pub name: Name,
    /// Universal-brain seam. The player entity carries a
    /// `Brain::Player(slot)`, an `ActionSet` (its full moveset), and
    /// an `ActorControl` that the brain-driver system fills each
    /// frame from `PlayerInputFrame`. Until Chunk 4d/e wires the
    /// authority to consume the frame, the brain and control
    /// component are *parallel* state — they're built but nothing
    /// reads them yet.
    pub brain: Brain,
    pub action_set: ActionSet,
    pub actor_control: ActorControl,
    /// Capability marker: this body uses the chargeable-projectile (Fireball)
    /// ability. Gates `emit_player_projectile_tick_messages` by CAPABILITY rather
    /// than `brain.is_player()`, so possession of this body keeps the charge
    /// mechanic. Pay-for-use: actors without it never enter the charge stream.
    pub charges_projectiles: crate::brain::ChargesProjectiles,
    /// Gameplay-space action origin / facing read model shared with
    /// non-player actors. Synced from `BodyKinematics`, not from any
    /// presentation `Transform`.
    pub actor_pose: ActorPose,
    // The 18 player cluster components. Authoritative player state —
    // every engine entry point reads / writes them through
    // `PlayerClustersMut`. See `engine_core/player_clusters.rs` for
    // the per-cluster shape.
    pub abilities: PlayerAbilities,
    pub kinematics: BodyKinematics,
    pub base_size: PlayerBaseSize,
    pub ground: PlayerGroundState,
    pub wall: PlayerWallState,
    pub jump: PlayerJumpState,
    pub dash: PlayerDashState,
    pub flight: PlayerFlightState,
    pub blink: PlayerBlinkState,
    pub ledge: PlayerLedgeState,
    pub dodge: PlayerDodgeState,
    pub shield: PlayerShieldState,
    pub body_mode: PlayerBodyModeState,
    pub env_contact: PlayerEnvironmentContact,
    pub mana: PlayerMana,
    pub offense: PlayerOffense,
    pub action_buffer: PlayerActionBuffer,
    pub lifetime: PlayerLifetime,
    pub combo_trace: PlayerComboTrace,
    /// Per-player projectile state — spawner cooldowns, charge timer,
    /// motion-input buffer, in-flight body list. Was previously a
    /// global `Res<PlayerProjectileState>`; per-actor migration so
    /// co-op / possession builds get one independent set per player.
    pub projectile: crate::projectile::PlayerProjectileState,
}

impl PlayerSimulationBundle {
    /// Build the canonical local-primary player bundle from a
    /// `PlayerClusterScratch` and initial `Health`. The result spawns
    /// with `PlayerSlot(0)`, `PrimaryPlayer`, and `LocalPlayer` — the
    /// single-player default.
    ///
    /// Future code that needs to spawn a second / guest / remote
    /// player should compose `PlayerIdentityBundle::new(PlayerSlot(n))`
    /// with the simulation components manually rather than calling
    /// this helper, since the second player should not inherit
    /// `PrimaryPlayer` and may not be `LocalPlayer`.
    pub fn from_scratch(scratch: ae::PlayerClusterScratch, health: crate::actor::Health) -> Self {
        let action_set = default_player_action_set(scratch.abilities.abilities);
        let initial_safe_pos = scratch.kinematics.pos;
        let ae::PlayerClusterScratch {
            abilities,
            kinematics,
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
            identity: PlayerIdentityBundle::new(PlayerSlot::PRIMARY),
            primary: PrimaryPlayer,
            primary_body: ambition_platformer_primitives::body::PrimaryBody,
            local: LocalPlayer,
            health: PlayerHealth::new(health),
            wallet: PlayerWallet::default(),
            combat: PlayerCombatState::default(),
            interaction: PlayerInteractionState::default(),
            anim: PlayerAnimState::default(),
            blink_cam: PlayerBlinkCameraState::default(),
            attack: ActivePlayerAttack::default(),
            safety: PlayerSafetyState::new(initial_safe_pos),
            input: PlayerInputFrame::default(),
            faction: ActorFaction::Player,
            name: Name::new("Player"),
            brain: Brain::Player(PlayerSlot::PRIMARY),
            // Player ActionSet derived from the player's AbilitySet.
            // Today nothing reads it for combat effects —
            // update_player still spawns hitboxes via the existing
            // pipeline. The set lights up when the ActionSet
            // effect-resolver flip lands (daytime). Possession of a
            // non-player body keeps that body's ActionSet — this
            // default fires only for actual player entities.
            action_set,
            actor_control: ActorControl::default(),
            charges_projectiles: crate::brain::ChargesProjectiles,
            actor_pose: ActorPose::from_parts(
                kinematics.pos,
                kinematics.size * 0.5,
                kinematics.facing,
            ),
            abilities,
            kinematics,
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
            projectile: crate::projectile::PlayerProjectileState::default(),
        }
    }
}

/// Default player moveset derived from an `AbilitySet`:
/// - `melee = Some(Swipe)` iff `abilities.attack`
/// - `ranged = Some(Bolt)` unconditionally today (the existing
///   fireball / hadouken path is itself gated by `projectile`)
/// - `special = Some(BubbleShield)` iff `abilities.shield`
///
/// The resolver won't emit `ActionRequest`s for capabilities the
/// player doesn't have, so EFFECTS consumers can
/// read the ActionSet as the authoritative "what can this player
/// actually do right now" surface without re-checking AbilitySet.
fn default_player_action_set(abilities: ae::AbilitySet) -> ActionSet {
    use crate::brain::{
        MeleeActionSpec, MoveStyleSpec, RangedActionSpec, SpecialActionSpec, SwipeSpec,
    };
    ActionSet {
        melee: abilities
            .attack
            .then_some(MeleeActionSpec::Swipe(SwipeSpec {
                // Player swipe is faster than enemy Striker default
                // — the player's combat tempo runs ~2× snappier.
                windup_s: 0.12,
                active_s: 0.10,
                recover_s: 0.18,
                damage: 1,
                reach_px: 36.0,
            })),
        // The player's "ranged" today is the fireball / hadouken
        // path. There's no separate `projectile` ability in
        // AbilitySet — ranged is always available on the player.
        // If a future ability flag gates fireball, narrow this
        // slot the same way melee + special are.
        ranged: Some(RangedActionSpec::Bolt {
            speed: 600.0,
            damage: 1,
        }),
        move_style: MoveStyleSpec::Walk,
        // Special slot: BubbleShield, gated by the shield ability.
        // A possessed non-player body keeps that body's ActionSet so
        // this default fires only for actual player entities.
        special: abilities.shield.then_some(SpecialActionSpec::BubbleShield),
    }
}
