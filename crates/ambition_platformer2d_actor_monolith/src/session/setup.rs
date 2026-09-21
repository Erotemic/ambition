//! Sim/presentation split for the sandbox's startup setup.
//!
//! This module factors the sim half into [`simulation_world`] so the headless binary can build the
//! world without presentation, while the visible-app setup keeps that seam clean.
//!
//! [`simulation_world`] takes `&mut Commands` plus borrowed resource handles
//! ([`SimulationSetup`]) so it can be invoked from any Bevy startup system
//! that has gathered the right parameters. It is not a Bevy system itself;
//! the `ambition_app` crate's startup setup (`app/setup_systems.rs`) does the
//! param wiring and pairs it with the presentation-side spawns.

use ambition_platformer2d_core as ae;
use bevy::prelude::*;

use ambition_platformer2d_core::config::{world_to_bevy, WORLD_Z_PLAYER};
use ambition_platformer2d_core::RoomGeometry;
use ambition_platformer2d_shared_tangle::lifecycle::PlayerVisual;
use ambition_platformer2d_shared_tangle::lifecycle::{SessionSpawnScope, SpawnSessionScopedExt};
use ambition_platformer2d_world::rooms::RoomSet;

/// The health pool a playable body gets when its worn character authors none.
///
/// Ambition's own protagonist takes real damage over a run; a character whose
/// game is "one hit and you start over" says so on its catalog row rather than
/// bending this.
pub const DEFAULT_PLAYER_HEALTH: i32 = 20;

/// Borrowed inputs for `simulation_world`.
///
/// Grouped as a struct because Bevy's max-system-param budget is tight and
/// keeping these as positional args would push the calling startup system
/// past 16 params again. The struct also documents what the simulation
/// half of setup actually needs.
pub struct SimulationSetup<'a> {
    /// ⛔ THE SESSION ROOT THIS SETUP IS BUILDING INTO. Setup publishes the
    /// session's content generation ON it (see `ActiveContentBinding`), so it
    /// must be handed the entity rather than looking up *"the live root"* — at
    /// activation the live root is the OUTGOING session's, or none at all.
    pub session_root: bevy::prelude::Entity,
    /// What happens to the first room's PUBLICATION RECEIPT.
    ///
    /// ⛔ A caller that will ask `publication_succeeded` — the shell provider,
    /// whose candidate session becomes authoritative only if this room published
    /// — says `UntilOwnerRetires` and owes a `retire_publication`. A direct-entry
    /// demo drops the handle and says `UntilTheVerdictIsRecorded`.
    pub publication_retention: crate::world::rooms::transaction::PublicationRetention,
    pub world: &'a RoomGeometry,
    pub room_set: &'a RoomSet,
    pub tuning: &'a ae::ActiveMovementTuning,
    /// Which catalog character the local player spawns as. `is_default()` (the
    /// `player` protagonist) takes the untouched `from_scratch` path.
    /// Whether this session builds a home body, and who it wears if so.
    ///
    /// a MATCH experience declares `NoInitialBody`: it realizes its own cast
    /// from a prepared roster, and a privileged avatar beside that cast is an
    /// actor nobody owns — the camera follows it and input drives it while the
    /// fighter the player chose stands somewhere else.
    pub initial_body: &'a crate::avatar::InitialBodyPolicy,
    /// The prepared cast, when this composition registered one.
    ///
    /// `None` is the ordinary case for a composition that registers no
    /// characters — not a degraded one — which is why it is an `Option` rather
    /// than a required authority like the catalog beside it.
    ///
    /// Setup needs it because the player is a body being CONSTRUCTED, and a
    /// prepared character states what a body physically is. Without it, the worn
    /// player took its health from the catalog row and its mass and box from
    /// nowhere, while a seated fighter wearing the same character took all three
    /// from the definition.
    pub prepared_characters: Option<&'a ambition_characters::prepared::PreparedCharacterRegistry>,
    /// App-local hostile archetype definitions used by authored room lowering.
    /// The installed App-local placement-lowering authority. Setup lowers the
    /// start room's authored placements through THIS registry — the same one
    /// room transition and snapshot restore consume — so there is no
    /// setup-only reconstruction of the six built-in interpreters.
    pub placement_lowering: &'a crate::world::placements::PlacementLoweringRegistry,
    /// The App-installed room-content staging seam. Setup drains the start
    /// room's registered content stagers exactly as transition, reset,
    /// hot-reload, and restore staging do — one construction authority.
    pub content_staging: &'a crate::features::RoomContentStagingRegistry,
    /// The App-installed construction recipe table plus the content generation
    /// this session was prepared under (Phase 3 planned families).
    pub construction: crate::features::ActorConstructionContext<'a>,
    /// App-local boss profiles, encounter specs, sheets, and special rows.
    pub boss_catalog: &'a ambition_boss_encounter::BossCatalog,
    /// Provider-selected default used only when `StartingCharacter` is empty.
    pub default_character_id: &'a str,
}

