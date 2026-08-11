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
//! - surface cling normal/gravity_scale → [`ActorSurfaceState`] (component;
//!   on_ground → [`crate::actor::BodyGroundState`], air jumps →
//!   [`crate::actor::BodyJumpState`])
//! - attack windup/active/cooldown/axis → [`BodyMelee`] (component)
//! - respawn/ai_mode          → [`ActorStatus`] (liveness → [`ambition_characters::actor::BodyHealth`];
//!   damage-blink + post-hit i-frame → [`ambition_characters::actor::BodyCombat`])
//! - tuning/brain_profile/brain/spawn baseline/sprite override/id/name → [`ActorConfig`]
//! - patrol path             → [`ActorMotionPath`]

use crate::features::enemies::ArchetypeSpecExt;
use bevy::ecs::query::QueryData;
use bevy::prelude::Component;

// The content-driven animation PIN is now
// `ambition_sprite_sheet::character::ActorAnimOverride`. It was a one-field
// newtype over `CharacterAnim`, a type that crate DEFINES, and every reader
// outside this crate (`ambition_sim_view`'s anim index + pose view, the runtime's
// rollback domain, Mary-O's shell state machine) sits ABOVE this crate. Named
// from its owner; deliberately NOT re-exported.

use super::super::components::BodyMelee;
use super::super::enemies::{ActorSpawnState, ActorSurfaceState, ArchetypeSpec, CharacterRoster};
use super::super::path_motion::PathMotion;
use ambition_characters::actor::character_catalog::CharacterCatalog;
use ambition_platformer2d_core as ae;
use ambition_platformer2d_core::AabbExt;

use crate::actor::{
    AncillaryMovementBundle, BodyAbilities, BodyActionBuffer, BodyBaseSize, BodyBlinkState,
    BodyComboTrace, BodyDashState, BodyDodgeState, BodyEnvironmentContact, BodyFlightState,
    BodyGroundState, BodyJumpState, BodyLedgeState, BodyLifetime, BodyMana, BodyModeState,
    BodyOffense, BodyShieldState, BodyWallState,
};
pub use crate::platformer_runtime::body::BodyKinematics;

/// Per-tick actor-control scalars: respawn countdown + last-evaluated AI mode.
///
/// Every body-generic fact has moved to the shared body components: liveness +
/// health → [`ambition_characters::actor::BodyHealth`] (`alive` is `health.alive()`, not a shadow
/// flag); the reaction timers (damage-blink `hit_flash` + post-hit i-frame) →
/// [`ambition_characters::actor::BodyCombat`], the SAME fields the player carries. What remains
/// here is genuinely actor-only (the player respawns via its own SafetyState; AI
/// mode is a brain concept).
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct ActorStatus {
    pub respawn_timer: f32,
    pub ai_mode: ambition_characters::actor::ai::CharacterAiMode,
}

/// Post-hit i-frame window for a body on the actor path, written onto the body's
/// authoritative [`ambition_characters::actor::BodyCombat::damage_invuln_timer`] on a landed hit
/// (the SAME field the player gates re-hits on). Deliberately shorter than the
/// player's attack cadence (~0.4 s swipe) so it never eats an intended combo hit,
/// yet long enough to collapse a 60 fps contact/overlap stream to a single hit per
/// window. Feel-tunable.
pub const ACTOR_DAMAGE_IFRAME_S: f32 = 0.2;

