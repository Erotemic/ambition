//! Initial session bodies from prepared or live session inputs.
//!
//! Room publication creates the world before body construction. Prepared
//! candidates supply [`SimulationSetup`]. Direct hosts use
//! [`LiveInitialBodySetup`] to read the live session's policy and frozen cast.

use ambition_platformer2d_core as ae;
use bevy::prelude::*;

use ambition_platformer2d_core::config::{world_to_bevy, WORLD_Z_PLAYER};
use ambition_platformer2d_core::RoomGeometry;
use ambition_platformer2d_shared_tangle::lifecycle::PlayerVisual;
use ambition_platformer2d_shared_tangle::lifecycle::{
    SessionRoot, SessionSpawnScope, SessionWorldRef, SpawnSessionScopedExt,
};

/// The health pool a playable body gets when its worn character authors none.
///
/// Ambition's own protagonist takes real damage over a run; a character whose
/// game is "one hit and you start over" says so on its catalog row rather than
/// bending this.
pub const DEFAULT_PLAYER_HEALTH: i32 = 20;

/// Prepared inputs for initial-body construction.
/// Candidate preparation supplies these values before publication.
pub struct SimulationSetup<'a> {
    pub world: &'a RoomGeometry,
    pub tuning: &'a ae::ActiveMovementTuning,
    /// Whether this session builds a home body, and who it wears if so.
    ///
    /// a MATCH experience declares `NoInitialBody`: it realizes its own cast
    /// from a prepared roster, and a privileged avatar beside that cast is an
    /// actor nobody owns — the camera follows it and input drives it while the
    /// fighter the player chose stands somewhere else.
    pub initial_body: &'a crate::initial_body::InitialBodyPolicy,
    /// What the home body holds, when there is one.
    pub home_body_resources: &'a crate::initial_body::HomeBodyResources,
    /// What the experience grants and permits the home body.
    pub home_body_abilities: &'a ambition_platformer2d_actor_monolith::avatar::HomeBodyAbilities,
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
    /// Provider-selected default used only when `StartingCharacter` is empty.
    pub default_character_id: &'a str,
}

/// Initial-body construction from the live session's configuration.
///
/// The session supplies policy, grants and its frozen cast. A prepared candidate
/// supplies its own generation through [`SimulationSetup`] instead.
#[derive(bevy::ecs::system::SystemParam)]
pub struct LiveInitialBodySetup<'w, 's> {
    tuning: Res<'w, ae::ActiveMovementTuning>,
    policy: SessionWorldRef<'w, 's, crate::initial_body::InitialBodyPolicy>,
    body_inputs: Query<
        'w,
        's,
        (
            &'static crate::initial_body::HomeBodyResources,
            &'static ambition_platformer2d_actor_monolith::avatar::HomeBodyAbilities,
        ),
        With<SessionRoot>,
    >,
    cast: ambition_characters::session_cast::SessionCast<'w>,
}

impl LiveInitialBodySetup<'_, '_> {
    pub fn spawn(
        &self, sprite_content: &ambition_sprite_sheet::SpriteContent,
        commands: &mut Commands,
        scope: SessionSpawnScope,
        world: &RoomGeometry,
        default_character_id: &str,
    ) -> SimulationWorld {
        if !self.policy.spawns_a_body() {
            return SimulationWorld { player: None };
        }
        let (resources, abilities) = self
            .body_inputs
            .single()
            .expect("a session that requests a home body must supply its resources and abilities");
        simulation_world(&sprite_content,
            commands,
            scope,
            SimulationSetup {
                world,
                tuning: &self.tuning,
                initial_body: &self.policy,
                home_body_resources: resources,
                home_body_abilities: abilities,
                prepared_characters: self.cast.get(),
                default_character_id,
            },
        )
    }
}

/// The home body produced by initial session construction, when requested.
pub struct SimulationWorld {
    /// The session's home body, when the experience declared one.
    pub player: Option<Entity>,
}

pub fn simulation_world(sprite_content: &ambition_sprite_sheet::SpriteContent,
    commands: &mut Commands,
    session_scope: SessionSpawnScope,
    params: SimulationSetup<'_>,
) -> SimulationWorld {
    let SimulationSetup {
        world,
        tuning,
        initial_body,
        home_body_resources,
        home_body_abilities,
        prepared_characters,
        default_character_id,
    } = params;
    // The first room is the session's activation room, published before this
    // (`ambition_platformer2d_publication::publish_first_room`), so the home body that starts
    // in it is in instance #0.
    let session_scope = session_scope.in_room(Some(
        ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance::ACTIVATION,
    ));

    let crate::initial_body::InitialBodyPolicy::SpawnCharacter(starting_character) = initial_body else {
        return SimulationWorld { player: None };
    };

    let player = spawn_home_body(&sprite_content,
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
    pub character: &'a crate::initial_body::StartingCharacter,
    /// Provider-selected default used only when `character` is empty.
    pub default_character_id: &'a str,
    /// The prepared cast, when this composition registered one.
    pub prepared_characters: Option<&'a ambition_characters::prepared::PreparedCharacterRegistry>,
    /// What the home body holds.
    pub resources: &'a crate::initial_body::HomeBodyResources,
    /// What the experience grants and permits the home body.
    pub abilities: &'a ambition_platformer2d_actor_monolith::avatar::HomeBodyAbilities,
}

impl HomeBody<'_> {
    fn definition(&self) -> Option<&ambition_characters::prepared::PreparedCharacterDefinition> {
        self.prepared_characters?.get(self.character.effective_id(self.default_character_id))
    }

    pub(crate) fn placement_shape(&self, sprite_content: &ambition_sprite_sheet::SpriteContent) -> ae::BodySpawnShape {
        let definition = self.definition();
        let size = definition.map(|prepared| ambition_body_seed::PhysicalBaseline::of(&sprite_content, prepared))
            .and_then(|physical| physical.standing_size())
            .unwrap_or_else(ae::default_player_body_size);
        let model = definition.map_or(
            ae::MotionModelSpec::AxisSwept(Default::default()),
            |definition| definition.motion_model,
        );
        model.spawn_shape(size, ae::DEFAULT_GRAVITY_DIR)
    }
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
pub fn spawn_home_body(sprite_content: &ambition_sprite_sheet::SpriteContent,
    commands: &mut Commands,
    session_scope: SessionSpawnScope,
    body: HomeBody<'_>,
    markers: impl Bundle,
) -> Entity {
    let physical = body.definition().map(|prepared| ambition_body_seed::PhysicalBaseline::of(&sprite_content, prepared));
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
    let base_abilities = abilities.base_for(
        prepared_characters
            .and_then(|registry| registry.get(worn_id))
            .and_then(|prepared| prepared.abilities),
    );
    let mut initial_scratch = ambition_platformer2d_actor_monolith::avatar::home_body_scratch(at, base_abilities);
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
    let player_bundle = ambition_platformer2d_actor_monolith::avatar::PlayerSimulationBundle::from_scratch_as_character(
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
    ambition_platformer2d_actor_monolith::avatar::sync_charge_projectile_capability(commands, player, ranged, false);
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
            ambition_platformer2d_actor_spawn::grant_prepared_character_body(&sprite_content,
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
        None => ambition_platformer2d_actor_monolith::avatar::apply_worn_motion_model(
            prepared_characters,
            commands,
            player,
            character.effective_id(default_character_id),
        ),
    }
    player
}

#[cfg(test)]
mod tests;
