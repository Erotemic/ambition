//! The extension host in the platformer composition (fast-iteration I4).
//!
//! The runtime chooses where each public phase sits in the simulation
//! schedule, and which domain adapters answer which ports. The host and the
//! adapters name no game algorithm; the game declares modules from its own
//! plugins.
//!
//! | phase | placement | ports |
//! |---|---|---|
//! | `technique_execution` | `CombatSet::ContentSpecials`, gameplay-gated | triggers `ambition.boss.special_cast` (boss domain), `ambition.fighter.special` and `ambition.body.status_tick` (combat domain) and `ambition.body.technique` (`ambition_abilities`); cost `ambition.resources.spend_mana` (`ambition_abilities`); requests `ambition.combat.body_strike`, `ambition.combat.sleep_area`, `ambition.resources.refill`, `ambition.combat.parry_window`, `ambition.combat.blast`, `ambition.body.hold_status`, `ambition.body.time_scale` and `ambition.body.status_clock` (combat domain), observations `ambition.combat.foes`, `ambition.body.in_play` and `ambition.combat.credit` (combat domain), `ambition.world.owned_entities`, `ambition.body.ledge_ahead` and `ambition.body.on_ground` (`ambition_abilities`), requests `ambition.world.spawn_module_item`, `ambition.world.end_owned_entity`, `ambition.feedback.body_line`, `ambition.feedback.burst` and `ambition.feedback.body_sound` (`ambition_abilities`), `ambition.projectiles.spawn` (projectile domain), `ambition.combat.damage_box`, `ambition.combat.held_damage_box` and `ambition.combat.held_ray` (combat domain), `ambition.boss.summon`, `ambition.boss.dismiss_summon`, `ambition.presentation.drawn_row` and `ambition.feedback.speech_bubble` (boss domain), `ambition.world.spawn_module_entity`, `ambition.feedback.effect` and `ambition.motion.velocity` (`ambition_abilities`), request `ambition.motion.transit` (held on the body and made in the next tick's `BodyPathSet::Carry`, `ambition_abilities::pending_transit`), observation `ambition.fighter.aimed_stick` (combat domain), request `ambition.world.summon_ride`, observations `ambition.body.ride`, `ambition.body.saddle`, `ambition.body.presses`, `ambition.body.tumbling` and `ambition.body.room`, requests `ambition.mount.dismount`, `ambition.body.hold_course` and `ambition.world.dismiss` (`ambition_abilities::ride`), request `ambition.portal.held_pair` (portal domain, with the `portal` feature) |
//! | `wielded_use` | `ItemPickupSet::WieldedAbilities`, before the movement cooldown ticks, gameplay-gated | triggers `ambition.items.wielded_use` and `ambition.items.wielded_alternate`; observations `ambition.items.mark` and `ambition.items.aim_cast`; requests `ambition.motion.transit` (lowered first, in `BodyPathSet::Carry`), `ambition.abilities.movement_cooldown`, `ambition.feedback.effect`, `ambition.feedback.burst`, `ambition.feedback.hit_mark`, `ambition.motion.velocity`, `ambition.items.set_mark`, `ambition.resources.spend_mana`, `ambition.feedback.body_sound` and `ambition.world.spawn_module_entity` (held-item domain, `ambition_abilities`), `ambition.combat.damage_box` and `ambition.combat.strike`, `ambition.projectiles.spawn` |
//! | `boss_conduct` | `WorldPrepSet::AfterIntegrate`, after the rider sync, gameplay-gated | trigger `ambition.boss.conduct`; observations `ambition.body.attachments` and `ambition.boss.mount`; requests `ambition.boss.conducted_pose`, `ambition.boss.limb_pose`, `ambition.boss.limb_opening`, `ambition.boss.limb_blow`, `ambition.boss.mount_launch`, `ambition.boss.mount_walk`, `ambition.boss.foe_launch`, `ambition.boss.wear`, `ambition.feedback.speech_bubble`, `ambition.presentation.drawn_row`, `ambition.feedback.burst`, `ambition.feedback.lightning`, `ambition.feedback.lightning_aura`, `ambition.boss.summon`, `ambition.boss.dismiss_summon` (boss domain), `ambition.combat.held_damage_box`, `ambition.combat.held_ray`, `ambition.combat.riding_hitbox` (combat), `ambition.world.held_force` (`ambition_abilities`), `ambition.projectiles.spawn`, `ambition.feedback.body_sound`, `ambition.portal.held_pair` (portal, with the `portal` feature) |
//! | `module_entity_tick` | `ItemPickupSet::WieldedAbilities`, after `wielded_use`, gameplay-gated | trigger `ambition.world.module_entity_tick`; observations `ambition.world.item_state`, `ambition.world.spawner_body` and `ambition.world.toucher_credits` (`ambition_abilities`); requests `ambition.feedback.body_sound`, `ambition.feedback.effect`, `ambition.world.move_module_entity`, `ambition.world.launch_toucher`, `ambition.world.pull_bodies` (lowered in `BodyPathSet::Carry`) and `ambition.world.end_module_entity` (`ambition_abilities`), `ambition.projectiles.spawn`, `ambition.combat.blast` and observation `ambition.combat.credit` (`ambition_combat`) |
//! | `actor_think` | after `ActorDecisionSet::Publish`, before `PlayerInputSet::ControlGate`, gameplay-gated | trigger `ambition.actor.think`; requests `ambition.actor.fire` (`ambition_combat`) and `ambition.motion.velocity` (`ambition_abilities`); trigger `ambition.actor.group_think`, requests `ambition.actor.steer`, `ambition.item.take` and `ambition.item.release`, observation `ambition.world.whereabouts` (`ambition_abilities`) |
//! | `hit_reaction` | `CombatSet::ContentFlavor`, gameplay-gated | triggers `ambition.combat.parried`, `ambition.combat.struck`, `ambition.combat.hit_receptions` (damage domain), and `ambition.combat.projectile_outcomes` (combat domain); requests `ambition.combat.answer_parry`, `ambition.body.hold_status`, and captured projectile emission (`ambition_combat`) |
//! | `body_contact` | `WorldPrepSet::AfterIntegrate`, after `boss_conduct`, before `WorldPrepSet::ContactDamage`, gameplay-gated | trigger `ambition.body.contact`; requests `ambition.body.contact_bounce` and `ambition.body.defeat` (`ambition_combat`) |

