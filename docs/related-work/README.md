# Related work

**How other engines solve the problems Ambition is solving, with citations.**

## Why this section exists

Jon, 2026-08-07, on Ambition's shell vocabulary: *"I wonder if we should start a
related work section in the docs to document how everyone else does it, and give
us a better reference for how we compare."*

Ambition is written from first principles on purpose, and that is a strength
until it becomes an excuse. A design argued only against itself has no way to
find out that a concept it invented already has a name, or that a distinction it
collapsed is one three other engines kept. This section is the outside view.

## What belongs here

A page per QUESTION, not per engine. The useful unit is *"how is this problem
solved elsewhere"* — a page per engine would be a tour, and nobody reads a tour
while making a decision.

Each page owes:

* **The Ambition concept it is about**, named in our terms, with a pointer to
  where we implement it.
* **What each engine calls it**, and — more importantly — *whether they have the
  concept at all*. An engine that does NOT split something we split is the most
  informative row in the table.
* **Citations that were checked**, with the date. See the rule below.
* **What it changed**, if anything. A related-work page that changed no decision
  should say so; that is a finding too.

## ⛔ The citation rule

**Every claim carries a link, and the link was FETCHED, not remembered.**

This is not bureaucracy. The first draft of the vocabulary page below asserted an
Unreal URL option from memory; the Epic page it was attributed to does not
document it, and the real citation turned out to be a different page entirely
(the claim was true, the source was wrong). A confidently-wrong citation is worse
than no citation, because the next reader stops checking.

So: fetch the page, quote the line, record the URL, date the check. Mark
third-party sources as third-party — a community blog is often the only place a
sample project's internals are written down, and that is fine as long as the
reader can see what kind of source it is.

⚠ **APIs move.** A citation is a point-in-time observation, same as everything
else in this repo. Re-check before acting on a detail, and prefer claims about
CONCEPTS over claims about spellings.

## Pages

* [Shell vocabulary: provider, experience, route](shell-vocabulary-in-other-engines.md)
  — what Unreal, Unity and Godot call the things our shell calls providers,
  experiences and routes. Checked 2026-08-07.
