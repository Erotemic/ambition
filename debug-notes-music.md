````markdown
# Debug Notes: First Goblin Music Transition

## Context

The first goblin encounter uses an adaptive music cue:

```text
first_goblin_tune_v2
````

The runtime currently plays **full-mix section files** for the cue:

```text
intro.full.ogg
wave1.full.ogg
wave2.full.ogg
wave3.full.ogg
recap_loop.full.ogg
outro.full.ogg
```

The relevant installed game assets live here:

```text
crates/ambition_sandbox/assets/audio/music/generated/first_goblin_tune_v2/
```

The generated/staging assets live here:

```text
tools/ambition_music_renderer/generated/first_goblin_tune_v2/
```

The active score spec is:

```text
tools/ambition_music_renderer/scores/active/first_goblin_tune_v2.music.yaml
```

The runtime cue spec is:

```text
crates/ambition_sandbox/src/music/first_goblin.rs
```

## Current symptom

The transition from `intro` to `wave1` is still noticeably audible. It is better after runtime changes, but it still feels like:

```text
track A ends -> track B begins
```

instead of one continuous musical idea.

Specific observations:

* `wave3` comes in much more naturally.
* `intro -> wave1` still has a perceived level/spectrum shift.
* The intro was too loud at one point.
* There is too much white noise / airy background noise.
* Runtime logs show the handoff is happening promptly, so this is probably not file loading latency.

Example useful log shape:

```text
start_adaptive_state cue=first_goblin_tune_v2 state=intro section=intro ...
queue_music_state cue=first_goblin_tune_v2 state=wave1 section=wave1 ...
start_adaptive_state cue=first_goblin_tune_v2 state=wave1 section=wave1 ... gain_start=target
started_music_sources cue=first_goblin_tune_v2 state=wave1 ...
```

If `gain_start=target` appears for `wave1`, then the runtime is no longer fading wave1 up from silence. Remaining problems are probably in the rendered assets or score arrangement.

## What already improved the transition

Runtime-side changes helped somewhat:

```text
STEM_GAIN_BLEND_SECONDS reduced
INTRO_TO_LOOP_CROSSFADE_SECONDS reduced
intro -> loop transition starts new bank at target gain
```

The key runtime improvement was adding the idea:

```text
intro -> first loop section: start target gain immediately
loop -> loop: normal smoother transition
```

This reduced the feeling that wave1 fades in late.

## Why wave3 sounds better than wave1

`wave2 -> wave3` is an easier transition class:

```text
loop component -> loop component
```

Both sections are loopable fight music, similar density, similar mix posture, and the listener expects intensification.

`intro -> wave1` is harder:

```text
short non-loop one-shot -> first loop component
```

It exposes every mismatch:

```text
- different density
- different perceived loudness
- different reverb/noise floor
- different phrase shape
- possibly different mastering behavior
- intro may resolve instead of leading into wave1
- wave1 may start too thin on beat 1
```

The runtime can hide small seams. It cannot make two separately rendered sections feel like one phrase if the composition/mastering creates a boundary.

## Most likely diagnosis

The remaining issue is probably generation/mastering/arrangement, not OGG latency.

Likely causes:

1. **Intro and wave1 are normalized/mastered independently.**
   Peak normalization can make sparse intro material feel too loud relative to denser loop material.

2. **Intro has a different ambience/noise floor.**
   Extra reverb, damping, or high-frequency tail can create audible white noise before the wave1 handoff.

3. **Intro ends like a completed phrase instead of a pickup.**
   If the intro musically “finishes,” wave1 will always feel like a new track.

4. **Wave1 starts too sparsely.**
   If wave1’s first bar is quieter/thinner than its average loop body, the transition feels like a drop even when gain is correct.

5. **Wave3 masks seams better.**
   More percussion/brass/density hides boundaries. Wave1 is leaner, so the seam is exposed.

## Debug process to resume later

### 1. Regenerate only this cue

Use the single-cue generator:

```bash
cd ~/code/ambition
./generate_audio_assets.sh --force
```

Despite the generic name, this currently only renders and installs:

```text
first_goblin_tune_v2
```

It renders to:

```text
tools/ambition_music_renderer/generated/first_goblin_tune_v2/
```

and installs to:

```text
crates/ambition_sandbox/assets/audio/music/generated/first_goblin_tune_v2/
```

### 2. Run the game directly in the encounter room

```bash
cd ~/code/ambition
cargo run -p ambition_sandbox --bin ambition_sandbox --features dev_hot_reload --release -- --start-room goblin_encounter
```

### 3. Capture music logs around the transition

Look for:

```text
start_adaptive_state ... state=intro ...
queue_music_state ... state=wave1 ...
start_adaptive_state ... state=wave1 ... gain_start=target
```

If `gain_start=target` appears, do not keep tuning fade-up-from-zero behavior. The next layer is generated audio quality.

### 4. Audit the generated OGGs

Run:

```bash
cd ~/code/ambition
python tools/ambition_music_renderer/audit_cue_balance.py \
  tools/ambition_music_renderer/generated/first_goblin_tune_v2