use ambition_extension_host::{ExtensionHostPlugin, ExtensionSet};

/// The declared modules as canonical text: a section of the prepared content
/// identity (D6). See `ambition_extension_host::ExtensionGeneration`.
pub use ambition_extension_host::ExtensionGeneration;

/// The name of the prepared-content section that holds [`ExtensionGeneration`].
pub const EXTENSION_MODULES_SECTION: &str = "extension.modules";
use ambition_extension_sdk::phases::{
    ACTOR_THINK, BODY_CONTACT, BOSS_CONDUCT, HIT_REACTION, MODULE_ENTITY_TICK, TECHNIQUE_EXECUTION,
    WIELDED_USE,
};
use ambition_platformer2d_shared_tangle::schedule::{
    CombatSet, GameplayGated, ItemPickupSet, SimScheduleExt,
};
use bevy::prelude::*;
mod module_files;
#[cfg(feature = "script_modules")]
pub use ambition_extension_rhai::{LibraryLimits, LibrarySource, PreparedScript, RhaiModules};
pub use module_files::{EXTENSION_MODULES_VAR, ExtensionModuleFiles};
#[cfg(any(feature = "wasm_modules", feature = "script_modules"))]
use module_files::load_developer_modules;
#[cfg(any(feature = "wasm_modules", feature = "script_modules"))]
pub use module_files::{
    ModuleFileChanges, ModuleFilesLookedWithContent, load_module_file,
    look_at_module_files, stage_module_file_changes, module_pair_was_refused,
    forget_module_pair_refusal, propose_staged_modules, publish_paired_modules,
    discard_paired_modules,
};


