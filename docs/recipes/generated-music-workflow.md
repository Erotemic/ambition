# Generated music workflow

This is the current recipe for generated/adaptive music. Older transition labs and balance notes were archived as historical experiments, but the useful debugging workflow is preserved here.

## Current model

- Source specs and renderer code live under `tools/ambition_music_renderer/`.
- Generated outputs are local until explicitly installed or published into runtime assets.
- Runtime playback belongs to the sandbox presentation/audio layer.
- Asset identity and packaging policy should flow through the asset catalog when a cue becomes part of the game.
- `first_goblin_tune_v2` is the current active adaptive-music lab. Do not turn its one-off script shape into the permanent architecture for every encounter without generalizing it.

## Common commands

From the repo root:

```bash
uv run --project ~/code/ambition/tools/ambition_music_renderer python -m ambition_music_renderer --help
uv run --project ~/code/ambition/tools/ambition_music_renderer python -m ambition_music_renderer cue_bundle <cue_id> --backend=pretty-midi --force --zip
uv run --project ~/code/ambition/tools/ambition_music_renderer python -m ambition_music_renderer cue_bundle <cue_id> --backend=pretty-midi --runtime_stem_gain_mode=shared --force --zip
uv run --project ~/code/ambition/tools/ambition_music_renderer python -m ambition_music_renderer cue_bundle <cue_id> --backend=pretty-midi --runtime_stem_gain_mode=shared --zip_report --force
./generate_audio_assets.sh --force
```

Auxiliary tools live under the package modal CLI:

```bash
uv run --project ~/code/ambition/tools/ambition_music_renderer python -m ambition_music_renderer audit --help
uv run --project ~/code/ambition/tools/ambition_music_renderer python -m ambition_music_renderer audit transition --help     # two-file transition seam
uv run --project ~/code/ambition/tools/ambition_music_renderer python -m ambition_music_renderer audit cue_balance --help    # sections within one cue
uv run --project ~/code/ambition/tools/ambition_music_renderer python -m ambition_music_renderer audit levels --check         # inter-cue catalog levels
uv run --project ~/code/ambition/tools/ambition_music_renderer python -m ambition_music_renderer audit arrangement --help    # score-level arrangement preflight
uv run --project ~/code/ambition/tools/ambition_music_renderer python -m ambition_music_renderer audit dissonance --help     # score-level note/layer clash hotspots
```

For one-cue composition/debug handoff, prefer `cue_bundle` first. It wraps rendering, scratch-stem retention, level reports, spectral localization, optional spectrograms, and a shareable bundle manifest around the current renderer without changing runtime publish policy.

Three lower-level audio-analysis tools, three scopes:
- `tools transition_audit` — two specific section files; visual transition-seam plots.
- `tools audit_cue_balance` — every section/stem inside one cue's output dir.
- `tools level_report` — every `<cue>/full.ogg` under the runtime music root; a
  sorted, diff-friendly table (duration, RMS dBFS, true peak dBTP, crest,
  target-RMS delta, optional LUFS) + a spread summary with CLIP/LOUD/QUIET
  flags. Use it to catch inter-cue loudness jumps and clipping across re-renders.
- `tools arrangement_audit` — expanded MusicIR notes before audio render; reports
  group prominence, likely buried stems, low-register density, bass/melody
  collision candidates, and long non-chord tones.
- `tools dissonance_audit` — expanded MusicIR notes before audio render; reports
  bars/beats/layers/groups with strong close seconds, sevenths, tritones, and
  register clusters. Use it when a cue sounds like notes are clashing rather
  than when it has spectral noise.

Use `python -m ambition_music_renderer audit reference_audio <audio> --outdir=<dir>` for broad reference-track surface features. It is useful for loudness/brightness/density targets, but it does not separate stems or recover instrumentation.

Prefer the tool README and CLI help over old recipe fragments when command flags drift.

## Edit protocol

1. Decide whether you are changing source composition, render code, publish/install policy, or runtime playback.
2. Search `dev/journals/` and `dev/benchmark-candidates/` for music director/refactor lessons.
3. Render locally into the tool's generated output path.
4. Audit balance/transitions if a cue set changes.
5. Use `cue_bundle <cue_id> --zip_report` when a cue needs lightweight review, handoff, or spectral/debug evidence. Use a full bundle only when the recipient needs audio.
6. Publish/install only when the generated assets are meant to become runtime inputs.
7. Update `docs/tools/generated-audio-tools.md` and `tools/ambition_music_renderer/README.md` if the workflow changes.


## One-cue debug and handoff bundles

Use this when regenerating a song and collecting useful diagnostics for review:

```bash
uv run --project ~/code/ambition/tools/ambition_music_renderer \
python -m ambition_music_renderer cue_bundle for_emmy_forever_ago \
  --backend=pretty-midi \
  --force \
  --zip
```

The bundle command writes reports and plots under the cue's generated output and
then copies manifest-referenced artifacts into
`tools/ambition_music_renderer/bundles/`. The bundle deliberately ignores stale
preview/adaptive files from older hashes.

Use `--zip_report` for chat/agent upload. Report zips exclude
large audio/scratch binaries while keeping source YAML, manifests, logs, TSV/JSON
reports, `spectral_fingerprint.json`, and JPEG spectrograms. Use full bundles
when the recipient needs to audition OGGs. Add `--publish` only when the cue
should also update the runtime `assets/audio/music/generated/<cue_id>/full.ogg`.
Add `--include_scratch_stems` only for local handoffs because raw NumPy stem
buffers can be large.

