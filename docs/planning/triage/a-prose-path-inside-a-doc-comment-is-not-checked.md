# A moved directory is a citation broken in silence

**Status:** closed. The title names the first of five related gaps. All five are
measured and repaired. No work is open. This page keeps the tools that exist,
the gates that were measured and rejected, and the habits that follow from them.
Git history holds the full census narrative.

## The five gaps

| # | gap | state |
|---|---|---|
| 1 | A path named in prose inside a `.rs` comment is not checked. | `check_planning_citations.py --comment-paths` reports it in the default lane. |
| 2 | A guard's waiver can quote source text verbatim, and nothing resolves the quotation. | Censused (783 quotations in `docs/planning`, 4 genuine defects). Repaired. Not gated. |
| 3 | A bare name in a source comment. `--vanished` read only documents. | `--comments` reads source. The baseline is the epoch root. |
| 4 | A comment loses its subject: a stranded `///` run attaches to the wrong item. | 25 found, 25 repaired. Not gated. |
| 5 | A `Qnnn` is a citation, and answering a question deletes it. | The 4 comments that deferred to a missing number now resolve. Not gated. |

The shape under all five: stale prose is usually not unnoticed. The correction
is often already in the same comment block, as a new sentence beside the old one
instead of a replacement. A filter that reads the whole block goes quiet at the
moment the file holds both answers.

## What runs

- `scripts/check_planning_citations.py` resolves paths and symbols in `docs/`.
- `--comment-paths` resolves paths named in Rust comments. It is registered in
  `scripts/run_tests.py` as "a path named in a source comment exists (reports,
  does not gate)" in `slow_python_checker_jobs`. `--rust` and `--maintenance`
  drop it and say so.
- `--comments` resolves symbols in Rust comments and implies `--comment-paths`.
  It stays advisory because a symbol citation can name something the checker
  cannot see (for example, an upstream method on a type name that this repo also
  defines).
- `scripts/tests/test_source_comment_path_citations.py` pins the reporting, the
  control, the `cite-ok` escape, the symbol/path split, and that a path in a
  string literal is data.
- `PLANNING_VANISHED_BASELINE` in `scripts/run_tests.py` is the epoch root.
  `test_the_vanished_baseline_is_a_commit_git_can_actually_reach` asserts
  ancestry, not only SHA shape. A baseline must be reachable from a ref, or the
  job is red on every other clone.

The path census went through three instrument revisions: 302 findings (every
path-shaped token), then 33 (`.rs` tokens only), then 20 (with the repository's
abbreviation habit, where `actor_monolith/<file>` names a file under
`crates/ambition_platformer2d_actor_monolith/src/`). Of the 20: 8 placeholders
named `foo`, 5 self-labelled as history, 7 stale live claims (all repaired).

## Rulings

- `--comment-paths` was registered `--strict` and demoted the same day, per
  AGENTS.md "Avoid bullshit guardrails". A stale path misleads a reader but
  breaks nothing. Do not make it a gate.
- One resolver, not a second script. `path_resolves` and `cite-ok` are the one
  authority for this question.
- Do not gate "a citation naming `some_fn` must land near `fn some_fn(`". It
  flagged 42 citations, and they were deliberate deep links into bodies and doc
  comments.
- Do not gate waiver quotations. "Every backticked identifier in a waiver
  exists" is green by construction and misses the real case. "Every quoted
  phrase appears in source" matches apostrophes. The real population was two.
- Do not gate stranded `///` runs or `Qnnn` resolution. Both populations are
  empty or self-contained, and the check would be prose judgement.

## Habits

- When a ruling lands, walk its inbound links. A repair made only where a search
  happened to surface the name is not a repair of the fact.
- Replace a dead name only after you read the sentence around it. The correct
  live owner can be a different mechanism (for example, the room transition
  sweep and the new-game reset sweep use different rosters). A rename can turn
  a stale sentence into a fluent wrong one.
- To find the commit that deleted a symbol, search the definition form:
  `git log -S"fn <name>"` or `-S"struct <name>"`. A bare-name search finds the
  last commit that edited prose about it.
- When one `path:N` citation into a file is wrong, check the other citations
  into the same file. A shared offset is one edit and predicts the rest.
- The record of a deleted item belongs in `//`, not `///`. A `///` run followed
  by a blank line and a second `///` run attaches to the item below the second.
- A comment that defers to a `Qnnn` ("see Q118") must point at a number that
  resolves in `awaiting-maintainer-decision.md` or `maintainer-decisions.md`.
- Tune a prose filter on this corpus. History words appear in capitals here
  (`UNTIL`, `DELETED`, `GONE`), so a case-sensitive filter misses them. Related:
  [`../../recipes/checks-that-did-not-run.md`](../../recipes/checks-that-did-not-run.md).
- A citation wrapped across a line break is invisible to the backtick pattern.
  Keep a backticked name on one line.
- A comment that states its own exit condition ("the cycle closes when X
  leaves `features`") is cheap to audit. Re-check the condition when X moves.

## Optional follow-up

Nobody has built a check for "several `path:N` citations into one file share
the same offset". It is the narrow form of the rejected line-landing gate. Build
it only if the defect recurs.