/// Authored configuration + identity for an actor (any disposition). Archetype-
/// free by construction: the named roster enum is resolved at spawn and projected
/// into generic kit data (`tuning` + `brain_profile` + the `CombatCapabilities`
/// component), so neither the per-frame integration nor the runtime brain
/// rebuilds (provoke, dismount) call back into the content roster. `spawn` records
/// the authored baseline `reset_to_spawn` restores.
#[derive(Component, Clone, Debug)]
pub struct ActorConfig {
    pub id: String,
    pub name: String,
    /// Per-frame runtime tuning snapshot (kit vocabulary), projected
    /// from the archetype's authored spec at spawn.
    pub tuning: crate::features::ecs::actor_tuning::ActorTuning,
    /// Generic brain-construction inputs (kit vocabulary), projected
    /// from the archetype at spawn so the runtime brain rebuilds
    /// reconstruct a brain without naming the roster enum.
    pub brain_profile: crate::features::ecs::actor_tuning::BrainProfile,
    pub brain: ambition_entity_catalog::placements::CharacterBrain,
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
    /// Seed a combat body's movement `AbilitySet` from its authored **movement
    /// kit** (`ArchetypeSpec::movement_kit`): the shared locomotion base
    /// unioned with the character's authored verbs (blink / fly / shield / dash),
    /// plus the `attack` verb every combat body carries. `is_aerial` forces
    /// flight on regardless of the kit. This is the one place a character's
    /// authored kit becomes the body's live capability set — the same
    /// `AbilitySet` the player runs, so there is no parallel enemy-only mask.
    /// `base_size` is the body's IDENTITY size — what it returns to on a reset.
    ///
    /// ⚠ it is a parameter because `BodyBaseSize::default()` is the default
    /// PLAYER size, and no enemy or boss spawn path ever wrote it. Every
    /// non-player body in the game carried a base size that was not its own,
    /// invisibly, because `base_size` is read only by `reset_body_clusters` and
    /// nothing reset an enemy through it — so the first path that did would have
    /// silently resized every enemy to a player. Asked here, where the answer is
    /// already in scope, so a body cannot be constructed without one.
    pub fn from_kit(kit: ae::AbilitySet, is_aerial: bool, base_size: ae::Vec2) -> Self {
        let mut abilities = Self::locomotion_abilities().union(kit);
        abilities.fly = is_aerial || abilities.fly;
        // A combat body HAS the attack verb (capability); WHETHER it swings is gated
        // by its `ActionSet.melee` (a peaceful NPC's empty set folds no `"attack"`
        // move, so it carries no `MovesetMelee`) and its brain (policy). The one
        // melee lifecycle is the moveset — no actor-only melee-start path.
        abilities.attack = true;
        let mut scratch = ae::BodyClusterScratch::new_with_abilities(ae::Vec2::ZERO, abilities);
        scratch.flight.fly_enabled = is_aerial;
        scratch.base_size.base_size = base_size;
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
    pub motion: &'a mut ActorMotionPath,
    /// Spawn-resolved special-behavior flags (kit vocabulary). Read-only:
    /// the per-frame integration and the damage hook branch on these
    /// instead of calling back into the named archetype enum.
    pub caps: &'a crate::combat::CombatCapabilities,
    /// The body's live held item, if it has one. See the query member.
    pub held_item: Option<&'a crate::combat::held_items::HeldItem>,
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
    /// shared [`crate::actor::AncillaryMovementBundle`], so this is required at
    /// the ECS query seam; only the owned scratch harness keeps an optional slot.
    pub sweep: &'static mut ae::SweepSample,
    pub status: &'static mut ActorStatus,
    pub health: &'static mut ambition_characters::actor::BodyHealth,
    pub surface: &'static mut ActorSurfaceState,
    pub attack: &'static mut BodyMelee,
    pub config: &'static mut ActorConfig,
    pub motion: &'static mut ActorMotionPath,
    pub caps: &'static crate::combat::CombatCapabilities,
    /// **What this body is holding RIGHT NOW**, if anything.
    ///
    /// `Option` because most bodies hold nothing, and because an OPTIONAL query
    /// member cannot silently filter a body out of the cluster the way a
    /// required one can. Read by the death path so a defeated body drops the
    /// weapon it actually has rather than the one its archetype was authored
    /// with.
    pub held_item: Option<&'static crate::combat::held_items::HeldItem>,
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

/// Owned seed used to construct the enemy ECS component cluster before spawn.
/// Runtime systems should query [`ActorClusterQueryData`] instead.
#[derive(Clone, Debug)]
pub struct ActorClusterSeed {
    pub kin: BodyKinematics,
    pub status: ActorStatus,
    /// The body's shared health (drives the spawned `BodyHealth` + the seed-based
    /// test harness's `ActorMut::health`).
    pub health: ambition_characters::actor::BodyHealth,
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
    /// Victim-owned contact material/profile resolved from the catalog row.
    pub hurt_feedback: ambition_vfx::HurtFeedback,
    /// The authored roster spec (resolved by string key from the spawn
    /// brain). Spawn-time ONLY: brain / combat-kit / held-item construction
    /// reads it here before the entity exists; it is deliberately NOT
    /// carried onto any spawned component, so the persisted [`ActorConfig`]
    /// stays roster-free. The named `CharacterArchetype` enum never reaches the
    /// spawn path — only this data does. `pub(crate)`: the seed type itself is
    /// publicly re-exported (content builds peaceful seeds) but this archetype
    /// field is internal-only.
    pub(crate) spec: ArchetypeSpec,
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
pub fn sprite_render_size_for_name_in(
    authored: &ambition_sprite_sheet::character::sheets::AuthoredSheets,
    catalog: &CharacterCatalog,
    name: &str,
    ldtk_fallback: ae::Vec2,
) -> Option<ae::Vec2> {
    catalog
        .id_for_authored_identity(name)
        .and_then(|cid| {
            crate::character_sprites::sprite_body_collision_for_character_id_in(
                authored,
                catalog,
                cid,
                ldtk_fallback,
            )
        })
        .map(|b| b.render_size)
}

/// Catalog tags that declare a body as MECHANICAL — the vocabulary an author
/// writes to make a material-aware strike land as a machine hit instead of a
/// flesh one. No character ID appears here: a body is a machine because its
/// catalog row SAYS so, which is what keeps a second provider's robot working
/// without teaching the engine its name.
const MECHANICAL_BODY_TAGS: [&str; 4] = ["robot", "automaton", "mechanical", "synthetic"];

fn actor_hurt_feedback(
    catalog: &CharacterCatalog,
    character_id: Option<&str>,
) -> ambition_vfx::HurtFeedback {
    let mechanical = character_id
        .and_then(|id| catalog.get(id))
        .is_some_and(|entry| {
            entry
                .tags
                .iter()
                .any(|tag| MECHANICAL_BODY_TAGS.contains(&tag.as_str()))
        });
    if mechanical {
        ambition_vfx::HurtFeedback::ROBOT
    } else {
        ambition_vfx::HurtFeedback::ENEMY
    }
}

impl ActorClusterSeed {
    /// Build an actor seed while resolving authored character identity from the
    /// caller's App-local catalog. Content-free tests pass an explicit empty
    /// catalog, so production construction never has a hidden fallback.
    #[allow(clippy::too_many_arguments)]
    pub fn new_in(
        authored: &ambition_sprite_sheet::character::sheets::AuthoredSheets,
        catalog: &CharacterCatalog,
        roster: &CharacterRoster,
        id: impl Into<String>,
        name: impl Into<String>,
        // **The ART identity, when the caller knows one that is not the label.**
        // `None` means *resolve from `name`*, which is what every caller did
        // before this parameter existed and what every level authored before
        // `EnemySpawnSpec::character_id` existed. It is not a default so much as
        // the older of two roads, and both stay open on purpose — see
        // `ambition_platformer2d_world::rooms::EnemySpawnSpec`.
        art_identity: Option<&str>,
        aabb: ae::Aabb,
        brain: ambition_entity_catalog::placements::CharacterBrain,
        paths: &[(String, ambition_platformer2d_core::KinematicPath)],
    ) -> Self {
        let spec = roster.spec_for_brain(&brain);
        let name: String = name.into();
        // Resolve this enemy's uniform sprite identity from the AUTHORED art
        // identity when one was given, and from its display name otherwise (the
        // same name → sheet join presentation does). `None` for a generic enemy
        // whose identity isn't a catalog character.
        // ⭐ **id first, display name second.** An authored placement that names
        // a `character_id` resolves directly and survives a rename; one that
        // carries a display name (every level today) keeps working through the
        // fallback. See `CharacterCatalog::id_for_authored_identity`.
        let sprite_character_id = catalog
            .id_for_authored_identity(art_identity.unwrap_or(name.as_str()))
            .map(String::from);
        let hurt_feedback = actor_hurt_feedback(catalog, sprite_character_id.as_deref());
        let motion = match &brain {
            ambition_entity_catalog::placements::CharacterBrain::Patrol {
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
        // [`sprite_render_size_for_name_in`] (the per-frame `CenteredAabb` sync then
        // follows this collision so the visual and hitbox stay together).
        let ldtk_size = spec.default_size.unwrap_or_else(|| aabb.half_size() * 2.0);
        let sprite_body = sprite_character_id.as_deref().and_then(|cid| {
            crate::character_sprites::sprite_body_collision_for_character_id_in(
                authored, catalog, cid, ldtk_size,
            )
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
                ai_mode: ambition_characters::actor::ai::CharacterAiMode::Idle,
            },
            health: ambition_characters::actor::BodyHealth::new(
                ambition_characters::actor::Health::new(spec.max_health),
            ),
            surface: ActorSurfaceState {
                surface_normal: ae::Vec2::new(0.0, -1.0),
                // Absence resolves the way the bare bool did; whether it should
                // instead defer to the catalog is the open product question.
                gravity_scale: if spec.is_aerial.unwrap_or(false) {
                    0.0
                } else {
                    1.0
                },
            },
            attack: BodyMelee::default(),
            config: ActorConfig {
                id: id.into(),
                name,
                tuning: spec.tuning(),
                brain_profile: spec.brain_profile(),
                brain,
                spawn: ActorSpawnState { pos, size },
                sprite_override_npc_name: None,
                sprite_character_id,
            },
            motion: ActorMotionPath(motion),
            body: ActorBody::from_kit(spec.movement_kit(), spec.is_aerial.unwrap_or(false), size),
            caps: spec.combat_capabilities(),
            hurt_feedback,
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
    /// `ArchetypeSpec`. Returns the seed plus the optional sprite render size
    /// Build a peaceful actor from the caller's App-local character catalog.
    pub fn new_peaceful_npc_in(
        authored: &ambition_sprite_sheet::character::sheets::AuthoredSheets,
        catalog: &CharacterCatalog,
        roster: &CharacterRoster,
        id: impl Into<String>,
        name: impl Into<String>,
        aabb: ae::Aabb,
        interactable: &ambition_interaction::Interactable,
        paths: &[(String, ambition_platformer2d_core::KinematicPath)],
    ) -> (Self, Option<ae::Vec2>) {
        // Only the motion attachment is derived from the placement here.
        // `patrol_radius` PARAMETERIZES a selected patrol brain (consumed during
        // brain resolution, not here), and `patrol_path_id` is the separate
        // `ActorMotionPath` movement authority — neither classifies AI behaviour.
        let motion = match &interactable.kind {
            ambition_interaction::InteractionKind::Npc {
                patrol_path_id: Some(path_id),
                ..
            } => paths
                .iter()
                .find(|(p_id, _)| p_id == path_id)
                .map(|(_, path)| PathMotion::new(path.clone())),
            _ => None,
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
                    catalog.body_kind(cid),
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
            crate::character_sprites::sprite_body_collision_for_character_id_in(
                authored,
                catalog,
                cid,
                ldtk_collision,
            )
        });
        let (collision_size, render_size) = match body {
            Some(b) => (b.collision, Some(b.render_size)),
            None => (ldtk_collision, None),
        };
        let pos = motion
            .as_ref()
            .and_then(PathMotion::start_pos)
            .unwrap_or_else(|| actor_spawn_center_for_collision(aabb, collision_size));
        // Body locomotion CAPABILITY vs AI POLICY (control-refactor convergence):
        // `max_run_speed` is the body's PHYSICAL top speed under direct control —
        // the same capability the player body has, so a possessed NPC is
        // responsive, not stuck at stroll pace. `patrol_speed`/`chase_speed` are
        // AI POLICY: the peaceful brain expresses them as NORMALIZED intent
        // (`locomotion_for(patrol_speed)` = patrol_speed / max_run_speed), which the
        // integrator scales back — so autonomous patrol still ambles at
        // NPC_PATROL_SPEED while the SAME body sprints at `max_run_speed` when a
        // player drives it. (Was: all three = NPC_PATROL_SPEED, conflating policy
        // with capability — the "possessed NPC moves extremely slowly" bug.)
        let tuning = crate::features::ecs::actor_tuning::ActorTuning {
            max_health: 1,
            patrol_speed: ambition_characters::brain::NPC_PATROL_SPEED,
            chase_speed: ambition_characters::brain::NPC_PATROL_SPEED,
            max_run_speed: ambition_platformer2d_core::MAX_RUN_SPEED,
            is_aerial,
            // STATED, not inherited from `Default`: an NPC is a unique named
            // placement, so its death is permanent (ADR 0022) even after it
            // provokes into a mob archetype authored `OnRoomReenter`. This is
            // the pin `ActorTuning::adopting_archetype` protects.
            respawn: ambition_entity_catalog::placements::RespawnPolicy::DeadStaysDead,
            ..Default::default()
        };
        // `config.brain` (the integrator-facing `CharacterBrain` read-model, which
        // only feeds patrol-stall intent) is DERIVED from the actor's resolved
        // autonomous `Brain` after spawn — never inferred here from `patrol_radius`
        // / motion presence. A nonzero radius or a path no longer classifies AI
        // behaviour; explicit brain selection is the sole authority. Seed it
        // `Passive`; `NpcActorSpawnPlan::peaceful` overwrites it to `Patrol` iff the
        // resolved brain is a Patrol brain.
        let config_brain = ambition_entity_catalog::placements::CharacterBrain::Passive;
        let seed = Self {
            kin: BodyKinematics {
                pos,
                vel: ae::Vec2::ZERO,
                size: collision_size,
                facing: 1.0,
            },
            status: ActorStatus {
                respawn_timer: 0.0,
                ai_mode: ambition_characters::actor::ai::CharacterAiMode::Idle,
            },
            health: ambition_characters::actor::BodyHealth::new(
                ambition_characters::actor::Health::new(1),
            ),
            surface: ActorSurfaceState {
                surface_normal: ae::Vec2::new(0.0, -1.0),
                gravity_scale,
            },
            attack: BodyMelee::default(),
            config: ActorConfig {
                id: id.into(),
                name: name.into(),
                tuning,
                brain_profile: crate::features::ecs::actor_tuning::BrainProfile::default(),
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
            body: ActorBody::from_kit(ae::AbilitySet::NONE, is_aerial, collision_size),
            caps: crate::combat::CombatCapabilities::default(),
            hurt_feedback: actor_hurt_feedback(catalog, character_id),
            // Inert: peaceful actors never spawn through the archetype path that
            // reads `spec`. `Passive` resolves to the roster's fallback row.
            spec: roster
                .spec_for_brain(&ambition_entity_catalog::placements::CharacterBrain::Passive),
        };
        (seed, render_size)
    }

    /// **A BODY, BUILT FROM ITS CHARACTER.**
    ///
    /// ⭐ **the seat's body no longer comes from an enemy archetype.** A match
    /// seat used to build through [`Self::new_in`], which starts
    /// `roster.spec_for_brain(&brain)` and takes health, tuning, capabilities,
    /// movement kit and aerial-ness off that row — so every fighter on the smash
    /// grid was physically a `combatant`, wearing a character. Jon's brief calls
    /// this out by name: *"No ordinary constructor should first build an
    /// `ArchetypeSpec` creature and then patch the character over it."*
    ///
    /// This is the same shape as [`Self::new_peaceful_npc_in`], which has built
    /// bodies without an archetype since it was written — proof the pattern was
    /// available the whole time, for the one population that never went through
    /// the roster.
    ///
    /// ```text
    /// character   size, health, weight, art identity, aerial-ness
    /// ruleset     death policy, stocks, ability mask   (applied by the match)
    /// controller  the BrainProfile passed in           (policy, not a body)
    /// ```
    ///
    /// ⚠ **the tuning is a FIGHTER default, not a character fact — yet.** Run
    /// speed, contact damage and the rest still have no authoring surface on a
    /// definition (campaign P1.8), so they are stated here, once, where a match
    /// can see them, rather than borrowed from whichever archetype a seat
    /// happened to name. Each becomes a character fact as its field lands.
    pub(crate) fn new_character_in(
        authored: &ambition_sprite_sheet::character::sheets::AuthoredSheets,
        catalog: &CharacterCatalog,
        id: impl Into<String>,
        character_id: &str,
        display_name: impl Into<String>,
        aabb: ae::Aabb,
        max_health: i32,
        brain_profile: crate::features::ecs::actor_tuning::BrainProfile,
        config_brain: ambition_entity_catalog::placements::CharacterBrain,
        // **What the CHARACTER says about its own body**, when it says anything.
        // A crawler that authors its locomotion crawls in a fighter seat too,
        // which is the whole of Jon's Puppy-Slug acceptance test: *"movement
        // input → uses Puppy Slug's actual authored locomotion"*.
        locomotion: Option<ambition_characters::actor::CharacterLocomotion>,
        contact_damage: Option<ambition_characters::actor::ContactDamage>,
        dream_seed: Option<f32>,
    ) -> Self {
        // The AUTHORED silhouette, resolved exactly as a peaceful NPC of the
        // same character resolves it — one body per character, however it is
        // spawned.
        let ldtk_collision = aabb.half_size() * 2.0;
        let sprite_body = crate::character_sprites::sprite_body_collision_for_character_id_in(
            authored,
            catalog,
            character_id,
            ldtk_collision,
        );
        let collision_size = sprite_body.map_or(ldtk_collision, |body| body.collision);
        // ⭐ **THE CHARACTER FIRST, the catalog second.** A migrated character
        // states whether it flies; one that has not stated it still gets the
        // catalog's answer, which is every unmigrated flyer. Without the first
        // half a character whose catalog row lives in another provider's
        // fragment could not fly in the demo that owns its level.
        let is_aerial = locomotion.is_some_and(|locomotion| locomotion.flies)
            || matches!(
                catalog.body_kind(character_id),
                Some(ambition_characters::actor::character_catalog::CharacterBodyKind::Floating)
            );
        let pos = actor_spawn_center_for_collision(aabb, collision_size);
        // ⭐ **THE CHARACTER'S OWN TOP SPEED WHEN IT STATES ONE.** A fighter
        // default otherwise: the stage has to give a body that has never said
        // how fast it is SOMETHING, and a match is the one place that may.
        let run_speed = locomotion
            .map(|locomotion| locomotion.run_speed)
            .filter(|speed| *speed > 0.0)
            .unwrap_or(ambition_platformer2d_core::MAX_RUN_SPEED);
        let tuning = crate::features::ecs::actor_tuning::ActorTuning {
            max_health,
            // A fighter is driven at full pace by whatever drives it. The
            // autonomous PACING is the profile's (`patrol`/`chase` effort), and
            // it is expressed as normalized intent against this capability —
            // the same split a possessed NPC gets, for the same reason.
            patrol_speed: run_speed * 0.5,
            chase_speed: run_speed,
            max_run_speed: run_speed,
            // Touching a body hurts only if its CHARACTER says so. A fighter
            // that authors none is safe to stand next to, which is what every
            // fighter has been.
            contact_strength: contact_damage.map_or(0.0, |contact| contact.strength),
            damage_amount: contact_damage.map_or(0, |contact| contact.amount),
            body_contact_damage: contact_damage.is_some(),
            dream_seed,
            surface_walker: locomotion.is_some_and(|locomotion| locomotion.surface_walker),
            cling_breaks_on_hit: locomotion
                .is_some_and(|locomotion| locomotion.cling_breaks_on_hit),
            // A match seat is a combatant whoever drives it; the disposition the
            // body carries is set by realization, and this is the tuning half.
            attacks_player: true,
            // ⚠ a fighter's death is the MATCH's business (stocks, blast zones),
            // never a room's respawn policy.
            respawn: ambition_entity_catalog::placements::RespawnPolicy::DeadStaysDead,
            is_aerial,
            ..Default::default()
        };
        Self {
            kin: BodyKinematics {
                pos,
                vel: ae::Vec2::ZERO,
                size: collision_size,
                facing: 1.0,
            },
            status: ActorStatus {
                respawn_timer: 0.0,
                ai_mode: ambition_characters::actor::ai::CharacterAiMode::Idle,
            },
            health: ambition_characters::actor::BodyHealth::new(
                ambition_characters::actor::Health::new(max_health.max(1)),
            ),
            surface: ActorSurfaceState {
                surface_normal: ae::Vec2::new(0.0, -1.0),
                gravity_scale: if is_aerial { 0.0 } else { 1.0 },
            },
            attack: BodyMelee::default(),
            config: ActorConfig {
                id: id.into(),
                name: display_name.into(),
                tuning,
                brain_profile,
                brain: config_brain,
                spawn: ActorSpawnState {
                    pos,
                    size: collision_size,
                },
                sprite_override_npc_name: None,
                // ⭐ the CHARACTER, stated rather than resolved from a display
                // name. A seat knows exactly which character it is seating.
                sprite_character_id: Some(character_id.to_string()),
            },
            motion: ActorMotionPath(None),
            // The MATCH declares what a fighter may do (`seat_abilities`), so
            // the seed grants nothing and the ruleset writes the real set in the
            // same flush that builds the body.
            body: ActorBody::from_kit(ae::AbilitySet::NONE, is_aerial, collision_size),
            // Death traits are the character's and arrive with the persona
            // derive, like its moves — a seed that guessed them would be a
            // second writer.
            caps: crate::combat::CombatCapabilities::default(),
            hurt_feedback: actor_hurt_feedback(catalog, Some(character_id)),
            // ⛔ INERT, and it is the point: no seat path reads `spec`, so a
            // fighter carries an empty archetype rather than somebody else's.
            spec: crate::combat::archetype_spec::ArchetypeSpec {
                inherits: None,
                movement: Default::default(),
                movement_resolved: Default::default(),
                max_health,
                run_speed: ambition_platformer2d_core::MAX_RUN_SPEED,
                patrol_effort: 0.5,
                chase_effort: 1.0,
                aggro_radius: 0.0,
                attack_range: 0.0,
                contact_strength: 0.0,
                damage_amount: 0,
                attack_cooldown_mult: 1.0,
                mass: 1.0,
                surface_walker: false,
                turns_at_walls: true,
                cling_breaks_on_hit: false,
                is_aerial: Some(is_aerial),
                is_sandbag: false,
                explodes_on_death: false,
                divides_on_death: false,
                charge_crash_explodes: false,
                never_dies: false,
                respawn: ambition_entity_catalog::placements::RespawnPolicy::DeadStaysDead,
                weight: 1.0,
                death_policy: Default::default(),
                dream_seed: None,
                mount_class: None,
                pilotable_mount_classes: Vec::new(),
                mount_death_splash: None,
                default_size: None,
                brain_template: ambition_characters::brain::CharacterBrainTemplate::StandStill,
                fighter_level: None,
                melee: None,
                ranged: None,
                held_item: None,
                smash_hit_band: None,
                smash_heavy: false,
                smash_dash_to_close: false,
                smash_duelist: false,
                can_blink: false,
                can_fly: false,
                can_shield: false,
                can_dash: false,
                provoke_forced_brute_min_aggro: None,
                attacks_player: true,
                body_contact_damage: false,
                ranged_visual: String::new(),
                signature_move: None,
                move_style: locomotion
                    .map(|locomotion| locomotion.move_style)
                    .unwrap_or_default(),
            },
        }
    }

    /// Borrow the seed's fields (and the scratch's 18 ancillary clusters) as an
    /// [`ActorMut`] view, for the test / pre-spawn paths that drive the
    /// integration without a live ECS entity. The runtime path borrows the SAME
    /// view from real components via [`ActorClusterQueryDataItem::as_actor_mut`].
    pub fn as_actor_mut(&mut self) -> ActorMut<'_> {
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
    pub fn update_for_test(
        &mut self,
        world: &ae::World,
        target_pos: ae::Vec2,
        tuning: crate::combat::FeatureCombatTuning,
        dt: f32,
        is_mounted: bool,
        frame: ambition_characters::actor::control::ActorControlFrame,
        motion_model: &mut crate::features::MotionModel,
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
                crate::time::feel::Platformer2dFeelTuningMonolith::default(),
                None,
                (0.0, 0.0),
            )
            .0
    }

    /// **Let the CHARACTER outrank the archetype for the facts it authors.**
    ///
    /// D73 phase 3's first seam, and the smallest one that can exist: an enemy
    /// body is built from an `ArchetypeSpec` and never reads its character at
    /// all, so a fact moved onto a definition today would be stated where
    /// nothing on this path looks. This is where it starts being looked at.
    ///
    /// ⚠ **precedence, not replacement.** Only fields the definition ACTUALLY
    /// AUTHORS move; everything it is silent about keeps the archetype's answer.
    /// That is what makes adopting a character behaviour-neutral until somebody
    /// authors something, which is the property the migration needs in order to
    /// proceed one fact at a time instead of in one unreviewable jump.
    ///
    /// ⛔ **and it is applied to the SEED, before the entity exists** — not by a
    /// later system correcting a body that was already built wrong. A second
    /// writer racing the first is the shape this campaign is removing.
    pub fn adopt_character_intrinsics(
        &mut self,
        definition: &crate::character_runtime::PreparedCharacterDefinition,
    ) {
        // Death traits. The first fact with a home on a definition (2026-08-10),
        // and the reason the mites are the migration's first candidates.
        if let Some(traits) = definition.death_traits.as_ref() {
            self.caps = crate::combat::CombatCapabilities::from(traits);
        }
        // The physical baseline the worn and seated paths already share, so an
        // enemy body cannot disagree with a fighter about the same character.
        let baseline = crate::character_runtime::PhysicalBaseline::of(definition);
        // `max_health_over` IS the precedence rule spelled as a function: the
        // character's pool when it authored one, the seed's own otherwise. The
        // seated path calls it the same way, which is the point — one applier,
        // one answer, whichever construction reaches the character.
        let standing = self.health.health.max;
        let max = baseline.max_health_over(standing);
        if max != standing {
            self.health = ambition_characters::actor::BodyHealth::new(
                ambition_characters::actor::Health::new(max),
            )
            .with_policy(self.health.policy());
        }
        if let Some(weight) = baseline.knockback_weight() {
            self.config.tuning.weight = weight;
        }
    }

    /// The authoritative components as a spawnable Bundle. Includes the body's
    /// shared [`ambition_characters::actor::BodyHealth`] (the one health authority — spawned with
    /// the cluster, not the combat bundle).
    pub fn into_components(
        self,
    ) -> (
        BodyKinematics,
        ActorStatus,
        ambition_characters::actor::BodyHealth,
        ActorConfig,
        ActorMotionPath,
        ActorSurfaceState,
        BodyMelee,
        AncillaryMovementBundle,
        crate::combat::CombatCapabilities,
        crate::combat::CombatTuning,
    ) {
        // Project the actor's authored weight onto the combat-owned carrier at
        // spawn (E2 verdict b): the damage paths read `CombatTuning`, never the
        // sim-heart `ActorConfig`.
        let combat_tuning = crate::combat::CombatTuning {
            weight: self.config.tuning.weight,
            attack_cooldown_mult: self.config.tuning.attack_cooldown_mult,
            sprite_character_id: self.config.sprite_character_id.clone(),
            // CM8: an ordinary actor reacts to being hit with the plain hurt
            // profile — no red player-hurt spray. This is the per-body seam for
            // catalog-authored reactions (robot-tagged bodies crunch; ordinary
            // bodies keep the ENEMY/flesh default).
            hurt_feedback: self.hurt_feedback,
        };
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
            combat_tuning,
        )
    }
}

#[cfg(test)]
impl ActorClusterSeed {
    /// Content-free convenience constructor for unit tests. Production spawn
    /// paths must use [`Self::new_in`] with their App-local catalog.
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        aabb: ae::Aabb,
        brain: ambition_entity_catalog::placements::CharacterBrain,
        paths: &[(String, ambition_platformer2d_core::KinematicPath)],
    ) -> Self {
        Self::new_in(
            &Default::default(),
            &CharacterCatalog::empty(),
            &super::super::enemies::test_roster(),
            id,
            name,
            None,
            aabb,
            brain,
            paths,
        )
    }

    /// Content-free peaceful-NPC constructor for unit tests.
    pub fn new_peaceful_npc(
        id: impl Into<String>,
        name: impl Into<String>,
        aabb: ae::Aabb,
        interactable: &ambition_interaction::Interactable,
        paths: &[(String, ambition_platformer2d_core::KinematicPath)],
    ) -> (Self, Option<ae::Vec2>) {
        Self::new_peaceful_npc_in(
            // A content-free constructor has no providers, so no authored
            // sheets — the empty registry is the honest value, not a stand-in.
            &Default::default(),
            &CharacterCatalog::empty(),
            &super::super::enemies::test_roster(),
            id,
            name,
            aabb,
            interactable,
            paths,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A mechanical body is one whose CATALOG ROW says so — the engine knows
    /// the tag vocabulary, never a character id. An unknown/absent row is
    /// flesh, so a content-free spawn keeps the ordinary enemy reaction.
    #[test]
    fn a_mechanical_tag_is_what_selects_the_machine_hurt_profile() {
        const CATALOG: &str = r#"(
            brain_presets: { "idle": StandStill },
            action_set_presets: { "peaceful": (move_style: Walk) },
            characters: {
                "tin_man": (
                    display_name: "Tin Man", spritesheet: "tin.png",
                    manifest: "tin_spritesheet.ron", tier: MainHall,
                    body_kind: Standard, composition: None,
                    default_brain: "idle", default_action_set: "peaceful",
                    tags: ["enemy", "automaton"],
                    barks: (),
                ),
                "ogre": (
                    display_name: "Ogre", spritesheet: "ogre.png",
                    manifest: "ogre_spritesheet.ron", tier: MainHall,
                    body_kind: Standard, composition: None,
                    default_brain: "idle", default_action_set: "peaceful",
                    tags: ["enemy"],
                    barks: (),
                ),
            },
        )"#;
        let catalog = CharacterCatalog::from_data(
            ambition_characters::actor::character_catalog::parse_catalog(CATALOG),
        );

        assert_eq!(
            actor_hurt_feedback(&catalog, Some("tin_man")),
            ambition_vfx::HurtFeedback::ROBOT,
        );
        assert_eq!(
            actor_hurt_feedback(&catalog, Some("ogre")),
            ambition_vfx::HurtFeedback::ENEMY,
        );
        assert_eq!(
            actor_hurt_feedback(&catalog, None),
            ambition_vfx::HurtFeedback::ENEMY,
        );
    }

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
    /// **D73 phase 3, the first seam: an enemy seed can be told what its
    /// CHARACTER says**, and a character that says nothing changes nothing.
    ///
    /// ⚠ the parity half is the load-bearing one. Every enemy in the game is
    /// built from an archetype and no definition today authors any of these, so
    /// a seam that quietly wrote defaults over the archetype would move every
    /// mob's health and death behaviour at once — the ~100-NPC regression, on
    /// the other spawn path.
    mod character_intrinsics {
        use super::*;

        fn seed_with(health: i32, weight: f32) -> ActorClusterSeed {
            let mut seed = ActorClusterSeed::new(
                "mite".to_string(),
                "Exploding Mite".to_string(),
                ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(12.0, 12.0)),
                ambition_entity_catalog::placements::CharacterBrain::Custom("combatant".into()),
                &[],
            );
            seed.health = ambition_characters::actor::BodyHealth::new(
                ambition_characters::actor::Health::new(health),
            );
            seed.config.tuning.weight = weight;
            seed.caps = crate::combat::CombatCapabilities::default();
            seed
        }

        fn definition(
            f: impl FnOnce(&mut crate::character_runtime::CharacterDefinition),
        ) -> crate::character_runtime::PreparedCharacterDefinition {
            let mut def = crate::character_runtime::CharacterDefinition::new(
                "exploding_mite",
                "Exploding Mite",
                "test",
            );
            f(&mut def);
            crate::character_runtime::prepare_and_finalize_for_test(
                def,
                &crate::character_runtime::CharacterBindings::default(),
            )
            .prepared
        }

        #[test]
        fn a_character_that_authors_nothing_leaves_the_archetype_alone() {
            let mut seed = seed_with(2, 1.4);
            seed.adopt_character_intrinsics(&definition(|_| {}));
            assert_eq!(seed.health.health.max, 2, "the archetype's pool stands");
            assert_eq!(seed.config.tuning.weight, 1.4, "and its weight");
            assert!(
                !seed.caps.explodes_on_death,
                "and it did not acquire a death trait nobody authored"
            );
        }

        #[test]
        fn an_authored_character_outranks_the_archetype_for_what_it_states() {
            let mut seed = seed_with(2, 1.4);
            seed.adopt_character_intrinsics(&definition(|def| {
                def.vitals.max_health = Some(9);
                def.death_traits = Some(ambition_characters::actor::CharacterDeathTraits {
                    explodes_on_death: true,
                    ..Default::default()
                });
            }));
            assert_eq!(seed.health.health.max, 9);
            assert!(seed.caps.explodes_on_death, "the mite's whole point");
            // ⭐ and the field it stayed SILENT about is untouched, which is what
            // makes this precedence rather than replacement.
            assert_eq!(
                seed.config.tuning.weight, 1.4,
                "an unauthored weight must keep the archetype's, or adopting a \
                 character silently retunes knockback across the cast"
            );
        }
    }
}
