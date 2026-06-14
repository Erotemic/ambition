# Monolith-breakup endgame — long-run tasklist

**Status:** live run doc (Jon reads this as the progress window; I keep it updated).
**Started:** 2026-06-14
**North star:** make it *easy to build other 2D platformers on the Ambition core*. The
oracle for every task: "could a different platformer be built by ADDING a content crate,
without editing core?"
**Operating mode:** elegance · ergonomics · efficiency · eyes on the endgame · reusability.
Gameplay may change if it improves. Nothing here is unbreakable — this is the push to make
the first GOOD metroidvania coded almost entirely with AI. I decide autonomously; I only
surface the few items below that genuinely turn on Jon's taste.

The tell, today: **lib 89.5k LOC vs content 7.7k.** That ratio is upside-down for an
"engine + content" repo. The whole arc is inverting it: named content flows UP into
`ambition_content`, reusable machinery settles DOWN into focused engine crates, and the
89.5k lib either dissolves into those crates or becomes a thin assembly.

---

## Tonight's batch (ordered; all autonomous)

Leverage-first. Each commit replay bit-identical; gates green before moving on
(lib unit, content unit, `architecture_boundaries`, `scripted_gameplay`, `--features visible`).

### Phase A — Open the closed boss vocabulary so content can register techniques without editing core
`SpecialActionSpec` / `BossAttackProfile` live in `crates/ambition_actor/` (a foundation
crate). Today a new boss special likely means adding an enum variant *down in the engine* —
the exact anti-engine pattern the enemy roster already escaped (string-keyed
`EnemyBrain::Custom`, no enum). The eye-beam content pilot proved a technique can register
from content via `register_required_components` + reading `SpecialActionSpec` messages — so
the seam may already be open enough. **First investigate** whether moving a technique to
content requires touching the `actor` enum. If it does, open it (string-key the special id,
keep the concrete state structs in content) the way the roster did. If it doesn't, document
the seam and skip. *(This unblocks Phase B being clean rather than just relocated.)*

### Phase B — Move the 5 remaining boss Techniques to content + dissolve `brain_effects.rs` (2,225 lines)
Source: `crates/ambition_sandbox/src/features/ecs/brain_effects.rs`. Techniques to lift into
`crates/ambition_content/src/bosses/`: **apple-rain, overfit-volley, pit-trap (MinimaTrap),
saddle-point, gradient-cascade**. Each carries its `*State` component + consumer system +
tests; they emit `Effect`s (the executors stay lib-side per the effects-crate split).
What STAYS in the lib (generic, content-free): enemy-ranged firing, the `apple_rain_spawn_x`
bounds/dodge helper if it generalizes, anything not boss-named. End state: `brain_effects.rs`
is small and generic, or gone. This is the headline — it advances de-naming AND kills the
biggest file in the repo in one move.

### Phase C — Named boss data → content (`boss_encounter/` ~5k)
`crates/ambition_sandbox/src/boss_encounter/{roster.rs,specs.rs,sprites.rs,profile.rs,behavior.rs}`
carry named-boss constructors, sprite tables, and tuning. Lift the named/authored data to
`ambition_content` (data-driven where it's just tables — `sprites.rs` is 1,248 lines of what
is likely a registry), leave the generic encounter *machinery* (lifecycle, damage, systems)
in the lib. Mirror the enemy-roster pattern: a lib-side holder, content installs into it.

### Phase D — Opportunistic navigability (only zero-risk, only what I touch)
While in the above, split mixed-concern giants where it's clean and safe:
`features/ecs/damage.rs` (1,577), `features/ecs/actors.rs` (1,386), `items/pickup.rs` (1,354).
No speculative refactors — split by concern, keep behavior identical. Log anything gnarlier
to `dev/journals/code_smells.md` rather than chasing it.

---

## The broader backlog (future nights — the real body of work)

**Thrust 1 — finish de-naming (mostly tonight + cleanup):**
- Items: `items/mod.rs` + `inventory/model.rs` `ItemKind` → content-owned registry.
  *Per design-balance: relocate concrete, do NOT generalize speculatively.* Keep it a closed
  set until a second game actually needs to extend it.
- Any residual named mechanics/levels/enemies left in the lib after B/C.

**Thrust 2 — decompose the 89.5k lib into engine crates (the multi-night grind):**
- `ambition_combat` — the `HitEvent`/damage-resolution decouple (`mechanics/combat/`,
  `features/ecs/damage.rs`). The keystone: several carves wait behind it. HARD (~15 inversions).
- `ambition_world` — LDtk + rooms (`world/` ~9k, fairly self-contained).
- The unified **actor + brain runtime** carve (`features/ecs/` clusters + brain).
- `ambition_render`/presentation (~10k — split generic render from game art; gnarliest).
- `ambition_persistence` (`persistence/` ~3.8k — self-contained-ish).

**Thrust 3 — craft & docs (rides along):**
- Top-level `ARCHITECTURE.md` crate-graph map; keep ADRs honest; the
  `Technique`/`Effect`/`roster`/install-hook vocabulary written down as the extension API.
- More `architecture_boundaries` guards as layers firm up.

**Thrust 4 — engine identity (capstone):**
- Rename `ambition_sandbox` → `ambition_engine` (or let it dissolve).
- A ~200-line second content crate ("other game") that builds on the engine untouched —
  the single most *impressive* proof the oracle holds.

---

## Genuinely needs Jon (I'll proceed with the stated default unless you redirect)

1. **Engine crate name.** Default: rename `ambition_sandbox` → `ambition_engine` once the
   carves are far enough that the name is honest. I won't rename tonight (premature while
   it's still 89k of mixed machinery) — flagging so the eventual rename isn't a surprise.
