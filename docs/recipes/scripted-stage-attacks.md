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

Equip Lightning from the inventory. Hold the Projectile action to send five rays from the hand in a cone. The script sets the mana cost per second and the time between pulses. When a ray does damage, it can jump once to the nearest new enemy in range with a clear path. Adjust `JUMP_CHANCE`, `JUMP_RANGE`, and `JUMP_DAMAGE` in the script.

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

## Reactive stage rules

The [guitar feedback script](../../game/ambition_content/rhai/boss_stage_feedback.rhai)
uses `boss_conduct` to read the live encounter and tracked body. Its `mount`
observation supplies standing ledges and the walkable slope in world units,
with +Y down. It uses the actual slope points for both the warning and the
strike. A climb starts a 1.1-second warning, a 0.25-second pulse, and a
4-second recovery. The script owns these durations.

`lightning` is a leased visual. It causes no damage. `held_ray` requests a
shared combat volume; its published geometry supplies the same procedural
renderer that Wolfenstein uses. Native combat owns walls, portals, damage,
and hit history. The script stores its phase, timer, and ray generation in
body records. A dead or inactive boss cancels the sequence.

These contracts are also available to native and portable Rust modules.
`Spot` accepts `#{ world: [x, y] }`, `#{ body: [x, y] }`, or a named art part.
An invalid endpoint or an unavailable phase is refused by its domain or by
module admission. This is not a general component mutation interface.

The [Gram Reaper script](../../game/ambition_content/rhai/gram_reaper.rhai)
uses `conducted_pose` to hold a position and facing through a strike and its
recovery. The body solver checks the requested position against the room.
Send `request("conducted_pose", #{})` to return control to the body controller.
`boss_guard` controls whether melee and projectile hits can damage the boss.
`camera_shake` sets the size of a shake in pixels on a confirmed frame.

The status and hit-response aliases can support future mechanics without a
current game consumer. `status_tick`, `time_scale`, `status_clock`, and
`in_play` share the body status service. `struck` reports an accepted on-hit
effect and `parried` reports a caught parry. `answer_parry` uses that phase's
parry index. `credit` and `struck.striker` supply opaque decimal tokens for
`blast.credit`. A script can retain a token in a `u64` record field. All
`u64` values use decimal strings in scripts, including initial values such
as `initial: "18446744073709551615"`. Use `u32` for small arithmetic counters.

## Script an encounter pattern

The [Gram Reaper](../../game/ambition_content/rhai/gram_reaper.rhai) uses
`Special("move_key")` for the warning and strike in
[`boss_profiles.ron`](../../game/ambition_content/assets/data/boss_profiles.ron).
Give each move its own pose and sound. Set its warning, strike, and rest
durations in the pattern. Bind a `boss_conduct` entry to the boss profile ID
to use the same script at all placements of that profile.
Set `telegraph_speed_scale` to limit steering for a move that must wind up
in place. Bounds are zero for a stop and one for full steering.

Import the SDK library with `import "ambition/boss_beats.rhai" as beats;`.
Declare a body record with `beats::schema("your_script.beat")`. On each
invocation, call `beats::advance(this.state.beat, this.trigger, this.dt)` and
save the returned `beat`. Read its `stage` for `warning`, `strike`,
`recovery`, or `idle`. Use `started` to set the shape and aim of a new
attack. Use `entered` for sound and particles at the start of a stage. The
pattern supplies the clock. The library gives a progress fraction for each
warning and strike.

Keep a committed rectangle in a body record with `beats::box_kind()`.
Use `beats::show_box` and `beats::strike_box` with that rectangle for the
warning and strike. Declare the `attack_mark` and `held_damage_box` request
aliases. Each box has a slot. During the strike, submit the box with the same
slot and the beat's generation on each tick. This keeps the hit history for
the full strike. Stop submitting it during recovery.
Use `beats::boxes` to play a list of rectangles together or in order.
Its `fired` field gives the first active slot on the tick it starts.
Use that edge for a sound or particle burst.
When the body needs its ordinary animation again, clear a pinned row with
`beats::pose("", 0.0)`.

Play the Reaper in Hylbert's Hotel or use its practice door in the Hall
of Bosses. Set `AMBITION_EXTENSION_MODULES=game/ambition_content/rhai/gram_reaper.rhai`
to edit its poses and regions in a running development session. The
Platformer2D host supplies the SDK library to shipped and development
scripts. The example uses the room's measured floor and side boundaries.
