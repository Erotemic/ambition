# Android power plan

This is a planning note for improving Android/mobile battery, thermals, and responsiveness. It is not a repeatable build recipe; use `docs/recipes/android-build.md` for build/sideload commands.

## Goals

- Keep touch input responsive under mobile frame pacing.
- Avoid accidental high-power debug settings in normal Android runs.
- Preserve asset loading behavior across embedded, served, and loose development assets.
- Add smoke tests or manual checks for suspend/resume, audio unlock, and orientation/window behavior when practical.

## Candidate work

1. Audit Android feature flags and default runtime settings.
2. Add a small Android smoke checklist to the build recipe once tested on hardware.
3. Keep mobile touch controls routed through the same gameplay action/control-frame path as other devices.
4. Capture any thermal/frame-pacing lessons in `dev/journals/` before promoting durable rules here.

Related docs: `docs/recipes/android-build.md`, `docs/systems/mobile-touch-controls.md`, `docs/concepts/platform-targets.md`.
