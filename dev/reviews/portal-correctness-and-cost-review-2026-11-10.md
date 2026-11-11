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
| R5: movement and lookup costs need measurement | Open. Optimized historical and current simulation samples are measured. The current Hall costs about twice the historical snapshot. Source-cell reuse is fixed in `a87a5936e`; displayed-host and device costs remain open. Body lookup and room-link reuse are fixed in `ae8f2b07a`. |
| R6: capture density and capacity are fixed | Fixed in `1d551594b`. Density follows the viewport; a byte budget reduces resolution and then selects material proxies. Device performance remains unmeasured. |
| R7: a failed limb correction can retain the proposed pose | Fixed in `482dcfc5b`; prefix admission is fixed in `670e4683c`. Sweep and admit the proposal against physical room solids before one commit. Refusal retains the accepted pose and hold. |
| R8: straddler lookup searches other rooms | Fixed in `b499e5090`. Publication resolves the link in the owning room. |

The [R5 cost receipt](../ambition_dev_measurements/summaries/portal-native-motion-cost-20261111.md)
records commands, source and input hashes, census, samples and limits.

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

After the upstream day-clock save merge, the assembled app check and all four
schema guards pass with schema v443. Evidence is
`target/portal-r4-r8-merged-gates.json` and its check and schema logs.

R6 fix: `1d551594b`. Each family keeps its full world bounds while target
resolution follows viewport density. Retained RGBA targets share a 32 MiB
default budget. Targets retire when idle, including hidden material bindings.
All admitted families render; there is no 16-family cutoff. Storage pressure
first reduces pixel resolution, then uses clipped canonical material proxies.
The proxy retains physical scale and reflection, with less visual detail.
Do not reduce geometry admission or omit a family to meet a texture budget.

The guards cover viewport and zoom changes, idle release, twenty independent
families, zero target bytes and persistent reflected material. Validation passes
985 owner cases, 35 portal app cases, 35 policy cases and seven capture-tool
cases. The assembled app check and capture-tool build pass. Evidence is
`target/portal-r6-gates.json`. The standalone debug-drawing fixture now creates
its required geometry observation resource in `ab5de0ba4`.

The before/after llvmpipe films each have 80 frames. At 1280 by 720, peak
retained target storage falls from 64 MiB to 2.3125 MiB. At 540 by 960 it falls
from 16 MiB to 0.578125 MiB. Idle target storage falls from 32 and 8 MiB to
zero. A zero-byte run renders material proxies with no capture targets.
Matched-tick player and item states are equal: 73 ticks before/after, 78 ticks
across viewport sizes and 78 ticks with proxies. Partial and cleared crossings
were inspected. Evidence is `target/portal-r6-before/resources.json`,
`target/portal-r6-after/resources.json` and
`target/portal-r6-after/physics-comparison.json`. These are software-renderer
resource measurements, not hardware GPU, Android or frame-time measurements.
R5 and full PSIM acceptance remain open.

After upstream room-rollback merge `dfdfa5f3b`, all four schema guards pass
with v445. The app, portal presentation and renderer all-target warning check
passes under default features. Evidence is `target/portal-r6-merged-gates.json`
and its warning and schema logs. Non-default feature-only code is not covered.

R5 current-engine measurement: three unprofiled four-fighter runs reach exactly
1,800 ticks with 100% sampled live-cast coverage. Mirror runs average 15.01
and 15.32 ms/tick; mixed-v1 averages 16.57 ms/tick. A warmed native sample
finds 4.40% inclusive user-CPU samples in body movement and 2.14% in
`MaterialCast::cells`. These shares overlap and do not measure wall-time cost.
The development build uses the assembled windowless host with GGRS. It does
not measure displayed frames. Do not introduce a separate AABB solver from
these numbers. The historical control, optimized/displayed-host comparison
and lookup scaling measurements remain open. Commands, machine, limits and
evidence are in the
[current cost receipt](../ambition_dev_measurements/summaries/portal-free-material-cost-20261110.md).

The station I acceptance film found a held-picture root defect.
`04a1625b1` supplies the holder root and ownership on both held-picture paths.
The common carry keeps the owner's material budget. A source-wall tip now
clips at the entry and appears at the exit. The owned-reach guard checks both
mesh bounds and a refused hop limit. The two-reflection films pass at zero
and ninety degrees with equal physics and held transforms at 138 common
ticks. The [accessory receipt](../ambition_dev_measurements/summaries/portal-held-accessory-20261111.md)
records commands, 791 active owner passes, 35 policy passes and the remaining
physical attachment and full PSIM requirements.


The GNU-ton render witness is fixed in `9d6f9da4f`. Each generated limb
picture uses its own physical root. Every animated root in a portal room
receives image demand. Cached images wait for their own source images and
GPU passes before they rest. The staged film retains the giant and both
independent fist paths. Owner tests and merged native contact/replay checks
pass. The [render receipt](../ambition_dev_measurements/summaries/portal-gnu-limb-render-20261111.md)
records the command, guards and GPU target descriptors. Device performance
and full PSIM acceptance remain open.


