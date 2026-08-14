# Queue sections pruned on 2026-08-13

These sections were removed or distilled from the live self-replenishing queue after their completed investigation/implementation history was verified. The live queue keeps only the remaining executable work or maintainer-decision pointer.

---

- ▢ **D33 ✔✔ THE CARVE LANDED `a64bf22f8` (2026-08-09) — `ambition_character_sprites`
  EXISTS AND THE CRITICAL PATH DID NOT MOVE.** The run's standing top priority.
  ▢ **one step remains and it is a measurement, not code** — see the bottom of
  this block.

  | | before | after |
  |---|---|---|
  | **`critical_path_crates`** | **12** | **12** — ⭐ *the plugin shape held* |
  | `worst_edit_cost_seconds` | 1,267.4 s | 1,271.9 s (**+4.5 s**) |
  | `edit_cost_seconds` (actor crate) | 979.3 s | 978.1 s |
  | `largest_unit_lines` | 113,381 | 111,237 (**−2,144**) |
  | first-party crates | 55 | 56 |

  ✔ gate exit 0 · `scripts/tests` **287** · contracts **25/25** · actor crate
  **1,191** tests · new crate **37**. ⭐ **the before-numbers were re-measured in a
  throwaway worktree at clean `HEAD`** because three other agents had landed
  meanwhile — identical, **so the whole +4.5 s is this carve's.**

  ### ⛔⛔ FOUR CORRECTIONS TO MY BRIEF, AND THE FIRST ONE IS MINE TWICE OVER

  1. ⛔⛔ **I called the ledger's numbers stale and I was the one who was wrong.**
     I re-ran `--carve` on `character_sprites/` and reported **3,887 lines /
     +7.6 s**, overriding the row's *"2,123 lines / ≈+4.1 s"*. But **3,887 is the
     whole DIRECTORY, and the whole directory cannot move**: `assets.rs` (1,091)
     and the module's `tests.rs` (595) must stay, because `assets.rs` names
     `crate::assets::platformer_assets`, `crate::persistence::settings` and
     `crate::character_roster`, and **8 actor-crate production files consume its
     symbols** — moving it recreates the owner→carve edge the plugin shape exists
     to kill. **The subset is 2,144 lines and it measured +4.5 s.** ⇒ the original
     figure was right to within 1%.
     ⭐ **this is my own [[reference_measure_the_suspect_not_the_aggregate]]**: I
     measured a *directory* and called it a *carve*, then corrected a correct row
     with it — and sent the wrong number to the worker as an urgent fix. ⚠ **the
     tell I ignored: I never asked whether the population I measured could
     actually move.**
  2. ⛔ **"repoint directly, not via a fresh alias" is FORBIDDEN BY POLICY.**
     `game.demo-mary-o-umbrella-only` allows Mary-O exactly one dependency
     (`ambition_platformer2d`) and `game.app-reaches-lower-through-facade` forbids
     `ambition_app/src` naming engine crates. So the facade gained
     `pub use ambition_character_sprites as character_sprites;`.
     ⭐ **and the worker's distinction is the right one**: the alias the carve
     removes is `actors::character_sprites`, and **`actors` IS the actor crate**,
     which is *why* it forced the edge. A re-export at the facade — already above
     everything — forces nothing. `sim_view` and `runtime` do name the crate
     directly.
  3. ⚠ **the repointing was 10 files across 6 crates**, not *"sim_view, runtime
     and ~4 test files"*. The one I missed is **production code**:
     `game/ambition_content/src/player_robot_lineage.rs`, four
     `authored_body_pixel_size` call sites.
  4. ⚠ **Mary-O had to be touched** despite my do-not-collide list — pure path
     renames in `{ai_slop,powerups,snake}.rs`. **They checked the other agent's
     actual files first and found zero overlap.** ⇒ the list was a proxy again;
     they verified the condition. [[feedback_briefs_name_the_condition_not_the_proxy]].

  ### ⭐⭐ THE HAZARD THAT NEARLY FIRED — and it is a general rule

  Moving a registration into a plugin that only `PlatformerEnginePlugins` adds is
  **[[reference_app_only_presentation_class]] in reverse**. Two places add
  `WorldPrepSchedulePlugin` **standalone** — `demo_mary_o/src/lib.rs:2589` and
  `src/movement/tests.rs:141`. Both are `#[cfg(test)]` and neither asserts
  anything about a posed body, **so nothing half-runs**.
  ⛔ **had either been production, Mary-O would have shipped an engine that
  half-runs with every test green.** ⇒ ⭐ **before moving a registration out of a
  plugin, grep for that plugin's OTHER adders.**

  ### ⭐ FIVE REGISTRIES A NEW CRATE TOUCHES THAT THE ROW NEVER NAMED

  * **capability footprint 40 → 41** (`check_absence_contracts.py` stops a new
    crate name) — moved in-commit with a `_comment`, per the
    `ambition_geometry` / `ambition_projectile_spec` precedent. ⚠ nothing new is
    *linked*; same code, new name.
  * **every crate needs a `MODULES.md`** or `test_modules_md_is_current.py` reddens.
  * **`docs/planning/status.md`** carries a `workspace-members count=` marker that
    `check_agent_kb.py` cross-checks (63 → 64).
  * **five lockfiles**, all verified under `cargo tree --locked`.
  * `ambition_entity_catalog` is a **dev-dependency** of the new crate, so
    `cargo tree --edges normal` never sees it.

  ### ✔ ONE TEST LOOSENED, DELIBERATELY AND CORRECTLY

  `test_an_unmeasured_crate_is_priced_at_the_median_and_reported_as_a_guess`
  asserted `unpriced_crates == ["ambition_invented"]`, which **silently also
  asserts "and the working tree has no unpriced crate"** — a different claim that
  *any* legitimate carve makes false before its `compile_collect.py` run. Now a
  membership check, reason written in. ⭐ the tree-level guard is the ratchet's own
  `UNPRICED` finding, which fired correctly.

  ### ⛔⛔ THE FIRST COLLECTION COULD NOT PRICE THE CRATE — a cold build supplies
  ### NO WEIGHTS, and re-freezing on it SILENCED the warning

  Ran `compile_collect.py --config release --phase first-party` the moment the
  fleet emptied. **685 unit rows in 1,082 s**, and the new crate is in them:
  `ambition_character_sprites, target: "", seconds: 4.44, lines: 2212,
  first_party: true, opt_level: "3"` ⇒ **2.007 ms/line**, not the 2.557 median.

  ⛔ **And the ratchet still says `1 at the median`.** The re-frozen
  `unit_weights` came from a build dated **2026-08-08**, not from that run:

  ```text
  MY RUN    fresh:   0   dirty: 685   ⇒ cache_class = COLD
  BASELINE  fresh: 632   dirty:  57   ⇒ cache_class = REBUILD
  ```

  ⇒ **weights are drawn from REBUILD-class builds only** — correctly, since a
  cold build measures a different thing — and **a first collection into a fresh
  target dir is always cold.** ⭐ **so `compile_collect.py` cannot price a crate
  on its first run into a new target dir, which is exactly when a carve needs
  it.** ⚠ **both runs carry the identical label** *"collector:
  release/first-party"*, so nothing in the output distinguishes them; only the
  counters do — [[reference_compile_cost_measurement]]'s *"a config LABEL lies"*,
  hitting the collector this time rather than the reader.

  ⛔⛔ **AND MY `--update` MADE IT WORSE BEFORE IT MADE IT BETTER.** Re-freezing
  wrote the median-priced crate **into the baseline**, so the `UNPRICED` finding
  stopped firing — the ratchet now reports *"ok, every guarded compile-cost
  number is inside its budget"* **while still pricing a real crate off a
  placeholder.** ⇒ **I silenced a warning by accepting it**, which is the exact
  failure this run has been prosecuting all day (D66's coincidence, the
  re-freeze-hides-the-limitation trap I refused twice for workers and then did
  myself).

  ### ✔✔ REPAIRED — 56 OF 56 CRATES MEASURED, `5ee38bc06`

  The second collection into the now-warm dir: **58 rows in 451 s, fresh 627 /
  dirty 58 ⇒ `cache_class = rebuild`.** `ambition_character_sprites` measures
  **4.80 s / 2,212 lines = 2.170 ms/line** against the 2.557 median it had been
  guessed at. Baseline re-frozen; the weights line now reads

  ```text
  weights   release/opt-3 rebuild, 2 build(s), 56 crates measured
  ```

  — **no `1 at the median`, and the `UNPRICED` finding is gone because it was
  ANSWERED, not accepted.** 287 repo-tooling tests pass.

  ⚠ **every guarded number moved**, and it is worth knowing why before someone
  reads it as a regression: `edit_cost_seconds` 978.9 → **1,082.0** (actor crate),
  1,268.1 → **1,385.9** (`platformer2d_core`), largest unit 222.6 → **241.1**.
  ⇒ **the graph did not change between the two runs** — the whole tree is now
  weighted from **two** rebuild builds instead of one. **New weights, not new
  cost.**

  ⭐ **the operational rule this leaves**: *"run `compile_collect.py` to measure
  them"* is **incomplete advice for a fresh target dir**. ⇒ **run it TWICE** — the
  first pass populates the directory and is cold and unusable for weights; the
  second is a rebuild and is the one that counts. **~1,082 s then ~451 s.**

  ⚠ **the first run also enumerated 55 lib roots where the second found 56**; the
  perturbation list was transiently short while the lockfile settled, though the
  cold build measured all 56 regardless. **Not chased** — the warm run is correct
  and this only mattered while the first was the only data.

  ### ✔ THE MEASUREMENT LANDED — re-verified 2026-08-10, and this ▢ was STALE

  The block below said the ratchet *"exits 1 on the `UNPRICED` finding"* and owed
  a `compile_collect.py` run on an empty fleet. Both were already discharged by
  `5ee38bc06`'s **56 of 56** repair, four blocks up in this very row. Checked
  directly rather than reasoned: `python3 scripts/compile_ratchet.py` **exits 0**,
  reports *"56 crates measured"*, and `ambition_character_sprites` is present in
  `dev/compile_ratchet_baseline.json`.

  ⛔ **the fifth stale ▢ in this ledger, and the same tell every time**: a row
  whose CHILD closed while the parent's summary kept its marker. The header warns
  about it; the checklist inside a long row is where it keeps happening, because
  closing the child means editing a different paragraph than the one carrying the
  box.

  *(kept for the rule it records: `compile_collect.py` takes one cargo build at a
  time in its own target dir and never backgrounds one — run it when nothing else
  is building, or a warm no-op reports 222 s.)*

  ▢ **and D33 itself is not finished**: `character_runtime` (12,543 lines) and
  `character_sprites/assets.rs` are untouched, and Wave B's lifecycle split is
  open. This is end state (1) for **one third** of the region, bought for +4.5 s.

  ---

  **The brief that preceded it:**

