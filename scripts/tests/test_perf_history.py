"""Tests for the runtime frame-cost history: ingest, refusal, and comparison.

Two refusals carry the design and both are exercised against synthetic bundles
built here rather than mocks, because what is being tested is the reading of
files the profiling front door actually writes:

⛔ a bundle whose game never started must append NOTHING — a failed warm build
   leaves a complete-looking bundle directory, and a row of zeroes for it would
   read as a spectacular improvement;
⛔ two records whose comparability fields differ must not be subtracted, and the
   refusal must NAME the field.

The fixture bundle is a faithful miniature of a real one: the same file names,
the same `key=value` metadata, the same census CSV headers, and the same
`tracy-csvexport` quirk of writing zone names unquoted with commas in them.
"""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

import pytest

# Runtime history is a self-contained measurement tool, not an Ambition runtime
# feature. Repo-wide validation excludes detached-tool tests; run them
# explicitly after editing the tool with `./run_tests.sh --tool-tests`.
pytestmark = pytest.mark.detached_tool

SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS / "lib"))

import profile_bundle_to_history as hist  # noqa: E402

PERF_HISTORY = SCRIPTS / "perf_history.py"

METADATA = {
    "mode": "timeline-run",
    "utc_stamp": "20260901T101112Z",
    "output_dir": "/somewhere/target/profiles/desktop-timeline-run-20260901T101112Z",
    "duration_seconds": "until-game-exits",
    "sampling_frequency_hz": "99",
    "cargo_profile": "profiling",
    "profile_dir": "profiling",
    "cargo_features": "profile",
    "package": "ambition_app",
    "binary": "ambition_game_bin",
    "binary_path": "/somewhere/target/profiling/ambition_game_bin",
    "launch_plan_status": "ok",
    "headless": "yes",
    "headless_ticks": "1800",
    "headless_scenario": "sandbox",
    "tracy_requested": "yes",
    "census_enabled": "yes",
    "census_hz": "1",
    "run_command": "/somewhere/run_game.sh profiling --features profile ",
    "script_command": "./scripts/profile_desktop.sh --headless ",
    "hostname": "testbox",
    "uname": "Linux testbox 6.8.0-110-generic #110-Ubuntu SMP x86_64 GNU/Linux",
    "rust_target": "x86_64-unknown-linux-gnu",
    "rustc_version": "rustc 1.97.1 (8bab26f4f 2026-07-14)",
    "git_head": "1234567890abcdef1234567890abcdef12345678",
    "git_head_short": "1234567890ab",
    "git_branch": "main",
    "perf_events": "task-clock,cycles",
}

HOST_ENVIRONMENT = """## CPU / memory
model name\t: Test CPU @ 1.00GHz
logical_cpus=6
MemTotal:       65841164 kB

## Session
XDG_SESSION_TYPE=<unset>
"""

FRAME_TIMES = """wall_s,t,frames,mean,p50,p95,p99,min,max
5.0,1.000,100,2.00,1.90,3.00,4.00,1.00,9.00
6.0,2.000,300,1.00,0.90,1.50,2.00,0.50,3.00
"""

SCHEDULE_PHASES = """wall_s,t,frames,First,Update,Last,outside
5.0,1.000,100,0.100,2.000,0.050,0.100
6.0,2.000,300,0.100,1.000,0.050,0.100
"""

SCHEDULE_CENSUS = """wall_s,t,schedules,systems,Update,PostUpdate
5.0,1.000,11,870,820,15
6.0,2.000,11,876,822,15
"""

RUNTIME_CENSUS = """wall_s,t,entities,archetypes,components,bodies,players
5.0,1.000,60,80,2000,2,1
6.0,2.000,64,85,2087,2,1
"""

FRAME_SPIKES = """wall_s,game_s,frame_ms
5.5,1.500,40.0
5.9,1.900,120.0
"""

# ⛔ Row two carries a Rust generic in the zone name, so it holds MORE commas
# than the header has columns. `tracy-csvexport` writes it exactly like this,
# and a naive DictReader shifts `counts` into `src_line`.
TRACY_ZONES = """name,src_file,src_line,total_ns,total_perc,counts,mean_ns,min_ns,max_ns,std_ns
system{name="a::b"},lib.rs,10,1000000,10.0,400,2500,100,9000,50
check_conditions{name="Assets<Ext<A, B>>::track"},lib.rs,11,500000,5.0,800,625,50,4000,20
system_commands{name="a::b"},lib.rs,12,250000,2.5,200,1250,20,2000,10
plugin build{name="X"},lib.rs,13,900000,9.0,1,900000,900000,900000,0
"""

