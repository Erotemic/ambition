//! Authoritative ECS components for the UNIFIED actor cluster (every actor —
//! was-NPC, was-enemy, encounter mob — shares this one cluster) + the `ActorMut`
//! view the per-tick integration mutates in place.
//!
//! Actor state lives as ECS components. Per-tick systems borrow those components
//! through [`ActorMut`] instead of rebuilding a runtime blob. Hostility is the
//! `ActorDisposition` state, not a cluster *type*.
//!
//! Field → component map:
//! - pos/vel/size/facing      → [`BodyKinematics`]
//! - on_ground/normal/gravity/air_jumps → [`ActorSurfaceState`] (component)
//! - attack windup/active/cooldown/axis → [`BodyMelee`] (component)
//! - respawn/hit_flash/ai_mode      → [`ActorStatus`] (liveness → [`crate::actor::BodyHealth`],
//!   post-hit i-frame → [`crate::actor::BodyCombat::damage_invuln_timer`])
//! - tuning/brain_spec/brain/spawn baseline/sprite override/id/name → [`ActorConfig`]
//! - patrol path             → [`ActorMotionPath`]

use bevy::ecs::query::QueryData;
use bevy::prelude::Component;

use super::super::components::BodyMelee;
use super::super::enemies::{
    spec_for_brain, ActorSpawnState, ActorSurfaceState, EnemyArchetypeSpec,
};
use super::super::path_motion::PathMotion;
use super::super::MAX_ENEMY_AIR_JUMPS;
use ambition_engine_core as ae;
use ambition_engine_core::AabbExt;

use crate::actor::{
    AncillaryMovementBundle, BodyAbilities, BodyActionBuffer, BodyBaseSize, BodyBlinkState,
    BodyComboTrace, BodyDashState, BodyDodgeState, BodyEnvironmentContact, BodyFlightState,
    BodyGroundState, BodyJumpState, BodyLedgeState, BodyLifetime, BodyMana, BodyModeState,
    BodyOffense, BodyShieldState, BodyWallState,
};
pub use crate::platformer_runtime::body::BodyKinematics;

/// Per-tick status scalars: respawn countdown, hit-flash timer, last-evaluated
/// AI mode. Health *and liveness* live on the shared [`crate::actor::BodyHealth`]
/// component now (one health authority for every body) — `alive` is derived as
/// `health.alive()` (`current > 0`), not a shadow flag the kill/revive paths had
/// to keep in lockstep. The cluster reads/writes health through
/// [`ActorMut::health`], no cluster copy + per-frame sync.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct ActorStatus {
    pub respawn_timer: f32,
    pub hit_flash: f32,
    pub ai_mode: ambition_characters::actor::ai::CharacterAiMode,
}

/// Post-hit i-frame window for a body on the actor path, written onto the body's
/// authoritative [`crate::actor::BodyCombat::damage_invuln_timer`] on a landed hit
/// (the SAME field the player gates re-hits on). Deliberately shorter than the
/// player's attack cadence (~0.4 s swipe) so it never eats an intended combo hit,
/// yet long enough to collapse a 60 fps contact/overlap stream to a single hit per
/// window. Feel-tunable.
pub const ACTOR_DAMAGE_IFRAME_S: f32 = 0.2;

/// Authored configuration + identity for an actor (any disposition). Archetype-
/// free by construction: the named roster enum is resolved at spawn and projected
/// into generic kit data (`tuning` + `brain_spec` + the `CombatCapabilities`
/// component), so neither the per-frame integration nor the runtime brain
/// rebuilds (provoke, dismount) call back into the content roster. `spawn` records
/// the authored baseline `reset_to_spawn` restores.
#[derive(Component, Clone, Debug)]
pub struct ActorConfig {
    pub id: String,
    pub name: String,
    /// Per-frame runtime tuning snapshot (kit vocabulary), projected
    /// from the archetype's authored spec at spawn.
    pub tuning: crate::combat::ActorTuning,
    /// Generic brain-construction inputs (kit vocabulary), projected
    /// from the archetype at spawn so the runtime brain rebuilds
    /// reconstruct a brain without naming the roster enum.
    pub brain_spec: crate::combat::EnemyBrainSpec,
    pub brain: ambition_characters::actor::EnemyBrain,
    pub spawn: ActorSpawnState,
    /// LDtk display name of the original NPC when this enemy was spawned
    /// by migrating a hostile NPC (keeps its own sprite sheet). `None`
    /// uses the default enemy sprite.
    pub sprite_override_npc_name: Option<String>,
    /// Uniform gameplay-side sprite identity: the catalog `character_id` this
    /// actor's sprite resolves to (via its display name, mirroring the
    /// presentation `npc_asset_for_name` join). `Some` for catalog characters
    /// (player, named NPCs/enemies, content actors); `None` for a generic
    /// enemy that renders from a kind-default sheet. Lets gameplay resolve any
    /// actor's `SheetRecord` / per-animation hit/hurt metrics — the same
    /// sprite-metadata path the player and bosses use — without reaching into
    /// the presentation registry. See [`CombatGeometry`].
    pub sprite_character_id: Option<String>,
}

