# Repository measurements

These commands measure source structure, simulation behavior, build cost, and
asset coverage. They write reports and measurement data. They do not own content
publication. Run a command with `--help` for its input and output options.
Commands that inspect the live content library need an existing publication.

## Unattended Smash profiling on Android

Smash has an all-CPU workload that uses the **real** character-select-to-stage
path and authored `duelist_l3` brains. No touch input is needed. The profile
uses a 40-stock match to keep the population active throughout the capture;
this is a profiling rule, not a change to the shipped three-stock default.

```bash
scripts/measure/autoprofile_android.sh smash --warmup 35 --duration 60
scripts/measure/autoprofile_android.sh smash_4p --warmup 35 --duration 60
```

`smash` seats two CPUs; `smash_4p` seats four. Both select an available fighter
pair from the current assembled roster (repeating it for four seats), so
compare reports from the **same content build**. The Android wrapper boots the
`smash_select` route, and the game inserts the all-CPU match roster before
routing to `smash_gameplay`. It reports `[smash-autoplay] LIVE` after the opening
ceremony; missing or ended matches are reported as failures, not valid samples.

The existing `profile_android.sh` captures the CPU and graphics reports and
stops the one-shot autoplay process afterwards. As with other profiles, follow
its printed directory to `symbolize_android_profile.py`. A slow phone may
need a longer warmup; the wrapper forwards `--warmup`, `--duration`, and other
`profile_android.sh` flags unchanged. Do not compare the 2P and 4P runs as
identical workloads: use them to separate fixed frame cost from fighter-count
scaling. Also consider thermal throttling over the capture interval.

The driver is only selected by `AMBITION_AUTOPLAY=smash` or `smash_4p`;
ordinary launches retain Smash character select and its normal rules.
