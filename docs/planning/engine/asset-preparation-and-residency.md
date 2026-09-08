# Asset preparation, device materialization and residency

**State:** OPEN — the lifecycle model and quality authority are established.
Current work is stage-specific observability, demand timing, pacing of expensive
completion, explicit residency ownership, and robust live-quality/readiness
semantics.

This page owns the **current asset lifecycle contract and open packets**. The
multi-day history of placeholder investigations, tier experiments, invalid
captures and per-run tables remains in git history and measurement artifacts.

Performance conclusions that span the whole frame belong in
[`performance-and-iteration.md`](performance-and-iteration.md).

## Goal

A visible asset should be requested early enough, prepared/materialized under one
quality authority, and resident when needed without turning asset readiness into
simulation authority.

The engine must distinguish:

```text
DECLARED / DEMANDED
    semantic content says an asset will be needed

CPU PREPARED
    source bytes/metadata are decoded and usable for materialization

DEVICE MATERIALIZED
    GPU/provider resources exist

RESIDENT USE
    the prepared/materialized representation is actually used by a visible
    presentation entity / first draw
```

Do not collapse these stages into one boolean called “loaded.”

## Authority contract

### Content declares demand

Content/domain systems identify semantic assets. Presentation/asset services
resolve those identities to concrete representations.

A room or character should not hand-write filesystem paths in every consumer.

The demand ledger (`ambition_asset_manager::image_stages`) stamps a road name on
every content-art demand. The vocabulary, every string literal reaching
`note_demand` / `load_sheet_image` / `load_sprite_pages`:

```text
asset-manifest   boss-sheet   character-sheet   entity-sprite   fx-sheet
held-item        parallax     portrait          projectile-art  shrine-sheet
vanity-card
```

**ELEVEN live roads.** Derived from the call sites by
`scripts/tests/test_demand_road_vocabulary_is_derived.py`; a road added in code
without a row here is red there. Menu icons, shell images and prop pngs are
deliberately not stamped.

### Quality has one authority

Every quality-aware materialization road consumes the active shared quality
budget/tier selection.

The earlier FX-sheet omission is repaired: FX loading now resolves the quality
variant through the same tier authority.

A deliberately full-resolution narrow loader may opt out only with that policy
stated at the call site.

### Quality transitions are swaps

Changing quality requests a replacement representation and retires the old one
when the replacement is ready. It is not a “demote the existing asset in place”
operation.

Avoid transient states where a body loses its only usable representation while a
new tier is still materializing.

### Demand, readiness and simulation are separate

Simulation must not become dependent on whether a texture/material finished
preparing on this machine.

Readiness may gate visible presentation or a transition whose contract explicitly
requires presentation readiness. It must not change deterministic gameplay
outcome.

### Residency has an owner

A cache/resident asset remains because some declared owner/budget keeps it. “It
happened to be loaded earlier” is not a residency policy.

When eviction becomes necessary, the policy must name:

- owner/scope;
- budget/pressure signal;
- re-demand behavior;
- minimum representation/readability requirements.

## Current measured conclusions

These are the conclusions still used for architecture decisions.

### Global quality tiers are the current policy

There is no room-level sprite tier cap. Do not reintroduce one through a local
loader or materialization shortcut.

### Oversampling/weak-tier cost is real

Authored high-resolution art can be significantly larger than the presentation
requires on weak targets. The correct response is quality-tier selection and
materialization policy, not arbitrary runtime rescaling disconnected from the
asset catalog.

### The visible hitch is not equivalent to “texture bytes are not ready”

Measurements separated asset readiness from visible-entity/materialization
latency. A body can be known to simulation while its presentation entity has not
yet appeared.

Instrumentation therefore needs to report all lifecycle stages rather than
blaming every placeholder/hitch on I/O.

### One-body/one-path claiming and asset preparation are different problems

A placeholder that persists because the body has no claimed visual entity is not
fixed by changing texture tier or preloading more bytes. Preserve ownership
instrumentation alongside asset timing.

### Live quality changes must preserve a usable representation