/// Optional patrol path the kinematic step advances each tick.
#[derive(Component, Clone, Debug, Default)]
pub struct ActorMotionPath(pub Option<PathMotion>);

/// Seed-side **construction helper** for an actor's 18 ancillary movement
/// clusters (ground/wall/jump/dash/flight/blink/ledge/dodge/shield/…) —
/// everything in the player cluster set EXCEPT [`BodyKinematics`] (the actor's
/// shared `kin` is the single source of kinematic truth).
///
/// This is **not** a spawned component: a spawned actor carries the 18 clusters
/// as real ECS components (via [`crate::actor::AncillaryMovementBundle`], the
/// SAME bundle the player nests), so the per-frame integration borrows them as
/// the non-kinematics half of a `BodyClustersMut` view exactly like the player.
/// `ActorBody` only holds the scratch while a [`ActorClusterSeed`] is being
/// assembled (so [`Self::from_caps`] can derive the ability mask before the
/// entity exists); [`ActorClusterSeed::into_components`] then explodes it into
/// the real components.
#[derive(Clone, Debug)]
pub struct ActorBody(pub ae::BodyClusterScratch);

impl Default for ActorBody {
    fn default() -> Self {
        Self::new()
    }
}

impl ActorBody {
    /// A fresh actor movement body with the locomotion-only ability mask (no
    /// capability verbs). Used for the `Default` impl + bodies with no kit.
    pub fn new() -> Self {
        Self(ae::BodyClusterScratch::new_with_abilities(
            ae::Vec2::ZERO,
            Self::locomotion_abilities(),
        ))
    }

    /// Build the movement body whose ability mask is DERIVED from the actor's
    /// [`CombatCapabilities`] — the verbs the shared movement pipeline owns for
    /// this body. Locomotion (run + jump) is always on; **dash** turns on with
    /// `can_dash` (the pipeline's real dash impulse replaces the actor's old
    /// speed-cap burst). **fly** turns on for an aerial body (it lives in flight
    /// mode) OR a body that can toggle flight (`can_fly`); an aerial body also
    /// starts with `flight.fly_enabled` so it runs the shared flight limb from
    /// spawn. **shield** turns on with `can_shield` (the pipeline's shield limb;
    /// the damage path reads `shield.active` off that ONE component). **blink**
    /// turns on with `can_blink` (the pipeline's blink limb; the driver emits the
    /// blink sfx/vfx from the returned `FrameEvents.blinks`).
    pub fn from_caps(caps: &crate::combat::CombatCapabilities, is_aerial: bool) -> Self {
        let mut abilities = Self::locomotion_abilities();
        abilities.dash = caps.can_dash;
        abilities.fly = is_aerial || caps.can_fly;
        abilities.shield = caps.can_shield;
        abilities.blink = caps.can_blink;
        let mut scratch = ae::BodyClusterScratch::new_with_abilities(ae::Vec2::ZERO, abilities);
        scratch.flight.fly_enabled = is_aerial;
        Self(scratch)
    }

    /// The grounded actor's locomotion ability mask: run + jump + double-jump the
    /// shared movement pipeline owns. Capability verbs are layered on by
    /// [`Self::from_caps`]. `reset` is OFF so the reset gesture never fires on an
    /// actor body; wall-cling / ledge-grab / dodge / swim stay OFF for now.
    pub fn locomotion_abilities() -> ae::AbilitySet {
        ae::AbilitySet {
            move_horizontal: true,
            jump: true,
            variable_jump: true,
            double_jump: true,
            reset: false,
            ..ae::AbilitySet::basic()
        }
    }
}

