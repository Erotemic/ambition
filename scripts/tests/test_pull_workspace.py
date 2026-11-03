"""Pull real parent and nested repositories with unrelated local files."""

import os
from pathlib import Path
import shutil
import subprocess

import pytest

SCRIPT = Path(__file__).resolve().parents[2] / "pull_workspace.sh"


def git(repo, *args):
    return subprocess.run(["git", *args], cwd=repo, check=True,
                          capture_output=True, text=True).stdout.strip()


def init(repo):
    repo.mkdir()
    git(repo, "init", "-qb", "main")
    git(repo, "config", "user.name", "Fixture")
    git(repo, "config", "user.email", "fixture@example.com")


def commit(repo):
    git(repo, "add", "-A")
    git(repo, "commit", "-qm", "Fixture update")
    return git(repo, "rev-parse", "HEAD")


@pytest.fixture
def workspace(tmp_path, monkeypatch):
    monkeypatch.setenv("GIT_ALLOW_PROTOCOL", "file")
    monkeypatch.setenv("GIT_CONFIG_GLOBAL", os.devnull)
    monkeypatch.setenv("GIT_CONFIG_NOSYSTEM", "1")
    leaf = tmp_path / "leaf"
    sub = tmp_path / "sub"
    origin = tmp_path / "origin"
    for source in (leaf, sub, origin):
        init(source)
        (source / "version.txt").write_text("first")
        (source / "local.txt").write_text("committed local file")
        (source / ".gitignore").write_text("cache/\n")
        commit(source)
    git(sub, "submodule", "add", "-q", str(leaf), "nested")
    commit(sub)
    git(origin, "submodule", "add", "-q", str(sub), "vendor/tool")
    shutil.copyfile(SCRIPT, origin / "pull_workspace.sh")
    commit(origin)
    local = tmp_path / "local"
    git(tmp_path, "clone", "-q", "--recurse-submodules", str(origin), str(local))
    for checkout in (local / "vendor/tool", local / "vendor/tool/nested"):
        git(checkout, "switch", "-q", "main")
    return local, origin, sub, leaf


def advance(origin, sub, leaf, *, added=None):
    (leaf / "version.txt").write_text("second")
    leaf_head = commit(leaf)
    git(sub / "nested", "fetch", "-q", "origin")
    git(sub / "nested", "checkout", "-q", leaf_head)
    (sub / "version.txt").write_text("second")
    if added:
        (sub / added).write_text("incoming source")
    sub_head = commit(sub)
    git(origin / "vendor/tool", "fetch", "-q", "origin")
    git(origin / "vendor/tool", "checkout", "-q", sub_head)
    (origin / "version.txt").write_text("second")
    origin_head = commit(origin)
    return origin_head, sub_head, leaf_head


def pull(local, mode):
    return subprocess.run(["bash", "pull_workspace.sh", *mode], cwd=local,
                          capture_output=True, text=True)


@pytest.mark.parametrize("mode", [[], ["--current-branches"], ["--remote"]])
def test_loose_files_and_unrelated_edits_do_not_block_nested_updates(workspace, mode):
    local, origin, sub, leaf = workspace
    checkouts = (local, local / "vendor/tool", local / "vendor/tool/nested")
    for checkout in checkouts:
        (checkout / "notes.txt").write_text("untracked notes")
        (checkout / "cache").mkdir()
        (checkout / "cache/output.bin").write_bytes(b"ignored output")
        (checkout / "local.txt").write_text("local edit")
    heads = advance(origin, sub, leaf)
    result = pull(local, mode)
    assert result.returncode == 0, result.stdout + result.stderr
    for checkout, expected in zip(checkouts, heads):
        assert git(checkout, "rev-parse", "HEAD") == expected, result.stdout + result.stderr
        assert (checkout / "notes.txt").read_text() == "untracked notes"
        assert (checkout / "cache/output.bin").read_bytes() == b"ignored output"
        assert (checkout / "local.txt").read_text() == "local edit"


@pytest.mark.parametrize("mode", [[], ["--current-branches"], ["--remote"]])
def test_an_untracked_file_in_the_incoming_path_is_kept(workspace, mode):
    local, origin, sub, leaf = workspace
    checkout = local / "vendor/tool"
    original = git(checkout, "rev-parse", "HEAD")
    (checkout / "notes.txt").write_text("untracked notes")
    advance(origin, sub, leaf, added="notes.txt")
    result = pull(local, mode)
    assert result.returncode != 0, result.stdout + result.stderr
    assert git(local, "rev-parse", "HEAD") == git(origin, "rev-parse", "HEAD")
    assert git(checkout, "rev-parse", "HEAD") == original
    assert (checkout / "notes.txt").read_text() == "untracked notes"


@pytest.mark.parametrize("ignored", [False, True])
def test_a_parent_file_in_the_incoming_path_is_kept(workspace, ignored):
    local, origin, sub, leaf = workspace
    advance(origin, sub, leaf)
    path = "cache/output.bin" if ignored else "notes.txt"
    (origin / path).parent.mkdir(exist_ok=True)
    (origin / path).write_text("incoming source")
    git(origin, "add", "-f", path)
    commit(origin)
    (local / path).parent.mkdir(exist_ok=True)
    (local / path).write_text("local file")
    original = git(local, "rev-parse", "HEAD")
    result = pull(local, ["--parent-only"])
    assert result.returncode != 0, result.stdout + result.stderr
    assert git(local, "rev-parse", "origin/main") == git(origin, "rev-parse", "HEAD")
    assert git(local, "rev-parse", "HEAD") == original
    assert (local / path).read_text() == "local file"
