# Re-measuring a planning claim

A planning row is a claim about a repository that changes under it, so
`docs/planning/README.md` tells every session to re-measure a row against `HEAD`
before acting on it. This page is how that goes wrong.

⛔ **Every rule below was learned by getting it wrong on 2026-09-02/03, in this
repository, with the commit that corrected it.** None of them is reasoning about
how one ought to work. They are here because the same session made each mistake,
published some of them to a coordinator, and had to retract.

⭐ **The shape they share: a re-measurement is a MEASUREMENT, and the usual way to
be wrong about a measurement is to be wrong about the instrument.** Most of what
follows is an instrument error — a filter that could not match, a scan at the
wrong granularity, a signature read instead of a body, a count that was a
property of its flags — and not a mistake about the code.
⚠ **This page deliberately carries no total.** It had one, it went stale the same
day it was written as rules were added, and *"a count is not a finding unless the
instrument travels with it"* is one of the rules below. A page that numbers its
own contents has to be re-counted by every editor; the sections are the list.

Sibling page: [`checks-that-did-not-run.md`](checks-that-did-not-run.md), which is
the same subject one level down — a CHECK that is correct and never runs.

---

### The dated verification header, and what it predicted

⭐ **OBSERVED 2026-09-02, over a sweep of eleven planning files: the ones
carrying a dated `> **Verified against <sha> (<date>).**` header were accurate,
and the ones without one had drifted — every time, in that sample.**

Accurate on re-check, all header-bearing: `frontend-audio-is-per-experience.md`,
`engine_rename_campaign.md`, `triage/declared-id-resolution-checks.md`,
`triage/gameplay-presentation-profiles.md`. So were two that carry a dated status
line instead (`engine/relativity.md`, `engine/shell-vanity-sequence.md`).

Drifted, none of them header-bearing at the time:

- `triage/ambition-test-support.md` — proposed a Layer 2 that had shipped that
  morning as `ambition_sim_harness`;
- `triage/ambition-registry-core.md` — 27 registries had become 31;
- `triage/stable-identifier-centralization.md` — `string_id!` turned out to be
  written out three times identically;
- `engine/dialogue-continuity.md` — built, with three of its four open questions
  answered by the code;
- `engine/capability-progression-and-world-gating.md` — four gate families'
  inputs already existed;
- `triage/character-dialogue-from-suggestions.md` — solved by hand-authoring
  instead of by the generator it designed.

⚠ **The header is a correlate, not a cause.** It marks a file somebody has
re-read against the code since writing it; that is the act that keeps a plan
true, and the header is just its receipt. A header added without the re-reading
would be worse than none, because it would buy the next reader's trust without
earning it.

### ⛔ An inventory that reads TYPES will mis-report BEHAVIOUR

Three errors in one inventory on 2026-09-02/03, all the same shape: the question
was about what a function does, and the measurement looked at a declaration.

| asked | measured | wrong because |
|---|---|---|
| do function addresses participate in equality? | whether entry types `derive(PartialEq)` | `PlacementLoweringRegistry` compares `fn_addr_eq` BY HAND inside `try_register`, where no derive scan reaches |
| which registries overwrite silently? | the return type of `register` | five of the seven `-> ()` registries state the replace in a doc comment, one is a test hatch |
| does this loader honour the quality tier? | the parameter list | `load_prop_sheet_for_target` omits the budget ON PURPOSE and says so in the body |

⭐ **The pattern: a signature tells you the SHAPE of a decision and the body tells
you the DECISION.** A count derived from signatures is a lower bound on
correctness and an upper bound on defects — it will find every place that could
be wrong and cannot tell which ones are.

⇒ **So an inventory row is not finished until somebody has read the function.**
That is expensive, which is a reason to scope an inventory to the registries or
loaders that matter, not a reason to substitute a grep over declarations and
report the count as a finding.

### ⭐ Some findings only exist BETWEEN two plans

Three times on 2026-09-02/03 the useful result came from reading two focused
plans against each other rather than either against the code. In each case both
pages were individually accurate and the conclusion was in neither.

