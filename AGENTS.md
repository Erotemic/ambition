# Agent guide for Ambition

This is the repository operating guide for coding agents. Keep it short, session-agnostic, and focused on routing. Put durable project knowledge in `docs/`, engineering memory in `dev/`, and generated navigation aids in `.agent/`.

## Core Values

* Avoid player-centrism. Value the principle of relativity.
* Find the elegant solution. Jon will push back on hacks.
* Correctness is emergent from elegance.
* **Pre-release engine, zero dependents.** Behavior and feel are NOT sacred until a polish pass — optimize for the elegant unified design, not for preserving current output. Delete duplicates, compat shims, and bridges on sight. Never fold a richer path onto a simpler one to "preserve" it; make the richer/general path universal and delete the rest.
* **Unified actors.** Every actor — the player included — is one body: kinematics + composable ability limbs + a capability mask, driven by a Controller (Human / Brain / RL) and observed via one `WorldView`. Player / Enemy / Boss / NPC are DATA (controller + capabilities), not types or code paths. The player's movement is the good base — make enemies and NPCs *rise to it* (adopt the rich limb pipeline), never drag the player down to a simpler path. Adding a character should be: author capabilities + pick a controller, zero core edits.
* **ONE BODY, ONE PATH — never bifurcate. This is the most-violated rule; read it before any combat/movement/visual/state change.** The player is an actor. Before you write *anything* keyed to "player" or to "actor/enemy/boss" — an attack, a hitbox, a damage rule, a VFX/SFX emit, a shield, a reset, a state machine, a brain hook — run the **bifurcation smell test**: *"Does the other controller kind already do this on its own code path?"* If yes you have found a **FORK**, and your job is to UNIFY onto the single shared seam and DELETE the other side — NOT to add a second site. ⛔ **Adding a parallel emission site / state component / system / spec for an effect that already exists elsewhere is a BUG, not a fix — even if it compiles and every test passes.** A green test on a forked path is worthless. If you genuinely cannot finish the merge in one pass, do NOT add the parallel path "for now": route the new caller *through the existing seam* (extract one shared fn/system/event if none exists) and log the remainder in `dev/journals/code_smells.md` with `BIFURCATION:` as the first word. ⭐ "unification" always means *delete one path*, never "make them behave similarly". **What is already unified (melee end-to-end, the movement driver, the two-clock blink), what stays deliberately separate and why, and the next elevation are in `docs/concepts/one-body-one-path.md`** — that inventory is STATUS and goes stale; the rule above does not.

## Cold start

For non-trivial work, read and localize in this order: (1) `README.md`,
`AGENTS.md`, `.agent/README.md`, `docs/README.md`; (2) `python
scripts/agent_query.py "<task words>"`, before any broad source search; (3)
`docs/concepts/engine-mental-model.md` for durable ownership/data-flow, skimming
`docs/concepts/invariants.md` — the traps that actually bite; (4)
`docs/planning/vision.md` plus the relevant `docs/planning/tracks.md` entry; (5)
the likely crate's generated packet and `MODULES.md`; (6) ONE focused
concept/system/recipe/tool doc or ADR; (7) `dev/journals` and
`dev/benchmark-candidates` for the symptom or invariant.

Do not read all of `docs/`, `dev/`, or a multi-megabyte flat index by default.
See `docs/recipes/fresh-agent-navigation.md` for the drill-down protocol.

## Authoring submodules are part of the architecture

Ambition has first-class authoring/content submodules even when a source export or
uninitialized clone leaves their directories empty. **Never infer a missing
authoring capability from an empty submodule directory.** Check `.gitmodules`, the
root README's Agent-native authoring toolchain table, and the canonical repositories:

