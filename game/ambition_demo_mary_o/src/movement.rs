//! Mary-O Classic presentation/control glue.
//!
//! The actual physics live in the reusable `AxisSwept` momentum-horizontal and
//! phased-gravity jump laws authored on Mary-O's catalog row. This module owns
//! only her two-gear input grammar — walk by default, run while the modifier is
//! held — plus gait facts and the modifier press-edge used by the cinder beacon.
//!
//! The throttle remains body-local. Acceleration, coasting, skidding, airborne
//! momentum, speed-banded launch, held/released gravity, collision, and rotated
//! gravity frames are all handled by the shared movement kernel.

use bevy::prelude::*;

use ambition_platformer2d::characters::control::ActorControl;
use ambition_platformer2d::characters::equipment::WornEquipment;
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::platformer::frame_env::ResolvedMotionFrame;
use ambition_platformer2d::platformer::markers::PrimaryPlayer;

use crate::powerups::CINDER_BEACON_ID;

/// Walking is 60% of Mary-O's run speed: 180 px/s versus 300 px/s in the
/// initial classic profile. Her catalog row owns the absolute cap; this system
/// owns only the semantic walk/run ratio.
pub const WALK_THROTTLE: f32 = 0.6;

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
///
/// Every field here is DERIVED from this tick's control frame and velocity, so
/// it is rebuilt from scratch each tick and needs no rollback registration. The
/// spark cooldown deliberately does NOT live here — see [`MaryOSparkCooldown`].
#[derive(Component, Debug, Default)]
pub struct MaryOGait {
    /// True while she is running (the slot is sustained) AND actually moving.
    pub running: bool,
    /// True while her input opposes her velocity at speed — the readable slide
    /// that says "she has weight". Drives the skid pose/SFX.
    pub skidding: bool,
}

/// Authoritative spark cadence — sim state, not presentation.
///
/// This gates whether a press FIRES, so two sims that disagree about it are in
/// different states: a rewind that restored input and projectiles but left this
/// at its future value would silently swallow the replayed press and diverge.
/// It therefore lives in its own rollback-registered component rather than
/// riding along on the derived [`MaryOGait`], which must stay unregistered
/// because it is rebuilt every tick.
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
/// gets nothing. [`MaryOGait`] is deliberately not touched: every field of it is
/// re-derived from this tick's control frame, so it has nothing to carry.
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

/// Attach the gait bookkeeping and the authoritative spark cadence to Mary-O's
/// body the first tick it exists.
pub fn ensure_gait(
    mut commands: Commands,
    bodies: Query<Entity, (With<PrimaryPlayer>, Without<MaryOGait>)>,
    uncooled: Query<Entity, (With<PrimaryPlayer>, Without<MaryOSparkCooldown>)>,
) {
    for body in &bodies {
        commands.entity(body).try_insert(MaryOGait::default());
    }
    for body in &uncooled {
        commands
            .entity(body)
            .try_insert(MaryOSparkCooldown::default());
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
    mut bodies: Query<
        (
            &mut ActorControl,
            &mut MaryOGait,
            &ae::BodyKinematics,
            Option<&ResolvedMotionFrame>,
        ),
        With<PrimaryPlayer>,
    >,
) {
    for (mut control, mut gait, kin, resolved_frame) in &mut bodies {
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
        let side_speed = resolved_frame
            .map(|resolved| kin.vel.dot(resolved.get().side()))
            .unwrap_or(kin.vel.x);
        gait.skidding =
            intent.abs() > 0.01 && side_speed * intent < 0.0 && side_speed.abs() > SKID_SPEED;
    }
}

/// Wind the authoritative spark cadence down.
///
/// Its OWN system rather than a line inside the gait policy: the gait policy
/// runs on every body with a `MaryOGait`, and folding an unrelated required
/// component into that query makes the whole walk/run throttle silently skip any
/// body missing it (Bevy queries drop non-matching entities — no error, no log).
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
    live_sparks: Query<&crate::powerups::MaryOSpark>,
) {
    for (mut control, mut spark, kin, worn) in &mut bodies {
        if !worn.wears(CINDER_BEACON_ID) {
            continue;
        }
        let frame = &mut control.0;
        if !frame.modifier_pressed || spark.remaining > 0.0 {
            continue;
        }
        if live_sparks.iter().count() >= MAX_LIVE_SPARKS {
            continue;
        }
        spark.remaining = SPARK_COOLDOWN_S;
        // Primarily along her facing; the shot's own authored gravity supplies the
        // arc, so no launch angle is baked in here.
        frame.fire = Some(
            ambition_platformer2d::characters::actor::control::ActorFireRequest::controlled_body_local(
                ae::Vec2::new(kin.facing.signum(), 0.0),
                0.0,
            ),
        );
    }
}

/// The slot's label follows what it currently does.
///
/// One button, two roles, and the prompt says so: `Run` on its own, `Run / Spark`
/// once the beacon is worn. Declaring it as a technique on the modifier slot is
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
            Option<&mut ambition_platformer2d::characters::action_scheme::ActorTechniques>,
            Option<&WornEquipment>,
        ),
        With<PrimaryPlayer>,
    >,
) {
    for (entity, techniques, worn) in &mut bodies {
        let armed = worn.is_some_and(|w| w.wears(CINDER_BEACON_ID));
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
                    ambition_platformer2d::characters::action_scheme::ActorTechniques(vec![
                        run_technique(label),
                    ]),
                );
            }
        }
    }
}

fn run_slot() -> ambition_platformer2d::entity_catalog::action_scheme::ControlSlot {
    ambition_platformer2d::entity_catalog::action_scheme::ControlSlot::Modifier
}

fn run_technique(label: &str) -> ambition_platformer2d::entity_catalog::action_scheme::ActionSpec {
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
