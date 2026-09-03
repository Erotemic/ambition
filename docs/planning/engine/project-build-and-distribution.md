# Project build, test iteration and distribution

**State:** OPEN — developer iteration has current measured pressure; external
project/release packaging remains later/product-driven.

## Goal

Make the supported path from checkout to tested/shippable game explicit and
resource-aware:

```text
clone/create
 -> configure providers/capabilities
 -> prepare/generate/validate content
 -> edit/build
 -> run targeted tests
 -> run pre-push/supported-composition gates
 -> package
 -> distribute/update
```

Build/test iteration is an engine productivity concern independently of runtime
frame performance. It is also part of the Godot-class engine-product bar: an
external project must have a reliable noninteractive path from clean checkout to
validated target artifact. A graphical export dialog is not required; reproducible
CLI/CI operation is preferable for agent-first development.

See [`godot-class-2d-capability.md`](godot-class-2d-capability.md).

## Current empirical lessons

### Dev profile choices should be measured, not inherited

Recent probes found several `opt-level = 0` development exceptions bought only
about **1–2%** on representative one-file rebuilds while the measured runtime
changed from about **5.12 ms to 2.96 ms** when those dependencies returned to
`opt-level = 1`.

That does not establish one universal profile for every crate. It does establish
that large runtime/debug penalties need a measured rebuild payoff.

### The wasm CHECK and LINK, both run 2026-09-03 — green, and the LINK now has a price

`scripts/run_tests.py` plans the web build CHECK in the default plan but the
release LINK only `if not only and everything`, with the comment that the
CHECK's price is worth paying and *"the LINK's price (a release wasm artifact) is
not"*. That judgement was made without the LINK's number. It has one now, from
the calculex host (6 vCPU, no GPU, `target/` bind-mounted to local ext4):

| job | result | cost |
|---|---|---|
| `cargo check … --target wasm32-unknown-unknown --features web_served_assets` | ✔ green | **42.6 s**, dependencies warm |
| `cargo build --lib --release … --features web` | ✔ green | **18 m 01 s** cold, full wasm32 release graph |

⭐ **Both are GREEN at HEAD**, which is the part nobody could previously assert:
the LINK is exhaustive-plan-only, so on a normal run it does not execute
anywhere.

⚠ **The artifact is 238 MB and that is NOT a shipped size.** `ambition_app.wasm`
straight out of `cargo build` is the PRE-`wasm-bindgen` intermediate;
`scripts/setup/web_prereq.sh` exists precisely to install the matching
`wasm-bindgen-cli` that runs next. Quoting 238 MB as "the web build" would be
wrong in the direction that stops work, so it is recorded here with its stage
named. What a bindgen'd, size-optimised artifact measures is unknown and is the
obvious next measurement — this host can take it.

### The web persona boots on a machine with no GPU (2026-09-03)

The third exhaustive-plan-only web job, `web persona BOOTS [visible_web_base,
native]`, also runs here: **16 m 01 s** to build, and it **SURVIVES startup** —
initial and active route `ambition_launcher`, 23 UI nodes, 10 UI texts, 0
sprites, 2 cameras, simulation host `Rollback`.

⭐ That is the answer to *"does the web composition come up on a host with no
GPU"*, and it is yes. The calculex host has no `/dev/dri` and no display.
⚠ **It does not prove the browser.** The job runs NATIVELY by design, so it
compiles the `not(wasm32)` branch; the wasm-only path remains unexercised, which
is the standing entry in
[`../../recipes/checks-that-did-not-run.md`](../../recipes/checks-that-did-not-run.md).

⇒ For the plan-shape question this doc owns: 18 minutes is a real price and an
honest argument for keeping the LINK out of the default plan. It is not an
argument for it never running — it had not run, and 18 minutes on an idle
no-GPU host is cheaper than discovering a broken build target by accident.

### Optimized incremental builds are not currently a default solution

The repository has seen invalid/corrupt link behavior in the affected optimized
incremental workflow. Launch tooling disables that path. Do not re-enable it as a
speed tweak without a reproducible correctness test on the actual build path.

