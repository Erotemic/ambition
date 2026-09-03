# Modal CLI: collapsing the probe binaries

> **Verified against `cd1a14ae9` (2026-09-03).** Every size, symbol count and
> feature constraint below was measured on this checkout, not estimated. The one
> number that is a PREDICTION is labelled as one.

Jon's ruling 2026-09-03: build a modal CLI with `clap` and combine related
binaries. Standalone executables stay standalone for the game and the demos —
`ambition_game_bin`, `sanic_demo`, `smash_demo`, `mary_o_demo`,
`twintrack_demo`. This page is the plan for everything else.

## Why: the measurement that motivates it

Nine binaries in `game/ambition_demo_smash_app/src/bin/` are 99.88% the same
program.

```text
binary            debug size    defined symbols
ladder_probe            634M    502,262
roll_probe              633M    501,899
ladder_rig              634M    502,053
capture_probe           635M    502,165
stage_diagram           474M    267,675
-------------------------------------------------
union of the five               503,084
sum if kept separate          2,276,054
```

⭐ **The union is 822 symbols larger than the LARGEST single binary — 0.16%.**
`ladder_probe` and `roll_probe` share 501,778 of their ~502,000 defined symbols;
each contributes a few hundred of its own (484 and 121). Section sizes agree:
`.debug_str` 182 MB, `.debug_info` 76 MB and `.debug_line` 41 MB are the same
size in every one of them, `stage_diagram` included, whose `.text` is half the
others'. This is one program with nine `main`s.

⇒ All nine are **~5.6 GB** on disk today. ⚠ **PREDICTED ~0.7 GB collapsed —
this figure is a symbol-union proxy and has not been built.** Confirming it is
Phase 0.

⛔ **The write volume is not the whole win and probably not the main one.** Nine
links become one. On a full `--all-targets` build of this crate the linker runs
nine times over ~500 K symbols each; that cost is paid per build, not per byte
saved.

## Scope: collapse WITHIN a crate, never across

**In scope — one modal binary, `smash_tool`,** replacing the nine bins in
`ambition_demo_smash_app`: `capture_probe`, `ladder_probe`, `ladder_rig`,
`match_diagram`, `match_report`, `match_shots`, `roll_probe`,
`select_walkthrough`, `stage_diagram`.

⛔ **OUT OF SCOPE, and this is a decision rather than an omission: do NOT
collapse the `capture_*` family across crates.** They look like the obvious
group — `capture_scene` (952 M, `ambition_app_tools`), `capture_twintrack`
(726 M), `capture_mary_o` (722 M), `capture_sanic` (710 M), `capture_probe`
(634 M) — but each lives in the demo crate it photographs. One binary covering
them would have to depend on all four demo crates at once, which
* inflates that binary's dependency closure to the union of every demo, and
* couples demos that today do not know about each other.

⇒ The repository already has a ratchet against exactly this shape
(`scripts/check_absence_contracts.py`, `capability-footprint-may-not-grow`), and
its whole purpose is to notice a consumer linking crates it never asked for.
A cross-crate capture tool would be that, deliberately. ⚠ The saving is also
smaller than it looks: those five share the engine, but each additionally links
its own demo, so the union is genuinely larger than any one of them — unlike the
nine, whose union is 0.16% over the largest.

**Out of scope — the standalone set stays standalone,** per Jon: the game
binary and the four `*_demo` shells. They are what a player or a demo runner
launches; a subcommand is the wrong shape for a product entry point.

## The constraint a naive plan misses: the bins do not share a feature set

Only two of the nine are declared in `Cargo.toml`; the other seven are
auto-discovered from `src/bin/`. Their requirements differ:

| bin | requirement |
|---|---|
| `match_shots` | `required-features = ["visible", "capture"]` |
| `ladder_probe`, `match_report` | `#[cfg(feature = "causal")]` in the source |
| the other six | no feature gate |

⛔ **A single bin requiring the UNION would be unbuildable in the default cell,**
and worse than that: `ambition_demo_smash_app`'s own `[features]` comment
records that `--features visible` **is not a supported test cell** — the
composition has no renderer, and 25 of 39 tests die in Bevy parameter
validation. A modal binary that requires `visible` inherits that.

⇒ **The single `[[bin]]` declares NO `required-features`; each subcommand is
gated at its own definition.** `smash_tool` builds in the default cell with the
six ungated subcommands; `--features causal` adds two; `--features
visible,capture` adds `match-shots`.

⚠ **AND THAT IS A BEHAVIOUR CHANGE THAT MUST BE MADE LOUD.** Today, `cargo
build` without `visible,capture` silently omits `match_shots` — cargo skips a
bin whose `required-features` are absent and says nothing. Tomorrow the
subcommand is simply absent from `smash_tool --help`. ⇒ Compile the gated
subcommands' NAMES unconditionally and have the disabled arm exit non-zero with
the feature to rebuild with. A missing subcommand must say why it is missing,
not read as a typo.

## Measured result — it helped

✔ **BUILT AND MEASURED 2026-09-03, apples to apples in the DEFAULT feature
cell.** The prediction was ~0.7 GB from a symbol-union proxy; the built figure
is better than that.

