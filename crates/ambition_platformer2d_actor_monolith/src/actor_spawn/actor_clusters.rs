//! The per-tick view of an actor: [`ActorMut`], the borrow the integration
//! mutates in place, and the query that assembles it from live components.
//!
//! The body a construction path spawns from is [`ambition_body_seed::ActorClusterSeed`];
//! this module binds that seed to the simulation ([`SeedActorMut`]) and owns
//! nothing about what a body IS.

use ambition_body_seed::{ActorClusterSeed, ActorMotionPath};
use ambition_characters::actor::ai::ActorStatus;
use ambition_combat::actor_tuning::ActorConfig;
use ambition_platformer2d_core::BodyKinematics;
use bevy::ecs::query::QueryData;

use ambition_combat::components::BodyMelee;
use ambition_platformer2d_core as ae;
use ambition_platformer2d_core::body_clusters::ActorSurfaceState;
use ambition_platformer2d_shared_tangle::body::SpawnBaseline;

use ambition_platformer2d_core::{
    BodyAbilities, BodyActionBuffer, BodyBaseSize, BodyBlinkState, BodyComboTrace, BodyDashState,
    BodyDodgeState, BodyEnvironmentContact, BodyFlightState, BodyGroundState, BodyJumpState,
    BodyLedgeState, BodyLifetime, BodyMana, BodyModeState, BodyOffense, BodyShieldState,
    BodyWallState,
};

/// Deliberately shorter than the player's attack cadence (~0.4 s swipe) so it never eats an
/// intended combo hit, yet long enough to collapse a 60 fps contact/overlap stream to a single hit
/// per window. Feel-tunable.
pub const ACTOR_DAMAGE_IFRAME_S: f32 = 0.2;

/// Mutable actor components assembled into the same [`ae::BodyClustersMut`]
/// movement view used by player-controlled bodies.
pub struct ActorMut<'a> {
    pub kin: &'a mut BodyKinematics,
    /// §3.1 motion record (optional only for owned scratch tests — ECS-spawned
    /// bodies carry the real component); the shared pipeline writes it via
    /// `clusters_mut()`, the surface-walker branch writes it directly around
    /// its own step.
    pub sweep: Option<&'a mut ae::SweepSample>,
    pub status: &'a mut ActorStatus,
    /// The body's shared health (the one `BodyHealth` component every actor
    /// carries) — the authoritative HP the damage / respawn / banter paths use.
    pub health: &'a mut ambition_characters::actor::BodyHealth,
    pub surface: &'a mut ActorSurfaceState,
    pub attack: &'a mut BodyMelee,
    pub config: &'a mut ActorConfig,
    pub spawn: &'a mut SpawnBaseline,
    pub motion: &'a mut ActorMotionPath,
    /// Spawn-resolved special-behavior flags (kit vocabulary). Read-only:
    /// the per-frame integration and the damage hook branch on these
    /// instead of calling back into the named archetype enum.
    pub caps: &'a ambition_combat::CombatCapabilities,
    /// The body's live held item, if it has one. See the query member.
    pub held_item: Option<&'a ambition_combat::held_items::HeldItem>,
    // ── The 18 ancillary movement clusters (real components) ──
    pub abilities: &'a BodyAbilities,
    pub base_size: &'a mut BodyBaseSize,
    pub ground: &'a mut BodyGroundState,
    pub wall: &'a mut BodyWallState,
    pub jump: &'a mut BodyJumpState,
    pub dash: &'a mut BodyDashState,
    pub flight: &'a mut BodyFlightState,
    pub blink: &'a mut BodyBlinkState,
    pub ledge: &'a mut BodyLedgeState,
    pub dodge: &'a mut BodyDodgeState,
    pub shield: &'a mut BodyShieldState,
    pub body_mode: &'a mut BodyModeState,
    pub env_contact: &'a mut BodyEnvironmentContact,
    pub mana: &'a mut BodyMana,
    pub offense: &'a mut BodyOffense,
    pub action_buffer: &'a mut BodyActionBuffer,
    pub lifetime: &'a mut BodyLifetime,
    pub combo_trace: &'a mut BodyComboTrace,
}

