# Prepared character definitions — per-field dependency census

**A6's hold, verbatim: "make a field/use census before moving types."** This page
is that historical measurement. It does not claim a type move has landed.
The selected I1/I2 uses at the end guide the independent authoring boundary;
they do not authorize a wholesale character-domain split.

Measured 2026-09-10 against `2418dc369`, after A11's admission work landed
(`8dd6ea426`) so the preparation barrier is the one the census describes. The
instrument is a **deprecation seal**, not a text scan:

```bash
python3 scripts/measure_field_readers_by_seal.py PreparedCharacterDefinition
```

⛔ **A `.field` SCAN CANNOT ANSWER THIS AND IS NOT A CHEAPER VERSION OF IT.** The
names that matter here are `id`, `body`, `kit`, `mount`, `vitals`, `sheet`,
`provider` — words on half the types in this tree — so attributing an access
needs type inference, not a regex. `#[deprecated]` makes the compiler report
every use site in one pass, including reads through a codec, a `Reflect` path, a
macro or a helper three calls down. ⚠ A visibility seal does NOT work here: a
private field is an ERROR, so the nearest dependent fails to compile and
everything downstream is never built (measured on `GroundItem`: 8 sites against
248). A deprecation is a WARNING and the build completes.

**200 use sites across 27 fields and 9 consumer crates**, production only unless
stated.

⛔ **27 IS A CLAIM ABOUT THE INSTRUMENT, NOT ABOUT THE TYPE — `PreparedCharacterDefinition`
HAS 32 FIELDS.** The seal tags every `pub` field, so it is structurally blind to a
private one: `voice`, `cue_dependencies`, `vfx_dependencies`, `checked` and
`unresolved` are retained and appear in no run. None of the nine dual-read fields
is affected and no count below moves — but a reader who re-derives 27 and calls it
the field count will be wrong about the type, which is why the blindness sits
beside the number rather than only in the caveats.

## Who reads what

| crate | fields | role |
|---|---:|---|
| `ambition_characters` | **27** | the definition's own home — reads everything |
| `ambition_platformer2d_actor_monolith` | 15 | live runtime |
| `ambition_platformer2d_actor_spawn` | 13 | materialization / construction |
| `ambition_app_tools` | 11 | tool binaries |
| `ambition_content` | 9 | game content |
| `ambition_combat` | 3 | action execution (was 4; `display_name` left at `ced8b7f7c`) |
| `ambition_body_seed` | 3 | body seeding |
| `ambition_match` | 3 | match activation |
| `ambition_demo_smash` | 2 | ruleset |
| `ambition_sim_harness` | 1 | harness |

## What the census says

⛔⛔ **PREPARATION AND MATERIALIZATION ARE NOT ALREADY SEPARATED: NINE FIELDS ARE
READ BY BOTH THE SPAWN ROAD AND THE RUNTIME.**

```
both  autonomous_profile  death_traits  id  kit  motion_model
      mount  movement_tuning  provider  sheet
spawn only    body  held_item  hurtboxes  vitals
runtime only  display_name  locomotion  portrait  provoked_profile
              provoked_profile_id  ranged_execution
```

Nine of thirteen fields `actor_spawn` touches are touched again by
`actor_monolith`. A split that put "prepared definition" on one side and "live
materialization" on the other has to answer for those nine, and they are not
homogeneous: `id`/`provider`/`sheet` are identity and asset keys read at both
moments legitimately, while `kit`, `movement_tuning` and `motion_model` are
MECHANICAL VALUES read twice.

⭐ **AND POLICY DOES STRADDLE, WHICH IS THE FORK WORTH RECORDING.**
`autonomous_profile` — brain policy — is read by `ambition_characters`,
`actor_monolith`, `actor_spawn` AND `ambition_content`: four crates, at
preparation, at construction and at runtime. Its siblings do not:
`provoked_profile` and `provoked_profile_id` are read by the runtime alone.
⇒ So "prepared definitions vs live materialization/policy" is three parties, not
two, and the third one is unevenly spread. A6 was written against a two-way
split.

