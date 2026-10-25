# Character generations as skins

**Status:** design, not implemented (2026-10-24). Open questions: Q170 in
[`../awaiting-maintainer-decision.md`](../awaiting-maintainer-decision.md#q170--which-generations-of-a-character-does-the-game-ship-as-skins)
and the list at the end of this page.

Jon, 2026-10-24: "I want to eventually have the game be able to switch between
different generations of a character as different skins." Jon, 2026-10-06:
each MAJOR redesign is browsable and can be used in the game; only big
redesigns count.

## Terms

- A **lineage** is one character as the player knows it. An **era** is one
  major look of it, pinned to a commit
  (`tools/ambition_sprite2d_renderer/design_history/lineages.yaml`).
- A **generation** is an era that the game can draw. In this plan it is the
  same thing as an era.
- A **skin** is presentation only: `WornSkin` (Q159 ruling, 2026-10-19). It
  never changes collision, attack geometry, movement or any other mechanic.
  The body, its kit and every input of the simulation stay the worn
  character's.

## What exists (measured 2026-10-24)

- `python -m design_history census`: 95 lineages, 221 eras; 113 of 259
  character targets and 119 of 243 catalog rows with a sheet have a lineage.
  A character with one design needs no lineage.
- `python -m design_history character <id>` goes from a catalog id, a target
  or a lineage to the eras, and from an era to its target and commit.
- `WornSkin(CharacterId)` names a catalog row whose sheet the body draws. The
  Smash select screen offers, as skins of a fighter, the rows with
  `form_of: Some(<fighter>)` and the `smash_skin` tag
  (`game/ambition_demo_smash/src/select.rs`).
- Two shapes of redesign. **(1)** A new `_vN` target beside the old one (15
  pairs, for example `paul_diracula` and `paul_diracula_v2`): both are
  published now, so the old generation already has a runtime sheet. **(2)** A
  redesign in place (Alice, Bob, Noether): the old look is only a commit.
  Of the 221 eras, 116 are the current look of a published target and 105
  are older looks of a published target, with no runtime sheet (REASONED:
  the last era of each target is taken to be its published look).

## What a generation must publish to be a skin

An era that is only a commit has no runtime art. To be a skin, it must be a
published unit like any other: sheet pages, layout (`_spritesheet.yaml` /
`.ron`), actor sidecar, part flipbook, portraits when it has them, and the
three quality tiers. The art must come from the era's own generator, run at
the era's commit, because the rule of the design history is that no old code
enters the tree.

Recommended shape: a publish step `design_history publish <lineage> <era>`
renders the era at its commit in a scratch worktree (as `build` does for a
still) and installs it under its own key, `<target>@<era-key>` (for example
`alice@toon_2026_05`), with a provenance record that names the era commit.
The step goes through the ordinary sprite publish and its staging root, so it
is never run beside another publish. The published files are the authority
for the game; the commit is the authority for regenerating them.

A `_vN` pair needs no new publish: the old target is already published.

## How a catalog row names a generation

A generation is a look of a character, not a second character. So it is a
catalog row with `form_of: Some(<character>)`, as the goblin looks are today,
plus a field that names its lineage era, for example
`generation: Some((lineage: "alice", era: "toon_2026_05"))`. The row names
the generation's sheet in `manifest` as any row does. A body draws it through
`WornSkin(<row id>)`; the Smash select screen and a later in-game choice list
the rows by `form_of`. No new engine type is needed: `WornSkin`, `form_of`
and the manifest already carry it. The `generation` field is metadata for
tools (the art inspector, the census) and a guard (the era exists in the
lineage).

A generation row states no body, kit, moveset or hurtboxes: those stay the
worn character's. The catalog validator refuses a `generation` row that
states mechanics.

## Rows and clips that do not match

Generations differ in rows, frame counts and frame sizes. Measured on
published pairs: `paul_diracula` has 43 rows at 128x128 and
`paul_diracula_v2` has 13 at 176x164 (10 shared); `carl_runga` 22 rows,
`carl_runga_v2` 8 (7 shared); `player_robot_v2` 42 rows, `player_robot_v3`
140 (41 shared); `stochastic_parrot` and its v2 have the same 12 rows.

Rules (all presentation; none changes the simulation):

1. The worn character's move picks the row name and the move's frame, as now.
   The skin draws its row of that name at the same phase of the move
   (frame index scaled by frame count), never by its own durations. A skin
   cannot make a move longer, shorter or earlier.
2. A row the skin does not have falls back through the existing chain
   (`CharacterAnim::base_pose`, the move's clip fallbacks), then to the
   skin's idle.
3. Hit boxes, hurt boxes, body boxes and hand landmarks come from the worn
   character's sheet only (the baked-sheet mechanics digest, Q157). A skin's
   own boxes are not read.
4. The skin is scaled so that its drawn body fits the worn character's body
   box (the feet anchor and the drawn height), not its own frame size.
5. A tool reports, per generation row, the rows the worn character uses that
   the skin lacks. A missing row is a visible fallback, not an error.

## Cost (Android packages every tier)

- Published bytes of a lineage target, all four tiers (99 targets with a
  provenance record): median 1.8 MB, mean 4.1 MB, max 47.5 MB
  (`imperfect_cellular_automaton`); the tiers add about 53% to the full tier
  (median ratio 1.53). MEASURED.
- An old era rendered at its own commit costs about the same as the current
  publish: `tech_bro_disruptor` at `6314c07` drew a 1.4 MB full-tier sheet
  (now 0.71 MB); `goblin` at `10347d4` drew 12.2 MB (now 12.9 MB). MEASURED,
  two eras.
- All 105 older looks as skins: about 105 x 1.8 MB = 190 MB at the median,
  about 430 MB at the mean, against 1.86 GB of sprites today (all tiers).
  REASONED from the numbers above.

So the game must not ship every era. Ship a curated set, or ship only the
lower tiers of a generation, or download generations on demand.

## Steps (when Jon rules)

1. Q170 ruling: which generations ship, and where the player chooses them.
2. The `generation` field on catalog rows, its validator rule, and the census
   check that each `generation` names a real era.
3. `design_history publish <lineage> <era>`: render at the commit, publish
   under its own key with a provenance record.
4. The phase-mapped skin draw (rule 1) and the body-box scale (rule 4), with
   a headless test that a skin changes no simulation state.
5. The art inspector lists the published generations of each character.

## Open questions for Jon

1. Which generations ship: every major era, a curated few per character, or
   only on request? (Q170)
2. Where does the player choose a generation: the Smash select screen only,
   a wardrobe in the main game, or both? Is a generation unlocked by
   progress?
3. Is a `_vN` target beside its base (Paul Diracula v1 and v2) a generation
   skin of one character, or does it stay a separate catalog character when
   the old one has its own kit? The player robot versions are separate
   characters by design (`player_robot_lineage.rs`).
4. On Android, does a generation ship all four tiers, or only the tiers that
   the device uses?
5. When a generation lacks a row (Paul Diracula v2 has no air attacks), is
   the fallback pose acceptable, or must a generation be completed before it
   ships?
