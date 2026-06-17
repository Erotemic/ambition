# Procedural audio benchmark candidates

Questions distilled from real Ambition music/SFX renderer maintenance mistakes.
The renderer pipeline is YAML music spec → motif/layer expansion → fluidsynth
synthesis → DSP postprocess → OGG, so most questions test how an agent
reasons across that whole chain rather than within a single layer.

---

## 2026-05-10: Track down a "squeaky high-pitch sequence" in a procedural music cue without listening

Tags: `procedural-audio`, `state-leak`, `cross-section-cc`, `timbre-vs-pitch`,
`yaml-spec-debugging`

**Status: OPEN — no confirmed fix yet.** Capture the question with the answer
pending so the actual fix can be filled in once the user verifies.

### Setup

You are given a YAML music spec at:

```text
tools/ambition_music_renderer/scores/active/how_to_kill_a_mockingbird.music.yaml
```

It is a 48-bar, 150 BPM, F# minor glam-metal boss cue. Sections (in order) are
`boss_entry` (4 bars) → `main_assault` (12) → `lift_bridge` (8) →
`chorus_fight` (12) → `shred_break` (8) → `boss_tag` (4). The renderer
(`tools/ambition_music_renderer/ambition_music_renderer/render_isolated.py`)
expands motifs into MIDI, runs each instrument group through fluidsynth in an
isolated subprocess, applies a DSP postprocess chain (compressor / reverb /
limiter / normalize), and emits a mastered OGG.

