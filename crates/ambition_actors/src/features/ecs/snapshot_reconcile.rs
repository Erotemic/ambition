//! Post-restore reconciliation of an actor's **autonomous configuration** from
//! its restored [`BrainBinding`] source.
//!
//! The snapshot registers the small, stable facts (the [`BrainBinding`] source,
//! disposition, health, gravity) but NOT the whole archetype config an actor
//! carries — its `ActorConfig` tuning / brain-spec, `CombatCapabilities`, and
//! `ActionSet`. That config is a deterministic function of the autonomous source
//! plus the actor's durable combat kit, so rather than serialize it, a rewind
//! RECONSTRUCTS it here, the same way spawn / provocation build it live. This is
//! what makes provocation rollback-correct in BOTH directions:
//!
//! - Rewind INTO a provoked snapshot ([`AutonomousSource::Provoked`]): rerun
//!   the roster archetype construction ([`project_provoked_archetype`], shared
//!   with the live provoke flip) to rebuild the hostile brain / action set /
//!   tuning / capabilities from the archetype id the binding retained.
//! - Rewind to BEFORE a challenge (a catalog source over a still-hostile config):
//!   restore the peaceful catalog config the character spawned with.
//!
//! The live `Brain` for a CATALOG source is rebuilt by the catalog pass
//! (`snapshot::codecs::reconcile_brain_bindings`, which runs first); this pass
//! owns the coupled CONFIG for catalog sources and the whole autonomous state for
//! provoked sources. Registered facts (disposition, health, gravity) are left to
//! their own blobs — this pass never overwrites them.
//!
//! Bodies under temporary control (player possession / mount) are skipped; their
//! control is reconciled separately (see the temporary-control reconcile pass).

use bevy::prelude::*;

use super::actor_clusters::ActorConfig;
use super::mount::{MountSlot, Mounted, MountedBrainCache, RidingOn};
use super::{CombatKit, HeldItem};
use crate::abilities::traversal::possession::PossessionState;
use crate::combat::CombatCapabilities;
use crate::features::ecs::actor_tuning::{ActorTuning, CharacterBrainSpec};
use crate::features::enemies::{CharacterArchetypeSpec, CharacterRoster};
use crate::features::TemporaryControl;
use ambition_characters::actor::character_catalog::{
    AuthoredBrainContext, AutonomousSource, BrainBinding, BrainBuildContext, CharacterBodyKind,
    CharacterCatalog,
};
use ambition_characters::actor::pose::ActorPose;
use ambition_characters::actor::{BodyHealth, Health};
use ambition_characters::brain::{ActorControl, Brain, PlayerSlot, NPC_PATROL_SPEED};
use ambition_entity_catalog::placements::CharacterBrain;
use ambition_platformer_primitives::markers::PrimaryPlayer;
use ambition_platformer_primitives::sim_id::SimId;

/// The complete set of components a provoked hostile archetype installs on an
/// actor — the deterministic projection of `(archetype spec, current config,
/// combat kit, held item)`.
///
/// Both the live provoke flip (`provoke_actor_in_place`) and the post-restore
/// reconstruction apply this exact projection, so a provoked actor is identical
/// whether it was just challenged or rebuilt from a snapshot.
pub(crate) struct ProvokedArchetype {
    pub tuning: ActorTuning,
    pub brain_spec: CharacterBrainSpec,
    pub gravity_scale: f32,
    pub max_health: i32,
    pub capabilities: CombatCapabilities,
    pub sprite_override_npc_name: Option<String>,
    /// The `ActorConfig.brain` read-model marker for a provoked actor.
    pub config_brain: CharacterBrain,
    pub brain: Brain,
    pub action_set: ambition_characters::brain::ActionSet,
}

