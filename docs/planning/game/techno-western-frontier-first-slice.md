# Techno-western frontier: first playable slice

**Status:** Implementation handoff, not approved canon. Read `docs/brainstorms/2026-11-20-hobford-gates-and-frontier.md` for intent and ownership of ideas. This task is separate from the cowboy robot chicken boss already under construction.

## Goal

Build a playable, connected area in the frontier dimension. The area should stand on its own before the later damage, scrapyard, Fia, and Oiler story is fixed. Its style draws from Western frontier adventures, industrial science fiction, and comic absurdity. Build a place with local life and credible reasons for conflict, not a row of unrelated boss rooms.

## First slice

1. **Auxiliary gate town:** A small station, a gate terminal, a recovery inn, a rail bridge, and access to a local mechanic district. Use existing room, terminal, and dialogue capabilities. Do not place the main GA metropolis here.
2. **Departure:** Alice and Bob need rest after travel. They can direct Scout toward an ordinary mechanic. Do not force Scout to join or abandon them. Keep the proposed Hobford dialer damage as story context until the required terminal mechanics are designed.
3. **Train robbery:** An outlaw locomotive attempts to steal freight cars from another train. Scout falls or is knocked from a bridge into the event. The train's goal is freight, not Scout. Fighting begins when Scout gets in its way. Make a small playable sequence of train-top traversal, cargo hazards, and a clear exit far from the gate town. A staged moving-scene illusion is acceptable if it uses existing engine capabilities.
4. **Arrival district:** Connect the train sequence to an industrial frontier settlement or outlying district. Provide at least one alternative route or loop and space for later NPCs and adventures. Avoid a long flat corridor.

## Scope and ownership

- Another agent owns the **cowboy robot chicken boss**. Do not change its encounter, animations, behavior, or directly owned files. Inspect the live checkout for ongoing changes and coordinate shared content files before touching them.
- Do not select the second major boss yet. Do not force Scout into the scrapyard in this slice. The chicken boss may or may not become part of that story.
- Use the repo's LDtk and content-pack patterns. See `docs/planning/game/world-composition.md`, `docs/authoring/level-design-guide.md`, and `docs/planning/game/gate-network.md`. Follow `AGENTS.md`. Do not create or switch worktrees.
- Keep named content in authored data rather than bespoke engine cases. Build a dense, connected room cluster with a readable default route; allow later manual LDtk edits.
- A broken dialer that leaves one selectable destination is **not** the current gate engine behavior. Do not silently change the global dialing contract to implement a story beat. Defer or implement it through an explicitly reviewed local terminal state.
- Keep other agents' files and unrelated working changes untouched. Stage only files owned by this task and commit those files with the required co-author attribution.

## Done for this iteration

A player can arrive at the frontier terminal, explore the inn and bridge area, enter a playable train robbery, and emerge in a different district. Connections and collisions work in the current game. Only add focused verification for this path; do not expand the regression suite for unrelated features. Record the exact authored files and any missing assets or deferred dialogue in the handoff.
