# Current next moves

1. Keep ADRs and concept pages modern enough that agents can trust them.
2. Continue shifting runtime integration toward data-driven Bevy ECS instead of parallel code-owned world state.
3. Replace obsolete migration docs with current systems/concepts or archive them.
4. Fix known wall-cling / collision / transition issues with trace-backed tests.
5. Expand platform smoke coverage for desktop, web, Android/mobile touch, controller, and Steam Deck.
6. Improve tool documentation so agents know which generator/validator to use.
7. Promote durable lessons from `dev/` into concepts, recipes, or ADRs when they stop being one-off postmortems.
8. **Finish the controllable-entity unification**: the universal-brain seam (Chunks 1–4f, 2026-05-24) wired `Brain` + `ActionSet` + `ActorControl` across player / NPC / enemy / boss + `ActorActionMessage` resolver stream. Daytime continuation: flip combat / projectile spawners onto the message stream and decompose `ae::Player` into ECS components. See `docs/systems/brain-driver.md` (overview), `docs/recipes/extending-brains-and-action-sets.md` (recipe), `TODO-controllable-entity.md` (plan), and `dev/journals/ae-player-field-usage-2026-05-24.md` (audit + what landed).
