# HANDOFF — finish the fable-review todo list (melee subsumption is now COMPLETE)

**You are continuing a large architecture refactor. Read this whole file before you
touch code. Then WORK — for hours, without stopping to ask.** This prompt is
self-contained: do not assume you can see any auto-memory. Everything you need is
here or in the review doc it points to
(`docs/reviews/fable-review-2026-07-02.md` — its E-log + BULK REVIEW QUEUE).

---

## 0. THE MANDATE — read this twice, then fight your instincts to violate it

These are the repo owner's (Jon's) words. They override your trained instincts. When
you feel the pull to stop, summarize, checkpoint, or ask — that pull is the thing to
fight.

> "I want you to finish every unblocked item on the fable-review todo list **without
> stopping to ask me.** … do everything that is unblocked."

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
   continue?" summary.** When one item is green and committed, start the next one **in
   the same breath**. The failure mode is *idleness* — you producing nothing while he
   waits. Chaining green commits for hours *is* the job.

2. **Do NOT feel-check. Do NOT ask him to feel-check. Do NOT defer work because it is
   "feel-sensitive."** Feel-sensitive / player-touching / AI-cadence / presentation
   changes are FINE to make. Implement them headless, mark the commit `BLIND`, and keep
   moving. He is continuously playing the game himself and will sweep feel regressions
   later. If he said "go," the system is fine.

3. **The ONE hard constraint: do not DELETE a player-facing feature outright.** But you
   ARE encouraged to **subsume** a feature into a more elegant, data-driven design even
   if the new version doesn't reproduce the old behavior perfectly. Right *shape* beats
   bit-perfect behavior. Tuning is cheap and comes later. (Concrete recent example:
   `sync_boss_strike_hitboxes` was DELETED because the boss's strikes now run through the
   shared moveset — the exact move to make.)

4. **The design-decision rule — subtle and important:**
   - **A "design decision that can be parameterized or tweaked easily later is NOT a
     design decision."** Implement it headless right now with a sensible default and drop
     a one-line note in the review doc's BULK REVIEW QUEUE for deferred tuning. Do not
     stop. Do not ask. Pick the elegant default, implement it, note it, move on.
   - A **genuine** design decision changes the *shape* of the architecture in a way that
     is expensive to reverse and that Jon alone can adjudicate. For THOSE — and only
     those — **do not guess.** Record it crisply in the review doc's BULK REVIEW QUEUE,
     pick the most defensible option, implement it if it's reversible, and keep going on
     everything else. **Do not block the whole run on one fork — route around it.**
   - Rule of thumb: if you're tempted to use `AskUserQuestion`, you are almost certainly
     wrong. Default to: implement the elegant thing, note it, move on.

5. **DON'T OVER-DELIBERATE.** Indecision is the same failure as idleness. When you have
   enough to act, ACT. Pick the elegant path, build it, test it, commit it. Commits are
   checkpoints, not monuments — a later commit fixes a wrong one.

---

## 1. THE GOAL (the north star that decides ties)

A **Godot/Unity-class engine for 2D platformers**, where a game is expressed as **data**,
not code. Every actor — player, enemy, NPC, boss — is ONE unified body driven through ONE
set of seams. Every move (melee, ranged, special, boss attack) is a **data-authored
moveset move** with windows / hit-volumes / timed effects on the owner's proper-time
clock. A second game should be buildable by ADDING a content crate, editing no engine
code. When you hit a tie between "less code / one path / data-driven" and "preserve this
exact behavior," the former wins every time. **Trust that correctness emerges through
elegance.**

---

## 2. WHERE THINGS STAND (the ground you build on — all green)

Full `cargo test --workspace` = green **except ONE pre-existing red** (see §5). **The
melee subsumption is COMPLETE** — specials, every non-boss actor's melee, AND the boss's
geometry strikes now all run through the ONE moveset runtime
(`crates/ambition_gameplay_core/src/combat/moveset.rs`):

- **Actor melee** (E49/E50): a body's swing is a data-driven `"attack"` move
  (`attack_move_from_melee` / `build_actor_moveset`), triggered on `melee_pressed`; a
  `MovesetMelee` marker retires the flat `BodyMelee` swing, projected back by
  `project_moveset_melee_to_body_melee`.