- **`capability-and-runtime-composition.md` × `actor-monolith-decomposition.md`**
  — the footprint's remaining 16 crates cannot be cut by any manifest change,
  because every one arrives through the actor monolith. The footprint page knew
  its number; the carve page knew its scope; neither said that one is the other's
  only lever.
- **`room-transition-loading.md` T2 × `asset-preparation-and-residency.md`
  open work 4** — T2's "resident memory without a budget" warning had its
  eviction half answered in the asset page, on the same day, and its prefetch
  half answered by a constant neither page named.
- **`platformer-navigation-and-reachability.md` × `agentic-character-runtime.md`**
  — the agentic page waits on three foundations, two of which now exist, so
  navigation is its last gate. The navigation page looks like a deprioritised
  capability until you know it is somebody else's blocker.

⇒ **So when a plan says "waits for X", "owned by Y", or "the residual is Z", go
READ X, Y and Z before believing the row.** A cross-plan dependency is invisible
from both ends by construction: each page states its own half correctly, and the
join is written down nowhere.

⚠ This is also why `Owner:` lines and `See also` links are load-bearing rather
than decorative — they are the only machine-followable record of a join that
otherwise lives in one session's head.

### ⚠ A guard sweep at SCRIPT granularity misses most of the guards

Earlier on 2026-09-02 every `scripts/check_*.py` was checked against this
directory and seven domain guards were given pointers in the programs they
protect. That sweep was too coarse, and the way it was too coarse is worth
keeping: **one referenced script can hide dozens of unreferenced rules.**

`check_absence_contracts.py` is a single script — referenced, therefore "covered"
by that sweep — and it runs **37 separate contracts**, each a standing
architectural prohibition with its own name and its own owner. Re-checked at
CONTRACT granularity at `dae963206`: **8 are named somewhere in `docs/planning`, 29
are named nowhere.**

The eight that are named cluster around two programs (public SDK, capability
footprint). The twenty-nine that are not include rules a plan would obviously
want to point at — `central-rollback-does-not-enumerate-domains`,
`rollback-wire-format-changes-are-declared`,
`engine-crates-do-not-consume-the-umbrella-facade`, `engine-core-is-the-floor`,
`geometry-is-the-floor`, `platformer-primitives-stays-a-foundation`,
`the-character-fold-is-not-a-public-capability`,
`the-seat-topology-has-one-engine-side-creator` — and others that are
self-explanatory and need no plan at all.

⛔ **Mapping the 29 to owners is NOT done and is not a mechanical job**: several
belong to campaigns that have closed, and a contract whose rule is obvious from
its name costs nothing by being unlinked. The number is recorded so the next
session starts from 29 rather than from zero.

⇒ **The general form: when you sweep for instrumentation, sweep at the
granularity a READER would look for.** Nobody greps for a filename; they grep for
the rule they are about to break.

### ⛔ The hardest rot to catch: a premise that is still true and no longer the point

Twice in one sweep, a planning doc's stated premise **verified clean by grep**
and its conclusion was nevertheless dead. This is worse than a false claim,
because the obvious check confirms it.

- `engine/participant-action-system.md` §P1: *"`GameMode::allows_gameplay()`
  still treats `Dialogue` as globally unable to route gameplay input."* Still
  literally true — the function is unchanged. But it is no longer the thing
  deciding per-seat gameplay routing; a context claim in `SeatInputContexts` is,
  and the whole open design question had been answered.
- `engine/kinematic-world-objects.md` §K2: *"path motion still carries a string
  `path_id`."* Also still literally true. But authoring became a native LDtk
  `EntityRef`, so the string is a RESOLVED reference rather than the ambiguous
  authored field the item existed to remove.

⇒ **So do not stop at confirming the sentence. Ask what the sentence was FOR,
and check whether anything else now does that job.** The cheap version: grep for
the concept, not only for the identifier — and read the tests, which state
intent in a way a type signature cannot. Both of these were settled by a test
name and its assertion message, not by the code they describe.

### ⛔ Before calling something an omission, look for the reason it is a decision

