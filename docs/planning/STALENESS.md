# Marking staleness — the convention

> **Jon, 2026-08-08:** *"we probably need something that starts off by marking
> which items in planning documents are actually stale or marking entire
> documents as stale in case we make a mistake. They can record the evidence that
> they are stale and then we can make a sweep up later. It also prevents people
> from reading them and taking them as up to date in the meantime."*

**Marking is not archiving.** Nothing is deleted, moved, or rewritten by marking
it. A mark is a warning to the next reader plus the evidence that produced it,
and the sweep is a separate, later, deliberate act.

## Why marking beats deciding

The long-open question *"which of the 33 engine design documents have become
history?"* sat unanswered for a week because it needs **someone who knows what is
live** — one whole-repository judgement nobody holds at once.

Marking needs only **local** knowledge. Whoever trips over a claim that is no
longer true records it, right where they tripped, with what they saw. The
judgement accumulates instead of being demanded.

⭐ **and it protects readers in the meantime**, which is the real cost. This
repo's prose is unusually good and therefore **believed**. On 2026-08-08 alone, a
stale doc claim and a stale section heading each nearly drove a wrong decision —
the heading said a route *did not* fix Jon's complaint when the body beneath it
had already been corrected to say the opposite.

## The two marks

### An ITEM is stale

Put it inline, at the item, so it cannot be read without being seen:

```markdown
⌛ **STALE 2026-08-08** — `spawn_block` no longer resolves art from `BlockKind`
alone for Mary-O; `MaryOBlockLook::Question` wears its own texture
(`ldtk_vocabulary.rs`). Evidence: read the enum, and Jon confirmed from play.
```

### A DOCUMENT is stale

Put it directly under the H1, before anything else:

```markdown
# Some Design Document

> ⌛ **STALE 2026-08-08 — read with suspicion, not yet swept.**
> The crate names throughout predate the `platformer2d` rename (`c74246de9`), and
> §3's schedule diagram shows `PreUpdate` ordering that ggrs replaced.
> Evidence: `grep -c ambition_platformer_` returns 0 in `crates/`.
```

## What a mark MUST carry

| part | why |
|---|---|
| `⌛ **STALE <ISO date>**` | the glyph is greppable and the date says how old the *suspicion* is |
| **what is wrong** | so a reader can route around it without a full investigation |
| **evidence** | a command, a file, a commit, or a person. ⛔ *"this looks old"* is not evidence and does not justify a mark |

⛔ **A mark without evidence is worse than no mark**, because the sweep will
either trust it blindly or have to redo the work. If you cannot say what you saw,
you have a suspicion — write it as a question somewhere, not as a mark here.

## What a mark does NOT do

- It does **not** delete, move, or rewrite anything.
- It does **not** decide the document is worthless. Half a stale document is
  usually still the best explanation of why something exists.
- It does **not** need permission. Marking is local and reversible; removing a
  mark is one line, which is the point of *"in case we make a mistake"*.

## Removing a mark

If the claim turns out to be current, delete the mark and say so in the commit
message. If it was stale and you fixed the content, delete the mark in the same
commit as the fix — a corrected document carrying a stale warning is the same
defect one layer up.

## Finding them

```
python3 scripts/check_stale_marks.py            # list every mark, newest first
python3 scripts/check_stale_marks.py --check    # fail if any mark is malformed
```

The checker validates SHAPE, never truth: it cannot know whether a claim is
stale, only whether the mark carries a date and some evidence. That limit is
deliberate — a checker that pretended to judge staleness would be the
`a_check_that_cannot_fail` defect wearing a new hat.
