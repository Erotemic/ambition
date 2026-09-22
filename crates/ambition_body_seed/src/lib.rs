//! The actor BODY SEED: the prepared body facts a construction path spawns from.
//!
//! Every actor — was-NPC, was-enemy, encounter mob, seated fighter — is built
//! from one [`ActorClusterSeed`]. The seed is a VALUE over the character,
//! combat and body-cluster vocabularies: its two constructors read the catalog
//! and the authored sheets, resolve the collision box, the tuning and the
//! patrol path, and hand back something that names no simulation. The actor
//! kernel consumes it — [`ActorClusterSeed::into_components`] is the whole
//! contract between preparation and construction — and the kernel's per-tick
//! `ActorMut` view is bound to the seed from the kernel's side.
//!
//! Field → component map:
//! - pos/vel/size/facing      → [`BodyKinematics`]
//! - surface cling normal/gravity_scale → [`ActorSurfaceState`] (component;
//!   on_ground → [`ambition_platformer2d_core::BodyGroundState`], air jumps →
//!   [`ambition_platformer2d_core::BodyJumpState`])
//! - attack windup/active/cooldown/axis → [`BodyMelee`] (component)
//! - respawn/ai_mode          → [`ActorStatus`] (liveness → [`ambition_characters::actor::BodyHealth`];
//!   damage-blink + post-hit i-frame → [`ambition_characters::actor::BodyCombat`])
//! - tuning/brain_profile/brain/spawn baseline/sprite override/id/name → [`ActorConfig`]
//! - patrol path             → [`ActorMotionPath`]

use ambition_characters::actor::ai::ActorStatus;
use ambition_combat::actor_tuning::ActorConfig;
use ambition_platformer2d_core::BodyKinematics;
use bevy::prelude::Component;

use ambition_characters::actor::character_catalog::CharacterCatalog;
use ambition_combat::components::BodyMelee;
use ambition_combat::path_motion::PathMotion;
use ambition_platformer2d_core as ae;
use ambition_platformer2d_core::body_clusters::ActorSurfaceState;
use ambition_platformer2d_core::AabbExt;
use ambition_platformer2d_shared_tangle::body::{AncillaryMovementBundle, SpawnBaseline};

pub mod physical_baseline;
mod snapshot_impls;
pub use physical_baseline::{
    BaselineBoundary, BodyGeometry, DisplacedPhysicals, PersonaBaseline, PhysicalBaseline,
    PhysicalRetraction,
};

/// Optional patrol path the kinematic step advances each tick.
#[derive(Component, Clone, Debug, Default)]
pub struct ActorMotionPath(pub Option<PathMotion>);

/// Seed-side construction helper for an actor's 18 ancillary movement
/// clusters (ground/wall/jump/dash/flight/blink/ledge/dodge/shield/…) —
/// everything in the player cluster set EXCEPT [`BodyKinematics`] (the actor's
/// shared `kin` is the single source of kinematic truth).
///
/// This is not a spawned component: a spawned actor carries the 18 clusters
/// as real ECS components (via [`ambition_platformer2d_shared_tangle::body::AncillaryMovementBundle`], the
/// SAME bundle the player nests), so the per-frame integration borrows them as
/// the non-kinematics half of a `BodyClustersMut` view exactly like the player.
/// `ActorBody` only holds the scratch while a [`ActorClusterSeed`] is being
/// assembled (so [`Self::from_caps`] can derive the ability mask before the
/// entity exists); [`ActorClusterSeed::into_components`] then explodes it into
/// the real components.
#[derive(Clone, Debug)]
pub struct ActorBody(pub ae::BodyClusterScratch);

impl ActorBody {
    /// Build an actor movement body from a RESOLVED ability set.
    ///
    /// ⛔ THIS CONFERS NOTHING, and that is the whole point. It was
    /// `from_kit`, which did `locomotion_abilities().union(kit)` and then
    /// `abilities.attack = true` — so a character's authored set could only be
    /// WIDENED by it. `npc_puppy_slug` authors `{ move_horizontal: true,
    /// ..AbilitySet::NONE }` and was handed a jump, a double jump and an attack;
    /// the perfect cellular automaton's own comment says *"It has no double
    /// jump, no fast fall, no dodge and no ledge grab"* and it had a double jump.
    /// A union cannot express a refusal, so authored data could not decline the
    /// floor.
    ///
    /// That also made [`ActionSet::gated_by`]'s attack term dead for every actor
    /// body: the gate asks `abilities.attack`, and the constructor had just
    /// forced it true.
    ///
    /// ⚠ `is_aerial` STILL DECIDES FLIGHT, and it is not a conferral: it is the
    /// character's own answer, resolved from its `body_kind` /
    /// `baseline_free_flight` before it reaches here. A floating body flies
    /// because it floats.
    ///
    /// `base_size` is required identity state used when resetting the body.
    pub fn from_abilities(
        abilities: ae::AbilitySet,
        is_aerial: bool,
        base_size: ae::Vec2,
    ) -> Self {
        let mut abilities = abilities;
        abilities.fly = is_aerial || abilities.fly;
        let mut scratch = ae::BodyClusterScratch::new_with_abilities(ae::Vec2::ZERO, abilities);
        scratch.flight.fly_enabled = is_aerial;
        scratch.base_size.base_size = base_size;
        Self(scratch)
    }

    /// The RULESET'S DEFAULT ACTOR BODY: what a character that states no
    /// abilities of its own is given.
    ///
    /// This is the set `from_kit` used to union into everybody, plus the
    /// `attack` it forced. Named and applied only where the authored answer is
    /// ABSENT, so it is a default rather than a floor — a character that states
    /// its own set now gets exactly that set, and can decline any of these.
    ///
    /// `reset` is off for an actor body: only a controlled body resets itself.
    pub fn default_actor_abilities() -> ae::AbilitySet {
        ae::AbilitySet {
            move_horizontal: true,
            jump: true,
            variable_jump: true,
            double_jump: true,
            // CAPABILITY, not policy: the moveset and the brain decide whether a
            // body that MAY attack ever does. A character that should not be
            // able to at all says so in its own set.
            attack: true,
            reset: false,
            ..ae::AbilitySet::basic()
        }
    }
}

