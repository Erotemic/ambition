# Sprite residency and live quality Apply

**Status: planned, 2026-08-08.** Jon's ruling plus a GPT 5.6 architecture brief
he forwarded. This file is the plan of record; the ledger row points here.

---

## The product requirement (Jon, 2026-08-08, overruling the review)

> *"When we hit apply on the new quality mode in the menu, we can do whatever
> work we need to to unload and load. There can be a delay in it happening, but
> the game needs to come back up, in the state you were in, but with different
> quality assets."*

⛔ **"Texture quality applies on next room load" is explicitly rejected.** A
delay is acceptable. A cover/fade is acceptable. Not converging is not.

⛔ **AND NO ARCHAEOLOGY.** The review asked for a regression hunt — *"this used
to work, find the commit"*. Jon: *"we had it working at some point in the past
(dont bisect, just move forward)."* **That section of the brief is void.** The
diagnosis below was reached from the live tree and needs no history.

## The defect, diagnosed from the tree

`character_runtime/mod.rs:471` — the materializer drains a demand queue:

```rust
for token in demand.take() {
    …
    let materialization = materialize_declared_character_sprite(…, quality, …);
```

and `CharacterSheetState` (`ambition_sprite_sheet/src/character/assets.rs:26`) is
already:

```rust
Ready(&CharacterSpriteAsset)   // decoded, holds a strong Handle<Image>
Declared { character_id }      // "nothing has materialized it yet"
Unknown
```

⭐ **Nothing ever re-demands a `Ready` character.** So on Apply:
`load_game_assets` re-runs, `140/140 catalog entries declared`, **+1 image**, and
every body on screen keeps the handle it already had. New characters materialize
at the new tier; old ones do not. Measured on device.

⭐⭐ **The state machine we need is already half-written.** `Declared` IS the
nonresident state — it simply has no path back from `Ready`. The fix is not a new
registry; it is **a return edge plus a tier stamp.**

## The design

**One new fact: the tier a realization was made at.**

```
Declared ──demand──▶ Ready @ tier T
   ▲                      │
   └──── Apply(T' ≠ T) ───┘        drop CharacterSpriteAsset ⇒ strong handles die
                                    ⇒ Bevy frees the Image ⇒ residency falls
```

* `Ready` carries the tier (or a monotonic `QualityGeneration` id) it was
  realized at.
* **Apply** bumps the generation, demotes every `Ready` whose stamp ≠ current to
  `Declared`, and re-requests the ones still live.
* Dropping the `CharacterSpriteAsset` drops the strong `Handle<Image>`. **There
  is no evictor to write** — Bevy frees the image when the last strong handle
  goes, and the ownership model does the rest.
* Logical identity (`character_id` / token) never changes, so **no body is
  respawned and no gameplay authority is rebuilt.** Only the physical
  realization moves.

⚠ **Two strategies, and Android takes the conservative one.** Where memory
allows, load-then-swap-then-release avoids a visible interruption. Where it does
not, cover → detach → drop → load → rebind → reveal. Start with the destructive
path on Android so peak memory stays bounded. ⛔ **Do not build a memory oracle
to choose between them.**

## What we are NOT building

⛔ LRU cache, popularity tracking, adaptive quality, byte-budget scheduler,
predictive streaming, one pack group per character, or policy machinery around
pack plans. `AGENTS.md`'s guardrail section applies with full force. Deterministic
ownership — session / room / encounter / quality-generation — replaces all of it.

## Convergence: one tier vocabulary

Two systems ship the same pixels today (~678 MB across three representations,
1.1 GB APK):

| | per-sheet roots | ultrapack |
|---|---|---|
| storage | `sprites_0_5x/`, `_0_25x/`, `_potato/` | `sprite_packs/{full,half,quarter,potato}/` |
| missing tier | ⛔ **silent** fallback to full | reports the tier it used |
| grouping | none | `pack_plan.yaml` residency cohorts |

**Ultrapack wins** and the per-sheet *runtime* roots go once consumers migrate.
Canonical full-res source and manifests stay — generation, body metrics, frame
metadata and pack rebuilds all need them. ⭐ **Every tier is packed
independently** (already true): group membership is logically stable, page
geometry varies by tier, and potato's 8 px frame floor stops being a special
case.

Groups should be **natural residency cohorts** (`always`, `intro`, `hall`, an
encounter…), named from real content lifetimes. ⛔ not one per character — that
throws away atlas locality and re-invents per-sheet textures the hard way. The
shared pool becomes the exception.

⚠ **Grouping does not replace unloading.** Groups answer *what loads together*;
ownership answers *why is this still alive*; the generation transition answers
*how it gets replaced*. All three are needed.

## Order of work

1. **Tier stamp + return edge + Apply transition** for character sprites. The
   core, and it is the slice that makes the feature true.
2. **Migrate consumers to ultrapack**; delete the per-sheet runtime roots; make
   fallback explicit and observable.
3. **Residency cohorts** in `pack_plan.yaml` mapped to session/room/encounter.
4. **Packaging** — stop shipping duplicate representations, then decide which
   tiers Android needs.
5. **ASTC**, last. ⛔ compression must not mask duplicate tiers, mixed paths, or
   permanent ownership.

## Verification — the contract, not the plumbing

⛔ Tests asserting "the profile changed" or "`load_game_assets` ran" are worthless
here: **both are already true while the feature is broken.**

* Medium → Low: the same body entity survives, its logical identity is
  unchanged, its presentation now references the Quarter realization, and
  nothing live still references the Half one.
* A body materialized *after* Apply uses the same generation as one that
  survived it.
* Low → High as well — memory behaves differently upward.
* A shared pack page changes coherently for its whole cohort (⛔ not one reload
  per entity).
* ⭐ **the invariant worth pinning: after Apply completes there is exactly ONE
  active quality generation for the live residency set.**

Residency must be shown to *fall* after a transition, not merely to stop rising.
`[image-census]` already reports totals; use it rather than building telemetry.

## Three small fixes, deliberately kept separate

Not part of this architecture and not allowed to shape it:

1. suspend/resume `.take()` before its guard (`host/platform/android.rs`);
2. the `player_robot_spritesheet` phantom in the regen postcondition;
3. stale `SheetRegistry` manifests — *"the survivor will crop with the wrong
   grid"*.

Related: [`dev/journals/android-what-an-agent-cannot-see-2026-08-08.md`](../../dev/journals/android-what-an-agent-cannot-see-2026-08-08.md)
