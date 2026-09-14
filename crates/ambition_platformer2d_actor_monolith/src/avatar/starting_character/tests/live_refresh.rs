use ambition_characters::actor::WornCharacter;
use ambition_characters::brain::ActionSet;
use ambition_platformer2d_core::BodyAbilities;
use ambition_platformer2d_shared_tangle::markers::PrimaryPlayer;
use bevy::prelude::*;

use ambition_combat::moveset::ActorMoveset;
use ambition_platformer2d_core::movement::{MotionModel, SurfaceMomentumMotion as MomentumMotion};
use ambition_platformer2d_shared_tangle::markers::PlayerEntity;

#[derive(Resource, Default)]
struct AbilityChangeObservations(u32);

fn observe_body_ability_changes(
    changed: Query<(), Changed<BodyAbilities>>,
    mut observations: ResMut<AbilityChangeObservations>,
) {
    observations.0 += changed.iter().count() as u32;
}

/// ⛔⛤ **A BODY BUILT DURING THE SIMULATION WEARS ITS ADMITTED ABILITIES ON THE
/// SAME TICK.**
///
/// Admission and projection were ONE system in
/// `PreUpdate/MechanicalEditSet::Publish` until 2026-09-14. A body reconstructed
/// by mechanical lifecycle code — a reset, a room load, a rebuild — arrives after
/// that system has already run, so it carried its raw base into every simulation
/// consumer for the rest of the tick and was reconciled only on the NEXT render
/// frame. `ActivePlayerBodyProfile` was split for this exact reason, in these
/// words: *"a body can be rebuilt by mechanical lifecycle code, so projection
/// must follow body existence rather than editor publication."*
///
/// ⚠ **THE REVIEW DID NOT ESTABLISH THAT A SHIPPED ROAD EXPOSES THAT INTERVAL**,
/// and neither does this arm — it constructs the interval deliberately. What it
/// pins is that the interval is no longer EXPRESSIBLE: the projection follows the
/// spawner because it lives in the same schedule bodies are built in.
#[test]
fn a_body_built_after_admission_is_projected_onto_in_the_same_tick() {
    #[derive(Resource, Default)]
    struct BuiltOnce(bool);

    fn build_the_body_mid_tick(mut commands: Commands, mut built: ResMut<BuiltOnce>) {
        if built.0 {
            return;
        }
        built.0 = true;
        let base = ambition_platformer2d_core::AbilitySet::compose(&[
            ambition_platformer2d_core::AbilityGrant::RunJump,
            ambition_platformer2d_core::AbilityGrant::AirJump,
        ]);
        commands.spawn((
            PlayerEntity,
            PrimaryPlayer,
            MotionModel::default(),
            ambition_platformer2d_core::BodyKinematics::default(),
            ambition_platformer2d_shared_tangle::body::AncillaryMovementBundle::from_scratch(
                ambition_platformer2d_core::BodyClusterScratch::new_with_abilities(
                    ambition_platformer2d_core::Vec2::ZERO,
                    base,
                ),
            ),
        ));
    }

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.init_resource::<ambition_dev_tools::dev_tools::EditableAbilitySet>();
    app.init_resource::<ambition_dev_tools::dev_tools::ActiveEditableAbilityMask>();
    app.init_resource::<ambition_dev_tools::dev_tools::EditableMovementTuning>();
    app.init_resource::<ambition_platformer2d_core::ActiveMovementTuning>();
    app.init_resource::<ambition_platformer2d_core::PendingMechanicalEdits>();
    app.init_resource::<ambition_platformer2d_core::MechanicalEditAdmission>();
    app.init_resource::<BuiltOnce>();
    // The shipped shape: admission on the host frame, projection where bodies
    // are built.
    app.add_systems(
        PreUpdate,
        (
            ambition_dev_tools::propose_editable_abilities,
            ambition_dev_tools::admit_editable_abilities,
        )
            .chain(),
    );
    app.add_systems(
        Update,
        (
            build_the_body_mid_tick,
            ambition_dev_tools::project_editable_abilities,
        )
            .chain(),
    );

    // The developer switches the air jump off BEFORE any body exists.
    app.world_mut()
        .resource_mut::<ambition_dev_tools::dev_tools::EditableAbilitySet>()
        .double_jump = false;
    app.update();

    // ⛔ THE PREMISE: a body really was built, and it was built with the air jump
    // in its BASE — otherwise "it does not have one" is about an empty world or
    // an ungranted verb rather than about the projection.
    let world = app.world_mut();
    let mut query = world.query_filtered::<(
        &BodyAbilities,
        &ambition_platformer2d_core::AbilityBase,
    ), With<PrimaryPlayer>>();
    let (abilities, base) = query
        .single(world)
        .expect("the spawner built exactly one primary body");
    assert!(
        base.abilities.double_jump,
        "the body was built without the air jump in its base, so the assertion \
         below is about an ungranted verb rather than about the mask",
    );
    assert!(
        !abilities.abilities.double_jump,
        "a body built during the tick still carried its raw base into the rest \
         of that tick: admission ran before it existed and nothing projected the \
         admitted mask onto it until the next render frame",
    );
}

