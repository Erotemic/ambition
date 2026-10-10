//! The extension host in the platformer composition (fast-iteration I4).
//!
//! The runtime chooses where each public phase sits in the simulation
//! schedule, and which domain adapters answer which ports. The host and the
//! adapters name no game algorithm; the game declares modules from its own
//! plugins.
//!
//! | phase | placement | ports |
//! |---|---|---|
//! | `technique_execution` | `CombatSet::ContentSpecials`, gameplay-gated | trigger `ambition.boss.special_cast` (boss domain); requests `ambition.projectiles.spawn` (projectile domain), `ambition.combat.damage_box` and `ambition.combat.held_damage_box` (combat domain), `ambition.boss.summon` (boss domain) |
//! | `wielded_use` | `ItemPickupSet::WieldedAbilities`, after the native wielded users and before the movement cooldown ticks, gameplay-gated | trigger `ambition.items.wielded_use`; requests `ambition.motion.transit` (lowered first, in `BodyPathSet::Carry`), `ambition.abilities.movement_cooldown`, `ambition.feedback.effect`, `ambition.resources.spend_mana`, `ambition.feedback.body_sound` and `ambition.world.spawn_module_entity` (held-item domain, `ambition_abilities`), `ambition.combat.damage_box` and `ambition.combat.strike`, `ambition.projectiles.spawn` |
//! | `boss_conduct` | `WorldPrepSet::AfterIntegrate`, gameplay-gated | trigger `ambition.boss.conduct`; requests `ambition.boss.conducted_pose`, `ambition.presentation.drawn_row`, `ambition.feedback.burst`, `ambition.boss.summon` (boss domain), `ambition.combat.held_damage_box`, `ambition.combat.riding_hitbox` (combat), `ambition.projectiles.spawn`, `ambition.feedback.body_sound` |
//! | `module_entity_tick` | `ItemPickupSet::WieldedAbilities`, after `wielded_use`, gameplay-gated | trigger `ambition.world.module_entity_tick`; requests `ambition.feedback.body_sound`, `ambition.world.pull_bodies` (lowered in `BodyPathSet::Carry`) and `ambition.world.end_module_entity` (`ambition_abilities`), `ambition.projectiles.spawn` |

use ambition_extension_host::{ExtensionHostPlugin, ExtensionSet};

/// The declared modules as canonical text: a section of the prepared content
/// identity (D6). See `ambition_extension_host::ExtensionGeneration`.
pub use ambition_extension_host::ExtensionGeneration;

/// The name of the prepared-content section that holds [`ExtensionGeneration`].
pub const EXTENSION_MODULES_SECTION: &str = "extension.modules";
use ambition_extension_sdk::phases::{BOSS_CONDUCT, MODULE_ENTITY_TICK, TECHNIQUE_EXECUTION, WIELDED_USE};
use ambition_platformer2d_shared_tangle::schedule::{CombatSet, GameplayGated, ItemPickupSet, SimScheduleExt};
use bevy::prelude::*;

/// Install every port this composition offers, with its adapter: the one list
/// of which ports exist. A test harness that declares the game's modules
/// calls it after adding `ExtensionHostPlugin`, so its admission sees what the
/// game's does. It places no phase in the schedule (the plugin does).
pub fn install_ports(app: &mut App) {
    // technique_execution
    ambition_boss_encounter::extension::install(app);
    ambition_projectiles::extension::install(app);
    ambition_combat::extension::install(app);
    // After the damage box: the host lowers request ports in install order.
    ambition_boss_encounter::extension::install_summons(app);
    // wielded_use
    ambition_abilities::extension::install(app);
    ambition_combat::extension::install_for_wielded_use(app);
    ambition_projectiles::extension::install_for_wielded_use(app);
    // module_entity_tick
    ambition_abilities::extension::install_module_entities(app);
    ambition_projectiles::extension::install_for_module_entity_tick(app);
    // boss_conduct: the boss-domain ports first (the pose, the drawn row, a
    // burst, a summon), then the swung volumes, the throws and the sounds.
    ambition_boss_encounter::extension::install_conduct(app);
    ambition_combat::extension::install_for_boss_conduct(app);
    ambition_projectiles::extension::install_for_boss_conduct(app);
    ambition_abilities::extension::install_for_boss_conduct(app);
}

