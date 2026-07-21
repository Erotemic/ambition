//! **Walk by default, run while the modifier slot is held** — Mary-O's own
//! locomotion policy, and the press-edge that fires her spark.
//!
//! The engine carries a sustained control slot into the simulation and attaches no
//! meaning to it. This file is where the meaning is assigned, and it is
//! deliberately tiny: the whole run/walk feel is expressed as a THROTTLE on the
//! body-local locomotion intent, which the shared movement kernel already resolves
//! as `throttle * max_run_speed` with authored acceleration.
//!
//! That one multiply buys every property of the classic feel, because the kernel
//! already has them:
//!
//! - **acceleration, not a velocity jump** — the kernel approaches its target at
//!   `run_accel`; Mary-O authors a low one in her catalog row, so top speed takes
//!   about a third of a second to reach;
//! - **skid on reversal** — reversing points the target at the opposite sign and
//!   the same approach walks velocity through zero, which reads as a slide;
//! - **release does not erase momentum** — letting go of run lowers the TARGET to
//!   walk speed; the body still decelerates into it at `run_accel`;
//! - **airborne momentum is preserved** — the kernel's air branch never brakes
//!   speed already beyond the cap in the held direction, so a running jump keeps
//!   its speed even if run is released mid-flight;
//! - **a running jump goes farther** — free, for the same reason.
//!
//! So there is no Mary-O movement code in the movement path at all. She states a
//! throttle; the ordinary body executes it. This is emphatically NOT the dash
//! impulse — nothing here adds velocity.

use bevy::prelude::*;

use ambition::actors::actor::PrimaryPlayer;
use ambition::characters::brain::ActorControl;
use ambition::characters::equipment::WornEquipment;
use ambition::engine_core as ae;

use crate::powerups::SPARK_BLOSSOM_ID;

/// Walking is half of Mary-O's run speed. Her catalog row authors the run speed
/// itself (`max_run_speed`), so this is the only number the policy owns: the
/// RATIO between her two gaits.
pub const WALK_THROTTLE: f32 = 0.5;

/// Below this speed a reversal is just a turn, not a skid. Presentation-only.
const SKID_SPEED: f32 = 120.0;

/// Seconds between sparks. Authored here because cadence is character feel.
pub const SPARK_COOLDOWN_S: f32 = 0.35;

/// At most this many of Mary-O's sparks may be alive at once — the classic
/// two-on-screen rule. Authored by the character, enforced by counting HER live
/// shots, so it constrains nobody else's projectiles.
pub const MAX_LIVE_SPARKS: usize = 2;

/// Mary-O's gait bookkeeping. Presentation reads it; the movement kernel does not
/// know it exists.
#[derive(Component, Debug, Default)]
pub struct MaryOGait {
    /// True while she is running (the slot is sustained) AND actually moving.
    pub running: bool,
    /// True while her input opposes her velocity at speed — the readable slide
    /// that says "she has weight". Drives the skid pose/SFX.
    pub skidding: bool,
    /// Counts down between sparks.
    pub spark_cooldown: f32,
}

/// Attach the gait bookkeeping to Mary-O's body the first tick it exists.
pub fn ensure_gait(
    mut commands: Commands,
    bodies: Query<Entity, (With<PrimaryPlayer>, Without<MaryOGait>)>,
) {
    for body in &bodies {
        commands.entity(body).try_insert(MaryOGait::default());
    }
}

/// **The policy.** Scale the body-local locomotion throttle down to a walk unless
/// the modifier slot is sustained.
///
/// Runs after the brain has produced this tick's `ActorControl` and before the
/// shared movement phase consumes it, so the scaled throttle flows through the
/// ENTIRE ordinary path — brain intent, `InputState`, the movement kernel,
/// replay, and rollback — rather than being applied at a device adapter where the
/// simulation could never see the difference between a walk and a half-pushed
/// stick.
pub fn walk_by_default_run_while_held(
    time: Res<ambition::time::WorldTime>,
    mut bodies: Query<
        (&mut ActorControl, &mut MaryOGait, &ae::BodyKinematics),
        With<PrimaryPlayer>,
    >,
) {
    for (mut control, mut gait, kin) in &mut bodies {
        let frame = &mut control.0;
        let running = frame.modifier_held;
        if !running {
            // A pure throttle cut. The TARGET speed drops; accumulated velocity is
            // left to the kernel's acceleration, which is what makes releasing run
            // a deceleration rather than a snap.
            frame.locomotion.x *= WALK_THROTTLE;
        }

        let intent = frame.locomotion.x;
        gait.running = running && intent.abs() > 0.01;
        gait.skidding =
            intent.abs() > 0.01 && kin.vel.x * intent < 0.0 && kin.vel.x.abs() > SKID_SPEED;
        gait.spark_cooldown = (gait.spark_cooldown - time.scaled_dt).max(0.0);
    }
}

