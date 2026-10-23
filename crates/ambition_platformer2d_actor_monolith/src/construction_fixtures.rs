//! The fixtures the construction tests share with the room coordinator's
//! (`ambition_platformer2d_publication`), which commit what they build.

#![cfg(any(test, feature = "test-support"))]
#![allow(unused_imports)]

use ambition_characters::actor::limb::{Limb, LimbIntents, LimbRig, LimbRouteState, LimbSlot};
use ambition_platformer2d_shared_tangle::construction::{
    ConstructionError, ConstructionPlan, ConstructionScope, SpawnOrigin,
};
use ambition_platformer2d_shared_tangle::lifecycle::SessionSpawnScope;
use ambition_platformer2d_shared_tangle::sim_id::SimId;
use bevy::prelude::{App, Commands, Update, World};
use ambition_platformer2d_construction::*;
// What the planner's module brought in when these lived inside it.
use ambition_platformer2d_actor_spawn::{SpawnActorKind, SpawnActorRequest};
use bevy::prelude::Entity;
use ambition_platformer2d_shared_tangle::construction::ContentBinding;
use crate::features::{
    ActorConstructionContext, RoomFeatureConstructionError, RoomFeatureConstructionPlan,
};
use ambition_platformer2d_core as ae;
use ambition_platformer2d_shared_tangle::construction::{
    verify_committed_roster, AuthoritativeScope, ConstructionReceipt, RelationCheck,
    RelationRequest, RosterViolation, TransactionBaseline,
};

pub const REAL_HELD_ITEM: &str = "gun_sword";

pub fn empty_room(id: &str) -> ambition_platformer2d_world::rooms::RoomSpec {
    ambition_platformer2d_world::rooms::RoomSpec::new(
        id,
        ae::World::new(id, ae::Vec2::splat(1000.0), ae::Vec2::ZERO, Vec::new()),
    )
}

/// THE GENERATION A CONTENT REPLACEMENT IS PUBLISHED OVER, for the two tests
/// that build a replacement plan.
///
/// `'static` for the same reason as [`fixture_cast`]: `GenerationMechanics`
/// borrows it and the context borrows that.
pub fn replacement_mechanics() -> &'static crate::session::mechanics::SessionMechanics {
    static MECHANICS: std::sync::OnceLock<crate::session::mechanics::SessionMechanics> =
        std::sync::OnceLock::new();
    MECHANICS.get_or_init(Default::default)
}

/// The cast [`replacement_mechanics`] was frozen with, `'static` for the same
/// reason.
pub fn replacement_cast() -> &'static ambition_characters::prepared::ActiveSessionCast {
    static CAST: std::sync::OnceLock<ambition_characters::prepared::ActiveSessionCast> =
        std::sync::OnceLock::new();
    CAST.get_or_init(|| ambition_characters::prepared::ActiveSessionCast(Some(fixture_cast().clone())))
}