/// Order the phases that depend on each other: an entity spawned in
/// `wielded_use` ticks in `module_entity_tick` of the same tick. The one
/// statement of that order; a test harness that runs both phases calls it.
pub fn order_phases(app: &mut App, schedule: impl bevy::ecs::schedule::ScheduleLabel) {
    app.configure_sets(
        schedule,
        ExtensionSet::Collect(MODULE_ENTITY_TICK).after(ExtensionSet::Lower(WIELDED_USE)),
    );
}

pub struct ExtensionCompositionPlugin;

impl Plugin for ExtensionCompositionPlugin {
    fn build(&self, app: &mut App) {
        let sim = app.sim_schedule();
        app.add_plugins(ExtensionHostPlugin::new(sim));
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
            (
                ExtensionSet::Collect(TECHNIQUE_EXECUTION),
                ExtensionSet::Invoke(TECHNIQUE_EXECUTION),
                ExtensionSet::Lower(TECHNIQUE_EXECUTION),
            )
                .in_set(GameplayGated)
                .in_set(CombatSet::ContentSpecials),
        );

        // `wielded_use` guarantees: the body's control frame, kinematics and
        // gravity frame are settled (the player phase), and a request is
        // consumed this tick (the effect and projectile executors run in the
        // combat phase, after it). After the native wielded users, so mana
        // has one order of spenders. Before the movement cooldown ticks: a
        // module reads and arms the cooldown at the same point of the tick as
        // a native user, so a cooldown armed this tick also ticks this tick.
        app.configure_sets(
            sim,
            (
                ExtensionSet::Collect(WIELDED_USE),
                ExtensionSet::Invoke(WIELDED_USE),
                ExtensionSet::Lower(WIELDED_USE),
            )
                .in_set(GameplayGated)
                .in_set(ItemPickupSet::WieldedAbilities)
                .after(ambition_abilities::traversal::grapple::grapple_system),
        );
        app.configure_sets(
            sim,
            ExtensionSet::Lower(WIELDED_USE).before(ambition_abilities::ability_cooldown::tick_ability_cooldown),
        );
        // `module_entity_tick` guarantees: an entity spawned in `wielded_use`
        // this tick exists (the ordering edge gives the spawn's commands a
        // sync point), and a request is consumed this tick.
        app.configure_sets(
            sim,
            (
                ExtensionSet::Collect(MODULE_ENTITY_TICK),
                ExtensionSet::Invoke(MODULE_ENTITY_TICK),
                ExtensionSet::Lower(MODULE_ENTITY_TICK),
            )
                .in_set(GameplayGated)
                .in_set(ItemPickupSet::WieldedAbilities),
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
        // alone), and a request is consumed this tick (the effect and
        // projectile executors run in the combat phase, after it).
        app.configure_sets(
            sim,
            (
                ExtensionSet::Collect(BOSS_CONDUCT),
                ExtensionSet::Invoke(BOSS_CONDUCT),
                ExtensionSet::Lower(BOSS_CONDUCT),
            )
                .in_set(GameplayGated)
                .in_set(ambition_platformer2d_shared_tangle::schedule::WorldPrepSet::AfterIntegrate),
        );
        // The side a conducted boss's module chose reaches the body through
        // its control, which the boss integration applies.
        app.add_systems(
            sim,
            ambition_boss_encounter::conduct::face_conducted_bosses
                .in_set(GameplayGated)
                .in_set(ambition_platformer2d_shared_tangle::schedule::BossSteerSlot),
        );
        order_phases(app, sim);
        install_ports(app);
        #[cfg(feature = "wasm_modules")]
        load_developer_modules(app);
        #[cfg(not(feature = "wasm_modules"))]
        if app
            .world()
            .get_resource::<ExtensionModuleFiles>()
            .is_some_and(|files| !files.0.is_empty())
        {
            panic!(
                "ExtensionModuleFiles names module files, and this build has no WebAssembly \
                 backend: enable the `wasm_modules` feature, or the linked modules would run \
                 instead of the ones asked for"
            );
        }
    }
}

