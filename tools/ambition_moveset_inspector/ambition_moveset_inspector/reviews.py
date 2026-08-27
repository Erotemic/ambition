"""Durable human feedback on a fighter or one of its moves.

Modelled on the music review bank: a review is authoring data, not a generated
artifact, so it lives in the repository under ``reviews/`` and survives the
bundle being regenerated.

⛔ THE SUBJECT IS THE STABLE ID, NOT THE NUMBERS. A music review keys on the
hash of the audio actually heard, because two renders of one cue are two
different things to judge. A moveset review is the opposite case: the whole
point is to say *"the pirate's forward smash is too strong"* and have that
survive the next tuning pass, so the key is ``<character>`` or
``<character>/<move>``. What the numbers WERE when the note was written is
recorded beside it — see ``snapshot`` — so a later reader can tell a note that
has been addressed from one that has not.

No Qt, no server import: agents and CI read this headlessly.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Iterable, Mapping
import re

import yaml

REVIEW_SCHEMA = "ambition.moveset_review.v1"

MIN_SCORE = 1.0
MAX_SCORE = 10.0

#: The same 1-10 semantics the music bank uses, so one reviewer holds one scale.
RUBRIC_BANDS: tuple[tuple[float, float, str, str], ...] = (
    (1.0, 2.99, "Replace", "Not worth tuning; the move needs a different design."),
    (3.0, 4.99, "Major polish", "The idea is right and the numbers are not."),
    (5.0, 6.99, "Acceptable", "Playable, and an obvious later polish candidate."),
    (7.0, 8.99, "Strong", "Ship-quality; change only for a concrete reason."),
    (9.0, 10.0, "Standout", "Use as the reference the rest of the cast is tuned against."),
)

#: What is wrong, in the vocabulary a balance pass acts on.
ISSUE_TAGS = (
    "too-strong",
    "too-weak",
    "startup",
    "endlag",
    "range",
    "knockback",
    "recovery",
    "feel",
    "unclear-read",
    "animation",
)

_SAFE = re.compile(r"[^a-z0-9_.-]+")


def band_for(score: float) -> tuple[str, str]:
    """The label and the sentence for a score."""
    for low, high, label, meaning in RUBRIC_BANDS:
        if low <= score <= high:
            return label, meaning
    return "Unrated", "No score recorded."


def normalize_score(value: Any) -> float | None:
    """A score, clamped to the scale, or ``None`` for an unrated review.

    ⛔ AN UNPARSEABLE SCORE IS UNRATED, NOT ZERO. Zero is off the scale, and a
    reviewer who typed a stray character should not silently acquire the
    strongest possible opinion.
    """
    if value is None or value == "":
        return None
    try:
        score = float(value)
    except (TypeError, ValueError):
        return None
    if score != score:  # NaN
        return None
    return max(MIN_SCORE, min(MAX_SCORE, score))


def subject_parts(subject: str) -> tuple[str, str | None]:
    """``"pirate/jab"`` -> ``("pirate", "jab")``; ``"pirate"`` -> ``("pirate", None)``."""
    character, _, move = subject.partition("/")
    return character, (move or None)


def _slug(text: str) -> str:
    return _SAFE.sub("-", text.strip().lower()).strip("-") or "unnamed"


@dataclass
class Review:
    """One reviewer's standing opinion about one subject."""

    subject: str
    character: str
    move: str | None = None
    score: float | None = None
    notes: str = ""
    issues: list[str] = field(default_factory=list)
    #: The numbers the subject had when this was written, so a later reader can
    #: tell an addressed note from a live one without re-reading the diff.
    snapshot: dict[str, Any] = field(default_factory=dict)
    cast_generation: int | None = None
    created_at: str = ""
    updated_at: str = ""

    def to_document(self) -> dict[str, Any]:
        label, _ = band_for(self.score) if self.score is not None else ("Unrated", "")
        return {
            "schema": REVIEW_SCHEMA,
            "subject": self.subject,
            "character": self.character,
            "move": self.move,
            "score": self.score,
            "band": label,
            "issues": sorted(set(self.issues)),
            "notes": self.notes,
            "snapshot": self.snapshot,
            "cast_generation": self.cast_generation,
            "created_at": self.created_at,
            "updated_at": self.updated_at,
        }

    @classmethod
    def from_document(cls, doc: Mapping[str, Any]) -> "Review":
        subject = str(doc.get("subject", ""))
        character, move = subject_parts(subject)
        return cls(
            subject=subject,
            character=str(doc.get("character") or character),
            move=doc.get("move") or move,
            score=normalize_score(doc.get("score")),
            notes=str(doc.get("notes") or ""),
            issues=list(doc.get("issues") or []),
            snapshot=dict(doc.get("snapshot") or {}),
            cast_generation=doc.get("cast_generation"),
            created_at=str(doc.get("created_at") or ""),
            updated_at=str(doc.get("updated_at") or ""),
        )