/// Install every port this composition offers, with its adapter: the one list
/// of which ports exist. A test harness that declares the game's modules
/// calls it after adding `ExtensionHostPlugin`, so its admission sees what the
/// game's does. It places no phase in the schedule (the plugin does).
pub fn install_ports(app: &mut App) {
    // technique_execution
    ambition_boss_encounter::extension::install(app);
    ambition_combat::projectile_spawn::extension::install(app);
    ambition_combat::extension::install(app);
    ambition_combat::extension::install_for_specials(app);
    // After the damage box: the host lowers request ports in install order.
    ambition_boss_encounter::extension::install_summons(app);
    ambition_abilities::extension::install_for_technique_execution(app);
    // A move's held portal pair (a recovery), under the same gate as the
    // boss's below.
    #[cfg(feature = "portal")]
    ambition_portal2d::extension::install_for_technique_execution(app);
    // wielded_use
    ambition_abilities::extension::install(app);
    ambition_combat::extension::install_for_wielded_use(app);
    ambition_combat::projectile_spawn::extension::install_for_wielded_use(app);
    // module_entity_tick
    ambition_abilities::extension::install_module_entities(app);
    ambition_combat::projectile_spawn::extension::install_for_module_entity_tick(app);
    ambition_combat::extension::blast::install_for_module_entity_tick(app);
    // boss_conduct: the boss-domain ports first (the pose, the drawn row, a
    // burst, a summon), then the swung volumes, the throws and the sounds.
    ambition_boss_encounter::extension::install_conduct(app);
    ambition_combat::extension::install_for_boss_conduct(app);
    ambition_combat::projectile_spawn::extension::install_for_boss_conduct(app);
    ambition_abilities::extension::install_for_boss_conduct(app);
    // The held portal pair: a game with no portals offers no such port, and
    // admission refuses an entry that asks for it.
    #[cfg(feature = "portal")]
    ambition_portal2d::extension::install_for_boss_conduct(app);
    // actor_think
    ambition_combat::extension::install_for_actor_think(app);
    ambition_abilities::extension::install_for_actor_think(app);
    // A room's group decides in one invocation (P4: the coconut swallows).
    ambition_abilities::group::install(app);
    crate::story_script::install(app);
    crate::action_replies::install(app);
    crate::actor_creation::install(app);
    crate::breakable_creation::install(app);
    crate::contact_consumption::install(app);
    ambition_platformer2d_items::inventory::install(app);
    // Where the things a scope's records name are (P5 step 3).
    ambition_abilities::whereabouts::install(app);
    // body_contact: retirement follows all ordinary request adapters.
    ambition_abilities::extension::install_for_body_contact(app);
    ambition_combat::extension::install_for_body_contact(app);
    // hit_reaction
    ambition_combat::projectile_spawn::extension::install_for_hit_reaction(app);
    ambition_combat::extension::install_for_hit_reaction(app);
    ambition_damage::extension::install(app);
    ambition_combat::projectile_outcomes::extension::install(app);
}

/// Order the phases that depend on each other: an entity spawned in
/// `wielded_use` ticks in `module_entity_tick` of the same tick. The one
/// statement of that order; a test harness that runs both phases calls it.
pub fn order_phases(app: &mut App, schedule: impl bevy::ecs::schedule::ScheduleLabel) {
    app.configure_sets(
        schedule,
        ExtensionSet::Phase(MODULE_ENTITY_TICK).after(ExtensionSet::Phase(WIELDED_USE)),
    );
}

pub struct ExtensionCompositionPlugin;

const BOSS_ENCOUNTERS: ambition_platformer2d_shared_tangle::capability::CapabilitySet =
    ambition_platformer2d_shared_tangle::capability::CapabilitySet(
        ambition_platformer2d_shared_tangle::capability::Capability::BossEncounters,
    );