impl<'a> ActorMut<'a> {
    /// Borrow `kin` + the 18 ancillary clusters as the shared
    /// [`ae::BodyClustersMut`] view the movement pipeline consumes — the exact
    /// aggregate the player builds, so the actor runs the identical code.
    pub fn clusters_mut(&mut self) -> ae::BodyClustersMut<'_> {
        ae::BodyClustersMut {
            kinematics: &mut *self.kin,
            sweep: self.sweep.as_deref_mut(),
            abilities: &*self.abilities,
            base_size: &mut *self.base_size,
            ground: &mut *self.ground,
            wall: &mut *self.wall,
            jump: &mut *self.jump,
            dash: &mut *self.dash,
            flight: &mut *self.flight,
            blink: &mut *self.blink,
            ledge: &mut *self.ledge,
            dodge: &mut *self.dodge,
            shield: &mut *self.shield,
            body_mode: &mut *self.body_mode,
            env_contact: &mut *self.env_contact,
            mana: &mut *self.mana,
            offense: &mut *self.offense,
            action_buffer: &mut *self.action_buffer,
            lifetime: &mut *self.lifetime,
            combo_trace: &mut *self.combo_trace,
        }
    }
}

#[derive(QueryData)]
#[query_data(mutable)]
pub struct ActorClusterQueryData {
    pub kin: &'static mut BodyKinematics,
    /// §3.1 motion record. Runtime actor/boss entities are spawned with the
    /// shared [`ambition_platformer2d_shared_tangle::body::AncillaryMovementBundle`], so this is required at
    /// the ECS query seam; only the owned scratch harness keeps an optional slot.
    pub sweep: &'static mut ae::SweepSample,
    pub status: &'static mut ActorStatus,
    pub health: &'static mut ambition_characters::actor::BodyHealth,
    pub surface: &'static mut ActorSurfaceState,
    pub attack: &'static mut BodyMelee,
    pub config: &'static mut ActorConfig,
    /// The body a reset hands back. Read by `reset_to_spawn`; the mount
    /// dismount reads the same component directly rather than through this view.
    pub spawn: &'static mut SpawnBaseline,
    pub motion: &'static mut ActorMotionPath,
    pub caps: &'static ambition_combat::CombatCapabilities,
    /// What this body is holding RIGHT NOW, if anything.
    ///
    /// `Option` because most bodies hold nothing, and because an OPTIONAL query
    /// member cannot silently filter a body out of the cluster the way a
    /// required one can. Read by the death path so a defeated body drops the
    /// weapon it actually has rather than the one its archetype was authored
    /// with.
    pub held_item: Option<&'static ambition_combat::held_items::HeldItem>,
    pub abilities: &'static BodyAbilities,
    pub base_size: &'static mut BodyBaseSize,
    pub ground: &'static mut BodyGroundState,
    pub wall: &'static mut BodyWallState,
    pub jump: &'static mut BodyJumpState,
    pub dash: &'static mut BodyDashState,
    pub flight: &'static mut BodyFlightState,
    pub blink: &'static mut BodyBlinkState,
    pub ledge: &'static mut BodyLedgeState,
    pub dodge: &'static mut BodyDodgeState,
    pub shield: &'static mut BodyShieldState,
    pub body_mode: &'static mut BodyModeState,
    pub env_contact: &'static mut BodyEnvironmentContact,
    pub mana: &'static mut BodyMana,
    pub offense: &'static mut BodyOffense,
    pub action_buffer: &'static mut BodyActionBuffer,
    pub lifetime: &'static mut BodyLifetime,
    pub combo_trace: &'static mut BodyComboTrace,
}

impl<'w, 's> ActorClusterQueryDataItem<'w, 's> {
    /// Borrow the components as an [`ActorMut`] view for one tick.
    pub fn as_actor_mut<'a>(&'a mut self) -> ActorMut<'a>
    where
        'w: 'a,
        's: 'a,
    {
        ActorMut {
            kin: &mut self.kin,
            sweep: Some(&mut *self.sweep),
            status: &mut self.status,
            health: &mut self.health,
            surface: &mut self.surface,
            attack: &mut self.attack,
            config: &mut self.config,
            spawn: &mut self.spawn,
            motion: &mut self.motion,
            caps: self.caps,
            held_item: self.held_item,
            abilities: &*self.abilities,
            base_size: &mut self.base_size,
            ground: &mut self.ground,
            wall: &mut self.wall,
            jump: &mut self.jump,
            dash: &mut self.dash,
            flight: &mut self.flight,
            blink: &mut self.blink,
            ledge: &mut self.ledge,
            dodge: &mut self.dodge,
            shield: &mut self.shield,
            body_mode: &mut self.body_mode,
            env_contact: &mut self.env_contact,
            mana: &mut self.mana,
            offense: &mut self.offense,
            action_buffer: &mut self.action_buffer,
            lifetime: &mut self.lifetime,
            combo_trace: &mut self.combo_trace,
        }
    }
}