/// Project a hostile roster archetype onto an actor's config. Pure: no ECS, no
/// mutation — the single definition of "what provocation produces", so the live
/// flip and a snapshot rebuild can never drift.
///
/// `current_config` is the actor's config at call time; the projection clones it
/// only to hand the archetype's HOSTILE tuning / brain-spec / id to the brain
/// builder (which reads them), never to keep any of the caller's live values.
pub(crate) fn project_provoked_archetype(
    spec: &CharacterArchetypeSpec,
    archetype: &str,
    current_config: &ActorConfig,
    combat_kit: &CombatKit,
    held_item: Option<&HeldItem>,
) -> ProvokedArchetype {
    let tuning = spec.tuning();
    let brain_spec = spec.brain_spec();
    let config_brain = CharacterBrain::Custom(archetype.to_string());

    // The brain builder selects the concrete hostile brain from the actor's
    // config (id + the HOSTILE tuning/brain_spec), so hand it a config carrying
    // those. Every other field is irrelevant to the builder.
    let mut hostile_config = current_config.clone();
    hostile_config.tuning = tuning.clone();
    hostile_config.brain_spec = brain_spec;
    hostile_config.brain = config_brain.clone();
    let (brain, action_set) = super::brain_builders::aggressive_brain_and_action_set_for_enemy(
        &hostile_config,
        combat_kit,
        held_item,
    );

    // Keep the actor's own sprite sheet (its NPC name) when hostile — except the
    // Kernel Guide, which uses the default enemy sheet (legacy quirk). Mirrors
    // the live provoke flip.
    let sprite_override_npc_name = if current_config.name != "Kernel Guide NPC" {
        Some(current_config.name.clone())
    } else {
        None
    };

    ProvokedArchetype {
        gravity_scale: if tuning.is_aerial { 0.0 } else { 1.0 },
        max_health: spec.max_health,
        capabilities: spec.combat_capabilities(),
        sprite_override_npc_name,
        config_brain,
        brain,
        action_set,
        tuning,
        brain_spec,
    }
}

/// The fixed peaceful catalog config a catalog-backed NPC spawns with. Mirrors
/// `ActorClusterSeed::new_peaceful_npc_in`: a health-1 stroller with default
/// brain-spec / capabilities, its authored combat kit as body-capability action
/// set, and `is_aerial` from the character's catalog body kind. The only
/// non-constant input is `character_id` (for `is_aerial` + the resolved
/// `config.brain` read-model).
pub(crate) struct PeacefulConfig {
    pub(crate) tuning: ActorTuning,
    pub(crate) brain_spec: CharacterBrainSpec,
    pub(crate) capabilities: CombatCapabilities,
    pub(crate) action_set: ambition_characters::brain::ActionSet,
    pub(crate) config_brain: CharacterBrain,
}

pub(crate) fn peaceful_config(
    catalog: &CharacterCatalog,
    character_id: Option<&str>,
    combat_kit: &CombatKit,
    resolved_brain: &Brain,
) -> PeacefulConfig {
    let is_aerial = character_id
        .map(|cid| matches!(catalog.body_kind(cid), Some(CharacterBodyKind::Floating)))
        .unwrap_or(false);
    let tuning = ActorTuning {
        max_health: 1,
        patrol_speed: NPC_PATROL_SPEED,
        chase_speed: NPC_PATROL_SPEED,
        max_run_speed: ambition_engine_core::MAX_RUN_SPEED,
        is_aerial,
        ..Default::default()
    };
    // `config.brain` (the integrator read-model) is DERIVED from the resolved
    // autonomous brain through the SHARED helper the spawn plan and runtime switch
    // both use, so the classification can never disagree with the actual brain.
    let config_brain = crate::features::brain_command::config_brain_for(resolved_brain);
    PeacefulConfig {
        tuning,
        brain_spec: CharacterBrainSpec::default(),
        capabilities: CombatCapabilities::default(),
        // Body CAPABILITY: the peaceful autonomous brain never presses attack, but
        // a possessing player can still throw the kit's punch/swing — the same
        // action set the spawn plan installs (`combat_kit.to_action_set(None)`).
        action_set: combat_kit.to_action_set(None),
        config_brain,
    }
}