impl Plugin for ExtensionCompositionPlugin {
    fn build(&self, app: &mut App) {
        let sim = app.sim_schedule();
        app.add_plugins(ExtensionHostPlugin::new(sim));
        // A module's clock is its scope's room's: a kill's hold or a slow-down
        // in one live room does not run another room's modules slow.
        app.insert_resource(ambition_extension_host::ScopePace(Box::new({
            let mut freeze: Option<
                bevy::ecs::system::SystemState<ambition_platformer2d_shared_tangle::lifecycle::RoomFreeze<'static, 'static>>,
            > = None;
            move |world: &mut World, scope: Entity| {
                freeze.get_or_insert_with(|| bevy::ecs::system::SystemState::new(world)).get(world).map_or(1.0, |freeze| freeze.scale_of(scope))
            }
        })));
        // The session root holds the session-attached module records: they
        // retire with the session, and a new session starts from initial
        // records.
        app.register_required_components::<
            ambition_platformer2d_shared_tangle::lifecycle::SessionRoot,
            ambition_extension_host::SessionRecords,
        >();
        // `technique_execution` guarantees: the asking body's facts are
        // settled, and a request is consumed this tick. `ContentSpecials`
        // sits before the effect and projectile executors that drain it.
        app.configure_sets(
            sim,
            ExtensionSet::Phase(TECHNIQUE_EXECUTION)
                .in_set(GameplayGated)
                .in_set(CombatSet::ContentSpecials),
        );
        // `hit_reaction` guarantees: this tick's hits are resolved and the
        // messages that name them are still readable (they are cleared on a
        // rewind, so a reader one tick later would diverge between peers),
        // and a request is consumed this tick. `ContentFlavor` sits between
        // the resolution and the settlement.
        app.configure_sets(
            sim,
            ExtensionSet::Phase(HIT_REACTION)
                .in_set(GameplayGated)
                .in_set(CombatSet::ContentFlavor),
        );

        // `wielded_use` guarantees: the body's control frame, kinematics and
        // gravity frame are settled (the player phase), and a request is
        // consumed this tick (the effect and projectile executors run in the
        // combat phase, after it). Before the movement cooldown ticks: a
        // module reads and arms the cooldown at the same point of the tick as
        // a native user, so a cooldown armed this tick also ticks this tick.
        app.configure_sets(
            sim,
            ExtensionSet::Phase(WIELDED_USE)
                .in_set(GameplayGated)
                .in_set(ItemPickupSet::WieldedAbilities),
        );
        app.configure_sets(
            sim,
            ExtensionSet::Phase(WIELDED_USE)
                .before(ambition_abilities::ability_cooldown::tick_ability_cooldown),
        );
        // `module_entity_tick` guarantees: an entity spawned in `wielded_use`
        // this tick exists (the ordering edge gives the spawn's commands a
        // sync point), and a request is consumed this tick.
        app.configure_sets(
            sim,
            ExtensionSet::Phase(MODULE_ENTITY_TICK)
                .in_set(GameplayGated)
                .in_set(ItemPickupSet::WieldedAbilities),
        );
        // Both phases run in the wielded-abilities set, so this plugin states
        // the set's place too: in the player phase, and asleep where the
        // room's game uses no held items. The abilities domain states the
        // same; a composition that leaves held use out (`level-platformer`)
        // has no such plugin, and without this the phases ran unordered and
        // ungated.
        app.configure_sets(
            sim,
            ItemPickupSet::WieldedAbilities
                .in_set(ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhase::PlayerSimulation)
                .in_set(ambition_platformer2d_shared_tangle::capability::CapabilitySet(
                    ambition_platformer2d_shared_tangle::capability::Capability::HeldUse,
                )),
        );
        // A transit is travel: its lowering is a body-path carry, before the
        // path's readers (constraints, contacts, crossings) this tick.
        app.configure_sets(
            sim,
            ExtensionSet::LowerPort(
                WIELDED_USE,
                <ambition_abilities::extension::TransitPort as ambition_extension_sdk::Port>::KEY,
            )
            .in_set(ambition_platformer2d_shared_tangle::schedule::BodyPathSet::Carry),
        );
        // A pull is travel: its lowering is a body-path carry, before the
        // path's readers (constraints, contacts, crossings) this tick.
        app.configure_sets(
            sim,
            ExtensionSet::LowerPort(
                MODULE_ENTITY_TICK,
                <ambition_abilities::module_entity::PullBodiesPort as ambition_extension_sdk::Port>::KEY,
            )
                .in_set(ambition_platformer2d_shared_tangle::schedule::BodyPathSet::Carry),
        );
        // `boss_conduct` guarantees: every non-boss body has integrated (the
        // boss integration runs later in `WorldPrep` and leaves a held pose
        // alone), a rider is synced to its mount (`ambition.boss.mount` reads
        // the pair as it stands), and a request is consumed this tick (the
        // effect and projectile executors run in the combat phase, after it).
        app.configure_sets(
            sim,
            ExtensionSet::Phase(BOSS_CONDUCT)
                .in_set(GameplayGated)
                .in_set(ambition_platformer2d_shared_tangle::schedule::WorldPrepSet::AfterIntegrate)
                // Conducting a boss is the boss capability's work: it sleeps
                // where the active room's game has no named bosses.
                .in_set(BOSS_ENCOUNTERS),
        );
        app.configure_sets(
            sim,
            ExtensionSet::Collect(BOSS_CONDUCT).after(ambition_mount::RidersSyncedToMounts),
        );
        // `actor_think` guarantees: the brains of this tick have published
        // their control frames and each body's target is settled; a fire asked
        // for here is in the frame the control gate restricts, and the body
        // integrates after it.
        app.configure_sets(
            sim,
            ExtensionSet::Phase(ACTOR_THINK)
                .in_set(GameplayGated)
                .after(ambition_platformer2d_shared_tangle::schedule::ActorDecisionSet::Publish)
                .before(ambition_platformer2d_shared_tangle::schedule::PlayerInputSet::ControlGate),
        );
        // `body_contact` guarantees: the bodies have integrated this tick, and
        // what a contact rule asks for is applied before the shared contact
        // damage reads the bodies, so a body it defeats hurts nobody.
        app.configure_sets(
            sim,
            ExtensionSet::Phase(BODY_CONTACT)
                .in_set(GameplayGated)
                .in_set(ambition_platformer2d_shared_tangle::schedule::WorldPrepSet::AfterIntegrate)
                .after(ExtensionSet::Phase(BOSS_CONDUCT))
                .before(ambition_platformer2d_shared_tangle::schedule::WorldPrepSet::ContactDamage),
        );
        // The side a conducted boss's module chose reaches the body through
        // its control, which the boss integration applies.
        app.add_systems(
            sim,
            ambition_boss_encounter::conduct::face_conducted_bosses
                .run_if(
                    bevy::prelude::any_with_component::<
                        ambition_boss_encounter::conduct::ConductedFacing,
                    >,
                )
                .in_set(GameplayGated)
                .in_set(ambition_platformer2d_shared_tangle::schedule::BossSteerSlot)
                .in_set(BOSS_ENCOUNTERS),
        );
        order_phases(app, sim);
        install_ports(app);
        #[cfg(any(feature = "wasm_modules", feature = "script_modules"))]
        load_developer_modules(app);
        #[cfg(not(any(feature = "wasm_modules", feature = "script_modules")))]
        if module_files::requested_module_files(app.world()).is_some_and(|files| !files.is_empty())
        {
            panic!(
                "ExtensionModuleFiles names module files, and this build loads none: enable \
                 the `wasm_modules` or `script_modules` feature, or the linked modules would \
                 run instead of the ones asked for"
            );
        }
    }
}

