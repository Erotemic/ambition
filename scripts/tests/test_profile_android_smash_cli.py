"""Exercise the Android Smash CLI launch without an attached device.

The fake Android process reads and removes the one-shot launch file. The tests
check that capture starts only after LIVE and that the file is cleaned up.
"""

from __future__ import annotations

import json
import os
from pathlib import Path
import shutil
import subprocess
import textwrap

import pytest

SCRIPT = Path(__file__).resolve().parents[1] / "measure" / "profile_android.sh"


@pytest.fixture
def fake_android(tmp_path: Path):
    root = tmp_path / "repo"
    measure = root / "scripts" / "measure"
    measure.mkdir(parents=True)
    (root / "target").mkdir()
    script = measure / "profile_android.sh"
    shutil.copy2(SCRIPT, script)
    script.chmod(0o755)
    env_capture = tmp_path / "build-env.txt"
    build = root / "build_for_android.sh"
    build.write_text(
        '#!/usr/bin/env bash\n'
        'printf "%s\\n" "${AMBITION_AUTOPLAY-unset}" '
        '"${AMBITION_BOOT_ROUTE-unset}" "${AMBITION_HUB_GAMES-unset}" '
        '> "$FAKE_BUILD_ENV_FILE"\n'
    )
    build.chmod(0o755)

    state_file = tmp_path / "device.json"
    state_file.write_text(json.dumps({"running": False, "launch_file": "", "events": []}))
    adb = tmp_path / "adb"
    adb.write_text(textwrap.dedent('''\
        #!/usr/bin/env python3
        import json
        import os
        import pathlib
        import sys

        state_file = pathlib.Path(os.environ['FAKE_ADB_STATE'])
        state = json.loads(state_file.read_text())
        args = sys.argv[1:]
        state['events'].append(' '.join(args))
        code = 0
        if args and args[0] == 'shell':
            cmd = args[1:]
            if cmd and cmd[0].startswith('cat > '):
                state['launch_file'] = sys.stdin.read()
            elif cmd[:2] == ['am', 'force-stop']:
                state['running'] = False
            elif cmd and cmd[0] == 'pidof':
                if state['running']:
                    print('12345')
            elif cmd and cmd[0] == 'monkey':
                state['running'] = True
                if os.environ.get('FAKE_AUTOPLAY_RESULT', 'live') != 'not-consumed':
                    state['launch_file'] = ''
            elif cmd[:2] == ['test', '-s']:
                code = 0 if state['launch_file'] else 1
            elif cmd[:3] == ['test', '!', '-e']:
                code = 0 if not state['launch_file'] else 1
            elif cmd[:2] == ['rm', '-f'] and cmd[-1].endswith('/launch_env'):
                state['launch_file'] = ''
            elif cmd and cmd[0] == 'dumpsys':
                print('mock gfxinfo')
        elif args and args[0] == 'logcat':
            result = os.environ.get('FAKE_AUTOPLAY_RESULT', 'live')
            if result == 'failed':
                print('[smash-autoplay] FAILED: mock startup failure')
            elif result == 'live':
                print('[smash-autoplay] LIVE: seats=4, ceremony_released=true, all_cpu=true')
        state_file.write_text(json.dumps(state))
        sys.exit(code)
    '''))
    adb.chmod(0o755)
    env = os.environ.copy()
    env.update(
        ADB=str(adb),
        FAKE_ADB_STATE=str(state_file),
        FAKE_BUILD_ENV_FILE=str(env_capture),
        AMBITION_AUTOPLAY='smash',
        AMBITION_BOOT_ROUTE='smash_select',
        AMBITION_HUB_GAMES='smash',
    )
    return script, state_file, env_capture, env, tmp_path / 'profiles'


def invoke(fake_android, *, result='live', profile_build=False):
    script, state_file, env_capture, env, out_base = fake_android
    env['FAKE_AUTOPLAY_RESULT'] = result
    cmd = [
        str(script), 'gfxinfo', '--launch', '--no-auto-symfs',
        '--launch-env', 'AMBITION_BOOT_ROUTE=smash_select',
        '--launch-env', 'AMBITION_AUTOPLAY=smash_4p',
        '--warmup', '0', '--duration', '1', '--name', 'smash-test', '--out', str(out_base),
    ]
    if profile_build:
        cmd.append('--profile-build')
    completed = subprocess.run(cmd, env=env, text=True, capture_output=True, timeout=35)
    state = json.loads(state_file.read_text())
    return completed, state, out_base / 'android-smash-test', env_capture


def test_cli_waits_for_live_then_cleans_launch_settings(fake_android):
    completed, state, out, _ = invoke(fake_android, profile_build=True)
    assert completed.returncode == 0, completed.stderr
    assert (out / 'launch-env-consumption.txt').read_text().strip() == 'consumed'
    assert (out / 'smash-autoplay-readiness.txt').read_text().strip() == 'live'
    assert (out / 'smash-autoplay-capture-status.txt').read_text().strip() == 'live'
    assert (out / 'smash-autoplay-preflight.log').is_file()
    assert state['running'] is False
    assert state['launch_file'] == ''
    events = state['events']
    assert any('shell monkey -p' in event for event in events)
    assert any(event.startswith('logcat -d --pid=12345') for event in events)
    assert any('shell am force-stop' in event for event in events)
    # These values were present in the calling shell, but were not baked in.
    assert (out / 'profile-build.command.txt').is_file()
    assert _.read_text().splitlines() == ['unset', 'unset', 'unset']


def test_failed_smash_autoplay_does_not_leave_one_shot_file(fake_android):
    completed, state, out, _ = invoke(fake_android, result='failed')
    assert completed.returncode != 0
    assert 'Smash autoplay failed before CPU profiling' in completed.stderr
    assert state['running'] is False
    assert state['launch_file'] == ''
    assert (out / 'smash-autoplay-readiness.txt').read_text().strip() == 'failed'
    assert not any('shell dumpsys gfxinfo' in event for event in state['events'])
