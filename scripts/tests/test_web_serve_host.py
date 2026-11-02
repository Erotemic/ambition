"""The web servers must bind to the host selected by the caller."""

import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import time
from urllib.request import urlopen

import pytest


REPO_ROOT = Path(__file__).resolve().parents[2]


def probe_server(command, cwd, env, log_path):
    """Start the server, read its assigned port, and request a page."""
    with log_path.open("w+") as log:
        process = subprocess.Popen(command, cwd=cwd, env=env, stdout=log, stderr=subprocess.STDOUT)
        try:
            deadline = time.monotonic() + 10
            output = ""
            match = None
            while time.monotonic() < deadline:
                log.seek(0)
                output = log.read()
                match = re.search(
                    r"Serving .+ at http://localhost:(\d+)/ \(bind (127\.0\.0\.1|0\.0\.0\.0):(\d+)\)",
                    output,
                )
                if match or process.poll() is not None:
                    break
                time.sleep(0.05)
            assert match, output
            port, host, bound_port = match.groups()
            assert port == bound_port
            with urlopen(f"http://127.0.0.1:{port}/", timeout=5) as response:
                assert response.read() == b"host test"
            return host
        finally:
            if process.poll() is None:
                process.terminate()
            process.wait(timeout=10)


@pytest.mark.parametrize(
    ("host", "expected_bind"),
    [("localhost", "127.0.0.1"), ("all", "0.0.0.0"), ("0.0.0.0", "0.0.0.0")],
)
def test_web_bundle_serve_host(tmp_path, host, expected_bind):
    (tmp_path / "index.html").write_text("host test")
    command = [
        sys.executable, str(REPO_ROOT / "scripts/package/web_bundle.py"),
        "serve", "--root", str(tmp_path), "--port", "0", "--host", host,
    ]
    assert probe_server(command, tmp_path, os.environ, tmp_path / "server.log") == expected_bind


@pytest.mark.parametrize(
    ("env_host", "flag_host", "expected_bind"),
    [
        (None, None, "127.0.0.1"),
        ("all", None, "0.0.0.0"),
        ("all", "localhost", "127.0.0.1"),
        ("localhost", "all", "0.0.0.0"),
        (None, "0.0.0.0", "0.0.0.0"),
    ],
)
def test_build_script_host_and_environment_precedence(tmp_path, env_host, flag_host, expected_bind):
    subprocess.run(["git", "init", "-q", str(tmp_path)], check=True)
    scripts = tmp_path / "scripts"
    scripts.mkdir()
    (scripts / "package").mkdir()
    shutil.copy2(REPO_ROOT / "build_for_web.sh", tmp_path / "build_for_web.sh")
    shutil.copy2(REPO_ROOT / "scripts/package/web_bundle.py", scripts / "package/web_bundle.py")

    web_dir = tmp_path / "game/ambition_app/web"
    web_dir.mkdir(parents=True)
    (web_dir / "index.html").write_text("host test")
    target_dir = tmp_path / "target"
    wasm = target_dir / "wasm32-unknown-unknown/release/ambition_app.wasm"
    wasm.parent.mkdir(parents=True)
    wasm.write_bytes(b"wasm placeholder")

    bin_dir = tmp_path / "bin"
    bin_dir.mkdir()
    cargo = bin_dir / "cargo"
    cargo.write_text('#!/bin/sh\nprintf \'{"target_directory":"%s"}\\n\' "$TEST_TARGET_DIR"\n')
    cargo.chmod(0o755)
    rustup = bin_dir / "rustup"
    rustup.write_text('#!/bin/sh\nprintf \'wasm32-unknown-unknown\\n\'\n')
    rustup.chmod(0o755)

    env = os.environ.copy()
    env.update({"PATH": f"{bin_dir}:{env['PATH']}", "CARGO": str(cargo), "TEST_TARGET_DIR": str(target_dir)})
    if env_host is None:
        env.pop("AMBITION_WEB_HOST", None)
    else:
        env["AMBITION_WEB_HOST"] = env_host
    command = ["bash", "./build_for_web.sh", "--skip-build", "--skip-bindgen", "--serve", "0"]
    if flag_host is not None:
        command.extend(["--host", flag_host])
    assert probe_server(command, tmp_path, env, tmp_path / "server.log") == expected_bind