Useful report files in a bundle:

- `reports/arrangement_audit_summary.txt` / `.md` — group prominence and score-level preflight issues before audio rendering.
- `reports/dissonance_hotspots_summary.txt` — where note choices/layers clash.
- `reports/dissonance_hotspots.md` — same information in a scan-friendly table.
- `plots/dissonance_timeline.<fmt>` / `plots/dissonance_layer_pairs.<fmt>` — visual hotspot aids.
- `reports/state_mix_report_summary.txt` — why adaptive previews sound similar
  or different and whether states only change loudness.
- `reports/spectral_fingerprint_summary.txt` — which stem groups dominate broad
  frequency bands.
- `reports/stem_amplitude_summary.txt` plus `plots/stem_amplitude_*.<fmt>` —
  direct runtime stem balance / envelope views.
- `reports/mix_diagnostics.txt` — raw stem levels vs mastered full and runtime
  stem gain policy.

Use `--runtime_stem_gain_mode=shared` when checking layered dynamic music. It
applies one shared reference gain to all runtime stems, preserving their balance
while making the exported stem set audible. Shared gain is capped by default; if
reports show capped or very large gain, raise source/layer levels in the score
instead of exporting amplified noise. The default `native` mode preserves
historical raw-stem levels for compatibility.

Fallback rendering is explicit opt-in: use `--backend fallback` only when you
really want that diagnostic backend. Normal authoring/debug defaults should use
`pretty-midi`.

## Diagnosing an audible transition seam

Use this sequence before changing runtime code:

1. **Regenerate only the relevant cue.** For the current goblin lab, `./generate_audio_assets.sh --force` renders and installs `first_goblin_tune_v2`.
2. **Run directly in the encounter room.** Start the sandbox in the room that triggers the cue and reproduce the transition.
3. **Capture runtime logs.** Look for `start_adaptive_state`, `queue_music_state`, `gain_start=target`, and the section/state names.
4. **Audit the OGGs.** Use `tools audit_cue_balance` on the generated cue directory and compare peak/RMS/duration across `intro.full.ogg`, `wave1.full.ogg`, `wave2.full.ogg`, `wave3.full.ogg`, `recap_loop.full.ogg`, and `outro.full.ogg`.
5. **Listen outside the game.** Queue adjacent files back-to-back. If the seam is already audible before runtime, fix arrangement/mastering before tuning code.

Questions to answer:

- Is intro peak high but RMS low?
- Is wave1 much lower RMS than intro?
- Is wave1 beat 1 quieter than the rest of the loop?
- Does intro have a noisy tail or a phrase ending that feels complete rather than leading into wave1?
- Is a later transition, such as wave2 -> wave3, better only because density masks the seam?

## Runtime vs generation diagnosis

If logs show the next section starts at target gain but the seam remains, do not keep tuning fade-up-from-zero behavior. The likely problem is generated audio quality: section mastering, density, arrangement, reverb/noise floor, phrase shape, or native runtime stems that are too quiet relative to the mastered full mix.

If logs show an unintended fade from silence, inspect the runtime music director and transition policy. TODO items track equal-power crossfade and live gain HUD improvements.

## Staging vs production

Three tiers, and — unlike sprites — **no audio is committed at all**:

| tier | location | git |
|------|----------|-----|
| **Source of truth** (edit + commit these) | score specs `tools/ambition_music_renderer/scores/active/*.music.yaml`, renderer code, install scripts | committed |
| **Staging / scratch** | `tools/ambition_music_renderer/output/`, `…/generated/`, `target/generated-audio/` | gitignored |
| **Runtime** (what the game loads) | the entire `crates/ambition_gameplay_core/assets/audio/` tree — music OGGs *and* `sfx.bank` | **gitignored** (`.gitignore` line 66) |

Consequences worth internalizing:

- **A fresh clone has no audio.** Runtime audio is regenerated, not stored —
  run `./regen_assets.sh` (or `./regen_music.sh` + `./regen_sfx.sh`) after
  cloning. This is the opposite of sprites, which *are* committed under
  `assets/sprites/` (see
  [generated-visual-tools.md](../tools/generated-visual-tools.md)).
- **You cannot accidentally commit generated audio** — the whole runtime tree
  and every staging tree is ignored. So the publish/edit loop is simply: edit
  the score spec → `./regen_music.sh` → playtest. The only thing you commit is
  the score-spec (or renderer) change; the OGGs it produces stay local.
- Specs and renderer code are reviewed like source code; the generated OGGs are
  not reviewed in diffs because they are never in a diff.

## Validation

```bash
python -m pytest tools/ambition_music_renderer/tests
python scripts/check_agent_kb.py
uv run --project ~/code/ambition/tools/ambition_music_renderer python -m ambition_music_renderer audit levels --check   # fail on any clipping cue
```

`tools level_report --check` exits non-zero if any cue's true peak exceeds
-1 dBTP — a cheap regression gate after a re-render. (It does not gate on the
loudness spread; that's a mastering call you read off the report, not a
pass/fail.) Runtime audio changes usually also need a sandbox smoke run or a
manual web-audio check depending on the target.