/// ⛔⛤ **THE MASK FILTERS THE BASE; IT DOES NOT BECOME IT.**
///
/// `SimulationSetup` took a `fallback_abilities` parameter until 2026-09-14 and
/// every caller passed `EditableAbilitySet::as_engine()`, so the developer's
/// panel WAS the base a character without an authored kit got. Two consequences,
/// and this arm is the second one — the semantic half a GPT architecture review
/// named: **an ability switched off in the panel was then absent from the base a
/// later edit is supposed to re-enable it from.** A mask that creates the base
/// it filters can only ever subtract, once.
///
/// ⭐ So: a base that GRANTS the air jump, a mask that turns it off, and then a
/// mask that turns it back on. The third step is the one that could not work
/// while the editor was the base.
#[test]
fn an_ability_the_mask_disabled_can_be_enabled_again_from_the_base() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.init_resource::<ambition_dev_tools::dev_tools::EditableAbilitySet>();
    app.init_resource::<ambition_dev_tools::dev_tools::ActiveEditableAbilityMask>();
    app.init_resource::<ambition_dev_tools::dev_tools::EditableMovementTuning>();
    app.init_resource::<ambition_platformer2d_core::ActiveMovementTuning>();
    app.init_resource::<ambition_platformer2d_core::PendingMechanicalEdits>();
    app.init_resource::<ambition_platformer2d_core::MechanicalEditAdmission>();
    app.add_systems(
        Update,
        (
            ambition_dev_tools::propose_editable_abilities,
            ambition_dev_tools::admit_editable_abilities,
            ambition_dev_tools::project_editable_abilities,
        )
            .chain(),
    );

    // THE BASE GRANTS THE AIR JUMP. This is the character's capability, not the
    // developer's opinion about it.
    let air_jump_base = ambition_platformer2d_core::AbilitySet::compose(&[
        ambition_platformer2d_core::AbilityGrant::RunJump,
        ambition_platformer2d_core::AbilityGrant::AirJump,
    ]);
    assert!(
        air_jump_base.double_jump,
        "the fixture's base does not grant the air jump, so nothing below is \
         about a mask filtering a capability",
    );
    let entity = app
        .world_mut()
        .spawn((
            PlayerEntity,
            PrimaryPlayer,
            MotionModel::default(),
            ambition_platformer2d_core::BodyKinematics::default(),
            ambition_platformer2d_shared_tangle::body::AncillaryMovementBundle::from_scratch(
                ambition_platformer2d_core::BodyClusterScratch::new_with_abilities(
                    ambition_platformer2d_core::Vec2::ZERO,
                    air_jump_base,
                ),
            ),
        ))
        .id();

    // The developer switches the air jump OFF.
    app.world_mut()
        .resource_mut::<ambition_dev_tools::dev_tools::EditableAbilitySet>()
        .double_jump = false;
    app.update();
    assert!(
        !app.world()
            .get::<BodyAbilities>(entity)
            .expect("the fixture's body has abilities")
            .abilities
            .double_jump,
        "turning the air jump off in the panel did not reach the body, so the \
         re-enable below would prove nothing",
    );

    // And back ON. Under the old road this asked the base for a verb the base no
    // longer had.
    app.world_mut()
        .resource_mut::<ambition_dev_tools::dev_tools::EditableAbilitySet>()
        .double_jump = true;
    app.update();
    assert!(
        app.world()
            .get::<BodyAbilities>(entity)
            .expect("the fixture's body has abilities")
            .abilities
            .double_jump,
        "the air jump could not be turned back ON: the developer's mask had \
         become the capability BASE, so switching a verb off removed it from the \
         set a later edit re-enables from",
    );

    // ⛔ AND THE MASK CANNOT ADD. Both edits above left `fly` ON in the panel —
    // `EditableAbilitySet::default()` is `sandbox_all` — while the base never
    // granted it. Without this the arm holds for a projection that simply WROTE
    // the mask onto the body, which is the other half of "mask, not base".
    assert!(
        !air_jump_base.fly,
        "the fixture's base already grants flight, so the assertion below cannot \
         tell a filtered mask from a copied one",
    );
    assert!(
        app.world()
            .resource::<ambition_dev_tools::dev_tools::EditableAbilitySet>()
            .fly,
        "the panel does not have flight ON, so nothing was there to leak into the \
         body and the assertion below is vacuous",
    );
    assert!(
        !app.world()
            .get::<BodyAbilities>(entity)
            .expect("the fixture's body has abilities")
            .abilities
            .fly,
        "the body gained FLIGHT, which its base never granted: the projection is \
         writing the developer's mask onto the body instead of intersecting it \
         with the base",
    );
}