/// Rebuild each autonomous catalog-backed actor's CONFIG from its restored
/// [`BrainBinding`] source. Skips gracefully when the world lacks the roster /
/// catalog (headless fixtures) or an actor lacks an `ActorConfig` (the catalog
/// pass already handled its live `Brain`).
pub fn reconcile_autonomous_actors(world: &mut World) {
    // Phase A: temporary control. Restore the live control mode (player possession
    // / mount) from the snapshot-persisted `TemporaryControl`, so a rewind that
    // crossed a possess/release boundary lands the body — and the player's home
    // avatar — in the correct mode. Runs first so phase B skips controlled bodies.
    reconcile_temporary_control(world);

    struct Job {
        entity: Entity,
        source: AutonomousSource,
        character_id: Option<String>,
    }

    // Collect the autonomous, config-bearing actors. `query` (not `try_query`) so
    // the optional `Mounted` component type is initialized even in a world that
    // never spawned one (a `try_query` returns `None` there and silently skips).
    let jobs: Vec<Job> = {
        let mut q = world.query::<(
            Entity,
            &BrainBinding,
            &Brain,
            Option<&ActorConfig>,
            bevy::ecs::query::Has<crate::features::Mounted>,
        )>();
        q.iter(world)
            .filter_map(|(entity, binding, brain, config, mounted)| {
                // Temporary control is untouchable and owned by its own pass.
                if brain.is_player() || mounted {
                    return None;
                }
                let config = config?;
                Some(Job {
                    entity,
                    source: binding.source.clone(),
                    character_id: config.sprite_character_id.clone(),
                })
            })
            .collect()
    };
    if jobs.is_empty() {
        return;
    }

    for job in jobs {
        match &job.source {
            AutonomousSource::Provoked { archetype } => {
                reconstruct_provoked(world, job.entity, archetype.as_str());
            }
            AutonomousSource::CatalogDefault | AutonomousSource::CatalogPreset(_) => {
                restore_peaceful_config(world, job.entity, job.character_id.as_deref());
            }
            // A boss's autonomous BossPattern brain is snapshotted by the ordinary
            // brain codec (it is a `Brain` variant), and a boss carries no
            // `ActorConfig` — so it is filtered out of this config-reconstruction
            // loop above. Its temporary-control resumption is handled by the
            // suspended-autonomous-runtime pass; nothing to reconstruct here.
            AutonomousSource::Boss { .. } => {}
        }
    }
}

/// Rerun the roster archetype construction for a provoked actor and install the
/// coupled config (tuning / brain-spec / capabilities / sprite / read-model brain
/// / live brain / action set). Leaves the registered disposition / health /
/// gravity to their own restored blobs.
fn reconstruct_provoked(world: &mut World, entity: Entity, archetype: &str) {
    let Some(spec) = world
        .get_resource::<CharacterRoster>()
        .map(|roster| roster.spec_for_brain(&CharacterBrain::Custom(archetype.to_string())))
    else {
        // Headless fixture without a roster: leave the live brain to its authority.
        return;
    };
    let (Some(config), Some(kit)) = (
        world.get::<ActorConfig>(entity).cloned(),
        world.get::<CombatKit>(entity).cloned(),
    ) else {
        return;
    };
    let held = world.get::<HeldItem>(entity).cloned();
    let proj = project_provoked_archetype(&spec, archetype, &config, &kit, held.as_ref());

    let Ok(mut em) = world.get_entity_mut(entity) else {
        return;
    };
    if let Some(mut config) = em.get_mut::<ActorConfig>() {
        config.tuning = proj.tuning;
        config.brain_spec = proj.brain_spec;
        config.brain = proj.config_brain;
        config.sprite_override_npc_name = proj.sprite_override_npc_name;
    }
    em.insert((proj.brain, proj.action_set, proj.capabilities));
}

