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
cat <<EOF
package=ambition_app_tools
binary=smash_match_profile
profile_dir=profiling
binary_path=$FAKE_REPO/target/profiling/smash_match_profile
features=profile
game_arg=--window
game_arg=--fighters
game_arg=4
game_arg=--seconds
game_arg=1
build_arg=cargo
build_arg=build
build_arg=-p
build_arg=ambition_app_tools
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
echo '[census] visual_quality t=1 profile=High frame_cap=Auto present_mode=Fifo' >&2
echo '[smash-profile] done seats_at_end=4' >&2
EOF
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
                printf 'synthetic-tracy-data\\n' > "$(dirname "$1")/tracy.trace" ;;
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
        self.assertTrue(calls[2].startswith("game --window --fighters 4 --seconds 1 runtime=from-launch-plan"), calls)
        self.assertFalse(any("launcher ran inside capture" in row for row in calls))
        bundle = self.root / "profiles/desktop-unit"
        self.assertEqual((bundle / "warm-build.status").read_text().strip(), "0")
        self.assertEqual((bundle / "perf-record.status").read_text().strip(), "0")
        summary = (bundle / "summary.md").read_text()
        self.assertIn("All-CPU roster in log: **yes**", summary)
        self.assertIn("Completed measured window: **yes**", summary)
        self.assertIn("REFRESH/CAP-LIMITED BENCHMARK", summary)
        self.assertIn("direct-built-binary", summary)
        self.assertTrue((self.root / "profiles/desktop-unit.tar.gz").is_file())
        # The local directory retains raw captures, but the uploadable
        # archive excludes them by default.
        self.assertTrue((bundle / "perf.data").is_file())
        self.assertTrue((bundle / "tracy.trace").is_file())
        with tarfile.open(self.root / "profiles/desktop-unit.tar.gz", "r:gz") as archive:
            names = archive.getnames()
            self.assertTrue(any(name.endswith("/summary.md") for name in names))
            self.assertFalse(any(name.endswith("/perf.data") for name in names))
            self.assertFalse(any(name.endswith("/tracy.trace") for name in names))
        self.assertIn("| `tracy.trace` | the raw Tracy trace, for the GUI | yes | no |", summary)

    def test_include_raw_data_is_explicit_opt_in(self):
        completed = self.capture("--include-raw-data")
        self.assertEqual(completed.returncode, 0, completed.stderr[-4500:])
        with tarfile.open(self.root / "profiles/desktop-unit.tar.gz", "r:gz") as archive:
            names = archive.getnames()
            self.assertTrue(any(name.endswith("/perf.data") for name in names))
            self.assertTrue(any(name.endswith("/tracy.trace") for name in names))
        summary = (self.root / "profiles/desktop-unit/summary.md").read_text()
        self.assertIn("| `tracy.trace` | the raw Tracy trace, for the GUI | yes | yes |", summary)

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
