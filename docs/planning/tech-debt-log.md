# Tech debt log

This file is a short list of open work. Closed migration notes belong in git history, `docs/archive/`, or `dev/journals/`, not in the active queue.

## Open

### Runtime / state

- **HIGH — Possible goblin-encounter lock-wall / ceiling teleport.**
  Historical traces and synthetic repros did not reproduce on the current engine, and the original trace only captured aftermath. Keep the existing repro guards. If the bug recurs, capture a fresh trace with the auto-dump path and fix the velocity-budget rejection around zero-TOI collision corrections.

- **MED — Hostile NPC conversion race invariant is tested but not enforced.**
  `features/world_overlay.rs::apply_save` can replace an NPC with an enemy while other systems might still hold stale assumptions. Prefer a typed conversion event or staged command path if this resurfaces.

- **LOW — `ProgressionResources` and `SandboxQueues` are pragmatic SystemParam bundles.**
  They work around Bevy arity limits. If they keep growing, split them into public, documented bundles with narrower ownership.

### Content / authoring

- **LOW/MED — `RAID_ENFORCER_SHEET` may be unused.**
  The sheet and tests exist, but the runtime sprite table appears to fall back to the generic render path. Either register it intentionally or remove the unused static.

- **LOW — Boss spec id still derives from LDtk name.**
  `encounter_id_from_name` works, but an explicit LDtk `encounter_id` field would separate display strings from save/runtime keys.

- **MED — Boss phase music tracks are placeholders.**
  The phase-swap mechanism works; authored per-phase track identity is still content work.

- **LOW — Quest log lines are appended to HUD text.**
  Real game UX wants a dedicated quest panel.

- **LOW — Map UI repaints room rectangles each frame.**
  Fine for current room counts; switch to persistent per-room entities if the map grows.

### Tests / observability

- **MED — Keep trace capture around teleport-class collision corrections healthy.**
  The auto-dump path is the guard against aftermath-only traces. Do not remove it without replacing the same diagnostic coverage.

### Build / repo

- **LOW — Superseded enemy-update impl appears dead.**
  `features/enemies.rs` still has a legacy update-method cluster reported as dead by `cargo build --lib`. Remove only with a full build and cfg audit.

- **MED — Truly headless builds still need Cargo-feature cleanup.**
  Render/UI Bevy features can still pull in `bevy_winit` through feature unification. Move visible-render features behind a render persona and cfg-gate code paths end-to-end.

- **LOW — Repo-root/tooling clutter accumulates.**
  Periodically remove untracked AppImages, temp config, and obsolete tool artifacts.

## Recently closed / archived

Closed entries removed from the active queue include player ECS migration, `SandboxRuntime` god-resource removal, `EnemyRuntime` movement authority cleanup, map UI bootstrap, hostile-NPC conversion test coverage, boss music-request assertions, ledge-grab probe regressions, parallel Update-chain cleanup, and `FeatureEventBus` removal.

Use git history or `docs/archive/completed-migrations/` for details when archaeology is needed.