/// **The same button's press edge fires a spark**, while its held level keeps
/// meaning run.
///
/// This is the dual-purpose half of the classic grammar, and it only works because
/// the slot's edge and level both survive into the simulation. Firing is a press —
/// there is no charge and no release edge to wait for.
///
/// It does not spawn anything. It raises the body's ordinary `fire` intent, which
/// the shared moveset picks up as the `"ranged"` verb; the projectile the blossom
/// granted is what actually launches, through the one shared projectile path.
pub fn fire_spark_on_run_press(
    mut bodies: Query<
        (
            &mut ActorControl,
            &mut MaryOGait,
            &ae::BodyKinematics,
            &WornEquipment,
        ),
        With<PrimaryPlayer>,
    >,
    live_sparks: Query<&crate::powerups::MaryOSpark>,
) {
    for (mut control, mut gait, kin, worn) in &mut bodies {
        if !worn.wears(SPARK_BLOSSOM_ID) {
            continue;
        }
        let frame = &mut control.0;
        if !frame.modifier_pressed || gait.spark_cooldown > 0.0 {
            continue;
        }
        if live_sparks.iter().count() >= MAX_LIVE_SPARKS {
            continue;
        }
        gait.spark_cooldown = SPARK_COOLDOWN_S;
        // Primarily along her facing; the shot's own authored gravity supplies the
        // arc, so no launch angle is baked in here.
        frame.fire = Some(
            ambition::characters::actor::control::ActorFireRequest::controlled_body_local(
                ae::Vec2::new(kin.facing.signum(), 0.0),
                0.0,
            ),
        );
    }
}

/// **The slot's label follows what it currently does.**
///
/// One button, two roles, and the prompt says so: `Run` on its own, `Run / Spark`
/// once the blossom is worn. Declaring it as a technique on the modifier slot is
/// what puts it in the action scheme at all, so the physical binding stays
/// configurable and the existing control-prompt machinery renders it with no
/// demo-side UI code — and no raw key check anywhere in the demo.
///
/// Upserts by SLOT rather than replacing the list, so a future Mary-O technique on
/// another slot is not collateral damage.
pub fn sync_run_action_scheme(
    mut commands: Commands,
    mut bodies: Query<
        (
            Entity,
            Option<&mut ambition::characters::action_scheme::ActorTechniques>,
            Option<&WornEquipment>,
        ),
        With<PrimaryPlayer>,
    >,
) {
    for (entity, techniques, worn) in &mut bodies {
        let armed = worn.is_some_and(|w| w.wears(SPARK_BLOSSOM_ID));
        let label = if armed { "Run / Spark" } else { "Run" };
        match techniques {
            Some(mut techniques) => {
                let current = techniques
                    .0
                    .iter()
                    .find(|a| a.slot == run_slot())
                    .and_then(|a| a.display_name.as_deref());
                if current == Some(label) {
                    continue;
                }
                techniques.0.retain(|a| a.slot != run_slot());
                techniques.0.push(run_technique(label));
            }
            None => {
                commands.entity(entity).try_insert(
                    ambition::characters::action_scheme::ActorTechniques(vec![run_technique(
                        label,
                    )]),
                );
            }
        }
    }
}

fn run_slot() -> ambition::entity_catalog::action_scheme::ControlSlot {
    ambition::entity_catalog::action_scheme::ControlSlot::Modifier
}

fn run_technique(label: &str) -> ambition::entity_catalog::action_scheme::ActionSpec {
    use ambition::entity_catalog::action_scheme as sch;
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