⭐ **THE CLEAN SLICES ARE AT THE EDGES, AND THEY ARE SMALL.** Two consumers read
a coherent, non-overlapping group and nothing else:
* `ambition_body_seed` → `body`, `locomotion`, `vitals`. Pure materialization,
  no policy in it.
* `ambition_combat` → `authored_moveset`, `kit`, `ranged_execution`. ✔ **CLEAN
  AS OF `ced8b7f7c` (2026-09-10).** It also read `display_name`, which is
  presentation and was the odd member of this slice. That field had exactly ONE
  reader in the workspace — `apply_worn_character_overlay`, which sets the body's
  `Name` — and that function already held both inputs the fallback needs, so the
  resolution moved to it with no plumbing added. `ambition_combat` now reads no
  `display_name` from a prepared definition. Re-measured after the move: three
  fields, this list.

⚠ **`ambition_app_tools` READS ELEVEN FIELDS INCLUDING `kit`, `vitals` AND
`movement_tuning`.** Tool binaries reach into mechanical values, so any narrowing
of the definition's public surface breaks the tools first. They are binary roots,
so nothing can be composed away from them — but they are also where a change is
noticed last.

⭐⭐ **MEASURED 2026-09-10 AT `36af83f89`: ELEVEN IS RIGHT, BUT NINE OF THE ELEVEN
ARE SERIALIZED, NOT COMPUTED WITH.** The paragraph above is true and its mechanism
is not what it implies. Keyed on the BINDING rather than the spelling:

| read | fields | site |
|---|---|---|
| **logic** | `kit` | `moveset_export.rs:649` and `moveset_takes.rs:716`, both `.projectable_moveset()` |
| **logic** | `portrait` | `moveset_export.rs:624`, into `portrait_for_declared_character` |
| **serialized** | `display_name`, `provider`, `vitals`, `locomotion`, `movement_tuning`, `abilities`, `mount`, `held_item`, `body` | all inside one `serde_json::json!` in `character_json` |

⇒ **`character_json` is a SERIALIZER.** It reads those nine to dump them. It does
not depend on what a `movement_tuning` MEANS; it depends on the field existing to
emit. ⇒ Moving one breaks a **JSON output schema**, not a tool's behaviour.

⚠ **THAT IS THE MECHANISM. WHETHER IT IS EASIER WAS A SEPARATE QUESTION AND IT WAS
ASSERTED BEFORE IT WAS MEASURED.** A serialized schema can be the HARDER thing to
change: a semantic read breaks at compile time in this tree, while a dump field
that stops appearing breaks whatever reads the JSON, silently, wherever that
lives. **Two things decide it, and both were then measured at `17c1f3e40`:**

1. **The bundle carries a schema id and a bump rule.** `moveset_export.rs:34`:
   `SCHEMA = "ambition.moveset_inspector.v2"`, documented on the line above it —
   *"Bump the version when a consumer would break."* Already at v2, so the rule
   has been exercised.
2. **The only consumer is inside this repository.**
   `tools/ambition_moveset_inspector`, fed by
   `tools/ambition_moveset_inspector/data/moveset_bundle.json` <!-- cite-ok: generated, never tracked --> — which is
   **generated and not tracked by git**, so no copy of it is held anywhere else.

⇒ **Both conditions hold, so the lighter ranking stands — on this evidence rather
than on the reasoning that first produced it.** ⛔ Had either failed, the nine
would have been the HARDER set, not the lighter one.

⛔ **AND THAT SPLITS THE THREE THIS PAGE SINGLED OUT.** Of `kit`, `vitals` and
`movement_tuning`: **`kit` is a genuine semantic dependency in two binaries**;
`vitals` and `movement_tuning` are dump fields. The gate is real for `kit` and
much lighter for the other two.

