# Non-player-centric actor unification — run log

Live progress for the autonomous run guided by
`non-player-centric-actor-unification.md`. Newest entries at the bottom of each
stage. Jon reviews the cumulative diff on `main`; this log is the trail + the
resume point.

**Decisions (confirmed):** improve player feel opportunistically · movement +
combat · bosses last · through Stage 6 (player → input+camera+HUD).

## Status: Stage 0 — vertical slice (dash as a component+system on a phase pipeline)

- [ ] Map the player movement monolith (verbs + ordering).
- [ ] Define the spine `SystemSet` phases (Intent → ModeSelect → Integrate → Sweep → Resolve).
- [ ] Extract `dash` into a `Dash` component + a phased `apply_dash` system.
- [ ] Run dash on the player AND the clone; verify (replay or pinned behavior).
- [ ] Commit.

### Notes / decisions / behavior changes
- (run begun)