/// A module the game ships as a script that declares itself (`fn
/// declare()`): its text is in the binary, so every target runs it with no
/// file to read, and it joins admission as any module does. `artifact` names
/// it in faults and in inspection (its source path). To change it while the
/// game runs, list the file in `AMBITION_EXTENSION_MODULES`: a file that
/// declares the same module replaces it.
///
/// ⛔ A shipped script that does not load stops the build of the App: it is
/// part of the game.
#[cfg(feature = "script_modules")]
pub fn add_shipped_script(app: &mut App, artifact: &str, text: &str) {
    let prepared = PreparedScript::new(
        artifact, text, script_ports(), ambition_extension_rhai::DEFAULT_MAX_OPERATIONS,
    ).unwrap_or_else(|e| panic!("the shipped script {artifact} does not load: {e}"));
    add_prepared_script(app, artifact, &prepared);
}

/// Install a captured source graph on any target, with no runtime file reads.
#[cfg(feature = "script_modules")]
pub fn add_prepared_script(app: &mut App, artifact: &str, prepared: &PreparedScript) {
    use ambition_extension_host::ExtensionAppExt;
    let (backend, modules) = prepared.bind_declared()
        .unwrap_or_else(|e| panic!("the shipped script {artifact} does not bind: {e}"));
    app.add_loaded_extension_modules(artifact, backend, modules, false);
}

