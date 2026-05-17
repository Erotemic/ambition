# Music transition notes

## Goblin intro -> wave1 handoff

The goblin encounter is authored as an adaptive cue. The runtime owns the
transition between sections, so generated section files do **not** need baked
multi-second fade-outs at their ends. In fact, baked fades can make the handoff
sound like two different tracks: the intro disappears, then the loop fades in.

Preferred asset shape:

- Intro / outro: may be one-shot phrases and can end decisively on a musical
  boundary.
- Loop sections: should be loopable and bar-aligned.
- All sections: may use a tiny de-click tail, but should avoid long baked fades
  unless the fade is part of the composition.
- Runtime: owns beat-aligned crossfade, de-click, and gain smoothing.

The current director settings make intro -> wave1 much more immediate:

- `INTRO_TO_LOOP_CROSSFADE_SECONDS = 0.35`
- `STEM_GAIN_BLEND_SECONDS = 0.18`
- Intro-to-loop loop sections start their new bank at target gain instead of
  fading up from silence.

The last bullet matters most for the goblin intro -> wave1 case. The old bank is
already fading out; if the new loop bank also fades in from zero, players hear a
brief hole and become aware that the engine switched files. For one-shot intro
into loop, the loop should arrive at its intended level immediately and the old
intro bank should get out of the way quickly. Longer transitions are still used
for loop-to-loop section changes and outro returns, where a musical blend is
desirable.

## Return to room music after encounter

The room/radio track should start under the adaptive outro tail before the outro
is fully finished. `DEFAULT_RETURN_OVERLAP_SECONDS = 2.25` keeps the return from
feeling like encounter music faded out, silence happened, and a new track faded
in.
