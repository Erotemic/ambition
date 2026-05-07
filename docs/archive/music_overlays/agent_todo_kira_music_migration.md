# Kira Music Migration Handoff

Task goal: migrate `ambition_sandbox` from a single generated Bevy audio loop to a Kira-backed procedural multi-track music system, preserving the old tune as a selectable track and adding a longer default tune.

Current status: implementation complete. Kira-backed generated audio, multi-track data, pause-menu switching, and docs updates are wired; validation passed.

Files already changed:
- `docs/agent_todo_kira_music_migration.md`
- `crates/ambition_sandbox/Cargo.toml`
- `crates/ambition_sandbox/src/data.rs`
- `crates/ambition_sandbox/src/audio.rs`
- `crates/ambition_sandbox/src/app.rs`
- `crates/ambition_sandbox/src/setup.rs`
- `crates/ambition_sandbox/src/pause_menu.rs`
- `crates/ambition_sandbox/assets/ambition/sandbox.ron`
- audio-related docs under `docs/`

Remaining work:
- None; this file is kept as a concise completion handoff for the migration.

Validation commands already run:
- `git status --short`
- `git log -1 --oneline`
- `cargo fmt`
- `cargo check -p ambition_sandbox` (passed after disabling `bevy_material_ui` full Bevy defaults)
- `cargo test -p ambition_sandbox audio` (passed)
- `cargo test -p ambition_sandbox data` (passed)
- `cargo test -p ambition_sandbox` (passed)
- `cargo tree -e features -p ambition_sandbox | rg "bevy_audio|rodio|bevy_kira_audio|kira|feature \"audio\"|feature \"wav\""` (passed: shows Kira, no Bevy built-in audio)

Validation still needed:
- None before commit.

Known blockers or risks:
- No current blockers. Remaining risk: no runtime audio-device/window smoke test was performed; validation is compile/test/feature-tree only.

Design decisions made:
- This migration will not preserve the old `audio.music` RON shape or old Bevy `AudioPlayer` backend.
- Generated audio now renders to Kira `StaticSoundData` frames directly instead of encoded in-memory WAV bytes.
- `long_lofi_drift` will be the configured default track; `original_lofi_loop` remains selectable.
- Kira is visible/presentation-only; headless still avoids audio.
- The new long track authors 32 bars of chords and bass roots to avoid the old clamp-to-final-bar behavior.
