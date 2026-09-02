//! The developer brain override reaches the simulation as a VALUE.
//!
//! ⛔⛔ AN APP-LEVEL GUARD BECAUSE REGISTRATION IS THE THING BEING PINNED, and
//! `ambition_dev_tools` cannot prove its own: that crate's `DevToolsSimPlugin`
//! has siblings needing resources it does not depend on, so a bare `App::new()`
//! cannot run its schedule — the same reason
//! `the_developer_hud_flash_still_winds_down` lives here.

/// ⭐⭐ THE DEV TOOL WRITES AND THE SIM READS, which is D33's stated inversion.
///
/// Until 2026-09-02 the actor kernel called
/// `ambition_dev_tools::brain_override::forced_preset()` and `forced_profile()`
/// from inside `resolve_npc_brain` — the simulation reaching UP into a developer
/// crate, mid-brain-construction, to decide what the world contains. The value
/// is a session resource now. This proves the publishing half: without it the
/// resource is simply absent, every road reads `None`, and the knob is dead
/// while every other test still passes.
///
/// ⛔ IT ASSERTS THE RESOURCE EXISTS AND IS QUIET, not that it is forced. The
/// suite runs with no `AMBITION_ACTOR_BRAIN_*` set, so "published, and saying
/// nobody is steering" is the whole of what this run can honestly claim — and
/// it is exactly what the wiring guarantees that absence would not.
#[test]
fn the_developer_plugin_publishes_a_brain_override_for_the_simulation_to_read() {
    let app =
        ambition_app::app::build_visible_app(ambition_app::app::VisibleRenderMode::NoWindow, true);
    let forced = app
        .world()
        .get_resource::<ambition_platformer2d::characters::brain::AuthoredBrainOverride>()
        .expect(
            "no `AuthoredBrainOverride` resource: `DevToolsSimPlugin` did not publish one, so \
             every construction road reads `None` and AMBITION_ACTOR_BRAIN_OVERRIDE / \
             AMBITION_ACTOR_BRAIN_PROFILE steer nothing at all",
        );
    assert_eq!(
        forced,
        &ambition_platformer2d::characters::brain::AuthoredBrainOverride::default(),
        "this suite runs with neither environment variable set, so the published \
         value must be `the author decides` — a non-default here means the cast \
         every other test measures is not the authored one"
    );
}
