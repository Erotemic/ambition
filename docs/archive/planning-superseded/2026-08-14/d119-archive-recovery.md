# D119 — recovering the work archived mid-flight on 2026-08-13 (DISCHARGED)

⚠ **EVIDENCE, NOT AUTHORITY.** This is the closed measurement record: each item
archived on 2026-08-13 was re-measured against HEAD, and the recurring result was
that a DIFFERENT campaign had already deleted the thing the item was waiting on.
⛔ do not reconstruct a deleted representation because this file names it.

- ▢ **D119 — Recover the work archived mid-flight on 2026-08-13, and tell Jon his
  guard has three checks that cannot fail.**

⛔⛔ **three of the run's thirteen goal checks grep files that DO NOT EXIST**, so
they pass vacuously and always will:

```text
! grep -q '▢' docs/planning/authority-convergance-campaign-2026-08-13.md
! grep -q '▢' docs/planning/overnight-campaign-2026-08-11.md
! grep -q '▢' docs/planning/character-template-architecture-2026-08-10.md
```

All three were archived under
[`../archive/planning-superseded/2026-08-13/`](../archive/planning-superseded/2026-08-13/)
in that day's cleanup. `grep` on a missing path fails, `!` inverts it, the check
reports satisfied. The goal PREAMBLE meanwhile still names all three by their
live paths and calls the first one the execution spine — which is why that
ordering has read as inapplicable to every session since.

⭐ **RE-VERIFIED against `.goal/active.json` on 2026-08-14 and REPORTED TO JON
IN-SESSION.** Checks `[0]`, `[4]` and `[5]` still name the three missing paths;
the other ten are live. ⛔ **the guard's own config was deliberately NOT edited by
the agent it judges** — quietly rewriting your own success criteria is not a
repair, and this is the maintainer's call. The remaining cost is the PREAMBLE,
not the checks: it routed the start of a fresh session to a campaign file that
`5e382342d` deleted when AC7 closed and the Engine 1.0 program superseded it.

⚠ **and the archive was not empty when it was made.** The authority campaign and
the structural-bug-classes file were genuinely complete (0 `▢`). The other two
were not: `overnight-campaign` carries 8 and `character-template` 5. Most of the
overnight ones are `▢` mentioned INSIDE rows whose status cell already says ✔/◐,
so that count overstates; the character-template ones are real bullets in
**appendices C and D, which Jon's brief says outrank its phase table**:

**Measured against HEAD 2026-08-14, and TWO of the four were already landed** —
which is this ledger's oldest lesson arriving on schedule:

- ✔ **Prepared completeness (ruling 8) for death traits — COMPLETE.** The
  archived item answers itself: *"this item does not need a flip; it completes
  when the legacy road goes."* That road was `adopt_character_intrinsics`, and it
  has **zero references** at HEAD (AC5.3). The character-first road reads
  `definition.death_traits.clone().unwrap_or_default()` — `None` means the
  character said nothing and takes the default, which IS ruling 8 — and both
  remaining `death_traits: None` sites are definition builders saying exactly
  that. ⭐ nobody re-read the item after a DIFFERENT campaign deleted what it was
  waiting on.
- ✔ **`PreparedSeat::character_id` string-typed — LANDED.** It is
  `ambition_entity_catalog::CharacterId`, and its doc says it was typed for the
  same reason the participant's was (P0.3), with `Borrow<str>` keeping the `&str`
  lookups working. The archived bullet predates its own fix.

⇒ **what genuinely remains, both in appendix D:**