/// Mutable borrow of every component the enemy integration touches,
/// assembled from a Bevy query via [`ActorClusterQueryData`].
///
/// The 18 ancillary movement clusters are borrowed as individual real-component
/// refs (`ground`, `wall`, …) — the same components the player carries — so
/// [`Self::clusters_mut`] can hand the shared movement pipeline a
/// [`ae::BodyClustersMut`] view built from `kin` + these refs, exactly like
/// the player's own query item does.
pub struct ActorMut<'a> {
    pub kin: &'a mut BodyKinematics,
    pub status: &'a mut ActorStatus,
    /// The body's shared health (the one `BodyHealth` component every actor
    /// carries) — the authoritative HP the damage / respawn / banter paths use.
    pub health: &'a mut crate::actor::BodyHealth,
    pub surface: &'a mut ActorSurfaceState,
    pub attack: &'a mut BodyMelee,
    pub config: &'a mut ActorConfig,
    pub motion: &'a mut ActorMotionPath,
    /// Spawn-resolved special-behavior flags (kit vocabulary). Read-only:
    /// the per-frame integration and the damage hook branch on these
    /// instead of calling back into the named archetype enum.
    pub caps: &'a crate::combat::CombatCapabilities,
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
    pub status: &'static mut ActorStatus,
    pub health: &'static mut crate::actor::BodyHealth,
    pub surface: &'static mut ActorSurfaceState,
    pub attack: &'static mut BodyMelee,
    pub config: &'static mut ActorConfig,
    pub motion: &'static mut ActorMotionPath,
    pub caps: &'static crate::combat::CombatCapabilities,
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
            status: &mut self.status,
            health: &mut self.health,
            surface: &mut self.surface,
            attack: &mut self.attack,
            config: &mut self.config,
            motion: &mut self.motion,
            caps: self.caps,
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

/// Owned seed used to construct the enemy ECS component cluster before spawn.
/// Runtime systems should query [`ActorClusterQueryData`] instead.
#[derive(Clone, Debug)]
pub struct ActorClusterSeed {
    pub kin: BodyKinematics,
    pub status: ActorStatus,
    /// The body's shared health (drives the spawned `BodyHealth` + the seed-based
    /// test harness's `ActorMut::health`).
    pub health: crate::actor::BodyHealth,
    pub surface: ActorSurfaceState,
    pub attack: BodyMelee,
    pub config: ActorConfig,
    pub motion: ActorMotionPath,
    /// Persistent player-movement ability state, spawned alongside the clusters
    /// by [`Self::into_components`].
    pub body: ActorBody,
    /// Spawn-resolved special-behavior flags (kit vocabulary), spawned
    /// alongside the clusters by [`Self::into_components`].
    pub caps: crate::combat::CombatCapabilities,
    /// The authored roster spec (resolved by string key from the spawn
    /// brain). Spawn-time ONLY: brain / combat-kit / held-item construction
    /// reads it here before the entity exists; it is deliberately NOT
    /// carried onto any spawned component, so the persisted [`ActorConfig`]
    /// stays roster-free. The named `EnemyArchetype` enum never reaches the
    /// spawn path — only this data does. `pub(crate)`: the seed type itself is
    /// publicly re-exported (content builds peaceful seeds) but this archetype
    /// field is internal-only.
    pub(crate) spec: EnemyArchetypeSpec,
}

/// Convert an authored LDtk actor rectangle plus a possibly sprite-derived
/// runtime collision size into the actor's initial body center.
///
/// The authored rectangle is a spatial placement footprint: its bottom edge is
/// the authored feet/floor contact. NPCs and enemies may replace that rectangle
/// with sprite-derived collision metrics at spawn time, but doing so must not
/// move the actor's feet below the platform the author placed it on. Preserve
/// the horizontal center and the authored bottom edge under the normal LDtk
/// down-gravity frame.
fn actor_spawn_center_for_collision(authored: ae::Aabb, collision_size: ae::Vec2) -> ae::Vec2 {
    ae::Vec2::new(
        authored.center().x,
        authored.bottom() - collision_size.y * 0.5,
    )
}

