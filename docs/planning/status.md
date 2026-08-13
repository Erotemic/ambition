# HEAD orientation

**Snapshot:** `cecd01ca064b` (2026-08-13).

This page is a cold-start orientation map, not an execution queue and not a
completion diary. The live continuation authority is
[`queue-72h-2026-08-08.md`](queue-72h-2026-08-08.md). The focused plan linked by
a queue row owns its technical design. [`tracks.md`](tracks.md) is the standing
reservoir used when the queue needs more work.

If this page disagrees with a focused active campaign or with current source,
update this page rather than appending another historical correction.

## What is active now

### D73 — authority convergence

[`authority-convergance-campaign-2026-08-13.md`](authority-convergance-campaign-2026-08-13.md)
is the current first ordering authority. At this snapshot the campaign has
already removed most of the legacy actor/template duplication:

- `ActorIntent` / `ActorCooldowns` mirror maintenance is gone;
- `BodyCombat` has been reduced toward actual reaction history instead of a
  mixed mirror/configuration object;
- prepared character bodies are complete enough that the old incomplete-body
  fallback is no longer the intended architecture;
- `adopt_character_intrinsics` is deleted;
- the enemy archetype/roster ontology has been deleted; and
- the campaign is in its final AC7 naming/documentation/amplification phase.

The campaign itself is the authority for its exact progress and hard exit
criteria. Do not recreate an earlier AC phase because an older plan still
mentions its pre-migration symbols.

### D72 — Smash as a body-generic engine customer

[`smash-body-generic-combat-2026-08-09.md`](smash-body-generic-combat-2026-08-09.md)
is the next major proving ground after the D73 ordering constraint. Smash is not
a mode that earns special body/combat paths; it is a customer that should force
the shared engine abstractions to become expressive enough.

### Continuation queue

The queue is intentionally inexhaustible. Finishing its current rows means
selecting the next highest-value unresolved work, recording it, and continuing.
Do not treat a short or temporarily empty row list as a project-completion
signal.

## Architectural state worth knowing before touching code

- **One body, one path is the default model.** A controlled body, NPC, hostile
  actor, boss, summon, and match fighter are ordinary actor bodies distinguished
  by authored capabilities and contextual control/relationship/session facts.
  See [`../concepts/one-body-one-path.md`](../concepts/one-body-one-path.md).
- **Character definitions are reusable authored composition.** Intrinsic body
  facts belong to the prepared character/body; controller, disposition,
  placement and ruleset/session facts remain contextual.
- **Construction is transactional.** Preparation resolves authored identifiers
  before commit; missing content should refuse before partially replacing a
  world/session.
- **Rollback authority is simulation authority.** Irreversible presentation
  effects caused by speculative simulation cross the confirmed external-effect
  seam rather than escaping directly.
- **Determinism must not depend on incidental Bevy graph topology.** The current
  authority-convergence campaign has a scheduler-perturbation canary; the larger
  system-parameter/phase decomposition remains successor work.
- **The actor monolith is drained by coherent ownership, not line-count quotas.**
  Use [`engine/actor-monolith-decomposition.md`](engine/actor-monolith-decomposition.md)
  and current dependency/ECS evidence.
- **The public facade is the compatibility boundary.** Internal historical crate
  topology is not an API promise.

## Important successor architecture work

These are deliberately not folded into D73 just because they are nearby:

- invert rollback declaration/registration ownership so the generic runtime no
  longer acts as a census of every gameplay domain;
- decompose Bevy parameter-ceiling systems such as large actor-brain ticks by
  semantic authority and simulation phase rather than tuple-packing parameters;
- continue actor-monolith decomposition where a carve improves ownership,
  dependency closure, API shape or iteration cost; and
- continue public-facade / optional-capability cleanup where consumers still
  inherit capabilities they did not request.

The standing reservoir for these and other non-immediate tasks is
[`tracks.md`](tracks.md).

## Current product / acceptance fronts

- **Sanic:** the shared movement/host seams, ring economy, badnik loop and restart
  path exist; the remaining acceptance list lives only in
  [`demos/sanic.md`](demos/sanic.md).
- **Super Mary-O:** use [`demos/super-mary-o.md`](demos/super-mary-o.md) and Jon's
  observations for remaining behavior/polish. Do not reconstruct its backlog
  from old execution ledgers.
- **Smash:** active body-generic proving ground; see the D72 plan above.
- **TwinTrack:** relativity research/acceptance remains active through
  [`engine/relativity.md`](engine/relativity.md) and
  [`demos/twintrack.md`](demos/twintrack.md).
- **Hollow-lite / bosses:** still useful engine customers; see
  [`demos/hollow-lite.md`](demos/hollow-lite.md) and
  [`engine/boss-design.md`](engine/boss-design.md).

## Explicitly deferred, not abandoned

- Matchbox/P2P predicted-input correction work waits until a game actually needs
  online play; see [`engine/netcode.md`](engine/netcode.md).
- Slower Light remains a future 3D relativity game; see
  [`engine/slower-light.md`](engine/slower-light.md).
- Water/oil extensions to falling-sand work are shelved product ideas, not
  rejected features; see [`engine/falling-sand.md`](engine/falling-sand.md).
- The Leafwing clash-scan optimization remains trigger-based maintenance; see
  [`triage/leafwing-clash-scan-patch-2026-07-23.md`](triage/leafwing-clash-scan-patch-2026-07-23.md).

## Where to look next

1. [`queue-72h-2026-08-08.md`](queue-72h-2026-08-08.md) for execution order.
2. The focused plan named by the selected queue row.
3. [`JONS_OBSERVATIONS_BUGS_AND_ISSUES.md`](JONS_OBSERVATIONS_BUGS_AND_ISSUES.md)
   for direct maintainer observations.
4. [`awaiting-maintainer-decision.md`](awaiting-maintainer-decision.md) only for
   questions that genuinely still need a maintainer decision.
5. [`tracks.md`](tracks.md) when replenishing the queue.
6. `docs/concepts/`, `docs/systems/`, `docs/architecture/` and `docs/adr/` for
   settled current truth; `docs/archive/` for history.
