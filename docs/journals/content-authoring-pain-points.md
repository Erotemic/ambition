# Content-authoring pain points — stochastic parrot run

A live log of friction encountered while adding ONE new character (the stochastic
parrot) end-to-end: friendly cove NPC + aggressive sky enemies. The goal is to
surface what makes adding content hard so we can refactor toward "author a new
character blind, in data, with a headless test that proves it."

Format: each entry = the friction + why it bites + a candidate fix.

## Pain points

### P1 — Two parallel rosters: `character_catalog.ron` vs `enemy_archetypes.ron`
- A character's sprite/body/hall-tier/default-brain lives in `character_catalog.ron`
  (keyed by `character_id`), but an enemy's combat stats live in a SEPARATE roster
  `enemy_archetypes.ron` (keyed by a brain string used by `EnemyBrain::Custom`).
  To make one creature both a friendly NPC and an aggressive enemy you touch BOTH
  files and keep a name in sync between them.
- Why it bites: no single source of truth for "a character"; the two can drift, and
  it's non-obvious which file owns what. Adding content means hunting across rosters.
- Candidate fix: let the catalog entry own (optionally) the combat stats too, or let
  an enemy archetype reference a `character_id` for its visuals instead of relying on
  name-matching. (See P2.)

### P2 — Enemy → sprite resolves by DISPLAY-NAME string match
- A spawned enemy gets its animated spritesheet via
  `CharacterSpriteAssets::npc_asset_for_name(display_name)` — i.e. the enemy's
  `display_name` string must EXACTLY equal the catalog entry's `display_name`, or it
  silently falls back to a generic sheet (warns once). (render `actors/mod.rs:243-275`.)
- Why it bites: a fragile, stringly-typed join. A decorated/variant name ("Parrot
  (sky)") breaks the sprite with only a log warning — invisible when authoring blind.
- Candidate fix: spawns should carry a `character_id` (a real key), not lean on
  display-name equality, to bind visuals.

### P3 — Hall-of-characters regen is a two-step manual command pair
- After a catalog edit you must run `generate_hall_of_characters` AND THEN
  `area_authoring <spec> --replace-existing`. Forgetting the second step leaves the
  hall stale with no error. (Mitigated: `docs/recipes/adding-a-character.md` documents
  both steps accurately.)
- Candidate fix: a single `regen_hall.sh` (or fold the apply into the generator).

### P4 — LDtk placement is hand-picked pixels + iterate-on-overlap
- Placing a spawn means choosing literal `px` coords, applying, running `validate`,
  reading "overlap within 4px" warnings, then `entity move` to nudge — a slow loop.
  The `entity move` spec key is `target:` (not `match:` like `set-field`), an
  inconsistency that cost a trial-and-error round.
- Candidate fix: a "place relative to <iid>/empty-floor" helper that auto-avoids
  overlaps; unify the entity-edit spec vocabulary (`target` everywhere).

### P5 — Attack patterns are limited to the fixed brain-template enum
- The aggressive parrot had to borrow `brain_template: Shark` (the only aerial
  pursuit brain). There's no aerial-melee-diver template, and the rich attack
  animations (`dive_bomb`, `hover_peck`, `banked_strafe`) aren't bound to any action
  — only `slash` is, via `action.melee.primary`. New movement/attack feels require a
  Rust brain-template addition, not data.
- Candidate fix (the ambitious + refactor commits): a data-authorable patrol/attack
  pattern (waypoints / phases) so a parrot's "fly–land–walk–bark" and "dive–strafe"
  are content, plus animation-binding keys for the extra attack rows.

### P6 — NPC brains ignore the catalog `default_brain` (hardcoded Patrol/StandStill)
- `NpcRuntime::build_brain` (features/npcs.rs) picks `Patrol` if `patrol_radius > 0`
  else `StandStill` — it NEVER reads the catalog `default_brain` preset. So a catalog
  row's `default_brain` is dead for an LDtk `NpcSpawn`; you cannot give a placed NPC a
  richer brain (e.g. the new lively `Aerial`) from data. (Enemies DO honor their
  archetype `brain_template` via `enemy_default_brain` — only the NPC path is stunted.)
- Also: the `Npc` interaction kind carries no `character_id`, so the cluster can't even
  resolve the catalog row to read `body_kind`/brain at spawn — the join is lost.
- Why it bites: the friendly parrot can't be authored as a flyer in data; it's stuck on
  the grounded Patrol brain regardless of its catalog row. This is the single biggest
  blocker to "author a new character's behavior blind, in data."
- Candidate fix (commit 3): thread `character_id` onto the NPC interaction, make
  `build_brain` resolve the catalog `default_brain` preset, and let `body_kind: Floating`
  zero gravity — so an `Aerial` peaceful NPC actually flies. THIS is the refactor that
  makes the friendly parrot's lively flight a data row.

### P7 — Hall regen duplicates the reciprocal hub→hall door
- `area_authoring --replace-existing` re-adds the hub→hall `LoadingZone` without
  noticing the existing one, producing two zones sharing id `hall_of_characters_door`
  → fails content-graph validation. Cost a separate fix commit.
- Candidate fix: the hall regen should upsert (dedup by id) its reciprocal door.

### P8 — NPC brains tick with `sim_time = 0.0` (hardcoded)
- `update_ecs_npcs` passes `sim_time = 0.0` into the brain ("NPC brains run Patrol/
  StandStill, which don't read the clock"). Any sim-time-driven NPC brain — like the
  new lively `Aerial` — gets a frozen clock (no waypoint variety, no dwell timing).
- Why it bites: blocks giving an NPC a time-based behavior. Pairs with P6: even once
  NPC brains are catalog-driven, they need the real sim clock to come alive.
- Candidate fix (commit 3): thread the real sim time into `update_ecs_npcs`.

## Commit 2 — ambitious behavior (what landed, what deferred)
- **New `Aerial` brain** (`brain/state_machine`): one pure, deterministic policy with
  two faces by `aggressiveness` — a lively peaceful bird (perch ↔ fly ↔ walk, drops
  beside the player to talk) and a hostile dive-bomber (stalk → dive → peck → recover).
  Captures its anchor from `actor_pos` on tick 1 (no spawn coord threading). Verified
  by 4 headless integration tests (flight, perch, talk-landing, the dive cycle).
- **Aggressive sky parrots** use it now (`brain_template: Aerial`) — they're already
  `is_aerial` (gravity-free), so the dive works end-to-end.
- **NPC idle barks** (`tick_npc_idle_barks`): ambient ~6–10s chatter; the parrot is the
  first user (stochastic-parrot riffs).
- **DEFERRED to commit 3 (the refactor):** the FRIENDLY parrot still flies only after
  P6 + P8 are fixed (catalog-driven NPC brains + real NPC sim clock + `Floating` ⇒
  gravity-free). That refactor is exactly "make the lively flyer a data row," so it is
  the natural payload of commit 3 rather than a special-case hack here.

## Positives (what already works well)
- **The spawn path is data-driven**: the basic friendly+hostile hookup needed ZERO
  Rust changes — catalog row + archetype row + yarn node + LDtk placement.
- **`docs/recipes/adding-a-character.md`** exists and is accurate (catalog → hall → walk in).
- **The actor unification holds**: one sprite/body serves a peaceful NPC and a hostile
  enemy with no special-casing; hostility is just `attacks_player` + brain choice.
- **Headless coverage is real**: the embedded-LDtk hall test proved the parrot pedestal
  resolves a safe sprite the moment the catalog row landed — caught wiring for free.

