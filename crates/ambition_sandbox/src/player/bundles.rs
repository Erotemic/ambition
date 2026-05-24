//! Player ECS spawn bundles.

use ambition_engine as ae;
use bevy::prelude::*;

use super::components::{
    ActivePlayerAttack, LocalPlayer, PlayerAnimState, PlayerBlinkCameraState, PlayerBody,
    PlayerCombatState, PlayerEntity, PlayerHealth, PlayerInputFrame, PlayerInteractionState,
    PlayerMovementAuthority, PlayerPlatformRideState, PlayerSafetyState, PlayerSlot, PrimaryPlayer,
};
use crate::brain::{ActionSet, ActorControl, Brain};
use crate::features::ActorFaction;

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
    pub local: LocalPlayer,
    pub authority: PlayerMovementAuthority,
    pub body: PlayerBody,
    pub health: PlayerHealth,
    pub combat: PlayerCombatState,
    pub interaction: PlayerInteractionState,
    pub anim: PlayerAnimState,
    pub blink_cam: PlayerBlinkCameraState,
    pub ride: PlayerPlatformRideState,
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
}

impl PlayerSimulationBundle {
    /// Build the canonical local-primary player bundle from an engine
    /// `Player` and initial `Health`. The result spawns with
    /// `PlayerSlot(0)`, `PrimaryPlayer`, and `LocalPlayer` — the
    /// single-player default.
    ///
    /// Future code that needs to spawn a second / guest / remote
    /// player should compose `PlayerIdentityBundle::new(PlayerSlot(n))`
    /// with the simulation components manually rather than calling
    /// this helper, since the second player should not inherit
    /// `PrimaryPlayer` and may not be `LocalPlayer`.
    pub fn new(player: ae::Player, health: ae::Health) -> Self {
        let action_set = default_player_action_set(player.abilities);
        let authority = PlayerMovementAuthority::new(player);
        let body = authority.body();
        let initial_safe_pos = authority.player.pos;
        Self {
            identity: PlayerIdentityBundle::new(PlayerSlot::PRIMARY),
            primary: PrimaryPlayer,
            local: LocalPlayer,
            authority,
            body,
            health: PlayerHealth::new(health),
            combat: PlayerCombatState::default(),
            interaction: PlayerInteractionState::default(),
            anim: PlayerAnimState::default(),
            blink_cam: PlayerBlinkCameraState::default(),
            ride: PlayerPlatformRideState::default(),
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
