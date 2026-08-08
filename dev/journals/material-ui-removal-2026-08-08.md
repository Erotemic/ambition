# `bevy_material_ui` removal — what actually made the title test flip

2026-08-08. Investigation before deletion. Raw observations first.

## The claim under test

`game/ambition_app/src/app/plugins.rs` carried (since 2026-08-03) a comment saying
`bevy_material_ui::dialog::DialogPlugin` is "load-bearing": dropping it makes
`the_title_screen_says_choose_game_and_is_readable` read the title at 20.0px
"because menu typography stops resolving". Found by a six-step bisect. The bisect
proved a CORRELATION; the mechanism was never checked.

## Observation 1 — baseline is green, and the resolver is a no-op here

`cargo test -p ambition_app --test app_it -- shell_host_rendered::the_title_screen_says_choose_game_and_is_readable`
→ **ok** on the current tree (Material installed).

## Observation 2 — the full `Text` population, WITH Material

Probe `probe_dump_title_screen_text_population` (temporary) dumps every `Text`
entity after `settle()`: entity, font size, `MenuTextHeightFraction`, `Name`,
ancestor chain, and whether an ancestor is `BasicShellUiRoot`.

29 text entities. The launcher subtree (`1175v0[bevy_ui menu root]`):

```
1179v0 size= 20.00 frac=None       label="Choose Game"  < tab[0] < menu tab bar < menu panel < root
1181v0 size= 60.48 frac=Some(5.6)  label="Ambition"     < menu body < menu panel < root   name="text"
1183v0 size= 20.00 frac=None       label="Ambition"     < control  < menu body  < menu panel < root
1185v0 size= 20.00 frac=None       label="Sanic"        < control  < ...
1187v0 size= 20.00 frac=None       label="Mary-O"       < control  < ...
1189v0 size= 20.00 frac=None       label="Pocket"       < control  < ...
1191v0 size= 20.00 frac=None       label="TwinTrack"    < control  < ...
1193v0 size= 20.00 frac=None       label="Smash"        < control  < ...
1195v0 size= 20.00 frac=None       label="Versus"       < control  < ...
1197v0 size= 20.00 frac=None       label="Exit"         < control  < ...
1198v0 size= 23.76 frac=Some(2.2)  label="Arrow keys select · Enter launches"  name="text"
```

Plus 18 non-launcher texts (FPS overlay, GGRS proof HUD, frame-axis glyphs, the
touch overlay cluster). No debug HUD "Ambition" — there is no session.

```
=== 'Ambition' entities: [(1181v0, 60.48), (1183v0, 20.0)] ===
=== primary windows: 0 ===
=== UiPlugin present: true ===
```

**⭐ THERE ARE TWO "Ambition" TEXTS AND BOTH ARE THE LAUNCHER'S.** `1181v0` is
the title. `1183v0` is the *game-select row label* — "Ambition" is the first game
in the roster, so the row for the game and the title of the screen carry the
identical string. The test's `rendered.iter().find(|(label,_)| label == "Ambition")`
picks whichever the global query reaches first.

**And `resolve_menu_text_size` cannot be the story**: primary windows = 0, so the
resolver's `unwrap_or(MENU_REFERENCE_VIEWPORT_HEIGHT)` branch is taken and it
writes exactly the size the spawner already wrote. 5.6% of 1080 = 60.48 — the
observed value. Running it or not running it is indistinguishable in this
composition. **The comment's stated mechanism is false.**

Row labels are 20.0px because `spawn_control` spawns them with no `font_size` and
no `MenuTextHeightFraction` — `TextFont`'s Bevy default is 20.0. That is where the
"Bevy's default 20.0" in the old comment came from; it was always a real entity,
not a failed resolve.

## Observation 3 — the full `Text` population, WITHOUT Material

Both plugins commented out of `add_ui_plugins`. Same probe.

**29 text entities. Same labels. Same sizes. The title is still 60.48px.**
The only difference in the whole dump is that every entity index shifted by
one or two (Material's `Startup: setup_dialog_overlay` spawns an entity), and
`ArchetypeId`s renumbered.

`=== 'Ambition' entities: [(1180v0, 60.48), (1182v0, 20.0)] ===` — the same two.

And the test FAILED:

```
panicked at tests/shell_host_rendered.rs:801:
the title renders at 20.0px — this is the units bug, not a taste question
```

**The world did not change. The test's answer did.**

## Observation 4 — the mechanism, by archetype

The probe was widened to print the query's own iteration order (unsorted) with
each entity's `ArchetypeId`. The title lives in the archetype that carries
`MenuTextHeightFraction`; the row labels live in the one that does not.

WITH Material — walk order by archetype `133, 118, 89, 120, 131, 94, 127`:

```
[ 2] 1198v0 arch=118  23.76  "Arrow keys select · Enter launches"
[ 3] 1181v0 arch=118  60.48  "Ambition"        <-- TITLE reached first
...
[12] 1183v0 arch=120  20.00  "Ambition"
```

WITHOUT Material — walk order by archetype `88, 86, 108, 117, 132, 126, 130`:

```
[ 2] 1182v0 arch=108  20.00  "Ambition"        <-- ROW LABEL reached first
...
[12] 1180v0 arch=117  60.48  "Ambition"
```

⭐ **That is the entire cause.** `bevy_material_ui` was never touching typography.
Its plugins register components and spawn a startup entity, which shifts
archetype/table creation order, which flipped which of two identically-labelled
entities a global `find()` reached first. Five days of "load-bearing" came from a
query-order coincidence.

## Observation 5 — the reviewer's `TextFont` hypothesis, tested

Hypothesis offered mid-investigation: *"an `"Ambition"` text with NO explicit
`TextFont` would produce Bevy's default size and explain the 20px immediately."*

**Confirmed in substance, with one correction.** `TextFont::default().font_size`
is **20.0**, and the row label reads exactly that (`bevy_default_size=true`). But
it is not that the entity lacks a `TextFont` — it cannot, `Text` requires one.
`spawn_control` (`crates/ambition_menu/src/render/bevy_ui/spawn.rs:245`) inserts
`TextFont { font: font.cloned().unwrap_or_default(), ..default() }` — it sets the
font HANDLE and leaves the SIZE at `..default()`. Same outcome, different reason,
and the difference matters: the handle was deliberately fixed there (the TOFU
work), the size was simply never considered.

Every launcher row label, and the "Choose Game" tab head, is a
`bevy_default_size=true` / `frac=None` node. So **the launcher's rows do not
scale with the window while its title and footer do.** That is a real
inconsistency in our own text path, it is NOT what made this test flip, and it is
left alone here — changing row typography is a layout/design change, not a
dependency removal.

## Verdict

1. `bevy_material_ui` was never load-bearing. The comment in `plugins.rs` was
   wrong and has been deleted, with the true finding put in its place.
2. The test was wrong: it let CONTENT TEXT act as ENTITY IDENTITY. It now scopes
   to the `BasicShellUiRoot` subtree, selects the title by ROLE (a launcher text
   node carrying `MenuTextHeightFraction` — which a control's label never has),
   and asserts the match is UNIQUE so a future duplicate fails loudly.
3. **Poison check**: with Material removed, the archetype order still puts the
   20px row label first (probe index 2, title at index 12) — i.e. the exact
   condition that produced the old false failure is still present — and the fixed
   test passes. The guard defends the gap, not just the fix.
