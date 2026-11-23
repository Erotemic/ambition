# Music playback diagnostics

The `ambition_music` log target reports music requests and backend playback.
`music_request` names the selected track, or the requested cue and state. It
also gives the director section. `music_output` names the track and playback
occurrences whose instances report playback. `music_playback` reports changes in backend
state. The observer reads Kira audio instances, not the director mode or the
cached track name.

`music_gap` is a warning. It starts after 0.5 seconds without expected playback
and repeats every five seconds. It gives the expected music, section, gap
duration, base playback, active layer count, and source load state when known.
`music_gap_end` records when the unexplained interval ends and its duration.
The observer runs outside gameplay gates and uses real time. It samples four
times per second. Durations are estimates at that resolution.

The following states permit silence: no active context outside gameplay, an explicitly silent
provider, zero music volume, the browser gesture gate, the initial presentation
gate, device suspension, and an observed one-shot that has ended. A recording
backend reports `Recording`; it does not claim device playback. Loading after
music is requested, missing content, a lost gameplay context, and a silent backend
are faults to inspect.
The observer does not restart playback or change content to hide a fault.

The common `MusicPlayback` executor owns simple tracks and adaptive groups.
One `MusicGoal` selects tracks and cues. The report records the selected goal
revision and the revision admitted by the current group. A revision can change
without a new voice when compatible playback keeps its position.
It keeps one pending choice, at most two groups, and at most twelve reserved
voices. A submitted voice remains reserved until its terminal result. Slots
do not identify playback. Cancellation and retirement target exact handles.

Each native music group owns one track in the existing Kira manager. The
executor wraps each accepted native handle as a Bevy audio instance. It submits
music paused and at zero gain, then schedules a common resume after all required
voices report paused. A canceled submission cannot resume. Partial
admission cancels the complete group. Admission absence, a five-second
admission timeout, and an unexpected loop stop permit one retry for the current
choice. An unresolved canceled voice keeps its reservation after timeout.
A completed one-shot does not restart from a held entry. A fresh entry has
its own completion and retry state, even when it uses the same media.

An ending retains its admitted entry, owner and cue generation. The latest
goal selects its successor. A release for an old owner cannot end the new
owner's music. Ending media has a finite readiness wait and a named result.

Music gain has one writer. The executor combines user and environment gain,
source gain, and the occurrence's transition envelope. Recording consumes the
same lifecycle and counts starts, resumes, gain changes, and stops. It keeps
bounded voice state and does not write to Kira command queues.

The director uses the current occurrence's native file cursor and shared Kira
clock. Ready continuations and prepared exits can run during a host stall.
A missed admission deadline selects another allowed exit. Completion receipts
also cover a whole ending that plays between host observations. The supported
guarantee remains frame-driven; sample-accurate and phase-preserving requests
fail preparation. Instance state does not prove speaker output.

Music continues on its own clock during scene pause and time-scale changes.
Phase bindings choose whether an admitted one-shot finishes or follows the next
ready phase. A held phase does not replay a completed entrance. Music readiness
and completion do not set gameplay deadlines.

The director requests cue sources together. It starts each section after all
layers of that section are decoded. Later sections do not block the opening.
A playing loop continues while the requested next section loads. A one-shot
can still end before its next section is ready; the observer reports that gap.
The native inspector states the supported entry and release operations. An
ending wait mix changes the current section's gains without starting another
section or changing its successor.

`MusicPlaybackDiagnostics` holds the current report, the current and longest
gap durations, and the number of gaps. It is presentation state, outside the
rollback timeline. Logs provide the history across deaths and room changes.

For a local run, retain the game log and search it with:

```bash
rg 'music_request|music_output|music_playback|music_gap' GAME_LOG
```

Kira playback is not a measurement of speaker output. A playing instance can
contain silent source samples. These diagnostics do not detect a silent mixer,
a disconnected speaker, or browser buffers that play at the wrong time.

The workspace uses published CPAL 0.18.2. The local Web Audio scheduling patch
was removed at the maintainer's request. The browser buffer defect after a long
main-thread stall can recur. That patch did not change native playback.