/// The kernel's side of the seed: drive a not-yet-spawned
/// [`ActorClusterSeed`] through the same integration a live entity gets.
///
/// A trait rather than inherent methods because the seed is a foreign type now
/// — it names no simulation, and the simulation is what these two bind it to.
pub trait SeedActorMut {
    /// Borrow the seed's fields (and the scratch's 18 ancillary clusters) as an
    /// [`ActorMut`] view, for the test / pre-spawn paths that drive the
    /// integration without a live ECS entity. The runtime path borrows the SAME
    /// view from real components via [`ActorClusterQueryDataItem::as_actor_mut`].
    fn as_actor_mut(&mut self) -> ActorMut<'_>;

    /// One integration tick over the seed, with every optional input defaulted.
    #[cfg(test)]
    #[allow(clippy::too_many_arguments)]
    fn update_for_test(
        &mut self,
        world: &ae::World,
        target_pos: ae::Vec2,
        tuning: ambition_combat::FeatureCombatTuning,
        dt: f32,
        is_mounted: bool,
        frame: ambition_characters::actor::control::ActorControlFrame,
        motion_model: &mut ambition_platformer2d_core::movement::MotionModel,
        motion_frame: ae::MotionFrame,
    ) -> ambition_characters::actor::control::ActorControlFrame;
}

impl SeedActorMut for ActorClusterSeed {
    fn as_actor_mut(&mut self) -> ActorMut<'_> {
        let body = &mut self.body.0;
        ActorMut {
            kin: &mut self.kin,
            // The seed is the non-ECS pre-spawn/test scratchpad; like
            // `BodyClusterScratch` it carries no motion record (spawned
            // bodies get theirs from `AncillaryMovementBundle`).
            sweep: None,
            status: &mut self.status,
            health: &mut self.health,
            surface: &mut self.surface,
            attack: &mut self.attack,
            config: &mut self.config,
            spawn: &mut self.spawn,
            motion: &mut self.motion,
            caps: &self.caps,
            // A seed is pre-spawn scratch: nothing is holding anything yet.
            held_item: None,
            abilities: &body.abilities,
            base_size: &mut body.base_size,
            ground: &mut body.ground,
            wall: &mut body.wall,
            jump: &mut body.jump,
            dash: &mut body.dash,
            flight: &mut body.flight,
            blink: &mut body.blink,
            ledge: &mut body.ledge,
            dodge: &mut body.dodge,
            shield: &mut body.shield,
            body_mode: &mut body.body_mode,
            env_contact: &mut body.env_contact,
            mana: &mut body.mana,
            offense: &mut body.offense,
            action_buffer: &mut body.action_buffer,
            lifetime: &mut body.lifetime,
            combo_trace: &mut body.combo_trace,
        }
    }

    #[cfg(test)]
    #[allow(clippy::too_many_arguments)]
    fn update_for_test(
        &mut self,
        world: &ae::World,
        target_pos: ae::Vec2,
        tuning: ambition_combat::FeatureCombatTuning,
        dt: f32,
        is_mounted: bool,
        frame: ambition_characters::actor::control::ActorControlFrame,
        motion_model: &mut ambition_platformer2d_core::movement::MotionModel,
        motion_frame: ae::MotionFrame,
    ) -> ambition_characters::actor::control::ActorControlFrame {
        self.as_actor_mut()
            .update(
                world,
                target_pos,
                tuning,
                dt,
                is_mounted,
                frame,
                motion_model,
                motion_frame,
                // No move playing on a scratch rig, so it is never helpless.
                None,
                ambition_combat::feel::Platformer2dFeelTuningMonolith::default(),
                None,
                &mut ambition_characters::actor::BodyCombat::default(),
                // A single-body rig: nobody to be solid to.
                // Not tumbling — a scratch harness body is not in a floor game.
                false,
                // In play — a scratch rig has no death window open.
                false,
                ae::BodyContactField::NONE,
            )
            .0
    }
}
