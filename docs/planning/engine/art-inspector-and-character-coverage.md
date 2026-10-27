# Art inspector and character coverage

**State:** implemented, 2026-10-26. Parent programs:
[authoring](authoring-and-tools.md) and
[inspection](inspection-diagnostics-and-workbench.md).
See the [inspector guide](../../../tools/art_inspector/README.md) for commands,
evidence states, and limits.

## Delivered

- The default townsfolk profile omitted reactions and death. Renderer commit
  `eaacd737c133` added hurt, death, and corpse rows to the common profile. All
  affected sheets were published at all quality tiers. Keep this policy common
  to mortal bodies; peaceful behavior does not remove mortality.
- The inspector started with local files and guessed generations from names.
  The derived inventory now joins prepared catalog identities, registered
  product plans, local products, curated concepts, and design history. Missing
  bundles remain visible. Explicit forms, parts, and derivations stay separate.
  Remove no history or identity merely because metadata is sparse.
- The CLI and offline page now share owner reports for game use, registration,
  generation state, tiers, metadata, and coverage. Status, list, inspect, and
  coverage queries need no game launch or Rust build after the inspection binary
  exists. The browser adds identity trees, facets, action comparison, scrubbing,
  and death-to-corpse playback.
- Sprite, background, terrain, quality, and atlas owners expose publication
  evidence. Focused and full sprite publication use the same input recorder.
  Required-member checks, status, and publication use the renderer's decision.
  Receipts move with staged sprite assets. A failed staged publish keeps the
  previous live receipts and pixels.
- The content compiler exports prepared identities, dependencies, authored move
  clip requests, and selected worlds. The sheet owner exports loose RON rows,
  page checks, semantic fallbacks, move bindings, corpse policy, and manifest
  fingerprints. YAML preview metadata is compared with the runtime manifests.
- Sparse metadata is authored for chargen, the crypto crew, and training
  equipment. Unknown and conflicting metadata remain visible. Name similarity
  is a navigation suggestion and cannot establish identity.
- `capture_scene --art-snapshot` exports read-only body and presentation facts.
  The inspector can compare loaded manifest selection with local products.
  Simulation state remains authoritative.

## Guards and standing limits

Focused tests cover explicit relation joins, absent bundles, unknown discovery,
source/profile/runtime coverage, shared inputs, individual specs, missing members,
failed conversion acceptance, and staged receipt publication. The Rust resolver
regression explains the original death fallback and the direct published clip.

Legacy generation records remain unknown until a publish records complete inputs.
Do not backfill a claim that today's source produced old pixels. Status must not
render art, draw previews, or walk history. Use the owner's status decision.
Compiled bindings that owners do not export have no asserted reference closure.
Live manifest evidence does not prove a graphics device has uploaded local
pixels. Keep these limits explicit.
