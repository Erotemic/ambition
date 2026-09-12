"""A failing job must record the lines that IDENTIFY its failure.

⛔⛤ **THIS GUARDS THE THING THE P0 FLAKY-`workspace` ROW SPENT WEEKS NOT
HAVING.** That row names three failing tests and, for one of them, no message
at all — and that victim contains NO ASSERTION, so it can only fail by
PANICKING and the panic text is the whole diagnosis. The row's remedy was an
instruction to a human: *"read every future gate failure and record its
message."* `FailureEvidence` makes the runner keep it instead.

⚠ EVERY ARM HERE IS ABOUT A PROPERTY THAT MATTERS, NOT ABOUT THE REGEX. The
collector is only useful if it (a) catches a panic printed FAR from the end of
a long run, (b) keeps the panic's MESSAGE and not just its location, and
(c) stays quiet on a green job — a collector that reports something for every
run is a collector nobody reads.
"""
import importlib.util
import pathlib
import sys

REPO = pathlib.Path(__file__).resolve().parents[2]
_spec = importlib.util.spec_from_file_location(
    "run_tests_for_evidence", REPO / "scripts" / "run_tests.py"
)
run_tests = importlib.util.module_from_spec(_spec)
# ⛔ REGISTERED BEFORE EXEC. `@dataclass` resolves annotations through
# `sys.modules[cls.__module__]`, so a module executed without being registered
# raises `AttributeError: 'NoneType' object has no attribute '__dict__'` from
# inside `dataclasses` — a failure that names neither this file nor run_tests.
sys.modules[_spec.name] = run_tests
_spec.loader.exec_module(run_tests)
FailureEvidence = run_tests.FailureEvidence


def collect(lines, **kw):
    ev = FailureEvidence(**kw)
    for line in lines:
        ev.feed(line)
    return ev.lines()


def test_a_panic_keeps_its_message_and_not_only_its_location():
    # libtest prints the location, then the message on the NEXT line. Keeping
    # only the first is the failure mode this arm exists for: a file:line with
    # no text names where, never what.
    got = collect([
        "thread 'a::b' (123) panicked at game/x.rs:171:5:\n",
        "only 48 frames had two seated bodies, so the match did not really run\n",
    ])
    assert any("panicked at game/x.rs:171:5" in line for line in got)
    assert any("only 48 frames" in line for line in got), got


def test_a_panic_printed_far_from_the_end_still_survives():
    # ⛔ THE REASON THIS IS NOT A TAIL SCAN. A panic is printed where it happens;
    # in a suite of thousands of tests that is a long way from the summary, and
    # the runner's existing 200-line tail would have dropped it.
    noise = [f"test some::test_{i} ... ok\n" for i in range(5_000)]
    got = collect(
        ["thread 'x' panicked at crates/y.rs:9:1:\n", "attempt to divide by zero\n"]
        + noise
        + ["test result: FAILED. 1 failed\n"]
    )
    assert any("divide by zero" in line for line in got), got


def test_a_green_run_records_nothing():
    # THE CONTROL, and it is the absence of the subject rather than another
    # instance of it: a passing run must produce an EMPTY list, or every status
    # file grows a section readers learn to skip.
    got = collect(
        [f"test some::test_{i} ... ok\n" for i in range(50)]
        + ["test result: ok. 50 passed; 0 failed\n"]
    )
    assert got == [], got


def test_the_collector_is_bounded():
    # A job that panics ten thousand times must not put ten thousand lines in
    # the status file.
    got = collect(
        ["thread 'x' panicked at a.rs:1:1:\n", "boom\n"] * 10_000, limit=8
    )
    assert len(got) == 8, len(got)


def test_the_failures_roster_and_assertion_text_are_kept():
    got = collect([
        "failures:\n",
        "    smash_cpu_cognition::both_emmy_seats_receive_one_cognitive_stream\n",
        "assertion `left == right` failed: the shipped sim installs it 0 time(s)\n",
    ])
    assert any(line.startswith("failures:") for line in got), got
    assert any("assertion `left == right` failed" in line for line in got), got


def test_a_failing_pytest_job_is_recorded_too():
    # ⛔⛤ THE ARM THAT WAS MISSING, AND ITS ABSENCE WAS NOT CAUGHT BY UNIT TESTS.
    # The first version of `FailureEvidence` was written from libtest output and
    # recorded NOTHING for a failing pytest job — found by running the runner
    # against a deliberately failing test and reading the status file. Roughly
    # half this gate's jobs are Python, so a collector that only speaks libtest
    # covers half the gate and reports success on the other half.
    got = collect([
        "=================================== FAILURES ==========================\n",
        "____________ test_zz_probe ____________\n",
        "    def test_zz_probe():\n",
        ">       raise AssertionError('the probe message')\n",
        "E       AssertionError: the probe message\n",
        "=========================== short test summary info ===================\n",
        "FAILED scripts/tests/test_x.py::test_zz_probe - AssertionError: the probe message\n",
        "1 failed, 140 passed in 7.06s\n",
    ])
    assert any("AssertionError: the probe message" in line for line in got), got
    assert any(line.startswith("FAILED scripts/tests") for line in got), got


def test_a_green_pytest_run_records_nothing():
    # The control for the arm above: widening the patterns must not make every
    # passing Python job grow a section.
    got = collect([
        "........................                                        [100%]\n",
        "140 passed, 1068 deselected in 7.06s\n",
    ])
    assert got == [], got


def test_the_real_p0_failure_would_have_been_recorded():
    # ⭐⭐ VERBATIM LINES FROM THE ACTUAL 2026-09-12 FAILURE, not a model of what
    # libtest prints. This is the exact failure the P0 flaky-`workspace` row
    # exists for, and the row recorded its NAME while the message survived only
    # because a human happened to be reading the terminal. Everything this arm
    # asserts is a line that run would have put in the status file.
    got = collect([
        "failures:\n",
        "\n",
        "---- smash_cpu_cognition::both_emmy_seats_receive_one_cognitive_stream_in_the_real_host stdout ----\n",
        "thread 'smash_cpu_cognition::both_emmy_seats_receive_one_cognitive_stream_in_the_real_host'"
        " (1087767) panicked at game/ambition_app/tests/smash_cpu_cognition.rs:171:5:\n",
        "only 48 frames had two seated bodies, so the match did not really run\n",
        "\n",
        "test result: FAILED. 90 passed; 1 failed; 3 ignored; 0 measured; 559 filtered out;"
        " finished in 184.32s\n",
    ])
    joined = "\n".join(got)
    # WHICH test, WHERE, WHY, and the tally -- the four things the row wanted.
    assert "both_emmy_seats_receive_one_cognitive_stream_in_the_real_host" in joined
    assert "smash_cpu_cognition.rs:171:5" in joined
    assert "only 48 frames had two seated bodies" in joined
    assert "test result: FAILED" in joined