⚠ **TWO INSTRUMENT FAILURES PRODUCED THIS TABLE AND BOTH ARE WORTH THE SPACE.**
A first pass read ONE function's window and reported five fields, not eleven — an
UNDER-report, the direction that says "already clean". And `moveset_takes.rs:1136`
and `moveset_render.rs:889` bind `prepared` to a **`bool`** returned by
`move_exercise::prepare`; a search on the word rather than the binding counts them
as definition reads. ⇒ **A matching identifier is not the same value.**

## The nine dual-read fields, ranked — re-run 2026-09-10 at `420de5a04`

⛔ **RANKING ONLY. NO BOUNDARY IS PROPOSED HERE.** A boundary over 27 fields and
9 crates is not something a ranking decides. This gives the packet real numbers to
decide on.

Instrument: `scripts/measure_field_readers_by_seal.py PreparedCharacterDefinition`
— the **deprecation** seal, the same default the original run used.

| field | sites | consumer crates | the census's reading |
|---|---:|---:|---|
| `kit` | **23** | **7** | mechanical, read twice; **the semantic tool dependency** |
| `id` | 14 | 3 | identity key, legitimately both moments |
| `provider` | 13 | 6 | identity/asset key, legitimately both |
| `autonomous_profile` | 9 | 5 | policy, three moments |
| `death_traits` | 8 | 4 | |
| `movement_tuning` | **8** | **6** | mechanical, read twice |
| `mount` | 7 | 4 | |
| `sheet` | 5 | 4 | asset key, legitimately both |
| `motion_model` | **4** | **4** | mechanical, read twice |

⇒ **`kit` is first by both measures and by a wide margin** — 23 sites and 7 crates,
against 14 and 6 for the next. It is also the only one of the eleven
`ambition_app_tools` reads that drives logic rather than being serialized. **Every
way of counting puts it first.**

⇒ **`motion_model` is the smallest at 4 sites**, but across 4 crates — so it is
thin, not narrow. **A field with one site per consumer has no cheap side.**

⚠ **A TEXT SCAN UNDERCOUNTED BOTH OF THE ONES I CHECKED, AND UNDERCOUNT IS THE
DANGEROUS DIRECTION.** Keyed on the binding, a `.field` grep gave `motion_model` 3
and `movement_tuning` 6; the seal gives 4 and 8. ⇒ **An undercount makes a field
look like a cheap landing.** The page already said a scan cannot answer this; this
is what the gap looks like when someone tries anyway.

⭐ **AND THE TOTAL BEING UNCHANGED AT 200/27 IS NOT A COINCIDENCE.** `ced8b7f7c`
moved a `display_name` read out of `ambition_combat` and into
`starting_character.rs:274`. **A move is not a removal**, so the count holds and
the SITE changes — `worn_kit.rs` is gone from that field's list and the new line is
in it.

⚠ **THIS RUN REPRODUCES THE ORIGINAL; IT DOES NOT INDEPENDENTLY CONFIRM IT.** Same
script, same default mode. Two runs of one instrument agreeing is weak evidence —
proven here the same day, when two runs of a *grep* disagreed and the disagreement
was the useful result. ⇒ What this run does establish is **stability**: the same
instrument returns 200/27 across roughly 200 commits between `2418dc369` and
`420de5a04`. That is a fact about the reader set holding still, not a second
opinion about the method.

⭐⭐ **AND ON 2026-09-10 AT `0f63e6cc5` THE MEMBER LIST WAS DIFFED, NOT JUST THE
TOTAL.** The instrument's own last warning is that a stable total hides a moved
member, so a third run was compared to `420de5a04` site by site rather than
count by count. Across 57 intervening commits:

* the `(field, file)` key set is **identical** — nothing entered, left, or
  changed crate;
* per-field counts are identical, `kit` 23 and `motion_model` 4 included, so the
  ranking below stands unre-derived;
* 168 of 200 sites are on the same line, and the other 32 all moved **+6, all
  inside `crates/ambition_characters/src/prepared.rs`** — six lines added to
  `StagedCastRevision` at `:420`, above every shifted site and below none of the
  unshifted ones. An edit inside the owning crate, not a dependency change.