/// THE FIXTURE CAST every construction test builds bodies from.
///
/// `'static` because `ActorConstructionContext` BORROWS the cast, and a
/// per-test local would not outlive the plan it is handed to.
pub fn fixture_cast() -> &'static ambition_characters::prepared::PreparedCharacterRegistry {
    static CAST: std::sync::OnceLock<ambition_characters::prepared::PreparedCharacterRegistry> =
        std::sync::OnceLock::new();
    CAST.get_or_init(|| {
        let mut registry = ambition_characters::prepared::PreparedCharacterRegistry::default();
        for (id, mount) in [
            ("medium_striker", None),
            ("puppy_slug", None),
            ("combatant", None),
            ("fixture_walker", None),
            // The giant's HANDS: `giant_cluster_rows` mints two limb rows naming
            // this character, so a cast without it refuses the whole cluster.
            ("npc_giant_gnu_hands", None),
            // The limbed host: a character with hands lowers to host + two hand
            // rows, and a CHARACTER is the only thing that says so now.
            ("fixture_giant", Some(("giant", &[][..]))),
            // A limbed host whose seated rider conducts it.
            ("fixture_conducted_giant", Some(("giant", &[][..]))),
            // The rideable body and its rider, the ADR 0020 pair.
            ("fixture_mount", Some(("shark", &[][..]))),
        ] {
            let mut definition =
                ambition_characters::actor::definition::CharacterDefinition::new(id, id, "test")
                    .with_locomotion(ambition_characters::actor::CharacterLocomotion {
                        run_speed: 155.0,
                        move_style: ambition_characters::brain::MoveStyleSpec::Walk,
                        ..Default::default()
                    });
            definition.vitals.max_health = Some(4);
            if let Some((class, pilotable)) = mount {
                definition.mount = Some(ambition_characters::actor::CharacterMount {
                    class: Some(class.to_string()),
                    pilotable_classes: pilotable.iter().map(|c: &&str| (*c).to_string()).collect(),
                    rider_conducts: id == "fixture_conducted_giant",
                    ..Default::default()
                });
                // The giants are limbed hosts: their hands are this character.
                if class == "giant" {
                    definition.hands = Some(ambition_characters::actor::CharacterHands {
                        character: "npc_giant_gnu_hands".to_string(),
                        rest: None,
                    });
                }
            }
            let finalized = ambition_characters::prepared::prepare_and_finalize_for_test(
                definition,
                &ambition_characters::prepared::CharacterBindings::default(),
            );
            registry.insert_prepared(finalized.prepared);
        }
        // The RIDER: it pilots a `"shark"`-class mount and is not itself one.
        let mut rider = ambition_characters::actor::definition::CharacterDefinition::new(
            "pirate_raider",
            "pirate_raider",
            "test",
        )
        .with_locomotion(ambition_characters::actor::CharacterLocomotion {
            run_speed: 155.0,
            move_style: ambition_characters::brain::MoveStyleSpec::Walk,
            ..Default::default()
        });
        rider.vitals.max_health = Some(4);
        rider.mount = Some(ambition_characters::actor::CharacterMount {
            class: None,
            pilotable_classes: vec!["shark".to_string()],
            ..Default::default()
        });
        let finalized = ambition_characters::prepared::prepare_and_finalize_for_test(
            rider,
            &ambition_characters::prepared::CharacterBindings::default(),
        );
        registry.insert_prepared(finalized.prepared);
        registry
    })
}

pub fn ground_item(id: &str, held_item: &str) -> ambition_platformer2d_world::rooms::GroundItemSpec {
    ambition_platformer2d_world::rooms::GroundItemSpec {
        id: id.to_string(),
        name: format!("{id} display"),
        held_item: held_item.to_string(),
        pos: ae::Vec2::ZERO,
        half_extent: ae::Vec2::splat(8.0),
    }
}

pub fn staged_enemy(id: &str, grudge_against: Option<&str>) -> SpawnActorRequest {
    SpawnActorRequest {
        id: id.to_string(),
        name: "test_walker".to_string(),
        pos: ae::Vec2::ZERO,
        half_size: ae::Vec2::splat(10.0),
        faction: ambition_characters::actor::ActorFaction::Npc,
        grudge_against: grudge_against.map(str::to_string),
        kind: SpawnActorKind::Enemy {
            brain: ambition_entity_catalog::placements::CharacterBrain::Custom(
                "medium_striker".into(),
            ),
            character: ambition_entity_catalog::CharacterId::from("medium_striker"),
        },
    }
}

/// A room with both planned authored + staged families, staged by a named
/// provider so the resulting provenance is real rather than a placeholder.
pub fn duelling_room() -> (
    ambition_platformer2d_world::rooms::RoomSpec,
    crate::features::RoomContentStagingRegistry,
) {
    let mut room = empty_room("hall");
    room.ground_items
        .push(ground_item("pickup_a", REAL_HELD_ITEM));
    let mut staging = crate::features::RoomContentStagingRegistry::default();
    staging
        .register("hall", "test_provider", "duel", "duel.v1", |_room| {
            vec![
                staged_enemy("duel_red", Some("duel_blue")),
                staged_enemy("duel_blue", Some("duel_red")),
            ]
        })
        .expect("stager registers");
    (room, staging)
}

pub fn prepare(
    room: &ambition_platformer2d_world::rooms::RoomSpec,
    staging: &crate::features::RoomContentStagingRegistry,
    recipes: &ActorConstructionRegistry,
) -> Result<RoomFeatureConstructionPlan, RoomFeatureConstructionError> {
    RoomFeatureConstructionPlan::prepare(
        room,
        &Default::default(),
        staging,
        &ambition_boss_encounter::test_boss_catalog(),
        ActorConstructionContext::new(
            recipes,
            &ambition_characters::actor::character_catalog::CharacterCatalog::empty(),
            &Default::default(),
            ContentBinding::content_unstated(ae::ContentEpoch(4)),
        )
        .with_prepared(fixture_cast()),
    )
}

