# Character-identity authority inventory — the Campaign 1 baseline

**Why this file exists.** The architecture campaign's own rule is *"a campaign is
not successful merely because code moved. At least one relevant complexity metric
must decrease."* That is unfalsifiable without a number taken BEFORE the work,
and after a migration nobody can reconstruct one — the code that would have been
counted is gone. So step X1 is not "list the authorities", it is **commit the
counts** ([architecture-campaign-2026-07-28.md](architecture-campaign-2026-07-28.md),
revision R-e).

**Measured at** `34706cf39` (2026-07-28), immediately after the action-set
precedence slice landed and before `ResolvedCharacterIdentity` exists.

**Method**, so the after-number is comparable rather than merely smaller:

```sh
git grep -n '<symbol>' -- 'crates/**.rs' 'game/**.rs' 'fixtures/**.rs' \
  | grep -vE '_tests?\.rs|/tests/|tests\.rs'
```

Test files are excluded because the campaign is about PRODUCTION authorities; a
test naming a catalog is a test of the catalog, not a body assembled from one.
`⚠` Re-run the identical command — a different exclusion pattern silently makes
any campaign look successful.

---

## The numbers

| authority | refs | files |
|---|---:|---:|
| `CharacterCatalog` | 349 | 65 |
| `ActionSet` | 276 | 53 |
| `ActorMoveset` | 57 | 19 |
| `BodyPresentationSource` | 33 | 14 |
| `PreparedCharacterRegistry` | 32 | 8 |
| `PreparedCharacterDefinition` | 13 | 2 |

**Read the shape, not the totals.** `CharacterCatalog`'s 349 is not 349 competing
authorities — most of it is one crate's own implementation plus display-name and
art lookups that are legitimately the catalog's. The campaign metric that matters
is the third table below: how many places independently DECIDE between prepared
and catalog values.

## Where the prepared registry is actually read (production)

Eight files. This is the complete list, and it is the one that must shrink to
"preparation, plus one consumer" when Campaign 1 finishes.

| file | class |
|---|---|
| `crates/ambition_actors/src/avatar/starting_character.rs` | **production body construction** — the one writer for a worn body's persona. The action-set/moveset precedence lives here. |
| `crates/ambition_actors/src/character_runtime/seating.rs` | **production body construction** — match seating. |
| `crates/ambition_actors/src/character_runtime/presentation.rs` | **runtime behaviour** — per-body presentation source, cue authorization, sprite declarations. |
| `crates/ambition_actors/src/character_runtime/audit.rs` | **diagnostics** — legitimately outside the campaign. |
| `game/ambition_app/src/app/startup_loading.rs` | **preparation** — art demand at load. |
| `game/ambition_app/src/app/world_flow/room_transition_assets.rs` | **preparation** — room-scoped art demand (4 sites). |

## Precedence resolvers — the metric that must decrease

A "precedence resolver" is a production site that asks *"is there a prepared
value, and if so does it beat the catalog one?"* Every one of them is a place two
authorities can disagree.

| site | resolves | status |
|---|---|---|
| `apply_worn_character_kit` — action set | prepared vs `default_action_set` | **landed 2026-07-28** |
| `apply_worn_character_kit` — moveset | prepared vs catalog-derived | landed earlier |
| `apply_worn_character_kit` — `wears_host_code_kit` | prepared vs row `playable_kit` | **landed 2026-07-28** (was asking the displaced authority) |
| `seating.rs` — action set | writes `ActionSet::default()`, real kit arrives downstream | **open** — X9 |
| `presentation.rs` — provider/source | prepared vs catalog | open — X12 |
| motion model / movement tuning | catalog only | open — X2 (deliberately deferred, R-a) |
| identity baseline residue | catalog only | open — X8–X13 |

**Baseline: 7 sites, 3 resolved.** Campaign 1 is complete when this table has one
row — preparation — and the rest is consumption.

### Re-measured the same day, after X8–X11

| site | status |
|---|---|
| `apply_worn_character_kit` — action set | ✔ |
| `apply_worn_character_kit` — moveset | ✔ |
| `apply_worn_character_kit` — `wears_host_code_kit` | ✔ |
| seating — action set | ✔ via `project_prepared_character_definitions` (X9) |
| `presentation.rs` — provider/source | open (X12) |
| motion model / movement tuning | open (X2's successor slice, R-a) |
| identity baseline residue | ✔ — `IdentityKit` is the baseline and equipment overlays it (X11) |

**5 of 7 resolved.** The two open ones are the two the campaign deliberately
deferred, which is the honest reading: nothing was left half-done, and what
remains was scheduled rather than forgotten.

⚠ **the raw counts went UP, and that is not a regression.** `CharacterCatalog`
349 → 355 and `ActionSet` 276 → 286. Both are the new projection, the new
precedence branch, and their tests — code that RESOLVES an authority reads it
more, not less, right up until the displaced one is deleted. This is exactly why
the campaign's metric is the resolver table and not the reference count: a count
falls at the END of a migration and rises through the middle of it, so a campaign
judged on the count would look like it was failing while it worked.

**Method for the after-number: identical commands.** Re-run the `git grep`
pipeline at the top of this file. A different exclusion pattern silently makes
any campaign look successful.

⚠ **`seating.rs` is the interesting one and it is not a simple migration.** It
writes an empty `ActionSet` deliberately, with a comment saying the real kit
arrives from `apply_worn_character_gameplay` — "the ONE writer for a worn body's
moves, and seating must not author a second opinion about them". That reasoning
is correct and it is also why a seated fighter's kit is assembled in two passes.
Moving it to a resolved identity means seating reads the identity DIRECTLY rather
than writing a placeholder and waiting to be corrected, which is a real change to
the ordering, not a rename.

## What is NOT in scope, stated so it is not re-derived

* **Display-name and art lookups on `CharacterCatalog`.** The catalog is the
  right authority for "what is this character called" and "which sheet". The
  campaign is about the KIT.
* **`ambition_characters/src/actor/character_catalog/` itself** (6 files). That
  is the catalog's own implementation. It stays; the question is who reads it.
* **Diagnostics and audit paths.** They are supposed to see both sides.