⇒ So the ranking is not merely un-recomputed; it is **measured unchanged**, and
this is the first run where "stable" means the member list rather than the total.

## The cheap end

Nine fields have **at most one consumer outside the owning crate**, so their
dependency is already a single edge:

| field | only consumer |
|---|---|
| `contact_damage`, `dream_seed`, `lineage`, `preserves_mirror_symmetry`, `ranged_vfx` | *(owner only)* |
| `hurtboxes` | `ambition_platformer2d_actor_spawn` |
| `practice_target` | `ambition_content` |
| `provoked_profile`, `provoked_profile_id` | `ambition_platformer2d_actor_monolith` |

At the other end, thirteen fields have four or more consumer crates, `kit`
highest at six.

⛔⛔ **THE CHEAP END IS NOT WORK — IT IS ALREADY CLEAN, AND SAYING SO IS THE
POINT.** A field with one consumer, or none outside its owner, has the dependency
shape A6 wants. There is nothing to move. ⇒ Re-read 2026-09-10 while looking for
the next landing after `display_name`: **none of the nine is misplaced.** The only
field this page identified as *in the wrong slice* was `display_name`, and it has
moved.

⇒ **So A6's remaining work is the nine DUAL-READ fields** — `autonomous_profile`,
`death_traits`, `id`, `kit`, `motion_model`, `mount`, `movement_tuning`,
`provider`, `sheet` — and that is a boundary proposal, not a cleanup. It is
gated by the `ambition_app_tools` constraint above, and `kit`, `movement_tuning`
and `motion_model` are mechanical values read at both moments rather than
identity keys legitimately read at both. **Do not expect another one-field
landing.**

## What this census cannot see

The script prints these on every run:
* a consumer carrying `#[allow(deprecated)]`;
* a `derive` on the struct warns inside the OWNING crate, so some
  `ambition_characters` rows are its own derive rather than a dependency;
* a consumer behind a feature this build does not enable;
* a workspace already red for an unrelated reason makes it an undercount.

⚠ It also does not distinguish a READ from a WRITE — a deprecation warns on both.
For "who writes this state", the sibling instrument is
`scripts/measure_state_writers.py`, and it has its own separate blind spots.

## Re-derived 2026-09-11 at `d7d8aaee4` — and the two-authority reading A6 asks for

Fourth run of the same instrument. **200 sites across 27 fields: unchanged**, and
every per-field count in the ranking above reproduces exactly — `kit` 23/7,
`id` 14/3, `provider` 13/6, `autonomous_profile` 9/5, `death_traits` 8/4,
`movement_tuning` 8/6, `mount` 7/4, `sheet` 5/4, `motion_model` 4/4. The nine
dual-read fields are still the same nine.

⛔ **AND THE MEMBER LIST COULD NOT BE DIFFED, BECAUSE NO RUN HAS EVER SAVED ONE.**
`802ce659b` added this page's own instruction — *"diff the SITES against your last
run, not the total"* — to the script's output and nothing was given to write the
sites to. The 2026-09-10 diff was possible only because two runs happened in one
session and one stdout was still on screen. ⇒ `--out` and `--diff` were added to
`measure_field_readers_by_seal.py` on 2026-09-11 and the reference lives at
`dev/prepared_definition_members.json`. **The next
re-derivation is one `--diff` instead of a full rebuild.**

### The 200 and the crate table are different populations

The run is `--workspace --all-targets`, so the 200 includes tests. Split by
`#[cfg(test)]` extent (brace-matched, not "first attribute wins"): **136
production, 64 test.** On the production population the crate table above is
exact for nine of its ten rows — `ambition_characters` 27, `actor_monolith` 15,
`actor_spawn` 13, `app_tools` 11, `body_seed` 3, `combat` 3, `match` 3,
`demo_smash` 2, `sim_harness` 1 — and **"9 consumer crates" is right.**

