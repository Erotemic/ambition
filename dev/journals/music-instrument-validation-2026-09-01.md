# Music instrument validation handoff — what actually works

**Session:** 2026-09-01
**Author:** Luna

This is the durable handoff from my real-SFZ investigation for the
`blazingly_fast` cue. The detailed WAVs and JSON are machine-local; this note
records the conclusions that a remote agent can use without access to
`/data/audio-tools`.

## The short version

The renderer can produce audio from every candidate tested, but “rendered” is
not the same as “musically trustworthy.” In the real smoke run, 17/17 probe
rows rendered and 12/17 also passed the stricter validation. The main failure
mode was an SFZ/sample whose metadata suggested one pitch while the raw sample
or rendered result was approximately an octave lower.

For `blazingly_fast`, I selected:

| Role | Alias / library | Decision |
|---|---|---|
| power guitar | `guitar.emily` / Emilyguitar | Use Emily power/chord entry point with keyswitch 33. It rendered and validated reliably. |
| locked bass | `bass.swag` / Swagbass Clean | Use without score transposition. Static regions, raw samples, and rendered probes agreed closely enough to trust it. |
| rock drums | `drums.big_rusty` / Big Rusty `01-full.sfz` | Use the explicit kit mapping. Rendered, validated, and showed strong round-robin variation. |

The resolver evidence for those three roles was clean: no missing aliases and
no out-of-range notes in the final cue. The selected aliases describe what the
renderer knows how to resolve; they do not prove that another machine has the
underlying library installed.

## Candidate findings

| Candidate | Result | Caveat |
|---|---|---|
| Emily basic/chords/wide | Reliable render and pitch validation | Chord entry points depend on keyswitches; the chosen power path uses keyswitch 33. |
| Black & Green normal / twang | Reliable in the tested probes | Staccato rendered, but at least one pitch probe was unreliable; do not treat every articulation as interchangeable. |
| Shiny electric | Reliable render and pitch validation | Requires its startup controls, including CC100/CC107, for the intended sound. |
| Swagbass | Best bass choice in this bakeoff | Reliable pitch across the tested notes; retain the authored notes rather than applying an octave correction. |
| Growlybass | Renders, but not trustworthy for pitched bass | Raw and rendered evidence consistently indicated roughly an octave-low output despite plausible SFZ metadata. |
| Black & Blue / Baby Blue | Renders, but pitch validation fails | Static regions claimed expected key centers, while independent raw-sample checks consistently favored the half-frequency result. Do not silently transpose it. |
| Fashionbass | Renders, but low-register result is uncertain | The lowest tested note was octave-ambiguous; higher notes were more reliable. Treat the low range as unresolved. |
| Pastabass | Renders, but not trustworthy for pitched bass | Independent raw checks favored roughly an octave-low output. |
| Gogodze, Big Rusty, Naked Drums, Muldjord | Usable drum renders | Drum keys are kit-specific. Some probes were skipped because the requested note was outside a kit’s raw range; that is not evidence of a broken renderer. |

## Rules for interpreting these results

1. **Separate renderability from validation.** Non-silent output only proves
   that the backend found and played something. Pitch, controls, range, and
   articulation still need separate checks.
2. **Do not infer pitch from `pitch_keycenter` alone.** For suspicious basses,
   compare SFZ region metadata, raw sample analysis, and a rendered probe. The
   raw check used both harmonic scoring and autocorrelation against the
   expected, half, and double frequencies.
3. **Do not octave-fold or transpose a suspicious sample as a “fix.”** The
   correct status is `actual_pitch_mismatch`, `octave_ambiguous`, or
   `unreliable` until independent evidence justifies a musical correction.
4. **Use the real control state.** Keyswitches and startup CC values are part
   of an instrument’s contract. A default-note smoke test can pass while the
   desired articulation is absent.
5. **Measure repeated strikes by waveform shape, not peak level alone.** The
   eight-strike probes showed round-robin variation for all tested candidates,
   but the amount varied substantially: sampled drums had strong variation,
   while some guitars and basses were more correlated. High correlation is
   not automatically a failure; it is a reason to inspect the SFZ round-robin
   setup and listen to the exported phrase.

The instrument audit code and tests live in
`tools/ambition_music_renderer/ambition_music_renderer/audit/sfz_measurement.py`
and `tools/ambition_music_renderer/tests/test_sfz_measurement.py`. Generated
reports remain disposable by design; this note is the source-controlled
summary of their conclusions.