/// The variable that names loaded module files for a developer run.
pub const EXTENSION_MODULES_VAR: &str = "AMBITION_EXTENSION_MODULES";

/// The `.wasm` files (or directories of them) whose modules replace the
/// linked ones. A composition inserts it BEFORE adding the engine plugins;
/// when it is absent, [`EXTENSION_MODULES_VAR`] gives the list. Read once, at
/// plugin build.
#[derive(Resource, Clone, Debug, Default)]
pub struct ExtensionModuleFiles(pub Vec<std::path::PathBuf>);

/// ⭐ THE NO-RELINK LOOP. `AMBITION_EXTENSION_MODULES` is a `:`-separated list
/// of `.wasm` files or directories of them. Each file's modules are declared
/// as EXPLICIT replacements of the modules the game links under the same key,
/// so a developer edits a module, rebuilds only the module crate
/// (`scripts/build_extension_modules.sh`), and runs the same game binary.
///
/// ⛔ A file that does not load stops the run. The developer asked for this
/// file; running the linked module instead would show them behaviour they did
/// not write.
#[cfg(feature = "wasm_modules")]
fn load_developer_modules(app: &mut App) {
    use ambition_extension_host::ExtensionAppExt;
    let listed = match app.world().get_resource::<ExtensionModuleFiles>() {
        Some(files) => files.0.clone(),
        None => match std::env::var(EXTENSION_MODULES_VAR) {
            Ok(list) => list.split(':').filter(|p| !p.is_empty()).map(Into::into).collect(),
            Err(_) => return,
        },
    };
    let mut watched = Vec::new();
    for path in wasm_files(&listed) {
        let bytes = std::fs::read(&path)
            .unwrap_or_else(|e| panic!("{EXTENSION_MODULES_VAR}: cannot read {}: {e}", path.display()));
        let (backend, modules) = ambition_extension_wasm::WasmModules::load(&bytes)
            .unwrap_or_else(|e| panic!("{EXTENSION_MODULES_VAR}: {} does not load: {e}", path.display()));
        info!(
            "{EXTENSION_MODULES_VAR}: {} provides {:?}",
            path.display(),
            modules.iter().map(|m| m.key.to_string()).collect::<Vec<_>>()
        );
        app.add_loaded_extension_modules(&path.display().to_string(), backend, modules, true);
        watched.push((path.clone(), modified(&path)));
    }
    if watched.is_empty() {
        return;
    }
    // ⭐ HOT RELOAD, ON THE ENGINE'S MECHANICAL-EDIT PROTOCOL (`Q120`): a
    // changed file is PROPOSED, the rollback timeline's owner ADMITS (and may
    // stop its own baseline to do so), and only then is the new code
    // PUBLISHED. A file that fails to load or to admit is reported and the
    // running code stays.
    app.insert_resource(WatchedModuleFiles {
        files: watched,
        frames_until_poll: 0,
    });
    app.init_resource::<ambition_platformer2d_core::PendingMechanicalEdits>();
    app.init_resource::<ambition_platformer2d_core::MechanicalEditAdmission>();
    ambition_platformer2d_shared_tangle::schedule::configure_mechanical_edit_sets(app);
    app.add_systems(
        PreUpdate,
        (
            propose_module_reload.in_set(ambition_platformer2d_core::MechanicalEditSet::Propose),
            publish_module_reload.in_set(ambition_platformer2d_core::MechanicalEditSet::Publish),
        ),
    );
}

/// The loaded files and the modification time each was last read at.
#[cfg(feature = "wasm_modules")]
#[derive(Resource)]
struct WatchedModuleFiles {
    files: Vec<(std::path::PathBuf, Option<std::time::SystemTime>)>,
    frames_until_poll: u32,
}

