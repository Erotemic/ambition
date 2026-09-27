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

/// Seconds between sparks. Authored here because cadence is character feel.
pub const SPARK_COOLDOWN_S: f32 = 0.35;

/// At most this many of Mary-O's sparks may be alive at once — the classic
/// two-on-screen rule. Authored by the character, enforced by counting HER live
/// shots, so it constrains nobody else's projectiles.
pub const MAX_LIVE_SPARKS: usize = 2;

/// Authoritative spark cadence — sim state, not presentation.
///
/// This gates whether a press FIRES, so two sims that disagree about it are in
/// different states: a rewind that restored input and projectiles but left this
/// at its future value would silently swallow the replayed press and diverge.
/// It is therefore a rollback-registered component on the body.
///
/// Every `PrimaryPlayer` carries it from the moment it is built (a required
/// component, registered by `MaryORulesPlugin`), so no pass adds it later.
///
/// Same lesson as `PipeEntryLatch`: an input-gating latch is authoritative even
/// when it looks like bookkeeping.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct MaryOSparkCooldown {
    /// Counts down between sparks.
    pub remaining: f32,
}

/// Mary-O's answer to "this body starts again".
///
/// The cadence is authoritative sim state that GATES a press, so a body
/// restarted mid-cooldown comes back unable to fire for up to
/// [`SPARK_COOLDOWN_S`] — a fighter who opens a round pressing the button and
/// gets nothing.
///
/// Inert for any body without the component, which is what lets this be
/// registered outside the mode gate.
pub fn clear_spark_cooldown_on_restart(
    restart: On<ambition_platformer2d::engine_core::BodyRestarted>,
    mut cooldowns: Query<&mut MaryOSparkCooldown>,
) {
    if let Ok(mut cooldown) = cooldowns.get_mut(restart.entity) {
        *cooldown = MaryOSparkCooldown::default();
    }
}

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

/// Wind the authoritative spark cadence down.
///
/// Its OWN system rather than a line inside the gait policy: a cadence in the
/// policy's query would make the walk/run throttle silently skip any body that
/// lacks it (Bevy queries drop non-matching entities, with no error and no log).
/// Keeping the cadence separate means neither system can disable the other.
pub fn tick_spark_cooldown(
    time: Res<ambition_platformer2d::time::WorldTime>,
    mut bodies: Query<&mut MaryOSparkCooldown, With<PrimaryPlayer>>,
) {
    for mut spark in &mut bodies {
        spark.remaining = (spark.remaining - time.scaled_dt).max(0.0);
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
pub fn fire_spark_on_run_press(
    mut bodies: Query<
        (
            &mut ActorControl,
            &mut MaryOSparkCooldown,
            &ae::BodyKinematics,
            &WornEquipment,
        ),
        With<PrimaryPlayer>,
    >,
    // Her live sparks: every shot drawn as her spark. The shot is an ordinary
    // shared projectile, and its visual id is the fact that makes it hers.
    shots: Query<&ambition_platformer2d::projectiles::ProjectileVisualId>,
) {
    for (mut control, mut spark, kin, worn) in &mut bodies {
        if !armed(worn) {
            continue;
        }
        let frame = &mut control.0;
        if !frame.modifier_pressed || spark.remaining > 0.0 {
            continue;
        }
        let live_sparks = shots
            .iter()
            .filter(|visual| visual.0 == crate::powerups::SPARK_VISUAL)
            .count();
        if live_sparks >= MAX_LIVE_SPARKS {
            continue;
        }
        spark.remaining = SPARK_COOLDOWN_S;
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