- **Specials** (E47/E48): the flat `ActionSet.special` arm + `dispatch_boss_special` are
  DELETED; the boss special path == the actor's.
- **Boss GEOMETRY strikes** (E51, `7ecae45a`): `boss_attack_moveset` builds one move per
  authored strike profile — a geometry profile carries its static hit volumes (from
  `volumes_for_profile` at a body-local origin), a `Special(key)` sustains an `Effect`.
  `trigger_boss_attack_moves` (was `trigger_boss_special_moves`) inserts the matching
  `MovePlayback` for the active profile; `advance_move_playback` spawns the Boss-faction
  hitbox through the shared `apply_hitbox_damage` path. **`sync_boss_strike_hitboxes` +
  `FrameDrivenBossStrike` are DELETED.**
- **C7-render** (E52, `323c2107`, BLIND): split-layer boss render is now a generic
  `{key}_body` / `{key}_hands` asset CONVENTION, not the `is_gnu_ton` string match.

The two open **genuine forks** (player-melee fold, ranged subsumption) + all deferred
tuning are in the **BULK REVIEW QUEUE** at the top of the review doc. The E-log (through
**E52**) has the blow-by-blow.

---

## 3. YOUR WORK — the "## Next" list in the review doc, biggest-lever first

Open `docs/reviews/fable-review-2026-07-02.md`, read the **"## Next (in order)"** section
+ the E-log tail, and **re-verify each item against code before working it** (the audit's
§A/§B/§C task text is STALE — trust the E-log). The genuinely-open, autonomous-friendly
items right now:

### 3a. Boss `BossAttackState` → full PROJECTION (E51's recorded next slice — the big one)
E51 banked the load-bearing win (one damage path; the bespoke hitbox poll is gone) but
**`BossAttackState` still OWNS strike timing** (the pattern cursor writes it; the moveset
move is slaved to `active_profile`). Finish the shape:
- Flip the `BossPattern` from a timing-OWNER into a pure **move-SEQUENCER** that inserts
  `MovePlayback`s; make **`BossAttackState` a PROJECTION derived from the live
  `MovePlayback`** (mirror `project_moveset_melee_to_body_melee` — the ~37 consumers keep
  reading the same fields, you only change the WRITER).
- Map the pattern's `Telegraph{profile}` / `Strike{profile}` / `Rest` steps onto a move's
  `Startup` / `Active` / `Recovery` windows.
- Watch the ordering trap: `tick_boss_pattern`'s movement code (`emit_desired_vel`,
  strike-speed-scaling, the `melee/special_pressed` edges) reads `active_profile` in the
  same tick — sequence the projection so those reads stay coherent.
- **Honest scope:** this is the "all-at-once-per-boss" refactor (13 boss suites — the
  boss's continuous+pattern-timed model doesn't partially route). Do it as GREEN,
  test-gated commits. **If you cannot reach a green checkpoint within a slice, that is the
  ONE place to be conservative rather than leave a red tree** — bank what's green and
  route to 3b.

### 3b. The other unblocked items (any order; pick what you can land green)
- **A7 — perception** (L): make `WorldView` + `WorldMemory` the ONLY world-out; wire
  peers/projectiles; migrate brains off `BrainSnapshot.target_pos`.
- **C1 — item catalog** (L): the 24-item `Item` enum → an installable `ItemCatalog`
  (consumed across menu IR / yarn / persistence — trace every consumer first).
- **C4 — app thinness** (L): machinery-owned `PlatformerEnginePlugin` group; fold
  `app/sim_systems.rs` into owning gameplay plugins; extract `host/mobile_input/` beside
  `ambition_input`; add an app-thinness boundary test.
- **C6 — named-boss residue** (M): post-E51 the 11 geometry `BossAttackProfile` variants
  are consumed by `volumes_for_profile` + hurtbox-pose selection + anim rows — collapse
  their hardcoded geometry toward authored rect DATA; migrate named constructors +
  `MOCKINGBIRD_*` consts; per-boss sheet specs → boss roster RON.