/// The ports a script may name, and the name it uses
/// (`this.requests.push(request("projectile", #{ .. }))`). Each port's
/// converter comes from its one description in its own crate
/// (`ambition_extension_sdk::structure`, `script_port!`). A port is added
/// here when a script needs it; an entry that uses a port not here faults on
/// its first invocation, naming the port. The generated reference is
/// `docs/sdk/script-ports.md`.
#[cfg(feature = "script_modules")]
pub fn script_ports() -> ambition_extension_sdk::script::ScriptPorts {
    let mut ports = ambition_extension_sdk::script::ScriptPorts::default();
    ports
        .add::<ambition_boss_special_port::BossSpecialCast>("boss_special")
        .add::<ambition_boss_special_port::BossConductPort>("boss_conduct")
        .add::<ambition_boss_special_port::MountViewPort>("mount")
        .add::<ambition_boss_special_port::DrawnRowPort>("drawn_row")
        .add::<ambition_combat_port::HeldRayPort>("held_ray")
        .add::<ambition_combat_port::LightningPort>("lightning")
        .add::<ambition_combat_port::LightningAuraPort>("lightning_aura")
        .add::<ambition_combat_port::TechniqueUsePort>("technique_use")
        .add::<ambition_combat_port::WieldedUsePort>("wielded_use")
        .add::<ambition_combat_port::WieldedAlternatePort>("wielded_alternate")
        .add::<ambition_combat_port::SpawnModuleEntityPort>("spawn_module_entity")
        .add::<ambition_combat_port::SpendManaPort>("spend_mana")
        .add::<ambition_combat_port::ModuleEntityTickPort>("module_entity_tick")
        .add::<ambition_projectile_spec::ProjectileSpawnPort>("projectile")
        .add::<ambition_combat_port::BodySoundPort>("sound")
        .add::<ambition_combat_port::EffectPort>("effect")
        .add::<ambition_combat_port::EndModuleEntityPort>("end_module_entity")
        .add::<ambition_combat_port::PullBodiesPort>("pull_bodies")
        .add::<ambition_combat_port::LaunchToucherPort>("launch_toucher")
        .add::<ambition_combat_port::ActorThinkPort>("actor_think")
        .add::<ambition_combat_port::ActorFirePort>("fire")
        .add::<ambition_combat_port::SetVelocityPort>("velocity")
        .add::<ambition_combat_port::GroupThinkPort>("group_think")
        .add::<ambition_combat_port::BodyContactPort>("body_contact")
        .add::<ambition_combat_port::HitReceptionsPort>("hit_receptions")
        .add::<ambition_combat_port::projectile_outcome::ProjectileOutcomesPort>("projectile_outcomes")
        .add::<ambition_combat_port::ParryWindowPort>("parry_window")
        .add::<ambition_combat_port::ContactBouncePort>("contact_bounce")
        .add::<ambition_combat_port::BodyDefeatPort>("defeat_body")
        .add::<ambition_combat_port::body_query::BodyQueryPort>("body_query")
        .add::<ambition_combat_port::body_query::BodyQueryResultPort>("body_query_result")
        .add::<ambition_extension_sdk::action::CancelActionPort>("cancel_action")
        .add::<ambition_world_port::creation::CreateActorsPort>("create_actors")
        .add::<ambition_world_port::breakables::CreateBreakablesPort>("create_breakables")
        .add::<ambition_extension_sdk::action::ActionResultPort>("action_result")
        .add::<ambition_extension_sdk::action::ActionResultsPort>("action_history")
        .add::<ambition_world_port::StoryTickPort>("story_tick")
        .add::<ambition_world_port::StoryCommandPort>("story_command")
        .add::<ambition_world_port::inventory::InventoryExchangePort>("inventory_exchange")
        .add::<ambition_world_port::consumption::ConsumeContactPort>("consume_contact")
        .add::<ambition_world_port::inventory::InventoryViewPort>("inventory")
        .add::<ambition_world_port::PlayCutscenePort>("play_cutscene")
        .add::<ambition_combat_port::SteerPort>("steer")
        .add::<ambition_combat_port::TakeItemPort>("take")
        .add::<ambition_combat_port::ReleaseItemPort>("release")
        .add::<ambition_combat_port::WhereaboutsPort>("whereabouts")
        .add::<ambition_combat_port::ParriedPort>("parried")
        .add::<ambition_combat_port::AnswerParryPort>("answer_parry")
        .add::<ambition_combat_port::StruckPort>("struck")
        .add::<ambition_combat_port::CreditPort>("credit")
        .add::<ambition_combat_port::BlastPort>("blast")
        .add::<ambition_combat_port::TimeScalePort>("time_scale")
        .add::<ambition_combat_port::StatusClockPort>("status_clock")
        .add::<ambition_combat_port::InPlayPort>("in_play")
        .add::<ambition_combat_port::HoldStatusPort>("hold_status")
        .add::<ambition_combat_port::StatusTickPort>("status_tick");
    ports
}