The established quality-transition contract is replacement/swap. Keep current
art visible until the requested representation is ready unless a product policy
explicitly chooses a lower fallback.

## Current work

### A1 — stage-specific observability

For a named asset/body, make it possible to answer when each stage occurred:

```text
demanded
CPU prepared
materialization requested
materialized
presentation entity claimed/created
first resident use / first draw
```

Requirements:

- stable semantic asset/body identity in the trace;
- current requested/resolved tier;
- no inference from “asset server says loaded” to “drawable exists”;
- capture/report tooling should print missing stages rather than substituting a
  later timestamp.

Acceptance: a visible hitch can be assigned to one stage without reading source
and guessing.

### A2 — demand before first visible use

Identify assets whose first semantic demand occurs too close to first visible
use.

Move demand earlier only at a domain boundary that already knows the asset will
be needed. Do not preload the whole game to hide one transition.

Acceptance:

- demand lead time is measurable;
- no new game/session loads unrelated rooms/characters;
- headless simulation can omit visual materialization.

### A3 — pace expensive completion, not declarations

If a burst is caused by materialization/device work, pace or budget that stage.
Do not delay cheap semantic declarations simply to spread a later expensive
stage.

Any pacing policy must preserve deterministic gameplay and define what happens
when first visible use arrives before the preferred budget slot.

### A4 — define residency scopes and budgets

Before adding eviction, inventory the real owners:

- boot/core shared assets;
- current session/room;
- character/roster assets;
- short-lived VFX;
- optional provider assets.

For each scope, decide what may remain resident across room/session transitions
and under what pressure it is evicted.

Do not implement a generic LRU before these scopes are known.

### A5 — eliminate accidental re-preparation/reload

A second request for the same semantic asset/tier should join/reuse the existing
preparation whenever its lifetime allows.

Instrument and poison:

- duplicate demand from two consumers;
- room leave/re-enter;
- character reappearance;
- quality swap away and back;
- session reset.

The acceptance question is whether repeated expensive preparation was required
by lifetime/policy, not whether the same path string appeared twice.

### A6 — live quality switching

A quality change must:

1. update the requested quality authority;
2. demand replacement variants where applicable;
3. keep the current usable representation until replacement is ready;
4. atomically switch presentation to the new representation;
5. retire old residency according to policy;
6. avoid simulation changes.

Include FX sheets and character sheets in the same acceptance surface.

### A7 — explicit readiness semantics

Every transition that waits on assets should state **which stage** it needs.
Examples:

- semantic simulation may need only catalog declaration;
- visible spawn may require a drawable-ready representation;
- screenshot/capture acceptance may require first draw;
- gameplay must not wait for GPU materialization unless the product contract
  explicitly makes visible readiness part of the transition.

Replace generic “assets ready” predicates when their callers require different
stages.

## Instrumentation expectations

Asset tooling should prefer generated/current facts over copied tables.

A useful report includes:

- semantic asset identity;
- source/variant selected;
- requested and resolved quality tier;
- stage timestamps/durations;
- bytes/pixels when relevant;
- owner/scope;
- current resident/use state.

Historical hall/tier tables are evidence, not live policy. Re-run the instrument
when a current number is needed.

## Explicit non-goals

- no room-level quality cap;
- no “preload everything” solution;
- no simulation dependency on GPU readiness;
- no generic eviction cache before residency ownership is defined;
- no second quality selector inside an individual loader;
- no treating every missing visual as an asset-I/O problem.

## Acceptance

The current architecture slice is complete when:

1. all important presentation asset roads use one quality authority;
2. demand/preparation/materialization/first-use are separately observable;
3. first-visible-use hitches have a named stage and owner;
4. expensive bursts are paced at the expensive stage;
5. residency ownership/budgets are explicit enough to support eviction if
   needed;
6. duplicate demand does not cause accidental duplicate expensive preparation;
7. live quality switching is replacement/swap and covers character + FX roads;
8. transitions wait on the readiness stage they actually require.

Use git history for the removed placeholder, Potato-tier, Mary-O, and hall
measurement chronology.