⚠ **`ambition_content` is the row that is wrong: ONE production field, not nine.**
Its single production read is `portrait` at `presentation/dialog.rs:973`. The other
eight sit inside one 1,237-line `#[cfg(test)] mod tests` in `character_catalog.rs`
— and one of those test rows is `a_character_states_its_policy_in_one_place`, the
content guard discussed below. ⇒ The crate is in the table because of a guard
against the condition this packet is about, which is the opposite of being a
consumer of the definition.

⚠ Two crates the table omits, `ambition_app` (8 fields, 22 sites) and
`ambition_demo_mary_o` (4 fields, 7 sites), are test-only readers for the same
reason. They are not missing from the production count; they are not in it.

### ⛔⛔ The dual-read axis cannot answer "two authorities", and here is why

**The seal measures readers of ONE struct.** Every one of the 200 sites reads the
same `PreparedCharacterDefinition`, so *by construction* no pair of them can be
two authorities. Reading the reader census harder cannot produce the distinction
this packet asks for. All sixteen production runtime reads were read by hand at
`d7d8aaee4`: every one resolves through `PreparedCharacterRegistry` or holds a
`&PreparedCharacterDefinition` handed to it. ⇒ **On the spawn/runtime axis, all
nine are two readers of one authority.**

⚠ And the crate axis is a proxy for the road axis, 15 of 16 accurate:
`actor_monolith/src/construction/mod.rs:1245` (`mount`) is materialization code
living in the runtime crate. Nothing turns on it here, but a boundary drawn on
crate membership would put it on the wrong side.

⭐⭐ **THE SECOND AUTHORITY IS A DIFFERENT PAIR, AND THIS REPOSITORY ALREADY NAMES
IT.** `character_runtime/audit.rs:106`, on `CharacterAuthorityConflict`:

> A disagreement between the prepared registry and assembled character catalog.
> **Both authorities are currently readable by different runtime paths**, so
> shared character ids must agree on identity, art, provider, and gameplay
> definition.

So the decidable question per field is not *who reads it* but **does the fact have
a second home in `CharacterCatalog`** — and if it does, is the disagreement
watched.

### The nine, ruled on that axis

| field | second home | watched by | reading |
|---|---|---|---|
| `kit` | — | n/a | **two readers of one authority; nothing to do** |
| `death_traits` | — | n/a | same |
| `mount` | — | n/a | same |
| `id` | the key both maps are keyed on | n/a | same |
| `provider` | `entry.provider` + `CharacterCatalogOwners` | ✔ `ProviderDisagreement` | two authorities, **already watched** |
| `sheet` | `entry.spritesheet` / `entry.manifest` | ✔ `SheetDisagreement` | two authorities, already watched |
| `autonomous_profile` | `entry.default_brain` → `catalog.autonomous_profile(key)` | ⚠ a CONTENT test, not the audit | two authorities, watched by the weaker thing |
| `movement_tuning` | `entry.axis_tuning` → `catalog.axis_tuning(id)` | ✖ nothing — **and correctly so** | ⛔ **SUPERSEDED, see the measurement below: FOLDED at the barrier, one authority** |
| `motion_model` | derived from `momentum` + `axis_tuning` → `catalog.motion_model_spec(id)` | ✖ nothing — **and correctly so** | ⛔ **SUPERSEDED, see below: FOLDED at the barrier, one authority** |

⭐ **AND THE SECTION BELOW REACHED THE SAME CONCLUSION INDEPENDENTLY, FROM THE
OTHER DIRECTION.** *"A value read at spawn and during simulation can legitimately
be one immutable definition; two reads do not require two authorities or two
copies"* was written as a migration RULE in `8ac8e1e9e`, in a commit this
measurement had not seen. The table above is that rule measured per field — and
it also names the three the rule does not cover, because they are not one
definition read twice.

⇒ **Six of the nine are fine as they are, and that is the result, not a deferral.**
`kit`, `death_traits` and `mount` have no second home; `id` is the key; `provider`
and `sheet` have one and it is audited. A type move would buy none of them
anything.

