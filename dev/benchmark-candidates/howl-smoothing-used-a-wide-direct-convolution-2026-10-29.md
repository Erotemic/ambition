# Use a linear-time mean for wide smoothing windows

Measured on 2026-10-29 in the SFX renderer.

The howl voice smooths random control values with a centered box window.
At 48 kHz and a 1.5 Hz drift rate, that window has 16,000 samples. Direct
convolution repeats the full window for every audio sample. Three five-second
cues were still running after more than six minutes and held up bank generation.

`scipy.ndimage.uniform_filter1d` computes this mean in linear time. Constant
zero padding preserves the boundary rule. Odd, even, and short windows match
the direct reference within `1e-12`. The spectral and deterministic-render
tests also pass. The three complete cue renders take 1.13 seconds in total.

Use this operation for box windows. A non-uniform kernel has different
semantics and needs its own measured implementation. Do not replace such a
kernel with a box to make it fast.

Owner: `ambition_sfx_renderer/backends/howl_voice.py` in the SFX tool.
Fix: SFX commit `b0f6669`.