### Blocked / do NOT autonomously start
- **C7 rider-name half** — mount composition still parses `" on Shark"` from the spawn
  NAME; the fix authors a `mount:` spawn field, which needs `ambition_ldtk_tools` (never
  hand-edit `.ldtk` JSON — build the missing subcommand or leave it).
- **The two genuine forks** (player-melee fold; ranged subsumption) — Jon's to adjudicate.
  Pick a defensible default, note it, route around; do NOT let a fork stall the run.

### The rule for the whole run
Loop: pick the next unblocked item → implement it elegantly, headless →
`cargo check --workspace --all-targets` → add/adjust tests → run the relevant suites →
commit (green) → **immediately pick the next item.** Every ~handful of commits, run the
FULL `cargo test --workspace`. Keep the review doc's E-log + BULK REVIEW QUEUE live.

---

## 4. HOW TO RECORD DESIGN DECISIONS (so Jon can bulk-review, without you stopping)

Append terse bullets to **"## BULK REVIEW QUEUE"** at the top of the review doc — two
kinds:
- **DEFERRED TUNING** (common — a parameterizable value/feel detail you already
  implemented): one line, a pointer for his eyes.
- **GENUINE FORK** (rare — shape-defining + expensive to reverse + Jon's to call): state
  the fork, the options, your chosen default, and whether you implemented the reversible
  version. Then **move on.**
Never use `AskUserQuestion` for these. Record and continue.

---

## 5. OPERATIONAL FACTS you need (don't relearn them the hard way)

- **cargo is at `~/.cargo/bin/cargo`.** The workspace build is ~10 min cold.
- **The gate that matters is `cargo test --workspace`** (the FULL suite). Run
  `cargo check --workspace --all-targets` after every change (catches feature-config +
  cross-crate breaks a single-crate check misses), and the full `cargo test --workspace`
  periodically — targeted checks miss cross-cutting regressions.
- **The ONE known-red test is PRE-EXISTING and NOT yours:**
  `unified_body_movement::home_body_and_actor_body_move_through_the_same_integration_phase`
  — a `cellular_automaton_fighter` chase-determinism bug (moves ~−0.6px, wants >5px, E39).
  Do NOT chase it (out of scope, needs a focused determinism slice). Everything else is
  green; keep it that way.
- **Commit discipline:** commit each green slice; end commit messages with
  `Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>`. Stage explicit
  paths (never `git add -A`/`-u` — the tree carries dev junk like
  `dev/resource_tally/...`, `tools/ambition_music_renderer`). Work directly on `main`.
- **Key files:** moveset runtime `crates/ambition_gameplay_core/src/combat/moveset.rs`;
  moveset schema `crates/ambition_entity_catalog/src/lib.rs`; boss attack moveset builder
  `crates/ambition_gameplay_core/src/features/bosses.rs` (`boss_attack_moveset`); boss
  tick + trigger `crates/ambition_gameplay_core/src/features/ecs/bosses/tick.rs`; boss
  pattern brain `crates/ambition_characters/src/brain/boss_pattern/{mod,tick}.rs`; boss
  attack geometry `crates/ambition_gameplay_core/src/boss_encounter/attack_geometry/mod.rs`.
- **Bevy 0.18** gotchas: `Event`→`Message`, `EventReader`→`MessageReader`; system tuples
  cap at 20 (nest sub-tuples); `Entity` Ord is NOT spawn-monotonic (sort order-sensitive
  systems by a stable id); `Commands` inserts flush at sync points (a component inserted
  by a trigger is visible NEXT frame — fine across a multi-frame test).
- **Auto-memory** exists at the usual path with more context (search it), but this file is
  the source of truth for THIS run — do not depend on the memory being loaded.

---

## 6. THE TONE TO HOLD

You are a senior engineer trusted to land a large refactor autonomously. Be **bold**:
right shape first, only gate is "it compiles + headless tests pass," commit = checkpoint,
KEEP MOVING, never stop early. Subsume aggressively; delete the old path once the new one
carries the behavior. Note tuning for later. Do not seek reassurance. Do not narrate
options you won't pursue. When you finish an item, the next sentence you write should be
you starting the next one. Get this repo to where a professional game developer would want
to use it. Go.
