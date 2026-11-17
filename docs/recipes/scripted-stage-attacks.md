# Author a stage attack on the built host

The [encore script](../../game/ambition_content/rhai/boss_stage_pyro.rhai) adds pyrotechnic jets to the Boss of the Game fight. The [pattern data](../../game/ambition_content/assets/data/boss_profiles.ron) selects its warning, strike, and recovery durations. The existing native boss moves keep their implementation.

## Run and edit

Use the normal desktop host after its first build:

```bash
AMBITION_EXTENSION_MODULES=game/ambition_content/rhai/boss_stage_pyro.rhai ./run_game.sh --reuse
```

Go to the Hall of Bosses practice stage, or dial `LEGACY` and enter the main stage. The jet beat starts during the encore. Edit `JET_SPEED`, `PULSE_INTERVAL_S`, or the script's sequence while the host runs. The module watcher compiles Rhai and publishes a new content generation. This needs no Cargo invocation. Invalid source keeps the last program. A rollback session can refuse a mechanical edit while it owns the timeline; use a local session for live authoring.

The Lightning inventory skill can use the same loop:

```bash
AMBITION_EXTENSION_MODULES=game/ambition_content/rhai/lightning.rhai ./run_game.sh --reuse
```

Equip Lightning from the inventory. Press the Projectile action for a three-bolt fan. Hold it to repeat. Each volley costs 9 mana. Release does not clear its cooldown.

## Contracts

`boss_special` describes the existing `BossSpecialCast` port. It runs in `technique_execution` for each bound special on each boss. `telegraphing` selects the warning; `pressed` selects the strike. The pattern can press on every tick of its strike. An entry must keep its own cadence or act once. `idle: "reset"` clears the beat records when neither part runs. Body records and the boss pattern rewind together.

The trigger gives settled body and combat-box positions, facing, projectile offset, optional target, and optional live room size. It does not expose arbitrary world mutation or another body's private state. A script must handle missing room and target observations. It must not keep a target position as if it were a durable body identity.

`effect` describes the existing `EffectPort`. Its `at` is a world point or `()` for the invocation's body at lowering. It accepts an authored effect id, scale, and clockwise angle in radians. It is presentation only. The effect cannot deal damage, change collision, or confirm a hit.

Both contracts already have native and portable Rust consumers. The
[native boss techniques](../../game/ambition_content_modules/src/bossofthegame.rs)
use the boss trigger; the [blink module](../../game/ambition_content_modules/src/blink.rs)
uses the effect request for travel feedback. Rhai uses the same port keys
and value codecs. The new aliases add descriptions, not separate domain
authority. The [generated reference](../sdk/script-ports.md) lists their fields.

Jets submit ordinary `projectile` requests. The native path assigns identity and allegiance, moves each projectile, casts collision, applies guard rules, and resolves damage. A request is not an accepted hit. The script's warning creates no projectile. A dead caster creates no warning or shot. The pattern supplies a rest after the strike. Each beat has a finite pulse count and request limit.

The stage floor and jet locations are named game content. Both stage generators define a floor 64 units above the room bottom. The jet fractions put them on the middle and low side risers, 224 and 96 units above the floor. The emitters start above those platforms; overhead collision can stop each jet. The central span remains clear for the climb. A different arena must author its own layout; this script is not a general floor detector.

## Measure before promotion

A host can insert `ambition_platformer2d::extension::ExtensionProfile::default()` to measure entries. Its rows expose calls, faults, total invocation time, and backend time. These diagnostics do not affect simulation and do not rewind. A replay counts its calls again. Remove the resource to stop measurement.

Run the focused assembled-session witnesses:

```bash
cargo test -p ambition_app --test app_it -- the_boss_stage_has_scripted_pyro --nocapture
cargo test -p ambition_app --test app_it -- lightning_is_authored_in_rhai --nocapture
```

After the test executable is built, it can repeat the edit witness directly. Use the executable path Cargo printed:

```text
<app_it executable> --exact the_boss_stage_has_scripted_pyro::the_prebuilt_session_accepts_a_script_edit_and_keeps_bad_code_out --nocapture
```

That witness writes a new script while the real game session runs, rejects bad source, and checks the speed of the next live jet. Its output measures edit-to-live time. The damage witness reports `PYRO_COST` for the encounter with and without rewinds. These development-profile results are iteration measurements; they do not establish release, browser, or mobile frame budgets.