/// 1. an unchanged inspector mirror must not mark `BodyAbilities` changed;
/// 2. a real ability edit on an Authored persona must not reapply movement
///    identity or erase `MomentumMotion`'s persistent riding state.
#[test]
fn live_ability_sync_does_not_rederive_authored_movement_identity() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.insert_resource(super::test_catalog());
    app.init_resource::<ambition_dev_tools::dev_tools::EditableAbilitySet>();
    // The ADMITTED mask, beside the editor resource it is admitted from.
    app.init_resource::<ambition_dev_tools::dev_tools::ActiveEditableAbilityMask>();
    app.init_resource::<ambition_dev_tools::dev_tools::EditableMovementTuning>();
    // The neutral authority `sync_live_player_dev_edits_system` reads (K1a).
    app.init_resource::<ambition_platformer2d_core::ActiveMovementTuning>();
    // ⛔⛤ **THE EDIT IS A PROPOSAL NOW (`Q120`), SO THE FIXTURE DRIVES THE
    // PROPOSER TOO.** Inserting only the editable and expecting the sync to apply
    // was true until 2026-09-13, when an ability edit stopped reaching the body
    // until something with a view of the rollback timeline admitted it. ⚠ These
    // arms are about what the sync DOES when it applies, so they run the real
    // proposer rather than setting the pending key by hand — a fixture that
    // stages its own admission is testing itself.
    app.init_resource::<ambition_platformer2d_core::PendingMechanicalEdits>();
    app.init_resource::<ambition_platformer2d_core::MechanicalEditAdmission>();
    app.init_resource::<AbilityChangeObservations>();
    app.add_systems(
        Update,
        (
            ambition_dev_tools::propose_editable_abilities,
            ambition_dev_tools::admit_editable_abilities,
            ambition_dev_tools::project_editable_abilities,
            super::super::apply_worn_character_gameplay,
            observe_body_ability_changes,
        )
            .chain(),
    );

    let entity = app
        .world_mut()
        .spawn((
            PlayerEntity,
            PrimaryPlayer,
            WornCharacter::new("sanic"),
            MotionModel::default(),
            Name::new("unset"),
            ActionSet::default(),
            ActorMoveset(Default::default()),
            ambition_platformer2d_core::BodyKinematics::default(),
            ambition_platformer2d_shared_tangle::body::AncillaryMovementBundle::from_scratch(
                ambition_platformer2d_core::BodyClusterScratch::new_with_abilities(
                    ambition_platformer2d_core::Vec2::ZERO,
                    ambition_platformer2d_core::AbilitySet::sandbox_all(),
                ),
            ),
        ))
        .id();

    app.update();
    app.world_mut()
        .resource_mut::<AbilityChangeObservations>()
        .0 = 0;
    let riding = ambition_platformer2d_core::SurfaceMotion::Riding {
        on: ambition_platformer2d_core::SurfaceRef::Chain(7),
        s: 123.0,
        v_t: 456.0,
    };
    {
        let mut model = app.world_mut().get_mut::<MotionModel>(entity).unwrap();
        match &mut *model {
            MotionModel::SurfaceMomentum(momentum) => momentum.state = riding,
            other => panic!("expected Sanic SurfaceMomentum after initial wear, got {other:?}"),
        }
    }

    // No inspector edit: merely querying the live body mutably must not create
    // a false Changed<BodyAbilities> edge.
    app.update();
    assert_eq!(
        app.world().resource::<AbilityChangeObservations>().0,
        0,
        "unchanged live dev resources must not mark BodyAbilities changed"
    );
    assert_riding_state(app.world(), entity, riding);

    // A real inspector ability edit does change BodyAbilities, but Sanic's
    // Authored persona does not derive its kit or movement from that source.
    app.world_mut()
        .resource_mut::<ambition_dev_tools::dev_tools::EditableAbilitySet>()
        .attack = false;
    app.update();
    assert_eq!(
        app.world().resource::<AbilityChangeObservations>().0,
        1,
        "the observer must see the genuine inspector edit"
    );
    assert_riding_state(app.world(), entity, riding);
}

