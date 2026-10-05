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
    /// Where the first room's commit gets what the save says about its bodies.
    ///
    /// ⛔ The shell provider states the save of the session's own experience:
    /// its session is built while another one can be live. A direct-entry demo
    /// has one experience and says `TheWorldAtTheCommit`.
    pub first_room_facts: crate::construction::CommitFactsSource,
    pub world: &'a RoomGeometry,
    pub room_set: &'a RoomSet,
    pub tuning: &'a ae::ActiveMovementTuning,
    /// Whether this session builds a home body, and who it wears if so.
    ///
    /// a MATCH experience declares `NoInitialBody`: it realizes its own cast
    /// from a prepared roster, and a privileged avatar beside that cast is an
    /// actor nobody owns — the camera follows it and input drives it while the
    /// fighter the player chose stands somewhere else.
    pub initial_body: &'a crate::avatar::InitialBodyPolicy,
    /// What the home body holds, when there is one.
    pub home_body_resources: &'a crate::avatar::HomeBodyResources,
    /// What the experience grants and permits the home body.
    pub home_body_abilities: &'a crate::avatar::HomeBodyAbilities,
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
    pub placement_lowering: &'a crate::construction::placements::PlacementLoweringRegistry,
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
}

pub fn simulation_world(
    commands: &mut Commands,
    session_scope: SessionSpawnScope,
    params: SimulationSetup<'_>,
) -> SimulationWorld {
    let SimulationSetup {
        session_root,
        publication_retention,
        first_room_facts,
        world,
        room_set,
        tuning,
        initial_body,
        home_body_resources,
        home_body_abilities,
        prepared_characters,
        placement_lowering,
        content_staging,
        construction,
        boss_catalog,
        default_character_id,
    } = params;
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
    // The first room is the session's activation room: nothing has been
    // published into this session yet, so its occupants, and the home body
    // that starts in it, are in instance #0.
    let session_scope = session_scope.in_room(Some(
        ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance::ACTIVATION,
    ));
    let room_plan = crate::rooms::RoomConstructionPlan::prepare_from_parts(
        room_set,
        room_set.activation(),
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
    let publication = room_plan.spawn_contents(commands, publication_retention, first_room_facts);
    // The first room's moving platforms go onto THIS session's live room root,
    // hidden with it when the session is a candidate. A candidate cannot write
    // the playing session's platforms, because they are on another root.
    // (Until OW1 cut 3b they were one process resource, so a candidate carried
    // them out as data and its adoption installed them.)
    let moving_platforms = ambition_platformer2d_world::collision::MovingPlatformSet(
        room_plan.platform_states().to_vec(),
    );
    let scope_id = session_scope.id();
    commands.queue(move |world: &mut bevy::prelude::World| {
        use ambition_platformer2d_shared_tangle::lifecycle::{
            live_room_root_for, sole_live_room_entity, LiveRoomInstance,
        };
        let root = match scope_id {
            Some(scope) => live_room_root_for(world, scope, LiveRoomInstance::ACTIVATION),
            None => sole_live_room_entity(world),
        };
        match root {
            Some(root) => {
                world.entity_mut(root).insert(moving_platforms);
            }
            None => bevy::log::error!(
                target: "ambition_platformer2d::construction",
                "session {scope_id:?} has no live room root for its first room's \
                 moving platforms"
            ),
        }
    });

    let crate::avatar::InitialBodyPolicy::SpawnCharacter(starting_character) = initial_body else {
        return SimulationWorld {
            player: None,
            publication,
        };
    };

    let player = spawn_home_body(
        commands,
        session_scope,
        HomeBody {
            seat: ambition_characters::control::PlayerSlot::PRIMARY,
            at: world.0.spawn,
            world,
            tuning,
            character: starting_character,
            default_character_id,
            prepared_characters,
            resources: home_body_resources,
            abilities: home_body_abilities,
        },
        // The primary body of the session: the camera follows it and the
        // gravity of the session resolves at it.
        (
            ambition_platformer2d_shared_tangle::markers::PrimaryPlayer,
            ambition_platformer2d_shared_tangle::body::PrimaryBody,
        ),
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
    }
}

/// What the home body of one seat is built from.
///
/// The experience states each input once, on the session root or in the App,
/// and each seat of the session gets the same ones: the worn character, what
/// the experience grants and permits it, and what it holds. A seat differs
/// from the next one in two facts only, who drives it and where it stands.
pub struct HomeBody<'a> {
    /// The seat that drives the body. It gives the canonical identity
    /// (`slot:N`) and the `DrivingParticipant`.
    pub seat: ambition_characters::control::PlayerSlot,
    /// Where the body stands, in room coordinates.
    pub at: ae::Vec2,
    /// The room the position is in.
    pub world: &'a RoomGeometry,
    pub tuning: &'a ae::ActiveMovementTuning,
    /// The character the body wears. Empty is the default of the provider.
    pub character: &'a crate::avatar::StartingCharacter,
    /// Provider-selected default used only when `character` is empty.
    pub default_character_id: &'a str,
    /// The prepared cast, when this composition registered one.
    pub prepared_characters: Option<&'a ambition_characters::prepared::PreparedCharacterRegistry>,
    /// What the home body holds.
    pub resources: &'a crate::avatar::HomeBodyResources,
    /// What the experience grants and permits the home body.
    pub abilities: &'a crate::avatar::HomeBodyAbilities,
}

