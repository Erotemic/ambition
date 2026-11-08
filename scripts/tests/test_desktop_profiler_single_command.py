"""A desktop profile must build first, then sample only the planned game binary.

Uses a fake launcher/compiler/perf and runs the actual script end-to-end. No
Rust compilation, perf privileges, window, or GPU are required for this test.
"""
from __future__ import annotations

import os
import shutil
import subprocess
import tarfile
import tempfile
import unittest
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]


class SingleCommandProfilerTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="ambition-profiler-")
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name) / "repo"
        self.root.mkdir()
        for relative in (
            "scripts/measure/profile_desktop.sh",
            "scripts/lib/profile_census_csv.py",
            "scripts/lib/profile_bundle_summary.py",
            "scripts/lib/profile_metadata_json.py",
            "scripts/lib/profile_timeline.py",
            "scripts/lib/profile_smash_match_window.py",
            "scripts/lib/tracy_zone_report.py",
        ):
            target = self.root / relative
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(REPO / relative, target)
        self.bin = Path(self.temp.name) / "bin"
        self.bin.mkdir()
        self._executable(self.root / "run_game.sh", '''#!/usr/bin/env bash
set -euo pipefail
if [[ "$1" != "--print-plan" ]]; then
    printf 'ERROR: launcher ran inside capture\\n' >> "$FAKE_CALL_LOG"
    exit 79
fi
features=
for arg in "$@"; do
    if [[ "$arg" == profile ]]; then features=profile; fi
done
cat <<EOF
package=ambition_app_tools
binary=smash_match_profile
profile_dir=profiling
binary_path=$FAKE_REPO/target/profiling/smash_match_profile
features=$features
game_arg=--window
game_arg=--fighters
game_arg=4
game_arg=--seconds
game_arg=1
build_arg=cargo
build_arg=build
build_arg=-p
build_arg=ambition_app_tools
EOF
# Echo passthrough game flags like the real launcher --print-plan. Without
# this, the stub only exposes its fixed --window/--seconds arguments.
passed=no
for arg in "$@"; do
    if [[ "$passed" == yes ]]; then printf 'game_arg=%s\n' "$arg"; fi
    if [[ "$arg" == -- ]]; then passed=yes; fi
done
if [[ "$features" == profile ]]; then
    printf 'build_arg=--features\nbuild_arg=profile\n'
fi
cat <<EOF
build_env=CARGO_INCREMENTAL=0
runtime_env=FAKE_PROFILE_RUNTIME=from-launch-plan
EOF
''')
        self._executable(self.bin / "cargo", '''#!/usr/bin/env bash
set -euo pipefail
printf 'cargo %s\\n' "$*" >> "$FAKE_CALL_LOG"
[[ "$CARGO_INCREMENTAL" == "0" ]]
if [[ "${FAKE_BUILD_FAIL:-}" == "1" ]]; then
    echo 'simulated compiler error' >&2
    exit 17
fi
mkdir -p "$FAKE_REPO/target/profiling"
cat > "$FAKE_REPO/target/profiling/smash_match_profile" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
printf 'game %s runtime=%s\\n' "$*" "$FAKE_PROFILE_RUNTIME" >> "$FAKE_CALL_LOG"
[[ "$FAKE_PROFILE_RUNTIME" == "from-launch-plan" ]]
echo '[smash-profile] cpu_seats=4 brain=duelist_l3' >&2
echo '[smash-profile] live after 40 frames; fighters=4' >&2
if [[ "${AMBITION_PROFILE_UNCAPPED:-}" == "1" ]]; then
    echo '[census] visual_quality t=1 profile=High frame_cap=Off present_mode=Immediate' >&2
else
    echo '[census] visual_quality t=1 profile=High frame_cap=Auto present_mode=Fifo' >&2
fi
if [[ "${FAKE_PROFILE_SLOW_GAME:-}" == "1" ]]; then
    sleep 0.7
    echo '[smash-profile] WARNING: the match ended -- later samples are results' >&2
    sleep 0.1
fi
echo '[smash-profile] done seats_at_end=4' >&2
EOF
if [[ " $* " == *" --features profile "* ]]; then
    printf '# TracyPlot\n' >> "$FAKE_REPO/target/profiling/smash_match_profile"
fi
chmod +x "$FAKE_REPO/target/profiling/smash_match_profile"
''')
        self._executable(self.bin / "perf", '''#!/usr/bin/env bash
set -euo pipefail
if [[ "$1" == "--version" ]]; then echo 'perf test-stub'; exit 0; fi
if [[ "$1" == "record" ]]; then
    printf 'perf record\\n' >> "$FAKE_CALL_LOG"
    shift
    while (( $# )); do
        case "$1" in
            -o) shift
                printf 'synthetic-perf-data\\n' > "$1"
                : ;;
            --) shift; exec "$@" ;;
        esac
        shift
    done
    exit 81
fi
if [[ "$1" == "report" ]]; then
    printf '1.0%% fake game symbol\\n'
    exit 0
fi
printf 'unexpected perf call: %s\\n' "$*" >> "$FAKE_CALL_LOG"
exit 82
''')
        self._executable(self.bin / "tracy-capture", '''#!/usr/bin/env bash
set -euo pipefail
printf 'tracy capture\n' >> "$FAKE_CALL_LOG"
[[ "$1" == "-o" ]]
printf 'fake-trace\n' > "$2"
''')
        self._executable(self.bin / "tracy-csvexport", '''#!/usr/bin/env bash
set -euo pipefail
printf 'tracy export %s\n' "$*" >> "$FAKE_CALL_LOG"
if [[ "${1:-}" == "--unwrap" ]]; then
    printf 'name,ns_since_start,exec_time_ns\nframe,1000000000,1000000\n'
else
    printf 'name,total_ns,counts,mean_ns,max_ns,min_ns\nframe,1000000,1,1000000,1000000,1000000\n'
fi
''')
        self.logfile = Path(self.temp.name) / "calls.log"

    @staticmethod
    def _executable(path: Path, contents: str):
        path.write_text(contents)
        path.chmod(0o755)

    def capture(self, *extra):
        env = dict(os.environ)
        env.update(FAKE_CALL_LOG=str(self.logfile), FAKE_REPO=str(self.root))
        env["PATH"] = f"{self.bin}{os.pathsep}{env.get('PATH', '')}"
        cmd = [
            str(self.root / "scripts/measure/profile_desktop.sh"),
            "--smash", "--smash-fighters", "4", "--smash-seconds", "1",
            "--name", "unit", "--out", str(self.root / "profiles"),
            "--no-tracy", "--no-record", "--chunks", "2",
            *extra,
        ]
        # The script intentionally refuses a windowed Smash launch without a
        # display. Provide a fake session name; our stub never opens a window.
        env["DISPLAY"] = ":unit-test"
        return subprocess.run(cmd, cwd=self.root, env=env, text=True,
                              capture_output=True, timeout=40)

    def test_build_before_perf_and_direct_runtime_with_complete_bundle(self):
        completed = self.capture()
        self.assertEqual(completed.returncode, 0, completed.stderr[-4500:])
        calls = self.logfile.read_text().splitlines()
        self.assertEqual(calls[0], "cargo build -p ambition_app_tools")
        self.assertEqual(calls[1], "perf record")
        self.assertTrue(calls[2].startswith("game --window --fighters 4 --seconds 1 "), calls)
        self.assertIn("--fighters 4 --cast mirror --stocks 40 --character performer", calls[2])
        self.assertFalse(any("launcher ran inside capture" in row for row in calls))
        bundle = self.root / "profiles/desktop-unit"
        self.assertEqual((bundle / "warm-build.status").read_text().strip(), "0")
        self.assertEqual((bundle / "perf-record.status").read_text().strip(), "0")
        summary = (bundle / "summary.md").read_text()
        self.assertIn("All-CPU roster in log: **yes**", summary)
        self.assertIn("Completed measured window: **yes**", summary)
        self.assertIn("VSync and frame cap were off in all sampled rows", summary)
        meta = (bundle / "metadata.txt").read_text()
        self.assertIn("profile_uncapped=yes", meta)
        self.assertIn("smash_cast=mirror", meta)
        self.assertIn("smash_stocks=40", meta)
        self.assertIn("smash_characters=performer,performer,performer,performer", meta)
        self.assertIn("scenario_id=smash-match-4p-mirror-performer-40stocks", meta)
        self.assertIn("direct-built-binary", summary)
        self.assertTrue((self.root / "profiles/desktop-unit.tar.gz").is_file())
        # The local directory retains raw captures, but the uploadable
        # archive excludes them by default.
        self.assertTrue((bundle / "perf.data").is_file())
        self.assertFalse((bundle / "tracy.trace").exists())
        with tarfile.open(self.root / "profiles/desktop-unit.tar.gz", "r:gz") as archive:
            names = archive.getnames()
            self.assertTrue(any(name.endswith("/summary.md") for name in names))
            self.assertFalse(any(name.endswith("/perf.data") for name in names))
            self.assertFalse(any(name.endswith("/tracy.trace") for name in names))
        self.assertIn("| `tracy.trace` | the raw Tracy trace, for the GUI | no | no |", summary)

    def test_mixed_cast_has_distinct_character_ids_and_history_identity(self):
        completed = self.capture("--smash-cast", "mixed-v1")
        self.assertEqual(completed.returncode, 0, completed.stderr[-4500:])
        calls = self.logfile.read_text().splitlines()
        self.assertIn("--fighters 4 --cast mixed-v1 --stocks 40", calls[2])
        self.assertNotIn("--character", calls[2])
        meta = (self.root / "profiles/desktop-unit/metadata.txt").read_text()
        self.assertIn("smash_cast=mixed-v1", meta)
        self.assertIn("smash_characters=performer,player_robot_v3,sanic,projectile_polygon", meta)
        self.assertIn("smash_cpu_level=3", meta)
        self.assertIn("scenario_id=smash-match-4p-mixed-v1-40stocks", meta)

    def test_custom_stocks_and_mirror_character_change_workload_identity(self):
        completed = self.capture("--smash-character", "sanic", "--smash-stocks", "60")
        self.assertEqual(completed.returncode, 0, completed.stderr[-4500:])
        calls = self.logfile.read_text().splitlines()
        self.assertIn("--cast mirror --stocks 60 --character sanic", calls[2])
        meta = (self.root / "profiles/desktop-unit/metadata.txt").read_text()
        self.assertIn("smash_characters=sanic,sanic,sanic,sanic", meta)
        self.assertIn("scenario_id=smash-match-4p-mirror-sanic-60stocks", meta)

    def test_invalid_cast_rejected_before_compilation(self):
        completed = self.capture("--smash-cast", "some-other-cast")
        self.assertNotEqual(completed.returncode, 0)
        self.assertIn("--smash-cast must be mirror or mixed-v1", completed.stderr)
        self.assertFalse(self.logfile.exists())

    def test_respect_video_settings_opt_out(self):
        completed = self.capture("--respect-video-settings")
        self.assertEqual(completed.returncode, 0, completed.stderr[-4500:])
        bundle = self.root / "profiles/desktop-unit"
        summary = (bundle / "summary.md").read_text()
        self.assertIn("REFRESH/CAP-LIMITED BENCHMARK", summary)
        self.assertIn("profile_uncapped=no", (bundle / "metadata.txt").read_text())

    def test_include_raw_data_is_explicit_opt_in(self):
        completed = self.capture("--include-raw-data")
        self.assertEqual(completed.returncode, 0, completed.stderr[-4500:])
        with tarfile.open(self.root / "profiles/desktop-unit.tar.gz", "r:gz") as archive:
            names = archive.getnames()
            self.assertTrue(any(name.endswith("/perf.data") for name in names))
            self.assertFalse(any(name.endswith("/tracy.trace") for name in names))
        summary = (self.root / "profiles/desktop-unit/summary.md").read_text()
        self.assertIn("| `tracy.trace` | the raw Tracy trace, for the GUI | no | no |", summary)

    def test_timed_smash_runs_clean_native_then_tracy_diagnostic_in_one_archive(self):
        completed = self.capture_with_tracy()
        self.assertEqual(completed.returncode, 0, completed.stderr[-5500:])
        calls = self.logfile.read_text().splitlines()
        native_build = next(i for i, row in enumerate(calls) if row == "cargo build -p ambition_app_tools")
        native_perf = calls.index("perf record")
        native_game = next(i for i, row in enumerate(calls) if row.startswith("game "))
        tracy_build = next(i for i, row in enumerate(calls) if row == "cargo build -p ambition_app_tools --features profile")
        tracy_game = next(i for i, row in enumerate(calls) if row.startswith("game ") and i > native_game)
        self.assertLess(native_build, native_perf)
        self.assertLess(native_perf, native_game)
        self.assertLess(native_game, tracy_build)
        self.assertLess(tracy_build, tracy_game)
        self.assertEqual(calls.count("perf record"), 1, calls)
        self.assertIn("tracy capture", calls)
        self.assertGreater(calls.index("tracy capture"), tracy_build)
        bundle = self.root / "profiles/desktop-unit"
        self.assertEqual((bundle / "warm-build.status").read_text().strip(), "0")
        self.assertEqual((bundle / "tracy_diagnostic/warm-build.status").read_text().strip(), "0")
        self.assertEqual((bundle / "tracy-diagnostic.status").read_text().strip(), "0")
        self.assertTrue((bundle / "tracy.trace").is_symlink())
        self.assertTrue((bundle / "tracy_zones.csv").is_file())
        self.assertTrue((bundle / "perf-report-active.txt").is_file(),
                        (bundle / "match-window.json").read_text() + completed.stderr[-1200:])
        window = (bundle / "match-window.json").read_text()
        self.assertIn('"status": "ok"', window)
        summary = (bundle / "summary.md").read_text()
        self.assertIn("Two-pass timed Smash capture", summary)
        self.assertIn("The `perf-report-active*.txt` tables exclude startup", summary)
        with tarfile.open(self.root / "profiles/desktop-unit.tar.gz", "r:gz") as archive:
            paths = archive.getnames()
            self.assertFalse(any(name.endswith("tracy.trace") for name in paths))
            self.assertFalse(any(name.endswith("perf.data") for name in paths))
            self.assertTrue(any(name.endswith("tracy_summary.md") for name in paths))
            self.assertTrue(any(name.endswith("perf-report-active.txt") for name in paths))

    def capture_with_tracy(self, *extra):
        # Preserve the default caller interface; drop only --no-tracy.
        env = dict(os.environ)
        env.update(FAKE_CALL_LOG=str(self.logfile), FAKE_REPO=str(self.root),
                   FAKE_PROFILE_SLOW_GAME="1", DISPLAY=":unit-test")
        env["PATH"] = f"{self.bin}{os.pathsep}{env.get('PATH', '')}"
        cmd = [
            str(self.root / "scripts/measure/profile_desktop.sh"),
            "--smash", "--smash-fighters", "4", "--smash-seconds", "1",
            "--name", "unit", "--out", str(self.root / "profiles"),
            "--no-record", "--chunks", "2", *extra,
        ]
        return subprocess.run(cmd, cwd=self.root, env=env, text=True,
                              capture_output=True, timeout=70)

    def test_failed_build_never_starts_perf_but_packages_diagnostics(self):
        # Use a wrapper for the environment variable without changing the
        # test stub's CLI contract.
        old = os.environ.get("FAKE_BUILD_FAIL")
        os.environ["FAKE_BUILD_FAIL"] = "1"
        try:
            completed = self.capture()
        finally:
            if old is None:
                os.environ.pop("FAKE_BUILD_FAIL", None)
            else:
                os.environ["FAKE_BUILD_FAIL"] = old
        self.assertNotEqual(completed.returncode, 0)
        self.assertEqual(self.logfile.read_text().splitlines(), ["cargo build -p ambition_app_tools"])
        bundle = self.root / "profiles/desktop-unit"
        self.assertEqual((bundle / "warm-build.status").read_text().strip(), "17")
        self.assertIn("simulated compiler error", (bundle / "warm-build.stderr").read_text())
        self.assertTrue((bundle / "capture.skipped").exists())
        self.assertTrue((bundle / "summary.md").exists())
        self.assertTrue((self.root / "profiles/desktop-unit.tar.gz").exists())

    def test_repeating_named_capture_preserves_first_bundle(self):
        first = self.capture()
        self.assertEqual(first.returncode, 0, first.stderr[-1800:])
        first_bundle = self.root / "profiles/desktop-unit"
        first_summary = (first_bundle / "summary.md").read_bytes()
        second = self.capture()
        self.assertEqual(second.returncode, 0, second.stderr[-1800:])
        dirs = sorted(p for p in (self.root / "profiles").glob("desktop-unit*") if p.is_dir())
        self.assertEqual(len(dirs), 2, dirs)
        self.assertEqual((first_bundle / "summary.md").read_bytes(), first_summary)
        for folder in dirs:
            self.assertTrue((folder / "summary.md").exists())
            self.assertTrue(Path(f"{folder}.tar.gz").exists())

    def test_real_launcher_plan_exports_autoplay_runtime_settings(self):
        env = dict(os.environ)
        env.pop("AMBITION_AUTOPLAY", None)
        env.pop("AMBITION_BOOT_ROUTE", None)
        result = subprocess.run(
            [str(REPO / "run_game.sh"), "--print-plan", "profiling", "--autoplay"],
            cwd=REPO, env=env, text=True, capture_output=True, timeout=10,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        rows = result.stdout.splitlines()
        self.assertTrue(any(row.startswith("runtime_env=AMBITION_AUTOPLAY=") for row in rows), rows)
        self.assertTrue(any(row.startswith("runtime_env=AMBITION_BOOT_ROUTE=") for row in rows), rows)

    def test_no_warm_build_requires_existing_binary(self):
        completed = self.capture("--no-warm-build")
        self.assertNotEqual(completed.returncode, 0)
        self.assertFalse(self.logfile.exists())
        bundle = self.root / "profiles/desktop-unit"
        self.assertIn("missing", (bundle / "capture.skipped").read_text())
        self.assertTrue((self.root / "profiles/desktop-unit.tar.gz").is_file())


if __name__ == "__main__":
    unittest.main()