#[test]
fn restricted_ability_base_survives_the_sandbox_default_mask() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.init_resource::<ambition_dev_tools::dev_tools::EditableAbilitySet>();
    // The ADMITTED mask, beside the editor resource it is admitted from.
    app.init_resource::<ambition_dev_tools::dev_tools::ActiveEditableAbilityMask>();
    app.init_resource::<ambition_dev_tools::dev_tools::EditableMovementTuning>();
    // The neutral authority `sync_live_player_dev_edits_system` reads (K1a).
    app.init_resource::<ambition_platformer2d_core::ActiveMovementTuning>();
    // ⛔⛤ **THE EDIT IS A PROPOSAL NOW (`Q120`), SO THE FIXTURE DRIVES THE
    // PROPOSER TOO.** Inserting only the editable and expecting the sync to apply
    // was true until 2026-09-13, when an ability edit stopped reaching the body
    // until something with a view of the rollback timeline admitted it. ⚠ These
    // arms are about what the sync DOES when it applies, so they run the real
    // proposer rather than setting the pending key by hand — a fixture that
    // stages its own admission is testing itself.
    app.init_resource::<ambition_platformer2d_core::PendingMechanicalEdits>();
    app.init_resource::<ambition_platformer2d_core::MechanicalEditAdmission>();
    app.add_systems(
        Update,
        (
            ambition_dev_tools::propose_editable_abilities,
            ambition_dev_tools::admit_editable_abilities,
            ambition_dev_tools::project_editable_abilities,
        )
            .chain(),
    );

    let run_jump = ambition_platformer2d_core::AbilitySet::compose(&[
        ambition_platformer2d_core::AbilityGrant::RunJump,
    ]);
    let entity = app
        .world_mut()
        .spawn((
            PlayerEntity,
            PrimaryPlayer,
            MotionModel::default(),
            ambition_platformer2d_core::BodyKinematics::default(),
            ambition_platformer2d_shared_tangle::body::AncillaryMovementBundle::from_scratch(
                ambition_platformer2d_core::BodyClusterScratch::new_with_abilities(
                    ambition_platformer2d_core::Vec2::ZERO,
                    run_jump,
                ),
            ),
        ))
        .id();

    for _ in 0..5 {
        app.update();
    }
    let effective = app.world().get::<BodyAbilities>(entity).unwrap().abilities;
    assert_eq!(
        effective, run_jump,
        "run-jump base must survive a sandbox_all mask unchanged"
    );
    assert!(
        !effective.blink && !effective.dash && !effective.wall_jump && !effective.fly,
        "the permissive mask must NOT conjure verbs the base lacks — masks only remove"
    );
    assert!(
        effective.jump && effective.move_horizontal,
        "the base's own verbs stay lit"
    );

    // A restrictive mask edit CAN still gate a base verb off (the dev workflow).
    app.world_mut()
        .resource_mut::<ambition_dev_tools::dev_tools::EditableAbilitySet>()
        .jump = false;
    app.update();
    assert!(
        !app.world()
            .get::<BodyAbilities>(entity)
            .unwrap()
            .abilities
            .jump,
        "the mask can still remove a verb the base grants"
    );
}

