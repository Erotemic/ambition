//! Mary-O Classic presentation/control glue.
//!
//! The actual physics live in the reusable `AxisSwept` momentum-horizontal and
//! phased-gravity jump laws authored on Mary-O's catalog row. This module owns
//! only her two-gear input grammar — walk by default, run while the modifier is
//! held — plus the modifier press-edge used by the cinder beacon.
//!
//! The throttle remains body-local. Acceleration, coasting, skidding, airborne
//! momentum, speed-banded launch, held/released gravity, collision, and rotated
//! gravity frames are all handled by the shared movement kernel.

use bevy::prelude::*;

use ambition_platformer2d::characters::control::ActorControl;
use ambition_platformer2d::characters::equipment::{EquipmentGrant, WornEquipment};
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::platformer::markers::PrimaryPlayer;


/// Walking is 60% of Mary-O's run speed: 180 px/s versus 300 px/s in the
/// initial classic profile. Her catalog row owns the absolute cap; this system
/// owns only the semantic walk/run ratio.
pub const WALK_THROTTLE: f32 = 0.6;

/// The policy. Scale the body-local locomotion throttle down to a walk unless
/// the modifier slot is sustained.
///
/// Runs after the brain has produced this tick's `ActorControl` and before the
/// shared movement phase consumes it, so the scaled throttle flows through the
/// ENTIRE ordinary path — brain intent, `InputState`, the movement kernel,
/// replay, and rollback — rather than being applied at a device adapter where the
/// simulation could never see the difference between a walk and a half-pushed
/// stick.
pub fn walk_by_default_run_while_held(
    mut bodies: Query<&mut ActorControl, With<PrimaryPlayer>>,
) {
    for mut control in &mut bodies {
        let frame = &mut control.0;
        if !frame.modifier_held {
            // A pure throttle cut. The TARGET speed drops; accumulated velocity is
            // left to the kernel's acceleration, which is what makes releasing run
            // a deceleration rather than a snap.
            frame.locomotion.x *= WALK_THROTTLE;
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
    mut bodies: Query<(&mut ActorControl, &ae::BodyKinematics, &WornEquipment), With<PrimaryPlayer>>,
) {
    for (mut control, kin, worn) in &mut bodies {
        if !armed(worn) {
            continue;
        }
        let frame = &mut control.0;
        if !frame.modifier_pressed {
            continue;
        }
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

/// The run technique on the modifier slot, labelled `label`.
///
/// One button, two roles, and the prompt says so. Her rules give the driven
/// body `Run` (a `DrivenTechniques` rule), and the cinder beacon grants
/// `Run / Spark` on the same slot, which replaces it while she wears the beacon
/// (`EquipmentGrant::Technique`). Declaring it as a technique is what puts it
/// in the action scheme, so the physical binding stays configurable and the
/// shared control prompt draws it with no demo-side UI code.
pub(crate) fn run_technique(label: &str) -> ambition_platformer2d::entity_catalog::action_scheme::ActionSpec {
    use ambition_platformer2d::entity_catalog::action_scheme as sch;
    sch::ActionSpec {
        id: sch::ActionId::new("run"),
        slot: sch::ControlSlot::Modifier,
        display_name: Some(label.to_string()),
        visual: None,
        gate: sch::ActionGate::Technique("run".to_string()),
    }
}

#[cfg(test)]
mod tests;
