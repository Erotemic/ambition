"""Ambition's moveset balance inspector.

`reviews` is the durable feedback bank and imports no server or GUI code, so an
agent can read standing feedback headlessly. `server` serves `web/` plus the
bundle `moveset_export` writes.
"""

from .reviews import Review, ReviewBank, ISSUE_TAGS, RUBRIC_BANDS

__all__ = ["Review", "ReviewBank", "ISSUE_TAGS", "RUBRIC_BANDS"]
