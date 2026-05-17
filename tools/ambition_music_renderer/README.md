# Ambition music renderer

Author-time renderer for generated Ambition music assets. Generated outputs are local until explicitly installed or published into runtime assets.

## Common commands

Run from the repo root:

```bash
PYTHONPATH=tools/ambition_music_renderer python -m ambition_music_renderer --help
```

Use the package CLI and scripts in this directory for current music-renderer work. Older docs may mention retired paths under `tools/audio/`; those paths are stale and should not be copied into new instructions.

## Useful files

- `ambition_music_renderer/cli.py` — package CLI.
- `render_first_goblin_transition_lab.sh` — local transition-lab helper.
- `install_first_goblin_tune_v2.py` — installer for the first-goblin tune asset path.
- `audit_cue_balance.py`, `transition_audit.py`, `spectral_compare.py`, `spectral_localize.py` — analysis helpers.
- `goals.md` — design/planning notes for renderer direction.

## Agent rules

- Keep generated audio out of runtime assets unless the task explicitly installs/publishes it.
- Preserve conservative gain ranges in tune specs; the runtime renderer can clip if stems are too hot.
- Update `docs/recipes/generated-music-workflow.md` and `docs/tools/generated-audio-tools.md` when the workflow changes.
