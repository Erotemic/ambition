// POISON FIXTURE — every banned legacy-runtime identifier, plus an ALLOW line.
//
// Mirrors the synthetic source in the retired legacy_runtime_guardrail.rs
// scanner self-test. Isolated in its own dir so it does not perturb the other
// poison self-tests. Not compiled by cargo (under fixtures/). Do not "fix" it.
pub struct SandboxRuntime;
pub fn touch_runtime() {
    let _ = runtime.player;
}
fn feature_runtime_phase() {}
mod thing {
    struct FeatureRuntime;
}
fn build() {
    let p = ae::Player::new(spawn);
    let snap = BodyClustersMut::to_player(&clusters);
    let scratch = PlayerKinematics::from_player(&p);
    let e = update_player_with_tuning(&world, &mut p, input, dt, tuning);
    let e = update_player_control_with_tuning(&world, &mut p, input, dt, tuning);
    let e = update_player_simulation_with_tuning(&world, &mut p, input, dt, tuning);
    let allowed = SandboxRuntime; // ALLOW_LEGACY_RUNTIME
}