⇒ **The three that are not fine are not fixed by moving a type either.** They are
resolve-then-fall-back pairs in one function each — and ⛔ **for two of the three
this paragraph was WRONG, corrected by the measurement further down: the barrier
already folds them, so the fall-back is a duplicate spelling of one rule rather
than a second authority. `autonomous_profile` is the one that stands.**

```rust
// avatar/starting_character.rs:185-188 — movement_tuning
match registry.and_then(|registry| registry.get(character_id)) {
    Some(prepared) => prepared.movement_tuning,
    None => catalog.axis_tuning(character_id),
}
```

`motion_model` at `:151-157` has the identical shape, and its own comment already
states the rule the other two do not enforce — *"A prepared character already
folded its row in at the barrier, so falling back here for one would be the
displaced authority getting a second vote."*

⭐ And `npc_policy.rs:75` says the quiet part for `autonomous_profile`: *"a content
guard … already forbids a character authoring a profile while its row names a
preset, so this branch and that guard agree today. The guard is the belt; this is
the structure — **a rule that only holds because content happens not to violate it
is not a rule**."*

### ⛔ What this page does NOT propose

No generic resolver, no request bus, no new trait to unify registry and catalog —
A6's own text and A7's standing prohibition both apply, and the count of
authorities is not a thing to buy. **No type moves.**

### MEASURED 2026-09-11 — and NEITHER of the two answers the question had

The question was *"is the catalog fallback reachable — is any id in
`CharacterCatalog` absent from `PreparedCharacterRegistry` in a real
composition?"*, with a branch prepared for each answer: dormant ⇒ delete the
fallback, live ⇒ add the two fields to the audit. **Neither branch fires**, and
the reason corrects the two rows above.

Instrument: `build_visible_app(VisibleRenderMode::NoWindow, …)` — the shipped
host, headless, with `finish()`/`cleanup()`/`update()` in the order `App::run`
uses, because `combat_schedule.rs` records that a guard driving `update()` by
hand otherwise takes a different barrier road than production. **Both routes
measured** (`shell_hosted` true, the launcher, and false, straight to gameplay);
every number below is identical on both.

```text
catalog rows                                              147
prepared characters                                        58
catalog-only (ids that REACH the read-site fallback)       89
overlap                                                    58

catalog rows authoring `axis_tuning`     3   mary_o, mary_o_fire, mary_o_tall
catalog rows, non-default motion model   5   + sanic, super_sanic
of the 89 that reach the fallback,
  ids the catalog authors a value for    0
overlap: both author and DIFFER          0
overlap: catalog authors, registry mute  0   ← the direction that would be a bug
overlap: registry authors, catalog mute  3   smash_duelist_a/_b, smash_george_booul
overlap: motion model differs            0
```

⭐⭐ **THE FALLBACK IS REACHED 89 TIMES PER BOOT AND ANSWERS THE DEFAULT EVERY
TIME.** Reachable is not the same as consulted-for-an-answer, and the first
number — 89 of 147 — reads like a finding until the control is run. Every id
that reaches it is an id the catalog authors nothing for.

⛔⛔ **AND THESE TWO FIELDS ARE NOT TWO AUTHORITIES AT ALL: THE BARRIER FOLDS
THEM.** `crates/ambition_characters/src/prepared.rs:1338` is `movement_tuning.or_else(|| catalog?.axis_tuning(&id))`
and `:1334` is the same shape for the motion model. The registry is the catalog's
FOLD, which is why the overlap disagrees zero times — and it is the same
treatment `vitals.max_health` got, with the reason written at `:1316`: *"a
registered character's authored pool and a catalog row's authored pool were two
authorities that never met. Folding here is what lets ONE applier serve the worn
player and the seated fighter."* `abilities` two lines below says the converse —
*"Carried, not folded: nothing else in the engine can state a body's verbs, so
there is no second authority to reconcile with."*

