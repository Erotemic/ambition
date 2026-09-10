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
| `ambition_combat` | 4 | action execution |
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
* `ambition_combat` → `authored_moveset`, `kit`, `ranged_execution` … **and
  `display_name`**, which is presentation. That one field is the odd member of an
  otherwise clean execution slice and is worth looking at before anything moves.

⚠ **`ambition_app_tools` READS ELEVEN FIELDS INCLUDING `kit`, `vitals` AND
`movement_tuning`.** Tool binaries reach into mechanical values, so any narrowing
of the definition's public surface breaks the tools first. They are binary roots,
so nothing can be composed away from them — but they are also where a change is
noticed last.

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
