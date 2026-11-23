# Music programs and owned playback

**State:** implementation in progress, 2026-11-23.
Reviewed source: `6d100f9d9e0d557fe4ae3dad61acd5aef71f684e`.
MUS1's bounded playback executor landed in `d4147e4c`; per-source music
publication and canonical attempts landed in `6d100f9d`. Release facts now
retain their transition attempt and pass through host confirmation. One scoped
goal now selects tracks and cues. Endings retain their admitted entry and
generation while the latest goal selects the successor. Prepared exit rules and
whole-asset recipes landed in `afeb52f24`. Native timing acceptance remains open. Music renderer:
`186a13f8115752a4bbdb4955fb9e46483562a2d4`.
The supplied exit log has UTC timestamps on 2026-11-23. Its executable
revision is not established. The historical diagnosis used source
`00bf92c2214274fdf2ecaacdc0abfd19be569a50`; the receipt below separates it
from current code. This review did not execute either revision.

**Execution:** the existing [music playback row](../queue.md#music-playback--owned-handoffs-and-prepared-entrances).
This page owns selection-to-playback contracts and MUS1–MUS4. It supersedes
the diagnosis-only direction for that row. It does not supersede the
completed heard-room selection, provider authority, or existing diagnostics.
[MA1](asset-authoring-and-prepared-media.md#ma1--export-playback-facts-link-game-music-policy)
retains compiler/export ownership. Implementation changes land in the native owners described below.

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

**Historical diagnosis, not a remaining bank implementation task.** The
report leaves Wolfenstein in `castle_godwin_throne` for
`castle_godwin_great_hall`: shut down Galvanic, resume the military track,
start Tritone with old bank B/new bank A, briefly observe output, then
remain silent. At `00bf92c2`, fresh start submitted new plays on A and
then stopped `old_bank.other()`, also A. Held-state and stopped-instance
handling could leave that request logically active. This source schedule
fits the log; it did not establish the executable revision or speaker output.

`d4147e4c` removed bank-owned retirement. The current
[playback executor](../../../crates/ambition_audio/src/music/playback.rs)
owns bounded groups, exact returned handles, cancellation, required-source
admission, bounded retry and terminal results for simple and adaptive music.
Keep that owner. Do not ask an agent to recreate it from the old diagnosis.
The unified goal reduction below removes the independent simple-resume path.

Native acceptance still needs the locked `bevy_kira_audio 0.26.0` /
`kira 0.12.4` adapter and the reported route. A command or instance flag
does not prove audible continuity. The old simple-track warning did not
identify the adaptive stop; the image decode, frame spike and missing
`SimId` messages do not establish causes of the sustained gap. The old
source and log analysis remain in Git history.

### Unified selection removes the intermediate track

The [intent adapter](../../../crates/ambition_platformer2d_audio/src/music/intent.rs)
now selects one goal from tracks and cues. Room publication can select the
destination's adaptive cue in that reduction. No simple-resume selector runs
between the old boss claim and the destination goal. The correct destination
is Tritone in the inspected content.

`6d100f9d` repaired the publication side: each boss/wave source publishes
independently, attempts come from the canonical encounter lifecycle, and
adaptive-only claims participate in heard-room priority without a dummy
simple track. The goal adapter consumes each published source and attempt,
checks its live lifecycle in its room, and applies fight overrides before
arbitration. Local listening rooms, primary-seat ties, and source recency use
the same law for both representations. A losing failed source cannot issue
an ownerless stop.

### The fight entrance is a separate readiness and clock exposure

Wolfenstein wakes when a driven participant in his room comes within 380
units. His Intro maps immediately to `galvanic_march/awaken`; it has no
authored music-delay field. The 16-beat section at 116 BPM is about 8.276
seconds. Both the encounter and conductor use an 8.28-second intro.

Room preparation now requests the ambient opening, known placed fight
entrances and their immediate continuations. It uses the incoming session's
frozen catalog and the existing bounded neighbor selection. The local startup
barrier waits for its ambient opening. Anticipated fights do not hold gameplay
or publish music claims. Catalog linkage, media decode and voice admission
remain separate facts.

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

### Preserve release cause separately from the latest goal

Current candidates state who can request music now. They do not explain
why a previously admitted attempt ended. The
[`reduce_encounter_lifecycles`](../../../crates/ambition_encounter/src/lifecycle.rs)
can accept Complete, Reset and Start before presentation reads the result.
It now stamps each accepted transition before the next command. Both
[`EncounterEventMsg`](../../../crates/ambition_encounter/src/events.rs)
and the separate presentation-only `EncounterRelease` carry the correct
attempt. The release also retains the source `SimId`, session, room, producing
tick and reason. Music publishers still withdraw terminal sources. Selection
must match the release to admitted playback instead of treating absence as victory.

Capture source/occurrence, attempt and accepted release reason at the
transition that produces that fact, before reset or retirement can erase
it. Do not stamp an earlier completion with the attempt found in the final
batch-publication loop. Preserve the accepted binding when a music source
and its lifecycle have different identities: a boss source uses its
`SimId`, while its attempt comes from the generic encounter occurrence.
A later archetype scan after either has gone is not that binding.

Deliver the small presentation-only release projection through its host's
declared policy. GGRS can reuse its
[confirmed-frame effect journal](../../../crates/ambition_platformer2d_runtime/src/external_effects.rs)
and abandoned-frame discard; non-speculative or already settled source facts
may be delivered immediately. For hosted execution, follow
[room replay ownership](../../systems/room-replay.md) and the
[host's confirmation boundary](island-rollback-and-compact-peer-input.md#the-live-confirmed-frame-boundary-on-a-hosted-host-2026-11-18).
Where replay can still invalidate a release fact, retain its source room,
attempt and producing cut, and apply that host's settlement/retraction rule
before treating it as an irreversible completion. Preserve other rooms'
pending facts and live readers. Registering a presentation replay channel
preserves those streams during replay; it does not prove settlement. Do not
substitute whole-frame replacement for source-room retraction, or impose a
new world-wide wait on facts the host already treats as settled.

Keep ordinary encounter messages available to their simulation readers
rather than quarantining the whole event family. Gameplay publishes its
result without consulting voice admission or waiting for music.

Consume released facts before goal reduction. Retain at most the matching
disposition needed by each currently owned playback/ending obligation;
facts for never-admitted or already disposed attempts create no ending.
Expire it when the obligation finishes, declared interruption cancels it,
or its session/provider ends. The host's correction window bounds pending
delivery; music needs no event archive. Coalescing unsent successor
goals must not overwrite this bounded outgoing disposition.

Completion and preemption are different. Leaving the listening set, radio
selection or a new fight can replace music without completing its source.
If candidates are visible ahead of terminal confirmation, absence means
withdrawal until its relevant cut settles. The transition policy either
keeps the bounded outgoing obligation for the settled reason or explicitly
allows interruption to suppress its ending. It must state which; it cannot
guess victory. This choice never holds gameplay or requires replaying every
transient candidate that presentation did not admit.

### Decide continuation at the selector-to-executor boundary

Compatible product data is necessary for reuse, but does not decide whether
an entry should start again. Current
[`MusicPlaybackRequest::can_continue`](../../../crates/ambition_audio/src/music/playback.rs)
compares entry, product, looping and source handles. Repeating it preserves retry
and exhausted state; a completed one-shot deliberately stays complete.
The adaptive director also recognizes held requests by cue/state. Adding
an attempt only to an outer candidate leaves those decisions unchanged.

The selected transition must explicitly retain compatible playback or
begin a fresh entry, and carry that decision through both the director
and the existing executor. A fresh entry owns fresh completion and bounded
retry state even with identical media. A held entry keeps its continuation
or completion. Compatible owner transfer can retain the native playback
occurrence and rebind entitlement. A gain, ownership or goal revision is
not by itself a request to restart. Old terminal results remain scoped to
the entry/occurrence that produced them.

| Case | Required result |
| --- | --- |
| Same held one-shot after completion | Remains complete or on its declared continuation; no rewind |
| New attempt with the same media and fresh-entry policy | New entry, completion and bounded retry scope |
| Compatible winning owner transfer | Retains musical position and native playback occurrence |
| Late terminal result from a replaced entry | Affects only the old entry/ending obligation |

This is a MUS2 extension of the native MUS1 boundary, not another playback
executor or repair watchdog. Product equality can remain a cheap compatibility
check after the transition's meaning is known.

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

Every supported MusicIR frontend now exposes its compiler clock through
`CompiledScore.clock`. V1 retains `exact_metadata: None` and projects its
procedural tempo map. V2/V3 project the exact meter and tempo owners. Native
preparation must consume these facts; it must not parse YAML or infer a
variable clock from average BPM. V3 procedural generator clips have narrower
clock restrictions than exact clips; inspection states that limit.

Display chapters, entry markers, and legal exits are different roles. Source
preparation now retains bar exits and V3 admits `valid_exit_local_bars`.
Encoded products retain the resolved frames. The native transition planner
queries these prepared bar and named-exit boundaries.

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
“next bar” into “next bar plus 80 ms”. The fixed minimum-delay clamp is
removed.

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

User and environment mix enter this owner as data. MUS1 replaced the direct
music-channel volume writer in
[`apply_audio_environment`](../../../crates/ambition_platformer2d_audio/src/environment.rs)
with the effective `MusicMix` value and removed the separate user-only sync.
The executor composes user/environment gain, program/layer gain, and the owned
envelope for current and retiring voices. A mix change does not start an
occurrence. No new bus framework is required.

### Four time facts must not be substituted

| Time fact | Owner and use |
| --- | --- |
| Gameplay/scene time | Canonical phase, attempt, and authored scene anchor; can pause, scale, or replay |
| Host monotonic time | Loading deadlines, diagnostics, and an explicitly approximate playback fallback |
| Music transport origin/position | Supported native scheduled start or position evidence for this playback occurrence |
| Score-to-product clock | Compiler map from musical coordinates to the selected media's frame coordinates |

The director reads the matching occurrence's file cursor. Native output uses
Kira observations and one shared Kira clock. A ready one-shot continuation is
admitted before its predecessor's prepared end. That clock schedules its resume
and the predecessor's exit envelope. Each bounded playback group owns one native
track. The track's exit gain is separate from the voices' entry fades and user
mix. Withdrawal replaces the exit command directly. Native instances are stopped
and released after retirement. Recording output uses an explicit host-clock model.
Host time owns loading deadlines and diagnostics. A requested mixer-clock origin
or a frame that observes Playing is not proof of the first audible sample.
Keep submission, admission observation and requested transport origin separate.

Ordinary playback may retain an explicitly best-effort host timing path.
Do not make sample accuracy a prerequisite for MUS1. Requests for stronger
timing must be admitted only when the pinned native adapter supports them.
Matching source frame origins is necessary for aligned layers, but does
not prove synchronized device starts.

### Scene coupling and late entry are game policy

Prepare the known opening before its scene deadline. When it becomes
realizable, compare its goal revision and current gameplay phase again.
A prepared but superseded `awaken` must not start after `march` has won.

Phase bindings now select `FollowPhase` or `CompleteSection` for a one-shot.
`FollowPhase` accepts the current requested phase when its target is ready.
`CompleteSection` lets the admitted section finish. Wolfenstein's Intro uses
`FollowPhase`; his `unleash` retains `CompleteSection`. Inugami retains his
separate delayed track entrance. A queued section does not replace the admitted
state or its policy. Joining a declared musical position still needs explicit
product and native support.

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

`d4147e4c` moved recording to the same bounded voice lifecycle and brought
user/environment gain into it. The current native adapter emits changed
gains and active envelopes; it does not retain the old twelve idle channel
volume writes per adaptive frame. This is a source-structure receipt, not
a measured saving or evidence of device behavior. Preserve that boundary.

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

**Owner:** `ambition_audio` director/playback/commands/output.

**Landed implementation (`d4147e4c`):** the common executor owns simple and adaptive submissions,
exact-handle retirement, late cancellation, required-group admission, bounded
retry, and recording output. User and environment gain enter this owner.
Native route acceptance remains open. MUS2 has removed split selection and
simple-resume authority; MUS3 still owns prepared timing.

**Remaining:** native route acceptance, with an honest timing guarantee.
Native group cancellation and resource release have explicit device tests.
Keep simple/adaptive entry points as adapters into this one executor.
Compatibility observations stay projections, not additional owners. Native
acceptance is not a prerequisite for the static selection work.

**Removed:** bank-role retirement, independently authoritative fading-bank
state, idle recording-channel controls, and the environment's direct
music-channel writer. Do not restore those owners to repair a selector bug.

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

**Progress:** simulation publication now keeps each source and its canonical
encounter attempt. Bosses and encounters no longer collapse to one source
per room. Adaptive-only sources have no required simple-track placeholder.
The presentation adapter now publishes one occurrence-owned `MusicGoal`.
Tracks, cues, radio, narrative, room and frontend policy use the same goal
and bounded executor. The separate default-start and simple-resume writers
are removed. A missing projection retains the goal; explicit silence retires it.
An ending names its admitted entry and claim, pins its cue generation, and
uses the latest successor goal. A new attempt or radio can interrupt it.
Missing ending media has a finite wait and a named terminal result.
Release publication now preserves each transition's attempt and cause.
GGRS defers only the presentation projection. The hosted host uses its live
confirmation boundary and retracts only the replayed room's pending releases.
Ordinary encounter messages remain available to simulation readers.

**Guard:** focused native recording tests cover admission delay, superseded
entrances, fresh attempts, shared continuity, stale dispositions, held
continuations, ending interruption, missing media and retained generations.
Selection tests cover local listeners, mixed choices, room overrides,
duplicate authored ids in different rooms and direct destination selection.
Plugin tests cover frontend continuity and direct startup readiness. Native
route and audible acceptance remain open; these tests do not prove speaker output.

Publish occurrence candidates with one priority law. Lower simple choices
to the one-source program and executor. Resolve destination selection and
old-owner release together. Keep the outgoing ending separate from the
latest successor goal, retaining its transition-stamped cause through the
host's settled presentation-effect boundary. Explicitly choose continuation or
fresh entry at the selector/executor boundary. Apply fight overrides before
arbitration.

**Remove:** adaptive entity/catalog rescans as a second selector; archetype
and authored-string ownership reconstruction; normal ownerless `StopNow`;
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
Complete attempt A, Reset and Start B before presentation reads: the retained
reason names A and cannot clear B. An abandoned predicted completion cannot
create a victory ending; establish that at each supported host's delivery
cut, preserving another room's pending facts during room replay.
A never-admitted completed source produces none.
Exercise the four continuation/entry cases in Contract 1 at the existing
command boundary, without requiring another native music executor.

### MUS3 — prepare entrances and plan transitions on the correct clock

**Owners:** native transport, existing room/residency demand, prepared game
scene policy; MA1 supplies producer facts.

**Producer progress:** all compiler frontends now expose their clock through
`CompiledScore.clock`. Source inspection and fresh render publication export
body ends, frame origins, bar exits, gain bases and retained material.

**Native progress:** published programs now link those facts through one
accepted library snapshot. The director queries the current section's
constant or variable clock. Constant grids retain the exported boundary
count. Hobford, Dinosaur, Galvanic and Tritone now consume those facts.
The director now uses the matching occurrence's file cursor for intros,
loop exits and endings. Boundary plans retain that occurrence and selected
exit. An undecoded target that misses its exit waits for another exit. A
changed request discards its old plan. The fixed minimum-delay clamp and
frame-nearest intro estimate are removed. Prepared section ends retain `f64`
seconds from published frame counts. Recording output models the same file-cursor wrap.
Verification: focused director and executor tests cover host-clock jumps,
paused voices, missed decode exits, withdrawn plans and exact boundaries.

Phase bindings now declare whether a one-shot follows the scene phase or
finishes its section. Native requests retain the state and policy with their
voices. The admitted group projects that state back to the director. A second
phase change can cancel a submitted section without losing the running section.
Selection requests the opening and one immediate continuation. Each continuation
then requests its own successor. It does not request distant sections.
Verification covers delayed opening decode, partial entrances, chained sections,
phase projection, delayed simple tracks and the authored Wolfenstein binding.

Incoming sessions now retain their prepared tracks and cue definitions before
publication. Their session roots retain that same catalog. Local room preparation
loads the ambient opening, known boss and wave entrances, and immediate
continuations for incoming, live, selected neighboring and transition-target
rooms. One source-path cache retains their handles across activation and releases
its departed-room sources. The neighbor choice comes from the existing bounded
room preparation policy. Startup waits for the selected ambient opening, including
all required adaptive layers. Anticipated fights grant no playback claim.
Verification covers real WAV decode, frozen catalog selection, handle reuse
and retirement. The rendered host decodes Wolfenstein's opening and first
combat section while he is dormant and has no music claim.

Native track and section caches now retain only the last director update's
source demand. An outgoing playback group retains its exact source handles
through retirement. Explicit preloads return a handle for their preparation
or preview owner to retain. Direct startup holds its source in the startup
manifest and releases that manifest when the cover retires. Verification
checks cache retirement, source survival during fades, final Bevy asset removal,
and preload-to-playback ownership.

`8dc1ee09f` includes adaptive cues in provider silence reports. Playback
diagnostics read decode readiness by the prepared source path, including sources
held by room preparation or explicit preloads. Native cache membership does not
prove whether another owner holds decoded media. Guard: 136 audio tests and
49 provider tests, including real preparation and ECS observations.

`afeb52f24` adds prepared exit rules, transition fades, return overlap, ending
media timeout and exhausted one-shot outcomes. Whole-asset sections use decoded
file ends without a score clock. Plans retain the source occurrence and target
revision. Admitted voices retain source frame counts through asset reload.
Guard: 146 audio tests cover named exits across loop wraps, delayed targets,
exhausted sections, ending handoff and layer alignment before resume.

`1c9f96ca3` adds native scheduling for prepared section and ending exits.
Plans use elapsed source transport across loop passes. A missed admission
deadline selects another allowed exit; it does not move the chosen boundary.
Guard: 149 audio tests and an explicit native loop-exit test with a host stall.
The baseline keeps the old section; the fix switches to the target tones.
PCM inspection finds no silent 10 ms windows across that exit.

Ready one-shot continuations also use native scheduling. Immediate exits and
outputs without a native clock retain immediate or frame-driven admission.
Stronger timing requests fail during preparation. Scene deadlines, pickups,
independent tails and bridge overlays stay open.

Keep catalog, decode, admission, and start facts distinct. Add the
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

**Producer progress:** `cue playback` inspects V1–V3 without synthesis. Source
validation checks typed exits and event form ownership. Encoded products
retain their measured frames and render identity through publication.

**Native progress:** content compilation lowers `PreparedMusicCues`. Direct
registration and reload accept that same type. Preparation rejects duplicate
IDs and slots, invalid numbers, empty playable sections, missing references,
and audible layers without sources. Published product sets also check media
identity, alignment, and gain basis. It resolves source paths and total gain
arrays once. The director retains that prepared cue with its generation.
The first four customers now use published descriptions. Compilation and
reload retain the host-selected library snapshot through publication.

`afeb52f24` adds strict whole-asset recipes and partial policy overrides.
`--inspect-music` reports prepared clocks, exits, gains, source products, voice
requirements and supported timing. Guard: 146 audio tests, 38 intent-adapter
tests and 3 content-CLI tests. Actual content inspection covers 12 shipped cues
and 150 tracks. Strong timing requests are rejected; pickup, independent-tail
and bridge-overlay recipes remain open.

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

The original review used static reads and the supplied log. Implementation
now has native recording, goal selection, decoder preparation and plugin checks.
The shipped host's multi-provider music route check passes with this checkout's
generated assets. The rendered host also prepares Wolfenstein's entrance while
he is dormant. These checks do not establish device timing or audible continuity.

`5ca72c113` adds an opt-in native executor test and bounded Linux ALSA capture.
The locked Bevy/Kira backend produces the expected PCM after cold decode and
two fresh cue handoffs, then silence after an explicit stop. Guard: 146 default
audio tests and the separate native test. Spectrum inspection confirms the
section tones. The baseline capture exposed 20–30 ms of silence at file-end joins.

`781520aba` introduced native scheduling for ready one-shot continuations.
`9ccde6c6f` replaces scheduled instance stops with native group exit envelopes.
Entry fades no longer freeze when an exit is prepared, and withdrawal does not
wait for host acknowledgement. The executor holds sample zero through admission. The
[pinned adapter patch](../../../third_party/bevy_kira_audio/PATCHES.md) exposes
the existing manager, native instance wrapper and scheduled resume. Guard: 148 audio tests,
38 intent-adapter tests, 35 workspace policy tests and three explicit native tests.
PCM checks find no silent 10 ms windows across the prepared joins, including a
one-second host stall and a source tail. Cancellation followed immediately by a
host stall preserves the old audio past its withdrawn exit. Source replacement
before admission cannot change the submitted frames. Silence releases all native
instances and group tracks. Mixer resolution is not a sample-accuracy guarantee.
`dab9fbbac` retains completion facts when a scheduled short section starts and
finishes between host observations. The director advances its state once. Guard:
a native 5 ms continuation across a host stall completes without duplicate plays.
`1c9f96ca3` schedules prepared loop and ending exits on the native clock.
`047f1134a` adds the opt-in Castle Godwin native route test. The visible host
prepares Wolfenstein while he is dormant, plays Awaken and March, crosses to
the great hall, and reenters. A withheld intro cannot delay Phase1 or start
after March. A committed death restore permits a fresh entrance. Both exits
select Tritone without an intermediate military track. Guard: two native
software captures. The final PCM correlates with the selected Tritone layers.
Native ending-customer acceptance and scene pause/scale policy remain open.
No physical-speaker or listening acceptance was performed.
The current recording path consumes the executor's actual voice commands;
it does not execute Kira drains or establish speaker-audible output. The
historical mode-only bank tests did not establish that old command ordering.

Implementing agents should prove the few ownership, arbitration, and clock
properties above at their native seams. Use one coherent command consumer,
not a parallel fake director. Do not add permanent source-name tests,
migration matrices, or a broad audio-test campaign.

Further native acceptance must let an authored ending finish and observe its
successor. Wolfenstein has no authored victory ending. Observe admission and
transport timing separately from speaker-audible output. The older
death/respawn gap did not recur in the new route test; this does not establish
its historical cause.

### Source entry points for an implementing agent

Paths and symbols are the current owners. Historical bank diagnosis belongs
to `00bf92c2`; it is not a description of these current files.

| Question | Source |
| --- | --- |
| Exact playback ownership, retry and entry equivalence | [playback](../../../crates/ambition_audio/src/music/playback.rs): `MusicPlayback`, `MusicPlaybackRequest::can_continue`; [adaptive director](../../../crates/ambition_audio/src/music/director/adaptive.rs): `drive_adaptive_cue_state` |
| Selection adapters and output interpretation | [director](../../../crates/ambition_audio/src/music/director/mod.rs); [diagnostics](../../../crates/ambition_audio/src/music/diagnostics.rs); [state](../../../crates/ambition_audio/src/music/state.rs) |
| Device/recording commands | [commands](../../../crates/ambition_audio/src/music/commands.rs): `MusicPlaybackCommands`; [playback](../../../crates/ambition_audio/src/music/playback.rs): `MusicVoiceOutput`; [output](../../../crates/ambition_audio/src/output.rs); [instance control](../../../crates/ambition_audio/src/spatial.rs) |
| Mix authority and legacy mastering | [environment mix](../../../crates/ambition_platformer2d_audio/src/environment.rs); [user mix](../../../crates/ambition_platformer2d_audio/src/music/mod.rs); [mastering policy](../../../tools/ambition_music_renderer/ambition_music_renderer/processing/mastering.py) |
| Current candidates and unified goal selection | [intent](../../../crates/ambition_platformer2d_audio/src/music/intent.rs); [claims](../../../crates/ambition_encounter/src/music.rs): `publish_occurrences`; [boss publication](../../../crates/ambition_boss_encounter/src/music.rs); [wave publication](../../../crates/ambition_encounter_features/src/music.rs) |
| Transition reason, attempt and presentation delivery | [lifecycle](../../../crates/ambition_encounter/src/lifecycle.rs): `reduce_encounter_lifecycles`; [events](../../../crates/ambition_encounter/src/events.rs): `EncounterEventMsg`; [external effects](../../../crates/ambition_platformer2d_runtime/src/external_effects.rs): `ExternalEffectJournal` |
| Actual game policy | [cue catalog](../../../game/ambition_content/assets/audio/music_cues.ron), 315–355, 403–430, 498–504, 572–585; [Wolfenstein](../../../game/ambition_content/assets/data/boss_encounters/wolfenstein_boss.ron); [Inugami](../../../game/ambition_content/assets/data/boss_encounters/wolfenstein_inugami.ron); [conductor](../../../game/ambition_content_modules/src/wolfenstein.rs), 724–742 and 1547–1551 |
| Readiness versus catalog registration | [cue assets](../../../crates/ambition_audio/src/music/catalog.rs): `LoadedMusicCueAssets`; [provider lifecycle](../../../crates/ambition_platformer2d_provider/src/lifecycle.rs), 504–575 and 1207–1224; [room assets](../../../crates/ambition_platformer2d/src/world_flow/room_transition_assets.rs), 39–93 and 1432 onward |
| Native preparation paths | [cue preparation](../../../crates/ambition_audio/src/cue_program.rs); [media link](../../../crates/ambition_audio/src/music_media/mod.rs); [clock](../../../crates/ambition_audio/src/music_media/clock.rs); [authored data](../../../crates/ambition_audio/src/cue_spec.rs); [content schema](../../../crates/ambition_audio/src/content_schema.rs); [catalog registration](../../../crates/ambition_audio/src/music/catalog.rs) |
| Compiler/frontends | [compile](../../../tools/ambition_music_renderer/ambition_music_renderer/musicir/compile.py); [V1 projection](../../../tools/ambition_music_renderer/ambition_music_renderer/render/score_layers.py), 1414–1528; [exact clock](../../../tools/ambition_music_renderer/ambition_music_renderer/render/exact_score.py); [V3 validation](../../../tools/ambition_music_renderer/ambition_music_renderer/validation/v3.py), 121–124 and 1052–1073 |
| Product construction and identity | [playback layers](../../../tools/ambition_music_renderer/ambition_music_renderer/render/playback_layers.py), 81–315; [dependencies](../../../tools/ambition_music_renderer/ambition_music_renderer/render/dependencies.py), 423–450; [publication](../../../tools/ambition_music_renderer/ambition_music_renderer/cli.py), 413–503 |

Keep [music diagnostics](../../systems/music-playback-diagnostics.md) as the
current observation contract until implementation changes it. Update that
system document with the actual lifecycle when the corresponding slice lands.