/// Drive the real `apply_summon_effects` system over one summon request.
pub fn run_summon(world: &mut World, summoner: Entity, spec: ambition_vfx::SummonSpec) {
    world.write_message(ambition_vfx::EffectRequest {
        owner: summoner,
        effect: ambition_vfx::Effect::Summon(spec),
    });
    world
        .run_system_cached(crate::features::apply_summon_effects)
        .expect("the summon executor runs");
    world.flush();
}

/// Everything `apply_summon_effects` reads, into a world that already exists.
///
/// Separate from [`summon_world`] so a summoner the REAL construction executor
/// built can be handed to the SAME executor — see
/// [`a_boss_the_construction_executor_built_can_summon`].
pub fn insert_summon_resources(world: &mut World) {
    world.init_resource::<bevy::ecs::message::Messages<ambition_vfx::EffectRequest>>();
    world.insert_resource(ambition_characters::actor::character_catalog::CharacterCatalog::empty());
    world.init_resource::<crate::session::mechanics::SessionMechanics>();
    world.insert_resource(ambition_boss_encounter::test_boss_catalog().clone());
    world.insert_resource(engine_construction_registry());
    world.insert_resource(fixture_cast().clone());
}

pub fn summon_spec(id: &str) -> ambition_vfx::SummonSpec {
    ambition_vfx::SummonSpec {
        id: id.to_string(),
        pos: ae::Vec2::ZERO,
        half_size: ae::Vec2::splat(8.0),
        character_id: "puppy_slug".into(),
        encounter_id: "enc_1".into(),
        faction: ambition_vfx::HitSide::Enemy,
        // A minion nobody rides, which is every summon these fixtures are about.
        ridden_by_summoner: None,
        // The sentinel's minions keep the vitals their character authors.
        health: None,
        // A boss minion keeps its character's hazard.
        keeps_contact_damage: true,
        holds: Vec::new(),
    }
}

pub fn room_loaded_count(app: &mut App) -> usize {
    app.world_mut()
        .resource_mut::<bevy::ecs::message::Messages<ambition_platformer2d_world::rooms::RoomLoaded>>()
        .drain()
        .count()
}

pub fn dynamic_scope() -> ConstructionScope {
    ConstructionScope::in_generation(ambition_platformer2d_shared_tangle::construction::ContentBinding::RuntimeDynamic, None)
}

pub fn bare_request(id: &str) -> ActorConstructionRequest {
    ActorConstructionRequest {
        sim_id: SimId::placement(id),
        origin: SpawnOrigin::ProviderStaged {
            provider: "test_provider".into(),
            room: "hall".into(),
            instance: id.into(),
        },
        parameters: ActorConstructionParams::StagedActor(staged_enemy(id, None)),
        relations: Vec::new(),
    }
}

pub fn test_services() -> ActorConstructionServices {
    ActorConstructionServices {
        context: ambition_platformer2d_construction::placements::ActorPlacementContext::new(
            &ambition_characters::actor::character_catalog::CharacterCatalog::empty(),
            &Default::default(),
        )
        .with_prepared(fixture_cast()),
        boss_catalog: ambition_boss_encounter::test_boss_catalog().clone(),
    }
}

pub fn verify_bare(
    world: &mut World,
    plan: &ActorConstructionPlan,
    receipt: &ConstructionReceipt,
    baseline: &TransactionBaseline,
) -> Result<(), Vec<RosterViolation>> {
    let transaction = plan.transaction(SessionSpawnScope::UNSCOPED);
    let scope = AuthoritativeScope::gather(world, &transaction);
    verify_committed_roster(plan, receipt, baseline, &scope, world)
}

/// A plan of `rows`, with `from` declaring `kind`/`payload` onto `to`.
pub fn related_actor_plan(
    rows: &[&str],
    from: &str,
    to: &str,
    relation: ActorRelation,
) -> ActorConstructionPlan {
    let requests: Vec<_> = rows
        .iter()
        .map(|id| {
            let mut request = bare_request(id);
            if *id == from {
                request.relations.push(RelationRequest {
                    to: SimId::placement(to),
                    relation: relation.clone(),
                });
            }
            request
        })
        .collect();
    ActorConstructionPlan::prepare(
        dynamic_scope(),
        requests,
        &Default::default(),
        &engine_construction_registry(),
    )
    .expect("the plan is valid")
}