/// Restore the peaceful catalog config a catalog-backed NPC spawned with —
/// reverting a config left hostile by a provocation the rewind undid. Idempotent
/// for an NPC that was never provoked (it re-sets the same fixed peaceful values).
/// The live catalog brain is rebuilt by the catalog reconcile pass; this only
/// owns the coupled config, and `config.brain` is derived from that live brain.
fn restore_peaceful_config(world: &mut World, entity: Entity, character_id: Option<&str>) {
    let Some(kit) = world.get::<CombatKit>(entity).cloned() else {
        return;
    };
    let Some(brain) = world.get::<Brain>(entity).cloned() else {
        return;
    };
    let Some(peaceful) = world
        .get_resource::<CharacterCatalog>()
        .map(|catalog| peaceful_config(catalog, character_id, &kit, &brain))
    else {
        return;
    };

    let Ok(mut em) = world.get_entity_mut(entity) else {
        return;
    };
    if let Some(mut config) = em.get_mut::<ActorConfig>() {
        config.tuning = peaceful.tuning;
        config.brain_spec = peaceful.brain_spec;
        config.brain = peaceful.config_brain;
        config.sprite_override_npc_name = None;
    }
    em.insert((peaceful.action_set, peaceful.capabilities));
}

/// Reset a body's live `BodyHealth` to a fresh archetype pool — used by the live
/// provoke flip. Reconstruction leaves health to its snapshot blob.
pub(crate) fn fresh_health_pool(max_health: i32) -> BodyHealth {
    BodyHealth::new(Health::new(max_health))
}

/// The autonomous `Brain` an actor resumes when no controller masks it — rebuilt
/// from its [`BrainBinding`] source (catalog preset, or a rerun of the provoked
/// roster construction). The single seam possession/mount RELEASE resumes from
/// and reconciliation rebuilds a Player→Autonomous transition with, so "resume
/// the selected autonomous source" means the *current* source, never a stale
/// cache. Returns `None` in a fixture missing the catalog / roster / config.
pub(crate) fn autonomous_brain_for_source(world: &World, entity: Entity) -> Option<Brain> {
    let binding = world.get::<BrainBinding>(entity)?;
    match &binding.source {
        AutonomousSource::Provoked { archetype } => {
            let roster = world.get_resource::<CharacterRoster>()?;
            let spec =
                roster.spec_for_brain(&CharacterBrain::Custom(archetype.as_str().to_string()));
            let config = world.get::<ActorConfig>(entity)?;
            let kit = world.get::<CombatKit>(entity)?;
            let held = world.get::<HeldItem>(entity);
            Some(project_provoked_archetype(&spec, archetype.as_str(), config, kit, held).brain)
        }
        AutonomousSource::CatalogDefault | AutonomousSource::CatalogPreset(_) => {
            let catalog = world.get_resource::<CharacterCatalog>()?;
            let preset = binding.active_preset()?.as_str().to_string();
            let ctx = world
                .get::<AuthoredBrainContext>(entity)
                .map(AuthoredBrainContext::build_context)
                .unwrap_or_else(|| {
                    BrainBuildContext::at(
                        world
                            .get::<ActorPose>(entity)
                            .map(|pose| pose.origin().x)
                            .unwrap_or(0.0),
                    )
                });
            catalog.build_brain_from_preset(&preset, &ctx)
        }
        // A boss's autonomous brain is not rebuilt from a catalog preset: it is
        // the live `BossPattern` captured into the suspended-autonomous-runtime at
        // possession and resumed from there, so this catalog-preset resolver
        // returns `None` for a boss source (the caller resumes from the captured
        // runtime instead).
        AutonomousSource::Boss { .. } => None,
    }
}

