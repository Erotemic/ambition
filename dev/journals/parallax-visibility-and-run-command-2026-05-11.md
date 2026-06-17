# Parallax scaffold: visibility and run-command follow-up

The first parallax overlay had two handoff issues.

1. The validation command used `cargo run -p ambition_gameplay_core`, but the package has multiple binaries, so the correct visible-game command is:

```bash
cargo run -p ambition_gameplay_core --bin ambition_gameplay_core
```

2. The first generated background stack was too subtle and too far behind the world backplane to be obvious in the current placeholder rooms. The follow-up keeps the parallax system presentation-only, but raises the default layer z values near the world backplane and regenerates higher-contrast placeholder art so the scaffold is visibly testable before final painted assets exist.

Pattern: when adding a visual scaffold with placeholder art, make the first pass deliberately visible and easy to verify. Once the system is confirmed, art direction can reduce contrast and saturation.
