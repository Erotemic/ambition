# Yardrat's open measurements

Numbers this machine took that are executable by someone else, kept out of
[`queue.md`](queue.md) because that file is worked by several sessions at once
and a measurement is not yet a task with an owner. Each row states what was
measured, how to reproduce it, and what is NOT yet decided. When a row acquires
an owner and an acceptance criterion, promote it into `queue.md` and delete it
here.

⛔ This is not a staffing table and not a second review ledger. If a row here
stops being reproducible, delete it rather than annotating it.

---

## Path citations in Rust COMMENTS are not judged, and ~8% do not resolve

`scripts/check_planning_citations.py` gained a bare-path class on 2026-09-02
(paths without a `:LINE` suffix, which is how nearly all prose cites a file).
That class runs over `docs/planning/` only. The script's `--comments` mode
judges SYMBOLS in Rust comments and not paths, so the same blind spot exists
one level down, in the place the script's own docstring says the original
fabricated citation reached.

**MEASURED 2026-09-02, on `ec55d4035`:** 252 path citations in `.rs` comments,
**21 distinct unresolved in 28 places**. Reproduce with the ordered-containment
rule the docs class already uses (basename must exist; every component of the
citation must appear in that file's path in order, matching exactly or as the
tail of an `ambition_`-prefixed crate directory).

⭐ **THE TRIAGE IS ALREADY VISIBLE AND IT IS NOT ONE CLASS.** At least three:

- **SCHEMATIC, not a path** — `tests/foo.rs`, `src/foo/tests.rs`. These name a  <!-- cite-ok: this row's subject IS the dead citation -->
  SHAPE, exactly like the `provider::local_name` case the script already  <!-- cite-ok: this row's subject IS the dead citation -->
  documents as unmatchable by regex. They are correct prose. Whoever does this
  either marks them `cite-ok` or teaches the matcher that a `foo` component is
  a metasyntactic placeholder — and the second is worth thinking about, since
  three `*_it_sync.rs` files use the same idiom for the same reason.
- **MOVED** — `features/ecs/bosses.rs`, `src/rl_sim/runtime.rs`,  <!-- cite-ok: this row's subject IS the dead citation -->
  `engine_core/body_clusters.rs`, `presentation/rendering.rs`,  <!-- cite-ok: this row's subject IS the dead citation -->
  `app/schedule.rs`, `character_sprites/sheets.rs`, `brain/boss_pattern.rs`.  <!-- cite-ok: this row's subject IS the dead citation -->
  Old layouts, and `features/ecs/` in particular is the same dead prefix that
  `00030e603` left behind in `demos/smash-parity-inventory.md`.
- **ASSET AND TOOL PATHS** — `sprites/robot_spritesheet.ron`,  <!-- cite-ok: this row's subject IS the dead citation -->
  `sprites/pirate_admiral_spritesheet.ron`,  <!-- cite-ok: this row's subject IS the dead citation -->
  `sprites/player_robot_v2_spritesheet.ron`,  <!-- cite-ok: this row's subject IS the dead citation -->
  `assets/data/dialogue/registry.ron`, `tools/ldtk_intgrid_migration.py`,  <!-- cite-ok: this row's subject IS the dead citation -->
  `tools/ambition_sprite2d_renderer/mockingbird_boss_sprite_generator.py`.  <!-- cite-ok: this row's subject IS the dead citation -->
  ⚠ These need a different judgement from the code ones, and the judgement is
  now concrete rather than a worry. ⭐ **The sprite manifests are GENERATED and
  gitignored** — `.gitignore` lines 123-124 cover
  `crates/ambition_platformer2d_actor_monolith/assets/sprites/*.ron` — so they
  are absent for exactly the reason `target/` is, and citing one is correct
  prose. ⇒ **Generalise the skip from a `target/` prefix to `git check-ignore`:**
  a citation whose path is ignored names a build output, whoever generates it.
  That is one rule instead of a growing prefix list, and it is the rule the
  `target/` case was always an instance of. The remaining tool paths
  (`tools/ldtk_intgrid_migration.py`,  <!-- cite-ok: this row's subject IS the dead citation -->
  `tools/ambition_sprite2d_renderer/mockingbird_boss_sprite_generator.py`) are  <!-- cite-ok: this row's subject IS the dead citation -->
  NOT ignored and want ordinary MOVED triage.