⇒ **THAT IS WHY THE AUDIT COVERS EXACTLY `display_name`, `sheet` AND `provider`.**
Those three are carried on both sides and can disagree. `movement_tuning` and
`motion_model` are reconciled at the barrier and cannot. ⇒ **Correct the table
above: their row is "one authority, folded", not "two authorities, unwatched",
and adding them to `CharacterAuthorityConflict` would be a structurally green
variant — the machinery `AGENTS.md` says to refuse.**

⇒ **The residue is real but much smaller than a boundary: THE FOLD IS SPELLED
TWICE.** `avatar/starting_character.rs:157` and `:187` re-perform it at read time
for the ids the registry does not hold, and the measurement says that second
spelling changes the answer for none of them. Deleting it would leave the barrier
as the only place the rule exists.

⚠ **AND THE 3 THAT LOOK LIKE A DISAGREEMENT ARE THE AUTHORING ROAD, NOT A BUG.**
`Some(DEFAULT_TUNING)` against a silent catalog row, written where the character
is CONSTRUCTED (`demo_smash/src/lib.rs:4344`), whose own comment already files it:
*"eleven of the fourteen fighters on the grid still play on the ACTOR baseline —
a levelled stage where thirteen bodies are floatier than the fourteenth is half a
decision … Filed for a later slice."*

### The guard the deletion would have needed, and its poison

Deleting the read-site fold would be behaviour-preserving **only while every
catalog row that authors feel is one the barrier prepared** — and nothing asserted
that. It is now `game/ambition_app/tests/authored_feel_reaches_the_prepared_cast.rs`,
over the shipped composition, with a floor that fails if fewer than five rows
author feel so it cannot pass by losing its subject. ⚠ The guard is worth keeping
on its own terms, but the section below is the reason it was not enough: it
constrains real compositions and says nothing about the in-crate fixtures that
actually caught the deletion.

⭐ POISON-VERIFIED: `axis_tuning` added to `npc_busy_beaver` — a row in the 89 —
fails the guard naming that row, and removing it passes. ⛔ **And the restore
needed a `touch`**: `cp -p` puts the ORIGINAL mtime back, the catalog is embedded
at compile time, and cargo's mtime check saw no change and reused the POISONED
binary. A byte-identical restore confirmed by `md5sum` and a clean `git status`
still ran the poison. Verify a restore by RE-RUNNING, not by comparing the file.

### Every composition measured, and the deletion tried and REVERTED

The paragraph that stood here said the other compositions were unmeasured and the
deletion waited on them. They are measured now — all of them, not the two that
were named:

| composition | catalog rows | prepared | rows authoring feel | ORPHANED |
|---|---:|---:|---:|---:|
| shipped host, launcher **and** direct | 147 | 58 | 5 | **0** |
| `ambition_demo_mary_o` | 7 | 7 | 3 | **0** |
| `ambition_demo_twintrack` | 2 | 2 | 2 | **0** |
| `ambition_demo_sanic` | 3 | 3 | 2 | **0** |
| `ambition_demo_smash` | 3 | 3 | 0 | **0** |

Each built through its own `build_demo_app()` at default features with
`finish()`/`cleanup()`/`update()`. ⭐ **In all four demos catalog and prepared are
the SAME SET, so the read-time fold is not merely answer-free there — it is never
reached.** Only the shipped host reaches it, 89 times a boot, for ids whose rows
author nothing.

⛔⛔ **AND THE DELETION STILL DOES NOT LAND. SIX TESTS WENT RED ACROSS THREE
FILES.** Removing both fall-backs and the parameters they left dead broke
`a_definition_authored_motion_model_beats_the_catalog_row`,
`a_worn_body_carrying_no_moveset_is_still_given_its_persona`,
`gameplay_derives_from_worn_identity_at_add_and_on_change`,
`live_refresh::cross_model_rewear_preserves_shared_state_and_initializes_axis_private_state`,
`live_refresh::live_ability_sync_does_not_rederive_authored_movement_identity` and
`rewearing_an_equivalent_momentum_profile_preserves_live_ride_state`.