`a91231348` makes projectile damage use admitted source cells against sampled
hurt material. `4825f1f1c` keeps separate physical bounds and prepares launch
units before commit. An invalid recipient between the two ends is excluded;
an invalid ally beside the hostile recipient is not queried. `LaunchSpeed`
follows the source-to-victim chart and `FeelScale` stays unchanged. The real
actor loses one HP before root transit at both 80/40 ratios. The spinning-exit
contact, owner, slice, policy, app and default warning checks pass. The
[contact receipt](../ambition_dev_measurements/summaries/portal-carried-projectile-contact-20261111.md)
records exact commands and validation stages. Moving targets, surviving
responses, general continuation and full PSIM acceptance remain open.

`d795c9268` prepares carried parries before movement, guard rewards or ownership
writes. The source root stops at the contact prefix; velocity uses the existing
material law at the physical witness. Reflection and absorption pass through
both H ratios. Translation, spin and response overflow pass in the core guard.
After audio merge `fec049833`, owner and app cases, the app check and default
warning checks pass. The
[interception receipt](../ambition_dev_measurements/summaries/portal-carried-projectile-interception-20261111.md)
records validation scope. Moving targets, other responses, general continuation
and full PSIM acceptance remain open.


`10f8eaf95` admits a surviving world rebound over all occupied material.
Geometry and response retain separate typed results. A reached response refusal
prevents movement and combat writes. An earlier terminal target outcome retires
the later response. Both H directions bounce before root transit and clear the
contact next tick. Owner and app checks pass after merge `eff565e6d`. The
[world response receipt](../ambition_dev_measurements/summaries/portal-world-response-admission-20261111.md)
records the full verification scope. Full PSIM acceptance remains open.


Timing merge `a0568d91b` passes 434 owner, 38 runtime projectile, 19 projectile
app, both H slice and both room-freeze cases. The slice includes reverse travel
and rewind/replay. The assembled app and default warning checks pass. These
results are recorded in the world response receipt; full PSIM remains open.


`cad13e2eb` continues projectile motion after an accepted root crossing.
One clock and one ordinary contact budget cover all legs. The shared portal
commit maps persistent material, velocity and acceleration. H in both
directions, L's moving exit, J's three roots and hop refusal pass. Owner,
runtime, app, reverse/replay and warning checks pass after merge `b0e5bd404`.
The [continuation receipt](../ambition_dev_measurements/summaries/portal-projectile-root-continuation-20261111.md)
records scope. General movement continuation and full PSIM acceptance stay open.


`065676df3` keeps authored guard regions in canonical body coordinates.
Exact material supplies contact, portal cuts and front/top coverage; world
bounds supply only the broad phase. Typed refusals prevent recipient writes.
Both aperture ratios and root parities pass in the local guard check. The
real Warden publisher and resolver pass for reflected half scale and turned
double scale. Combat, Warden, projectile, reverse/replay, Smash, app compile
and warning checks pass. The
[guard receipt](../ambition_dev_measurements/summaries/portal-guard-material-20261111.md)
records scope. Deflect motion response, autonomous facing, physical attachment
release and full PSIM acceptance remain open.


`e5a0e4ac1` fixes the reflected-facing error exposed by the guard witness.
Brain snapshots and kernel requests use explicit control/material projection.
The actor write after the kernel is removed, and Axis requests use its turn
rules. The real Warden case keeps its brain active. Reflected crawler control
and travel share the route parity map; the existing curved cast uses a tighter
signed-plane envelope for its angled-chain contact. Typed refusals remain.
Owner, app and replay checks pass after room-state merge `09f0afbc4`. The
[facing receipt](../ambition_dev_measurements/summaries/portal-material-facing-20261111.md)
records the broad source checks, merged integration checks and focused Sanic
loop result. Full workspace, device, performance and PSIM acceptance remain open.


`16d3d2ff6` carries the meeting strike and guard charts into deflect response.
Guard speed units follow canonical material; response is relative to the
guarded body and returns through each path's velocity map. Both launches are
prepared before state changes. Owner, Warden, reverse/replay, Smash, policy
and warnings pass. App, Warden and schemas pass after merge `10b07aef7`.
The [guard receipt](../ambition_dev_measurements/summaries/portal-guard-material-20261111.md)
keeps the midpoint position estimate, swept-contact work and broader open
scope explicit.


`a66c9209d` adds an admitted item release with a canonical pickup material
basis. A refusal preserves the object and cannot mint a replacement. Menu
and participant changes prepare their releases first. `03a9efeaf` keeps the
shared-load anchor. The real H javelin passes scaled release and rewind/replay.
Combined item/cutscene schema 458 passes app, schema, javelin and absence
checks in `dbc086852`. The [release receipt](../ambition_dev_measurements/summaries/portal-item-release-20261111.md)
records each batch and its limits. Hand/wrist and held render geometry,
free-item motion/collision, durable material history and full PSIM acceptance
remain open.