/// Restore the live temporary-control mode (player possession / mount) from each
/// body's snapshot-persisted [`TemporaryControl`].
///
/// The `Brain` cursor cannot restore a `Brain::Player`, and possession/mount
/// relationships were re-derived from live components, so after the registered
/// blobs land the live control can disagree with the restored `TemporaryControl`.
/// This rebuilds it — both the controlled body's live `Brain`/`Mounted` AND the
/// coupled relationships (the vacated home avatar, `PossessionState`, `RidingOn`/
/// `MountSlot`) — from the stable ids, in BOTH rewind directions.
///
/// Order: possession first (there is exactly one player), then mounts. A body
/// whose control ended resumes its autonomous brain from its binding source (via
/// [`autonomous_brain_for_source`]); a body that gained control gets the player /
/// mounted brain installed.
fn reconcile_temporary_control(world: &mut World) {
    struct Body {
        entity: Entity,
        control: TemporaryControl,
        live_is_player: bool,
        live_mounted: bool,
    }

    let bodies: Vec<Body> = {
        let mut q = world.query::<(
            Entity,
            &TemporaryControl,
            &Brain,
            bevy::ecs::query::Has<Mounted>,
        )>();
        q.iter(world)
            .map(|(entity, control, brain, mounted)| Body {
                entity,
                control: control.clone(),
                live_is_player: brain.is_player(),
                live_mounted: mounted,
            })
            .collect()
    };
    if bodies.is_empty() {
        return;
    }

    // The player's home avatar (keeps `PrimaryPlayer` even while its brain is
    // vacated onto a possessed body).
    let home = {
        let mut q = world.query_filtered::<Entity, bevy::ecs::query::With<PrimaryPlayer>>();
        q.iter(world).next()
    };
    // Stable-id → entity, to rebuild raw-`Entity` relationships (mount links).
    let by_sim_id: std::collections::BTreeMap<String, Entity> = {
        let mut q = world.query::<(Entity, &SimId)>();
        q.iter(world)
            .map(|(entity, id)| (id.as_str().to_string(), entity))
            .collect()
    };

    // ── Possession ──────────────────────────────────────────────────────────
    let possessed = bodies
        .iter()
        .find(|b| matches!(b.control, TemporaryControl::Player { .. }))
        .map(|b| b.entity);

    if let Some(target) = possessed {
        // A possessed body: install the player brain, vacate the home avatar, and
        // rebuild the possession bookkeeping. `restore_brain` is the CURRENT
        // autonomous source (so a source changed during possession resumes on
        // release), never a stale cache.
        let restore_brain = autonomous_brain_for_source(world, target);
        if let Ok(mut em) = world.get_entity_mut(target) {
            em.insert((Brain::Player(PlayerSlot::PRIMARY), ActorControl::default()));
        }
        if let Some(home) = home {
            if let Ok(mut em) = world.get_entity_mut(home) {
                em.remove::<Brain>();
                em.insert(ActorControl::default());
            }
        }
        if let Some(mut possession) = world.get_resource_mut::<PossessionState>() {
            possession.possessed = Some(target);
            possession.home = home;
            possession.restore_brain = restore_brain;
        }
    } else {
        // No possession: the home avatar drives, and any body left player-brained
        // by the abandoned future resumes its autonomous source.
        for body in &bodies {
            if body.live_is_player && body.control.is_autonomous() {
                if let Some(brain) = autonomous_brain_for_source(world, body.entity) {
                    if let Ok(mut em) = world.get_entity_mut(body.entity) {
                        em.insert((brain, ActorControl::default()));
                    }
                }
            }
        }
        if let Some(home) = home {
            let home_drives = world
                .get::<Brain>(home)
                .map(Brain::is_player)
                .unwrap_or(false);
            if !home_drives {
                if let Ok(mut em) = world.get_entity_mut(home) {
                    em.insert((Brain::Player(PlayerSlot::PRIMARY), ActorControl::default()));
                }
            }
        }
        if let Some(mut possession) = world.get_resource_mut::<PossessionState>() {
            possession.possessed = None;
            possession.home = None;
            possession.restore_brain = None;
        }
    }

    // ── Mounts ──────────────────────────────────────────────────────────────
    for body in &bodies {
        match &body.control {
            TemporaryControl::Mounted { mount } => {
                // Install the mounted mode (BOTH the mounted brain AND its action
                // set, from the rider's cache) and rebuild the rider↔mount link
                // from the stable mount id. Installing the brain without the action
                // set would leave a mounted rider with a mismatched pair.
                let cached = world
                    .get::<MountedBrainCache>(body.entity)
                    .map(|cache| (cache.brain.clone(), cache.action_set.clone()));
                let mount_entity = by_sim_id.get(mount.as_str()).copied();
                if let Ok(mut em) = world.get_entity_mut(body.entity) {
                    if !em.contains::<Mounted>() {
                        em.insert(Mounted);
                    }
                    if let Some((brain, action_set)) = cached {
                        em.insert((brain, action_set));
                    }
                    if let Some(mount_entity) = mount_entity {
                        em.insert(RidingOn {
                            mount: mount_entity,
                        });
                    }
                }
                if let Some(mount_entity) = mount_entity {
                    if let Some(mut slot) = world.get_mut::<MountSlot>(mount_entity) {
                        slot.rider = Some(body.entity);
                    }
                }
            }
            // Not mounted per the snapshot, but a stale `Mounted` marker survived
            // the rewind (and it is not player-possessed): dismount it back to its
            // autonomous brain.
            TemporaryControl::Autonomous if body.live_mounted => {
                let brain = autonomous_brain_for_source(world, body.entity);
                if let Ok(mut em) = world.get_entity_mut(body.entity) {
                    em.remove::<Mounted>();
                    if let Some(brain) = brain {
                        em.insert(brain);
                    }
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::enemies::test_roster;
    use ambition_characters::actor::character_catalog::{
        parse_catalog, BrainPresetId, HostileArchetypeId,
    };
    use ambition_engine_core as ae;

    const CATALOG: &str = r#"(
        brain_presets: { "wanderer_x": Wanderer(speed: 40.0, aggressiveness: 0.0) },
        action_set_presets: { "peaceful": (move_style: Walk) },
        characters: {
            "npc_x": (
                display_name: "X", spritesheet: "x.png", manifest: "x_spritesheet.ron",
                tier: MainHall, body_kind: Standard, composition: None,
                default_brain: "wanderer_x", default_action_set: "peaceful", tags: [],
            ),
        },
    )"#;

    fn catalog() -> CharacterCatalog {
        CharacterCatalog::from_data(parse_catalog(CATALOG))
    }

    fn config_fixture() -> ActorConfig {
        ActorConfig {
            id: "npc".into(),
            name: "Npc".into(),
            tuning: ActorTuning::default(),
            brain_spec: CharacterBrainSpec::default(),
            brain: CharacterBrain::Passive,
            spawn: crate::features::enemies::ActorSpawnState {
                pos: ae::Vec2::ZERO,
                size: ae::Vec2::splat(8.0),
            },
            sprite_override_npc_name: None,
            sprite_character_id: Some("npc_x".into()),
        }
    }

    fn wanderer(world: &World) -> Brain {
        world
            .resource::<CharacterCatalog>()
            .build_brain_from_preset("wanderer_x", &BrainBuildContext::at(0.0))
            .expect("wanderer builds")
    }

    /// A rewind INTO a provoked snapshot reruns the roster construction: the
    /// hostile archetype config (brain kind marker + HP pool) is reconstructed
    /// from the stable archetype id alone.
    #[test]
    fn reconstructs_a_provoked_actor_from_its_archetype_id() {
        let mut w = World::new();
        w.insert_resource(test_roster());
        let mut config = config_fixture();
        config.tuning.max_health = 1; // peaceful HP left over from the present.
        let e = w
            .spawn((
                SimId::placement("npc"),
                BrainBinding::new(
                    BrainPresetId::new("wanderer_x"),
                    AutonomousSource::Provoked {
                        archetype: HostileArchetypeId::new("combatant"),
                    },
                ),
                config,
                CombatKit::default(),
                Brain::stand_still(),
                TemporaryControl::Autonomous,
            ))
            .id();

        reconcile_autonomous_actors(&mut w);

        let config = w.get::<ActorConfig>(e).unwrap();
        assert!(
            matches!(&config.brain, CharacterBrain::Custom(id) if id == "combatant"),
            "config.brain marks the provoked archetype"
        );
        assert_eq!(
            config.tuning.max_health, 4,
            "the combatant HP pool is reconstructed from the roster"
        );
        assert_ne!(
            w.get::<Brain>(e).unwrap().label(),
            "stand_still",
            "the live brain is rebuilt to the hostile archetype, not left peaceful"
        );
    }

    /// A rewind to BEFORE a challenge (a catalog source over a config the present
    /// left hostile) restores the peaceful catalog config.
    #[test]
    fn reverts_a_catalog_actor_to_its_peaceful_config() {
        let mut w = World::new();
        w.insert_resource(test_roster());
        w.insert_resource(catalog());
        let mut config = config_fixture();
        // The present is still hostile (provoked, then rewound past the challenge).
        config.tuning.max_health = 100;
        config.brain = CharacterBrain::Custom("combatant".into());
        let brain = wanderer(&w);
        let e = w
            .spawn((
                SimId::placement("npc"),
                BrainBinding::new(
                    BrainPresetId::new("wanderer_x"),
                    AutonomousSource::CatalogDefault,
                ),
                config,
                CombatKit::default(),
                brain,
                TemporaryControl::Autonomous,
            ))
            .id();

        reconcile_autonomous_actors(&mut w);

        let config = w.get::<ActorConfig>(e).unwrap();
        assert_eq!(
            config.tuning.max_health, 1,
            "the peaceful HP pool is restored"
        );
        assert!(
            matches!(config.brain, CharacterBrain::Passive),
            "config.brain is derived from the live wanderer brain: Passive"
        );
    }

    /// Possession rollback — rewind INTO a possessed snapshot: the NPC becomes
    /// player-controlled, the home avatar is vacated, and `PossessionState` is
    /// rebuilt from the stable ids.
    #[test]
    fn restores_possession_across_a_rewind() {
        let mut w = World::new();
        w.insert_resource(test_roster());
        w.insert_resource(catalog());
        w.init_resource::<PossessionState>();
        let home = w
            .spawn((
                SimId::player_slot(0),
                Brain::Player(PlayerSlot::PRIMARY),
                PrimaryPlayer,
            ))
            .id();
        let brain = wanderer(&w);
        let npc = w
            .spawn((
                SimId::placement("npc"),
                BrainBinding::new(
                    BrainPresetId::new("wanderer_x"),
                    AutonomousSource::CatalogDefault,
                ),
                config_fixture(),
                CombatKit::default(),
                brain, // present: autonomous (released)
                TemporaryControl::Player {
                    controller: SimId::player_slot(0),
                },
                AuthoredBrainContext::from_placement(0.0, 0.0),
            ))
            .id();

        reconcile_autonomous_actors(&mut w);

        assert!(
            w.get::<Brain>(npc).unwrap().is_player(),
            "the possessed NPC carries the player brain again"
        );
        assert!(
            w.get::<Brain>(home).is_none(),
            "the home avatar is vacated (its player brain moved onto the NPC)"
        );
        let possession = w.resource::<PossessionState>();
        assert_eq!(possession.possessed, Some(npc));
        assert_eq!(possession.home, Some(home));
    }

    /// Possession rollback — rewind to an AUTONOMOUS snapshot from a possessed
    /// present: the NPC resumes its autonomous brain and the home avatar drives
    /// again (exactly one player brain).
    #[test]
    fn releases_possession_across_a_rewind() {
        let mut w = World::new();
        w.insert_resource(test_roster());
        w.insert_resource(catalog());
        w.init_resource::<PossessionState>();
        // Home vacated in the present (possessing).
        let home = w.spawn((SimId::player_slot(0), PrimaryPlayer)).id();
        let npc = w
            .spawn((
                SimId::placement("npc"),
                BrainBinding::new(
                    BrainPresetId::new("wanderer_x"),
                    AutonomousSource::CatalogDefault,
                ),
                config_fixture(),
                CombatKit::default(),
                Brain::Player(PlayerSlot::PRIMARY), // present: possessed
                TemporaryControl::Autonomous,       // snapshot: autonomous
                AuthoredBrainContext::from_placement(0.0, 0.0),
            ))
            .id();

        reconcile_autonomous_actors(&mut w);

        assert!(
            !w.get::<Brain>(npc).unwrap().is_player(),
            "the NPC resumes its autonomous brain"
        );
        assert!(
            w.get::<Brain>(home).unwrap().is_player(),
            "the home avatar drives again"
        );
        assert_eq!(w.resource::<PossessionState>().possessed, None);
    }
}
