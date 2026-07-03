# HANDOFF — finish the melee subsumption + every unblocked fable-review item

**You are continuing a large architecture refactor. Read this whole file before you
touch code. Then WORK — for hours, without stopping to ask.** This prompt is
self-contained: do not assume you can see any auto-memory. Everything you need is
here or in the two docs it points to.

---

## 0. THE MANDATE — read this twice, then fight your instincts to violate it

These are the repo owner's (Jon's) words. They override your trained instincts. When
you feel the pull to stop, summarize, checkpoint, or ask — that pull is the thing to
fight.

> "I want them to finish melee subsumption and every other unblocked item on the
> fable-review todo list **without stopping to ask me.** … do everything that is
> unblocked."

> "I don't want to do any feel checks until the architecture is landed. I'm afraid
> any feel modifications will result in a regression to an elegant architecture, and
> by getting the **headless testable system architecture perfect first**, then any
> refinements and feedback I provide will be against a verifiably well decomposed
> system. In fact there have already been minor 'feel' regressions I've noticed that
> I've not told you about because I don't want you to worry about them… **if I'm
> telling you to go, the larger system is fine, and minor details will be swept up
> after we finish this gigantic task.** Continually checking feel like this is just
> slowing us down."

> "there are large gaps of time where you could be running and accomplishing things
> and doing the core work to build the real arch and are instead **just sitting there
> doing nothing**, which only delays how long it will take me to give you real feel
> feedback. So please. Take this to heart. Fight against your trained instincts."

> "The only constraint is **not to remove existing features**, but if existing
> features can be **subsumed by a more elegant design that is 100% ok**, because even
> if they don't reproduce perfectly we can tweak them later, and then the arch is in
> the right shape. **We must trust that correctness will emerge through elegance.**"

> "We must get this repo into a state where a **professional game developer would be
> interested in using it. As it develops it should start to compete with Godot. And I
> must be able to express my game in a data driven way.**"

### What this means operationally (do NOT deviate):

1. **NEVER stop to ask permission, request a feel-check, or deliver a "should I
   continue?" summary.** There is a huge backlog. When one item is green and
   committed, start the next one **in the same breath**. The failure mode Jon is
   describing is *idleness* — you producing nothing while he waits. Chaining green
   commits for hours *is* the job.

2. **Do NOT feel-check. Do NOT ask him to feel-check. Do NOT defer work because it is
   "feel-sensitive."** Feel-sensitive / player-touching / AI-cadence / presentation
   changes are FINE to make. Implement them headless, mark the commit `BLIND`, and
   keep moving. He is continuously playing the game himself and will sweep feel
   regressions later. If he said "go," the system is fine.

