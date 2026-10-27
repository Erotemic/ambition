# ADR 0039: Keep only crate boundaries with demonstrated architectural value

## Status

**Proposed governance (2026-10-26); conservative consolidation rule accepted for review.** This
reframes the historical same-crate proto-boundary method in [ADR 0019](0019-pluginized-platformer-runtime.md); it does not authorize any merge or rename.

## Context

Ambition has many small domain and adapter crates and a former actor monolith whose name no
longer fully represents its responsibilities. Small specifications, identity contracts and
portable SDKs sometimes exist precisely to prevent heavier dependencies. Conversely, crates with
overlapping ownership or mandatory paired installation may be candidates for consolidation. Pure
size, crate count, or an old extraction plan cannot tell the difference.

## Proposed decision

- Keep **modules**, **Cargo crates**, **runtime capabilities**, **public compatibility
  surfaces**, and **game compositions** as distinct concepts. One capability may span several
  crates, and one crate may install several related capabilities.
- A crate must justify an independent semantic owner, dependency direction, feature/dependency
  closure, compilation/rebuild boundary, portable API, or independently useful customer. Its
  small size is not a defect.
- Require affirmative evidence before merging: identical responsibility and lifecycle; no
  independently useful consumer; no cycle, ownership violation, or unintended dependency growth;
  meaningful improvement in simplicity or cost; and tested restoration of the same gameplay
  behavior.
- For every candidate merger, measure **resolved** dependency and feature closure, not only
  manifest edges, and include external/headless/Smash/Mary-O/Sanic/Ambition/extension consumers
  where relevant.
- Prefer a module move, narrow shared contract, explicit composition helper, or rename when that
  solves the real problem without eliminating an independently useful boundary.
- Names reflect present responsibility. `monolith` and `shared_tangle` are historical warning
  labels, not eternal mandated names or proof that a crate must be split further; rename only
  after verifying imports, content identity, scripts, assets and public consumers.
- Public stability is deliberate and consumer-driven. Internal crate paths carry no blanket
  compatibility promise; supported semantic facade APIs and explicitly supported focused SDKs
  are the compatibility commitments.
- Prototype potentially destructive consolidation on a disposable branch, preserve module seams
  and source history, compare build/behavior before and after, and do not combine unrelated
  physics/networking refactors with mechanical moves.

## Rejected criteria

Do not merge by line count, direct consumer count, package-name similarity, or because the
flagship always installs two crates together. Do not add a wrapper crate just to hide an
unresolved dependency cycle. Do not treat deletion of a crate as a success metric.

## Evidence gates

- A before/after manifest and resolved-closure diff for affected engine and game profiles.
- Tests showing an excluded capability truly disappears, plus a representative external consumer
  and relevant portability builds.
- A source-level explanation of lost and retained boundaries, with asset paths, persistent owner
  IDs, rollback schema names and authored references enumerated.
- One independent migration and one rollback path documented before committing to the
  consolidation.

See [crate/capability planning](../planning/engine/architecture-reassessment-and-composable-multiplayer.md) and
[public compatibility](0031-public-facade-is-the-compatibility-boundary.md).