/// Give a committed rider/mount pair the capability components their archetypes
/// would carry, so the mount postcondition's capability checks have something to
/// read. The bare fixtures build generic enemy bodies, which are neither mounts
/// nor pilots; `verify_mount` legitimately requires `Mountable` on the mount and
/// a compatible `CanPilot` on the rider, so a wiring test must equip the pair for
/// the same reason a real room's archetypes do.
pub fn equip_mount_pair(world: &mut World, rider: Entity, mount: Entity) {
    world.entity_mut(mount).insert(ambition_mount::Mountable {
        seat: ae::Vec2::ZERO,
        class: ambition_mount::MountClass("giant".into()),
        control_grant: ambition_mount::ControlGrant::Total,
        death_impact: ambition_mount::MountDeathImpact::Dismount,
    });
    world.entity_mut(rider).insert(ambition_mount::CanPilot {
        classes: vec![ambition_mount::MountClass("giant".into())],
    });
}

pub fn hand(slot: LimbSlot) -> ActorRelation {
    ActorRelation::Limb {
        slot,
        home_offset: ae::Vec2::new(12.0, -4.0),
    }
}

pub fn preflight(requests: Vec<ActorConstructionRequest>) -> Result<(), ActorConstructionError> {
    preflight_actor_relations(
        &requests,
        &ambition_boss_encounter::test_boss_catalog(),
        Some(fixture_cast()),
    )
}

pub fn giant_room() -> ambition_platformer2d_world::rooms::RoomSpec {
    let mut room = empty_room("arena");
    room.enemy_spawns
        .push(ambition_platformer2d_world::rooms::Authored::new(
            "boss_mount",
            "Giant GNU",
            ae::Aabb::new(ae::Vec2::new(100.0, 100.0), ae::Vec2::splat(60.0)),
            ambition_platformer2d_world::rooms::EnemySpawnSpec::new(
                ambition_entity_catalog::placements::CharacterBrain::Custom("fixture_giant".into()),
                "fixture_giant",
            ),
        ));
    room
}

pub fn staged_giant(id: &str) -> SpawnActorRequest {
    SpawnActorRequest {
        id: id.to_string(),
        name: "Giant GNU".to_string(),
        pos: ae::Vec2::new(100.0, 100.0),
        half_size: ae::Vec2::splat(60.0),
        faction: ambition_characters::actor::ActorFaction::Enemy,
        grudge_against: None,
        kind: SpawnActorKind::Enemy {
            brain: ambition_entity_catalog::placements::CharacterBrain::Custom(
                "fixture_giant".into(),
            ),
            character: ambition_entity_catalog::CharacterId::from("fixture_giant"),
        },
    }
}

pub fn rig_faults(plan: &ActorConstructionPlan, receipt: &ConstructionReceipt, world: &World) -> usize {
    let faults = ambition_platformer2d_construction::verify_rig_composition(plan, receipt, world);
    for fault in &faults {
        assert!(
            matches!(fault, RosterViolation::RigComposition { .. }),
            "the composition pass only speaks RigComposition: {fault:?}"
        );
    }
    faults.len()
}

/// A room with a shark mount and its pirate rider, linked the way LDtk's
/// `mounted_on` entity-ref lowers: `RoomSpec.mount_links = [(rider, mount)]`.
pub fn mounted_pair_room() -> ambition_platformer2d_world::rooms::RoomSpec {
    let mut room = empty_room("cove");
    room.enemy_spawns
        .push(ambition_platformer2d_world::rooms::Authored::new(
            "sky_shark",
            "Burning Flying Shark",
            ae::Aabb::new(ae::Vec2::new(200.0, 100.0), ae::Vec2::new(63.0, 26.0)),
            ambition_platformer2d_world::rooms::EnemySpawnSpec::new(
                ambition_entity_catalog::placements::CharacterBrain::Custom("fixture_mount".into()),
                "fixture_mount",
            ),
        ));
    room.enemy_spawns
        .push(ambition_platformer2d_world::rooms::Authored::new(
            "sky_rider",
            "Pirate Raider",
            ae::Aabb::new(ae::Vec2::new(200.0, 40.0), ae::Vec2::new(22.0, 39.0)),
            ambition_platformer2d_world::rooms::EnemySpawnSpec::new(
                ambition_entity_catalog::placements::CharacterBrain::Custom("pirate_raider".into()),
                "pirate_raider",
            ),
        ));
    room.mount_links
        .push(("sky_rider".to_string(), "sky_shark".to_string()));
    room
}

