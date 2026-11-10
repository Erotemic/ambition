# Portal correctness and cost review receipt

The maintainer supplied eight findings from a source review of `0f64b41bf`.
The review did not run gameplay or benchmarks. Keep the canonical material
model and transactional movement authority.

| Finding | Status |
| --- | --- |
| R1: projectile recipient refusals became misses | Fixed in `741212079`. All recipient queries complete before motion or hits commit. Refusal retains the pose, retires travel and advances lifetime. |
| R2: portal scale changed dimensionless knockback | Fixed in `741212079`. `FeelScale` keeps its value; `LaunchSpeed` uses the material chart conversion. |
| R3: material sweeps use root-AABB penetration repair | Open. Prove the stationary partial-body case and use material contact for repair. |
| R4: independent bodies share one capture root and depth | Open. Keep each body's geometric and depth semantics. |
| R5: movement and lookup costs need measurement | Open. Measure portal-free gameplay, body-to-visual lookup and repeated room-link construction. |
| R6: capture density and capacity are fixed | Open. Measure viewport and capture costs before changing resource policy. |
| R7: a failed limb correction can retain the proposed pose | Open. Sweep and admit the proposed pose before one commit. |
| R8: straddler lookup searches other rooms | Open. Resolve the link in the body's own room. |

R1 has a real-app recipient refusal/resume witness. It checks retained pose,
zero travel, no hit, lifetime progress and successful contact after admission.
R2 extends the existing chained-strike witness across root rebasing for both
magnitude types. No new test-only API is required.

Validation on the correction packet: 819 combat tests, 29 projectile-contact
tests, all 15 app projectile traversal tests, 73 Smash tests and 35 workspace
policy tests pass. Three combat probes and six Smash cases remain ignored.
The assembled app check passes. Logs use `target/portal-review-1-2-`,
`target/portal-review-1-projectiles.log` and
`target/portal-review-1-refusal-app.log`.

Do not convert a refused contact query to a miss. Do not apply a length
conversion to a dimensionless combat multiplier. Full PSIM acceptance and the
six open review findings remain open.
