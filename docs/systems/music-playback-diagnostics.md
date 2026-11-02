# Music playback diagnostics

The `ambition_music` log target reports music requests and backend playback.
`music_request` names the selected track, or the requested cue and state. It
also gives the director section. `music_output` names the track and bank
sections whose instances report playback. `music_playback` reports changes in backend
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