pub fn placement_registry() -> ambition_platformer2d_construction::placements::PlacementLoweringRegistry {
    let mut registry = ambition_platformer2d_construction::placements::PlacementLoweringRegistry::default();
    registry
        .try_register(
            ambition_entity_catalog::placements::PlacementKind::Pickup,
            "ambition_platformer2d_actor_monolith",
            "test",
            "placement.pickup.v1",
            crate::features::ecs::spawn_static::lower_pickup_placement,
        )
        .unwrap();
    registry
        .try_register(
            ambition_entity_catalog::placements::PlacementKind::Interactable,
            "ambition_platformer2d_actor_monolith",
            "test",
            "placement.interactable.v1",
            crate::features::ecs::spawn_static::lower_interactable_placement,
        )
        .unwrap();
    registry
}

pub fn placement_room() -> ambition_platformer2d_world::rooms::RoomSpec {
    // `PickupKind` comes from the crate ROOT, not from `placements` — it is
    // public there and re-exported privately here, which `d3bd6e95a` exposed
    // when it deleted `PickupKindSpec`.
    use ambition_entity_catalog::placements::{
        HazardRespawn, InteractableSpec, InteractionKindSpec, PickupSpec, PlacementSchema,
    };
    use ambition_entity_catalog::PickupKind;
    let mut room = empty_room("gallery");
    room.placements.push(
        ambition_platformer2d_world::placements::PlacementRecord::new(
            "ring_1",
            PlacementSchema::Pickup(PickupSpec {
                kind: PickupKind::Health { amount: 1 },
                respawn: HazardRespawn::Never,
                sprite: None,
            }),
            ae::Aabb::new(ae::Vec2::new(64.0, 32.0), ae::Vec2::splat(8.0)),
        ),
    );
    room.placements.push(
        ambition_platformer2d_world::placements::PlacementRecord::new(
            "door_1",
            PlacementSchema::Interactable(InteractableSpec::new(
                "Enter",
                InteractionKindSpec::Door { target: None },
            )),
            ae::Aabb::new(ae::Vec2::new(128.0, 32.0), ae::Vec2::splat(16.0)),
        ),
    );
    room
}

pub fn prepare_with_placements(
    room: &ambition_platformer2d_world::rooms::RoomSpec,
) -> Result<RoomFeatureConstructionPlan, RoomFeatureConstructionError> {
    RoomFeatureConstructionPlan::prepare(
        room,
        &placement_registry(),
        &crate::features::RoomContentStagingRegistry::default(),
        &ambition_boss_encounter::test_boss_catalog(),
        ActorConstructionContext::new(
            &engine_construction_registry(),
            &ambition_characters::actor::character_catalog::CharacterCatalog::empty(),
            &Default::default(),
            ContentBinding::content_unstated(ae::ContentEpoch(4)),
        )
        .with_prepared(fixture_cast()),
    )
}

/// Every `TransactionId` insertion observed by a hook, across the arm below.
///
/// ⚠ A HOOK IS A `fn` POINTER, NOT A CLOSURE, so the count travels through a
/// static rather than by capture. It is only incremented in worlds this arm
/// registered the hook on, so a parallel test inserting `TransactionId` in its
/// own world cannot reach it.
pub static CANDIDATE_INSERTIONS: std::sync::Mutex<usize> = std::sync::Mutex::new(0);

pub fn count_candidate_insertions(world: &mut bevy::prelude::World) {
    use bevy::ecs::lifecycle::HookContext;
    use bevy::ecs::world::DeferredWorld;
    world
        .register_component_hooks::<
            ambition_platformer2d_shared_tangle::construction::TransactionId,
        >()
        .on_insert(|_world: DeferredWorld, _ctx: HookContext| {
            *CANDIDATE_INSERTIONS.lock().expect("insertions") += 1;
        });
}