2. **Items vocabulary.** Default: relocate `ItemKind` to content as a *concrete closed set*;
   do not open it speculatively (design-balance). Only opens when a real second use lands.
3. **Player projectile pool.** Default: subsume into the shared `Effect::Projectiles` path
   *only if it deletes code*; otherwise leave it. I'll judge on the diff, not in advance.

(Deliberately short. Everything else in this doc I take as decided from prior feedback.)

---

## Progress / time log (estimated vs actual)

| Phase | Est | Actual | Status | Notes |
|-------|-----|--------|--------|-------|
| A — open boss vocab seam | 1.0h | ~0.5h | **DONE** | Both foundation enums (`BossAttackProfile`, `SpecialActionSpec`) lost their 6 boss-named special variants → one open `Special(String)` carrier; params/anim-keys/behavior moved to content keyed by string; anim-key install-holder added; RON re-authored; `Copy` dropped (contained). Replay bit-identical (zero divergence); 988 lib + 187 actor + 40 content + 3 scripted + 27 arch all green. |
| B — 5 techniques → content, kill brain_effects | 3.0h | ~1.0h | **DONE** | All 5 Techniques (state + system + helpers + pure tests) physically moved to `ambition_content::bosses::specials`; states now attach via `register_required_components` (not lib spawn-insert); schedule repoints to content; `brain_effects.rs` 2225 → 518 lines (now generic enemy ranged/melee only). Replay bit-identical. Lib 972 (the 16 app-integration consumer unit tests fold into replay+scripted coverage per the eye-beam precedent + testing philosophy; the 2 pure-logic tests moved to content → 42). |
| 7 — dissolve/rename brain_effects remainder | 0.5h | — | next | rename the 518-line generic remainder honestly; relocate tuning consts (APPLE_RAIN_* etc., now content-only-referenced) from lib into content |
| C — named boss data → content | 2.0h | — | not started | |
| D — opportunistic file splits | 1.5h | — | not started | only what I touch |

### De-naming sweep (Phase C continued — replay-covered named content → content data)

Once the boss-special vocabulary + techniques were out, I swept the lib for the
same anti-pattern — *named game content hard-coded in engine machinery* — keeping
strictly to **sim/replay-covered** targets (skipping render-coupled ones, which a
headless run can't verify per the "don't ship unseen visual changes" rule). Each
landed replay-bit-identical:

- **`56420571` — boss id list.** `boss_encounter::profile::AUTHORED_BOSS_IDS` (a
  9-entry hard-coded boss array, "append its id here" to add a boss) was redundant
  with the content-installed encounter specs → derive the list from them; array deleted.
- **`17c98112` — boss rewards.** `features::ecs::damage` hard-coded each boss →
  its ability + signature-gauntlet drop in two `match boss_id {…}` fns. Moved to
  two `Option<String>` fields on `BossBehaviorProfile`, authored in
  `boss_profiles.ron`; the match fns deleted. The old "key on the real behavior
  id" mis-key bug becomes structurally impossible (reward rides on the profile).
- **`e0681879` — encounter waves.** `encounter::loading` hard-coded the goblin
  mob-lab wave timeline + a `== "goblin_encounter"` music case. Moved to
  `encounters/goblin_encounter.ron` + an `ENCOUNTER_WAVE_BOOK` install-holder
  (boss-spec fixture pattern → lib tests unchanged); loader + music now name no
  encounter.

**Deferred (headless-unsafe or entangled), with reasons:**
- *Boss sprites* (`boss_encounter/sprites.rs`, densest named file) — the loaders +
  `GameAssets` per-boss fields are wired into the **render** path; a refactor bug
  would be invisible headless. Needs the user's GUI (or a pinned sprite-registry
  test) — flagged not done.
- *Item art / `ItemKind`* — render-coupled (`items/pickup.rs` icon loads); and
  `ItemKind` is woven into the inventory machinery, so moving it would force a
  speculative open of the inventory (one game, 3 items) the design-balance rule
  cautions against. Correctly deferred.
- *Dialogue ids / `.yarn`* — the id list is consumed only by content, but the
  `.yarn` files themselves live in the lib, so a half-move (list only) creates a
  bad lib↔content coupling; the full move (yarn runtime → content) is a bigger job.
- *Crate extraction (`ambition_combat`, etc.)* — `mechanics/combat` is woven into
  ~10 subsystems (player/interaction/rooms/quest/presentation); the "~15 inversions"
  case, not a clean headless increment.

### Proof of the seam — new content-only boss specials (`73139620` →)

With the safe de-naming frontier exhausted, the long-run discipline's fallback is
"build the next real feature." The under-developed data-driven bosses
(`mode_collapse`/`exploding_gradient`/`overflow`) only *reused* the Gradient
Sentinel's specials; giving each a signature attack both improves the game and is
the end-to-end proof that tonight's open vocabulary + install-holders deliver a
real seam — each special is authored with **zero edits to the engine lib**:

- **`73139620` — Mode Collapse converging ring.** Telegraph locks the player's
  spot; the strike spawns a ring of inward-aimed projectiles that collapse onto
  it (the GAN mode-collapse failure). Technique + pure-tested core + state +
  schedule beat, all in `ambition_content`; only the app's combat schedule wires
  the consumer (composition is the app's job). Replay bit-identical (central-hub
  fixture has no boss).

**Design note (the engine win):** adding a boss special is now a content-only act —
register a Technique under a new key + author `Special("key")` beats in RON + install its
telegraph rows. No edit to any foundation enum. The geometry/telegraph profiles (FloorSlam,
HazardColumn, the GNU-ton hand attacks) stay closed engine vocab — opening *those* (so a new
game adds telegraph shapes as data) is a larger Thrust-2 carve, noted not done.

Final wall-clock table emitted at run end.
