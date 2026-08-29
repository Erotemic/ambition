# The 72-hour run, 2026-08-08 → 2026-08-11

**This file is the SPINE and the ledger `scripts/goal_guard.py` reads.** One of
its checks is `! grep -q '▢' docs/planning/queue-72h-2026-08-08.md`, so an open
row blocks the stop. That is the whole mechanism: **if every row here is `✔`, the
next item has not been WRITTEN DOWN yet** — go read `docs/planning`, find it, add
it, and get back to work.

This is intentional operating policy, not a temporary quirk of the August 8
run. **The queue is a continuation engine and is allowed to be inexhaustible.**
Finishing the rows currently written down means selecting the next highest-value
unresolved work, recording it here, and continuing. `tracks.md` is a reservoir;
focused plans own technical design; this ledger owns execution order.

Jon, 2026-08-08, setting this run: *"Restart the 72 hour goal … and have it
continue doing what you are doing, integrating the new SR code in
`untracked/ambition-twintrack-relativity-festival-overlay-2026-08-06-1538.zip`,
and then working on docs/planning starting with the new monolith crate
decomposition task, which takes priority after your current review task, the SR
unpack task, and the monolith decomp."*

So the lane order is **fixed by Jon**: A the review campaign in flight, B the SR
overlay, C the monolith decomposition, D everything else in `docs/planning`.

Rows carry their reasoning and their evidence. A row is a CLAIM ABOUT THE CODE
and goes stale like any other: **re-measure before working it.**

⛔ **THE COMMONEST STALENESS IS A ▢ ON WORK THAT ALREADY LANDED.** Before working
any `▢`, grep for the thing it says is missing. The previous run's ledger hit
this twice inside its own handoff text.

The previous ledger is [`../archive/queue-72h-2026-08-06.md`](../archive/queue-72h-2026-08-06.md)
(archived 2026-08-08, fully discharged); read it for the standing state
it recorded, not for its open rows.

⚠ **the guard's check is a bare `grep -q '▢'`, so the three mentions in this
header count too — it can never report clean, by construction.** That is
consistent with *"there is no completion condition"* and is not a bug to fix;
but it means **the hook's row count is not the number of open rows.** Read the
rows. And ⛔ **do not leave `▢` on a struck-through row** — a closed row keeps
its text and loses its marker, because `▢` is the only index a reader greps and
three false hits is how a real one gets missed.

---

## ⭐⭐ THE DAY'S PATTERN: one question, two answers, only one authoritative

Four bugs fixed 2026-08-08, found by four unrelated investigations, all the same
shape — and **all four violate a principle `decision-principles.md` already
states in Jon's own words**: *"an obvious source of truth … does not require
callers to remember hidden ordering rules"*, and *"avoids parallel paths,
compatibility shims, and duplicate mechanisms."*

| the two answers | which one ran | symptom |
|---|---|---|
| the same selection check at TWO lifecycle sites | only the first (the other was fixed 10 days earlier and unreachable) | **no game started at all** for any non-default character |
| `SimId` and `SimIdCounter` as two facts | only the id was minted | a shipped boss's summon did nothing |
| two app builders | only one installed the room visuals | the phone proxy photographed a void for two days |
| a drop's identity and its provenance | only identity | every coin drew as a magenta box |

⛔ **so the rule was not missing — the violations were INVISIBLE.** Each pair
looked like one fact in the file you were reading, and the second copy lived in
another layer. That is why three of the four were mis-diagnosed by static reading
and settled in minutes by a probe.

⭐ **three are now impossible by construction**: `#[require(SimIdCounter)]` welds
the pair, one builder removes the second composition, and the two lifecycle sites
point at each other. ▢ **the fourth is not** — nothing stops a runtime spawn
spelling `SimId::placement(..)`; that needs a `PlacementId` newtype so the wrong
namespace is unspellable, which is a 70-site refactor and is recorded as the
shape of the answer rather than started (see `engine/netcode.md`).

---

## ⇥ WHERE THIS STANDS (2026-08-08, updated in place)

⚠ this file is **3,200+ lines**; it grew that way in two days. Read this block,
then only the row you are working.

**OPEN ROWS — regenerated 2026-08-09 by the command this block used to tell you
to run.** The old hand-kept table said *"the six open rows"* and listed three
that were already ✔; it had rotted exactly the way its own footnote predicted.
⇒ **do not hand-maintain this. Regenerate it:**

```sh
grep -n '^- ▢ \*\*' docs/planning/queue-72h-2026-08-08.md | sed 's/\*\*//g'
```

**Regenerated 2026-08-09 (third time today — it rots within the hour).** Line
numbers drift on every edit; the ROW IDS do not — grep for `**D47` rather than
jumping to a line. ⭐ **regenerate with**
`grep -n '^- ▢ \*\*' docs/planning/queue-72h-2026-08-08.md`.

**⇥ REGENERATED 2026-08-09 (SEVENTH time today).** ⇒ **it rots in about two
hours. Regenerate from `grep -n '^- ▢ \*\*D'`; never patch** — patching it once
left two stale copies stacked below the live one. ⭐ **the curation is the point**:
the row headers already carry landed/open, so what this adds is *what a reader
should do next*.

| row | what | state |
|---|---|---|
| **D73** | ⭐⭐ **a character is a reusable TEMPLATE; delete the enemy-archetype system** | ▢ **NEW TOP PRIORITY, set by Jon 2026-08-10** — his brief verbatim in `character-template-architecture-2026-08-10.md`; answers D48, 8 phases, ~2,437 lines of legacy to delete |
| **D76** | Sanic's transform button read **"Fly Toggle"** | ✔✔ **ALREADY FIXED — verified 2026-08-10, filed so the CHARTER stops listing it.** `declare_sanic_techniques` closed it; the guard asserts the label AND that the word never returns, read in the `fly_toggle`-enabled state that produced it |
| **D75** | ⭐ Smash has **no knockback GROWTH** — Jon reported it and was right | ✔✔ **FIXED 2026-08-10** — growth is a DECLARED ruleset dial (`DeclaredCombatRules::knockback_growth`), Smash declares `0.01`, Ambition stays flat. A worn opponent now flies |
| **D74** | registering the PCA diverges a FALLING body's `vel.x` at **step 4** | ✔✔ **CLOSED 2026-08-13** — ran the probe the row asked for (loads per step, both builds): `staged=3 ready=0` at every step and the trail identical, because D73 made registration DECLARATIVE. The last of six hypotheses described a coupling that no longer exists. **The PCA is on `PLAYABLE_ROSTER`**; P5.40 landed with it |
| **D72** | ⭐⭐ **Smash is the engine test, not a mode with exceptions** | ▢ **TOP PRIORITY, set by Jon 2026-08-09** — brief verbatim in `smash-body-generic-combat-2026-08-09.md`; first slice landed (`78fffd933`, `aa52b3cce`, `af5dd1ced`), step 0 is STABILIZE |
| **D71** | ⭐⭐ rooms changed with **no transition transaction** | ✔ diagnosis holds · ✔ **the SILENT DOOR half FIXED 2026-08-10** — the cue rides the intent · ▢ the transaction half (preload / barrier / `GameMode` gate) still bypassed |
| **D70** | Mary-O blocks stay spent across a restart | ▢ **ONE EXPLANATION LEFT** — 2026-08-13 enumerated the crate: 5 resources, only 2 hold block state, BOTH rearm on the replay, and neither touches an entity ⇒ **no third state exists**. The pit route is tested and green. ⇒ it IS D68 wearing another face, and needs Jon's one word |
| **D69** | a promoted hidden block loses its picture | ✔ **FIXED** — a name in BOTH overlay lists is a REPLACEMENT; the removal half no longer wins. ⛔ the row's "added-block reconciler" was the wrong shape |
| **D68** | Jon's death→restart | ✔✔ **ALL THREE ROUTES GREEN** (`5fddacd76`, `4077cb2cc`) ⇒ ⛔ **needs one word from Jon: which GAME?** |
| **D66** | a ratchet test pinned a coincidence | ✔ **FIXED `af37438e1`** — third draft; *name both denominators* |
| **D64** | four more of Jon's observations, unfiled | INDEX only · ⚠ one is answered (1-2's tile texture is a taste call) |
| **D60** | the portal gun has D51's bug | ✔✔ **FIXED 2026-08-10** — the adapter spends the press it answers; no new state, no arbiter branch, touch button untouched |
| **D59** | `ambition_render` lib test cannot link | ✔✔ **RESOLVED 2026-08-10** — 101 passed from a COLD crate build; the shared dir was rebuilt. Jon's rust-analyzer row stays open as hygiene, never was the fix |
| **D56** | the renderer binds a sheet by DISPLAY NAME | ✔✔ **FIXED 2026-08-10** — art identity resolves first, name stays as fallback ⇒ **the D48 deadlock is broken**; D48 is now a pure content edit |
| **D54** | a slash with no owner | ✔ hazard fixed · ✔ art ruled out · ✔ **2026-08-13: ZERO raw `kin.pos` consumers in `ambition_render`** (the census the row asked for) ⇒ the sibling-overlay class is refuted, and the ring cannot see the anchor because an `Anchor` is a SPRITE COMPONENT, invisible to every other entity ⇒ ▢ **still needs one reproduction from Jon** |
| **D53** | the Android suspend `.take()` | ✔✔ decision **tested**, glue **typechecked** ⇒ ▢ only a device is left |
| **D52** | live quality Apply | ✔✔ **CLOSED 2026-08-13** — all three flagged defects fixed, and the prop stamp is now STRUCTURALLY unreachable: `refresh_prop_sprites_on_game_assets_change` no longer takes the quality profile at all. `resident_tiers()` reads `resolved_tier`. ⚠ the 193→527 MB question is a DIFFERENT row and is not closed by this |
| **D50** | the dropped weapon is session-scoped | ✔✔ ability drop fixed `481072760` · ▢ **Jon's laser-sword lifetime is still his** |
| **D48** | is an enemy a CHARACTER or an ARCHETYPE? | ✔✔ **ANSWERED by Jon 2026-08-10** — a reusable TEMPLATE, and larger than the fork asked ⇒ superseded by **D73** |
| **D47** | the drawing-vs-body function | ✔ falsifier run — a RIG project, not an engine one · ⛔ **its census is STALE in every number (2026-08-13)**: `character_id` 0→**63 of 65**, `mounted_on` 3→**7**, and **zero** rider-brained spawns lack a mount ⇒ *"the pirates no longer ride their sharks"* is authored now |
| **D46** | the unclaimed-body warning | ✔ **SPLIT LANDED `3fc23259f`** · ⛔ **its own hypothesis is dead** — see D71 |
| **D45** | sixteen observations from Jon | INDEX only |
| **D42** | the patent clerk's rig | ✔ answered — an editor fix if anything |
| **D36** | the `SheetRegistry` collision warning | LOW — same defect class as D46 |
| **D33** | the actor-monolith decomposition | ✔✔ **CARVE LANDED `a64bf22f8`**, path held at **12**, **56/56 crates priced** ⇒ ▢ `character_runtime` + `assets.rs` untouched |
| **D28** ×2 | compile/test time in real work | recorder ✔; rhythm answered |
| **D26** ×2 | the death-beat freeze instrument | ✔ **second fixture landed `51fa22de1`** — it measures something now |

⛔ **BLOCKED ON JON**: **D23's projectile precision** (⭐ new 2026-08-10, and it
is the one that now blocks code: does a bolt test the authored hurtbox rectangles
instead of the coarse box? it retires `strict_intersects` for shots and changes
how every one connects — and until it is answered, `HitTarget::UnresolvedFeatures`
cannot retire for bosses) · ~~D48's character-vs-archetype fork~~ (ANSWERED
2026-08-10 ⇒ D73) · the
measurement-submodule pointer · the rust-analyzer setting. ~~which game was the
death in (D68)~~ ✔ **resolved by Jon 2026-08-09**.

⭐⭐ **and one whole CLASS of question left the blocked list with it.** Jon,
2026-08-09: *"the concepts of what the engine needs to do things like hitstun,
knock back, techs, and other smash things are things that are objective and you
should not need my input … smash games are so well documented getting the
framework to make them work right and elegantly in this engine should not need
my input,"* and *"the same thing goes for mario (maryo). the mechanics of the
game are standard."* ⇒ **a documented genre's mechanics are RESEARCH, not a
maintainer decision.** Implement the standard mechanic; expose the numbers as
authored tuning so he can dial feel without touching structure. ⛔ do not file a
`awaiting-maintainer-decision.md` row asking what a genre already answers.


✔ **LANDED 2026-08-09**: D44 `afc36b390` · D39 `830d386ce` · D37 `3333a4b0f`
(**schema v20**) · D40 `7860e5c02` · D43 `adbf5f0ac` · D33 step 4 `d4b7423db` ·
D52's prop stamp · D54 · D57's spike tests · D49's guard.

⭐ **fourteen rows (D46–D59) filed this day; six close a Jon observation
diagnostically** and **none needed a device or a repro** — every one was read out
of the tree.

⛔⛔ **and SEVEN of the day's rows carried a WRONG mechanism that only died to a
probe** — D39 (three identity args, not two collapsed), D40 (`label_for` returned
`Some`, and my *correction* was wrong too), D43 (an absence grepped over two
guessed filenames), D46 (three refuted hypotheses, caught by a vacuity guard),
D51 (blamed `Auto`, then the equip path; the answer was a union), D55 (argued in
a circle and stopped), D59 (called it a stale cache through three cleans).
⇒ **a row's diagnosis is a hypothesis until something runs.** The ones that
survived contact were the ones with a falsifier written in *before* the work.

**Established today, so nobody re-derives it:**

* the bbox coupling is real and it is **four sites, not three** — and it does not
  fix *"way too big"*, which is a separate authored number;
* the `conversation` carve prices at **~1%**; `critical_path_crates` is right in
  hops and **2.2x wrong in seconds**;
* a cold build and a rebuild want **opposite fixes** (frontend wins 5.2x on a
  rebuild, which is the loop an agent pays);
* the death beat never cut the animation — 0.12s clip, 3.2s beat — and the freeze
  fixture **stopped measuring its own question** when the enemies gained dormancy;
* **zero** ignored tests hide a red; all **25** absence contracts can still fail;
* ⛔ four premises died on contact with measurement today, always toward the tidy
  answer. **The last one was mine and it lasted an hour.**

### ⭐⭐ ESTABLISHED 2026-08-09 — THIS REPO KEEPS SHIPPING CAPABILITIES WITH ZERO ADOPTERS

Three found independently in one session, each well-built, well-documented, and
reaching almost nothing:

| capability | adopters | found by |
|---|---|---|
| `authored_body_pixel_size` — the only thing that knows the drawing from the body | **1 caller**, the player-robot lineage | D47 |
| `EnemySpawn.character_id` — art identity separate from the label, added 2026-08-06 | **0 of 65** authored instances | D48 |
| `BodyMetrics::body_pixel_parts` — the disjoint-piece body union | **0 of 190** shipped sheets even emit the field | below |

⇒ **the recurring failure is not bad design, it is a landed mechanism nobody
wired to content.** ⭐ so when a row says *"the engine cannot do X"*, the first
move is not to design X — it is to grep for X and then **count its adopters**,
because "exists" and "reaches production" are different questions and the gap
between them is where these live.
⚠ and each one degrades silently by design: the display-name join still resolves
art, the alpha bbox still produces a box, the static bbox still sizes a quad. **A
capability with no adopters looks exactly like a capability that is working.**

* ✔ **and that settles the one concern GPT 5.6 raised but declined to promote**
  — that `body_pixel_extent` prefers static `body_pixel_parts` over the
  per-animation hurtbox while the docs give the per-anim form precedence. It is a
  real ordering question and **it cannot bite: `body_pixel_parts` appears in 0 of
  190 shipped sheets**, so that branch is dead on all shipped content. GPT was
  right to hold it; it lacked the sheets to decide. ⛔ **do not re-investigate.**
* ⚠ while checking it I nearly filed a worse claim: only **1** sheet
  (`player_robot_v3`) publishes per-animation hitbox polys, which looked like it
  would strand D44's fix for the other 189. It does not — `pose_body_bbox` ends
  with `.or(self.body_pixel_bbox)`, so the static bbox catches them and D44's
  "184 of 190" stands. **Read the fallback before reporting the hole.**

---

## ⇥ START HERE

- ▢ **D73 ⭐⭐ THE RUN'S TOP PRIORITY, SET BY JON 2026-08-10: A CHARACTER IS A
  REUSABLE AUTHORED TEMPLATE, AND THE ENEMY-ARCHETYPE SYSTEM IS DELETED.** The
  brief is long and it is HIS, so it lives verbatim in its own file rather than
  being paraphrased here:
  [`character-template-architecture-2026-08-10.md`](character-template-architecture-2026-08-10.md).
  **Read that file before touching character construction.** It answers D48 —
  as **(a), and then further than (a)**: the endpoint is ONE character
  authority, not `CharacterCatalog` for half the facts, `PreparedCharacter
  Definition` for another half and `ArchetypeSpec` for a third set selected
  through a field called `brain`.

  The sentence that decides every judgement call inside it: *"A character is a
  reusable authored template, not a singleton person."* `spawn Goblin` three
  times and `spawn Fretjaw` twice are **the same engine operation**.

  ⛔ **the two failure modes he names explicitly**: do NOT migrate
  `ArchetypeSpec` into `CharacterDefinition` wholesale (it is a god-object
  holding three authorities — intrinsic body, controller policy, placement
  policy), and do NOT stop when the new path works beside the old one.
  ⚠ **the acceptance signal is a DELETION**: ~2,437 lines are obvious legacy
  (`ArchetypeSpec` 319, roster/enemies 1,198, `character_archetypes.ron` 845,
  `enemy_roster.rs` 75); a result of *+4000 new / −2400 old* means the old model
  was wrapped rather than removed.

  ⭐ **the phase table in that file is the resumption point after a compact** —
  update it as phases land, and a phase is `✔` only when its deletions happened.

  ⛔⛔ **A COURSE CORRECTION AND A SMASH ADDENDUM LANDED 2026-08-10 and they
  OUTRANK the phase table's earlier ordering** — appendices C and D of the same
  file, both verbatim. Two rulings a resumed session must not rediscover the
  hard way:
  1. `adopt_character_intrinsics` is a **probe seam, not the final model**.
     Growing it field by field only moves the god-object's precedence logic into
     a patch function. The identity/domain seam and the real common constructor
     come **BEFORE** group A's content migration, not after.
  2. **Smash is this row's proving ground, not a competing row.** D72 and D73
     are the same work seen from two ends. The acceptance demo is *"the same
     Fretjaw definition works in the Hall, in a hostile encounter, under
     possession, and in Smash, with only controller and contextual rules
     changing"* — concretely, remove `PreparedMatch`'s `CharacterRoster` /
     `ArchetypeSpec` dependency and watch how much of `smash_fighter_kit()` and
     `fighter_abilities` stops being necessary. ⭐ that hack exists because
     seven of twelve selected fighters are Hall NPCs whose catalog rows say
     `peaceful` — **a CONTROLLER fact recorded as a BODY fact**, the same error
     as `EnemySpawn.brain` deciding health, pointing the other way.

  ✔ **landed against the correction (2026-08-10), newest last:** the
  authored-enemy path stopped inferring gameplay identity from
  `sprite_character_id` and asks the placement (`e67468819`); the phase-3
  harness that was missing — an authored enemy built against a populated
  prepared registry — exists with the inversion's poison in it (`a235947ac`);
  the persona derive no longer silently drops a body that carries no
  `ActorMoveset` (`6c040d2a0`), and mints at most one (`bda9bc61e`);
  `CharacterDeathTraits` moved below `ambition_combat` so the definition stops
  reaching up (`5c708c409`); the moveset verb ids moved beside the contract
  (`4daad4691`); `BrainProfileRef` now distinguishes an authored reference from
  a resolved `BrainPresetId` (`bda9bc61e`); `CharacterId` is typed in
  `ambition_entity_catalog` (`049c36d74`).

  ✔ **`CharacterSpawnPlan` + `SpawnContext` LANDED** (`43152492b`) and the
  authored enemy lowers through them; `plan.definition(registry)` is the single
  place construction asks which character a body is. ⚠ it carries `character` +
  `context` only — `controller` and the autonomous-profile override were written
  and then removed because no current caller reads either, and they return with
  the callers that do.

  ⛔⛔ **THE REMAINING WORK IS NOW ONE ORDERED CHECKLIST** — 23 items in
  `character-template-architecture-2026-08-10.md` under *"REMAINING WORK — THE
  ONE CHECKLIST"*, each with the measurement that sizes it and the blocker above
  it. ⇒ **resume there, not from this row.** Landed since the correction: the
  identity/type layer, `CharacterSpawnPlan`, and the FIRST content migration —
  the two mites are off the archetype roster and `character_archetypes.ron` is
  smaller for the first time.

  ⭐ **THE NEXT THREE, in order:** (A1) decide whether `build_actor_moveset`
  follows the verb constants down, which is the last thing blocking the type
  move; (A3) a `BrainProfile` type, the single biggest unblocker left — the
  mites' rows cannot disappear without it, group B is entirely it, and the spawn
  plan's `autonomous_profile_override` has no reader without it; then (B6) route
  the NPC path through the plan, which is what
  brings the override member back with a reader. It is the one that makes the
  next ten cheapest because it is the second of six authoring surfaces, and the
  second is where a shared contract either proves general or is revealed as the
  enemy path wearing a new name.
  ⚠ **measured cost, so the next session does not rediscover it**: the NPC's
  identity is `character_id: Option<String>` on BOTH
  `ambition_interaction::InteractionKind::Npc` and the
  `ambition_entity_catalog::placements::InteractionKindSpec` mirror, with ~37
  construction/match sites across the workspace. Typing it as `CharacterId`
  (the same treatment `EnemySpawnSpec` already had) is the prerequisite, and it
  is a wide mechanical change that wants its own slice and its own full
  verification run — ⛔ do not start it with less than an hour.
  ⛔ **do not migrate content fields through `adopt_character_intrinsics` in the
  meantime**; that is the specific failure appendix C names.

  ⚠ **two things measured today that the next session must not re-derive**:
  `definition.rs` reaches into `ambition_combat` in exactly ONE place now
  (`build_actor_moveset`), so the type move is a design call rather than a
  survey; and `WornCharacter` cannot become the universal `CharacterIdentity`
  yet because attaching it enrolls a body in `apply_worn_character_gameplay`,
  which re-derives its kit THROUGH THE CATALOG — phase 2's endpoint arriving
  before phase 2.

  ⚠ **three facts this repo measured, so a resumed session does not re-derive
  them**: (1) **93 `EnemySpawn`s** across the four worlds — 28 already author a
  `character` id, 41 name a catalog character without one, 24 are role names;
  (2) **two spawn paths, two authorities** — `NpcSpawn` reads the catalog row's
  `default_brain` through `resolve_initial_brain`, while `EnemySpawn` reads
  `ArchetypeSpec` through `enemy_default_brain` and **never consults the catalog
  row**, which is the whole of Iron Mary's fireballs; (3) **D56 has landed**
  (`2d327f455`), so authoring character ids no longer un-arts the spawns it is
  meant to fix — the deadlock that made D48 unlandable is gone.

- ~~**D75 SMASH HAS NO KNOCKBACK GROWTH, AND JON IS RIGHT ABOUT IT.**~~ ✔✔
  **FIXED 2026-08-10.** The fix is neither of the two routes the row weighed.
  Growth is a **declared RULESET dial** — `DeclaredCombatRules::knockback_growth`
  beside `di_max_angle`, folded by `project_combat_rules`, read where the launch
  already resolves per victim. A volume that authors its own `knockback_growth` still
  wins outright; the ruleset speaks only for the swings that author none, which
  is every prefab-derived attack in the game.

  ⭐ **why a ruleset dial rather than per-move data**: the value is a FRACTION of
  each move's own base launch per point of damage, so one number makes a jab
  grow gently and a smash grow hard. A per-move table would restate that for
  every move, and widening the five melee specs would have touched 47 sites to
  serve one consumer. ⛔ and authoring a moveset instead does NOT work — the
  prefab-derived attack family is merged ON TOP of an authored signature
  moveset by `build_actor_moveset`, so an authored `attack` is overwritten.

  Smash declares `0.01` (a hit doubles its launch at 100%); Ambition and the
  generic versus stage declare nothing and stay flat, because versus ends on
  health rather than a blast zone. Four unit tests including a parity case and
  a weight case, poisoned; the stage test now refuses a zero growth for the same
  reason it refuses a zero DI budget.

  ### ⇥ the original diagnosis, kept because it is what made the fix obvious

  ⛔ no marker on the text below — a `▢` here is what a sweep greps, and a
  closed row wearing one is how a finished item gets worked twice.

  **Smash had no knockback growth, and Jon was right about it.** His
  observation *"in smash there does not seem to be any knockback"* was marked
  answered by the combat campaign and should not have been. The ENGINE is
  complete — `scaled_knockback` grows launch by the victim's percent and weight,
  `reaction_scale` scales hitstun and hitlag off the resulting launch, DI steers
  it — but the duelists never reach any of it:

  * their kit is `action_set_presets: { "duelist": … melee: Some(Swipe(…)) }`
    (`game/ambition_demo_smash/src/lib.rs`), which lowers through
    `attack_move_from_melee` → `simple_melee`;
  * `simple_melee` hardcodes `knockback: 120.0` and `knockback_growth: 0.0`, with its
    own comment saying *"prefab swings are flat-knockback; percent growth is
    authored on explicit RON volumes (CM1) — a prefab growth param can follow"*;
  * ⛔ and `scaled_knockback` returns `base` **immediately** when growth is zero.

  ⇒ **a hit at 150% launches exactly as far as a hit at 0%.** Percent
  accumulates and moves nothing, which is precisely what he reported.

  ⭐ **the fix is authoring, not architecture, and the route matters.** Do NOT
  widen the five melee spec structs (28 Rust literals + 19 authored `Swipe(`
  rows) to carry launch — that is a large mechanical change for one consumer.
  The duelists should author a real MOVESET, which the engine already supports
  per-volume (`knockback`, `knockback_growth`, `launch_dir`) and which already outranks
  the derived prefab. A prefab growth param can follow when a second character
  wants one. ⚠ knockback growing with percent is documented genre standard, so
  per Jon's 2026-08-09 ruling this is research, not a decision to put to him —
  the NUMBERS are authored dials he can retune.

- ✔✔ **D76 JON'S "SANIC'S TRANSFORM BUTTON STILL READS 'FLY'" IS FIXED AND
  GUARDED — verified 2026-08-10, filed only so the charter stops listing it.**
  The run charter names this as one of three still-open observations. It is not:
  `declare_sanic_techniques` (`ambition_demo_sanic/src/lib.rs:1429`) exists
  specifically to close it, and its doc states the mechanism — the
  transformation read the RAW Utility edge, so Sanic's derived scheme still
  carried the engine's `fly_toggle` movement action in that slot and the button
  wore its label.

  ⭐ **the guard is the shape a guard should have.** `the_utility_button_reads
  _transform_and_never_fly` asserts the label IS `"Transform"`, and then asserts
  separately that it does not contain `"fly"` in any case — *"the poison, stated
  as the symptom rather than as its cause: whatever the label is derived from,
  the word Jon saw must not come back."* ⇒ a future refactor that re-derives the
  label from somewhere else still reds. It also builds the fixture with
  `abilities.fly = true; abilities.fly_toggle = true`, which is the state that
  produced the bug — the assertion is read in the case that can fail.

  ✔ `cargo test -p ambition_demo_sanic --lib` 78/78 on 2026-08-10.
  ⛔ **do not reopen this**; grep `TRANSFORM_TECHNIQUE_ID` before believing any
  text that says otherwise. [[feedback_when_you_fix_it_grep_the_warning]] —
  this is the same class as D48's inverted warning, caught from the other side:
  a charter listing work that landed.

- ✔ **D75 SEVERAL HOSTS REACHED ENEMY CONSTRUCTION WITH AN EMPTY PREPARED CAST.**
  Measured 2026-08-11 while flipping D73's `CharacterSpawnPlan` warning into a
  refusal: at the moment an authored enemy is built, the
  `PreparedCharacterRegistry` contains **ZERO characters** in the multi-game
  shell host (`composes_through_the_sdk::the_second_mounted_experience_launches…`)
  and in the rollback door fixture (`door_entry::a_door_opens_under_a_rollback_host…`).
  Probed directly — the panic printed `registry has 0 ids: []` — so it is not
  "the character was missed", it is "nothing was published at all".

  ⛔ **the consequence is silent today and was invisible before the migration.**
  With a cast published, a placement naming a character builds that character;
  with none, EVERY placement falls back to its archetype, and a migrated
  creature whose row is gone becomes a generic `combatant` wearing its name.
  That is why Mary-O's and Sanic's demo roster rows were RESTORED hours after
  being deleted — they are the fallback those hosts still need, and they are
  dead weight the day this row closes.

  ⇒ the refusal is scoped to `!prepared.is_empty()` so a host that publishes no
  cast is not refused for somebody else's fault; the scoping is documented at
  the site and pinned by
  `a_composition_with_no_cast_at_all_falls_back_instead_of_refusing`.

  ⛔⛔ **ONE HYPOTHESIS IS ALREADY DEAD, and killing it cost an hour of
  self-deception worth writing down.** The obvious story — the barrier latches
  shut at `PreStartup` and a shell that mounts an experience later stages
  registrations nobody folds — was implemented three ways at once (re-close the
  barrier every frame, guard on emptiness instead of a latch, merge instead of
  replace). All ten tests went green and it looked fixed.

  ⚠ **it was not.** Poisoning each change one at a time left the tests green,
  which should have been the tell; poisoning all three together ALSO left them
  green. What had actually made them pass was scoping the refusal to
  `!prepared.is_empty()`. And the probes that "proved" the fix printed nothing
  because libtest captures stdout for PASSING tests — the output only appears
  with `--nocapture`, which is the whole reason a green run read as evidence.

  ⇒ re-measured with `--nocapture`, with and without all three changes: the
  registry is **0 ids at spawn time either way**. The three changes are reverted
  ([[reference_causal_instrument_gotchas]]: an instrument that cannot say no).

  ⭐⭐ **THE CAUSE, found 2026-08-11 by wiring the programmatic path**: in those
  hosts the `PreparedCharacterRegistry` **resource does not exist at all**. Made
  visible by taking it as a required `Res<..>` in `apply_spawn_actor_requests` —
  Bevy refused the system by name: *"Parameter `Res<PreparedCharacterRegistry>`
  failed validation: Resource does not exist"*. Every seam that reads it takes
  an `Option` and reads absence as an empty cast, so the state was unobservable
  from inside.

  ⇒ so it is not a race and not a second registry: those compositions load
  Ambition's ROOMS without registering Ambition's CAST. A room whose placements
  name characters, in a host that has none, is the actual inconsistency.

  **Where to start**: `ActorPlacementContext::prepared` defaults to an empty
  registry and is filled by `.with_prepared(..)` only when the caller has one —
  `spawn_actors.rs`'s `prepared_characters: Option<Res<..>>` and
  `spawn/mod.rs`'s `construction.prepared`. Both are `None` in these hosts.
  Decide whether such a host is legitimate (a fixture that wants rooms without a
  cast) or a composition bug, and make the answer explicit rather than an
  `Option` that reads the same either way.

  ⇥ ◐ **THE DECISION IS MADE AND IT IS ALREADY IN THE CODE** (found 2026-08-12
  by reading the seam rather than the row). `report_unprepared_character` has
  branched on `prepared.is_empty()` since P0.1 landed, and its comment states the
  ruling this row was waiting for: *"absence is legitimate —
  `CharacterPreparationPlugin` is installed by `try_register_character`, so a
  host that registers nobody never publishes, and 'no cast' is exactly what that
  means. What must not happen is a room full of character-named placements
  quietly becoming generics with nothing said about WHY."* The two facts are said
  DIFFERENTLY now: an empty cast reports a COMPOSITION gap once, naming the
  fallback; a non-empty cast missing one id reports a borrowed character. ⇒ the
  `Option` reading the same either way is no longer an accident — `None` and
  `Some(empty)` mean the identical thing (nobody registered), and that is the
  answer, not a gap.

  ⇥ ⭐⭐ **AND THE 0-ID FINDING IS STALE — MEASURED 2026-08-12, out of the
  finished world.** ⛔ a green run could not have said so: `bevy::log::warn!`
  prints nothing without a `LogPlugin`, so the composition-gap warning is
  invisible to a test, which is the same class of trap this row was fooled by
  once (libtest capturing a passing test's stdout made three reverted changes
  look like a fix). So the registry was asked directly instead. The shell host
  (`PlatformerApp::headless().mount(SanicGame).mount(MaryOGame)`) publishes a
  registry that **exists and holds EIGHT ids** — `ai_slop`, four Mary-O bodies,
  three Sanic bodies. Not zero, and not absent.

  ⇒ so for that host the answer is the ruling, not a bug: a composition publishes
  the cast the games it MOUNTED register, and this one never mounted Ambition.
  `a_two_demo_host_publishes_exactly_the_cast_its_demos_register` pins both
  terms — the mounted demos' protagonists are present AND `player_robot_v3` is
  absent, because a test that only checked presence would pass on a host that
  published the entire workspace.

  ⇥ ✔ **AND THE DOOR FIXTURE PUBLISHES TOO** — same probe, same day.
  `the_rollback_door_host_publishes_a_prepared_cast` asks the GGRS sync-test
  harness's world directly and finds a registry holding Ambition's own cast,
  `player_robot_v3` included. Both terms again: non-empty AND the right
  protagonist, because a registry full of somebody else's characters would
  satisfy the first while describing a different bug.

  ⇒ ✔✔ **THIS ROW IS CLOSED.** Neither named host reaches enemy construction
  with an empty cast any more, the legitimacy question is answered in
  `report_unprepared_character` (a composition that mounts nobody publishes
  nobody, said once about the COMPOSITION rather than once per placement), and
  both hosts now carry a guard that asks the registry rather than trusting a
  warning nothing prints. ⚠ what made this row survivable for a day longer than
  it needed to was the instrument, twice: a `warn!` with no `LogPlugin` and a
  libtest run that captured its own probe. The measurement that settled it in
  both directions was reading the resource out of the finished world.
  ⚠ `ambition_app`'s own shipped composition DOES publish (300+ app_it tests and
  `character_provider_namespace` depend on it), so this is about the other
  hosts, not the game.

- ✔ **D74 A FIGHT THAT STARTS BEFORE ITS SHEETS LAND NEVER RECOVERS.** Found
  while reading `PLAYABLE_ROSTER` for D73 phase 2 — recorded in
  `game/ambition_content/src/character_catalog.rs` as a known, *"unexamined"*
  finding, which is why it is a row rather than a discovery: **a combat geometry
  resolved from a missing sheet appears to STICK for the life of the body.**
  Measured there: with one extra sheet in flight, `duel_arena_room_is_a_real_
  neutral_attack_defense_fight`'s two fighters throw **zero melee for a
  sixty-second bout**; settling 180 frames first turns it into a real fight
  (melee 4).

  ⚠ **the cost is live and visible**: `perfect_cellular_automaton` is held OUT
  of `PLAYABLE_ROSTER` purely to keep that instrument green, so the smash grid
  is one portrait short of the roster it advertises. ⛔ the comment there is
  explicit that this is *"a WORKAROUND holding a fragile instrument green, not a
  statement about the PCA"*, and that a longer settle alone is not the fix — it
  just moves the failure to the robot's shield count.

  ⇒ **why it is worth a row now**: D73 phase 2 wants the PCA registered like
  every other character, so this workaround has to come out, and it cannot come
  out while the underlying stickiness stands.

  ### ⇥ PROBED 2026-08-10 — the stated reason NO LONGER HOLDS, and what fails
  ### instead is a different and weaker thing

  ⭐ **the row's own suggestion was answered first and came back clean**:
  `sync_sprite_posed_bodies` re-derives collision, quad and base size from the
  sheet **every tick**, and skips (`continue`) only while the sheet does not
  resolve — so a body whose art arrives late heals itself. Geometry is not
  sticky. ⇒ the "sampled once at spawn" theory is refuted.

  Then the workaround was tested directly: `perfect_cellular_automaton` added to
  `PLAYABLE_ROSTER`, `duel_arena` re-run.

  ```text
  duel_pca_body_is_sprite_authored_not_the_tiny_ldtk_box     ok
  resetting_the_room_restages_the_duel_fighters_fresh        ok
  duel_arena_room_is_a_real_neutral_attack_defense_fight     ok   ← the test the
                                                                    workaround
                                                                    exists for
  duel_fighters_actually_enact_their_abilities_on_the_body   FAILED
      robot: flight must engage on the body (regroup high-ground) (got 0 frames)
  ```

  ⛔⛔ **so the comment in `character_catalog.rs` is now WRONG in its specifics.**
  It says the extra sheet makes both fighters *"throw ZERO melee for a
  sixty-second bout"* — that test **passes** with the PCA registered. What fails
  is a different assertion in a different test, about a RARE EMERGENT behaviour:
  the robot's damage-triggered high-ground regroup never fires in 1800 steps.
  ⚠ and note WHOSE: the *robot's*, not the PCA's. Registering one character
  changes the trajectory of the other's fight.

  ⇒ **this is a fragile-instrument problem, not a geometry one.** An assertion
  that a rare reactive behaviour occurs at least once in a 30-second bout is
  sensitive to any change in the fight's path, and registering a character
  changes that path. ⭐ the next step is therefore NOT to chase a caching bug:
  it is to decide whether `fly_frames > 0` can be made a claim about the BODY's
  capability and its trigger rather than about one emergent bout — the same
  move the row above it already made when *"the aggregate dash assertion is
  gone"* (queue F0e). Fix that, and the PCA goes on the roster.

  ⚠ **the old symptom may simply have been fixed since.** The comment dates from
  when it was written and nothing re-measured it until now; do not carry its
  sentence forward as current.

  ### ⇥ THE REPLACEMENT EVIDENCE EXISTS NOW (2026-08-10)

  ✔ `a_fly_capable_grounded_body_leaves_the_floor_when_it_toggles_flight` and
  `a_body_without_the_fly_kit_stays_on_the_floor_pressing_the_same_button`
  (`enemies/integration/dash_tests.rs`) prove the fly wiring directly, with no
  opinion about the AI's mood — the same shape as the dash pair that replaced
  the aggregate dash assertion in F0e. ⚠ **written on a clean tree, before the
  change that will need them**, so they cannot be mistaken for a test bent to
  make something pass.

  ⭐ **and writing them found the thing worth knowing**: a grounded-base flyer
  in flight is steered by `velocity_target`, NOT by the locomotion axes.
  Driving `locomotion` upward moved the body **0.2px in 45 ticks** with
  `fly_enabled` true the whole time. Flight is not "walking with the up axis",
  and a reader of the duel test could not have guessed that.

  ### ⇥ AND THEN THE ROSTER CHANGE WAS TRIED, AND FOUND THE REAL BLOCKER

  ✔ the assertion was replaced (capability claim + citation of the two direct
  tests, exactly as the dash comment does) and `perfect_cellular_automaton`
  added to `PLAYABLE_ROSTER`. `duel_arena` went green. **`app_it` did not.**

  ```text
  possession_end_to_end::attack_while_possessing_starts_the_possessed_actors_melee_not_the_home
      the possessed actor's swing spawned a strike hitbox OWNED by the actor
  ```

  ⛔⛔ **the possessed actor IS the PCA**, its melee lifecycle still engages, and
  no strike hitbox owned by it is ever observed.

  ### ⛔⛔ CORRECTION — I DIAGNOSED THAT AS A LOST KIT AND IT IS NOT

  Commit `a6573dc04` called it the ~100-NPC blanket-registration regression in
  miniature: *"a bare registration says the character authors no kit … the
  archetype's melee is the only place the PCA's swing is stated."* **Probed, and
  false on both clauses.** With the PCA registered, the possessed body carries:

  ```text
  ActorMoveset verbs = ["attack", "attack_air", "attack_air_back",
                        "attack_air_down", "attack_air_up",
                        "attack_down", "attack_up"]
  ActionSet.melee    = Some
  ```

  Its catalog row declares `default_action_set: "striker_swipe"`, and the
  finalization fold uses the row's action set whenever a definition authors
  none — so a bare registration leaves this character fully kitted. ⭐ **the
  general claim about ~100 NPCs may still be true for rows whose facts live ONLY
  in an archetype; it is not true here, and I generalised from a headline
  instead of checking the row.**

  ### ⛔ THE RETIMING HYPOTHESIS WAS ALSO WRONG — and then the probe found it

  I next guessed the sampler was missing the `0.08 s` active window, because the
  test's own comment says one `sim.step` can advance many sim frames. **Measured
  with `world_log::frame()`: exactly 1 frame per step, all 30.** The sampler
  cannot miss anything. Second hypothesis dead.

  ⭐ **the third probe read the actual playback, and that ended it:**

  ```text
  MovePlayback { spec: MoveSpec { id: "attack_air", …
                 gates: MoveGates { grounded: Some(false) } },
                 was_grounded: false, t: 0.167 }
  ```

  ⇒ **the body is AIRBORNE.** Registering the PCA flips it from grounded to
  floating, because its catalog row says `body_kind: Floating` while its
  archetype says `is_aerial: Some(false)` — *"grounded-base HYBRID … prefers to
  fight on the ground, so it descends on provoke"*. Two authorities for one
  fact, and registration is what hands the fight to the catalog. The aerial
  variant plays instead of the grounded swing.

  ⭐⭐ **this is judgement call #1 in D73's field-ownership appendix, hit in the
  wild** — and the `is_aerial` field doc had already named the PCA as the live
  case. ⇒ the row unblocks when **D73 phase 2** resolves that conflict, and
  resolving it is a decision about how this character should PLAY, not a repair.

  ⚠ **three hypotheses, three refutations, in one row.** Each was plausible and
  each was wrong, and the pattern is worth carrying: every one of them was a
  claim about a MECHANISM (a lost kit, a missed sample, a fragile instrument)
  reached without reading the body's live state. The probe that settled it was
  four lines printing a component.

  ### ⇥ AND THE ROUTE IS NAMED — it is the provocation path, file and line

  ⛔ **the flip does NOT come from the hostile spawn seed.** `ActorClusterSeed
  ::new_in` builds aerial-ness from `spec.is_aerial.unwrap_or(false)` — the
  archetype, correctly. The catalog's `body_kind` reaches a body through two
  OTHER sites:

  * `ActorClusterSeed::new_peaceful_npc_in` — the NPC spawn path;
  * ⭐ `autonomous_reconcile::peaceful_config` (`is_aerial = matches!(catalog
    .body_kind(cid), Some(Floating))`), reached from
    `brain_command::apply_catalog_mode`, which keys on
    `config.sprite_character_id`.

  ⛔⛔ **AND I THEN CLAIMED POSSESSION RUNS THE SECOND ONE. THAT IS UNVERIFIED
  AND THE CODE ARGUES AGAINST IT.** `apply_brain_commands` **`continue`s before
  `apply_catalog_mode`** for a body whose brain `is_player()` — which a possessed
  body's is (`brain_command.rs:255`). So the rebuild path I named is, on the face
  of it, not reached by the case I was explaining. ⚠ the claim is struck rather
  than deleted because the machinery it describes is real and IS a phase-5
  target; what is wrong is attributing THIS symptom to it.

  ⇒ **what is actually measured, and nothing more**: with the PCA registered,
  the possessed body is AIRBORNE at the sampling point and therefore plays
  `attack_air` (gated `grounded: Some(false)`) instead of its grounded swing. The
  hostile spawn seed is not the cause — `ActorClusterSeed::new_in` takes
  `gravity_scale` from `spec.is_aerial.unwrap_or(false)` and never consults the
  catalog. Why the body is off the ground is **UNKNOWN**.

  ### ⇥ THE PROBE WAS RUN. FOUR MECHANISMS DOWN, INCLUDING `is_aerial`.

  Same three values, both builds, at the end of the attack window:

  ```text
  registered      gravity_scale 1.0   on_ground FALSE   size 38.1x96.3   x =  663
  not registered  gravity_scale 1.0   on_ground true    size 38.1x96.3   x = 1246
  ```

  ⛔⛔ **so the `is_aerial` / `body_kind: Floating` story is REFUTED too.**
  Gravity is 1.0 in both — nothing floated the body. The collision size is
  byte-identical, so the resize candidate is dead as well. What differs is that
  the body is **580 px away** and off the ground: the 900-step possession
  sequence plays out somewhere else entirely, and the grounded swing never
  happens because the body is not grounded.

  ⇒ **the symptom is a MOVEMENT divergence during possession, not a combat or
  identity fault.** Everything downstream — the aerial move, the missing strike —
  follows from being in the air at that spot.

  ⛔ **five mechanisms have now been asserted for this one row and four are
  refuted** (a lost kit, a missed sample, a fragile instrument, the provocation
  rebuild, `is_aerial`). ⇒ do not add a sixth from reading code.

  ### ⇥ THE DIVERGENCE IS LOCATED: STEP 4, HORIZONTAL VELOCITY

  Per-step trail through the possession loop, both builds. Steps 0–3 are
  **identical to the last decimal**. Step 4 is where they part, and it is
  `vel.x`:

  ```text
  step   baseline x / vel.x            registered x / vel.x
     3   1008.194  /  -43.33           1008.194  /  -43.33     ← identical
     4   1008.194  /    0.00           1007.292  /  -54.17     ← diverge
     5   1008.014  /  -10.83           1006.208  /  -65.00
  ```

  **The baseline's horizontal velocity is ZEROED every fourth step; the
  registered body's accumulates continuously** at −10.83/step. The body is
  falling in both. 35 px apart by step 120, 580 px by the end of the attack
  window.

  ⇒ ⛔ **and the peaceful-rebuild theory is refuted a second time, directly.**
  Probed at the same point in both builds:

  ```text
  hp = (60, 60)   brain = Player(PlayerSlot(0))     ← BOTH builds
  ```

  Not a 1-HP peaceful NPC, not a different brain, identical collision size,
  identical gravity scale. Whatever registration changes, it is none of the
  facts anyone has looked at.

  ⇒ **what remains**: find what zeroes `vel.x` on a 4-step cadence in the
  baseline and stops doing so when the character is registered. It is upstream of
  combat entirely — a movement or contact decision on a FALLING body — which is
  why every combat-shaped hypothesis missed.

  ### ⇥ AND THE ACTOR IS IDENTICAL AT SPAWN, WHICH LEAVES ONE CANDIDATE

  Probed at spawn, before the first step, in both builds:

  ```text
  brain      = StateMachine(Smash { cfg: SmashCfg { aggro_radius: 540.0, … }})
  cfg.brain  = Custom("cellular_automaton_fighter")
  sprite_cid = Some("perfect_cellular_automaton")
  aerial     = false
  ```

  **Byte-identical, both builds.** ⛔ note especially that `sprite_character_id`
  ALREADY resolves to the PCA without registration — so registration does not
  change the character id, the brain, the archetype, or aerial-ness. Every
  identity-shaped hypothesis is now dead, including the ones about resolution.

  ⇒ ⭐⭐ **the surviving candidate is the ORIGINAL 2026-08-07 claim, which nobody
  had pinned**: *"one more registered character is one more sheet demanded at
  load."* Registration changes ASSET LOAD TIMING and nothing else observable
  about this actor. The four-step cadence and the fall are the shape of a body
  whose world is still settling. ⚠ still not proven — but it is the only
  hypothesis left standing, and it is the one the file started with.

  ⇒ **the next probe is about the WORLD, not the actor**: count in-flight asset
  loads / `CharacterLoadStates` per step in both builds and see whether the step-4
  divergence coincides with a load completing. ⛔ and note the shape of this
  whole row when reading that result: five identity hypotheses, five refutations,
  and the answer looks like being the environmental one that was written down
  first and treated as too vague to check.

  ⚠ **and mark the D73 appendix accordingly**: judgement call #1 (`is_aerial`)
  is still a real two-source conflict, but this row is NOT an instance of it and
  must stop being cited as one.

  ### ⇥ ✔✔ CLOSED 2026-08-13 — THE PROBE RAN, AND THE LAST HYPOTHESIS DESCRIBED
  ### A COUPLING THIS CAMPAIGN HAD ALREADY DELETED

  The probe this row asked for — *"count in-flight asset loads /
  `CharacterLoadStates` per step in both builds and see whether the step-4
  divergence coincides with a load completing"* — was run against the possession
  loop, both builds, first twelve steps:

  ```text
    D74 step 0  staged=2 ready=0  kin=(1010.000,   0.00,   0.00) grounded=false
    D74 step 4  staged=3 ready=0  kin=(1008.194, -43.33,  96.67) grounded=false
    D74 step 5  staged=3 ready=0  kin=(1008.194,   0.00, 120.83) grounded=false
    …identical to the last decimal in BOTH builds, and the test PASSES
  ```

  ⛔ **there is no extra sheet, so there is no timing to differ.** `staged` is 3
  and `ready` is 0 at every step of the window in both builds, and the step-4
  divergence does not reproduce at all.

  ⭐⭐ **because REGISTRATION STOPPED DEMANDING ART.** The surviving hypothesis
  was *"one more registered character is one more sheet demanded at load"* —
  true when it was written, and deleted since by D73's own work:
  `try_register_character` is declarative and ends WITHOUT calling
  `CharacterLoadDemand::request`, because loading is driven by what a session
  STAGES. The row's last standing mechanism was a coupling that no longer
  exists, which is why the symptom went with it and nobody had to fix anything.

  ⇒ ✔ **`perfect_cellular_automaton` IS ON `PLAYABLE_ROSTER`**, the workaround
  comment is replaced by this finding, and the smash grid is no longer a portrait
  short of the roster it advertises. Verified before landing rather than
  inferred: `ambition_app` 337 + 179 + 1, `ambition_content` 192 + 32,
  `ambition_demo_smash` 67, and the workspace gate — all green.

  ⚠ **six hypotheses, five refuted and the sixth obsolete, is the lesson of this
  row.** Every one was a claim about a MECHANISM reached without re-measuring
  whether the row's premise still held. The thing that closed it was re-running
  the row's own suggested probe against a tree that had moved underneath it.

  ⇒ **P5.40 unblocks**: it was explicitly *"BLOCKED ON D74"*.

  ⇥ ~~the roster line is reverted, the comment at `character_catalog.rs` carries
  the measured refutation, and the fly assertion stays replaced because it was
  wrong on its own merits.~~ (superseded above; the fly assertion still stands
  replaced, on its own merits.)

- ▢ **D72 ⭐⭐ SET BY JON 2026-08-09 (now second to D73): SET BY JON 2026-08-09: SMASH IS THE ENGINE
  TEST, NOT A MODE THAT GETS EXCEPTIONS.** The brief is long and it is HIS, so it
  lives verbatim in its own file rather than being paraphrased here:
  [`smash-body-generic-combat-2026-08-09.md`](smash-body-generic-combat-2026-08-09.md).
  Read that file before touching combat. The sentence that decides every
  judgement call inside it: *"getting Smash working is **not permission to add
  Smash-specific exceptions**; Smash is the test case that should force the
  engine toward the body-generic architecture you wanted from the beginning."*

  **This outranks every other row below**, including D71's outstanding fix and
  D33's remainder. Those stay open and stay next; they are not cancelled.

  * ✔ **the first slice already landed** — `78fffd933` (one body-melee victim
    resolver), `aa52b3cce` (`ambition_sim_view::CombatGeometryView`; F1 reads
    authoritative geometry; facing stopped being a velocity side effect),
    `af5dd1ced` (pogo consumes `LandedBodyHit`; world rebound stays separate;
    rollback schema bumped). `docs/concepts/one-body-one-path.md` carries all
    three — ⛔ **do not re-derive them, and do not revert them to get green.**
  * ▢ **step 0 is STABILIZE**: compile the affected crates, run the focused
    suites, repair what the three commits left. Jon's explicit order — *"First
    compile and repair the current work rather than reverting architectural
    changes just to get green tests."*
  * ✔ **the eight-item roadmap and the four ranked feel items have all landed**
    (through `573bd9794`), plus air speed and **jump-squat** — each recorded with
    its measurement, its blocker and its wrong turn in the campaign file. ▢ what
    is left of the feel list: **hitbox tracks**, **pose-aware hurtboxes**, and
    **DI** (which Jon put last, after launch and hitstun are clean — they now
    are).
  * ▢ **the roadmap's marked remainders**: the player-faction projectile branch
    still broadcasts `HitTarget::Volume` instead of naming its victim, and
    `HitTarget::UnresolvedFeatures` is scaffolding that retires when bosses and
    breakables become victims in their own right.
  * ▢ **the original roadmap ordering**, kept for the record. My
    execution order and Jon's own leverage ranking (move-facing snapshot,
    authoritative move timeline, hitstop/hitstun, landing-lag/autocancel) are at
    the bottom of the campaign file.
  * ⛔ **two standing prohibitions from Jon, both stated twice**: no `cargo fmt`
    commands, and no git-diff-checking commands. `rustfmt --config
    skip_children=true <file>` on a single file is still fine.
  * ⛔ **do not weaken a meaningful regression to make it pass.** An unrealistic
    fixture gets fixed to model production construction instead.

```bash
python3 scripts/goal_guard.py --status               # confirm the run is armed
git status --short                                   # expect only the two submodules
python3 scripts/check_absence_contracts.py --check    # 25/25, ~11s
$HOME/.cargo/bin/cargo check -p ambition_app          # the gate, ~25s
```

**Standing state at the open of this run:**

- ⛔⛔ **A WORKTREE AGENT IS NOT AN ISOLATED BUILD, AND IT DOES NOT START ON
  `main`** — both found the hard way 2026-08-09, dispatching D37.
  * a worktree spawned while `main` was at `637e60845` came up on **`f7e492252`**,
    missing D33 steps 2 and 3 — the two commits that MOVED rollback-registered
    types. Its deliverable was a regenerated `rollback_schema_baseline.txt`, so it
    would have produced a baseline describing a registration set that no longer
    exists: authoritative-looking, self-consistent, wrong. **Check the base commit
    with a grep for something only current `main` has, BEFORE the agent
    regenerates anything.**
  * `.cargo/config.toml` sets an **absolute** `target-dir`, so every worktree and
    the main checkout build into ONE directory. Isolating the source does not
    isolate the build; concurrent jobs thrash each other's fingerprints and the
    symptom is a stale rlib (`undefined symbol: anon.<md5>.llvm.<n>`), not a cache
    miss. ⇒ **`export CARGO_TARGET_DIR=/home/joncrall/ambition-target-<job>` per
    concurrent job.**
  * ~~and a worktree has **no art or audio** … name an asset-free verification
    set~~ ⇒ ✔ **SOLVED, and the tool already existed:
    `scripts/mirror_assets_for_worktree.py`** — one command, 4111 files
    symlinked. Found by the D40 worker 2026-08-09 while I was telling agents to
    work around the problem. **Put it in the brief instead.**
    [[reference_count_the_adopters_not_the_capability]] again: I designed a
    workaround for a capability that shipped.
  * ⛔ **FOURTH worktree defect, undocumented until 2026-08-09: the map submodule
    is uninitialised**, so `sanic_speedway.ldtk` is a dangling symlink and
    `ambition_demo_sanic` **fails to compile** — not a test failure, a build
    failure. ⚠ **`git submodule update --init` does NOT fix it**: the remote
    rejects the recorded commit (`upload-pack: not our ref`). Fetch from the main
    checkout's object store instead:
    ```sh
    git fetch /home/joncrall/code/ambition/.git/modules/game/ambition_map_assets
    ```
    ⭐ two independent workers hit this and both solved it the same way, so it is
    the standing recipe now.
  * ⚠⚠ **AND CAP THE FLEET AT ~2 BUILD-RUNNING JOBS.** Measured 2026-08-09: four
    worktree agents, each correctly isolated with its own target dir, each
    cold-building the Bevy tree, put **load average at 30+ on 8 cores** and
    everything crawled. The isolation was right and the COUNT was wrong.
    ⭐ the *"parallel agents cost 1.16x"* figure in the build baseline is for
    agents sharing a WARM target dir and **does not transfer to isolated cold
    ones**. A third job that is docs-only or read-only is free; a third that runs
    `cargo` is not.
  * ⚠ **I EXCEEDED THIS CAP THE SAME DAY IT WAS WRITTEN — four build-running
    agents, 2026-08-09.** Measuring instead of guessing: **load average 11.0 on
    8 cores**, 3 `cargo` + 3 `rustc`, 394% CPU. ⇒ **oversubscribed ~1.4×, not the
    30+ collapse**, because the shapes differ: the 30+ figure was **four ISOLATED
    COLD** worktree builds; this was **three sharing one WARM target dir** plus
    one worktree. ⇒ ⭐ **the cap is not a count, it is a count PER COLD ISOLATED
    TARGET DIR.** Three warm sharers is tolerable; three cold isolated ones is
    not.
  * ⛔⛔ **but the shared-target-dir risk is the one to actually fear here, and it
    is silent.** `ls -d /home/joncrall/ambition-target*` returned **exactly one
    directory** while three main-tree agents were building into it. That is the
    fingerprint-thrash configuration, and its symptom is a **stale rlib
    (`undefined symbol: anon.<md5>.llvm.<n>`), not a cache miss** — i.e. it
    surfaces inside somebody's job as a link error that looks like their code.
    ⇒ **when a concurrent worker reports an impossible link failure, check the
    fleet size before believing the error**, and `grep -l <md5> *.rlib` names the
    crate in seconds.
  * ⚠ `/home/joncrall/` is not writable by the agent user
    (`Permission denied (os error 13)`) — put alternate target dirs under the
    scratchpad.
  * ⛔⛔ **THE rustfmt INVOCATION IN EVERY BRIEF TODAY WAS WRONG — corrected
    2026-08-09.** `--edition` and `--style-edition` are **different knobs** and
    this repo needs one of each:
    ```bash
    rustfmt --edition 2021 --style-edition 2024 --config skip_children=true <file>
    ```
    The **language** edition is 2021 (workspace `Cargo.toml`; every crate is
    `edition.workspace = true`). The **style** edition the tree is actually
    formatted with is **2024** — there is no `rustfmt.toml`, so `cargo fmt` took
    the modern default. ⇒ `--edition 2021` **alone** implies
    `style_edition = 2021` and re-sorts imports the repo does not sort that way.
    ⭐ **measured on `game/ambition_app/src/app/mod.rs`, rustfmt 1.9.0**:
    `--edition 2021` → **24 changed lines**; adding `--style-edition 2024` →
    **4**. ⚠ **a worker had to hand-revert an import cascade my instruction
    caused**, and it had already gone out in ~10 briefs. ⚠ **4 is not 0** — that
    file has hand-formatted regions rustfmt disagrees with, so still format only
    files whose pending diff is your own code.
  * ⚠⚠ **`compile_collect.py` DIRTIES 55 `lib.rs` FILES WHILE IT RUNS**, and a
    `git status` taken mid-run looks alarming: *"56 files changed"*, every
    first-party crate. It is appending a probe to force a per-crate rebuild —
    ```rust
    #[allow(dead_code)]
    fn _compile_collect_probe(x: u32) -> u32 { x.wrapping_add(10) }
    ```
    — and it removes them at the end. ⇒ ⛔ **do not commit them, and do not
    "clean up" a tree that looks wrecked mid-collection.** ⚠ if the run dies, the
    probes are what is left behind; they are `_compile_collect_probe` and nothing
    else, so `grep -rl _compile_collect_probe crates/ game/` finds every one.
    ⭐ **and it uses its OWN target dir** (`ambition-telemetry-target/<config>`),
    so it does not poison the main build — but its own docstring forbids a second
    cargo alongside it, and the symptom of ignoring that is **a warm no-op
    reporting 222 s**, which reads as a slow machine rather than a mistake.
  * ⛔⛔ **AND THE SETUP TAX CAN EAT THE WHOLE SESSION. Measured 2026-08-09: a
    worktree agent spent FOUR HOURS AND SEVENTEEN MINUTES on environment
    preparation and never reached its task.** Its last words were *"Assets
    intact. Now submodules — cloning from the main checkout's module dirs"* — it
    was still working through the four defects above when it was killed.
    ⇒ **the four worktree defects are not four small taxes, they are one large
    one**, and each is only cheap if you already know the recipe.
  * ⛔⛔ **A DEAD AGENT AND A FINISHED AGENT LOOK IDENTICAL FROM THE REPO.** I
    checked that worktree twice — `git log` showed its merged commits, `git
    status` was clean — and concluded it was between steps. Both are exactly what
    a corpse looks like. ⚠ **the liveness check is the OUTPUT FILE'S MTIME**, not
    anything in git:
    ```sh
    stat -c '%y  %s bytes' /tmp/.../tasks/<agentId>.output
    ```
    Last written **4h17m earlier**. ⇒ ⭐ **check mtime before believing a
    dispatched task is progressing, and check it on a schedule** — two briefs I
    sent it were queued *"for delivery at its next tool round"* and there was
    never going to be one. **A queued message to a dead agent fails silently.**
    ⛔ **and it is the MTIME ONLY — the size is not a signal.** I first wrote
    *"134 bytes, last written 4h17m earlier"* as though both mattered; checking
    two healthy agents immediately afterwards, **both were also 134 bytes**. That
    is the stub's normal size. **Reading size as evidence would condemn a live
    agent.**
    ⚠ **and the mtime needs a THRESHOLD, or it produces false alarms.** Measured
    on two healthy agents minutes later: **8 and 17 minutes stale, with zero
    `cargo`/`rustc` processes running** — an agent reading and planning writes
    nothing for a long time, and that is normal. ⇒ ⭐ **the honest composite is
    three signals, and "dead" needs all three**: output mtime **> ~30 min** old,
    **no writes** to the files it is editing (`stat` them from `git status`), and
    **no `cargo`/`rustc`** in `ps`. The corpse scored 4h17m on the first and zero
    on the other two; a working agent fails at least one of them.
  * ⇒ ⭐ **prefer the MAIN TREE for a task that must land.** A worktree buys
    isolation from concurrent edits and costs the whole setup gauntlet; it is
    worth it only when two agents genuinely must write the same files. Otherwise
    name the do-not-touch crates in the brief and work in the main checkout.
- `tools/ambition_sprite2d_renderer` and `tools/ambition_music_renderer` are
  DIRTY with Jon's scratch work. ⛔ hands off, and `git add` by path only.
- The gate is `cargo check -p ambition_app`, never per-crate.
  ⛔⛔ **AND IT DOES NOT SEE `#[cfg(test)]` CODE — found 2026-08-09 by the D33
  step-4 worker.** Its change broke the monolith's own test module (it used a
  private helper that moved), and **`cargo check -p ambition_app` stayed green**.
  ⇒ **the gate at the top of every brief I have written this run is incomplete**,
  and the completion is one flag:
  ```sh
  cargo check -p ambition_app --all-targets     # ← the tests, benches and examples too
  ```
  ⚠ **`--all-targets` is what I used for the D52 prop fix and it is what caught
  a dead helper there**, so I had the habit in one place and the wrong
  instruction in the briefs. **Say `--all-targets` in every future brief.**
  ⭐ the deeper point: *"the gate is green"* was a claim about the LIBRARY
  targets, in the same way *"the suite is green"* turned out to be a claim about
  a feature set (D57). **Both sentences needed a qualifier nobody was saying.**
- `cargo test -p ambition_app --test app_it` is **321 passed / 0 failed / 11
  ignored** / **307 s**, measured 2026-08-09 on `e1f080cf8` — i.e. **after the
  D37 and D40 merges**, so the merged state is verified, not assumed.
  ⛔ **this line was stale at 318, then 319, and I briefed a worker with 320 from
  memory** — it caught me. **Re-read this block before quoting a baseline into a
  brief; do not quote one from recollection.**
  ⚠ **and note the RUNTIME moved: ~175 s → 307 s.** That is the number a brief's
  `timeout` has to respect, and a worker who budgets from the old figure kills
  its own verification run at 57% and reports a hang.
  ⚠ **a crate suite may be feature-gated, so "green" is a claim about a FEATURE
  SET**: `-p ambition_input` runs 55 of 84, `-p ambition_touch_input` runs 4 of
  45 (`bevy_plugin` is behind `mobile_touch`), and all four
  `spikes_spend_rings` tests **fail** under `--features input` while passing
  without it (D57). ⇒ **say which features a verification ran under, or it does
  not mean what it sounds like.**
- The rollback schema is at **v26** (`registry.rs`). Baseline-row and encoded-type
  counts are derived inventories; recompute them when a decision actually needs
  the count rather than caching another number in this live ledger. ⚠ this line
  previously drifted through v17, v19 and v24;
  `test_the_ledgers_rollback_schema_version_matches_the_source` reads this claim
  against the constant so the version itself fails loudly instead of becoming
  another historical measurement.
  found it. A registration change needs **four** things in ONE commit: the
  version bump + its reason, `tests/rollback_schema_baseline.txt` regenerated,
  `docs/planning/engine/slice-evidence/rollback-schema-baseline.json` updated,
  **and this line**. The absence-contract ratchet catches the third and the
  pointer guard catches the fourth; nothing catches you forgetting the first.
  ⛔ **"347 stable names" was the WRONG DENOMINATOR and I quoted it repeatedly.**
  347 is what the slice-evidence scanner counts (`rollback/mod.rs` +
  `rollback/domains/**` only). The LIVE dump carries **423 rows**, because
  game- and demo-crate registrations join it. Both numbers are real; they answer
  different questions, and the absence contract still says 347 because that is
  its own population. [[reference_measure_the_suspect_not_the_aggregate]].
  ⚠ **`--test <filename>` does not resolve in the AGGREGATOR crates** — they set
  `autotests = false` with a single `[[test]]` target that `mod`s every file:
  | crate | the one target | filter like |
  |---|---|---|
  | `ambition_app` | `app_it` | `--test app_it -- rollback_schema_baseline` |
  | `ambition_demo_sanic_app` | `sanic_it` | `--test sanic_it -- spikes_spend_rings` |
  ⛔ **the error is `no test target named <x>`, which reads like "that file is not
  wired in"** — and rust-analyzer says the same thing ("not included anywhere in
  the module tree") because it cannot see the aggregator's `mod` list either.
  **Both are wrong; check the aggregator before believing either.** I nearly
  filed "the spike tests are not registered with Cargo" on those two agreeing
  signals. ⭐ *two stale sources agreeing is not corroboration* —
  [[feedback_grep_for_capability_not_type_name]].
  ⚠ **v19 (D29) moved the version and the baseline's HEADER LINE and nothing
  else**, which is the shape to expect from a change to what a registration
  *means* rather than to the set of registrations: `ConversationInstanceId` took
  in the `DialogueContext`, so the `resource.active_conversation` checksum moved
  while all 347 names stayed put. The slice-evidence sets do not move for it
  either — only its `_comment` record.

---

## Lane A — the review campaign in flight (GPT 5.6 through `43373f72d`)

Plan: `~/.claude/plans/lively-drifting-conway.md`. **LANE A IS DISCHARGED** —
`f043882cd`, `97ec5f450`, `e10d269da`, `8121fb9ca`, `2388f4631`. The write-up is
`docs/archive/reviews/review-gpt56-through-43373f7.md`, and the next review baseline is
`2388f4631`.

- ✔ **A1 `ConversationInstanceId`** — content-derived, history-free. A nonce
  cannot be re-minted by a resimulation; a rollback-rewound counter re-mints
  perfectly and still hands a record to the wrong conversation, because it
  encodes history rather than content. Minted from tick + node + both bodies'
  `SimId`s.
- ✔ **A2 `NarrativeInputLedger<M>`** — the mirror image of
  `ExternalEffectJournal`. Edge release, instance-gated, pruned to the replay
  horizon, never by consumption. `ObservedNarrativeEnd`'s depth-one slot is
  gone, and with it the argument that "a player has to read the first one".
- ✔ **A3 presentation attachment** — the memo means "the box is attached to
  this instance", not "I projected this once". Opening and closing are one
  system, because they were two and only one wrote bookkeeping.
- ✔ **A4 every gameplay-bearing Yarn command through the ledger** — plus the
  classification table in `dialog/yarn_bindings.rs` that says which commands
  are NOT, and why.
- ✔ **A5 one authority for combat participation** (review finding 6). ⚠ **it is
  worse than the review says: there are TWO proxies, not one.** `CombatStanding::of`
  reads `RulesetOwnsDeath` (damage), and the stand-down guard at
  `features/ecs/actors/update.rs:526` reads a *different* one, `Has<MatchSeat>`.
  A seat is not participation either — an eliminated fighter keeps its seat.
  Jon's call (2026-08-07): an **explicit component on the body**, attached beside
  `RulesetOwnsDeath` in `prepared_match.rs` and removable on elimination or
  between phases — not a match-phase derivation (every reader would name the
  ruleset, and a training dummy has no answer) and not a third
  `ActorDisposition` arm (that is the conflation this ends). Then
  `sync_actor_components_from_cluster` keeps attack state for an active
  combatant, and the anti-clump board admits one. The state that must become
  representable IS the test: *active combatant · human controlled · socially
  non-hostile · damageable · able to attack*.
- ✔ **A6 stop hardening `ParticipantId == ControlChannelId`** (finding 5). ⛔ a
  RULE, not a type — the review explicitly asks for no refactor and no new
  newtype. State the target chain in `ambition_input/src/channels.rs` and
  `character_runtime/participant_seat.rs`, and add ONE row to `tracks.md` naming
  the missing `SessionSeatId`/`ControlChannelId` materialisation with the two
  lifetimes that differ.
- ✔ **A7 the opportunistic pair** (findings 7–8). `scripts/regen_music_registry.py:41`
  still hard-codes the consuming crate as a fallback default, beside a comment
  saying *"two readers of one declaration, never two declarations"*;
  `scripts/lib/asset_roots.sh:31` is the declaration. And trim provenance
  ("GPT X found Y on date Z") from production comments **only in files this
  campaign already edited** — not a sweep.
- ✔ **A8 `docs/archive/reviews/review-gpt56-through-43373f7.md`** — what was done, what
  was confirmed WORSE than the review said, what was deliberately not done. Same
  shape as `docs/archive/reviews/review-gpt56-through-ffa57c5.md`. Three things belong in it that the
  review did not know: ordinary play is a GGRS host (`cli.rs:649,1011`), so all
  of this is live rather than hypothetical; `PendingChallenge` was not rollback
  state at all; and combat participation had two proxies.

## Lane B — the SR / TwinTrack overlay

`untracked/ambition-twintrack-relativity-festival-overlay-2026-08-06-1538.zip`,
30 files, 770 KB, stamped 2026-08-06 15:38 — **older than `main`'s current
tree**, so this is a MERGE and not an unpack. **LANE B IS DISCHARGED**
(`ac0bf991c`); the file-by-file verdict is
`docs/archive/reviews/twintrack-overlay-merge-2026-08-08.md`.

⚠ **the row below guessed FOUR contested files and there were SIX**, and the two
it missed are the instructive ones: `abilities.rs` and
`character_catalog/entry.rs` moved on 08-06 evening, after the zip. ⛔ and the
real hazard was the opposite shape entirely — main moved in files the overlay
does NOT carry (`ambition_relativity2d`'s `telemetry.rs`/`lib.rs`, via
`a301a79a0`), so copying the overlay's demo wholesale landed it on an
incompatible API. **A merge verdict has to ask what moved UNDER the overlay, not
only what it overwrites.**

- ✔ **B1 diff before writing anything.** Extract to the scratchpad and diff each
  of the 30 paths against the working tree. ⛔ four of them are files this repo
  has changed since 08-06 — `rollback/registry.rs` (now v16),
  `game/ambition_app/tests/rollback_schema_baseline.txt` (regenerated today),
  `docs/planning/tracks.md`, and `docs/planning/engine/slice-evidence/rollback-schema-baseline.json`.
  Overwriting any of them silently reverts landed work. Produce the file-by-file
  verdict FIRST and write it down.
- ✔ **B2 land the parts that are genuinely new** — ⚠ NONE of the 30 were new; every path already existed. What landed is the overlay's content merged onto main's API.: `ambition_relativity2d/src/signals.rs`,
  `game/ambition_demo_twintrack{,_app}`, `docs/planning/demos/twintrack.md`,
  `docs/planning/engine/relativity.md`, `docs/planning/engine/slower-light.md`,
  ADR 0011. Gate: `cargo check -p ambition_app` and the twintrack integration
  test.
- ✔ **B3 reconcile the contested files by MERGING, never replacing**, and
  say in the commit which side won each hunk and why.

## Lane C — the actor-monolith decomposition

`docs/planning/engine/actor-monolith-decomposition.md`. Jon, 2026-08-07: *"if we
add things to the monolith, try to do it so it's obvious what the decomposition
should be … we will need to address that bloat in the coming"* — and
`awaiting-maintainer-decision.md:423` already records that this collides with
another decision.

- ✔ **C1 re-measure the doc against the tree before working it** — and it is
  CURRENT, not stale: 110,911 → 112,020 lines, deps 28 → 29, root areas
  unchanged in rank. ⚠ the row's own guess ("the oldest lane, most likely
  describing a monolith that has already moved") was wrong; the doc is one day
  old. Measuring first is still what established that.
- ✔ **C2 the conversation module is NOT the first carve** — re-derived, and the
  row's premise is refuted by the plan's own scorecard: 1,907 lines and no Cargo
  edge removed, because every crate it names is named elsewhere in the monolith.
  Its inward edges ARE still exactly two (both the bark), so the module's own
  accounting survived the ledger. ⭐ the right unit is the DIALOGUE domain
  together, which is what takes `ambition_dialog` — and with it `ambition_ui_nav`
  — out of a movement-only game's graph. ~~the accounting is already written~~ — `conversation/mod.rs` names its two inward
  edges (`npc_ambient_bark_line`, `PreparedCharacterRegistry`), says both are
  the BARK rather than continuity, and concludes the carve is "a port plus a
  `Cargo.toml`". ⚠ that accounting predates the ledger; **re-derive it** —
  `ledger.rs` added edges to `ambition_platformer2d_core::ConfirmedFrameBoundary`
  and `ambition_time`, both below the monolith, and `items/narrative.rs` is a
  new sim-side consumer.
- ✔ **C3 pick the next carve by DEPENDENCY DIRECTION, not by line count** —
  measured per-dep module counts and landed the free one (`ambition_ui_nav`, a
  conduit edge named by zero modules). Accounting written into the doc first, as
  the plan requires. ⛔ **and the footprint did not move**, which is the finding
  worth carrying: the 15 leaked capability crates are not 15 removable edges.
- ✔ **C4a `features` is unpinned from `ambition_dialog`** (`69b53c42d`). The
  blocker was TWO production lines, both in `features/ecs/interact.rs`; the
  dialogue decision moved to `conversation::opening` behind a port that takes
  `&str` and `Entity` only, so the carve accounting stayed at two inward edges.
  `interact.rs` 350 → 216 lines. ⭐ the shape generalises: almost every
  low-count dep is pinned by `features` doing one domain's job inline.
- ✔ **C4b measured, and the row's own premise was wrong** — there is no single
  dialogue domain to lift. `dialog/` is ONE file of NAMED GAME VOCABULARY whose
  owner is `game/ambition_content` (which already pushes two installers through
  the same `YarnContentBindings` seam); `conversation/` is the reusable
  continuity authority and keeps the `ambition_dialog` edge wherever it lives.
  ⚠ **neither move alone satisfies the scorecard** — the edge leaves only when
  `conversation` does. Accounting in the decomposition doc.
- ✔ **C4c the game's Yarn verbs moved to `game/ambition_content`**
  (`73873491b`), and the plugin split with them: `YarnBindingsPlugin` keeps the
  generic wiring and registers no vocabulary. Monolith 112,020 → 111,493.
  ⚠ committed as an OWNERSHIP correction, not banked as a carve — no Cargo edge
  moved.
- ✔ **C4d re-measured, and the row's own claim was false** (`54b690ae4`):
  `opening.rs` HAD added a third inward edge — `participant_seat` — and my
  commit message claimed it had not. Worse than a count: `participant_seat`
  exists to keep the `ParticipantId` ↔ `PlayerSlot` correspondence in ONE place,
  so a second caller inside a module that wants to leave the crate would have
  had to take the correspondence with it. `open_between` takes the owner as a
  parameter now. Back to two, both the bark, and `conversation/mod.rs` says how
  to re-derive rather than just asserting.
- ✔ **C4e's work is DONE; the carve itself is deliberately NOT taken.** The bark
  port landed (`a7013ef82`) and `conversation` has ZERO inward edges — it is
  liftable whenever wanted. ⛔ **but lifting it now buys ~2,164 lines out of a
  111k-line recompilation unit (≈2%) and no footprint change**, which is the
  "moving files without improving any of those measures" the plan warns against.
  It waits for the capability decision, which is what turns it into a real win.
  ⛔ **the gating answer recorded here was WRONG and is corrected in the doc.**
  It is a COMPILE-ISOLATION win, not a footprint win: five production files in
  the monolith consume `crate::conversation`, so the new crate would be a
  non-optional monolith dependency and `ambition_dialog` would still reach a
  movement-only game through it. Same error class as the `ui_nav` one, made
  right after writing the warning against it.
  ✔ **step 1 (the bark port) is DONE** (`a7013ef82`) and `conversation` now has
  ZERO inward edges, so the module is liftable whenever the carve is wanted.
  ▢ **the remaining question is the maintainer's**: may a game compose this
  engine WITHOUT dialogue? Only an `optional = true` dependency behind a
  `dialogue` feature turns this into a footprint win, and `ambition_causal` is
  the only optional `ambition_*` dep the monolith has — which is why the
  unasked-for footprint is fifteen crates. Added to
  `awaiting-maintainer-decision.md`.
- ✔ **C5 measured, and it reframes the lane.** Neither `ambition_menu` nor
  `ambition_gameplay_trace` is a conduit — both are real consumers
  (`menu/map/*`, `dev/trace/*`). ⛔ **but the bigger finding is that this plan's
  second trigger blames the wrong crate**: of the fifteen capability crates a
  movement-only game inherits, exactly ONE (`ambition_platformer2d_ldtk`) has
  the monolith as its only direct dependent. `ambition_platformer2d_runtime`
  declares TEN of them and is a direct facade dep, and for `ambition_cutscene`
  and `ambition_items` its only reason is `rollback/domains/*.rs`. Accounting in
  the decomposition doc.
- ✔ **C6 measured, and it is NOT available.** The single production reference
  is a blanket `pub use ambition_platformer2d_ldtk::*` in `world/ldtk_world`,
  which looks like a stale facade and is not one: SEVEN production files in
  seven root modules consume LDtk types through it, including `session/setup`
  and `features`. The monolith genuinely uses LDtk.
  ⛔ **so lane C has no footprint win left at all**, and that is the honest
  state: three rows in a row (C4e, C5, C6) had premises that did not survive
  measurement, always in the same direction — assuming a crate is pinned by one
  removable thing. What remains available is COMPILE ISOLATION, which is real
  (112k lines, one recompilation unit, incremental off) and which `conversation`
  is ready for.
  ⭐ **lane C is otherwise blocked on the maintainer decision**, now broadened
  from dialogue to the general question, because the measurement showed it was
  never dialogue-specific.
- ✔ **C7 measured: the inversion is NOT a free win, and does not work below the
  runtime.** `registry.rs` is "a thin layer over `bevy_ggrs`" and imports it, so
  the vocabulary cannot move to the floor without dragging a rollback backend
  there. `ambition_content` can declare its own schema only because it sits
  ABOVE the runtime; every never-asked-for crate but `ambition_persistence` sits
  below. ⛔ and the declare/install halves cannot be split by data alone —
  installation is generic per type and `T` is not recoverable from a
  `type_name`. C7 collapses into the same maintainer question as the rest of
  lane C. Accounting in the decomposition doc so nobody attempts it as a slice.

## Lane D — everything else in `docs/planning`

- ✔ **D0 the music renderer's two reds, fixed** (`578fb9c` in the submodule).
  Jon's guitar-performance work and three ensemble scores committed on his
  say-so; then the pre-existing pair: `publish_root()` raises by design and
  three CLI configs used it as a `default_factory`, which runs at PARSE time —
  so `render <cue>` demanded a publish destination for a run that never
  publishes. The raise MOVED to use time rather than being softened, which is
  the shape `audit.level_report` had already reached independently. 138/2 → 140/0.

Open because lane C is measured out and blocked on a maintainer decision. Read
`roadmap.md`, `vision.md`, `status.md`, `tracks.md` and the engine design docs;
pick the item that makes the next ten cheapest; **add it here as a row WITH its
reasoning before working it.** Do not invent work outside `docs/planning`.

- ✔ **D1 `awaiting-maintainer-decision.md` gained a row and one was broadened** —
  "may a game compose this engine without a given capability", raised as a
  dialogue question and broadened the same day when the measurement showed it
  was never dialogue-specific.
- ✔ **D2 Jon's sprite-scale row measured** (`245974e36`): the bbox route it asks
  us to "decide first" ALREADY SHIPPED, and is not the three coupled changes it
  describes — the quad keeps frame size so nothing stretches, and the offset does
  the aligning. The blocker is data: 2 of 190 sheets declare `authored_body`.
- ✔ **D3 DONE, and it answers the decision rather than the row.** The row asked
  for an authored body box; the 08-07 correction had already shown that changes
  nothing (`render` is the frame whether the body is authored or measured), so
  what ran was the smallest step that *could* discriminate: **site 1 alone,
  photographed.** Result — the quad/box ratio goes `2.46x → 1.00x` and the art is
  destroyed getting there (snake 41% height at 90% width; Mary-O 41% width at 64%
  height; both matching `bbox/frame` to measurement error). ⭐ **the coupling
  claim is CONFIRMED and site 2, the sub-rect crop, is the load-bearing one.**
  ⛔ **and the decision's site list was an undercount: there are TWO render-size
  publishers and it names one.** The AI Slop in the same frame measured
  0.99x/1.02x — unmoved — because it never attaches `SpritePosedBody`. Evidence
  written into `awaiting-maintainer-decision.md`; the code was REVERTED, because
  the fork is Jon's to take.
- ~~**D3 author ONE body box — the snake — and read the ratchet.**~~ The smallest
  step that turns Jon's complaint into a measurement instead of a taste
  argument: author `body_metrics` for `snakes_on_a_*`, regenerate the sheet, and
  read `enemy_quad_matches_its_box` (it ratchets the disagreement at 2.47x) plus
  a `capture_scene` before/after. ⭐ **UNBLOCKED 2026-08-08** — Jon: *"you can
  commit any sprite or music work."* ⚠ and this row's premise was half wrong:
  `tools/ambition_sprite2d_renderer` was CLEAN the whole time; only the music
  renderer was dirty, and it is committed now.
- ◐ **D23 SPLIT THE BOLT/HURTBOX FINDING: one half is a bug, one half is Jon's.**
  `step_projectiles` never consults `DamageableVolumes` (`614f098f2`, filed in
  `tracks.md` track 8). The card as written is one change; it is two, and only
  one needs Jon:
  * ✔ **INTANGIBILITY WAS A BUG — FIXED 2026-08-08.** An EMPTY
    `DamageableVolumes` means *this body cannot be hit*. A bolt hit it anyway,
    and ATE ITSELF doing so. `DamageableVolumes::intangible()` names the state,
    is `strike_reaches_victim`'s first arm, and `step_projectiles` asks it via
    `StrikeVictimItem::is_intangible()`.
    ⭐ **LIVE, and the probe is what decided it.** The trigger is not an authored
    invulnerability — swept every `.ron` under `crates/` and `game/` and no
    shipped HURTBOX doc has an empty window (the only `volumes: []` in the tree
    are ATTACK windows in `character_archetypes.ron`), so *that* trigger is
    latent. What is live is the CORPSE: `refresh_body_damageable_volumes` empties
    a dead body's list, the body stays standing until a ruleset removes it
    (`spend_fighter_stocks`), and versus/smash seats alternate `Player`/`Enemy`
    (`prepared_match::faction_for`) so seat 1's bolts route through this exact
    hostile loop at seat 0's corpse. The projectile loop had no corpse check
    either. Red on the shipped publisher + shipped stepper before, green after:
    `a_bolt_passes_through_a_body_that_published_no_hurtbox`.
  * ▢ **PRECISION IS JON'S — still open, and as of 2026-08-10 it BLOCKS.**
    Making a bolt test the authored hurtbox rectangles instead of the coarse
    `CenteredAabb` retires `strict_intersects` for projectiles and changes how
    every shot connects. That is feel, and it was deliberately NOT taken even
    though the call site is one line away.
    ⭐⭐ **it stopped being a nicety on 2026-08-10.** The combat campaign found
    that a boss is ALREADY a body victim — melee names `HitTarget::Body(boss)`
    today — but the damage consumer's boss branch only runs for an event that
    names no actor, so the identified hit lands nowhere and the boss's HP is
    moved by the anonymous `UnresolvedFeatures` half instead. Fixing that
    requires BOTH producers to name bosses, and the projectile victim query
    excludes `BossConfig` precisely because this loop still tests the coarse box
    — and a boss's coarse AABB is a giant composite envelope, so including it
    without `reached_by` would let bolts hit the bounding rectangle instead of
    the authored head/hand volumes (the GNU-ton seam, undone).
    ⛔ **so this one feel call now gates retiring `HitTarget::UnresolvedFeatures`
    for bosses.** One question, one line, and a scaffolding variant comes out
    with it.
- ✔ **D4 CLOSED 2026-08-08 — its whole enumerated set is discharged.** D5 closed
  by Jon (*"the oni leader bug was fixed"*), D6 landed (submodule `6203ae9`,
  pointer `73adaa72c`), D7 measured (animation 0.12 s — the 3.2 s dwell never
  cut it). What remains under this row is one item **blocked by design** and one
  that was never work, both restated below and neither owed a `▢`.
  ⛔ **this row kept its `▢` after every child closed — the fourth time today**,
  which is the exact staleness this ledger's own header warns about. The tell is
  cheap and I keep missing it: a parent row's marker is not maintained by closing
  its children, so ANY row that enumerates sub-items must be re-read when the
  last one lands.
  The original row, kept for its survey: each item is a claim about the code and
  should be re-measured before working. ⚠ **re-surveyed
  2026-08-08 and the row was both DUPLICATED and INCOMPLETE** — it listed the art
  re-render twice and missed the one item in that file with no triage mark at
  all. The true open set is D5–D7 below plus:
  * the **humanoid-sheet judgement** (obs:191) — ⛔ **BLOCKED, and blocked by
    design.** The decision doc says decide the bbox question BEFORE the humanoid
    pass, because `--suggest` retunes the very field the bbox route deletes.
    Working it now spends a 116-row judgement that D3 may make moot. It waits on
    D3's evidence and Jon's answer.
  * obs:576 is not work — it is a scope note saying what the instrument
    deliberately does not assert. No row owed.

- ✔ **D35 DONE `e58e308f3` / `7f3a11185` — and the cause was neither typography
  nor Material.** `bevy_material_ui` is out of the graph; the test it "held up"
  was reading the wrong entity. Journal, with the full observation table:
  `dev/journals/material-ui-removal-2026-08-08.md`.

  ⭐⭐ **THE MEASURED CAUSE: `"Ambition"` is on the title screen TWICE, and always
  was.** It is the launcher's title (60.48 px, carries `MenuTextHeightFraction`)
  **and** the roster row for the game called Ambition (20.0 px — `TextFont`'s
  default size, because `spawn_control` sets the font HANDLE and leaves the size
  at `..default()`). The test's global `find(|(label,_)| label == "Ambition")`
  returned whichever archetype the query reached first.

  **The proof is that the two worlds are IDENTICAL.** 29 `Text` entities with and
  without Material, same labels, and **the title measures 60.48 px in both**. Only
  the walk order differs:

  ```text
  WITH    Material — archetypes 133, 118, 89, 120 …   title at index  3 → PASS
  WITHOUT Material — archetypes  88,  86, 108, 117 …  row   at index  2 → FAIL
  ```

  Material's plugins register components and spawn one `Startup` entity, shifting
  archetype/table creation order. **That is the entire five-day "dependency".**

  ⛔ **and `resolve_menu_text_size` was refuted, not merely doubted**: that App has
  **0 primary windows**, so the resolver takes `unwrap_or(REFERENCE_HEIGHT)` and
  writes back exactly what the spawner wrote. Running it and not running it are
  indistinguishable here — a dead resolver predicts the test **passing**, which is
  the opposite of the observed failure.

  **Verified independently by the supervisor**, not taken from the agent's report:
  `the_title_screen_*` 3/3 · `cargo test -p ambition_app` **320 passed 0 failed** ·
  `pytest scripts/tests` 251 · `check_absence_contracts --check` 25/25 ·
  `cargo tree -i` reports no such package for both crates.

  | measure | result |
  |---|---|
  | packages out of the graph | **5** — `bevy_material_ui`, `google-material-design-icons-bin`, `hct-cam16`, `lz4_flex`, `png` |
  | `Update` census, Material's OWN share | **482 → 456 = −26**, all 26 unsetted (in-a-set stayed 231). Corroborates the upstream read: 16 `dialog_*` systems + ~10 from focus/ripple/icons/i18n. `GgrsSchedule` unchanged at 534. |
  | rustc work removed | **160.05 s** (`03878f81b`, dev, dirty=false) |
  | wall clock | **~20 s of 540 s** — see the critical-path retraction under D33 |

  ⚠ **the `ui` FEATURE stayed in both crates.** It is the dialogue/Yarn umbrella
  and callers select it for that; only the Material edge was deleted.
  `add_ui_plugins` was left empty and so is gone entirely.

  ⛔ **NEW BLIND SPOT FOUND, and it is general** — `git ls-files '*Cargo.lock'`
  returns **4**; the filesystem has **5**. `fixtures/external_consumer/Cargo.lock`
  is hidden by a **nested** `.gitignore` two directories down, while the checker
  that validates locks discovers sub-workspaces by **walking the filesystem**. The
  repo-tooling job failed on exactly that gap. ⭐ **enumerating with git and
  validating with the filesystem is a guaranteed miss** — derive both populations
  from one source. (`git check-ignore -v <path>` names the file and line; reading
  the root `.gitignore` would not have shown this.)

  ⛔⛔ **AND THIS PARAGRAPH FAILED TO PREVENT THE SAME MISTAKE HOURS LATER,
  2026-08-09.** Mid-carve I read a worker's `git status`, saw four lockfiles
  modified and not `fixtures/external_consumer`, and sent an instruction to
  *"regenerate it and commit it with the manifest"*. **Both halves wrong**: it is
  gitignored so it cannot be committed, and **`git status` never shows it
  whatever its state**, so "untouched" was never observable that way. Corrected
  within minutes, but the instruction had already gone.

  ⇒ ⭐⭐ **the fact was written down and the SUMMARY I actually read said "three
  lockfiles".** The census here is right; my index of it was stale and short.
  **A correct fact behind a wrong summary is worse than an unrecorded one**,
  because the summary is what gets acted on and it carries the recorded fact's
  authority. ⇒ **when a count changes, fix the index in the same edit as the
  detail** — the rule this run has now learned three times, after the open-rows
  table and the decision file's *"2 open"*.

  ✔ **the census, verified by `find` + `git ls-files` + `git check-ignore` on
  each, 2026-08-09**: root, `fixtures/minimal_game`, `examples/capability_demo`
  and `examples/portal_tutorial` are **tracked**; `fixtures/external_consumer` is
  **ignored**. ⚠ `portal_tutorial` names neither the monolith nor
  `ambition_platformer2d`, so it usually will not move.

  ▢ **LEFT OPEN DELIBERATELY, and it is a real inconsistency**: every launcher ROW
  label and the tab head are `TextFont`-default 20.0 px with **no**
  `MenuTextHeightFraction`, so **the rows do not scale with the window while the
  title and footer do.** Not what flipped the test; changing row typography is a
  layout decision, not a dependency removal. ⚠ one adjacent datum for the
  unsolved TOFU mystery (`launcher.rs:65-85`): `spawn_control` sets the font
  handle and the font size from different places.

  ⚠ **pre-existing, not from this work**: `check_agent_kb.py` fails on
  `crates/ambition_platformer2d_core/src/abilities.rs` (missing inline-test review
  marker), introduced by another session's `ac0bf991c`. Confirmed by `git log` on
  that path. Left alone.

  **The original brief follows, kept unedited for the compile retraction it
  carries.**

  **What the crate is worth.** One clean dev build, `03878f81b`, `dirty=false`,
  from `dev/ambition_dev_measurements/compile_units.jsonl` — one profile, one commit, no pooling:

  | unit | frontend | codegen | total |
  |---|---:|---:|---:|
  | `bevy_material_ui` | 14.31 s | 132.49 s | **146.80 s** |
  | `google-material-design-icons-bin` | 1.13 s | 12.12 s | **13.25 s** |

  ⚠ **that is COLD cost and it is cached in the inner loop.** It is paid by every
  fresh clone, cold CI leg, `cargo clean`, and dep bump — not by every edit. Do
  not sell it as a rebuild win.

  ⛔⛔ **AND IT IS NOT A 160-SECOND WALL-CLOCK WIN EITHER — corrected 2026-08-08,
  see the retraction under D33.** That build is **96% saturated** (4153.3 s of
  unit work in 539.9 s wall, parallelism 7.69 of 8), and `bevy_material_ui` runs
  `358.6 → 505.4` while the critical path is `monolith (→524.4) → ambition_app
  (→539.9)`. **It is not on the critical path.** Removing it frees 3.9% of the
  work, worth on the order of **20 s of wall**, which this build's 54%
  run-to-run spread cannot resolve.

  ⇒ ⭐ **so state this campaign as what it is**: deleting a dependency nothing
  imports and a `⛔` warning that was false, which also stops 160 s of pointless
  CPU per clean build. **Not a compile-performance campaign.** Both I and GPT 5.6
  independently called it "an unusually high-value dependency to eliminate" on
  the strength of the per-unit number, and neither of us checked the critical
  path first.

  **What it buys.** Nothing. `git grep material_ui -- '*.rs'` returns **one file,
  four mentions** (`game/ambition_app/src/app/plugins.rs`), two of which are the
  `add_plugins` call itself. No Ambition code constructs a Material widget, reads
  `MaterialTheme`, or names any type from the crate. Every menu, the dialogue box
  and the HUD are plain `bevy_ui` drawn through `ambition_menu`'s own backend,
  with typography this repo owns outright (`MenuFont`, `MenuTextHeightFraction`,
  `resolve_menu_text_size`).

  ⛔ **and the monolith wires it into a feature with no code behind it** —
  `crates/ambition_platformer2d_actor_monolith/Cargo.toml:289` lists
  `dep:bevy_material_ui` under `ui`, and **zero** `.rs` files in that crate name
  the crate. Enabling `ui` drags 160 s of cold compile in on behalf of nothing.

  **⛔ THE STALE CLAIM THIS ROW RETRACTS.** `plugins.rs:827-838` carries a
  `⛔ do not simplify this` warning I wrote, saying `DialogPlugin` is load-bearing
  because removing it makes the title render at 20px "because menu typography
  stops resolving." **The mechanism is unsupported.** Read upstream
  `bevy_material_ui-0.2.7/src/dialog.rs:25`: `DialogPlugin` adds three messages,
  two resources, `Startup: setup_dialog_overlay`, and 16 `dialog_*` systems.
  `lib.rs:524`: `MaterialUiCorePlugin` inits `MaterialTheme` + `MaterialLocale`
  and adds i18n/focus/ripple/icons. **Neither initializes the crate's
  `Typography` resource.**

  ⭐ **the arithmetic refutes it independently, and this is the transferable
  part.** `ambition_menu/src/render/bevy_ui/spawn.rs:50` spawns menu text at
  `MenuTextHeightFraction(size).reference_pixels()` = **60px** for the 5.6% title;
  `resolve_menu_text_size` only *corrects* that against the live window. So a
  resolver that never ran leaves a **60px** title and
  `the_title_screen_says_choose_game_and_is_readable` **passes** (`>= 32.0`). A
  20px reading is Bevy's `TextFont::default()` — an entity the menu spawner never
  touched. ⇒ **the bisect proved "removing this flips the test", and I wrote down
  "removing this breaks typography".** Those are different sentences, and the six
  bisect steps only ever supported the first. This is
  `a_comment_describes_intent_not_the_code` in its most expensive form: a guess
  installed as a `⛔` warning, which then deterred the next reader for five days.

  **The suspected real defect** (probe running, NOT yet confirmed): the test does
  a global `Query<(&Text, &TextFont)>` and picks the title with
  `.find(|(label,_)| label == "Ambition")` — **first match in archetype order**.
  There is at least one other `Text::new("Ambition")` (`scene_setup.rs:292`, the
  debug HUD). Adding or removing a plugin changes registration order, so a plugin
  with no typography behaviour can still flip the assertion. ⚠ the HUD spawns at
  14px, not 20 — so the population is not yet fully accounted for and the row
  stays open until the dump says what is actually there.

  **Fix the test either way** (GPT 5.6, and it is right): a launcher-title test
  must not search every `Text` in the app and take the first string match. That
  makes **content text function as identity** and imports query-order dependence
  into an assertion about typography. Scope it to the launcher root or a stable
  semantic marker. This lands regardless of what the probe finds.

  **Definition of done.** (1) name the entity that produced the 20px reading;
  (2) scope the test to semantic launcher ownership; (3) fix any genuine ordering
  defect the probe exposes; (4) both plugins out; (5) `dep:bevy_material_ui` out
  of the monolith `ui` feature — ⚠ the *feature* stays, it still owns yarnspinner
  and dialog for its callers; (6) `cargo tree -i bevy_material_ui` proves it left
  the **active graph**, not just the grep; (7) schedule census delta — ⛔ report
  Material's OWN share, the previously-quoted "428" is the whole `Update`
  schedule; (8) targeted launcher/menu tests + the repo-tooling contracts job
  (a new/removed dep edge needs `fixtures/minimal_game/Cargo.lock` regenerated
  and committed WITH the manifest); (9) after-telemetry in the **same profile** as
  the baseline above.

  ⛔ **not in scope: a replacement widget framework.** `bevy_egui` is
  immediate-mode with its own render pass and does not participate in `bevy_ui`
  layout; `bevy_feathers` is Bevy 0.18's experimental, editor-oriented, unstable
  widget set. Adopting either would replace a dependency we do not use with a
  dependency we do not use. Target is plain `bevy_ui` + the typography already
  owned. If a dev-tools panel ever wants egui, that is a separate opt-in decision
  that stays out of the shipped graph.

- ✔ **D34 ANSWERED 2026-08-08 — it is NOT the frontend, and the "already
  falsified" dead end was the answer.** Full working, method and confidence
  grading: `dev/journals/compile-cost-what-actually-drives-it-2026-08-08.md`,
  final section. Measured cold, own target dir, `CARGO_INCREMENTAL=0`, one unit
  per run, idle machine.
  * ⭐⭐ **the frontend claim is off by 14x.** The crate's whole rustc frontend is
    **1.8 s of a 28.1 s dev build (6.5%)**; `type_check_crate` is 1.1 s. The
    monolith compiles in **28.0 s** — the same — for 7.6x the lines.
  * ⭐⭐ **`frontend_seconds` in the ledger is TIME-TO-RMETA, not the frontend.**
    Metadata encoding needs `exported_symbols`, which forces the monomorphization
    collector first: `cargo check` spends **0.008 s** in `generate_crate_metadata`
    with no collector at all; the link build spends **12.87 s**, of which
    **11.29 s is `monomorphization_collector_graph_walk`**. The column has now
    produced two wrong conclusions and `dev/compile_telemetry_schema.md` says so
    at the column.
  * ⭐⭐ **the cost is 150,261 monomorphized instantiations** — 10,140 per 1000
    lines against the monolith's 579 — at an ordinary per-item rate (75 vs 59 µs).
    The monolith *defines* the engine's systems and instantiates **zero** system
    wrappers; the runtime *registers* them and instantiates **1,205** system types
    and 866 query shapes. Registration is where Bevy's ECS generics land.
  * ⛔⛔ **the handed-over "dead end" was never tested.** Re-running the
    `rollback/domains/` subtraction against a **build** instead of `cargo check`:
    **28.13 s → 8.66 s (−69%)**, instantiations **150,261 → 42,725 (−71.6%)** —
    and `type_check_crate` moves **1.118 → 1.165**, i.e. nothing, which is exactly
    what the old experiment measured and reported as "Refuted". `cargo check`
    cannot monomorphize, so it was blind to 94% of the cost. The probe was
    reverted; the tree was verified clean.
  * **release**: 73.7 s cold, and it is LLVM (`LLVM_passes` 48%, `LLVM_thinlto`
    29%, frontend 1.4%) — same 150k instantiations, optimised instead of walked.
    ⚠ neither `run_tests.py` nor `goal_guard.py` builds release, so **in the
    profile the loop actually pays, this crate ties the monolith.** The 3.2x
    release gap that motivated this row is real and is paid by
    `compile_collect.py --config release` and by shipping.
  * ⭐ **the `opt-level = 0` override is working and wants nothing added** — it
    removes ThinLTO from this crate entirely (9.3 s of the monolith's build).
  * ⛔ **what it does NOT license**: it prices REMOVING the domains, not RELOCATING
    them. A move relocates ~107k instantiations across ~11 crates rather than
    deleting them, and `rollback/domains/mod.rs` already records why they live
    here (the vocabulary is here; domain crates must not depend on the runtime;
    R1's schema-vocabulary extraction comes first). ⭐ **the falsifier is cheap and
    specific: relocate ONE domain and measure the whole workspace build both
    ways.** Nothing here measures that, and §4 of the same journal says the
    per-crate-toll question is unsettled.
  * ⚠ **scope was fenced and held**: measurement only. No carve, no module moves,
    no rollback surgery, no new compile-analysis infrastructure — the two probes
    used `-Ztime-passes` and `-Zdump-mono-stats`, both built into nightly, and
    nothing was installed.
  * ⚠ **n**: the baseline is 2 runs (28.13 / 26.32 s, 6% apart); the subtracted
    build is **1**. A repeat costs 40 s and has not been run.

- ▢ **D33 ACTOR-MONOLITH DECOMPOSITION — continue only with a coherent ownership cut.**
  The `ambition_character_sprites` carve and its compile-cost measurement are complete.
  What remains is the broader decomposition: re-measure the current graph, choose the next
  boundary that actually lowers dependency/change amplification, and preserve the plugin
  shape so moving code does not recreate an owner→carve up-edge. Current candidates include
  `character_runtime` and the lifecycle/session Wave-B seam; `character_sprites/assets.rs`
  should stay with its owner if moving it would immediately recreate the dependency.
  Technical authority: [`engine/actor-monolith-decomposition.md`](engine/actor-monolith-decomposition.md).
  The completed D33 investigation/measurements are archived in
  [`../archive/planning-superseded/2026-08-13/queue-pruned-sections.md`](../archive/planning-superseded/2026-08-13/queue-pruned-sections.md).

- ▢ **D28 MEASURE COMPILE AND TEST TIME IN THE CONTEXT OF REAL WORK.** Jon,
  2026-08-08, verbatim:
  > *"I still want to continue measuring how compile time is impacted over time
  > so we can nail down our understanding, but that also needs to be done in the
  > context of performing work. I want to see the effect of real workflows on
  > compile and test time, which will help us make them generally faster and show
  > that we have, and also inform how often we should be running some of these
  > commands versus adding code in batches, or working in the background while
  > compiles and tests run."*
  ⭐ **this reframes every instrument built today.** `compile_collect.py` runs
  SYNTHETIC builds in their own target dir; `compile_ratchet.py` builds nothing;
  the `-Ztime-passes` work gave one crate the whole machine. **None of them
  observes the loop an agent or Jon actually pays.** The journal's own §5 says
  the two kinds of number answer different questions — this asks for the one
  nobody is collecting.
  * **three outputs Jon named, and they are different asks**: (a) make the loop
    faster; (b) **demonstrate** that it got faster, which needs a comparable
    series over time rather than one-off runs; (c) **decide the working
    RHYTHM** — how often to run a check, whether to batch edits, and when to
    work in the background instead of waiting.
  * ⭐ **(c) is the one no instrument here can answer today**, and it is the one
    with daily leverage. It needs edit→check→test cycles recorded *as they
    happen*, with what was being done, not builds staged for measurement.
  * ⚠ **the observer problem is already measured and is the main design
    hazard**: the goal guard's own Stop-hook checks run `cargo check -p
    ambition_app` and the `app_it` suite on the DEFAULT target dir every turn —
    833.9s vs 540.0s for the same 688 units, a 35% swing. **A passive recorder
    must stamp contention** (`getloadavg`, a count of foreign cargo processes)
    or it will record the supervisor's reporting cadence as if it were the code.
  * ⭐ the cheap shape: a wrapper or hook that appends a row per real cargo
    invocation — command, profile, units dirty/fresh, seconds, load average,
    concurrent cargo count, and a free-text "what was I doing". `run_tests.py`
    already writes `dev/ambition_dev_measurements/run_tests_cost.jsonl` for the suite; the gap is the
    per-edit `cargo check` loop and the correlation between them.
  * ⛔ **and the cheapest outstanding item from D24 belongs here**: all 2,145
    collector rows read `incremental = false`, so a whole axis of the earlier
    question is unanswered. One collector run with `CARGO_INCREMENTAL=1` closes
    it, and the real-workflow recorder makes it permanent.

  ### ⛔⛔ FIRST FINDING, and it needed no new instrument at all

  `.goal/state.json` records **`blocks: 114`**, `first_block_at
  2026-08-08T01:48:25Z`, `last_block_at 13:05:12Z`. Every one of those is a Stop
  hook that ran **`cargo check -p ambition_app` AND the full `app_it` suite** —
  `goal_guard.py` runs *"Every check, always — a partial answer would hide the
  second open item."*

  **So the exact loop Jon is asking about executed 114 times in 11h17m, and the
  guard stores no duration for any of them.** Three instruments were built today
  for synthetic builds and none observed this.

  Order-of-magnitude, with the assumption stated: `app_it` measured 197.6 s and
  210.7 s in back-to-back runs today (D23's baselines). At ~205 s,
  **114 × 205 s ≈ 6.5 hours of suite time inside an 11.3-hour window.**
  ⚠ **that is an upper bound and probably a loose one** — most of those runs
  rebuilt nothing, so their cost is test *execution* plus a warm no-op build, and
  the one job that reports both halves splits 54.8% build / 45.2% run. A tighter
  estimate needs the thing that does not exist: the timing.

  ⭐⭐ **the answer to Jon's "how often should we run these" is therefore
  probably: far less often than 114 times, and the cadence is set by MY reporting
  rhythm rather than by the code.** A turn that edits one planning document pays
  the same full suite as a turn that changes combat.
  ⛔ **and this is self-inflicted in a way the earlier measurement already
  half-knew**: the 35% contention swing (833.9 s → 540.0 s for the same 688
  units) was *caused by these checks*, so the guard both dominates the machine
  and corrupts every timing taken while it runs.

  ### ✔ LANDED `0af0e718b`, AND THE FIRST ROW ALREADY ANSWERS THE RHYTHM QUESTION

  The guard now times every check and appends a row to `.goal/check_cost.jsonl`
  (under `.goal/` deliberately — the "nothing is left uncommitted" check excludes
  that path, so the recorder cannot fail the run it measures). First real row:

  | seconds | check |
  |---|---|
  | 0.5 | the app compiles (warm `cargo check -p ambition_app`) |
  | 2.0 | the architectural absence contracts |
  | **158.4** | **the `app_it` suite** |
  | 0.1 | nothing is left uncommitted |
  | **161.1** | **total** — load 2.32 → 5.9 |

  * ⭐⭐ **98% of a Stop check is the integration suite.** The other four checks
    together are 2.6 s. Every question about cadence is a question about that one
    command and nothing else.
  * ⭐ **`cargo check` warm is 0.5 s.** So the "should I check more often or batch
    edits" trade is not about checking — checking is free. It is entirely about
    when to run the SUITE.
  * ⭐ the earlier estimate is now grounded: 114 blocks x 158.4 s ≈ **5.0 hours**
    of suite inside an 11.3-hour window. I had guessed ≈6.5 h from a 205 s figure
    that included build time; the honest number is lower and still enormous.
  * ⚠ **one sample, and a WARM one** — `cargo check` at 0.5 s means nothing was
    rebuilt. A turn that edits Rust will show a different shape entirely, and
    that contrast is exactly what the ledger now accumulates. ⛔ **do not tune the
    cadence on one row.**
  * ⭐ **the actionable shape once there are more rows**: split `app_it` so a
    docs-only turn runs a cheap subset and a combat-touching turn runs the whole
    thing. That is a real fix rather than "run it less", which trades correctness
    for speed — the move this ledger keeps catching.

  ### ▢ THE ACTIONABLE FIX, SCOPED — and it needs Jon, because it weakens a gate

  `app_it` is **one test binary** (`game/ambition_app/tests/app_it.rs`) with
  **96 `mod` submodules** — collapsed deliberately, because Rust links one binary
  per top-level `tests/*.rs` and that removed ~45 link steps. Its own doc records
  the filter: `--test app_it -- <module_name>`.

  ⭐ **so a cheaper gate is directly expressible today, no restructuring**:
  `cargo test -p ambition_app --test app_it -- <subset>`. And the measurements
  say where the money is — on a warm turn the build is cached, so the 131–216 s
  is almost entirely test EXECUTION, which a filter cuts directly.

  **The design**: the guard picks its suite from what actually changed.
  * any `.rs` touched since the last check (dirty **or** committed) ⇒ **full
    suite**, unchanged;
  * only `docs/planning` and other prose ⇒ a small named smoke subset.
  `.goal/state.json` already stores `last_head`, so "what changed since the last
  check" is available without new bookkeeping.

  ⚠⚠ **why this is Jon's call and not mine.** It makes the gate weaker on
  purpose, and *"we skipped the suite"* is exactly how a regression hides. My own
  note two paragraphs up says do not cut cadence before measuring — I have now
  measured, which earns the proposal, not the decision.
  * ⭐ **the case FOR**: most turns in this run edit only planning prose, and each
    pays 131–216 s of combat, boss, rollback and rendering tests that no prose can
    break. Across 114 blocks that is the single largest cost in the session.
  * ⛔ **the case AGAINST, stated honestly**: "only docs changed" is a claim about
    the diff, and this repo has been bitten repeatedly by claims about diffs. A
    generated file, a submodule pointer, or a `.ron` the tests read would all look
    like "not Rust" while changing behaviour. **The subset must be chosen so the
    failure mode is a slower turn, not a missed regression** — e.g. anything
    outside `docs/` forces the full suite, rather than a list of "safe" paths.
  * ⚠ and it must be a NAMED subset with a stated reason per module, not "the
    fast ones". A subset chosen by runtime is a subset nobody can defend.

  **The cheapest first move — and it is agent tooling, so it is in bounds**:
  stamp a duration per check into `.goal/state.json` (or its own ledger).
  114 samples of the real loop already happened unmeasured; the next 114 need not.
  ⚠ **do NOT reduce the guard's cadence as a "fix" before measuring it** — the
  checks are what keep the run honest, and trading correctness for speed on an
  unmeasured guess is exactly the move this ledger keeps catching.

- ✔ **D29 LANDED `437c73868` — and the agent's design beat the brief.**
  ⭐⭐ **it did not COPY the context into the key; it MOVED it in and deleted
  `LiveConversation.context`.** `mint` now takes `&DialogueContext` and
  `context()` reads the whole value back out, so a caller **cannot** mint an
  identity for one context and enter Yarn with another. My brief asked for the
  ids to be included; that would have left two copies of one fact in step by
  convention. **This is the sixth "one question, two answers" closed by
  construction today**, and the only one where the agent improved on the
  instruction rather than executing it.
  * `speaker_is_self` deliberately **not stored** — a pure function of the two
    ids, and `DialogueContext::between` is the one place that comparison is made.
  * ⭐ **`ConversationInputOwner` deliberately EXCLUDED, and the reasoning is
    better than the review's suggestion to include it.** It derives from the
    initiator's `Brain`, which **possession transfers at runtime** — so keying on
    it would make *"somebody took over the body mid-sentence"* a DIFFERENT
    conversation, the in-flight narrative end would stop matching, and the
    projection would restart the runner from the top under a player who is
    mid-sentence. **That is precisely the defect the attachment memo exists to
    prevent.** It publishes nothing into Yarn and is re-read off the rollback
    authority every tick, so a correction repairs it without identity's help.
  * ✔ `actors.rs:197`'s separate context hash is **deleted** — after the fold it
    could not compile, and the comment two lines up already says the instance id
    is hashed WHOLE *"so a new ingredient joins the fingerprint by
    construction"*. `input_owner` stays hashed, with a note that "do two peers
    agree about the live conversation" is strictly broader than "is this the same
    conversation".
  * ✔ schema **v18 → v19**, all three artifacts in one commit plus the queue
    doc's `v18` line that the repo-tooling job asserts against source. Descriptor
    list byte-identical — the wire-change class only the version constant sees.
  * ⚠ **the no-`SimTick` case is NARROWED, NOT CLOSED** — a re-wear between two
    visits now separates them; two visits under identical identities at a
    standing tick still collide. Documented in `instance.rs` rather than folded
    into the "degenerate clock" note, because calling it that is not answering
    it. No nonce was invented.
  * ⚠ it transiently reddened `test_compile_ratchet.py` (its +403 lines moved the
    tree-dependent assertion); fixed independently in `bf923ce27`. **Two agents
    and a measurement interacting is itself a D28 datapoint.**
  The original row, kept for the diagnosis it carries:
- ⊙ **[original row, superseded by the ✔ above] D29 `ConversationInstanceId` stops one layer short — GPT 5.6, 2026-08-08,
  MEDIUM-HIGH and the review's #1 priority.** ⚠ **VERIFY BEFORE WORKING** per the
  charter; the claim is specific enough to check in minutes.
  The id is `(opened_at, node, initiator SimId, talker SimId)`. The review's
  argument is that `LiveConversation` carries further **simulation-authoritative**
  opening facts that the key omits — chiefly `DialogueContext`, which publishes
  `$speaker_id` / `$listener_id` / `$speaker_is_self` into Yarn, and whose speaker
  identity resolves from the initiator's **current `WornCharacter`** — runtime
  mutable AND rollback-owned.
  * so two corrected timelines can share tick, both `SimId`s and node while
    differing in `WornCharacter` ⇒ different `DialogueContext` ⇒ **same instance
    id**. Two failures follow: `project_the_dialog_ui_from_the_conversation`
    concludes "same conversation" and leaves Yarn's variable storage carrying the
    abandoned branch's context; and every instance-gated
    `NarrativeInputLedger` record from the old context matches the corrected one.
  * ⭐ **the invariant the review proposes, which is the deliverable**: *if two
    authoritative conversation openings can cause Yarn to observe different
    narrative semantics, they must not share a conversation-instance identity.*
  * ✔✔ **VERIFIED BY THE SUPERVISOR, 2026-08-08 — every link in the chain holds.**
    1. `conversation/instance.rs:53` — the id is exactly those four fields;
       `context` is not among them.
    2. `conversation/authority.rs` — `LiveConversation` carries
       `pub context: ambition_dialog::DialogueContext` as a sibling of
       `pub instance`.
    3. `conversation/opening.rs:82` — `speaker_id()` ends
       `.or_else(|| self.worn.get(body).ok().map(|worn| worn.id().to_string()))`,
       so the speaker **does** resolve from `WornCharacter`.
    4. `rollback/mod.rs:360,445,466` — `WornCharacter` is rollback-registered and
       those comments discuss *"a rewind that restores an EARLIER
       `WornCharacter`"* directly. So it differs across corrected timelines.
  * ⭐⭐ **and there is corroboration the review did not cite**: the rollback probe
    at `rollback/domains/actors.rs:197` already hashes `live.context.speaker_id`
    **separately from the instance id**. So the tree ALREADY treats the dialogue
    context as identity-relevant — just not in the type whose whole job is
    identity. ⛔ **that is today's recurring shape again** (one question, two
    answers, only one authoritative), and it means the fix has a witness already
    in the tree rather than needing one invented.
  * ⚠ it also flags `ConversationInputOwner` for the same analysis, and
    explicitly says `speaker_name` is presentation and must NOT be added merely
    to make the struct exhaustive. ⛔ and it says to fix the authoritative-context
    issue rather than invent a nonce for the no-`SimTick` composition — the
    "degenerate clock" answer sidesteps the contract instead of satisfying it.

- ⭐⭐ **D30 SUPERVISOR INTEGRATION — I UNDERSOLD THE SEVERITY, and the agent
  found the reason.** I wrote *"one authoring step away"*. It is the **DEFAULT
  outcome of the obvious authoring action**: `character_catalog/entry.rs:268-275`
  (`AxisTuningSpec`) exposes `flight_direct_velocity` and `flight_invariant_speed`
  as **adjacent `#[serde(default)]` knobs**, and defaults `flight_terminal_speed`
  to `FLIGHT_TERMINAL_SPEED` = **760**. So authoring a direct-velocity
  relativistic flyer the natural way — write those two fields, inherit the rest —
  commanded exactly 760 against c=600. ⚠ **latent because nobody had authored one
  yet, not because it was hard to reach.**
  * ⭐ **it also refused a scarier claim after checking it.** It suspected the boss
    path was worse (`spawn_actors.rs:784` forces `flight_direct_velocity: true`
    and `BOSS_FLIGHT_SPEED` is 1200 — *twice* c) and found it unreachable:
    `ambition_combat`'s `BodyMovementTuning::body_tuning` builds over
    `MovementTuning::default()`, whose `invariant_speed` is `None`. A different
    type from the catalog's `AxisTuningSpec`. **Verified by reading, not assumed**
    — and a 2c boss would have been the headline if it had been true.
  * ⭐ **why validation / unrepresentable were rejected, which is the durable
    fact**: `flight_terminal_speed` is **mutated in place at runtime** — dev-tool
    sliders and `enemies/integration.rs:284`, which overwrites it every tick from
    chase speed. A validated constructor or newtype cannot hold a field three
    sites assign after construction. ⛔ **that rules out a whole family of "make
    it unrepresentable" fixes in this crate**, and is worth knowing before anyone
    proposes one again.
  * ⭐ **the target keeps the RAW terminal on purpose** — `enemies/integration.rs:300`
    normalises a boss's `velocity_target` by `flight_terminal_speed` to recover
    stick deflection, so scaling the target by a lowered cap would slow every
    *subluminal* command (a 400 px/s command would drop to 316). Only the clamps
    move. I had not seen that and would have briefed the slower fix.
  * ⚠ **stated non-coverage**: run / fall / dash speeds are NOT c-bounded when a
    flight invariant is set. The doc promise is the flight limb's; a whole-body
    "nothing exceeds c" belongs to a spacetime model, not to movement tuning.

- ✔ **D30 LANDED 2026-08-08 — the subluminal guarantee is now a POSTCONDITION of
  the flight limb, not a property of one branch.** The invariant-speed API did
  not enforce its own invariant on the direct-velocity path — GPT 5.6, MEDIUM.
  `MovementTuning::flight_invariant_speed` documents a guaranteed subluminal
  coordinate velocity. The integration branches
  `if direct_velocity { take target verbatim } else if invariant_speed { relativistic }`,
  and the later radial clamp uses the **raw authored `flight_terminal_speed`**
  rather than `min(terminal, c)`. So `direct_velocity = true`,
  `flight_terminal_speed = 760`, `flight_invariant_speed = 600` yields a
  coordinate speed of 760.
  * **latent, not live**: current TwinTrack content has direct-velocity false and
    a terminal below c. ⚠ **but the new test suite deliberately exercises the
    combination**, so it is a supported surface, not absurd authoring.
  * ✔✔ **VERIFIED BY THE SUPERVISOR — the review is right on every point**, in
    `crates/ambition_platformer2d_core/src/movement/integration.rs:684-757`:
    - `:690` `if tuning.flight.direct_velocity { (target_run, target_descend) }`
      returns verbatim and **skips the relativistic branch entirely**, even when
      `invariant_speed` is `Some`;
    - `:684` the target is `local_stick.x * tuning.flight.terminal_speed` — the
      raw authored terminal;
    - `:746` the per-axis clamp is `±tuning.flight.terminal_speed`, raw;
    - `:749` the radial clamp fires on `invariant_speed.is_some()` and normalizes
      to **`tuning.flight.terminal_speed`**, also raw.
  * ⭐⭐ **the correct value is already computed THREE LINES AWAY and simply not
    used by the clamps.** Inside the relativistic branch, `:703` reads
    `let terminal = tuning.flight.terminal_speed.abs().min(c * (1.0 - 1.0e-5));`
    — the c-bounded terminal exists, scoped to the one branch that does not need
    the clamp. ⛔ **one question, two answers, again**: the branch knows the
    terminal is capped by `c` and the clamp does not. **The minimal fix is to
    hoist that binding** so both the branch and both clamps read one value.
  * ✔ **and "latent" is confirmed, not assumed.** TwinTrack
    (`demo_twintrack/src/lib.rs:150-153`) authors `flight_terminal_speed: 540.0`,
    `flight_direct_velocity: false`, `flight_invariant_speed: Some(600.0)` —
    terminal below c, direct velocity off. No `.ron` in the tree sets
    `flight_invariant_speed` at all.
  * ⚠⚠ **but the review's "760/600" example is NOT hypothetical — both numbers
    are real values in this tree.** `platformer_defaults.ron:74` authors
    `flight_terminal_speed: 760.0`, and 600.0 is TwinTrack's invariant. So the
    defect is one authoring step away: enable `direct_velocity` on a body that
    inherits the default terminal while setting an invariant, and you get 760
    against a c of 600. That is a plausible accident, not a contrived one.
  * ⭐ the review asks for the fix to be **structural**: whenever
    `invariant_speed` exists the final velocity postcondition is `< c`, whichever
    flight-control policy produced it — or make the combination unrepresentable.
    ⛔ relying on today's TwinTrack constants defeats the point of putting this in
    *shared* movement tuning.
  * ✔✔ **LANDED — the POSTCONDITION route.** `FlightTuning::coordinate_speed_cap`
    is the one bound the limb enforces on its output: the authored terminal, held
    strictly below `c` when an invariant speed exists. `integrate_flight_clusters`
    binds it ONCE above the policy switch, and the branch plus both clamps read
    that binding. A future fourth control policy inherits the bound because the
    clamps are downstream of every arm. ⚠ the *target* deliberately keeps the RAW
    terminal: `enemies/integration.rs:300` normalises a boss's `velocity_target`
    by `flight_terminal_speed`, so lowering the target's scale would slow every
    *subluminal* command too. Only the clamps move.
  * ⛔ **validation / unrepresentable were both rejected, and for one decisive
    reason**: `flight_terminal_speed` is MUTATED IN PLACE at runtime — dev-tool
    sliders (`dev_tools/editable.rs`), and `enemies/integration.rs:284` overwrites
    it every tick from the body's chase speed. A validated constructor or a
    newtype cannot hold a field that three sites assign after construction, so
    validation would have to be re-asserted at each of them; and fusing the three
    flight knobs into one enum to make the pair unrepresentable would also force a
    fallible decode in `motion_codec.rs` for a value that is trivially clampable.
    Rejecting the combination also *forbids* something an author may reasonably
    want ("fly at the engine default terminal, but you cannot reach `c`").
  * ⚠⚠ **and the "one authoring step away" reading was CHARITABLE — 760/600 is
    the DEFAULT for a row that opts in.** `character_catalog/entry.rs:268-275`
    (`AxisTuningSpec`) exposes `flight_direct_velocity` and
    `flight_invariant_speed` as adjacent `#[serde(default)]` knobs and defaults
    `flight_terminal_speed` to `FLIGHT_TERMINAL_SPEED` (760). A catalog row that
    writes only those two fields — the natural way to author a direct-velocity
    relativistic flyer — commanded exactly 760 against a `c` of 600. The probe
    authors precisely that.
  * ✔ **no shipped behaviour changed.** TwinTrack authors terminal 540 < c 600,
    so its cap is unchanged at 540; the monolith's flight paths carry
    `invariant_speed: None` (`BodyMovementTuning::body_tuning` builds over
    `MovementTuning::default()`), so their cap is the raw terminal exactly as
    before. Both pre-existing flight tests (`diagonal_free_flight_…`,
    `proper_velocity_free_flight_…`) pass unchanged.

- ⊘ **D31 BLOCKED ON JON — not owed work.** The review calls this a live bug;
  Jon's own observations file already examined the identical mechanism and called
  it documented design plus a product question he owns, and the charter says his
  file outranks inferred work. A new row in `awaiting-maintainer-decision.md`
  asks the broader version (should "declared nothing" mean the dev kit for ANY
  character?). ⭐ what proceeds without him: shaping the preparation seam so
  body-owned intrinsic capabilities are EXPRESSIBLE. Flipping the default is his.
  The original row, kept for its verification:
- ⊙ **[original row, superseded by the ⊘ above] D31 the Blink capability bug is LIVE and no commit in this range fixed it
  — GPT 5.6.** ⚠ the review is explicit that the decomposition plan diagnoses it
  correctly and that **nothing here should be mistaken for having landed it.**
  `session/setup.rs` still does
  `character_catalog.ability_set(..).unwrap_or_else(|| editable_abilities.as_engine())`,
  and `platformer_defaults.ron`'s shared set includes Blink, precision Blink and
  the Blink wall permissions — so *"this character declared no intrinsic
  abilities"* still means *"inherit the broad session/development kit"*.
  ⭐ the destination the plan already names: prepared character/body definition →
  intrinsic body abilities → explicit session/dev restriction mask → movement
  systems. Review ranks it #4, as an early monolith-decomposition seam.
  ⛔⛔ **CROSS-CHECK DONE — AND IT RE-SCOPES THIS ROW. JON HAS ALREADY RULED.**
  `JONS_OBSERVATIONS_BUGS_AND_ISSUES.md:248` describes the **identical
  mechanism** and reaches a different verdict:
  > *"▢ **blink is still there, and it is a QUESTION FOR YOU rather than a bug.**
  > It comes from the home body's own `AbilitySet` (the dev `EditableAbilitySet`
  > grants the full traversal kit), which is the documented "the box keeps its
  > traversal kit" design. **Whether Sanic should keep blink is product, not
  > repair** — say the word either way."*
  * ✔ **the mechanism is verified and matches both accounts.**
    `session/setup.rs:199-201` is
    `character_catalog.ability_set(..).unwrap_or_else(|| editable_abilities.as_engine())`,
    and its own comment states the behaviour — *"A row without an authored set
    keeps that shared sandbox set"*. `platformer_defaults.ron:18-21` grants
    `blink`, `precision_blink`, `blink_through_soft_walls`,
    `blink_through_hard_walls`, all true.
  * ⛔ **so the review calls "bug" what Jon examined and called documented design
    plus a product question he owns.** Per the charter — *"Jon's own observations
    file outranks inferred work"* — **this row is NOT owed work. It is BLOCKED on
    Jon**, and one word from him unblocks it in either direction.
  * ⚠ **the review's scope is broader than Jon's note and that part is fair**:
    Jon's ruling is about *Sanic keeping blink*; the review's point is that
    "declared no abilities ⇒ inherit the full dev kit" is the wrong DEFAULT for
    any character. That is a design change to a design Jon has called documented,
    so it still needs his agreement — but the question to put to him is the
    broader one, not just Sanic's.
  * ⭐ **what IS in scope without him**: D33's brief asks only that the character
    preparation seam be *designed so it naturally supports body-owned intrinsic
    capabilities*. Shaping the seam is architecture; flipping the fallback is
    product. Do the first, not the second.
  * ⚠ my grep for this fallback initially returned NOTHING because the call spans
    two lines — the regex-over-Rust trap in its absence direction. The file the
    review named had it exactly where the review said.

- ✔ **D32 FIXED `3a57eaa9a`.** Retracted in place rather than deleted, and the
  reason that only becomes statable now a counter exists was added: a counter is
  rollback state and a derivation is not, so deriving keeps these ordinals stable
  across a rewind for free. ⭐ **closed in the same breath as the work** — five
  rows today landed and kept their marker, and closing on integration has not
  fixed it. The row closes when the commit lands, not when someone notices.
  The original row:
- ⊙ **[original row, superseded by the ✔ above] D32 a comment I wrote went false two commits later — GPT 5.6, LOW.**
  `0146789` explains the derived death-drop `SpawnOrigin` sequence partly by
  saying construction-built bodies have `SimId` but **no `SimIdCounter`**, and
  says it was measured. `da1563e` then made `SimId` `#[require(SimIdCounter)]`,
  so the sentence is false in the final tree. ⭐ the ordinals are still correct —
  the uniqueness argument stands on its own — but the supporting architecture
  explanation now misleads anyone deciding whether dynamic children can obtain
  identities. ⚠ review says fix opportunistically, not as a campaign.
  ⛔ **this is the fourth instance today of the same class** — a comment
  describing a RELATIONSHIP to another system, invalidated by a change to that
  other system. Same as `damage_predicates.rs`'s module doc (fixed in D25) and
  `player_schedule.rs`'s hurtbox comment (fixed in `3f9d8c4f3`).

- ⭐ **D25 INDEPENDENTLY CONFIRMED by GPT 5.6** (its finding #2, MEDIUM, ranked
  #2), verified by it from both paths, and it explicitly declines to claim the
  discovery. It agrees with the queued remedy including the `Option<&..>` detail.
  ⭐ **two independent routes reaching the same conclusion is the strongest
  evidence this row will get** — one from reading the D23 fix's neighbourhood,
  one from a cold review of the range.

- ✔ **D27 LANDED — `worst_edit_cost_seconds` is the fifth guarded number**, a
  measured-weight sum over the dependency walk the ratchet already did. 10 tests
  in `scripts/tests/test_compile_ratchet.py` (suite 222 → 232), each watched red
  against the naive draft it rejects. **What it found, and what the brief below
  got wrong, is at the end of this entry.** Original row unedited:

  ▢ **the compile ratchet guards LINES, and lines are the one thing measured
  not to predict compile cost.** A critique of D8, which I wrote, on evidence
  that did not exist when I wrote it.
  * **the four guarded numbers are all line counts or graph shape**:
    `largest_unit_lines`, `worst_edit_cost_lines`, `edit_cost_lines`,
    `critical_path_crates`. The first three are lines.
  * ⛔ **`corr(ms/line, lines) = −0.23` across 52 crates.** So "how many lines
    must recompile" does not predict "how long the recompile takes" — the
    quantity the gate exists to protect. The monolith is 111,790 lines at
    **1.13 ms/line** and `ambition_platformer2d_runtime` is 14,746 at **8.55**,
    so a carve that moved 10,000 lines out of the monolith and into a
    runtime-shaped crate would **improve every guarded number while making the
    build slower**. That is precisely the failure the `critical_path_crates`
    guard was added to catch, arriving through a door it does not cover.
  * ⭐ **the fix is available from data already on disk.** `compile_units.jsonl`
    carries measured `seconds` per crate, so `worst_edit_cost_seconds` is a
    weighted sum over the same dependency walk the ratchet already does — the
    graph traversal is unchanged, only the weight per node. Keep the line
    columns as reported context; make the SECONDS one the guarded number.
  * ⚠ **and keep the gate deterministic.** The whole reason D8 refused a
    wall-clock threshold stands: the weights come from a committed ledger, not
    from timing the build at gate time, so the check still builds nothing and
    still cannot fail randomly. A stale weight is a known, reviewable number in
    a file — which is the trade this instrument was designed to make.

  ### ✔ What landed, and where the brief above is wrong

  **The gate now fails on `worst_edit_cost_seconds`** — the same dependent
  closure, summed with a measured `ms/line` per crate times that crate's current
  line count — plus `edit_cost_seconds` for the two watched crates. The four line
  numbers are unchanged and still guarded. Runtime **0.75s**, no build, no clock
  read. `largest_unit_seconds` is printed as context and deliberately not
  guarded.

  ⛔ **the `1.69` / `18.14` ms/line figures in D24's release table, which this
  row quotes, POOL a cold build with a rebuild.** Release has exactly two
  recorded builds: a cold one (541 dirty units) and a first-party rebuild (57).
  D24 averaged them. Split by cache state the monolith is **0.61 ms/line** in the
  rebuild and 2.77 cold; the runtime is **14.77** and 21.50. So the density gap
  the row is built on is **24x, not 10.7x** — the correction makes the case
  stronger. The weights use the REBUILD alone, because "an edit forces these
  crates to recompile, dependencies cached" is what a blast-radius guard asks.

  ⚠ **`run_id` is not the build.** The collector reuses one `run_id` for its cold
  and warm passes, so four of the eight recorded builds share a `run_id` with
  another and a group-by on it silently pools two cache states. Group by
  `build_source`. This is a second face of the trap the brief already names about
  `build_label`.

  ⭐ **release, not `test`, and the reason is inside a single build.**
  `Cargo.toml` pins `runtime`, `render` and `app` to `opt-level = 0` under
  `[profile.dev]` and writes no `[profile.release]` table at all — so a
  test-profile weight table prices three crates on a different setting from the
  other 52, which is the pooling trap arriving without any averaging. Release is
  the only config where all 55 weights are mutually comparable, which is what a
  SUM over a closure needs.

  ⛔ **a fitted `seconds ~ a + b·lines` was tried as the fallback for an
  unmeasured crate and REJECTED at R² = 0.12.** It predicts 24.7s for a
  10,000-line crate where the flat median rate predicts 25.6s — the same answer,
  from a fitted parameter explaining an eighth of the variance. So size cannot
  substitute for measuring a new crate, and the design says so out loud: an
  unmeasured crate is priced at the population median **and raises an `UNPRICED`
  finding** naming it. Zero was the alternative and it is the failure mode where
  a carve looks free.

  **The synthetic demonstration** — 10,000 lines out of the monolith into a
  runtime-shaped sibling that keeps the monolith's dependencies:

  | number | before | after | verdict |
  |---|---|---|---|
  | `largest_unit_lines` | 111,904 | 101,904 | **CARVED — "this is a WIN"** |
  | `worst_edit_cost_lines` | 427,768 | 427,768 | silent |
  | `edit_cost_lines` (monolith) | 249,070 | 239,070 | **CARVED — "this is a WIN"** |
  | `critical_path_crates` | 12 | 12 | silent |
  | **`worst_edit_cost_seconds`** | 1,249.6s | **1,391.2s** | **REGRESSED, +141.6s against a 25.0s budget** |

  ✔ **run against the pre-D27 gate from git rather than argued**: the old
  `evaluate` returns two `CARVED` findings and nothing else for that carve.
  ⚠ **and the guard's limit, tested as such**: priced at the median placeholder
  instead of the runtime's rate the same carve reads **+19.5s**, inside budget —
  so for an unmeasured crate the magnitude arm does NOT catch it and `UNPRICED`
  is the only thing standing there.

  ⭐ **`--carve` now prints the seconds rows, and its first answer was a
  finding**: lifting `conversation` (2,167 lines) out of the monolith makes the
  build **4.2s slower** and the critical path one crate longer, at the median
  placeholder — i.e. before assuming anything pessimistic about the new crate.
  ⛔ **and fixing that revealed a defect in the simulator itself**: it gave the
  simulated crate NO dependencies, so it fell out of every floor crate's closure
  and `worst_edit_cost` reported the carve as removing those lines from the graph
  entirely. It now inherits the owner's dependencies, labelled as the upper bound
  it is.

  ⚠ **not done, and worth knowing**: the weight table rests on **one** release
  rebuild. Because the weights are frozen constants the gate stays deterministic
  either way — a thin sample mis-prices, it never flickers — but a second release
  rebuild would make the median mean something. The obvious follow-up is
  `compile_collect.py --config release` a few more times, not a code change.

- ✔ **D44 LANDED 2026-08-08 — THE BBOX QUAD ROUTE IS IN, AND IT FIXES NONE OF
  JON'S FIVE SIZING COMPLAINTS.** Both halves of that sentence are measured.

  ### What landed

  `sprite_render_size_scaled` now draws the sheet's FRAME at the one uniform
  scale that puts the sheet's own body rectangle on the collision box. Measured
  over the 136-character shipped cast, drawn-body-height ÷ collision-height:

  ```text
    before   0.63 .. 2.97   (135 of 136 characters drawn at the wrong size)
    after    1.00 .. 1.00   (0 of 136)
    worst disagreement between the TWO render-size publishers: 196.5% -> 0.0%
  ```

  ⭐ **`collision_scale` is inert for every character, probed rather than
  argued**: forcing all 34 sheet-authored + 31 catalog-authored values to `777`
  leaves the whole cast measuring identically. It still drives ONE thing — the
  collision box of the non-`Standard` body kinds, which
  `CharacterBodyKind::default_standing_height` deliberately declines to answer
  for. That is the derivation those characters were already left with; it was not
  widened.

  ⛔ **ONE site, not four. The row's four-site list was wrong.** Sites 2/3/4 (crop
  the atlas sub-rect, `sprite_offset`, `feet_anchor_norm`) need NO change,
  because `posed_body_geometry` already demonstrates the correct arithmetic —
  whole frame, uniform scale, offset — and site 1 only had to adopt it. The
  stretch the row warns about came from sizing the quad to the BODY while still
  sampling the FRAME, which is a different (and wrong) reading of "the bbox
  route". The frame padding is transparent; there is nothing to crop.

  ⭐ **and a second fork fell out of measuring it**: `body_pixel_extent` in
  `character_sprites/assets.rs` read the static `body_pixel_bbox` while
  `posed_body_geometry` read `pose_body_bbox` (the per-anim `idle` hurtbox), so a
  body could be COLLIDED from one rectangle and DRAWN from another — up to 1.30x
  on width (`npc_vera_ruin`), 1.12x on height (`npc_davy_hylbert`). One reader
  now: `BodyMetrics::body_pixel_extent`.

  ### ⛔ AND IT CHANGES ALMOST NOTHING ON SCREEN, WHICH THE ROW HAD BACKWARDS

  * **the named falsifier cannot move, and did not.**
    `enemy_body_scale::print_enemy_bodies_against_the_player` asks
    `posed_body_geometry` at `world_per_pixel = 1.0`: its `collision` column is
    the sheet's bbox in sheet pixels, its `render` column the sheet's FRAME in
    sheet pixels, and `x_vs_p`/`y_vs_p` divide the COLLISION column across sheets
    of different pixel densities. Byte-identical before and after. It was cited
    three times as this row's falsifier; it is a report about `.ron` files.
    `print_the_two_render_size_publishers` (same file) is the instrument.
  * ⛔ **the player robot's drawn size did not change.** `[sprite-bind]` says
    `render=79x79` before and `118x118` after — but `[sprite-size] player first
    observed at 118x118` in BOTH runs, because `sync_visuals`' `authored_render`
    branch overwrites the bind on the first frame. **The player was already on the
    other publisher and already correct**; the fix makes the loser agree with the
    winner. Jon's *"the collision / hurt box is larger than the player sprite"* is
    NOT the quad.
  * ⛔ **hardly anything reaches the path that changed.** Probed live:
    `hall_of_characters` 125 of 129 bodies publish `ActorRenderSize`, `duel_arena`
    2 of 2, hub/sandbox 1 of 2 (the other is a gravity switch). So **4 bodies in
    the whole Hall** drew through the fixed function. The Mary-O slop/snake scene
    is byte-identical before and after; the Hall differs only where the
    `pose_body_bbox` fork moved a box.

  ### ⭐⭐ WHERE THE FIVE COMPLAINTS ACTUALLY LIVE — `authored_body`, not `collision_scale`

  The combat-overlay capture of the Hall shows several characters (Imperfect
  Cellular Automaton, Vera Ruin) as a small figure inside a much larger green
  box, and the arithmetic above says the box IS the drawn body. Both are true
  because `body_pixel_bbox` is the ALPHA bbox — *"the extent of the drawing, hat
  and outstretched arms included"*, and for the ICA, its sparkles.
  `BodyMetrics::authored_body` exists precisely to distinguish the drawing from
  the body, and ~~**2 of 190 sheets set it**~~ — ⛔ **that number is WRONG and
  both of us repeated it without measuring. It is 34.** Counted 2026-08-09 over
  every shipped sheet: **34 set it, 0 set it false, 156 omit the field** (it is
  emitted only when true). The "2" came from Jon's observations row and travelled
  into this report unchecked. ⇒ **see D47**, which also carries the two findings
  that change what the fix is: the flag is generator-emitted from 13 `body_inset`
  rig declarations, and `imperfect_cellular_automaton` sets it and is still
  wrong.

  ### (the original analysis, kept for the measurements it carries)

- **D44-ORIGINAL (superseded by the ✔ block above — marker stripped, text kept)
  ⭐⭐ THE BBOX QUAD ROUTE IS ANSWERED, UNIMPLEMENTED, AND IS
  PROBABLY FIVE OF JON'S SIZING COMPLAINTS AT ONCE.** Highest-value item on the board.
  Found 2026-08-08 by asking why Jon's finished issues file contains FIVE
  unrelated characters with sizing bugs.
  * ✔ **Jon ANSWERED it**: *"I think the bbox route sounds ok."* — recorded in
    `awaiting-maintainer-decision.md` and moved to `maintainer-decisions.md`.
  * ⛔ **and it was never taken.** `sheets/geometry.rs:25` is still verbatim
    frame-driven:
    ```rust
    let height = collision.x.max(collision.y).max(8.0) * spec.collision_scale * visual_scale.max(0.05);
    let width  = height * (spec.frame_width as f32 / spec.frame_height as f32);
    ```
    ⇒ `collision_scale` remains *"a reciprocal-of-padding fudge and nothing
    else"* — **116 hand-tuned values approximating a computable quantity** — and
    the width line takes the PADDED FRAME's aspect, which its own comment states
    as the intent.
  * **measured consequence already on record**: 13 scientist sheets on an
    explicit `1.0` render at figure height **0.84** while 33 high-fill sheets on
    the `1.5` default render at **1.42** — two populations of people in one Hall,
    **1.69x apart**.
  * ⚠ **FOUR coupled sites, and the "do them together" warning is an OBSERVATION,
    not an argument** — site 1 was done alone and photographed, and the art
    stretched first try in OPPOSITE directions: snake **2.20x vertical squash**,
    Mary-O **0.65x horizontal**, both matching `bbox/frame` to measurement error.
    `Sprite::custom_size` scales the whole atlas frame into the quad per axis, so
    shrinking the quad without cropping the source divides the padding into the
    body.
    1. `sheets/geometry.rs:25` — height off one constant, width off `bbox.w/bbox.h`;
    2. ⭐ **the load-bearing one** — draw the bbox SUB-RECT, not the whole frame
       (`ambition_render/src/rendering/actors/`). Without it site 1 is not "a quad
       that matches its box", it is a non-uniform scale applied to the art;
    3. `sprite_offset` (`rendering/actors/mod.rs:345`, `:771`) — still computed at
       FRAME scale, floated the snake ~8px off the floor. ⚠ the row originally
       named `feet_anchor_norm` here and that was WRONG for the characters that
       matter: snake and Mary-O take the `sprite_offset` branch, which sets
       `Anchor::CENTER` and never reads it;
    4. `feet_anchor_norm` (`character/sheets/mod.rs:476`) for the sheets that DO
       publish a body.
  ### ✔✔ FALSIFIER RUN 2026-08-08 — IT CONFIRMS, and names Jon's characters

  `cargo test -p ambition_app --test app_it -- enemy_body_scale::print_enemy_bodies_against_the_player --ignored --nocapture`

  ```text
              target     collision        render   x_vs_p   y_vs_p
     player_robot_v3     57x91        224x224       1.00x    1.00x
         solid_snake    117x52        128x128       2.05x    0.57x
             ai_slop    257x167       271x232       4.51x    1.84x
           mary_o_v2     64x120       160x192       1.12x    1.32x
  ```

  * ⭐⭐ **`ai_slop` renders 4.51x the player's width and 1.84x its height** —
    Jon's *"the snake and AI slop are still way too big visually"*, measured.
  * ⭐⭐ **`solid_snake` is 2.05x wide and 0.57x TALL** — stretched wide and
    squashed flat at once, which is Jon's *"the sprite might not match the box
    for the snake"* and is the exact non-uniform signature the stretch
    measurement predicts (a 117x52 body inside a 128x128 frame).
  * ⚠ **`player_robot_v3`: collision 57x91, render 224x224.** A SQUARE quad for a
    tall body. Consistent with Jon's *"the collision / hurt box is larger than
    the player sprite"* — the quad is huge but mostly padding, so the visible ink
    is smaller than the box even though the quad is not.

  ⛔ **AND THE ROW'S OTHER NAMED FALSIFIER IS THE WRONG INSTRUMENT.** I listed
  `hall_scale_spread::print_how_tall_every_character_stands` beside the one
  above. Ran it: it reports **collision-box height and width**, not render size,
  and its headline — *"136 characters measured; tallest
  perfect_cellular_automaton at 452.0px, shortest npc_puppy_slug at 31.0px,
  spread 14.58x"* — is **authored cast diversity**, exactly what a roster holding
  a giant automaton and a puppy slug should show. It can neither confirm nor
  refute this row. ⚠ **I assumed from its name.** `enemy_body_scale` is the
  instrument for this question because it prints collision AND render side by
  side; `hall_scale_spread` answers *"how tall is everyone"*.

  ⭐ what it adds incidentally, from the other side: `solid_snake`'s collision is
  **52 tall x 117 wide** and `player_robot_v3`'s is **91 x 57** — both markedly
  non-square, while their quads are 128x128 and 224x224. **Square quads for
  non-square bodies** is the defect restated.

  ⇒ **two of the five are now MEASURED rather than inferred, and a third is
  consistent.** The remaining two are untested — and ⛔ **JON HAS SINCE RULED THE
  PIRATES OUT OF THIS ROW** (`7ace7b5e7`): *"The pirates in the cover are
  horribly miss-sized. The heavies need to get a little smaller (**this should
  probably be something done in data by the sprite renderer, not in code**)."*
  ⇒ **do not count the pirates as a D44 beneficiary and do not fix them here.**
  Only super Sanic's clipped spikes stays untested inference. ⭐ his steer is
  consistent with the route rather than against it — the bbox path makes the
  SHEET the authority, so a pirate rescale becomes a data change by
  construction.

  * ⚠ **what was VERIFIED vs INFERRED before the run.** Verified: the decision is answered, the
    code is unchanged, the 1.69x spread and the two stretch measurements. **Inferred:
    that this explains the snake/AI-slop size, super Sanic's clipped spikes,
    player V3's oversized hurtbox, the miss-sized cover pirates and D42's patent
    clerk.** Those five share a symptom, not a proven cause — ⭐ **the falsifier
    is cheap: run `enemy_body_scale::print_enemy_bodies_against_the_player` and
    `hall_scale_spread::print_how_tall_every_character_stands`** (both exist,
    both `#[ignore]`d) and see whether the offenders are the low-fill sheets the
    arithmetic predicts.

- ▢ **D45 UNRESOLVED OBSERVATIONS FROM JON — triage only the items that still reproduce.**
  Completed diagnoses/fixes from the original 16-item sweep were removed from the live queue.
  Remaining observations to reproduce or route are:
  * check Jon's Android transition log specifically for `cover gave up waiting`; do not tune
    the settle deadline from ordinary pre-settle `no render family claimed` warnings;
  * combat/Smash: verify attacker self-damage/no-knockback reports, top-left attack VFX if it
    still occurs, and the missing knockdown/getup/tech/getup-attack vocabulary;
  * held items: the portal gun still needs the same Attack-routing behavior other held weapons use;
  * pirate sky: authored rider mounts are restored, but verify the pirates actually ride their
    sharks in play; verify Iron Mary's intended weapon behavior;
  * transitions/camera: reproduce the LDtk-separated-room pan behavior. The gravity-camera
    feature question moved to `awaiting-maintainer-decision.md`;
  * input: evaluate the requested hold-Up-for-2-seconds alternate interact gesture without
    changing the standing rule that a single Up press is not Interact;
  * art/perf: regenerate the misplaced shield-bubble art if it is still visible, determine whether
    title-menu 60 FPS versus gameplay 140 FPS is deliberate frame pacing, and route PCA's C4
    challenge toward the Smash track if still desired.
  Historical diagnoses are archived in
  [`../archive/planning-superseded/2026-08-13/queue-pruned-sections.md`](../archive/planning-superseded/2026-08-13/queue-pruned-sections.md).

- ▢ **D42 THE PATENT CLERK'S RIG IS MESSED UP — and my TwinTrack swap EXPOSED
  it rather than caused it.** Jon, 2026-08-08, in his own scratch notes: *"Patent
  clerk's rig is all messed up. This is something Jon might have to fix in the
  editor."*
  * **measured**, sheet geometry against the toon family:

    | sheet | frame | body bbox | body fills |
    |---|---|---|---|
    | **patent_clerk** | **165×164** (square) | 57×119 @ x:63 | **35% of width** |
    | craig | 164×226 | 149×213 | 91% |
    | alice | 225×252 | 75×205 | 33% w / 81% h |

    The frame is near-SQUARE where the toon family is tall, and the body sits at
    x:63 inside a 165-wide frame — off-centre and small.
  * ⚠ **I did not cause this and I should not claim I fixed it.** `2f8016e48` /
    `9dbf87305` made the patent clerk TwinTrack's traveler; the sheet predates
    both. Putting a character on screen is what made its rig visible. ⭐ but the
    swap IS why it matters now, so it belongs to me to see through.
  * ✔✔ **ANSWERED 2026-08-09: code is NOT involved. The pipeline already
    compensates for every oddity in this rig.** Measured across the toon family:

    ```text
    sheet             frame      bbox            body dx from      feet_anchor
                                                 frame centre        _norm.x
    patent_clerk    165x164   63,19,57x119        +5.5%             +0.0515
    craig           164x226    7, 8,149x213       -0.3%             -0.0061
    alice           225x252   69,32, 75x205       -2.7%             -0.0289
    bob             227x253   77,35, 72x202       -0.2%             -0.0044
    walter          167x210    7, 8,152x197       -0.3%             -0.0060
    architect       174x223    2, 8,163x211       -2.0%             -0.0230
    absurd_general  176x222    0, 0,168x215       -2.3%             -0.0256
    erdish          239x247   88,21, 66x216       +0.6%             +0.0042
    ```

    ⭐⭐ **`feet_anchor_norm.x` IS the body's offset from frame centre, on every
    sheet.** It is not a coincidence and not a tuning knob — the generator emits
    the correction, and the render applies it. ⇒ the patent clerk's off-centre
    body (+5.5%, by far the largest) is **already absorbed**, and my prediction
    that it would draw visibly shifted was wrong.
  * ⇒ **the rig IS odd and the oddity IS real** — a near-square 165×164 frame
    where every sibling is tall, and a body filling **34.5%** of the width where
    craig fills 90.9%. But odd is not broken: D44's quad route scales the frame so
    the BODY lands on the box, and the surplus frame is transparent padding.
    ⇒ **Jon's read is right — if anything is wrong it is an EDITOR fix**, and
    nothing in code needs to change for it.
  * ⚠ **still not observed**, and this row should not claim more than it has: the
    arithmetic says it renders correctly, which is a different statement from
    "somebody looked". ⭐ `capture_scene` costs **167 s** (re-measured today) —
    one picture settles it.

- ✔ **D43's FORK IS FIXED `adbf5f0ac` (merged 2026-08-09) — AND IT IS NOT JON'S
  BUG.** ⛔⛔ **My diagnosis was wrong at its load-bearing link, and the real
  cause is D58.**

  * ✔ **what landed, and it was worth landing**: all three exits now call one
    `surface_prompt(kind, cue, fallback)`. The argument that lets one function
    serve all three is `fallback` — the caller's *independent* proof that a
    surface owns input. Exits with proof pass `Some(verb)`; the no-subject exit
    has none, so it must earn its context from a published cue. The cold-start
    poison holds (no body, no cue ⇒ `Empty`), and the worker **proved the poison
    can fail** by widening the exit and watching it go red.
  * ⛔⛔ **"smash mentions GameMode: NEVER" was FALSE, and it was my grep's
    fault.** `declare_the_select_input_context`
    (`game/ambition_demo_smash/src/lib.rs:1082`) has held a **capturing**
    `SELECT_CONTEXT` at priority 130 since `ca5042eba`, 2026-08-01. **I grepped
    `select.rs` and `select_screen.rs`** — the two files whose names matched the
    screen — **and the arbitration lives in `lib.rs`.** Probing the real host:
    ```
    route=Some("smash_select")  owner=Some(InputContextId("select"))
    prompt=Some("Menu confirm=Some(\"Select\")")  top_cue=Some("None")
    ```
    ⇒ the select screen **already resolved `Menu`**, the stick and confirm
    buttons were **already alive**, and it never reaches the `Empty` exit.
  * ⭐⭐ **THE HABIT TO FIX, and it is precise.** My four-link chain was checked
    link by link and I still got it wrong, because **one link was an ABSENCE
    established by a grep scoped to the files I guessed were relevant.** A
    presence-grep that misses is harmless — you keep looking. **An absence-grep
    that misses ends the investigation.** ⇒ **when a chain depends on "X never
    happens", grep the whole CRATE, not the files whose names match the
    feature.** [[feedback_grep_for_capability_not_type_name]] says a name search
    finds absence reliably — that is true only if the SCOPE is right, and mine
    was not.

- ✔ **D59 RESOLVED — re-verified 2026-08-10 and it links.** `cargo test -p
  ambition_render` runs **101 passed**, and it does so from a COLD crate build:
  `cargo clean -p ambition_render` (8,983 files, 6.4 GiB removed) then a full
  rebuild, still green. ⭐ that is the decisive form, because the row below
  records `cargo clean -p ambition_render` as one of the things that did NOT fix
  it — so this is not the same target dir being lucky.

  ⇒ **the row's own frame was right and its recipe was incomplete.** The symbol
  really was stale state in the shared dir; what cleared it was the dir being
  rebuilt wholesale in the intervening day, not any per-crate clean. ⚠ the
  standing recipe stays as recorded — an `undefined symbol: anon.<md5>` means a
  fresh `CARGO_TARGET_DIR`, and a per-crate clean is not a substitute — because
  that is exactly what the failed attempts below prove.

  ⚠ **and the rust-analyzer decision filed for Jon was NEVER the fix** — it was
  offered as hygiene and labelled as such. It stays open on its own merits, and
  he should not read it as answering this.

  *The row as filed:*

- **D59-ORIGINAL (superseded; marker stripped, text kept) `cargo test -p
  ambition_render` CANNOT LINK — a stale artifact in the shared target dir, and
  the usual recipe does NOT fix it.** Found 2026-08-09.
  ⚠ **PRE-EXISTING and not caused by any change today** — proved by stashing.

  ```
  error: linking with `clang` failed
  mold: error: undefined symbol: anon.01b8339c3d4862ae8039eb9d59d28bad.34.llvm.15476166777788133686
  mold: error: undefined symbol: _ZN9hashbrown3map…3get17hfd434e78da0b7dd7E.llvm.15476166777788133686
  mold: error: undefined symbol: …drop_in_place$LT$…bevy_image..texture_atlas..TextureAtlas$GT$…
  ```

  * ✔ **it is not mine**: `git stash` the only edit in that crate, re-run →
    **byte-identical failure** (`BASE=101`). ⭐ that probe is why this is a row
    and not an hour — [[feedback_ask_if_a_red_test_is_new_first]].
  * ✔ **the workspace is FINE**: `cargo check -p ambition_app --all-targets`
    passes with **zero warnings**, and `app_it` is **322 passed**. Only the
    `ambition_render` **lib-test** target fails to link. ⇒ a check that
    typechecks everything and a link that cannot resolve a private constant are
    not describing the same problem.
  * ⛔ **`cargo clean -p ambition_render` did NOT fix it** — ran it, failure
    unchanged. So the stale side is a DEPENDENCY cargo believes is fresh.
  * ⛔ **and the standing recipe comes up empty**: grepping the md5 **and** the
    `.llvm.<n>` id across every `*.rlib` and every file under `deps/` finds
    **nothing**. The symbol is defined nowhere on disk.
  ### ⛔⛔ THREE CLEANS LATER: "STALE ARTIFACT" IS PROBABLY THE WRONG FRAME

  **Ruled out, so nobody repeats them** — the failure is byte-identical after
  each:

  | attempt | result |
  |---|---|
  | `cargo clean -p ambition_render` (the crate being built) | unchanged |
  | `cargo clean -p ambition_sprite_sheet` (the symbol-name suspect) | unchanged |
  | `rm -rf $CARGO_TARGET_DIR/debug/incremental` | unchanged |

  ⇒ ⛔ **I titled this row "a stale artifact" and I do not know that.** A cache
  problem that survives all three is not behaving like a cache problem, and the
  standing recipe's grep finding the symbol in no rlib fits "never emitted"
  better than "emitted by something old".

  ⭐ **the better hypothesis, from the evidence rather than the habit**: the
  `.llvm.<n>` suffix marks a symbol **internalized by codegen-unit merging**, and
  this workspace builds dev at **`opt-level` > 0** (cargo prints *"dev profile
  [optimized + debuginfo]"*). A `--test` build partitions the crate into
  different codegen units than the plain lib build, so a `--test` binary can
  reference an internalization the lib's CGUs performed. **That is a profile /
  CGU interaction, not rot** — and it would explain why every clean is
  irrelevant, why `app_it` links (it never builds `ambition_render` as a test),
  and why `cargo check --all-targets` is silent (no codegen at all).

  ### ✔✔ BOTH PROBES ANSWERED — IT IS THE SHARED TARGET DIR, AND `clean -p` CANNOT REACH IT

  ```text
  CARGO_TARGET_DIR=<fresh>  cargo test -p ambition_render    →  94 passed, 0 failed   ✔
  (main dir)                cargo test -p ambition_sim_view  →  43 + 4 passed         ✔
  (main dir)                cargo test -p ambition_render    →  LINK FAILURE          ⛔
  ```

  ⇒ **the code is fine and the CGU hypothesis is refuted too.** A fresh target
  dir builds and passes; the control crate links in the poisoned dir. So the
  damage is real, is confined to `ambition_render`'s lib-test artifacts in
  `/home/joncrall/ambition-target`, and **three targeted cleans could not reach
  it** — including `cargo clean -p` on the failing crate itself.

  ⭐⭐ **THE OPERATIONAL RULE, which is the transferable part**: when
  `cargo clean -p <crate>` fails to clear an `anon.<md5>.llvm.<n>` link error,
  ⛔ **stop cleaning and run the crate's tests under a FRESH
  `CARGO_TARGET_DIR`.** It is one command, it costs a cold build of that crate's
  tree only, and it separates *"my code is broken"* from *"this directory is"* —
  which is the question every other step was implicitly guessing at. **I made
  three guesses before running the one probe that decides.**

  ⇒ **the fix is to remove the main target dir and pay the cold rebuild**, or to
  keep using a side dir for this crate. ⚠ **not urgent**: nothing in the goal's
  gate touches it — `app_it` is 322 green, `--all-targets` checks clean — so this
  is a developer-experience cost, not a correctness one. ⛔ **do not nuke the dir
  while jobs are building in it.**
  * ⭐ my two wrong frames, kept because both were plausible: *"a stale rlib"*
    (the standing recipe's grep found the symbol in no rlib — I read that as
    "look harder" rather than as a result) and *"a codegen-unit internalization
    under `opt-level > 0`"* (refuted the moment a cold build passed).

  ### ⭐⭐ AND THE LIKELY POISONER IS `rust-analyzer`, FOUND BY LOOKING AT `ps`

  A verification run blocked on *"Blocking waiting for file lock on build
  directory"*, so I looked at what held it:

  ```
  1652717  cargo check --workspace --all-targets --keep-going  ← rust-analyzer's
  1655411  cargo check -p ambition_app --all-targets           ← mine, waiting
  ```

  **Three `rust-analyzer-mcp` processes are live**, and RA has no target-dir
  override, so its continuous `cargo check --workspace --all-targets` writes into
  **the same `/home/joncrall/ambition-target`** as every build and every test.

  ⇒ ⭐ **that explains what neither earlier frame could**: a `check` run emits
  `.rmeta` where a test build needs `.rlib`, and RA re-populates the directory
  immediately after any `cargo clean -p` — so cargo sees "fresh" artifacts that
  cannot satisfy a link. **It is why cleaning the failing crate changed nothing
  and why a fresh directory works.**
  ⚠ it is also a standing tax on every build today: RA competes for the lock and
  for cores, on a box that already had four agents on it.

  * **the fix is one line and it is JON'S to make** — `.vscode/settings.json` is
    **untracked** (his local scratch, hands off):
    ```json
    "rust-analyzer.cargo.targetDir": true
    ```
    which puts RA's artifacts in `<target>/rust-analyzer` and leaves builds alone.
  * ⚠ **not proven, and say so**: this is the best explanation, not a
    demonstrated one. **The falsifier is cheap** — set the override, remove the
    main target dir once, and see whether `cargo test -p ambition_render` stays
    green through a day of editing. ⛔ do not record it as the cause until that
    runs.
  * ⚠ **the likely origin is mine, operationally**: four worktree agents ran
    today, and although each had its own `CARGO_TARGET_DIR`, the main checkout
    was also building throughout while branches were merged under it. ⇒ **this is
    the cost side of the fleet-size lesson in the standing block**, showing up as
    a corrupted cache rather than as slowness.

  ### ⚠⚠ 2026-08-09, LATER — THE SYMPTOM WENT GREEN WITH THE CAUSE STILL PRESENT

  The D52 worker ran **`cargo test -p ambition_render`** as part of their gate and
  got **94 passed** — it linked. And I checked `ps` at the same time:
  **11 `rust-analyzer` processes are live** (this row counted *three* when it was
  written; it has grown, not shrunk).

  ⇒ ⛔ **do NOT read the green as a fix.** Two readings, and they are not equally
  likely but both survive:
  * **the race simply did not fire** — RA was not mid-`check` when the link ran.
    Consistent with the theory, and the theory predicts intermittency.
  * **RA is not the cause** — the artifact was rebuilt at some point and whatever
    poisoned it is gone.

  ⭐⭐ **but the honest summary is that the theory got WEAKER, not stronger**: the
  hypothesized poisoner was present in force and the failure did not occur. That
  is evidence against *"RA makes this fail"*, and only neutral toward *"RA can
  make this fail sometimes"*.

  ⚠ **this matters because the recommendation is a change to JON'S machine.**
  I have `"rust-analyzer.cargo.targetDir": true` queued as a question for him on
  the strength of a theory that has now failed to reproduce once, with the cause
  present. ⇒ **tell him that when asking.** It is still a good setting on its own
  merits — RA competing for the build lock and for cores is a real, separately
  observed tax — but **it should be offered as hygiene, not as the fix for D59.**

  ⇒ ⭐ **the falsifier in the bullet above is unchanged and is still the only
  thing that settles it**: set the override, remove the main target dir once, and
  see whether it stays green *through a day of editing*. ⛔ **a single green run
  is not that test**, and treating it as one is how an intermittent bug gets
  closed twice.

- ✔ **D58 LANDED `727cf8458` (2026-08-09) — A FINGER CAN NOW DRIVE THE SMASH
  SELECT CURSOR.** The real answer to Jon's *"on android, I could not use touch
  controls on some menus, e.g. in smash"* — found and fixed without hardware.

  * ✔ **red first, and it named the gap precisely**:
    ```text
    a_finger_moves_the_cursor_and_chooses_a_fighter ... FAILED
      left:  Vec2(320.4, 244.0)   <- still where initial placement left it
      right: Vec2(586.0, 446.0)   <- where the finger was
    ```
    ⇒ green at **3 passed**; `cargo test -p ambition_demo_smash` **56 passed**;
    the gate exit 0; `scripts/tests` **287 passed**, so no dep edge was added.
  * ✔ **my coordinate-space claim was verified by a DIFFERENT route** rather than
    from the doc comment I quoted: `bevy_winit-0.18.1/src/state.rs:356` calls
    `touch.location.to_logical(scale_factor())` *before* `convert_touch_input`.
    ⇒ logical top-left pixels, no conversion. ⚠ **had it been physical px, every
    tap on a 2.75× phone would land 2.75× off and the headless test would never
    have shown it** — the one failure mode a desktop test cannot see, closed by
    reading the winit source instead of trusting a comment.

  ### ⭐⭐ THE WORKER'S FINDING I DID NOT HAVE: Android RECYCLES pointer ids

  The obvious multi-touch rule — *"the primary finger is the lowest `id`"* — is
  **wrong on Android, and wrong in a way a desktop test would never catch**:

  > Finger A down (id 0), finger B down (id 1), **A lifts**, finger C lands →
  > C is handed **id 0**, *below* B which is still down. Plain `min_by_key(id)`
  > hands the cursor to C mid-drag.

  ⇒ "lowest id" only *looks* like "a second finger never takes over". The shipped
  rule is **sticky**: the finger already driving keeps the cursor while it is
  down; a second finger neither moves nor clicks; the lowest id breaks the tie
  only when there is no driver. ⭐ **and it matches the mouse arm it sits beside**
  — pressing a second mouse button does not relocate the pointer, and four
  players share ONE cursor here by design.

  ⚠ **the stakes are not cosmetic**: `(Some(slot), _, _) => drop_it()` is a live
  arm, so an intruding press *lands* — the token is dropped or committed to the
  wrong fighter. The test gives the intruder the **lower** id (5 driving, 2
  intruding), which is the only way it discriminates the two rules.

  * ✔ **three falsifiers, each run and each individually red** — swap sticky for
    lowest-id, delete the `just_released` arm, delete the `iter_just_released`
    position lookup. ⇒ **no line of the fix is unpinned.** ⭐ the third is subtle:
    a released touch is already gone from `iter()` in the frame its release edge
    fires, so without the lookup the whole drag idiom silently does nothing.

  ### ⚠ TWO CORRECTIONS TO MY BRIEF

  1. **`Touches` has no public writer** — private collections, no `press()`. The
     test sends `TouchInput` messages through Bevy's own `touch_screen_input_system`
     instead, which is **better** (the fixture cannot drift from the production
     path) but means *"just poke the resource"* is unavailable to anyone writing
     a touch test here. ⇒ worth knowing before the next one.
  2. **A bare tap on a portrait selects nothing, BY DESIGN** — the press arm is
     `(None, None, Some(Portrait(..))) => {}`. You pick your token up first
     (tap-token → tap-portrait, or one continuous drag). ⇒ *"tap the portrait"*
     was never one gesture, and touch now has both idioms the mouse and pad have
     — **no more**, which is the right scope.

  ### ▢ THE HONEST BOUND

  ⛔ **not run on Android hardware, and the commit message says so in those
  words.** Three headless synthetic-touch tests through Bevy's real fold system
  is the limit. **Genuinely unchecked on-device**: whether the touch overlay
  draws over the select route and intercepts the tap. ✔ nothing in the repo
  mutates `Touches` (no `clear_just_pressed` / `release_all` anywhere), so it is
  not *consumed* — but overlay geometry on a real phone is outside what any test
  here can see. ⇒ **ask Jon to tap a portrait once.**

  ---

  **The scoping that preceded the fix:**

  * **the chain, verified upstream**: smash's select screen is a **cursor**
    screen. It has no `bevy_ui` `Button`/`Interaction` nodes; it hit-tests its
    own rects against `Window::cursor_position` + `MouseButton`. Bevy fills that
    field **only** from `WindowEvent::CursorMoved`
    (`bevy_winit-0.18.1/src/state.rs:291`) — never from touch — and nothing in
    `ambition_touch_input` synthesizes it.
  * ⭐ **and it explains "SOME menus" properly, which my prompt story never
    did**: the launcher is built from `bevy_ui` buttons, and picking drives those
    via `PointerId::Touch`. **A `bevy_ui` menu works; a self-hit-testing cursor
    menu does not.** That is the actual dividing line, and it is a UI-construction
    fact rather than an input-context one.
  * ⇒ **the only way into the select screen on a phone today is stick-snap +
    confirm** — which now works, because D43's fork fix and the published cue
    landed. ⚠ so the two rows are not redundant: D43 made the fallback road
    usable, D58 is why the obvious road is not.
  * ⛔ **the bridge was deliberately NOT built.** Routing smash to
    `TouchControlPlacement` would add a game→touch-overlay dependency edge that
    the contracts job rejects. ⇒ **the seam is engine-side**: either touch
    synthesizes a pointer the way `bevy_ui` picking already consumes, or the
    select screen stops hit-testing by hand. **The second is smaller and is
    probably right** — a screen that reinvents hit-testing is the thing that fell
    off the platform's road.

  ### ✔✔ NEITHER — THE SEAM IS ALREADY IN SMASH, AND TOUCH IS ITS MISSING FOURTH DRIVER

  Scoped 2026-08-09, and this is much smaller than the paragraph above.
  `SelectCursor::move_to(Vec2)` (`select_screen/cursor.rs:174`) is a
  **source-agnostic** cursor, and `drive_the_cursor` already feeds it from
  **three** places:

  ```rust
  pointer.move_to(rect.center());          // :801  initial placement
  pointer.move_to(position);               // :813  Window::cursor_position  (MOUSE)
  pointer.move_to(target.rect.center());   // :854  stick-snap               (PAD)
  ```

  ⇒ ⭐ **touch is simply the fourth driver on a seam that already exists** — the
  classic odd-one-out shape, and joining the majority rather than inventing a
  mechanism. Three facts make it cheap:

  * ✔ **the coordinate space already matches.** `HitRect`'s doc: *"in LOGICAL
    window pixels with a top-left origin — the same space
    `Window::cursor_position` reports"*, which is exactly what `Touches` reports.
    **No conversion.**
  * ✔ **no new dependency edge.** `bevy` is a direct dep of
    `ambition_demo_smash`, so `bevy::input::touch::Touches` needs nothing from
    `ambition_touch_input` — which is what the contracts job would have rejected.
  * ✔ **precedent exists**: `ambition_touch_input/src/menu_bridge.rs:33` already
    takes `Res<Touches>` for the overlay's virtual controls.

  ⇒ **the change is roughly six lines**: `move_to` the primary touch position,
  and add `touches.any_just_pressed()` beside the
  `mouse.just_pressed(MouseButton::Left)` at `:863`.
  ⚠ **still unverified on hardware** — no device here. ⭐ but the desktop
  falsifier is real: synthesize a `Touches` press in a headless test and assert
  the cursor moved, which is the same shape as `standard_input_path.rs` driving
  `ButtonInput<KeyCode>`.
  * ✔✔ **CONFIRMED BY A SECOND ROUTE (me, independently of the worker).** The
    worker traced it downward from `bevy_winit`; I checked the smash side:
    ```
    select_screen.rs:811  windows.iter().next().and_then(Window::cursor_position)
    select_screen.rs:863  pressed  |= mouse.just_pressed(MouseButton::Left)
    select_screen.rs:864  released |= mouse.just_released(MouseButton::Left)
    grep Interaction in game/ambition_demo_smash/src/  →  no hits
    ```
    ⇒ the screen reads the cursor and the mouse button, and **owns no
    `Interaction` component anywhere**, so `bevy_ui` picking has nothing to drive.
    Two routes, one answer — which is the bar this row's predecessor failed.
  * ⚠ **falsifier if someone doubts it**: on a phone, the launcher's buttons
    respond and the smash portraits do not. Jon can settle it in one session
    without instrumenting anything.
  * ⭐ **strong prior art, and it is the same subsystem**:
    [[reference_two_tables_decide_a_touch_buttons_life]] — a touch button is
    DRAWN by one table and BOUND by another, and when they disagreed **Mary-O
    could not RUN on a phone**. A menu whose buttons draw but do not respond is
    exactly that signature.
  * ⚠ related but distinct from **D40** (the same subsystem naming a button
    wrongly). D40 is a label; this is a binding. Do not merge them.
  ### ⭐⭐ MECHANISM FOUND WITHOUT A DEVICE — 2026-08-08

  The touch stick is hidden — **and dead, not merely invisible** — when the
  prompt's context is `Empty`. `sync_touch_stick_visibility_from_context`
  (`touch_input/src/bevy_plugin.rs:1209`) says so in a comment that records this
  exact class of bug being fixed once already:

  > *"The stick STEERS A MENU too, and hiding it there cost the player their only
  > way to move a selection … a hidden node takes no drags, so it really was
  > dead, not merely invisible."*

  Menu and Dialogue were rescued then. **`Empty` was deliberately left hiding it**
  — *"a control nobody can use must not be on screen"*.

  ⭐ **`Empty` has exactly ONE producer** (`sim_view/src/control_prompt.rs:271`),
  and it is reached when `GameMode` **allows gameplay** but there is **no
  controlled body with authorities**: *"Cold start (no player yet) or a
  controlled body without authorities."* The Menu branch above it is gated on
  `!mode.get().allows_gameplay()`, so a screen that runs UNDER gameplay mode
  never reaches it.

  ⇒ **a smash select screen is precisely that state** — gameplay allowed, no
  fighter controlled yet — so it resolves to `Empty`, the stick is hidden, and
  the player cannot move the selection. ⭐ **that also explains Jon's "SOME
  menus"**: a menu running under a non-gameplay `GameMode` gets `Menu` and works;
  one running under gameplay-with-no-body gets `Empty` and is dead.

  ⭐ **the defect is a conflation, not a missing feature.** `Empty` means both
  *"nothing to control, correctly hide the stick"* and *"a screen the player must
  navigate that happens not to own a body"*. The rule is right; the state is two
  states.

  ### ✔✔ FALSIFIER RUN — IT CONFIRMS. Every link verified, still no device.

  ```
  GameMode::default() == Playing                 schedule.rs:576
  allows_gameplay()   == matches!(self, Playing) schedule.rs:612  -> true
  smash mentions GameMode:  NEVER (grepped select.rs + select_screen.rs)
    => the select screen stays Playing, so it can never reach the Menu branch
    => no fighter controlled yet => no authorities => ControlContextKind::Empty
    => stick hidden => "a hidden node takes no drags" => dead
  ```

  ⭐ **this is the first chain today where every link is verified rather than
  joined by an inference.** It cost four greps.

  **The fix, and it is engine-shaped.** ⛔ do NOT give smash a menu `GameMode` —
  that would make a select screen lie about being a menu to win a stick, and the
  next screen with the same need copies the lie. `ambition_input/src/cues.rs`
  already exists for exactly this: *"A surface that owns (or may own) an input
  context publishes a `UiCue`."* **Smash's select screen publishes nothing**, so
  the prompt cannot know a surface owns input. Either it publishes a cue, or
  `Empty` learns to distinguish *"nothing to control"* from *"a surface owns
  input without owning a body"* — the latter is the honest split, since the
  conflation is the actual defect.

  ### ⭐⭐ IT IS A FORK, AND THE FORK IS FOUR LINES APART — read 2026-08-09

  I checked whether publishing a cue from smash would be *enough on its own*.
  **It would not**, and the reason names the defect better than "the conflation"
  did. `ActiveUiCues` is consulted in exactly ONE place:

  ```rust
  // sim_view/src/control_prompt.rs:245  — inside `if !mode.get().allows_gameplay()`
  let confirm = cues.as_deref().and_then(ActiveUiCues::top)
      .map(|cue| cue.submit_label.clone())
      .unwrap_or_else(|| fallback.to_owned());
  set_prompt(&mut prompt, context, Vec::new(), Some(confirm));
  return;                                    // ← the cue-aware exit

  // :267 — twenty lines later, the cue-BLIND exit
  let Some((abilities, moveset, action_set, techniques)) = … else {
      set_prompt(&mut prompt, ControlContextKind::Empty, Vec::new(), None);
      return;
  };
  ```

  ⇒ **two exits answer the same question — "is the player navigating a surface
  rather than driving a body?" — and only one of them asks the resource that
  knows.** A cue published by smash today is dropped on the floor, because
  control flow never reaches the reader.
  [[reference_unifying_a_fork_exposes_what_it_hid]]: the two sides differ in
  STRICTNESS, and the stricter side (`Empty`) is the one that runs for smash.

  **The fix is therefore both halves, and this time both are real** (unlike D40,
  where I invented the engine half):
  1. **engine** — the no-subject exit consults `ActiveUiCues` first. A published
     cue means a surface owns input ⇒ resolve `Menu` + that cue's
     `submit_label`, the same shape the branch above already builds. `Empty`
     survives for its true case: no body AND no surface — a genuine cold start,
     where hiding the stick is still right.
  2. **smash** — the select screen declares a cue for its context. It publishes
     nothing today, which is why nothing can know it owns input.
  ⭐ **prefer extracting the shared resolution over copying it**, or the fork
  simply moves: `ambition_game_shell/src/basic_presentation.rs:139` is the
  working reference for a surface that declares one.

  ⚠ **the probe before the guard.** A test that drives the no-subject path with
  a cue present and asserts `ControlContextKind::Menu` must be watched FAIL
  first — today it fails on `Empty`, and that red is the whole proof the fork
  exists. `control_prompt.rs:561` already builds a cue-driven prompt fixture
  ("Equip"), so the harness is there.

  ⚠ superseded, kept for the reasoning: confirm smash's select screen
  actually has `allows_gameplay() == true` with no controlled body. If it instead
  runs under a menu `GameMode`, this whole chain is wrong and the cause is
  elsewhere. Headless and device-free — the same shape of check that killed my
  spike hypothesis in one grep.

  * ⛔ **the DEVICE half still needs Jon** — see
    `dev/journals/android-what-an-agent-cannot-see-2026-08-08.md`. The tractable
    half without hardware: check whether smash's menu contexts publish an input
    context that the touch layer claims, since a menu that never claims one
    receives nothing however correctly it is drawn.

- ✔ **D41 "SPIKES INSTA-KILL SANIC" — THE RING MECHANIC ALREADY EXISTS, SO THE
  BUG IS SOMEWHERE ELSE.** Jon's observation row (2) asks for *"a fairly faithful
  reimplementation of sonic physics and mechanics"*. ⛔ **do not reimplement it —
  most of it is already there and tested.** Diagnosed 2026-08-08 by grepping for
  the thing the row says is missing, per the charter's rule.

  * ✔ **the ring shield is IMPLEMENTED AND PINNED.** `demo_sanic/src/tests.rs:1401`:
    *"a hit taken with rings never reaches HP — that is what carrying rings
    buys"*, *"and it costs every ring"*, *"which scatter as real pickups, so they
    can be run back down"*. A second test,
    `scattered_rings_burst_outward_and_then_became_collectible`, pins Jon's
    EARLIER bug (*"the rings don't explode outward"*) as fixed — they launch
    with outward velocity and arc before rejoining the pickup economy. The
    no-currency lethal path is pinned in the shared resolver tests.
  * ✔ **the tile-vs-volume vulnerability fork was already UNIFIED on 2026-08-04.**
    `lib.rs:1437` records it: *"an authored hazard drawn as an ECS volume asked
    `body_vulnerable`, while the same hazard drawn as a TILE became an
    unconditional teleport-to-spawn nothing could see. `integrate_home_body` now
    applies the one predicate to both roads."*
  * ⭐⭐ **THE LEAD, and it means spikes may not be HITTING him at all.** The same
    doc block continues: *"⚠ the PIT still swallows him, and that is the line.
    **A hazard tile is never a collision surface**, so a super Sanic falls
    straight through the strip at the bottom and leaves the world — and
    `ResetCause::LeftTheWorld` exempts nobody. Falling out is not something that
    HIT you."* ⇒ if the speedway's spike strip is a non-colliding hazard TILE,
    the death is `LeftTheWorld`, **not damage** — and adding ring-loss to the
    damage path would fix nothing because that path never fires.

  ### ⛔ WHAT IS NOT VERIFIED, AND MUST BE OBSERVED FIRST

  I have **not** watched Sanic die on the strip. Three facts above are read from
  source; the conclusion joining them is not. Specifically unknown: the room
  authors **two** hazards (`tests.rs:159` — *"the pit floor and the mid-course
  spike strip"*), and whether the MID-COURSE strip sits over solid ground or over
  the void decides everything. Over ground, falling through is harmless and the
  real cause is something else entirely.

  ### ✔ FALSIFIER RUN — MY LEAD WAS WRONG, AND THE REAL CAUSE IS SIMPLER

  Ran the row's own first action before acting on the lead. It died immediately.
  `game/ambition_demo_sanic/tools/author_speedway_ldtk.py:209` authors both
  hazards explicitly:

  ```python
  rect("HazardBlock", (PIT_LEFT, 704), (PIT_RIGHT - PIT_LEFT, 16), name="pit_hazard"),
  rect("HazardBlock", (5648,     656), (96, 16),                   name="mid_spikes"),
  ```

  `FLOOR_TOP` is **672.0**, and `mid_spikes` spans y **656 → 672** — its bottom
  edge IS the floor's top edge. ⛔ **the strip sits ON THE GROUND, so Sanic
  cannot fall through it into the void and `LeftTheWorld` is not the cause.**
  Only `pit_hazard` (y=704, under the pit) is below the floor.

  ⭐⭐ **the actual defect: `HazardBlock` has exactly ONE behaviour.** The
  generator says so in its own words at line 44 — *"The pit (bottomless in
  spirit; **a hazard strip resets to spawn**)"*. There is no hazard kind meaning
  *"hurt me"*. A pit floor and a row of spikes are authored with the same noun
  and therefore get the same outcome, and for a pit that outcome is right.

  ⇒ **the fix is a hazard that DAMAGES rather than resets** — either a second
  `HazardBlock` kind, or authoring `mid_spikes` as a damage volume so it reaches
  the resolver that already spends rings. ⭐ **the ring shield needs no work
  either way; it is waiting for a damage event that never arrives.** ⚠ the pit
  must keep resetting: *"falling out is not something that HIT you."*

  ### ✔ SHIPPED 2026-08-08 — and one sentence above is WRONG

  ⛔ **"There is no hazard kind meaning *hurt me*" is false.** `DamageVolume` is
  exactly that, it is defined in every world file's shared defs, and moving the
  strip onto it produced the whole wanted behaviour on the first headless run —
  no engine mechanism was missing. What WAS missing is that four separate places
  told an author `HazardBlock` was the static damage surface, and it damages
  nothing:
  * `area_authoring.py` — *"Use HazardBlock for static damage surfaces and
    DamageVolume only for moving / variable-damage hazards."* **That sentence is
    why the speedway's spikes reset.** Nothing ever required a damage volume to
    move.
  * `intgrid.rs` — value 5 documented as *"Hazard tile: damages the player on
    contact"*. It teleports; no health, currency, or i-frame is consulted.
  * `surfaces.rs` — `HazardBlock` parsed to `SurfaceContact::Damage { amount }`
    from a `damage` field the entity does not have, and the compile step threw
    the amount away.
  * `editor_art.py` — `HazardBlock` was DRAWN as `hazard_spikes` in the editor.
    The runtime draws the flat `hazard_tile` for the block it becomes.

  ⇒ **all four now say reset**, `SurfaceContact::Damage` is renamed
  `ResetToSpawn` (the variant no longer claims a thing it cannot do), and the
  kernel's `BlockKind::Hazard` doc says outright that a hazard which HURTS is a
  damage volume. `HazardBlock` stays the pit noun — every authored use in the
  tree is a death gap (`gap`, `the_gap`, `death_floor`, `pit_floor`).

  **The demo side**: `mid_spikes` is a `DamageVolume`; Sanic and Super Sanic
  author `max_health: Some(1)` — the engine's own documented classic-platformer
  contract — because on the host's 20-point pool a ringless spike hit cost 1 HP
  and he walked on with 19. Four contract cases run headlessly in
  `ambition_demo_sanic_app::spikes_spend_rings` (rings spent + scattered · fatal
  at zero rings · super untouched · pit still resets at any ring count), and the
  poison runs are recorded: re-author the strip as `HazardBlock` and case 1 fails
  with `rings: 12, scattered: 0, sent_home: true`, which is Jon's bug verbatim.

  ⚠⚠ **two loaded footguns found on the way, both fixed in the generator.**
  `author_speedway_ldtk.py` claimed to be the world file's only author and was
  not: (a) it `unlink()`ed a **tracked symlink into the map submodule** and wrote
  a 296 KB regular file in its place; (b) re-running it would have DELETED the
  ring `sprite` bindings and the badnik `character_id`s (added out-of-band) and
  turned the monitors back into `Solid` — the walls that stopped Sanic dead at
  x=1474 in July. ⭐ **the lesson: a generator that has not been re-run since the
  file was last hand-edited is a regression waiting for the next author.** The
  only way to know is to run it and diff SEMANTICALLY (entity instances + IntGrid
  counts), because the raw JSON diff is thousands of lines of uid churn.

  ⚠⚠ **the process note, which is the reusable part.** The lead below was my
  fourth plausible-but-wrong causal chain of the day — but the first one I wrote
  a named falsifier for BEFORE acting, and the falsifier cost one grep and killed
  it instantly. **Keep doing that**; the cost of the discipline was minutes and
  the cost of skipping it, three times earlier today, was rework and a wrong
  claim to Jon.

  ⇒ ~~**first action is to observe the actual `ResetCause`** on a headless run that
  walks Sanic into the mid-course strip. `LeftTheWorld` and a damage cause imply
  completely different fixes. ⚠ this row was written by an agent who produced
  three plausible-but-wrong causal chains earlier the same day; treat the lead as
  a hypothesis with a named falsifier, which is why the falsifier is the first
  step rather than the last.

- ✔ **D40 LANDED `7860e5c02` (merged 2026-08-09).** The button says "Transform".
  ⛔⛔ **AND THE ROW WAS WRONG THREE TIMES — including the CORRECTION I wrote to
  fix it being wrong.** That is the durable lesson here, not the label.

  * **the probe, failing, before the fix**:
    ```
    the_utility_control_is_named_by_the_worn_persona_not_by_a_generic_fly_verb
      left:  Some("Fly Toggle")
      right: Some("Transform")
    ```
  * ⛔⛔ **`label_for(Utility)` never returned `None`.** Both the original row AND
    my "correction" say *"'Fly' is reached only because
    `prompt.label_for(Utility)` returns `None`, which is the documented fallback
    working correctly."* **It returned `Some("Fly Toggle")`.** Sanic's body has
    `fly && fly_toggle`, so the engine's own `movement_actions` claimed Utility
    with the `fly_toggle` id and `ActionSpec::display()` title-cased it. The
    button was showing a **derived engine label**, not a spawn constant, and the
    whole `ButtonVerb` fallback story is irrelevant to this bug.
    ⭐ **the tell I had and ignored**: under my model the button would have been
    HIDDEN, because availability is `label_for(Utility).is_some()`. Jon could see
    it. **His screenshot refuted my mechanism and I did not notice.**
  * ✔ **routing Utility was NINE LINES, and my "no device verb" premise was also
    false** — `ActorControlFrame::fly_toggle_pressed` is that verb and always
    was. Shaped exactly like the `Modifier` arm. `unroutable` confirmed to have
    exactly one consumer, by grepping the capability (`resolve_control_slots`
    callers) and not just the type name.
  * ⭐ **and a latent fork fell out**: the demo had **two systems inserting
    `ActorTechniques`**, which would have silently dropped whichever landed
    first. Now one.
  * **4 stale comments swept**, all describing this subsystem as something it
    outgrew — including the `ButtonVerb::fallback` line that misled me. The
    two-table hazard was checked: `layout.rs:169`, `bevy_plugin.rs:1009` and
    `virtual_device.rs:300` **agree**, no drift.

  ⭐⭐ **THE LESSON, and it is about me rather than the code.** I wrote a
  confident mechanism, was corrected by the tree, wrote a *second* confident
  mechanism in the same row, and was corrected again by a worker's probe. Both
  times I reasoned from code I had read *near* the answer instead of running the
  thing. ⇒ **a row that has already been wrong once earns a probe before its
  second theory, not a better paragraph.**
  [[feedback_ask_the_tool_dont_model_it]].

- **D40-ORIGINAL (superseded; marker stripped, text kept) SANIC'S TRANSFORM
  BUTTON READS "FLY" BECAUSE A TOUCH LABEL IS AN
  ENGINE CONSTANT, NOT THE GAME'S VERB.** Jon's observation row (4), diagnosed
  2026-08-08. Small, and the fix is engine-shaped rather than a string edit.
  * **the label**: `ambition_touch_input/src/layout.rs:169` —
    ```rust
    scaled(TouchActionButton::FlyToggle, "Fly", 124.0, 0.0, 62.0, 14.0),
    ```
    a hardcoded string keyed to the BUTTON KIND.
  * **the meaning**: `bevy_plugin.rs:1009` maps
    `TouchActionButton::FlyToggle => ControlSlot::Utility`. So the button is the
    **utility slot** wearing the flagship game's verb.
  * **Sanic's side**: it binds its transformation to Utility —
    *"the transformation consumes Utility before generic flight can see it"*
    (`game/ambition_demo_sanic/src/tests.rs:632`), and `lib.rs:1407` *"Utility
    belongs to this mode-local transformation."* ⇒ Sanic is correct; the button
    is telling the player about a different game.
  * ⭐ **the engine already KNOWS these are contextual** and says so at
    `bevy_plugin.rs:976` — *"contextual meaning at all (FlyToggle / Start /
    Reset)"* — but the LABEL does not follow the binding. ⇒ the fix is to let a
    game name its own utility verb, not to change `"Fly"` to `"Transform"` and
    hand the next game the same bug.
  ### ⭐⭐ THE FIX IS TO CONNECT TWO THINGS THAT ALREADY EXIST — designed 2026-08-08

  ⛔ **do not add a label-override table.** A naming authority is already here,
  and it is already per-slot and already player-facing:

  ```rust
  // ambition_entity_catalog/src/action_scheme.rs:146
  pub struct ActionSpec {
      pub id: ActionId,
      pub slot: ControlSlot,
      /// Player-facing label. `None` falls back to a title-cased id
      pub display_name: Option<String>,
  }
  // ActionSpec::display() — "spin_dash" -> "Spin Dash"
  ```

  ⭐ **and `ambition_input/src/cues.rs` predicted this consumer in its own module
  doc**: cues are keyed by context *"so gameplay's `ActionSchemeContract` labels
  and any future surface join the same vocabulary rather than a parallel prompt
  system."* A touch label read from `ActionSpec` IS that vocabulary; a new table
  would be the parallel system it warns against.

  ### ⛔⛔ CORRECTION 2026-08-09 — "HALF 1" IS ALREADY BUILT. I DESIGNED A
  ### MECHANISM THAT SHIPPED.

  I wrote the two halves above from the diagnosis without grepping for the
  thing I claimed was missing. It is there, and it is there in the exact shape
  I proposed:

  ```rust
  // touch_input/src/bevy_plugin.rs:973 — ALREADY the fallback/current split
  pub struct ButtonVerb { fallback: &'static str, current: Option<String> }

  // :1051 update_button_verb_from_prompt — ALREADY resolves slot -> label
  ControlContextKind::Gameplay => touch_button_slot(*action)
      .and_then(|slot| prompt.label_for(slot))
  ```

  ⇒ **the engine already renames every touch button from the controlled
  subject's own action scheme**, and `touch_button_slot` already maps
  `FlyToggle → ControlSlot::Utility`. "Fly" is reached only because
  `prompt.label_for(Utility)` returns `None`, which is the documented fallback
  working correctly. ⭐ **the whole fix is content-side.**

  ⚠ **and a comment lied to me on the way in.** `ButtonVerb::fallback`'s doc
  says it is *"the permanent text of the buttons that carry no contextual
  meaning at all (FlyToggle / Start / Reset)"* — but `touch_button_slot` returns
  `None` for Start/Reset and `Some(Utility)` for FlyToggle. FlyToggle does not
  belong in that list; it is contextual and always was.
  [[reference_a_comment_describes_intent_not_the_code]] again — **the comment
  states an intent the code outgrew**, and grouping FlyToggle with the two
  genuinely-static buttons is what made "the label is a constant" believable.
  Fix the comment in the same change.

  **The actual work, and it is one-sided:**
  1. **Sanic** declares an `ActionSpec` on `ControlSlot::Utility` for the
     transformation. The seam exists: `derive_action_scheme(abilities, moveset,
     action_set, techniques)` layers **content-declared `techniques` LAST and
     they override any base action on the same slot** — carried per-body by
     `ActorTechniques`. An id of `transform` title-cases to "Transform", so
     `display_name` is optional.
  2. ⚠ **one engine gap is real, and it is NOT the label.**
     `resolve_control_slots` (`characters/src/action_scheme.rs:255`) lists
     `Utility` among the slots with *"NO device verb in this frame. A technique
     placed there has no wired path yet → reject, never drop"*, so a
     `Technique`-gated spec on Utility comes back in `unroutable`. Sanic does not
     need the routing (`lib.rs:1407` consumes the raw Utility edge itself), but
     declaring a gate nothing routes is a lie. **Route Utility techniques** and
     the declaration becomes true —
     [[reference_an_authority_that_needs_a_follow_up_call]].
     ⭐ the blast radius is one assertion: `unroutable` is consumed in exactly
     ONE place workspace-wide, the test at `starting_character.rs:980`.

  ⭐ **the lesson, which is bigger than the row**: the charter says *grep for the
  thing an open row says is missing* and I skipped it because the row was mine
  and one day old. **A stale row is not only stale about landed WORK — it goes
  stale about the MECHANISM too**, and a design paragraph written from a
  diagnosis reads exactly as authoritative as one written from the code.

  * ⚠ **this is the two-table hazard again** —
    [[reference_two_tables_decide_a_touch_buttons_life]]: a button is DRAWN by
    one table and BOUND by another, and they disagreed once before badly enough
    that Mary-O could not run on a phone. Here the label lives in the draw table
    while the meaning lives in the bind table. **Check both when fixing.**

- ✔ **D57's SPIKE TESTS ARE FIXED 2026-08-09 — 4/4 in BOTH builds.** They now
  press `ArrowRight` through the real keyboard seam where a bridge exists (the
  idiom `standard_input_path.rs` already uses in the same directory), and keep
  the direct `ControlFrame` write where none does. ⭐ **the previous 0/4 under
  `--features input` is what proves the new arm exercises the real path** — the
  fix is verified by the failure it removes, not by its own green.
  ⚠ **the two arms are two COMPOSITIONS, not a fork**: with the feature there is
  a device→`ControlFrame` bridge and the honest way in is the keyboard; without
  it no bridge exists and `ControlFrame` is the only seam there is. Gating the
  file on `input` instead would have cost its default-build coverage of the
  spike/ring mechanics for nothing.
  ▢ **what remains of D57 is the SWEEP**, not the spikes — see the 23-crate
  feature-gate table below.

- **D57-ORIGINAL (superseded; marker stripped, text kept) ⛔⛔ THE SPIKE TESTS
  ARE GREEN ONLY IN THE CONFIGURATION WHERE INPUT
  DOES NOT EXIST.** Found by the D40 worker 2026-08-09, pre-existing, **not
  caused by any work today** — verified identical on base `3b1b2b065`.

  ```text
  cargo test -p ambition_demo_sanic_app --test spikes_spend_rings                     4 passed
  cargo test -p ambition_demo_sanic_app --test spikes_spend_rings --features input    0 passed, 4 FAILED
  ```

  * **the mechanism**: the tests write `ControlFrame` directly. With the `input`
    feature on, the real input bridge **overwrites it**, so the body never moves
    — `max_right` comes out at exactly `RUN_UP_X + half_width`, i.e. it never
    left the spot.
  * ⛔ **so D41's spike fix — Jon's observation (2) — is defended by four tests
    that only pass with the input system absent.** They are not wrong about the
    ring mechanic; they are silent about whether a *player* can reach a spike.
    [[reference_a_check_that_cannot_fail]]: the guarded property holds in a
    configuration the game never ships.
  * ⚠ **invisible to the prescribed command.** `cargo test -p
    ambition_demo_sanic -p ambition_demo_sanic_app` does not enable `input`, so
    every gate I have run today reports these green. ⭐ **that is the transferable
    part** — a feature-gated test population means "the suite is green" is a
    claim about a FEATURE SET, and neither the goal harness nor any brief I wrote
    said which one.
  * ⇒ **the fix is to drive the sanctioned seam, not to widen the gate**: a test
    that writes `ControlFrame` under the real bridge is asserting against a
    value the engine owns. Feed input where the player does.
  * ⚠ **related, same shape, already known and still true**: `-p ambition_input`
    runs **55 of 84**, and `-p ambition_touch_input` runs **4 of 45** because
    `bevy_plugin` is `mobile_touch`-gated. **Three instances now.**

  ### ✔ THE SWEEP — 23 crates gate test code behind a feature (2026-08-09)

  **Copy this into a brief instead of guessing.** A bare `cargo test -p <crate>`
  in any of these runs a SUBSET, silently:

  | crate | features that gate its tests |
  |---|---|
  | `ambition_app` | `audio, bevy_ui_menu, dev_tools, frame_pacing, input, kaleidoscope_menu, portal, rl_sim` |
  | `ambition_touch_input` | `mobile_touch` ⚠ 4 of 45 without it |
  | `ambition_input` | `input` ⚠ 55 of 84 without it |
  | `ambition_platformer2d_actor_monolith` | `audio, causal, input` |
  | `ambition_content` | `audio, falling_sand, portal, portal_render, ui` |
  | `ambition_characters` | `causal, content_pack` |
  | `ambition_combat` | `causal, content_pack` |
  | `ambition_platformer2d` | `ambition_render, causal, content_pack` |
  | `ambition_platformer2d_runtime` | `causal, portal` |
  | `ambition_platformer2d_host` | `input, portal_render` |
  | `ambition_render` | `capture, portal_render` |
  | `ambition_dialog` | `input, ui` |
  | `ambition_audio` | `content_pack, kira` |
  | `ambition_demo_sanic_app`, `ambition_demo_twintrack` | `visible` |
  | `ambition_game_shell`, `ambition_load_presentation` | `basic_presentation` |
  | `ambition_encounter`, `ambition_items` | `content_pack` |
  | `ambition_asset_manager`, `ambition_causal`, `ambition_sfx` | `bevy` |
  | `ambition_portal2d_presentation` | `effect_view_cones` |

  ### ✔ THE TABLE IS REGENERABLE NOW, AND IT HAS THE MISSING NUMBER (2026-08-10)

  `python3 scripts/feature_gated_tests.py [--markdown]` rebuilds the table above
  and adds the figure the hand version never had: **how many tests** are behind
  each gate, not merely which features they are. ⭐ **24 crates hide 619 tests.**
  `--verify <crate>` asks cargo for the exact pair when a number has to be
  quoted.

  ⭐ **it is calibrated against cargo, and the calibration found a bug in it.**
  Scan vs `cargo test -p <crate> -- --list`: `ambition_touch_input` **4 of 45**
  both ways, `ambition_causal` **21 of 22** both ways, `ambition_input` 54 of 115
  against cargo's 55 of 117. ⛔ the first draft said `touch_input` ran **10 of
  45** — over-stating bare coverage by six, the UNSAFE direction — because it
  did not follow `#[cfg(feature)] mod x;` into its file, and because the brace
  tracker never counted the brace `mod x {` swallows, so a gate closed on the
  first `}`. An estimate nobody checked against the real thing is the same
  species of claim this row exists to correct.

  ⚠ **still an estimate, and still under-counting in one direction** — it cannot
  see `required-features` on a `[[test]]` target, and two `ambition_input` tests
  are invisible to its regex. ⇒ **treat a crate's absence from the table as "not
  proven complete", not as "runs everything."**
  ⭐ **name both denominators** — the whole point of the table is that
  *"the suite is green"* was never a complete sentence.
  [[reference_measure_the_suspect_not_the_aggregate]].

- ✔ **D56 FIXED 2026-08-10 — the art identity names the art.**
  `ActorRenderView` carries `sprite_character_id`, and `upgrade_actor_sprites`
  resolves **override label → ART IDENTITY → display name**. The display-name arm
  stays LAST rather than being deleted: every authored spawn in the game relies
  on it, so removing it would un-art the whole cast to fix a case with no
  occurrences.

  ⭐ **nothing in the game changes today, and that is the point.** 0 of 65
  `EnemySpawn`s set `character_id`, so this lands with zero visible effect —
  which is exactly what makes it safe alone and what makes **D48 a pure content
  edit** instead of a change that un-arts every level it touches. The two no
  longer have to land together; this half is done.

  Two guards, and the second is the one that keeps it from being a rename: a body
  whose id and label differ binds its ID's sheet, and a body with NO id still
  binds by name. Falsified — deleting the identity arm reds the first and leaves
  the second green.

  *The row as filed:*

- **D56-ORIGINAL (superseded; marker stripped, text kept) ⭐⭐ THE RENDERER BINDS
  A SHEET BY DISPLAY NAME WHILE EVERYTHING ELSE USES THE CHARACTER ID.** Found by the D39 worker while fixing the goblins,
  2026-08-09. ⛔ **this is the mechanism behind the magenta boxes, and D39 only
  fixed the instance of it that Jon reported.**

  * **the asymmetry**: a spawn resolves `sprite_character_id` and that identity
    reaches the barks, the hurt feedback, the sprite-derived collision box and
    the authored attack volumes. But `upgrade_actor_sprites` binds the SHEET from
    `ActorRenderIndex`, which `rebuild_actor_render_index` fills from
    **`ActorConfig::name`** — the display name. ⇒ **the one thing bound off
    presentation is the art.**
  * ⛔ **so `EnemySpawnSpec::character_id` cannot do the job it exists for.**
    That field was added 2026-08-06 precisely so a level's LABEL and its ART
    IDENTITY could differ — its converter comment says renaming a character
    *"silently un-arted every level that placed it"*. **Any authored `EnemySpawn`
    whose `character_id` differs from its display name is un-arted today**, by
    this path, and the field's whole purpose is to allow exactly that difference.
  * ⭐⭐ **and D48 measured why nobody has hit it: 0 of 65 `EnemySpawn` instances
    set `character_id`.** The feature has no adopters, so its brokenness has no
    witnesses. ⇒ **fixing D48 by authoring character ids WOULD EXPOSE THIS** —
    the two rows must land together or the first one makes things worse.
    [[reference_count_the_adopters_not_the_capability]]: the adopter count was
    hiding a second defect, not just an unused feature.
  * **the fix**: `rebuild_actor_render_index` keys on the resolved character
    identity, with the display name as the fallback it already is elsewhere
    (`presentation_identity()` — renamed from `art_identity()` on 2026-08-10 —
    is the existing helper and already encodes *"authored id, else the name"*;
    ⛔ it answers ART ONLY, and `gameplay_character_id()` is the one to ask for
    identity). ⇒ one reader, one rule, and the
    display-name road stays intact for the 65 spawns that rely on it.

  ### ✔✔ THE FIELD IS ALREADY ON THE STRUCT — and its doc explains the deadlock

  `ActorConfig` (`features/ecs/actor_clusters.rs:75`) — **the very struct
  `rebuild_actor_render_index` already reads `.name` from** — carries:

  ```rust
  /// Uniform gameplay-side sprite identity: the catalog `character_id` this
  /// actor's sprite resolves to (VIA ITS DISPLAY NAME, mirroring the
  /// presentation `npc_asset_for_name` join). `Some` for catalog characters …
  pub sprite_character_id: Option<String>,
  ```

  ⇒ **the fix is one expression**, on a field already in hand:
  `a.config.sprite_character_id.as_deref().unwrap_or(&a.config.name)`.

  ⭐⭐ **and the parenthesis in that doc is the whole D48/D56 deadlock, stated by
  the code itself**: `sprite_character_id` is resolved *via the display name*, so
  **today the two hold the same value** and swapping the renderer to it changes
  nothing observable. The value only diverges once a spawn authors a
  `character_id` — which is D48.

  ⇒ **the deadlock is exact and symmetric**:
  * **D56 alone is invisible** — nothing yet makes the two differ, so the fix
    looks like a no-op and would very reasonably be reverted as churn;
  * **D48 alone is a REGRESSION** — authoring `character_id` makes them differ
    and the renderer keeps using the name, un-arting the very spawns it was meant
    to fix.
  ⇒ ⛔ **neither is landable alone, and each looks pointless or broken without the
  other.** ⭐ **that is the argument for the pairing, and it is much stronger than
  the sequencing note it replaces** — a reviewer seeing only D56 would be right to
  reject it.
  * ✔ **VERIFIED MYSELF 2026-08-09** rather than on the worker's word.
    `rebuild_actor_render_index` (`sim_view/src/view_index.rs:589`) is fed
    `&a.config.name` — the display name — plus one override, and nothing else.
  * ⭐ **and the override is a FOURTH zero-general-adopter capability.**
    `sprite_override_npc_name` exists on `ActorConfig` and is read right there,
    so a body CAN wear art its label does not name. But its only production
    driver is `features/ecs/autonomous_reconcile.rs`, gated on
    ```rust
    if current_config.name != "Kernel Guide NPC"
    ```
    — a **hardcoded display-name comparison** serving the possession case. ⇒ it
    is a possession mechanism, not a general art-identity seam, and D56's finding
    stands. [[reference_count_the_adopters_not_the_capability]], instance four.
    ⚠ **and it is keyed on the very string D56 says should stop being the key**,
    so it inherits the bug it looks like an escape from.
  * ⚠ **FALSIFIER still worth running before the fix**: author `character_id` on
    ONE `EnemySpawn` whose display name differs from its catalog id, and confirm
    the body draws unclaimed. ⭐ cheap, and it also builds D48's first adopter.

- ✔✔ **D55 FIXED AND VERIFIED BY EYE — renderer `c8af045`, pointer `c5f1c8195`
  (2026-08-09).** Jon's *"the bubble in the wrong place, just kinda to the upper
  left"*, closed. **One line.**

  ```python
  -   box = (64 - 40 * pulse, 63 - 48 * pulse, 64 + 40 * pulse, 63 + 48 * pulse)
  +   cx, cy = world["torso"].origin
  +   box = (cx - 40 * pulse, cy - 48 * pulse, cx + 40 * pulse, cy + 48 * pulse)
  ```

  * ✔✔ **THE PREDICTION HELD, and it is the independent confirmation.** This row
    predicted the frame-rect anomaly would collapse. Measured after regeneration:
    ```text
    block   126 x 145  ->  81 x 120
    vs idle 1.77x wide / 1.45x tall  ->  1.14x / 1.19x
    ```
    ⇒ **the very anomaly that identified this bug** — `block` being the only row
    of 37 larger than its idle in **both** axes — **is gone.** Nothing was tuned
    to make that happen; it fell out of centring the ellipse.
  * ⭐⭐ **six engine mechanisms died before this, and every one was consistent
    with the numbers.** The alpha bbox, the feet anchor, a one-offset visual,
    sim-vs-presented pose, a second shielding body, a body-size bug. The engine's
    `ShieldRingsView` measures correct to the last decimal — `pos == kin.pos`,
    drawn-centre offset `(+0.00, +0.00)`. ⇒ **there was never a wrong expression
    to find, which is why reading code could not end it.**
  * ⭐ **what ended it was looking at the picture** — extracting `block` frame 0
    by its own sidecar rect and viewing it. ⇒ **verify art by eye, always**: this
    row is the case where every number was self-consistent and the art was wrong
    the whole time. [[reference_text_in_a_capture_may_be_pixels]] is the same
    lesson in the other direction.
  * ✔ **the installed sheet is byte-identical to the regenerated one**, and it is
    **gitignored and rebuilt per clone**, so the generator change is the durable
    fix and nothing binary was committed. ⚠ regenerate with the renderer's OWN
    venv — `tools/ambition_sprite2d_renderer/.venv/bin/python main.py sheet
    player_robot_v3`; the system interpreter lacks `resvg_py` and dies in
    `rasterize_subset`.
  * ▢ ⚠ **ONE THING LEFT FOR JON, and it is taste not a defect**: the bubble now
    surrounds the **torso** and the head pokes above it. Whether a shield should
    cover the whole robot is **a radius** (`40 x 48` today). ⛔ **do not "fix"
    that without asking** — it is a look, and the engine's own
    `BubbleShieldVisual` covers the whole body at `size * (1.55, 1.25)`, so the
    two now disagree about how much a shield covers.

  ---

  **The investigation, kept for the six dead mechanisms:**

- **D55-ORIGINAL (superseded by the ✔✔ block above — marker stripped, text kept)
  ⭐⭐ SOLVED BY MEASUREMENT: THE MISPLACED BUBBLE IS PAINTED INTO THE
  SPRITESHEET.** Jon's *"The main character shield sprite has the bubble in the
  wrong place, just kinda to the upper left."*
  ⇒ **the engine is innocent on every count, and the fix belongs to the art
  generator.** Everything below the `### ✔✔ THE PROBE` heading is the answer; the
  investigation above it is kept because **six mechanisms died here** and the
  record of how is worth more than the row.

  ### ✔✔ THE PROBE ANSWERED — outcome 3, and it inverts the whole reading

  `game/ambition_app/tests/shield_ring_probe.rs` (`e7db486b6`, `#[ignore]`d,
  print-only, asserts nothing). Driven through the **real input path** —
  `ButtonInput<KeyCode>.press(KeyCode::KeyE)`, exactly what `capture_scene`'s
  `hold:e` does — never by setting the shield state directly.

  **`ShieldRingsView` is clean to the last decimal:**

  ```text
  ShieldRingsView.0.len() = 1
    [0] pos=(110.000, 1528.000) size=(30.066, 48.000)   ==  kin.pos, kin.size
  BubbleShieldVisual: 1 entity, anchor (0,0), custom_size (46.60, 60.00)
                      == size * (1.55, 1.25) exactly
  Transform.translation == world_to_bevy(kin.pos) exactly
  other sprites drawing the SAME shield texture: 0
  drawn-centre offset from the player: (+0.00, +0.00)
  ```

  ⇒ one row, one entity, centred, no duplicate, **zero offset**. Shield down →
  `len()==0` and the pooled ring goes `Hidden`. Every one of my five earlier
  mechanisms is refuted by this block alone.

  ### ⭐ THE INVERSION: the two artefacts are the opposite of what I assumed

  The probe's marker-free proximity query — *"list every sprite within 150 units,
  whatever it is"*, which I had not thought to ask for — caught **the player's own
  sprite changing size the instant the guard goes up**:

  ```text
  idle   37.45 x 53.80
  block  63.30 x 75.96      <- 1.77x wider, 1.45x taller
  ```

  That is the `block` animation row of
  `assets/sprites/player_robot_v3_spritesheet.png`, and **that row paints a thin
  cyan bubble into the art, beside the robot rather than around it** (robot
  bottom-right, detached circle top-left). The sidecar says so without a picture:
  every other row is `~71 x 101` at `off (79, 57)`; `block` is `119..126 x
  144..149` at `off (22..25, 12..17)`. `120 x 144` at the sheet's own `0.5275`
  world-per-pixel is `63.30 x 75.96` — **exact**.

  ⇒ ⛔ **I had the two backwards for the whole investigation.** The soft glow
  centred on the robot IS `BubbleShieldVisual` (`Srgba(0.5,0.8,1.0,0.55)`, drawn
  behind the body); the thin up-left ring Jon is complaining about is **the
  spritesheet**. Re-shot with and without `--combat-overlay`: the thin ring
  survives without it, so no gizmo is involved either.

  ### ⭐ AND THE POPULATION CHECK NAMES v3 AS THE OUTLIER

  Of the **37 sheets carrying both `idle` and `block`**, `player_robot_v3` is the
  only one **1.77× wider AND 1.45× taller** than its idle. Alice and Bob are 1.40×
  wider at the *same* height — an arm extending — and both draw their arc **in
  front of the body**. `player_robot_v2` draws a rectangular panel. **v3 is alone
  in painting a detached ring.**

  ⇒ ⭐⭐ this is [`the-odd-one-out-among-siblings`](../../dev/benchmark-candidates/the-odd-one-out-among-siblings-2026-08-09.md)
  again — the **eighth** instance, and the first found by a *population* check
  rather than a call-site diff. 36 sheets agree; the defect is the one that does
  not. **The method generalises past code.**

  ### ✔✔ SEEN, NOT INFERRED — 2026-08-09, and the picture changes the fix

  Extracted `block` frame 0 from the sheet by its own sidecar rect
  (`x=1 y=152 w=126 h=145`) and looked at it. **It is exactly Jon's report**: the
  robot sits bottom-right, and a large pale-cyan ellipse occupies the upper-left
  **60% of the frame**, detached from the body.

  ⭐ **but it is DELIBERATE ART, not a stray artifact — and neither the probe nor
  I knew that.** There is a **magenta emitter** drawn between the robot and the
  bubble: a small round head on a stalk, held out from the chest. ⇒ somebody drew
  *"the robot projects a shield"*, and the projection is simply **placed wrong
  relative to the body**.

  ⛔ **so "just delete the painted ring" — what this row recommended an hour ago —
  is now the WEAKER option**, because it would leave the emitter holding nothing.
  ⇒ two honest choices, and this is a **taste call**:
  * **(a) reposition** the bubble so it centres on the robot, keeping the emitter.
    The art then says what it means, and `BubbleShieldVisual` becomes redundant
    for this character.
  * **(b) delete bubble AND emitter**, and let `BubbleShieldVisual` be the only
    thing that says *"there is a shield here"* — one authority, and it already
    draws correctly at `size * (1.55, 1.25)`.

  ⚠ **and the SVG rig cannot be the fix site**: `player-robot-v3.svg` has **20
  `part-*` ids and none of them is a shield** — no bubble, no ring, no aura. ⇒
  the bubble is **not** a rig part to hide.

  ### ✔✔✔ FOUND, AND IT IS ONE HARDCODED PAIR OF COORDINATES

  `targets/characters/player_robot_v3.py:347`:

  ```python
  if animation == "block":
      pulse = 1.0 + 0.05 * math.sin(t * math.tau)
      box = (64 - 40 * pulse, 63 - 48 * pulse, 64 + 40 * pulse, 63 + 48 * pulse)
      fd.ellipse(box, fill=(65, 222, 255, 24), outline=(63, 229, 255, 190), width=2)
  ```

  ⇒ **the bubble is pinned at `(64, 63)` in canvas space.** And the sheet's own
  sidecar says where the body actually is, in that same space:

  ```text
  block frame 0   chest anchor = (112.290, 128.977)     bubble centre = (64, 63)
                  offset       = (−48.3, −66.0)          ⇒ UP AND LEFT
  ```

  ⭐⭐ **that is Jon's sentence, quantified to the pixel**: *"the bubble in the
  wrong place, just kinda to the upper left."* Nothing about the engine, the
  anchor, the bbox or the quad was ever involved.

  * **the fix, one line**, and the value is already in scope — `:429` uses
    `world["torso"].origin` to emit that very anchor:
    ```python
    cx, cy = world["torso"].origin
    box = (cx - 40 * pulse, cy - 48 * pulse, cx + 40 * pulse, cy + 48 * pulse)
    ```
  * ⭐ **thirteenth [odd-one-out](../../dev/benchmark-candidates/the-odd-one-out-among-siblings-2026-08-09.md),
    and this time inside ONE function**: every **body-attached** effect here
    positions from an anchor — `hand = world["near_arm_hand"]` (`:352`), the
    blade from `hand.tip`, the aim/charge glow from `base`. The **ambient** ones
    (`swim` bubbles, `hit` sparks) hardcode, correctly, because they are not
    attached to anything. **The shield is body-attached and hardcodes.**
  * ⭐ **it should also collapse the frame-rect outlier**. A chest-centred bubble
    spans roughly x 72..152, y 81..177 against a body of x 79..150, y 57..158 —
    so the union stops being 1.77× wider and 1.45× taller than idle, which is the
    anomaly that identified this row in the first place. ⚠ **that is a prediction;
    check the regenerated rect against the other rows.**
  * ⚠ **regeneration is required and it is a submodule** — `tools/ambition_sprite2d_renderer`.
    Commit inside, then move the pointer, the way D49/D61 did for the map.
    ⚠ **verify by extracting the new `block` frame and LOOKING at it**, not by the
    diff: the whole row exists because the numbers were consistent and the picture
    was not.

  ### ▢ WHAT IS STILL OPEN

  **Regenerate `player_robot_v3`'s `block` row so the guard art matches its
  siblings** — no detached ring, drawn in front of the body, and the frame rect
  back in family with the other rows. This is [`project_sprite_bone_toolkit`]
  work, not engine work. ⚠ **do not "fix" it by moving the engine ring** — the
  engine ring is measured correct and moving it would put the *right* artefact in
  the wrong place to compensate for the wrong one.

  ⭐ **and the cheap partial is worth naming**: because `BubbleShieldVisual`
  already draws a correct centred bubble, the `block` row arguably should not
  paint one *at all*. Deleting the painted ring is a smaller regeneration than
  redrawing it, and leaves exactly one authority for "there is a shield here" —
  the same one-authority argument D47 makes about the body.

  ---

  **The investigation that preceded the answer** (kept: six dead mechanisms):

  * ✔ **verified, and it is two authorities**:
    * the bubble: `rebuild_shield_rings_view` (`sim_view/src/pose_view.rs:307`)
      publishes `pos: kin.pos, size: kin.size` — **pure collision-box geometry**,
      and `sync_bubble_shield_visual` centres the ring on it at
      `size * (1.55, 1.25)`;
    * the character: positioned by its sheet's **feet anchor**, drawn at FRAME
      size, with the body sitting wherever its alpha bbox falls inside that
      frame.
  ### ⛔ MY FIRST EXPLANATION WAS WRONG AND THE FALSIFIER KILLED IT IN ONE SUM

  I wrote that the alpha bbox is asymmetric — hat and arms pulling the box centre
  up and to one side — and that the ring follows it. **Ran the arithmetic on the
  actual sheet before filing the claim:**

  ```text
  player_robot_v3   frame 224x224   body_pixel_bbox (86, 67, 57x91)
    bbox centre  (114.5, 112.5)      frame centre (112.0, 112.0)
    offset       dx = +2.5           dy = +0.5        ← ~1% of the frame, and RIGHT/DOWN
  ```

  ⇒ **the player robot's body is centred in its frame.** The asymmetry story is
  refuted, and it predicted the wrong direction besides.

  ### ✔ THE REAL MISMATCH, from the same sheet: the FEET ANCHOR

  ```text
  feet_anchor_norm: (x: 0.008929, y: -0.200893)
  ```

  ⭐ **the sprite is displaced by 20% of its frame height and the ring is not.**
  The character's quad is placed through `feet_anchor_norm` so its feet land on
  the box; `sync_bubble_shield_visual` centres the ring on raw `kin.pos` with no
  anchor applied anywhere. ⇒ the bubble sits roughly a fifth of a body-height
  away from the drawn character — which is *"kinda to the upper left"*, and the
  x term (0.9%) correctly predicts that the horizontal error is the small one.

  ⇒ **so it is NOT a D47 symptom after all** — it is a third party ignoring an
  offset that the art pipeline already publishes and the sprite path already
  honours. ⚠ **the row above is left standing as written, wrong hypothesis and
  all**, because the useful part is that a free sum killed it: this is the fifth
  chain today of the shape *verified facts joined by an unverified arrow*, and
  the first where the falsifier ran before the claim reached anybody.

  ### ⛔⛔ AND THE CORRECTION IS ALSO UNVERIFIED. I AM STOPPING HERE.

  Read the anchor arithmetic before writing a fix
  (`sheets/geometry.rs:110`):

  ```rust
  let ay = spec.feet_anchor_y + half_collision_y / render_height;
  Anchor(Vec2::new(0.0, ay))
  ```

  ⇒ the anchor's whole job is to **register the sprite TO the box** — it offsets
  the quad so the character's feet land on the box's bottom edge. So the sprite
  is not "displaced by 20% and the ring is not"; the sprite is placed
  *correctly*, and my second mechanism is as unproven as the first.

  ⭐ **what both stories actually share is the structure, and that part holds**:
  the ring is centred on the box while the character is FEET-ANCHORED inside it.
  Those coincide only when the drawn body fills the box. ⇒ if the box is taller
  than the character — which is **exactly what D47 measures**, because the box
  comes from an alpha bbox that includes a hat — the character sits low in it and
  a box-centred ring floats above them. **That is my first hypothesis again**,
  and I have now argued myself in a circle.

  ⛔ **so this row gets no fix until something is OBSERVED.** Two candidate
  mechanisms, both plausible, both reasoned from code I read *near* the answer.
  ⭐ **the decisive evidence is one picture**: `capture_scene --combat-overlay`
  draws the green collision box over the art, which is how D44 settled the same
  class of question. Get a shielded player into one frame and the answer is
  visible, not inferred.

  ### ✔ AND HERE IS THE COMMAND, so nobody has to find it again

  ```sh
  cargo run -p ambition_app_tools --bin capture_scene -- \
      hall_of_characters player OUT.png 960x540 \
      --press hold:e --warmup 40 --combat-overlay
  ```

  ⭐ **`e` is the shield.** `quick_action: KeyCode::KeyE` in the default keyboard
  preset (`ambition_input/src/presets.rs:109`), `QuickAction` is what
  `control.rs:128` reads as `shield_held`, and `e` is one of the fourteen tokens
  `capture_scene` accepts. ⚠ finding that chain took four greps —
  **preset → action → control field → capture vocabulary** — which is why it is
  written down rather than re-derived.
  ⚠ `--press` restarts the capture clock, so `--warmup` after it counts sim ticks
  **into the held state**, which is what puts the ring on screen.
  ⚠ the capture costs **~167 s** and the whole point is that it is cheaper than
  the two wrong mechanisms this row already produced.

  ### ✔✔ TAKEN 2026-08-09 — AND IT SHOWS SOMETHING NEITHER MECHANISM PREDICTED

  Both of my stories assumed **one** shield visual, offset. The picture shows
  **two**:

  1. a **pale radial glow** centred on the robot, sitting correctly inside the
     cyan collision box — the box is tight around him and he fills it;
  2. ⭐ a **separate thin cyan RING**, drawn up and to the left, overlapping the
     door and the health bar and **not touching the character at all.**

  Measured off the capture (960×540, 4× crop): ring centre ≈ **(84, 257)**,
  robot centre ≈ **(115, 296)** ⇒ **31 px left and 39 px up.** That is Jon's
  *"just kinda to the upper left"*, to the word.

  ⇒ ⛔ **the row's whole framing was wrong.** *"The ring tracks the box while the
  character tracks its feet"* predicts a small offset of one visual. What is
  actually on screen is an extra ring in the wrong place while a correctly-placed
  glow does the job — a different defect with a different fix.

  ⚠ **and I am NOT naming which sprite is which from a picture.** Candidates, in
  the order worth checking:
  * `rendering/mod.rs:204` names *"morph-ball sprite + bubble-shield sprite"* as
    following **the same pattern** — so the misplaced ring may be the morph ball,
    not the shield at all;
  * `sync_bubble_shield_visual` POOLS its rings and hides unassigned ones; a ring
    assigned and then not repositioned would sit wherever it last was;
  * a ring belonging to a **different shielder** would sit at that body — but no
    NPC stands by the door, which argues against it.

  ⭐ **the next step is cheap and is now obvious**: run the same capture with the
  bubble-shield system disabled, or tint the two sprites differently, and see
  which one moves. **One more picture names the culprit** — do that before
  touching any positioning code.
  ⚠ three mechanisms have now died on this row (alpha bbox, feet anchor, and
  "one offset visual"). ⛔ **the fourth idea does not get to skip the camera.**

  ### ✔✔ THE CONTROL CAPTURE — both visuals are the SHIELD, and the ring is real

  Same frame, `--press hold:e` removed:

  ```text
  with the shield   glow on the robot  +  thin ring 31 px left / 39 px up
  without it        NEITHER
  ```

  ⇒ **the stray ring is not the door's gate ring** (my next guess, from
  `primitives.rs`'s *"the gate ring + gate portal stay owned by the portal
  systems"*) — **it is the bubble shield**, and Jon's complaint is real and
  reproducible in one command.
  ⭐ **and `build_bubble_shield_image` generates a RING**, hollow, `inner_r 0.32`
  → `outer_r 0.46`, white so `Sprite.color` tints it. So the *glow* is the
  correctly-placed ring at the player's own size (30×48 × 1.55/1.25 ≈ 46×60,
  which reads as a soft ellipse at this zoom) — **and the thin circle is a
  SECOND, larger one.**

  ⇒ **the narrowed suspect is the POOL.** `sync_bubble_shield_visual` grows it
  with *"the new rings get positioned next frame"* and hides the unassigned —
  so a ring that was spawned and neither positioned nor hidden is exactly the
  shape of what is on screen. ⭐ **the probe is to count `BubbleShieldVisual`
  entities and print their transforms** — not to look harder at a picture.

  ⚠ **BUT re-read the system before assuming the pool.** Every iteration either
  positions AND shows a ring or hides it:
  ```rust
  if let Some(ring) = active.get(assigned) { …position…; *vis = Visible; assigned += 1; }
  else { *vis = Hidden; }
  ```
  ⇒ a stray **visible** ring is impossible from this loop alone. It requires
  `active.len() >= 2` — i.e. **a SECOND BODY IS SHIELDING**, and its ring is
  drawn at its own `pos`/`size`. ⭐ **that is the first thing the probe should
  print: `ShieldRingsView.len()`.** If it is 1, the pool theory is back; if it is
  2, find out who.

  ### ⭐⭐ THE SPAWN TRANSFORM IS THE WORLD ORIGIN

  `new_ring_sprite` (`bubble_shield.rs:82`) — the pool's constructor, used by
  BOTH the seeder and the grow-on-demand path:

  ```rust
  Sprite { custom_size: Some(Vec2::new(48.0, 64.0)),
           color: Color::srgba(0.5, 0.8, 1.0, 0.0), .. },   // alpha 0
  Transform::from_xyz(0.0, 0.0, WORLD_Z_PLAYER - 0.05),      // ⛔ THE ORIGIN
  Visibility::Hidden,
  ```

  ⇒ **an unpositioned ring sits at world (0,0)** — the same default-to-origin
  shape as D54's slash, in the same crate, found the same day. ⭐ **and the
  player entered at `hall_of_characters_entry`, the far-left door**, so world
  (0,0) is plausibly ~31 px left and ~39 px up of where he is standing. That is
  not a coincidence worth ignoring.

  ⚠ two guards are supposed to prevent it being SEEN — `Visibility::Hidden` and
  `alpha 0.0` — so if it is visible, one of them was cleared without the
  positioning that belongs with it. **That is the thing to find.**

  ⭐ **THE DECISIVE EXPERIMENT, and it needs no probe**: capture with the player
  MOVING while shielded (`--press hold:e,hold:right --warmup 90`). **World-anchored
  ⇒ the ring stays behind as he walks away. Body-anchored ⇒ it follows.** That is
  the same-frame-one-thing-changed control the pixel diff could not be.

  ### ⛔ RAN IT. THE ORIGIN HYPOTHESIS IS DEAD — AND SO IS "LAG".

  The player walked ~360 px right. **The ring travelled with him and kept the
  SAME displacement** — up-left by the same amount, glow still centred on him.

  * ⇒ **not world-anchored**: it is not the unpositioned pool ring at (0,0).
  * ⇒ **not a lag either**: a stale pose would trail by an amount that varies
    with speed and direction. This offset is **constant and body-relative**.
  * ⭐ **and it is the WRONG SIZE.** Measured off the 5× crop, the stray ring is
    ~120 px across against the ~46×60 a 30×48 player body implies
    (`ring.size * (1.55, 1.25)`). **≈2.5× too big** ⇒ if it is a shield ring at
    all, it is sized from a body of roughly **77×96**, which is not the player.

  ⇒ ⛔ **five mechanisms dead** (alpha bbox · feet anchor · one-offset-visual ·
  sim-vs-presented pose · spawn-at-origin), and the picture route is exhausted:
  a constant offset AND a 2.5× size mismatch is more than an image can
  adjudicate.
  ⭐⭐ **the probe is now mandatory and its content is exact** — print
  `ShieldRingsView`: `len()`, and for each entry its `pos` and `size`, beside the
  player's own `kin.pos` and `kin.size`. **`len() == 2` names a second shielder;
  `len() == 1` with a 77×96 `size` names a body-size bug; `len() == 1` with a
  46×60 `size` means the ring on screen is not from this view at all.** Three
  outcomes, three different rows.

  ⛔ **and do NOT try to settle it by pixel archaeology — I did, and it failed.**
  Diffing the shield-held and shield-released captures looked like a clean
  isolation and is not: **178 connected components across the whole frame**,
  because the two captures also differ in ordinary animation. The largest
  component (2,632 px, bbox 73×84 at centre (100,284)) **merges the glow and the
  ring**, which touch, so a flood fill cannot separate them and the implied body
  size it yields (47×67) is meaningless. ⚠ **an image diff of two live frames is
  not a controlled experiment** — the control has to be the same frame with one
  system changed, not the same scene at a different moment.

  ### ⛔ AND MY FOURTH MECHANISM DIED TOO — the presented-pose fix did not move it

  `rebuild_shield_rings_view` read raw `kin.pos` while **both** sibling overlays
  read the presented pose and say why (`unauthored_volumes.rs:140`,
  `slash_visuals.rs:359`). I aligned it. **The ring did not move** — a pixel diff
  of the two captures returns a bbox nowhere near it.

  ⇒ **the change is KEPT, and its framing is the point**: three presentation
  overlays now agree on one pose source, which is right on its own terms and is
  what the other two already document. ⛔ **it is not a fix for this row and the
  code comment must not imply it is.** ⚠ interpolation between two sim ticks
  cannot produce a 39 px gap anyway — the arithmetic said so before the capture
  did, and I ran it regardless because the alignment was worth having.
  ⚠ **do not "nudge the ring" under either story** — a constant offset is a third
  authority and it breaks the moment D47 corrects the box.
  * ⇒ ⛔ **do not "fix" this by nudging the ring with a constant.** That would be
    a third authority and it would break the moment a sheet's anchor changes.
    **The ring must read the same `feet_anchor_norm` the sprite reads** — one
    published fact, two consumers, which is the shape this repo already uses for
    the pose read-model.
  ### ⇥ ✔ THE CENSUS THIS ROW ASKED FOR IS DONE, AND IT COMES BACK EMPTY

  The instruction was *"grep for `kin.pos` consumers in `ambition_render` and
  count them — count who IGNORES the anchor, not who uses it"*. Run 2026-08-13:

  ```text
    raw `kin.pos` / `kinematics.pos` consumers in ambition_render:   0
  ```

  Every position that crate draws from is a VIEW value (`view.pos`, `pose.pos`,
  `request.pos`, …). ⚠ and the overlay this row names as *"the first place to
  look"* — the morph-ball sprite, *"the same pattern"* as the bubble shield —
  already reads the presented pose, and says so at the assignment: *"the sphere
  IS the body while morphed, so it draws where the body is presented — not where
  its last tick left it."*

  ⇒ **the "other overlays have the same defect by construction" worry is
  refuted**, which removes a whole class rather than another single mechanism.

  ### ⇥ ⭐⭐ AND THE REASON THE RING CANNOT SEE THE ANCHOR IS STRUCTURAL

  This row's own prescription is *"the ring must read the same `feet_anchor_norm`
  the sprite reads — one published fact, two consumers"*. Measured why it does
  not: **the anchor is not a published fact at all. It is a `bevy::sprite::Anchor`
  COMPONENT on the sprite entity** (`sprite_sheet/boss.rs:433`,
  `Anchor(Vec2::new(0.0, self.feet_anchor_y))`, with the character path passing
  `spec.feet_anchor_y` the same way).

  ⚠ **an `Anchor` shifts the image within its own entity and is invisible to
  every sibling.** So a ring drawn at the body's world position and a sprite drawn
  at the same world position with a non-zero anchor CANNOT coincide — the robot's
  sheet authors `feet_anchor_norm: (x: 0.0059, y: -0.3047)`, so the art is offset
  by roughly a third of its render height from the origin the ring uses.

  ⇒ that is a mechanism with a reason, not a fifth guess: the two consumers this
  row wants are not disagreeing about a value, they are reading DIFFERENT KINDS OF
  THING — one a component Bevy applies during layout, the other a world position.
  ⛔ the fix therefore cannot be "make the ring read the anchor" as a value copy;
  the anchor has to become a pose-view fact both can read, which is the shape the
  row already argues for. ⚠ **not attempted here**: it changes where every
  anchored sprite draws, and this row has paid four times for acting on a
  mechanism before the arithmetic was checked against a capture.

  * ⚠ **and check the other pooled overlays before declaring it fixed.** Anything
    else positioned from raw `kin.pos` has the same defect by construction — the
    morph-ball sprite is named beside the bubble shield at
    `rendering/mod.rs:204` as *"the same pattern"*, so it is the first place to
    look. **Grep for `kin.pos` consumers in `ambition_render` and count them**
    ([[reference_count_the_adopters_not_the_capability]] in reverse: count who
    ignores the anchor, not who uses it).

- ⏸ **D54 THE ENGINEERING IS DONE; THE ROW IS WAITING ON ONE REPRODUCTION.**
  Verified against HEAD 2026-08-13: `owner_pos` returns `Option<Vec2>`
  (`slash_visuals.rs:352`), and its one caller skips the effect with a warn
  naming the entity rather than drawing at the world origin. The `Vec2::ZERO`
  fallback this row was written about does not exist any more.

  ⇒ **there is no code left to write here.** Jon's *"in smash, choosing robot v3,
  if you do your attack the VFX happens in the top left corner"* either prints
  that warn on the next reproduction — confirming the missing-owner chain the row
  deliberately refused to assume — or does not, which falsifies it outright and
  sends the search elsewhere. ⚠ **an agent cannot run this**: it needs the report
  reproduced, and the instrument now makes one attempt decisive.

  ⇥ AS WRITTEN: ▢ **A SLASH WHOSE OWNER CANNOT BE FOUND IS DRAWN AT THE WORLD ORIGIN.**
  Candidate for Jon's *"in smash, choosing robot v3, if you do your attack the
  VFX happens in the top left corner, not in the authored area"* — ⚠ **filed as a
  latent hazard that is VERIFIED, plus a link to his bug that is NOT.** Keeping
  those apart is the point of the row.

  ### ✔ 2026-08-09 — THE ART IS RULED OUT, so the hazard is now the leading candidate

  D55 proved that **this same character's `block` row paints a detached element
  up-left into the spritesheet**. Same character, same corner, same complaint
  shape ⇒ the obvious next question is whether v3's *attack* rows do it too, which
  would make Jon's two reports one art bug. **They do not.** Every frame rect in
  `player_robot_v3_spritesheet.yaml`, ranked by area:

  ```text
  block          w=119..126  h=144..149   <- the D55 outlier, big in BOTH axes
  dash_startup   w=130..142  h=112..113   <- wide only: a pose extending
  slide          w=134..144  h=110..110   <- wide only
  dash           w=140..143  h=103..104   <- wide only
  ...
  attack_down    w= 71.. 71  h=106..114   ✔ in family
  attack_side    w= 76.. 76  h=103..103   ✔
  attack_up      w= 74.. 79  h= 97.. 97   ✔
  slash          w= 70.. 72  h=100..103   ✔
  idle           w= 70.. 71  h=101..103   (the baseline)
  ```

  ⇒ **the attack rows are indistinguishable from idle.** There is no detached
  element in the attack art, so **Jon's line-85 smash bug is NOT D55**, and the
  two must stay separate rows.

  ⭐ **two things follow, and the negative result is worth more than a positive
  one would have been:**
  1. **this row's own worry is now retired.** It said the `Vec2::ZERO` fallback
     *"reads as an art or anchor bug and sends the next person into the sprite
     pipeline"* — I went into the sprite pipeline, and it is clean. ⇒ **the
     world-origin hazard is the leading candidate again**, on elimination rather
     than on assertion.
  2. **the observable now exists.** `owner_pos` returns `Option` and warns instead
     of drawing at `(0,0)`, so the next reproduction either prints the warn —
     confirming the missing-owner chain this row deliberately refused to assume —
     or does not, which falsifies it outright. ⇒ **ask Jon to reproduce it once.**

  ⚠ **and it sharpens D55 too**: `dash`, `slide` and `dash_startup` are all
  *wider* than idle without being taller, which is what a pose extending
  horizontally looks like. `block` is the only row that grows in **both** axes —
  the signature of an added *detached* element rather than a bigger pose.

  * ✔ **the hazard, verified** (`ambition_render/src/rendering/slash_visuals.rs:321`):
    ```rust
    fn owner_pos(owners: &Query<(&BodyPoseView, Option<&PresentedPose>)>, owner: Entity) -> ae::Vec2 {
        owners.get(owner)
            .map(|(pose, presented)| presented.map_or(pose.pos, |p| p.presented()))
            .unwrap_or(ae::Vec2::ZERO)          // ⛔ owner not in the query → WORLD ORIGIN
    }
    ```
    ⇒ *"I could not find the swinging body"* silently becomes *"draw the swing at
    (0,0)"*, and in a room whose content sits at positive coordinates that is
    **the top-left corner**.
  * ⛔ **the default destroys the diagnosis.** Returning `Option` and skipping
    would make the failure *nothing drawn* — legible, and greppable with one
    warn. Instead it draws garbage in a corner, which reads as an art or anchor
    bug and sends the next person into the sprite pipeline. Same family as
    [[reference_a_system_wide_return_on_a_singleton]]: a fallback that keeps the
    system running by inventing an answer.
  * ⚠ **AND THIS IS WHERE I STOP, BECAUSE THE NEXT LINK IS UNVERIFIED.** For
    smash to hit it, its fighters must lack `BodyPoseView`. Against that:
    `rebuild_body_pose_views` is installed by `FeatureViewSyncSchedulePlugin`
    from `ambition_platformer2d_runtime:419`, and smash composes
    `PlatformerEnginePlugins::fixed_tick()`, so it very likely DOES run. The
    system's own comment even asserts the population is complete: *"requires only
    `BodyKinematics`, so every body this matched still matches."*
    ⇒ **the arrow "Jon's VFX is at the top-left BECAUSE of this fallback" is
    exactly the shape that has cost me four wrong chains.** Do not write it down
    as the cause.
  * ⭐ **FALSIFIER, and it is cheap**: make the fallback loud — one `warn!` on
    the `unwrap_or` path — and have Jon reproduce once. Fires ⇒ confirmed, and
    the real question becomes *which* bodies lack the view. Silent ⇒ the slash is
    positioned correctly and the top-left thing is a different system (check the
    unauthored-volume stand-in at `unauthored_volumes.rs:116`, which resolves its
    owner through the same read-model and has its own skip).
  * ⇒ **the fix is worth doing on its own merits either way**: a missing owner is
    not the origin. That is a two-line change plus a warn, and it converts a
    future instance of this class from a corner-of-the-screen mystery into a log
    line naming the entity.

  ### ✔ FIXED 2026-08-09 — an absent owner is now an absent slash

  `owner_pos` returns `Option`; the caller skips and warns, naming the entity.
  ⛔ **and the row is NOT claiming this fixes Jon's smash VFX** — the comment in
  the code says so explicitly, because the population is filled by
  `rebuild_body_pose_views` from a plugin smash composes, so the miss may never
  fire. ⭐ **the warn is the falsifier**: if it appears in a device log, the chain
  is confirmed and the question becomes *which* bodies lack the view; if it never
  appears, the top-left VFX is a different system and this row stops being a
  candidate.
  ⚠ **that is the whole shape worth copying** — a fallback that invents an answer
  was replaced by one that reports, so the NEXT occurrence diagnoses itself
  instead of costing an investigation.

  ### ✔ 2026-08-10 — AND THE OTHER CANDIDATE NOW REPORTS TOO

  The falsifier above says: *if the slash warn never fires, the top-left VFX is a
  different system — check the unauthored-volume stand-in.* That site skipped
  correctly (no origin draw) and **silently**, so the test had no second half:
  whichever system was responsible, the log said nothing and Jon's one
  reproduction would have been spent for nothing.

  It warns now, `warn_once` (it runs per strike per frame; a live swing would
  otherwise bury the log), naming the owner and saying outright which system it
  implicates. ⇒ **the pair is a decision procedure**: slash warn ⇒ the pose
  read-model misses that body; stand-in warn ⇒ this system; neither ⇒ both are
  positioned correctly and the stray VFX is a third thing.

  ⛔ still needs the one reproduction from Jon — that has not changed. What
  changed is that the reproduction now returns an answer either way.

- ▢ **D53 ANDROID SUSPEND/RESUME — device validation and the residual first-freeze gap.**
  The decision logic was extracted from Android glue, unit-tested on the host, and the destructive
  `.take()` behavior was fixed. What remains cannot be established from this checkout:
  * reproduce/validate the suspend→resume sequence on an Android device/NDK build;
  * if the report persists, remove the residual dependence on receiving another lifecycle edge — a
    refused restore currently remains pending until `just_changed` fires again;
  * preserve the distinction between a pause Ambition forced for lifecycle reasons and a pause the
    participant intentionally requested, so recovery never stomps an intentional menu pause.
  Do not redo the old host-side diagnosis; it is archived in
  [`../archive/planning-superseded/2026-08-13/queue-pruned-sections.md`](../archive/planning-superseded/2026-08-13/queue-pruned-sections.md).

- ▢ **D70 ⛔ SOME MARY-O BLOCKS STAY SPENT ACROSS A RESTART — and it is NOT a
  symptom of D68.** Jon's *"when you restart the level all item blocks and enemies
  should reset; currently some blocks from the last run remain spent."* **Filed
  2026-08-09, un-coupled from D68 the moment its falsifier came back GREEN.**

  * ✔ **the mechanism is present and correct** — `rearm_bricks_for_a_fresh_attempt`
    and `rearm_power_blocks_for_a_fresh_attempt` both clear on `RoomLoaded` **or**
    `RoomReplayRequested`, both registered (`lib.rs:1932-33`) on
    `ContentRoomReplayResetSet`, anchored **before** the generic replay consumer
    *"so a death clears them the same frame the request lands."*
  * ✔✔ **and the request DOES land** — D68's falsifier proves the fatal-hit route
    replays the room end to end. ⇒ **so the conditional this row hung on is
    satisfied, and the complaint survives it.**
  * ⭐ **which makes this the interesting shape**: a fix that is present,
    registered, correctly ordered, and provably reached — **and Jon still sees
    stale blocks.** ⇒ **the defect is in WHICH blocks**, not in whether the reset
    fires.
  * ⚠ **the falsifier, and it must name a SPECIFIC block**: bonk a `?`-block,
    break a brick, die, and assert **both** come back. ⛔ **do not assert "the
    sets are empty"** — `SpentPowerBlocks::rearm_all` and `BrokenBricks::clear`
    obviously empty their own sets; the question is whether every kind of
    per-attempt block state lives in one of those two sets.

  ### ⛔ CORRECTION TO THIS ROW, SAME HOUR — my "start here" was wrong

  I wrote *"a discovered Hidden block is promoted through the overlay, which is
  neither a spent power block nor a broken brick — start there."* **Checked it
  before dispatching anyone at it. It is false.** A discovered Hidden block is
  tracked in `SpentPowerBlocks` like every other `?`-block —
  `contribute_discovered_hidden_blocks_to_overlay` reads it through
  `discovered_solid(&spent, block)` — so `rearm_all` clears it with the rest.

  ⇒ ✔ **the art side re-derives too**: `dress_power_blocks` (`:1224`) takes
  `spent: Res<SpentPowerBlocks>` and re-evaluates **every block every frame**, so
  clearing the set restores the unspent look without anyone rewriting art.

  ⇒ ⛔ **both block kinds and both halves — state and picture — are complete.**
  **I have no third state to point at, and I am saying so rather than inventing
  one.**

  ### ⭐⭐ WHICH MAKES D70 AND D68 THE SAME SHAPE, AND POSSIBLY THE SAME BUG

  Two of Jon's reports now read identically: *"the restart does not clean up"*,
  where the cleanup is **present, registered, correctly ordered, and provably
  reached on every route that has a test.**

  ⇒ **the common suspect is the ROUTE, not the cleanup** — and exactly one death
  route has no end-to-end test: `publish_kernel_reset_death`, which is **a pit, a
  spike, any hazard**. ⭐ **if Jon was dying that way, one untested route explains
  both complaints**, and both fixes are the same fixture.

  ### ⇥ ⛔⛔ THE "ONE UNTESTED ROUTE" IS TESTED, AND IT PASSES — 2026-08-13

  This row's central hypothesis is *"the common suspect is the ROUTE, not the
  cleanup — exactly one death route has no end-to-end test:
  `publish_kernel_reset_death`, which is a pit, a spike, any hazard"*. Grepped
  before working it:

  * ✔ **`room_replay::a_pit_death_returns_her_to_spawn_and_rearms_a_spent_block`
    exists and is GREEN.** It drops her into the pit inside the blast margin,
    asserts the death was charged to the controlled body with source
    `LeftTheWorld` (so the fixture provably exercises
    `publish_kernel_reset_death` and not a hit or a timeout), and asserts a
    SPECIFIC `?`-block taken from the live room comes back.

  ⇒ **so the pit route resets `?`-blocks correctly**, and the route hypothesis is
  refuted for that half. Whatever Jon is seeing is not "the hazard death never
  reaches the replay".

  ### ⇥ ✔✔ AND THE ROW'S OPEN QUESTION IS ANSWERED — there is NO third state

  This row's falsifier spec exists to settle *"whether every kind of per-attempt
  block state lives in one of those two sets"*, and it ends *"I have no third
  state to point at, and I am saying so rather than inventing one"* — an absence
  of evidence. Turned into evidence of absence 2026-08-13, by enumerating rather
  than searching:

  * ⭐ **`ambition_demo_mary_o` declares exactly FIVE resources**: `BrokenBricks`,
    `SpentPowerBlocks`, `FlagPole`, `MaryOEntryRoom`,
    `MaryOQuasarShaderSettings`. Only the first two hold per-attempt BLOCK state,
    and **both consume `RoomReplayRequested`** (`bricks.rs:209`,
    `powerups.rs:1384`).
  * ⭐ **and neither block kind touches an ENTITY.** `break_bricks`'s whole effect
    is `broken.mark(&block.name)` — no despawn, no component write — and
    `contribute_broken_bricks_to_overlay` re-derives the overlay from the set
    every frame, exactly as `dress_power_blocks` does for `?`-blocks. State in a
    set, picture re-derived: clearing the set restores both, and there is nothing
    for a replay to miss.

  ⇒ **so the "third kind" hypothesis is closed, not merely unexamined.** Both
  kinds live in the two sets, both sets rearm on the replay, both pictures follow
  their set, and the pit route provably reaches the replay
  (`a_pit_death_returns_her_to_spawn_and_rearms_a_spent_block`, green).

  ⇒ **which leaves exactly one explanation standing for Jon's report: a death
  ROUTE that does not emit `RoomReplayRequested`** — and that is D68's question,
  awaiting his one-line answer. The row predicted this landing; what is new is
  that the alternative is now eliminated rather than unchecked.

  ### ⇥ ▢ AND THE BRICK HALF OF THE FALSIFIER IS SIZED — it needs a HEAD-BONK fixture

  That test says so itself: *"this does NOT claim there is no third kind of
  per-attempt block state; that is D70's open question"*. The row's own falsifier
  spec is *"bonk a `?`-block, break a BRICK, die, and assert BOTH come back"*, and
  only the first half is written. Measured why:

  * ⛔ **`BrokenBricks`'s mutators are private** — `mark`, `is_broken`, `clear`
    are all crate-local; the only public observation is `checksum()`. An
    integration test in `ambition_demo_mary_o_app` cannot poke a brick broken, and
    should not: poking is what the `?`-block half deliberately avoided by using
    the same public write `bonk_power_blocks` performs.
  * ⇒ **the brick must be broken the way the game breaks one**: `break_bricks`
    consumes `PlayerBodyFrameOutput`'s head contacts and checks her FORM (a small
    Mary-O breaks nothing), so the fixture has to put a big body under an authored
    brick and make it jump. That is a real fixture, not a line.
  * ⭐ **and `checksum()` is the observation it should use** — a value that
    changes when the set does, without the test naming a private field.

  ⚠ **this does not need Jon's D68 answer.** It is the half of this row's own
  falsifier that was never written, and it either reproduces his report on the
  brick path or narrows the search to a third state.

  ⇒ ⛔ **do not dispatch D70 before Jon answers the D68 question**
  (`awaiting-maintainer-decision.md`). ⚠ **a worker sent at this now would
  re-derive the two paragraphs above and stop**, which is a wasted dispatch, not
  a finding. **The next real step is his one-line answer.**

- ✔✔ **D68 THE FALSIFIER CAME BACK GREEN — `5fddacd76` (2026-08-09).** The chain
  works composed: she is held at the death site through the beat, the room replays
  once, and she is **at spawn** when it ends.

  * ✔✔ **the green was poisoned, and the red is Jon's report VERBATIM.** Removing
    `restart_level_after_death`'s `replay.write(..)` produces
    ```text
    she is still at Vec2(664.6, 384.1)
    ```
    ⇒ **the test can see the bug he describes, and it does not happen.** Both
    terms observed — the fixture asserts she was **away** from spawn during the
    beat before asserting she came home.
  * ⇒ ⭐ **per this row's own pre-stated reading, GREEN means ASK HIM, not hunt.**
    ⛔ the worker stopped there, correctly. **See the question filed for Jon.**
  * ✔✔ **ALL THREE ROUTES ARE NOW COVERED, AND ALL THREE ARE GREEN.**
    `a_pit_death_returns_her_to_spawn_and_rearms_a_spent_block` (`4077cb2cc`)
    closes the last one: `death_respawn_player` (a hit) ✔, the level timeout via
    `spend_lives_on_death` ✔, and **`publish_kernel_reset_death` (a pit, a
    hazard) ✔.**
    ⇒ ⛔ **no tested death route reproduces Jon's report.**

  ### ⭐⭐ HOW THE KERNEL RESET WAS DRIVEN — the part I could not specify

  **Nothing was written directly.** The fixture relocates her via `transit_body`
  to *below the room floor but INSIDE the blast margin*, then **only steps
  frames**: gravity carries her past `World::blast_margin`,
  `apply_world_hazard_gate` (`movement/kernel.rs:425`) raises
  `ResetCause::LeftTheWorld`, `integrate_home_body` turns that into a
  `BodyReset`, and `publish_kernel_reset_death` publishes. **No death, no reset,
  no health value and no replay request is written by the test.**

  ⭐⭐ **and it asserts WHICH route fired before concluding anything from it.** A
  `DeathsSeen` recorder captures `(victim, cause.source)` off `ActorDiedMessage`
  and the fixture requires `HitSource::LeftTheWorld` on the primary player.
  `death_source_of` is the only producer of that source for the controlled body,
  so **the fixture cannot go green having quietly driven the fatal-hit route
  instead.** ⇒ **that is the discipline that makes a green mean something** — the
  obvious version of this test would have proven the route it was not aimed at.

  ✔ **poisoned, and one deleted line produces BOTH of Jon's sentences**:
  ```text
  SHE DIED IN A PIT AND maryo_block:Question:TowardLantern:… IS STILL SPENT:
  the room was put back 0 time(s), she is at Vec2(78.0, 973.4) and spawn is Vec2(78.0, 375.0)
  ```

  ### ⚠ THREE CORRECTIONS, and the first is a near-miss worth keeping

  1. ⭐ **they nearly documented the OPPOSITE of what they measured.** A kernel
     reset teleports the body to `world.spawn` by itself, so they expected the
     void to carry her home even under the poison, leaving only block state
     discriminating — and had already written *"the position term is weakly
     discriminating on THIS route"* as prose. **Measured: she is still in the pit
     when the window closes.** The committed doc carries the measurement.
     ⇒ **a plausible mechanism written down before the run would have been a
     false claim in a comment.**
  2. ⚠ **my brief said "assert the same two things the hit fixture asserts" — it
     does not assert block state at all**, only position and `resets >= 1`. They
     built the block assertion new, naming a specific `?`-block read from the
     **live room's** `RoomGeometry` rather than the level constructor.
  3. ⛔ **`BrokenBricks` CANNOT be driven from an integration test** — `mark`,
     `is_broken`, `clear` and `broken_names` are all private; only `checksum()`
     is public. ⇒ **D70's *"break a brick, die, assert it comes back"* half is
     unreachable** without a real head-bonk (needs her big, in 1-2's cavern) or
     widening that API. **Know this before dispatching D70.**

  ⚠ **and a fixture hazard for anyone seeding per-attempt state**: a block spent
  on the frame the body first becomes queryable is **silently wiped** — activation
  keeps emitting `RoomLoaded` for two or three more frames plus a `Manual` reset,
  and both re-arm. ⭐ **their first run failed the PRE-assertion rather than going
  vacuously green**, because the spend is asserted before the kill. Settle 60
  frames first.

  ### ⛔⛔ MY D26 BRIEF'S KILL ROUTE WAS INERT, AND IT WOULD HAVE SHIPPED THE BUG

  I told them to kill her with `e.get_mut::<BodyHealth>().unwrap().health.current
  = 0`, citing `power_loop.rs:1124`. ⛔ **That kills a SNAKE**, where enemy logic
  polls `alive()`. **Nothing polls the controlled body's health for death.**
  Measured, not modelled: **a hand-zeroed Mary-O walked the room at `hp = 0` for
  120 frames and the beat never armed.**

  ⇒ **had they followed the brief, D26 would have shipped a fixture that measured
  a live, undying Mary-O and called it a death beat** — *the exact class of defect
  D26 exists to repair.* ⭐⭐ **the row would have been closed by an instrument with
  the same blindness as the one it replaced.**

  ⚠ **and the repo already said so, in prose I did not read**
  (`game/ambition_app/tests/versus_stage.rs:784`):
  > *"Writing health to zero directly proves nothing… a hand-zeroed body never
  > invokes it and the test passes whether or not the fix exists."*

  ⇒ both fixtures now kill with a real lethal `HitEvent` through a shared
  `deal_a_lethal_hit` helper.

  ### ⚠ AND "NOTHING CAN SEE IT" WAS THREE-QUARTERS TRUE, NOT TRUE

  `room_replay.rs::the_level_timeout_actually_replays_the_room` **already**
  composed the app, ran a death through `MaryODeathSequence`, and asserted
  `home.distance(spawn) < 64.0`. ⇒ **one death route was observed end to end**;
  the fatal-hit route was not. The row's core finding stands — the *unit* test
  counts messages — but **the green was more likely than I implied**, and I should
  have found that fixture before writing "nothing proves it".

  ### ✔ D26's SECOND FIXTURE LANDED `51fa22de1` — and it measures something

  `the_death_beat_is_measured_with_the_world_awake`. The pit fixture is untouched;
  its 4000 depth stands. It **asserts the instrument can SEE before it reports**:
  at least one policy-bearing body awake, and the signature actually varying.
  ```text
  4 of 17 policy-bearing bodies awake for all 192 frames of the dwell
  signature 37,300 → 36,840, drifting continuously, then snapping
  ```
  ⇒ **the world keeps living through the beat** — the same conclusion the old row
  reached from captures, now **from an instrument that could have said
  otherwise.** ✔ poisoned with the pit fixture's own `displace`: *"the world fell
  asleep DURING the beat (0 awake at the floor)"*.

- **D68-ORIGINAL (superseded by the ✔✔ block above — marker stripped, text kept)
  ⛔⛔ THE DEATH→RESTART CHAIN IS TESTED IN TWO HALVES THAT NEVER MEET, SO
  JON'S BUG IS INVISIBLE TO EVERY TEST.** His *"when you die the level doesn't
  restart, you just stay right where you were"* — **no prior ledger row.**
  Scoped 2026-08-09. ⚠ **I did NOT find the bug. I found that nothing can see
  it**, which is a better handoff than a fifth hypothesis would be.

  ### ✔ THE WHOLE CHAIN EXISTS AND IS COMPOSED — four hypotheses refuted

  Per the charter, grepped for what the complaint implies is missing. **None of
  it is missing:**

  | link | verified |
  |---|---|
  | the beat holds her `DEATH_DWELL = 3.2s` | `death.rs:45` |
  | `restart_level_after_death` writes `RoomReplayRequested` | `death.rs:290`, registered at `lib.rs:1804` |
  | a consumer exists | `sandbox_reset.rs:116`, *"the one `RoomReplayRequested` consumer every host drains"* |
  | the consumer is COMPOSED for Mary-O | `PlatformerEnginePlugins::fixed_tick()` adds `RoomReplaySchedulePlugin` (`runtime/lib.rs:482`); `mary_o_app/src/lib.rs:26` composes it |
  | the reset MOVES her | `reset_sandbox` → `ae::reset_body_clusters(.., world.spawn, ..)` |

  ⇒ ⛔ **so "the consumer is missing / not composed / doesn't reposition" are all
  dead**, and the group's own comment shows someone already hit and fixed the
  missing-consumer version: *"without a consumer here, a standalone demo binary
  writes the message into a channel nothing drains."*

  ⚠ **and one of my greps lied on the way**: a narrowed
  `RoomReplayRequested | (fn|read|MessageReader)` returned **nothing**, which
  reads exactly like *"no consumer exists"*. The unfiltered grep found it
  immediately. [[feedback_grep_for_capability_not_type_name]] — **a narrowed
  absence-grep is the one that ends investigations wrongly.**

  ### ⭐⭐ WHAT IS ACTUALLY WRONG: the test asserts LESS THAN ITS NAME

  `death/tests.rs::she_dies_in_place_holds_the_pose_and_then_the_level_restarts`
  — the name promises a level restarting. What it composes:

  ```rust
  fn app() -> App {                       // :9
      app.add_message::<RoomReplayRequested>();          // registers the CHANNEL
      app.add_systems(Update, (begin_death_sequence,
                              run_death_sequence,
                              restart_level_after_death).chain());
      app                                  // ⛔ no consumer, no room, no body reset
  }
  fn replays(app) -> usize {              // :67
      ...Messages::<RoomReplayRequested>::iter_current_update_messages().count()
  }
  ```

  ⇒ **it counts MESSAGES IN A CHANNEL.** *"The level restarts"* is never
  observed — only *"the request was written"*. ⭐ **the two halves of this chain
  are each tested and they never meet**: Mary-O proves it **asks**, the engine
  proves its consumer **works**, and **nothing proves that Mary-O's death returns
  her to spawn in a composed app.** ⇒ **Jon can be right while every test is
  green**, which is exactly what he reports.

  ⭐ **same defect class as D26 on the same beat** — the freeze instrument that
  *"measures nothing and would report SUCCESS for an unimplemented freeze"*.
  **Two instruments pointed at one death beat, neither able to see its subject.**

  ### ▢ THE FALSIFIER — one test, and it decides everything

  Compose Mary-O the way `mary_o_app/src/lib.rs:26` already does, kill her, run
  past `DEATH_DWELL`, and **assert her position is `world.spawn`**.
  * **red** ⇒ Jon's bug is real and now has a home; the four links above are each
    verified, so the defect is in their *composition* — ordering, the
    `RoomTransitionCooldown` the consumer takes as `ResMut`, or
    `sequences.single_mut()` finding 0 or 2 `MaryODeathSequence`s.
  * **green** ⇒ the chain works composed, and the question becomes what his build
    had that this does not — ⛔ **at which point ASK HIM rather than hunt.**
  ⚠ **write it before touching anything.** This row already killed four
  mechanisms cheaply by refusing to act on them; D55 cost six because it acted.

  ⚠ **Jon also asks for LIVES** (*"restart the level with 1 less life … allow
  lives to go negative, no game over screen yet"*). ⛔ **that is a FEATURE and it
  does not exist** — do not fold it into the bug. Sequence it after, and only if
  the falsifier comes back green.

  ### ⭐⭐ A SECOND OBSERVATION OF JON'S HANGS OFF THIS SAME FALSIFIER — 2026-08-09

  Scoping his *"when you restart the level all item blocks and enemies should
  reset; currently some blocks from the last run remain spent"* — a separate,
  unfiled complaint — lands on **exactly this chain**, and the mechanism is
  already built:

  * ✔ **both rearm systems exist and are REGISTERED** —
    `bricks::rearm_bricks_for_a_fresh_attempt` and
    `powerups::rearm_power_blocks_for_a_fresh_attempt` (`lib.rs:1932-33`), each
    clearing on `RoomLoaded` **or** `RoomReplayRequested`.
  * ⭐⭐ **and this exact bug was already fixed once**, in the past tense, in the
    registration's own comment:
    > *"They used to hang off the `FeatureInteraction` chains reading
    > `RoomLoaded` alone, **which a death never emits**; … the host anchors it
    > before its generic replay consumer, **so a death clears them the same frame
    > the request lands.**"*

  ⇒ ⭐ **"the same frame the request lands" is a conditional, and D68 is the
  question of whether it lands at all.** ⇒ **the two complaints are coupled:**
  * **falsifier RED** ⇒ *"blocks stay spent"* is a **SYMPTOM of D68**, not its own
    bug, and one fix closes both of Jon's reports.
  * **falsifier GREEN** ⇒ they are genuinely separate and the spent-block report
    needs its own investigation — ⚠ **starting from the fact that the mechanism
    is present and correct**, which is where this scoping ends.

  ⇒ ⛔ **do not file the spent-block complaint as its own row until the falsifier
  runs.** Filing it now would create a second row for what is probably one defect,
  and the ledger already carries enough of those.

- ✔ **D69 FIXED 2026-08-09 — AND THE ROW'S OWN PRESCRIPTION WAS THE WRONG SHAPE.**
  The probe below is un-ignored and green; `demo_mary_o_app --features visible`
  is **45 passed / 0 failed / 0 ignored**, `ambition_render` 99, `app_it` 323.

  ⭐ **it is a REPLACEMENT read as a DELETION, not a missing reconciler.**
  `contribute_discovered_hidden_blocks_to_overlay` pushes the block's own name
  into `removed_block_names` **and** the promoted block into `blocks` — same
  name, same box, same `GeoId`. `sync_removed_block_visuals` read only the first
  half and despawned the sprite the second half was asking for. ⇒ the fix is one
  clause in that reconcile: **a subtracted name the SAME overlay is re-adding is
  skipped.** `dress_power_blocks` then dresses the surviving entity with the
  spent plate, every frame, from `SpentPowerBlocks` — no new system, no new
  component, no new draw path.

  * ⛔⛔ **the row's "add an added-block reconciler, generalising
    `sync_lock_wall_visuals`" was the wrong abstraction**, and the ordering trap
    it warns about **exists only inside that wrong shape**. Two reconcilers
    reading the two halves of one statement would have to be ordered against each
    other; ONE reconcile reading both halves of one frame's overlay cannot race
    itself. ⇒ **no ordering assertion is owed and none was written.** ⭐ the trap
    was real *conditional on the design*, which is the useful form of that
    warning to keep.
  * ✔ **the six moving `PogoOrb` volumes are ANSWERED, and they are why the
    generalisation would have been actively wrong.** `rebuild_feature_ecs_world_overlay`
    (`actor_monolith/src/world/overlay.rs`) fills `overlay.blocks` with
    engine-contributed **invisible collision volumes**: pogo-bounce targets
    published by actors/NPCs/bosses (`ecs-pogo-target <FeatureId> <idx>`), the
    legacy fallback, and breakable ghosts — all `GeoId::anon()`, all synthesised
    names no authored room uses, all riding their owners (hence "moving"). ⇒
    **"draw everything in `overlay.blocks`" would have started drawing six orb
    volumes that must never be drawn.** And ⇒ none of them can appear in
    `removed_block_names`, so the narrow fix cannot rescue one by accident.
  * ✔ **the guard carries its own poison, and both halves were falsified.**
    `a_replaced_block_keeps_its_visual_while_a_removed_one_does_not` asserts the
    invariant **and** that an added block under a DIFFERENT name does not rescue
    an ordinary removal. Deleting the skip → first assertion red; widening it to
    `!overlay.blocks.is_empty()` → second assertion red, while the pre-existing
    removal test stays green under both. ⇒ **that too-broad form would have
    shipped**: the same frame that discovers the hidden block carries six
    unrelated overlay blocks.
  * ⚠ **the ADDITION gap is still open and is a different gap.** Nothing on the
    render side spawns a visual for an overlay block with **no** authored
    counterpart; only `gate_solids` has one (`sync_lock_wall_visuals`). D69 never
    needed it — its block already had a visual — and the accounting below is
    otherwise still true.
  * ⚠ **`MaryOBlockLook::Hidden`'s stale `▢` mark is now doubly stale** — both
    halves are implemented. Left alone here rather than swept in with a fix.

  ### The original filing, kept

- **D69-ORIGINAL (superseded by the ✔ block above) ⛔ A DISCOVERED HIDDEN BLOCK
  PAYS OUT AND THEN HAS ITS PICTURE DELETED.**
  Found 2026-08-09 by the D67 worker, **observed not inferred**
  (`overlay.removed=["maryo_block:Hidden:AlwaysCoin:…"]`, `visual_present=false`
  after 60 frames). ⚠ **deliberately NOT folded into D67** — documented at
  `apply_block_art` and at the test instead.

  * **the mechanism**: discovery promotes a struck hidden block to a `Solid`
    through `FeatureEcsWorldOverlay`. That overlay's `removed_block_names`
    **despawns the original block's visual** — and the overlay's *added* blocks
    have **no render pass at all**; only `gate_solids` has one, in
    `sync_lock_wall_visuals`. ⇒ **the payout lands and the picture is deleted.**
  * ⭐ **this settles an inherited claim I explicitly refused to repeat.** The D65
    worker inferred that `dress_authored_blocks`' *"reveals itself by being
    used"* is false for **every Hidden block in both levels**. ✔ **the claim is
    true** — and ⛔ **"in both levels" is wrong: 1-1 authors no Hidden block at
    all. There is exactly ONE in the game.** ⇒ I was right to hold it as
    unverified, and the correction is a *population* error, which is the shape
    that keeps recurring today.
  * ⚠ **and a second stale mark, left alone deliberately**: `MaryOBlockLook::Hidden`
    carries *"▢ the classic block becomes a visible solid once struck, and that
    is NOT implemented"* — `contribute_discovered_hidden_blocks_to_overlay`
    **does** implement the collision half. ⇒ the `▢` overstates what is missing;
    only the render half is.

  ### ✔ SCOPED 2026-08-09 — THE RECONCILIATION IS ONE-DIRECTIONAL

  Read the render side rather than the overlay. **The gap is a missing half of a
  pair, not a missing feature:**

  ```text
  crates/ambition_render/src/rendering/world.rs
    :75    spawn_room_visuals          — draws `world.blocks`, at ROOM LOAD only
    :1335  sync_removed_block_visuals  — reconciles REMOVALS mid-run
           (no counterpart)            — ⛔ nothing reconciles ADDITIONS
  ```

  ⇒ **removals are handled while the room is live and additions are not.** A
  block the overlay adds mid-run is drawn only if the room happens to reload.
  ⭐ **fourteenth odd-one-out, and the cleanest kind**: a symmetric pair where one
  half was written and the other was not, so every consequence lands on one side.

  ⭐ **and the engine already knows HOW** — `sync_lock_wall_visuals` (`:1154`)
  draws blocks the overlay adds, for `gate_solids` **only**. ⇒ this is
  [[reference_count_the_adopters_not_the_capability]]: the capability exists with
  exactly one adopter, so *"the engine cannot draw an overlay-added block"* is
  false and the work is adoption, not invention.

  * **the fix**: an added-block counterpart to `sync_removed_block_visuals`,
    generalising what `sync_lock_wall_visuals` does for one kind. ⚠ **write the
    probe first**: strike 1-2's hidden block, run past the promotion, and assert a
    visual **exists** for the promoted block — red today, and the D67 worker
    already has the observation (`visual_present=false` after 60 frames).
  * ⚠ **check the removal side does not immediately undo it.** Promotion pushes
    the old name into `removed_block_names` *and* adds the new block; if both
    reconcilers run in the same frame the order decides the outcome. ⛔ **assert
    the order in a test rather than trusting system-set attributes** — a
    cross-schedule `.after` is silently vacuous here.
  * ⚠ **exactly ONE hidden block exists in the game**, so this fixes one visible
    thing and prevents a class. **Do not let its size argue against it** — the
    same gap silently drops every future overlay-added block.

- ✔✔ **D67 LANDED `4af02a1d0` — and the fix is BETTER than the one I sketched.**

  * ✔ **red, through the real pipeline** (`--features visible`):
    ```text
    the cavern's ?-block is not drawing the bonus plate
      left:  UuidHandle<Image>(97128bb1-…)            ← the default white quad
     right:  StrongHandle{ path: sprites/entities/bonus_block_tile.png }
    ```
    ⇒ **3 passed**; gate exit 0; `platformer2d_core` **360**, `render` **94**,
    `demo_mary_o` **135+10**, `demo_mary_o_app` **30** (+**40** under `visible`),
    `scripts/tests` **287**.
  * ⭐⭐ **they took (b) and improved it.** My sketch was *"let a placeholder block
    carry a `BoundEntitySprite`"* — which has the hazard I flagged **and a second
    one I did not know**: `refresh_entity_sprite_handles_on_game_assets_change`
    queries `BoundEntitySprite` **without requiring `BlockArt`**, so a spawn-time
    binding would let the next `GameAssets` reload paint the kind's texture over
    an honest placeholder. ⇒ instead **`apply_block_art` creates the binding when
    it takes over.** It only touches blocks with `BlockArt` — a game explicitly
    naming art — so **a block nobody dresses is unreachable by construction**,
    with a poison test for exactly that.
  * ⭐ **it also had to CLEAR the authored tint, and that is not tidiness**:
    `Sprite::color` **multiplies** into the image, so a transparent placeholder
    would bind the reveal texture and still draw nothing.
  * ⚠ **which forced a content deletion**: `dress_power_blocks`' `Brick =>
    BlockArt(SolidTile)` arm. Already redundant (`spawn_block` resolves every
    `BlockKind::Solid` through `block_tile_sprite`; the two-spawn-path fork its
    comment cites is gone) — **and once art could reach a painted block it would
    have stripped the cavern's stone off precisely the cammo brick, announcing
    the secret.** ⇒ verified by capture that 1-1's bricks are unchanged.

  ### ⛔⛔ MY PREDICTION ON THE INVISIBLE BRICK WAS WRONG — `d5f1d4621`

  I predicted it *"does trigger and simply cannot show it"* and told them to
  assert the payout rather than the picture. **They did, and the trigger is
  genuinely dead.** Her head stopped **exactly** at the underside (y=288 vs 288,
  so the block is there and solid to a rising head) and **no contact was emitted
  at all**.

  ⭐⭐ **the cause is a comment asserting a control flow the language does not
  have** (`movement/collision.rs`): `BonkOnly` held its own arm of an
  `if / else if` chain, under

  > *"It falls through to the ordinary face resolution below, which is what
  > produces the head contact the bonk reads."*

  ⛔ **`else if` chains do not fall through.** Taking that arm skipped the
  head-contact arm twelve lines down. The arm was never needed —
  `moving_toward_feet` is false for a rising head anyway — so **the fix is its
  deletion**, pinned with both terms observed and a *"must not become a floor"*
  poison. ⇒ **fourth [[reference_a_comment_describes_intent_not_the_code]] today**,
  and the most expensive: it hid a dead game mechanic behind a sentence that
  sounded like an explanation.

  ### ⚠ TWO CORRECTIONS TO MY SCOPING

  1. ⛔ **only TWO of the three reports collapse into this row.** *"No tile texture
     in 1-2"* does **not** — the colour sweep is doing exactly what it says.
     ⭐ **and they measured the tempting fix rather than trying it**: making
     `art_color` a *tint* over the kind's tile gives `solid_tile.png` mean RGB
     (0.265, 0.300, 0.388) × `UNDERGROUND_STONE` (0.20, 0.17, 0.28) ≈
     **(0.05, 0.05, 0.11) — near black.** ⇒ **the existing constants are FILL
     values, not multipliers**, so tint semantics needs Jon's colours re-picked.
     **That is a taste row, not this one.**
  2. ⛔ **`art_color` was never the reason a Hidden block cannot reveal itself** —
     `BonkOnly` resolves to **no kind art at all**, so it never had a binding
     regardless of paint. ⇒ **option (a), the content patch I offered as the
     cheap alternative, could never have fixed it.**

  ---

  **The diagnosis that preceded it:**

- **D67-ORIGINAL (superseded by the ✔✔ block above — marker stripped, text kept)
  ⛔⛔ ONE LINE IN `level_1_2()` OPTS EVERY BLOCK IN THE LEVEL OUT OF ART
  UPDATES, PERMANENTLY — and it explains THREE of Jon's observations at once.**
  Found 2026-08-09 by the D65 worker the moment 1-2 became photographable, and
  **verified by me from the source, not from the picture.**

  * ✔ **the mechanism, both halves checked**:
    ```rust
    // level_1_2():  the cavern is cut from ONE stone
    for block in &mut room.world.blocks { block.art_color = Some(UNDERGROUND_STONE); }

    // spawn_block (ambition_render/rendering/world.rs:725):
    //   "An authored placeholder colour wins over every art path: content has
    //    said this shape has no sprite yet"
    let sprite_key = if placeholder.is_some() { None } else { sprite_key };
    //   … and `BoundEntitySprite` is only inserted `if let Some(key) = sprite_key`

    // apply_block_art (:617) — the ONLY system that changes a block's picture mid-run:
    mut blocks: Query<(&BlockArt, &mut BoundEntitySprite, &mut Sprite), …>
    ```
    ⇒ **`art_color` ⇒ no `BoundEntitySprite` ⇒ invisible to `apply_block_art`
    forever.**
  * ⭐⭐ **and the code says the quiet part four lines above the bug**: *"[the art]
    a block wears changes mid-play (a bonus block becomes a used one), so a value
    read once at spawn could never be right for long."* **It knows.** The
    placeholder path then opts out of exactly that, silently.
  * ⇒ **three of Jon's rows collapse into this one line**:
    * *"There is also no tile texture in 1-2"* — everything is one flat plum
      rectangle, terrain and `?`-blocks alike. **Observed in the capture.**
    * *"Spent blocks in 1-2 don't look spent"* — they *cannot*. Nothing can
      repaint them.
    * *"In 1-2 jumping into the invisible brick from below doesn't seem to
      trigger it"* — ⚠ **a PREDICTION, not an observation**: it may be triggering
      and simply unable to show it. ⭐ **that is a falsifier worth running before
      anyone debugs the trigger** — check the coin count, not the picture.
  * ⭐ **twelfth [odd-one-out](../../dev/benchmark-candidates/the-odd-one-out-among-siblings-2026-08-09.md),
    and the author was half-aware**: `level_1_2()`'s own comment reads *"the
    colour goes on before the by-name dressing rather than instead of it:
    `dress_authored_blocks` then takes the pole back out again."* ⇒ **the POLE was
    excluded from the colour sweep and the four reactive `MaryOBlock`s were not.**
    One exclusion was written; the other was needed and missed.
  * ⚠ **and it makes a stale-comment claim elsewhere**: `dress_authored_blocks`
    says a Hidden block *"reveals itself by being used"*, which the D65 worker
    argues is false for **every** Hidden block in **either** level. ⛔ **not
    verified — that one is inferred from the same code path, and it should be
    observed before it is repeated.**
  * **the fix, and it is a choice**: either exclude the reactive blocks from the
    colour sweep the way the pole already is, or make `spawn_block` bind a
    `BoundEntitySprite` even for a placeholder so `apply_block_art` can still
    reach it. ⭐ **the second is the engine fix and the first is the content
    patch**; the second also fixes the next level that does this. ⚠ **write the
    probe first**: bonk a `?`-block in 1-2 and assert its art changes — red today.

- ✔ **D65 LANDED `00d8bbc59` — EVERY MARY-O ROOM ID NOW STARTS IN ITS OWN
  GEOMETRY, AND 1-2 HAS BEEN PHOTOGRAPHED.**

  * ✔ **red first**, and the assertion names the whole bug:
    ```text
    a session entering `mary_o_1_2` got another room's geometry
      left:  ("Ambition: mary o 1 1", [3328.0, 768.0], …)
      right: ("Ambition: mary o 1 2", [1920.0,  448.0], …)
    ```
    ⇒ green; `-p ambition_demo_mary_o` **135**, `-p ambition_demo_mary_o_app`
    **30**, gate exit 0, `scripts/tests` **287** (the ratchet did **not** redden).
  * ⭐ **the probe loops `provider::MARY_O_ROOM_IDS`** — one list, also used by the
    new `--room` validator — carries a **distinctness poison** so two
    indistinguishable rooms cannot make it vacuous, and asserts the entry room is
    in the set **exactly once**, which was the duplication trap I flagged.
  * ⭐⭐ **THE SAME FORK EXISTED ONE LAYER UP AND I DID NOT NAME IT.**
    `build_windowed_demo_app_with_home` inserted **no** `MaryOEntryRoom` at all
    *and* bound its assets from `mary_o_session_world()` — 1-1, hardcoded. ⇒
    **fixing only the provider would have left `capture_mary_o` still shooting
    1-1**, and I would have read that as the fix not working. Now
    `build_windowed_demo_app_entering(render, home_route, entry_room)`, a direct
    replacement across 2 call sites, answering both from one argument.
    ⚠ **checked, not assumed**: no other caller depended on the 1-1 fallback.
  * ⚠ **scope the worker flagged rather than hid**: the test-course room set no
    longer carries `level_1_2()` (an artifact of the concatenation) nor the
    authored links, silencing two pre-existing `room graph warning: unknown room`
    lines it printed on every run. ⇒ **accepted.**
  * ✔ **the capture is real, verified by `[world-event] room-loaded mary_o_1_2`
    in the log rather than by the pixels** — the standing trap is that a capture
    writes a file having rendered nothing. A bad `--room` exits 2 and writes
    nothing.
    ```sh
    capture_mary_o out.png 960x540 --room mary_o_1_2 --at 1410,340
    ```
    ⚠ **aiming caveat for the next person**: `--at` within ~150 px of 1-2's pole
    (x=1600) triggers `room-reset reasons=[Manual]` and the shot comes back at
    the spawn.
  * ✔✔ **AND IT CLOSES D61 VISUALLY** — 1-2's flagpole now shows *"white shaft,
    round white finial on top, red banner with a gold shield."* ⇒ **the fix I
    dispatched this morning is confirmed on screen, not just by its guard.**

- **D65-ORIGINAL (superseded by the ✔ block above — marker stripped, text kept)
  ⭐⭐ NO MARY-O ROOM BUT 1-1 CAN BE CAPTURED, AND THE SEAM THAT WOULD FIX
  IT IS A FORK WEARING A GENERAL DOC COMMENT.** Found 2026-08-09 by the D61
  worker, who spent **8m38s** discovering it while trying to take the "optional
  if cheap" screenshot I suggested. ⇒ ⛔ **my framing was wrong and the cost is
  the finding**: there is currently **no way to look at Mary-O 1-2**, which is
  the level D61, D64's spent-blocks row and D64's invisible-brick row are all
  about.

  * ⛔ **`capture_scene mary_o_1_2 …` panics**: *"start-room 'mary_o_1_2' did not
    match any room id/name"*. It composes `ambition_app`, whose registry holds
    **72 Ambition rooms and zero Mary-O rooms**. ⚠ this quietly invalidates
    [[reference_capture_scene_is_the_phone_proxy]] **for the Mary-O demo** — the
    proxy does not reach it at all.
  * ⛔ **`capture_mary_o` can see Mary-O and has no room selector.** It boots
    `MARY_O_GAMEPLAY_ROUTE` (1-1), and `--at` only teleports *within* the loaded
    room. ⇒ **two tools, and the union of them still cannot open 1-2.**
  * ⭐⭐ **the seam that looks like the fix is a FORK, and its doc comment hides
    that** — `provider::mary_o_session_world_entering(entry)`
    (`demo_mary_o/src/provider.rs:78`) is documented:

    > *"The same world, started in `entry`."*

    **but the body branches on `test_course` and otherwise builds 1-1.** So the
    `RoomSet` entry id receives `entry` while `geometry` and `metadata` receive
    **1-1's**. ⇒ **only two of the three room ids actually work through it**, and
    a caller passing the third gets a world that disagrees with itself rather
    than an error.
  * ⇒ ⭐ this is [[reference_a_comment_describes_intent_not_the_code]] again — the
    doc states the general contract the author *meant*, and the code implements
    two cases. ⚠ **and it is a silent partial**: the entry id is right, so the
    failure presents as *"the wrong room loaded"*, not as *"that room does not
    exist"*.
  * **the fix**: make `mary_o_session_world_entering` honour `entry` for
    `geometry` and `metadata` too — 1-2 is already `level_1_2()` and reachable —
    then give `capture_mary_o` a room argument. ⚠ **write the falsifier first**:
    a test that the world returned for `mary_o_1_2` has 1-2's geometry, which
    must fail today. ⛔ **do not "fix" it by registering Mary-O rooms into
    `ambition_app`** — that is a composition change to make a screenshot work.
  * ⚠ **NOT TOUCHED**: `provider.rs` was dirty from the D62/D63 worker at the
    time. Sequence it after them.

- ✔ **D62 LANDED `2707d86d9` — BOTH COIN PATHS NOW VOICE THE SAME CUE.**
  ⭐ **the mid-flight correction was the whole value of the row.** I filed this on
  an unverified arrow (*"Mary-O's coins go through `collect_ecs_pickups`"*), caught
  it in a self-audit, and sent it while the worker was building — **who confirmed
  they had not spotted it.** The half-fix would have landed reading closed.

  * ✔ **loose coins** — **28** `currency:1` pickups in `mary_o.ldtk`. The engine
    voices them; the declaration is the whole fix. My count of **9** declared
    specs was exact, confirmed by the red.
  * ⭐⭐ **the block path was NOT SILENT, which is better than either of us
    expected.** `bonk_power_blocks` was emitting **`SfxMessage::Hit` — the masonry
    thunk** — behind a comment reading *"there is no `Pickup` cue in the shared
    vocabulary yet"*. **True when written, stale now.** ⇒ popping a coin block
    played the *brick-breaking* sound. Another
    [[reference_a_comment_describes_intent_not_the_code]], and the third today.
  * ✔ both paths now name the same declared id and **both are pinned**; the spec
    table was extracted to `mary_o_sfx_specs()` — *"the shape Sanic already had,
    and the reason its guard was writable at all."*
  * ▢ ⚠ **A THIRD CREDIT PATH IS LEFT OPEN, DELIBERATELY, AND IT IS MINE TO
    RULE ON.** `refuse_a_weaker_form_pickup` (`powerups.rs:476`) also does
    `purse.add(COINS_PER_BLOCK)` and still emits `Hit`. It is the *"touched a
    redundant powerup, got coins instead"* case. ⇒ **ruling: leave it on `Hit`,
    and this is a product judgement not an oversight.** The coin ding says *"you
    collected a coin"*; here the player reached for a **powerup** and was
    consoled with currency, which is a different event and should not sound
    identical to picking one up. ⚠ **but say so to Jon** — he is the one who will
    hear it, and if he disagrees it is one line. ⭐ the worker was right to flag
    rather than widen scope on their own judgement.

- **D62-ORIGINAL (superseded by the ✔ block above — marker stripped, text kept)
  ⭐⭐ THE COIN SOUND IS EMITTED AND THEN SILENCED BY POLICY — MARY-O NEVER
  DECLARES IT.** Jon's *"In mary-o we need an SFX for when you collect coins."*
  Filed 2026-08-09, **no prior ledger row**. ⇒ **dispatch-ready: ~12 lines of data,
  no new sound, no engine change.**

  ### ⛔⛔ CORRECTION TO MY OWN ROW, 2026-08-09 — I ASSERTED AN UNVERIFIED ARROW

  This row said *"Mary-O's coins go through that loop"* as though it were one
  path. **Auditing my own claim while the worker was already building on it:**

  * ✔ **loose coins DO** — `mary_o.ldtk` authors `PickupSpawn` entities, **8 in
    1-1 and 6 in 1-2**, and those reach `collect_ecs_pickups`. The diagnosis holds
    here, and this is what the declaration fixes.
  * ⛔ **block coins DO NOT.** `powerups.rs:656` grants a `BlockPayout::Coins`
    with `purse.add(amount)` **directly** — no pickup is ever constructed, so
    **nothing emits the cue and no declaration can voice it.**

  ⇒ **two paths, and the fix as written covers one.** Jon's words are *"an SFX for
  when you collect coins"*, and popping a `?`-block for coins is collecting coins
  to a player. ⚠ **so this row could have landed reading CLOSED while he still
  heard nothing from a block.**

  ⭐ **this is the exact failure my own standing lesson names** —
  [[feedback_ask_the_tool_dont_model_it]]: *"write the FALSIFIER into the row
  BEFORE acting; four wrong causal chains in one day, all verified facts joined by
  an unverified arrow."* Both ends here were verified — the id exists and is
  emitted; the registry does not declare it — **and the arrow between them was
  assumed.** ⇒ the audit cost one grep and I only ran it because I went looking
  for my own unverified arrows on purpose. **Do that on purpose.**

  ⇒ the worker was told to either voice the block path from the same cue (if
  cheap, pinning both) **or stop at loose coins and SAY SO in the commit message**,
  so the gap gets its own row instead of vanishing into a half-fixed one.

  * ⛔ **the sound already exists and already fires.** `ids::WORLD_COIN_PICKUP`
    (`ambition_sfx/src/ids.rs:190` → `"world.coin.pickup"`) is emitted by the
    engine's own `collect_ecs_pickups` (`features/ecs/pickups.rs:230`) whenever
    **any** currency pickup is collected. Mary-O's coins go through that loop.
  * ⭐ **so why is it silent? — a DECLARATION gate, and Mary-O's own file says so**
    (`demo_mary_o/src/provider.rs:243`):
    > *"under provider-relative audio a session only plays cues its fragment
    > declares — an undeclared `player.jump` is gated to silence."*

    ⇒ **the emission is not missing; the authorisation is.** This is the failure
    mode that reads as *"we never wrote that sound"* and is really *"we wrote it,
    play it, and then throw it away."*
  * ✔ **the measurement.** Mary-O's `SfxRegistry` declares **9 specs**: cues
    `Jump`, `Hit`, `Pogo`, plus 6 ids (5 powerup transitions + `PIPE_WARP_SFX`).
    **`world.coin.pickup` is not among them.**
  * ⭐⭐ **and SANIC DECLARES IT, WITH A TEST** — `demo_sanic/src/lib.rs:231`:
    ```rust
    /// `collect_ecs_pickups` loop emits `ids::WORLD_COIN_PICKUP` on pickup — voicing
    /// … kept in sync with `ambition_platformer2d::sfx::ids::WORLD_COIN_PICKUP` by a test.
    pub const SFX_RING: &str = "world.coin.pickup";
    ```
    ⇒ **eleventh [odd-one-out](../../dev/benchmark-candidates/the-odd-one-out-among-siblings-2026-08-09.md)**:
    two demos collect coins through the *same engine loop*; one authorises the
    cue and pins it with a test, the other does not.
  * **the fix**: one `SfxSpec` with `id: Some("world.coin.pickup")` in Mary-O's
    registry — a short bright blip, the classic coin ping. ⚠ **copy Sanic's guard
    too** (`demo_sanic/src/tests.rs:1330`, *"the demo authorises + voices exactly
    that"*): it asserts the declared id equals the constant the engine emits, so
    the two cannot drift. **Without that test the fix is a string literal nobody
    is watching.**

- ✔ **D63 LANDED `b24651aed` — SMALL MARY-O CANNOT HEADBUTT BRICKS APART.**

  * ✔ **the predicate, and I was right not to guess it**: `worn_form_rank(worn)
    >= 1`, exposed as `powerups::is_small`. Three existing readers of the ladder
    already agreed, so nothing was invented:

    | form | wears | rank |
    |---|---|---|
    | small | neither row | 0 |
    | tall | `STAR_WAND_ID` | 1 |
    | fire | `CINDER_BEACON_ID` | 2 |

  * ⭐⭐ **asking the RANK rather than the ids is load-bearing**, and this is the
    trap my brief would have walked into if I had named a predicate: **the beacon
    is worn ALONE at the top** — it downgrades *into* the wand rather than
    stacking — so `wears(STAR_WAND_ID)` would have **muted fire Mary-O**. ⇒ a
    fire-form player would silently lose the ability to break bricks, and the
    obvious test (small cannot, tall can) would never have caught it.
  * ⭐⭐ **THREE EXISTING BRICK TESTS WOULD HAVE GONE VACUOUSLY GREEN.** They
    spawned bare bodies with no form, so the new gate turned them into tests of
    *"a formless body cannot break a brick"* — passing for a reason unrelated to
    what they assert. They were re-armed with the tall form.
    ⚠ **the sharpest is `a_head_bonk_on_a_non_brick_breaks_nothing`** — it *"would
    have passed for the wrong reason forever."*
    ⇒ ⭐ **this is the cost of a new gate that nobody bills**: adding a
    precondition silently converts every existing test that lacks it into a
    tautology. **When you add a gate, list the tests that now satisfy it by
    accident** — [[feedback_a_guard_that_pins_the_fix_defends_the_gap]].
  * ✔ **the fix itself was poisoned to check the other half was real** — swapping
    `is_small` to demand the wand turned the fire arm red, naming exactly that
    failure. **Both terms observed**, not one asserted and one assumed.
  * ⚠ correction to my scoping: 1-1 authors **two** breakable bricks, not three,
    so the test strikes one brick in three fresh apps — **stronger**, because form
    is then the only variable.

- **D63-ORIGINAL (superseded by the ✔ block above — marker stripped, text kept)
  ⭐ SMALL MARY-O BREAKS BRICKS BECAUSE `break_bricks` NEVER ASKS WHAT FORM
  SHE IS IN.** Jon's *"small mary-o should not be able to headbutt bricks to break
  them. Only tall or fire should be able to."* Filed 2026-08-09, **no prior
  ledger row**.

  * ✔ **verified by reading every gate in the function** (`demo_mary_o/src/bricks.rs:99`).
    `break_bricks` breaks a block when **two** conditions hold:
    1. `contact.kind == ContactKind::Head`, and
    2. the block is a `Brick` whose contents `breaks_when_empty()`.

    ⇒ **there is no third condition.** No `WornEquipment`, no form, no size — the
    parameter list does not even take it.
  * ⭐ **and the sibling in the same feature DOES take it**: `reward_for(contents,
    worn: Option<&WornEquipment>)` (`powerups.rs:816`) already threads her form
    through to decide what a block pays. ⇒ **the value is in hand one file over**;
    this is a missing argument, not a missing concept.
  * **the fix**: give `break_bricks` the same `WornEquipment` read and require
    tall-or-fire. ⚠ **write it red first** — a test that small-form head contact
    leaves the brick intact *and* that tall-form head contact still breaks it.
    ⛔ **one-sided is the trap here**: asserting only "small cannot break" passes
    trivially if the system stops breaking anything at all.
  * ✔ **arrow audited 2026-08-09: `break_bricks` is the ONLY path.**
    `broken.mark(..)` has exactly one production caller (`bricks.rs:161`); every
    other mention is the type, the resource registration, or a test. ⇒ **no second
    system already gates this**, so the missing form check really is the whole
    reason small Mary-O breaks bricks. ⚠ checked because the same audit on D62
    the same hour found an arrow that did **not** hold.

- ▢ **D64 MARY-O / PRESENTATION RESIDUE FROM JON'S OBSERVATION SWEEP.**
  The multi-coin mechanic, flagpole, and several originally indexed observations are implemented.
  Remaining work is:
  * add the requested coin-pop VFX for a multi-coin question block; the direct purse credit and SFX
    already exist and should remain authoritative;
  * reproduce the 1-2 reports that spent blocks do not look spent / an invisible block does not
    trigger from below. Source and tests currently show the tile data, lowering, head-contact path,
    contents, and spent dressing are all present, so do not rewrite those mechanisms without a
    reproduction;
  * the proposed reference-frame gravity-camera mode is a product feature decision and now lives in
    [`awaiting-maintainer-decision.md`](awaiting-maintainer-decision.md), not this engineering row.
  Historical measurements are archived in
  [`../archive/planning-superseded/2026-08-13/queue-pruned-sections.md`](../archive/planning-superseded/2026-08-13/queue-pruned-sections.md).

- ▢ **D47 ⭐⭐ THE FUNCTION THAT KNOWS "THE DRAWING" FROM "THE BODY" HAS EXACTLY
  ONE CONSUMER, AND IT IS THE PLAYER ROBOT.** Filed 2026-08-09 out of D44's
  landing report. **This, not the quad, is where Jon's five sizing complaints
  live.**

  * **the distinction already exists, stated better than I would state it** —
    `character_sprites/posed_body.rs:108`:
    > *"the honest answer is no for a measured alpha bbox: it is the extent of the
    > drawing, hat and outstretched arms included, and using it as a body is how a
    > collision box ends up 1.28× the character inside it. So this refuses rather
    > than returning a number that looks usable."*

    `authored_body_pixel_size` returns `None` unless the sheet sets
    `BodyMetrics::authored_body` — the sheet's own claim that somebody drew the
    box rather than measuring the alpha.
  * ⛔ **and it is asked by ONE caller in the whole workspace**:
    `game/ambition_content/src/player_robot_lineage.rs` (4 call sites, all the
    player lineage). ⇒ **every other character's body is the alpha bbox** — hat,
    outstretched arms, and for the Imperfect Cellular Automaton its sparkles. The
    Hall capture shows exactly that: a small figure inside a much larger green
    box.
  * ⭐⭐ **and D44's quad route made the picture agree with that box**, which is
    precisely why it *"fixes none of Jon's five complaints"*. The drawing now
    fills a rectangle that was always too big. **Both changes are right; only
    together are they the fix.**

  ### ⛔⛔ "2 OF 190 DECLARE `authored_body`" IS WRONG. IT IS 34.

  Counted over all 190 shipped sheets (`find … -name '*_spritesheet.ron'`,
  excluding the `potato`/`0_5x`/`0_25x` variant trees — ⚠ `grep -r` would have
  missed these, the art tree is gitignored):

  ```text
  authored_body: true    34
  authored_body: false    0      (the field is emitted only when true)
  field absent          156
  ```

  ⚠ **the "2" came from Jon's observations row, was repeated in D44's landing
  report, and was never measured by either.** Third inherited number this run to
  survive into a plan unchecked. It matters here more than usual: *2 of 190* says
  "nobody authored one, this is a 188-file content project", while *34 of 190*
  says **a third of the cast is already waiting for a consumer**.

  ### ⛔ AND "ADOPT THE FLAG" IS NOT THE FIX EITHER — D44's landing report, `afc36b390`

  Two facts from the worker that reshape this row before it starts, both
  measured rather than reasoned:

  * ⭐⭐ **`authored_body` is GENERATOR-EMITTED, never hand-written.** Two
    producers — `authoring/sheet.py:266` (a rig declaring `body_inset()`) and
    `authoring/sheet_build.py:897` (a target passing `body_metrics_fn`) — and
    `manifest_ron.py:113` only ever writes `true`. Absence means *"measured alpha
    bbox"*. **13 files declare `body_inset`.** ⇒ **the work is a rig-config +
    regen pass, not a data pass over 156 `.ron` files**, and editing the sheets
    directly would be overwritten by the next `./scripts/regen/sprites.sh`.
  * ⛔⛔ **a sheet can set the flag and still be wrong.**
    `imperfect_cellular_automaton` **already sets `authored_body`** and still
    publishes a **199×232 body in a 256×256 frame**, because its inset does not
    exclude its sparkles. Compare `vera_ruin` at **94×157 in 192×192**. ⇒ the
    defect is *"the inset values are too loose for effect-heavy rigs"*, not
    *"nobody adopted the flag"* — and the ICA is one of the two characters the
    Hall capture shows as a small figure in a big box, so it is precisely the
    case the naive reading would have declared already-fixed.

  ### The fix, restated

  1. **engine** — the authored-vs-measured refusal stops being a player-lineage
     courtesy and becomes the general rule: a body's collision box prefers the
     sheet's authored rect wherever one exists. Necessary, and **not sufficient**
     — the ICA proves it.
  2. **rigs** — the 13 `body_inset` declarations get audited against what they
     actually exclude, and the effect-heavy rigs get insets that mean *the
     character* rather than *the drawing*. Then regen. ⚠ Jon's standing call on
     this class: *"this should probably be something done in data by the sprite
     renderer, not in code."* That is this half, and it is where the row's weight
     is.

  ### ✔✔ FALSIFIER RUN 2026-08-09 — AND IT REVERSES THE ROW'S EMPHASIS

  Computed declared-body-area ÷ frame-area over all **184 sheets that publish a
  bbox** (34 of them `authored_body: true`). **Of the characters Jon named, all
  but one are in the 156 that declare nothing:**

  ```text
  character                       fill    w        h      authored_body
  player_robot_v3                 10.3%   25.4%   40.6%   TRUE
  patent_clerk                    25.1%   34.5%   72.6%   false
  pirate_cutlass_viper            33.1%   47.7%   69.3%   false
  solid_snake                     35.0%   91.4%   38.3%   false   ← long+flat
  vera_ruin                       40.0%   49.0%   81.8%   TRUE
  pirate_heavy_iron_mary          42.9%   65.5%   65.5%   false
  pirate_heavy_broadside_bess     43.3%   63.3%   68.4%   false
  pirate_lookout / navigator /
    quartermaster / raider        56.3%   57.3%   98.2%   false   (identical — one rig)
  ai_slop                         68.3%   94.8%   72.0%   false   ← very wide
  imperfect_cellular_automaton    70.4%   77.7%   90.6%   TRUE
  pirate_heavy_axe                87.4%   97.7%   89.5%   false
  ```

  ⇒ ⛔ **half 1 — "make the authored rect the general rule" — fixes almost NONE
  of Jon's complaints**, because the pirates, `ai_slop`, `solid_snake` and the
  patent clerk declare no authored body. **D47 is a rig/data project wearing an
  engine hat**, which is exactly what the falsifier was written to find out.
  ⭐ that also matches Jon's own instinct: *"this should probably be something
  done in data by the sprite renderer, not in code."*

  ⚠ **what this metric does and does not say.** It is bbox ÷ frame, so a small
  character in a deliberately padded frame is legitimately low — **it is not a
  looseness score.** ✔ but it reproduces the two data points the D44 worker
  measured independently (ICA 199×232 in 256×256 → 70.4%; `vera_ruin` 94×157 in
  192×192 → 40.0%), so the numbers are sound for the question actually asked:
  **who has declared one at all.**

  ⭐ **two shapes fall out that the sizing arithmetic used to mangle**, both now
  visible as numbers: `solid_snake` at **w 91.4% / h 38.3%** is the long flat
  animal D44's old *"height off the collision box's LARGER axis"* drew as tall as
  it is wide, and `ai_slop` at **w 94.8%** is the other extreme. Those two are
  the shapes to re-check first once rig insets exist.
  ⚠ the four `pirate_lookout / navigator / quartermaster / raider` sheets are
  **byte-identical in geometry (56.3% / 57.3% / 98.2%)** — one rig, four
  characters. ⇒ **fixing that rig fixes four of Jon's cover pirates at once**,
  which is the cheapest entry point on this row.
  * ⚠ do NOT infer this makes the player's box right — the player's 30×48 is a
    TUNED movement body, not sheet-derived, which is a third authority again and
    wants naming before anyone "unifies" the two.
  * ⚠ **⛔ 13 rig files and a regen means this row touches the art pipeline**, so
    it inherits every trap there: `scripts/regen/sprites.sh` also owns the ultrapack
    atlases and the quality variants, and a partial regen leaves the tiers
    disagreeing with the base sheets.

- ✔ **D71 — ONE HALF FIXED 2026-08-10: THE DOOR MAKES A SOUND AGAIN.**

  ⭐⭐ the diagnosis below is confirmed and the first concrete casualty of the
  bypass is repaired. `detect_room_transition_system` resolves the zone's cue
  (`world.door.open` for a Door, `world.portal.enter` for an EdgeExit or Walk),
  hands it to `RoomTransitionRequested` — and on the deferred branch **dropped it
  on the floor one branch later.** The eager commit plays it; the deferred
  committer emitted nothing at all. So on the shipped rollback binary **every
  door and every portal was silent**, and no test noticed because the cue was
  computed exactly as before and simply never read.

  The cue rides `LifecycleIntent::Transition` now — it has to, because the commit
  runs on a confirmed frame long after the zone that named it is out of reach and
  the intent names its room by id rather than carrying a zone. Snapshot codec
  extended to match (rollback state, so it encodes). ⚠ emitted straight into
  `Messages<OwnedSfxMessage>` with ownership resolved exactly as `SfxWriter::write`
  does — an unowned cue is REFUSED by playback whenever an owned context is
  active, so a world-level emit that skipped that would be silently dropped
  instead of loud. Safe to emit directly rather than through the external-effect
  journal because the committer runs only on a confirmed frame and can never
  re-run speculatively.

  ▢ **what is still bypassed**: the transaction itself.
  `begin_room_transition_load_system` keys on `RoomTransitionRequested`, so the
  deferred path opens no load transaction — no asset preflight, no
  `ROOM_READY_BARRIER`, no `GameMode` gate, no superseding of an in-flight load.
  The deferred committer instead calls `RoomConstructionPlan::prepare` +
  `apply_to_world` **synchronously**, which is why rooms change at all. Closing
  this is not "also write the message": it means opening the readiness
  transaction at DETECTION (host-side, off the rollback timeline, keyed on the
  recorded intent — which is earliest-sticky, so re-detection cannot duplicate)
  and making the commit WAIT on the barrier instead of preparing inline. That is
  a real integration, not a line.

- ✔ **D71 ANSWERED 2026-08-09 — THE BYPASS HAS A NAME, AND IT IS THE HOST.**
  ⛔ **still reproducible; not stale.** The census ran, and the two hosts do not
  merely differ in timing — **they change rooms by two different routes.**

  ```text
  cargo test -p ambition_app --test app_it --features rl_sim -- --ignored d71_
  fixed-tick host (ConfirmedFrameBoundary absent):  11 room changes, 11 transactions,  0 deferred intents
  ROLLBACK host   (ConfirmedFrameBoundary present): 24 room changes,  0 transactions, 24 deferred intents
  shipped app: ConfirmedFrameBoundary present=true, LocalSyncTest / LocalMaintainer
  ```

  ⭐ **1:1 both times, and never the same one.** Every room change is claimed by
  exactly one route, and which route it is turns *only* on whether a rollback
  host is composed — and the shipped desktop binary composes one, so **Jon's play
  took the transaction-free route for every room change he made.**

  * ⭐ **the route**: `detect_room_transition_system`
    (`actor_monolith/src/world/rooms/systems.rs:163`) forks on
    `Option<Res<ConfirmedFrameBoundary>>`. With a rollback host present it records
    a `PendingLifecycleCommit` and **returns before writing
    `RoomTransitionRequested`**. ⇒ `begin_room_transition_load_system` never runs,
    no `ambition_load` barrier opens, `GameMode::RoomTransition` is never
    requested, **and no cover is ever presented.** The room is rebuilt instead by
    `runtime::lifecycle_commit::commit_confirmed_lifecycle` in `PreUpdate`,
    outside `GgrsSchedule`.
  * ✔ **verified by two routes that are not the sampled resource** (the resource
    alone is weak — the census samples once per `sim.step()`):
    * the **unconditional** `[world-event] room-transition begin seq=N` line
      prints 11× under the fixed-tick host and **0×** under the rollback host,
      while `room-loaded` prints under both;
    * temporary markers at **all five** room-construction call sites: the rollback
      host's 24 loads came through **none** of the four transactional sites. They
      came through `RoomConstructionPlan::apply_to_world`, whose only caller is
      `commit_confirmed_lifecycle`.
  * ⚠ **the deferral itself is deliberate and correct** — `session::lifecycle_commit`'s
    module docs say so plainly: the load machine is not rollback-registered, so it
    must not run on a speculative frame. ⛔ **the defect is that the deferred route
    inherited none of the transaction's OBLIGATIONS.** It is a
    fork whose own docstring says it *"mirrors
    `commit_room_transition_geometry` + `apply_room_transition_resets`"* — and a
    "mirrors X" comment is a citation, so what it did **not** copy is invisible:
    the cover, the barrier, the `GameMode` request, and the load telemetry.
  * ▢ **the fix is a composition question and deliberately NOT in the census
    commit.** The shape: the deferred commit must open and retire the same
    transaction the eager one does (or the cover must key off something both
    routes publish). ⭐ **whichever is chosen, the guard is a test that asserts a
    room change opens a cover UNDER THE ROLLBACK HOST** — the eager-host tests
    that exist today pass while the shipped binary has no cover at all, which is
    exactly how this survived.
  * ⚠ **the row's original framing is preserved below** and was right about the
    direction ("start from *a room changed* and ask which had a transaction") and
    wrong only in expecting the answer to be stale.

  ### The original filing, kept

  * ✔ **the measurement**, from `target/profiles/desktop-timeline-run-20260808T185222Z/`:
    ```text
    room transition … BEGIN     seq=1   mary_o_1_1 -> mary_o_1_2   t=257.006s (retired 257.040)
    …and that is the ONLY one in 290 seconds of play.
    music target flips              t=208.92s  and  t=210.69s      ← rooms plainly changed
    ```
    ⭐ `mint_sequence` is a **monotonic global with no reset anywhere in the
    workspace**, so `seq=1` is not a per-room counter — **one transaction is one
    transaction for the whole process.**
  * ⇒ ⛔ **either Ambition had a room-change route that BYPASSED the covered path
    at `40d896593`, or Jon changed rooms by some means other than an authored
    door.** The worker could not settle which from the log and did not guess.
  * ✔ **and today's door path works** — `app_it` crossings through
    `proving_grounds_hub_door` produce covered transitions. ⇒ **whatever the
    bypass was, it is not the door.**

  ### ⭐⭐ WHY THIS MATTERS MORE THAN D46's FIX DID

  D46 spent its whole life on *"the cover retires too eagerly"*. **No warning in
  that capture falls inside the one cover window that existed** (nearest bursts
  243.4 s and 273.2 s, cover 257.006–257.040). ⇒ **not one of the 190 magenta
  stand-ins was drawn while a cover existed**, so cover *timing* — eager snapshot
  or otherwise — **cannot be what Jon saw.**

  ⇒ ⭐ **an uncovered room change explains his report completely and needs no
  other defect.** D46's row named this alternative itself and then spent its
  effort on the other branch.

  * ⚠ **the falsifier, and it is cheap**: instrument every room change and count
    how many open a transaction. ⛔ **do not start from the transition code** —
    start from the *"rooms changed"* signal (the music-target flip is one) and ask
    which of those had a transaction. **That is the direction the log evidence
    already points, and the opposite of how D46 was worked.**
  * ⚠ **the capture is from `40d896593` and much has landed since.** ⇒ **re-run a
    session before assuming the bypass still exists**; this row may already be
    stale, and that would be the best outcome.

- ✔ **D46 THE SPLIT LANDED `3fc23259f` (2026-08-09) — the cover waits on undrawn
  VIEWS now, and the diagnostic got its grace period.** ▢ **but the row's whole
  hypothesis was refuted by the capture — see D71.**

  * ✔ `draw_unclaimed_feature_views` publishes `UnclaimedFeatureViews` from the
    set it already built, **without subtracting `already_standing`** (a stand-in
    is not art). The cover reads that resource; the diagnostic waits **5
    consecutive unclaimed frames** before drawing a box or warning.
  * ✔✔ **the ordering trap I flagged was real and is pinned.** Both ends are in
    `Update`; the test builds the **real** `build_visible_app` graph and asserts
    the edge `PresentationVisualSync → RoomTransitionCoverSet` **plus** that each
    set has members *in `Update`*. ⭐ **that second clause is the anti-vacuity
    guard** — a name-only edge passes the first assertion and fails the second.
  * ✔✔ **all three new guards falsified, not trusted**: subtract
    `already_standing` → red; grace period `N=1` → red; delete the `configure_sets`
    edge → red.
  * ⭐ **two things a RESOURCE needs that an entity did not**, both found by the
    worker: `SessionWorldRef` is a `Single`, so the census was **silently skipped**
    whenever no session world existed; and session-scoped placeholder *entities*
    were swept by the lifecycle while a resource is not — **a stale non-zero
    census is a cover holding a black screen for its full 8 s deadline**, the
    exact 2026-08-05 regression. Publishing is now unconditional and a dormant
    session forgets the census.

  ### ⛔⛔ AND THE ROW'S CENTRAL HYPOTHESIS IS DEAD — read D71

  Every version of this row hunted **cover retirement**. The capture says **no
  warning in 290 s falls inside the one cover window that existed**, so **not one
  of the 190 stand-ins was drawn while a cover was up.** ⇒ **cover timing cannot
  be what Jon saw**, and the alternative this row named in passing — *a room
  change that gets no cover at all* — is where the evidence points. **Filed as
  D71.**

  ⇒ ⭐ **the honest accounting of what this fix bought**:
  * ✔ *"the cover retires over undrawn art"* is **structurally eliminated**, and
    pinned by a test. **That claim is solid.**
  * ⛔ *"Jon's flash is fixed"* is **NOT claimed** — and probably false, because
    the cover was never up.
  * ✔ **the instrument is honest now** regardless: a warning means a view stayed
    unclaimed 5+ frames, and it names the streak. ⇒ the 190-false-positives
    problem is gone whatever D71 turns out to be.
  * ⚠ **`N = 5` is UNMEASURED** and documented as such on the constant — the log
    warns once per id, so it cannot say how long a transient lasts. ⭐ **the cost
    of a wrong N is now bounded**, because the cover no longer depends on it.

  ---

  **The investigation, kept — including the hypothesis the capture killed:**

- **D46-ORIGINAL (superseded by the ✔ block above — marker stripped, text kept)
  THE UNCLAIMED-BODY WARNING CANNOT SAY WHETHER ITS BOX IS STILL ON
  SCREEN**, and that is why 190 of them read as a disaster and meant almost
  nothing. Filed 2026-08-09 while chasing D39's "third mechanism", which this
  row **retires**.
  * **the mechanism, read off one function** —
    `ambition_render/src/rendering/features.rs:draw_unclaimed_feature_views`:
    ```rust
    // :247  the real thing arrived: retire the stand-in
    for (entity, id) in &stand_ins {
        if known.contains(id) { commands.entity(*entity).try_despawn(); }
    }
    // :269  …and only then, for anything still unclaimed:
    warn!("no render family claimed `{id}` …; drawing the unclaimed-body placeholder");
    ```
    ⇒ **the warning does not report a magenta box on screen. It reports a
    magenta box being SPAWNED**, and the same function despawns it the moment a
    render family claims the id. A body that is claimed one frame late warns
    exactly like a body that is never claimed at all.
  * ⭐ **and it warns ONCE per id**, because `already_standing` suppresses the
    repeat while a placeholder exists. So *190 warnings* means *190 distinct ids
    each spent at least one frame unclaimed* — not 190 boxes, and not one box
    seen 190 times. ⛔ **I read that number as a severity and it is a
    cardinality.**
  * ✔ **the transient case is already documented as normal** by a neighbour:
    `features/ecs/damage_drops.rs:112` describes *"`no render family claimed
    \`coin:EnemySpawn-…\`` per transition"* as the expected shape. Somebody
    already knew; the log line just cannot say it.
  * ⇒ **this retires D39's open "third mechanism".** `NpcSpawn` is authored
    correctly (all 163 carry a `character_id`, counted by parsing the worlds),
    its display name resolves, and **no catalog display name is duplicated
    (checked: 125 entries, 0 duplicates)** — so `id_for_display_name` cannot be
    ambiguous either. Nothing is broken; the warnings are the race resolving.
    ⛔ **do not open a bug row for it.**
  ### ⭐⭐ AND JON SEES THE TRANSIENT ONE, SO IT IS NOT ONLY A LOG PROBLEM

  I filed the above as "the harmless case is merely noisy". **It is not
  harmless** — it is one of the sixteen observations in `7ace7b5e7`:

  > *"Changing rooms flashes magenta squares for a brief moment. We need to have
  > cleaner transitions between rooms than that."*

  ⛔ **and that batch is DESKTOP** (its neighbours quote the title menu at 60 FPS
  against ambition's 140, and smash VFX placement). Desktop is where I measured
  **0 `cover gave up waiting`** across a real 290 s session. ⇒ **the cover never
  expired and the player still saw magenta.** Those two facts cannot both be
  innocent, and together they say the cover is not the whole story.

  * ⚠ **this is a REGRESSION or an incomplete fix, not a new bug.** The cover's
    settle condition exists *because Jon reported this exact symptom on
    2026-07-30*, quoted verbatim in the code at
    `room_transition_presentation.rs:390`: *"It flashes squares, which then flash
    the placeholder sprite, and then it flashes to the characters."* He is
    reporting it again nine days later.
  * ⭐ **the hypothesis, and it explains 0-expiries-plus-a-flash exactly**: the
    retirement condition at `:400` is `unclaimed.iter().count() == 0` — a
    **snapshot**, not a settled state. The cover retires the first instant
    nothing is unclaimed. A feature view published one flush LATER spawns a fresh
    placeholder with no cover left to hide it. The code's own comment concedes
    the timing shape: *"render families spawn through `Commands`, and a room with
    many actors takes several flushes to draw."* Nothing makes the count
    monotonic, so "zero right now" is not "done".
  * ⚠ **FALSIFIER — run it before anyone changes the deadline or the cover.**
    Count `UnclaimedBodyPlaceholder` **spawns that occur after the cover is
    retired**, on a real room change. Non-zero ⇒ this hypothesis holds and the
    cover is being retired too eagerly. Zero ⇒ it is wrong, the flash comes from
    somewhere else (a transition class that gets **no** cover at all — the module
    doc says *"every VISIBLE transition gets an opaque cover"*, so ask which ones
    are not visible), and this paragraph should be struck rather than tuned
    around. ⛔ **do not lengthen `presentation_settle_deadline`** — a longer
    deadline cannot fix a cover that has already legitimately retired.

  ### ⛔⛔ THE FALSIFIER RAN. IT CANNOT SEE THE PHENOMENON — 2026-08-09

  Built as `room_boundary_unclaimed_views::
  no_magenta_placeholder_is_visible_while_the_cover_is_down`, landed
  **`#[ignore]`d** with the reason in its doc comment. **Three attempts, all
  refuted:**

  | attempt | placeholders ever drawn |
  |---|---|
  | cross hub → `proving_grounds` | **0** |
  | cross hub → **Hall of Characters** (129 bodies, the heaviest room) | **0** |
  | …with the harness's 4 ms asset sleep removed | **0** |

  ⭐⭐ **THE FIRST RUN PASSED, AND MEANT NOTHING.** The condition it checks
  (`placeholders && !cover`) was never once evaluated with placeholders present.
  Caught only because the test asserts its own non-vacuity — `saw_cover_up` (the
  cover WAS observed, so the name handle works) and `saw_placeholders`
  (⛔ **false, every time**). ⇒ **a falsifier needs a falsifier**, and the cheap
  one is *"did the thing I am testing ever get a chance to fail?"*
  [[reference_a_check_that_cannot_fail]].

  ⇒ **the flash is not reproducible in `build_visible_app` on desktop.** The 190
  `no render family claimed` lines in the 290 s profile come from somewhere this
  composition does not reach — a different route, or the windowed app's own
  presentation wiring ([[reference_app_only_presentation_class]]). ⛔ **do not
  tune `presentation_settle_deadline` on the strength of this row**; nothing here
  has observed the phenomenon.
  ⚠ the sleep hypothesis was the third to die. The harness's `step()` sleeps 4 ms
  *"so the asset threads make progress"*, which looked exactly like an instrument
  whose noise floor sits above its signal. Removing it changed nothing.
  ⭐ **the test is ready to un-ignore** the moment a crossing produces the
  transient — the sampling, the assertion and both guards are correct.

  ### ⭐ ONE CHANGE PLAUSIBLY FIXES ALL THREE

  **Do not draw the placeholder until an id has been unclaimed for N consecutive
  frames.** It is a grace period on the *diagnosis*, not on the cover:

  1. a race that resolves in a flush or two never draws a box at all ⇒ **Jon's
     flash goes away even when it happens after the cover has retired**, which is
     the case a longer deadline cannot reach;
  2. the warning then fires only for genuinely-stuck bodies ⇒ **the instrument
     becomes honest**, same defect class as D36;
  3. `unsettled` at `:400` stops counting transients, so the cover's own
     retirement condition gets less jumpy for free.

  ⛔ **it must not become a way to hide a permanent failure** — the whole point
  of the magenta box is that *"a feature that NO family will ever claim is a real
  bug this diagnostic exists to show"*. N stays small, and D39's goblins must
  still go magenta. **That is the guard: a poison test with a body nothing will
  ever claim, asserting the box appears anyway.**

  ### ✔ THE HARNESS ALREADY EXISTS — do not build one

  `game/ambition_app/tests/room_boundary_unclaimed_views.rs` (in `app_it`, green
  today) already drives `build_visible_app` into gameplay and **crosses two
  authored doors**, because it *"needs a real room-unload path, so it pays for
  the real app"*. It asserts the stand-in population settles back to the
  destination room's own clean baseline and that no unclaimed id survives a
  crossing.

  ⇒ ⭐ **the PERMANENT case is already defended, and that is why D46 is only
  about the transient one.** Two prior instances are written up in its header —
  Jon's 8-second black screen (`dd73a3087`, drops missing `RoomScopedEntity`) and
  unclaimed coins in the room they fell in (fixed 2026-08-08 by stamping
  `SpawnOrigin`) — both of which were *permanent* stand-ins. Neither would
  produce a brief flash.

  ⇒ **extend that file rather than writing a new one**: it already has the
  transition, the timing control (`TimeUpdateStrategy`) and the placeholder
  query. What it does not yet sample is **whether a placeholder is ever drawn at
  a moment when no cover exists** — which is exactly the falsifier above, and is
  a few lines inside a test that already pays the expensive part.
  ⚠ its header states the design rule to respect: *"this is deliberately NOT a
  test that a coin dies with its room"* — it defends the OBSERVABLE, not the two
  spawn sites a commit happened to touch. A transient-flash assertion should be
  written the same way: no magenta is visible to the player during a crossing,
  whatever caused it.
  * ⚠ **THE FALSIFIER, and write the result here before acting**: if the
    placeholder is in fact NOT retired — if `known` never gains the id for a
    correctly-authored NpcSpawn — then every one of those 190 is a live magenta
    box and this row is wrong in the reassuring direction. **Check by entity, not
    by log**: count `UNCLAIMED body placeholder:` named entities alive at a
    steady state in a real room, after transitions have settled. Zero confirms
    this row; a persistent set names real defects and D39's third mechanism is
    back.
    ⛔ this is the fourth causal chain today of the shape *verified facts joined
    by an unverified arrow*, so the arrow — "spawned ⇒ retired" — is the thing to
    measure, not the thing to assume.
    [[feedback_ask_the_tool_dont_model_it]].
  * ⚠ pairs with D36 (`SheetRegistry` collision warning, same defect class) and
    with the standing lesson that **a WARN on the happy path is not a symptom**
    — 190 of these against 0 `cover gave up waiting` in one 290 s session, and
    both Jon and I read the 190 as the bug.

  ### ⛔⛔ THE GRACE-PERIOD PROPOSAL BELOW IS REFUTED — and the refutation is the
  ### best thing in this row. Retracted in place 2026-08-09, an hour after filing.

  I proposed delaying the stand-in by N frames so a one-flush ordering gap never
  draws a magenta box. **It would have made Jon's flash WORSE.** I never asked who
  else reads the thing I was proposing to delay:

  ```rust
  // room_transition_presentation.rs:259
  unclaimed: Query<(), With<UnclaimedBodyPlaceholder>>,
  // :400
  let unsettled = unclaimed.iter().count();     // the cover retires on ZERO
  ```

  ⇒ **the placeholder is not only a diagnostic. It is the COVER'S SETTLE
  SIGNAL.** Delay its spawn by N frames and `count() == 0` becomes true *during
  the very gap the cover exists to hide* ⇒ **the cover retires early, and the art
  pops in with nothing over it.**

  ⛔ **and `features.rs` says so in its own doc**, twice — *"the room-transition
  cover holds until no `UnclaimedBodyPlaceholder` remains"* (`:204`, `:413`).
  I read that file to measure its timing and did not read its contract.
  ⇒ [[reference_thread_or_project_a_value]] and
  [[reference_a_system_wide_return_on_a_singleton]]: **grep the guarded value's
  uses before changing when it is produced.** Full census, run after the fact:
  the marker has **one production reader outside its own module** — the cover —
  and that one is decisive.

  ### ⭐⭐⭐ WHAT THE REFUTATION EXPOSES: the placeholder is a CONFLATION

  One entity is doing two jobs with one lifetime:

  | role | question it answers | wants |
  |---|---|---|
  | **diagnostic** | *"did somebody forget a family marker?"* | to fire **late** — only when a view stays unclaimed long enough to be a real orphan |
  | **settle signal** | *"is the new room finished drawing?"* | to fire **immediately** — the instant a view is unclaimed, so the cover keeps waiting |

  ⇒ ⭐⭐ **they want OPPOSITE timings, which is why no single delay can be right**,
  and why my proposal looked obviously correct for one job while breaking the
  other. It is the same one-thing-two-answers shape as every other finding today,
  and it explains the row's central puzzle exactly: **190 warnings, 0 cover
  expiries, and a player who still sees magenta.** The cover cannot distinguish
  *"art has not arrived yet"* from *"art will never arrive"* **because it is
  reading the diagnostic for both.**

  * ⇒ **the real fix is to SPLIT them, not to time them.** The cover's question
    can be answered with no placeholder entity at all — compare
    `FeatureViewIndex` against the claimed `FeatureVisual` ids, which
    `draw_unclaimed_feature_views` already computes in its first pass. The
    diagnostic then gets its grace period **for free**, because nothing is
    waiting on it.
  * ⭐ **that also fixes the instrument**: a warn that fires only after a view has
    been unclaimed for N frames means something, where 190-per-session means
    nothing. **Both halves of this row get better from one separation.**
  * ⚠ **still unmeasured, and say so**: whether the cover is retiring early *today*
    is the falsifier already written below (count placeholder spawns after
    retirement). ⛔ **the split is the right shape regardless, but do not claim it
    fixes Jon's flash until that runs** — I have now been wrong about this row's
    mechanism twice.

  ### ✔ SCOPED FOR DISPATCH 2026-08-09 — the number already exists in the loop

  `draw_unclaimed_feature_views` builds exactly the right set in its **first
  pass**, then throws it away:

  ```rust
  let mut known: HashSet<&str> = …;          // ids with a REAL visual (no marker)
  …
  for (id, view) in views.iter() {
      if known.contains(id) || already_standing.contains(id) { continue; }
      if view.size.x <= 0.0 || view.size.y <= 0.0 { continue; }   // no body, not a diagnosis
  ```

  ⇒ **the cover's honest question is `count of views where size > 0 and
  !known.contains(id)`.** ⚠ note it must **NOT** subtract `already_standing` —
  a stand-in is not art, and a view wearing one is still unsettled. That is the
  one place the two counts differ today, and it is the whole bug.

  * **the seam**: publish that count as a `Resource` from `ambition_render` and
    have the cover read it instead of `Query<(), With<UnclaimedBodyPlaceholder>>`.
    ✔ **the dependency edge already exists** — `room_transition_presentation.rs`
    imports `ambition_platformer2d::render::rendering::UnclaimedBodyPlaceholder`
    from that very module, so nothing new is added and the contracts job has
    nothing to reject.
  * ⛔⛔ **THE RISK, and it is the one that would make this silently wrong:
    ORDERING.** The cover reads *entities* today, which arrive through a
    `Commands` flush and are therefore self-synchronising. A **resource** is not:
    if the cover runs before the publisher, it reads **last frame's count**, and a
    one-frame-stale "zero" retires the cover exactly as early as the bug I was
    trying to fix. ⇒ **the publisher must be ordered before the cover in the same
    schedule.** ⚠ [[reference_bevy_schedule_semantics]] — **a cross-schedule
    `.after` is SILENTLY vacuous**, so check which schedule each one is in
    *before* writing the constraint, and assert the order in a test rather than
    trusting the attribute.
  * ⭐ **and the guard writes itself from the conflation**: a test where a view is
    published and its family claims it two flushes later must show **the cover
    still down on the intermediate frames** — which is precisely what the
    placeholder-based count cannot express once a grace period exists. That test
    is red today only if the cover is already retiring early, so **write it after
    the falsifier below**, not before.

  ---

  **The refuted proposal, kept because the refutation needs it:**

  ### ⭐⭐ A SECOND FIX THE ROW DID NOT CONSIDER: the stand-in has ZERO GRACE

  Everything above hunts the **cover** — whether it retires too eagerly, whether
  a transition gets one at all. ⇒ **there is an independent lever, and it does not
  need the cover to be involved.** Read
  `draw_unclaimed_feature_views` (`ambition_render/src/rendering/features.rs:222`)
  for its *timing* rather than its logic:

  > a view that is unclaimed **on this frame** gets a magenta sprite and a `warn!`
  > **immediately**. There is no counter, no deadline, no second look.

  ⇒ **a one-flush ordering gap is indistinguishable from a permanent orphan**, and
  the function's own neighbour already concedes the gap is routine: *"render
  families spawn through `Commands`, and a room with many actors takes several
  flushes to draw."*

  ⭐⭐ **the numbers already in this row are the argument.** 190 warns, 0 cover
  expiries, and a neighbouring comment calling the transient case expected ⇒ **an
  instrument whose entire purpose is to say "somebody forgot a family marker"
  fires 190 times on a HEALTHY run.** It is not noisy at the margin; on the happy
  path it is **all** false positives.

  * **the fix**: stand in only after a view has been unclaimed for **N
    consecutive frames** (a small `HashMap<id, first_seen_unclaimed>` beside the
    sets it already builds). ⭐ **it costs the real bug nothing** — a genuinely
    orphaned view is unclaimed *forever*, so it still gets its box and its warn,
    N frames later, which is irrelevant for a diagnostic nobody is watching in
    real time.
  * ⇒ **it fixes the product symptom and the instrument in one edit.** Jon stops
    seeing magenta on ordinary transitions, **and** a surviving warn starts
    meaning something — which is exactly the observable D46 has been missing and
    the reason its falsifier had nothing to look at.
  * ⚠ **and it is orthogonal to the cover hypothesis above**, so it does not
    depend on that one being right. If the cover IS retiring early, the grace
    period hides the consequence; if the cover is fine and some transitions get
    none, the grace period hides it there too. ⛔ **that is also its risk** — it
    would mask a real cover regression, so **land the cover falsifier's answer
    first, or land both and keep the cover's own counter as the honest signal.**
  * ⚠ **FALSIFIER, with the interpretation fixed in advance.** Measure, on a real
    room change, the **distribution of frames between a view being published and
    its family claiming it**:
    * **max ≈ 1–3 frames** ⇒ a grace of ~5 kills every transient box with margin.
      **Take it.**
    * **a long tail (tens of frames)** ⇒ the grace would have to outlast the
      cover, at which point it is hiding the cover's job rather than fixing a
      race. ⛔ **the idea is weak and this paragraph should be struck**, not
      tuned.
    ⇒ **the measurement decides it, and the two outcomes were written before it
    ran.**

- ✔ **D39 LANDED `830d386ce`.** A wave mob now names its character, and the
  goblins are goblins. ⛔ **and the row below was WRONG about the mechanism in a
  way that would not have fixed the bug** — see the correction block.

  * **the guard is the one that matters and it failed first, twice.** With the
    field absent, `character: "goblin"` was refused as an unknown field
    (`deny_unknown_fields`, as briefed). With the field present but no reference
    emitted, the poison `character: "gobln"` **compiled clean — zero
    diagnostics, zero resolved references**, which is the silent-typo shape
    exactly. Green only once the `PendingRef` landed.
  * ✔ **no new crate edge**: a cross-schema reference is by schema-id STRING
    (`PendingRef::new(SchemaId::new("character"), …)`), the way `boss_encounter`
    already names `music_track`. So no `fixtures/minimal_game/Cargo.lock`
    regeneration.
  * **`large_brute` stays `None`** — no catalog row is the goblin lab's heavy,
    and inventing one is content. ⭐ corroboration for the other half:
    `medium_striker`'s own archetype comments say *"Goblins poke with a thrown
    rock"* and *"Goblins dash to close a large gap"* — that archetype was written
    for goblins, so naming `goblin` on it is not a guess.
  * **eight call sites, not four**, plus two touchpoints the brief missed:
    `EncounterEvent::SpawnCommand` (the seam between `waves.rs` and
    `systems.rs`) and `snapshot_impls.rs`, whose rollback encoding claimed to
    carry each pending mob *"verbatim"* and was about to make that a lie.
    ⇒ the spawner now takes an `EncounterMobSeed { id, character, brain, pos,
    size }`, which also retired a `#[allow(clippy::too_many_arguments)]`.
  * ✔ **the field means ART ONLY, and now says so.** Traced: `character` reaches
    the sheet, the sprite-derived box, hurt feedback, barks and attack volumes;
    the brain still comes from `roster.spec_for_brain(kind)` and neither it nor
    the action set consults the catalog. **Same reach as
    `EnemySpawnSpec::character_id`**, and the two docs now cross-reference each
    other by name rather than being unified — D48's design question stays open
    and stays Jon's.

  ### ⛔⛔ CORRECTION — MY "TWO COLLAPSED INTO ONE" WAS WRONG, AND IT MATTERS

  The row below says *"the instance id is passed as BOTH the character id and the
  name"*. It is not. `ActorClusterSeed::new_in(…, id, name, art_identity, …)`
  takes **three** identity parameters: the collapsed pair is the actor **id** and
  the **name**, and the character slot is a *separate third* argument the
  encounter passed as `None`. ⇒ **acting on my sentence literally would have
  rewired the wrong two arguments.**

  ⭐ **and even the right argument is not sufficient**, which is the finding:
  `upgrade_actor_sprites` binds a sheet from `ActorRenderIndex`, and
  `rebuild_actor_render_index` fills that from **`ActorConfig::name`** — the
  display name. **The renderer never reads the seed's resolved
  `sprite_character_id`.** So `sprite_character_id` was already the
  identity-first road for barks, hurt feedback, body sizing and attack volumes,
  while the *sheet* alone is bound off presentation. The fix had to set both.
  ⇒ **that asymmetry is live beyond D39 and gets its own row — see D56.**

- **D39-ORIGINAL (superseded; marker stripped, text kept) THE GOBLIN ENCOUNTER
  DRAWS MAGENTA BOXES BECAUSE ITS MOBS ARE NOT
  CHARACTERS.** Jon's observation row, and his hypothesis (*"I think 'Goblin'
  was never a proper enemy multi-instance character"*) is **confirmed** — the
  encounter never names a goblin at all.
  * `game/ambition_content/assets/data/encounters/goblin_encounter.ron` spawns
    `kind: "medium_striker"` and `kind: "large_brute"` across three waves.
    ⛔ **neither is a character id.** `"goblin"` IS a proper catalog row
    (`character_catalog.ron:375`, with sheet, tuning, brain, barks and a hall
    dialogue) and the encounter does not reference it.
  * ⛔ **no `kind → character id` mapping exists anywhere in the workspace** —
    grepped for the capability, not the name. `kind` is documented in
    `ambition_encounter/src/spec.rs:79` as a size/role hint (`small_skitter`,
    `medium_striker`, `large_brute`) and that is all it has ever been.
  * **so the art lookup cannot succeed**: `spawn_actors.rs:1197` resolves the
    sprite with `sprite_render_size_for_name_in(authored_sheets, catalog, …)` —
    **by NAME** — while the mob's name derives from the kind. No sheet resolves,
    the render family does not claim the body, and the placeholder is drawn.
    ⇒ **the magenta box is the unclaimed-body placeholder**, the same one Jon's
    Android log reports for `NpcSpawn-*` and `hub_gravity_switch`.
    See [[feedback_presentation_binding_fails_silently]].
  * **the fork, and it is a design choice rather than a bug fix**:
    (a) give each wave mob a **character id** alongside its size hint, so the
    encounter spawns real goblins and `kind` stays a role hint; or
    (b) map kinds to characters per-encounter, which keeps the authored file
    terse but adds an indirection nobody reading the RON can see.
    ⭐ **(a) matches Jon's own framing** — he described the defect as goblin not
    being a proper multi-instance character, which is a statement that the mob
    should BE the character. It also fits the engine rule that Player/Enemy/Boss
    are DATA: an encounter mob is an actor with a character and a brain, not a
    third spawn path.
  * ⛔ **"ONE DEFECT WITH THREE FACES" WAS MY GUESS AND IT IS WRONG.** I wrote
    that the unclaimed `NpcSpawn-*` and `hub_gravity_switch` bodies in Jon's
    device log share this cause. Checked, and they do not — **do not fix them
    together**:
    * `NpcSpawn` **is** authored properly. `convert_npc_spawn`
      (`ldtk/src/conversion/entity_converters.rs:364`) reads a `character_id`
      LDtk field and only falls back to the bare identifier when it is EMPTY —
      and **all 163 `NpcSpawn` entities across every world carry one** (counted
      by parsing the `.ldtk` files, not by grep). So my follow-up guess, an
      authoring gap, is refuted too. ⇒ **a third mechanism, still unexplained**,
      and it wants its own row when someone works it.
    * `hub_gravity_switch` is a `(Switch)`, a different render family entirely.
    ⭐ two of my hypotheses died here, which is the useful part: the goblin fix
    below is verified and self-contained, and merging it with the other two
    would have made a clean change unlandable behind an open investigation.
    * ✔✔ **THE "THIRD MECHANISM" IS NOT A DEFECT — RESOLVED 2026-08-09, see D46.**
      The `NpcSpawn-*` warnings are the TRANSIENT case of a warning that cannot
      say which case it is. No new bug row is needed; the instrument gets one.

  ### ✔ CHAIN VERIFIED END TO END, statically — no repro needed

  I first filed this as inference from three facts and said so. Traced the rest;
  it is now a closed chain, and the exact defect is **narrower and more fixable**
  than "the mobs are not characters":

  1. `ambition_encounter/src/waves.rs:149` mints the mob's identity:
     ```rust
     let id = format!("encounter:{}:w{}:{}", self.spec.id, wave_index, self.spawn_counter);
     //   → "encounter:goblin_encounter:w0:1"
     ```
     — a per-INSTANCE id, unique by construction so ids never collide across
     attempts. Correct for what it is.
  2. `encounter/systems.rs:320` spawns with
     `spawn_encounter_mob(…, id, CharacterBrain::Custom(kind), …)`.
     ⭐⭐ **`kind` becomes the BRAIN.** It was never meant to be art, and the row
     above was wrong to imply the lookup uses it.
  3. `spawn_actors.rs:1638` then calls
     `ActorClusterSeed::new_in(authored_sheets, catalog, roster, id.clone(), id.clone(), …)`
     — **the instance id is passed as BOTH the character id and the name**, and
     `sprite_render_size_for_name_in` resolves art from it.

  ⇒ **no catalog character is named `encounter:goblin_encounter:w0:1`**, so no
  sheet resolves and the body is drawn unclaimed. ⭐ **the character slot is
  occupied by the instance id** — there is nowhere left to say *which character*
  this mob is, which is precisely Jon's *"never a proper multi-instance
  character"* stated in code.

  ⇒ **the fix is one added field, not a redesign**: a wave mob needs a
  `character` alongside its `kind`, threaded to the character-id argument while
  `id` stays the instance identity and `kind` stays the brain. Three parameters
  for three different questions — who it is, which body instance, how it thinks —
  where two are currently collapsed into one.

  ### ▢ READY TO DISPATCH — the four touchpoints, verified 2026-08-09

  1. **`EncounterMobSpec`** (`ambition_encounter/src/spec.rs:77`) gains
     `character: Option<String>` — and ⛔ **it MUST carry `#[serde(default)]`**,
     because the struct is `#[serde(deny_unknown_fields)]` and its doc records
     an audit where a bogus field compiled clean. `Option` rather than a required
     field: an encounter assembled from LDtk `EnemySpawn` markers (the fallback
     path named in `goblin_encounter.ron`'s own header) has no character to name,
     and today's behaviour must stay reachable for it.
  2. **`spawn_encounter_mob`** (`spawn_actors.rs:1621`) takes the character id
     as its own parameter. The line to change is the collapsed pair:
     ```rust
     ActorClusterSeed::new_in(authored_sheets, catalog, roster,
         id.clone(),   // ← character id: should be the AUTHORED character
         id.clone(),   // ← name/instance: stays the minted instance id
     ```
     ⚠ there are **two** `spawn_encounter_mob`s — the definition at
     `spawn_actors.rs:1621` and a re-export wrapper at `spawn/mod.rs:496`. Both
     signatures move or the second silently keeps the old shape.
  3. **`encounter/systems.rs:320`** passes it through; `waves.rs` keeps minting
     the instance id untouched.
  4. **`goblin_encounter.ron`** — the ONLY authored encounter file in the tree
     (the nine `boss_encounters/*.ron` are a different type) — names
     `character: "goblin"` on its `medium_striker` mobs. ⚠ `large_brute` has no
     obvious catalog character; leaving it `None` keeps it on today's path and
     keeps the change honest rather than inventing content.

  ⭐ **the guard defends the INVARIANT, not the fix**
  ([[feedback_a_guard_that_pins_the_fix_defends_the_gap]]): the schema handler
  `ambition_encounter/src/content_schema.rs` (which already has its own test
  module) should reject a `character` that names no catalog row. A test that
  merely asserts the goblins now resolve passes forever once the RON is right; a
  test that a MISSPELLED character is refused at load is the one that catches the
  next author. Poison it with `character: "gobln"`.

- ✔ **D38 THE PROCEDURAL SFX PATH HAS A LOUDNESS TARGET. ⛔ AND JON'S PREMISE
  ABOUT THE MUSIC IS REFUTED — Sanic's score was not turned down.**
  Instrument landed `993242d32` (`scripts/audio_levels.py` → `dev/audio_loudness_report.md`,
  87 s cold / 1–4 s warm over 782 sounds).

  ⚠ **CHECKED 2026-08-10: the charter still lists Jon's loudness observation as
  open, and the INSTRUMENT half of it is not.** *"He wants the relative levels
  of every SFX and score inspectable programmatically"* — that exists, it ran
  over 782 sounds, and it already answers his question with numbers: `sanic` is
  **+3.6 dB over the sfx median** (19 procedural sounds, loudest −17.8 RMS
  dBFS), while its score sits +1.8 LU over the music cohort — which is the
  refutation above, quantified. ⇒ what remains is the RETUNE, not the tool.

  ✔ **REGENERATED 2026-08-10 against the current script, and the conclusion
  HOLDS.** The report was older than `scripts/audio_levels.py`, so its numbers
  were not guaranteed to be what the current script produces — the report's own
  rule (*"regenerate after any audio edit"*) applied to the MEASURER rather than
  the measured. Re-run: **783** sounds (one more than the 782 measured on
  08-08), sfx median −23.5 → **−23.6** RMS dBFS, and the three hot owners are
  unchanged: `ability` +3.6, **`sanic` +3.6**, `ui` +3.2.
  ⚠ the re-run served 783/783 from cache, so it re-derived the AGGREGATION with
  the current code rather than re-measuring the audio. That is the half that
  could have drifted; a cache-busting pass is only needed if a sound file
  itself changes.

  * ⛔⛔ **"The Sanic music is much louder than the rest of the tunes" is FALSE**,
    and I verified it by a second route — plain `ffmpeg -af ebur128` on the files:

    ```
    you_are_too_slow           -17.7 LUFS   ← Sanic
    mary_o_you_died            -14.4 LUFS   ← 3.3 dB LOUDER
    how_to_kill_a_mockingbird  -14.8 LUFS   ← also louder
    ```

    Sanic is **19th of 77** by integrated loudness, +2.2 LU over the cohort
    median, true peak −3.7 dBTP which is *below* the music median. ⭐ **Mary-O is
    the actual music outlier** at +3.8 dB over the field. `cue.relative_volume`
    is 1.0 everywhere and both directors read one `MusicMix`, so the file IS the
    level — nothing playback-side explains a gap that does not exist.
  * ⭐⭐ **the real cause is a STRUCTURAL SPLIT in SFX**: packed clips median
    **−23.7 dBFS**, procedural specs median **−15.3** — **+8.4 dB** — and **Sanic
    ships no bank at all, so 100% of its cues take the hot path**. The renderer
    peak-normalises the bank to ≈−8 dBFS; **the procedural path has no loudness
    target whatsoever**. Sanic authors 0.38–0.5 on square/saw where engine cues
    are 0.16–0.26 on sine/triangle — twice the amplitude and the worse crest
    factor. Per owner vs the SFX population: **sanic +12.1**, pocket +10.4,
    twintrack +8.1, mary_o +3.3, content −4.6.
  * **the fix, and it is not "lower Sanic"**: give the procedural path a loudness
    target the way the bank already has one. Sanic 0.5 → ~0.18 is −8.9 dB and
    lands on the engine cohort; pocket (0.3) and twintrack (0.28–0.34) need the
    same. ⚠ re-run the instrument after — it rewrites in place.
    ⛔ **the per-provider retune in this sentence was NOT done and is not
    needed** — the engine target below moved pocket and twintrack onto the
    cohort with their authored numbers untouched. Superseded; see the ✔ block.
  * ✔ **nothing clips.** Worst true peak in the tree is `between_objectives` at
    −1.4 dBTP. Every finding is relative level, not distortion — so the
    "blow somebody's ear out" risk is real but is a mix problem, not clipping.
  * ⚠ **three things the instrument had to learn that a naive one would miss**,
    kept because the next person will repeat them: all 347 loose `.ogg` are
    MUSIC (no SFX file exists on disk — they are 381 clips in `sfx.bank` plus
    `SfxSpec` rows synthesised at runtime); **ebur128 is structurally blind to
    SFX** because integrated LUFS is undefined below 400 ms and ffmpeg returns
    the −70 gate floor rather than an error, so SFX rank on RMS and true peak;
    and scoring each cohort against **its own** median hides a uniformly hot
    population — Sanic reads +4.6 against its cohort and +12.1 against what it
    actually plays alongside.
  * ✔ bank and loose renderer output agree to **0.001 dB across all 381 clips**.

  ✔ **LANDED, and the fix is one engine constant plus one changed meaning.**
  `SfxSpec::volume` is now a **loudness** trim, not a peak: the cue's body is
  rendered at unit scale, its RMS measured off the actual samples, and one gain
  puts it at `volume` × `PROCEDURAL_CUE_REFERENCE_RMS_DBFS` (−11.0 dBFS, in
  `ambition_audio::render`). Nothing authored was retuned — all 54 specs across
  5 providers keep their numbers.

  | | before | after |
  |---|---|---|
  | `sfx_procedural` cohort median | −15.3 | **−23.4** |
  | its Δ vs the `sfx` population | **+7.6** | **+0.1** |
  | `sfx_packed` median (untouched) | −23.7 | −23.7 |
  | owners flagged ≥ +3 dB | sanic +12.1, pocket +10.4, twintrack +8.1, mary_o +3.3 | **sanic +3.6** |
  | loudest procedural peak | −6.0 dBFS / −3.9 dBTP | −11.4 dBFS / −9.6 dBTP |

  * ⭐ **the crest-factor half of the diagnosis holds, and the amplitude half
    was UNDERSTATED.** Sanic sat 9.1 dB above the engine's own provider
    (−10.7 vs −19.8) and content sat 3.0 dB above the packed cohort; that is the
    +12.1. Of Sanic's 9.1: **≈2.9 dB is waveform** (its median crest 3.3 dB vs
    content's 6.2 — square/saw against sine/triangle) and **≈7.4 dB is authored
    amplitude** (median `volume` 0.42 vs 0.18), not the ~5.5 dB the row guessed.
    ⚠ the two terms sum to 10.3, not 9.1, because a median of ratios is not the
    ratio of medians — they are shares, not an exact decomposition. Only the
    first term is an engine defect and only it is fixed.
  * **why pre-envelope.** The body is measured before `attack`/`release` so they
    stay shape controls. Normalising the enveloped clip makes `release` a
    loudness control — a long tail would be boosted until the whole-clip average
    matched, leaving its body louder than a short cue at the same `volume`.
  * **why the noise mix is inside the measurement.** Uncorrelated noise lowers
    RMS while leaving the peak at 1.0, so under the peak rule an airy cue came
    out quieter than a clean one at the same number — `player.robot.slash.air`
    (Saw, noise 0.70) lost **2.4 dB** to its noise mix alone, and Mary-O's
    `cue:Hit` (Square, noise 0.65) lost **5.8**. Under the target they do not.
  * **why measured, not a per-waveform table.** A closed form would be a table
    to forget: a new `WaveformSpec`, a partial cycle at 1 Hz, or a pitch sweep
    falls outside it silently. The gain divides by the RMS of the samples that
    actually ship.
  * ⚠ **relative intent survives inside a provider but is COMPRESSED**, and that
    is the fix working: each provider's spread narrows (sanic 10.9 → 4.7 dB,
    content 8.5 → 6.1) because part of the old spread was crest factor, not
    authorship. Cue order by loudness therefore reshuffles wherever two cues were
    ranked by their waveform rather than by their `volume`.
  * ⛔ **the residual `sanic +3.6` is authored amplitude and is NOT an engine
    bug** — Sanic writes 0.35–0.50 where the engine writes 0.14–0.26. It is a
    content call for whoever owns Sanic's voice, and it is now a 3.6 dB question
    instead of a 12.1 dB one. Symmetrically **`content` now reads −3.9** — its 15
    procedural specs are *all* shadowed by packed bank clips (verified id by id),
    so they are fallbacks heard only when the bank fails to load, and 4 dB quiet
    is the safer side of the same authorial spread.
  * ⛔ **the instrument lied once and now cannot.** Procedural rows are cached by
    the SPEC, not by rendered bytes, so the first post-fix run reported
    `0 fresh, 782 cached` and served the OLD sound's numbers for the new one
    without a word. `METRICS_VERSION` is 4 and its comment now says the
    synthesizer is part of the definition.
  * ✔ **the Python port was cross-checked against the Rust, not assumed**: four
    specs rendered by `cargo test` and by `synthesize()` agree to ≤ 0.002 dB RMS.
  * ⛔ **the music was not touched**, per the refutation above.

- ✔ **D37 LANDED `3333a4b0f` (merged 2026-08-09). SCHEMA v20.** A crate name is
  no longer part of the rollback wire format: the fingerprint hashes a type's
  **final segment alone**. ⇒ **every carve in the decomposition campaign stops
  being a netplay compatibility break**, which was the whole point.

  * **the probe failed first**, on the real `24b43f93a` relocation pairs, and
    carries the poison that a final-segment *rename* must still move the
    fingerprint. The duplicate-identity guard landed in the same commit.
  * ✔ **stable names, kinds and details byte-identical**, 423 rows each.
  * ✔ **the guard fires on nothing today**: 423 rows → 384 distinct type names →
    **384 distinct final segments**, injective by construction. Workspace-wide
    there are exactly two final-segment duplicates (`ActiveMatch`,
    `EncounterGate`) and both are one type spelled two ways, which `type_name`
    canonicalises.
  * ⛔ **the decision doc's claim that this "leans on existing behaviour" was
    FALSE**, and the worker caught it: `RollbackRegistry`'s duplicate-**name**
    check cannot see two `Cooldown`s arriving under two different stable names —
    which is exactly the legitimate-looking case. Genuinely new code.
  * ⭐ **(b′) was already written in the decision file and the fork list ignored
    it** — a section titled *"A third option, which dissolves the trade"* says
    *"hash the type's IDENTITY without its PATH"*, eighty lines before "The fork"
    offers only (a) and (b). ⇒ **the answer I "derived" this morning was sitting
    in the document I was deriving it from.** Same family as the zero-adopter
    capabilities above: written, correct, unreachable by the reader who needed it.
  * ✔ `test_rollback_baseline_paths_are_live.py` **deleted**, as its own docstring
    instructed for exactly this outcome; ADR 0027 corrected (it still claimed
    owners were hashed, six weeks after v5 removed them).
  * ▢ **left for the main tree**: re-freeze `dev/compile_ratchet_baseline.json`.
    The synthetic-carve ratchet test is red — ⚠ **and it was red on `main`
    without D37**: the worker measured 1273.4 s on its base, 1275.1 s with main's
    837 net added lines replayed and no D37, 1276.9 s with D37. **The tree
    crossed the budget on its own; D37's 235 lines cost +3.47 s.** The real gate
    passes with 17 s of headroom.

- **D37-ORIGINAL (superseded; marker stripped, text kept) ANSWER "IS A CRATE NAME
  PART OF THE ROLLBACK WIRE FORMAT?" — (b), AND
  IMPLEMENT IT.** This is the item that makes the next ten cheapest, which is the
  charter's own selection rule, and **the decomposition in flight is paying its
  tax right now**.
  * **the fact**: `descriptor()` stores `std::any::type_name::<T>()`,
    `schema_dump()` writes it, `schema_fingerprint()` hashes the dump. So moving a
    type between MODULES — not just crates — changes `SnapshotSchemaFingerprint`,
    and two peers with byte-identical wire formats refuse to agree.
  * ⭐ **measured live today**: D33 step 2 moved two rollback-registered
    components and `rollback_schema_baseline.txt` changed exactly two lines —
    stable names `actor.anim_override` / `player.blink_camera_state` **identical**,
    schema still v19, only the `type_name` column moved. *Nothing a peer can
    observe changed.* That is the whole argument in one diff.
  * ~~**the answer is (b)**: hash the type's final segment plus its module path
    BELOW the crate.~~ ⛔⛔ **REFUTED 2026-08-09 BY THIS ROW'S OWN EVIDENCE.**
    I cited step 2's baseline diff as the argument and then never read it. Here
    it is, both changed lines, `24b43f93a`:

    ```diff
    -actor.anim_override  …  ambition_platformer2d_actor_monolith::features::ecs::actor_clusters::ActorAnimOverride
    +actor.anim_override  …  ambition_sprite_sheet::character::anim::ActorAnimOverride
    -player.blink_camera_state  …  ambition_platformer2d_actor_monolith::avatar::components::PlayerBlinkCameraState
    +player.blink_camera_state  …  ambition_platformer2d_shared_tangle::camera_ease::PlayerBlinkCameraState
    ```

    **The below-crate path moved too** — `features::ecs::actor_clusters` →
    `character::anim`, `avatar::components` → `camera_ease`. A carve relocates a
    type inside its new crate as readily as between crates, because the whole
    point of the move is to put it somewhere it belongs. ⇒ (b) as written would
    have changed the fingerprint on **both** of the very moves it was proposed to
    protect. Only the FINAL SEGMENT survived either one.

  * ⭐ **the answer is (b′): hash the final segment ALONE.** Drop the crate and
    the module path. That is the only component of `type_name` that a carve
    leaves alone, and the diff above is the proof rather than the argument.
    ⭐ the precedent is one level down and already shipped — **v5 stopped hashing
    the registration owner** because *"which module registered this"* is not a
    wire-format fact, and `registry.rs:312-324` still carries that reasoning
    verbatim. A crate/module path is the same category of organisational label;
    unlike the owner, nobody chose to hash it — it came along inside a string
    being used for identity.
  * ⭐⭐ **and (b′) is what makes the collision guard NECESSARY rather than
    tidy.** Once the final segment is the identity, two `Cooldown`s in two crates
    become indistinguishable to the fingerprint — so the "reject duplicates
    LOUDLY" item below stops being a nicety and becomes the thing that keeps
    (b′) sound. The two halves are one design; landing the first without the
    second silently widens what the hash treats as equal.
  * ⚠ **the alternative worth stating and rejecting**: drop `type_name` from
    `schema_dump()` entirely, exactly as `owner` went in v5. It is defensible —
    the stable `name` is the contract — but it removes the last signal that a
    *different Rust type* got registered under an existing stable name. (b′)
    keeps that signal for the cost of one guard.
  * **what (b) costs**: a `GGRS_ROLLBACK_SCHEMA_VERSION` bump (exactly as v5 did)
    and a baseline regeneration. **What (a) costs**: every carve is a
    compatibility break forever, and the row measured that the cheapest
    three-crate cut in the workspace (`ambition_input`, which imports exactly ONE
    item from core) is gated on this, as are two of the other three top cuts.
  * ⚠ **the work item that must land IN THE SAME COMMIT**, named by the row and
    not optional: two types with the same final segment in different crates must
    be **REJECTED LOUDLY**, not silently merged. `RollbackRegistry` already
    rejects duplicate stable `name`s, so this leans on existing behaviour — but
    it is a determinism-critical hash and it gets its own probe.
  * ⚠ **who decided**: I did, under Jon's 2026-08-07 note *"I've left rollback
    design to agents"* — the same delegation the session-scope row is being
    answered under. Flagged rather than buried: this is a behaviour change to a
    determinism-critical hash, and Jon can overrule it.
  * ⚠ ~~**blocked until D33 step 2/3 lands** — both touch
    `rollback_schema_baseline.txt` and `rollback/domains/actors.rs`.~~
    ✔ **UNBLOCKED 2026-08-09.** Step 2 landed and step 3 landed at `a5d722c60`
    (its own subsection above records the retraction). ⇒ **this row is now the
    top of the queue by the charter's own selection rule**, and its evidence is
    already sitting in the tree: step 2's diff moved exactly two `type_name`
    columns while both stable names and the schema version stayed put — the
    argument for (b) written as a real diff rather than an argument.
    ⚠ **do not start it while D44 is in flight.** D44 edits
    `ambition_sprite_sheet/src/character/sheets/mod.rs`, the home `SpritePosedBody`
    moved to in step 3, and that type is rollback-registered
    (`rollback/domains/actors.rs:461`). Two jobs regenerating
    `rollback_schema_baseline.txt` from different trees is how a baseline lands
    describing neither.
    ⭐ **the session charter is stale about this and the ledger is not** — the
    charter still lists steps 2 and 3 as the work in flight. Working the charter
    text instead of this file would have re-done two landed steps.
    [[reference_a_jobs_verdict_is_not_its_first_targets]] — check the ledger row,
    then check the tree, and believe the tree.
  * ✔ ~~the decision file's index says **1 open and this row is not in it**~~ —
    already fixed. `awaiting-maintainer-decision.md` now reads *"2 open"* and
    lists S30 first, ordered by what it blocks. ⇒ **when D37 lands, S30 moves to
    `maintainer-decisions.md` and its row here is DELETED** — that file's own
    header forbids keeping an answered question, and seven of them once sat there
    for a week wearing their answers as a paragraph.

- ▢ **D36 THE `SheetRegistry` COLLISION WARNING CANNOT TELL ITS HARMFUL CASE
  FROM ITS HARMLESS ONE**, and fires ~30 times on every Android startup. Found in
  Jon's device log 2026-08-08, then diagnosed here rather than from the log.
  * **what fires**: `ambition_sprite_sheet/src/lib.rs:572` warns whenever two
    records claim one `target` with different frame geometry —
    *"One of these manifests is stale; the survivor will crop with the wrong
    grid."* Measured: **17 sheets claim `target: "toon"`, 18 claim `robot`, 9
    claim `goblin`**, and every one of the 17 is a review-cue character
    (`absurd_general`, `architect`, `craig` … `walter`). They are chained, so the
    log carries 16 warnings for `toon` alone.
  * ⭐ **the warning's own comment already knows sharing is legitimate** — it says
    so, listing `toon` x17 — and then asserts differing geometry never is. Both
    are true separately and **contradictory together**: 17 characters drawn on
    one rig legitimately have 17 different frame sizes. The condition it tests
    cannot distinguish them.
  * ✔ **and for these 17 the asserted harm does not occur.** The catalog resolves
    art by PATH, not by the shared key:
    `"npc_craig": (… manifest: "sprites/craig_spritesheet.ron")`. Each character
    loads its own geometry; nothing asks the registry for `"toon"`.
    ⚠ **but the harm is real for the case that motivated it** — the comment
    records `pirate_heavy_broadside_bess` loading the right image and cropping it
    with a dead manifest's grid, *"not cropping out the right part of the sprite
    sheet"* in Jon's words, found by bisecting the asset tree.
  * ⇒ **the fix is to narrow the condition to the case that can bite**: warn when
    a *loser* is something a reader can actually resolve by that target, not
    whenever two records disagree. ⛔ do NOT silence it — the incident it caught
    cost a day. ⛔ and do not "fix" the 17 by giving them unique targets; the
    shared rig target is the authored truth.
  * ✔ **NUMBERS RE-VERIFIED 2026-08-09** (`find … | xargs grep`, because
    `grep -r` skips this tree): **18 `robot`, 17 `toon`, 9 `goblin`**, then 3
    `sandbag`, 2 `shrine`, 2 `ninja`, and two singletons. The row is exact.
  * ⭐ **and the chaining arithmetic is checkable**: N records claiming one target
    produce **N−1** warnings, so the theoretical ceiling is
    `17+16+8+2+1+1 = 45` per boot. The device log showed ~30, which is consistent
    with only part of the cast being resident — ⇒ **the count is a function of
    what LOADED, not of what is wrong**, which is the whole complaint.
  ### ⇥ ⭐ MEASURED 2026-08-13 — the harmful case fires on NOTHING, and the
  ### proposed narrowing needs a fact the registry does not have

  Walking every non-variant `*_spritesheet.ron` under the actor assets and
  grouping by `(target, image)`:

  ```text
    (target, image) pairs                                   354
      with MORE THAN ONE frame geometry                       0
    targets claimed by more than one record                 146
      e.g. toon 32 records / 18 distinct images
         robot 26 / 20,  goblin 13 / 11,  sandbag 5 / 5
  ```

  ⇒ **the legitimate sharers differ in IMAGE**, every one of them, and no PNG in
  the tree is described by two different grids. So the obvious narrowing — *warn
  only when one image is claimed twice with different geometry* — would fire on
  nothing today and would be a clean guard against a stale manifest for a PNG.

  ⛔ **but it would NOT have caught the incident this row exists for.** Bess
  loaded the RIGHT image and cropped it with a dead manifest's grid: the catalog
  resolves the image by PATH while the registry supplies the grid by TARGET, so
  the two records need not name the same image for the harm to occur. Image
  equality is a different condition, not a narrower one.

  ⇒ **the row's own prescription is the right one and it needs an input this
  function lacks**: *"warn when a LOSER is something a reader can actually resolve
  by that target"*. `toon` / `robot` / `goblin` are RIG names nothing looks up;
  `pirate_heavy_broadside_bess` is a character id something does. `SheetRegistry::
  from_baked_table` cannot tell those apart — it has records and nothing else, and
  a crate whose whole claim is to be *"a content-free, reusable sprite-sheet
  vocabulary"* must not learn which keys a game resolves.
  ⇒ the check belongs where the resolvable set IS known — the character catalog's
  own sheet resolution, or the caller that passes the baked table — and that is a
  small API change (hand in the resolvable targets), not a condition tweak.
  ⛔ **do not narrow it on image equality and call the row closed**: it would go
  green while the day-costing case stayed unguarded, which is worse than the noise.

  ⇥ ⭐ **SIZED 2026-08-13 (one fact, then stopping — the row asked not to be
  re-investigated and it was right to).** `SheetRegistry::from_baked_table` has
  **exactly one production caller**, `lib.rs:679`, plus two in tests. So the
  "hand in the resolvable targets" change is one signature, one call site, and
  wherever the game chooses to compute the set — the crate still learns nothing
  about which keys a game resolves, because the caller supplies them. ⚠ that is
  the blast radius, not a recommendation to do it now.

  * ⚠ **priority is LOW and this row exists to stop it being re-investigated.**
    It is log noise today, not a defect. What makes it worth a row at all is that
    30 warnings per boot asserting a day-costing failure mode train a reader to
    scroll past the one that is real.

- ✔ **D26 CLOSED — the second fixture exists and it can tell the difference
  (re-verified 2026-08-10).** `the_death_beat_is_measured_with_the_world_awake`
  kills her by damage beside a live enemy and leaves the pit fixture measuring
  the pit, which is exactly what the dispatch note below asked for. **2 passed.**

  ⭐ **and it carries four vacuity guards, which is why it is worth closing rather
  than merely green**: a body must declare a dormancy policy at all
  (`policy_bodies > 0`), the gate must be OPEN where she stands
  (`awake_alive > 0`), the beat must be observed for >100 frames, and the world
  must not fall asleep DURING it (`awake_floor > 0`). Each maps to a way the old
  instrument reported success for a freeze nobody implemented.
  [[reference_a_check_that_cannot_fail]] §7 is this shape, and this fixture is
  the answer to it rather than another instance.

  ⚠ **the ▢ was stale by two days** — the eighth this run, and the same tell each
  time: the fix landed under one marker while another paragraph kept the box.

  *The re-verification that was true when written:*

- **D26-REVERIFIED (superseded; marker stripped) ✔ RE-VERIFIED STILL OPEN
  2026-08-08 (not stale).** Checked under the
  charter's grep-first rule, because most rows checked today turned out to have
  landed already. This one has not: `AI_SLOP_WAKE_RADIUS` is still **720.0**
  (`demo_mary_o/src/ai_slop.rs:66`, applied at `:252`), the fixture still does
  `displace(&mut app, Vec2::new(200.0, 4000.0))`
  (`demo_mary_o_app/tests/death_reset_timing.rs:128`), and `enemy_signature`
  (`:107`) is still the instrument. **Every enemy is still dormant for the whole
  beat, so the instrument still cannot distinguish a frozen world from a sleeping
  one** — and it would report SUCCESS for a freeze that was never implemented.
  ⭐ dispatch-ready: add a SECOND fixture that kills her by damage BESIDE an
  enemy and leave this one measuring the pit (the pit death is worth keeping, and
  4000px down is how she dies *in a pit*). The kill needs no displacement —
  `power_loop.rs:1124` shows the direct route,
  `e.get_mut::<BodyHealth>().unwrap().health.current = 0` — and `boot()`,
  `settle_until_playable()` and `enemy_signature()` are all reusable as-is.

- **D26-ORIGINAL (superseded by the ✔ block above; marker stripped, text kept)
  the death-beat freeze instrument measures nothing, and would report SUCCESS for
  an unimplemented freeze.** Not a maintainer decision — the decision
  row (`awaiting-maintainer-decision.md`, "Does the world FREEZE during a death
  beat?") is in good shape with three options and Jon's dialogue ruling already
  narrowing it to two. This is the broken instrument underneath it.
  * **what happened**: the fixture kills Mary-O at `y = 4000`, ≥3500 px from
    every enemy. Both Mary-O enemies gained `AwakeNearObservers { radius: 720 }`
    *after* the fixture was written, so **every enemy is dormant for the whole
    beat** and the signature sits constant for f15–f195 — 94% of it.
  * ⛔ **the failure mode is the dangerous direction.** A dormant world looks
    exactly like a frozen one, so this instrument reports "the world holds still"
    whether or not a freeze exists. If Jon picks *freeze it*, this is the thing
    that would confirm the implementation worked without testing it.
  * ✔ **verified by the supervisor from the fixture itself**, not from the
    decision doc's prose: `death_reset_timing.rs:128` is
    `displace(&mut app, Vec2::new(200.0, 4000.0))`, and the fixture's own
    assertion at :173 reads *"she never died in a pit 4000 units below the room
    — the fixture is not …"*. Only Mary-O's enemies are at risk: `AI_SLOP_WAKE_RADIUS`
    and `SNAKE_WAKE_RADIUS` are both **720**, while Sanic's `BADNIK_WAKE_RADIUS`
    is **4800** and will not silently sleep.
  * ⚠ **the depth is DELIBERATE, so "just move her closer" breaks the fixture.**
    4000 units below the room is how she dies *in a pit*, which is the death this
    test is about. Moving her within 720 px of an enemy changes the death cause.
    The two honest routes are to put an enemy near the pit floor, or to add a
    SECOND fixture that kills her by damage beside an enemy and leave this one
    measuring the pit. Prefer the second — the pit death is worth keeping.
  * ⭐ **and the second fixture is nearly free — every piece already exists in
    that file.** `boot()` (:52), `settle_until_playable()` (:63) and
    `enemy_signature()` (:107, the actual instrument) are all reusable as-is, and
    the kill needs no displacement at all: `power_loop.rs:1124` shows the direct
    route, `e.get_mut::<BodyHealth>().unwrap().health.current = 0`. **Killing her
    where she stands is both simpler than `displace` and strictly more faithful**
    — a real death happens next to what killed her, which is the whole reason the
    dormancy gate should be open. Check her spawn is within 720 px of an enemy
    first; if it is, the new test is `boot` → `settle` → zero her health → sample
    `enemy_signature` across the beat, and no coordinate is hand-authored.

  ### ✔✔ PRECONDITION CHECKED 2026-08-09 — IT HOLDS, WITH ROOM TO SPARE

  Parsed `mary_o.ldtk`, level `mary_o_1_1`. `PlayerStart` is at **(64, 352)**:

  ```text
     210 px   Solid Snake   at (274, 352)   ← inside 720
     338 px   AI Slop       at (402, 352)   ← inside 720
     434 px   Solid Snake   at (498, 352)   ← inside 720
     690 px   AI Slop       at (754, 352)   ← inside 720
     914 px   Solid Snake   at (978, 352)
  ⇒ 4 of 17 enemies are inside the 720 px wake radius of where she spawns
  ```

  ⇒ **kill-where-she-stands opens the dormancy gate on four enemies at once**, so
  the second fixture needs no displacement, no authored coordinate, and no change
  to the pit test. ⭐ **the row's "nearly free" is now measured rather than
  hoped** — the only open work is writing it.
  ⚠ and the nearest sits at **210 px, under a third of the radius**, so the
  fixture is not balanced on the edge of the gate it depends on: a content edit
  would have to move an enemy 500 px to silently break it.
  * **the fix is the fixture, not the code**: the beat must be observed with at
    least one enemy inside 720 px so the dormancy gate opens. The conclusion it used to support still holds on
    other evidence — eight captures inside the beat put the slop at screen x =
    334 → 202 with the camera anchored, so the world demonstrably keeps living.
  * ⚠ **do not make it assert.** The row says the test deliberately prints rather
    than asserts, because pinning today's behaviour would be a regression test
    over unpolished behaviour Jon has not ruled on. Restore the measurement; do
    not promote it to a guard.
  * ⭐ same class as *an instrument shaped exactly like the bug it hunts* — worth
    a sweep afterwards for other fixtures whose subject sits outside a dormancy
    radius, since `AwakeNearObservers` arrived after several of them.

- ✔ **D25 LANDED `8d6f36084`.** ⭐⭐ **SUPERVISOR INTEGRATION NOTE — the agent
  found the better generalisation, and my brief had it half-wrong.** I told it
  the module doc's *"same overlap rules … including multi-part boss hurtboxes"*
  was false because actors were unwired. It checked the third family and found
  **breakables consult `DamageableVolumes` at NEITHER end** —
  `ecs_hit_event_hits_breakable`'s `broken()` / `allows_hit()` / `pogo_refresh`
  gate is byte-for-byte its applier's own loop, and both test the coarse box, so
  that pair **already agreed exactly.** ⭐ **so the rule is not "every predictor
  must consult the volumes"; it is: each family owes its applier TANGIBILITY,
  spelled its own way, while PRECISION is a separate question the families
  deliberately do not all share.** That is a sharper statement of the invariant
  than the one I briefed, and the rewritten doc says it.
  ⭐ **it also found the concrete path to the divergent state**, which upgrades
  "latent": `refresh_body_damageable_volumes` → `ResolvedHurtboxes::world_volumes`
  returns `Some(vec![])` for an authored-but-empty window → `publish(vec![])`. The
  code path exists and is exercised; only the *content* is absent. ⚠ so "no
  shipped hurtbox authors an empty window" is a statement about the CONTENT, not
  about reachability — one authored window makes it live.
  ⚠ and it declined to write a `dev/benchmark-candidates/` entry with a reason
  worth keeping: that directory holds constructed benchmark PROBLEMS for
  evaluating models, not general lessons. Correct, and I have been sloppy about
  that distinction.

  **The fix — the hit-test that PREDICTS the applier now agrees with it on
  tangibility.** `ecs_hit_event_hits_actor` takes `Option<&DamageableVolumes>` and
  refuses a published-EMPTY body by asking `DamageableVolumes::intangible()` — the
  SAME function `strike_reaches_victim`'s first arm asks, not a second rule. Both
  call sites widened their existing tuple, so no new system parameter.
  * **the probe was written first and watched fail**, and it failed on exactly the
    fourth row: *"published EMPTY: an authored invulnerable window offers no target
    at all …"*. The first three assertions passed before it, which is the useful
    part — they proved states 1–3 already held, so the fix had to leave them alone.
  * **four states pinned in one test** (`damage_predicates.rs`'s first, the module
    had none): no component ⇒ coarse box; `published == false` ⇒ coarse box;
    published non-empty ⇒ **still** the coarse box (the precision half NOT taken,
    and the assertion message says so); published empty ⇒ miss.
  * **the module doc was rewritten, not patched.** Its claim of "the same overlap
    rules … including multi-part boss hurtboxes" was false in two directions, and
    the second one was found while fixing the first: breakables consult
    `DamageableVolumes` at NEITHER end — the breakable predicate's
    `broken()`/`allows_hit()`/`pogo_refresh` gate is byte-for-byte its applier's
    own loop, so that pair already agreed. The doc now says what is true: each
    family owes its applier TANGIBILITY, spells it its own way, and does **not**
    share PRECISION.
  * ⛔ **still NOT the precision half.** Whether a bolt tests authored
    *rectangles* instead of the coarse box remains Jon's feel call, unchanged.
  * **the record of what was wrong, kept for its diagnosis:**
  * **the applier is correct.** `apply_feature_hit_events`
    (`features/ecs/damage/mod.rs:466`) calls `strike_reaches_victim(&event.volume,
    victim_volumes.get(actor_entity).ok(), aabb)`, whose first arm is
    `DamageableVolumes::intangible()` since `9ee8a246e`. A published-empty body
    takes no damage.
  * **the predictor is not.** `ecs_hit_event_hits_actor`
    (`features/ecs/damage_predicates.rs:46`) checks `combat.alive &&
    event.volume.intersects_aabb(aabb.aabb())` and never reads the volumes.
    ⭐ **its own doc comment says its whole job is to predict the applier** —
    *"Read-only hit test used by systems that need immediate projectile / attack
    feedback while damage application is still drained through typed Bevy
    messages."* A predictor that disagrees with the thing it predicts is wrong by
    construction, not by taste.
  * **they diverge on exactly one state**: published-empty volumes with
    `alive == true` — an authored invulnerable window. The corpse case agrees
    (the publisher clears AND `alive` goes false). So this is the SAME state D23
    fixed, reached through the Player-faction path instead of the hostile one.
  * ⭐⭐ **the repo already contains the argument.** `ecs_hit_event_hits_boss`, the
    very next function, guards this and says why: *"Check against
    `damageable_volumes` so the hit-check matches what `apply_feature_hit_events`
    will actually apply damage to … checking against the gross AABB would
    over-trigger projectile termination on the body without ever applying
    damage."* Its empty-parts list returns `false`. The boss path pays this cost;
    the actor path does not.
  * **latent, not live** — the D23 worker swept every `.ron` under `crates/` and
    `game/` and found no shipped hurtbox authoring an empty window (the only
    empty lists are ATTACK windows). It becomes reachable the first time content
    authors an invuln window, which is an ordinary thing to author.
  * ⭐⭐ **the MODULE doc was the fork declaration, and it was false in writing.**
    `damage_predicates.rs:3-6`: *"The `ecs_hit_event_hits_*` functions let the
    projectile / attack systems pre-check whether a queued `HitEvent` will land
    … **using the same overlap rules `apply_feature_hit_events` (in `damage/`)
    applies — including multi-part boss hurtboxes.**"* It singled out boss
    hurtboxes because those are the ones that got wired; actor hurtboxes were not
    consulted at all. That sentence is what would have stopped the next reader
    from finding this, which is why it was rewritten in the same commit.

- ✔ **D24 LANDED — `scripts/compile_report.py` reads all five ledgers into one
  self-contained HTML page** (`dev/compile_report.html`, gitignored, regenerates
  in under a second; `--print-summary` for the text digest). It runs no build and
  invokes no cargo. 9 tests in `scripts/tests/test_compile_report.py`, each one
  watched red against the naive draft it rejects. **What it found is at the end of
  this entry** — the "cheapest per line" claim needed one qualifier it did not
  have. Original brief below, unedited.
  Jon, 2026-08-08: *"I also want to see a graph over time that
  looks at modules, maybe what modules they were split out of if there is a
  lineage, time per module, lines of code in the module at time of compile.
  things like that. debug vs release, optimization mode. we may want to quantify
  test time like this too. basically we should start recording this so we can
  build statistics and gain more insights into how to optimize compile time in
  maybe non obvious ways."*
  ⭐ **the recording half is DONE and the reading half does not exist.** All five
  ledgers the schema names are present and populated — `compile_units` 2164 rows,
  `run_tests_cost` 75, `compile_cost` 4, `compile_graph` 1, `carve_lineage` 1 —
  and `scripts/` contains no plotter, no summariser, no trend report. Jon's
  sentence has two verbs and only the first one shipped: *"start recording this"*
  ✅, *"see a graph"* ⬜. He asked to SEE it.
  ⚠ **the honest scope, stated before building.** The time axis is thin where he
  most expects depth: `compile_graph` and `carve_lineage` have ONE row each, so
  "over time" is a promise the data cannot keep yet for the graph shape or the
  lineage. What the data CAN support today is real and non-obvious:
  * **cost per line, per crate** — 2164 unit rows carry `seconds` and the crate's
    `lines` at ingest, which is the scatter that says which crates are expensive
    *for their size* rather than merely large. This is the one that answers
    "non-obvious ways", and it is the measurement that already overturned the
    intuition once (the monolith is the CHEAPEST crate per line).
  * **debug vs release, opt-level, incremental** — explicit columns since schema
    1; the comparison Jon named is a group-by, not new instrumentation.
  * **test time as a first-class cost** — 75 job rows with `executed_seconds`
    split out, so `seconds − executed_seconds` isolates the build graph from the
    running. That split is what makes "quantify test time like this too"
    answerable rather than a single wall clock.
  * **codegen vs frontend** — `compile_units` carries both, and the finding that
    the cost is codegen-bound is currently a sentence in a journal rather than
    something anyone can look at.
  ⛔ **CORRECTED BELOW — the first version of this table POOLED debug and release
  builds** and is left in place because the correction is the more useful record.
  Per-crate CV was 51–73%, the samples mixed `release/opt-3` with `test/opt-1`
  and `test/opt-0`, and the two crates it compared **did not share an opt level**.
  Averaging across configs is exactly the distinction Jon asked to be able to
  make. The corrected per-config numbers are after the table, and they make the
  finding stronger rather than weaker.

  ⭐⭐ **SUPERVISOR FINDING while the reader was being built — the data already
  overturns lane C's target.** Computed directly from `compile_units.jsonl`
  (first-party `lib` units, mean of 8 builds; the per-unit times sum to 1,124 s
  while wall clock is far less, because units run in parallel):

  | crate | s/build | share | lines | ms/line |
  |---|---|---|---|---|
  | `ambition_platformer2d_actor_monolith` | 126.4 | 11.2% | 111,790 | **1.13** |
  | `ambition_platformer2d_runtime` | 126.1 | 11.2% | 14,746 | **8.55** |
  | `ambition_content` | 69.8 | 6.2% | 21,496 | 3.25 |
  | `ambition_demo_mary_o` | 49.7 | 4.4% | 15,110 | 3.29 |
  | `ambition_characters` | 41.5 | 3.7% | 35,475 | 1.17 |
  | `ambition_audio` | 39.9 | 3.6% | 7,026 | 5.68 |
  | `ambition_render` | 39.4 | 3.5% | 17,497 | 2.25 |
  | `ambition_relativity2d` | 37.8 | 3.4% | 2,840 | **13.32** |

  * ⭐ **`ambition_platformer2d_runtime` costs the SAME as the monolith — 126.1 s
    against 126.4 s — on 13% of the lines.** Two crates, one twelfth the size,
    identical price. Nobody has looked at the small one.
  * ⭐ **the monolith is the CHEAPEST substantial crate per line** (1.13 ms/line,
    with `ambition_characters` at 1.17 the only peer). ⛔ **so "it is 111,790
    lines" is not a compile argument for carving it** — by wall clock it is the
    best-behaved big thing in the workspace. The carve case is architectural and
    edit-blast-radius, which is what the ratchet already guards; this measurement
    removes one argument rather than adding one.
  * ⭐ **`ambition_relativity2d` is the density outlier — 13.32 ms/line, ~12× the
    monolith, 91% codegen, on 2,840 lines.** Small enough to read in an
    afternoon, and generic/monomorphization-shaped by that codegen share.
  * codegen is **71–91% of every expensive unit**, which corroborates the earlier
    codegen-bound finding on a much larger sample than it was made on.
  * the top 8 crates are 47% of first-party compile time.
  ⚠ **these are 8 samples from one machine in one config**, and `seconds` is a
  unit's own rustc duration, not its wait. Directionally strong, not a benchmark.

  ### ✔ CORRECTED — the same measurement, split by config

  | profile / opt | crate | s | lines | ms/line |
  |---|---|---|---|---|
  | **release / 3** | `ambition_platformer2d_runtime` | **267.5** | 14,747 | **18.14** |
  | release / 3 | `ambition_platformer2d_actor_monolith` | 188.5 | 111,579 | 1.69 |
  | release / 3 | `ambition_content` | 124.0 | 21,496 | 5.77 |
  | test / 1 | `ambition_platformer2d_actor_monolith` | 105.6 | 111,579 | **0.95** |
  | test / 1 | `ambition_content` | 51.8 | 21,496 | 2.41 |
  | test / **0** | `ambition_platformer2d_runtime` | 78.9 | 14,747 | 5.35 |

  * ⭐⭐ **in RELEASE, `ambition_platformer2d_runtime` is the single most
    expensive crate in the workspace — 267.5 s, 42% MORE than the monolith, on
    13% of the lines.** 18.14 ms/line against 1.69 is **10.7×**. The pooled
    version reported them tied; they are not.
  * ⭐ **and the reason the pooled version lied is itself the finding.**
    `Cargo.toml` already carries `[profile.dev.package.ambition_platformer2d_runtime]
    opt-level = 0`, plus the same for `ambition_render` and `ambition_app` —
    three hand-picked crates someone already identified as expensive and
    mitigated **in dev only**. Release has no such override, so the shipped
    build and every `--release` capture pays the full 267.5 s. ⭐ this measurement
    *confirms* an existing engineering decision and shows where it stops.
  * ⭐ **the headline conclusion SURVIVES the correction, which is the point of
    doing it**: `corr(ms/line, lines)` is negative **within every config** —
    −0.11 (release/3), −0.27 (test/1), −0.78 (test/0). Lines do not predict cost,
    and that is not an artifact of pooling.
  * ⛔ ~~the monolith is the cheapest substantial crate in **both** real
    configs (1.69 release, 0.95 test).~~ **RETIRED by the reader** — see "what
    the reader found" below. It is rank 1 in exactly one of eight builds, and
    that build had 669 of 688 units cached so it ranked only 17 crates. Across
    honest full rebuilds it places 2nd–6th warm and 14th–25th cold, and
    `ambition_platformer2d_core` beats it 0.665 to 0.674 in the dev rebuild — so
    the word "substantial" does not rescue it either. ⭐ **the conclusion that
    mattered survives**: at 32% of the population mean rate, the monolith's size
    is still not a compile argument for carving it. The superlative is what was
    wrong, and I had written it twice.

  **Full release ranking — 55 crates, 1,630 s of unit time, top 12 = 69.2%:**
  runtime 267.5 s (16.4%), monolith 188.5, content 124.0, demo_mary_o 93.8,
  render 93.1, relativity2d 68.1, platformer2d_host 62.3, demo_sanic 58.5,
  demo_twintrack 52.5, demo_smash 46.4, portal2d_presentation 40.2,
  touch_input 32.7.

  ⭐ **the four DENSEST crates are all small consumers, all 84–93% codegen**:
  `relativity2d` 23.98 ms/line, `platformer2d_host` 20.96, `runtime` 18.14,
  `demo_twintrack` 15.98 — against the monolith's 1.69. Every one of them is a
  few thousand lines sitting on top of the big definition crates.

  ⚠ **HYPOTHESIS, recorded as one and NOT acted on**: cost follows generic
  *instantiation*, and instantiation happens in the CONSUMER. The monolith and
  `ambition_platformer2d_core` DEFINE types cheaply (plain code, amortized over
  many lines); the small crates above INSTANTIATE them — Bevy systems, queries
  and plugin wiring monomorphized over types declared elsewhere — and pay
  codegen for code they did not write. That would explain the negative
  lines↔cost correlation exactly, and it carries an uncomfortable corollary for
  lane C: **a new crate boundary can ADD instantiation cost**, because generics
  crossing it must be codegen'd in the consumer. ⛔ this is a story that fits the
  data, which is the most dangerous kind — three perf theories died that way this
  week. It is settled by `-Z self-profile` on `ambition_relativity2d` (2,840
  lines, 68.1 s in release, 93% codegen), which names the instantiated generics
  directly. **Do not repeat this hypothesis anywhere as a finding until that run
  exists.**

  ### ⛔ THE HYPOTHESIS WAS TESTED THE SAME HOUR AND IS NOT SUPPORTED

  Cheap discriminating test, no build required: invert `direct_dependents` in
  `dev/compile_ratchet_baseline.json` to get each crate's transitive
  DEPENDENCIES, then correlate against measured release seconds. If cost followed
  instantiation-in-the-consumer, dependency size should beat own size. Over 55
  crates:

  | predictor | corr with release seconds |
  |---|---|
  | own lines | **+0.576** |
  | transitive dependency lines | +0.497 |
  | number of dependencies | +0.514 |

  * **own size wins.** Dependency size is not a better predictor, so the story
    that small consumers pay for their dependencies' generics does not survive
    its first test.
  * ⭐ **the case that kills it**: `ambition_content`, `ambition_demo_mary_o` and
    `ambition_demo_sanic` have **identical** transitive dependency lines
    (364,978 — the demos all depend on everything) and cost 124.0 / 93.8 / 58.5 s.
    Dependency size cannot order them. Their OWN lines (21,496 / 15,110 / 7,705)
    order them exactly.
  * ⚠ **but own size does not explain the outlier either**: `relativity2d` is
    2,840 lines at 68.1 s — more than `demo_sanic`'s 7,705 lines at 58.5 s.
  * ⭐ **so the honest state is: no structural variable available in this repo
    explains `relativity2d`.** Own lines is the best predictor at +0.58 and it is
    not good enough. That is a stronger reason to run the profile than the
    hypothesis was — it is now the only remaining route, rather than one of two.
  * ⚠ the test's own weakness, stated: `deplines` saturates (most crates sit on
    the same big base), so it has little discriminating power. A null from a
    low-variance predictor is weak evidence. It is enough to stop the hypothesis
    being repeated as fact; it is not enough to call it refuted.

  ### ⛔ TWO METHODOLOGICAL FAULTS IN THE PROFILE BELOW — read this first

  Found within the hour, by asking whether one number was possible.

  ### ✔ FAULT 1 RESOLVED — the confound was real, and my guess about it was BACKWARDS

  Re-ran the identical command with `CARGO_INCREMENTAL=0` in a fresh target dir:

  | config | total | ThinLTO | share |
  |---|---|---|---|
  | `incremental = true` (the repo default) | 12.77 s | 6.47 s | 50.7% |
  | `CARGO_INCREMENTAL=0` | **28.40 s** | 16.68 s | **58.7%** |

  * ⭐⭐ **ThinLTO dominates in BOTH configurations.** Turning incremental off did
    not reduce its share — it *raised* it, 50.7% → 58.7%. **The finding survives
    the confound and the caveat is lifted**: the backend, and ThinLTO
    specifically, is where release compile time goes, independent of the
    incremental setting.
  * ⛔ **and I had the direction wrong.** I reasoned that incremental forces many
    small CGUs and therefore *manufactures* ThinLTO work, so removing it should
    shrink the share. It grew. ⚠ **plausible mechanism, wrong sign** — exactly
    the kind of story this ledger has been burned by three times, and the only
    reason it did not become a finding is that the run was already queued before
    I wrote the hypothesis down.
  * ⚠ **BONUS, single sample, do not act on it yet**: `CARGO_INCREMENTAL=0` made
    this release build **2.2x slower** (28.40 s vs 12.77 s). The likely mechanism
    is CGU count — incremental uses many small codegen units, which parallelise
    better on 8 cores even though each is optimised less. If that reproduces on
    a second crate it is a real and counterintuitive result, because incremental
    is usually justified for the dev loop alone and this is a release build.
    ⛔ different target dirs and one sample each; it is an observation, not a
    measurement.

  1. ⛔ ~~**the ThinLTO attribution has a CONFOUND I did not control:**~~
     **← RESOLVED ABOVE, kept for the reasoning.**
     `.cargo/config.toml` sets `incremental = true` for the whole workspace.**
     Incremental compilation deliberately uses many small codegen units and leans
     on ThinLTO to recover the performance that costs — **so "ThinLTO is 50.7%"
     may be a consequence of this repo's own incremental setting rather than of
     cargo's stock release profile.** The two stories have opposite fixes. A
     `CARGO_INCREMENTAL=0` re-run of the same crate is in flight to separate
     them; **until it lands, treat the ThinLTO share below as measured-but-not-
     attributed.** ⭐ note the setting is deliberate and justified for the DEV
     loop (its own comment records 4.9x), and applying to RELEASE may simply
     never have been considered — which is a finding either way.
  2. ⛔ **the `runtime` and `monolith` rows are RECOMPILES against a warm
     incremental cache, and are not comparable to the `relativity2d` row.** Both
     were built once as dependencies and then rebuilt with `-Ztime-passes`. The
     tell is impossible: `type_check_crate` reads **0.019 s for the 14,747-line
     runtime**, less than the 2,840-line `relativity2d`'s 0.196 s. Its whole
     frontend is 2.1 s, nearly all `generate_crate_metadata`. **That is reuse,
     not speed.** Their BACKEND figures are the interesting half and are less
     affected, but no frontend share from those two rows should be quoted.

  ⭐ the lesson is the one already in memory, arriving through a new door: *an
  implausible number is a broken instrument.* 0.019 s to type-check 14,747 lines
  is not a result.

  ### ✔✔ ANSWERED (with the caveats above) — nightly `-Z time-passes`

  `cargo +nightly rustc -p ambition_relativity2d --release -- -Ztime-passes`.
  Total **12.77 s** for the crate's own rustc invocation:

  | pass | s | share |
  |---|---|---|
  | `LLVM_thinlto` | 6.47 | **50.7%** |
  | `codegen_crate` | 4.28 | 33.5% |
  | `LLVM_passes` | 4.20 | 32.9% |
  | `monomorphization_collector_graph_walk` | 1.41 | 11.0% |
  | frontend (typeck + borrowck + coherence + expand) | **0.56** | **4.4%** |

  (passes overlap — `codegen_crate` spawns the LLVM work; the shares do not sum.)

  * ⭐⭐ **it is not generics, not the dependency graph, and not the crate's own
    code. It is ThinLTO.** Type-checking this crate takes **0.196 s**. The
    frontend in total is 4.4%. Over half the compile is a single LLVM pass.
  * ⭐ **and it is a DEFAULT nobody chose.** There is no `[profile.release]`
    section in `Cargo.toml` at all — the `lto = "thin"` / `codegen-units = 1` at
    lines 98–99 belong to `[profile.android-size]`. So `--release` runs cargo's
    stock profile: `codegen-units = 16` with **local ThinLTO across those 16
    units**. Nobody opted into paying this; it arrived with the default.
  * ⭐ **this is the "non-obvious way" Jon asked for** — the lever is codegen
    configuration, not code structure. `codegen-units`, `lto`, and the
    debug/release split are three knobs nobody in this repo has tuned for
    *release*, while dev already carries hand-picked `opt-level = 0` overrides
    for the three worst crates.
  * ⚠ **and it reframes my 68.1 s figure for this crate — the two numbers are not
    the same measurement, and neither is wrong.** The ledger's 68.1 s is a unit's
    wall duration *inside a real full build*, sharing 8 cores with up to 7 other
    rustc processes that are each internally parallel for codegen. The 12.77 s is
    the same crate with the machine to itself. ⭐ **so: use the LEDGER number to
    ask "what does this crate cost the build I actually run" (it is the right
    input for prioritising), and the TIME-PASSES number to ask "what does this
    crate intrinsically cost" (the right input for diagnosing why).** ⛔ what is
    NOT legitimate is comparing one to the other, which is what I did when I
    first wrote this bullet and called the ledger "inflated". Same error family
    as `cargo check` ≠ a build's frontend phase.
  * ⚠ the ratio is consistent across both crates measured so far — 68.1/12.77 =
    5.3x, 267.5/66.0 = 4.1x — which is about what 8 cores of sharing predicts,
    and is itself weak evidence that the ledger's per-unit durations are
    dominated by core sharing rather than by anything crate-specific.
  ### It DOES generalise, and the shape is the explanation

  Same command on the other two (⚠ both warm-cache recompiles — backend figures
  only, per fault 2 above):

  | crate | lines | total | ms/line | ThinLTO | share | ThinLTO ms/line |
  |---|---|---|---|---|---|---|
  | `relativity2d` | 2,840 | 12.77 | 4.50 | 6.47 | **50.7%** | **2.28** |
  | `platformer2d_runtime` | 14,747 | 65.99 | 4.48 | 24.47 | 37.1% | 1.66 |
  | `actor_monolith` | 111,790 | 76.07 | **0.68** | 15.46 | **20.3%** | **0.14** |

  * ⭐⭐ **ThinLTO's share falls monotonically as the crate grows — 50.7% → 37.1%
    → 20.3% — and its per-line cost falls 16x.** The monolith has **39x** the
    lines of `relativity2d` and only **2.4x** the ThinLTO.
  * ⭐⭐ **that is the explanation for the negative lines↔cost-per-line
    correlation.** A large part of the backend cost is per-CRATE, not per-line,
    so a bigger crate amortizes it. It is not that big crates are efficient; it
    is that small crates each pay a fixed toll.
  * ⭐ **and the monolith at 111,790 lines costs 76.07 s against the runtime's
    65.99 s at 14,747** — 7.6x the code for 15% more time, machine to itself.
  * ⛔⛔ **the corollary for lane C, now measured rather than argued: splitting
    one crate into N multiplies that per-crate toll by N.** Carving the monolith
    into six crates adds five more of it. ⚠ **this is not a veto** — more crates
    also means more parallelism in a real build, which can cut wall clock even as
    total CPU rises, and the ledger's build-context numbers are the right input
    for that half. But "carving reduces compile time" now has measured evidence
    against it, and the burden of proof has moved.

  ⭐ **and across all 52 first-party crates ≥300 lines, LINES DO NOT PREDICT
  COST** — `corr(ms/line, lines) = −0.23`, slightly *negative*. Bigger crates are
  cheaper per line here, not more expensive. That is the single most useful thing
  the ledger has said, because line count is the unit every carve discussion in
  `docs/planning` has used.
  ⚠ **I tried to name the cause and could not, so it is NOT recorded as one.**
  Bevy system density looked like the answer and only half is:
  `corr(ms/line, add_systems+add_observer per kloc) = +0.42`, with two
  counterexamples that kill the simple story — `ambition_app` has 4.6 systems per
  kloc and is the third CHEAPEST per line (0.69), while `ambition_demo_twintrack`
  has 1.8 and is the third most expensive (9.47). `ambition_relativity2d` itself
  contains no `impl<`, no `macro_rules!`, and 21 generic fns, so it is not the
  crate's own generics either.
  ⭐ **the cheap experiment that would settle it, and it is much cheaper than the
  one D16's tail proposed**: run nightly `-Z self-profile` on
  **`ambition_relativity2d`** — 2,840 lines, 37.8 s, 91% codegen, the highest
  density in the workspace — instead of on a whole build. One small crate with an
  extreme signal beats a 1,124 s build with a mixed one.

  ⛔ **so the deliverable is a READER, not more collection** — one script over
  the ledgers that already exist, emitting something Jon can open. Do not add a
  sixth ledger, do not re-measure, and **state on the page itself where the time
  axis is one point**, because a trend line through a single sample is the
  prettiest way this could lie.

  ### ✔ WHAT THE READER FOUND — six refinements to the record above

  ⭐ **"the monolith is the cheapest crate per line" is rank 1 in exactly ONE of
  the 8 recorded builds, and that build measured 17 crates rather than 55.** The
  back-filled 2026-08-07 report had 669 of 688 units already cached, so only 17
  first-party crates were dirty — and among *those* the monolith was indeed
  cheapest, at 0.45 ms/line. Once the collector recompiled all 55, the monolith
  placed **2nd–6th in the warm rebuilds and 14th–25th in the cold builds**. The
  qualifier "substantial" above is what rescues it, and even that is edge-case
  false in one build: `ambition_platformer2d_core` (29,062 lines — substantial by
  any reading) beats it 0.665 to 0.674 in the honest dev rebuild. ⭐ **the useful
  half survives intact**: at 32% of the population mean rate the monolith's size
  is still not a compile argument for carving it, which is the conclusion that
  actually mattered. What is retired is the *superlative*, not the finding.

  ⛔ **one recorded build's `phase` column is contradicted by its own cache
  counters, and pooling by the label corrupts every per-config average.**
  `cargo-timing-20260808T111707964Z` is labelled `dev/first-party` and has
  `build_fresh_units: 0` — nothing cached, so it recompiled all 688 units
  including third-party, in 540 s wall against the two honest first-party
  rebuilds' 188 s and 210 s. The reader derives `cache_state` from the counters
  and flags the row; it never groups by `phase`. ⚠ **this is the same hazard as
  the pooled debug/release table corrected above, one level down** — and it is
  why the reader's atomic unit is a *build* (`build_source`), never a
  configuration name. A `cargo test` filter would not catch it; only the counters
  do. ⚠ the release `cold` phase is also not fully cold (148 of 689 cached), so
  the reader carries three cache states rather than two.

  ⚠ **the `incremental` axis Jon named cannot be answered from `compile_units` at
  all: all 2,145 collector rows read `false`.** The column exists, the collector
  correctly *sets* rather than inherits it — and it has only ever been set one
  way. The sole incremental-on measurements in the repo are 3 of the 4 rows in
  `compile_cost.jsonl`. The page says this in place of drawing a one-bar chart.
  ⭐ **one collector run with `CARGO_INCREMENTAL=1` would close a whole dimension
  of Jon's question**, and it is the cheapest outstanding item here.

  ⭐ **codegen is confirmed at a scale the original claim was never made on, and
  it has TWO denominators 4.4 points apart** — 79.6% of the seconds that carry a
  frontend/codegen split, 75.2% of every unit-second recorded. The gap is 412
  units with no split at all (proc-macros, build scripts, bins, tests, and
  `ambition_app`'s `cdylib` — they emit no metadata, so cargo reports no phases).
  Per build the split share is 74–83%, cold or warm, dev or release. Quote either;
  name which.

  ⭐ **test time: 11.6 h of suite wall clock across 75 invocations, of which 70%
  is not running tests.** ⚠ that headline overstates, knowably:
  `executed_seconds` is libtest-only, so every pytest and `cargo check` job books
  its whole wall clock as "build" — the acceptance job alone contributes 6,992 s
  of it. **The number to trust is `workspace (default features)`, the one job that
  reports both: 54.8% build, 45.2% running**, over 72 runs and 7.4 h.

  ⚠ **and the edit-scenario ledger is thinner than 4 rows suggests: 3 of the 4
  have a warm pass that was not warm** (`warm_noop_seconds` at or above half the
  after-edit time). The tell is the two `test-build` rows — same scenario, same
  commit, same environment, warm passes of 14.81 s and 0.50 s. The reader refuses
  to compute an edit cost for those and shows `⚠ n/a`.

- ✔ **D5 the smash → ambition character leak — CLOSED, by Jon** (2026-08-08:
  *"the oni leader bug was fixed"*). The code agrees: `demo_smash/src/lib.rs:1307`
  names his exact reproduction and releases the roster, `PreparedMatch`, the
  select state, the cursor and the seating source, each **by OWNER** rather than
  by type. ⭐ **the whole cost of this row was that his entry carried no triage
  mark**, so it read as untouched work; it is marked in his file now.
  ⚠ **one factual gap recorded, not reopened**: `the_full_multi_game_lifecycle_is_leak_free`
  walks Sanic → Mary-O → Pocket → TwinTrack → Ambition → Sanic. It never runs
  **smash → ambition**, which is the sequence Jon actually reported, so what
  defends this fix is the scope declaration and the smash-specific tests rather
  than the lifecycle walk. Worth knowing before anyone cites that test as the
  guard for it.
  The original row, kept for its diagnosis:
  *"When you play a round of smash and choose your character, that becomes your
  character if you quit to title and play ambition itself. That is a big
  architecture problem … there is some persistent state that is likely a global
  resource that is not reset."*
  ⭐ **his second sentence is the diagnosis, not colour**: *"I was the oni leader
  and I could talk to other characters, but not the oni leader himself."* Failing
  to talk to your own pedestal is exactly `$speaker_is_self` suppression —
  `conversation::opening` enters `<dialogue_id>__self` only if content authored
  that node, and returns `false` otherwise. So the symptom CONFIRMS the cause:
  the ambition session's `WornCharacter` really is the smash pick, and this is a
  leaked selection rather than a rendering or naming coincidence.
  ⚠ **this is the class the memory already names three times** (experience-scoped
  state, a pause is two globals the session does not own, a global roster needs
  an owner), and each time the census came back clean while the leak was in a
  resource nobody thought of as session state. ⛔ so the deliverable is the
  OWNER, not a `reset` call at the quit site: `app.experience_owns(..)` is the
  seam, and a plain `releasing::<T>()` from two games deletes the other's match.
- ✔ **D6 DONE** (submodule `6203ae9`, pointer `73adaa72c`). A binding-spec
  defect: `part-neck` was authored INSIDE `part-torso`, and `_descendant_ids`
  collects with `elem.iter()`, which recurses into nested part groups — so torso
  silently absorbed the neck's path. Only nested group among 170 across seven
  rigged SVGs. ⚠ **`9521978` introduced it** by adding the check and migrating
  Carl but not the clerk; the `data-rig-z` renumbering was a second failure
  queued behind the first. ⭐ **not cosmetic** — the stray slab was covering the
  far arm's sleeve and cuff. The check was NOT relaxed; the data was made to
  satisfy it. Diff audited on integration: the moved path lines are md5-identical.
- ~~**D6 `scripts/regen/sprites.sh --target patent_clerk` cannot re-render Jon's art**
  (obs:47) — ⭐ **UNBLOCKED 2026-08-08** by Jon's *"you can commit any sprite or
  music work"*, and the failure is precise rather than vague:
  `SVG view 'Patent Clerk - Side Left' does not have one-to-one drawable
  ownership: multiply_assigned={'path1115': ['torso', 'neck']}`. The sheets in
  the tree came from a 13:58 render installed from `generated/`, so **the game is
  fine today and a fresh clone is not** — which is the repo's own
  regen-stays-fresh-clone-safe invariant, currently violated.~~
- ⭐ **D8 LANDED 2026-08-08 — `scripts/compile_ratchet.py`, wired into the suite
  as `compile-cost ratchet (frozen weights, not a stopwatch)`, ~1.6s, NO build.**
  ⚠ **superseded in part by D27 above**, which added a fifth guarded number in
  measured SECONDS and renamed the job; the four below are still guarded and
  still say what they said. Guards
  four numbers against `dev/compile_ratchet_baseline.json`: largest
  recompilation unit (111,579), worst edit blast radius (427,218 lines / 46
  crates, `ambition_geometry`), the monolith's blast radius (248,672 / 17), and
  the serial chain length (12) — the last because **a carve that inserts a layer
  makes the wall clock worse while improving every other number**, and nothing
  else in the repo would notice. Violations exit 1 by default; there is no
  `--check` flag, deliberately.
  ⭐⭐ **AND IT ANSWERS THE C4e QUESTION BELOW: the `conversation` carve buys
  −1.94% of the largest unit, −0.87% of the monolith's edit cost, and EXACTLY
  0.00% for editing `conversation` itself** — six files in the monolith name
  `crate::conversation`, so the new crate lands BELOW it and the isolation runs
  one direction only. ⛔ **so "a COMPILE-ISOLATION win" is priced at ~1% and the
  architectural case is the whole case.** The decision is now makeable; details
  in `dev/journals/compile-time-and-disk-2026-08-07.md`. The row's original
  reasoning is kept below because it is the design rationale.
  ⛔⛔ **AND THE SECONDS ROWS D27 ADDED MAKE THAT ~1% NEGATIVE.** The same
  simulation now reads `first_party_seconds` **1,307.9s → 1,312.1s (+4.2s)** and
  `critical_path_crates` **12 → 13**, because 2,167 lines leave a crate measured
  at 0.61 ms/line and arrive in a new one that will not be that cheap — priced
  here at the population median 2.56, which is the OPTIMISTIC assumption. So the
  compile-time argument for this carve is not "small", it is **the wrong sign**,
  and the architectural case is now the only case there is.
- ✔ **D8 (original brief — superseded by the row above, kept for its reasoning) QUANTIFY WHAT A CARVE BUYS, AND GUARD THE NUMBER.** Jon, 2026-08-08:
  *"what will be valuable is tracking compile times and measuring if we get any
  wins from making crate carves. I want to quantify those compile wins as we do
  those. And to guard against compile time regressions."*
  ⭐ **this is the missing half of lane C, and lane C is why it matters.** C4e
  concluded the `conversation` carve is "a COMPILE-ISOLATION win, not a footprint
  win" — and then nobody could say how big, so the carve stalled on a maintainer
  decision it might not have needed. A carve whose payoff cannot be stated is a
  refactor nobody can justify.
  ⚠ **the instrument already exists — do not write a second one.**
  `scripts/compile_cost.py` measures an EDIT (append a fn, run a real cargo
  command, revert without git), warms first, and appends to
  `dev/ambition_dev_measurements/compile_cost.jsonl`. It already established the finding that decides the
  design: the cost is **codegen**, not link (9.3s to relink a 769 MB binary with
  mold) and not frontend. What it lacks is (a) any way to attribute a measurement
  to a specific carve, and (b) a gate.
  ⛔ **and the gate must NOT be wall-clock.** A timing threshold on a shared
  machine fails randomly, gets waived, and then gets ignored — this repo's own
  "a check that CANNOT FAIL" lesson arriving from the opposite direction. Guard
  the DETERMINISTIC CAUSE and track the noisy effect: the size of the largest
  recompilation unit and how many crates a consumer's rebuild must traverse are
  exact integers that move precisely when a carve works, and they belong in the
  absence-contract ratchet idiom (exit codes, 25 contracts, ~11s) rather than in
  a timer. Wall-clock stays in the ledger, plotted and never a gate.
- ⭐ **D9 SCHEMA LANDED 2026-08-08 — `dev/compile_telemetry_schema.md`.** One
  envelope (`schema`/`kind`/`recorded_at`/`commit`/`dirty`/`run_id`/`label`),
  four kinds, four files: `compile_graph.jsonl` (deterministic, seeded),
  `compile_units.jsonl` (per-module wall time, **19 real rows back-filled**),
  `compile_cost.jsonl` (the scenario stopwatch, now with explicit
  `profile`/`opt_level`/`incremental` columns), `run_tests_cost.jsonl` (test
  time, envelope added), and `carve_lineage.jsonl` for the one dimension with no
  other source. ⭐ **`cargo build --timings=json` is nightly-only, but the STABLE
  HTML report embeds the identical per-unit JSON as `const UNIT_DATA`, frontend/
  codegen split included** — so the collector needs no nightly toolchain, which
  is half of why ADR 0013's "quarterly" never ran.
  ⛔ **and the back-fill re-checked two of this campaign's numbers and neither
  reproduced**: "255 of 313 unit-seconds are codegen" is really **197.6 (63%)**,
  and "18% frontend" is really **30%**. The direction survives, the magnitude
  does not. The collector still owes a real timed build in a quiet target dir.
- ✔ **D11 COLLECTOR LANDED 2026-08-08 — `scripts/compile_collect.py`, 1,457 rows
  from five real builds (2,357s of cargo).** Named configurations, each with its
  own `CARGO_TARGET_DIR`; `opt-level` read off the rustc line `cargo -v` prints
  rather than modelled; `CARGO_INCREMENTAL` set rather than inherited;
  `--analyze` reads it back and builds nothing. Full write-up:
  `dev/journals/compile-time-and-disk-2026-08-07.md`, addendum 2.
  ⭐ **the ratchet's four numbers were finally tested against seconds.**
  `largest_unit_lines` rho +0.83…+0.86; `worst/watched_edit_cost_lines` rho
  +0.99 — but its LINE WEIGHTING adds nothing, an unweighted crate count
  predicts the same seconds; `critical_path_crates` is right in hops and **wrong
  by 2.2x in seconds**, because pipelined compilation releases a dependent at
  the predecessor's rmeta so only the FRONTEND is serial.
  ⭐⭐ **the non-obvious win: a cold build and a rebuild want OPPOSITE fixes.**
  Cold is core-bound (767.6s of packing against a 418.9s floor) so halving
  codegen saves 282.7s; the 55-crate rebuild is dependency-bound (123.9s against
  168.4s) so halving codegen saves 11.8s and halving the frontend saves 61.6s.
  The rebuild is what an agent pays before one test runs, and the repo has been
  prioritising it with the cold build's number. Fresh split: **codegen 72–80%**,
  frontend 17–25% — so 63% was right about its 19-unit artifact and is not the
  repo's figure.
  ⚠ open, not closed: `ambition_app` is the only lib declaring a `cdylib`, which
  is why it is the only unit with no frontend/codegen split; removing it drops
  the dependency floor 10% and moved the wall clock not at all in a paired run
  that was 12% more contended. **Unresolved.** Also skipped: the
  `dev-incremental` configuration.
- ✔ **D9 (original brief — the SCHEMA landed above; the COLLECTOR landed as D11 above) COMPILE TELEMETRY: record enough to find non-obvious wins.** Jon,
  2026-08-08, expanding D8 — *"I also want to see a graph over time that looks at
  modules, maybe what modules they were split out of if there is a lineage, time
  per module, lines of code in the module at time of compile. things like that.
  debug vs release, optimization mode. we may want to quantify test time like
  this too. basically we should start recording this so we can build statistics
  and gain more insights into how to optimize compile time in maybe non obvious
  ways."*
  ⭐ **the operative word is STATISTICS, and it changes the deliverable.** D8 is a
  gate — one number, pass or fail. This is a TIME SERIES with enough dimensions
  to regress against: per-unit wall time, LOC at the time of that compile,
  profile (debug/release) and opt-level, and carve lineage so a module that moved
  can be followed across the split that created it. Nobody can name the
  non-obvious win in advance; the point of recording the dimensions is that the
  data names it.
  ⚠ **the schema is the deliverable, and it is the part that is expensive to get
  wrong.** Rows recorded without a dimension cannot be back-filled — a year of
  measurements with no opt-level column simply cannot answer an opt-level
  question. So the columns land BEFORE the collector, even for dimensions not yet
  populated.
  ⚠ `cargo build --timings=json` is the per-unit source and it already exists;
  ADR 0013 prescribes it "quarterly", which is why nobody runs it. The lineage
  half has no source at all — git can approximate it with `--follow`, but a carve
  should RECORD what it split from, at the moment it splits, because that is the
  one time anybody knows.
  ⛔ **collection requires real builds, so it cannot run beside other cargo
  work** (`compile_cost.py`'s own docstring: a warm no-op read 222s because two
  builds shared a target dir). Sequence it, or give it its own
  `CARGO_TARGET_DIR`.
- ✔ **D13 BOTH DISCHARGED.** E1a is a guard on `main` (`265ec780b`,
  `crossing_a_room_boundary_leaves_no_repeating_unclaimed_population`), probed
  red against Jon's own log. E1b is answered: **zero** ignored tests hide a red.
  ⭐ **so `queue-72h-2026-08-06.md` is now archivable** — nothing open remains in
  it. It still has six inbound references from other planning docs, so archiving
  means updating those, and this repo's convention is partial
  `*-closed-sections.md` files rather than whole ledgers. Worth 1,229 lines
  against a budget 3x over, but it is a link-rewrite, not a `git mv`.
- ~~▢ **D13 two rows ORPHANED between runs, carried forward from `queue-72h-2026-08-06.md`.**
  That ledger is retired and 1,229 lines long, and the current header says to
  read it for standing state rather than open rows — so its 24 `▢` marks are 24
  false leads for anyone grepping. ⛔ **it must NOT be archived yet**, because
  two of those marks are real work nobody carried over. (Most of the rest are
  the same marker-hygiene defect fixed in this file today: `▢` left on a row
  whose own text says ✔ — F2 is the clearest example.)
  * **E1a — no test that a drop dies with its room.** From Jon's 2026-08-05
    playtest: an enemy drop was SESSION-scoped while its visual is a
    `RoomVisual` and therefore room-scoped, so a coin survived a room change and
    its picture did not — `no render family claimed coin:EnemySpawn-…`, eight
    per transition, forever, because no real visual was ever coming. Fixed in
    `dd73a3087` and ⛔ **the fix is UNGUARDED.** Needs a real room-unload
    fixture, not the hand-built worlds the drop tests use. ⭐ same family as
    *"drops outliving a reset"*, and the presentation-binding-fails-silently
    class generally.
    ⛔ **DESIGN NOTE, 2026-08-08 — do NOT write "a coin dies with its room".**
    That pins the fix and defends the gap: `dd73a3087` adds `RoomScopedEntity` to
    two spawn sites, and a test naming those two sites passes forever while the
    third site somebody adds next month reintroduces the bug. This repo has paid
    for that shape before (*a guard that pins the FIX defends the gap*).
    ⭐ **assert the INVARIANT instead, and the symptom is the better observable.**
    The bug's signature is that `draw_unclaimed_feature_views` spawns a stand-in
    for a view nothing will ever draw — **every transition, forever**, because no
    real visual is coming. So the property is: *crossing a room boundary must not
    leave a repeating unclaimed-view population.*
    ⛔⛔ **AND MY PRESCRIBED ASSERTION WAS WRONG — it would have been GREEN WITH
    THE BUG.** I specified *"count stand-ins across two consecutive transitions;
    a number that does not return to its baseline is the defect"*. **The
    population does not ratchet.** Stand-ins are `RoomVisual`, so they die with
    the room and are re-spawned: the count sits flat at **8 → 8** across every
    crossing, with and without the fix. What repeats is the **IDENTITY**, not the
    quantity.
    ⭐ so the load-bearing clause is **set-disjointness** — `before ∩ after` must
    be empty — and the count clause compares against the DESTINATION room's clean
    baseline (0), never the source's. Caught only because the guard was probed red
    first; a count-based guard would have passed its own motivating case, which is
    precisely the failure this repo has a standing lesson about and which I quoted
    in the brief while specifying it.
    ⚠ that also catches the class rather than the instance — any entity whose
    visual is a `RoomVisual` but which is itself session-scoped. And it is what
    Jon actually saw: eight `no render family claimed` per transition, and a black
    screen for the full 8-second cover deadline, because the cover WAITS on those
    stand-ins.
    ⚠ **and it must be probed red first** — revert the two `RoomScopedEntity`
    lines, watch it fail, restore them by hand (⛔ never `git checkout --`).
  * **E1b — how many `#[ignore]`d tests are hiding a red?** ⛔ **the row said 30
    and the real number is 14** — the 2026-08-07 count matched doc-comment
    MENTIONS of `#[ignore]` alongside actual attributes, and this file is full of
    prose about ignoring things. `grep -E '^\s*#\[ignore'` is the honest query.
    ⭐ **and the reasons classify them, which narrows the job from 14 to 3.** An
    `#[ignore]` that ASSERTS NOTHING cannot hide a red, and six say so in their
    own reason string — three *"audit listing: … read it, do not assert on it"*,
    two *"measurement, not an assertion"*, one *"diagnostic trace"*. Three more
    are conditional tools (*"run when the oracle above is red"*), and two are
    deliberately retired with a named replacement (*"route tuned to 1-1's old
    arrangement; replaced by a fixture course (queue G1 PICK 11)"*).
    ⭐⭐ **that leaves exactly THREE bare `#[ignore]` with no reason at all**, and
    a missing reason is the whole tell — nothing records what would make it
    expire:
    * `game/ambition_app/tests/hall_scale_spread.rs:63`
    * `game/ambition_app/tests/enemy_body_scale.rs:46`
    * `game/ambition_demo_mary_o/src/ldtk_migration_tests.rs:134`
    ⚠ **the first two are the sprite-scale instruments**, which is not a
    coincidence worth ignoring: they measure exactly the quantity Jon's open bbox
    decision turns on. If either is red, it is red about that.
    ✔ **ANSWERED 2026-08-08, and the answer is ZERO — no cargo run needed.**
    ⛔ **and my own three-test shortlist was wrong the same way the row was**: I
    inferred from the ABSENCE of a reason instead of reading the tests. All three
    are `print_*` instruments that assert nothing —
    `print_how_tall_every_character_stands`,
    `print_enemy_bodies_against_the_player`, `print_what_the_file_authors`. A
    missing reason string is not evidence of a missing reason. *Ask the tool,
    don't model it* — twice in one row.
    ⭐ **so all 14 are accounted for**: 9 non-asserting (6 self-declared, 3 found
    by reading), 3 conditional diagnostics that run when a named oracle is
    already red, 2 retired routes. **Nothing is hiding a red.**
    ✔ **the two retired routes' replacement EXISTS and runs**:
    `course_playthrough::the_session_enters_the_fixture_course_when_asked` on
    `test_course()`. And the gap it left was found and closed independently —
    `level_circuit.rs:88 grabbing_the_authored_pole_carries_you_out_of_the_level`
    covers the join that actually broke (Jon, 2026-08-05: *"you can keep playing
    after you hit the flag"*), which had **two green tests either side of it and
    none on it**.
    ⚠ **the one honest residual, and it is deliberate**: nothing walks 1-1 for
    real any more. `level_circuit.rs:81` states the reasoning — *"the honest fate
    of a route test: it rots every time the level moves, and then it is switched
    off and covers nothing"* — so the pole's position is read from the level and
    only the arrival is faked. Recorded as a choice, not a gap.
    ⭐ **worth running anyway, for a different reason**: `hall_scale_spread` and
    `enemy_body_scale` PRINT the Hall spread and the enemy-vs-player ratios. Those
    are the numbers Jon's open bbox decision turns on, and no one has read them
    since the D3 measurement. Cheap, and it is evidence rather than a gate.
- ✔ **D14 audited all 25 absence contracts for the *can it fail* defect — all
  25 are HEALTHY.** Not a queued row; done because this repo's standing lesson
  is that a guard whose target moved passes forever, and a ratchet nobody has
  re-examined is exactly where that hides. The audit asks two things the checker
  itself cannot: does each contract's path still contain files, and does its
  pattern still match SOMEWHERE in the repo (a pattern matching nowhere is
  guarding a shape that no longer exists).
  Result: 15 absence contracts scan 221–1,940 files each with live patterns; 6
  dependency contracts all name existing crates; 4 hand-written ones
  (`capability-footprint-may-not-grow` at 40 crates / 15 unwanted,
  `rollback-wire-format-is-frozen` at 347 names / 83 types, and the two SDK
  allowlists) are live.
  ⛔ **I raised TWO false alarms doing it, and both are the same error the audit
  exists to catch — modelling the tool instead of asking it.**
  1. *"`the-worlds-path-is-confined-to-ldtk-paths` scans nothing"* — I globbed
     `**/*.rs` at a directory of **Python**, and its `paths` are git pathspecs
     (`:(exclude)…`) the script feeds to `git grep`, not a Python glob. Asking
     git: 221 files, pattern matches 0 outside the exclusions and **twice inside
     the declaration file**, which is precisely the proof it can still fire.
  2. *"the two SDK allowlists are vacuous — `0 of 0 baseline modules still
     named`"* — that message reports invariant 2 (`baseline ⊆ named`), which IS
     vacuous when the baseline is empty. Invariant 1 (`named ⊆ allowed ∪
     baseline`) is the live one and still fails on any new non-SDK module. **An
     empty baseline is the success state**, not a disarmed check — the API 1.0
     campaign drove it 18 → 0.
  ⭐ the reusable part is the QUESTION, not the result: *would this check fail if
  the thing it forbids came back?* Ask it of a guard before trusting a green.
- ✔ **D15 the "parallel agent" contending with the timed builds was THE GOAL
  GUARD, driven by my own reporting cadence.** The collector reported another
  agent holding `cargo test`/`cargo check` on the default target dir all session
  at load 14–18 on 8 cores, confirmed by `/proc` rather than inferred. No such
  agent existed. `long-run-72h-2026-08-08-goal.json` runs
  `cargo check -p ambition_app` AND `cargo test -p ambition_app --test app_it`
  (318 tests, ~150s) as Stop-hook checks, on `/home/joncrall/ambition-target`
  (`.cargo/config.toml:11`) — **every time a turn ends.** The collector's run
  spanned roughly fifteen of mine.
  ⛔ **so the supervisor's own reporting frequency is the throttle on measurement
  quality, and nothing in either system knows about the other.** The guard is
  doing exactly its job; the collector is doing exactly its job; they simply
  share a machine and a target dir. Two identical dev rebuilds differed 12%
  (187.7s vs 210.5s) and one crate read 46% apart between them.
  ⭐ **what survives contention and what does not**: ratios, rank orders and
  correlations survive — every conclusion the collector drew is one of those.
  **Absolute seconds do not**, and they are the numbers a future reader will be
  most tempted to quote.
  ⛔ **SECOND INSTANCE, and this one corrupts a CORRECTNESS signal rather than a
  timing one (2026-08-08, same day).** The guard reported *"the app integration
  suite is green — ▢ FAILED"* while an agent was mid-probe: it had added
  `room_boundary_unclaimed_views.rs`, wired it into `app_it.rs`, and — **as
  instructed** — reverted the fix it guards so it could watch the test go red.
  The guard ran `app_it` against that live tree and reported a repository-level
  failure.
  ⚠ **so the guard's red is not always the repository's red.** The standing rule
  *don't run the suite while editing — every job reads the LIVE tree* applies to
  the GUARD too, and the guard cannot know an agent is editing. A supervisor who
  trusts that signal chases a phantom; one who ignores it misses a real break.
  **The tell is `git status`** — a dirty tree at the moment of the check means the
  verdict is about a tree nobody intended to test.
  ⚠ **the practice this implies**, recorded because it is not obvious and costs
  a whole campaign to rediscover: a timed-measurement run needs either a quiet
  window with the goal DISARMED, or a supervisor that stops reporting for its
  duration. The collector's own mitigation — sampling `getloadavg` and counting
  foreign cargo by `/proc/<pid>/comm`, never `pgrep -f` — is the right floor: if
  you cannot get a quiet machine, **record the contention onto every row** so a
  later reader can tell a slow machine from a real regression.
- ✔ **D16 REFUTED, within the hour, by the subtraction test.** Baseline vs
  domains-removed, `cargo clean -p` + `cargo check` on a quiet machine:
  **2.52 / 2.58 / 2.52 s** against **2.59 / 2.42 / 2.44 s**. Removing 2,130 lines
  and 74 of 79 generic functions changes the runtime's check time by nothing.
  The frontend half of the convergence is dead; the dependency isolation and
  C7's blocker are grep-measured and stand.
  ⚠ **bounded**: `cargo check` reads 2.5s where the timed build attributed 24.8s
  of frontend — a 10x gap, so `check` is not the build's frontend phase. What is
  refuted is *the generics dominate type-checking*; whether they dominate the
  build's frontend is still open and still wants `-Z self-profile`.
  ⛔ **and it took FOUR attempts to measure, three of them invalid** — each
  failing the same way: I read a number without asking whether it was plausible.
  `touch` + `cargo check` reported **0.64s** for a 14,747-line generic-heavy
  crate and I ran a second experiment on it before noticing. Then a real content
  edit gave 0.77s — still fresh, because incremental is ON, so I was timing the
  cost of adding one function. Only `cargo clean -p` forces the full recheck the
  question needed. ⭐ **the tell was available immediately: 0.64s is not a
  plausible answer to "how long does this crate take to type-check".**
- ~~▢ **D16 CONFIRM (or refute) that `rollback/domains/` is the frontend cost —
  needs per-module attribution cargo cannot give.** Three findings converged on
  those 2,130 lines today (dependency leak, frontend cost, C7's refactor
  blocker), and the convergence is written into
  `engine/actor-monolith-decomposition.md`. ⛔ **one link in it is INFERENCE.**
  * **measured**: `ambition_cutscene` appears in exactly one runtime file
    (`domains/cutscene.rs`, 5 refs), `ambition_items` in exactly one (4 refs);
    211 of ~222 `rollback_component_*<T>` call sites are inside the runtime;
    the runtime costs 24.8s frontend vs the monolith's 23.9s from 7.6x fewer
    lines; 74 of its 79 generic fns are in `rollback/`.
  * **inferred**: that the generics *cause* the frontend cost. **Cargo's unit is
    a crate, so it cannot attribute time below one.**
  ⭐ **the instrument is nightly `-Z self-profile`**, which the collector
  explicitly skipped and named as the gap. It attributes to query and to
  generic instantiation, which is exactly the join that is missing.
  ⛔ **my first draft of this row proposed a falsifier that does not work**, and
  the error is worth keeping because it is the same one the row is about: I
  suggested comparing frontend seconds between `rollback/` and `room_transition/`
  — **which requires exactly the sub-crate attribution the hypothesis needs.** A
  falsifier that needs the instrument you lack is not a cheaper test.
  ⭐ **the falsifier that DOES work needs no nightly toolchain: subtract the
  module and re-check.** `cargo check` is frontend-only, so:
  1. `cargo check -p ambition_platformer2d_runtime` — baseline, warmed;
  2. `#[cfg(any())]` out `rollback/domains/` (or the whole `rollback` module) and
     re-check, ignoring the resulting errors elsewhere — **the check TIME is the
     measurement, not its exit code**;
  3. if the frontend time falls roughly in proportion to the generic surface
     removed, the hypothesis survives; if it barely moves, it is dead.
  ⚠ warm first with the identical command, and ⛔ restore by writing the bytes
  back, never `git checkout --`.~~
  ⛔ **do not start a carve on this before the measurement.** Three findings
  agreeing is a reason to measure next, not a substitute — and this campaign has
  already had three premises die on contact with measurement, always in the
  direction of the tidy answer being wrong.
- ✅ **D17 A DROPPED COIN WAS A MAGENTA BOX — FIXED (`0146789`).** The diagnosis
  held on re-checking: `rebuild_dynamic_feature_views` REQUIRES a `SpawnOrigin`
  on a pickup and all three drop spawners stamped none, so every drop fell out
  of the query, no family claimed it, and the floor drew its stand-in.
  `proving_grounds` settled at 0 clean and **8 after seven defeats**; it is 0
  after. Confirmed through the real render stack too — `capture_scene`, same
  command and seed, logs ``no render family claimed `coin:EnemySpawn-5910`
  (Pickup)`` before and nothing after.
  ⭐ **the fix is provenance, not a marker.** A drop states its parent's `SimId`,
  READ off the dying body rather than spelled from its id, with a DERIVED
  sequence (coin 0, heart 1, ability 2) for the reason `SimId::strike_volume` is
  derived — and because a construction-built body carries a `SimId` and **no
  `SimIdCounter`**, which was probed rather than assumed. No `SimId` is minted on
  the drop: that would enrol it in `TransactionBaseline::capture`, whose roster a
  room-scoped entity leaves mid-transition.
  ⚠ **a second bug was underneath it**: the dynamic-view row hardcoded
  `EntitySprite::PickupCurrency`, so the first thing a drawn heart would have
  done is wear a coin. Now resolved from the pickup's live kind.
  ⚠ **nobody reported the magenta box**, which is still worth asking about — Jon
  reported the black screen (the cover waiting on stand-ins) and not the coins,
  so either the stand-in reads as intended art or drops are rarely seen.
  E1a's fixture now asserts it, stated against the ids the death path MINTED
  rather than a count, and probed red by hand first (8 of 12 undrawn, while the
  four split offspring in the same set were drawn — so it discriminates).
- ✅ **D18 A1.5 — continuity across a room transition. CLOSED 2026-08-08, and
  three quarters of it was already closed when it was queued.**
  The mark carried from [`../archive/queue-24h-2026-07-25.md`](../archive/queue-24h-2026-07-25.md) was
  *"continuity: score, coins, lives, and worn power across the transition. Assert
  against the BODY, never the emitter's bookkeeping."*
  ⛔ **"grepped before queuing and nothing covers it" was WRONG.**
  ⚠ **and the CAUSE recorded here first was itself wrong, which matters because
  the two imply different fixes.** It said the grep *"never opened
  `game/ambition_demo_mary_o_app/`"*. It did — both searches named that directory
  explicitly. **They were truncated with `head -8`**, and
  `the_run_survives_the_crossing` sorted below the cut. So the preventive lesson
  is *do not truncate a search whose question is ABSENCE*, not *search the right
  directory* — the latter would not have helped.
  ⭐ **the row's own section is still a good second signal**: A1.5 sits under
  *"A1. A demo can have two rooms + Mary-O World 1-2"*, beneath A1.3 and A1.4.
  Score / coins / lives / worn power are not engine vocabulary — they are the four
  readouts of **Mary-O's HUD** — so the guard was always going to live in the demo
  crate, and a reader who noticed that would have looked there deliberately
  instead of relying on a repo-wide sweep.
  ⭐ **`the_run_survives_the_crossing` (`beaa9f5b7`, 2026-07-25 — the same day the
  row was written) already held coins, lives and score**, and is green. It reads
  the wallet off the body and the run state off the mode owner, on both sides of
  a played crossing. The row was three-quarters answered within hours of being
  filed and the ▢ simply never moved.
  ⭐ **the ONE real gap was `worn power`, and that test's own note named it**:
  *"NOT covered yet: crossing while GROWN … a set-up equip would prove the
  transition preserves something a player never obtained."* Now closed by
  `she_crosses_wearing_the_form_she_earned` in the same file: the wand is knocked
  out of 1-1's authored ?-block by a real head contact (`bonk_power_blocks`
  mints it) and claimed by the engine's touch-to-collect, so nothing in the test
  writes worn state; then she walks the pole into 1-2 and the row is read back
  off `WornEquipment` **on her body**.
  ⭐ **probed red at the engine seam and it DISCRIMINATES.** Clearing
  `WornEquipment` on the transiting body in `room_transition/commit.rs` (three
  lines, restored by hand) failed the new guard with the form named — while the
  other four tests in the file, `the_run_survives_the_crossing` included, stayed
  green. That is the proof the gap was real rather than assumed: the coins/lives/
  score guard cannot see a stripped form.
  ⚠ **where the four actually live, since the row's instruction turns on it.**
  Coins are `BodyWallet` and worn power is `WornEquipment` — both **components on
  the body**, both asserted there. Score and lives are `MaryOLevelState`, a
  component on the **mode-owner entity**, not on the body: a run-scoped rules
  object that survives a room change and dies with the mode. That is a legitimate
  authority (the HUD readout is what it publishes, and the readout is what the
  instruction rules out), but it is *not* the body, and the row's phrasing
  presumes a body that does not carry it.
  ⚠ **and the main Ambition route has no score, no lives and no worn power at
  all** — grep `game/ambition_content/` for any of the three. Its HUD is HP / MP /
  `$`, and `$` is the same `BodyWallet`. A continuity guard on hub ↔
  `proving_grounds` could therefore only ever cover coins, over the same engine
  commit path Mary-O's crossing already exercises — so one was not written.
  ⭐ **[`../archive/queue-24h-2026-07-25.md`](../archive/queue-24h-2026-07-25.md) is archivable now**: A1.5 was its last open mark
  and is marked ✅ there.
- ✔ **D19 DONE — one slice promoted, and it found a live combat bug on the way**
  (`614f098f2`). `ambition_combat::hitbox::StrikeVictim` is derived `QueryData`,
  iterated by both `apply_hitbox_damage` and `step_projectiles`; two victim
  tuples, a nested `Option<(..)>` arity workaround and **both** duplicate
  `victim_frames` lookups deleted. Promoted into `tracks.md` track 8 per the
  doc's own rule. ⛔ **it rejected the doc's nominated pilot with a reason worth
  keeping**: `resolve_camera_observation` has ONE call site, and every benefit
  `QueryData` promises is a benefit of a SECOND one — a view with one consumer
  is a rename.
  ⛔ **and my numbers were wrong a third time.** Real split **29 systems / 20
  kernels**, not 36/11 — a Bevy system **cannot take a `&T` parameter at all**,
  which is the reliable discriminator; mine counted `&mut MessageWriter` as
  proof. And *"17 systems ≥16"* was impossible on its face: **the ceiling is a
  hard compiler stop**, so 12 sit at exactly 16 and none exceed it — which
  invalidated "pick the worst offender" before I proposed it. `QueryData` was
  **4**, not 5.
  ⭐ **the finding: a bolt has never consulted `DamageableVolumes`** — filed as
  its own card in `tracks.md` and in Jon's observations, NOT closed, because
  retiring `strict_intersects` changes how projectiles connect.
- ~~▢ **D19 promote ONE card from `triage/bevy-system-parameter-architecture.md`,
  which has been unpromoted for two weeks while its thesis gets applied without
  it.** 566 lines, **zero inbound references**, header reads *"PROPOSED
  DIRECTION"*, and it carries a six-card migration plan whose promotion rule is
  written into it: a slice goes to `tracks.md` *"only when it can name the
  systems, invariants, performance measurements, and deletion target owned by
  that slice."* Nobody has promoted one.
  ⭐ **the pressure is real and has cost this week**: the Bevy param-panic class
  (B0001) is a RUNTIME failure — `update_doppler_music_visuals` in the TwinTrack
  merge compiled and panicked on first run, and `capture_scene` is on record as
  that class's unrun probe.
  ⭐ **re-measured 2026-08-08, and the one number that matters moved the wrong
  way**: the doc's first recommendation is *"use derived `QueryData` to name
  stable entity views"*.
  * derived `SystemParam`: **35 → 40**
  * derived `QueryData`: **5 → 5**
  * `#[allow(clippy::too_many_arguments)]` sites: **126**
  **The recommended tool is untouched while the workaround grew by five.** That
  is the cheapest possible argument for promoting the `QueryData` card first.
  ⚠ one of the doc's own alarms is STALE in the good direction: `PlatformerPreparation`
  was *"itself at the 16-field ceiling"* and now has **9** fields.
  ⭐ **the parameter count, re-derived properly.** A first attempt counted `:`
  inside the paren span, so `::` paths and turbofish inflated it — **197
  parameters for one function**, which is how it was caught. A nesting-aware
  split on top-level commas (respecting `<> () [] {}` and strings, dropping
  `self`) gives numbers that are believable:

  | | count |
  |---|---:|
  | functions with **≥13** non-`self` params, production | **68** |
  | of those, **≥16** (Bevy's system ceiling) | **39** |
  | test/bench | 8 |

  ⛔ **CORRECTED — those two numbers conflate SYSTEMS with KERNELS, and the
  ceiling only binds systems.** Re-measured with comment lines stripped and
  Bevy-param types (`Query`/`Res`/`Commands`/`MessageWriter`/…) as the
  discriminator:

  | | count |
  |---|---:|
  | Bevy **systems** with ≥13 params | **36** |
  | plain **kernels / adapters** with ≥13 | **11** |
  | **systems at or above the 16 ceiling** | **17** |

  ⭐ **so the real pressure is 17, not 39** — and the worst offender by raw count,
  `handle_player_damage_events` at 28, is a `pub(crate) fn` taking
  `&mut MessageWriter<..>` by hand rather than a registered system. A long
  parameter list there is a design smell, **not a B0001 risk**, and the doc itself
  warned about this: *"many `#[allow(clippy::too_many_arguments)]` sites that mix
  healthy pure kernels, Bevy adapters, one-time setup, and genuine orchestration
  monoliths."* I counted the mixture it told me not to count.
  ⚠ **and `QueryData` — the card I proposed promoting first — does not help a
  kernel at all.** It names stable ENTITY VIEWS inside a system's `Query`. The
  17 systems are the candidate population; the 11 kernels want ordinary value
  structs (the doc's recommendation 3), which is a different card.

  Worst: `handle_player_damage_events` **34**, `reload_ldtk_world_from_disk` 32,
  `integrate_actor_body` 30, `apply_feature_hit_events` 29, `apply_actor_hit` 26.
  **31 of the 68 are in the actor monolith**, 15 in `ambition_app`.
  ⚠ **do NOT read 27 → 68 as growth.** The doc's population is unstated and mine
  is defined above; two different definitions are not a trend. What is defensible
  is the *shape*: 39 production functions sit at or beyond the ceiling Bevy
  enforces on systems, and the concentration is where the doc predicted.

  ✔ **D19 DONE 2026-08-08 — promoted *name the strike victim* into `tracks.md`
  track 8** (combat unification), landed rather than planned. `StrikeVictim`
  query data in `ambition_combat::hitbox` is now iterated by both
  `apply_hitbox_damage` and `step_projectiles`; deleted the 10- and 7-member
  victim tuples, the nested `Option<(..)>` arity workaround, and **both**
  `victim_frames` lookup queries. Gate + `app_it` + pytest + contracts all
  unchanged. The doc's header, executive conclusion and promotion recommendation
  now say what was promoted and why.
  ⛔ **THREE numbers in this row were wrong, including two I corrected once
  already.** Re-measured with a REFERENCE-typed parameter as the discriminator
  (a Bevy system cannot take `&T`, which is what actually separates a system from
  a kernel — `#[allow(too_many_arguments)]` and raw count do not):

  | | this row | measured 08-08 |
  |---|---:|---:|
  | Bevy systems ≥13 params | 36 | **29** |
  | kernels ≥13 | 11 | **20** |
  | systems at the ceiling | 17 | **12** |
  | derived `QueryData` | 5 → 5 | **4** → 5 |
  | `too_many_arguments` sites | 126 | **124** |

  ⚠ **"at or above 16" is not a thing** — the ceiling is a hard compiler stop, so
  all 12 sit at EXACTLY 16 and none can exceed it. There is no severity tail to
  rank; pick by which system owns a nameable concept.
  ⭐ **and `QueryData` was 4, not 5** — the recommended tool was even *less* used
  than this row claimed, which strengthens the row's own argument.
  ⭐ **the yield was not the parameter count.** `step_projectiles` stayed at 16
  slots. What naming the role found is that it **never consulted
  `DamageableVolumes`** — it tests the coarse box while melee and feature hits both
  ask `strike_reaches_victim` — while its comment claimed it shared "the SAME
  published hurtbox" as melee. The tuple that would have carried the silhouette had
  run out of arity. **A tuple at the ceiling does not fail to build; it fails to
  ask a question, and the comment above it goes on claiming the question is asked.**
  Filed as its own ▢ in track 8, since closing it is a combat behaviour change.
- ✔ **D20 FIXED 2026-08-08.** `SimId` is now `#[require(SimIdCounter)]`, so the
  pair is a property of the TYPE rather than something six mint sites each have
  to remember — the executor was one of the four that did not. The hand-pairings
  in `ensure_sim_id` and in Sanic's scattered rings are deleted with it.
  * **Guard**: `construction/tests.rs::a_boss_the_construction_executor_built_can_summon`
    builds the summoner through `RoomFeatureConstructionPlan::prepare` →
    `spawn_room_feature_entities_from_plan` → `commit_entity`, then summons.
    Probed RED before the fix: `left: []`, `right: ["placement:warden/0"]`.
  * **Live, in the real app** on `sandbox:basement_boss` (the LDtk room that
    ships `BossSpawn-0158`, brain `PhaseScript:clockwork_warden`), emitting the
    Minima Trap's own `EffectRequest`. Before: `counter=None`, no descendants, no
    "Puppy Slug". After: `counter=Some(0)`, `placement:BossSpawn-0158/0`, and a
    live body named `Puppy Slug`.
  * ⚠ **rejected**: repairing `commit_entity` alone (fixes one site of six) and
    widening `ensure_sim_id`'s `Without<SimId>` filter into a backfill sweep (a
    mop that contradicts its own charter and lands a tick late). Blast radius on
    rollback state: no registration moved, so the schema version and both
    baselines are untouched; every entity with a `SimId` now also snapshots an
    8-byte counter, and a required component is supplied only when ABSENT, so a
    restore that puts back `SimIdCounter(7)` keeps 7 and nothing double-mints.
  * ⭐ **the general rule stands and is now written into the test**: *a test that
    constructs its subject's preconditions by hand cannot detect that production
    never establishes them.*

  **The original write-up, kept because its reasoning is the durable part.**
  A construction-built body gets
  a `SimId` and **no `SimIdCounter`**; `apply_summon_effects` requires BOTH on
  the summoner and `warn!`s + `continue`s otherwise. So the summon silently does
  nothing.
  **Verified independently** (I traced the counter's producers, not the agent's
  reasoning):
  * `rollback/codecs.rs:147` — `ensure_sim_id` is filtered
    **`Without<SimId>`**, so a body that already has an id is skipped entirely
    and never backfilled with a counter;
  * `spawn_actors.rs:1763` — `let (Ok(summoner), Ok(counter)) = (identities.get(..),
    counters.get(..)) else { warn!(..); continue; }`;
  * production `SimIdCounter` inserts are **`ensure_sim_id`,
    `mint_spawned_sim_ids` (projectiles), and Sanic's rings** — and
    `construction/mod.rs` has **ZERO**. Its 16 mentions are 15 in `tests.rs`
    plus one doc comment on `summoned_minion_request` saying the sequence
    *"comes from the summoning body's own `SimId` and `SimIdCounter`"* — a
    counter nothing gives it.
  * the agent's live probe: `counter=None` on every authored enemy in
    `proving_grounds`.
  ⛔ **and the unit tests cannot catch it: they build the summoner BY HAND.**
  Verified — `construction/tests.rs:928` is
  `.spawn((SimId::placement("boss_1"), SimIdCounter::default()))`, and `:654` /
  `:688` do the same. **The fixture supplies exactly the component the real path
  omits.**
  ⭐⭐ **and the test's own header shows the author was worried about precisely
  this class and still missed it**: *"`apply_summon_effects` had no test at all
  before this. It is the only place the runtime-dynamic family actually reaches
  the world, so a change there could otherwise ride a fully green suite."* The
  test is thorough about the EXECUTOR and blind to whether anything can REACH it.
  ⭐ **the general rule, worth more than this bug**: **a test that constructs its
  subject's preconditions by hand cannot detect that production never establishes
  them.** Sibling of *a guard that pins the FIX defends the gap*, but distinct —
  that one is about which assertion you choose, this one is about what the
  FIXTURE quietly supplies. Ask of any hand-built fixture: *which of these
  components does the shipped path actually attach?*
  ⛔ **so the deliverable is a guard that builds the summoner THROUGH THE REAL
  CONSTRUCTION PATH**, not a fix alone. A fix with a hand-built test would leave
  the identical hole one component over.
  ✔ **THE LAST LINK IS CLOSED: IT IS LIVE.** `Effect::Summon` is constructed in
  production content at **`game/ambition_content/src/bosses/specials/gradient_sentinel.rs:604`
  and `:860`** — the Minima Trap, summoning a *"Puppy Slug"* minion. That boss is
  shipped: it has a sheet (`boss_sheets.ron:134`), a cutscene intro
  (`boss_intro_gradient_sentinel`), and a second encounter derived from it
  (`clockwork_warden.ron`). **So a shipped boss's summon warns and does nothing.**
  ⛔⛔ **AND I NEARLY RECORDED THE OPPOSITE, from `head -6`.** My first search for
  production `Effect::Summon` sites printed six lines — two doc comments and four
  test constructions — and I concluded *"constructed only in tests, so this is
  latent."* **The gradient sentinel's two sites were on lines 7 and 8, below the
  cut.** ⭐ **`head -N` on a grep converts PRESENCE into apparent ABSENCE**, and an
  absence is exactly what a "is this reachable in production?" question is looking
  for — so the truncation lands on the side that ends the investigation. Sibling
  of the standing rule that a suspiciously round zero is a tool result rather than
  a measurement. **Count first (`| wc -l`), then truncate.**
- ⭐ **D21 PROMOTED TO A MAINTAINER DECISION** (`awaiting-maintainer-decision.md`,
  *"Should an AUTHORED id be unspellable by a runtime spawn?"*). The fix that
  matches the other three identity closures — make the wrong thing
  unrepresentable — is a `PlacementId` newtype across **~70 call sites**, which
  is a cost Jon would notice. ⛔ **it stays there, not here**, so the ledger and
  the decision file do not both claim to own it. Original row kept below for its
  measurement.
- ~~▢ **D21 `spawn_split_offspring` mints `SimId::placement(..)` for a RUNTIME
  spawn** — the authored namespace, which a runtime spawn categorically is not
  in, and no `SpawnOrigin`. Same gap D17 just closed for drops, one seam over.
  ⚠ **not player-visible** — offspring are claimed as staged actors, so they
  draw. It is an identity-provenance defect, and `SimId::as_str`'s own doc
  forbids exactly this spelling shortcut.
  ⚠ **narrowed by D20**: the offspring now carries a `SimIdCounter` (`SimId`
  requires one), so it can be descended from. What is still wrong is the
  NAMESPACE and the missing `SpawnOrigin` — the whole of what this row asks for.
- ✔ **D22 THREE ROWS THAT READ AS OPEN ARE SITTING IN THE ARCHIVE — all three
  settled, none of them owed work.** Surfaced by
  `scripts/tests/test_planning_pointers_are_live.py` on its first run:
  * `archive/queue-24h-2026-07-26-closed-sections.md:2557` — **AE6**, *"match
    rules are still borrowed globals, not session-owned tuning"*;
  * `archive/queue-72h-2026-07-31-closed-sections.md:1259` — **S49**, *"a
    BODY-LOCAL vector copied BETWEEN two bodies"*;
  * same file `:1968` — *"milestone 5 is NOT reached, and here is the number"*.
  ⚠ **those two archives are IMMUTABLE by their own contract** (*"Nothing here is
  edited"*, moved verbatim and losslessly), so the marks may not be stripped and
  the checker excludes those files by name. **The question is whether the work
  was carried forward, which no check can answer.**
  ✔ **CHECKED 2026-08-08 — and the base rate for ARCHIVED rows turns out to be
  different from retired-ledger rows.** None of the three is orphaned work
  (⚠ the third was written up as *"GENUINELY UNRESOLVED"* on a reading and was
  then measured fixed — see its own bullet):
  * ✔ **AE6 — LANDED.** It asked to *"project a resolved combat tuning from the
    mode/session BEFORE the rollback session starts, and let the stage read it
    instead of writing the world's."* `ResolvedCombatTuning` now spans **13
    files** with its own test (`tests/resolved_combat_tuning.rs`), and there are
    **zero** `insert_resource`/`ResMut` sites for `di_max_angle`/`FriendlyFire` —
    the borrow-and-put-back pattern the row described is gone.
  * ✔ **S49 — DELIBERATELY DEFERRED, and documented better than most closed
    work.** The copy is still there (`mount/mod.rs:270`,
    `mount_frame.locomotion = rider_frame.locomotion`) — and the site carries the
    whole argument: why it is sound today (`sync_riders_to_mounts` zeroes the
    rider's gravity SCALE, not its direction, so both bodies resolve the same
    frame), the case that breaks it (a surface-walking mount), the evidence it is
    unreachable (*"the only two authored `mount_class` archetypes are the shark …
    and the giant"*, checked 2026-08-01), the trigger that makes it live (a mount
    with a crawler/adhesive motion model), the fix (convert rather than copy), and
    the words **"Queue S49"**. ⭐ **a deferral that names its own trigger at the
    code site is not a dropped row** — this is the shape the repo wants.
  * ✔ **milestone 5 — FIXED, MEASURED 2026-08-08, and every premise I queued it
    with was wrong.** The row said *"the pad's DPadRight moved BOTH — 14.19px on
    the keyboard player's fighter against 11.44px on the pad player's."* Driven
    now, on `527a619f7`:

        smash, keyboard + pad   pad drives DPadRight   own 175.17px   other 0.00px
        smash, keyboard + pad   keyboard drives Right  own  86.13px   other 0.00px
        versus, two pads        pad two drives         own  83.54px   other 0.00px
        versus, two pads        pad one drives         own  83.54px   other 0.00px
        versus, FOUR pads       each pad in turn       own  57.71px   other 0.00px ×3

    **Twelve cross-seat pairs, every one of them exactly 0.00px.** Not "below a
    threshold" — zero.
    ⛔ **"No test asserts that a pad moves only its own fighter" was false three
    times over**, which is what I get for searching names instead of running
    something. The tests are
    `smash_in_the_host::a_keyboard_player_and_a_pad_player_drive_different_fighters`
    (keyboard + pad, both directions — S41 wrote it on 08-01, in the same archive
    file, 17 lines below the ▢ I carried),
    `versus_stage::two_controllers_make_versus_a_two_player_game` (two pads), and
    `versus_stage::four_pads_each_move_their_own_fighter_and_nobody_else_s` (four).
    ⭐ **so no guard was added: the guard already existed and was already green.**
    ⚠ **the confound was checked rather than assumed** — a seat reporting 0.00px
    because it is dead reads identically to one that is isolated. Driving the pad
    again after a settle returns 175.17px on its own fighter and 0.00px on the
    other, so both bodies are live at the moment of every measurement. (Measuring
    WITHOUT the settle reports 17.21px of leak, which is momentum decay from the
    key release, not crosstalk. Worth knowing before anyone re-probes this.)
  * ⭐ **what fixed it, and the timeline that explains why 08-07 disagreed.**
    `65d31c116` *"A mirror match was one fighter to every id-keyed index in the
    runtime"*: `realize_seat` passed the CHARACTER id as the body's identity, so
    two seats wearing one character were ONE body to the anti-clump slot board,
    `spawn_dynamic_feature_visuals`, `entity_to_id`, `faction_by_id` and
    `target_entity_by_id` — all `HashMap<String, _>` keyed on it. The site names
    this outcome itself: *"the same shadow the registration gap and the couch
    crosstalk came out of."* It was committed on the `review-fixes-2026-08-06`
    branch at 08-06 22:58 and reached `main` in the `b6ee30a1d` merge at **08-07
    12:09** — **ten hours after** the 02:14 re-measurement that recorded it still
    failing. Nothing regressed and nothing was mysterious; the measurement simply
    predated the fix by one merge.
    ⚠ **and the 08-07 note's own reading was right**: it argued from the SIGN
    (*"-50.05px against 40.34px … opposite directions with the presser going
    backwards is the signature of CONTACT"*) that this was push-apart, not shared
    input, and that the next measurement should be spawn separation. It was
    push-apart — two seats sharing one slot-board entry stood in one place.
    Spawn separation is 192px now, in both games.
  ⭐ **the lesson, which cuts against the framing I queued this with**: I wrote
  *"the base rate says check — two for two."* That base rate was from RETIRED
  LEDGERS, where a mark means work in flight. In an ARCHIVE a mark more often
  means a closed row nobody de-marked, or a deferral citing itself. Two different
  populations; I applied one's prior to the other.
- ✔ **D12 DONE — and the headline item was never what it said.** `AMBITION_START_CHARACTER=sanic`
  did not grant the wrong verbs; **the game did not start** (`a93fa707f`). Three
  static readings over three weeks — including this row's own groundwork — aimed
  at `ActionScheme` data, catalog order and a moveset overlay **that had been
  deleted 2026-07-05**. One probe settled it. Sanic runs 449px/60t against the
  protagonist's 265 and apexes higher; blink is the home body's own kit and is a
  product question. ⚠ the possession-aware-dialog item closed earlier; the
  remaining four are morph-ball, shrine+glider, kernel-guide patrol, and
  listener-side dialogue adaptation.
- ~~▢ **D12 tracks §9, Jon's own fix list — re-measure all six before working any.**
  `tracks.md:559`, sourced from `untracked/jonnotes-FIXES.md`. Queued because the
  goal ranks Jon's own reports above inferred work, and because **one of the six
  was already stale when checked**, which is the usual rate here:
  * ✔ **possession-aware dialog — DONE**, and not by this track. It landed as a
    side effect of the review campaign: `interact.rs:118` takes `speaker_id` from
    the SUBJECT body and `:138` takes `listener_id` from the interactable, with
    no `is_player` branch anywhere in the conversation path. ⚠ its second clause
    (*model listener-side adaptation*) is genuinely open — identity is
    possession-aware, but nothing makes an NPC say a different line because of
    who is wearing the body in front of it.
  * the remaining five, each still a CLAIM to re-measure: morph-ball worn
    presentation as the general transform/worn-identity rule (not a special
    case); shrine + glider sprite repair (the shrine mechanic itself is a stub);
    kernel-guide NPC peaceful patrol from authored brain policy;
    `AMBITION_START_CHARACTER=sanic` granting blink/fireballs while losing
    move/jump — *"fix as data/seams, not special cases"*.
  ⭐ **the sanic one is the cheapest and the most diagnostic**: it has a literal
  repro command, and "a persona grants the wrong verbs" is a per-character
  `ActionScheme` + host input-hookup question, which is exactly the seam the
  input-identity rule (A6) was written to protect.
  ⭐ **STATIC GROUNDWORK DONE 2026-08-08 — start here, do not re-derive it.**
  * **Sanic's row is NOT in Ambition's catalog.** `character_catalog.ron:341`
    says so explicitly: *"Sanic … is no longer authored here: that identity now
    belongs to the standalone Sanic experience PROVIDER (`ambition_demo_sanic`).
    A single character id has a single owning provider, so the Ambition launcher
    host surfaces Sanic by LINKING that provider … not by duplicating the row."*
    The row is built by `game/ambition_demo_sanic/src/provider.rs::sanic_authored_catalogs()`.
  * **so the bug is a COMPOSITION question, not a data typo**: the persona's row
    arrives from a provider whose PLUGIN may not be composed in the same breath.
    That matches the 2026-07-19 deep review's untested guess — *"likely the sanic
    character row grants defaults it shouldn't + missing control hookups outside
    the demo app; needs a trace"* — and it is still a guess.
  * **the mechanism to check first** is `overlay_character_moveset`, which by its
    2026-07-03 design *"overlays the character's authored melee/ranged/special
    onto the player kit"* so the player KEEPS its traversal kit. If move/jump is
    genuinely lost, either that overlay is replacing rather than overlaying, or
    the row's `default_action_set` gates traversal. **Both are one read away and
    neither has been read.**
  ⚠ still needs a real run — `AMBITION_START_CHARACTER=sanic` — because "which
  verbs does the body end up with" is a composed-app fact.
  ⭐ **RUN, AND FIXED 2026-08-08 — and every static guess above, mine included,
  was aimed at the wrong layer.** `overlay_character_moveset` does not exist (it
  was deleted 2026-07-05, and its NOTE is the only thing left); the catalog
  re-assembles on every fragment registration so composition ORDER was never the
  issue; `SanicExperiencePlugin` *is* in the shipped host. The actual defect was
  one layer up and much larger: **the selection never activated a session at
  all.** `PlatformerPreparation::prepare` failed `PREPARE_DEFAULTS_WORK_ID`
  (`retryable(false)`) for any effective starting character ≠ the experience's
  authored default, and returned before publishing — so the shell had no world.
  The same check had been deleted from its twin `prepare_platformer_content` on
  2026-07-29 ("A SELECTION IS NOT A DEFAULT"), but the twin runs at
  PREPARE_SESSION, downstream of this early return, so the 07-29 commit's own
  claim that it fixed `--character` was never true in a composed App.
  ⭐ **the general rule: a provider's authored DEFAULT and a session's SELECTION
  are two facts, and only ONE site may answer "does the selection resolve".**
  Fix + poison-checked regression: `app_it -- starting_character_selection`.
  Measured after: Sanic runs 449px/60t (vs 265), jumps 103px (vs 83), no
  `ChargesProjectiles`, empty moveset — so *fireballs* were already gone and
  *blink* is the home body's own traversal grant, not the persona's.
- ✔ **D10 DONE** — and it was one file, then it was more than the file. `capture_scene`
  had ALREADY composed the shell (08-06); what the migration dropped was
  `install_ambition_shell_visuals`, so the phone proxy photographed a void with a
  HUD on it for two days (`fb8755333`). Then the two-builder fork behind it was
  closed (`8bbbfc273`): **−288 lines of duplicated composition, 4 of 5 known
  drifts now structurally impossible**, plus `--show-window` (never worked),
  `--character` on route captures (ignored), the missing GGRS host, and route
  mode's silenced engine log. ⚠ 2 drifts remain possible and the row says which.
- ~~▢ **D10 (= tracks K2b.1) migrate `headless` / `rl_sim` / `capture_scene` to
  COMPOSE THE SHELL.** Picked as the lane-D item that makes the next ten
  cheapest, on the plainest possible argument: **the app has two ways to come up,
  and every future change has to be right in both.** `run_headless` and
  `Platformer2dSimHarness::build` compose `AmbitionGameSimulationPlugin` without
  the CLI, never insert `AmbitionShellHosted`, and get their world root at build
  time; everything a player runs goes through the shell. That fork is why
  `capture_scene` is on record as an unrun probe for the Bevy param-panic class —
  a composition nobody exercises is where a silent half-running engine hides.
  ⛔ **CORRECTED WITHIN THE HOUR — K2b.1 HAS MOSTLY LANDED, and this row's first
  draft was the exact staleness the ledger's own header warns about.** I wrote
  the paragraph above from `tracks.md` and a `grep` for `AmbitionGameSimulationPlugin`,
  which finds the *import* in a migrated file and reads as "not migrated". Then I
  read the files:
  * `headless.rs:132` — **migrated**, and the comment says so: *"K2b: the
    headless report runs the same host a player does … it used to compose the
    simulation plugin alone and take its session root at plugin-build time."*
  * `rl_sim/mod.rs:84` — **migrated**, *"K2b edit 2: the harness composes the
    SHELL, like every other entry … now that the publisher is gone."*
  * `capture_scene.rs:286` — **NOT migrated.** It still composes
    `AmbitionGameSimulationPlugin` directly.
  ⭐ **so the row is one file, not three**, and it is the file that matters most:
  `capture_scene` is the phone proxy AND the composition already on record as an
  unrun probe for the Bevy param-panic class. The last unexercised composition is
  the tooling one — which is the shape you would predict and nobody had checked.
  ⚠ **`tracks.md` K2b-ii is stale and should be corrected there too**, or the
  next reader re-derives this a fourth time. Its "two composers" framing is spent.

  ✔ **CLOSED 2026-08-08, and the third bullet above was itself stale in exactly
  the way this row warns about.** `capture_scene` migrated on 2026-08-06 in
  `9266bdca9`; `:286` is the `add_plugins` line, and the *composition call*
  twelve lines below it was already `compose_ambition_shell_host_booting_to(…,
  AMBITION_GAMEPLAY_ROUTE)`. The row said "read the composition call" and then
  cited the import site — the same mistake one level down.

  ⛔ **but the migration had eaten the room, and nothing said so.** K2b edit 5
  composed the shell and deleted the build-time session root; the Startup path
  that used to spawn a room's visuals had gone with the `direct_entry` gate (K2b
  edit 3), and **nothing installed `install_ambition_shell_visuals`** — the only
  thing that registers `SessionRoomVisualsPlugin` and
  `ambition_activate_session_visuals`, which are what draw parallax, static room
  visuals, signage and the LDtk spine on ACTIVATION. So for two days the phone
  proxy photographed a **void with a HUD on it**: exit 0, a valid 640x360 PNG,
  the player, an NPC, the HUD and the touch bezel all present, and no world.
  Everything that hangs off the SESSION drew; only what hangs off the ROOM was
  missing, so the image read as a dark corner rather than a broken composition —
  and the 08-06 migration was signed off on that image ("draws the robot, the
  HUD, the touch bezel and an NPC", which was all true).
  ⭐ **the payoff the row predicted, arriving one notch quieter than predicted.**
  This IS the param-panic class — a divergence between the composition a player
  runs and the one the tool builds — but it did not panic, because the missing
  piece was a SPAWN system rather than a `Res` reader. The two earlier instances
  of this same fork (`VisualQualityPlugin` 2026-07-31, `sync_portal_quality_budget`
  2026-08-04, both recorded in `plugins.rs`) *did* panic and were caught in
  minutes. The silent variant is the dangerous one.
  ⛔ **and the capture was reading the user's real save.** ROUTE mode is built by
  `build_visible_app`, which redirects audio away from the speakers and
  persistence away from `~/.local/share/ambition/` for every windowless host;
  ROOM mode assembles its own App and had neither. Visible in the two images: the
  money readout is **`$115` before and `$0` after**, because the before-run was
  reading a real player's save file. Fixed unconditionally rather than by render
  mode — `--show-window` is still a screenshot tool, not a player.
  ⚠ **what was CONCEDED**: room mode still spells the three composition steps by
  hand instead of calling `compose_ambition_gameplay_host`, and that is correct
  rather than lazy. That function is the SIM-ONLY shorthand — `build_visible_app`,
  the player's own path, spells the same three steps out for the same reason: the
  visible plugins must build between the simulation and the shell. Room mode
  cannot use `build_visible_app` either, because `StartRoomOverride` /
  `StartRoomMustResolve` / `StartingCharacterOverride` are consumed at
  PLUGIN-BUILD time by `init_sandbox_resources`, and that function builds the
  plugins internally. **So there is no composer a room capture can call**, and the
  fork survives — five bugs deep now (`--route` as a positional, the headless
  display surface, `--dev-overlays`, `--combat-overlay`, and the entire room).
  Closing it means giving `build_visible_app` a pre-simulation hook; that is the
  next real item here, not this one.

  ✔✔ **AND THE CONCESSION IS SPENT — same day.** The hook exists:
  `build_visible_app_with(render, shell_hosted, |app| …)`, one closure run after
  `AmbitionShellHosted` and before the simulation plugin builds, which is the
  deadline every composition input has. `build_visible_app` is that function with
  an empty closure, so the tree has exactly ONE visible builder and `capture_scene`
  calls it for both a room and a route. **−288 lines of duplicated composition,
  +36 of hook.**
  ⭐ **a closure, not a struct of known inputs**, deliberately: a struct has to
  enumerate the composition inputs, and the next one added elsewhere would not be
  reachable — the same "a caller cannot say this" hole one release later. The hook
  says *when*; the resources say *what*.
  ⚠ **four of the five drifts are now structurally impossible** — the display
  surface, the shell visuals, the audio/persistence redirect and the asset root
  are stated once and inherited. `--dev-overlays` and `--combat-overlay` are NOT:
  they are systems, and each mode still lists its own (a room needs the snapshot
  applier, a route needs camera adoption). That is the irreducible half; both are
  currently in both lists.
  ⭐ **deleted with the fork**, each of which was a copy that had already been
  wrong once: `desktop_asset_root` (the Z′14 `::`-for-`_` typo that pointed room
  captures at a tree with no sprites), `pin_the_clock` (the host pins the same
  1/60 for every windowless build), the guarded `HostGameplayPresentationPlugin`
  add, and `--show-window` — which had **never worked**: it opened a window and
  then `setup_capture_target` retargeted every camera to the offscreen image, so
  it rendered a blank rectangle for the whole run.
  ⚠ **what the shared builder brought, unasked**: room captures now run the GGRS
  simulation host, `serialize_frame_schedules` and the `game://` asset source,
  none of which the hand-assembled app had. Verified by capture, not by compiling:
  same command before and after (`central_hub_complex player 640x360 --include-ui
  --warmup 12`), both read. The room is present in both — floor tiles, three
  doors, the `military_tower_door` / `hall of bosses door` / `pirate cove door`
  labels, the authored *"Drop through the floor opening…"* signage, the parallax
  backdrop — the money readout is `$0` in both (the save redirect held), the
  subject pose is `(950.0000, 904.0000)` after 12 warmup ticks in both, and 27 of
  230400 pixels differ by more than 100: all of them the robot's foot, one
  animation step apart.
  ⚠ **and one thing it took away, put back on purpose.** `OffscreenGpu` disables
  `LogPlugin` — right for tests, which build several Apps per process, wrong for a
  binary that builds one — and that silenced every engine `INFO`/`WARN` a room
  capture used to print (`room 'central_hub_complex' has 38 neighbours…`, the
  encounter registry count, the presentation layout line). Re-added after the
  group, so both modes get it; route mode had been running blind since it was
  written.
  ⚠ two things are named `direct_entry` — `shell_host.rs:51` records its own as
  deleted, while `cli.rs:245,271,886` still carries a live `cli_direct_entry()`.
  Do not conflate them.
  ⛔ **sequencing**: `capture_scene` is in active use by the sprite and
  death-beat measurements. Do not migrate it while a capture-based measurement is
  in flight.
- ✔ **D7 MEASURED: no, and it never could have.** Animation **0.12 s** (one
  static frame, 120 ms, non-looping); beat **3.2 s** (`DEATH_DWELL`); music
  **3.200 s** exactly. The reset lands 3.08 s after the clip ends — 26.7×.
  Confirmed three ways: deterministic sim log, three real captured deaths, and
  in-beat screenshots.
  ⛔ **retracts the row's own hypothesis** — there is NO tumble to cut. The death
  animation is a single static frame, authored that way, so a 1.6 s beat could
  never have clipped a 0.12 s clip. The only thing 1.6 s cut was the music.
  ⛔⛔ **and it found an instrument that stopped measuring its question.** The
  freeze row's drift evidence no longer reproduces: its fixture kills Mary-O at
  `y = 4000`, ≥3500 px from every enemy, and both Mary-O enemies gained
  `AwakeNearObservers { radius: 720 }` AFTER that row was written. Every enemy is
  dormant for the whole fixture beat, so the test would now report a frozen world
  whether or not one existed. The conclusion survives on new evidence (eight
  in-beat captures show the slop moving), but the fixture needs to kill her NEAR
  an enemy before it means anything again.
- ~~**D7 does the death reset still cut the animation at 3.2s?** (obs:231) The
  code reading is SETTLED — the reset is correctly gated behind the beat, and the
  beat was raised 1.6s → 3.2s because the sting was being cut off. ⛔ do NOT do
  the "one reset at one time" refactor; the ordering was never the symptom. What
  is left is one death, watched once, which a capture can answer.~~

- ~~**D76 ⭐ THE LIMBED-HOST PREDICATE READS THE ROSTER, SO A GIANT CANNOT LEAVE
  `character_archetypes.ron`.** Measured 2026-08-11 by deleting the `giant_gnu`
  row: 18 red tests, `left: 1, right: 3` — "host + two hands". The construction
  planner decides which enemies lower into a host + two driven hand rows with
  `crate::features::spec_is_limbed_host(&roster.spec_for_brain(brain))`, whose
  whole body is `spec.mount_class.as_deref() == Some("giant")`. Three call sites
  (`construction/mod.rs:1530`, `:1605`, `:1730`) hold a placement and a
  `CharacterRoster` and no `PreparedCharacterRegistry`, so a giant that authors
  `CharacterMount { class: Some("giant") }` on its definition — `npc_giant_gnu`
  does, as of `bc7e02ab3` — is invisible to them and lowers as a handless host.
  ⇒ the fix is to thread the prepared registry (or a resolved
  `Option<&PreparedCharacterDefinition>`) to those three sites and have the
  predicate ask the CHARACTER first, falling back to the spec — the same
  character-first-then-catalog shape `new_character_in` already uses for
  `is_aerial`. ⛔ do NOT generalize the `"giant"` string while doing it; the
  predicate's own doc defers a data-driven flag until a second limbed mount
  exists, and that is still true. Everything else about the giant is migrated
  and pinned (`the_giant_gnu_authors_the_mount_its_archetype_row_used_to`).
  ⇥⇥ **UPDATED 2026-08-11 (`12ac7bd5d`) — TWO OF THE THREE LAYERS ARE DONE.**
  (1) `features::is_limbed_host(character, spec)` asks the CHARACTER first, the
  prepared cast is threaded to all three planning entry points, and a guard
  (`a_character_that_authors_a_giant_mount_plans_its_hands_without_a_row`) pins
  that a definition alone produces host + two hands with the roster answering
  `combatant`. (2) The ACTIVATION path built its `ActorConstructionContext`
  without the cast while `self.prepared_characters` sat two lines above it —
  measured `prepared=None` → `prepared=Some(35)`. (3) ⛔ **WHAT IS LEFT**: with
  both fixed, deleting the row now fails FURTHER ALONG, in relation
  verification — *"`placement:EnemySpawn-6836` is the mount of relation
  `ambition.mount` but is constructed as a `giant-host`, which cannot hold that
  end"*. Reproduce by (a) deleting the `"giant_gnu"` row, (b) adding
  `character_id: "npc_giant_gnu"` + `respawn: "OnRoomReenter"` to
  `EnemySpawn-6836` in `ambition_content/worlds/sandbox.ldtk` (the `respawn`
  fieldDef does not exist there yet — copy `disposition`'s, uid 6858; the parser
  at `entity_converters.rs:626` already reads it), then running
  `cargo test -p ambition_app --test app_it -- content_dormancy`. ⇥⇥ **CLOSED
  2026-08-11 (`35a80b485`)**: the third layer was `mount_capabilities_of`, which
  resolved `mount_class`/`pilotable_mount_classes` from the roster in every arm,
  so a mount whose row was gone read as "not a mount". It believes the
  character's mount block as a WHOLE now. `character_archetypes.ron` 707 → 678;
  `app_it` 327 passed, 0 failed. ⚠ pre-existing and unrelated, reproduced on a
  stashed tree: `ambition_demo_mary_o_app::level_1_acceptance::a_small_mary_o_dies_to_one_hit_and_the_level_restarts`
  is red.~~

- ✔ **D77 CLOSED 2026-08-12 — EVERY ROW IT SIZED IS GONE BUT ONE, AND THE
  "MISSING" SURFACE ALREADY EXISTED.** ⛔ its headline was wrong: `is_sandbag` had
  an authoring surface the whole time (`CharacterDefinition::practice_target`,
  doc-aliased to that exact name, with `ActorClusterSeed` already writing
  `is_sandbag: practice_target`). Grepping for the thing a row says is missing is
  what found it, four sessions after the row was written. ⇒ of the 14 rows this
  sized, TWO remain: `medium_striker` (one unnamed placement, a product question)
  and `combatant` (the fallback itself, which goes with the roster). The rest
  migrated or were deleted as unreachable. ⇥ the original measurement follows,
  unedited, because the numbers in it are how the distance was tracked.
  ⇥ **D77 ⭐ THE NEXT ARCHETYPE ROWS, SIZED — AND `is_sandbag` IS THE ONE
  MISSING AUTHORING SURFACE.** Measured 2026-08-11 after the giant, both shark
  riders and the giant's hands migrated (`character_archetypes.ron` 843 → 619).
  **14 rows remain.** Seven still have LDtk placements — `small_skitter`,
  `medium_striker` (9), `large_brute`, `gradient_seeker`, `sandbag_finite` (3),
  `sandbag_infinite` (2), `ranged_skirmisher` — and seven have none:
  `combatant` (the fallback itself), `cellular_automaton_fighter`,
  `player_robot`, `small_lurker`, `large_colossus`, `pirate_raider`,
  `pirate_heavy`.
  ⇒ **THE SANDBAGS ARE THE CHEAPEST REAL MIGRATION** and the catalog already has
  a `"sandbag"` character with its own sheet. Everything else on both rows is
  authorable TODAY: `never_dies` is `CharacterDeathTraits::never_dies`,
  `respawn: InPlace(0.85)` is a placement field (`sandbox.ldtk` gained the
  `respawn` fieldDef on 2026-08-11), and `body_contact_damage: false` means
  neither authors `contact_damage` — ⚠ note the row's own comment claims it
  "still deals light CONTACT damage if you walk into it", which the flag beside
  it contradicts; the flag is the gate, so the comment is stale and the migrated
  sandbag must not gain a contact hitbox.
  ⇥⇥ **THE FINITE SANDBAG LANDED 2026-08-11 (`a874ac502`)** — `practice_target`
  is on the definition, `InPlace(0.85)` is on its three placements, and the row
  is deleted (619 → 601). ⚠ **`sandbag_infinite` is what is LEFT of this row,
  and its blocker is not `is_sandbag`**: `never_dies` is a `CharacterDeathTraits`
  field, so the immortal dummy is a DIFFERENT creature from the finite one and
  needs its own registered character — and the catalog has exactly one
  `"sandbag"` row, so a second registered-only character would resolve no sheet
  (sprite lookup goes through `sheet_for_character_id_from_data`, whose
  manifest-by-id fallback has no `sandbag_immortal` manifest). ⇒ either add a
  second catalog row sharing the sandbag sheet (⚠ it would also appear as a Hall
  exhibit — a content change worth ASKING Jon about, not assuming), or make
  mortality placement-authorable, which is a third-authority decision and
  bigger. **Do not guess between those two.**
  ⛔ **the historical `is_sandbag` blocker, for the record**, a bool on `ArchetypeSpec` with four
  live consumers — `save_sync.rs:110` (excluded from the save), the path
  assignment at `actor_clusters.rs:497`, `actors/update.rs:1852` and
  `actors/conversion.rs:96` (sprite selection). `new_character_in` writes
  `is_sandbag: false` via `..Default::default()`, so a migrated sandbag would
  silently join the save file and change sprite. ⇒ author it on the definition
  (a `practice_target` intrinsic beside `locomotion`), NOT as a catalog tag: the
  plane-swarm lesson is that a body reading an intrinsic from a catalog row it
  cannot see gets the wrong answer in a standalone demo.
  ⚠ `pirate_raider` and `pirate_heavy` look unplaced and easy and are NOT: their
  live consumer is the PROVOCATION path (`actors/conversion.rs` picks a hostile
  archetype by NAME MATCHING — "broadside bess", "quartermaster", "lookout"),
  which is campaign item P2.20 and its own slice.

- ~~**D82 ✔✔ THE PROTAGONIST CARRIES ITS OWN MOVES — LANDED 2026-08-11.** All
  three reds are resolved: the persona overlay (a composition bug), the pogo (a
  ruleset split, `DeclaredCombatRules::downward_hit`), and the death fixture
  (repaired, not weakened — it stopped dying because the robot's jab got 14
  frames faster). `player_robot_v3` authors all eleven timelines; Smash declares
  `Spike` and Ambition keeps `Pogo`. Kept below for the measurement trail.**~~
  ⛔ **WAS: THE PROTAGONIST'S CANONICAL MOVESET IS WRITTEN AND CANNOT BE
  ATTACHED YET — the blocker is a PRODUCT question, measured 2026-08-11.**
  Jon's redirect §15 asks for the rich Smash move table to MOVE onto the
  reusable robot character rather than being copied into a second definition.
  The table is moved: `ambition_content::player_robot_moveset` holds all eleven
  authored timelines and `game/ambition_demo_smash` no longer owns the canonical
  copy conceptually. **Attaching it to `player_robot_v3` turned three `app_it`
  regressions red at once**, and they are three different problems:
  1. ✔ **FIXED — the bubble shield vanished.** `derive_persona_moveset` did
     `authored.unwrap_or(derived)`: an authored moveset REPLACED the body's
     folded kit wholesale, and the robot's shield and charge are host-kit verbs
     that live in that derivation. Authored moves now OVERLAY per verb, which is
     Jon's §18 shape (*capabilities + grants − restrictions*) one layer down.
     The first symptom to name itself was *"the folded special move started this
     tick"*.
  2. ⛔ **OPEN — the pogo.** `gravity_symmetry::pogo_bounces_away_from_gravity`
     goes red because the authored `air_down` is a SMASH SPIKE
     (`launch_dir: (0.0, 1.0)`, driving the victim down and ending a stock
     offstage) while Ambition's down-air is a POGO that bounces the ATTACKER up
     off whatever it hit. Same press, same geometry, two readings — and only the
     MODE can choose between them, which is exactly the split §16 describes and
     is not a line of wiring. ⚠ the direction is now PINNED
     (`the_down_air_is_a_spike_which_is_what_a_pogo_mode_has_to_reinterpret`) so
     a retune tells whoever changes it that another game reads it.
  3. ⛔ **OPEN — `rollback_lifecycle_reset::a_player_death_reset_survives_the_rollback_window`.**
     Unmeasured beyond "it goes red with the moveset attached"; do not guess,
     and do not assume it is the pogo's sibling.
  ⇒ **the shape of the fix**: a down-attack states its geometry once, and the
  RULESET says whether a downward hit pogos the attacker or spikes the victim —
  the same seam `DeclaredCombatRules` already carries DI and knockback growth
  through. ⛔ do NOT fix it by authoring the robot a second, Ambition-only d-air:
  that is the duplicate-moves outcome §16 explicitly forbids.
  ⚠ **the module is registered and guarded but NOT attached**, deliberately, and
  `player_robot_lineage::definition` says so at the seam. Attaching it is one
  line once the mode split lands.

- ~~**D83 ✔✔ THE `player_robot` ARCHETYPE ROW IS DELETED — 80 LINES, 2026-08-11.**
  556 → 476 lines, 11 → 10 rows. All three authorities separated: the BODY and
  the verbs onto the robot LINEAGE (shared by v0/v2/v3), the CONTROLLER into a
  published `robot_duelist` profile, the combat kit into a `robot_duelist_kit`
  catalog preset, the Hadouken onto a new `CharacterDefinition::ranged_vfx`, and
  the `theorem_chain` two-hit combo onto v2's moveset. Both readers go
  character-first. The two archetype-side tests were deleted with the row and
  their claims re-asserted on the character, with a note at each old site saying
  where the claim went.**~~
  ⛔ **WAS: half-migrated, 80 lines, one action set left.** The biggest single row in
  `character_archetypes.ron` and a textbook case of the three authorities fused:
  a BODY (health 60, run 200, `Walk`, contact 0.6/1, a movement patch), a
  CONTROLLER (aggro 560, attack range 60, patrol 0.55, `Smash`, dash-to-close,
  duelist neutral game) and a PLACEMENT policy (`respawn: OnRoomReenter`).
  **Migrated 2026-08-11:** the body onto the robot LINEAGE (shared by v0/v2/v3 —
  they are one robot at three ages), the controller into a shared published
  `robot_duelist` profile, and the duel arena's exhibition opponent now names
  `player_robot_v2` so it builds character-first.
  ⭐ **and it exposed a real defect on the way**: the character-first constructor
  granted `AbilitySet::NONE` unconditionally, on the reading that a MATCH
  declares what a fighter may do. True of a seated body, false of every other one
  — the duel robot came out unable to blink, shield or dash, capabilities its
  archetype had granted. It now takes the character's authored verbs.
  ⇥⇥ **UPDATED 2026-08-11 (second pass).** The kit LANDED as a catalog preset,
  `robot_duelist_kit` (the row's Swipe and Rock verbatim — a migration that
  retuned on the way would be two changes in one commit), and `player_robot_v2`
  names it. Both production readers of the archetype now go character-first: the
  duel arena and `player_robot_fights_player`.
  ⛔ **AND THE ROW STILL CANNOT DIE — two facts have nowhere to go.** Deleting it
  was attempted and REVERTED, because deleting content that has no home is how
  facts disappear silently:
  1. `ranged_visual: "hadouken"` — the projectile ART. `ActorTuning` carries it
     and the character-first constructor writes `String::new()`, so a migrated
     robot fires an unadorned rock. The catalog row already has `attack_vfx` for
     the melee; a `ranged_vfx` sibling is the obvious home and does not exist.
  2. `signature_move` `theorem_chain` — the two-hit combo, and the ONE thing in
     the repo proving the moveset expresses multi-hit combos as data across
     characters rather than as a PCA one-off. It would go on v2's moveset, which
     is free (v2 is not `HostCode` — that worry was about v3 and does not apply).
  ⚠ **the v3/HostCode worry recorded in the first pass is DISCHARGED**: the duel
  fields v2, and v2's catalog row carries no `playable_kit`, so it is an ordinary
  authored-kit character. Nothing about v3 blocks this.
  ⇥ **the original note, kept:** the row's ACTION SET has no home yet —
  `melee: Swipe(0.16/0.06/0.20, dmg 1, reach 34)`, `ranged: Rock(360, dmg 1)`
  with `ranged_visual: "hadouken"`, and the `theorem_chain` two-hit signature
  move. `game/ambition_app/tests/player_robot_fights_player.rs` is the regression
  that reads them: it spawns the robot as an ENEMY and expects a Hadouken opener
  into melee.
  ⚠ **the awkward part, and it needs a decision rather than a guess:** v3 is
  `playable_kit: HostCode` (its ActionSet is the progression kit) and already
  carries the Smash move table as its moveset. So the duelling NPC's repertoire
  is v2's, not v3's — which is defensible (the exhibition fields v2) but means
  authoring an ActionSet on a lineage member whose catalog row says the host owns
  it. ⛔ do not resolve that by giving v3 a second action set.
  ⇒ once the action set lands, the row is orphaned:
  `every_archetype_row_is_placed_somewhere_or_deliberately_code_selected` will
  say so, and `player_robot` leaves `CODE_SELECTED` with it. **80 lines.**

- ~~**D84 ✔✔ THE PIRATE HALF IS DONE — `pirate_raider` and `pirate_heavy`
  DELETED, 2026-08-11.** 414 → 355 lines, 7 → 5 rows. Nine characters state their
  own `provoked_profile_ref`; the matcher's two pirate arms are gone; THREE
  readers were found and closed (the provoke, the spawn-time combat kit, the
  dismount melee). Measured before deleting: no pirate-named placement in any
  world lacks a `character_id`, and a body built from a named character keeps
  that id through construction. The rows moved to the TEST fixture, where the
  giant's note already said a test's cast belongs to the test. ⇥ the PCA arm and
  the generic `combatant` fallback remain — see below.**~~
  ⛔ **WAS: PROVOCATION PICKS AN ARCHETYPE BY SUBSTRING-MATCHING A DISPLAY
  NAME — and it is the ONLY thing keeping three archetype rows alive.**
  `hostile_brain_id_for_actor` asks *does this actor's id, display name or
  dialogue node contain "pirate" / "cellular automaton"* and hands the body that
  archetype's tuning, HP pool and capabilities. That is the fused ontology at its
  most literal: a peaceful pirate that gets struck is given a different BODY
  rather than a different attitude.
  **Measured 2026-08-11 (the full placement census):** `pirate_raider`,
  `pirate_heavy` and `cellular_automaton_fighter` are placed in **ZERO** levels.
  Nothing but this matcher reaches them. Together they are **186 lines** of the
  414 left.
  ⭐ **LANDED this pass:** `CharacterDefinition::provoked_profile_ref` — the
  policy a creature adopts when provoked, resolved like every other profile
  reference — and `provoke_actor_in_place` prefers it, keeping the body exactly
  as the character built it and changing only the driver and the relationship.
  Both pirates author theirs (`pirate_boarder`, `pirate_boarder_heavy`, lifted
  verbatim off their rows' controller halves). The legacy matcher stays for
  bodies whose character says nothing, which is everything else.
  ⇥⇥ **MEASURED, second pass 2026-08-11 — and the answer is YES.**
  `a_body_built_from_a_named_character_remembers_which_one` builds a peaceful
  quartermaster through `ActorClusterSeed::new_in` with a level's LABEL ("Pirate
  Quartermaster") and a named character, and the seed's
  `sprite_character_id` comes out as the named id — so the branch's precondition
  holds on the archetype road too, which is the road every cove NPC takes.
  ⭐ and all NINE characters the matcher answers now state their own provoked
  policy (`every_pirate_answers_the_provocation_question_for_itself`), heavy and
  light split exactly as the matcher splits them. That is the content half done.
  ⇥⇥ **THE SECOND READER IS FOLLOWED, third pass 2026-08-11.** A peaceful NPC
  carries at SPAWN the combat kit it will fight with once provoked, and that was
  resolved by handing an archetype the NPC's DISPLAY NAME — so the cove pirates,
  who author a bolt, a swipe and a gun-sword, were handed `pirate_raider`'s
  instead. It now asks the character first and falls back to the matcher only for
  an NPC whose placement names no character.
  ⇥ **AND A THIRD READER EXISTS, found by grep rather than by reasoning:**
  `brain_builders.rs:301` rebuilds a DISMOUNTED rider's melee from
  `spec_for_brain("pirate_raider")` when its stored kit has none — the shark-rider
  dismount path. Plus four `mount/tests.rs` fixtures spawn `hostile("rider",
  "pirate_raider", ..)`. ⛔ that is the shape this row keeps having: each reader
  looks like the last one, and the census (*"zero placements"*) counts LEVELS,
  not code. Three readers found so far, two closed.
  ⇥ **superseded:** a live provoke, watched. The
  precondition is measured and the content is authored; what is NOT measured is
  the whole path end to end — `hostile_spec_for_actor` also feeds a provoked
  NPC's stored `CombatKit` at SPAWN time, before any of this runs, and that
  reader is untouched. ⛔ deleting the rows without following that second reader
  is how a provoked pirate ends up with the right mind and no weapon.
  ⇥ **superseded:** whether the
  cove's peaceful pirate NPCs actually carry `sprite_character_id =
  npc_pirate_raider` / `npc_pirate_heavy_iron_mary`. The new branch keys on that
  field; if those placements resolve their sprite by DISPLAY NAME instead, the
  authored policy never fires and deleting the rows would silently return every
  provoked pirate to `combatant`. ⛔ do not delete on the strength of the code
  reading right — instrument a provoke and watch which branch runs.
  ⇥ **THE PCA is the third row and needs its own decision:** it is a BOSS body,
  and `perfect_cellular_automaton` exists as a character. Its row also feeds the
  duel arena's exhibition fighter, which still says `character: None` with a note
  explaining why (*"naming the character here before it authors one would build a
  fighter with no melee"*). That note is the work item.
  ⇥ the generic arm (`"combatant"` for any other provoked NPC) is the FALLBACK
  and dies last, with the row.

- ~~**D85 ⛔ THE MATCH SEAT IS THE LAST CONSTRUCTION PATH THAT ASKS TO BE
  FINISHED**~~ **CLOSED 2026-08-11 — all five grants moved, the stamp went last.
  Original row kept below for the order it prescribed, which worked.**
  **D85 THE MATCH SEAT IS THE LAST CONSTRUCTION PATH THAT ASKS TO BE
  FINISHED — and it is harder than the other two, measured 2026-08-11.**
  Jon's second redirect, P0: ordinary construction must not carry
  `RecharacterizeBody`. The enemy road needs none (it grants and stamps), the
  protagonist needs none (its bundle already ran the overlay; the only gap was two
  projectile markers whose input the bundle computed and discarded). The SEAT
  still asks, and an attempt to close it was made and REVERTED.
  ⇥ **what the attempt did, and it was the right half:** `PreparedSeat` gained
  `match_kit` — the roster is in scope exactly once, at preparation, and carrying
  its answer forward is what lets a seat be built with a real repertoire instead
  of `ActionSet::default()`. Seating then stamped `PersonaBaseline` and granted
  the character's `CombatCapabilities`.
  ⇥ **why it was reverted:** stamping makes the derive skip, and the derive does
  MORE for a seat than for an enemy. Nine seat tests went red at once — the
  crawler's locomotion, the ability mask, the countdown hold, channel assignment,
  the authored-moves grant. Each is a separate thing the derive was supplying:
  `IdentityKit`, `CombatKit`, the moveset, the motion-model switch, the physical
  baseline (health/mass). ⛔ do not re-attempt by stamping first and fixing the
  reds one at a time — that is how a stamp ends up certifying work nobody does.
  ⇒ **the order that will work:** move each of those grants into seating FIRST,
  one per change, each with the seat test that names it going green on its own.
  The stamp goes LAST, when the list is empty. The enemy road was easy because
  `grant_prepared_character_body` already did all of it; the seat has no such
  function yet, and building one IS Jon's "one materializer" (redirect P0).
  ⚠ measured cost: nine tests, five distinct grants. Not a session's work, but
  not a single commit either.
  ⇥ **PROGRESS, one grant per change as prescribed:**
  * ✔ **the KIT** — `PreparedSeat::match_kit` carries the roster's answer forward
    from preparation, so `realize_seat` builds a real `ActionSet` instead of
    `default()`. Preparation is where the roster is in scope; only the derive
    could see it before.
  * ✔ **the DEATH TRAITS** — set on the seed's `caps` rather than inserted beside
    it. ⛔ the first attempt inserted a second `CombatCapabilities` into the same
    spawn bundle, which is a DUPLICATE COMPONENT: Bevy refuses the whole bundle
    and five seat tests went down at once. The component is already a cluster
    member; a grant that the bundle already carries has to be SET, not added.
  * ✔ **`IdentityKit` AND the moveset**, together, because they come from one
    call. ⭐ the OVERLAY now runs once, at PREPARATION, where the catalog is in
    scope — `realize_seat` has none and so could never have run it, which is
    precisely why the persona derive was doing it on the body's first tick. That
    is the "one materializer" shape in practice: called once per seat, not once
    at preparation and again a tick later.
  * ✔ **the MOTION-MODEL switch** — the seat derived its model from the SEED's
    tuning and the derive switched it to the character's a tick later, so a
    crawler seated as a fighter spent its first frame on the axis-swept solver.
    ⚠ the physical baseline was ALREADY on this road (`PhysicalBaseline::of(..)
    .apply_to_body(..)` in `realize_seat`) — the revert's list of five counted a
    grant that had landed, which is the ledger's own commonest staleness caught
    inside a row I wrote myself.
  * ✔ **THE STAMP, LAST** — `realize_seat` writes `PersonaBaseline { id,
    generation, displaced: Default::default() }` and `RecharacterizeBody` is
    GONE from the seat. Empty displacement is the assertion that matters: a
    replacement records what it displaced so it can retract to it, and a body
    BUILT as this character has nothing to retract to.
  ⇥ ⛔⛔ **AND IT WAS CLOSED ONE STEP EARLY — reopened and re-closed 2026-08-11**
  (GPT 5.6 §1). Removing `RecharacterizeBody` silences the PERSONA derive and
  nothing else. `project_prepared_character_definitions` is a SECOND template
  observer, fires on `Changed<WornCharacter>`, and a seated body had no
  `ProjectedCharacterKit` — so the seat stopped asking one observer and was still
  finished a tick later by the other: hurtboxes, posed body, movement tuning,
  motion model. ⭐ **the same mis-verification D78 cost a session to: I checked
  one writer and reported the answer as if it covered both.**
  ⇒ **the fix is the SHARED materializer, not a third hand-copy.** The seat calls
  `grant_prepared_character_body` with a new `KitOwnership::CallerResolved` —
  the gate was asking *who writes the kit* when the question it needed was *is a
  derive coming*. The seat's own stamp and its own motion-model switch are
  DELETED; the one materializer does both.
  ⇥ ⚠ **and the first guard was insufficient, measured not assumed.** With the
  grant disabled, `..._asks_for_nothing` STAYS GREEN — the persona derive writes
  a baseline that looks exactly like construction's. Only
  `a_seated_fighter_is_complete_and_the_next_pass_changes_nothing` goes red, and
  it is the one that asserts D73's invariant: both records current on the
  construction frame, and a second update with no reload changing nothing.
  ⇒ **CLOSED.** `RecharacterizeBody` now has exactly ONE production writer —
  Mary-O's powerup (`game/ambition_demo_mary_o/src/powerups.rs:1152`), the
  genuine re-template the component exists for. Guard:
  `a_seated_fighter_carries_its_applied_template_and_asks_for_nothing`, which
  asserts the STAMP rather than the absence of the request, because a body with
  neither looks identical to one that never got either. monolith 1239, app_it 330.

- ~~**D87 ⛔⛔ THE PUBLISHED-POLICY ARM WAS VACUOUS, NOT MERELY UNPOPULAR**~~
  **CLOSED 2026-08-11 — and it is the sharpest instance yet of Jon's "the new
  path works BESIDE the old one" failure mode, because the new path did not work
  at all and everything was green.**
  ⇥ **the measurement.** Assembly keys published policies `provider::name`
  (`registry.rs`'s `namespaced`). A CPU seat names a BARE key — `"duelist"`.
  `seat_brain_profile` asked `BrainProfileId::new(key)` with no provider, so it
  matched NOTHING in any real game and every seat fell through to the archetype
  table. Smash's own `duelist` profile, published four commits earlier for
  exactly this, was never once read.
  ⇥ ⛔⛔ **WHY THE TEST DID NOT CATCH IT: the fixture built a shape production
  never builds.** `BrainProfileRegistry::from_catalog_for_test` copied the
  catalog map VERBATIM — bare keys — so the bare lookup matched. Jon's rule
  (*repair an unrealistic fixture to model production construction*) is what this
  row is for: the fixture now namespaces like assembly, and a QUALIFIED name it
  is handed is honoured rather than double-qualified.
  ⇒ **the fix, and the deletion it unlocked.** The reference resolves in the
  MATCH's provider first (`roster.published_by`) and the character's second — a
  seat names a policy the match published, and Smash seats guest fighters whose
  own provider is `ambition_content`. Then `SMASH_ROSTER_RON` was **DELETED**:
  six archetype rows that existed only to answer this lookup, each carrying a
  body no seat had read since a fighter's body came from its character, differing
  from one another in exactly one field — `fighter_level`. They are six
  `autonomous_profiles` now. ⭐ that is the D73 thesis in one file: stating a
  difficulty required declaring a whole creature.
  ⇥ **guards.** `every_authored_difficulty_is_a_published_controller_policy`
  pins the deletion (a miss REFUSES the seat, loudly, rather than standing it
  still); the seat test gained a poison — another provider's policy must not
  answer — because a bare-key fallback beside the resolution looks like harmless
  generosity and is the exact hole that was there.

- ~~**D90 `smash_it` WAS 14/18 AND THE GATE COULD NOT SEE IT**~~ **CLOSED
  2026-08-11 — 17/17, and the count fell because a test was DELETED rather than
  rescued.** The stale direct-position test is gone: a raw `BodyKinematics::pos`
  write is not the semantic operation *"this fighter lost a stock"*, and
  `restart_pending` is a one-sim-tick flag a fixed-tick host can raise and clear
  inside a single `app.update()`. Its intent moved to the REAL knockout —
  `a_launched_fighter_is_taken_by_the_world_and_spends_a_stock` launches at
  2400px/s across the actual blast boundary and now proves the whole chain from
  one KO: exactly one stock spent, the other fighter untouched, a `BodyRestarted`
  TRIGGER observed for that body and not the other, and a respawn at the
  ruleset's placement. ⚠ the observer caught nothing at first and it was the
  FIXTURE, not a defect: the loop breaks the instant the stock drops, and the
  announcement is a later phase of the same frame.
  **D90 `smash_it` WAS 14/18 AND THE GATE COULD NOT SEE IT — 16/18 now, two
  behavioural failures left.** Measured 2026-08-11 (the D88 blind spot again:
  `cargo check -p ambition_app --all-targets` + `app_it` never build
  `ambition_demo_smash_app`'s tests).
  ⇥ ⭐ **two of the four were MINE, from deleting `SMASH_ROSTER_RON`**, and both
  were the same shape: a check that knew only the authority the campaign is
  removing.
  * `smash_roster_at_levels` did not call `published_by`. Harmless while a CPU
    seat's key was an ARCHETYPE key — an archetype table is global — and fatal
    once the key is a published POLICY, because a provider-relative name has
    nothing to resolve against on an unpublished roster.
  * `unsatisfiable_seats` consulted the archetype table ALONE, so four perfectly
    seatable fighters were reported unseatable. It asks BOTH authorities now,
    resolved exactly as `seat_brain_profile` resolves them. ⚠ the check's INTENT
    — *a demo must not declare a seat its own composition cannot fill* — is
    unchanged and was right; only the place it asked was stale. Same repair in the
    ladder test.
  ⇥ ⭐⭐ **AND THE "FROZEN FIGHTERS" WERE THE COUNTDOWN.** The stage opens
  `opens_suspended` with `opening_countdown_ticks = 3 * 60`, which stamps
  `ScriptedControl` on every fighter for the whole 3-2-1-GO — and the test warmed
  up 60 ticks before sampling, so its window sat ENTIRELY inside the hold. It was
  reporting *"neither fighter moved"* about fighters that were correctly forbidden
  to move. ⚠ the countdown is the campaign's OWN feature, so this was a stale
  WINDOW, not a stale assertion: the warm-up reads `opening_countdown_ticks` off
  the ruleset now instead of guessing a number.
  ⇥ ✔ **and "the brain travels but never commits" was ONE CPU PACING A STATUE.**
  The test's own comment said what it needed — *"CPU seats: a human with no
  controller correctly does nothing"* — and then called `smash_roster`, whose seat
  0 is HUMAN. So it measured a lone duelist circling an inert body. Seated through
  `smash_roster_at_levels` (every slot a CPU, same rung both sides) the fighters
  engage and it PASSES. ⚠ the engine was never wrong here; two fixture defects in
  one test hid each other, and the countdown one had to be fixed before the
  targeting one could even be seen.
  ⇥ ▢ **ONE LEFT, and it is not the reset — THE TELEPORT NO LONGER LANDS.**
  `losing_a_stock_announces_a_body_restart` throws a fighter to `y = 100_000` and
  expects a knockout. **Measured**: after the write the body is back at a normal
  stage height (`y ≈ 266`), holding all THREE stocks, walking around as though
  nothing happened. So nothing was ever knocked out and the missing
  `restart_pending` is a consequence, not the defect.
  * ⚠ **already ruled out**: it warms up 240 ticks so it is NOT the 3-2-1-GO hold,
    and it grabs seat 1 — a real CPU, not the inert human seat.
  ⇥ ⭐⭐⭐ **ROOT CAUSE ISOLATED 2026-08-11 — `body_kind` CARRIES TWO FACTS, and
  the geometry one is what shrank the body.**
  `CharacterBodyKind::Standard` supplies `default_standing_height = Some(48.0)`;
  `Floating` supplies `None`, meaning *the SHEET decides how tall this is*
  (`catalog_join.rs`). That single number is the entire 68.8 → 48.0.
  * ⛔ **my earlier isolation was contaminated and its conclusion was wrong.** I
    reported that the size regressed "even with `body_kind` restored" — it did
    not: I had also changed the PCA's authored locomotion in the same tree.
    Measured cleanly: three-state `flies` ALONE gives **22.3 x 68.8 and the test
    PASSES**; flipping ONLY `body_kind` to `Standard` gives **15.5 x 48.0**.
  * ⇒ so geometry and locomotion are coupled **through the enum, not through
    `is_aerial`** — `from_kit` never resizes anything. The review's instruction is
    exactly right: `Floating` may stay presentation/footprint vocabulary (it is a
    real answer to *how tall*), and must stop being locomotion authority.
  ⇥ ✔✔ **THE CUT LANDED 2026-08-11.** `finalize_character` no longer folds
  `body_kind == Floating` into `flies`; silence resolves to GROUNDED and the
  three characters that genuinely fly state it themselves (`stochastic_parrot`,
  `npc_burning_flying_shark`, both plane swarms already did). The PCA keeps
  `Floating` — **its body is still 22.3 x 68.8** — and states `flies: Some(false)`.
  ⚠ the test that asserted *fill-never-overrule* was rewritten rather than
  deleted: its §14 intent (a PREPARED character carries one concrete answer, so no
  constructor asks the catalog twice) is unchanged and is what it still pins;
  only the SOURCE of the answer moved. It gained a second term — an authored
  `Some(true)` must survive preparation untouched — because cutting the fold must
  not stop a bird flying.
  ⇥ ⇒ **the ORIGINAL cut, for the record**: delete the `body_kind == Floating ⇒
  flies = true` fold in `finalize_character`. **Six** characters carry
  `Floating` — the two plane swarms (which already state `flies: Some(true)`),
  `stochastic_parrot`, `npc_burning_flying_shark`, and the two automatons. The
  first two are already correct; the parrot and shark must state their flight;
  the automatons SHOULD lose it, which is Jon's call made literal. ⚠ the PCA then
  keeps `Floating` — so its body stays 68.8 — and states `flies: Some(false)`.
  * ⇒ **MEASURED FURTHER, and it splits into two facts.** ONE app update after
    the write the body is at `(537, 320)` — a normal stage position, not a clamp
    of `100_000` — so **the body IS restarted, promptly**. And the stock is still
    NOT spent. *A restart happened without a stock loss.*
  * ⚠ **the flag is unobservable from where the test looks.** `restart_pending`
    lives for ONE SIM TICK: the reset raises it, `announce_body_restarts` clears
    it in the next `WorldPrep` (`features/mod.rs`). The test samples between
    `app.update()` calls and a fixed-tick host advances several sim ticks per app
    update, so raise and clear both fall inside one sample gap. The engine
    publishes a `BodyRestarted` TRIGGER for exactly this; an observer collecting
    it is the right instrument.
  * ⛔ **but do NOT make it green with the trigger alone.** The test's NAME says a
    stock loss announces the restart, and the measurement says a restart occurred
    without one. Repairing the instrument while that is true would hide the more
    interesting half — find out why the KO spent no stock first.
  ⇥ ⛔ **and the standing lesson is the GATE, not these tests.** Two crates
  (`ambition_platformer2d_host`, `ambition_demo_smash_app`) have now been found
  red while every check the run performs was green. Smash is the PROVING GROUND;
  a proving ground nobody runs proves nothing.

- ~~**D89 THE PCA AUTHORS ITS WHOLE BODY AND THE DUEL STILL WILL NOT TAKE IT**~~
  **CLOSED 2026-08-11 — `cellular_automaton_fighter` IS DELETED (87 lines; the
  file is 355 → 284).** Parity was MEASURED before the deletion: with the duel
  arena on the character road all four of its tests pass with numbers identical
  to the archetype road — body 22.3 x 68.8, shield 157 frames, dash 6, melee 29,
  flight toggles 1800 → 13. Four things went with the row: the string matcher's
  last arm, `hostile_brain_id_for_actor`'s three identity PARAMETERS (it is a
  constant now), the dialogue-node plumbing that fed it through three files, and
  two tests of a mechanism that no longer exists — one re-homed to
  `ambition_content` where the pulse now lives. The original row, for its
  measurements:
  **D89 THE PCA AUTHORS ITS WHOLE BODY AND THE DUEL STILL WILL NOT TAKE IT
  — the shield never goes up.** Measured 2026-08-11 (sprite redirect P5, and the
  last live consumer of `cellular_automaton_fighter`).
  ⇥ **what LANDED, and it is most of the migration.**
  `perfect_cellular_automaton` and `imperfect_cellular_automaton` are registered
  characters authoring: 60 HP, 168 run speed, the 0.24/0.08/0.30 swipe, the
  glider (`Rock` at 300 with `ranged_vfx: "glider"`), the four body capabilities
  (blink / fly / shield / dash), a Smash `BrainProfile` (notice 540, commit 150,
  dash-to-close, duelist, patrol 0.5714) and **Cellular Pulse** as a real
  `MovesetContract` in `cellular_automaton_moveset.rs` — the 0.40s tell, the
  0.14s active window, the `pca.cellular_pulse` cue, numbers verbatim.
  ⇥ ⛔ **what BLOCKS the flip:** naming the character on the duel placement makes
  `duel_fighters_actually_enact_their_abilities_on_the_body` fail with *"PCA:
  shield must actually go up on the body (got 0 frames)"*.
  ⇥ ⭐ **two hypotheses are RULED OUT — do not repeat them.** (a) the placement
  brain: `Passive` and `Custom("cellular_automaton_fighter")` fail identically,
  so the archetype key is not what supplies the shield. (b) the special slot: the
  moveset binds `special → cellular_pulse` and the action set also did, which
  looked exactly like the shield's slot being taken — freeing it changes nothing.
  ⇥ ⭐⭐ **AND THE TEST WAS ALREADY PRINTING THE ANSWER — I spent three
  hypotheses not reading its output.** With `-- --nocapture`:

  ```text
  PCA: caps[blink=true shield=true dash=true fly=true]
       shield_frames=0  dash_frames=0  fly_frames=900  fly_toggles=1800  blinks=0
  ```

  **1800 toggles in 900 frames is two per frame, and `fly_frames` is 900/900.**
  The PCA is not failing to shield; it is doing nothing BUT toggling flight, and
  the shield/dash/blink zeroes are downstream of that. `smash/mod.rs:744` presses
  the toggle `if want_air != obs.self_aerial` — so the press is not changing what
  the brain OBSERVES, and it re-presses forever. ⇒ **the next probe is
  `obs.self_aerial` for a character-first body**: the body flies (`fly_enabled`
  every frame) while the observation apparently does not agree.
  ⇥ ⚠ **three hypotheses are dead and must not be re-run**: the placement brain
  (`Passive` behaves identically to the archetype key), the special slot
  contending with the pulse, and an unauthored provoked policy — `cellular_duelist`
  is published and named now, that change is KEPT because it is correct on its own
  merits, and it changed nothing here. The authored profile has also been compared
  FIELD BY FIELD against `ArchetypeSpecExt::brain_profile()`'s lowering of the row
  and they are identical, including the 36.0 hit-band default; the abilities match
  too (`movement_kit` maps `can_fly` to BOTH `fly` and `fly_toggle`, which is what
  the definition authors).
  ⇥ ⛔ **do not flip the placement to make the row deletable.** A duel where the
  PCA never blocks is a worse game than one archetype row.
  ⇥ ⭐⭐⭐ **ROOT CAUSE FOUND 2026-08-11, and Jon named it independently the same
  hour**: *"in smash PCA should not have the fly ability. I made a wrong call
  there earlier."*
  * `body_kind: Floating` on the catalog row is not decoration — preparation
    reads it and forces `locomotion.flies = true`. The archetype row DISAGREED
    (`is_aerial: Some(false)`, a grounded hybrid) and won on its own road.
  * `CharacterLocomotion::flies` is a bare `bool` whose own doc admits *"false
    does not mean grounded, it means this character did not say"* — so a
    character cannot contradict its row. ⭐ **`ArchetypeSpec::is_aerial` already
    solved this as `Option<bool>`, and its doc already names the PCA as the live
    case**: *"Floating in its catalog row, played grounded by the shipped duel."*
  * ⇒ **the fix is to migrate that three-state onto the character**, not to flip
    `body_kind`.
  ⇥ ⭐ **MEASURED with the fix applied** — the PCA becomes a real fighter:
    `shield_frames 0 → 305`, `dash 0 → 6`, `blinks 0 → 4`, `fly_toggles 1800 → 0`,
    and `duel_arena_room_is_a_real_neutral_attack_defense_fight` **PASSES**.
  ⇥ ⛔ **TWO THINGS BLOCK LANDING IT, and the work is STASHED not discarded** —
  `git stash list`, message *"D89: three-state ... + PCA no-fly"*:
  1. **the body shrinks to 48px** where `duel_pca_body_is_sprite_authored...`
     wants >60. ⚠ measured: the size regresses with the three-state applied even
     with `body_kind` restored to `Floating` and the fly abilities restored — so
     the coupling is NOT simply `is_aerial`, and I did not isolate it.
     * ⛔ **AND IT IS NOT D74.** Settling the harness **185 frames** instead of 5
       before measuring leaves it at exactly `15.549 x 48.0` — so this is not the
       sheet-arrives-late hazard, and the two rows are separate bugs. That
       hypothesis is dead; do not re-run it.
     * ⚠ 48 ≈ the authored duel box (28x46) × `collision_scale 1.12`, so the body
       is taking the **LDtk fallback branch** of
       `sprite_body_collision_for_character_id_from_data` — the sprite body is not
       resolving at all on this road, while the archetype road resolves it. ⇒ the
       next probe is WHICH `character_id` each road passes to that resolver.
     * the work is stashed (`git stash list`); re-apply and bisect the remaining
       three engine files.
  2. **`duel_fighters_actually_enact_their_abilities_on_the_body` asserts
     `caps_fly`** — a test demanding the ability Jon has just decided against. It
     is a product change, not a weakening, but it should land WITH the fix.

- ✔ **D88 CLOSED 2026-08-12 — every red it found is green and the gate that
  could not see them now runs SMASH.** The original row follows.
  ⇥ **D88 ⚠ THE `ambition_platformer2d_host` TEST SUITE IS RED, AND WAS BEFORE
  THIS CAMPAIGN TOUCHED IT** — measured 2026-08-11 by stashing the working tree
  and re-running, so this is not mine.
  ⇥ **the symptom:** every test in `demo_shell_smoke` panics with *"Resource does
  not exist"*. The system is the encounter reconciler, which takes
  `Res<PreparedCharacterRegistry>`, `Res<CharacterRoster>`,
  `ResMut<QuestRegistry>` and `ResMut<AmbitionGameSave>` as PLAIN resources — the
  demo shell inserts none of them.
  ⇥ ⛔ **the finding is not the panic, it is that nobody noticed.** The gate is
  `cargo check -p ambition_app --all-targets` plus `app_it`, and neither runs this
  crate's tests. A whole crate can go red and stay green to the run.
  ⇒ ✔ **FIXED**: a run condition (`any_with_component::<Encounter>`), not six
  `Option<Res<..>>` — an absent resource read as `Option` would mean *skip this
  encounter* inside a game that HAS encounters, which is the silent-disable this
  repo bans. 8 tests came back.
  ⇥ ⭐⭐ **AND THE SWEEP FOUND THE REST: 59 test binaries green, 4 RED.**
  * ✔ **`ambition_workspace_policy` (2)** — and both were REAL architectural
    violations the gate could not see. (a) `powerups.rs` iterated a `HashSet` and
    a `HashMap`: correct by a commutativity argument the code even documents, but
    ADR 0023's lint is right that an argument has to be re-made by every reader —
    now `BTreeSet`/`BTreeMap`, which makes the question unaskable (`GeoId`,
    `GeoSource` and `PlacementId` gained `Ord`). (b) `update.rs` read
    `tuning.surface_walker` at RUNTIME, which ADR 0024 §8 forbids — that boolean
    is spawn-time SELECTION and afterwards a stale copy. `BodyMotionFacts` now
    publishes `adhesive_crawling` and the brain snapshot reads the FACT. ⚠ the
    facts builder returned `default()` for a crawler, so it had been claiming
    every crawler was not crawling — which is exactly why its consumer went on
    reading the flag.
  * ✔ **`ambition_demo_smash_app` `smash_it` — GREEN, 17/17** (2026-08-12). All
    four reds were fixed in the days after this row was written, and the last of
    them was not what its name said: `losing_a_stock_announces_a_body_restart`
    was measuring a FIXTURE defect twice over — the sampling window sat inside
    the 3-2-1-GO countdown, and behind that one CPU was pacing a human-seat
    statue. ⚠ and the row's own instruction is what closed D88's larger half:
    the sweep it ordered found two whole crates red that the gate could not see,
    which is why `smash_it` is IN the goal guard now.

- ✔ **D86 CLOSED 2026-08-12 — AND MOST OF IT WAS ALREADY DONE WHEN I CHECKED.**
  `playable_kit: HostCode` has ZERO adopters in the shipped catalog (the only
  occurrence is a comment) and `PlayableKitSource` no longer HAS that variant. What
  was left was a NAME: `PreparedKit::HostCode` was still called after the deleted
  selector while what actually reaches it is *nobody authored a kit*. It is
  `PreparedKit::Unauthored` now, and the error a content author sees no longer
  opens by telling them they took a kit they cannot have selected. ⇥ the original
  row follows.
  ⇥ **D86 ROBOT v3 OFF `HostCode` — ⭐ THE PRODUCT DECISION IS IN: THE ROBOT
  KEEPS ITS CHARGE** (2026-08-11, GPT 5.6 §4 via Jon; no longer blocked).
  ⇒ **half of it has landed.** `RangedExecution::HostCharge` is
  `ChargedProjectile`, and it is an AUTHORED field on `CharacterDefinition`
  (default `MovesetVerb`) that Robot v3 sets — so the charge is a fact about the
  character's ranged attack rather than about which arm of `PlayableKitSource`
  built it. ⛔ the persona derive's Authored arm used to return `MovesetVerb`
  unconditionally, which means the day the robot authored its own `ActionSet` it
  would have silently stopped charging. Guarded both ways: an authored charge
  survives an authored kit, and an unmigrated character does NOT acquire one.
  ⇒ **§5 half landed: THE ROBOT IS OFF `HostCode`.** It authors
  `player_robot_action_set()` — the swipe, the bolt, the bubble shield, numbers
  verbatim — and `ActionSet::gated_by(abilities)` is the general form of the
  filter host code applied in the same expression that BUILT the kit. Its catalog
  row says `Authored`. ⭐ the measurement that made this safe: app_it stayed green
  across the flip, and the two reds were catalog-only monolith fixtures asserting
  a kit that now lives in CONTENT — which the monolith cannot depend on, and that
  is the correct outcome rather than a gap (the engine should not know the
  protagonist's moves). Both were re-homed to what they still own.
  ⇥ ⛔ **and one seam had to open first**: `from_scratch_as_character` passed
  `None` for the prepared cast with a comment claiming *"a from-scratch bundle
  predates the world"* — a claim about the CALLER that had stopped being true.
  `session::setup` holds the registry and was reading its generation four lines
  later. Without it the protagonist's authored kit was invisible on the one path
  that spawns the protagonist.
  ⇥ ⛔⛔ **AND FLIPPING THE ROW SHIPPED A SILENT REGRESSION FOR ONE COMMIT.**
  `gate_worn_player_control` asked whether the catalog row said
  `playable_kit: HostCode` and cleared EVERY projectile press if it did not — so
  the hour Robot v3 started authoring its kit, the gate began disabling the
  Hadouken. **Nothing failed**: pressing the projectile button as the protagonist
  is covered nowhere, so a full green suite said the protagonist could still fire.
  ⭐ this is exactly the failure §4's rename was meant to prevent, arriving
  through a READER nobody had counted — the lesson is the ledger's own
  *count the ADOPTERS, not the capability*, applied to a fact rather than a type.
  The gate asks the CHARACTER how it fires now, and the regression is
  poison-verified: restore the row test and it goes red.
  ⇒ ✔ **`PlayableKitSource::HostCode` is DELETED** (2026-08-11). Its adopters went
  3 → 1 → 0: smash's three duelists authored a `duelist` action set and said
  `HostCode` on the next line anyway, pocket's runner never fights, and the last
  was the host smoke fixture that existed to test the variant. Its two match arms
  were identical to the unknown-id arm, which SURVIVES — an id nobody wrote down
  still needs a defined answer, and that is a different question from a row
  claiming engine code owns its kit.
  ⇥ ▢ **`PreparedKit::HostCode` remains**: the fallback every character with no
  authored action set takes. Deleting it is the unauthored-floor bridge and `PreparedKit::HostCode` is the fallback every character with no
  authored action set takes — deleting that one is the unauthored-floor bridge,
  which is a bigger migration than this row.
  ⇥ **the original row, for the measurement it carries:**
  **does the protagonist keep its CHARGE?** Jon's second redirect P5, measured 2026-08-11.
  The robot now owns *what its attacks ARE* (eleven canonical timelines, its
  verbs, its projectile art) and host code still owns *what actions it HAS*.
  **Measured, so no step begins with a survey:**
  * `playable_kit: HostCode` selects `default_player_action_set(abilities)`,
    which is a pure function of the LIVE `AbilitySet`: melee `Swipe` iff
    `abilities.attack`, `special` bubble-shield iff `abilities.shield`, and
    `ranged` **unconditionally** (the row's own comment: *"there's no separate
    `projectile` ability in AbilitySet"*).
  * ⭐ that is EXACTLY Jon's target shape already — *canonical repertoire, gated
    by runtime progression* — with the repertoire inlined in Rust instead of
    authored. Authoring it is `ActionSet` on the definition plus a
    `gated_by(abilities)` mask, and the mask is those three conditionals.
  ⛔⛔ **BUT `HostCode` ALSO SELECTS `RangedExecution::HostCharge`, and that is
  not a kit fact — it is a MECHANIC.** `HostCharge` means the ranged press is the
  chargeable-projectile system's, not a moveset verb; it also decides that the
  `special` marker (not `ranged`) is folded into the derived moveset, and it
  gates `ChargesProjectiles` + `PlayerProjectileState`. An authored character gets
  `MovesetVerb` instead. So dropping `HostCode` as written **takes the fireball
  charge off the protagonist**.
  ⇒ **the decision Jon owns:** does the robot keep the charge? If yes, "my ranged
  is a charge" is a CHARACTER fact and needs an authoring surface
  (`RangedExecution` on the definition, defaulting to `MovesetVerb`) — at which
  point `PlayableKitSource::HostCode` and `PreparedKit::HostCode` have nothing
  left to decide and both die. If no, the charge becomes a moveset move like any
  other and the same two types die.
  ⛔ do NOT resolve it by authoring the robot a kit and leaving the row
  `HostCode`: `PreparedKit::HostCode { authored_moveset }` honours an authored
  MOVESET but takes the action set from the host regardless, so the character
  would state a repertoire nothing reads.
  ⚠ this is also the clean path to Jon's other note: Hall peacefulness would stop
  being `default_action_set: "peaceful"` deciding what somebody intrinsically
  knows how to do, and become a controller/placement fact.

- ▢ **D81 ⭐ THE `BrainPreset` RETIREMENT, MEASURED — 125 ADOPTERS, 8 LIVE KEYS,
  ONE DEAD ONE.** Jon's redirect §10/P1.12: there must end up with ONE
  autonomous-controller vocabulary, the migration is SEMANTIC, and ⛔ **do NOT
  build a generic converter** — a preset states absolute px/s and a profile
  states normalized effort against a body, so a mechanical conversion needs body
  context it does not have.
  **The census (2026-08-11), so no step begins with a survey.** Adopters counted
  by `default_brain:` in `character_catalog.ron`:
  | key | adopters | absolute speeds it holds | relationship it holds |
  |---|---|---|---|
  | `stand_still` | 44 → 38 (08-12) → **37** (08-13) + the 6 other providers | none | none |
  | `patrol_peaceful` | 43 → **43** (08-13, unmoved) | `speed: 28.0` | `aggressiveness: 0.0` |
  | `melee_brute_striker` | 23 → 16 (08-12) → **14** (08-13) | `chase_speed: 110.0` | `aggressiveness: 1.0` |
  | `melee_brute_brute` | 7 → 6 (08-12) → **4** (08-13) | `chase_speed: 75.0` | `aggressiveness: 1.0` |
  | `wanderer_puppy_slug` | 3 → **1** (08-13 — the 08-12 count was already stale) | `speed: 36.0` | `aggressiveness: 0.0` |
  | `skirmisher_ranger` | 3 → **1** (08-13) | `strafe_speed: 85.0` | `aggressiveness: 1.0` |
  | ~~`cellular_automaton_raider`~~ | **DELETED 08-12** | — | — |
  | ~~`parrot_lively`~~ | **DELETED 08-12** | — | — |
  | ~~`sniper_default`~~ | **DELETED** | — | — |
  ⭐ **RE-MEASURED 2026-08-13, and seven rows left by AUTHORING rather than by
  converting.** The six pirates and Carl Stargan now state their policy on their
  character (`with_autonomous_profile`), so their catalog rows carry
  `default_brain: ""` — which is what `a_character_states_its_policy_in_one_place`
  demands the moment a character authors a profile, since a row and a definition
  both stating it means one of them decides nothing. ⇒ **that is the migration
  path this row was looking for and it is not a converter**: a character adopting
  its own policy empties its row as a side effect, exactly as §10 wants, and no
  absolute px/s is ever mechanically turned into an effort fraction. Seven down
  by doing the thing that was worth doing anyway.

  ⚠ `wanderer_puppy_slug` fell 3 → 1 without my touching it, so the 08-12 column
  was already stale — recount before acting on any number here.

  ⚠ **re-measured 2026-08-12**: `stand_still` is 44 (not 43), every other count
  above still holds, and `sniper_default` is gone. A census is only useful while
  it is true.
  ⭐ **AND THE DUPLICATION WAS MEASURED, 2026-08-12: SIXTEEN characters author a
  policy on their definition AND still name a preset.** Of those, the six naming
  `stand_still` dropped it the same day — the giant, its hands, both plane swarms,
  and both sandbags — because that preset holds no speeds and no `aggressiveness`,
  so nothing came across with it. ⛔ the other ten name presets that DO carry a
  relationship (`melee_brute_striker`/`_brute` say `aggressiveness: 1.0`,
  `patrol_peaceful` says `0.0`), and the parrot cost four attempts teaching me
  what happens when that is dropped without moving it to the placements first.
  ⚠ **two of the sixteen are false positives** of the arm-scan: the pirate admiral
  and the patent clerk have arms that author only a MOVESET, so their
  `default_brain` is their only policy and must stay.
  ⇒ ✔✔ **ALL SIXTEEN ARE DONE, and the eight I thought were blocked were not**
  (2026-08-12). I had exempted them because their presets carry `aggressiveness:
  1.0` and the parrot proved that dropping a preset's relationship breaks a body.
  ⭐ **the parrot's case was the OPPOSITE one**: `0.0` SUPPRESSED hostility, so
  losing it changed what that bird is; `1.0` merely restates what an `EnemySpawn`
  placement already means, so losing it changes nothing. Probed on the goblin
  alone first — whole suite green, `enemy_attacks_player` included — then applied
  to the other seven. ⇥ counts: `stand_still` 44→38, `melee_brute_striker` 23→16,
  `melee_brute_brute` 7→6. **The exemption list is EMPTY.**
  ⇥ ⚠ a rule learned from one case was applied to eight without checking which
  DIRECTION it ran in; the guard's exemption list is what made that visible enough
  to re-examine.
  ⇒ ✔ **the rule is frozen by a guard**:
  `a_character_states_its_policy_in_one_place` refuses any character that authors
  a policy and still names a preset, with those eight exempted BY NAME and by
  reason (each names a preset carrying an `aggressiveness`). ⛔ its exemption list
  cannot rot — one that gets fixed must LEAVE the list or the test fails — and it
  caught my own imprecision before landing: the first version read only
  `autonomous_profile` and missed `autonomous_profile_ref`, so the goblin and the
  lab raider (which name the shared `medium_striker` POLICY) looked clean. A named
  authority is still an authority. Poison-verified.

  ⭐⭐ **START WITH THE TWO FREE ONES.**
  * ✔ **`sniper_default` IS GONE** (verified 2026-08-12: no `.rs`, no `.ron`, only
    two comments recording it as history). It did not survive another session,
    which is what this bullet asked for.
  * ⭐ **AND `parrot_lively` IS THE NEXT FREE ONE — with a live disagreement in
    it** (found 2026-08-12). One adopter, the stochastic parrot, whose DEFINITION
    authors `BrainProfile { Aerial, aggro 620, attack_range 60 }` while the preset
    its row names says `aggro 120, attack_range 0`. One bird, two authorities,
    different answers. The definition WINS (`resolve_npc_brain` ranks it above the
    row's `default_brain`, and the enemy road builds character-first), so this is
    dead weight stating wrong numbers rather than a live conflict — but it is
    exactly the two-authorities-one-fact shape this campaign exists to delete.
    ⇒ ✔ **THE SCHEMA UNBLOCK LANDED** (2026-08-12): `default_brain` may now be
    EMPTY — `#[serde(default)]` rather than `Option`, because 144 shipped rows
    write a bare string and RON will not read those as `Some(..)` without
    `implicit_some`. Four readers learned it (the fragment validator, the pack's
    reference emitter, the assembly's namespacing, and `build_default_brain`), so
    a character whose definition states its policy can stop naming a vocabulary it
    left.
    ⇥ ⛔⛔ **BUT THE PARROT WAS NOT THE FREE ONE AFTER ALL, AND ITS TEST IS WHY.**
    Deleting `parrot_lively` turned
    `stochastic_parrot_is_friendly_in_the_cove_and_hostile_in_the_sky` red — the
    preset also carries `aggressiveness: 0.0`, which is *the cove parrot's
    PEACEFULNESS*, and a `BrainProfile` has nowhere to put it BY DESIGN
    (`attacks_player` was deleted from that type on 08-11 because hostility is a
    relationship). So the deletion took a live fact with it. Reverted; the
    schema change kept.
    ⇥ ⚠ **AND THAT DIAGNOSIS WAS ALSO WRONG — the third attempt found the real
    blocker.** Tracing the parrot's placements: it appears three times, as ONE
    `NpcSpawn` in `pirate_cove` and TWO `EnemySpawn`s in `pirate_sky_lookout`. The
    cove one is peaceful because it is an NPC placement, not because of
    `aggressiveness: 0.0` — so the relationship was ALREADY where D81 says it
    belongs, and the preset's copy decided nothing. The red test was asserting the
    CONTENTS of a preset, one authority away from anything the game does; it now
    pins the bird's authored flight instead.
    ⇒ ⭐⭐ **the real blocker is that `default_brain` CARRIES THE NAMESPACE.**
    `qualify_preset_like` infers a character's provider by splitting this field's
    assembled `provider::name`, so a row that names no preset leaves
    `validate_brain_override` unable to qualify a Hall pedestal's override — and
    the full-host Hall validation fails. ⛔ **`qualify_preset_like`'s own doc
    already names this smell and the fix**: take the provider from the authority
    that owns it (a `CharacterDefinition` states one directly) rather than
    inferring it from a neighbouring key. That is the next step, and it is
    engine work rather than content.
    ⇒ ✔ **AND BLOCKER (3) IS FIXED** (2026-08-12): `CharacterCatalogEntry` states
    its `provider`, filled by ASSEMBLY at the one moment the pairing is known for
    certain, and `validate_brain_override` takes the namespace from there. The two
    neighbouring-key inferences remain as ORDERED fallbacks — `default_action_set`
    (still required of every row) before `default_brain` (no longer required) —
    so each disappears as its authority arrives.
    ⇒ ✔ **AND `cellular_automaton_raider` FOLLOWED THE SAME DAY**, along the path
    the parrot cost five attempts to open: both automatons share one arm authoring
    `BrainProfile { Smash, aggro 540, attack_range 150, dash-to-close, duelist }`,
    the preset was a second copy with different numbers, and their hostility is
    their `EnemySpawn` placements'. Two presets down; SEVEN key rows left.
    ⇥ ⛔⛔ **`wanderer_puppy_slug` IS BLOCKED ON A PHASE, NOT ON A ROW — and I
    tried it to find out.** Its three adopters are `npc_puppy_slug`,
    `npc_burning_flying_shark` (both author profiles; their `default_brain` could
    go today) and `npc_puppy_slug_variant2`, a HALL-ONLY character with four
    pedestals and no authored arm.
    ⇥ the obvious move — publish the slug's inlined Wanderer policy as a shared
    `autonomous_profiles` entry and have the variant name it — DOES NOT WORK, and
    the reason is worth writing down: `authored_intrinsics` only runs for ids in
    `buildable_cast()`, and the variant is not registered. Its arm would never
    execute, so it would lose its wander and stand still on its pedestal.
    ⇥ ⛔ and REGISTERING it is the trap `PLAYABLE_ROSTER`'s own doc names: a bare
    registration means the character authors no body, preparation correctly
    RETRACTS what a persona does not author, and that is the measured ~100-NPC
    regression. The variant authors no locomotion.
    ⇒ so this preset waits on the HALL CAST being registered with real bodies —
    a phase, not a row.
    ⇒ ✔ **but its two OTHER namers dropped it anyway** (2026-08-12), because a
    preset surviving for a third character is no reason for the first two to keep
    stating their policy twice. `npc_puppy_slug` authors the same Wanderer policy
    on its definition and `disposition: Peaceful` on its ten placements; the SHARK
    authors `ChargeCrash` and was pointing at a slug's wander because that row
    happened to exist — two authorities, and the one that made no sense was still
    read wherever the definition was not. ⭐ **the row now has ONE namer**, so the
    day the Hall cast is registered it is a one-line deletion rather than a
    three-character migration.
    ⇒ ✔✔ **`parrot_lively` IS DELETED, on the FIFTH attempt** — the first
    character in the game to name no brain preset.
    ⇥ ⛔⛔ **AND MY FOURTH DIAGNOSIS WAS WRONG, WHICH IS THE MOST USEFUL THING HERE.**
    I inferred "a catalog that is half assembled" from a symptom and wrote it into
    this row as a finding. Then a test asked the assembled entry DIRECTLY — is the
    provider set, is `default_brain` namespaced — and it was perfectly fine. The
    real cause: there are TWO qualification sites, `CharacterCatalog::
    validate_brain_override` and `resolve_initial_brain`, and I had taught only the
    first. Fixing half of a duplicated rule looks exactly like the data being
    broken. ⭐ **ask the thing itself before believing a symptom** — the fourth
    time this campaign has paid for that lesson, and the first time the wrong
    inference was already committed as a fact.
    ⇥ **the superseded text is below, unedited:**
    ⇥ ~~AND THE FOURTH BLOCKER IS A DEFECT IN ITS OWN RIGHT: A CATALOG THAT IS
    HALF ASSEMBLED.~~ With the parrot naming no preset, the Hall's
    `brain_override: "stand_still"` still resolved BARE — which means the catalog
    reaching the NPC road has entries that never went through assembly (no
    `provider`, `default_action_set` still LOCAL) while its preset map IS
    namespaced. Two halves of one value, disagreeing about whether they have been
    namespaced. ▢ measuring which path publishes that catalog is where the next
    attempt starts; emptying `default_brain` is safe the moment a character's
    provider is reliably on its entry.
    ⇒ ⭐⭐ **AND THE REMAINING SIX PRESETS ARE ONE BLOCKER, NOT SIX ROWS — measured
  2026-08-13, and it retires this row's own framing.** The census asked the
  question the other way round: not *how many adopters does each preset have* but
  *how many of those adopters could stop naming it*. Splitting
  `character_catalog.ron` by row and cross-checking each id against the
  `authored/*.rs` arms that call `with_autonomous_profile`:

  ```text
    patrol_peaceful      43 adopters,  0 author their own BrainProfile
    stand_still          38 adopters,  0
    melee_brute_striker  16 adopters,  0
    melee_brute_brute     6 adopters,  0
    skirmisher_ranger     3 adopters,  0
    wanderer_puppy_slug   1 adopter,   0
    TOTAL               107 adopters,  0
  ```

  ⇒ **the sixteen-character duplication this row set out to delete is fully
  drained.** Every surviving `default_brain` is that character's ONLY policy
  statement, so there is nothing left to delete AS DUPLICATION — dropping any of
  them takes a live fact with it, which is the parrot's lesson generalised.
  `skirmisher_ranger` looked like the next free one (3 adopters, and
  `aggressiveness: 1.0` is the value the goblin+7 proved droppable); its adopters
  are two pirates that receive only a `provoked_profile_ref` from the prefix rule
  and `npc_helpful_liar`, which has no arm at all.

  ⇒ **so this row now waits on exactly what `wanderer_puppy_slug` waits on**: the
  Hall cast registered with real bodies, which needs their VITALS, which is D96
  item 8 and Jon's. ⛔ it is not six migrations that can be done one at a time —
  treating it as one would burn a session finding what this census found in a
  minute.

  ⇥ ⚠ **the row was emptied and restored twice and the tree is green.** Both
    reverts were the measurement disagreeing with me, for two different reasons,
    and both reasons are written into the row itself now — the second one is
    directly above `default_brain` in `character_catalog.ron`, where the next
    person to try this will read it before trying.
  * `stand_still` holds **no speeds and no relationship** — it is exactly
    `BrainProfile { template: StandStill, ..Default::default() }`. It is also the
    ONLY preset the five demo providers (`sanic`, `mary_o`, `twintrack`,
    `pocket`, `versus_fighters`) and the dialog fixture author at all. So one
    conversion retires the preset vocabulary from FIVE providers outright, and
    the 43 flagship adopters follow the same line.
  ⛔ **`aggressiveness` is the `attacks_player` trap wearing another name.** It
  is a relationship, not a policy — the thing deleted from `BrainProfile` on
  2026-08-11 — so it must land on the placement's `SpawnDisposition`, NOT be
  carried across. Every `aggressiveness: 0.0` adopter is a placement that should
  say `Peaceful`; the flagship has 87 of them.
  ⚠ **the speeds are the only part needing body context**, and that is why the
  order is: migrate the CHARACTER's `run_speed` first (it is a
  `CharacterLocomotion` field that already exists), then the preset's absolute
  number becomes the effort fraction that reproduces it. `wanderer_puppy_slug`
  is the worked example already in the tree — the slug's migrated character
  authors `run_speed: 36.0`, exactly the preset's number, so its
  `patrol_effort` is 1.0 and nothing else has to be decided.
  ⛔ the acceptance signal is the DELETION of `BrainPreset` and its
  `brain_presets` map, not the existence of profiles beside them.

- ~~**D78 ✔✔ FIXED 2026-08-11 — AND THE CAUSE WAS THE DUPLICATE TEMPLATE
  APPLIER, exactly as Jon's second redirect said.** `apply_worn_character_gameplay`
  re-applied every freshly constructed character-first body, because
  `stale_cast = PersonaBaseline.is_none_or(..)` and construction never wrote that
  stamp. Construction stamps it now (with an EMPTY displacement — nothing was
  taken from a body that was BUILT as this character), and an identity change
  alone no longer re-applies. **All four named tests are green with
  `npc_ai_slop` AND with `npc_puppy_slug`**, and the three intro placements that
  said "Puppy Slug" and spawned humanoid raiders now spawn puppy slugs.
  ⇥ **what I got wrong, kept because it cost the most:** I instrumented
  `project_prepared_character_definitions`, watched it skip a constructed body,
  and reported "the projection never touches it again". There were two appliers
  and I checked one. Twelve probes bisected to "the projection writes ActionSet
  mid-run" — which was true of the wrong writer.**~~
  ⛔ **WAS: A HOSTILE CHARACTER-FIRST CRAWLER DESYNCS UNDER ROLLBACK.**
  ⛔⛔ **ADJUDICATED 2026-08-11 by Jon's redirect
  (`redirect-2026-08-11-finish-the-architecture.md` §1–§3): STOP PROBING
  CHECKSUMS. The measurements already answered.** This is not primarily a
  rollback bug — it is that ordinary character construction is still TWO-PHASE
  (build a partial body, attach `WornCharacter`, let a projection notice it and
  derive `ActionSet`/moves a tick later). The fix is to make a normal character
  actor exist COMPLETE on its construction frame, and to split the stable
  identity fact from the explicit re-template operation. ⛔ do NOT fix this by
  making the delayed projection happen to be rollback-order-safe, and do NOT
  keep `Changed<CharacterIdentity> → reconstruct body` after any rename — that
  only renames D78. Everything below is the measurement record; the WORK is
  §20 P0 items 2–5 of the redirect.

  ⛔⛔⛔ **AND THE ROOT FIX LANDED AND DID NOT FIX IT. Measured 2026-08-11, after
  the redirect.** Construction is now single-phase for the character-first enemy
  road: `spawn_enemy_with_faction_into` calls `grant_prepared_character_body`
  (the extracted grant the re-template pass also uses) in the same batch as the
  body, memo included, so the body is COMPLETE on its construction frame and the
  projection never touches it again. Verified by instrumentation, not by
  inference — `CONSTRUCT 33v0 id=npc_ai_slop` fires once, the projection's own
  candidate sweep logs that body as current and skips it, and the two
  archetype-staged puppy slugs in the same room are the only bodies the
  projection still grants. **The oracle is still RED, at frame 556 — the SAME
  frame as the two-phase build.** Landing the kit a tick earlier did not move the
  divergence by one frame.
  ⇒ ⛔ **so "the projection writes the kit mid-run" was NOT the mechanism**, and
  the earlier probe that read as proof of it ("disable the `ActionSet` grant →
  GREEN") proves something weaker than it looked: with that grant disabled the
  body never received the character's kit AT ALL, so what went green was a
  character-first body behaving like an archetype one. Timing was never isolated
  from value in that probe.
  ⇒ **what the frame number says**: 556 for `npc_ai_slop` under BOTH construction
  shapes, 563 for `npc_puppy_slug`. Identical under a changed construction shape,
  different per character. The divergence tracks WHICH BODY was built, not when
  its parts arrived. The next instrument is the one this row already named and
  nobody has run — the per-frame census (`rollback_exit_oracle.rs:1082-1275`)
  extended to log `(frame, entity, <suspect field>)` from the live pass and the
  resimulated one and DIFF them — and it should be pointed at what a
  character-first body has that an archetype one does not, not at when it got it.
  ⚠ the construction change is KEPT: it is right on its own terms (Jon's redirect
  §3) and the whole `app_it` suite is green with it (327 passed). It simply is
  not this bug's fix.

  Measured 2026-08-11, and it is a determinism defect the character migration
  EXPOSED rather than caused. **Reproduction, one field:** add
  `character_id: "npc_puppy_slug"` to `EnemySpawn-104857` in
  `ambition_content/worlds/intro.ldtk` (the intro placement literally NAMED
  "Puppy Slug" that spawns a generic `medium_striker` body today), then run
  `cargo test -p ambition_app --test app_it -- rollback_exit_oracle::a_player_taking_hp`.
  ⇒ *"frame 563: GGRS sync-test checksum mismatch at frames [560, 561, 562]"*.
  Four rollback tests go red together (`a_player_taking_hp_damage_survives_rollback`,
  `enemy_death_and_inplace_revive_survive_rollback`,
  `combat_equipment_switch_and_breakable_survive_forced_rollback_identically`,
  `a_player_death_reset_survives_the_rollback_window`).
  ⇥⇥ **FOUR PROBES, and they killed my first three hypotheses.** Each is one
  field on `EnemySpawn-104857` plus the one-test command above (~12s):
  1. `character_id: npc_puppy_slug` → **RED**.
  2. same + `disposition: Peaceful` → **RED**. ⇒ **NOT hostility.** (Ten other
     slug placements are character-first and peaceful; that is not what makes
     them clean.)
  3. `disposition: Peaceful` ALONE, no character → **GREEN**. ⇒ the control: a
     placement edit that does NOT take the character road is rollback-clean, so
     it is not "the room changed".
  4. `character_id: npc_ai_slop` (complete, a WALKER, not a crawler, no
     `dream_seed`, no `cling_breaks_on_hit`) → **RED**. ⇒ **NOT the crawler and
     NOT the slug.** It is the character-first construction path itself.
  ⛔⛔ **AND THE COVERAGE INSTRUMENT IS ALREADY LOOKING AND SEEING NOTHING.**
  `every_component_in_unswept_populations_is_registered_derived_or_waived`
  (`game/ambition_app/tests/rollback_coverage.rs`) sweeps `vertical_shaft`,
  which contains a character-first `npc_puppy_slug` — and passes. So the
  unregistered thing is probably NOT a component on the body: suspect a
  RESOURCE, a spawn-ORDER dependence (entity indices shift when an extra actor
  is added to the merged world — the intro world is merged into the sandbox
  sim, which is why an intro edit moves a sandbox checksum), or a system that
  runs outside the rollback schedule.
  ⇥ **AND THE ROOM IS THE ORACLE'S OWN.** `rollback_exit_oracle` starts in
  `combat_calibration_lab`, and all three "Puppy Slug" placements live there —
  so this is a character-first body the oracle actually SIMULATES, not a
  global-ordering perturbation. That kills the ordering hypothesis before
  anybody spends a session on it.
  ⇥⇥ **AND THE COMPONENT INSTRUMENT WAS POINTED AT THAT EXACT ROOM, WITH THE
  BODY PRESENT, AND PASSED.** `combat_calibration_lab` is now in the
  unswept-populations sweep permanently (`8f932f5c9`+); re-running it with
  `character_id: npc_puppy_slug` on `EnemySpawn-104857` reports nothing. ⇒ **the
  divergence is NOT an unaccounted component.** What is left: a RESOURCE the
  resimulation reads and does not restore, an unordered query read (see
  `reference_unordered_bevy_reader_is_deterministically_wrong` — ordered SETS,
  not luck), or a registered component whose CHECKSUM is unstable (a `HashMap`
  iteration, a `String` capacity, a float that is `-0.0` on one path).
  ⇥⇥⇥ **BISECTED TO ONE LINE, 2026-08-11.** Six more probes, each = edit +
  `cargo test -p ambition_app --test app_it -- rollback_exit_oracle::a_player_taking_hp`
  (~90s with the rebuild), fixture `character_id: npc_ai_slop` on
  `EnemySpawn-104857`:
  * disable the `WornCharacter` insert in `spawn_actors.rs`'s character branch →
    **GREEN**. ⇒ it is the persona projection the worn id triggers.
  * in `character_runtime/presentation.rs::project_prepared_character_definitions`:
    disable the motion-model switch → RED. Disable the hurtbox grant → RED.
    Disable the `ActorMoveset` grant → RED.
    **Disable the `ActionSet` + `CombatKit` grant → GREEN.**
  * split that pair: grant `CombatKit` only → **GREEN**; grant `ActionSet` only
    → **RED**.
  ⇒ **granting `ActionSet` to a character-first enemy body is the desync.**
  ⇥⇥ **TWO MORE PROBES, and they kill the obvious reading:**
  * grant the `ActionSet` **DISARMED** (`melee`/`ranged`/`special` forced to
    `None`, so the body cannot attack at all) → still **RED**. ⇒ it is not the
    body attacking, and not the strike-volume family. The COMPONENT's presence
    is the trigger.
  * the change-tick recovery fix below was IMPLEMENTED AND TRIED — a
    `(With<WornCharacter>, Without<ProjectedCharacterKit>)` query unioned into
    the candidate set, so a rewind that restored a memo-less body would
    re-project it → still **RED**, and reverted. ⇒ the memo IS being restored;
    the projection is not silently skipping.
  ⇥⇥⇥ **THE DECISIVE PROBE, 2026-08-11: grant the SAME `ActionSet` at SPAWN
  instead of one tick later and the oracle is GREEN.**
  (`spawn_actors.rs`, character-first branch: `definition.kit.action_set()` →
  `commands.entity(root).insert((action_set, combat_kit))`, with the
  projection's own grant disabled. Everything else in the projection — moveset,
  hurtboxes, motion-model switch, memo — still runs.)
  ⚠ and spawn-grant WITH the projection's grant still enabled is **RED**, so it
  is not the archetype change and not the value: it is the projection WRITING
  the component mid-run at all.
  ⇒ **the mechanism, now that the pieces line up**: the projection is gated on
  `Changed<WornCharacter>`; change ticks do not rewind. After a rewind the memo
  `ProjectedCharacterKit` is restored PRESENT and agreeing, so the projection
  correctly declines to re-run — but the `ActionSet` bevy_ggrs restored is the
  pre-grant one, and nothing will ever grant it again. The resimulated body
  keeps the spawn-time action set (different `move_style`, so different
  locomotion) while the live pass had the character's. That is the SKIP failure
  `rollback/mod.rs:440-475` predicts, and it is why the `Without<ProjectedCharacterKit>`
  recovery query did not help: the memo is present, not absent.
  ⇒ **THE FIX DIRECTION, and it is architecturally better anyway: BUILD THE BODY
  COMPLETE.** The character-first road already holds the
  `PreparedCharacterDefinition` at construction; granting the kit there removes
  the two-phase construction entirely and the projection becomes what it is for
  — cast REPLACEMENT and re-wear. ⚠ the projection's grant must stay for SEATED
  bodies, so the shape is: grant at spawn, and make the projection skip a write
  whose value the body already carries (it needs `Option<&ActionSet>` in its
  candidate columns to compare). Measure with the same one-line fixture.
  ⛔ **AND THE OBVIOUS IMPLEMENTATION OF THAT FIX WAS TRIED AND IS NOT ENOUGH.**
  Granting the kit at spawn AND making the projection skip a write whose value
  the body already carries (`Option<&ActionSet>` added to its candidate columns,
  `.filter(|authored| current != Some(*authored))`) is still **RED** — reverted.
  ⚠ **and my first explanation of WHY was wrong — corrected here after reading
  both writers.** The two are DISJOINT by construction: the projection grants a
  kit only `if !persona_bodies.contains(entity)`
  (`presentation.rs:553`), and `apply_worn_character_gameplay` requires
  `&mut ActionSet` + `&mut IdentityKit` + `&mut MotionModel` as query columns.
  A character-first enemy is in the projection's population on the tick it
  spawns and the derive's afterwards, so both write it — at different times —
  but neither is "a second writer racing the first" and the equality skip failed
  for a reason still unmeasured.
  ⇥ **ALSO ELIMINATED BY READING:** the persona derive is gated
  `if character.is_changed() || stale_cast`, and `stale_cast` is
  `baseline.is_none_or(|b| b.id != id || b.generation != generation)` — a
  ROLLBACK-STATE test, not a change tick. So the derive DOES recover after a
  rewind, and "the derive skips" cannot be the whole mechanism. Same for the
  projection, whose memo is registered.
  ⇒ **what is genuinely unknown**: why a mid-run write of this component moves a
  checksum when the component's own probe is presence-only and its value is
  bevy_ggrs-restored. The next instrument is not another guess — it is
  observing BOTH passes: log `(frame, entity, action_set.move_style)` from the
  live step and from the resimulated one and diff them. `rollback_exit_oracle`'s
  per-frame census (`rollback_exit_oracle.rs:1082-1275`) is the existing tool
  that cornered the `Collected` latch and is the thing to extend.
  ⇒ **superseded candidates, kept so nobody re-runs them:** (a) `rollback_component_clone` may snapshot
  without CONTRIBUTING A CHECKSUM (read the plain variant's implementation — its
  entity-ref sibling's doc says "no GGRS checksum" explicitly), in which case
  `ActionSet` is restored but its ABSENCE/PRESENCE across a rewind is invisible
  to the sync-test and something else moves; (b) inserting a component CHANGES
  THE ARCHETYPE, so a query that iterates unordered sees a different order after
  the insert lands on a different tick — the
  `reference_unordered_bevy_reader_is_deterministically_wrong` shape; (c) the
  insert is not RETRACTED on rollback, so the resimulated body carries it a tick
  early. ⚠ note `CombatKit` is registered through the SAME
  `rollback_component_clone` and is green, so whatever it is distinguishes the
  two — the likeliest difference is that the granted `CombatKit` equals what the
  spawn already put there while the granted `ActionSet` does not.
  ⭐ **superseded hypothesis, kept because it was the obvious one**:
  `ActionSet` is registered `rollback_component_clone` (`domains/characters.rs:96`)
  and is granted through `commands.entity(entity).insert(..)`, while the memo
  that says *"already projected"* is written IMMEDIATELY. A save taken between
  the queue and the apply restores a world with the memo set and the component
  absent — so the re-derive early-exits and the resimulated body never gets its
  ActionSet, which the registry's own note predicts word for word: *"the derive
  skips, and the resimulation runs a fighter with somebody else's moves"*
  (`rollback/mod.rs:440-475`). The downstream checksum move is then the brain
  gate: no `ActionSet` ⇒ no swing ⇒ no strike volume ⇒ different aggregate.
  ⇒ **the probe that would confirm it**: log the tick at which the grant is
  APPLIED vs the tick the memo is written, on both the live pass and the
  resimulated one. If they differ, the fix is to write the memo in the same
  deferred step as the grant (or to make the grant unconditional and idempotent
  rather than memo-gated).
  ⇥ **THE MISMATCH FRAME MOVES WITH THE CHARACTER**, which is a clue worth more
  than it looks: `npc_puppy_slug` diverges at frame 563 and `npc_ai_slop` at 556,
  in the same fixture with the same inputs. A construction-time defect would
  diverge at the same frame every time — the body is built once, early. A frame
  that MOVES says the divergence follows a per-body EVENT (first hit taken,
  first attack, a death/respawn beat), so the probe to run next is the oracle
  with a body that never gets touched.
  ⇥ **TWO MORE ELIMINATIONS, both by grep, both already checked:**
  every persona-derive OUTPUT is rollback-registered
  (`ActionSet` + `IdentityKit` in `rollback/domains/characters.rs`, `CombatKit`
  and `ActorMoveset` in `combat.rs`, `PersonaBaseline` in `actors.rs`), and
  `apply_worn_character_gameplay` runs in the SIM schedule
  (`player_schedule.rs:219`, `PlayerInputSet::Persona`) — so it is neither an
  unregistered persona output nor a cross-schedule derive. ⚠ read the three prior
  instances of this shape first — `rollback/mod.rs:440-475` documents
  `IdentityKit`, `PersonaBaseline` and the projection memo, all "registered
  rather than assumed derived", all found the same way.
  ⛔ blocks three intro placements — 104851 (`Patrol:lab_patrol_line`), 104856
  (`Guard:96`) and 104857 — all named "Puppy Slug" and all spawning generic
  striker bodies until it is fixed. That is a live content defect on its own:
  the level says Puppy Slug and the game spawns a humanoid raider. ⚠ and it
  means every already-migrated creature may desync in a room nobody
  rollback-tests.

- ~~**D79 ⭐ MARY-O'S TWO ROSTER FRAGMENTS — LANDED `f8a047e77`.** Sanic's went on 2026-08-11 (`5e050e050`) and Mary-O's are the
  same shape: `snake::SNAKE_ROSTER_ROWS` (`mary_o_snake`) and
  `ai_slop::AI_SLOP_ROSTER_ROWS` (`mary_o_ai_slop`), plus
  `plane::SNAKES_ON_A_PLANE_ROSTER_ROWS`, all registered from `lib.rs:493-495`.
  **Measured, so no step begins with a survey:**
  * Both creatures are already complete registered characters (`solid_snake`
    authors `run_speed`/`Walk`/contact 0.5/1 and a `Wanderer` profile at
    `snake.rs:559`; the slop's is beside it), and **all 24 Mary-O placements
    already name a `character_id`** — 14 `ai_slop`, 6 `solid_snake`, 2+2 plane
    swarms. There is no LDtk naming work.
  * ⚠ **the patrol trap, twice**: both rows author `patrol_effort: 1.0` and
    neither character does, so `BrainProfile`'s 0.5 default would halve every
    snake and every slop in the demo. Move the number onto each profile FIRST —
    that is exactly what the badnik needed.
  * ⛔ **the real blocker is `respawn: OnRoomReenter`.** Both rows author it and
    `mary_o.ldtk`'s `EnemySpawn` def has no `respawn` field (its fieldDefs are
    `name, brain, path_id, mounted_on, character_id`). ⇒ add the fieldDef by
    copying `sandbox.ldtk`'s (uid one past the file's max — the parser at
    `entity_converters.rs:626` already reads it) and author `OnRoomReenter` on
    the placements, exactly as the pirates and the sandbags did.
  * ⭐ **snake identity is SAFE**: `is_snake_brain` matches the placement's brain
    STRING (`CharacterBrain::Custom("mary_o_snake")`), not the roster row, so
    deleting the row does not stop a stomp. Check `ai_slop.rs` for the same
    shape before assuming it.
  * ⚠ expect `app_local_catalog_composition` to need the same honest repair
    Sanic's did: it asserts Mary-O publishes a hostile roster, and the property
    worth keeping is per-provider App-local ownership, not the roster's
    existence.
  ⇥ and the stale warning is gone: those rows carried a doc comment saying they
  were RESTORED because "a multi-game shell host publishes an EMPTY prepared
  cast" (D75). Sanic's deletion disproved it — shell-host suite 20/20, `app_it`
  327/327.
  ⇥⇥ **DONE, and every prediction above held**: both rows and both per-enemy
  registration functions deleted, `patrol_effort: 1.0` moved onto the shared
  character helper first, `respawn: OnRoomReenter` authored on all twenty
  placements behind a new `respawn` fieldDef (uid 107068, EntityRef 2 → 2), and
  the `power_loop` fixtures repaired to register the demo's ONE remaining
  fragment the way production does. Snake identity was safe exactly as measured.
  The PLANE SWARMS keep their rows — they are Ambition's characters and a
  standalone Mary-O has no cast entry for them. ⇒ what is left of P2.17 is that
  one fragment, and it needs the swarms registered by a provider Mary-O loads
  standalone.~~

- ~~**D80 ⭐⭐ GROUP B/C'S MISSING SENTENCE — LANDED `8d112cf99` + `787165763`.** Measured 2026-08-11, and it is the gate on the last
  ten archetype rows (`character_archetypes.ron` is at 601 lines / 13 rows;
  every remaining migration except the sandbags' immortal half runs through
  this).
  **The two vocabularies, side by side:**
  * `BrainProfile` (`ambition_characters::brain::profile`) — flat, reusable,
    `template` + DISTANCES (`aggro_radius`, `attack_range`) + normalized EFFORT
    (`patrol_effort`, `chase_effort`) + tactics. A character carries one by
    VALUE (`CharacterDefinition::autonomous_profile`).
  * `BrainPreset` (`character_catalog/entry.rs:751`) — a rich per-variant enum
    (`StandStill`, `Patrol{..}`, `Wanderer{speed, aggressiveness}`,
    `MeleeBrute{chase_speed, ..}`, `Skirmisher{strafe_speed, standoff_px,
    fire_cooldown_s}`, `Sniper`, `Aerial{cruise_speed, dive_speed, roam_radius}`)
    carried by NAME (`CharacterDefinition::default_brain_profile`, a
    `BrainProfileRef` into the catalog's `brain_presets`), resolved by the NPC
    road at `features/npcs.rs:107`.
  ⇒ **so a character can NAME a policy or CARRY one, but not name the one the
  enemy road reads.** Group B ("extract shared AI behavior into real
  BrainProfiles") and Group C ("classify generic roles") both need the second.
  ⛔ **the conversion is NOT mechanical, and that is the whole design question**:
  `BrainPreset` authors ABSOLUTE speeds and `BrainProfile` authors effort
  fractions of the body's own `run_speed` (§4.7). Lowering one to the other
  needs the BODY, which the preset does not know — `profile.rs`'s header already
  says so: *"the same fork seen from the other side and is the thing to fix when
  the two vocabularies merge — not a precedent to copy."*
  ⇒ **the shape that avoids the question**: a `BrainProfileFragment` registry
  per provider, exactly mirroring `CharacterRosterFragment` (which three demos
  used and two have now deleted, so the pattern is proven and its assembly rules
  are known — ONE fragment per provider, BTreeMap-composed, App-local). A
  definition gains `autonomous_profile_ref: Option<String>` resolved at
  PREPARATION into the existing `autonomous_profile` value, so nothing
  downstream changes and `BrainPreset` is left alone until somebody decides to
  merge the two vocabularies properly.
  ⇒ **what it unblocks, concretely**: `medium_striker` (9 placements),
  `small_skitter`, `large_brute`, `gradient_seeker`, `ranged_skirmisher` stop
  being whole-body archetypes and become a named POLICY plus whichever character
  the placement actually is — `goblin` for the five `*_goblin_*` spawns,
  `npc_lab_raider` and `npc_salvage_guard` for the two intro placements that are
  literally named that. ⚠ the sandbox `0140-0146` block is a DEMO ROW of
  one-of-each archetype (`patrol cutter`, `small skitter`, `guard striker`,
  `medium striker`, `gradient seeker`, `large brute`) with no creature identity
  at all — those are a product question for Jon, not a migration.

- ✔ **D91 TWO OF JON'S THREE 08-12 REPORTS WERE ONE FACT, AND IT WAS NOT IN THE
  ENGINE** (2026-08-12). *"When I change the video quality in ambition, my sprite
  went from the robot v3 character to the robot v2 character"* and *"I see the
  new emmy sprite on the select screen, but her character is the old sprite in
  the match."*
  ⇥ **measured, not reasoned:** `sprites/noether_spritesheet.png` and
  `sprites/player_robot_v3_spritesheet.png` were 08-11; their twins under
  `sprites_0_5x` / `sprites_0_25x` / `sprites_potato` were **08-08**. 163 of 192
  sheets were stale. Those roots are what the runtime loads under the Low /
  Medium / Potato quality profiles, so the game drew four-day-old art at one
  quality setting and current art at another — and a select screen (full-res
  portraits) disagreed with the body beside it for the same reason. Nothing was
  swapping characters; two different generations of the same character were
  on screen.
  ⇥ ⛔ **I spent an hour looking for this in the engine first** — `ControlledSubject`
  staleness, sheet-token fallbacks, `PLAYABLE_ROSTER` cycling, the prepared-cast
  stamp. All refuted. The tell I walked past twice: a `find | head -20` hid the
  `sprites_0_5x` roots entirely, so I concluded "no scaled variants exist" from
  a truncated listing. ⭐ [[reference_grep_r_skips_symlinked_assets]] names this
  exact trap and I still made it.
  ⇒ ✔ **the drift is closed and so is the hole it came through.**
  `scripts/regen/sprites.sh` chained the variant generator at the BOTTOM and `exit 0`ed
  on a fingerprint cache hit at the TOP — and the fingerprint covers renderer
  sources plus the presence of full-res outputs, which says nothing about whether
  the reduced tiers match them. The stage is now a function both paths call.
  ⇒ ✔ **and a check that can fail:** `scripts/check_quality_variants_are_fresh.py`
  (in the goal guard). mtime with a ten-minute tolerance — clone-safe at one end,
  four-days-stale at the other — verified green, verified RED on a poisoned
  fixture, and verified to REFUSE rather than pass when pointed at a root with no
  tiers in it.

- ◐ **D92 THE LOADING-ZONE DETECTOR WAS THE LAST SWEPT READER RECONSTRUCTING
  MOTION FROM VELOCITY** (2026-08-12). Jon: *"I moved into a loading zone and the
  room didn't change. that is not a key binding issue."*
  ⇥ ⛔ **MY FIRST VERSION OF THIS ROW WAS WRONG, AND WRONG IN THE WAY THAT COSTS
  DAYS.** It said "not reproduced", listed four green door tests as evidence, and
  offered a keyboard-preset theory. Jon's one sentence disproved the theory (an
  `EdgeExit` needs no key) and GPT 5.6 found the defect in production code within
  ten minutes. ⭐ **the green tests were green about the wrong seam**:
  `a_fast_body_cannot_tunnel_a_walk_loading_zone` picks both endpoints and then
  manufactures `vel = (end - start) / dt` — it HANDS the detector the perfect
  velocity whose absence is the bug — and the door tests teleport the body into
  the zone. Not one of them enters a zone by moving. *"Every test is green" is not
  evidence when no test exercises the path.*
  ⇥ **the mechanism, verified in the tree and not taken on the review's word:**
  `movement/collision.rs::zero_axis_vel` advances a body to time-of-impact and
  then zeroes that axis (five call sites). An edge exit sits at a room boundary,
  which is exactly where that happens. So on the arrival frame the body's TRUE
  path crosses the zone, `SweepSample.delta()` says so, and
  `detect_room_transition_system` computed `kin.vel * dt` = **0** — discarding the
  segment that proves entry. A body left touching the band rather than strictly
  inside it never transitions, however long it stands there.
  ⇥ **`SweepSample`'s own doc names this as the rule**: *"Swept readers (hazard
  touch, CC6's relative portal sweep) consume `prev → curr`; bodies without the
  component … fall back to the historical `vel·dt` approximation."* The hazard
  reader and the portal sweep already obey it; room transitions missed the
  migration.
  ⇒ ✔ **FIXED**: the detector reads `Option<&SweepSample>` and uses
  `sample.delta()`, with `vel · dt` kept only as the documented fallback. ⭐ it
  reaches production because `SweepSample` is a field of the CORE BODY BUNDLE —
  every player and actor carries one, so the `Option` covers scratch fixtures
  alone.
  ⇒ ✔ **and a regression that models the collision instead of pretending it away**:
  `a_body_stopped_at_the_boundary_still_crosses_the_zone_it_walked_into` — velocity
  ZERO, sample carrying the travelled segment, transition fires. Its second half is
  the poison: the same body with no sample does NOT fire, so the fixture cannot
  quietly stop modelling the bug.
  ⇒ ✔ **GPT's second finding, fixed too**: `commit_confirmed_lifecycle` returned
  silently when `session_health()` reported a mismatch — which turns EVERY door and
  loading zone inert while a desync stands, and reports it as "the room did not
  change". It now says what it is holding and why (`error_once`).
  ⇥ ▢ **what is still open, and it is GPT's fact (1):** drive a body through the
  REAL movement kernel into a zone. The regression models the kernel's output
  rather than producing it, so it cannot see a future change to where the sample is
  written. ⚠ and Jon has not yet confirmed the fix on his build — until he does,
  this row is ◐, not ✔.
  ⇥ **the full review is recorded verbatim** in
  `redirect-2026-08-12-the-detector-ignores-the-sweep-sample.md`.

- ◐ **D93 THIS CAMPAIGN BROKE A SHIPPED BOSS AND NOTHING SAID SO** (found
  2026-08-12). The Gradient Sentinel summons its minions BY NAME from Rust:
  `MINIMA_TRAP_MINION_ARCHETYPE = "puppy_slug"` and
  `GRADIENT_CASCADE_MINION_ARCHETYPE = "small_lurker"`. Both archetype rows were
  deleted within a week — the slug because it became `npc_puppy_slug` (group A),
  the lurker because a census on 08-11 reported it *"PLACED IN ZERO LEVELS"*.
  ⇥ ⛔ **the census counted LDTK PLACEMENTS and could not see a Rust constant.**
  `spec_for_brain` answers `combatant` for an unknown key, so from the day those
  rows went, every minima trap and every gradient cascade spawned a generic body:
  wrong health, wrong speed, no crawl, no cling. **Nothing failed** — a fallback
  is a real body — and the only tell was on a boss screen nobody was watching.
  ⇥ ⭐ **the guard that existed asked the wrong DIRECTION.**
  `every_archetype_row_is_placed_somewhere_or_deliberately_code_selected` asks
  *does every row have a placement?* and has a `CODE_SELECTED` allowlist for rows
  the engine names. Neither of these two was on it. The missing question is the
  other one: *does every name the CODE summons still resolve?* A row and a
  constant can each be individually defensible while the pair is broken.
  ⇒ ✔ **the summon road is character-first**: `spawn_runtime_minion_into` resolves
  the prepared cast first and builds through `new_character_in` — the same
  constructor the enemy road, the NPC road and the match seat use — and REPORTS
  through P0.1's shared rule when an id resolves neither a character nor a row.
  ⇒ ✔ **the trap summons `npc_puppy_slug`** and gets the crawler back.
  ⇒ ✔ **a guard in the direction that was missing**:
  `summoned_minions_resolve::every_summoned_minion_id_resolves_a_body`, listing
  each summon constant and where it is written, poison-verified by re-pointing one
  at its dead name. Its exemption list cannot rot — an id that starts resolving
  again must LEAVE the list or the test fails.
  ⇥ ⭐⭐ **AND ASKING THE SAME QUESTION ONE FILE OVER FOUND A SECOND ONE.**
  `encounters/goblin_encounter.ron` names `kind: "large_brute"` in three waves and
  that row is gone too — so three waves of a shipped fight spawn generic
  combatants. Same week, same census, same shape. ⇒ the guard now READS THE
  SHIPPED BYTES for wave kinds instead of transcribing them (a transcribed list is
  a snapshot and cannot see the wave somebody adds tomorrow), and the encounter
  road warns at runtime the way the summon road does. ⇥ ▢ which creature a goblin
  fight's HEAVY is, is the same product question as the lurker: the encounter is
  not broken, it is UNDER-CAST.
  ⇥ ▢ **`small_lurker` IS A PRODUCT QUESTION AND IT IS JON'S.** The cascade wants
  "2 small lurkers" (`boss_profiles.ron` beat 3) and no such character exists.
  ⛔ I left the constant pointing at the dead name deliberately: re-aiming it at a
  convenient neighbour would hide the question behind a body that happens to
  spawn. What IS a small lurker — its own character, or should the cascade summon
  something the game already has?

- ✔ **D94 TWENTY ENGINE TESTS WERE MEASURING THE FALLBACK, AND THE RESPAWN ONES
  WERE VACUOUS** (2026-08-12). Same class as D93, found by carrying its question
  into the test suite: *who else names an archetype by string?*
  ⇥ `cellular_automaton_fighter`'s row was deleted on 08-11 when the PCA became a
  character. Twenty engine tests still name it — the dash tests, the
  respawn-policy tests, the brain-effect tests, the fighter harness — and
  `spec_for_brain` answers `combatant` for an unknown key, so every one of them
  has been asserting about the generic fallback.
  ⇥ ⛔⛔ **the respawn-policy tests went VACUOUS, not merely wrong.** `combatant`
  also authors `OnRoomReenter`, so *"this archetype respawns on room re-entry"*
  kept passing while its subject had ceased to exist. A test that cannot tell its
  subject from the fallback is not a weaker test, it is a different one.
  ⇥ ⛔ **and `COMBAT_BRAIN_KEYS` LISTED FIVE KEYS AND MEASURED TWO ROWS** —
  `puppy_slug` and the PCA both fell through, so each loop asserted one row three
  times while reporting five subjects. Vacuous by duplication.
  ⇒ ✔ the engine owns a duelist SHAPE now (Smash-brained, dash/blink/fly, 60 HP)
  in `fixture_roster_with_mount`, the same answer the pirates, the shark and the
  sandbag got — and the key list names what the shipped file actually has.
  ⇒ ✔ **and a guard for the whole class**:
  `every_engine_fixture_row_differs_from_the_combatant_fallback` pins each fixture
  row on a fact the fallback does not have, with a CONTROL asserting an
  unauthored id really does land on the fallback. Poison-verified by renaming the
  row: *"fixture row `cellular_automaton_fighter` is indistinguishable from the
  `combatant` fallback, so every test naming it would pass with the row deleted"*.
  ⇥ ⛔⛔ **AND A THIRD LIVE ONE, in a PLAYER WEAPON.** The puppy-slug gun's
  `SLUG_ARCHETYPE` was `"puppy_slug"` — so the ally a player summons with their
  own weapon was a generic combatant: wrong health, wrong speed, no crawl, no
  cling. It names `npc_puppy_slug` now, and it is on the summon guard's list even
  though the constant lives in the ENGINE rather than in content, because that is
  where the question gets asked.
  ⇥ ⇒ two more vacuous assertions repaired while here: the contact-damage test's
  second positive named `puppy_slug` and therefore repeated its first line against
  `combatant` while appearing to add a subject; it names an engine fixture row
  now. The `attacks_player` arm that matched on the two departed keys is annotated
  rather than collapsed — the next shipped row that does not attack belongs in
  that `matches!`, not in a rewritten expression.
  ⇥ ⭐⭐ **AND THE GUARD STOPPED BEING A LIST.** It transcribed the summon
  constants by hand, which is a snapshot — and this class had already outrun a
  snapshot three times before the snapshot existed. It now WALKS `crates/` and
  `game/` for `const …_ARCHETYPE: &str = "…"` and checks whatever it finds, so a
  constant written tomorrow is covered the day it is written. The hand-written
  list is DELETED rather than kept beside it: a transcription that duplicates a
  scan is a second place to forget.
  ⇥ ⛔ **the first version of the scan OVER-matched** — it took
  `CHARACTER_ARCHETYPES_FILE` and reported a RON path as an unresolvable creature.
  The name must END in `_ARCHETYPE`. A scanner that cries wolf gets muted exactly
  as fast as one that sees nothing, and both halves now assert they FOUND
  something.
  ⇥ ⭐ **the lesson is D93's, one layer in.** A fallback that is a real value makes
  every consumer of a deleted name look healthy — content, code AND tests. The
  question that finds it is never "does this pass?" but "could this tell the
  difference?"

- ◐ **D95 THIRTY-TWO PLACEMENTS HAVE THE RIGHT RESPAWN POLICY BY COINCIDENCE**
  (measured 2026-08-12). Counting every `.ldtk` in the tree: **86 placements name
  a brain key that resolves nothing**. Most are harmless — the `PhaseScript:` /
  `Patrol:` / `Guard:` / `Passive` values are a different vocabulary, and every
  migrated creature among them names a `character_id` so the CHARACTER builds the
  body. What the dead string is still read for is one field: the placement's
  respawn policy.
  ⇥ ⚠ **and it agrees with the fallback**, which is why nothing broke and why it
  had to be fixed anyway. `combatant` answers `OnRoomReenter`, and so did every
  deleted row I checked (`exploding_mite`, `dividing_mite`, `puppy_slug`,
  `burning_flying_shark`, `sky_parrot`). So ~32 placements have been correct by
  coincidence and would change silently the day somebody retunes the fallback.
  ⇒ ✔ **the eight MITE placements author it themselves now**, and the note that
  said this field *"has nowhere else to live yet"* is corrected — it has, and the
  shark riders proved it the same day by authoring `respawn: OnRest` on their own
  seven.
  ⇒ ✔ **AND THE REST OF AMBITION'S OWN, same day**: 20 more across
  `vertical_shaft`, `pirate_sky_lookout`, `basement_enemies`,
  `intro_escape_shaft` and `pirate_sky_arena` — the puppy slugs, the burning
  flying sharks, the sky parrots, the AI slop. Every value checked against its
  deleted row IN GIT HISTORY per key rather than assumed. **No placement in
  Ambition's worlds now depends on a dead brain key for anything.**
  ⇒ ✔ **and the demos were then asked SEPARATELY, which is what that correction
  bought.** MARY-O: its `mary_o_ai_slop` (14) and `mary_o_snake` (6) rows are
  deleted — both are characters — and all twenty placements ALREADY author their
  respawn. Nothing to do. Its two PLANE-SWARM keys resolve fine: `plane.rs`
  authors those rows.
  ⇥ ⛔⛔ **SANIC'S BADNIK IS A DIFFERENT FINDING, and it is not a regression —
  it never worked.** `badnik.rs` says its "body + walk + contact damage come from
  a demo-owned roster archetype (`sanic_badnik`, a 1-HP `Wanderer` that paces and
  reverses at walls)". **There is no such row** — not in the demo, not in
  Ambition's file, and not anywhere in its history. So every badnik has always
  been built with the generic `combatant` body, and the sentence describing a
  1-HP wanderer has read as DESIGN for as long as it has been wrong.
  ⇒ ✔ both comments corrected (`badnik.rs` header and the catalog row's note in
  `lib.rs`): a comment describing an implementation that does not exist is worse
  than none, because it ends the search.
  ⇥ ▢ **and the product question**: should a Sanic badnik BE the 1-HP wanderer its
  own file describes? It already has a catalog row for sprite and name, so
  authoring it as a character is small — but it changes how Sanic's enemies feel,
  which is Jon's to decide, not a migration's.

- ✔ **D99 JON ADDED THREE FIGHTERS TO THE SMASH GRID AND ONLY TWO ARRIVED.**
  Found 2026-08-12 by asking D98's question of a different list.

  Jon, 2026-08-11: *"add Stargan, the Patent Clerk and the PCA."* The PCA was
  already there; the Clerk arrived when D98 registered it; **Carl Stargan was
  never on the grid at all** and had not been since the day he was added.

  ### ⛔ the mechanism is a guard that asks the wrong table

  `SmashRoster::assemble` filters `SMASH_ROSTER` against the prepared REGISTRY,
  and its own doc says why: *"a catalog row says what a character IS;
  `register_character` is what makes one BUILDABLE, and only the second is what
  a seat needs."* It records that filtering on the CATALOG once put eight
  unpickable portraits on the screen.

  ⇒ and the test written to catch a dropped fighter — `every_smash_roster_id_resolves_in_the_shipped_host` —
  **checked the catalog.** Stargan has a row, so it passed; nothing registered
  him, so `assemble` dropped him. The check and the thing it checks disagreed
  about which table decides, and the disagreement is invisible because dropping
  is the SAFE behaviour and safe behaviour is silent.

  ### ✔ fixed, and poisoned

  * the guard asks the REGISTRY now — the same table `assemble` asks.
  * **probed RED**: removing Stargan's registration fails it with
    *"the smash roster names 1 fighter(s) the SHIPPED host cannot SEAT"*.
  * he is registered BARE, and here that is provably safe rather than assumed:
    the rule against bare registration protects ARCHETYPE-built vitals, and he
    has exactly one placement in the game — a Hall `NpcSpawn` with
    `brain_override: stand_still` — so he has never had any. The exemption
    carries that placement evidence, because the evidence IS the argument.

  ⇥ ⚠ **and putting him on the grid immediately reported what the grid had been
  hiding**: the melee census now lists him as unarmed, because his catalog row
  says `peaceful` three ways. He is armed by the STAGE like every other seat, so
  this is not a break — it is D96 item 5 (*does Carl Stargan fight?*) becoming
  visible, which it could not be while he was silently absent.

- ✔ **D98 CLOSED — all seven are registered, and the guard's exemption list is
  EMPTY** (verified 2026-08-13 against HEAD). The six pirates author bodies in
  `authored/npc_pirate_crew.rs` and reach `buildable_only_cast()` through
  `authored_ids()`, so the prefix rule's `provoked_profile_ref` now reaches all
  nine; the Patent Clerk's eleven-move repertoire reaches a body that is built.
  ⭐ **the regression the row named is the part that mattered** — six pirates were
  provoked into `pirate_boarder` before the migration and fell to the generic
  `combatant` after it — and it is repaired by registration rather than by
  restoring the deleted string matcher.
  ⇥ AS WRITTEN: ▢ **SEVEN CHARACTERS AUTHOR FACTS NOTHING EVER READS — and one of them is
  a repertoire this run wrote.** Found 2026-08-12 by the guard that asks the
  question in the direction nobody had: not *"is everything on the list
  authored?"* but *"is everything authored on the list?"*

  Registration iterates `buildable_cast()` = `PLAYABLE_ROSTER ∪
  BUILDABLE_ONLY_CAST`. An `authored_intrinsics` arm for an id outside that union
  **runs for nobody** — and nothing fails, because a body that is never built
  cannot break. The work sits in the file looking done.

  ### ✔ the seven, measured by handing every catalog row a bare definition and asking whether it came back changed

  * **six of the NINE pirates** — `cutlass_viper`, `heavy_broadside_bess`,
    `heavy_salt_annet`, `lookout`, `navigator`, `quartermaster`. The prefix rule
    gives every `npc_pirate_*` row a `provoked_profile_ref`; only the three that
    are registered receive it. ⛔⛔ **and this is a REGRESSION, not just dead
    code**: the string-matcher arms that used to hand all nine the pirate policy
    were deleted in the same change that added the rule, on the measured premise
    that *"every pirate-named placement carries a `character_id`"* — which is
    true, and is not the same as that character being REGISTERED. So six pirates
    were provoked into `pirate_boarder` before the migration and fall to the
    generic `combatant` after it.
  * **`special_patent_clerk`** — authors the eleven-move repertoire written this
    run (P3.24), and is on the Smash select grid, so it APPEARS: the grid
    resolves from the catalog. Its moveset rides a definition nobody registers,
    so the table reaches no body.

  ### ⛔ and it cannot be fixed by adding ids to the list — PROBED

  Adding all seven turns `every_build_only_id_authors_something` red on the first
  id, which is that guard working. Its claim is measured, not prose: a bare
  registration says *"this character authors no body"*, preparation correctly
  retracts what a persona does not author, and the recorded cost is ~100
  exploration NPCs losing their archetype-built vitals.

  ### ⛔⛔ AND THE UNBLOCK I FIRST WROTE HERE WAS WRONG — corrected the same hour

  I published *"author each character's body from the numbers it gets today (the
  `combatant` fallback: 4 HP, 155 px/s, 0.70 contact)"* before checking how these
  seven are actually placed. They are **`NpcSpawn` placements, every one, all
  seven with `brain_override: stand_still`** — so they spawn through the PEACEFUL
  road and get `max_health: 1` and the shared `MAX_RUN_SPEED`. `combatant`'s
  numbers only reach them when they are PROVOKED.

  ⇒ carrying `combatant` across would have been a **buff wearing a migration's
  commit**, which is the exact failure this campaign keeps naming. The premise
  was one `grep` away and I wrote the prescription first.

  ⇒ **so this is a CONTENT DECISION, not a migration.** Authoring `max_health: 1`
  as an intrinsic makes a placeholder permanent — that value is precisely *"what
  a placement gets when nothing knows who it is"* — and authoring anything else
  is deciding how tough a pirate quartermaster is. Filed as D96 item 8.

  ### ✔✔ AND THE UNBLOCKED HALF LANDED — all seven are registered

  ⭐⭐ **the older guard was applying a body rule to a policy.** *"A bare
  registration says this character authors no body"* is right about BODIES, and
  it was written as a blanket — so it also refused a character that states only a
  CONTROLLER policy, which has no body to retract and whose statement is true
  whether or not anyone ever authors its vitals. That refusal is what left six
  pirates unable to deliver a fact the rule had already given them. **A guard
  that blocks a fact from reaching the game is doing damage, not preventing it.**

  ⚠ **probed, not argued**: registering all seven and running the app suite is
  330 integration tests green, and the invariant the blanket rule protects is
  pinned separately — `an_unmigrated_character_still_gets_the_roads_defaults`
  asserts a registered-but-incomplete character keeps the road's `max_health: 1`
  and `MAX_RUN_SPEED`, because the peaceful road reads body facts only from a
  body-COMPLETE blueprint. Policy-only registration retracts nothing.

  ⇒ six pirates deliver `pirate_boarder` / `pirate_boarder_heavy` again, and the
  Patent Clerk's eleven-move repertoire reaches a body for the first time. The
  new guard asserts BOTH halves per pirate — the rule states a policy AND the id
  is one registration visits — because either alone is silent.

  ⇥ ▢ their VITALS are still unauthored and still D96 item 8: they stand in the
  Hall at `max_health: 1`, which is what a body gets when nothing knows who it
  is. Registration made their POLICY reachable; it did not decide how tough they
  are, and it must not.

  ⚠ the seven are a rot-checked exemption list on the new guard, so the count is
  real and one that gets fixed must leave it.

- ✔ **D97 THE `default_brain` DELETIONS RAN AHEAD OF THE RUNTIME, AND TWO SHIPPED
  ROOMS PANICKED.** GPT 5.6 review, handed over by Jon 2026-08-12. ⭐ **verified
  before it was worked, by a falsifier, and every number in the review held.**

  The claim: migrated characters state their normal behaviour as a
  `BrainProfile`, their catalog `default_brain` was emptied so one authority
  decides, and the NPC road never learned the new vocabulary — it still resolves
  `default_brain_profile`, a `BrainPreset` reference that **zero characters in
  the repo author**. So the default resolved to the empty string.

  ### ✔ measured against the shipped worlds, not argued

  * **19 characters** have an empty `default_brain` in the assembled catalog.
  * **23 NpcSpawn placements** name one of them.
  * **2 of those author no `brain_override` at all** — `sandbox.ldtk`'s
    `pirate_cove` parrot and `gravity_lab` puppy slug. Their spawn calls
    `resolve_initial_brain(.., None, None, ..)`, which returned
    `UnknownPreset { preset: "" }`, which `resolve_npc_brain` turns into
    `panic!`. **Entering either room crashed the game.** A test calling exactly
    that, against the real assembled catalog, reproduced it first try.
  * **The other 21 spawn fine and are worse**, because nothing says so: they
    carry `default_preset: Some("")`, so every `RestoreDefault` — every
    possession release, every "you are free" — was rejected with *"unknown brain
    preset ``"* and the body kept whichever mind it had, for the session.

  ### ⭐ the fix is the vocabulary, not the symptom

  ⛔ **`Option<BrainPresetId>` could not tell two absences apart**, and that is
  why the empty string existed at all: `None` already meant *"a boss, rebuilt
  from its own authority"*, so a character with no preset could not use it. Three
  facts now have three variants — `AutonomousDefault::{None, Preset, CharacterProfile}` —
  and `AutonomousSource` gained the matching live variant.

  ⚠ **the lowering deliberately does NOT move into `ambition_characters`.** §4.7
  pairs a policy's normalized effort with the BODY's own top speed, and that
  crate has no body. So `resolve_initial_brain` REFUSES precisely
  (`NoAutonomousDefault`) and the three callers that hold a body answer it with
  one shared call — spawn (`resolve_npc_brain`), rewind
  (`autonomous_brain_for_source`) and live restore (`apply_brain_selection`) all
  reach `enemy_default_brain(config, abilities)`, so they agree by construction
  rather than by three matching implementations.

  ⛔⛔ **and `restore_default()` was writing the wrong source.** It set
  `CatalogDefault` unconditionally — harmless only while every default WAS a
  preset. On a profile-defaulted body it left the live source claiming a preset
  that `active_preset()` reports as absent, so the next rewind rebuilt no brain
  at all. Same class of silence, one function over.

  ### ⚠ the review's one over-reach, kept for the record

  It read the `npc_ai_slop` deletion as conflating contextual semantics. The
  definition-side `Wanderer` and the row's `melee_brute_striker` were not two
  contexts: the row is what a body got when nobody knew who it was, and the
  guard `a_character_states_its_policy_in_one_place` exists to keep exactly one
  of them. The deletion was right; only the runtime was behind, which is what
  the review's own headline says.

  ### ✔✔ AND THE LOSING HALF IS DELETED, same day

  ⭐ `default_brain_profile` is **gone** — the field on `CharacterDefinition`, its
  builder, the staging carrier, the prepared mirror, and the `definition_default`
  parameter on `resolve_initial_brain`. It had **zero authors in the entire
  repo** and one consumer; the only thing that ever wrote it was a test fixture.

  ⛔⛔ **and deleting it exposed that Jon's precedence ruling was not being
  enforced anywhere.** *"A character definition may state its normal autonomous
  behaviour, and it outranks the catalog row"* (2026-08-10) was expressed ONLY
  through that unused field — so in practice the row outranked the character on
  every body in the game, and three tests in `binding.rs` asserted otherwise
  while exercising a road no content took. The rule is now structural in
  `resolve_npc_brain`: **override → the character's own `BrainProfile` → the
  row's `default_brain`**, in the vocabulary characters actually use. The content
  guard `a_character_states_its_policy_in_one_place` still forbids the double
  declaration — but a rule that holds only because content happens not to violate
  it is not a rule.

  ⚠ the three retired tests left a note where they stood naming the ruling they
  guarded and where it is asserted now, because the ruling survived and only its
  vocabulary changed. ⚠ the catalog ROW's preset vocabulary is untouched and
  still has ~125 adopters (D81); what is gone is a definition being able to say
  the same thing in the row's words.