PERF_BY_THREAD = """# Overhead  Command
#
    40.00%  ambition_game_b
    25.00%  Tracy Symbol Wo
    10.00%  Tracy Profiler
"""

GAME_LOG = """[    1.000s] [census] frame t=1.000 frames=100 mean=2.00 p50=1.90 p95=3.00 p99=4.00 min=1.00 max=9.00
[    2.000s] [census] ecs t=2.000 entities=64 archetypes=85 components=2087 bodies=2 players=1
[    3.500s] the session is running
"""


def write_bundle(root: Path, name: str, **overrides) -> Path:
    """A miniature of a real bundle: same filenames, same formats."""
    meta = {**METADATA, **overrides.pop("metadata", {})}
    bundle = root / name
    bundle.mkdir(parents=True)
    lines = [f"{key}={value}" for key, value in meta.items()]
    lines += ["git_status_porcelain_begin", "git_status_porcelain_end"]
    (bundle / "metadata.txt").write_text("\n".join(lines) + "\n")
    (bundle / "host-environment.txt").write_text(HOST_ENVIRONMENT)
    (bundle / "game-stderr-stamped.txt").write_text(GAME_LOG)
    (bundle / "warm-build.status").write_text("0\n")
    (bundle / "perf-record.status").write_text("0\n")
    (bundle / "perf.data").write_bytes(b"\x00" * 64)
    files = {
        "frame_times.csv": FRAME_TIMES,
        "frame_spikes.csv": FRAME_SPIKES,
        "schedule_phases.csv": SCHEDULE_PHASES,
        "schedule_census.csv": SCHEDULE_CENSUS,
        "runtime_census.csv": RUNTIME_CENSUS,
        "tracy_zones.csv": TRACY_ZONES,
        "perf-report-by-thread.txt": PERF_BY_THREAD,
    }
    files.update(overrides.pop("files", {}))
    for filename, content in files.items():
        if content is None:
            continue
        (bundle / filename).write_text(content)
    assert not overrides, f"unused overrides: {sorted(overrides)}"
    # The front door derives metadata.json from metadata.txt; do the same rather
    # than hand-writing a second copy that could disagree with the first.
    subprocess.run(
        [sys.executable, str(SCRIPTS / "lib" / "profile_metadata_json.py"), str(bundle)],
        check=True,
    )
    return bundle


@pytest.fixture()
def bundle(tmp_path: Path) -> Path:
    return write_bundle(tmp_path, "desktop-timeline-run-20260901T101112Z")


# ── ingest ────────────────────────────────────────────────────────────────


@pytest.mark.detached_tool
def test_one_hostname_on_two_machines_does_not_become_one_machine(tmp_path):
    """⛔⛔ THE HOSTNAME IS NOT THE MACHINE, and this is the failure that proves it.

    Two different boxes have both answered to `aivm-2404` -- an i7-7700HQ with 6
    logical CPUs and an i9-11900K with 12 -- and a timing baseline recorded on
    one was read as belonging to the other. Keying on the hostname makes those
    two rows compare; keying on `/etc/machine-id` refuses them.
    """
    first = write_bundle(
        tmp_path / "one",
        "run-one",
        metadata={"hostname": "aivm-2404", "machine_id": "aaaa1111"},
    )
    second = write_bundle(
        tmp_path / "two",
        "run-two",
        metadata={"hostname": "aivm-2404", "machine_id": "bbbb2222"},
    )

    one = hist.build_record(str(first))
    two = hist.build_record(str(second))

    assert one["host"]["machine_id"] == "aaaa1111"
    assert one["host"]["machine_id_source"] == "machine-id"
    assert one["comparable_key"] != two["comparable_key"], (
        "two machines sharing a hostname must not share a comparability key"
    )