#[derive(Clone, Debug)]
pub struct ActorClusterSeed {
    pub kin: BodyKinematics,
    pub status: ActorStatus,
    /// The body's shared health (drives the spawned `BodyHealth` + the seed-based
    /// test harness's `ActorMut::health`).
    pub health: ambition_characters::actor::BodyHealth,
    /// The body's shared combat component, seeded here for the same reason
    /// `health` is: it is a component every body carries, the player included,
    /// and the seed is where a body's components are decided.
    pub combat: ambition_characters::actor::BodyCombat,
    pub surface: ActorSurfaceState,
    pub attack: BodyMelee,
    pub config: ActorConfig,
    /// The body a reset hands back — position, authored size, authored gravity
    /// scale. ⛔ IT IS NOT PART OF `ActorConfig`: a mount dissolving a dead
    /// shark restores this and nothing else about the rider's identity, and a
    /// mount crate cannot name the monolith's authored-actor definition.
    pub spawn: SpawnBaseline,
    pub motion: ActorMotionPath,
    /// Persistent player-movement ability state, spawned alongside the clusters
    /// by [`Self::into_components`].
    pub body: ActorBody,
    /// Spawn-resolved special-behavior flags (kit vocabulary), spawned
    /// alongside the clusters by [`Self::into_components`].
    pub caps: ambition_combat::CombatCapabilities,
    /// Victim-owned contact material/profile resolved from the catalog row.
    pub hurt_feedback: ambition_vfx::HurtFeedback,
    /// ⛔⛤ **THE PRESENTATION HALF OF THE BODY THIS SEED RESOLVED, OR `None`
    /// FOR A CHARACTER THAT AUTHORS NO `BodySource` — 2026-09-21.**
    ///
    /// [`Self::kin`]`.size` is the collision third of one
    /// [`PosedBodyGeometry`]; the render quad and the quad offset are the other
    /// two, and they are not this crate's to insert — a seed is plain data and
    /// the spawn sites own which components a body gets. Carrying them means
    /// the spawn site can seed a body's geometry from the SAME resolution that
    /// sized it, instead of asking the catalog join a second time and getting a
    /// different answer (Solid Snake: 118x118 against the sheet's 23x23).
    ///
    /// `None` is the ordinary case and means "nothing here overrides the
    /// catalog road", so every body that does not author a sprite body is
    /// untouched by this.
    pub posed: Option<ambition_sprite_sheet::character::sheets::PosedBodyGeometry>,
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
            // ⭐ THE DERIVATION, NOT THE MONOLITH'S ADAPTER AROUND IT. The
            // adapter's whole body is `catalog.data()` plus this call, and this
            // caller already holds both halves — so naming it here made
            // `actor_clusters` look coupled to the monolith over a `.data()`.
            ambition_sprite_sheet::character::catalog_join::sprite_body_collision_for_character_id_from_data(
                authored,
                catalog.data(),
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

/// What [`ActorClusterSeed::into_components`] spawns.
///
/// ⛔ NAMED because two test modules hand-wrote this tuple and both went stale
/// the moment `SpawnBaseline` was added to it. The compiler caught them, which is
/// the lucky case; a name means there is nothing to keep in step.
pub type ActorClusterBundle = (
    BodyKinematics,
    ActorStatus,
    ambition_characters::actor::BodyHealth,
    ActorConfig,
    SpawnBaseline,
    ActorMotionPath,
    ActorSurfaceState,
    BodyMelee,
    AncillaryMovementBundle,
    ambition_combat::CombatCapabilities,
    ambition_combat::CombatTuning,
);
impl ActorClusterSeed {
    /// Put this un-spawned body somewhere, once.
    ///
    /// A seed's placement is TWO fields — where the body starts (`kin.pos`) and where a respawn
    /// returns it (`SpawnBaseline::pos`) — and they are the same fact.
    ///
    /// this is a SEED, not a body, which is the whole reason a bare write
    /// is not the answer even though ADR 0024's pose authority is about live
    /// bodies. There is no entity yet, no `MotionModel` to reconcile and no frame
    /// to transit through, so `transit_body` cannot be called here at all — the
    /// thing worth having is not an authority call, it is ONE name for the fact.
    /// `engine.pose-writes-are-authority-only` cannot tell a pre-spawn seed
    /// from a live body (it matches the receiver's NAME), so it read the old pair
    /// as a bare relocation; see that policy's rationale.
    pub fn place_at(&mut self, pos: ae::Vec2) {
        self.kin = BodyKinematics { pos, ..self.kin };
        self.spawn.pos = pos;
    }

    //  [`Self::new_character_in`] is the only body constructor. A body is what
    // its CHARACTER says it is, and an identifier that names no character is a
    // construction refusal rather than a silent downgrade.

    /// Build a PEACEFUL actor seed from catalog/NPC spawn inputs    /// Build a PEACEFUL actor seed from catalog/NPC spawn inputs — the unified
    /// replacement for `NpcClusterScratch::new_with_paths`. A peaceful actor is
    /// the same cluster as a hostile enemy, just with peaceful tuning
    /// (`is_hostile = false`, zero aggro, `max_run_speed = NPC_PATROL_SPEED`,
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
        // The prepared cast, so this road can ask the CHARACTER whether it
        // flies before it asks the catalog. `Option` because a composition with
        // no registered characters is the ordinary case, not a degraded one.
        prepared: Option<&ambition_characters::prepared::PreparedCharacterRegistry>,
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
            } => ambition_platformer2d_core::resolve_kinematic_path(paths, path_id)
                .map(|path| PathMotion::new(path.clone())),
            _ => None,
        };
        let character_id = match &interactable.kind {
            ambition_interaction::InteractionKind::Npc {
                character_id: Some(cid),
                ..
            } => Some(cid.as_str()),
            _ => None,
        };
        // DOES THIS BODY FLY? ASK THE CHARACTER, THEN THE CATALOG.
        //
        // two spawn paths decided aerial-ness and NEITHER asked the character: this one read
        // the catalog's `body_kind: Floating`, the hostile `EnemySpawn` path read
        // `ArchetypeSpec:flies` (see the doc on that field, which names the split).
        //
        // A PREPARED CHARACTER ALWAYS ANSWERS, and getting that precise matters more than
        // it sounds.
        //
        //  the catalog rule below is therefore NOT a tiebreak between two
        // authorities. It answers for a character with NO PREPARED ENTRY AT ALL,
        // which is ~150 of the game's 163 NPC placements today and shrinks by one
        // every time a character is migrated. When the registry holds everything,
        // the `unwrap_or` arm becomes unreachable and goes with the catalog read.
        let authored_flight = character_id
            .and_then(|cid| prepared.and_then(|prepared| prepared.get(cid)))
            .and_then(|prepared| prepared.locomotion)
            .and_then(|locomotion| locomotion.baseline_free_flight);
        let floats_by_catalog = matches!(
            character_id.and_then(|cid| catalog.body_kind(cid)),
            Some(ambition_characters::actor::character_catalog::CharacterBodyKind::Floating)
        );
        let gravity_scale = if authored_flight.unwrap_or(floats_by_catalog) {
            0.0
        } else {
            1.0
        };
        let is_aerial = gravity_scale <= 0.001;
        // Sprite metadata supersedes the LDtk spawn box (see the old
        // `NpcClusterScratch`): size the collision to the visible body and
        // remember the render-quad size so the sprite still draws at scale.
        let ldtk_collision = aabb.half_size() * 2.0;
        let body = character_id.and_then(|cid| {
            ambition_sprite_sheet::character::catalog_join::sprite_body_collision_for_character_id_from_data(
                authored,
                catalog.data(),
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
        // Body capability comes from authored character data when available;
        // patrol/chase speed remains controller policy. A possessed NPC can use
        // its physical top speed without changing its autonomous stroll. Respawn
        // policy remains placement-owned rather than character-owned.
        let authored_body = character_id
            .and_then(|cid| prepared.and_then(|prepared| prepared.get(cid)))
            .and_then(|prepared| prepared.body_blueprint().ok());
        // The pool this body spawns with, held as a local because
        // `BodyHealth` is the only thing that keeps it (AC6.2): `ActorTuning`
        // carried a `max_health` beside it, and the two were written
        // independently.
        let max_health = authored_body.as_ref().map_or(
            ambition_characters::actor::DEFAULT_UNAUTHORED_BODY_HEALTH,
            |body| body.max_health,
        );
        let tuning = ambition_combat::actor_tuning::ActorTuning {
            patrol_speed: ambition_characters::brain::NPC_PATROL_SPEED,
            chase_speed: ambition_characters::brain::NPC_PATROL_SPEED,
            max_run_speed: authored_body
                .as_ref()
                .map_or(ambition_platformer2d_core::MAX_RUN_SPEED, |body| {
                    body.locomotion.run_speed
                }),
            is_aerial,
            // STATED, not inherited from `Default`: an NPC is a unique named
            // placement, so its death is permanent (ADR 0022) even after it
            // provokes into a mob archetype authored `OnRoomReenter`. This is
            // the pin protects: `ActorTuning` carries no respawn field, so the
            // archetype projection cannot supply one and the placement's
            // `RespawnPolicy` below is the only answer. (It used to name an
            // `ActorTuning::adopting_archetype` field (cite-ok: naming the dead
            // field is the point); respawn moved onto the
            // placement in `8fec52282` and the field went with it.)
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
        // ONE CONSTRUCTION PATH: A MIGRATED NPC IS BUILT FROM ITS
        // CHARACTER, then dressed as a placement (P1.10).
        //
        // patching two fields onto the peaceful seed was never the finish
        // line, and the fields it did NOT patch say why: this road hands every
        // body `AbilitySet::NONE`, `CombatCapabilities::default()`, no contact
        // damage, no `surface_walker`, no `ranged_visual` and a default brain
        // profile. So an exploding mite standing in a room could not explode, a
        // crawler did not cling, and a character's authored projectile came out
        // unadorned — each of them a fact its definition states and this
        // constructor threw away, one field at a time, invisibly.
        //
        //  when the character can carry a body, `new_character_in` builds it —
        // the SAME constructor the authored-enemy road and the match seat use.
        // What stays here is the part that is genuinely about the PLACEMENT.
        if let Some(body) = authored_body {
            let mut seed = Self::new_character_in(
                authored,
                catalog,
                id,
                body,
                aabb,
                config_brain.clone(),
                paths,
            );
            // THE PLACEMENT'S THREE FACTS, and only those.
            //
            // `new_character_in` defaults it true because every match seat is a combatant; an
            // NPC placement is the other answer, and the aggression component
            // `NpcActorSpawnPlan::peaceful` sets is the same claim said to the brain.
            seed.config.tuning.is_hostile = false;
            // The patrol PATH is authored on the interactable, and a body that
            // starts on one starts at its first waypoint.
            if let Some(start) = motion.as_ref().and_then(PathMotion::start_pos) {
                seed.place_at(start);
            }
            seed.motion = ActorMotionPath(motion);
            // Presentation identity: an NPC resolves its sheet through the
            // catalog id it named, exactly as it did before.
            seed.config.sprite_character_id = character_id.map(String::from);
            seed.hurt_feedback = actor_hurt_feedback(catalog, character_id);
            // `respawn` is already `DeadStaysDead` on that road — a match
            // seat's death is the match's business and an NPC's is permanent
            // (ADR 0022), and the two happen to agree. Stated here so the
            // agreement is a fact somebody checked rather than a coincidence.
            debug_assert_eq!(
                seed.config.tuning.respawn,
                ambition_entity_catalog::placements::RespawnPolicy::DeadStaysDead,
                "an NPC placement's death is permanent (ADR 0022)"
            );
            // ⛔ THE SEED'S OWN RESOLUTION WINS. `render_size` above is the
            // catalog join's answer, computed before we knew this placement
            // names a character that authors its body. When it does, the seed
            // has already resolved all three geometry facts from the sheet,
            // and handing the caller the catalog's render size beside the
            // sheet's collision size is the two-derivations defect on the
            // peaceful road.
            let render_size = seed.posed.map(|geometry| geometry.render).or(render_size);
            return (seed, render_size);
        }
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
            // THE POOL HAS ONE OWNER (AC6.2). This read `tuning.max_health` — itself
            // introduced (P1.10) to stop a second literal `1` written beside this one from
            // agreeing by coincidence. `BodyHealth` is where a body's health lives, for the
            // player and for every actor, so it is the only thing that holds it now.
            health: ambition_characters::actor::BodyHealth::new(
                ambition_characters::actor::Health::new(max_health),
            ),
            // An NPC placement is nobody's practice target; a character that IS
            // one reaches the character road below.
            combat: ambition_characters::actor::BodyCombat::default(),
            surface: ActorSurfaceState {
                surface_normal: ae::Vec2::new(0.0, -1.0),
                gravity_scale,
            },
            attack: BodyMelee::default(),
            // ⭐ THE AUTHORED GRAVITY SCALE IS RECORDED, not re-derived. Three
            // sites used to spell `if is_aerial { 0.0 } else { 1.0 }` — here,
            // `reset_to_spawn`, and the mount dismount — which is two
            // representations of one authored fact agreeing by convention.
            spawn: SpawnBaseline {
                pos,
                size: collision_size,
                gravity_scale,
            },
            config: ActorConfig {
                id: id.into(),
                name: name.into(),
                tuning,
                brain_profile: ambition_combat::actor_tuning::BrainProfile::default(),
                brain: config_brain,
                sprite_override_npc_name: None,
                // Peaceful actors already resolved their catalog id above.
                sprite_character_id: character_id.map(String::from),
                // this road takes no `CharacterBodyBlueprint`, so no authored
                // character trait reaches it. A peaceful catalog NPC has no CPU
                // fighter brain to give a stream to, so `false` is the answer
                // rather than a gap — and if this road ever grows a fighter, it
                // grows a blueprint first.
                preserves_mirror_symmetry: false,
            },
            motion: ActorMotionPath(motion),
            // A floating catalog body (the stochastic parrot) flies through the
            // shared flight limb from spawn; a grounded NPC runs the grounded spine.
            // ⚠ THIS ROAD DOES NOT ASK THE CHARACTER. It passed
            // `AbilitySet::NONE` and the constructor's union turned that into
            // the floor, so the default was arriving disguised as an authored
            // answer. Naming it changes nothing today and makes the gap
            // visible: a catalog NPC that authors an ability set is not read
            // here (`new_character_in` is the road that reads one).
            body: ActorBody::from_abilities(
                ActorBody::default_actor_abilities(),
                is_aerial,
                collision_size,
            ),
            caps: ambition_combat::CombatCapabilities::default(),
            hurt_feedback: actor_hurt_feedback(catalog, character_id),
            // This is the road for a placement that names no character it can
            // build a body from, so there is no `BodySource` to have resolved.
            posed: None,
        };
        (seed, render_size)
    }

    /// A BODY, BUILT FROM ITS CHARACTER.
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
    /// the tuning is a FIGHTER default, not a character fact — yet. Run
    /// speed, contact damage and the rest still have no authoring surface on a
    /// definition, so they are stated here, once, where a match
    /// can see them, rather than borrowed from whichever archetype a seat
    /// happened to name. Each becomes a character fact as its field lands.
    pub fn new_character_in(
        authored: &ambition_sprite_sheet::character::sheets::AuthoredSheets,
        catalog: &CharacterCatalog,
        id: impl Into<String>,
        // THE PREPARED CHARACTER, AS ONE VALUE.
        //
        // this took eleven pre-unpacked arguments — character id, display
        // name, max health, brain profile, locomotion, contact damage, dream
        // seed, practice target — every one of them already resolved on the same
        // prepared definition, re-listed by every caller. That is a
        // hand-assembled projection rather than
        // `prepared definition + context → body`, and its cost is not
        // aesthetic: each new character fact meant a new parameter and a new
        // chance for one road to pass it and another to forget, which is exactly
        // how `practice_target`, the patrol path and an authored `run_speed: 0.0`
        // each went missing on this road while being correct on the other.
        //
        // completeness is the blueprint's EXISTENCE, so nothing in here
        // asks whether the character said enough — see
        // `PreparedCharacterDefinition::body_blueprint`.
        body: ambition_characters::prepared::CharacterBodyBlueprint<'_>,
        aabb: ae::Aabb,
        config_brain: ambition_entity_catalog::placements::CharacterBrain,
        // The room's authored kinematic paths, so a `Patrol { path_id }`
        // placement gets its path — see the note at [`Self::motion`] below.
        paths: &[(String, ambition_platformer2d_core::KinematicPath)],
    ) -> Self {
        let ambition_characters::prepared::CharacterBodyBlueprint {
            character_id,
            display_name,
            max_health,
            locomotion,
            contact_damage,
            dream_seed,
            preserves_mirror_symmetry,
            practice_target,
            autonomous_profile,
            abilities,
            ranged_vfx,
            body: body_source,
            sheet,
            ..
        } = body;
        // a body with no policy still needs one to be paced against. The
        // default is the shared middling striker, which is what every
        // character-first body got before a definition could name a profile.
        let brain_profile = autonomous_profile.unwrap_or_default();
        // The AUTHORED silhouette, resolved exactly as a peaceful NPC of the
        // same character resolves it — one body per character, however it is
        // spawned.
        let ldtk_collision = aabb.half_size() * 2.0;
        // ⛔⛤ **THE AUTHORED BODY SOURCE FIRST, AND IT IS THE SAME RESOLUTION
        // THE LIVE POSE PASS USES — 2026-09-21.** A `SpriteAuthored` character
        // states the scale its art is drawn at, and `sync_sprite_posed_bodies`
        // sizes it every tick from `posed_body_geometry`. Construction used to
        // ignore that and ask the catalog/sheet join instead, which answers
        // from the catalog's standing height — so a body was built at one
        // scale and resized to another on its first pose tick. For Mary-O's
        // Solid Snake that was 108x48 becoming 21.3x9.5, with the render size
        // following a tick later still; presentation binding inside that window
        // latched a quad five times too big and nothing invalidated it.
        //
        // ⚠ `Idle` is the STANDING pose, which is what a spawn is and what
        // `base_size` is restored to — the same pose that pass reads for
        // `base_size`. Asking one function is the point: two derivations of one
        // authored scale is the defect, not the arithmetic in either.
        let posed = match (body_source, sheet) {
            (
                Some(ambition_characters::actor::definition::BodySource::SpriteAuthored {
                    world_per_pixel,
                }),
                Some(sheet),
            ) => ambition_sprite_sheet::character::sheets::posed_body_geometry(
                sheet,
                ambition_sprite_sheet::character::CharacterAnim::Idle,
                *world_per_pixel,
            ),
            _ => None,
        };
        let sprite_body = ambition_sprite_sheet::character::catalog_join::sprite_body_collision_for_character_id_from_data(
            authored,
            catalog.data(),
            character_id,
            ldtk_collision,
        );
        let collision_size = posed
            .map(|geometry| geometry.collision)
            .unwrap_or_else(|| sprite_body.map_or(ldtk_collision, |body| body.collision));
        // ASKED ONCE, AT PREPARATION. This read
        // `locomotion.baseline_free_flight || catalog.body_kind(character_id) == Floating` — a
        // constructor rediscovering what the character is.
        // The fold now happens in `finalize_character`, so `flies` on a prepared
        // character already carries the catalog's answer for every body that did
        // not state one.
        // silence reads as GROUNDED here: preparation has already folded the
        // catalog answer in, so an unresolved  at construction means no
        // authority said this body flies.
        let is_aerial = locomotion.baseline_free_flight.unwrap_or(false);
        let pos = actor_spawn_center_for_collision(aabb, collision_size);
        // THE CHARACTER'S OWN TOP SPEED WHEN IT STATES ONE. A fighter
        // default otherwise: the stage has to give a body that has never said
        // how fast it is SOMETHING, and a match is the one place that may.
        // an authored `0.0` is a SPEED, not a silence. This filtered
        // zeroes out and fell through to the stage default, which conflates *"I
        // did not say"* with *"I do not move"* — the same conflation P0.1 exists
        // to delete, and `CharacterLocomotion::run_speed`'s own doc says a zero
        // is meant to stand still visibly. The giant GNU is the case: a
        // stationary mount that authors 0.0 would have been handed a sprinter's
        // top speed. Only an ABSENT locomotion block takes the default.
        let run_speed = locomotion.run_speed;
        let tuning = ambition_combat::actor_tuning::ActorTuning {
            // the PROFILE's pacing against the BODY's top speed — §4.7's
            // brain→body seam, both halves finally stated by their own
            // authority. These were `run_speed * 0.5` and `run_speed`, hard
            // coded, so every character-first body ambled at exactly half pace
            // whatever its archetype row had said: `pirate_shark_rider`'s
            // tuned 0.4783 and `medium_striker`'s 0.44 would both have been
            // silently rounded to one shared number by migrating them.
            patrol_speed: run_speed * brain_profile.patrol_effort,
            chase_speed: run_speed * brain_profile.chase_effort,
            max_run_speed: run_speed,
            // Touching a body hurts only if its CHARACTER says so. A fighter
            // that authors none is safe to stand next to, which is what every
            // fighter has been.
            contact_strength: contact_damage.map_or(0.0, |contact| contact.strength),
            damage_amount: contact_damage.map_or(0, |contact| contact.amount),
            body_contact_damage: contact_damage.is_some(),
            dream_seed,
            surface_walker: locomotion.surface_walker,
            cling_breaks_on_hit: locomotion.cling_breaks_on_hit,
            // A match seat is a combatant whoever drives it; the disposition the
            // body carries is set by realization, and this is the tuning half.
            is_hostile: true,
            // a fighter's death is the MATCH's business (stocks, blast zones),
            // never a room's respawn policy.
            respawn: ambition_entity_catalog::placements::RespawnPolicy::DeadStaysDead,
            is_aerial,
            // WHAT THIS CHARACTER'S PROJECTILE LOOKS LIKE.
            //
            // `brain_effects` reads `tuning.ranged_visual` when it spawns the shot; the
            // archetype road filled that in and this road left it empty.
            ranged_visual: ranged_vfx.unwrap_or_default().to_string(),
            ..Default::default()
        };
        // ONE spelling of the authored scale, read by the live surface state
        // AND recorded on the baseline a reset restores.
        let gravity_scale = if is_aerial { 0.0 } else { 1.0 };
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
            // THE CHARACTER'S OWN `practice_target`, written ONCE (AC6.2).
            combat: ambition_characters::actor::BodyCombat {
                training_dummy: practice_target,
                ..Default::default()
            },
            surface: ActorSurfaceState {
                surface_normal: ae::Vec2::new(0.0, -1.0),
                gravity_scale,
            },
            attack: BodyMelee::default(),
            // ⭐ THE AUTHORED GRAVITY SCALE IS RECORDED, not re-derived. Three
            // sites used to spell `if is_aerial { 0.0 } else { 1.0 }` — here,
            // `reset_to_spawn`, and the mount dismount — which is two
            // representations of one authored fact agreeing by convention.
            spawn: SpawnBaseline {
                pos,
                size: collision_size,
                gravity_scale,
            },
            config: ActorConfig {
                id: id.into(),
                name: display_name.to_string(),
                tuning,
                brain_profile,
                brain: config_brain.clone(),
                sprite_override_npc_name: None,
                // the CHARACTER, stated rather than resolved from a display
                // name. A seat knows exactly which character it is seating.
                sprite_character_id: Some(character_id.to_string()),
                // the character's own answer, carried on the blueprint —
                // so a seat, a room spawn and a rewind rebuild all give this
                // body the same cognitive stream.
                preserves_mirror_symmetry,
            },
            // a practice target is skipped for the same reason the archetype
            // road skips it: a dummy on a patrol path is a dummy that walks away
            // from the player practising on it.
            motion: ActorMotionPath(match &config_brain {
                ambition_entity_catalog::placements::CharacterBrain::Patrol {
                    path_id: Some(path_id),
                } if !practice_target => {
                    ambition_platformer2d_core::resolve_kinematic_path(paths, path_id)
                        .map(|path| PathMotion::new(path.clone()))
                }
                _ => None,
            }),
            // THE CHARACTER'S OWN VERBS, when it authored any.
            //
            // this granted `AbilitySet::NONE` unconditionally, on the reading
            // that the MATCH declares what a fighter may do (`seat_abilities`)
            // and writes the real set in the same flush. That is true of a
            // SEATED body and false of every other one built here: the duel
            // arena's exhibition robot is a character-first ROOM actor, and it
            // came out unable to blink, shield or dash — abilities its archetype
            // row had granted and its character now states.
            //
            // a seat is unaffected: `seat_abilities` still intersects, and a
            // character that authored nothing still gets `NONE` here and the
            // mode's set there.
            // THE CHARACTER'S OWN SET, OR THE RULESET'S DEFAULT — never both.
            // `unwrap_or(NONE)` fed a union that could only widen, so a
            // character that authored a narrow body got the default anyway.
            body: ActorBody::from_abilities(
                abilities.unwrap_or_else(ActorBody::default_actor_abilities),
                is_aerial,
                collision_size,
            ),
            // Death traits are the character's and arrive with the persona
            // derive, like its moves — a seed that guessed them would be a
            // second writer.
            caps: ambition_combat::CombatCapabilities::default(),
            hurt_feedback: actor_hurt_feedback(catalog, Some(character_id)),
            // Resolved above, beside the collision size, and carried rather
            // than re-derived — see the field's own note.
            posed,
        }
    }
    pub fn into_components(self) -> ActorClusterBundle {
        // Project the actor's authored weight onto the combat-owned carrier at
        // spawn (E2 verdict b): the damage paths read `CombatTuning`, never the
        // sim-heart `ActorConfig`.
        let combat_tuning = ambition_combat::CombatTuning {
            weight: self.config.tuning.weight,
            attack_cooldown_mult: self.config.brain_profile.attack_cooldown_mult,
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
            self.spawn,
            self.motion,
            self.surface,
            self.attack,
            AncillaryMovementBundle::from_scratch(self.body.0),
            self.caps,
            combat_tuning,
        )
    }
}

/// A NEUTRAL fixture body, for engine unit tests that need an actor and do
/// not care which creature it is.
///
/// It states the facts the deleted `combatant` row stated, so the bodies these
/// tests measure did not change shape when the row went: 4 HP, a 155 px/s run
/// paced at 0.6774 idle / 1.0 engaged, and a 1-damage body contact.
#[cfg(any(test, feature = "test-support"))]
pub fn fixture_body_blueprint(
    display_name: &str,
) -> ambition_characters::prepared::CharacterBodyBlueprint<'_> {
    ambition_characters::prepared::CharacterBodyBlueprint {
        character_id: "fixture_body",
        display_name,
        max_health: 4,
        locomotion: ambition_characters::actor::CharacterLocomotion {
            run_speed: 155.0,
            ..Default::default()
        },
        contact_damage: Some(ambition_characters::actor::ContactDamage {
            strength: 0.70,
            amount: 1,
        }),
        dream_seed: None,
        // A fixture body is nobody's twin.
        preserves_mirror_symmetry: false,
        practice_target: false,
        autonomous_profile: Some(ambition_characters::brain::BrainProfile {
            patrol_effort: 0.6774,
            chase_effort: 1.0,
            aggro_radius: 460.0,
            attack_range: 150.0,
            ..Default::default()
        }),
        mount: None,
        held_item: None,
        death_traits: None,
        abilities: None,
        ranged_vfx: None,
        body: None,
        sheet: None,
    }
}