/// Frames between two looks at the files: a stat per file is cheap, and a
/// third of a second is below a developer's switch from editor to game.
#[cfg(feature = "wasm_modules")]
const POLL_FRAMES: u32 = 20;

#[cfg(feature = "wasm_modules")]
struct ExtensionModuleCode;

#[cfg(feature = "wasm_modules")]
fn module_code_domain() -> ambition_platformer2d_core::MechanicalDomain {
    ambition_platformer2d_core::MechanicalDomain::of::<ExtensionModuleCode>("extension module code")
}

#[cfg(feature = "wasm_modules")]
fn modified(path: &std::path::Path) -> Option<std::time::SystemTime> {
    std::fs::metadata(path).and_then(|m| m.modified()).ok()
}

#[cfg(feature = "wasm_modules")]
fn propose_module_reload(world: &mut World) {
    let changed: Vec<std::path::PathBuf> = {
        let mut watched = world.resource_mut::<WatchedModuleFiles>();
        if watched.frames_until_poll > 0 {
            watched.frames_until_poll -= 1;
            return;
        }
        watched.frames_until_poll = POLL_FRAMES;
        let mut changed = Vec::new();
        for (path, seen) in &mut watched.files {
            let now = modified(path);
            if now.is_some() && now != *seen {
                *seen = now;
                changed.push(path.clone());
            }
        }
        changed
    };
    // ⛔ ONE POLL IS ONE CANDIDATE: every changed file that loads goes into
    // one staged generation (`stage_loaded_replacements`). A file that does
    // not load keeps its running code; the others still reload.
    let mut loaded = Vec::new();
    for path in &changed {
        match std::fs::read(path)
            .map_err(|e| format!("cannot read: {e}"))
            .and_then(|bytes| ambition_extension_wasm::WasmModules::load(&bytes).map_err(|e| e.to_string()))
        {
            Ok((backend, modules)) => loaded.push(ambition_extension_host::reload::LoadedArtifact {
                artifact: std::sync::Arc::from(path.display().to_string()),
                backend,
                modules,
            }),
            Err(reason) => error!(
                "{EXTENSION_MODULES_VAR}: {} changed and does NOT load; its running code \
                 stays: {reason}",
                path.display()
            ),
        }
    }
    if loaded.is_empty() {
        return;
    }
    let names: Vec<String> = loaded.iter().map(|a| a.artifact.to_string()).collect();
    match ambition_extension_host::reload::stage_loaded_replacements(world, loaded) {
        Ok(()) => {
            info!("{EXTENSION_MODULES_VAR}: {names:?} changed; reload proposed");
            world
                .resource_mut::<ambition_platformer2d_core::PendingMechanicalEdits>()
                .propose(module_code_domain());
        }
        Err(reason) => error!(
            "{EXTENSION_MODULES_VAR}: {names:?} changed and are NOT loaded; the running \
             code stays: {reason}"
        ),
    }
}

#[cfg(feature = "wasm_modules")]
fn publish_module_reload(world: &mut World) {
    if *world.resource::<ambition_platformer2d_core::MechanicalEditAdmission>()
        != ambition_platformer2d_core::MechanicalEditAdmission::Publish
    {
        return;
    }
    if world
        .resource_mut::<ambition_platformer2d_core::PendingMechanicalEdits>()
        .take(module_code_domain())
        && ambition_extension_host::reload::publish_staged_replacement(world)
    {
        remint_session_content(world);
    }
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

#[cfg(feature = "wasm_modules")]
fn wasm_files(listed: &[std::path::PathBuf]) -> Vec<std::path::PathBuf> {
    let mut files = Vec::new();
    for path in listed.iter().cloned() {
        if path.is_dir() {
            let mut found: Vec<_> = std::fs::read_dir(&path)
                .unwrap_or_else(|e| panic!("{EXTENSION_MODULES_VAR}: cannot list {}: {e}", path.display()))
                .filter_map(|entry| entry.ok().map(|e| e.path()))
                .filter(|p| p.extension().is_some_and(|x| x == "wasm"))
                .collect();
            found.sort();
            files.extend(found);
        } else {
            files.push(path);
        }
    }
    files
}