def test_ingest_reads_the_derived_artifacts_not_the_raw_trace(bundle: Path):
    record = hist.build_record(str(bundle))

    # Frame statistics are weighted by the frames in each census window: the
    # 300-frame window must outvote the 100-frame one.
    assert record["frame_ms"]["mean"] == pytest.approx((2.0 * 100 + 1.0 * 300) / 400)
    assert record["frame_ms"]["max"] == 9.0, "max is exact, not a weighted mean"
    assert record["frame_ms"]["min"] == 0.5
    assert record["run"]["frames"] == 400

    # Phases are per-frame means, also frame-weighted, and the phase list comes
    # off the row so a new phase needs no edit here.
    assert record["phases_ms"]["Update"] == pytest.approx((2.0 * 100 + 1.0 * 300) / 400)
    assert set(record["phases_ms"]) == {"First", "Update", "Last", "outside", "_budget_ms"}

    # The settled schedule population, plus the per-schedule breakdown that
    # turns "Update is 65% of the frame" into an answerable question.
    assert record["scheduler"]["registered_systems"] == 876
    assert record["scheduler"]["schedules"] == 11
    assert record["scheduler"]["per_schedule"] == {"Update": 822.0, "PostUpdate": 15.0}

    assert record["scene"] == {
        "entities": 64.0, "entities_peak": 64.0,
        "archetypes": 85.0, "archetypes_peak": 85.0,
        "components": 2087.0, "bodies": 2.0, "bodies_peak": 2.0, "players": 1.0,
    }
    assert record["spikes"]["count"] == 2
    assert record["spikes"]["worst_ms"] == 120.0
    assert record["spikes"]["per_1000_frames"] == pytest.approx(5.0)

    # ⛔ This fixture predates the front door recording `/etc/machine-id`, so the
    # id falls back to the hostname AND SAYS SO. A bare "testbox" here would mean
    # a weak id was being passed off as a real one.
    assert record["host"]["machine_id"] == "hostname:testbox"
    assert record["host"]["machine_id_source"] == "hostname"
    assert record["host"]["cpu_model"] == "Test CPU @ 1.00GHz"
    assert record["host"]["logical_cpus"] == 6
    assert record["host"]["mem_total_kb"] == 65841164
    assert record["host"]["kernel"] == "6.8.0-110-generic"
    assert record["commit"] == "1234567890ab"
    assert record["dirty"] is False
    assert record["measured_at"] == "2026-09-01T10:11:12Z"
    assert record["bundle"]["name"] == bundle.name


def test_zone_counts_survive_a_comma_bearing_rust_generic(bundle: Path):
    """⛔ The `tracy-csvexport` quirk that silently multiplies counts.

    A `check_conditions` zone name holds a Rust generic with a comma in it and
    is written unquoted, so a plain DictReader reads `src_line` as `counts` and
    the run-condition rate comes out thousands of times too high — wrong in a
    way that looks exactly like a finding.
    """
    scheduler = hist.build_record(str(bundle))["scheduler"]
    assert scheduler["system_executions_total"] == 400
    assert scheduler["run_condition_evaluations_total"] == 800
    assert scheduler["command_flushes_total"] == 200
    assert scheduler["run_condition_evaluations_per_frame"] == pytest.approx(2.0)
    assert scheduler["per_frame_divisor"] == 400


def test_a_headless_run_records_headless_rather_than_unknown(bundle: Path):
    record = hist.build_record(str(bundle))
    assert record["gpu"]["rendering"] == "headless"
    assert record["display"]["resolution"] == "headless"
    assert record["scenario"]["id"] == "sandbox"
    assert record["scenario"]["headless"] is True
    assert record["scenario"]["ticks"] == 1800


def test_missing_optional_instruments_degrade_to_null_not_zero(tmp_path: Path):
    """A `--no-tracy` bundle is still a legitimate frame measurement.

    ⛔ Zero would read as "the scheduler executed no systems". Null reads as
    "nothing here measured it", which is the true statement.
    """
    bundle = write_bundle(
        tmp_path,
        "no-instruments",
        metadata={"tracy_requested": "no", "cargo_features": ""},
        files={"tracy_zones.csv": None, "perf-report-by-thread.txt": None,
               "schedule_phases.csv": None},
    )
    record = hist.build_record(str(bundle))

    assert record["instruments"]["tracy"] is False
    assert record["instruments"]["profiler_cycle_share_pct"] is None
    assert record["scheduler"]["system_executions_total"] is None
    assert record["scheduler"]["run_condition_evaluations_per_frame"] is None
    assert record["phases_ms"] is None
    # The frame series, which needs no profiler at all, is still there.
    assert record["frame_ms"]["mean"] == pytest.approx(1.25)
    assert record["scheduler"]["registered_systems"] == 876


