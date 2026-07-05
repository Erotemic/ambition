# Ambition on the engine — the first customer, and the demo host

**Authored by fable, 2026-07-05.** How Ambition-the-game relates to the
engine and the demo suite. Story/pillars: [`vision.md`](vision.md);
bosses: [`bosses.md`](bosses.md); cast: [`characters.md`](characters.md).

## The relationship

Ambition is `ambition_content` + the host choices in `ambition_app` —
structurally identical to a demo, just bigger. Every engine capability
lands with Ambition as its first consumer (the sandbox is the integration
lab); the full game build-out (story beats, biomes, the theorem-upgrade
arc) happens on the FINISHED engine, per the north star ordering.

## The sandbox hosts the demo suite

The composability showcase ([`../vision.md`](../vision.md) §5): the
sandbox world gains a wing per demo (the Sanic zone, the Mary-O pipe
room, the Colosseum for Super Smash Siblings, the Hollow Lite well), each
mounted via the mode-scope pattern
([`../demos/README.md`](../demos/README.md)) — `ambition_app` depends on
the demo CONTENT crates; the demo's rules plugin runs `.run_if(in_mode)`
over its zone's rooms. Possess Sanic in the Hall of Characters, walk
through the Sanic door, and you are playing the Sanic demo — same
systems as its standalone app, different chrome. The Hall is therefore
also the roster surface: whatever body you bring INTO a wing is the body
the mode seeds (SSB seeds slot 1 from it; Mary-O wears you into Mary-O
unless you ARE someone).

## Game-side queues (kept, unchanged in priority)

- The intro narrative slice (wake → raid → escape → gate stack) and the
  Alice/Bob arc remain the story spine (game/vision.md).
- The Noether Chamber, the PCA encounter (PAUSED after S3), Oiler
  ([`../engine/falling-sand.md`](../engine/falling-sand.md) §2), and the
  gnuton mounted-boss payoff are the standing set-piece backlog.
- "Morrowind rules" (killable questline NPCs; the reality-rift
  consequence line) is Ambition POLICY on the engine's dead-stays-dead
  default — codified in `docs/storylines/cannon.md` §Story Continuity.
- The feel queue (BLIND commits ledger in [`../tracks.md`](../tracks.md))
  is Jon's.
