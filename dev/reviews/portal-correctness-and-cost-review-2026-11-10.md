# Portal correctness and cost review receipt

The maintainer supplied eight findings from a source review of `0f64b41bf`.
The review did not run gameplay or benchmarks. Keep the canonical material
model and transactional movement authority.

| Finding | Status |
| --- | --- |
| R1: projectile recipient refusals became misses | Fixed in `741212079`; candidate scope is fixed in `670e4683c`. All recipient queries complete before motion or hits commit. Refusal retains the pose, retires travel and advances lifetime. |
| R2: portal scale changed dimensionless knockback | Fixed in `741212079`. `FeelScale` keeps its value; `LaunchSpeed` uses the material chart conversion. |
| R3: material sweeps use root-AABB penetration repair | Fixed in `5f07fbb17`. Movement, rest repair, initial support, line settling and platform support use admitted material. |
| R4: independent bodies share one capture root and depth | Fixed in `ae8f2b07a`. Each drawable family keeps its root, capture and depth order. |
| R5: movement and lookup costs need measurement | Open. Measure portal-free gameplay, body-to-visual lookup and repeated room-link construction. |
| R6: capture density and capacity are fixed | Open. Measure viewport and capture costs before changing resource policy. |
| R7: a failed limb correction can retain the proposed pose | Fixed in `482dcfc5b`; prefix admission is fixed in `670e4683c`. Sweep and admit the proposal against physical room solids before one commit. Refusal retains the accepted pose and hold. |
| R8: straddler lookup searches other rooms | Fixed in `b499e5090`. Publication resolves the link in the owning room. |

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
remaining review findings remain open.

R7 replaces the runtime correction pass with prepared translation admission.
The core guard checks near-side, half-scale, double-scale and chained stops.
The pose-owner guard checks a refused hand beside an accepted hand, including
history at tick end. The encounter guard checks root ownership, traversal and
far-side damage when contact stops a diagonal proposal before its final axis.

R7 validation: 1,892 owner tests and all 42 active GNU-ton cases pass across
the full run and the corrected punch rerun. One GNU-ton measurement probe is
ignored. The assembled app check, 73 active Smash cases and all 35 workspace
policy cases pass. Six Smash probes are ignored. The policy run required the
GOBS dependency declaration in `0a516ddc3`; the failed engine case then passes.
Evidence uses `target/portal-review-7-*.log`. This is a focused lane. It does
not prove the full workspace, device, GPU or performance requirements.

After the NPC planning merges, the assembled app check and all 35 workspace
policy cases pass on `1deda4d6c`. The minimal-game lockfile and footprint
baseline now include the two upstream vocabulary packages. All 53 absence
contracts pass. The NPC planning contract remains an unrequested dependency
in that baseline. Evidence uses `target/portal-review-7-published-*.log`.

P1/P2 follow-up: `670e4683c`. The shared hurt sweep checks source bounds and
possible portal entry and exit regions before recipient partition admission.
Unknown bounds keep the recipient as a candidate. Primitive and projected
bounds round out to retain thin contacts after a large chart offset cancels.
The app guard keeps a remote invalid recipient present while a nearby
recipient refuses, then accepts contact after its own budget is repaired.

External translation checks fields and history before the sweep. If the full
cast refuses, it searches at most 128 prefixes for an admitted solid contact.
Only the accepted prefix and its carried values pass final admission. If no
contact is certified, the operation keeps its refusal and commits nothing.
The guard checks an early wall stop, the refused path with no wall, and a
refused carried value. Do not require an unreachable suffix to pass admission.

The packet passes 874 core, 246 boss, 29 projectile-contact, 73 Smash and
35 policy tests. The final bounds correction passes all 821 combat tests,
all 15 projectile app tests and the assembled app check. After the upstream
merge, the app check, all 15 projectile cases and three GNU-ton punch cases
pass again. The punch cases include rewind. Four owner probes and six Smash
probes are ignored. Evidence uses `target/portal-p1-p2-*.log` and the three
gate files: `target/portal-p1-p2-gates.json`,
`target/portal-p1-p2-final-gates.json` and
`target/portal-p1-p2-merged-gates.json`. This lane does not cover the full
workspace, GPU, device or performance requirements.

R3 fix: `5f07fbb17`. Axis movement and stationary repair use the same material
cells and physical room solids. Line settling and platform support keep that
geometry. Physical cut constraints reject empty host regions before canonical
projection. A source that fits an opening retains its front spatial cell before
first touch. Keep a source crossing admitted until the movement transaction
ends and transit rebases it.

The guards check stationary far-side support and repair, initial contact,
ground-line openings at rest and during transit, source-chart retention,
canonical floor rounding and rotated host boundaries. The real-app slice
also pauses a partial body before both crossings and deterministic rewind.
Do not restore a root-AABB repair or support-placement pass after these queries.

Validation passes 878 core, 595 actor, 178 world, 35 portal app, 73 Smash and
35 policy tests, plus the assembled app check. One core probe and six Smash
probes are ignored. Evidence is `target/portal-r3-acceptance-gates.json` and
`target/portal-r3-acceptance-*.log`. This lane does not prove the full workspace,
other movement models, device/GPU or performance requirements. R5 and R6
remain open. Full PSIM acceptance remains open.

After upstream merge `a40680fd4`, both 80/40 slice cases and both chained-lab
cases pass again. The app and core all-target warning check passes under
their default features. Evidence is `target/portal-r3-merged-gates.json` and
`target/portal-r3-merged-*.log`. Non-default feature-only code is not covered
by that warning check.

R4 fix: `ae8f2b07a`. Independent bodies and free drawables no longer merge
into one capture. Explicit drawable families still stay together. Each family
keeps its own root and source depth. Exit pieces retain source depth order
inside the reserved band below the panes. The guard compares two bodies with
the same policy and link against their isolated geometry in both spawn orders.
Do not share a blended capture by collapsing independent roots or depths.

R8 fix: `b499e5090`. Straddler publication passes the owning room. The guard
uses identical apertures in two rooms with different link keys and requires
the owning room's link. Do not search all rooms by aperture geometry.

All 1,483 active owner cases, 35 portal app cases and 35 policy cases pass.
Four owner probes are ignored. The assembled app check and capture tools build
pass. Evidence uses `target/portal-r4-r8-*.log` and
`target/portal-r4-r8-gates.json`.

The Vulkan llvmpipe film at `target/portal-r4-visible/close/` shows a robot and
an independent javelin partly in the same 80/40 pair. Frames 47–49 have the same
carried path with separate capture layers and targets. Frame 64 shows the robot
clear at persistent half scale and reflected parity. The witness is
`target/portal-r4-visible/witness.json`. This is software render-stack evidence,
not hardware GPU or mobile performance evidence.

`ae8f2b07a` also indexes feature bodies by room and identity, and reuses each
room's links within the carry pass. R5 still needs portal-free before/after
measurements. R6 still needs viewport adaptation, resource limits and an
explicit overflow policy. More independent captures make that cost important.

The app, portal presentation and host all-target warning check passes under
their default features. Evidence is `target/portal-r4-r8-warnings.log`.