def test_the_profiler_share_is_summed_across_tracys_threads(bundle: Path):
    record = hist.build_record(str(bundle))
    assert record["instruments"]["profiler_cycle_share_pct"] == pytest.approx(35.0)
    assert record["instruments"]["tracy"] is True


def test_p90_stays_null_because_the_census_does_not_emit_one(bundle: Path):
    """A series cannot be back-filled with a dimension it never recorded."""
    frame = hist.build_record(str(bundle))["frame_ms"]
    assert frame["p90"] is None
    assert "p90" in frame["p90_note"]


# ── the died-before-start refusal ─────────────────────────────────────────


def test_a_failed_warm_build_is_refused_and_writes_nothing(tmp_path: Path):
    bundle = write_bundle(tmp_path, "build-failed")
    (bundle / "warm-build.status").write_text("101\n")
    (bundle / "warm-build.stderr").write_text(
        "error: the package 'x' does not contain this feature: profile\n"
    )
    with pytest.raises(hist.BundleDiedBeforeStart, match="BUILD FAILURE"):
        hist.build_record(str(bundle))

    ledger = tmp_path / "series.jsonl"
    result = subprocess.run(
        [sys.executable, str(SCRIPTS / "lib" / "profile_bundle_to_history.py"),
         str(bundle), "--ledger", str(ledger)],
        capture_output=True, text=True,
    )
    assert result.returncode == 3
    assert "⛔" in result.stderr
    assert "101" in result.stderr
    assert not ledger.exists(), "a build failure must leave the series untouched"


def test_a_bundle_that_never_launched_is_refused(tmp_path: Path):
    """The interrupted-build shape: metadata and a host environment, nothing else."""
    bundle = write_bundle(tmp_path, "never-launched")
    for name in ("perf-record.status", "warm-build.status", "frame_times.csv",
                 "game-stderr-stamped.txt"):
        (bundle / name).unlink()
    with pytest.raises(hist.BundleDiedBeforeStart, match="no capture status file"):
        hist.build_record(str(bundle))


def test_a_launch_with_no_frame_series_is_refused(tmp_path: Path):
    """The game logged and then died during App construction."""
    bundle = write_bundle(tmp_path, "no-frames", files={"frame_times.csv": None})
    with pytest.raises(hist.BundleDiedBeforeStart, match="no frame series"):
        hist.build_record(str(bundle))


def test_a_frame_series_reporting_zero_frames_is_refused(tmp_path: Path):
    bundle = write_bundle(
        tmp_path, "zero-frames",
        files={"frame_times.csv": "wall_s,t,frames,mean,p50,p95,p99,min,max\n5.0,1.0,0,0,0,0,0,0,0\n"},
    )
    with pytest.raises(hist.BundleDiedBeforeStart, match="zero frames"):
        hist.build_record(str(bundle))


# ── comparability ─────────────────────────────────────────────────────────


def seed(tmp_path: Path, ledger: Path, *bundles) -> None:
    with ledger.open("w", encoding="utf-8") as handle:
        for record in bundles:
            handle.write(json.dumps(record, sort_keys=True) + "\n")


def run_tool(ledger: Path, *args: str) -> subprocess.CompletedProcess:
    return subprocess.run(
        [sys.executable, str(PERF_HISTORY), "--ledger", str(ledger), *args],
        capture_output=True, text=True,
    )


def test_the_same_setup_on_two_commits_shares_a_comparability_key(tmp_path: Path):
    old = hist.build_record(str(write_bundle(tmp_path, "old")))
    new = hist.build_record(
        str(write_bundle(tmp_path, "new",
                         metadata={"git_head_short": "ffffffffffff",
                                   "utc_stamp": "20260902T101112Z"}))
    )
    assert old["comparable_key"] == new["comparable_key"]
    # ⭐ The commit is the axis you compare ALONG; it must never be in the key.
    assert "commit" not in old["comparable_fields"]