#[cfg(any(test, feature = "test-support"))]
impl ActorClusterSeed {
    /// Content-free convenience constructor for unit tests: a NEUTRAL body,
    /// built by the ONE production constructor.
    ///
    /// The numbers here are the ones the deleted `combatant` row carried, so a
    /// test that did not care which row it got sees the body it always saw.
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        aabb: ae::Aabb,
        brain: ambition_entity_catalog::placements::CharacterBrain,
        paths: &[(String, ambition_platformer2d_core::KinematicPath)],
    ) -> Self {
        let name: String = name.into();
        Self::new_character_in(
            &Default::default(),
            &CharacterCatalog::empty(),
            id,
            fixture_body_blueprint(&name),
            aabb,
            brain,
            paths,
        )
    }

    /// Content-free peaceful-NPC constructor for unit tests.    /// Content-free peaceful-NPC constructor for unit tests.
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
            // A content-free constructor registers no characters either, so the
            // catalog rule is the only one that can apply.
            None,
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
    /// A blueprint the way a prepared character produces one, for the
    /// construction tests.
    #[cfg(test)]
    fn test_blueprint<'a>(
        character_id: &'a str,
        display_name: &'a str,
        max_health: i32,
        locomotion: ambition_characters::actor::CharacterLocomotion,
        profile: ambition_combat::actor_tuning::BrainProfile,
        practice_target: bool,
    ) -> ambition_characters::prepared::CharacterBodyBlueprint<'a> {
        ambition_characters::prepared::CharacterBodyBlueprint {
            character_id,
            display_name,
            max_health,
            locomotion,
            contact_damage: None,
            dream_seed: None,
            preserves_mirror_symmetry: false,
            practice_target,
            autonomous_profile: Some(profile),
            mount: None,
            held_item: None,
            death_traits: None,
            abilities: None,
            ranged_vfx: None,
            body: None,
            sheet: None,
        }
    }

    /// A CHARACTER-FIRST BODY FIRES ITS OWN PROJECTILE, NOT A ROCK.
    ///
    /// `new_character_in` destructured `ranged_vfx` off the blueprint and never used it, so
    /// `ActorTuning:ranged_visual` — the field `brain_effects` reads when it spawns the shot —
    /// stayed EMPTY for every character-first body.
    ///
    /// `CharacterDefinition::ranged_vfx` had exactly one reader in the
    /// repository and it was a TEST asserting the value is authored. It was:
    /// authored, carried, and dropped one step from use. An "is it authored"
    /// assertion cannot see that.
    #[test]
    fn a_characters_authored_projectile_art_reaches_its_tuning() {
        let catalog = ambition_characters::actor::character_catalog::CharacterCatalog::from_data(
            ambition_characters::actor::character_catalog::parse_catalog(
                "(autonomous_profiles: {}, brain_presets: {}, action_set_presets: {}, characters: {})",
            ),
        );
        let authored = ambition_sprite_sheet::character::sheets::AuthoredSheets::default();
        let seed_for = |vfx: Option<&str>| {
            let mut blueprint = test_blueprint(
                "gunner",
                "Gunner",
                10,
                Default::default(),
                Default::default(),
                false,
            );
            blueprint.ranged_vfx = vfx;
            ActorClusterSeed::new_character_in(
                &authored,
                &catalog,
                "gunner",
                blueprint,
                ae::aabb_from_min_size(ae::Vec2::ZERO, ae::Vec2::new(32.0, 32.0)),
                ambition_entity_catalog::placements::CharacterBrain::Custom("idle".to_string()),
                &[],
            )
        };

        assert_eq!(
            seed_for(Some("hadouken")).config.tuning.ranged_visual,
            "hadouken",
            "the character's authored projectile art never reached the body, so \
             its shot is drawn as the default rock"
        );
        assert_eq!(
            seed_for(None).config.tuning.ranged_visual,
            "",
            "a character that authors NO projectile art was given one"
        );
    }

    use super::*;

    /// **A SPRITE-AUTHORED BODY IS BUILT FROM THE SHEET, NOT FROM THE CATALOG.**
    ///
    /// `sync_sprite_posed_bodies` sizes a `BodySource::SpriteAuthored` body every
    /// tick from `posed_body_geometry`. Construction asked the catalog join
    /// instead, which answers from the catalog's standing height — so the body
    /// was BUILT at one scale and RESIZED to another on its first pose tick, and
    /// presentation binding inside that window latched the wrong quad with
    /// nothing left to invalidate it.
    ///
    /// The fix is not to make the binder recover. It is that there is only one
    /// resolution of one authored scale, so the two answers cannot differ — this
    /// asserts they are the SAME CALL's answer, not that they are merely close.
    #[test]
    fn a_sprite_authored_body_is_constructed_from_its_sheet() {
        const SHEET: &str = "robot";
        const WORLD_PER_PIXEL: f32 = 0.5;

        let authored = ambition_sprite_sheet::character::sheets::AuthoredSheets::default();
        let catalog = CharacterCatalog::empty();
        // Deliberately NOT the sheet's answer, so a constructor that ignores the
        // body source cannot pass by coincidence.
        let ldtk = ae::aabb_from_min_size(ae::Vec2::ZERO, ae::Vec2::new(108.0, 48.0));

        let seed_for = |body: Option<&ambition_characters::actor::definition::BodySource>| {
            let mut blueprint = test_blueprint(
                "sheet_body",
                "Sheet Body",
                10,
                Default::default(),
                Default::default(),
                false,
            );
            blueprint.body = body;
            blueprint.sheet = Some(SHEET);
            ActorClusterSeed::new_character_in(
                &authored,
                &catalog,
                "sheet_body",
                blueprint,
                ldtk,
                ambition_entity_catalog::placements::CharacterBrain::Custom("idle".to_string()),
                &[],
            )
        };

        let from_the_sheet = ambition_sprite_sheet::character::sheets::posed_body_geometry(
            SHEET,
            ambition_sprite_sheet::character::CharacterAnim::Idle,
            WORLD_PER_PIXEL,
        )
        .expect("the baked `robot` sheet resolves a posed body");

        // ANTI-VACUITY: the sheet's answer must differ from the box the
        // placement carries, or "construction used the sheet" is unfalsifiable.
        let placement_size = ldtk.half_size() * 2.0;
        assert!(
            from_the_sheet.collision.distance(placement_size) > 1.0,
            "the sheet resolves {:?}, which is the placement box {placement_size:?} —              this fixture cannot tell the two authorities apart",
            from_the_sheet.collision
        );

        let source = ambition_characters::actor::definition::BodySource::SpriteAuthored {
            world_per_pixel: WORLD_PER_PIXEL,
        };
        let built = seed_for(Some(&source)).kin.size;
        assert_eq!(
            built, from_the_sheet.collision,
            "a `SpriteAuthored` body was constructed at {built:?} while the pose              pass sizes it at {:?} — two derivations of one authored scale, and              the body is resized out from under whatever already bound it",
            from_the_sheet.collision
        );

        // ALL THREE FACTS, NOT ONLY THE BOX. The render quad and the quad
        // offset are the other two thirds of the same resolution, and the
        // spawn sites seed the body's `ActorRenderSize` / `ActorSpriteOffset`
        // from them (`spawn_render_geometry`). Asserting only the collision
        // size would leave the catalog join answering for the other two — the
        // exact split this slice exists to close, one component over.
        assert_eq!(
            seed_for(Some(&source)).posed,
            Some(from_the_sheet),
            "the seed did not carry the resolved presentation geometry, so the \
             spawn site has nothing to seed the quad and offset from and falls \
             back to the catalog join"
        );

        // THE CONTROL: a character that authors NO body source still gets the
        // placement's box, so the arm above is about the source being consumed
        // rather than about the sheet being consulted unconditionally.
        let unsourced_seed = seed_for(None);
        assert_eq!(
            unsourced_seed.posed, None,
            "a body that authors no source carries a resolved sheet geometry"
        );
        let unsourced = unsourced_seed.kin.size;
        assert_eq!(
            unsourced, placement_size,
            "a body that authors no source was sized from a sheet it never named"
        );
    }

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

    /// Character-first construction must preserve profile-authored patrol/chase
    /// pacing and hostility. The fixture uses deliberately distinct values so a
    /// constructor that substitutes defaults cannot pass by coincidence.
    #[test]
    fn a_character_first_body_paces_and_targets_by_its_profile() {
        const CATALOG: &str = r#"(
            brain_presets: { "idle": StandStill },
            action_set_presets: { "peaceful": (move_style: Walk) },
            characters: {},
        )"#;
        let catalog = CharacterCatalog::from_data(
            ambition_characters::actor::character_catalog::parse_catalog(CATALOG),
        );
        let authored = ambition_sprite_sheet::character::sheets::AuthoredSheets::default();
        let locomotion = ambition_characters::actor::CharacterLocomotion {
            run_speed: 200.0,
            ..Default::default()
        };
        let profile = ambition_combat::actor_tuning::BrainProfile {
            patrol_effort: 0.25,
            chase_effort: 0.75,
            ..Default::default()
        };
        let seed = ActorClusterSeed::new_character_in(
            &authored,
            &catalog,
            "gnu",
            test_blueprint("giant_gnu", "Giant GNU", 42, locomotion, profile, false),
            ae::aabb_from_min_size(ae::Vec2::ZERO, ae::Vec2::new(40.0, 40.0)),
            ambition_entity_catalog::placements::CharacterBrain::Custom("idle".to_string()),
            &[],
        );
        assert_eq!(
            seed.config.tuning.max_run_speed, 200.0,
            "the BODY's capability"
        );
        assert_eq!(
            seed.config.tuning.patrol_speed, 50.0,
            "0.25 of it, not half"
        );
        assert_eq!(seed.config.tuning.chase_speed, 150.0, "0.75 of it, not all");
        assert!(
            seed.config.tuning.is_hostile,
            "construction has no relationship to state, so it states the ordinary \
             one; the mount's PLACEMENT is what says `Peaceful`, and a policy \
             that answered this instead is the thing §6 deleted"
        );

        // An authored ZERO is a speed. A stationary mount that says so must
        // not be handed the stage's sprinter default.
        let still = ActorClusterSeed::new_character_in(
            &authored,
            &catalog,
            "gnu",
            test_blueprint(
                "giant_gnu",
                "Giant GNU",
                42,
                ambition_characters::actor::CharacterLocomotion::default(),
                profile,
                false,
            ),
            ae::aabb_from_min_size(ae::Vec2::ZERO, ae::Vec2::new(40.0, 40.0)),
            ambition_entity_catalog::placements::CharacterBrain::Custom("idle".to_string()),
            &[],
        );
        assert_eq!(
            still.config.tuning.max_run_speed, 0.0,
            "a body that authored 0.0 was given the stage default instead"
        );
        assert!(
            !still.combat.training_dummy,
            "an ordinary body is not a practice target"
        );

        let dummy = ActorClusterSeed::new_character_in(
            &authored,
            &catalog,
            "dummy",
            test_blueprint(
                "sandbag",
                "Sandbag",
                6,
                ambition_characters::actor::CharacterLocomotion::default(),
                profile,
                true,
            ),
            ae::aabb_from_min_size(ae::Vec2::ZERO, ae::Vec2::new(40.0, 40.0)),
            ambition_entity_catalog::placements::CharacterBrain::Custom("idle".to_string()),
            &[],
        );
        assert!(
            dummy.combat.training_dummy,
            "a character that authors itself a training dummy did not reach the body"
        );
    }

    /// A CHARACTER-FIRST BODY STILL WALKS ITS PLACEMENT'S PATROL PATH.
    ///
    /// this constructor wrote `ActorMotionPath(None)` unconditionally while
    /// the archetype road resolved `Patrol { path_id }` into a `PathMotion`, so
    /// the first patrolling creature to migrate would have stopped patrolling —
    /// and a body standing still looks exactly like a body whose path was
    /// authored badly, so there would have been nothing to read. No migrated
    /// placement uses `Patrol` today, which is precisely why it was invisible.
    ///
    /// the practice-target control is the archetype road's rule kept: a dummy
    /// on a patrol path is a dummy that walks away from whoever is practising.
    #[test]
    fn a_character_first_body_walks_the_patrol_path_its_placement_names() {
        let catalog = CharacterCatalog::from_data(
            ambition_characters::actor::character_catalog::parse_catalog(
                r#"(brain_presets: {}, action_set_presets: {}, characters: {})"#,
            ),
        );
        let authored = ambition_sprite_sheet::character::sheets::AuthoredSheets::default();
        let paths = vec![(
            "lab_patrol_line".to_string(),
            ambition_platformer2d_core::KinematicPath::line(
                ae::Vec2::new(0.0, 0.0),
                ae::Vec2::new(120.0, 0.0),
                60.0,
            ),
        )];
        let seed = |practice_target: bool| {
            ActorClusterSeed::new_character_in(
                &authored,
                &catalog,
                "slug",
                test_blueprint(
                    "npc_puppy_slug",
                    "Puppy Slug",
                    2,
                    ambition_characters::actor::CharacterLocomotion::default(),
                    ambition_combat::actor_tuning::BrainProfile::default(),
                    practice_target,
                ),
                ae::aabb_from_min_size(ae::Vec2::ZERO, ae::Vec2::new(32.0, 48.0)),
                ambition_entity_catalog::placements::CharacterBrain::Patrol {
                    path_id: Some("lab_patrol_line".to_string()),
                },
                &paths,
            )
        };
        assert!(
            seed(false).motion.0.is_some(),
            "the placement named a path and the body did not take it"
        );
        assert!(
            seed(true).motion.0.is_none(),
            "a practice target must not walk away from the player practising on it"
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
    // that rule has no code left to govern. Every registered character can build its own body
    // and every shipped placement names one, so a body is built from a character or from an
    // archetype and never from one patched over the other.

    /// **A BODY CONSTRUCTOR MAY NOT WIDEN WHAT A CHARACTER AUTHORED.**
    ///
    /// `from_kit` resolved a body as `locomotion_abilities().union(kit)` with
    /// `attack` then forced true. A union cannot express a refusal, so every
    /// authored `false` on a floor bit was unreachable: `npc_puppy_slug` authors
    /// `{ move_horizontal: true, ..AbilitySet::NONE }` and got a jump, a double
    /// jump and an attack; the perfect cellular automaton's own comment says
    /// *"It has no double jump, no fast fall, no dodge and no ledge grab"* while
    /// its body had one.
    ///
    /// It also made [`ActionSet::gated_by`]'s attack term dead for actors — the
    /// gate asks `abilities.attack` and the constructor had just set it.
    ///
    /// ⚠ THE EDIT THAT MAKES THIS FALSE is reintroducing a union or a forced
    /// field in [`ActorBody::from_abilities`]; it fails naming the bit that came
    /// back.
    #[test]
    fn an_authored_body_is_not_widened_by_the_constructor() {
        // The slug's own authored set, verbatim: it walks and does nothing else.
        let slug = ae::AbilitySet {
            move_horizontal: true,
            ..ae::AbilitySet::NONE
        };
        let built = ActorBody::from_abilities(slug, false, ae::Vec2::new(16.0, 16.0))
            .0
            .abilities
            .abilities;

        assert!(
            built.move_horizontal,
            "fixture: the authored ability must SURVIVE, or this arm cannot tell \
             a constructor that widens from one that discards"
        );
        assert!(
            !built.jump && !built.double_jump && !built.attack,
            "the constructor conferred a capability this character declined \
             (jump={}, double_jump={}, attack={}) — an authored body cannot be \
             widened, or authoring a narrow creature is impossible",
            built.jump,
            built.double_jump,
            built.attack,
        );
    }

    /// And the DEFAULT still reaches a character that states nothing, which is
    /// the arm that stops the one above passing on a constructor that simply
    /// discards its argument.
    #[test]
    fn a_character_that_states_nothing_gets_the_rulesets_default_body() {
        let default = ActorBody::default_actor_abilities();
        assert!(
            default.move_horizontal && default.jump && default.double_jump && default.attack,
            "the ruleset default cannot act, so an unauthored body is now inert \
             rather than defaulted"
        );
        assert!(
            !default.reset,
            "only a controlled body resets itself; an actor taking `reset` from \
             `basic()` is the floor leaking back in"
        );
    }
}

#[cfg(test)]
mod npc_flight_tests;

