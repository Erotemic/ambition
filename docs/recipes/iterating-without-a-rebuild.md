---
status: current
last_verified: 2026-10-01
---

# Iterating without a rebuild

Use this page to change the game while it runs. It tells you, for each kind of
file, what a running game does when you save it, and what an edit costs when
the game is not running. The numbers are measured on the agent machine.

All of it applies to a desktop development build: the default
`cargo run -p ambition_app` (no `static_content`). A web, Android or bundled
build embeds its content (`static_content`, `static_map`), so it reads no file
from disk and watches nothing.

## What a running game does with a saved file

| You save | The running game | Where it is checked |
| --- | --- | --- |
| A move table (`game/ambition_content/assets/data/movesets/*.ron`) | Compiles the pack again from disk and reloads it: the bodies play the new moves about 20 frames after the save. | `edit_to_play_through_the_shell::a_content_file_saved_while_the_game_runs_is_played` |
| `data/boss_profiles.ron`, `data/boss_encounters/*.ron` | The same reload. A boss built after it has the new tuning and the new HP. | `a_boss_tuning_saved_while_the_game_runs_is_played` |
| `data/character_catalog.ron` (a row's health, body, abilities, brain) | The same reload; every character is folded again against the new catalog. A row added or removed (a character that starts or stops being built) is refused: restart. | `a_character_row_saved_while_the_game_runs_is_played` |
| `data/fighter_brain_ladder.ron`, `data/encounters/*.ron` | The same reload. | `ambition_content::reload` tests |
| `crates/ambition_platformer2d_actor_monolith/assets/ambition/platformer_defaults.ron` (movement tuning, and combat/time feel under `feel:`) | Writes a changed tuning to the F3 inspector's mirror (`EditableMovementTuning`, `EditableFeelTuning`); the developer-edit road publishes it about 20 frames after the save. A file that does not parse, or a `feel:` field the engine does not have, is refused. The starting abilities in the file need a restart. | `a_movement_tuning_saved_while_the_game_runs_is_played` |
| A Yarn file (`assets/dialogue/sandbox/*.yarn`) | Compiles the whole dialogue project with the new file, then uses it. A dialogue that is open starts its current node again. | `content_it::a_saved_dialogue_edit_is_played` |
| A procedural module (`game/ambition_content_modules/src/*.rs`) | Nothing until you rebuild the `.wasm` (next section); then the game loads the new code. | `a_module_file_that_changes_while_the_game_runs_is_reloaded` |
| The LDtk world | Press F11 to apply the edit, or F12 to apply each save. | `an_edit_reaches_the_shipped_game` |

A reload is refused, and the game keeps what it runs, when:

- the pack does not compile (the log shows the compiler's message);
- the change is to a file the reload does not take yet: `items.ron`, the
  audio registries, a fighter facet (`data/fighters/*.ron`; read only by a
  match that plays fighter damage, such as a Smash match that seats the robot), `boss_seeds.ron`, `boss_validator_bands.ron`;
- a catalog change adds or removes a character that is built. For these, restart the game (no rebuild; see
  below);
- a rollback timeline another owner holds is live (a networked match).

Read the log for the result. Each reload writes one line:
`content reload requested for route ...`, `dialogue reloaded from [...]`,
`movement tuning reloaded from ...`, `feel tuning reloaded from ...`,
`extension modules reloaded: ...`, or a refusal with its reason.

## Procedural modules: rebuild only the module

1. Run the module build in watch mode. It rebuilds the `.wasm` each time you
   save a module source, in about 1 to 2 s, and does not compile the engine:

   ```bash
   scripts/build_extension_modules.sh --watch
   ```

2. Start the game once with the loaded modules, and leave it running:

   ```bash
   AMBITION_EXTENSION_MODULES=target/extension-modules/wasm32-unknown-unknown/release \
       cargo run -p ambition_app
   ```

3. Save a module source. The watch rebuilds the file; the game sees the new
   file within about 20 frames and loads it. A module that the new build no
   longer exports leaves with the old build. Records that a module keeps
   across ticks are carried to the new field list by field tag.

See [`writing-a-procedural-module.md`](writing-a-procedural-module.md) to write
one.

## When the game is not running: what an edit costs

Every content file below is read from disk at startup, so an edit costs a
restart and no Rust build. Measured as `cargo build -p ambition_app --bin
ambition_game_bin` after touching the file, with a warm target:

| File | Rebuild |
| --- | --- |
| `boss_profiles.ron` | 0.44 s, no crate compiled |
| `audio/sfx_registry.ron` | 0.52 s, no crate compiled |
| `data/boss_sheets.ron` | 0.48 s, no crate compiled |
| A Yarn file | 0.43 s, no crate compiled |
| Movement defaults and feel (`crates/ambition_platformer2d_actor_monolith/assets/ambition/platformer_defaults.ron`) | 0.49 s, no crate compiled (embedded, it was 14.10 s: the actor monolith and 14 crates after it; a feel value in `ambition_combat`'s `Default` costs 14.98 s and 27 crates) |
| A demo's move table, catalog or fighter facet (`game/ambition_demo_*/assets/`, the versus pack in `game/ambition_app/assets/`) | 0.43 to 0.45 s, no crate compiled (embedded, it was 6.62 s: the demo crate and the app) |
| A file that is still embedded (`include_str!`) | about 6 to 7 s: `ambition_content` and the app compile again |

`assets/pack.ron` itself stays embedded: a new source also needs a new
declaration in `game/ambition_content/src/pack.rs`.

A demo's pack is written with `ambition_platformer2d::content_pack!`, which
reads every text (its `pack.ron` included) off disk and embeds them only under
the demo crate's own `static_content` feature. A new source is one more line in
the macro's `sources`. `ambition_app`'s `static_content` turns the feature on
for the demos it hosts.

## Validation

- After a content edit: the log line above, and the change on screen.
- After a refusal: the reason is in the log; fix the file and save again.
- To check the pack without the game: see
  [`validating-a-content-pack.md`](validating-a-content-pack.md).