/// ⭐ D6: A PUBLISHED RELOAD IS A NEW GENERATION OF THE SESSION'S CONTENT.
/// When the declared modules changed, the session's prepared content gets the
/// new [`EXTENSION_MODULES_SECTION`] and a new epoch, and its identity and its
/// content binding move with it (`publish_session_content`, the road a world
/// reload publishes by). The local rollback baseline (stopped at admission) starts
/// again in `Update`, against this identity. A reload of the same code changes
/// nothing, and keeps the epoch.
///
/// ⚠ The live rooms are not built again: their roots keep the generation they
/// were built in, as the rooms a world reload does not rebuild do. A room plan
/// made before the reload is refused as stale at its boundary.
pub fn remint_session_content(world: &mut World) {
    use crate::content_identity::{ContentEpochSequence, PreparedContent};
    use ambition_platformer2d_shared_tangle::lifecycle::session_world_component;
    let Some(generation) = world.get_resource::<ExtensionGeneration>() else {
        return;
    };
    let bytes = generation.0.clone().into_bytes();
    let Some(content) = session_world_component::<PreparedContent>(world) else {
        return;
    };
    let unchanged = content
        .sections()
        .iter()
        .any(|s| s.name == EXTENSION_MODULES_SECTION && s.canonical_bytes() == bytes.as_slice());
    if unchanged {
        return;
    }
    let content = content.clone();
    let Some(mut epochs) = world.get_resource_mut::<ContentEpochSequence>() else {
        error!(
            "extension modules reloaded, and this composition has prepared content but no \
             ContentEpochSequence: the session's content identity still names the old modules"
        );
        return;
    };
    let epoch = epochs.allocate();
    let next = content
        .with_section(EXTENSION_MODULES_SECTION, bytes, epoch)
        .expect("a prepared content's sections are named and distinct");
    crate::content_identity::publish_session_content(world, next);
    info!("extension modules reloaded: the session's content is now generation {epoch:?}");
}