```

Compare:

```text
intro.full.ogg
wave1.full.ogg
wave2.full.ogg
wave3.full.ogg
recap_loop.full.ogg
outro.full.ogg
```

Specifically check:

```text
peak_db
rms_db
duration
```

Questions to answer:

```text
- Is intro peak high but RMS low?
- Is wave1 much lower RMS than intro?
- Is wave1 beat 1 quieter than the rest of wave1?
- Does intro have a long noisy tail?
- Is wave3 simply much denser/louder than wave1?
```

The current audit tool gives basic peak/RMS, not LUFS/noise floor. A future improvement would be adding:

```text
integrated LUFS
short-term LUFS
tail RMS / noise floor
first-bar RMS
last-bar RMS
```

### 5. Listen to files directly

Listen outside the game to isolate runtime from generation:

```bash
cd ~/code/ambition
xdg-open tools/ambition_music_renderer/generated/first_goblin_tune_v2/adaptive/intro/intro.full.ogg
xdg-open tools/ambition_music_renderer/generated/first_goblin_tune_v2/adaptive/wave1/wave1.full.ogg
```

Or use a player that can queue them back-to-back.

Important question:

```text
Do intro.full.ogg and wave1.full.ogg already sound mismatched before the game touches them?
```

If yes, the fix is in the YAML/generator, not the runtime director.

## Score-level things to try

In:

```text
tools/ambition_music_renderer/scores/active/first_goblin_tune_v2.music.yaml
```

Try:

### Reduce intro loudness

The intro should be a warning / lead-in, not the loudest part.

Possible changes:

```yaml
sections:
  - id: intro
    postprocess:
      gain_db: negative value
      target_peak_db: lower / more headroom
```

Also reduce intro-specific material velocities if needed.

### Reduce white noise / airy tail

Focus on:

```yaml
postprocess:
  reverb_wet
  reverb_decay_seconds
  reverb_damping_hz
  stereo_width
  limiter_drive
  compressor_makeup_db
```

And group-specific values:

```yaml
group_postprocess:
  winds:
  percussion:
  choir_pad:
  strings:
```

Likely direction:

```text
less reverb wet
shorter decay
darker damping / lower high-frequency shelf
less limiter drive
less compressor makeup
darker winds/percussion
quieter hats/noisy percussion
```

### Make intro lead into wave1

The most important musical fix may be compositional:

```text
intro last bar should be a pickup into wave1
wave1 first bar should share at least one obvious element from intro
avoid making intro cadence feel final
avoid a long reverb/noise tail into a dry wave1
```

Possible arrangement changes:

```text
- add a low pulse or motif in the last intro bar that continues in wave1
- make wave1 beat 1 include the same low/string/wind identity from intro
- avoid a full stop before wave1 starts
- ensure intro does not resolve too strongly
```

### Make wave1’s first bar stronger

If wave1’s loop body is okay but the first bar feels weak:

```text
- add a subtle downbeat anchor
- add low strings/contrabass root on beat 1
- bring in one percussion hit or pulse earlier
- avoid making the groove take several beats to become audible
```

## Runtime things to avoid over-tuning for now

Do not keep changing these unless logs show a runtime problem:

```text
STEM_GAIN_BLEND_SECONDS
INTRO_TO_LOOP_CROSSFADE_SECONDS
gain_start=target
AudioTween timings
```

Those helped, but the remaining issue sounds like asset content.

Runtime should own:

```text
- scheduling
- section selection
- crossfade / de-click
- preventing two incompatible music sources from playing
```

Generator/score should own:

```text
- section loudness consistency
- spectral consistency
- white-noise/reverb floor
- phrase continuity
- loop/cut friendliness
```

## Important principle

For adaptive music, generated sections should not rely on long baked fade-outs to hide seams.

Good adaptive section assets should be:

```text
bar-aligned
loop/cut-friendly
level-matched by perceived loudness
spectrally compatible
mostly free of long noisy tails
allowed to end abruptly if the runtime owns the crossfade/de-click
```

Long baked fades can fight the runtime crossfade and make transitions feel like two tracks.

## Current best hypothesis

The remaining intro→wave1 break is probably caused by:

```text
intro is too loud / too reverby / too noisy
wave1 starts with a different density and noise floor
the intro phrase resolves instead of handing off
```

Wave3 sounds good because it is a denser loop-to-loop intensification and naturally masks the seam.

## Next concrete TODO

When returning to this:

1. Run the balance audit.
2. Listen to `intro.full.ogg` and `wave1.full.ogg` directly.
3. Lower/dry/darken intro.
4. Strengthen wave1 beat 1.
5. Make intro’s last bar a pickup into wave1.
6. Regenerate only this cue.
7. Test in `goblin_encounter`.

Commands:

```bash
cd ~/code/ambition

python tools/ambition_music_renderer/audit_cue_balance.py \
  tools/ambition_music_renderer/generated/first_goblin_tune_v2

./generate_audio_assets.sh --force

cargo run -p ambition_sandbox --bin ambition_sandbox --features dev_hot_reload --release -- --start-room goblin_encounter
```

```
```