Relevant motif (F# minor):

```yaml
- id: brass_boss_hook
  root: F#4
  intervals: [0, 7, 5, 7, 10, 7, 3, 5, 7, 5, 2, 3, 5, 7, 12, 10]
  rhythm:    [0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5]
```

This motif is played by `neon_brass_lead` (GM `synth_brass_1`, vol 104, chorus
14, modulation 6) via the `hook_voice` template in `chorus_fight`. A
`saw_shadow` template plays the same motif on `razor_saw_answer` with
`transpose: -12`.

The previous section (`lift_bridge`) contains:

```yaml
- kind: automation
  group: lead
  automation:
    - {group: lead, cc: expression, from: 84, to: 116, curve: smooth, points: 12}
    - {group: guitars, cc: expression, from: 88, to: 112, curve: smooth, points: 12}
```

The user reports: **"There is a squeaky high-pitch sequence at :40 seconds.
Fix the YAML."** They cannot see which layer is responsible — only that it
sounds piercing in that window. You do not have the rendered audio in your
working environment; you can only read the YAML, the renderer source, and the
GM soundfont's documented program characteristics.

Earlier passes have already:

- dropped `brass_boss_hook` and `brass_boss_answer` roots from F#5 → F#4
  (capping the motif at F#5 instead of F#6);
- capped `final_boss_call` intervals to remove F#6 / E6 peaks;
- lowered `bell_flash` volume and removed the +12 octave transpose on the
  sparkle layer;
- pulled `saw_shadow` velocity in `chorus_fight` to 56;
- removed `sparkle_downbeat` from `chorus_fight`;
- lowered `reverb_damping_hz` from 7200 → 6400 globally.

The user still reports a squeaky sequence at :40s.

### Question

Without re-listening to the rendered audio, enumerate **every** mechanism
through which this YAML + renderer combination could produce piercing
high-frequency content in the 38–43 second window, and rank them by how
plausible they are as the dominant remaining contributor. For each, name the
exact YAML edit (or renderer change) you would propose to test the
hypothesis, and the static check you would run before re-rendering to confirm
the change is the only thing that moved.

The answer must consider, at minimum, all of:

1. MIDI pitch height (motif root + intervals + per-layer transpose +
   `instrument_octave_offsets`).
2. CC state inherited across section boundaries (does
   `lift_bridge`'s `automation` block reset at section end? See
   `render_layer_*` and `apply_automation` in `musicir_renderer.py`).
3. Soundfont program brightness independent of pitch (e.g., GM
   `synth_brass_1` vs `synth_brass_2` partial structure at F#5).
4. Chorus / modulation depth interacting with bright timbres (chorus on a
   detuned synth lead at high register produces beat frequencies in the
   2–6 kHz band).
5. Layer doublings whose harmonic content extends well above their
   fundamental (saw waves, tubular bells — tubular bells at F#4 still radiate
   strong partials at F#5, C#6, F#6).
6. Postprocess chain interactions (compressor knee, reverb damping,
   limiter drive — does the master chain emphasize any band?).
7. Per-section `postprocess` overrides (do they apply to the full mastered
   preview, or only per-section adaptive OGGs? See
   `render_isolated.py:300-316`).
8. Renderer behaviors not visible in the YAML (clamp behavior in
   `add_note`, pitch-bend interactions, fluidsynth tail handling).

### Expected answer

**Unverified as of 2026-05-10**, but a spectral attribution pass against the
per-group `.npy` debug stems narrowed the candidate set substantially.
The first agent's pre-spectral hypothesis (motif F#5 stabs on
`neon_brass_lead`) was **wrong by an order of magnitude in absolute energy**
and is preserved here as the canonical bait.

Spectral attribution (per-group 3–6 kHz energy at :40.0 s, from
`scratch_stems/*.npy` after `render_isolated --keep-debug-stems`):

| group   | fraction of vhigh band at :40.0s |
|---------|----------------------------------|
| guitars | **69.5%** (rhythm_guitars distortion + wide_guitars overdrive) |
| lead    | 14% (brass + saw motifs combined)        |
| brass   | 8%  (hero_brass_stack power-hit stabs)   |
| drums   | 4%  (in this bucket; dominant elsewhere) |

Across the 38–43 s window the band alternates between `guitars` on bar
downbeats and `drums` on hi-hat/cymbal pulses. In the air band (6–12 kHz),
`drums` dominates 60–97 % of every bucket (crash decay + open_hat + china).
The `lead`/`brass` groups only spike briefly (lead 80 % at :39.25 s, the
F#5 motif stab) and contribute far less total band energy than the
distortion guitars and the cymbal/hat layer.

So the leading candidates after measurement (still unverified by listening
test):

1. **`arena_drums_big` cymbal/hat density in `chorus_fight`.** The
   16-per-bar `closed_hat`, `open_hat` on 1.75 / 3.75, `crash` at velocity
   108 every 4 bars, and `china` at velocity 86 every 4 bars beat 2 are the
   dominant air-band content. Cheapest test: swap to `arena_drums` (8
   hats/bar, no china), or drop `closed_hat` velocity 60→48 and remove
   `china`.
2. **`power_hits` ring-through.** `duration_beats: 0.85` lets the
   distortion-guitar + brass chord stab ring almost the full bar, so the
   3–6 kHz harmonic shelf rebuilds on every bar. Cheapest test: shorten
   `duration_beats` to ≈0.40 and / or drop velocity 88→72. `rhythm_guitars`
   uses GM `distortion_guitar`; switching to `overdrive_guitar` may reduce
   harmonic spill.
3. **CC state leak from `lift_bridge` automation.** `apply_automation`
   (`musicir_renderer.py:544`) writes CC values that persist past section
   end; the `lift_bridge` ramps `lead` to CC11=116 and `guitars` to
   CC11=112, neither of which is reset by `chorus_fight`. Lower the `to:`
   upper bound (e.g. 116→96, 112→92) or add a `chorus_fight` automation
   block that resets expression at bar 0. Note this **may** explain why the
   guitars are loud enough in 3–6 kHz to dominate, even if it isn't the
   pitch source.

To be filled in once verified by the user: **which mechanism actually
dominated, and what was the minimal YAML or renderer change that fixed
it?**

### Why this was easy to miss

A natural agent strategy on a "squeaky high pitch" complaint is to enumerate
melodic motifs by maximum MIDI pitch and lower the highest ones. That
strategy succeeds on the obvious squeaks (F#6 stabs, octave-up bells) and
appears to work because the user confirms one round of fixes. But the
remaining problem is not in the motif intervals — it is in **state that
carries across section boundaries** (CC automation), in **timbre that does
not track pitch height** (program brightness, chorus beating), or in
**layers whose perceptual contribution exceeds their fundamental** (saw,
bell). None of those show up if the agent only inspects motif tables. The
trap is mistaking "I capped the motif's max pitch" for "I capped the
high-frequency content."

A second trap: the renderer has a per-section `postprocess` override
mechanism that only applies to the per-section adaptive OGGs (see
`render_isolated.py:300-316`); the **master mix preview the game actually
plays** ignores it. An agent that proposes "add a low-pass to chorus_fight's
postprocess" will appear to fix the problem in the per-section file while
the published `full.ogg` is unchanged.

### References

- Cue YAML at the time of the investigation:
  `tools/ambition_music_renderer/scores/active/how_to_kill_a_mockingbird.music.yaml`
  (untracked in working tree at HEAD `202e264b44056363740873b06035910e2de0be71`).
- Motif → MIDI conversion: `motif_notes` in `musicir_renderer.py:521-541`.
- Layer rendering with octave / transpose: `render_layer_motif`
  in `musicir_renderer.py:680-716`.
- Automation expansion: `apply_automation` in `musicir_renderer.py:544-590`.
- Per-section vs master postprocess split:
  `render_isolated.py:300-316`.
- Game asset destination: `crates/ambition_gameplay_core/assets/audio/music/generated/how_to_kill_a_mockingbird/full.ogg`
  (only the master preview is consumed in-game).

### Validation

A reusable spectral-attribution tool was added during this investigation:

```bash
cd tools/ambition_music_renderer && python3 -m ambition_music_renderer.render_isolated \
  scores/active/how_to_kill_a_mockingbird.music.yaml \
  --outdir generated/how_to_kill_a_mockingbird \
  --backend pretty-midi --simple-mix --keep-debug-stems --force

python3 tools/ambition_music_renderer/spectral_localize.py \
  tools/ambition_music_renderer/generated/how_to_kill_a_mockingbird \
  --window 38 43 --bucket 0.25
```

`spectral_localize.py` reads every `.npy` under `<outdir>/scratch_stems/`,
buckets band energy through the supplied window, and prints both
window-mean band-fraction tables and per-bucket dominant-group
attributions. Use it as the first step on any "what's making that sound at
T seconds" complaint — the answer derived from the measurement frequently
disagrees with the answer derived from reading the YAML.

A complementary static-only check: dump every MIDI note that lands in the
target window across all layers, with assigned program and active CC
state. CC carryover across section boundaries is the ambiguous bit and the
renderer source (`apply_automation`, `render_layer_motif`) is ground
truth — YAML semantics alone are not enough.