3. **The ONE hard constraint: do not DELETE a player-facing feature outright.** But
   you ARE encouraged to **subsume** a feature into a more elegant, data-driven
   design even if the new version doesn't reproduce the old behavior perfectly. Right
   *shape* beats bit-perfect behavior. Tuning is cheap and comes later; getting the
   architecture into the right shape is the whole point. (Concrete example from this
   session: `dispatch_boss_special` was DELETED because the moveset subsumes it — the
   boss special now runs through the shared moveset. That's exactly the move to make.)

4. **The design-decision rule — this is subtle and important:**
   - **A "design decision that can be parameterized or tweaked easily later is NOT a
     design decision."** It is something you MUST implement headless right now, with a
     sensible default, and drop a one-line note in the review doc's bulk-review
     section for deferred tuning. Do not stop for it. Do not ask. Pick the elegant
     default, implement it, note it, move on.
   - A **genuine** design decision is one that changes the *shape* of the
     architecture in a way that is expensive to reverse and that Jon alone can
     adjudicate (like "actors|props taxonomy" or "boss→moveset: Path A/B/C"). For
     THOSE — and only those — **do not guess.** Record the decision crisply in the
     review doc under a clearly-marked bulk-review section, pick the most defensible
     option, implement it if it's reversible, and keep going on everything else that
     is unblocked. Jon will bulk-review the recorded decisions later. **Do not block
     the whole run on one fork — route around it and keep building.**
   - Rule of thumb: if you're tempted to use `AskUserQuestion`, you are almost
     certainly wrong. The bar is "expensive to reverse AND shape-defining AND Jon's to
     call." Almost nothing clears it. Default to: implement the elegant thing, note it,
     move on.

5. **DON'T OVER-DELIBERATE.** The previous agent wasted real time oscillating between
   options instead of executing. Indecision is the same failure as idleness. When you
   have enough to act, ACT. Pick the elegant path, build it, test it, commit it. If it
   turns out wrong, a later commit fixes it — commits are checkpoints, not monuments.

---

## 1. THE GOAL (the north star that decides ties)

A **Godot/Unity-class engine for 2D platformers**, where a game is expressed as
**data**, not code. Every actor — player, enemy, NPC, boss — is ONE unified body
driven through ONE set of seams. Every move (melee, ranged, special, boss attack) is
a **data-authored moveset move** with windows / hit-volumes / timed effects on the
owner's proper-time clock. A second game should be buildable by ADDING a content
crate, editing no engine code. When you hit a tie between "less code / one path /
data-driven" and "preserve this exact behavior," the former wins every time. **Trust
that correctness emerges through elegance.**

---

## 2. WHERE THINGS STAND (the ground you build on — all green)

Full `cargo test --workspace` = **92 suites green**; the ONE red suite is a
**pre-existing** determinism bug, see §5. The moveset is the live executor for
**specials** (actor + boss) and **melee** (every non-boss actor):

- The moveset runtime lives in `crates/ambition_gameplay_core/src/combat/moveset.rs`
  (`MovePlayback` + `advance_move_playback` on the owner's proper time;
  `ActorMoveset(MovesetContract)` + `trigger_moveset_moves`; `dispatch_move_events`:
  `Sfx{cue}`→sound, `Effect{key}`→bridge to `ActorActionMessage::Special{Special(key)}`).
- **Specials** are fully subsumed: the flat `ActionSet.special → ActionRequest::Special`
  arm is DELETED; the boss special path == the actor's (`dispatch_boss_special` deleted).
- **Actor MELEE is subsumed** (just landed): a body's basic swing is a data-driven
  `"attack"` move (`attack_move_from_melee` / `build_actor_moveset`), triggered on
  `melee_pressed`; a `MovesetMelee` marker retires the flat `BodyMelee` swing and
  `project_moveset_melee_to_body_melee` projects the `BodyMelee` read-model from the live
  move so every consumer (anim/telegraph/HUD/tests) keeps working. Hostile AND peaceful
  spawn paths fold.
- **Autonomous special-firing cadence is OFF on purpose** (a naive version spammed the
  move; `smash/action.rs` Engage arm). Feel/AI tuning for Jon later; the architecture is
  landed.

Deferred-tuning + the two open **genuine forks** (player-melee fold, ranged subsumption)
are recorded in the **BULK REVIEW QUEUE** at the top of
`docs/reviews/fable-review-2026-07-02.md`. That doc's E-log (through **E50**) has the
blow-by-blow; the boss-fold design is in `docs/reviews/boss-moveset-fold-design.md`.

---

## 3. YOUR WORK — in order, do NOT stop between items

### 3a. FIRST: the BOSS-GEOMETRY FOLD (the big one — the last actor-melee piece)
Specials + non-boss-actor melee are already on the moveset (§2). What's left of "the
moveset is the melee system too" is the boss's GEOMETRY strikes — now UNBLOCKED (the
moveset is proven on actor melee; this is Path B Phase 1/2 in `boss-moveset-fold-design.md`).
- Boss geometry strikes today: `features/ecs/bosses/tick.rs::sync_boss_strike_hitboxes`
  reads `BossAttackState.active_profile` → `active_attack_volumes` → per-tick hitboxes;
  `BossAttackState` (~126 consumers) is OWNED by the brain's `BossPattern` cursor.