@pytest.mark.parametrize(
    "overrides, expected_field",
    [
        # A lavapipe run must never group with a hardware-GPU run. Headless is
        # the third state and is its own group again.
        ({"headless": "no"}, "gpu.rendering"),
        # Dropping --features profile is what makes a Tracy run 9x slower than
        # an unprofiled one on this project.
        ({"cargo_features": ""}, "build.features"),
        ({"hostname": "otherbox"}, "host.machine_id"),
        ({"headless_scenario": "smash"}, "scenario.id"),
        ({"cargo_profile": "dev"}, "build.cargo_profile"),
        ({"census_hz": "10"}, "instruments.census_hz"),
        ({"package": "ambition_demo_smash_app"}, "build.package"),
    ],
)
def test_compare_refuses_across_a_key_difference_and_names_the_field(
    tmp_path: Path, overrides: dict, expected_field: str
):
    left = hist.build_record(str(write_bundle(tmp_path, "left")))
    right = hist.build_record(str(write_bundle(tmp_path, "right", metadata=overrides)))
    assert left["comparable_key"] != right["comparable_key"]

    ledger = tmp_path / "series.jsonl"
    seed(tmp_path, ledger, left, right)
    result = run_tool(ledger, "compare", "left", "right")
    # Exit 2, not the 1 a genuine regression returns: a caller must be able to
    # tell "slower" from "you asked for a meaningless subtraction".
    assert result.returncode == 2, result.stdout + result.stderr
    assert "refusing to compare" in result.stderr
    assert expected_field in result.stderr


def test_a_software_renderer_never_groups_with_a_hardware_one(tmp_path: Path):
    software = (
        '[    1.000s] AdapterInfo { name: "llvmpipe (LLVM 17)", vendor: 65541, '
        "device: 0, device_type: Cpu, driver: \"llvmpipe\", driver_info: \"\", "
        "backend: Vulkan }\n" + GAME_LOG
    )
    hardware = (
        '[    1.000s] AdapterInfo { name: "NVIDIA GeForce RTX 3090", vendor: 4318, '
        "device: 8708, device_type: DiscreteGpu, driver: \"NVIDIA\", "
        "driver_info: \"595.84\", backend: Vulkan }\n" + GAME_LOG
    )
    windowed = {"headless": "no", "script_command": "./scripts/profile_desktop.sh "}
    lavapipe = hist.build_record(
        str(write_bundle(tmp_path, "lavapipe", metadata=windowed,
                         files={"game-stderr-stamped.txt": software}))
    )
    gpu = hist.build_record(
        str(write_bundle(tmp_path, "gpu", metadata=windowed,
                         files={"game-stderr-stamped.txt": hardware}))
    )
    assert lavapipe["gpu"]["rendering"] == "software"
    assert gpu["gpu"]["rendering"] == "hardware"
    assert lavapipe["comparable_key"] != gpu["comparable_key"]

    ledger = tmp_path / "series.jsonl"
    seed(tmp_path, ledger, lavapipe, gpu)
    result = run_tool(ledger, "compare", "lavapipe", "gpu")
    assert result.returncode == 2
    assert "gpu.rendering" in result.stderr


# ── comparison output ─────────────────────────────────────────────────────


def slower(record: dict, factor: float) -> dict:
    record = json.loads(json.dumps(record))
    for key in ("mean", "p50", "p95", "p99", "max"):
        if record["frame_ms"][key] is not None:
            record["frame_ms"][key] *= factor
    return record


def test_a_regression_over_the_threshold_is_flagged_and_exits_one(tmp_path: Path):
    base = hist.build_record(str(write_bundle(tmp_path, "base")))
    new = slower(
        hist.build_record(
            str(write_bundle(tmp_path, "newer",
                             metadata={"git_head_short": "ffffffffffff",
                                       "utc_stamp": "20260902T101112Z"}))
        ),
        1.20,
    )
    ledger = tmp_path / "series.jsonl"
    seed(tmp_path, ledger, base, new)

    result = run_tool(ledger, "compare", "base", "newer")
    assert result.returncode == 1, result.stdout
    assert "REGRESSION" in result.stdout
    assert "+20.0%" in result.stdout

    # A 20% move is under a 25% threshold and must not be flagged.
    tolerant = run_tool(ledger, "--threshold", "25", "compare", "base", "newer")
    assert tolerant.returncode == 0, tolerant.stdout
    assert "REGRESSION" not in tolerant.stdout


def test_a_tracy_row_carries_its_observer_effect_into_the_comparison(tmp_path: Path):
    base = hist.build_record(str(write_bundle(tmp_path, "base")))
    new = hist.build_record(
        str(write_bundle(tmp_path, "newer",
                         metadata={"git_head_short": "ffffffffffff",
                                   "utc_stamp": "20260902T101112Z"}))
    )
    ledger = tmp_path / "series.jsonl"
    seed(tmp_path, ledger, base, new)
    result = run_tool(ledger, "compare", "base", "newer")
    assert result.returncode == 0, result.stdout
    assert "ran under Tracy" in result.stdout
    assert "35.0% of sampled cycles" in result.stdout