/// Build the home body of one seat, owned by the session of `session_scope`.
///
/// ⛔ ONE RECIPE FOR EACH SEAT. The primary body of a session is seat 0 built
/// here, and its caller gives the two primary markers as `markers`. A body for
/// another seat gives `()`: it has the identity `slot:N` and is driven by
/// seat N, and it is not primary. `ensure_sim_id` gives a primary body with no
/// identity `slot:0`, so a primary marker on a second body is a second claim
/// on the identity of the first.
///
/// The body is session-scoped and not room-scoped, so a room replay does not
/// sweep it. The room instance it is in comes from `session_scope`.
///
/// ⚠ No production caller builds a seat above 0. When a second seat joins,
/// where it enters and what it shares are open (`Q153`).
pub fn spawn_home_body(
    commands: &mut Commands,
    session_scope: SessionSpawnScope,
    body: HomeBody<'_>,
    markers: impl Bundle,
) -> Entity {
    let HomeBody {
        seat,
        at,
        world,
        tuning,
        character,
        default_character_id,
        prepared_characters,
        resources,
        abilities,
    } = body;
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
    // over this base (`ActiveEditableAbilityMask`, contributed as a ceiling and
    // folded in by `project_body_abilities`), never the base
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
    //
    // The worn character answers first: what its prepared definition AUTHORS
    // (`None` is no contribution, not `NONE`; the barrier has already folded
    // the catalog row's grants into it), then the host baseline. The persona derive only READS `BodyAbilities`, so
    // an answer not given here is never given — the default V3 stood in
    // `sandbox_all` (reset and grab included) while wearing a set that grants
    // neither.
    //
    // The experience then grants and permits over that kit
    // (`HomeBodyAbilities`): Morph Ball reaches Ambition's home body this way,
    // and never through the character or the fallback below.
    let worn_id = character.effective_id(default_character_id);
    let authored_abilities = prepared_characters
        .and_then(|registry| registry.get(worn_id))
        .and_then(|prepared| prepared.abilities)
        .unwrap_or(ae::AbilitySet {
            // An unauthored body gets every intrinsic verb. Morph Ball is
            // progression an experience grants, so a fallback never carries it.
            morph: false,
            ..ae::AbilitySet::sandbox_all()
        });
    let base_abilities = abilities.apply(authored_abilities);
    let mut initial_scratch = crate::avatar::primary_player_scratch(at, base_abilities);
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
    let physical = prepared_characters
        .and_then(|registry| registry.get(worn_id))
        .map(ambition_body_seed::PhysicalBaseline::of);
    let player_health = ambition_characters::actor::Health::new(match physical.as_ref() {
        Some(physical) => physical.max_health_over(DEFAULT_PLAYER_HEALTH),
        // An id the cast does not hold: every catalog row is prepared (AP30),
        // so no row is left to ask.
        None => DEFAULT_PLAYER_HEALTH,
    });
    // The authored BOX, on the exploration player, built at the size it stands in:
    // an `Explicit` character's box, or a `SpriteAuthored` character's standing
    // rectangle. The pose pass projects poses from there; it does not size the
    // body into existence.
    //
    // The IDENTITY box is only the explicit one. A sprite character's standing
    // box is granted by `grant_prepared_character_body` below, which records
    // the box it displaced so a later change of character can put it back;
    // seeding the sprite box here would make that record the sprite box too.
    if let Some(size) = physical.as_ref().and_then(|p| p.standing_size()) {
        initial_scratch.kinematics.size = size;
    }
    if let Some(size) = physical.as_ref().and_then(|p| p.explicit_size()) {
        initial_scratch.base_size.base_size = size;
    }
    // HOW THIS BODY FIRES, resolved by the overlay the bundle already runs
    // and kept rather than discarded — see below.
    let mut ranged = ambition_characters::brain::RangedExecution::ChargedProjectile;
    // ONE road for every resolved id. An unselected start still wears the
    // content default, and `PersonaBaseline` below says that persona was
    // applied — so building the host-code kit here instead left the default
    // character's authored repertoire unapplied, with the derive told it was
    // current.
    let player_bundle = crate::avatar::PlayerSimulationBundle::from_scratch_as_character(
        seat,
        initial_scratch,
        player_health,
        worn_id,
        // the prepared cast, which this function already held and the
        // bundle was not given — see the parameter's own note.
        prepared_characters,
        &mut ranged,
    );
    // Session ownership is captured by the caller when world construction
    // is requested. Deferred command application cannot reassign this body to a
    // later activation. Historical startup/RL callers pass `UNSCOPED`.
    //
    // ONE SPAWN, with the markers of the caller in it: an observer of this
    // spawn sees the primary body with its markers.
    let player = commands
        .spawn_session_scoped(
            session_scope,
            (
                Transform::from_translation(world_to_bevy(&world.0, at, WORLD_Z_PLAYER)),
                PlayerVisual,
                // The canonical playable-persona identity: WHICH catalog character
                // this control box wears. Simulation-owned, so gameplay config AND
                // presentation both derive from this ONE relationship instead of
                // rediscovering the selection from separate authorities. Resolved to
                // a concrete id (the content default when unset) so the identity is
                // never empty on the entity.
                ambition_characters::actor::WornCharacter::new(
                    character.effective_id(default_character_id),
                ),
                player_bundle,
                markers,
            ),
        )
        .id();

    // How this body fires, which a `Bundle` cannot conditionally carry: the
    // overlay resolved it and the bundle holds no answer of its own. And the
    // applied-template stamp, with an EMPTY displacement: nothing was taken
    // from a body that was BUILT as this character.
    crate::avatar::sync_charge_projectile_capability(commands, player, ranged, false);
    // What the body holds, from the same prepared bank a reset returns it to
    // the start of. Absent is the answer for an experience that declared none.
    if let Some(bank) = resources.bank() {
        commands.entity(player).insert(bank.clone());
    }
    commands
        .entity(player)
        .insert(ambition_body_seed::PersonaBaseline {
            id: character
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

    // THE PREPARED BODY, granted at construction like every other character
    // body: the posed silhouette, authored hurtboxes, movement feel and motion
    // model land with the player, in one batch, rather than on its first tick
    // from the re-template pass. The kit is the worn derive's. An id the cast
    // does not hold gets the default movement identity.
    match prepared_characters.and_then(|registry| registry.get(worn_id).map(|p| (registry, p))) {
        Some((registry, prepared)) => {
            ambition_platformer2d_actor_spawn::grant_prepared_character_body(
                &mut ambition_platformer2d_shared_tangle::construction::EntityScope::new(
                    commands, player,
                ),
                prepared,
                registry.generation(),
                ambition_platformer2d_actor_spawn::KitOwnership::PersonaDerive,
                prepared.movement_tuning,
                ambition_characters::repertoire::Hand::Empty,
            );
        }
        None => crate::avatar::apply_worn_motion_model(
            prepared_characters,
            commands,
            player,
            character.effective_id(default_character_id),
        ),
    }
    player
}