class ReviewBank:
    """The ``reviews/`` directory, as a dictionary keyed by subject."""

    def __init__(self, root: Path):
        self.root = Path(root)

    def path_for(self, subject: str) -> Path:
        character, move = subject_parts(subject)
        stem = _slug(move) if move else "_character"
        return self.root / _slug(character) / f"{stem}.yaml"

    def load(self, subject: str) -> Review | None:
        path = self.path_for(subject)
        if not path.exists():
            return None
        doc = yaml.safe_load(path.read_text()) or {}
        return Review.from_document(doc)

    def save(self, review: Review) -> Path:
        """Write ``review``, editing an existing note in place.

        ⭐ IN PLACE, and the creation timestamp is kept. A reviewer refining a
        note is refining an opinion, not adding a second one — the music bank
        learned this and the same reasoning holds: an opinion history nobody
        asked for makes "what does Jon think of this move" ambiguous.
        """
        now = datetime.now(timezone.utc).isoformat(timespec="seconds")
        existing = self.load(review.subject)
        review.created_at = existing.created_at if existing and existing.created_at else now
        review.updated_at = now
        path = self.path_for(review.subject)
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(yaml.safe_dump(review.to_document(), sort_keys=False, allow_unicode=True))
        return path

    def all(self) -> list[Review]:
        """Every review, ordered by subject so a report is stable."""
        out: list[Review] = []
        for path in sorted(self.root.rglob("*.yaml")):
            doc = yaml.safe_load(path.read_text()) or {}
            if doc.get("schema") != REVIEW_SCHEMA:
                continue
            out.append(Review.from_document(doc))
        return out

    def open_work(self, threshold: float = 6.0) -> list[Review]:
        """Reviews that ask for a change: scored below ``threshold``, or tagged.

        This is the list a *"address the feedback on the pointed polygon"*
        request resolves to, which is why it is here rather than in the UI.
        """
        return [
            review
            for review in self.all()
            if (review.score is not None and review.score < threshold) or review.issues
        ]


def snapshot_of(character: Mapping[str, Any], move_id: str | None) -> dict[str, Any]:
    """The few numbers worth recording beside a note.

    Not the whole move: a snapshot that copied every field would make every
    review file churn on an unrelated tuning edit, and the point is to remember
    what the reviewer was looking at.
    """
    if move_id is None:
        return {
            "max_health": character.get("vitals", {}).get("max_health"),
            "run_speed": (character.get("locomotion") or {}).get("run_speed"),
            "moves": len(character.get("moves", [])),
        }
    for move in character.get("moves", []):
        if move.get("id") == move_id:
            derived = move.get("derived", {})
            return {
                "startup_f": derived.get("startup_f"),
                "active_f": derived.get("active_f"),
                "endlag_f": derived.get("endlag_f"),
                "max_damage": derived.get("max_damage"),
                "max_knockback": derived.get("max_knockback"),
                "reach": derived.get("reach"),
            }
    return {}


def format_report(reviews: Iterable[Review]) -> str:
    """A plain-text digest, for an agent that was told to address the feedback."""
    lines: list[str] = []
    by_character: dict[str, list[Review]] = {}
    for review in reviews:
        by_character.setdefault(review.character, []).append(review)
    for character in sorted(by_character):
        lines.append(character)
        for review in sorted(by_character[character], key=lambda r: r.subject):
            score = "—" if review.score is None else f"{review.score:.1f}"
            tags = f" [{', '.join(review.issues)}]" if review.issues else ""
            target = review.move or "(character)"
            lines.append(f"  {target:<34} {score:>4}{tags}")
            if review.notes.strip():
                for line in review.notes.strip().splitlines():
                    lines.append(f"      {line}")
    return "\n".join(lines)