### Test resource shape matters

A large app integration suite can exhaust machine memory at default test
concurrency while passing with a bounded thread count. The correct response is a
resource-aware lane/preset, not treating parallelism as always beneficial.

### Feature combinations need explicit proof

The first broad combination sweeps found real compile/configuration gaps that
single default builds could not expose. Supported product/capability combinations
should have a bounded matrix rather than relying on accidental workspace feature
unification.

### Clean checkout/generated assets are part of the contract

A test/build that succeeds only because an ignored/generated artifact is already
present locally is not a reproducible project path. Generated-art freshness and
source-to-output cache keys are distinct concerns; output digests cannot detect a
correctly cached output whose source dependency was omitted from the key.

## Current program areas

### B1 — development profile policy

Keep a small measured table for dependencies/crates whose dev optimization level
materially affects runtime/tooling. Change an override only with both:

- representative edit/rebuild cost;
- representative runtime/tool cost.

Avoid profile folklore copied from an old bottleneck.

### B2 — test lanes and concurrency

Maintain explicit tiers:

- touched-crate/narrow tests while editing;
- product integration tests for changed cross-crate behavior;
- pre-push workspace/library or policy gates where appropriate;
- resource-bounded presets for large monolithic test binaries.

Do not make every turn run the whole workspace merely because it is comprehensive.

### B3 — supported feature/product matrix

Define the combinations the repository claims to support—headless/rendered,
rollback/nonrollback where applicable, relevant capability subsets, key platform
personas—and compile/test those combinations deliberately.

A broad matrix is useful only if it maps to real products/hosts. Do not enumerate
the power set of Cargo features.

⚠ MEASURED 2026-09-02: three programs answer to "the smash tests". Per-crate
default features (no device layer), the gate's `nextest --workspace` feature
UNION (`visible` → `input` on, so seats read devices), and
`-p ambition_demo_smash_app --features visible` alone — under which
`build_demo_app` panics in `bind_game_assets` on a missing
`Assets<TextureAtlasLayout>` (the MinimalPlugins foundation never registers it;
the union happens to). The full lane only `cargo check`s that third cell, so
nothing claims it runs; the product binary uses `build_windowed_demo_app` and
is fine. The first two disagreed about one test until `f4a757328`.