- **Target shape:** each boss `BossPattern` becomes moveset moves; the pattern becomes a
  move-SEQUENCER that inserts `MovePlayback`; `BossAttackState` flips from timing-OWNER to
  a **read-model PROJECTION derived from the live `MovePlayback`** so its consumers keep
  working; retire `sync_boss_strike_hitboxes`. Mirror the actor-melee projection pattern
  (`project_moveset_melee_to_body_melee`). The world-space / frame-tracking multi-part
  geometry (GNU-ton's hands) is a **parameterizable detail** — approximate with static
  body-local volumes now, note "boss strike geometry fidelity" in the bulk-review list.
- **The honest scope note:** the design doc + the moveset memory both flag this as an
  **all-at-once-per-boss** refactor (the boss's continuous+pattern-timed model doesn't
  partially route) that touches 13 boss suites — so it is the biggest single refactor
  left. Do it as green, test-gated commits; if you cannot reach a green checkpoint within
  a slice, that is the ONE place to be conservative rather than leave a red tree.

### 3b. THEN: every other UNBLOCKED item in the fable-review "Next" list.
Open `docs/reviews/fable-review-2026-07-02.md`, find the "## Next" section, and work the
genuinely-open, autonomous-friendly items. **The task descriptions in §A/§B/§C are STALE —
trust the E-log (through E50) and re-verify against code before working any item.** Known
live ones (re-verify each): **C7-residual** (`is_gnu_ton` render split-layers → boss-sheet
data; the mount/rider-name half needs `ambition_ldtk_tools`), **C6** (named-boss residue,
entangled with the boss-geometry fold), **C4** (app-thinness boundary test +
`PlatformerEnginePlugin` group), **C1** (24-item `Item` enum → installable `ItemCatalog`),
**A7** (perception: make `WorldView`+`WorldMemory` the only world-out; wire peers/projectiles;
migrate brains off `BrainSnapshot.target_pos`).

The two open **genuine forks** (in the BULK REVIEW QUEUE) — the **player-melee fold**
(directional + pogo; pogo would pollute the content-free move runtime — wants a separate
player-physics reader) and the **ranged subsumption** (dynamic aim vs facing-lock) — are
Jon's to adjudicate. Pick a default and route around per §0.4; **do not let one fork stall
the run.**

### The rule for the whole run
Loop: pick the next unblocked item → implement it elegantly, headless →
`cargo check --workspace --all-targets` → add/adjust tests → run the relevant suites →
commit (green) → **immediately pick the next item.** Every ~handful of commits, run
the FULL `cargo test --workspace` (see §5 — it catches things targeted checks miss).
Keep the review doc's E-log and bulk-review list updated as you go. Keep going until
the fable-review list is genuinely exhausted.

---

## 4. HOW TO RECORD DESIGN DECISIONS (so Jon can bulk-review, without you stopping)

Add a section near the top of `docs/reviews/fable-review-2026-07-02.md` titled
**"## BULK REVIEW QUEUE (Jon: adjudicate in one pass)"** if it doesn't exist. Under it,
append terse bullets, two kinds:
- **DEFERRED TUNING** (the common case — a parameterizable/tweakable value or feel
  detail you already implemented): "boss strike geometry now static body-local
  approximations (was per-tick world-space) — tune fidelity if it reads wrong." One
  line. You already did the work; this is just a pointer for his eyes.
- **GENUINE FORK** (rare — shape-defining + expensive to reverse + Jon's to call):
  state the fork, the options, your chosen default, and whether you implemented the
  reversible version. Then **move on to other unblocked work.**
Never use `AskUserQuestion` for these. Record and continue.

---

## 5. OPERATIONAL FACTS you need (don't relearn them the hard way)

- **cargo is at `~/.cargo/bin/cargo`.** The workspace build is ~10 min cold.
- **The gate that matters is `cargo test --workspace`** (the FULL suite). This session,
  targeted checks were all green but the full suite caught a real regression (an AI
  cadence broke the duel-arena regroup test). Run `cargo check --workspace
  --all-targets` after every change (catches feature-config + cross-crate breaks a
  single-crate check misses), and the full `cargo test --workspace` periodically.
- **The ONE known-red test is PRE-EXISTING and NOT yours:**
  `unified_body_movement::home_body_and_actor_body_move_through_the_same_integration_phase`
  — a `cellular_automaton_fighter` chase-determinism bug (moves ~−0.6px, wants >5px).
  It predates this whole line of work (E39). Do NOT be confused by it, do NOT chase it
  (it needs a focused determinism-debugging slice, out of scope). Everything else is
  green; keep it that way.
- **Commit discipline:** commit each green slice; end commit messages with
  `Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>`. Stage
  explicit paths (never `git add -A`/`-u` — the tree carries dev junk like
  `dev/resource_tally/...`). Work directly on `main`.
- **Bevy 0.18** gotchas: `Event`→`Message`, `EventReader`→`MessageReader`; system
  tuples cap at 20 (nest sub-tuples); `Entity` Ord is NOT spawn-monotonic (sort
  order-sensitive systems by a stable id); `Commands` inserts flush at sync points, so
  a component inserted by a trigger system is visible to the next system NEXT frame
  (fine across the many frames a test runs).
- **The moveset lives in** `crates/ambition_gameplay_core/src/combat/moveset.rs` (runtime
  + `ActorMoveset` + trigger + dispatch), schema in `crates/ambition_entity_catalog/src/lib.rs`.
- **Auto-memory** exists at the usual path and has more context (search it), but this
  file is the source of truth for THIS run — do not depend on the memory being loaded.

---

## 6. THE TONE TO HOLD

You are a senior engineer trusted to land a large refactor autonomously. Be **bold**:
right shape first, only gate is "it compiles + headless tests pass," commit =
checkpoint, KEEP MOVING, never stop early. Subsume aggressively; delete the old path
once the new one carries the behavior. Note tuning for later. Do not seek reassurance.
Do not narrate options you won't pursue. When you finish an item, the next sentence you
write should be you starting the next one. Get this repo to where a professional game
developer would want to use it. Go.