/// The tuning sibling of [`restricted_ability_base_survives_the_sandbox_default_mask`]:
/// a body that authors its own feel must read its air-jump COUNT from that
/// authored tuning, never from the shared F3 dev tuning. The dev editable
/// defaults `air_jumps` to 1 (a double jump); a demo protagonist authoring a
/// triple jump carries [`AuthoredMovementTuning`] with `air_jumps = 2`, and the
/// live sync must replenish two air jumps, not one. Without the per-body tuning
/// source the global editable would silently cap her at a double jump.
#[test]
fn authored_movement_tuning_drives_the_air_jump_count_not_the_dev_editable() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    // Default editable = air_jumps 1: the value that would cap a double jump.
    app.init_resource::<ambition_dev_tools::dev_tools::EditableAbilitySet>();
    // The ADMITTED mask, beside the editor resource it is admitted from.
    app.init_resource::<ambition_dev_tools::dev_tools::ActiveEditableAbilityMask>();
    app.init_resource::<ambition_dev_tools::dev_tools::EditableMovementTuning>();
    // The neutral authority `sync_live_player_dev_edits_system` reads (K1a).
    app.init_resource::<ambition_platformer2d_core::ActiveMovementTuning>();
    // ⛔⛤ **THE EDIT IS A PROPOSAL NOW (`Q120`), SO THE FIXTURE DRIVES THE
    // PROPOSER TOO.** Inserting only the editable and expecting the sync to apply
    // was true until 2026-09-13, when an ability edit stopped reaching the body
    // until something with a view of the rollback timeline admitted it. ⚠ These
    // arms are about what the sync DOES when it applies, so they run the real
    // proposer rather than setting the pending key by hand — a fixture that
    // stages its own admission is testing itself.
    app.init_resource::<ambition_platformer2d_core::PendingMechanicalEdits>();
    app.init_resource::<ambition_platformer2d_core::MechanicalEditAdmission>();
    app.add_systems(
        Update,
        (
            ambition_dev_tools::propose_editable_abilities,
            ambition_dev_tools::admit_editable_abilities,
            ambition_dev_tools::project_editable_abilities,
        )
            .chain(),
    );

    // A base that grants the air-jump capability (RunJump + AirJump).
    let air_jump_base = ambition_platformer2d_core::AbilitySet::compose(&[
        ambition_platformer2d_core::AbilityGrant::RunJump,
        ambition_platformer2d_core::AbilityGrant::AirJump,
    ]);
    let entity = app
        .world_mut()
        .spawn((
            PlayerEntity,
            PrimaryPlayer,
            MotionModel::default(),
            ambition_platformer2d_core::BodyKinematics::default(),
            ambition_platformer2d_shared_tangle::body::AncillaryMovementBundle::from_scratch(
                ambition_platformer2d_core::BodyClusterScratch::new_with_abilities(
                    ambition_platformer2d_core::Vec2::ZERO,
                    air_jump_base,
                ),
            ),
            // Authored feel: a TRIPLE jump (two air jumps).
            ambition_platformer2d_core::AuthoredMovementTuning(
                ambition_platformer2d_core::MovementTuning {
                    air_jumps: 2,
                    ..ambition_platformer2d_core::DEFAULT_TUNING
                },
            ),
        ))
        .id();

    // Force the sync's cluster refresh to run this frame by diverging the
    // effective set from the base (the sync early-returns when they already
    // agree); the refresh is where `air_jumps_available` is recomputed.
    app.world_mut()
        .get_mut::<BodyAbilities>(entity)
        .unwrap()
        .abilities = ambition_platformer2d_core::AbilitySet::NONE;
    app.update();

    let available = app
        .world()
        .get::<ambition_platformer2d_core::BodyJumpState>(entity)
        .unwrap()
        .air_jumps_available;
    assert_eq!(
        available, 2,
        "the authored tuning's air_jumps (2) must drive the count, not the \
         editable default (1)"
    );
}

