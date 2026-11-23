# Music programs and owned playback

**State:** planned from static source and a maintainer report, 2026-11-22.
Runtime baseline: `00bf92c2214274fdf2ecaacdc0abfd19be569a50`. Planning base:
`adc7db4863467c32f903fe439e111530414a3f69`. Music renderer:
`186a13f8115752a4bbdb4955fb9e46483562a2d4`.
The supplied exit log has UTC timestamps on 2026-11-23. Its executable
revision is not established. The source gives a complete failure schedule
that fits that log; this review did not execute it.

**Execution:** the existing [music playback row](../queue.md#music-playback--owned-handoffs-and-prepared-entrances).
This page owns selection-to-playback contracts and MUS1–MUS4. It supersedes
the diagnosis-only direction for that row. It does not supersede the
completed heard-room selection, provider authority, or existing diagnostics.
[MA1](asset-authoring-and-prepared-media.md#ma1--export-playback-facts-link-game-music-policy)
retains compiler/export ownership. No runtime code, authored music, or binary
asset changes are part of this planning overlay.

## Goal

An author can bind a musical program to a room, fight, scene, or frontend
route. The program can have one file, an intro and loop, several sections,
aligned layers, an ending, or an optional transition segment. Changing that
representation must not change the choice's gameplay priority or lifetime.

The runtime must know what should play, what media can express, and what
the backend has actually accepted. It must retire the exact playback it
replaces. A request, a decoded file, and an accepted voice are different
facts. A successful command submission is not proof of audible output.

Keep the native owners: MusicIR and its compiler, prepared content, the
game's scoped candidates, and `ambition_audio`. This work does not require
a new crate, a music VM, another event history, or a backend-neutral audio
framework. A simple track is the one-voice case of the same lifecycle.

## Evidence and confidence

### The reported door exit has a concrete command-order defect

The report leaves `castle_godwin_throne` during Wolfenstein's fight and
enters `castle_godwin_great_hall`. These are the relevant observations:

| UTC time | Supplied observation | Source-supported meaning |
| --- | --- | --- |
| 00:27:53.991 | Shut down `galvanic_march`; resume `military_iron_resolve` | Release clears cue identity, retains the bank role, and submits the destination's simple fallback |
| 00:27:54.022 | Start `tritone_decree/decree/song`, old bank B, new bank A, three sources | This is the fresh-start branch that submits plays on A and then stops A |
| 00:27:54.043 | Simple track reported refused | The base instance is absent and not Queued; the observer has no backend refusal reason |
| 00:27:54.049 | Actual output names Tritone | The observer counts Playing, Pausing, and Stopping as playback |
| 00:27:54.293 | Actual output empty; BackendSilent | No tracked voice passes that observation; decoded section sources are present |
| 00:27:54.551 to 00:28:04.916 | Gap grows from 0.507 to 10.846 seconds | The request remains active while observed playback is absent |

In [adaptive playback](../../../crates/ambition_audio/src/music/director/adaptive.rs),
`shutdown_adaptive_cue` stops both banks and calls `clear_active_cue`.
That clear operation removes cue/claim/state/section but keeps
`active_bank` ([state](../../../crates/ambition_audio/src/music/state.rs)).
A fresh start then does this:

1. Capture old bank B.
2. Select A because there is no active cue.
3. Submit the new section's plays on A.
4. Take the no-active-cue cleanup branch.
5. Stop `old_bank.other()`, which is A.
6. Record the new cue as active.

This targets the new group after its play commands. The old-bank-A case
does not have the same alias. The 80 ms stop also fits the brief output
report: [diagnostics](../../../crates/ambition_audio/src/music/diagnostics.rs)
count a Stopping voice as playback. The exact dependency implementation
was not available in this source export. The lockfile selects
`bevy_kira_audio 0.26.0` and `kira 0.12.4`; verify those versions when
implementing the adapter. Do not substitute newer upstream behavior as proof.

The persistent silence also has a source path.
[`refused_by_backend`](../../../crates/ambition_audio/src/music/mod.rs)
requires an absent instance that is not Queued. A present Stopped instance
does not pass. The held same-state path only updates gains. It can therefore
keep an accepted-but-stopped loop logically active without another play.
Partial group acceptance is also not repaired by the all-layers-refused
predicate.

The simple warning is not the adaptive stop. That branch only clears
`MusicPlaybackState`; it cannot stop adaptive handles. Its label also
cannot distinguish refusal from cancellation before materialization.
The image decode messages, one frame spike, and missing `SimId` warning
in the report do not establish causes of the sustained music gap.

### Selection creates an unnecessary intermediate track

The [intent adapter](../../../crates/ambition_platformer2d_audio/src/music/intent.rs)
first resolves adaptive encounter/boss directives. After room publication,
Wolfenstein is no longer in the heard room. The resolver returns an unowned
`StopNow` for his old claim. That value prevents the destination room's
adaptive resolver from running in the same reduction.

Shutdown then resumes the simple candidate `military_iron_resolve`.
On the next frame the cleared boss claim no longer blocks the great hall's
`tritone_decree`. This explains the selection sequence before the bank
failure. It creates a needless start/cancel and makes its warning ambiguous.
The correct destination is Tritone in the inspected content.

The split has other consequences:

- Simple boss publication keeps the first nonempty track per room and one
  `BOSS_MUSIC_OWNER`; waves also keep a first track per room. Adaptive boss
  resolution keeps archetype/phase and discards the live occurrence.
- An adaptive-only boss does not raise `EncounterMusicRequest::priority_of`
  when its simple phase-track fields are empty. Wolfenstein is such a boss.
  The primary participant's ambient room can therefore win over a secondary
  participant's fight, absent another claim.
- An adaptive encounter is resolved before an adaptive boss, independently
  of the simple candidate tiers. The room's fight override only affects
  simple candidates.
- A bound encounter in Failed returns `StopNow` without checking ownership.
  A losing source can thus stop the winner.

These are residuals in the existing Q72/Q150 contract. Preserve the completed
room filter and primary-seat tie rule. Remove the separate reductions that
bypass or collapse them.

### The fight entrance is a separate readiness and clock exposure

Wolfenstein wakes when a driven participant in his room comes within 380
units. His Intro maps immediately to `galvanic_march/awaken`; it has no
authored music-delay field. The 16-beat section at 116 BPM is about 8.276
seconds. Both the encounter and conductor use an 8.28-second intro.

Adaptive media is requested only when Play first wins. All cue sources are
requested then; only the selected section's decoded sources gate its start.
Provider readiness proves catalog registration. The room/neighbor asset
manifest proves image readiness. Neither proves that this intro is decoded.

The encounter and conductor can advance before the audio begins. If the
intro becomes ready later, it can start its full length after the scene has
started. If the phase has already changed, it can instead skip that unstarted
intro. Matching the two durations does not solve cold loading, pause,
slow motion, or a late backend start.

The supplied log contains no fight-opening interval. These are source
exposures, not a proven cause or duration of the reported entrance silence.
Quiet source audio also needs separate evidence. Keep that uncertainty
visible while fixing the preparation contract.

### Existing content intent to preserve

| Customer or event | Current intended policy |
| --- | --- |
| Wolfenstein | Intro → awaken; Phase1 → march; Transition → unleash → march; Phase2/Stagger/Enrage → march. No Death binding or victory outro |
| Inugami | Separate `iron_toccata` entrance, intentionally delayed 7.26 seconds into its intro. Do not remove it while correcting Wolfenstein |
| Great hall | Adaptive `tritone_decree/decree/song`; `military_iron_resolve` is its simple room choice |
| Shared castle/forest and Dinosaur cues | Compatible room/state changes can retain musical position; ownership may change without a new start |
| Held one-shot request | After its continuation starts, the same held request must not rewind the one-shot |
| Completed encounter or saved dead boss | An ending finishes only an admitted occurrence. Loading or revisiting completed state must not replay it |
| Restart during an ending | The new attempt can replace it; old completion cannot clear the new playback |
| Radio, conversation, and day phase | Preserve their declared precedence, release scope, and authored crossfade preferences through common selection |

The score's unused `howl` and `storm` sections do not override Wolfenstein's
explicit game binding. Departure, failure, replacement, and victory can have
different endings. Do not infer victory from a generic release.

## Contract 1 — one scoped musical goal

### Candidates preserve their owners

Project the existing canonical phase, room, participant, route, and lifecycle
facts after the relevant simulation/publication cut. A candidate contains:

- its runtime-derived owner and attempt, session/provider authority, and scope;
- one prepared musical choice, state, authored priority, and continuity policy;
- entry, replacement, and release preferences that the transport can execute.

Use the existing lifecycle/identity owners. A placement or archetype ID alone
does not distinguish two live fights or a new attempt after reset. Authors
name content and policy; they do not type runtime tokens.

Use one reduction for tracks and layered programs:

1. Form the eligible candidates for each local listening participant.
2. Apply a room fight override to that fight's choice. Keep its owner, scope,
   and priority. Preserve the normal fight choice as an explicit fallback
   where the game intends one.
3. Select by authored priority.
4. Across equal-priority participant choices, prefer the primary participant.
5. Resolve the remaining tie with a documented stable rule.

If within-room recency remains a policy, place it in that within-room
reduction. It must not outrank the primary-participant rule across rooms.
ECS iteration, catalog order, decoded readiness, and media representation
must have no ranking effect. Future remote locality supplies the local
listening set; do not treat a remote seat as another local speaker.

Publish one coherent goal revision. An unchanged goal retains its revision.
It names a program/state and its owner,
or deliberate silence with an owner/reason. A missing projection or pending
media is not an implicit silence command. A departing or losing candidate
withdraws only itself. It cannot issue a global stop.

### Ownership, continuity, and playback are different identities

| Identity | Question it answers |
| --- | --- |
| Gameplay occurrence and attempt | Which live fight, scene, route, or source is entitled to request this music? |
| Prepared program/product identity | Which provider and accepted generation define these sources and musical facts? |
| Continuity policy/key | May a new winning owner keep this compatible musical position? |
| Playback occurrence | Which particular submission/start owns these voices, completion, and retirement? |

A shared castle loop can transfer between room claims without restarting.
Two bosses with the same song still have separate attempts. A same-named
cue from another provider or accepted generation cannot inherit old
playback by string equality.

An outgoing ending is a bounded transport obligation, not a permanent
high-priority candidate. Capture its occurrence, reason, selected products,
and release policy. Obtain its successor from current arbitration when the
successor can start. A victory tail must not later restore a room track
captured before another crossing, a new fight, or a radio selection.

An ending may overlap the new winner or delay it for a declared interval.
State which interruptions can shorten that interval. Pending successor work
carries the goal revision and is superseded when that goal changes. Existing
explicit session/provider silence remains authoritative.

## Contract 2 — a prepared program describes possible playback

A program is a small immutable native description of playable sections,
mix states, and supported transitions. It is not an arbitrary graph
interpreter. A one-file loop or one-shot requires one section and one voice.
The runtime may specialize that shape without changing its semantics.

MA1 exports musical and product facts. Game content owns associations,
state/phase mappings, mix intent, priority, continuity, and late-entry policy.
A pure preparation step links them once into IDs, fixed source/gain arrays,
and boundary queries. Use it for content compilation, direct registration,
and reload. Do not validate one entry path more weakly than another.

### Keep three levels of evidence separate

| Level | Facts it can establish | Facts it cannot establish |
| --- | --- | --- |
| Score-only preparation | Form, compiler clock, event ownership, legal markers, continuations, requested product roles | Final encoded length, trim, acoustic quality, or device timing |
| Accepted product preparation | Selected media identity, sample/frame origin, body and loop span, retained pickup/tail, layer alignment, gain basis | Whether separate voices actually start together |
| Native playback observation | Submission, admission, supported transport origin/position, cancellation and terminal state | Speaker-audible output from an instance flag alone |

The accepted description and media use the same product/bundle identity.
The compiled-score fingerprint alone is insufficient: playback and mix
settings also affect the product. Reuse the render dependency identity and
existing accepted-bundle/content-generation owners.

### End, loop, pickup, and tail have distinct meanings

Support these two simple end contracts:

- **AssetEnd:** play the complete selected asset once, including its ending.
  It needs no invented BPM or earlier musical end.
- **MusicalEnd:** an authored musical boundary can admit a successor while
  declared remaining audio follows its release policy.

A whole-asset loop also needs no beat metadata unless an operation asks
for musical quantization. Metadata requirements follow the operation.

For structured products, record the body origin, logical end, loop period,
audible end, entry/exit roles, and how ring material was built. Current loop
export folds a tail into the head and a lead-in into the end. That ring is
present on a cold first pass. Keeping that loop's voice alive continues its
loop content; it does not isolate a release tail. A discarded pickup cannot
be recreated by seeking or scheduling.

Provide a separate cold-entry, pickup, exit, or tail product only when the
requested operation needs material the selected product cannot supply.
Otherwise reject the operation or use an explicit authored alternative.
Do not require every simple cue to export unused variants.

Use composition-level mastering for new recipes. Preserve existing V1/V2
regeneration policy and accepted media; the renderer deliberately retains
`legacy_section_postprocess` for those sources. Changing that policy is a
separate reviewed product change, not a side effect of clock export.
A mastered full mix and grouped pre-master
layers are alternative product sets with different gain bases. Do not sum
the full mix with its own stems, normalize every section independently, or
apply export attenuation twice. Preserve existing omitted-gain-means-zero
rows when migrating them; a new preset's defaults must be explicit.

### Clock and marker preparation

Every supported MusicIR frontend exposes its compiler-owned clock. V1
already renders a tempo map but sets `CompiledScore.exact_metadata` to
`None`. V2/V3 retain richer exact metadata. Adapt those compiler facts;
do not add another YAML interpreter or infer a variable clock from average
BPM. V3 procedural generator clips have narrower clock restrictions than
exact clips; inspection must state the supported form.

Display chapters, entry markers, and legal exits are different roles.
The current playback plan drops exit markers. V3 validation also rejects
`valid_exit_local_bars` although exact-score lowering can preserve it.
Make the public authoring form and its supported lowering agree.

Use the same cut and rounding as the selected product. For the current
global cut convention, a local marker frame is:

```text
round(global_marker_seconds * sample_rate)
  - round(global_section_start_seconds * sample_rate)
```

Rounding a local duration instead can differ by one frame. Product acceptance
checks final body lengths and codec trim semantics. A decoded file's full
duration alone cannot supply its musical end or loop phase.

Retain constant clock arithmetic. Use a prepared cursor or ordered boundary
table for variable tempo/meter, holds, and exit sets. Preserve section
origin and loop wrap. No note traversal, source parsing, render work, or
source hashing belongs in the game loop.

## Contract 3 — playback groups own all starts and stops

The backend owner keeps a desired target, bounded pending work, accepted
playback, and captured retiring occurrences. Bank A/B and channel slots
are storage capacity. They are not lifetime identities.

Each submission owns an occurrence, selected product identity, required
source set, returned handle identities, and lifecycle result. The current
play API can return a handle before its `AudioInstance` materializes.
Keep both facts. Capture the old group before changing current state.

| Lifecycle fact | Required rule |
| --- | --- |
| Preparing | Decode the required group; a held target does not allocate a new request every frame |
| Submitted | Retain every returned handle and reservation; do not call submission a successful start |
| Admitted | Account for every required voice; partial admission is not a complete layered section |
| Started | Attach a defined transport origin and timing guarantee; admission observation alone does not establish sample zero |
| Retiring | Fade/stop the captured occurrence's handles; reuse of a slot cannot redirect that operation |
| Canceled | Invalidate the occurrence immediately; retain its accounting until every submitted voice is canceled, rejected, or retired |
| Terminal | Distinguish natural completion, requested stop, canceled start, reported refusal, and unexplained loss |

Native [SFX instance control](../../../crates/ambition_audio/src/spatial.rs)
already obtains an `AudioInstance` by handle and uses `stop(AudioTween)`
and `set_decibels`. It is a useful native precedent. It does not solve
cancellation of an unmaterialized play. The locked adapter must supply a
concrete cancel/drain strategy. A generation tag alone cannot cancel a
command already queued elsewhere.

For a required layer group, reserve the intended voices and prepare all
sources before submission. If the backend admits only part of it, retire
those exact voices and retain the permitted old group or choose the declared
failure result. Do not add the missing stem later from frame zero. Where
strict synchronized admission cannot be provided, report the supported
timing level; a prepared full mix can be an explicit alternative.

### Bound actual work, including unresolved submissions

The budget includes submitted, partially admitted, current, and retiring
voices. Two bank labels do not by themselves bound active voices: a reused
channel can still contain a prior fading occurrence.

Keep one latest unsent target. If A is current and B is retiring, a third
request supersedes pending preparation. It does not submit a third group
until capacity is available. Policy may shorten B's ending, but releases
the reservation on lifecycle evidence, not only on a host fade timer.
Specify a finite timeout/failure path if the adapter cannot account for a
submission. Never forget an unresolved handle to make room.

For the first migration, use at most two live or unresolved music groups,
at most six layers per group, and at most twelve reserved music voices in
total. Simple tracks, endings, and partial/canceled submissions count in
that total. These are executor reservations, not a guarantee of Kira's
physical capacity; other audio can still cause admission to fail. A later
stinger capability must fit this bound or explicitly revise the budget.
A simple track uses one voice and the same admission,
cancellation, and retirement code. It must not incur a synthetic graph,
six-slot processing, or another resume state machine.

A desired loop that unexpectedly stops has an owned failure to reconcile.
Allow bounded retry only for the still-current goal and failure class.
Do not restart a completed one-shot, resurrect a superseded intro, or add
an independent watchdog that blindly issues Play each frame. Diagnostics
observe this owner; they do not become a second repair authority.

## Contract 4 — a transition is a plan on a declared clock

Resolve one transition from the accepted playback to the current goal.
Its native record needs:

- the outgoing occurrence and destination goal revision;
- an allowed source exit rule and destination entry rule;
- any declared bridge, pickup, or tail product;
- paired gain envelopes and an overlap/admission budget;
- readiness, latest useful start, interruption, and failure policy.

The first exit set can be immediate, AssetEnd, MusicalEnd, an explicitly
permitted next bar, or the next member of a named exit set. The first entry
is section start. More capable entry rules need explicit product and native
support. A graph route solver is not required.

A beat/bar clock locates time. It does not grant permission to leave a
piece there. Select the earliest **allowed** boundary whose complete entry
operation meets the native not-before deadline. With a pickup, that deadline
applies to the pickup's earlier start, not only its destination downbeat.
If the deadline is missed, choose another allowed boundary. Do not turn
“next bar” into “next bar plus 80 ms”; current `max(MIN_TRANSITION_DELAY_SECONDS)`
can make that distinction matter.

A loop can continue until a later allowed exit. An exhausted one-shot
cannot continue forever. Its recipe must define an admitted continuation,
fallback, finish, or permitted silence. Schedule predictable intro-to-loop
continuations ahead when the native adapter supports it; waiting for the
frame after an intro ends cannot promise gapless output.

One transition owns its envelopes. Avoid multiplying a start fade, an
unrelated gain smoother, and an outgoing fade without declared meaning.
State whether a same-section change adjusts its existing layer mix or
starts another product. A held target with unchanged gains emits no new
play or volume work.

User and environment mix also enter this owner as data. The current
[`apply_audio_environment`](../../../crates/ambition_platformer2d_audio/src/environment.rs)
writes the base music channel volume directly, while
[`sync_music_mix`](../../../crates/ambition_platformer2d_audio/src/music/mod.rs)
copies user volume without environmental attenuation. The former source
warns that channel-volume commands replace instance volumes. Leaving that
writer beside new per-instance envelopes can overwrite them and bypass the
recording sink. Compose effective user/environment gain, program/layer gain,
and the owned envelope once for current and retiring voices. A mix change
does not start a new occurrence. No new bus framework is required.

### Four time facts must not be substituted

| Time fact | Owner and use |
| --- | --- |
| Gameplay/scene time | Canonical phase, attempt, and authored scene anchor; can pause, scale, or replay |
| Host monotonic time | Loading deadlines, diagnostics, and an explicitly approximate playback fallback |
| Music transport origin/position | Supported native scheduled start or position evidence for this playback occurrence |
| Score-to-product clock | Compiler map from musical coordinates to the selected media's frame coordinates |

The current `Time<Real>` counters are host-frame time, not a sound-device
clock. A frame that notices admission or Playing is not necessarily the
first rendered sample. Keep submission time, admission observation, and
transport origin separate.

Ordinary playback may retain an explicitly best-effort host timing path.
Do not make sample accuracy a prerequisite for MUS1. Requests for stronger
timing must be admitted only when the pinned native adapter supports them.
Matching source frame origins is necessary for aligned layers, but does
not prove synchronized device starts.

### Scene coupling and late entry are game policy

Prepare the known opening before its scene deadline. When it becomes
realizable, compare its goal revision and current gameplay phase again.
A prepared but superseded `awaken` must not start after `march` has won.

Use a small explicit late-entry policy: enter the current requested state;
join a declared musical position/boundary if supported; or deliberately
play a full entrance. Wolfenstein needs this choice at his scene binding.
Preserve his intended intro-to-fight coordination without removing
Inugami's authored delayed entrance.

If a scene's authored duration should come from a score span, link that
finite value during content preparation. Gameplay then uses its pinned
scene definition. A visual/audio quality choice cannot retime that gameplay
definition. Decoder readiness, local mute, audio completion, and
device availability must not decide when combat starts. Pause, slow motion,
rollback correction, reentry, and a new attempt need an explicit continuation
or resynchronization policy. Subtracting simulation elapsed time from a
real-time musical duration is not that policy.

## Contract 5 — preparation anticipates playback without claiming it

Use existing room/neighbor/encounter preparation demand. A prepared room
can name its ambient program, known placed fight entrances, and useful
immediate continuations. Wolfenstein's dormant placement is known before
the wake radius; the great hall is known before the door publication.
Resolve exact selected product identities and decoder needs there.

Prefetch grants no playback claim. It remains bounded by the existing
destination/residency policy and its generation/session ownership. Request
the opening and imminent continuation before distant sections where useful;
do not decode the entire music catalog or require every future section to
be ready before starting. Keep already-decoded products reusable.

Catalog linked, media decoded, voices admitted, and output running are
separate readiness states. A pending replacement keeps only the outgoing
playback that its release policy permits. A deliberate cut or silence can
retire it immediately. A configured fallback is a selected outcome with a
reason, not a silent substitution based on which asset happens to be ready.

Audio device unlock and decoder timing are local presentation facts. They
must not enter peer-room consensus or gameplay readiness. If a scene needs
a local presentation readiness condition, use the existing readiness owner
with a bounded failure result. Do not create another room transaction or
an indefinite cover that waits for a speaker.

## Authoring interface and rejection

Keep declarative MusicIR and the existing game policy surface. Provide a
few inspectable recipes that lower into the same native program:

| Recipe | Author supplies | Preparation supplies |
| --- | --- | --- |
| Full-asset loop | Product and mix level | One section/voice, whole-asset repeat, no musical clock obligation |
| Full-asset one-shot | Product and completion/release choice | AssetEnd and owned completion |
| Intro → loop | Named sections and continuation | Logical end, required products, next-section demand |
| Intro → loop → ending | Named ending, exit policy, release reasons | Legal exit query and tail/overlap obligations |
| Layered section | Aligned layer set and mix states | Total gain arrays, common phase facts, required group |
| Bridge or stinger | Named product, timing role, interruption/ducking policy | Supported schedule, owned completion, admitted overlap budget |

Inspection prints the expanded defaults, source locations, selected products,
clock/end semantics, exits, maximum musical wait, continuity and late policy,
required voices, and unresolved product/native support. A long section with
one legal exit is valid; its wait must be visible. No new GUI is required.

The compiler/exporter validates musical clock, marker, and ring construction
from its event provenance. One native preparer validates the linked program
for content compilation, direct registration, and reload. Inspection combines
their diagnostics; native code must not import notes or reinterpret MusicIR.
Together these boundaries reject:

- unknown fields or wrong types, duplicate cue/section/state/layer/marker IDs,
  duplicate slots/sources/gains, and invalid balance override references;
- empty playable sections, missing required sources, excess layer count,
  or unresolved state/continuation/marker references;
- impossible or zero-time automatic continuation cycles, invalid clock
  ranges, out-of-range exits, and unreachable requested endings;
- mismatched layer origins, loop periods, sample rates/trim contracts,
  product generations, or gain domains;
- independent attacks hidden in a declared note-free ring region, and a
  requested entry/tail operation unsupported by its product;
- synchronization or phase-preservation requirements stronger than the
  native adapter's declared support.

Intentional silence and positive-duration loops remain valid representations.
An empty source list must not become a playable section through an
all-sources-ready check over zero elements. Validate duplicate cue IDs
before collecting them into a map. The current direct native registration
must not bypass checks performed by content compilation.

Current ring validation checks adjacency, not compiled event ownership.
An early humanized attack from the loop head can belong to its lead-in;
an unrelated new phrase is different. Source inspection can check that
ownership. It cannot prove the acoustic decay has finished.

Support must be explicit for these further operations:

| Operation | Additional requirement |
| --- | --- |
| Variable tempo/meter exit queries | Compiler clock projection; rendered audio needs no runtime time stretching |
| Phase-preserving change across products | Declared phase relation and compatible clock/period plus synchronized seek/start or prepared entry product; equal BPM is insufficient |
| Pickup before a downbeat | Retained pickup product/offset, sufficient lead time, and native scheduling support |
| Independently released loop tail | Retained exit/tail material or supported release operation |
| Sample-timed layer start | Common native transport origin, group admission, and backend evidence |
| Stinger/bridge over a bed | Named musical role, timing/completion and ducking policy, explicit voice budget |

The plan supports professional music concepts without making every capability
an immediate implementation gate. Official FMOD descriptions of
[timeline transitions](https://www.fmod.com/docs/2.04/studio/authoring-events.html)
and [instrument timing](https://www.fmod.com/docs/2.04/studio/working-with-instruments.html)
are reference examples for quantized transitions, relative position, and
transition material. They do not establish support in this native adapter
or require adopting middleware.

## Output, inspection, and efficient execution

Device and recording modes consume the same music decisions. The recording
sink must actually consume starts, gains, cancellation, and retirement
without writing to device channels that have no drain. It need not emulate
DSP or sound quality. It must expose the same ownership/order decisions
that the device adapter executes.

In the inspected recording path, layer Play is gated by output mode but
stop/volume operations are not. Gain smoothing submits twelve channel volume
controls per active frame while Kira drains are absent. This is source-level
work that should disappear. The locked queue's storage behavior was not
inspected; no unbounded-memory claim or measured saving follows.

Expose a compact observation with goal/claim revision, selected product,
preparation stage, playback occurrence, submitted/admitted voice count,
transport guarantee/position, next boundary, outgoing occurrence, and the
last cancel/terminal reason. A Stopping outgoing voice can explain residual
audio; it cannot prove the desired incoming group has started. Keep
BackendSilent, deliberate silence, mute, suspended device, awaiting gesture,
and missing context distinct. Report observed absence when no refusal
reason exists.

The production path should retain these cost properties:

- One-source tracks stay one-source. No per-song system or generic type
  instantiation is needed; use common concrete native records.
- Prepare IDs, source layouts, total gain arrays, and boundary queries once
  per accepted generation. Keep steady-state work proportional to active
  candidates/groups, not the whole cue catalog.
- Keep fixed layer storage and bounded current/pending/retiring work.
  Changed gains or active fades can emit controls; empty slots and steady
  gains need none. Prefer a supported native tween where it owns the envelope.
- A silent layer required for future mix changes cannot simply restart from
  zero later. Any virtualization needs a supported phase-restoration policy;
  it is not part of a blind zero-gain optimization.
- Recording consumes the same semantic operations with bounded state.
  Source parsing, music synthesis, product hashing, and render inspection
  stay outside gameplay.

These are design cost constraints. This review measured no CPU, memory,
compile-time, binary-size, or listening result.

## Implementation cuts and deletion conditions

The queue preserves existing product priorities. MUS1 and MUS2 address the
reported handoff and shared selection correctness. MA1 can proceed in
parallel. MUS3 consumes its prepared facts for structured timing; MUS4
adds authoring recipes and only the further capabilities selected by a real
customer. Basic strict native preparation starts with the first new records.

### MUS1 — own playback groups and consume commands at one output boundary

**Owner:** `ambition_audio` director/state/channels/output.

Build the bounded occurrence lifecycle below the current intent entry
points. Capture actual handles, partial admission, cancellation, retirement,
and terminal results. Define the locked adapter's drain/cancel behavior.
Keep its timing guarantee honest. Existing simple/adaptive callers may be
temporary adapters into this one executor; do not write two group owners.
Make recording consume the same command plan.

**Remove:** retirement derived from `old_bank.other()`; independently
authoritative `fading_bank`/fade countdown state; mode-reset retries based
only on missing instance assets; idle/empty-slot writes into undrained
recording channels; the environment's direct music-channel volume writer
after its effective gain enters the common executor. Compatibility
observations must be projections, not additional owners.

**Narrow acceptance:** the reported old-B/fresh-A start cannot stop itself;
repeat with old A. Supersede an unmaterialized submission, reuse a channel
role, and deliver its late result or old retirement. Reuse is allowed only while
all unresolved voices remain counted within the total budget, or after the
old reservation is fully accounted for. Only the old occurrence is affected.
Refuse one required layer and stop an accepted desired loop:
both reach explicit bounded outcomes. A naturally finished one-shot stays
finished. A user/environment gain change preserves the transition envelope
and reaches both current and retiring groups. These checks use the actual
command boundary, not only director mode/gain fields.

### MUS2 — select one occurrence-owned goal for all music

**Owners:** native music intent, encounter/boss/wave publication, frontend
selection, and the existing scoped claim owner.

Publish occurrence candidates with one priority law. Lower simple choices
to the one-source program and executor. Resolve destination selection and
old-owner release together. Keep the outgoing ending separate from the
latest successor goal. Apply fight overrides before arbitration.

**Remove:** adaptive entity/catalog rescans as a second selector; archetype
and authored-string ownership reconstruction; simple boss/wave
first-track-per-room consolidation; normal ownerless `StopNow`;
independent adaptive/simple winner fields; unconditional
`resume_simple_music`, its mode-setting boolean and
`default_resume_started`; duplicate simple acceptance/retry authority.
Migrate radio/scene/default callers through the same owner without requiring
simultaneous reauthoring of every asset.

**Narrow acceptance:** throne-to-great-hall chooses Tritone once, with no
temporary military track unless an explicit fallback selected it. An
adaptive-only fight in a secondary local room beats primary ambient music.
Reverse insertion order for two same-archetype bosses and mixed
simple/adaptive sources. A losing failed owner cannot stop the winner.
During an ending, cross again and select radio: no captured old fallback
starts. Preserve held continuations, shared-room continuity, cleared-save
entry, and restart-during-ending behavior.

### MUS3 — prepare entrances and plan transitions on the correct clock

**Owners:** native transport, existing room/residency demand, prepared game
scene policy; MA1 supplies producer facts.

Prefetch the known opening and imminent continuation without granting
claims. Keep catalog, decode, admission, and start facts distinct. Add the
small boundary/entry/late-policy plan and a defined transport origin.
Ordinary playback can remain best effort; stronger requests need verified
native capability. Link any scene duration derived from music during
preparation, never from device completion.

**Remove:** hand-copied clock/path values for migrated MA1 customers;
host elapsed time labeled as device time; fixed minimum-delay clamping
that moves a selected musical boundary; hardcoded resume overlap as the
only way an ending can hand off.

**Narrow acceptance:** delay Wolfenstein's preparation beyond Intro.
Gameplay retains its scene schedule; music follows the declared late-entry
policy and never starts a superseded awaken. A prepared intro-to-loop
handoff uses its declared logical end and tail. A missed loop exit selects
the next allowed exit; a pickup meets its earlier deadline. An exhausted
one-shot uses its declared outcome. Device timing and audible continuity
remain later native acceptance, not claims of a recording sink.

### MUS4 — make authoring recipes strict and inspectable

**Owners:** MusicIR/compiler/exporter and native cue preparation.

Expose compiler clock and typed marker support for every supported
frontend. Make final product construction and support limits explicit.
Use one strict preparer for content, direct registration, and reload.
Add the small common recipes and inspection described above.

**Remove:** permissive duplicate/map-collapse admission; unchecked empty
playable sections; silent loop-type/default mistakes; raw-YAML timing
reconstruction; independent numeric/path copies once the prepared link
owns them. Keep composition mastering and accepted generation selection.

**Narrow acceptance:** invalid products/references/slots fail at their source
locations. A simple AssetEnd one-shot needs no score clock. A V1 tempo-map
piece and V3 variable-clock loop produce correct inspection boundaries
without synthesis. A requested independent tail on a folded-only product
fails or chooses its authored alternative. A source change cannot retime
retained old audio.

Keep MA1's Hobford and Dinosaur first customers. Add Galvanic/Tritone for
the reported handoff. Use existing `fable` and `billions_and_billions`
as variable-clock inspection customers, not claims of shipped failures.
Use `shatter_the_iron_sky` to move its prose exit constraint into an
executable marker policy when that content is selected.

## Verification limits and future acceptance

This review used static reads and the supplied log. No build, test,
renderer, game, backend drain, device, or listening run was performed.
Existing recording tests remain useful for state/continuation policy.
They do not execute Kira drains and cannot prove the new-bank stop ordering
through mode assertions alone.

Implementing agents should prove the few ownership, arbitration, and clock
properties above at their native seams. Use one coherent command consumer,
not a parallel fake director. Do not add permanent source-name tests,
migration matrices, or a broad audio-test campaign.

Later authorized native acceptance must use the locked backend and the
reported route: approach Wolfenstein cold and prepared, start the fight,
leave during it, reenter, and let relevant endings finish. Observe admission
and transport timing separately from speaker-audible output. Keep the
older unexplained death/respawn gap open until its own evidence is obtained.
The current report does not establish its cause.

### Source entry points for an implementing agent

Paths are the current owners; line numbers are navigation hints for this
baseline, not fixed API contracts.

| Question | Source |
| --- | --- |
| Exact fresh-start/stop alias | [adaptive.rs](../../../crates/ambition_audio/src/music/director/adaptive.rs), start 325–441 and shutdown 499–534; [state.rs](../../../crates/ambition_audio/src/music/state.rs), clear 73–79 |
| Retry and output interpretation | [music/mod.rs](../../../crates/ambition_audio/src/music/mod.rs), 24–35; [director](../../../crates/ambition_audio/src/music/director/mod.rs), 91–198 and 250–265; [diagnostics](../../../crates/ambition_audio/src/music/diagnostics.rs), 180–357 |
| Device/recording commands | [channels](../../../crates/ambition_audio/src/music/channels.rs), 89–164; [output](../../../crates/ambition_audio/src/output.rs); [gains](../../../crates/ambition_audio/src/music/director/gains.rs), 77–95; [instance control](../../../crates/ambition_audio/src/spatial.rs), 148–175 |
| Mix authority and legacy mastering | [environment mix](../../../crates/ambition_platformer2d_audio/src/environment.rs), 150–180; [user mix](../../../crates/ambition_platformer2d_audio/src/music/mod.rs), 30–37; [mastering policy](../../../tools/ambition_music_renderer/ambition_music_renderer/processing/mastering.py), 45–72 |
| Selection and occurrence loss | [intent](../../../crates/ambition_platformer2d_audio/src/music/intent.rs), 118–237 and 416–554; [claims](../../../crates/ambition_encounter/src/music.rs); [boss publication](../../../crates/ambition_boss_encounter/src/systems.rs), 404–443; [wave publication](../../../crates/ambition_encounter_features/src/systems.rs), 633–647 |
| Actual game policy | [cue catalog](../../../game/ambition_content/assets/audio/music_cues.ron), 315–355, 403–430, 498–504, 572–585; [Wolfenstein](../../../game/ambition_content/assets/data/boss_encounters/wolfenstein_boss.ron); [Inugami](../../../game/ambition_content/assets/data/boss_encounters/wolfenstein_inugami.ron); [conductor](../../../game/ambition_content_modules/src/wolfenstein.rs), 724–742 and 1547–1551 |
| Readiness versus catalog registration | [cue assets](../../../crates/ambition_audio/src/music/catalog.rs), 178–215; [provider lifecycle](../../../crates/ambition_platformer2d_provider/src/lifecycle.rs), 504–575 and 1207–1224; [room assets](../../../crates/ambition_platformer2d/src/world_flow/room_transition_assets.rs), 39–93 and 1432 onward |
| Native preparation paths | [cue spec](../../../crates/ambition_audio/src/cue_spec.rs), 244–313; [content schema](../../../crates/ambition_audio/src/content_schema.rs), 169–210 and 280–319; [catalog registration](../../../crates/ambition_audio/src/music/catalog.rs), 22–29 and 283–343 |
| Compiler/frontends | [compile](../../../tools/ambition_music_renderer/ambition_music_renderer/musicir/compile.py); [V1 projection](../../../tools/ambition_music_renderer/ambition_music_renderer/render/score_layers.py), 1414–1528; [exact clock](../../../tools/ambition_music_renderer/ambition_music_renderer/render/exact_score.py); [V3 validation](../../../tools/ambition_music_renderer/ambition_music_renderer/validation/v3.py), 121–124 and 1052–1073 |
| Product construction and identity | [playback layers](../../../tools/ambition_music_renderer/ambition_music_renderer/render/playback_layers.py), 81–315; [dependencies](../../../tools/ambition_music_renderer/ambition_music_renderer/render/dependencies.py), 423–450; [publication](../../../tools/ambition_music_renderer/ambition_music_renderer/cli.py), 413–503 |

Keep [music diagnostics](../../systems/music-playback-diagnostics.md) as the
current observation contract until implementation changes it. Update that
system document with the actual lifecycle when the corresponding slice lands.