- **D33-BRIEF (superseded by the ✔✔ block above — marker stripped, text kept)
  THE DECOMPOSITION BRIEF (GPT 5.6, handed over by Jon 2026-08-08).**
  Jon's framing, which sets its rank: *"We don't have to take it if it disagrees
  with your assessment, and it is lower priority than fixing the issues
  identified in the review. We don't want to leave issues alive for too long or
  they will be easy to lose."* ⇒ **D29 / D25 / D30 first; this after.** He also
  said *"the subagent should be the thing taking care of most of the review tasks
  at your direction"*, which is now the standing dispatch rule.
  ⭐⭐ **and this is the payoff Jon named for recording measurements at all**:
  *"this is the payoff for recording them, so external agents can review and be a
  second pair of eyes."* The telemetry existed for one day and a second reviewer
  has already used it.

  ⛔ **I BLOCKED THIS ON D35 AND THE BLOCK WAS WRONG. Retracted the same hour,
  2026-08-08 — unblocked.** Kept rather than deleted because the error is the
  useful part.

  The block said: `bevy_material_ui` is 160.05 s of the clean dev build at
  `03878f81b`, so *"ranking first-party carves against a total that includes a
  160 s accidental dependency ranks them against noise — take it out first, then
  rank."* GPT 5.6 had reached the same conclusion independently (*"otherwise
  Material is distorting the cost profile we are trying to optimize"*), which is
  presumably why neither of us checked it.

  ⛔ **it distorts total WORK and not wall time, and carves are decided on wall
  time.** Same build, from `dev/ambition_dev_measurements/compile_units.jsonl`:

  ```
  sum of all unit work  4153.3 s      wall clock  539.9 s
    → average parallelism 7.69 of 8 cores (jobs=8, ncpu=8) — 96% saturated

  critical path:   monolith (385.4 → 524.4) → ambition_app (524.4 → 539.9)
  bevy_material_ui:         358.6 → 505.4   — ends 19 s BEFORE the monolith
  units in flight:  8/8 at t=500s · 6 at t=515s · 1 at t=530s
  ```

  Material finishes and the build keeps going for another 34 s on other units.
  Removing 160.1 s of work from a 96%-saturated build is **3.9% of the work** and
  roughly `160/7.69 ≈ 20 s` of the 540 s wall — unresolvable against a 54%
  run-to-run spread between the two clean dev builds on record (539.9 s, 833.7 s).

  ⭐⭐ **and it inverts the conclusion**: the wall-clock critical path runs
  **straight through `monolith → ambition_app`**, which is precisely what D33 and
  D34 are about. Material was never in the way. ⇒ D33 is **unblocked**, and this
  is mild evidence FOR its priority rather than against it.

  ⚠ **the lesson, which is the third instance today.** A per-unit `--timings`
  number and a build number are different quantities, and on a saturated parallel
  build the first does not convert into the second. `measure_the_suspect_not_the_aggregate`
  says name both denominators before dividing; the denominator here was never
  written down at all. **⛔ before quoting any unit's seconds as a build win,
  check whether it is on the critical path** — `start_seconds + seconds` against
  the build's own tail is a ten-second query and it is now the standing check.

  **Supervisor assessment — where it agrees with today's evidence, and the one
  sentence I would soften.**

  ✔ **Its compile framing is BETTER than the naive one and is not in tension with
  today's measurements.** *"Optimize for independent rebuild units, not number of
  crates"* and *"do not optimize only clean-build duration"* are exactly the
  argument today's numbers support. It also independently reaches D8's
  conclusion: a carve that lands BELOW the monolith (`new → monolith → runtime`)
  still rebuilds the monolith on every edit — which is precisely why the
  `conversation` carve priced at **0.00%** for editing conversation itself.
  ✔ It says outright *"do not treat `conversation` as the primary compile win
  merely because it is easy"* — agreeing with the measured ~1%.
  ✔ Its numbers are a **dev** build (monolith 138.99 s total / 20.97 s frontend /
  118.02 s codegen; runtime 70.24 s / 23.32 s / 46.92 s). Mine were **release**
  (188.5 s / 267.5 s). ⛔ **not a contradiction — different configs**, and the
  same trap I fell into this morning. Both stand.
  ⛔⛔ **SUPERSEDED BY D34 on the frontend half.** The `frontend` column in those
  numbers is cargo's TIME-TO-RMETA, which on a registration-heavy unit is mostly
  monomorphization collection. The runtime's actual rustc frontend is **1.8 s**,
  not 23.32 — measured cold, uncontended, incremental off. So the brief's
  *"surprisingly expensive frontend phase"* names a real anomaly and misattributes
  it; every argument below that leans on the word "frontend" should be re-read as
  "time before this unit unblocks its dependents". D34's row has the numbers.

  ### ⛔⛔ ADDED AFTER D27 LANDED — the brief's TARGET may be the wrong crate

  D27's agent split the release measurements by **cache state**, which my own
  correction had not done (I split by profile and stopped). Verified
  independently by the supervisor from `compile_units.jsonl`, grouping on
  `build_source`:

  | release build | monolith | runtime |
  |---|---|---|
  | 541 dirty (cold-ish, 148 fresh) | 309.0 s / 2.77 ms/line | **317.1 s** / 21.50 |
  | 57 dirty (first-party rebuild) | 68.1 s / **0.61** | **217.8 s** / **14.77** |

  * ⭐⭐ **in BOTH builds the most expensive first-party unit is
    `ambition_platformer2d_runtime`, not the monolith.** On the rebuild — the loop
    an agent or Jon actually pays — it is **3.2x the monolith** (217.8 s vs
    68.1 s) on **one seventh the lines**. Density gap **24x**, not the 10.7x I
    reported from the pooled figures.
  * ⭐ **the brief half-saw this and targeted the monolith anyway.** Its own text
    says *"`ambition_platformer2d_runtime` has a surprisingly expensive frontend
    phase despite being far smaller. Compile architecture needs to be reasoned
    about as a dependency graph, not merely by LOC."* It then makes the monolith
    the campaign. ⭐ **its instinct and its target disagree, and the measurement
    backs the instinct.**
  * ⛔ **and D27 priced the easy carve: lifting `conversation` out of the monolith
    makes the build 4.2 s SLOWER and the critical path one crate longer**, at the
    optimistic median rate. That inverts C4e's already-thin "~1%" compile
    argument to a negative.
  * ⚠⚠ **held loosely, and here is exactly how loosely**: this is **one release
    rebuild** and one cold-ish build. D27's own report says the weight table
    rests on a single release rebuild and that *"a second release rebuild would
    make the median mean something."* ⛔ **that is `compile_collect.py --config
    release` a few more times — cheap, and it should happen BEFORE anyone
    redirects a campaign on these numbers.** I have now been wrong about this
    exact table three times (pooled configs, then pooled cache states, then the
    superlative), each time confidently.
  * ⭐ **the actionable, low-regret reading**: the brief's §"Also investigate the
    runtime frontend hotspot" is not a side quest — **it may be the main event**,
    and it is cheap. Do that measurement before committing to the character-seam
    campaign, not after.

  ### ✔ REPLICATED — a second uncontended release build, and it sharpens the claim

  Ran `compile_collect.py --config release --phase first-party` on an idle
  machine (`build_foreign_cargo_peak: 1`, i.e. only its own cargo). Three release
  builds now exist:

  | build | monolith | runtime | ratio |
  |---|---|---|---|
  | cold, 541 dirty | 309.0 s (2.77 ms/line) | 317.1 s (21.50) | 1.03x |
  | **rebuild, 57 dirty** | 68.1 s (0.61) | **217.8 s (14.77)** | **3.20x** |
  | cold, 689 dirty *(new)* | 192.7 s (1.71) | 229.5 s (15.53) | 1.19x |

  * ⭐⭐ **`ambition_platformer2d_runtime` is the most expensive first-party unit
    in ALL THREE — 3 of 3.** That is the claim that needed replication and it
    replicated.
  * ⭐ **the density gap is large in every build: 7.8x, 24x, 9.1x.** The runtime
    costs an order of magnitude more per line than the monolith regardless of
    cache state.
  * ⛔ **but my "3.2x" was REBUILD-SPECIFIC and I did not say so.** On a cold
    build the two are comparable in absolute terms (1.03x, 1.19x); the 3.2x
    appears only on a first-party rebuild, because the monolith recompiles far
    cheaper when its dependencies are warm. ⭐ **that is still the case that
    matters** — a rebuild is what an agent or Jon pays while iterating — but the
    honest sentence is *"the runtime dominates the REBUILD; on a cold build they
    are comparable"*, not *"the runtime costs 3.2x the monolith"*.
  * ⭐ **so the D33 target question resolves like this**: the brief's instinct was
    right that the runtime deserves attention, and the monolith is not
    disqualified — on a cold build it is 193–309 s of real cost. **Both are worth
    work; only the runtime is being ignored.** ⚠ and neither result says a CARVE
    helps, which remains unmeasured.

  ⚠ **the one sentence I would soften**: *"The monolith is now demonstrably
  expensive enough to justify decomposition."* It is 17.8% of first-party
  unit-seconds — expensive in ABSOLUTE terms, true — but it is the cheapest
  substantial crate per line, and today's three-crate profile suggests a large
  part of backend cost is **per-crate**, so carving adds tolls rather than
  removing them. ⭐ **the honest restatement**: *the monolith is expensive because
  it is large, not because it is inefficient; splitting it does not obviously
  reduce total compile time, and the case is ISOLATION* — which is the case the
  brief actually argues everywhere else.
  ⚠⚠ **and I hold that caution loosely**: the per-crate-toll reading is n=3 with
  two warm-cache recompiles, graded medium-low in
  `dev/journals/compile-cost-what-actually-drives-it-2026-08-08.md` §0. **Nothing
  I measured measures a carve.** It is a reason to not sell a carve on build
  time, not a reason to refuse one.

  ✔ **its §"Also investigate the runtime frontend hotspot" is DONE — see D34.**
  Today's first attempt,
  `cargo +nightly rustc -p ambition_platformer2d_runtime --release -- -Ztime-passes`,
  gave 65.99 s total, `LLVM_passes` 37.45 s (56.7%), `LLVM_thinlto` 24.47 s
  (37.1%); it was a RELEASE build AND a warm-cache recompile, so its frontend
  figures were not quotable (`type_check_crate` read 0.019 s — reuse, not speed).
  ⭐ **the cold, `CARGO_INCREMENTAL=0`, dev-profile run has since been made** and
  its backend half replicates (cold release: 73.68 / 35.58 / 21.60). The answer:
  **the dev frontend is 1.8 s, not 23.32** — cargo's `frontend` column is
  time-to-rmeta and subsumes monomorphization collection.
  ⛔⛔ **and the falsification recorded here was WRONG — it is the answer.**
  Removing ~2,130 lines of rollback-domain registration changed `cargo check` time
  essentially not at all, which is true and reproduces exactly
  (`type_check_crate` 1.118 → 1.165 s). Measured as a **build**, the same
  subtraction is **28.13 s → 8.66 s, −69%**, with instantiations dropping 71.6%.
  `cargo check` cannot monomorphize, so it could not see 94% of the cost.
  ⛔ *"restructure rollback because it looks generic-heavy"* is therefore **not
  refuted** — but D34 prices REMOVAL, not RELOCATION, and relocation is the thing
  anyone would actually do. Read D34's last two bullets before acting on this.

  ✔ `.agent/index/crates/graph-resolved.json` **exists** (57 KB, 2026-08-07), as
  do per-crate files under `.agent/index/crates/`. Its instruction to use the
  recorded graph rather than infer one is right and cheap.

  ⛔ **the substantive target it names is `character_runtime` + `character_sprites`
  (~16.5k lines, ~15% of the monolith), and it explicitly forbids moving them
  wholesale** — 14 and 24 incoming monolith consumers respectively. The task is
  to find the durable seam, and to **stop and say so** if the seam is not clean
  rather than force a crate. ⭐ that instruction is the most important line in the
  brief and any agent taking this must be held to it.
  ⚠ **it ties into D31 (Blink)** — and see D31: **Jon has already ruled that the
  Blink fallback is documented design plus a product question he owns.** So the
  brief's *"design the seam so it naturally supports body-owned intrinsic
  capabilities"* is in scope; *fixing* the fallback is not, until Jon says.

  ### ⭐⭐ THE MANDATE, AS AMENDED — GPT 5.6 via Jon, 2026-08-08

  A second review re-read the whole campaign against `51c67eddac3a` (verifying
  the Material result independently) and **kept D33 but changed what it is
  for.** This paragraph outranks the brief's own framing wherever they differ.

  > **Decompose the monolith for independent rebuild units and architectural
  > ownership. Do not assume decomposition reduces total clean-build work.**

  ⛔ **so the task is NOT "our first substantial compile-effective carve."** It is:

  > Find the first substantial **compile-isolating ownership boundary** in the
  > character preparation/presentation area. Only create the crate if its
  > dependency direction actually produces independent edit classes and does not
  > simply insert another serial crate before the monolith.

  ⭐ **the justification that survives**, and it is a good one: 112k lines of
  unrelated actor responsibilities share **one recompilation and ownership
  unit**, incremental is deliberately off, and the measured dev build's critical
  tail really does run `actor_monolith → ambition_app → done`. Shortening or
  isolating that unit *can* move wall time. What is no longer claimed is that it
  *will* — D27 priced the easy carve at **4.2 s slower**, and that is the warning
  shot: moving 15% of the lines out is not evidence of a speedup, and a badly
  placed crate makes things worse.

  **Two end states are both a success, and the agent must be told so:**

  1. a real new crate whose edits compile independently of large unrelated actor
     code; **or**
  2. a substantially narrowed seam where the next crate move is mechanical.

  ⛔ **do not force (1) merely to be able to say the monolith has been
  decomposed.** ⭐ **and do not begin by creating a crate — begin by finding the
  dependency direction.** `character_runtime` and `character_sprites` are the
  strongest region to investigate; their current DIRECTORY boundaries are **not
  presumed to be crate boundaries**.

  ⚠ **judge a proposed boundary with `.agent/index/crates/graph-resolved.json`
  and the SECONDS-WEIGHTED compile ratchet — never line-count intuition.** Lines
  correlate with cost at **−0.23** here; that is the whole reason this row was
  nearly aimed at the wrong crate twice.

  ⭐ **D34 does NOT redirect this campaign, and the review is right about why.**
  D34 measured **deletion** of the rollback-domain registrations (28.13 s →
  8.66 s, instantiations −72%), not **relocation**. Move those registrations into
  eleven crates and the compiler still instantiates them somewhere — possibly at
  higher total per-crate overhead, possibly on a longer critical path. ⇒ the
  one-domain relocation experiment is **a cheap and excellent falsifier and it is
  a separate errand**. It does not block D33 and D33 does not wait on it.

  ⚠ **and the Material framing is settled the same way**: ~160 s of rustc CPU
  work removed, **not on the critical path**, ~96% saturated machine, Material
  ending 19 s before the monolith. Still a good deletion — less work, fewer
  packages, 26 fewer systems, simpler architecture — but it was never going to be
  a 160 s wall-clock win, and nobody should re-derive that claim.

  ### ✔ INVESTIGATION PHASE DONE, 2026-08-08 — the answer is END STATE (2)

  Read-only agent pass, then verified by the supervisor on a second route. **The
  proposal is: do NOT create a crate yet. Narrow the seam.**

  ⛔⛔ **the directory carve is PRICED and it is negative.**
  `compile_ratchet.py --carve …/src/character_sprites`:

  ```
  resulting placement   BELOW ambition_platformer2d_actor_monolith
  first_party_seconds   1,308.5s -> 1,316.3s   +7.8s  (+0.59%)
  critical_path_crates       12 ->      13     +1     (+8.33%)
  edit_cost(the module)  249,619 -> 249,619    +0.00%
  ```

  ⭐ **the `+0.00%` is the whole lesson**: the owner would depend on the new
  crate, so an edit to the carved code still rebuilds the monolith and everything
  above it. Isolation runs one direction only. Identical shape to `conversation`.

  ⭐⭐ **THE FINDING: the entanglement is largely a FACADE ILLUSION.** The brief's
  *"24 incoming consumers"* for `character_sprites` is really **6**. Causes, each
  verified in source:
  * `src/lib.rs:99` is `pub use ambition_combat as combat;` — every
    `crate::combat::*` in the region already points OUTSIDE the monolith;
  * `src/actor.rs` is a 178-line re-export shim; `src/features/mod.rs:59`
    re-exports `crate::combat::components`; `src/schedule` re-exports
    `shared_tangle::schedule`, so schedule placement is not a monolith coupling;
  * `character_sprites/mod.rs` is **itself a facade** — 22 of ~40 exported names
    are `pub use ambition_sprite_sheet::…` pass-throughs, and `AuthoredSheets`
    alone accounts for 28 consuming files while owning **zero** monolith lines.

  **Net: of 114 `crate::` references in the region's production code, 67 (59%)
  resolve to crates that already live below the monolith.** Real coupling is 47
  references over 16 distinct paths. ⚠ **and two of the "consumers" of
  `pick_actor_anim` are intra-doc links in comments — there is no call site.**
  ⇒ [[reference_orphan_rule_adjudicates_crate_placement]]'s *"re-export paths LIE
  about ownership"* is the single most load-bearing fact in this row.

  **The seam does not follow the directory boundary — it cuts `character_sprites`
  in half.** `{attack_hitbox, anim, posed_body}` (~1,054 prod lines) answer
  UPWARD only: `attack_hitbox`'s one external consumer is
  `ambition_platformer2d_runtime::combat_schedule`, `anim`'s are
  `ambition_sim_view`. `{assets.rs}` plus all of `character_runtime` stay — they
  are bidirectionally coupled to `features`/`avatar`/`assets`/`persistence`/
  `control` through 13 monolith production consumers.

  ⛔ **and the instrument REFUSES to price the subset, correctly:**

  > *"…is a NESTED module, and coupling here is detected one path segment at a
  > time. Simulating it would report the parent's edges as this module's and
  > **very likely answer SIBLING when it is not**."*

  ⭐ **so the narrowing work is the PREREQUISITE FOR THE MEASUREMENT, not just
  for the carve.** Nobody can price this until the modules are hoisted.

  **The three narrowing steps, in order, each mechanical:**
  1. **de-facade `character_sprites`** — delete the 22 `ambition_sprite_sheet`
     pass-throughs, repoint ~28 call sites at the real crate. Takes the module's
     apparent consumer count 18 files → 6 and its API ~40 → ~18 symbols;
  2. **move two pure-data components down** — `PlayerBlinkCameraState`
     (`avatar/components/mod.rs:31`, four `f32`s and two `Vec2`s, no
     monolith-typed fields) and `ActorAnimOverride`
     (`features/ecs/actor_clusters.rs:39`, a one-field newtype over
     `ambition_sprite_sheet::character::CharacterAnim`);
  3. **invert `posed_body_for`** (`character_runtime/presentation.rs:365-378`) so
     the monolith stops constructing `SpritePosedBody`.

  After those, the subset is a sibling with no edges either way and the move is a
  `git mv` plus a `Cargo.toml`.

  ⚠ **HONEST SIZE, and it must not be oversold**: ~1,054 production lines, **~1.7%
  of rlib mass. This will not move the wall clock.** The case is ownership, which
  is exactly what the amended mandate asks for.

  ✔ **two fixes landed immediately out of this** (`3acebd2bf`, `5f7563f8f`): the
  `#[allow(unused_imports)]` at `character_sprites/mod.rs` justified itself with
  *"player_attack_hitbox_world is the live consumer"* and **that symbol has no
  consumer anywhere** — three of four re-exports deleted, making the module's
  surface look 4x less entangled than the waiver implied; and
  `architecture.md`'s Tier 3 read as a standing "it remains one crate" with
  nothing pointing at `decomposition.md`'s active ruling.

  ⭐ **corrections the agent made to MY brief, all of which I accept**:
  `character_sprites` incoming **24 → 6**; region mass **16,536 → 9,140**
  rlib-relevant lines (the rest is `#[cfg(test)]`; the monolith is 44% test, the
  runtime only 13%); and on that corrected denominator the runtime's per-line
  multiple is **7.2x, not 8x** — the finding survives, the multiplier moves.

  ✔ **STEP 1 LANDED `a73b21517`** — the de-facade. 22 pass-throughs deleted,
  **two of which were laundered one hop deeper** (`CharacterAnim` via
  `anim/mod.rs`, `CharacterSpriteAssets` via `assets.rs`) and would have survived
  a read of `mod.rs` alone. Consumers **52 → 29 files**; consuming crates 10 → 9.
  One Cargo edge gained (`ambition_platformer2d_runtime` → `ambition_sprite_sheet`,
  needing a policy-allowlist entry); three more crates would have gained forbidden
  edges and were repointed at the existing `ambition_platformer2d::character`
  facade instead, so the game tier gained **zero**.
  ⚠ my "18 → 6" estimate was wrong; the agent measured 52 → 29 and could not
  reproduce mine under any definition. Its number stands.

  ✔ **STEP 2 LANDED `24b43f93a`.** `PlayerBlinkCameraState` →
  `ambition_platformer2d_shared_tangle::camera_ease` (that module's own header
  already carried the precedent: moved below the actor crate in F1.5 *"so
  render/host can share camera presentation timing state without depending on the
  actor-domain crate"*); `ActorAnimOverride` →
  `ambition_sprite_sheet::character::anim`, beside the `CharacterAnim` it wraps.
  ⛔ **NOT `ambition_combat` beside its siblings `ActorRenderSize` /
  `ActorSpriteOffset`** — that needs a new `ambition_combat → ambition_sprite_sheet`
  edge, giving a content-free combat MODEL a dependency on sprite vocabulary, for
  a newtype over a type `ambition_sprite_sheet` DEFINES. Neither is re-exported
  from its old path.
  ⭐ **and the row's cheap-because-the-wire-name-is-a-literal argument was HALF
  right.** The stable names are literals and did not move
  (`rollback-wire-format-is-frozen` green at 347). But `schema_dump` records
  `type_name`, so the FINGERPRINT moved:
  `test_rollback_baseline_paths_are_live` went red on both lines in under a
  second, statically, no build. `rollback_schema_baseline.txt` rewritten; v19 NOT
  bumped (the dump itself changed, so the fingerprint already differs — the
  version constant is for the opposite class). **This is the live case D37
  answers.**

  ✔ **STEP 3 LANDED `a5d722c60`, and the row asked for the shape that does not
  work.** "Invert `posed_body_for` so the monolith stops constructing
  `SpritePosedBody`" means the construction moves UP into the runtime. It cannot:
  the insert is one arm of `project_prepared_character_definitions` and its
  removal is one arm of `GrantedBodyFacts::retract`, a struct designed so adding a
  fact is a compile error until it is retracted too. Moving the grant leaves the
  retraction in another crate, another system, another change-detection query —
  [[reference_an_authority_that_needs_a_follow_up_call]], bought for 1.7% of rlib
  mass. ⭐ **so the DATA moved down instead of the CONSTRUCTOR moving up**:
  `SpritePosedBody` is `{ target, world_per_pixel }`, two facts about a sheet, and
  now lives in `ambition_sprite_sheet::character::sheets` next to the
  `record_for_target` its target indexes. Same edge removed, grant and retract
  still in one function.

  ⇒ **answering the row's own open question**: the runtime crate *can* own the
  construction and *should not*. The seam is `SpritePosedBody`'s HOME, not its
  constructor's.

  **▢ WHAT IS LEFT BEFORE THE `git mv`** — measured after step 3, on the whole
  `{attack_hitbox, anim, posed_body}` subset:
  * **down (subset → monolith): ONE, and it is `attack_hitbox`.**
    `attack_hitbox.rs:131,247` call `super::assets::sheet_for_character_id_in`
    and `super::assets::sprite_body_collision_for_character_id_in`, and `assets`
    STAYS. ⛔ **on the LIVE path** — `authored_attack_volume_resolver`, the
    module's one external consumer, reaches both through
    `player_attack_hitbox_world` / `actor_attack_hitbox_world`. ⚠ so step 1's
    note that those two symbols *"have NO consumer anywhere"* is true of the
    deleted RE-EXPORTS and false of the functions. Both `assets` entry points
    are thin wrappers over `*_from_data(authored, catalog.data(), …)`; the only
    monolith-owned type in either signature is `SpriteBodyCollision`, of which
    `attack_hitbox` reads one field (`render_size`).
    ⭐ `anim` and `posed_body` are **down-edge-free**: their other 14 distinct
    `crate::`/`super::` paths all resolve through re-exports to crates already
    below — `crate::actor::Body*` is `ambition_platformer2d_core`,
    `crate::combat::*` / `crate::MeleeSwing` /
    `crate::features::{ActorRenderSize, ActorSpriteOffset}` are `ambition_combat`.
  * **up (monolith → subset): ONE.** `features/mod.rs:641` registers
    `sync_sprite_posed_bodies` into `WorldPrepSet::BeforeIntegrate`, a
    `shared_tangle` set — so a sibling-crate plugin installed by
    `ambition_platformer2d_runtime` reproduces it exactly, the shape
    `authored_attack_volume_resolver` already uses. Everything else naming the
    subset from inside the monolith is a doc link
    (`features/ecs/anim_helpers.rs:11`, `actor_bundles.rs:141`) or
    `character_sprites/mod.rs`'s own re-export block, which goes away with the
    move.

  ⚠ still genuinely undetermined: whether the hoisted subset clears the ratchet's
  `critical_path_crates` guard (only the simulator can say, and only after the
  hoist — it **refuses** to price a nested module, correctly).

  ### ⭐ STEP 4 — THE LAST DOWN-EDGE DISSOLVES, AND HERE IS EXACTLY HOW

  Traced 2026-08-08 so the next pass is mechanical instead of another
  investigation. The edge is `attack_hitbox` → `super::assets`, via two `pub`
  wrappers. **Both wrappers are one line over a private `*_from_data` form, and
  those forms are made almost entirely of types that already live below the
  monolith:**

  ```rust
  fn sheet_for_character_id_from_data(          // assets.rs:63
      authored: &sheets::AuthoredSheets,        //   ambition_sprite_sheet
      catalog:  &CharacterCatalogData,          //   ambition_characters
      character_id: &str,
  ) -> Option<CharacterSheetSpec>               //   ambition_sprite_sheet
  ```

  ⭐⭐ **zero monolith types.** It could move to `ambition_sprite_sheet` today.

  `sprite_body_collision_for_character_id_from_data` (`assets.rs:334`) is the
  same except for its return type — **`SpriteBodyCollision` is the ONLY
  monolith-owned type in the whole edge**, it is defined at `assets.rs:298`, and
  it is two `Vec2`s:

  ```rust
  pub struct SpriteBodyCollision { pub collision: ae::Vec2, pub render_size: ae::Vec2 }
  ```

  ⇒ **move those two `_from_data` fns and `SpriteBodyCollision` into
  `ambition_sprite_sheet`, and the subset's last down-edge is gone.** The `_in`
  wrappers stay in the monolith and delegate, so nothing above changes. ⚠ note
  `attack_hitbox` reads exactly one field (`render_size`), so if the struct turns
  out to be awkward to move, returning the `Vec2` alone also closes the edge.

  After that the only remaining coupling is the ONE up-edge (the
  `WorldPrepSet::BeforeIntegrate` registration), which has a working precedent in
  `authored_attack_volume_resolver` — and end state (2), *"a substantially
  narrowed seam where the next crate move is mechanical"*, is genuinely reached.

  ### ✔✔ STEP 4 LANDED `d4b7423db` (merged 2026-08-09) — THE DOWN-EDGE IS ZERO

  `attack_hitbox` has **0** production references into the monolith (grepped, not
  asserted). Both `*_from_data` functions and `SpriteBodyCollision` now live in
  `ambition_sprite_sheet::character::catalog_join`, and the type left
  `character_sprites/mod.rs`'s re-export list rather than being laundered through
  it. The new `ambition_characters` edge was **already transitive** via
  `ambition_interaction`, so the capability footprint stays at 40 and no cycle
  appears.

  ⇒ **ratchet cost of the move: +0.6 s** (`worst_edit_cost` 1,259.6 → 1,260.2),
  `critical_path_crates` **unchanged at 12**, `largest_unit_lines` **−209**.

  ### ⭐⭐ THE HOIST IS NOW A DECISION, NOT A MEASUREMENT

  The chain of 12 is
  `geometry → core → characters → interaction → combat → monolith → sim_view →
  runtime → host → platformer2d → content → app`.

  * **registration stays in the monolith** ⇒ the new crate C sits *between*
    `combat` and the monolith on the critical edge: `height(C)=8`, which pushes
    `sprite_sheet` and `combat` to 9, `interaction` 10, `characters` 11, `core`
    12, `geometry` 13. ⛔ **12 → 13, a PATH finding, `--update` required.**
  * **registration moves into C as its own plugin** (the
    `authored_attack_volume_resolver` shape) ⇒ `height(C)=7`, dependents are only
    `sim_view` and `runtime`, **nothing on the path moves and it stays 12.**
    Cost: repointing `ambition_platformer2d::actors::character_sprites::…` for
    `sim_view`, `runtime` and 4 app/demo test files — ⭐ **`actors` IS the
    monolith, so that facade alias is itself what forces the dependency.**
  * ⚠ expect an **`UNPRICED`** finding either way: a new crate has no measured
    weight, so 2,123 subset lines reprice from 0.61 to the population median
    2.557 ms/line ⇒ **≈ +4.1 s**, inside budget but ~16% of the remaining
    headroom, and a placeholder until `compile_collect.py` runs.

  ⇒ **do the plugin version.** It is the difference between a free carve and one
  that lengthens the critical path, and the precedent already exists in the tree.

  ### ✔✔ THE SIMULATOR ANSWERED IT WITHOUT THE HOIST — 2026-08-09

  `compile_ratchet.py --carve …/character_sprites --new-crate ambition_character_sprites`:

  ```text
  resulting placement    BELOW ambition_platformer2d_actor_monolith (owner depends on it)
  inward edges           8 files in the owner name it
  outward edges          [actor, assets, character_roster, combat, persistence]

  critical_path_crates          12 ->     13     +1
  worst_edit_cost_seconds   1,261.4 -> 1,268.8  +7.4s
  edit_cost(the module)       973.6 ->   981.1  +7.4s   ⛔ DOES NOT FALL
  ```

  ⛔ **"this carve makes the BUILD SLOWER by 7.4 s of first-party rustc time"** —
  3,816 lines leave the monolith at **0.6102 ms/line** and arrive at the
  population median **2.557**. ⭐ **that is the whole objection in one ratio, and
  it is the compile-cost lesson stated as a number**: cost is per-crate, so
  carving multiplies it.

  ⭐⭐ **and the sharpest line is not about seconds**: *"`edit_cost(the module)`
  DOES NOT FALL, and that is the finding. The owner depends on the new crate, so
  an edit to the module still rebuilds the owner and everything above it — the
  isolation runs one direction only."*

  ### ⇒ THE ANSWER IS CONDITIONAL, AND THE CONDITION IS THE PLUGIN

  Both objections are **placement** facts, not size facts — they hold however few
  lines move. So:

  * **carve with the registration left in the monolith** ⇒ the owner still
    depends on the subset ⇒ **+1 critical path, ~+4 s, and the module's own edit
    cost does not improve.** ⛔ **measurably worse than today. Do not do this.**
  * **carve with the registration moved into the crate as its own plugin** ⇒ the
    monolith stops naming the subset ⇒ the simulator's placement verdict no
    longer applies, the path stays 12, **and the "isolation runs one direction
    only" objection dissolves** — which is precisely the prize.

  ⚠ **the simulator CANNOT confirm the second case**: it infers placement from
  edges that exist today, and the plugin move is the thing that changes them.
  ⚠ it also **refuses nested modules** — `--carve …/character_sprites/anim`
  answers *"coupling here is detected one path segment at a time … very likely
  answer SIBLING when it is not"*, which is the correct refusal the row predicted.
  Pricing the subset alone needs it hoisted to the crate root first.

  ⭐ **two independent methods agree on the cost**: the step-4 worker predicted
  **≈ +4.1 s** from the dependency graph; scaling the simulator's +7.4 s by the
  subset's share of the lines (2,127 of 3,816) gives **+4.1 s**. ⇒ the number is
  not an artifact of either method.

  ▢ **so the hoist is no longer needed to DECIDE — only to DO**, and it should be
  done as the plugin version or not at all. ⚠ **an agent stalled on this twice**
  (74 records, no process, 19 GB of cold artifacts) before producing anything;
  the setup cost of an isolated worktree is most of the job here.

  ### ⛔ TWO CORRECTIONS THE WORKER MADE TO MY BRIEF

  * **my "tidy the `_from_data` suffix away" instinct was wrong and the policy
    caught it.** `sheet_for_character_id(` and
    `sprite_body_collision_for_character_id(` are **forbidden identifiers** in
    `engine.character-authority-is-app-local` — they were the retired
    process-global lookups. Dropping the suffix turned the policy red in **7
    places**. ⭐ `catalog_join`'s module doc now records why, so the next person
    tidying it hits a sentence instead of a red.
  * **two sub-workspace lockfiles needed the new edge, not one** —
    `fixtures/minimal_game` **and `examples/capability_demo`**. ⚠ and
    `cargo generate-lockfile` rewrites the whole file (it downgraded
    `avian_derive` 0.2.3 → 0.2.2); `cargo tree --offline` gives the minimal
    one-line update. [[reference_a_new_dep_edge_fails_the_contracts_job]] said
    one file; it is two.

  ✔ **RE-VERIFIED 2026-08-09, AFTER D44 rewrote `attack_hitbox.rs`** — the trace
  above survives the rewrite, so it is still mechanical:
  * `attack_hitbox` still has **exactly the two** down-edges named, at `:131`
    (`sheet_for_character_id_in`) and `:247`
    (`sprite_body_collision_for_character_id_in`). D44 touched the file and added
    none.
  * ⭐ **`SpriteBodyCollision` has NO production consumer outside `assets.rs`.**
    Grepped workspace-wide: one re-export (`character_sprites/mod.rs:72`) and two
    `///` doc mentions in `game/ambition_app/tests/enemy_body_scale.rs`. Nothing
    destructures it elsewhere, so moving the type is a move, not a migration.
  * ⇒ **dispatch-ready.** Verified in a worktree (`cargo check -p ambition_app`,
    the crate tests and `check_absence_contracts.py` are the whole gate for a
    carve — no art needed), and it does not collide with D37/D39/D40/D43.
  ⚠ hold until a build slot frees: 4 concurrent jobs already put load average at
  9.4 on 8 cores, and each worktree needs its own `CARGO_TARGET_DIR`.

  ### ✔✔ THE HOIST LANDED 2026-08-09 — `ambition_character_sprites`, and the path stayed 12

  Done in the MAIN tree, plugin shape, `git mv` + a `Cargo.toml`. Measured
  before and after on the same tree, minutes apart:

  ```text
  critical_path_crates          12 ->       12     UNCHANGED   ⭐ the whole point
  worst_edit_cost_seconds  1,267.4 -> 1,271.9s     +4.5s
  edit_cost_seconds(monolith) 979.3 ->  978.1s     -1.2s
  largest_unit_lines       113,381 ->  111,237     -2,144
  first-party crates            55 ->       56
  UNPRICED  ambition_character_sprites, 1 crate at the population median 2.557 ms/line
  ```

  ⭐ **the plugin shape held exactly as predicted.** The actor crate names
  nothing in the new crate, so the simulator's BELOW placement never happened;
  `ambition_sim_view` and `ambition_platformer2d_runtime` are the only engine
  dependents, and no crate on the serial chain moved.

  ⭐ **the `UNPRICED` finding is accepted as a guess, not re-frozen.** The
  ratchet exits 1 and says so in its own words — "or say in the commit that the
  guess is accepted for now" — and the commit does. `--update` was deliberately
  NOT run: the baseline is tight and other agents' landings are in the tree.
  The real number wants a `compile_collect.py --config release` run.

  ### ⛔ FOUR CORRECTIONS TO THE DISPATCH BRIEF, all found by doing it

  1. ⛔⛔ **the subset is 2,144 lines, not 3,887, and the +7.6 s prediction
     priced a carve that CANNOT HAPPEN.** 3,887 is the whole
     `character_sprites/` DIRECTORY — which includes `assets.rs` (1,091) and the
     module's own `tests.rs` (595). `assets.rs` **stays**: it names
     `crate::assets::platformer_assets`, `crate::persistence::settings` and
     `crate::character_roster`, and **8 monolith production files consume its
     symbols**, so moving it would recreate the very owner→carve edge the plugin
     shape exists to avoid. This row said so at investigation time ("the seam
     cuts `character_sprites` in half") and the dispatch number quietly reverted
     to the directory. ⭐ **the ledger's 2,123 lines / ≈+4.1 s was RIGHT and was
     called stale wrongly** — measured +4.5 s.
  2. ⛔ **"repoint at the new crate directly, not via a fresh alias" is
     FORBIDDEN for game crates, by policy, and the policy is right.**
     `game.demo-mary-o-umbrella-only` allows Mary-O exactly ONE dependency
     (`ambition_platformer2d`) and `game.app-reaches-lower-through-facade`
     forbids `ambition_app/src` from naming engine crates at all. So the facade
     gained `pub use ambition_character_sprites as character_sprites;` and every
     game-side repoint is the deletion of one path segment.
     ⭐ **that is NOT the alias the carve removes.** The removed one is
     `actors::character_sprites`, and `actors` IS the actor crate — which is
     exactly why it forced the dependency. A re-export at
     `ambition_platformer2d`, which already sits above everything, forces
     nothing. Engine crates (`sim_view`, `runtime`) name the new crate directly,
     as they must.
  3. ⛔ **the repointing is 10 files across 6 crates, not "sim_view, runtime and
     ~4 app/demo test files".** The one the brief missed is PRODUCTION code:
     `game/ambition_content/src/player_robot_lineage.rs`, four call sites of
     `authored_body_pixel_size`. The rest: `sim_view` ×2, `runtime` ×1,
     `ambition_app/tests` ×2, `ambition_demo_mary_o/src` ×3,
     `ambition_demo_mary_o_app/tests` ×1.
  4. ⚠ **Mary-O had to be touched** — 3 `src/` files plus one app test, all pure
     path renames. Checked first: the other live agent's uncommitted files there
     were `mary_o/src/provider.rs`, `mary_o_app/src/lib.rs` and
     `mary_o_app/src/bin/capture_mary_o.rs`, none of which this carve names.

  ### ⭐ THE HAZARD THAT DID NOT FIRE, AND WHY IT IS WORTH RECORDING

  Moving a registration into a plugin that only `PlatformerEnginePlugins` adds
  is the [[reference_app_only_presentation_class]] shape in reverse: any
  composition that added `WorldPrepSchedulePlugin` *by itself* would silently
  lose the posed-body pass. **Two do** —
  `game/ambition_demo_mary_o/src/lib.rs:2589` and
  `game/ambition_demo_mary_o/src/movement/tests.rs:141`. Both are inside
  `#[cfg(test)]`, and neither test asserts anything about a posed body, so
  nothing half-runs. ⛔ **grep for the owner plugin's other adders before moving
  a registration out of it** — had either been production, the carve would have
  shipped Mary-O an engine that half-runs and every test would still have been
  green.

  ### ⚠ TWO REGISTRIES THE ROW DID NOT NAME, AND ONE OVER-SPECIFIED TEST

  * **the capability-footprint ratchet stops a new crate name**, correctly:
    40 → 41 in `capability-footprint-baseline.json`, moved in the same commit
    with a `_comment` line, per the `ambition_geometry` / `ambition_projectile_spec`
    precedent. ⭐ nothing new is LINKED — the same code, under its own name.
  * **every crate needs a `MODULES.md`** or `scripts/tests/test_modules_md_is_current.py`
    goes red. `python3 scripts/modules_md.py --write`.
  * ⛔ **`test_an_unmeasured_crate_is_priced_at_the_median_and_reported_as_a_guess`
    went red on a correct change.** It asserted
    `unpriced_crates == ["ambition_invented"]`, which also asserts *"and the
    working tree has no unpriced crate"* — a different claim, and one that a
    legitimate carve landing before its `compile_collect.py` run makes false.
    Loosened to membership; the tree-level guard is the ratchet's own UNPRICED
    finding, which fired.
  * `ambition_entity_catalog` is a **dev-dependency** of the new crate (the
    picker's tests name `DamageKind`); it stays out of `cargo tree --edges
    normal`, so the footprint does not see it.
  * **five lockfiles, three of them tracked and changed**: root,
    `fixtures/minimal_game`, `examples/capability_demo`.
    `examples/portal_tutorial` references neither the facade nor the monolith
    and did not move; `fixtures/external_consumer` is **gitignored** — regenerate
    it locally (`cargo tree --offline`) or leave a red nobody's diff explains,
    and commit nothing.

  ⇒ **what is left of D33**: `character_runtime` (12,543 lines) and
  `character_sprites/assets.rs` are untouched, and Wave B's lifecycle split is
  still the open question. This carve is end state (1) for one third of the
  region and it cost +4.5 s of first-party rustc work to buy real edit
  isolation — ⚠ **do not sell the next one on build time either.**

- **D28 ✔ THE RECORDER LANDED AND THIS ROW DID NOT KNOW IT — 2026-08-08.**
  *(marker dropped 2026-08-11: the row's own premise is spent. The OPEN D28 is the
  compile/test-time measurement row further down, which keeps its `▢`.)*
  Caught by the charter's own rule (*"grep for the thing the row says is
  missing"*). The row says the guard *"stores no duration for any of them"*.
  It does now: **`.goal/check_cost.jsonl`, 25 records**, carrying `total_seconds`,
  per-check `seconds`, `head`, **and the contention stamp this row called the
  main design hazard** — `load_before`, `load_after`, `foreign_builds`.

  ### ⭐ THE RHYTHM ANSWER, MEASURED (Jon's ask (c), the one nothing could answer)

  25 real cycles, 13:42 → 23:13 on 2026-08-08:

  ```
  full check cycle        median 177.9s    min   5.9s    max 463.5s
    app integration suite median 159.8s                  max 396.7s   ← 90% of it
    absence contracts     median   2.0s
    cargo check (gate)    median   0.51s warm            max 166.5s cold
    uncommitted / ledger  median   0.1s
  ```

  * ⭐⭐ **the gate is FREE and should be run constantly**: `cargo check -p
    ambition_app` warm is **0.51 s** (n=15). Cold it is 11–166 s. There is no
    argument for batching edits to avoid it.
  * ⭐⭐ **`app_it` IS the loop** — 90% of every cycle. It is the only thing worth
    batching, working in the background around, or narrowing.
  * ⛔ **the observer problem is REAL BUT SMALLER THAN THIS ROW FEARED.** It
    predicted a 35% swing from contention. Measured: `app_it` is **158.4 s on a
    quiet box (n=17) vs 183.3 s with ≥1 foreign cargo build (n=8) — 1.16x**,
    with up to **9** concurrent builds and load to 16.4 seen. ⇒ **running agents
    in parallel costs ~16% on the suite, not 35%**, which is a cheaper price for
    parallelism than I have been quoting all day.

  **What is genuinely left of D28**: (a) making the loop faster — untouched, and
  the data says the only target is `app_it`; and D24's leftover, that all 2,145
  collector rows read `incremental = false`, closed by one collector run with
  `CARGO_INCREMENTAL=1`. Jon's ask (b), *demonstrating* improvement over time,
  is now possible for the first time because a comparable series exists.

  ⚠ **the original row text is kept below** because its reasoning is why the
  recorder has the fields it has — but read it as history, not as open work.


---

- ▢ **D45 SIXTEEN MORE OBSERVATIONS FROM JON (`7ace7b5e7`) — INDEX, not triage.**
  Recorded so they are tracked. ⚠ **Jon said explicitly that mentioning them does
  not change priorities**, so nothing here is ranked or worked.
  * ✔ **DIAGNOSED 2026-08-08 — it is NOT D39, and the engine already designs for
    it.** `game/ambition_app/src/app/world_flow/room_transition_presentation.rs:400`
    holds the transition COVER until every published feature is claimed by a
    render family, and gives up on a deadline:
    ```rust
    if since_commit < config.presentation_settle_deadline { return; }   // keep waiting
    warn!("... revealed with {unsettled} unclaimed body placeholder(s) still
           drawn after {:.0} ms — the cover gave up waiting.");
    ```
    ⇒ **a magenta flash on room change is the cover EXPIRING**, not art that never
    resolves. The comment above it is explicit that never hanging is deliberate:
    *"a feature that NO family will ever claim is a real bug this diagnostic
    exists to show, and holding a black screen over it forever would hide the bug
    behind a worse one."*
  * ⭐⭐ **so the three "faces" are TWO mechanisms, and I can now say which**:
    * **permanent** — D39's goblins. Art never resolves because the instance id
      occupies the character slot. A cover of any length would not help.
    * **transient** — this flash, and (I believe) the device log's `NpcSpawn-*`
      warnings, which fire immediately after `room-loaded central_hub_complex`
      rather than in steady state. Art resolves, just after the deadline.
    ✔✔ **CHECKED 2026-08-08, and it reframes the symptom.** Grepped the desktop
    profile capture (`target/profiles/desktop-timeline-run-…/game-stderr-stamped.txt`,
    a real 290 s session):

    ```
    no render family claimed  ->  190 occurrences
    cover gave up waiting     ->    0 occurrences
    ```

    ⭐⭐ **the cover NEVER expired.** So those 190 warnings are the ORDINARY
    pre-settle state — the placeholder is drawn, the warning fires, the cover
    hides it, and the reveal happens after everything settles. ⇒ **a
    `no render family claimed` line is not by itself evidence of a defect**, and
    both Jon and I had been reading the device log's copies of it as one.
    ⚠ **so the Android question narrows to one grep of HIS log for
    `cover gave up waiting`.** Present ⇒ the deadline is too short for a phone
    (a tuning question). Absent ⇒ the cover is not covering that transition at
    all, which is a different and more interesting bug. ⛔ **do not tune the
    deadline before knowing which.**
    ⚠ superseded, kept for the reasoning: whether the warning actually
    appears in Jon's device log, or whether those warnings are the ordinary
    pre-settle state that the cover normally hides. That distinguishes "the
    deadline is too short on a phone" from "something never settles", and it is
    one grep of a captured log. ⇒ **do that before tuning any deadline.**
  * ⚠ **and there is a known sibling already in the tree**: `damage_drops.rs:112`
    documents `no render family claimed \`coin:EnemySpawn-…\`` firing per
    transition — so a coin case was seen and reasoned about before.

  * **the observation, verbatim**: *"Changing rooms flashes magenta
    squares for a brief moment."* **That is the unclaimed-body placeholder
    again** — the same artifact as D39's goblins and the device log's
    `NpcSpawn-*`. ⇒ **a THIRD face**, and this one is TRANSIENT: bodies are drawn
    before their art resolves. ⚠ ~~D39 records the NpcSpawn case as a separate
    unexplained mechanism; this may be that mechanism seen from the player's side
    rather than from the log.~~
    ✔ **RESOLVED 2026-08-09 — it is D46, and yes: this IS the log's mechanism
    seen from the player's side.** The `NpcSpawn` warnings are the transient case
    and no bug row was needed for them; what IS a defect is that Jon still sees
    the flash on **desktop**, where the cover expired **zero** times in a 290 s
    session. ⇒ the cover retires on a snapshot (`unclaimed.count() == 0`) rather
    than on a settled state, so a view published one flush later has no cover
    left. **D46 carries the falsifier — run it before touching the deadline.**
  * **combat / smash** — attacking hurts the attacker; no knockback; robot v3's
    attack VFX lands in the top-left corner instead of its authored area
    (⇒ **D54** found and fixed a slash whose owner defaults to the world origin —
    ⚠ **not proven to be this**, but its new warn is the falsifier);
    knockdown / slow-getup / tech / getup-attack animations (or at least
    architecture slots) missing. ⭐ Jon's *"developing out the smash combat system
    and the ambition combat system should be very similar and feed each other"*
    is DIRECTION, not a bug — read it before treating smash as a side demo.
  * **items** — a laser sword persisted into the ninja dojo after a fight ⇒ **this is queue D50**, whose ability-drop half landed at `481072760` (the drop is room-scoped now); what remains there is the laser sword's own lifetime, which is a design call
    elsewhere (⇒ **D50**, diagnosed: the death drop is session-scoped;
    ⭐ **and a SECOND unfiled site turned up in the same file** — of three drop
    functions, only `drop_ability_pickup` never marks its drop `RoomScopedEntity`,
    which makes an unclaimed boss reward a live lead on the magenta flash);
    holding an item does not reroute the normal attack to its action
    (⇒ ✔ **D51 FIXED** `09abaea3f` — a weapon in hand owns the Attack press).
    ⚠ **D60**: the portal gun still has D51's bug and was out of reach.
  * **pirate sky** — the pirates no longer ride their sharks ⇒ ⛔ **the AUTHORING half is fixed (measured 2026-08-13, queue D47)**: `mounted_on` went 3 → **7** across the four worlds and **zero** rider-brained `EnemySpawn`s lack a mount, so D47's hypothesis that the raiders carry no mount is stale. ⚠ whether they RIDE correctly in play is a different question and is not measured
    (⇒ ✔✔ **D49 CLOSED**: an LDtk editor session nulled every `EntityRef` in
    `sandbox.ldtk` on 2026-07-06; the four refs are restored from the
    `5e4d6448e` blob and both ends are pinned);
    iron marry shoots fireballs instead of her swordgun (⇒ **D48**, and it is
    now a **scoped question for Jon** in `awaiting-maintainer-decision.md`).
  * **camera / transitions** — LDtk-separated rooms move as if a pan should
    happen; a gravity camera mode following the player's reference frame ⚠ with
    reference-frame inputs, which is a relativity design point rather than a
    camera tweak.
  * **input** — holding Up for 2 s as an alternative door/interact. ⚠ connects to
    the standing ruling that a single-press Up is NOT doors.
  * **art / misc** — the shield bubble sits up-and-left of the character ⇒ **this is queue D54**, and 2026-08-13 found the mechanism: an `Anchor` is a SPRITE COMPONENT, invisible to sibling entities, so a ring drawn at the body's world position cannot coincide with art the sheet anchors at `feet_anchor_norm.y = -0.3047`. ⚠ the census that row asked for came back ZERO — no overlay in `ambition_render` reads a raw `kin.pos`
    (⇒ ✔✔ **D55 SOLVED**: the engine's ring is measured correct to the last
    decimal — `pos == kin.pos`, drawn-centre offset `(+0.00, +0.00)` — and the
    misplaced ring is **painted into `player_robot_v3`'s `block` spritesheet row**.
    ⚠ **the two artefacts were inverted for the whole investigation**: the centred
    glow IS the engine's bubble. ▢ what remains is regenerating the art);
    the
    title menu runs 60 FPS against gameplay's 140 (⚠ possibly deliberate
    framepacing — confirm before "fixing"); PCA's C4 symmetry-room challenge
    should switch to a smash track.


---

- ▢ **D50 ✔✔ THE ABILITY DROP IS ROOM-SCOPED — `481072760`.** The unfiled sibling
  found in this row is fixed. ▢ **Jon's laser sword itself is still the design
  half below, and still his.**

  * ✔ **the red named the culprit rather than merely failing**:
    ```text
    a drop outlives its own picture: ["ability_drop:guard"] spawned without
    RoomScopedEntity, but every feature visual is a RoomVisual and therefore
    room-scoped.
    ```
    ⇒ green after one line; crate suite **1,193 passed**.
  * ⭐⭐ **the guard is TWO tests that only work together**, and the pairing is the
    interesting part:
    1. **the invariant** — spawns the drops into a real `World` and asks **the
       World**, not the source, whether each is room-scoped. ⚠ deliberate:
       `#[require]` puts components on an entity **without their name appearing
       at the spawn site**, so a grep-shaped guard would false-accuse — which it
       already did once in this very investigation. It also asserts the
       denominator, so a drop that stopped spawning cannot make it vacuous.
    2. **the coverage** — scans the module's own source via `include_str!` for
       every function constructing a `PickupFeature`, and fails if one is missing
       from the table half 1 drives.
    ⇒ **a fourth drop function reddens half 2 the day it is written**, and the
    only way back to green is joining the table — at which point half 1 starts
    checking its lifetime.
  * ✔✔ **demonstrated, not asserted**: a temporary fourth drop function turned
    half 2 red **while half 1 stayed green**. ⭐ **that is the whole argument for
    having both halves**, and it is the poison I would not have thought to run.
  * ✔ **the `#[require]` check ran first and could have killed the finding.**
    Nothing in that spawn tuple carries `#[require]` at all, and
    `spawn_session_scoped` does **not** insert `RoomScopedEntity` — its sibling
    `spawn_room_in_session` (`lifecycle/session.rs:514`) does. **The marker really
    was absent.**

- **D50-ORIGINAL (superseded in part by the ✔✔ block above — marker stripped, text kept)
  ⭐ A DROPPED WEAPON IS SESSION-SCOPED, SO IT FOLLOWS YOU INTO EVERY
  ROOM.** Jon's observation *"After I fought in the pirate sky … I walked into
  the ninja dojo and there was a laser sword gun just existing there."*
  **Diagnosed 2026-08-09; the mechanical half is closed, the design half is
  Jon's.**

  ### ⭐⭐ A SECOND, UNFILED SITE — found 2026-08-09 by enumerating the siblings

  Looking for D50's site I enumerated **every drop function in
  `features/ecs/damage_drops.rs`**, and three of them answer one question:

  | function | spawn | `RoomScopedEntity` |
  |---|---|---|
  | `drop_currency_coin` (:86) | `spawn_session_scoped` | ✔ :117 |
  | `drop_health_pickup` (:213) | `spawn_session_scoped` | ✔ :233 |
  | **`drop_ability_pickup` (:249)** | `spawn_session_scoped` | ⛔ **absent** |

  ⇒ **ninth instance of [the odd-one-out](../../dev/benchmark-candidates/the-odd-one-out-among-siblings-2026-08-09.md)**,
  and the coin's marker carries a **19-line comment stating the rule** — including
  Jon's own 2026-08-05 log and the black screen it caused:

  > ⛔ **A DROP BELONGS TO THE ROOM IT FELL IN.** … the sim kept publishing a
  > Pickup view for an entity nothing was drawing, so `draw_unclaimed_feature_views`
  > spawned a stand-in for it in the NEW room, every transition, forever …
  > **those stand-ins are what the room-transition cover waits on, so his screen
  > stayed black for the full 8-second deadline.** Two lifetimes for one thing is
  > the bug; the entity and its picture now share one.

  ⭐⭐ **so this is a live lead on D46 and on Jon's black screen, not a tidiness
  item.** A boss's ability reward left unclaimed is exactly the long-lived drop
  that would keep re-standing-in, and it is the one drop function that never got
  the fix its two siblings got.

  ⚠ **and the bug does not depend on the design question.** Whether a boss reward
  *should* persist across rooms is genuinely arguable (a Metroidvania would say
  yes). It is a defect either way, because **the visual is a `RoomVisual` and
  therefore room-scoped regardless** — so the mismatch, not the lifetime, is the
  bug. Two repairs, and they differ in what they mean, not in whether they work:

  * **(a) add `RoomScopedEntity`** — one line, matches both siblings and the
    comment's stated rule, and the reward is gone if you walk out. **Recommended**
    on the "join the majority" rule.
  * **(b) make the visual session-scoped too** — the reward survives the room the
    way an unclaimed upgrade arguably should, and costs a second lifetime
    decision on the presentation side.

  ⚠ ⛔ **NOT YET EDITED.** The file is inside
  `crates/ambition_platformer2d_actor_monolith` and **D33's carve agent is live in
  that crate right now.** One line, thirty seconds, dispatch the moment D33 lands
  — held back only to keep two writers out of one crate.

  ### ✔✔ THE ARROW WAS AUDITED 2026-08-09 AND IT HOLDS — both ends checked

  ⚠ I filed the finding on *"the marker is absent"*, which is only one end. Went
  back and checked the two claims joining it to the symptom, because
  [[feedback_ask_the_tool_dont_model_it]] and because **the same audit on D62 the
  same hour found a broken arrow**:

  * ✔ **it is LIVE, not latent** — `drop_ability_pickup` is called from
    `features/ecs/damage/boss_hit.rs:304`. A boss death really does drop one, so
    the leak is reachable in play rather than a dormant code path.
  * ✔ **the visual really is room-scoped**, which is what makes the mismatch a
    mismatch. Every feature-visual spawn in `ambition_render/rendering/features.rs`
    (`:91`, `:112`, `:275`, and `:447`) carries `RoomVisual`, and `RoomVisual` is
    `#[require(RoomScopedEntity)]`.

  ⇒ **session-scoped sim entity + room-scoped picture = the view outlives its
  drawing**, which is precisely the shape the coin's 19-line comment describes.
  **The chain is verified end to end, not assumed.**

  ### ✔ THE SWEEP: is this defect class anywhere else? — ONE site, and it is this one

  Ran the same question repo-wide rather than trusting one file. The naive sweep
  (*every `spawn_session_scoped`, does its file mention `RoomScopedEntity`?*) gives
  **42 files** and is useless — session-scoped HUD, parallax and fx are *supposed*
  to outlive a room. Narrowing to **sim entities that publish a feature view**
  (`FeatureSimEntity` + `spawn_session_scoped`) gives a population of **five**:

  | file | verdict |
  |---|---|
  | `features/ecs/damage_drops.rs` | ⛔ **the defect** — `drop_ability_pickup`, above |
  | `features/ecs/encounter_rewards.rs` | ✔ **false positive** — see below |
  | `boss_encounter/encounter_entity.rs` | ✔ an `Encounter` coordinator is encounter-lived by design |
  | `abilities/ranged/sentry.rs` | ✔ carries `SENTRY_LIFETIME_S`, self-expires |
  | `abilities/ranged/vortex.rs` | ✔ carries `VORTEX_LIFETIME_S`, self-expires |

  ⇒ **the ability drop is the only one.** Good news for the fix and bad news for
  anyone who wants a ratchet out of it — a one-member population is a guard, not a
  trend.

  ### ⛔⛔ THE FALSE POSITIVE IS THE PART WORTH KEEPING — `#[require]` again

  `encounter_rewards.rs` spawns two chests session-scoped with **no
  `RoomScopedEntity` anywhere in the file**, which is exactly the shape of the
  defect. It is **not** a defect:

  ```rust
  #[derive(Component, Default, Clone)]
  #[require(RoomScopedEntity)]      // <- markers.rs:25
  pub struct RoomVisual;
  ```

  Both chests carry `RoomVisual`, so Bevy inserts `RoomScopedEntity` for them.
  ⇒ **the grep that found the real bug also produced a false accusation against
  correct code, by the same mechanism, in the same population.**

  ⚠ **and this is a starred lesson I already hold** — *"`#[require]` invalidates
  'lacks column X'"*. It nearly cost a wrong ledger row anyway. So the check ran
  in both directions before the finding was trusted: none of
  `FeatureSimEntity` / `PickupFeature` / `CenteredAabb` / `SpawnedThisAttempt`
  carries a `#[require]`, which is **why the ability drop's absence is real** and
  the chests' absence is not.

  ⇒ ⭐ **a component-absence claim now has a mandatory second step**: resolve
  `#[require]` on every component in the bundle before saying a column is missing.
  The first grep answers *"is the name written here"*; only the second answers
  *"is the component on the entity"*, and **they disagreed for 2 of 5 candidates.**

  * **the site, and its comment names his exact item**
    (`features/ecs/damage/actor_hit.rs:566`):
    ```rust
    // Steal the enemy's weapon: a defeated enemy that was wielding a held item
    // drops it as a `GroundItem` the player can grab + wield (e.g. a pirate's
    // gun-sword), via the existing pickup path.
    if let Some(spec) = caps.drops_held_item.clone() {
        writers.commands.spawn_session_scoped(          // ⛔ SESSION, not room
            session_scope,
            (GroundItem { … }, Name::new("Dropped weapon"),
             crate::features::ecs::SpawnedThisAttempt),
    ```
    `boss_hit.rs:319` is the same shape.
  * ⛔ **`SpawnedThisAttempt` is not a room.** The drop is cleaned up by an
    *attempt reset*, and walking through a door is not one. So the two markers it
    carries are session lifetime and attempt lifetime, and **the lifetime it
    needs — the ROOM — is the one it does not have.**
  * ✔ **four `GroundItem` spawn sites, and the scopes disagree**:

    | site | scope |
    |---|---|
    | `spawn_static.rs:396` (authored ground items) | `insert_room_in_session` ✔ |
    | `items/pickup/mod.rs:530` | `spawn_room_scoped` ✔ |
    | `damage/actor_hit.rs:567` (enemy death drop) | ⛔ `spawn_session_scoped` |
    | `damage/boss_hit.rs:319` (boss death drop) | ⛔ `spawn_session_scoped` |

    ⚠ `reset.rs:346` looks like a fifth unscoped site and **is a TEST** — checked
    before claiming it.
  * ⭐⭐ **THIS WAS PREDICTED IN WRITING AND NOBODY ACTED ON IT.**
    `game/ambition_app/tests/room_boundary_unclaimed_views.rs`'s header says, of
    the coin/heart fix at `dd73a3087`:
    > *"the same death path still mints a `GroundItem` weapon under session scope
    > alone, and every future spawner is a fresh chance at the same mismatch."*

    The coins and hearts got `RoomScopedEntity`; **the weapon at the same site
    did not.** [[feedback_a_guard_that_pins_the_fix_defends_the_gap]] — the fix
    was pinned to the two instances that had been observed, the invariant was
    named in prose, and the third instance walked into the ninja dojo a week
    later.

  ### The bug and the question are separable — do the bug

  ⭐ **room-scoping the drop is right under EITHER answer to Jon's design
  question**, because no reading of *"what happens to an item you leave
  somewhere"* puts it in an unrelated room. So it does not wait on him.

  ⚠ but the guard must be the invariant, not the site: **every `GroundItem`
  carries a room lifetime**, poisoned with a session-scoped one, in the same file
  that already crosses two authored doors. Fixing `actor_hit` and `boss_hit`
  alone repeats exactly the mistake this row is about.

  ### ✔ THE EXACT CALL, found 2026-08-09 — it is ONE IDENTIFIER

  `spawn_room_in_session(scope, bundle)` already exists
  (`shared_tangle/src/lifecycle/session.rs:460`) with **the same signature** as
  the `spawn_session_scoped` the two death drops call. ⇒ the change at
  `actor_hit.rs:567` and `boss_hit.rs:319` is the identifier and nothing else:

  ```rust
  writers.commands.spawn_session_scoped(session_scope, ( … ))   // today
  writers.commands.spawn_room_in_session(session_scope, ( … ))  // owned by the room too
  ```

  ⭐ **and that is exactly what the AUTHORED ground items already use** —
  `spawn_static.rs:396` populates them with `insert_room_in_session`, the
  insert-shaped sibling. ⇒ **the death drop is the odd one out among four
  `GroundItem` spawn sites**, and the fix makes it agree with the authored ones
  rather than inventing a lifetime.
  ⚠ no new resource, no new component, no scope plumbing: `spawn_room_scoped` is
  literally *"insert `RoomScopedEntity`"*, so room ownership is a marker the
  damage systems can already apply.
  ⇒ **this row is dispatch-ready**: two identifiers plus the invariant guard.

  ### ▢ AND THE QUESTION IS JON'S, verbatim, not yet scoped

  > *"in the ambition game, what should happen if you leave an item somewhere?
  > Should it despawn? When? If you come back should it still be there? For the
  > skyrim aspect of the game I think sometimes we do need items to remember
  > where they were and if they were moved, but maybe we defer that and just have
  > items be scoped to rooms."*

  ⭐ **he proposes the deferral himself** — room-scoped now, persistence later —
  which is exactly what the bug fix above implements. ⇒ **do the room scoping,
  and leave a note where the persistence hook would go**; do not build a
  Skyrim-style item memory on a guess. ⚠ this belongs in
  `awaiting-maintainer-decision.md` only once someone scopes what "items remember
  where they were" costs — it is not scoped, so it is not filed there.


---

- ▢ **D53 ✔✔ THE DECISION NOW RUNS ON A MACHINE WITH NO ANDROID — `578d1b5c7`.**
  The blind fix has been executed. ▢ **the GLUE is still uncompiled** — see the
  bottom of this block.

  * ✔ **red, with the pre-fix `.take()` temporarily restored** — exactly the
    assertion this row predicted, and it names the bug in its own message:
    ```text
    a_refused_restore_keeps_the_saved_mode_for_the_next_edge ... FAILED
      a refused restore must KEEP the saved mode; taking it here is the bug
        left: None     right: Some(Playing)
    a_second_capture_overwrites_a_kept_mode ... FAILED
    ```
    ⇒ green with the peek restored: **4 passed**.
  * ✔ **extracting changed no behaviour**, and two things were checked rather
    than assumed: every branch condition, every `world_log` string and every
    `next_mode.set` is identical; `ResMut` change detection is unaffected
    (`detect_android_suspend_state` already `DerefMut`s the resource every
    frame); and `suspended` was hoisted so the two-phase borrow cannot be a
    borrowck surprise **in code that cannot be compiled here**.
  * ⭐⭐ **one deliberate deviation from my signature, and it is the right call**:
    `saved` is `&mut Option<GameMode>`, not a value. **The bug IS a mutation of
    the saved slot**, so a by-value signature would have left the defect in the
    caller and made the red undemonstrable. ⇒ my signature would have produced a
    test that passed without proving anything.
  * ✔✔ **THE GLUE IS NOW TYPECHECKED — done 2026-08-09 on an empty fleet.**
    Retargeted every gate in the file to the host, ran the gate **with
    `--features audio`**, restored from a byte-copy (⛔ **never `git checkout --`**),
    and re-ran the gate to confirm the restore.
    ```text
    cargo check -p ambition_app --all-targets --features audio   EXIT=0
    ```
    ⇒ **every `#[cfg(target_os = "android")]` item in this file compiles**,
    including `mod audio_lifecycle` and its `bevy_kira_audio` use. The row's
    standing caveat *"uncompiled — no NDK here"* is retired.
    ⚠ **typechecked is not verified**: nothing here proves Android *delivers*
    these events in this order. **That still needs a device**, and the decision
    table's unit tests are the only behavioural evidence.

  * ⛔⛔ **AND THE FLIP LIED TO ME FIRST — a partial cfg flip produces a PLAUSIBLE
    ERROR.** My first pass used
    `sed 's/cfg(target_os = "android")/cfg(target_os = "linux")/'`, which matched
    the 8 **simple** gates and missed the compound one at `:344`:
    ```rust
    #[cfg(all(target_os = "android", feature = "audio"))]
    mod audio_lifecycle { … }
    ```
    ⇒ the check failed with **`E0433: cannot find module or crate
    audio_lifecycle`** — a real-looking error naming a real module, which reads
    exactly like *"the Android code references something that does not exist"*.
    ⭐ **it was my own flip.** The fix is to substitute on `target_os = "android"`
    itself rather than on the whole attribute, so compound gates come with.
    ⚠ **that also rewrites the string inside DOC COMMENTS that quote the
    attribute as prose** (two here) — harmless for the check, and the reason the
    restore must be a byte-copy rather than a reverse `sed`.
  * ⭐ **what the refactor bought**: the uncompiled surface is now *"a `match`
    over five variants plus formatting"*. **All branch logic is executed by the
    desktop suite.**

- **D53-ORIGINAL (superseded by the ✔✔ block above — marker stripped, text kept)
  THE ANDROID SUSPEND `.take()` IS FIXED, AND THE FREEZE IS NOW
  RECOVERABLE RATHER THAN IMPOSSIBLE.** Jon asked for this directly and GPT 5.6
  re-flagged it as *"still not fixed, only instrumented"* — correct, it was.
  Fixed 2026-08-09, `game/ambition_app/src/host/platform/android.rs`.

  ### ⭐⭐ IT CAN BE TESTED WITHOUT A DEVICE, AND THE MODULE IS ALREADY REACHABLE

  The standing excuse on this row is *"no NDK here, so it cannot be compiled,
  let alone tested"*. ⛔ **That is true of the GLUE and false of the DECISION**,
  and the decision is the entire thing that was fixed blind.

  `apply_android_suspend_to_game_mode` is a pure state machine wearing a Bevy
  system's clothes. Its whole input is:

  ```text
  (suspended: bool, observed: GameMode, saved: Option<GameMode>)
      -> capture / capture-skipped / restore / restore-refused / idle
  ```

  ⇒ **nothing in that is Android.** `AppLifecycle`, `NextState` and `world_log`
  are how the decision is *delivered*; they are not how it is *made*.

  ✔ **and the placement problem does not exist** — checked, not assumed:
  `host/platform/mod.rs:10` is a bare `pub mod android;`, **not** gated. Only
  **seven items inside** the file carry `#[cfg(target_os = "android")]`. ⇒ **an
  un-gated `fn decide_suspend(..) -> SuspendDecision` in that same file compiles
  and unit-tests on desktop today** — no new module, no new crate, no
  restructuring, and the gated system shrinks to glue that calls it.
  ⚠ one detail: `GameMode`'s `use` is currently *inside* a cfg (`:33`) and has to
  come out with it. Nothing about `GameMode` is platform-specific.

  ### ⭐ AND THE TEST WRITES ITSELF FROM THE COMMENT THAT REPLACED IT

  The fix's own comment already states the failing scenario in full — *"on a
  spurious short suspend/resume pair — a 0.6 s one is in the 2026-08-08 device
  log — the resume edge can run while `observed` is still the PRE-pause mode, so
  the restore is refused"*. That is a **three-line unit test**:

  ```text
  suspend(Playing)                 -> saved = Some(Playing)
  resume(observed = Playing)       -> refused, and saved is STILL Some(Playing)   <- the bug
  resume(observed = Paused)        -> restored to Playing
  ```

  ⛔ **under the old `.take()` the second line dropped the value**, so this test
  is red on the pre-fix code and green after. ⇒ **the blind fix becomes an
  observed one**, which is the only thing this row is actually missing.

  * ⚠ **what it still does NOT prove**: that this was Jon's freeze. The decision
    table is testable; *"does Android deliver these events in this order"* is not,
    and needs a device. ⭐ **say both, separately** — the row's current honesty
    (*"makes the freeze recoverable, not impossible"*) is worth preserving exactly.
  * ⭐ **the general lesson, which is bigger than Android**: *"this platform is
    untestable here"* is almost always a claim about the **glue**. Split the
    decision out and the untestable part shrinks to the part that genuinely is.
    ⇒ [[feedback_reorganize_not_adapt]].

  * **what changed**: the resume branch `.take()`d the saved mode *before* the
    guard decided whether to accept it, so a refused restore discarded it with no
    second chance. It now peeks and clears **only on the branch that restores**.
  * ⭐⭐ **and the sharper mechanism, which the old comment did not have:
    `NextState` is DEFERRED.** The suspend edge calls
    `next_mode.set(GameMode::Paused)` and the transition applies later. On the
    spurious 0.6 s suspend/resume pair in the 2026-08-08 device log, the resume
    edge can run while `observed` is still the PRE-pause mode ⇒
    `matches!(observed, Paused)` is false ⇒ restore refused ⇒ **and then the
    deferred `Paused` lands.** Under `.take()` that is a game paused with the
    only thing that could unpause it already thrown away — which is Jon's report
    word for word: *"I can still do the menu … but I can't move my character."*
  * ⛔ **BLIND FIX — no device, no `adb`, and NOT EVEN COMPILED.** Reasoned from
    the log and the code.
    ⛔⛔ **I tried to compile-verify it and could not, which is itself the
    finding.** The `aarch64-linux-android` rustc target IS installed, so
    `cargo check --target aarch64-linux-android -p ambition_app` looks like it
    should work. It dies before reaching a single first-party crate:
    ```
    error: failed to run custom build command for `android-activity v0.6.1`
      error occurred in cc-rs: failed to find tool "aarch64-linux-android-clang++"
    ```
    The NDK is absent, and `android-activity` needs a C++ compiler in its build
    script. ⇒ **every `#[cfg(target_os = "android")]` line in this repo is
    unreachable by any check available here — not just untested, UNCOMPILED.**
    A typo in that function would survive every gate the goal harness runs.
    ⛔ **and the journal already said so — §0, in the file I wrote yesterday.**
    I burned a cold cross-compile rediscovering a documented fact. It also
    records the workaround I should have reached for:
    > *"temporarily retarget a file's `cfg` gates to the host, typecheck, and
    > **poison-check the probe** (inject a type error, confirm it surfaces)
    > before trusting the green. Restore from a copy afterwards and re-count the
    > gates."*
    ▢ **not done yet, deliberately**: that procedure edits a file `app_it` will
    compile, and D39 is mid-suite in this tree. Doing it now would hand another
    job a red that is not theirs
    ([[feedback_dont_run_the_suite_while_editing]]). **Run it when the tree is
    free** — it is the difference between "uncompiled" and "typechecked".
  * ✔ **what I could establish**: `GameMode` derives `Copy`
    (`shared_tangle/src/schedule.rs:571`), so `if let Some(prev) =
    state.mode_before_suspend` copies rather than moving out of the `ResMut`,
    which is the one way this edit could fail to compile.
  * ⚠ **so Jon's next Android build is the compile check.** Say so when handing
    it over; do not report this row as landed-and-verified. See
    `dev/journals/android-what-an-agent-cannot-see-2026-08-08.md`.

  ### ▢ THE RESIDUAL GAP, stated rather than papered over

  The function early-returns unless `just_changed`, so a refused restore is only
  retried on the **next** suspend/resume edge. ⇒ background-and-foreground once
  more and the mode returns; **a player who never backgrounds again is still
  stuck.** The fix removes the unrecoverable state, not the first freeze.

  Closing it needs the guard to stop inferring *"we forced this pause"* from
  `observed`. Two candidates, both behaviour changes on an untestable platform:
  * consult the pending `NextState` — accept when a `Paused` transition is
    in flight;
  * record the fact on the capture edge (`we_forced_pause`), and stop
    re-deriving it.
  ⚠ the second is cleaner and is the same shape as
  [[reference_a_derive_memo_is_rollback_state]] in reverse — **don't infer an
  authority you already know.** ⛔ but the guard exists to protect a user who
  navigated while backgrounded, and overriding it without a device is how you
  trade a rare freeze for a common stomp. **Wait for Jon's next device run.**


---

- ▢ **D64 FOUR MORE OF JON'S OBSERVATIONS HAVE NO LEDGER ROW — index, not triage.**
  Found 2026-08-09 by reading all 40 bullets of `JONS_OBSERVATIONS_BUGS_AND_ISSUES.md`
  against the queue. ⭐ **the method is worth repeating**: D61, D62 and D63 all came
  out of this pass, and all three were fully diagnosable in minutes.
  ⚠ **these four are NOT scoped** — one fact each, gathered while sweeping:

  * ✔ **multi-coin block — LANDED 2026-08-10.** `MaryOBlockContents::Coins(u8)`,
    its parse (`Coins` / `Coins5` / `coins = 5`; a bare `Coins` is ONE, per Jon's
    *"num-coins=1 is the default instance"*), and the `reward_for` arm.

    ⛔ **the scoping above was incomplete, and the missing part is the mechanic.**
    "A new variant, its parse, and an arm" is the AUTHORED side; a multi-coin
    block is also STATEFUL — it pays one coin per hit until exhausted, and
    `SpentPowerBlocks` was a `HashSet`, a block being spent or not. So the count
    had nowhere to live. ⇒ the resource carries a per-block tally beside the set,
    `take_one_coin` promotes a block to spent on its last coin, and `rearm_all`
    clears both halves so "until reset" means the reload that already re-arms
    everything else.
    ⭐ **the set stays the authority for "done"** — a partial entry is a block
    mid-payout, so `is_spent`, the bonk gate and the spent ART all keep their
    exact existing meanings and a `Coins(1)` block is byte-identical to today's.

    ⚠ **rollback: schema v24**, because the checksum must see the COUNT — two
    peers whose blocks have paid a different number of coins must not agree on
    the hash. ⛔ **and `rollback_codec_shape.py` is BLIND to this class**: it
    watches `put_*` sequences, and this is a `clone_checksum` registration whose
    wire-visible half is a projection. Second blind spot in that guard, recorded
    in the v24 note rather than discovered later.

    Payout side needed nothing: `BlockPayout::Coins` already credits the purse
    directly and plays the coin cue — Jon's *"not a real coin entity, just a vfx
    and your coin count directly goes up by 1"* was already true.
    ▢ **what is NOT done**: the coin *pop* VFX. The block flinches
    (`BlockStruck`) and the cue plays, but nothing draws a coin arcing out.
  * **spent blocks in 1-2 don't look spent, and 1-2 has no tile texture.**
    ⚠ likely the **same authoring class as D61** — 1-2 is the level that also
    lacks its flagpole banner and finial. ⇒ **check D61's fix first; it may share
    a cause.**

    ⇥ ⛔ **BOTH HALVES CHECKED 2026-08-13, and neither reproduces.** D61's class
    was *"the room authors no companion block, so the picture is silently
    missing"*; this is not that.

    * ⛔ **"no tile texture" is FALSE at the authoring level.** `mary_o_1_2`'s
      `CollisionArt` AutoLayer carries **1,104 tiles** on tileset uid `106934` —
      *more* than `mary_o_1_1`'s **840**, on the SAME tileset. What differs is
      `bgColor`: `#5c94fc` (sky) versus `#14101c` (near-black), which is 1-2
      being underground.
    * ⛔ **and the spent-look chain is correct for both levels.**
      `dress_power_blocks` re-derives from `SpentPowerBlocks` every frame and
      needs no companion block, `"power"` parses to `Question` (so 1-1's three
      `Power` entries are dressable), and `Question` / spent-`Hidden` both resolve
      `EntitySprite::SpentBlockTile`.

    ⇥ ⭐ **a reading of the report that needs no defect**, offered as a hypothesis
    rather than a finding: **1-2 authors 1 `Question`, 2 `Brick`, 1 `Hidden`**,
    and a `Brick` is *deliberately* never dressed — *"a brick wears the level's
    masonry"*, left alone so it draws the level's own tile. The three blocks at
    px 768/800/832 are a `Question` flanked by two `Brick`s, so striking that row
    changes ONE of three pictures by design. In 1-1 five of eight are dressable.
    ⚠ **that is a guess about what was SEEN, not about the code**, and it is here
    so the next reader can confirm or dismiss it in one look rather than
    re-deriving the chain above.
  * **1-2's invisible brick does not trigger from below.** ⚠ note
    `ldtk_vocabulary.rs:252` maps `MaryOBlockLook::Hidden` to
    `Always(MaryOPickup::Coin)`, so the contents are authored — suspect the
    *contact* side, and D63 is about to be in `break_bricks`'s head-contact gate
    anyway. ⇒ **cheap to look at while D63 is open.**

    ⇥ ✔ **THE WHOLE CHAIN IS VERIFIED, 2026-08-13 — every link is tested and the
    block is reachable.** The *"suspect the contact side"* guess above is
    refuted; each step was checked rather than assumed:

    ```text
      authored      mary_o_1_2 px [1280,256], kind "Hidden"      (the only one in the game)
      lowered       Hidden ⇒ BlockKind::BonkOnly                 ldtk_vocabulary.rs:335
                    …and that lowering is TESTED                 ldtk_vocabulary.rs:746
      contact       rising into a BonkOnly block reports a
                    Head contact — TESTED                        movement/tests/contacts.rs:90
      contents      Hidden ⇒ Always(Coin), no field needed
      consumed      bonk_power_blocks takes ContactKind::Head
                    + ContactSource::Block
    ```

    ⇥ ⚠ **AND MY REACH ARITHMETIC WAS WRONG THE FIRST TIME, which is why the
    numbers are here.** Ground in that column is row 24 (y 384) and the block's
    underside is y 288, so a standing head at 336 must rise **48 px**. A plain
    jump is `450²/(2·2250) = 45 px` — three pixels short, and I nearly filed that
    as the defect. It ignores `held_rise_gravity_scale: 0.2`: holding jump makes
    the rise gravity 450, and a band-3 launch is `450+30 = 480`, so the real apex
    is `480²/(2·450) = 256 px`. **Reachable with five times the margin needed.**

    ⇒ **no mechanism explains the report, and the placement does not either.**
    Same landing as D70 and D54: this now needs a reproduction rather than
    another inspection. ⛔ do not re-derive the table above.
  * **a gravity camera mode that follows the player's reference frame** (*"we
    should be careful so we use player-reference-frame inputs in this mode"*).
    ⚠ this is a **feature**, not a bug, and it lands on
    [[project_reference_frames]] — the typed-at-the-seam work. ⇒ **do not start it
    without Jon**; it is the only one of the four that is a design commitment.


---

- ▢ **D114 ⛔ HITLAG IS PRIMARY-PLAYER-SCOPED — a hit between two fighters that
  are neither produces NO freeze at all** (found 2026-08-13 while checking the
  player/actor roads for more of D108's shape).

  The law is stated twice in the code and it is symmetric: *"the same freeze the
  ATTACKER takes, from the same law — a landed hit is one event"* (`damage_apply`)
  and *"the attacker freezes for exactly as long as its victim, from the one
  hitlag law"* (`damage/mod.rs`). Both sites arm `BodyCombat::hitstop_timer`, on
  the victim and on the attacker, whoever they are.

  **Nothing on the actor road reads it.** Every reader is one of four:

  ```text
    body_integration.rs:174   sim_dt = 0 — the PLAYER road only
    time_control.rs:358       a clock-scale request — `With<PrimaryPlayer>`
    hit_camera_shake.rs:107   camera amplitude (reads across bodies)
    combat_geometry_view.rs   `hitlag_s`, a debug/render projection
  ```

  ⇒ so for a hit involving the primary player, hitlag happens: the player freezes
  outright and the world drops to `bullet_time_scale` **0.125**. ⛔ **for a hit
  between two bodies where neither is the primary player — CPU versus CPU on the
  Smash stage, or any seat that is not slot 0 — neither reader fires and there is
  no freeze and no slow.** The timer is armed, decays, and moves a camera.

  ⚠ **what is NOT claimed**: that the player-involved case is wrong. The attacker
  freezing at 0 while its victim runs at 0.125 is an asymmetry the comments do
  not describe (*"exactly as long"*, *"one event"*), but a 1/8 world plus a frozen
  attacker is a defensible hit-emphasis and may be authored. ⇒ **the certain part
  is the third case**, and it is the one Smash is made of.

  ⭐ **same class as D108, third instance**: a per-body timer armed for every body
  and consumed only on the player's road.

  ⇥ ✔ **AND IT HAS THE TREATMENT NOW.** `BodyCombat::is_in_hitlag()` names the
  question and the player road asks it instead of spelling `hitstop_timer > 0.0`.
  ⛔ that does NOT fix the row — the actor road still has no hitlag branch at all
  — but adding one is a call rather than a re-derivation, and the asymmetry is
  greppable: **one road asks the body, the other never asks.** Pinned by
  `hitlag_is_a_body_question_even_though_only_one_road_asks_it`.

  ⇥ ⛔⛔ **THE OBVIOUS FIX WAS ALREADY TRIED AND REJECTED, AND THIS ROW DID NOT
  SAY SO** (found 2026-08-13 by grepping for the thing the row says is missing,
  before writing it). `features/ecs/actors/update.rs` carries the experiment in a
  standing note directly above the actor's integration call:

  > *"NOTE on hitstop: the resolver arms `combat.hitstop_timer` on every body,
  > but an actor's sim dt is NOT frozen by it (tried; per-victim freezes in
  > AI-vs-AI fights made duels degenerate — fighters spent whole bouts frozen).
  > The player-involved hitstop beat stays the global-clock rule
  > (`emit_player_time_intent_system`); a per-body proper-time beat is a future
  > ProperTimeScale concern (ADR 0011 seam)."*

  ⇒ **so D114 is not the bug it reads as.** *"Give the actor road a hitlag
  branch"* is the one thing that has been measured and produced worse play, and
  landing it would be re-introducing a rejected change with a ledger row as
  cover. The asymmetry the row measured is real; the repair it implies is not
  available.

  ⚠ **what is actually open is a FEEL question for Jon, and it is small**: what
  should a hit between two bodies neither of which is the primary player do? The
  three candidates are all authorable — nothing (today), a per-body proper-time
  freeze (tried, degenerate at current durations), or the global 0.125 beat the
  player-involved case already uses, extended to any hit between seated
  fighters. ⭐ the third is the one that fits Smash, costs one run condition, and
  has never been tried; it is also the one a CPU-versus-CPU match would show off.

  ⇥ ⚠ **and the ADR 0011 pointer is the real answer's home.** A per-body freeze
  that does not degenerate needs proper time per body, not a zeroed `dt` — which
  is a successor campaign, not this row.


---

- ▢ **D113 JON'S "MARY-O CAN ONLY HAVE 1 FIREBALL OUT" IS SATISFIED IN CODE AND
  UNRECORDED ANYWHERE** — so the observation still reads open (verified
  2026-08-13).

  `movement.rs:36` authors `pub const MAX_LIVE_SPARKS: usize = 2`, and the fire
  gate at `movement.rs:192` reads it: `if live_sparks.iter().count() >=
  MAX_LIVE_SPARKS { continue }`. The constant's own doc calls it *"the classic
  two-on-screen rule. Authored by the character, enforced by counting HER live
  shots, so it constrains nobody else's projectiles"* — which is both halves of
  what Jon asked for, including the scoping.

  ⇒ **the row exists because nothing else says so.** `MAX_LIVE_SPARKS` appears
  in exactly two places, both above; there is no ledger row, no campaign row and
  **no test**. A one-character edit to that constant restores the behaviour Jon
  reported and nothing goes red.

  ⇥ ✔ **GUARDED the same day** —
  `movement::tests::she_may_have_max_live_sparks_out_at_once_and_not_one_more`
  fires with an empty screen, with one spark up, and at the cap; only the third
  refuses.

  ⛔⛔ **AND MY OWN PRESCRIPTION HERE WAS WRONG.** This row said *"asserting the
  CONSTANT equals 2 would be worthless: it must count live shots."* The first
  version of the test followed that advice — every expectation derived from
  `MAX_LIVE_SPARKS` — and **it passed with the constant set to 1**, which is
  precisely the bug Jon reported. A mechanism test cannot see the number it
  counts to.

  ⇒ **the guard needs BOTH**: the gate counts her live shots (survives a retune)
  AND the cap is at least two (the product decision the observation was about).
  Falsified by setting the constant to 1 — the second assertion is the one that
  goes red. ⭐ generalizable: when the observation is about a NUMBER, a test
  parameterised by that number is vacuous against it.

  ### ⇥ AND THE SAME SWEEP CHECKED FOUR MORE OF JON'S LINES (2026-08-13)

  ```text
    "1 fireball out at a time, allow 2"      ✔ DONE  MAX_LIVE_SPARKS = 2, now guarded
    "small mary-o shouldn't break bricks"    ✔ DONE  only_a_tall_or_fire_mary_o_breaks_a_brick,
                                                     and its fixture already carries the
                                                     vacuity note
    "we need an SFX for collecting coins"    ✔ DONE  COIN_PICKUP_SFX, guarded at
                                                     powerups.rs:2246
    "goblins in the goblin encounter have    ◐ TRACED it is the uncast large_brute in
     magenta boxes"                                  waves 3-4, not the goblins — see
                                                     awaiting-maintainer-decision.md
    "sanic still says FLY instead of         ✔ DONE  the Utility slot reads "Transform",
     super transform/untransform"                    with a poison asserting the label may
                                                     never contain "fly" again — stated as
                                                     the SYMPTOM rather than its cause,
                                                     which is why it still holds
    "1-2's flagpole has no flag"             ▢ UNRESOLVED — the pole is authored as
                                                     TILES, not an entity (neither level
                                                     has a flag/pole entity at all), so
                                                     this needs a render check rather
                                                     than a data one. Not guessed.
  ```

  ⇒ **four of Jon's lines are done and only one of them said so anywhere.** ⚠ the
  observations file is his to edit, so this is recorded here rather than there —
  its own header says reasoning belongs in the ledger.