```text
BEFORE — the eight default-cell binaries      4.79 GB
AFTER  — one `smash_tool`                     0.50 GB   (535,099,456 bytes)
                                              -------
saved                                         4.30 GB   (89.6%)
```

⭐ **The one binary is SMALLER than any single one it replaced** except
`stage_diagram`: 510 MB against 633–634 MB for each probe. Collapsing did not
add the parts up, it stopped writing eight copies of the same program.

⚠ **THE COMPARISON IS DEFAULT-CELL ONLY, and that is stated rather than
glossed.** `match_shots` (705 MB) was built with `visible,capture`, so it is
excluded from both sides; the nine-binary 5.48 GB figure quoted elsewhere mixes
feature cells and is not a fair "before". ⛔ The featured build was **started
and deliberately killed** — it pulls in the whole render stack and took the
volume from 8 GB to 2 GB free. A measurement that fills a shared disk is not
worth the number, and the default-cell result already answers the question.

✔ **BEHAVIOUR IS UNCHANGED, verified rather than assumed.** The old
`ladder_rig` binary was still on disk, so both were run with the same flags:
`smash_tool ladder-rig --sweep-below --seeds 1` and
`ladder_rig --sweep-below --seeds 1` produce **byte-identical** reports, all
seven `[ladder_rig]` lines. Exit codes checked without a pipe in the way
(`$?` after `head` is head's): `--help` 0, unknown subcommand 2, and
`match-shots` without its features 2 with the rebuild line.

ⓘ **AN OPERATIONAL FINDING WORTH KEEPING: `cargo clean -p` cannot remove a
binary whose target no longer exists.** It removed 1,405 files and 6.4 GB from
this package and left all nine old executables — 4.79 GB — because cargo no
longer knows those `[[bin]]` targets. ⇒ **Every bin rename or removal strands
its artifact forever** unless someone deletes it by name, which is what
reclaimed the space here. Worth remembering the next time a target is renamed.

✔ **`split-debuginfo` TRIED 2026-09-03, AND IT DOES NOT PAY HERE.** The plan
said attempt it first because it is one line; measured, it is the smaller lever
and the two overlap.

```text
                        executable      .dwo        total on disk
line-tables-only            510 MB      0 MB            510 MB
+ split-debuginfo=unpacked  302 MB    532 MB            834 MB   (9,938 files)
```

⇒ The executable shrank 41%, and the **total on disk went UP by 324 MB.** The
`.dwo` set is SHARED, so unpacking wins only when many executables would each
carry their own copy of the same DWARF — which is exactly the redundancy the
collapse already removed, and removed harder: 4.79 GB → 0.50 GB against this
lever's roughly 1.3 GB had it been applied to the nine.
⚠ **It is also not cheap to adopt.** Changing `[profile.dev]` invalidates every
crate: the experiment recompiled **352, of which 342 were third-party**, at
7m24s against the control's 2m53s. "One line" describes the edit, not the cost.
⇒ Left UNSET, with the measurement recorded at the setting in `Cargo.toml` so
the next person does not re-run it. Revisit only if a crate ever has many
binaries again.

⭐ **AND A WORKSPACE-ONLY CLEAN IS THE RIGHT RESET, confirmed empirically.**
`cargo clean -p` over all 77 workspace packages removed **48,931 files and
129.6 GiB**, taking the volume from 2 GB free to 123 GB, and the rebuild after
it compiled **60 crates, ALL OURS, ZERO third-party**. So the expensive
dependency graph survives a full reset of our own code. ⇒ That is the command to
reach for when the volume is short — not `cargo clean`, which throws away the
third-party compiles too.
ⓘ Where the space actually was: `debug/deps/` held **106.7 GB in 337
executables** against 17.8 GB of `.rlib` — test and bin artifacts, not
libraries. Cleaning libraries would have missed it.

## Status

✔ **PHASES 1–3 LANDED 2026-09-03.** `smash_tool` exists; the nine binaries are
gone; the three documented invocations are retargeted; the guard is in place and
poison-verified. `cargo check -p ambition_demo_smash_app --bins` is clean in the
default cell **and** under `--features visible,capture,causal`, so both arms of
the feature gate type-check.

✔ **PHASE 0's SIZE QUESTION IS ANSWERED** — see the measured result above:
4.79 GB → 0.50 GB in the default cell, better than the prediction. What remains
of Phase 0 is the `split-debuginfo` experiment and a featured-cell comparison on
a box with headroom.

⚠ **AND `split-debuginfo` WAS NEVER TRIED**, which the plan said to do FIRST
because it is one line and may take most of the win. Doing the collapse before
the cheap experiment is the wrong order; it happened because Jon asked for the
collapse directly. The experiment is still worth running — if it is most of the
win, that is worth knowing before the same collapse is proposed for another
crate.

## Phases

### Phase 0 — confirm the prediction before writing any code

⛔ Nothing below is worth doing if the collapsed size is not what the symbol
union predicts, and that number has never been built.

1. **The one-line lever FIRST.** `[profile.dev]` already spends
   `debug = "line-tables-only"`, but sets no `split-debuginfo`. Set
   `split-debuginfo = "unpacked"`, rebuild the nine, and measure the executables
   **plus** the `.dwo`/object output — both, because the question is whether
   DWARF stops being COPIED per executable, not whether the binaries shrank
   while the bytes moved. If this alone takes most of the win, stop: the answer
   is a one-line `Cargo.toml` change and the rest of this page is unnecessary.
2. **Confirm the union at n=2, not n=9.** One throwaway bin dispatching to two
   existing probes' entry logic; build it and compare against those two built
   separately. ⭐ **Write the prediction down first: ~635 MB against ~1,267 MB
   separate.** If it lands near 1,267 the symbol-union proxy is wrong and this
   plan is void — report that outcome as loudly as the confirming one.

⚠ Needs a box with headroom: a full `--all-targets` build of this crate is nine
link steps and the measurement needs both the before and after trees.

### Phase 1 — the dispatcher, with no behaviour change

3. Add `clap` to the workspace. ⓘ **It is not currently a dependency anywhere in
   the workspace** — this is a new third-party dependency and a real decision,
   not a formality. Pin an exact version in `[workspace.dependencies]`.
4. `src/bin/smash_tool.rs`: a `#[derive(Parser)]` with one subcommand per
   existing bin, each subcommand's body calling a `pub fn run(args) -> ExitCode`
   moved out of the old `main`.
5. Move each old `src/bin/<name>.rs` to `src/tools/<name>.rs`, changing `fn
   main()` to `pub fn run(...)` and nothing else. ⭐ **One bin per commit**, so a
   bisect lands on one tool.

### Phase 2 — the argument surface, which is where behaviour will drift

⛔ **Eight of the nine parse `argv` by hand, and one of them scans it four
separate times.** `ladder_rig` alone calls `std::env::args()` in eight places —
`--sweep-below`, `--scenarios`, `--weight`, `--no-rollout` and a
`while let Some(arg)` loop that re-reads the whole vector. `match_shots` three,
`capture_probe`/`match_diagram` two, four others one each; only
`select_walkthrough` reads none.

6. Port each flag to a clap field **keeping its exact spelling**. The flags are
   the public surface: `--sweep-below`, `--scenarios`, `--weight`,
   `--no-rollout` and the rest must survive verbatim.
7. ⚠ Hand-rolled `args().any(|a| a == "--flag")` accepts the flag ANYWHERE,
   including after a `--` separator or repeated; clap does not. Where a caller
   depends on that laxity it will now error. That is an improvement, but it is a
   behaviour change and belongs in the commit message, not in a surprise.

### Phase 3 — the callers, and a guard so they cannot rot

8. Update every invocation. Measured: **11 references across 6 files**,
   all of the form `cargo run -p ambition_demo_smash_app --bin <name> -- <flags>`
   → `--bin smash_tool -- <subcommand> <flags>`. `ladder_probe` is named in 6
   files, `ladder_rig` in 3, `roll_probe` / `capture_probe` / `match_report` in 1
   each; `match_diagram`, `match_shots`, `select_walkthrough` and `stage_diagram`
   in none.
9. ⭐ **Add the guard, because nothing currently catches this class.**
   `check_planning_citations.py` resolves cited SYMBOLS and paths; a bin name
   inside a fenced command is prose to it, so every one of those 11
   references can go stale silently — which is precisely the failure
   [`../recipes/checks-that-did-not-run.md`](../recipes/checks-that-did-not-run.md)
   is about. The guard is small: for every `--bin <name>` in `docs/` and
   `scripts/`, assert `<name>` is a bin cargo actually builds
   (`cargo metadata`). Poison it by renaming one bin.

## Risks, stated before they are discovered

* **Any probe edit relinks all nine.** Near-free here — they already share
  ~100% of their code, so a shared-crate edit already relinks all nine today.
* **`stage_diagram` is the odd one** (474 M, 267 K symbols, `.text` half the
  others'): it links a smaller subset. Folding it in makes *its* build heavier
  even as the total falls. It is 19 lines; fold it anyway, but expect its
  individual cost to rise.
* **A modal binary hides which tool is heavy.** Today `ls -l target/debug` shows
  nine numbers and `stage_diagram` visibly differs. After the collapse there is
  one number and no per-tool signal. If that signal matters, keep measuring
  symbol counts per module rather than per binary.
* **The subcommand names are a new public surface** and will end up in prose.
  Choose them once (`ladder-probe`, `roll-probe`, …, kebab-case per clap
  convention) and let the guard in step 9 hold them.

## Acceptance

* `smash_tool <sub>` reproduces each old binary's output byte-for-byte on the
  documented invocations — check `ladder_rig --sweep-below --seeds 1` against
  its recorded output in
  [`engine/fighter-brain.md`](engine/fighter-brain.md) first, since that one is
  quoted in prose and therefore checkable.
* Measured total for the crate's binaries falls from ~5.6 GB, with the actual
  figure recorded here beside the ~0.7 GB prediction — **whichever way it
  falls**.
* The nine old bin names appear nowhere in `docs/` or `scripts/` as `--bin`
  targets, enforced by the step-9 guard.