Three times on 2026-09-02 a re-measurement found code "missing" something, and
twice the absence was deliberate and documented in place. The check costs a
minute; publishing the wrong one costs somebody a fix aimed at correct code.

| looked like | actually |
|---|---|
| `load_prop_sheet_for_target` never consults the quality budget | it hard-codes `Full` **and says why** — "nothing was asked for beyond `Full`" — on a road its docstring scopes to one demo prop |
| `tests/typography.rs` `include_bytes!` a git-ignored font, breaking a fresh checkout | it mirrors `embed_core_assets!`, which embeds the same faces the same way; a runtime read would stop it testing the path the game uses |
| a 7.6 MP sheet loaded outside the demand road, "our loader" | `bevy_ecs_ldtk` loading four `.ldtk` editor-preview tilesets — not our loader at all, and already a known queue row |

⭐ **The tell is consistent: a deliberate absence usually SAYS SO in place, or
mirrors something that does.** `ensure_fx_sheet_loaded` hard-codes `Full` exactly
like the prop loader and gives no reason — which is what makes it a finding and
the prop loader not one. The difference is not the behaviour; it is whether
anybody wrote down that they chose it.

⭐ **AND HERE IS HOW THE UNJUSTIFIED ONES GET WRITTEN, traced on 2026-09-02:
copying a call shape copies its ARGUMENTS but not its JUSTIFICATION.**
`ensure_fx_sheet_loaded` was added the same day, passing
`TextureResolutionScale::Full, Full` in the identical shape as
`load_prop_sheet_for_target`, which had held that pattern for six weeks. The
prop loader's `Full` is correct BECAUSE of a scope note about one demo prop —
and the scope note stayed behind when the pattern moved. Nobody decided FX art
should ignore the tier; a signature was matched.

⇒ **So a sibling that documents itself is weak evidence for the copy.** Ask
whether the sibling's stated reason is about the road you are on, or about its
own.

⇒ **So the question to ask of every "X does not do Y" is: is there a comment, a
sibling, or a scope note that makes Y wrong here?** If yes, the finding is that
the reason is undiscoverable, not that the code is broken — a much smaller and
much more accurate claim.

⇒ **So: when you re-measure a planning file against `HEAD`, leave the receipt** —
the sha, the date, and what you found, including "nothing had changed". A reader
who cannot tell whether a claim was checked yesterday or six weeks ago has to
re-derive it, which is the cost this whole directory exists to avoid.

### ⛔ A COUNT is not a finding unless the instrument travels with it

Three separate planning rows were re-measured on 2026-09-03 by counting
something, and all three counts turned out to be properties of the measurer
rather than of the repository.

| the row | the count it carried | what a careful re-count gave |
|---|---|---|
| resolver population (`declared-id-resolution-checks.md`) | 14, then 42 | **12 / 14 / 29 / 34** depending only on the `grep -A` window |
| knockdown art (`JONS_OBSERVATIONS…`) | 203 sheets, 10 complete | 182 sheets, 12 complete — on a tree where **`git ls-files` tracks zero of them** |
| capability crates (`[census] owners`) | 20 named, `crates=82` | absence uninformative for 62 crates; the row was a silent top-20 |

⇒ **Each one was about to become a finding.** "The resolver population collapsed
from 42 to 14." "Two fighters gained knockdown art." "Sixteen of seventeen
capabilities are dead." All three are false, and all three are what the numbers
say if you compare them without asking how they were produced.

⛔ **The two failure modes, and they are different.**
* **A count taken with an ad-hoc grep** is a property of the flags. The resolver
  number moved by a factor of nearly three across four reasonable ways of asking
  the same question, and none of them is wrong — they are different questions.
* **A count taken over generated or untracked files** is a property of the
  machine. Every sprite manifest under
  `crates/ambition_platformer2d_actor_monolith/assets/sprites` is gitignored, so
  two agents on two hosts get two honest, different, incomparable answers, and
  neither can reproduce the other's.

