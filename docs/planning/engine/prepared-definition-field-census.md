# Prepared character definitions — per-field dependency census

**A6's hold, verbatim: "make a field/use census before moving types."** This page
is that. No type has moved and no split is proposed; the frontier says the hold
is released by the census, so the census is the deliverable.

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