/// The authored sprite RENDER size (the full sprite quad) for a named catalog
/// character, or `None` for a generic enemy whose display `name` isn't a catalog
/// character. Lifted onto the shared `ActorRenderSize` at the hostile spawn sites
/// so a named character draws at its authored scale — the same render size the
/// peaceful-NPC path resolves — making e.g. the PCA identical whether it spawns
/// peaceful (symmetry room) or hostile (duel). `ldtk_fallback` only seeds the
/// collision fallback inside the resolver; the render size comes from the sheet.
pub fn sprite_render_size_for_name(name: &str, ldtk_fallback: ae::Vec2) -> Option<ae::Vec2> {
    crate::character_roster::character_id_for_display_name(name)
        .and_then(|cid| {
            crate::character_sprites::sprite_body_collision_for_character_id(cid, ldtk_fallback)
        })
        .map(|b| b.render_size)
}

impl ActorClusterSeed {
    /// Build enemy component seed state from authored spawn inputs.
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        aabb: ae::Aabb,
        brain: ambition_characters::actor::EnemyBrain,
        paths: &[(String, ambition_characters::actor::KinematicPath)],
    ) -> Self {
        let spec = spec_for_brain(&brain);
        let name: String = name.into();
        // Resolve this enemy's uniform sprite identity from its display name
        // (the same name → sheet join presentation does). `None` for a generic
        // enemy whose name isn't a catalog character.
        let sprite_character_id =
            crate::character_roster::character_id_for_display_name(&name).map(String::from);
        let motion = match &brain {
            ambition_characters::actor::EnemyBrain::Patrol {
                path_id: Some(path_id),
            } if !spec.is_sandbag => paths
                .iter()
                .find(|(p_id, _)| p_id == path_id)
                .map(|(_, path)| PathMotion::new(path.clone())),
            _ => None,
        };
        // A NAMED catalog character sizes its body to the authored sprite — the
        // SAME `sprite_body_collision_for_character_id` resolution a peaceful NPC
        // uses — so e.g. the Perfect Cellular Automaton has ONE consistent body /
        // hitbox whether it spawns peaceful (waiting in the symmetry room) or
        // hostile (the duel). A generic enemy with no catalog character keeps the
        // archetype `default_size` / LDtk spawn box, exactly as before. The matching
        // sprite RENDER size is lifted onto `ActorRenderSize` at the spawn sites via
        // [`sprite_render_size_for_name`] (the per-frame `CenteredAabb` sync then
        // follows this collision so the visual and hitbox stay together).
        let ldtk_size = spec.default_size.unwrap_or_else(|| aabb.half_size() * 2.0);
        let sprite_body = sprite_character_id.as_deref().and_then(|cid| {
            crate::character_sprites::sprite_body_collision_for_character_id(cid, ldtk_size)
        });
        let size = sprite_body.map_or(ldtk_size, |b| b.collision);
        let pos = motion
            .as_ref()
            .and_then(PathMotion::start_pos)
            .unwrap_or_else(|| actor_spawn_center_for_collision(aabb, size));
        let seed = Self {
            kin: BodyKinematics {
                pos,
                vel: ae::Vec2::ZERO,
                size,
                facing: -1.0,
            },
            status: ActorStatus {
                respawn_timer: 0.0,
                hit_flash: 0.0,
                ai_mode: ambition_characters::actor::ai::CharacterAiMode::Idle,
            },
            health: crate::actor::BodyHealth::new(ambition_characters::actor::Health::new(
                spec.max_health,
            )),
            surface: ActorSurfaceState {
                on_ground: false,
                surface_normal: ae::Vec2::new(0.0, -1.0),
                gravity_scale: if spec.is_aerial { 0.0 } else { 1.0 },
                air_jumps_remaining: MAX_ENEMY_AIR_JUMPS,
            },
            attack: BodyMelee::default(),
            config: ActorConfig {
                id: id.into(),
                name,
                tuning: spec.tuning(),
                brain_spec: spec.brain_spec(),
                brain,
                spawn: ActorSpawnState { pos, size },
                sprite_override_npc_name: None,
                sprite_character_id,
            },
            motion: ActorMotionPath(motion),
            body: ActorBody::from_caps(&spec.combat_capabilities(), spec.is_aerial),
            caps: spec.combat_capabilities(),
            spec,
        };
        seed
    }
    /// Build a PEACEFUL actor seed from catalog/NPC spawn inputs — the unified
    /// replacement for `NpcClusterScratch::new_with_paths`. A peaceful actor is
    /// the same cluster as a hostile enemy, just with peaceful tuning
    /// (`attacks_player = false`, zero aggro, `max_run_speed = NPC_PATROL_SPEED`,
    /// `health = 1`) and a `Passive`/`Patrol` AI brain; its movement is driven by
    /// the catalog `Brain` component attached at spawn, not by this `config.brain`
    /// (which only feeds the integrator's patrol-stall intent). The seed's `spec`
    /// field is filled with an inert default (peaceful actors never spawn through
    /// the archetype path), so callers — including the content crate — need no
    /// `EnemyArchetypeSpec`. Returns the seed plus the optional sprite render size
    /// (lifted onto the shared `ActorRenderSize` at spawn so it survives a flip).
    pub fn new_peaceful_npc(
        id: impl Into<String>,
        name: impl Into<String>,
        aabb: ae::Aabb,
        interactable: &ambition_interaction::Interactable,
        paths: &[(String, ambition_characters::actor::KinematicPath)],
    ) -> (Self, Option<ae::Vec2>) {
        let (patrol_radius, patrol_path_id, motion) = match &interactable.kind {
            ambition_interaction::InteractionKind::Npc {
                patrol_radius,
                patrol_path_id,
                ..
            } => {
                let motion = patrol_path_id.as_deref().and_then(|path_id| {
                    paths
                        .iter()
                        .find(|(p_id, _)| p_id == path_id)
                        .map(|(_, path)| PathMotion::new(path.clone()))
                });
                (patrol_radius.max(0.0), patrol_path_id.clone(), motion)
            }
            _ => (0.0, None, None),
        };
        let character_id = match &interactable.kind {
            ambition_interaction::InteractionKind::Npc {
                character_id: Some(cid),
                ..
            } => Some(cid.as_str()),
            _ => None,
        };
        // A `Floating` catalog body = a gravity-free flyer (the stochastic
        // parrot): zero gravity so the brain's full 2D velocity drives flight
        // through the shared aerial integrator.
        let gravity_scale =
            match character_id {
                Some(cid)
                    if matches!(
                    crate::character_roster::body_kind_for_character_id(cid),
                    Some(ambition_characters::actor::character_catalog::CharacterBodyKind::Floating)
                ) =>
                {
                    0.0
                }
                _ => 1.0,
            };
        let is_aerial = gravity_scale <= 0.001;
        // Sprite metadata supersedes the LDtk spawn box (see the old
        // `NpcClusterScratch`): size the collision to the visible body and
        // remember the render-quad size so the sprite still draws at scale.
        let ldtk_collision = aabb.half_size() * 2.0;
        let body = character_id.and_then(|cid| {
            crate::character_sprites::sprite_body_collision_for_character_id(cid, ldtk_collision)
        });
        let (collision_size, render_size) = match body {
            Some(b) => (b.collision, Some(b.render_size)),
            None => (ldtk_collision, None),
        };
        let pos = motion
            .as_ref()
            .and_then(PathMotion::start_pos)
            .unwrap_or_else(|| actor_spawn_center_for_collision(aabb, collision_size));
        let has_patrol = patrol_radius > 0.0 || motion.is_some();
        let tuning = crate::combat::ActorTuning {
            max_health: 1,
            patrol_speed: ambition_characters::brain::NPC_PATROL_SPEED,
            chase_speed: ambition_characters::brain::NPC_PATROL_SPEED,
            max_run_speed: ambition_characters::brain::NPC_PATROL_SPEED,
            is_aerial,
            ..Default::default()
        };
        let config_brain = if has_patrol {
            ambition_characters::actor::EnemyBrain::Patrol {
                path_id: patrol_path_id,
            }
        } else {
            ambition_characters::actor::EnemyBrain::Passive
        };
        let seed = Self {
            kin: BodyKinematics {
                pos,
                vel: ae::Vec2::ZERO,
                size: collision_size,
                facing: 1.0,
            },
            status: ActorStatus {
                respawn_timer: 0.0,
                hit_flash: 0.0,
                ai_mode: ambition_characters::actor::ai::CharacterAiMode::Idle,
            },
            health: crate::actor::BodyHealth::new(ambition_characters::actor::Health::new(1)),
            surface: ActorSurfaceState {
                on_ground: false,
                surface_normal: ae::Vec2::new(0.0, -1.0),
                gravity_scale,
                air_jumps_remaining: 0,
            },
            attack: BodyMelee::default(),
            config: ActorConfig {
                id: id.into(),
                name: name.into(),
                tuning,
                brain_spec: crate::combat::EnemyBrainSpec::default(),
                brain: config_brain,
                spawn: ActorSpawnState {
                    pos,
                    size: collision_size,
                },
                sprite_override_npc_name: None,
                // Peaceful actors already resolved their catalog id above.
                sprite_character_id: character_id.map(String::from),
            },
            motion: ActorMotionPath(motion),
            // A floating catalog body (the stochastic parrot) flies through the
            // shared flight limb from spawn; a grounded NPC runs the grounded spine.
            body: ActorBody::from_caps(&crate::combat::CombatCapabilities::default(), is_aerial),
            caps: crate::combat::CombatCapabilities::default(),
            // Inert: peaceful actors never spawn through the archetype path that
            // reads `spec`. `Passive` resolves to the roster's fallback row.
            spec: spec_for_brain(&ambition_characters::actor::EnemyBrain::Passive),
        };
        (seed, render_size)
    }

    /// Borrow the seed's fields (and the scratch's 18 ancillary clusters) as an
    /// [`ActorMut`] view, for the test / pre-spawn paths that drive the
    /// integration without a live ECS entity. The runtime path borrows the SAME
    /// view from real components via [`ActorClusterQueryDataItem::as_actor_mut`].
    pub fn as_actor_mut(&mut self) -> ActorMut<'_> {
        let body = &mut self.body.0;
        ActorMut {
            kin: &mut self.kin,
            status: &mut self.status,
            health: &mut self.health,
            surface: &mut self.surface,
            attack: &mut self.attack,
            config: &mut self.config,
            motion: &mut self.motion,
            caps: &self.caps,
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
    pub fn update_for_test(
        &mut self,
        world: &ae::World,
        target_pos: ae::Vec2,
        tuning: crate::combat::FeatureCombatTuning,
        nearest_neighbor: Option<ae::Vec2>,
        dt: f32,
        is_mounted: bool,
        frame: ambition_characters::actor::control::ActorControlFrame,
        gravity_dir: ae::Vec2,
    ) -> ambition_characters::actor::control::ActorControlFrame {
        self.as_actor_mut()
            .update(
                world,
                target_pos,
                tuning,
                nearest_neighbor,
                dt,
                is_mounted,
                frame,
                gravity_dir,
            )
            .0
    }

    /// The authoritative components as a spawnable Bundle. Includes the body's
    /// shared [`crate::actor::BodyHealth`] (the one health authority — spawned with
    /// the cluster, not the combat bundle).
    pub fn into_components(
        self,
    ) -> (
        BodyKinematics,
        ActorStatus,
        crate::actor::BodyHealth,
        ActorConfig,
        ActorMotionPath,
        ActorSurfaceState,
        BodyMelee,
        AncillaryMovementBundle,
        crate::combat::CombatCapabilities,
    ) {
        (
            self.kin,
            self.status,
            self.health,
            self.config,
            self.motion,
            self.surface,
            self.attack,
            AncillaryMovementBundle::from_scratch(self.body.0),
            self.caps,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sprite_sized_spawn_preserves_authored_feet() {
        let authored = ae::aabb_from_min_size(ae::Vec2::new(10.0, 20.0), ae::Vec2::new(42.0, 70.0));
        let collision_size = ae::Vec2::new(44.0, 73.0);

        let center = actor_spawn_center_for_collision(authored, collision_size);

        assert_eq!(center.x, authored.center().x);
        assert_eq!(center.y + collision_size.y * 0.5, authored.bottom());
        assert_ne!(
            center.y,
            authored.center().y,
            "different collision height should move the center to keep feet planted"
        );
    }

    #[test]
    fn ldtk_sized_spawn_keeps_authored_center() {
        let authored = ae::aabb_from_min_size(ae::Vec2::new(10.0, 20.0), ae::Vec2::new(42.0, 70.0));
        let collision_size = authored.half_size() * 2.0;

        let center = actor_spawn_center_for_collision(authored, collision_size);

        assert_eq!(center, authored.center());
    }
}
