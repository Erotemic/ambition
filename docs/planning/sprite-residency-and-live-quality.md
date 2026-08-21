# Sprite residency and live quality Apply

**Status: step 1 LANDED 2026-08-08; steps 2–5 planned.** Jon's ruling plus a GPT
5.6 architecture brief he forwarded. This file is the plan of record; the ledger
row points here.

**Step 1, as built.** `CharacterSpriteAsset` carries the tier it answers;
`CharacterSpriteAssets` keeps its declarations past the decode so `Declared` is
reachable from `Ready`; `character_runtime::converge_character_residency_to_active_quality`
retires every stale realization the engine owns and re-demands it, one system
before the materializer. Presentation stamps `BoundSpriteQuality` from the
realization instead of from the active setting, which is what lets a body
converge on a LATER frame than the one the table changed on — the new pages are
`asset_server.load`ed and land several frames after Apply. The app's
`reload_visual_quality_assets_on_scale_change` no longer rebuilds the character
table (it was replacing it with 140 declarations and zero residents, leaving
nothing for the transition to notice, and silently deleting props and
`publish_under` art on the way).

Two things step 1 deliberately did NOT converge, both step 2's: per-`Prop.kind`
sheets, and realizations a host published itself — the engine has no recipe to
remake either, so retiring them would be a one-way deletion.

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

1. ~~**Tier stamp + return edge + Apply transition** for character sprites. The
   core, and it is the slice that makes the feature true.~~ **Done 2026-08-08.**
   ⚠ one thing the design above did not anticipate: the stamp must be the tier a
   realization ANSWERS, not the tier its bytes came from. Not every sheet has
   every variant baked (a fresh clone has none), so a `Half` budget legitimately
   loads a full-res PNG — and stamping that `Full` leaves it permanently unequal
   to the active tier, so the transition rebuilds it every frame forever.
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

## Jon's "quality change swapped my character" report — SIX causes eliminated

> *"When I change the video quality in ambition, my sprite went from the robot v3
> character to the robot v2 character."*

Three were eliminated before (missing art; a missing `_actor.ron` sidecar; a
per-tier sheet collision on a rig target). Three more, measured 2026-08-20 — all
by replicating what the RUNTIME does rather than by reading intent:

| # | eliminated | how |
| --- | --- | --- |
| 4 | the tier sheet INDEX picks the wrong file | replicated `record_index`'s keying over all four tier directories: `player_robot_v3.{0_5x,0_25x,potato}` each resolve to `player_robot_v3_spritesheet.ron`, and **zero** keys collide anywhere in the index |
| 5 | two characters sharing a DISPLAY NAME | `CharacterSpriteAssets::declare` maps BOTH the id and the display name to the id and is LAST-WINS, so a shared display name would make the quality transition re-demand the wrong character. Measured: of 127 catalog rows, **zero** display names are claimed twice, and none equals another row's id |
| 6 | a row whose PNG and MANIFEST name different sheets | `resolve_variant_pair` asks the CATALOG for the variant PNG (from `spritesheet`) and the SHEET INDEX for the variant spec (from `manifest`); a row where those roots differ would load one character's pixels with another's grid at reduced quality only. Measured: **zero** of 127 rows split them |

⚠ **cause 5 is eliminated by CONTENT, not by construction, and the hazard is
still latent.** `declare` really is last-wins, and nothing stops a future row
from reusing a display name. If this report ever reproduces, check that first.

▢ **what is left is the re-materialization ORDER** — `3bf154974` (2026-08-08)
made a quality change re-materialize on-screen bodies instead of only the next
room, and `demote_stale_realizations` returns catalog IDS derived from
`declared[token]` while the retired entries are TOKENS. That mapping is exactly
where a body could be re-demanded as a neighbour. ⛔ and it is not reproducible
by inspection — the next step is a live Apply with `[image-census]` on, which is
what this plan's verification section already asks for.

## Three small fixes, deliberately kept separate

Not part of this architecture and not allowed to shape it:

1. suspend/resume `.take()` before its guard (`host/platform/android.rs`);
2. the `player_robot_spritesheet` phantom in the regen postcondition;
3. stale `SheetRegistry` manifests — *"the survivor will crop with the wrong
   grid"*.

Related: [`dev/journals/android-what-an-agent-cannot-see-2026-08-08.md`](../../dev/journals/android-what-an-agent-cannot-see-2026-08-08.md)