- [sprite renderer](https://github.com/Erotemic/ambition_sprite2d_renderer)
- [music renderer](https://github.com/Erotemic/ambition_music_renderer)
- [SFX renderer](https://github.com/Erotemic/ambition_sfx_renderer)
- [development measurements](https://github.com/Erotemic/ambition_dev_measurements)
- [LDtk map assets](https://github.com/Erotemic/ambition_map_assets)

The preferred authoring model is agent-native: inspect semantically, mutate through
supported source formats or intent-level tools, validate/prepare before runtime, and
generate concise review artifacts. A graphical editor is optional unless the task
actually calls for manual visual editing. Read
`docs/concepts/agent-native-authoring.md` before designing a new authoring surface.

## Generated navigation protocol

`.agent/` holds commit-matched navigation: `.agent/index/catalog.json` for the
overview, per-crate packets under `.agent/index/crates/`, and
`.agent/ecs_inventory/crates/` for Bevy ownership, scheduling, resources,
messages and spawn sites. Reach them through `scripts/agent_query.py` rather than
dumping whole JSON indexes into context. Generated data only LOCALIZES an owner —
confirm in source, which wins for implementation fact (active planning/ADRs win
for intended direction).

## Source-of-truth order

(1) fresh user instructions; (2) **the master plan under `docs/planning/`**, the
primary coordination surface for direction and tasking — keep it current when
work materially changes status or direction (exact same-commit bookkeeping is not
required); (3) ADRs under `docs/adr/` and concepts under `docs/concepts/`; (4)
focused docs under `docs/systems/`, `docs/tools/`, `docs/recipes/`; (5)
brainstorms under `docs/brainstorms/` (Jon's — agents never write there); (6)
engineering memory under `dev/` and generated indexes under `.agent/`.

`docs/current/` is retired; `docs/vision/` holds auxiliary notes only — direction
lives in `docs/planning/`. `docs/archive/` is evidence, not authority.

## Current architectural stance

- Ambition is Bevy-native. Do not resurrect backend-neutral constraints unless a new ADR says so.
- Prefer data-driven ECS flow: authored/generated data -> Bevy components/entities -> systems -> messages/effects.
- LDtk owns world/level authoring. RON room manifests are historical; RON may still be used for tuning, save/settings, and other data where appropriate.
- Preserve desktop, web, Android/mobile/touch, controller, and Steam Deck paths. iOS is deferred for hardware, not excluded.
- **Binary asset payloads are git-ignored but PRESENT on disk.** *Git-ignored is
  not missing* — `ls` before concluding an asset is unavailable, and never build
  fetch/hydration machinery as part of a feature (distribution is Jon's). A
  feature owes only: degrade visibly when a file is absent. ⚠ **they do not
  travel to a `git worktree`** — `crates/ambition_platformer2d_actor_monolith/assets/sprites/` has 972
  files on `main` and 4 in a fresh worktree, so an asset-touching test run there
  fails for reasons that have nothing to do with the change. Full rules:
  `docs/recipes/adding-an-asset.md`.
- **Crate layering:** foundations and domain services feed the unified
  simulation heart; observation/presentation consume it; runtime/provider/host
  compose it; game providers own named content. `ambition_platformer2d_actor_monolith` is not awaiting
  a size-driven carve. Current roles and accepted extractions are in
  `docs/planning/engine/architecture.md` and `docs/planning/tracks.md`.

## Autonomous decision-making

When operating autonomously and you hit an architecture or design fork, **make
the choice Jon would most likely make and act** — read
`docs/planning/decision-principles.md` + `docs/concepts/autonomous-decision-making.md`.
Reserve questions for product/scope, irreversible/outward-facing acts, or true
intent ambiguity; otherwise infer and keep going. Until a polish pass, output/feel
is not a constraint. The gates: it compiles (including `ambition_app`) and
invariants hold.

## Verification

* **Drive the real headless sim — don't say "I can't test it."** Step the actual
  sim (`headless` / `trace_replay`) and observe; if a state can't be exercised
  headlessly, fixing THAT is the priority. Only visual feel ships BLIND.
* **Test invariants/properties, not tuned values or feel** — strongest are
  symmetry/covariance (C4 gravity, through-portal); no regression tests pinning
  unpolished behavior.
* **Replay/bit-identical tests are canaries, not cages** — a failure is info;
  re-baseline when the diff isn't egregious. Full doctrine:
  `docs/planning/engine/headless-verification.md`.
* **`cargo check -p <one_crate>` is not the gate — `cargo check -p ambition_app`
  is.** A per-crate check (even `cargo test -p <crate> --lib`) has been observed
  green on a crate that fails to compile in the app build. "It compiles" means
  the app compiled. App-level tests build into ONE `app_it` target, so
  `--test <file_name>` will not resolve: `cargo test -p ambition_app --test
  app_it -- <module>`.
* ⛔ **AND `ambition_app` IS NOT THE WHOLE GATE EITHER — run
  `cargo test -p ambition_demo_smash_app` when you touch character, movement or
  combat.** Two crates have been found red while every check the run performed
  was green (`ambition_platformer2d_host`, `ambition_demo_smash_app`), and one of
  them is SMASH — the proving ground the campaign measures itself by. A proving
  ground nobody runs proves nothing. The goal guard now runs its suite; that is a
  backstop, not a substitute for running it beside the change that could break
  it.
* ⛔ **A COMPILING GAME CAN STILL DRAW THE WRONG ART, AND NO SUITE SEES IT.**
  `sprites_0_5x` / `sprites_0_25x` / `sprites_potato` are what the runtime loads
  under the Low / Medium / Potato quality profiles. A stale PNG there is a valid
  PNG, so every test stays green while the game draws last week's art at one
  quality setting and today's at another — which is what Jon saw on 2026-08-12
  (*"my sprite went from the robot v3 character to the robot v2 character"*, and
  Emmy new on the select screen but old in the match: 163 of 192 sheets were four
  days behind). Publishing art means `./regen_visual_quality_variants.sh` too;
  `python3 scripts/check_quality_variants_are_fresh.py` answers the question in a
  second and is in the goal guard.

## The cheapest sufficient command

**Pick the narrowest command that covers what you changed:**
[`docs/recipes/cheapest-sufficient-check.md`](docs/recipes/cheapest-sufficient-check.md)
— a table of what to run, what each row does NOT cover, and every cost MEASURED
on 2026-08-03 against a warm target dir. Four rules Jon set live there too: the
write-ahead worktree, interlacing architecture with feature work, one big sweep
then targeted only, and getting a DISTRIBUTION before theorising about a slow run.

Five of its facts belong in your face rather than behind a link:

- ⚠ **`cargo check -p ambition_app` is the gate, never `-p <one_crate>`.** A
  per-crate check has been observed green on a crate that fails the app build.
  21s; there is no excuse.
- ⛔ **but a CHANGED AUTHORED TYPE is the one thing no build covers.** Several
  games author their content as RON in a Rust string literal
  (`SNAKES_ON_A_PLANE_ROSTER_ROWS`, `SMASH_ROSTER_RON`, `SANIC_CATALOG_RON`,
  `POCKET_CATALOG_RON`, …), and nothing typechecks the inside of a `&str`. On
  2026-08-07 `ArchetypeSpec::is_aerial` went `bool` → `Option<bool>`, the app
  check stayed green, and both of Mary-O's flying snakes failed to parse at
  startup — taking her WHOLE roster down, because assembly `.expect()`s.
  ⭐ **the guards existed and would have caught it in 9 seconds**; what was
  skipped was running them. So after changing a type an authored struct uses:
  `grep -rn '<field_name>' --include=*.rs --include=*.ron` and **run the tests of
  every crate that grep touches.** The `*.rs` half is the half that matters.
  ⛔ **but that command CANNOT SEE THE SPRITE SHEETS, and they are authored
  `.ron`.** Measured 2026-08-08: `grep -rlS --include=*_spritesheet.ron
  body_pixel_bbox .` returns **0** — across **184 files that contain it**. This
  grep honours ignore files, and the art is gitignored
  (`.gitignore:110`, `…/assets/sprites/**/*.ron`). A recursive search also skips
  SYMLINKED assets in a worktree unless given `-S`. **Two independent silent
  skips, both returning a clean `0`.**
  ⭐ **when a search touches `assets/`, use `find … | xargs grep`** — it has
  neither behaviour:
  `find . -path '*/assets/sprites/*.ron' -not -path './target/*' | xargs grep -l '<field>'`
  ⚠ **and know what DOES guard the sheets, because it is one sheet, not 190.**
  Catalog ROWS are swept by several running tests (`declared_art_resolves`,
  `rendered_identities_are_registered`, `character_containment`). The sheets' own
  parsed CONTENT is reached by `posed_body_geometry`, which a running test calls
  for **the snake** (`enemy_quad_matches_its_box`) — so a change that breaks every
  sheet's parse fails loudly, and **a change that breaks only some does not**. The
  only whole-population sweep, `hall_scale_spread::print_how_tall_every_character_stands`,
  is `#[ignore]`d and asserts nothing by design.
  ⚠ do NOT audit coverage by asking whether a test mentions the constant — five
  of these are named exactly once outside their own definition and are covered
  anyway, because the crate's plugin-composition test parses them transitively.
- ⛔ **the 11-second `repo tooling (scripts/tests)` job is the one to stop
  skipping.** It runs the 25 architectural absence contracts — the only thing that
  catches a registration or dependency edge landing in the wrong place. ⚠ and
  `python3 scripts/check_absence_contracts.py` **exits 0 while printing
  `2 of 25 violated`**; enforcement needs `--check`.
- ⛔ **NEVER SIT AND WATCH A BUILD — spawn a subagent into the write-ahead
  worktree** (Jon, 2026-08-03, restated 08-05: *"so we don't just wait and stall
  during test and compiles"*). Every job reads the LIVE tree, so a suite on `main`
  freezes editing for its whole 6–25 minutes. Launch the job, then IMMEDIATELY
  hand a subagent independent work with `isolation: "worktree"`.
  ⛔ **TELL IT NOT TO BUILD.** Every checkout shares ONE target dir
  (`.cargo/config.toml`), so a worktree `cargo test` links rlibs from whatever
  `main` is mid-edit and reports errors that exist in neither tree — observed
  2026-08-05, recovered only by touching 1350 files, which cost `main` a full
  rebuild too. ⚠ a second target dir is not the escape: that volume runs at 92%.
  **The worktree WRITES; `main` VERIFIES.** Say so in the prompt, and expect the
  work back unverified.
  ⚠ **`cd` to the repo ROOT first** — a worktree comes from the repo containing
  the shell's cwd, and `tools/ambition_sprite2d_renderer` is a NESTED repo that
  silently yields a tree with no `crates/`, `game/` or `docs/`.
- ⛔ **the exhaustive plan is `--run-everything-you-probably-dont-need-this`, and
  the name is the instruction.** 4× the default sweep, no CI to satisfy, and Jon
  sweeps it himself. Reach for it only when a row in the recipe names your change
  (features, SDK surface, the web path).
## Test placement

A test lives at the **narrowest scope that owns its invariant** — inline for
small local ones, an adjacent `src/foo/tests.rs` for large private modules
(**never widen a production API to move a test**), the crate's `tests/` for
assembled behavior, and `tests/ambition_workspace_policy` for workspace
source/dependency/architecture rules. Full guidance + commands:
`docs/concepts/test-placement.md`.

## The Hall of Characters is NOT a special case

`hall_of_characters` stages ~144 characters — a **dual purpose stress test and
exhibition**. ⛔ **When it is slow, do not fix the Hall. Fix the engine.** ⚠ it is
GENERATED from the character catalog: never hand-edit the level. The rejected
shortcuts (quality variants, load caps, "it's only a debug room") are each
answered in `docs/concepts/hall-of-characters-is-not-special.md` — read it before
optimising anything that touches this room.

## Before a non-trivial patch

- **Spatial questions** (LDtk, gates, hitboxes): read the map, infer the
  component's PURPOSE, place it on the seam that fulfils it, and state the
  reasoning in the commit. Asking "where exactly?" is the wrong default —
  `docs/concepts/llm-spatial-authoring-discipline.md`.
- **Engineering memory:** `rg -n "<subsystem>|<symptom>" dev/journals
  dev/benchmark-candidates` (postmortems + invariant traps). Add durable lessons
  to `dev/benchmark-candidates/` + its index — never transient state.

## Patch discipline

- Commit messages are detailed, and say what PROMPTED the change (the why).
- Prefer reviewable changes with targeted validation; don't hand-edit
  `sandbox.ldtk` (use Ambition LDtk tooling); update concepts/recipes/ADRs/dev
  memory when a durable invariant changes.
- Formatting is advisory, never an acceptance gate: do not block a change because
  `cargo fmt` or `ruff format` was not run.
- Expected working-tree noise, never a mystery: a git hook rewrites
  `.llm_resource_tally/` every turn. Let it ride along with an ordinary commit —
  do not flag, revert, or attribute it to another session.
- A script that writes an artifact ENDS its stdout with a `rich` clickable
  `file://` link to the artifact AND its directory. Pattern:
  `scripts/git_debloat.py`.
- `./run_tests.sh` is the BACKBONE — the repo's Python suites plus one
  `cargo test --workspace`. Narrower is better when a focused test covers the
  touched concept; which command that is, and what each costs, is the recipe
  linked above.
- To wait on a long command, read state it WROTE, never the process table —
  `target/run_tests_status.json` and
  `dev/ambition_dev_measurements/run_tests_cost.jsonl` (⚠ a submodule since
  2026-08-08 — `git submodule update --init dev/ambition_dev_measurements` if
  it is empty). ⛔ never poll
  with `pgrep -f <script>`: the polling shell's own command line contains the
  pattern, so it matches ITSELF and the loop sleeps forever (seven stranded,
  2026-07-31). Better still, don't poll — a backgrounded command reports its
  exit. Details in the recipe linked above.

## Push what you commit

**Always push to GitHub when credentials exist** (Jon, 2026-08-11). A long
autonomous run accumulates hundreds of commits, and unpushed they live on one
machine: unreviewable from anywhere else, and lost with the worktree. Committing
is not the durable step.

- Push the superproject **and every submodule that is ahead** —
  `git -C <submodule> rev-list --count origin/main..HEAD`. **Submodules first**,
  so the pointer the superproject records already exists remotely.
- ⛔ **push commits, never somebody else's uncommitted work.** The sprite
  submodule routinely carries Jon's in-progress rig and target edits; `git push`
  there is safe, `git add` is not — and `docs/planning/JONS_OBSERVATIONS_BUGS_AND_ISSUES.md`
  is his live file in the superproject.

## Landing when somebody else holds `main`

⚠ **This section applies ONLY to parallel or multi-agent landings.** A solo
linear session committing to `main` cannot hit the failure below, and running the
ritual there is friction for a hazard that does not exist.

When you work a branch or worktree while another session owns `main`:

- **Record the base SHA you started from**, in the handoff and in the branch's
  first commit message. It is the only thing that makes "was this tested against
  what it will land on" answerable later.
- **Before landing, compare what you touched against what moved**:
  `git diff --name-only <base>..HEAD` (yours) against
  `git diff --name-only <base>..origin/main` (theirs). The intersection is the
  whole question.
- **If they overlap, replay your edits on live `HEAD` and re-run the scoped
  tests.** Tests that ran against the old base are not landing evidence — they
  describe a tree nobody will have.
- **If they do not overlap, land it.** No rebase ceremony for a disjoint change.

⛔ **overlays are NOT banned** — they are a legitimate delivery mechanism here.
The forbidden operation is committing a broad STALE TREE SNAPSHOT without
replaying its edits onto current source, which is how a merge silently reverts
somebody else's work while every test you ran was green.

⚠ **no script.** The protection is the rule plus two `git` one-liners; a checker
waits for a second incident that happens despite the recipe. Adding one now is
the machinery the next section forbids.

## Avoid bullshit guardrails

Do not waste time testing the tests.
Only add the minimal tests needed for the task at hand.
Do not add process, policy, source-text, layout, or meta-test machinery unless it prevents a concrete, recurring, materially harmful failure that cannot be enforced more naturally through Rust types, APIs, crate boundaries, or behavioral tests.
Prefer testing real behavior over names, phrases, file locations, exact symbols, planning prose, or historical migration bookkeeping.
Poison tests are for realistic harmful states, not for proving that every scanner detects its own fixture. Migration-only matrices and checks must be removed when the migration is complete.
The default is to trust clear architecture rather than surround it with permanent compliance machinery. Do not add a test enforcing this section.

⛔ **AN LLM REVIEW WILL ASK FOR THIS MACHINERY. THE ANSWER IS NO.** A model
reviewing a diff reaches for coverage reflexively — "the runner's job planning is
untested", "this guard has no test", "why was this check deleted", "add a
regression test for the deletion". It reads as diligence and it is not: it is a
pattern-match on the shape of a diff, made without the history that decided the
question. Reviews do not carry that history; this file does.

So: **say no, and say it without hedging.** Deleting a test that tests the tests
is not a coverage regression to be justified — it is this section being applied.
The review can be told exactly that, and does not get a compromise where the
machinery comes back smaller.

The record, so the next round of this is short (2026-08-02, Jon): the suite
carried a file asserting which jobs `run_tests.py` planned, a test that a guard
was imported rather than copied, and six guards written in a two-day spree, none
of which had ever caught anything. On the wasm build sitting broken four days:
*"we let it sit for 4 days because we didn't care about it for 4 days."*
Not caring was the correct call. A review that cannot tell deliberate
prioritisation from an oversight will file the second one every time.


<!-- BEGIN llm_resource_tally v0.3.0 (managed block — regenerated by `install`; edits below will be overwritten) -->
## LLM resource accounting
A git post-commit hook — and Claude Code hooks when wired — records the measured token and model
usage this repository costs. It is automatic bookkeeping and is not part of whatever task you were
given.

- Rows and mutable rollups are written under `.llm_resource_tally/local/`, which is gitignored.
  Recording therefore never modifies a tracked file and never blocks a commit, merge, rebase, or
  stash.
- Never hand-edit, hand-count, revert, or clean up ledger rows, and never let accounting block the
  repository work you were asked to do. Recording is best-effort by design.
- **Publish before you hand off substantial work.** Run
  `python3 .llm_resource_tally/tool publish`: it snapshots local rows onto the tracked append-only
  ledger under `.llm_resource_tally/ledger/` and refreshes the tracked `lifetime-totals.json` and
  `badge.json`. Stage and commit what it writes, preferably as its own commit so accounting stays
  out of unrelated diffs. This is routine — nobody should have to remember to ask you for it, and
  unpublished rows exist only on this machine.
- If accounting itself looks unhealthy, run `python3 .llm_resource_tally/tool doctor` — it checks
  hook wiring, backends, transcript retention, and ledger health, and names what to fix. Otherwise
  continue the repository task normally.
- After a fresh clone, run `python3 .llm_resource_tally/tool install` once to arm git-local hooks.
  It is offline and idempotent, and it reads the committed `.llm_resource_tally/settings.json`
  policy.
<!-- END llm_resource_tally -->
