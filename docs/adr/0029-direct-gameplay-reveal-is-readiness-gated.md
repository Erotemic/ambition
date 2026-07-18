# ADR 0029: Direct gameplay reveal is readiness-gated

## Status

**Accepted; implemented for the Ambition direct sandbox** (2026-07-18).

## Context

The direct development path constructs its canonical simulation world and
presentation entities during Bevy `Startup`. The image, LDtk, font, and music
handles referenced by those entities resolve asynchronously afterward. The
window was visible immediately, gameplay simulation was already authorized, and
the default music started independently when its handle happened to settle.

That exposed implementation order to the player: an uninitialized compositor
frame, a black surface, partial room and actor visuals, late HUD/text, then audio
arriving after the world appeared. A decorative timer or arbitrary sleep would
hide none of the underlying readiness problem and would be wrong on both fast
and slow machines.

The room-transition path already establishes the stronger invariant: an opaque
cover exists before incomplete presentation can be observed, readiness comes
from real asset evidence, and the cover remains through a complete ready frame.
Initial direct entry should use the same reveal semantics.

## Decision

Visible direct-entry hosts install an optional `InitialGameplayReadiness`
resource in the closed state. `simulation_authorized` treats a present closed
gate as additional authority that has not yet been granted; apps that do not
install the resource are unchanged.

The Ambition app owns its product-specific startup presentation:

- the desktop window is created hidden where the platform supports it;
- Startup constructs a fully opaque Ambition loading surface using only the
  built-in font, so the loading UI does not depend on the assets it reports;
- the window is exposed only after that cover exists;
- the active room's concrete sprite/parallax manifest, every LDtk project
  handle, installed UI-font handles, and the initial music handle are polled
  through `AssetServer`;
- failed required assets keep the cover present and report their identities;
- an all-ready state must survive a complete covered presentation frame before
  the gate opens and the cover retires;
- direct default music observes the same gate, so visual and audio activation
  share one reveal boundary.

This is an activation-critical manifest, not an instruction to eagerly load the
entire game. Content intended for later rooms remains eligible for ordinary
room-transition loading and prefetch.

## Consequences

- `./run_game.sh sandbox` responds with an intentional surface instead of
  exposing black or partially materialized gameplay.
- The first visible gameplay frame is coherent and live; simulation did not run
  invisibly while startup presentation lagged behind.
- Startup duration remains honest. The cover fixes reveal correctness but does
  not claim synchronous construction or asset I/O became faster.
- The reusable engine owns only the optional readiness gate. Ambition owns its
  wordmark, colors, progress copy, and exact dependency manifest.
- Shell-hosted startup keeps its route/load lifecycle and is not silently
  redirected through the direct-sandbox presenter.

## Current implications for agents

- Do not replace readiness with a fixed delay, frame count, or fake percentage.
- Add new activation-critical direct-start assets to the startup manifest or to
  a lower reusable manifest seam; do not allow them to pop in after reveal.
- Keep the loading surface independent of asynchronously loaded product assets.
- Preserve one covered all-ready frame before reveal.
- Treat loading presentation and startup optimization as separate tasks: measure
  and reduce synchronous startup work even though it is now correctly covered.
