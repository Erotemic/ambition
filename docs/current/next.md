# Current next moves

1. Keep ADRs and concept pages modern enough that agents can trust them.
2. Continue shifting runtime integration toward data-driven Bevy ECS instead of parallel code-owned world state.
3. Replace obsolete migration docs with current systems/concepts or archive them.
4. Fix known wall-cling / collision / transition issues with trace-backed tests.
5. Expand platform smoke coverage for desktop, web, Android/mobile touch, controller, and Steam Deck.
6. Improve tool documentation so agents know which generator/validator to use.
7. Promote durable lessons from `dev/` into concepts, recipes, or ADRs when they stop being one-off postmortems.
8. Extend the controllable-entity unification: route the player through the shared `ActorControlFrame` brain→sim seam (`crates/ambition_engine/src/actor_control.rs`) so the player path matches the enemy + boss path, unblocking "play as a goblin" and multi-player with per-character `AbilitySet`s. See `docs/systems/character-ai-refactor.md` and `docs/planning/player-singleton-audit.md`.
