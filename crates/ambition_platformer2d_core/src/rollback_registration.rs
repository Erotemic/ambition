//! The engine floor's own rollback declarations.
//!
//! ⭐⭐ WHY THE FLOOR DECLARES ITS OWN, and it was an open judgement rather than
//! an oversight. The runtime IS the engine composition and these are not a
//! foreign domain's types, so "the composition may declare the floor's state" is
//! a defensible reading — but `ambition_time` is equally a floor crate and has
//! declared its own since 2026-08-26. Two floor crates answering the same
//! question differently is the split the federation exists to end, and the
//! hand-kept list had already grown a duplicate row nobody noticed
//! (`QuestAdvanceRequested`, twice, twenty lines apart, silently deduped). A list
//! nobody owns grows rows nobody notices; that is the argument as a defect rather
//! than as a principle.
//!
//! ⭐ AND THE PAYOFF IS COMPILER-VISIBLE: a new body-cluster component's
//! registration is now an obligation next to the type, not a thing to remember in
//! a distant crate.
//!
//! ⛔ THE STABLE NAMES DO NOT MOVE. They are identities on the wire, not
//! addresses, so relocating a declaration is not a schema change — only the OWNER
//! string moves, from the composition to the crate that defines the types. Same
//! rule the clock's move and `ambition_persistence`'s followed.

use crate::snapshot::RollbackRegistrar;

const OWNER: &str = env!("CARGO_PKG_NAME");

pub fn register_rollback_state<R>(registrar: &mut R)
where
    R: RollbackRegistrar,
{
    use crate::body_clusters as bc;

    // The canonical live-session root's geometry.
    registrar.rollback_component_clone::<crate::RoomGeometry>(OWNER, "root.geometry");

    // Core body state.
    registrar
        .rollback_component_canonical::<bc::BodyAbilities>(OWNER, "body.abilities")
        .rollback_component_canonical::<bc::BodyGroundState>(OWNER, "body.ground")
        .rollback_component_canonical::<bc::BodyWallState>(OWNER, "body.wall")
        .rollback_component_canonical::<bc::BodyJumpState>(OWNER, "body.jump")
        .rollback_component_canonical::<bc::BodyDashState>(OWNER, "body.dash")
        .rollback_component_canonical::<bc::BodyFlightState>(OWNER, "body.flight")
        .rollback_component_canonical::<bc::BodyBlinkState>(OWNER, "body.blink")
        .rollback_component_canonical::<bc::BodyDodgeState>(OWNER, "body.dodge")
        .rollback_component_canonical::<bc::BodyShieldState>(OWNER, "body.shield")
        .rollback_component_canonical::<bc::BodyOffense>(OWNER, "body.offense")
        .rollback_component_canonical::<bc::BodyLifetime>(OWNER, "body.lifetime")
        .rollback_component_canonical::<bc::BodyActionBuffer>(OWNER, "body.action_buffer")
        .rollback_component_canonical::<bc::BodyBaseSize>(OWNER, "body.base_size")
        .rollback_component_canonical::<bc::SweepSample>(OWNER, "body.sweep_sample")
        .rollback_component_canonical::<bc::BodyMana>(OWNER, "body.mana");

    // Per-body state a live match lands on, and the body's own shape.
    registrar
        .rollback_component_canonical::<crate::geometry::CenteredAabb>(OWNER, "actor.centered_aabb")
        .rollback_component_canonical::<bc::BodyModeState>(OWNER, "actor.body_mode")
        .rollback_component_canonical::<bc::BodyLedgeState>(OWNER, "actor.ledge")
        .rollback_component_canonical::<crate::MotionModel>(OWNER, "actor.motion_model")
        .rollback_component_canonical::<bc::BodyComboTrace>(OWNER, "actor.combo_trace");

    // Value-bearing bookkeeping a recreated entity cannot re-derive.
    registrar.rollback_component_clone::<bc::AbilityBase>(OWNER, "body.ability_base");

    // DECLARED DERIVED — not state, and each says what rebuilds it.
    registrar
        .declare_rollback_derived_component::<bc::BodyEnvironmentContact>(
            OWNER,
            "derived.body_environment_contact",
            "rewritten every movement step from body geometry and the live world",
        )
        .declare_rollback_derived_component::<crate::BodyMotionFacts>(
            OWNER,
            "derived.body_motion_facts",
            "republished from MotionModel every movement step",
        )
        .declare_rollback_derived_resource::<crate::control_frame::ControlFrame>(
            OWNER,
            "derived.control_frame",
            "per-tick input frame regenerated from the synchronized input stream",
        );
}