✔ **What to do instead**, in order of preference:
1. **Track the row by NAMED items, not by a total.** `declared-id-resolution-checks.md`
   reached this independently — *"reopen only on a named site, never on the
   count"* — and the knockdown row's nine named fighters re-measured cleanly on a
   host whose totals could not be compared at all.
2. If a number is genuinely the point, **record the instrument beside it**: the
   exact command, and whether its inputs are tracked. A count with no instrument
   cannot be re-run, only re-invented.
3. Before reporting that a count MOVED, reproduce the OLD number with your new
   instrument. If you cannot, you have measured two different things and the
   trend is not evidence.

⚠ The tell that saved all three: the first re-count returned **exactly** the
row's old number (14), which is far too lucky for an independent measurement of
a repository that had moved for a week. A suspiciously clean agreement is the
same warning as a suspiciously clean absence.

### ⚠ Two instruments can use one WORD for different things

A near-miss on 2026-09-03, caught before it was written down. Two censuses both
say "parallax" about the same room and appear to contradict each other:

* `[census] draws` reports `area_parallax=21771264` in `hall_of_characters` —
  21.7 megapixels of sprite area;
* the image ledger from a `capture_scene` of the same room reports
  `parallax 4×2.4MP` resident and **all four never drawn**.

Resident-but-never-drawn backdrop art in a room that clearly renders is a
finding shape — and the room authors no `parallax_theme` value at all, which
looks like confirmation.

⇒ **They are not measuring the same thing.** `area_parallax` sums sprites by
RENDER LAYER (`PARALLAX_BACKGROUND_LAYER`,
`crates/ambition_render/src/runtime_census.rs:575`); the ledger's `parallax`
groups images by the DEMAND ROAD that loaded them. A sprite drawn on the
parallax layer whose image arrived by another road counts in the first and not
the second, and neither number is wrong.

✔ **The check that dissolves it** is the same one that dissolves a suspicious
count: find the instrument's own definition before reconciling two outputs. One
`grep` for where the field is computed answered it, and answering it took less
time than writing the finding would have.
⚠ This is the failure the sibling page calls a conditionally-blind check, one
level up: not an instrument that cannot see, but two instruments that see
different things and report them in the same vocabulary.


## Sweeping every named test at once — and what the yield tells you

A `✔` that names a test is only worth its ink if the test still exists. That is
mechanically checkable across the whole plan set, so it is worth doing once
rather than one page at a time:

```bash
# every sentence-shaped backticked identifier in docs/planning …
grep -rhoE '`[a-z][a-z0-9_]{19,}`' docs/planning --include='*.md' | tr -d '`' | sort -u
# … minus every function that exists anywhere in the tree
grep -rhoE '\bfn [a-z0-9_]+' --include='*.rs' --exclude-dir=target . | sed 's/fn //' | sort -u
```

⛔ **Search `.py` and `.mjs` too, not just `.rs`.** Several plan pages cite
`scripts/tests/*.py` and `check_*.mjs` tests by name; a Rust-only sweep reports
them as missing and every one of those is a false positive.

⭐ **THE YIELD IS THE POINT, AND IT WAS LOW — WHICH IS ITSELF THE RESULT.** Run
2026-09-03 over 162 sentence-shaped candidates: **2** named a function that
exists nowhere in the tree, and `git log -S` showed both were RENAMES rather
than deletions —
`an_intangible_body_publishes_no_hurtbox_and_names_the_reason` →
`the_artifact_distinguishes_intangible_from_a_coarse_fallback`, and
`a_subjectless_replay_is_admitted_without_recording_an_intent` →
`a_subjectless_replay_records_a_reconstitution_and_owns_the_slot`. Neither test
had been lost; both plan pages had simply kept the pre-rename name.

⇒ **So do not build this into a gate.** A check that fires on ~1% of its
candidates and whose every hit so far is a benign rename would spend more
reviewer attention than it returns, and the false-positive rate before filtering
(377 of 820 raw candidates) is what a naive version would actually report.
⇒ And when a sweep like this DOES hit, `git log -S"<name>" --all -- '*.rs'`
answers "renamed or deleted?" in one command — which is the difference between
repointing a citation and reopening a closed row.
