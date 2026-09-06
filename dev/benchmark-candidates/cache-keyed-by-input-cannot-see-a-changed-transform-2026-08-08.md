# A cache keyed by the INPUT cannot see a change to the TRANSFORM

**Tags:** `tooling-invariant`, `measurement`, `caching`, `false-green`,
`agent-verification`, `procedural-audio`

## The shape

An instrument measures derived artifacts: it takes a *description* of a thing,
renders it, and measures the render. Rendering is slow, so results are cached.
The cache key is the obvious one — the description:

```python
def spec_cache_key(spec):
    return f'v{METRICS_VERSION}:spec:{spec.owner}|{spec.key()}'
```

That key is correct for the question "did the authored content change?" and
**silently wrong for the question the instrument is usually re-run to answer:
did the thing that turns content into output change?**

⛔ **The failure is fully invisible and reads as a success.** The run prints
`782 sounds measured in 3.6s (0 fresh, 782 cached)`, rewrites its report, and
every number in it describes the *previous* renderer. There is no warning,
because from the cache's point of view nothing happened.

## The instance (2026-08-08)

`scripts/audio_levels.py` carries a Python port of
`ambition_audio::render::audio_source_from_sfx_spec` so procedural SFX — which
exist nowhere on disk — can be measured on the same axis as packed clips. Ledger
row D38 changed that synthesizer: `SfxSpec::volume` stopped meaning peak
amplitude and started meaning loudness, which moves **every procedural cue by
3–11 dB**.

The verification run, whose entire purpose was to show that movement:

```
782 sounds measured in 3.6s (0 fresh, 782 cached)
```

and the regenerated report was byte-identical in every procedural row. The
specs had not changed. Only the function had.

⚠ **the spec key was itself a considered fix**, and a good one — its docstring
records that keying on the rendered bytes made every row miss, because
libsndfile stamps a float WAV's `PEAK` chunk with the creation time. Moving from
the output to the input cured a cache that never hit and created a cache that
never misses. **Both failures are silent and they are each other's mirror.**

## The rule

⭐ **A derived measurement's cache key must cover every input to the
derivation** — and the code that performs it is one of those inputs.

Concretely, three shapes work:

1. **Version the transform** and put the version in the key. Cheap, manual,
   and needs a comment that says the transform counts, because the next person
   will read `METRICS_VERSION` as "the metric definition" and change the
   renderer without touching it.
2. **Hash the transform's source** into the key. Automatic; over-invalidates on
   a comment edit, which for a 44-second sweep is free.
3. **Key on the output** and make the output deterministic first (here: strip
   the timestamped `PEAK` chunk).

What does not work is choosing between "key on the input" and "key on the
output" as though they were the same axis. They cover different changes.

## The tell, and how to check in five seconds

⭐ **After a change that must move the numbers, the cache-hit line is the first
thing to read** — before the report, before the verdict. `0 fresh, N cached` on
a run that was supposed to re-measure is the whole bug, printed.

```sh
python3 scripts/audio_levels.py | tail -3     # "(N fresh, 0 cached)" or it lied
```

Generalised: **an instrument that reports no work done after you changed the
work is not fast, it is stale.** Same family as
"an implausible number is a broken instrument", but harder, because the number
here is entirely plausible — it is last week's correct answer.

## The question a benchmark should ask

Give the model `spec_cache_key`, the `METRICS_VERSION` comment as it read
before the fix ("Bump when the ffmpeg filter chain or a derived metric
changes"), and a task that edits the synthesizer. Ask what the next run of the
instrument reports and whether it can be trusted.

A passing answer names the staleness **before** running anything, and fixes it
by bumping the version *and* correcting the comment — the comment is what
misled, since a synthesizer change is neither "the ffmpeg filter chain" nor "a
derived metric".

## Related

* [`enumerate-one-way-validate-another-2026-08-08.md`](enumerate-one-way-validate-another-2026-08-08.md)
  — the sibling at the POPULATION layer: right check, wrong set of things.
  Here the set is right and the answers are old.
