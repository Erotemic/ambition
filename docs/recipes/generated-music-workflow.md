
# Generated music workflow

This is the current recipe for generated/adaptive music. Older transition labs and balance notes were archived as historical experiments.

## Current model

- Source specs and renderer code live under `tools/ambition_music_renderer/`.
- Generated outputs are local until explicitly installed or published into runtime assets.
- Runtime playback belongs to the sandbox presentation/audio layer.
- Asset identity and packaging policy should flow through the asset catalog when a cue becomes part of the game.

## Common commands

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
6. Update `docs/tools/generated-audio-tools.md` if the workflow changes.

## Validation

```bash
python -m pytest tools/ambition_music_renderer/tests
python scripts/check_agent_kb.py
```

Runtime audio changes usually also need a sandbox smoke run or a manual web-audio check depending on the target.
