"""Shared pytest setup for the sprite renderer test suite.

Adds a ``slow_render`` marker for tests that require full-resolution
rendering and have no low-resolution equivalent yet. Skipped by
default to keep the regression net fast (seconds, not minutes);
opt in with ``pytest --run-slow-render``.

See ``GOALS.md`` for the rationale — the long-term goal is to remove
this marker entirely once every target supports a ``scale`` parameter,
at which point every test is fast.
"""

from __future__ import annotations

import pytest


def pytest_addoption(parser: pytest.Parser) -> None:
    parser.addoption(
        "--run-slow-render",
        action="store_true",
        default=False,
        help=(
            "Run tests marked `slow_render` (full-resolution sprite renders, "
            "minutes of runtime). Skipped by default. See GOALS.md."
        ),
    )


def pytest_configure(config: pytest.Config) -> None:
    config.addinivalue_line(
        "markers",
        "slow_render: test requires full-resolution sprite rendering; "
        "skipped by default, opt in with --run-slow-render",
    )


def pytest_collection_modifyitems(config: pytest.Config, items: list) -> None:
    if config.getoption("--run-slow-render"):
        return
    skip_slow = pytest.mark.skip(
        reason=(
            "slow_render: full-resolution sprite render. Opt in with "
            "--run-slow-render. See GOALS.md for the low-res-mode plan."
        ),
    )
    for item in items:
        if "slow_render" in item.keywords:
            item.add_marker(skip_slow)