def test_latest_compares_within_the_baselines_group_only(tmp_path: Path):
    base = hist.build_record(str(write_bundle(tmp_path, "base")))
    peer = hist.build_record(
        str(write_bundle(tmp_path, "peer",
                         metadata={"git_head_short": "ffffffffffff",
                                   "utc_stamp": "20260902T101112Z"}))
    )
    # A newer row in a DIFFERENT group. Taking the globally newest record would
    # make `latest` refuse every time somebody profiled another scenario.
    stranger = hist.build_record(
        str(write_bundle(tmp_path, "stranger",
                         metadata={"headless_scenario": "smash",
                                   "utc_stamp": "20260903T101112Z"}))
    )
    ledger = tmp_path / "series.jsonl"
    seed(tmp_path, ledger, base, peer, stranger)

    result = run_tool(ledger, "latest", "--against", "base")
    assert result.returncode == 0, result.stdout + result.stderr
    assert "base → peer" in result.stdout
    assert "stranger" not in result.stdout


def test_latest_refuses_when_nothing_since_shares_the_group(tmp_path: Path):
    base = hist.build_record(str(write_bundle(tmp_path, "base")))
    stranger = hist.build_record(
        str(write_bundle(tmp_path, "stranger",
                         metadata={"headless_scenario": "smash",
                                   "utc_stamp": "20260903T101112Z"}))
    )
    ledger = tmp_path / "series.jsonl"
    seed(tmp_path, ledger, base, stranger)
    result = run_tool(ledger, "latest", "--against", "base")
    assert result.returncode == 2
    assert "shares its group" in result.stderr


def test_scenario_and_list_separate_the_groups(tmp_path: Path):
    traced = hist.build_record(str(write_bundle(tmp_path, "traced")))
    untraced = hist.build_record(
        str(write_bundle(tmp_path, "untraced", metadata={"cargo_features": ""},
                         files={"tracy_zones.csv": None}))
    )
    ledger = tmp_path / "series.jsonl"
    seed(tmp_path, ledger, traced, untraced)

    scenario = run_tool(ledger, "scenario", "sandbox")
    assert scenario.returncode == 0
    assert "2 record(s) in 2 group(s)" in scenario.stdout
    assert "NOT comparable" in scenario.stdout

    listing = run_tool(ledger, "list")
    assert "2 comparability group(s)" in listing.stdout


def test_the_markdown_report_names_every_group(tmp_path: Path):
    traced = hist.build_record(str(write_bundle(tmp_path, "traced")))
    untraced = hist.build_record(
        str(write_bundle(tmp_path, "untraced", metadata={"cargo_features": ""},
                         files={"tracy_zones.csv": None}))
    )
    ledger = tmp_path / "series.jsonl"
    seed(tmp_path, ledger, traced, untraced)
    out = tmp_path / "report.md"
    result = run_tool(ledger, "report", "-o", str(out))
    assert result.returncode == 0, result.stderr
    text = out.read_text()
    assert "# Runtime frame cost" in text
    assert traced["comparable_label"] in text
    assert untraced["comparable_label"] in text
    assert "may only be compared WITHIN a group" in text


# ── the seeded baseline ───────────────────────────────────────────────────


def test_the_shipped_baseline_rows_are_marked_as_transcribed():
    """The two seeded rows describe the same commit under different instruments.

    They exist to be the first row of a real series, and they must be readable
    as prose transcriptions rather than as machine-extracted measurements.
    """
    rows = hist.load(hist.LEDGER)
    if not rows:
        pytest.skip("measurements submodule not checked out")
    seeded = [row for row in rows if row.get("provenance", {}).get("backfilled")]
    assert seeded, "the baseline rows should be marked backfilled"
    for row in seeded:
        assert "performance-and-iteration.md" in row["provenance"]["recorded_from"]
        assert row["provenance"]["transcribed_from_prose"]
        assert row["provenance"]["caveats"]
        assert row["frame_ms"]["source"] == "prose"

    keys = {row["comparable_key"] for row in seeded}
    assert len(keys) == len(seeded), (
        "the Tracy and --no-tracy baselines are 9x apart and must not share a group"
    )