/// Spawn simulation-only entities and resources.
///
/// Returns the player entity so `presentation_world` (or any future RL
/// adapter) can attach presentation components without re-querying.
///
/// This includes:
/// * logging room layout warnings
/// * spawning the `LdtkWorldBundle` so `bevy_ecs_ldtk` can own LDtk entity
///   lifecycle and the runtime-spine systems have something to query
/// * spawning the player entity with gameplay-essential ECS components
///   (`PlayerSimulationBundle` for sim clusters plus `Transform`,
///   `PlayerVisual`, etc.).
///   Leafwing's `ActionState` and `InputMap` live on the persistent
///   `InputParticipant` entity (spawned once at boot by the host input
///   plugin), NEVER on the player/actor entities; sim-only builds stay
///   leafwing-free per the ADR 0012 input seam.
/// What one simulation setup produced: the session's home body, if it built one,
/// and the receipt of the first room it built.
pub struct SimulationWorld {
    /// The session's home body, when the experience declared one.
    pub player: Option<Entity>,
    /// The first room's publication. A caller that asked to retain it decides
    /// what happens to the session from this verdict.
    pub publication: crate::world::rooms::transaction::PublicationHandle,
    /// The first room's moving-platform state — INSTALLED already for a
    /// published session, and owed by the caller's publication for a candidate
    /// one. See the comment at its construction.
    pub moving_platforms: ambition_platformer2d_world::collision::MovingPlatformSet,
}