⭐ **AND THE MOVED HALF IS NOW TRIAGED, so that part is mechanical.** Every one
has real history — `git log --all --diff-filter=AD` finds the commit that moved
it, so none is FABRICATED — and each resolves to one current home:

  features/ecs/bosses.rs          game/ambition_content/src/bosses/mod.rs
                                  (64631503f, test-block split)
  presentation/rendering.rs       crates/ambition_render/src/rendering/mod.rs
                                  (09d945694, <mod>/mod.rs normalisation)
  character_sprites/sheets.rs     crates/ambition_sprite_sheet/src/character/sheets/mod.rs
                                  (aa7cb8f60)
  brain/boss_pattern.rs           crates/ambition_characters/src/brain/boss_pattern/mod.rs
                                  (4ff42f510)
  rendering/foreground.rs         GONE, not moved — d09229ceb ("Two views draw two
                                  pictures") ended it; the comment needs re-deriving,
                                  not repointing
  dialog/yarn_bindings.rs         GONE from that path — 73873491b moved this game's
                                  Yarn verbs to the crate that owns the game; nearest
                                  live file is
                                  crates/ambition_conversation/src/dialog/yarn_harness.rs,
                                  which is NOT the same thing and must be read before
                                  citing
  player/systems.rs               GONE — 5ba894709 ("player/ no longer exists — the
                                  fold's conclusion"); re-derive

⚠ Two of the seven are **not repointings**: `rendering/foreground.rs` and  <!-- cite-ok: this row's subject IS the dead citation -->
`player/systems.rs` name things that were ENDED rather than relocated, so the  <!-- cite-ok: this row's subject IS the dead citation -->
sentences around them are claims to re-check, not links to fix. That is the
difference the checker's own triage insists on, and it is why this row says
"triage first" rather than "sed".

⚠ **WHAT IS NOT DECIDED, and why this is a measurement rather than a task:**
whether the comment lane should judge paths AT ALL. The script is explicitly a
worklist and not a gate, but `--comments` already emits findings, and adding 21
more without triaging them first is the "teaches its reader to skim" failure the
script's own docstring warns about. ⇒ **Triage first, extend second.** A change
that turns the class on before the findings are resolved makes the tool worse.

---

## Checked and CLEAN: cross-links between planning pages (do not build a checker)

Recorded so the next person does not build the tool this obviously suggests.
After the bare-path class landed, the natural follow-up is "validate the
`[text](other.md#anchor)` links too". ⇒ **Measured 2026-09-03: 328 internal
links across `docs/planning/**/*.md`, 0 dead files, 0 dead anchors — and only
ONE link carries an anchor at all.** A checker for that class would be a
permanent maintenance surface guarding a single citation.

⭐ The anchors that DO rot are the ones in policy `source_doc` fields (239 of
them, 15 repointed off dead `decomposition.md#…` anchors on 2026-09-02), and
those are guarded now by `every_source_doc_names_a_real_file_and_heading` in
`tests/ambition_workspace_policy/tests/policy.rs`. That is where the class
lives; the prose links are not it.

---

## The `zero_duration_pump` bisect recipe, and the correction to it

Kept because the recipe worked and the correction is the part that would be
re-learned the hard way.

Bisect only the commits in the range that touch COMPILED files — a docs commit
cannot change behaviour, and on the 2026-09-02 range that was 21 of 70, turning
7 builds into 5. Each probe is a full `cargo test` of one test, so the saving is
real (roughly 20 minutes a probe on this VM).

⛔ **BUT BISECT THE ANCESTRY CHAIN, NOT THE `rev-list` INDEX.** On a branch with
merges the two are different, and I reported an exclusion that did not hold
because of it: a GOOD result at one index bounded nothing, because that commit
sat on a side branch that merged in later and was an ancestor of neither
candidate. Check with `git merge-base --is-ancestor A B` before treating a
result as a bound. The verdict survived — `06a494f4e`, confirmed independently
from the mechanism side — but it survived for a different reason than the index
suggested.
