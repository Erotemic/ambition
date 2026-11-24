# Repository measurements

These commands measure source structure, simulation behavior, build cost, and
asset coverage. They write reports and measurement data. They do not own content
publication. Run a command with `--help` for its input and output options.
Commands that inspect the live content library need an existing publication.

`capture_native_audio.py` runs a Linux ALSA command through a bounded software
capture. It uses no physical audio device. See the
[native music capture recipe](../../docs/recipes/generated-music-workflow.md#native-software-capture-on-linux).

## Unattended Smash profiling on Android

Use the Android CLI to start the profiling match. The installed APK keeps its
normal launcher and manual play. Autoplay settings apply to one launch only.
The profiler stops that run after capture. It does not make an autoplay APK.

```bash
# Four CPUs, full Smash stage, 40 stocks, level-3 fighter brains.
scripts/measure/autoprofile_android.sh smash_4p --warmup 35 --duration 60

# Two CPUs, otherwise the same workload.
scripts/measure/autoprofile_android.sh smash --warmup 35 --duration 60
```

The wrapper calls `profile_android.sh all --profile-build --launch` with two
`--launch-env` values. For direct use of `profile_android.sh`:

```bash
scripts/measure/profile_android.sh all --profile-build --launch \
  --launch-env AMBITION_BOOT_ROUTE=smash_select \
  --launch-env AMBITION_AUTOPLAY=smash_4p \
  --warmup 35 --duration 60 --name android-smash-4cpu
```

`smash_4p` seats four CPUs, while `smash` seats two. The game uses the
normal select-to-stage route and `duelist_l3` brains. The profiling roster
uses 40 stocks. Normal Smash matches retain their authored stock count.
Both profiles choose a fighter pair from the assembled roster and repeat
that pair when needed. Keep the APK content and device conditions fixed
when you compare two captures.

The profiler checks that Android consumed the one-shot launch file. For Smash,
it also waits for `[smash-autoplay] LIVE:` in logcat before it starts CPU
sampling. It rejects a failed or ended match. Find the checks in the output
bundle at `launch-env-consumption.txt`, `smash-autoplay-readiness.txt`, and
`smash-autoplay-capture-status.txt`. A slow phone can need more than the
35-second warmup. Use Android CPU time and per-thread utilization to judge
CPU efficiency; FPS may be capped by the display. Compare the recorded CPU
clocks and thermal state between runs.

The profiler prints an upload tarball path. The full capture is in the
matching directory without the `.tar.gz` suffix. For the most recent four-CPU
wrapper run, analyze that directory:

```bash
profile_dir="$(find dev/ambition_dev_measurements/profiles -maxdepth 1 -type d \
  -name 'android-all-smash_4p-auto-*' | sort | tail -1)"
scripts/measure/symbolize_android_profile.py "$profile_dir" --ours
scripts/measure/symbolize_android_profile.py "$profile_dir" --inclusive
```

The `--profile-build` option builds a symbol-friendly APK and installs it.
The CLI clears compile-time boot and autoplay overrides for one-shot runs.
Do not prefix the build with `AMBITION_AUTOPLAY=...`: that is a different,
compile-time mechanism. The one-shot CLI settings are also removed on failure.
Open the installed game from its icon after the capture to play normally.

## Profiling Sanic on Android

```bash
scripts/measure/autoprofile_android.sh sanic_all
```

This command starts Act 1, jumps the spike pit, and runs through the goals in
Acts 2 and 3. The CPU capture starts at launch and runs for 120 seconds by
default. The bundle contains `sanic-acts.txt` with average FPS and p50 and p95
frame times for each completed act. The act results exclude loading and
results screens. If the phone does not finish in time, run with
`--duration 180`. The script saves the incomplete bundle.

### Act 3 only

This profile starts in Act 3 and holds Right through its ground route and
portals. After the normal results card, it replays Act 3. It uses the shared
scripted input and room replay paths.

```bash
scripts/measure/autoprofile_android.sh sanic_act3 --warmup 25 --duration 30
```

The launch settings apply to this capture only. A normal launch still starts
in Act 1. This route does not visit every optional road or portal.

## Clip transitions of one fighter

`pose_transitions.py` reads a published sprite target's parts and reports, for
each gameplay transition (fall to land, run to jump squat, ...), how far the
body parts jump at the cut against how far they move between frames inside
either clip. It is POSE-CONTINUITY's measurement
(`docs/planning/demos/smash-expressive-fighter-poses.md`, Phase 3).

```bash
python3 scripts/measure/pose_transitions.py player_robot_v3
```