pub fn simulation_world(
    commands: &mut Commands,
    session_scope: SessionSpawnScope,
    params: SimulationSetup<'_>,
) -> SimulationWorld {
    let SimulationSetup {
        session_root,
        publication_retention,
        world,
        room_set,
        tuning,
        initial_body,
        prepared_characters,
        placement_lowering,
        content_staging,
        construction,
        boss_catalog,
        default_character_id,
    } = params;
    // ⛔ ONE CARRIER FOR THE CATALOG. Setup used to take it as its own
    // `SimulationSetup` field AND hand it to room construction, which took it as
    // two more positional arguments — three spellings of one authority, any two
    // of which could be handed different snapshots without a compiler noticing.
    // It rides on the construction context now, beside the prepared cast and the
    // policies, and this is the one place setup names it.
    let character_catalog = construction.characters;

    for warning in room_set.layout_warnings() {
        bevy::log::debug!(target: "ambition_platformer2d::room_layout", "{warning}");
    }
    // The LdtkWorldBundle spawn lives in the Ldtk-runtime startup system
    // (`crate::schedule::add_ldtk_runtime_plugin`) because asset_server.load on a
    // typed `LdtkProject` handle requires `LdtkPlugin` to be registered.
    // Headless builds skip LdtkPlugin (its tile pipeline needs RenderApp),
    // so this function must not assume the LDtk asset type is available.
    // AND SIMULATION SETUP NO LONGER TOUCHES ASSETS AT ALL.
    // `ldtk_index` went first, then `sandbox_data_asset`,
    // `sandbox_asset_collection` and `asset_server` followed as this comment
    // predicted they would. All four were borrowed here and read by nothing:
    // the two collections were cloned into `_`-prefixed locals that dropped on
    // the next line, which keeps NOTHING alive — the resources holding those
    // handles are what keep the assets loaded, and they outlive this call by
    // construction. The cost of the superstition was structural, not runtime:
    // it made an LDtk asset handle look like something a headless simulation
    // needed, and it kept an `AssetServer` in the provider's system params for
    // the sole purpose of handing it to a `let _ =`.

    // The session's content generation, published for the commit boundary:
    // every later room transaction (transition, reset, reconstruction) must be
    // prepared against THIS binding or be refused publication as stale.
    commands
        .entity(session_root)
        .insert(crate::world::rooms::transaction::ActiveContentBinding(
            construction.binding,
        ));
    let room_plan = crate::rooms::RoomConstructionPlan::prepare_from_parts(
        room_set,
        room_set.active(),
        placement_lowering,
        content_staging,
        boss_catalog,
        session_scope,
        construction,
    )
    .unwrap_or_else(|error| panic!("initial room construction failed: {error}"));
    // ⛔⛤ **THE FIRST ROOM'S RECEIPT IS THE ACTIVATION DECISION.** A candidate
    // session becomes authoritative only if the room it was built around
    // published, so the caller that owns that decision retains this handle. See
    // `SimulationSetup::publication_retention`.
    let publication = room_plan.spawn_contents(commands, publication_retention);
    // ⛔⛤ **A CANDIDATE SESSION MAY NOT WRITE THE LIVE PLATFORM STATE — 2026-09-14.**
    // This installed the first room's moving platforms as a process resource the
    // instant the room was STAGED. Constructing a candidate session beside a live
    // one would therefore have replaced the playable session's platforms with the
    // candidate's before anything had verified the candidate — the room's bodies
    // hidden, the world they move in already swapped.
    //
    // ⇒ A PUBLISHED session installs its own live state, as it always did; a
    // CANDIDATE carries it out as data and its publication installs it. The value
    // is returned either way, so the caller cannot mistake which happened.
    let moving_platforms = ambition_platformer2d_world::collision::MovingPlatformSet(
        room_plan.platform_states().to_vec(),
    );
    if session_scope.visibility()
        == ambition_platformer2d_shared_tangle::lifecycle::SessionSpawnVisibility::Published
    {
        commands.insert_resource(ambition_platformer2d_world::collision::MovingPlatformSet(
            moving_platforms.0.clone(),
        ));
    }

    let crate::avatar::InitialBodyPolicy::SpawnCharacter(starting_character) = initial_body else {
        return SimulationWorld {
            player: None,
            publication,
            moving_platforms,
        };
    };

    // Capability set travels WITH the worn character when the row authors one
    // (the per-character analogue of the motion model below): a restricted-kit
    // demo character — classic run + jump — declares it in the catalog instead of
    // forcing the whole multi-game host onto the session's shared set. A
    // row without an authored set keeps that shared sandbox set, so Ambition's own
    // protagonist is untouched.
    // ⛔⛤ **THE SHARED FALLBACK IS AN ENGINE CONSTANT, AND IT WAS A PARAMETER
    // UNTIL 2026-09-14.** The field's own doc said *"AN ENGINE VALUE, NOT THE
    // DEV-TOOLS MIRROR … who edits the set is the caller's business"* — and every
    // one of the four callers passed `editable_abilities.as_engine()`. Removing
    // the type from this signature moved the upward dependency on
    // `ambition_dev_tools` to the caller; it did not remove it.
    //
    // ⇒ **SO THE PARAMETER IS GONE.** A developer's ability selection is a MASK
    // over this base (`ActiveEditableAbilityMask`, projected by
    // `project_editable_abilities` as `base ∩ mask`), never the base
    // itself. While the editor WAS the base, a value the rollback timeline had
    // REFUSED still entered simulation through construction — and an ability the
    // developer had switched off was absent from the base a later edit is
    // supposed to re-enable it from. Both are inexpressible now: there is no
    // argument to pass. Found by the GPT architecture review 2026-09-14, which
    // named one of the four call sites; the other three are the demos and the
    // host smoke test.
    //
    // ⚠ `sandbox_all()` is what all four passed in practice —
    // `EditableAbilitySet::default()` IS `AbilitySet::sandbox_all()`, so this
    // preserves behaviour exactly for an untouched panel.
    let base_abilities = character_catalog
        .ability_set(starting_character.effective_id(default_character_id))
        .unwrap_or_else(ae::AbilitySet::sandbox_all);
    let mut initial_scratch = crate::avatar::primary_player_scratch(world.0.spawn, base_abilities);
    ae::refresh_movement_resources_clusters(
        &initial_scratch.abilities,
        &mut initial_scratch.dash,
        &mut initial_scratch.jump,
        &mut initial_scratch.dodge,
        tuning.air_jumps,
        // A body being built has nothing outstanding.
        ae::RecoveryRefresh::Answered,
    );

    // The player is a control box that WEARS a character. The protagonist takes
    // the untouched canonical path; any other selected character overlays its
    // moveset + name onto the same box (its sprite is bound presentation-side).
    //
    // What the body physically IS travels with the worn character, through
    // the same resolver a seated fighter uses. How fragile it is, how much it
    // weighs, and how big its box is are one statement made once — a
    // classic-platformer character authors `max_health: 1` (armor absorbs, then
    // the next hit is fatal) rather than forcing the whole host onto a one-hit
    // pool, and a character that authors none keeps the standard pool, so
    // Ambition's own protagonist is untouched.
    //
    // The registry now folds the catalog row at its barrier, so consulting the prepared value is
    // strictly more informed than consulting the row — and a registered-only character (every
    // versus fighter) has no row to consult.
    let worn_id = starting_character.effective_id(default_character_id);
    let physical = prepared_characters
        .and_then(|registry| registry.get(worn_id))
        .map(ambition_body_seed::PhysicalBaseline::of);
    let player_health = ambition_characters::actor::Health::new(match physical.as_ref() {
        Some(physical) => physical.max_health_over(DEFAULT_PLAYER_HEALTH),
        // No prepared character: the catalog row is still the authority for the
        // legacy cast, which is most of it.
        None => character_catalog
            .max_health(worn_id)
            .unwrap_or(DEFAULT_PLAYER_HEALTH),
    });
    // The authored BOX, on the exploration player. `SpriteAuthored` needs nothing here — its
    // per-pose projection reaches every body on every path — so this is the `Explicit` case,
    // which was seating-only until now.
    if let Some(size) = physical.as_ref().and_then(|p| p.explicit_size()) {
        initial_scratch.kinematics.size = size;
        initial_scratch.base_size.base_size = size;
    }
    // HOW THIS BODY FIRES, resolved by the overlay the bundle already runs
    // and kept rather than discarded — see below.
    let mut ranged = ambition_characters::brain::RangedExecution::ChargedProjectile;
    let player_bundle = if starting_character.is_default() {
        crate::avatar::PlayerSimulationBundle::from_scratch(initial_scratch, player_health)
    } else {
        crate::avatar::PlayerSimulationBundle::from_scratch_as_character(
            character_catalog,
            initial_scratch,
            player_health,
            starting_character.character_id.as_str(),
            // the prepared cast, which this function already held and the
            // bundle was not given — see the parameter's own note.
            prepared_characters,
            &mut ranged,
        )
    };
    // Session ownership is captured by the caller when world construction
    // is requested. Deferred command application cannot reassign this body to a
    // later activation. Historical startup/RL callers pass `UNSCOPED`.
    let player = commands
        .spawn_session_scoped(
            session_scope,
            (
                Transform::from_translation(world_to_bevy(&world.0, world.0.spawn, WORLD_Z_PLAYER)),
                PlayerVisual,
                // The canonical playable-persona identity: WHICH catalog character
                // this control box wears. Simulation-owned, so gameplay config AND
                // presentation both derive from this ONE relationship instead of
                // rediscovering the selection from separate authorities. Resolved to
                // a concrete id (the content default when unset) so the identity is
                // never empty on the entity.
                ambition_characters::actor::WornCharacter::new(
                    starting_character.effective_id(default_character_id),
                ),
                player_bundle,
            ),
        )
        .id();

    // Two pieces were outstanding and both are answered here. The projectile
    // MARKERS a `Bundle` cannot conditionally omit — the overlay resolved how
    // this body fires and the bundle discarded that answer, so it is kept now.
    // And the applied-template stamp, with an EMPTY displacement: nothing was
    // taken from a body that was BUILT as this character.
    crate::avatar::sync_charge_projectile_capability(commands, player, ranged, false);
    commands
        .entity(player)
        .insert(ambition_body_seed::PersonaBaseline {
            id: starting_character
                .effective_id(default_character_id)
                .to_string(),
            generation: prepared_characters
                .map(ambition_characters::prepared::PreparedCharacterRegistry::generation)
                .unwrap_or_default(),
            displaced: Default::default(),
        });

    // The authored MASS. Health and the box are already on the body above (both
    // are construction inputs the bundle consumes); this is the remainder, and it
    // goes through the shared applier so the exploration player and a seated
    // fighter cannot drift apart again.
    if let Some(physical) = physical.as_ref() {
        physical.apply_to_body(
            ambition_body_seed::BaselineBoundary::Construction,
            &mut commands.entity(player),
            None,
            None,
            None,
            ambition_body_seed::PhysicalRetraction::NONE,
        );
    }

    // Movement identity travels WITH the worn character. Every body already
    // carries one explicit policy; the App-local catalog selects or refreshes
    // that policy without using component absence as an axis-swept sentinel.
    crate::avatar::apply_worn_motion_model(
        character_catalog,
        commands,
        player,
        starting_character.effective_id(default_character_id),
    );

    // The player entity is returned to the caller (the provider session builder
    // or the direct-entry startup system). Presentation discovers this home
    // avatar by its `PrimaryPlayer` marker — no process-global handle bag records
    // it — and spawns the HUD/quest text as session-scoped, marker-tagged
    // entities during its own setup.
    //
    // `Option`: "there is always exactly one primary player"
    // was an engine-wide assumption, and a match experience is the counterexample.
    SimulationWorld {
        player: Some(player),
        publication,
        moving_platforms,
    }
}
