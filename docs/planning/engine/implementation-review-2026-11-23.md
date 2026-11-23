# Implementation review: preserve decisions across native boundaries

**Source:** `1bef25200e002f01b6afdbc7033e3f00b1ec0b72`, compared with
`6d100f9d9e0d557fe4ae3dad61acd5aef71f684e`. The supplied snapshot contains
137 intervening commits, including the preceding planning overlays.
**Method:** static source, schedule, authored-data, fixture and evidence
review. No build, engine, runtime test, audio device or benchmark was run.
**Use:** implementation guidance for the existing owners below. The
[queue](../queue.md) keeps its current product order.

## Assessment and next work

The agents made substantial native changes. Keep them. The remaining
corrections are at boundaries where the selected meaning is only partly
carried to the next consumer. A shared predicate can still answer for the
wrong weapon; a retained value can still belong to the wrong execution.

| Owner | Correction | Why the existing change is incomplete |
| --- | --- | --- |
| [ACT1](body-action-execution.md#act1-follow-through--resolve-the-request-before-spending) | Resolve the complete native attempt; stamp the bounded facts needed by a later shot | The drawn source is retained after another source supplied admission/spend; fallback cancellation uses the wrong identity; a final event loses use-owned facts |
| [IN2](participant-action-system.md#in2-follow-through--retain-direction-meaning-through-the-guard) | Retain selected aim source and project only feasible travel | Equal vectors do not establish a shared input source; posture Y can leak into side travel |
| [OBS3](agentic-character-runtime.md#obs3-follow-through--publish-availability-for-this-decision) | Separate stable demand from availability of the current decision product | A skipped native decision leaves a prior projection that a module treats as current |
| [MUS2/MUS3](music-programs-and-playback.md#a-continuation-use-is-distinct-from-its-scheduling-deadline) | Distinguish a new continuation use from its optional native deadline | Closed by `2fb366d1e`: same-product continuation retains its use across host stalls; live mixes retain playback |

These are source-backed corrections, not measured gameplay regressions.
Several witnesses use supported compositions rather than a known shipped
scene. The distinctions below identify which is which.

## Progress to preserve

- Ranged events now retain the selected source and occurrence through a
  windup equipment change. `OwnMoveFact` carries the actual playing spec.
- Accepted reactions carry the interrupted victim occurrence. A later result
  cannot cancel a replacement use. Prepared hit statuses now reach the native
  receiver instead of independent post-contact forwarding callbacks.
- ACT2 holds move and physical-pose progression with local hitlag and removes
  the primary-body shortcut from the world clock. Its partial-expiry, sustain
  and floor-time residuals are already stated; they are not new findings.
- IN3 selects a qualified press as a whole. IN2 retains the accepted portal
  map rather than rereading a later convention.
- OBS1 admits the attended set to tactical memory. OBS2 keeps observed
  liveness, evidence source/cut and hit hostility distinct. OBS3 provides an
  explicit sighted module port and a union of native and module demand.
- Summon completion now comes from the construction receipt, and mount
  admission shares a native pair/scope law. Keep the separate transfer work.
- Music has bounded native groups, exact handle retirement, prepared media,
  scheduled continuations/exits, retained short-completion facts, room
  preparation, demand-based media retention and producer-owned score inventory.
  The old bank-order diagnosis is retired.

The source and receipts for these changes are in the respective owner plans.
They should not be replaced by another general action, relationship or music
framework in response to the findings below.

## Combat — resolve the attempt and retain the emitted use

### The retained weapon did not supply its own admission

[`trigger_moveset_moves` and `start_move`](../../../crates/ambition_combat/src/moveset/mod.rs)
now disagree about when source selection happens. The trigger computes
`weapon_has_room` and `refire_s` from the body's `ActionSet.ranged` before
it selects the candidate. Only after admission does `start_move` resolve
the candidate's prospective `equips` source. Playback retains that source,
but the spend still arms the default body's interval.

The shipped Pirate Admiral exposes the distinction. Its
[default pistol](../../../game/ambition_content/assets/data/character_catalog.ron)
uses the 1.1-second default. Its
[side-special](../../../game/ambition_content/assets/data/movesets/pirate_admiral.ron)
draws `admiral_gun_sword`, whose
[source definition](../../../crates/ambition_characters/src/brain/action_set/mod.rs)
explicitly uses 0.0 seconds because the move's recovery owns its cadence.
The current start retains the drawn shot while spending the pistol interval.
This is a code/configuration disagreement; no play symptom was measured here.

A supported limit witness needs no equipment change during windup: give the
body a limited default source A with its shot already live, then request a
move that draws unlimited B. The default-source Boolean refuses B. Reversing
the limits can admit B under A's permission. The primary and fallback moves
can also select different drawn sources.

**Correction:** resolve each candidate's actual source before its gates.
Use that selection for live-shot admission, spend, forecast frame data and
playback. Keep the shared body refire state and its current policy; changing
the selected source does not require a new cooldown inventory. Preserve live
custody and physical-birth checks at their own later boundaries.

### A request's cancel name and its fallback's execution name differ

Production checks the active move's cancel window against requested move X
and its verb names, then selects an affordable `when_refused` Y. The new
[`attack_kit_of`](../../../crates/ambition_platformer2d_actor_monolith/src/body/actors/update.rs)
selects Y first and asks `legality_of` about Y. Shared affordability does not
make these contextual decisions equal.

A window permitting only X, with unaffordable X and affordable Y, executes Y
but offers `BlockedByPlayback`. A window permitting only Y can offer `Now`
for the X request that execution refuses. Exact-ID cancel entries and this
one-hop fallback are supported; this is not a claim about a measured match.

The native resolution result must keep three facts:

| Fact | Consumers |
| --- | --- |
| Requested binding and cancel namespace | Contextual permission to leave the current action |
| Selected move after the bounded fallback | Frame data, price, recovery, playback and observed execution identity |
| Selected emission basis | Candidate weapon gates, refire spend and later ranged events |

Resolve these in production order through one native seam. The offer borrows
the result without spending. Acceptance resolves against current mutable facts
before teardown/payment; an earlier forecast is no reservation. The separate
grab candidate also needs the touched admission owner instead of bypassing it.
Remove independent forecast ordering rather than patch only one ID argument.

### A final event must carry facts its completed use owned

The [move validator](../../../crates/ambition_entity_catalog/src/lib.rs)
accepts events at `duration_s`. Advancement emits that final event and removes
the finished playback. The [production chain](../../../crates/ambition_platformer2d_runtime/src/combat_schedule.rs)
applies the removal before dispatch. Source and occurrence survive on the
event, but dispatch still reads `shot_damage`, charge fraction and captured
start aim from the body's current playback.

Thus a valid endpoint shot can lose its damage override, charge payoff or
captured aim. No replacement action or exceptional timestep is required.
Checking the occurrence cannot recover data from a removed playback.

Stamp those small use-owned values while emitting the ranged event. Keep
the lowering order: source/damage override, current worn modifiers, then
charge scaling. Live fire input still precedes captured aim; current body and
muzzle geometry remain live. Remove those late playback lookups. Extending
playback lifetime or cloning the whole move/actor would obscure the actual
event contract and retain more than its consumer needs.

The new windup-swap witness proves retention of A's numbers. The offer helper
witness proves its resource fallback. Neither covers the source-selection,
cancel-namespace or final-event crossings above. Extend those existing native
checks with these cases; no roster matrix or helper-agreement suite is needed.

## Input — a direction includes its selected meaning

The [player translator](../../../crates/ambition_characters/src/brain/player.rs)
chooses either a captured C-stick direction or the movement axis. The
[portal guard](../../../crates/ambition_platformer2d_runtime/src/portal_seat.rs)
later uses `attack_axis == locomotion` to infer movement-derived aim.

With both sticks resolving to `(1,0)`, a held reflection `diag(-1,1)`
reverses an independently selected C-stick attack. A different C-stick
vector survives. Numeric coincidence has replaced the intended input policy.
This counterexample needs no portal-induced gravity turn.

Retain the translator's actual source choice in resolved control, including
its existing fallback when a qualified aim-stick press has no valid vector.
Keep the physical-device interpretation at that boundary. A small derived
semantic value is sufficient; do not add another input resolver.

There is a related space error. A walker's Y may mean crouch/descend. The
guard converts the whole `(x,y)` to world space, constrains it, then restores
only Y. That cannot remove Y's contribution to the new X. For down `(r,r)`,
side `(r,-r)`, `r = sqrt(1/2)`, exit normal `(-1,0)`, and local input `(0,1)`,
the constraint produces X `-0.5`. Pure posture becomes side movement.

Project the travel command the current motor can express; keep posture out
of that vector. A flyer still has two travel axes. Attack aim has its own
policy and must not reuse the walker's posture-preserving projection. The
current quarter-turn fixture hides this cross-axis coupling. Add the oblique
case and coincident C-stick direction to the existing crossing. This is
current control semantics, not selection of the speculative gravity program.

## Observation — unchanged demand does not mean a current product

[`ModuleObservation`](../../../crates/ambition_characters/src/perception.rs)
combines demand selected from a binding with a projection prepared for a
decision. The [demand resolver](../../../crates/ambition_combat/src/extension/think.rs)
keeps an existing `believed` value when demand is unchanged. Native decision
excludes dormant bodies and can skip missing senses/world facts, while the
sighted queue accepts any retained `Some` as prepared now.

The [sighted aimer fixture](../../../fixtures/extension_fixture_modules/src/lib.rs)
establishes a complete action path. An awake armed hostile observes a target.
The participant moves beyond the room's wake radius; native dormancy clears
control and skips decision. The binding is unchanged, so the old projection
survives. The module submits fire from that target; lowering writes it back
before the control gate. With health, readiness and a clear muzzle, ordinary
emission can publish a shot. No downstream dormancy check settles that new
autonomous decision. This supported composition has not been run here.

An old remembered position can be a valid result of a new decision. The
defect is that an old projection is treated as proof that a decision ran.
Keep retained belief and current-product availability separate. Invalidate
the product before producer eligibility filters at each execution, or use an
equivalent native publication boundary. A replay of the same `SimTick` must
also invalidate it; matching a tick number alone is insufficient.

The two custom-body think routes need the native decision owner's eligibility.
Preserve their different knowledge contracts. Avoid both a copy of the native
filter list in every trigger and actor dormancy policy inside the generic
extension executor. Use the existing decision publication and bounded module
state. Preserve the no-demand fast path, historical memory, ongoing accepted
actions, and independent projectiles. Extend the existing fixture across
sleep/wake and a skipped producer in restoration.

## Music — a new use can reuse the same section

**Closed by `2fb366d1e`.** A valid A -> B one-shot graph could skip B when
both states used the same section and A completed before host observation.
The bounded owner now retains the selected continuation use independently of
its optional deadline. Requests, admitted groups and completion retain the use;
its retry budget survives deadline changes. The outer entry and claim remain.
A live same-section mix change keeps its playback occurrence.

Guard: 154 audio tests cover early scheduling, late completion, delayed decode,
self-repeat and a live mix control. The existing native short-completion test
now also plays the same section again after a host stall. It passes through
native software capture. Assembled-app compilation passes. No new executor,
event history or stronger timing guarantee was added.

## What the recorded measurements establish

The source contains meaningful native-audio receipts. The following are the
agents' reported observations in the
[music plan](music-programs-and-playback.md#verification-limits-and-future-acceptance),
not independent measurements made by this review.

| Evidence | Reported result | Limit |
| --- | --- | --- |
| Native software capture, then scheduled continuations | Initial file-end joins had 20–30 ms gaps; later captures had no silent 10 ms windows across prepared joins, including a one-second host stall | A bounded Linux mixer capture; no sample-accuracy or physical-speaker claim |
| Castle Godwin native route (`047f1134a`) | Entrance, exit/reentry, withheld intro and death restore select the intended programs; exits select Tritone without an intermediate military track | Raw PCM is not in this source export; the retained receipt and test source can be reviewed |
| Hobford native ending (`b60a3b918`) | Shipped Aftermath completes into the latest workshop music | Confirms this customer, not every valid program graph |
| Scene pause/slow motion (`6715d352d`) | Music continues on its independent native clock while scene progression is held/scaled | Does not measure overall engine speed |
| Exported measurements submodule since the baseline | Only LLM resource-accounting ledger/totals changed | No new exported paired frame-time, compile-time, binary-size or monomorphization dataset |

The capture recipe writes recordings to ignored local paths. The archive
also excludes dirty/untracked source and notes dirty submodules, so other
local evidence may exist. Missing raw recordings are a limit on reanalysis,
not proof that the reported captures did not run. This review cannot verify
their host conditions or PCM arithmetic from the exported files alone.

Do not infer a runtime or build speedup from test counts, resource accounting,
fewer callbacks or a new shared helper. The corrections here are established
by native semantics. A later performance claim needs its own comparable
workload and host evidence, under the existing measurement tools.

## Implementation boundaries and cost

Keep each correction under its current owner and queue priority. Shared
combat selection and the final-shot payload can land separately. IN2 needs
no gravity solver change; OBS3 needs no richer observation service; music
needs no replacement executor. The work removes independent reconstruction
at actual consumer boundaries.

Borrow prepared move/source facts during forecasting, retain only an active
use's bounded facts, and preserve live aim/modifier reads. Do not clone an
actor snapshot or rebuild a repertoire every tick. Keep no-demand perception
cheap and music groups bounded. No new utility crate, global provenance
wrapper or generic transition framework is justified by these findings.

Already-open ACT2 expiry/cadence, IN1 recording, IN5 binding withdrawal,
gravity-turn composition, and transfer-cohort work retain their existing
scope. They were not claimed complete and are not agent execution failures.
The next useful audit starts from implementation evidence for these native
boundaries, rather than another broad rewrite of the same plans.