fn assert_riding_state(
    world: &World,
    entity: Entity,
    expected: ambition_platformer2d_core::SurfaceMotion,
) {
    let model = world.get::<MotionModel>(entity).unwrap();
    match model {
        MotionModel::SurfaceMomentum(MomentumMotion { state, .. }) => {
            assert_eq!(
                *state, expected,
                "ability synchronization must preserve the surface follower's persistent state"
            );
        }
        other => panic!("expected Sanic SurfaceMomentum, got {other:?}"),
    }
}

/// A CROSS-model runtime re-wear (momentum persona → axis persona) preserves
/// every shared body fact (world position, velocity, facing) and initializes
/// ONLY the destination policy's private state — the ADR 0024 §7 swap
/// invariant, exercised through the production worn-character seam and
/// therefore independent of who controls the body (the system never reads a
/// controller).
#[test]
fn cross_model_rewear_preserves_shared_state_and_initializes_axis_private_state() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.insert_resource(super::test_catalog());
    app.add_systems(Update, super::super::apply_worn_character_gameplay);

    let entity = app
        .world_mut()
        .spawn((
            PlayerEntity,
            PrimaryPlayer,
            WornCharacter::new("sanic"),
            MotionModel::default(),
            Name::new("unset"),
            ActionSet::default(),
            ActorMoveset(Default::default()),
            ambition_platformer2d_core::BodyKinematics::default(),
            ambition_platformer2d_shared_tangle::body::AncillaryMovementBundle::from_scratch(
                ambition_platformer2d_core::BodyClusterScratch::new_with_abilities(
                    ambition_platformer2d_core::Vec2::ZERO,
                    ambition_platformer2d_core::AbilitySet::sandbox_all(),
                ),
            ),
        ))
        .id();
    app.update();
    assert!(
        matches!(
            app.world().get::<MotionModel>(entity).unwrap(),
            MotionModel::SurfaceMomentum(_)
        ),
        "wearing sanic selects the momentum policy"
    );

    // Live shared state accumulated while riding as sanic. Axis maneuver
    // state cannot even EXIST under the momentum policy now (it lives inside
    // the AxisSwept variant, ADR 0024 O4) — the assertions below pin that the
    // fresh axis destination starts with none.
    let pose = ambition_platformer2d_core::BodyKinematics {
        pos: ambition_platformer2d_core::Vec2::new(321.0, 654.0),
        vel: ambition_platformer2d_core::Vec2::new(900.0, -50.0),
        size: ambition_platformer2d_core::Vec2::new(24.0, 40.0),
        facing: -1.0,
    };
    *app.world_mut()
        .get_mut::<ambition_platformer2d_core::BodyKinematics>(entity)
        .unwrap() = pose;
    app.world_mut()
        .get_mut::<ambition_platformer2d_core::BodyDashState>(entity)
        .unwrap()
        .charges_available = 2;

    app.world_mut().entity_mut(entity).insert((
        WornCharacter::new("player"),
        ambition_characters::actor::RecharacterizeBody,
    ));
    app.update();

    assert!(
        matches!(
            app.world().get::<MotionModel>(entity).unwrap(),
            MotionModel::AxisSwept(_)
        ),
        "re-wearing the protagonist selects the axis policy"
    );
    assert_eq!(
        *app.world()
            .get::<ambition_platformer2d_core::BodyKinematics>(entity)
            .unwrap(),
        pose,
        "world pose, velocity, and facing survive the swap untouched"
    );
    let MotionModel::AxisSwept(axis) = app.world().get::<MotionModel>(entity).unwrap() else {
        unreachable!("asserted axis-swept above");
    };
    assert_eq!(
        axis.state.coyote_timer, 0.0,
        "the axis destination begins with NO imported coyote grace"
    );
    assert!(!axis.state.wall_clinging, "no imported wall engagement");
    assert_eq!(
        app.world()
            .get::<ambition_platformer2d_core::BodyDashState>(entity)
            .unwrap()
            .charges_available,
        2,
        "body RESOURCES (dash charges) are shared facts and survive"
    );
}
