//! Every per-attempt resource a shipped demo holds is ON the retraction slot.
//!
//! ⛔⛔ THE GUARD THAT WAS HERE READ THE WRONG HALF. `scripts/per_attempt_resource_census.py`
//! certifies that the three known per-attempt resources "retract through `AttemptScoped`"
//! by finding `impl AttemptScoped for T` — the DECLARATION. What retracts them is a
//! registration in [`ContentRoomReplayResetSet`] — the MECHANISM. MEASURED 2026-09-07:
//! renaming `rearm_attempt_scoped` to `POISONED_rearm_attempt_scoped` at all three
//! production registrations left the census printing *"ok: all 3 known per-attempt
//! resources retract through `AttemptScoped`"* and exiting 0. A trait impl is a claim
//! about intent; only a registration is evidence of behaviour.
//!
//! ⭐ SO THIS ASKS THE SCHEDULE, not the source. It builds each demo's rules plugin,
//! initializes the schedule, and reads the members `ContentRoomReplayResetSet` actually
//! holds. That survives the failure a call-site grep cannot see — an installer that is
//! called and registers nothing.
//!
//! ⚠ It is a COMPOSITION test, not a behaviour test. It says the re-arm is on the slot;
//! `ambition_demo_mary_o_app/tests/room_replay.rs` is where a pit death proves the slot
//! runs. `SpentMonitors` — the resource whose shipped bug motivated the trait — still has
//! no behavioural counterpart.

use bevy::ecs::schedule::{ScheduleLabel, Schedules};
use bevy::prelude::*;

/// What Bevy renders instead of a system name when `bevy_utils/debug` is OFF.
///
/// ⛔⛔ AND IT IS ON HERE ONLY BECAUSE OF AN INSPECTOR GUI. MEASURED 2026-09-07
/// (`cargo tree -e features -i bevy_utils@0.19.1`): 65 manifests in this workspace
/// pin `bevy = { default-features = false, .. }`, NONE list `"debug"`, and the one
/// dependent that turns it on is `bevy_egui 0.40.1` — reached through the OPTIONAL
/// `bevy-inspector-egui`, behind the `dev_tools` feature, which `ambition_app`'s
/// `default = ["desktop_dev"]` happens to include.
///
/// ⚠ THE FEATURE SET THAT LOSES IT ALREADY EXISTS. `android = [.., "rl_sim", ..]`
/// carries the sim harness WITHOUT `dev_tools` (`android_dev` is the one that adds
/// it), so a build under that persona renders every system as the placeholder. The
/// gate's feature jobs deny `android`, so no lane hits it today — which is exactly
/// the kind of "true until someone trims features" edge that goes unrecorded.
///
/// ⇒ So this guard states its precondition instead of assuming it. Without the
/// check below, a build with names stripped would fail saying THE RE-ARM IS
/// MISSING, and send its reader to fix a schedule that was never broken. A guard
/// whose failure message names the wrong cause is worse than a count.
const NAMES_STRIPPED: &str = "<Enable the debug feature to see the name>";

/// The names of the systems `ContentRoomReplayResetSet` holds in `label`.
///
/// The set is asked by IDENTITY, so a renamed set is a `SetNotFound` here rather
/// than a silently empty list.
fn retraction_slot_members(app: &mut App, label: impl ScheduleLabel) -> Vec<String> {
    let label = label.intern();
    let slot = ambition_platformer2d::actors::session::reset::ContentRoomReplayResetSet.intern();
    app.world_mut()
        .resource_scope(|world, mut schedules: Mut<Schedules>| {
            let schedule = schedules.get_mut(label).expect("the schedule exists");
            // The graph is built lazily; a schedule that has never run has no
            // structure to read.
            let _ = schedule.initialize(world);
            let by_key: std::collections::HashMap<_, _> = schedule
                .systems()
                .expect("initialized just above")
                .map(|(key, system)| (key, format!("{}", system.name())))
                .collect();
            schedule
                .graph()
                .systems_in_set(slot)
                .expect("the demo anchors ContentRoomReplayResetSet in this schedule")
                .iter()
                .map(|key| by_key[key].clone())
                .collect()
        })
}

/// Assert one type's re-arm is on the slot, by NAME, and say what is there when it
/// is not — a diagnostic that names the neighbours is the difference between "this
/// demo lost its retraction" and "this demo has none at all".
fn assert_rearms(members: &[String], type_name: &str, demo: &str) {
    assert!(
        !members.iter().any(|name| name.contains(NAMES_STRIPPED)),
        "{demo}: THIS BUILD RENDERS NO SYSTEM NAMES, so nothing below can identify \
         `{type_name}` and the failure you would otherwise read is not the one you \
         have. Names come from `bevy_utils/debug`, supplied ONLY by `bevy_egui` via \
         the optional `bevy-inspector-egui` behind the `dev_tools` feature. Someone \
         trimmed that feature out of this build. Members: {members:#?}"
    );
    let hit = members
        .iter()
        .any(|m| m.contains("rearm_attempt_scoped") && m.contains(type_name));
    assert!(
        hit,
        "{demo} holds `{type_name}` as per-attempt state but its re-arm is not in \
         `ContentRoomReplayResetSet`. A resource with the impl and without the \
         registration is the shipped Sanic bug: the state exists and nothing takes it \
         back. The slot currently holds: {members:#?}"
    );
}

/// Mary-O's two block ledgers, on BOTH roads the plugin ships.
///
/// ⚠ `hosted()` and `global()` are two `add_systems` branches, so one can lose the
/// retraction while the other keeps it. The launcher takes the first, the standalone
/// demo binary takes the second, and a player meets both.
#[test]
fn mary_o_puts_both_of_its_block_ledgers_on_the_retraction_slot() {
    for plugin in [
        ambition_demo_mary_o::MaryORulesPlugin::hosted(),
        ambition_demo_mary_o::MaryORulesPlugin::global(),
    ] {
        let mut app = App::new();
        app.add_plugins(bevy::MinimalPlugins);
        app.add_plugins(plugin);

        let members = retraction_slot_members(&mut app, Update);
        assert_rearms(&members, "BrokenBricks", "Mary-O");
        assert_rearms(&members, "SpentPowerBlocks", "Mary-O");

        // ⚠ ANTI-VACUITY: the installer owns the resource too, so a re-arm on the slot
        // with no resource behind it would be a re-arm that can never run.
        assert!(
            app.world()
                .get_resource::<ambition_demo_mary_o::bricks::BrokenBricks>()
                .is_some()
                && app
                    .world()
                    .get_resource::<ambition_demo_mary_o::powerups::SpentPowerBlocks>()
                    .is_some(),
            "`install_attempt_scoped` puts the resource in the world in the same statement"
        );
    }
}

/// Sanic's monitor ledger — the resource whose player-visible bug named this class.
#[test]
fn sanic_puts_its_monitor_ledger_on_the_retraction_slot() {
    for plugin in [
        ambition_demo_sanic::SanicRulesPlugin::hosted(),
        ambition_demo_sanic::SanicRulesPlugin::global(),
    ] {
        let mut app = App::new();
        app.add_plugins(bevy::MinimalPlugins);
        app.add_plugins(plugin);

        let members = retraction_slot_members(&mut app, Update);
        assert_rearms(&members, "SpentMonitors", "Sanic");
        assert!(
            app.world()
                .get_resource::<ambition_demo_sanic::monitors::SpentMonitors>()
                .is_some(),
            "`install_attempt_scoped` puts the resource in the world in the same statement"
        );
    }
}
