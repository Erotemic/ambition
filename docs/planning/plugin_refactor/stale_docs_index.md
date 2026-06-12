# Plugin refactor stale-doc index

**Status:** largely resolved by the 2026 docs cleanup and crate bisection.

Use this as a quick checklist when reviewing old docs:

- `ambition_sandbox` is machinery, not the app shell.
- `ambition_app` owns binaries, host composition, full-stack tests, menu host stack, and dev overlays.
- `ambition_content` owns named content.
- Foundation crates must not import sandbox/content/app.
- New gameplay subsystems should own their `Plugin` registration instead of being hand-wired in `app/plugins.rs`.
- Portal, runtime, input, time, audio, menu, SFX, and asset-manager rules live in current crate/system docs, not in old stage plans.

If an old document conflicts with this checklist, either rewrite the conflict into current docs or leave the old file as historical archive only.
