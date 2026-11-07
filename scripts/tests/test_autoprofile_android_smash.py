"""Smash's Android autoplay profiles select a real CPU match, not character select.

Exercise the wrapper with a fake profile_android.sh; never build an APK or
require adb just to prove argument forwarding and one-shot launch selection.
The Bevy-side all-CPU roster and readiness contract is tested in
`game/ambition_demo_smash/src/autoplay.rs`.
"""

from __future__ import annotations

import os
import subprocess
from pathlib import Path

import pytest

SOURCE = Path(__file__).resolve().parents[1] / "measure" / "autoprofile_android.sh"


def invoke(tmp_path: Path, profile: str, *args: str) -> list[str]:
    measure = tmp_path / "scripts" / "measure"
    measure.mkdir(parents=True, exist_ok=True)
    wrapper = measure / "autoprofile_android.sh"
    wrapper.write_bytes(SOURCE.read_bytes())
    wrapper.chmod(0o755)
    target = measure / "profile_android.sh"
    target.write_text('#!/usr/bin/env bash\nprintf "%s\\n" "$@"\n')
    target.chmod(0o755)
    env = os.environ.copy()
    env.update(
        AMBITION_BOOT_ROUTE="old_route",
        AMBITION_AUTOPLAY="old_game",
        AMBITION_HUB_GAMES="ambition",
    )
    completed = subprocess.run(
        [str(wrapper), profile, *args],
        capture_output=True,
        text=True,
        check=True,
        env=env,
    )
    # First line is the wrapper's own informational log; all later lines are
    # the forwarded script arguments, one argument per line.
    return completed.stdout.splitlines()[1:]


def launch_value(args: list[str], key: str) -> str:
    for index, arg in enumerate(args):
        if arg == "--launch-env" and args[index + 1].startswith(key + "="):
            return args[index + 1].split("=", 1)[1]
    raise AssertionError(f"missing launch env {key}")


@pytest.mark.parametrize("profile", ["smash", "smash_4p"])
def test_smash_profiles_boot_the_select_route_for_all_cpu_matches(tmp_path, profile):
    args = invoke(tmp_path, profile)
    assert args[0] == "all"
    assert "--profile-build" in args and "--launch" in args
    assert launch_value(args, "AMBITION_BOOT_ROUTE") == "smash_select"
    assert launch_value(args, "AMBITION_AUTOPLAY") == profile
    assert "--warmup" in args and args[args.index("--warmup") + 1] == "35"
    assert "--duration" in args and args[args.index("--duration") + 1] == "60"
    assert "--name" in args
    assert args[args.index("--name") + 1].startswith(f"all-{profile}-auto-")


def test_cli_overrides_defaults(tmp_path):
    args = invoke(tmp_path, "smash", "--warmup", "45", "--duration", "90")
    assert args[-4:] == ["--warmup", "45", "--duration", "90"]


def test_mary_o_policy_remains_unchanged(tmp_path):
    args = invoke(tmp_path, "mary_o")
    assert launch_value(args, "AMBITION_BOOT_ROUTE") == "mary_o_gameplay"
    assert "--warmup" in args and args[args.index("--warmup") + 1] == "25"
    assert "--duration" in args and args[args.index("--duration") + 1] == "30"