✔ **DECIDED 2026-09-02: THE CELL IS UNSUPPORTED, and it is not one registration
away.** The atlas was the first missing resource, not the only one. Fixing each
revealed the next: `Assets<TextureAtlasLayout>` (`ImagePlugin` does not supply it
— it is `TextureAtlasPlugin`'s), then `GizmoConfigStore` for `draw_debug_viz`,
then `Assets<Mesh>` + `Assets<HitFlashMaterial>` for the hit-flash pass, which
belong to `MeshPlugin` and in this repo are only ever hand-registered inside
tests. `build_demo_app` has no renderer whatever features are on; `visible` adds
presentation systems whose parameters are render-stack resources. A chain that
regrows each time it is cut is a composition asking for a renderer, so it is
declared unsupported in `ambition_demo_smash_app`'s feature docs with that
evidence, and the supported paths are named there (the workspace union, and
`build_windowed_demo_app`).

⭐ **TWO REAL DEFECTS CAME OUT OF IT AND ARE FIXED AT THE SOURCE**, both worth
having on their own account rather than for this cell:
`PlatformerAssetsPlugin` now registers the `Assets<TextureAtlasLayout>` its own
`Startup` system consumes (guarded on the resource — `init_asset` is NOT
idempotent, so an unguarded call would replace a live store and drop every handle
in it), and `draw_debug_viz` now carries the
`run_if(resource_exists::<GizmoConfigStore>)` guard `avatar::trail.rs` already
had. The cell went from panicking in `build_demo_app` to running, with 14 of 39
passing.

⚠ **AND THE FAILURE NAMES NOTHING.** Bevy reports these as
`Parameter <Enable the debug feature to see the name> failed validation: Resource
does not exist`. `RUST_BACKTRACE=1` and the `run_unsafe<fn(..)>` frame is the
only place the system's parameter list appears; without that, this row is
unactionable.

### B4 — generated-content/bootstrap contract

> ⛔ **A FRESH CLONE ON A USED MACHINE IS NOT A FRESH MACHINE — measured
> 2026-09-03 by actually cloning this repo into a scratch directory.** The
> difference is the per-machine venv store, and it is keyed inconsistently:
> `ambition_tool_venv_dir` takes `basename` of what it is given
> (`scripts/setup/python_tools.sh:78`), so
>
> - the six TOOL projects key on `ambition_music_renderer`,
>   `ambition_ldtk_tools` … — the same key from any checkout, so a new clone
>   **inherits them**;
> - the `scripts/` environment keys on the REPO ROOT's basename, so a clone in
>   `freshclone/` gets a different key and **has none**.
>
> ⇒ `python_tools.sh --verify` in the clone therefore reports exactly one
> problem — *"no interpreter for the scripts/ environment"* — and passes the six
> tool venvs it did not build. ⚠ **A genuinely new machine would fail all seven**,
> so this check's output on a clone is a partial truth shaped by the host, not by
> the clone. It is also two checkouts with the same directory name sharing one
> `scripts/` venv, which is a surprise waiting for anyone who keeps
> `ambition/` and `ambition-2/`.
>
> ⭐ Two facts worth having beside that: **a clone is 75 MB** (the bulk is
> submodules and git-ignored generated content, which is why the bootstrap
> contract matters at all), and all four entry points — `run_developer_setup.sh`,
> `run_headless.sh`, `run_tests.sh`, `scripts/setup/audio_libraries.sh` — are
> present and executable with the submodule directories still empty. The
> orchestrator's `--help` works from a bare clone and names the three
> status/verify commands, which is the right shape: a new machine can ASK what
> it is missing before it installs anything.


> ⭐ **THE ASK ITSELF, RUN END TO END 2026-09-03 AFTER FIVE CARVES:
> `./run_headless.sh --ticks 600` → `headless run completed: 600 ticks`, exit 0.**
> Jon's standing ask is that a fresh clone reaches a runnable game; this is the
> nearest thing to it that a box with no display can execute, and it passes on a
> tree where `ambition_held_items`, `ambition_body_seed`, `ambition_match`,
> `ambition_encounter_features` and `ambition_abilities` all left the actor
> kernel the same day. It validates the sandbox world first, then steps the sim
> shell, and it builds in RELEASE (`target/release/headless`) — so this also
> exercises the optimized profile that the dev-profile discussion below is
> about.
>
> ⚠ **What it does NOT prove**, so nobody quotes it further than it goes: this
> tree is not a fresh clone. It has generated content, fetched fonts and the
> sampled instrument libraries already in place, which are exactly the
> prerequisites the rows below record as the things a clone lacks. A real
> fresh-clone proof needs a clone; this proves the SIMULATION composes and runs
> after the carves, which is the half a carve campaign puts at risk.
>
> Incidentally measured from its own census line: **661 systems in `Update`**.

> ⭐ **AND ASK #2 HAS THE MATCHING END-TO-END EVIDENCE, run against real shipped
> cues rather than fixtures (2026-09-03).** The per-library preflight gate
> (`ambition_music_renderer` `8b10c5a`, pointer bumped here the same day) was
> exercised both ways:
>
> - **No false positives on a healthy box.** Three shipped cues from
>   `scores/active` — `a_possible_morning`, `aether_severance`,
>   `argand_overdrive` — all report `unresolvable=[]`. That is the arm that
>   matters for adoption: a gate that cries wolf gets an env var set
>   permanently, which is how the General-MIDI fallback becomes the default
>   again by another road.
> - **It names the missing family when there is one.** Rewriting one real cue's
>   `library_ref` to `freepats.a_library_no_machine_has` in memory makes the gate
>   return exactly that reference — not a generic "something is missing", which
>   would send a reader back to a 38 GB install they mostly already have.
>
> ⇒ So both standing asks now have a run on this box rather than a unit test:
> ask #1 through `./run_headless.sh --ticks 600`, ask #2 through the gate's two
> arms. ⚠ Same caveat as above — this box HAS the libraries, so what is proven
> is that the gate discriminates, not that a fresh machine acquires them.


A clean checkout should have an explicit path to produce every required generated
artifact or obtain it from the intended cache/submodule. Cache keys must include
all source dependencies that affect output.

⚠ **Measured on a genuinely fresh clone, 2026-09-02, and this row had three
live violations — two now closed.** They shared one shape: the producing command
EXISTED and nothing called it, so the artifact was missing on every machine
except the one that had run the command by hand.

⚠ **A FOURTH TURNED UP ON 2026-09-03 AND IT IS A DIFFERENT SHAPE**, so the
enumeration above should not be read as closed. `Pillow` was missing from the
repo venv: `scripts/tests/test_asset_writes_do_not_follow_worktree_symlinks.py`
loads `scripts/generate_visual_quality_variants.py` to reach one guard function,
that script's `from PIL import Image` is at MODULE scope, and three tests went
red at collection with `ModuleNotFoundError: No module named 'PIL'` — from a
file whose name is about symlinks. ⛔ Here no producing command was missing;
the SUITE acquired a dependency and setup never learned about it, which is the
same fresh-clone failure by a different road and would not be found by looking
for uncalled commands. ⇒ Fixed in `scripts/setup/python_tools.sh`, and the rest
of that class was swept rather than guessed: parsing every module-scope import
under `scripts/` leaves three unresolved and NONE reachable from a test
(`networkx`, `scriptconfig`, and `ambition_sprite2d_renderer`, which arrives via
a `sys.path` insert). The sweep method is recorded beside the install list so
the next added dependency re-runs it.

⛔ **AND THE VERIFIER COULD NOT HAVE CAUGHT IT EITHER — found 2026-09-03 by
asking what `--verify` actually checks.** `python_tools.sh --verify` walked the
six per-tool authoring venvs, asking each for one import, and never touched the
repo-root `scripts/` environment where the `pillow` failure lived. The
seven-module import loop existed but ran only on the INSTALL path, so a machine
whose scripts env had rotted answered *"existing tool environments are usable"*
— true of six environments, and silent about the sick one. ⇒ Fixed: one list
(`scripts_env_modules`), read by both the install loop and a new
`verify_scripts_environment` on the `--verify` path. ⚠ Poisoned before being
believed, because a green that would be green anyway is the same defect again.

⭐ **The generalisation, which is the part worth keeping:** a per-component
health check answers about the components it enumerates, and a fresh-clone
failure lands wherever the enumeration does not reach. Both B4 violations found
on 2026-09-03 have that shape — the suite acquired a dependency setup never
learned about, and the verifier enumerated the wrong set — so when adding a
check here, ask what it CANNOT see before trusting it.

- ✔ **Bundled UI fonts.** `ambition_render`'s typography test `include_bytes!`s
  three faces from a git-ignored directory, so their absence is not a missing
  picture at runtime — `cargo check --all-targets` exits 101, and TWO gate jobs
  run exactly that command. `scripts/grab_font_assets.py` had been in the tree
  the whole time; `scripts/setup/generated_content.sh` now runs it.
- ✔ **Sampled instrument libraries.** Opt-in behind a flag, so a default clone
  rendered the whole catalogue through General MIDI and reported success —
  indistinguishable downstream from the real cues. Now installed by default, and
  the renderer refuses rather than shipping stand-ins.

  ⛔ **BUT THE REFUSAL WAS ALL-OR-NOTHING, AND THE LIKELIER FAILURE IS ONE
  MISSING FAMILY.** Measured 2026-09-03. `_sampled_libraries_installed()` asks
  whether the machine has ANY sampled libraries; a box that has most of them and
  is missing one passes it, and only the cues naming that family come out in the
  wrong instrument — the rest are correct, so the run looks healthy.
  ⚠ **And the catalogue cannot defend itself: all 247 sfz instrument backends in
  `scores/active` are `optional: True`** (231 explicitly, 16 by
  `_is_optional_instrument_backend`'s default), and only 2 of 75 scores set
  `render.strict_backends`. The renderer's own guard is
  `(wants_sfizz and not optional) or strict_backends`, which is FALSE for every
  instrument Ambition ships: each warns once to stderr, falls back, exit 0.

  ⭐ **Closed by a preflight gate that ignores `optional:` deliberately**, since
  honouring it would make the gate dead code on the whole catalogue. It costs a
  healthy machine nothing, which is what makes a gate legitimate rather than
  another warning: resolving all 247 references through the renderer's own
  resolver on this box gave **247 resolved, 0 unresolved**, `salamander_grand`
  among them. Memoised, because one reference costs 0.36–5.54 s of globbing and
  the 247 backends name only 68 distinct references.

  ⚠ Two things this does NOT establish. It runs on cues that will actually
  render, so a fully-cached tree is not re-checked — correct for cost, but it
  means a machine can lose a library after its last render and not hear about it
  until something changes. And `audio_libraries.sh`'s header says the cues name
  *"54 distinct library references"*; the count over `scores/active` is 68. The
  two may be different scopes (shipped registry versus every active score), so
  the script's number is left alone rather than corrected on a guess.
- ▢ **The `.ipfs` sidecars still have no hydration command**, which is the
  remaining instance of exactly this class: six git-ignored payload directories
  whose only restore path is a manual `ipfs get`. ⛔ Do NOT fold this into
  feature work — `dev/journals/code_smells.md` (2026-07-19) records it as
  backlog-only because Jon owns asset distribution. Noted here so B4's exit
  criterion is not read as met while it is outstanding. ⚠ The fonts sidecar that
  entry names (`.../assets/fonts/bundled.ipfs`) is itself absent now, so that
  payload has lost even its CID.
  ⭐ **AND NOW THE CONSEQUENCE IS MEASURED, which the row was missing.** It is
  not only that a payload cannot be restored: on a fresh clone
  **`package_asset_guard.py compose` FAILS**, so no shippable asset tree can be
  produced at all. Measured 2026-09-03, `--profile steamdeck --materialize link`:

  ```text
  asset contract failed:
  runtime-declared assets are absent from the composed desktop source roots:
    - vanity_card/frame_00.png … frame_08.png   (9 files)
        declared by manifest:data/vanity_card.ron:11-22:path
  ```

  ⚠ **Exactly nine files, all one family, and nothing else is missing** — so
  this is the whole gap between a fresh clone and a composable package, not a
  sample of a larger one. The sidecar `assets/vanity_card.ipfs` declares that
  family (`rel_path: vanity_card`, 51 items, 12.67 MiB) and nothing fetches it.
  ⛔ Still backlog-only and still not to be folded into feature work; the number
  is here so the size of the gap is known before Jon decides, rather than
  discovered by whoever first tries to cut a build.
  ⚠ Do not confuse this with the vanity card `scripts/regen/sprites.sh` DOES
  build: that exporter writes `vanity_card_made_this_meme`, a different,
  TRACKED manifest. The missing family is `data/vanity_card.ron`, and running
  the regen does not produce it.

### B5 — platform prerequisites

Desktop remains the primary local path. Android/web/cross targets should use
repository-owned prerequisite/setup scripts and clearly distinguish:

- code failure;
- unsupported target;
- missing external toolchain/prerequisite.

Do not report an absent NDK/GPU/display as proof that the target's code is broken.

> **⚠ TESTED ON A MACHINE WITH ALL THREE ABSENCES, `17de0f816` (2026-09-02) — the
> distinction holds in two places and FAILS in a third.** The calculex VM has no
> GPU (`/dev/dri` absent), no display (`XDG_SESSION_TYPE=tty`), and no
> `ANDROID_NDK_HOME`:
>
> - ✔ **Web.** With `wasm32-unknown-unknown` absent, the gate PLANS no web job and
>   says so: *"SKIPPING the web build CHECK — the wasm32-unknown-unknown target is
>   not installed … The web build is UNCHECKED in this run."* That is exactly the
>   distinction B5 asks for, and it is recent — the footer used to claim the CHECK
>   had run regardless.
> - ✔ **GPU/display.** A Cpu adapter seeds `Potato` and says it is doing so;
>   `[census] phases_trust` reports `trustworthy=no_render_backend` and states
>   that the phase split is still usable. The absence is named, not inferred.
> - ⛔ **Fonts — THE ONE THAT FAILS.** `cargo check --workspace --all-targets` on
>   a fresh checkout dies with `error: couldn't read
>   …/assets/fonts/bundled/JetBrainsMono-Regular.ttf: No such file or directory`
>   and then `could not compile ambition_render (test "typography")`. The
>   directory is GIT-IGNORED and fetched by `scripts/grab_font_assets.py`, so this
>   is a missing PREREQUISITE reported as a compile error in a test target —
>   indistinguishable from code failure unless you already know the file is not
>   tracked. It cost a full workspace lane before I recognised it.
>
>   ⛔ **AND THE TEST IS NOT THE THING TO FIX**, which is worth saying because it
>   is the obvious move. `tests/typography.rs` uses `include_bytes!` precisely
>   because the APP does: `embed_core_assets!`
>   (`crates/ambition_asset_manager/src/platformer_assets/embedded.rs`) embeds the
>   same three faces the same way. Converting the test to a runtime read would
>   stop it mirroring the path the game actually uses, which is the only reason
>   the test is worth having.
>
>   ⚠ Scope, measured rather than assumed: the app's embedding is
>   `#[cfg(feature = "static_core_assets")]` — NOT a default feature, and denied
>   by the gate's feature scan — so an ordinary game build never reads these
>   files. The typography test's `include_bytes!` is unconditional, which is why
>   `--all-targets` is exactly where a fresh checkout meets this and `--lib` never
>   does.
>
> ⇒ **So B5's rule is honoured where somebody has been bitten and absent where
> nobody has.** The remedy is the same one the web job already uses: fail with the
> prerequisite named and the fetch command quoted, rather than with a read error
> from whichever target happens to open the file first.

> **⛔ A FOURTH ABSENCE, AND THE WORST-BEHAVED: DISK. Hit 2026-09-03 running the
> exhaustive plan — `/` reached 100% (290G used, ~260K free) partway through.**
>
> It is B5's class exactly — a missing prerequisite, not a code failure — but it
> is the one absence that cannot name itself, and the reason is mechanical: tool
> output goes through `/tmp`, so once the volume is full commands fail with
> ENOSPC and **lose their own output**. A `git commit` died with exit 128 and no
> message at all. Nothing in the failure says "disk"; it just stops making sense.
> ⇒ **When a lane starts failing incoherently, `df -h /` before believing the
> failure.**
>
> ⚠ **AND `df` ON THE WORKING DIRECTORY WILL TELL YOU IT IS FINE.** `target/` is
> 182G and bind-mounted from `~/.cache/ambition-targets`, which lives on `/`; the
> repository is on a different volume with 220G free. The obvious check looks at
> the wrong filesystem — `scripts/setup/target_bindmount.sh --status` names the
> pair, and that is the thing to read.
>
> Where it sits, measured the same day: `target/debug` 137G (`incremental` 31G of
> it), `target/profiling` 19G, `target/outlander` 18G, `target/release` 9.8G.
> ⚠ **Attribution is NOT established** — the cache is shared across sessions and
> there was no before-reading — so the honest claim is that the plan is the
> largest disk consumer in the repo (49 jobs, many of them distinct feature
> combinations that each get their own units, plus `fixtures/minimal_game/target/`
> as a second target directory entirely), not that this run filled it.
>
> ⛔ **CORRECTION, SAME DAY: THE NUMBER EXISTS AND SO DOES A GUARD.** I wrote
> that the plan's disk cost was "a NUMBER this page cannot yet state". The repo
> states it: `scripts/check_disk_headroom.py` sets `MIN_FREE_GB = 40.0`, and its
> comment names this exact failure — *"refuse BEFORE a job dies of ENOSPC and
> reports it as a compile error."* `run_tests.py` calls it, and it refused a
> 39.1 GB tree with the remedy quoted. I had described an unguarded class that
> was already guarded, which is what looking for a gap instead of looking for
> the guard gets you.
>
> ⇒ **The real gap is narrower and worse: the check runs TWICE — once before the
> first job, once after the last — and never between.** A 68-minute suite that
> starts above the floor can exhaust the disk halfway and die incoherently. That
> is the shape of three of the exhaustive run's seven failures (`compile-cost
> ratchet`, `outlander`, `capability demo`), one of them a bare
> `error: linking with clang failed` whose reason line never reached the log.
> The post-run half already prints `disk: N GB free (±M this run)` and warns
> below the floor, so the machinery to check mid-run exists; only the call site
> is missing.
>
> Measured 2026-09-03: `target/debug/deps` alone is **141 GB**; `--rust` spends
> ~5 GB; one `cargo test --workspace` takes **14 GB in under three minutes**;
> the 49-job plan runs **68 minutes** and exhausted a 290 GB volume mid-run.
> Every feature job builds its own variant of the graph, cargo never prunes the
> last one, and five crates were carved out of the actor monolith that day —
> the decomposition campaign pays for itself in disk.

### B6 — packaging/distribution

Keep packaging/release work product-driven, but Engine 1.0 needs at least one
complete external-project path rather than leaving packaging indefinitely
abstract. The eventual external-project layout, SDK templates, update mechanism
and broad release-target policy should follow the public SDK and real distribution
customers.

Minimum competitive proof:

- one clean external/minimal project can select capabilities/providers without
  workspace-private wiring;
- preparation/build/test/package are noninteractive and scriptable;
- at least one desktop release artifact is reproducible from documented inputs;
- web/Android/headless profiles state their prerequisites and failure modes;
- target packaging uses the same logical asset/content identities as development
  rather than a target-specific shadow application;
- CI can distinguish source failure, missing prerequisite, unsupported profile and
  packaging failure.

One-click GUI export is not an acceptance requirement.

### B7 — agent iteration budget

Track the wall-clock and resource shape of the common agent loop: inspect/edit,
compile, targeted test, preparation/generation, representative run, and package
when required. Optimize the dominant measured step rather than applying generic
Cargo folklore.

The goal is not a universal fixed time budget across machines. The goal is enough
telemetry that an agent can choose a narrow fast path and know when a change has
accidentally expanded the iteration surface.

## Candidate tool shape

Build orchestration belongs primarily in repository/tooling surfaces rather than
a giant runtime project-manager service. Runtime package/asset manifests should
remain narrow data contracts consumed by engine domains.

Tools should expose noninteractive, inspectable commands suitable for humans and
agents: plan/check/build/test/package with clear artifact/cache ownership.

## Acceptance

- a fresh checkout can follow a documented bootstrap/build/test path without
  relying on accidental local ignored files;
- representative edit/rebuild timing is known for any nondefault dev-profile
  exception retained for speed;
- large test suites have a bounded-memory invocation that is part of normal
  workflow;
- supported capability/platform combinations compile in deliberate gates;
- a platform prerequisite failure is distinguishable from a source/build defect;
- packaging work does not introduce a second runtime composition model;
- a clean external/minimal consumer can produce at least one release artifact
  through supported noninteractive tooling;
- common agent edit/build/test/preparation loops have enough measurement to avoid
  optimizing the wrong phase.

## Open design questions — deliberately unresolved

- eventual external project layout and template format;
- which generated artifacts should be checked in versus produced/fetched;
- web as first-class release target versus later experiment;
- third-party capability/plugin version locking;
- required asset hot-reload guarantees for agent iteration;
- final split between Cargo features and runtime/provider configuration.