⇒ **They are not one fixture shape: they are the wear/re-wear road, which is the
road the fold actually serves.** One of them asserts the deleted behaviour in as
many words — *"an unauthored character stopped inheriting its catalog row"* — with
a comment calling it *"the migration path, and the half that keeps this safe to
put in front of every character at once."*

⇒ **Re-baselining six tests to land a duplicate-spelling cleanup is the
canary-versus-cage call going the wrong way, so it was reverted.** The production
measurement was right about production and is not a sufficient basis for the
change: what the deletion really decides is **what an UNPREPARED id should inherit
at wear time**, and that is a design question, not a cleanup. ⚠ Note also that the
app-level guard could not have caught this — it constrains real compositions, and
these fixtures are legitimately partial. The in-crate suite is what covered it.

⇒ **The open A6 question is now that ruling**, not a boundary and not an audit
variant. Until it is made, the fold stays spelled twice and the guard keeps the
orphan case from arising.

### ⛔ One sibling keeps its reference and the other does not

Found while answering whether a prepared definition can be re-staged (I2). Both
policy fields are authored either inline or as a named reference, and
`finalize_character` resolves both — but it retains only ONE of the references:

```rust
autonomous_profile: resolve_autonomous_profile(&id, &provider, autonomous_profile,
                                               autonomous_profile_ref.as_ref(), profiles),
provoked_profile:   resolve_autonomous_profile(&id, &provider, None,
                                               provoked_profile_ref.as_ref(), profiles),
provoked_profile_id: provoked_profile_ref.as_ref()
                        .map(|reference| reference.resolve_in(&provider)),
```

⇒ `provoked_profile_ref` survives as `provoked_profile_id`; `autonomous_profile_ref`
survives as nothing. **"Authored inline" and "named a profile" are distinguishable
after preparation for the provoked policy and indistinguishable for the autonomous
one.** That is the same asymmetry `autonomous_profile` already shows in this page's
table — it is the one dual-read field whose second home is watched by a content
test rather than the audit — and it is not obviously deliberate. It blocks nothing
today; it is filed here because a re-stage, a hot revision or a save codec each
need the distinction and none of them can recover it.

### ⛔ And the census counts 27 PUB fields of 32

`measure_field_readers_by_seal.py` tags every `pub` field, so it is structurally
blind to a private one. `PreparedCharacterDefinition` has **32 fields**: the 27
this page enumerates plus `voice`, `cue_dependencies`, `vfx_dependencies`,
`checked` and `unresolved`, all private to `ambition_characters`. None of the nine
dual-read fields is affected — they are all `pub` — but *"200 use sites across 27
fields"* is a claim about the instrument's population, not about the type.

## Selected uses for the independent authoring boundary

This census guides I1/I2; it does not require copying PreparedCharacterDefinition
wholesale into a portable schema or moving all its readers to one new crate.

| Field family | Selected responsibility | Migration rule |
| --- | --- | --- |
| id/provider and logical references | Pure identity where already appropriate | Preserve canonical meaning; no parallel identity algebra |
| authored moves, kit, motion model and tuning | Immutable mechanical definition with owner validation | Portable only through pure owned values; live application remains with the body/action owner |
| body/hurtboxes/vitals | Authored facts plus typed construction input | Separate from live body health/materialization; do not serialize ECS state as content |
| autonomous/provoked policy | Prepared controller policy and referenced identity | Preserve the same resolver at spawn, live selection and rewind; do not move brain execution into the compiler |
| sheet/portrait/voice | Logical asset/presentation references with declared mechanical dependencies | Collision-bearing metadata is mechanical even if an art tool produced it |
| checked/dependency inventories | Derived preparation/discovery evidence | Recompute from one validator; do not trust a serialized 'checked' bit as installed admission |

For each moved field, record preparation input, runtime reader, invalidation and
reload policy. Test a nondefault value through the consumer. A value read at spawn
and during simulation can legitimately be one immutable definition; two reads do
not require two authorities or two copies. Follow [generation/reload](content-generation-and-reload.md).