- ✔ **`EnemySpawnSpec::character_id` is REQUIRED — DONE 2026-08-14.** The
  expensive half of the item was stale: it was deferred because *"the field is
  genuinely absent on unmigrated entities, so making it required is a content
  migration"*, and measuring every `.ldtk` in the repo — content, both demos, the
  `ambition_map_assets` submodule — found **184 `EnemySpawn` entities, 0 without
  an id**. AC6.1 had already deleted the archetype road absence used to fall back
  to, so the migration finished as a side effect and nobody looked.

  Landed as a compiler-driven change: `Option<CharacterId>` → `CharacterId`,
  `EnemySpawnSpec::new` takes the character, the LDtk lowering REFUSES an
  authored entity that names none, `presentation_identity` lost its `name`
  parameter and its display-name fallback, `gameplay_character_id` lost its
  `Option`. **Deletions:** `impl From<CharacterBrain> for EnemySpawnSpec` (it
  said a brain alone is a placement, which is what the required field refutes);
  the construction panic that asked "and no character?"; and
  `only_the_uncast_placements_still_ride_the_display_name_fallback`, on its own
  written instruction — *"when the type makes the absence unrepresentable, delete
  this test with the fallback."*

  ⭐ **and the STAGED request followed it, which the compiler drove.** The two
  `expect`s the authored change forced at the staged boundary were the tell:
  `SpawnActorKind::Enemy::character` kept an `Option` whose doc justified it as
  *"`None` keeps the archetype road"* — a road AC6 deleted. Requiring it too, and
  then deleting whatever the compiler reported dead, removed
  `ActorConstructionError::BodyNamesNoCharacter` + its `Display` arm + the
  preflight arm that raised it, both `expect`s, `PlannedBody::named_by`,
  `PlannedBody` itself, and `Platformer2dSimHarness::spawn_enemy_at` (whose only
  difference from its sibling was passing `None`, with zero callers). None of
  those were sought — each was reported once the thing above it went.

- ✔ **`WornCharacter` → universal `CharacterIdentity` — UNBLOCKED; what remains
  is the RENAME, and that is Jon's.** Both stated blockers were measured at HEAD
  and neither survives:

  - *"the 14 characters that cannot build a body from their own definition still
    fall back to the catalog, so enrolling everybody hands them the catalog's
    kit"* — that population went **14 → 7 → 0 on 2026-08-13**, and its ratchet
    was deleted on its own instruction when it hit zero. Seven were vitals Jon
    settled; the last seven were each missing exactly one fact (locomotion).
    Nothing falls back, because nothing is incomplete — and construction now
    refuses an incomplete character at preparation rather than patching it.
  - *"`catalog: Res<CharacterCatalog>` is also still a REQUIRED resource"* — ⛔
    **and it must stay required.** `PlatformerAssetsPlugin` PANICS when the
    catalog is absent, deliberately — its comment says *"the silent version is
    an empty catalog and a world drawn as coloured rectangles, which is the
    exact failure this plugin exists to end"* — so every composition installs
    one on purpose. Making this reader `Option` would reintroduce precisely that
    silence. Filed as a smaller thread; it is a thread that must not be pulled.

  ⇒ nothing engineering-side is holding it. The overnight campaign's P1.6 row
  already says the substance is done and *"WHAT IS LEFT IS THE RENAME, AND IT IS
  JON'S"*.

  ⛔ **NOTED, NOT ASKED, AND NOT BLOCKING (2026-08-14).** Jon's standing rule:
  *"you put these questions down as notes and work around them. YOU NEVER stop an
  autonomous session to ask me questions"* — a blocking question converts the rest
  of an unattended run into idle time, which costs incomparably more than the
  question is worth. ⇒ **the substance of this row is CLOSED; only a cosmetic name
  is open, and it is a one-command reversal either way.**

  ⭐ **and the recommendation is to KEEP `WornCharacter`.** The rename was proposed
  before D73 settled, and D73's own conclusion argues against it: a character is a
  **reusable authored template a body WEARS**, not an innate identity — which is
  exactly what `WornCharacter` says and what `RecharacterizeBody` (the verb that
  changes it) presupposes. `CharacterIdentity` would quietly assert the thing D73
  spent a campaign refuting. ⚠ an agent should not rename *away* from a name that
  encodes a settled model on its own initiative; that is the one direction with
  asymmetric downside.

⛔ **verify each against HEAD before working it** — this ledger's oldest rule, and
these rows are four days stale in an archive nobody reads. Promote what survives
as its own row; delete what landed.

