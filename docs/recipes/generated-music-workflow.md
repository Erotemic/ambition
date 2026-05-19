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
PYTHONPATH=tools/ambition_music_renderer python -m ambition_music_renderer --help
./generate_audio_assets.sh --force
```

From the tool directory unless the tool README says otherwise:

```bash
cd tools/ambition_music_renderer
python -m ambition_music_renderer --help
./render_first_goblin_transition_lab.sh
python transition_audit.py --help
python audit_cue_balance.py --help
```

Prefer the tool README and CLI help over old recipe fragments when command flags drift.

## Edit protocol

1. Decide whether you are changing source composition, render code, publish/install policy, or runtime playback.
2. Search `dev/journals/` and `dev/benchmark-candidates/` for music director/refactor lessons.
3. Render locally into the tool's generated output path.
4. Audit balance/transitions if a cue set changes.
5. Publish/install only when the generated assets are meant to become runtime inputs.
6. Update `docs/tools/generated-audio-tools.md` and `tools/ambition_music_renderer/README.md` if the workflow changes.

## Diagnosing an audible transition seam

Use this sequence before changing runtime code:

1. **Regenerate only the relevant cue.** For the current goblin lab, `./generate_audio_assets.sh --force` renders and installs `first_goblin_tune_v2`.
2. **Run directly in the encounter room.** Start the sandbox in the room that triggers the cue and reproduce the transition.
3. **Capture runtime logs.** Look for `start_adaptive_state`, `queue_music_state`, `gain_start=target`, and the section/state names.
4. **Audit the OGGs.** Use `audit_cue_balance.py` on the generated cue directory and compare peak/RMS/duration across `intro.full.ogg`, `wave1.full.ogg`, `wave2.full.ogg`, `wave3.full.ogg`, `recap_loop.full.ogg`, and `outro.full.ogg`.
5. **Listen outside the game.** Queue adjacent files back-to-back. If the seam is already audible before runtime, fix arrangement/mastering before tuning code.

Questions to answer:

- Is intro peak high but RMS low?
- Is wave1 much lower RMS than intro?
- Is wave1 beat 1 quieter than the rest of the loop?
- Does intro have a noisy tail or a phrase ending that feels complete rather than leading into wave1?
- Is a later transition, such as wave2 -> wave3, better only because density masks the seam?

## Runtime vs generation diagnosis

If logs show the next section starts at target gain but the seam remains, do not keep tuning fade-up-from-zero behavior. The likely problem is generated audio quality: section mastering, density, arrangement, reverb/noise floor, or phrase shape.

If logs show an unintended fade from silence, inspect the runtime music director and transition policy. TODO items track equal-power crossfade and live gain HUD improvements.

## Staging vs production

- Staging/generated files belong under the renderer output tree.
- Runtime files belong under `crates/ambition_sandbox/assets/audio/music/generated/<cue>/` only when explicitly installed/published.
- Specs and renderer code should be reviewed like source code.
- Generated scratch output should stay uncommitted unless the task explicitly asks to update runtime assets.

## Validation

```bash
python -m pytest tools/ambition_music_renderer/tests
python scripts/check_agent_kb.py
```

Runtime audio changes usually also need a sandbox smoke run or a manual web-audio check depending on the target.
