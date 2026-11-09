//! Mary-O Classic presentation/control glue.
//!
//! The actual physics live in the reusable `AxisSwept` momentum-horizontal and
//! phased-gravity jump laws authored on Mary-O's catalog row. This module owns
//! only her two-gear input grammar — walk by default, run while her `run`
//! technique is held — plus that technique's press edge, which the cinder
//! beacon makes a spark.
//!
//! `run` is a technique on the Attack slot (she has no melee verb), so every
//! device reaches it through the button it binds to Attack, and the shared
//! gate routes the edge (`ResolvedTechniqueEdges`). No device binds a key for
//! her.
//!
//! The throttle remains body-local. Acceleration, coasting, skidding, airborne
//! momentum, speed-banded launch, held/released gravity, collision, and rotated
//! gravity frames are all handled by the shared movement kernel.

use bevy::prelude::*;

use ambition_platformer2d::characters::action_scheme::ResolvedTechniqueEdges;
use ambition_platformer2d::characters::control::ActorControl;
use ambition_platformer2d::characters::equipment::{EquipmentGrant, WornEquipment};
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::platformer::markers::PlayerEntity;

/// The id of her run technique, and of the edge the gate routes for it.
pub(crate) const RUN: &str = "run";

/// The policy. Scale the body-local locomotion throttle down to a walk unless
/// the run technique is held.
///
/// Runs after the control gate has routed this tick's technique edges and
/// before the shared movement phase consumes the frame, so the scaled throttle
/// flows through the ENTIRE ordinary path — brain intent, the movement kernel,
/// replay, and rollback — rather than being applied at a device adapter where the
/// simulation could never see the difference between a walk and a half-pushed
/// stick.
pub fn walk_by_default_run_while_held(
    mut bodies: Query<(&mut ActorControl, &ResolvedTechniqueEdges), With<PlayerEntity>>,
) {
    for (mut control, edges) in &mut bodies {
        let frame = &mut control.0;
        if !edges.edge(RUN).held {
            // A pure throttle cut. The TARGET speed drops; accumulated velocity is
            // left to the kernel's acceleration, which is what makes releasing run
            // a deceleration rather than a snap.
            frame.locomotion.x *= crate::rules::mary_o_rules().walk_throttle;
        }
    }
}

/// The same button's press edge fires a spark, while its held level keeps
/// meaning run.
///
/// This is the dual-purpose half of the classic grammar, and it only works because
/// the slot's edge and level both survive into the simulation. Firing is a press —
/// there is no charge and no release edge to wait for.
///
/// It does not spawn anything. It raises the body's ordinary `fire` intent, which
/// the shared moveset picks up as the `"ranged"` verb; the projectile the beacon
/// granted is what actually launches, through the one shared projectile path.
/// The moveset refuses the move while the weapon recharges or while as many
/// sparks fly as the spark authors (`max_live`), so this system keeps no clock
/// and no count of its own.
pub fn fire_spark_on_run_press(
    mut bodies: Query<
        (&mut ActorControl, &ResolvedTechniqueEdges, &ae::BodyKinematics, &WornEquipment),
        With<PlayerEntity>,
    >,
) {
    for (mut control, edges, kin, worn) in &mut bodies {
        if !armed(worn) || !edges.pressed(RUN) {
            continue;
        }
        let frame = &mut control.0;
        // Primarily along her facing; the shot's own authored gravity supplies the
        // arc, so no launch angle is baked in here.
        frame.fire = Some(
            ambition_platformer2d::characters::actor::control::ActorFireRequest::controlled_body_local(
                ae::Vec2::new(kin.facing.signum(), 0.0),
            ),
        );
    }
}

/// Whether a worn row grants her a ranged verb. The spark is the beacon row's
/// own grant, so the button asks for the grant, not for the row's id.
fn armed(worn: &WornEquipment) -> bool {
    worn.rows
        .iter()
        .flat_map(|row| &row.grants)
        .any(|grant| matches!(grant, EquipmentGrant::Ranged(_)))
}

/// The run technique on the Attack slot, labelled `label`.
///
/// One button, two roles, and the prompt says so: the classic run button is
/// the attack position, and it fires the spark. Her rules give the driven
/// body `Run` (a `DrivenTechniques` rule), and the cinder beacon grants
/// `Run / Spark` on the same slot, which replaces it while she wears the beacon
/// (`EquipmentGrant::Technique`). Declaring it as a technique is what puts it
/// in the action scheme, so the physical binding stays configurable and the
/// shared control prompt draws it with no demo-side UI code.
pub(crate) fn run_technique(label: &str) -> ambition_platformer2d::entity_catalog::action_scheme::ActionSpec {
    use ambition_platformer2d::entity_catalog::action_scheme as sch;
    sch::ActionSpec {
        id: sch::ActionId::new(RUN),
        slot: sch::ControlSlot::Attack,
        display_name: Some(label.to_string()),
        visual: None,
        gate: sch::ActionGate::Technique(RUN.to_string()),
    }
}

#[cfg(test)]
mod tests;
