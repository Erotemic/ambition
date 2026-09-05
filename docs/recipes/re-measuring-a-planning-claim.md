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
⭐ **TWO SECTIONS ARE THE SPINE; the rest are instances.** If you read only
enough to be careful today, read *"The error is not a bad tool, it is a claim
wider than the tool's scope"* and *"A green tool is not a green claim"*. The
first is four tools too narrow for one question; the second is three tools each
green beside a false claim. Between them they cover most of what follows, and
both end in the same one-line defence: **state in a clause what your instrument
actually resolves, then check whether your sentence is bigger than that clause.**

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

### ⛔⛔ A NUMBER ATTACHED TO THE WRONG NOUN, LAUNDERED ONE DOCUMENT AT A TIME

The sections around this one are about figures that went STALE. This is worse
and looks identical: a figure that was never true of the thing it is attached
to, gaining authority at each hop because every reader trusts the last
document.

**Caught 2026-09-03, one step from becoming permanent.** A session killed a
`--rust` run because it had merged under it, reran at the settled tip, and got
6/6 in **1639 s**. That is the duration of the CLEAN RERUN. The killed run has
no duration at all — it never wrote a verdict, which is the entire point of the
story. A handoff then recorded that the session *"lost a 1639 s Rust run"*, and
a second agent read the handoff, believed it, and drafted it into `AGENTS.md`
as the cautionary figure behind a new rule.

⇒ **Three hops, each one reasonable.** The original account was accurate. The
handoff attached its number to the wrong noun. The draft promoted a handoff
line to doctrine, where it would have been permanent, cited, and unreachable
from its source. Nobody lied and nobody was careless; the number simply got
further from the sentence that made it true.

⭐ **WHAT STOPPED IT was refusing to write an unattributable figure.** The
drafter asked *"is this detail right as I have it — I would rather not put a
number in AGENTS.md I cannot attribute"*, and the author of the handoff line
went back to the source and found their own error. ⇒ That question is the whole
defence, and it costs one message.

⇒ **The rule: a number crossing a document boundary must arrive with the noun
it measures**, and promoting a figure into doctrine means re-deriving it from
the source, not from the document you read it in. ⛔ Ask what the number is a
duration OF, a count OF, a size OF — the noun is where these break, not the
digits. And when the honest answer is that the quantity does not exist — a
killed run has no duration — say so instead of substituting the nearest one
that does.

### ⛔⛔ A CUT-OFF METRIC READS A SLOWDOWN AS A REDUCTION

The sections above ask whether a number is stale or attached to the wrong noun.
This one asks a different question: **what would this metric read if the same
thing happened, just later?** If the answer is "a smaller number", the metric
cannot support any claim about magnitude — and it will read every slowdown as a
reduction, in a sentence that sounds like a mechanism.

Anything measured at a fixed cut-off is this: stocks left at the timer, items
collected in N frames, rooms reached inside a budget, encounters that never
started. Each is a *how much has happened by the buzzer* quantity, and none can
separate **less happens** from **the same happens later**.

**Measured 2026-09-04 (peer session, smash ladder).** A 60-second window said
platform stages "halve the lethality" — stocks left `1.64` (platforms) against
`3.78` (flat). Re-run at the shipped clock, nine fixtures, both stages:

| | flat | platforms |
|---|---:|---:|
| mean survival | 91.5s | 167.7s |
| mean **stocks left** | **0.00** | **0.00** |
| fixtures where every bout resolved | 9/9 | 9/9 |

⇒ **Zero on both.** The whole `1.64` vs `3.78` spread was the cut-off. Platforms
were not preventing kills, they were postponing them: 1.83× as long to reach the
*same* end. Pace, not lethality — and the two want different design responses,
so the wrong reading would have driven the wrong change. The companion metric
fell with it: *"41 of 540 bouts unfought"* measured APPROACH TIME, not refusal to
engage, because those bouts had not started yet.

⭐ **THE FIX IS NOT A BETTER STATISTIC. It is a clock long enough for the thing
being counted to finish.** Then a real reduction still shows, and a slowdown
shows up as a duration, which is what it is.

⚠ **The tell that the window produced your spread**: every arm reaches the SAME
terminal value on the longer clock. A metric that separates arms at 60s and
agrees at 170s was reading the timer.

⚠ **And state the prediction before the run.** The peer predicted this outcome in
writing, kept the prediction unedited above the result, and reported it as
loudly as if it had been refuted. A prediction adjusted afterwards is worth
nothing; one that could have been wrong is what makes the agreement evidence.

### ⛔ ONE INSTRUMENT FROM TWO REFERENCE POINTS LOOKS EXACTLY LIKE TWO INSTRUMENTS

The section below is about one word meaning two things. This is its neighbour
and it is harder, because nothing in either figure hints that anything differs.

`status.md` said the actor kernel went **112,733 → 101,042** lines that day.
`queue.md` said **108,364 → 98,808** for the same crate on the same day. Two
bare four-digit line counts, one subject, one date, on two pages a reader
consults together — the only available conclusion is that one page drifted.

⚠ **I concluded they were different instruments and published that**: a raw
`src/` count against the compile ratchet's `largest_unit_lines`. Plausible,
tidy, and wrong. ⇒ Running `scripts/compile_ratchet.py` settles it in one
command: it reports `largest_unit_lines … 108,364 -> 98,509`, and a plain
`wc -l` over the crate's `src/` gives **98,509 exactly**. Same ruler. The
108,364 is the ratchet's **stored BASELINE**, not where the crate stood that
morning — 112,733 is. The pair was never a disagreement about method, only a
delta measured from two reference points.

⛔ **A wrong explanation for a real confusion is worse than the confusion**,
which is why the correction is visible on all three copies rather than silently
swapped in: a plausible explanation settles the question and stops the next
reader looking. Three copies, because the figure had spread —
`status.md`, `queue.md`'s D33 post-mortem and `engine_rename_campaign.md` —
and fixing one leaves the others.

⇒ **The rule: a figure travels with its method AND its reference point.**
Drift-from-a-baseline is not comparable with change-over-a-day no matter how
alike the two numbers look, and "same subject, same date, same units" is not
evidence that two numbers are commensurable. ⭐ And the cheap test is the one I
skipped: when two figures disagree, RUN the instrument rather than reasoning
about what it might be measuring. It took one command.

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


### Sweeping every named test at once — and what the yield tells you

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

### Five shapes a stale planning page actually takes

⭐ Measured, not theorised: these are every distinct way the five oldest pages in
`docs/planning` had gone wrong when they were re-measured on 2026-09-03. Sorted
by how invisible each is to the gate, because that predicts how long it survives.

**1. The page contradicts ITSELF, newest text against oldest.** Re-measurement
blocks accumulate at the TOP while the body keeps its original tense, so the
freshest and stalest text end up on one screen. `dialogue-continuity.md`
declared station-keeping *"⛔ the gap"* in "what exists to build this on" while a
block below it said that gap was closed — and the top is what a reader meets
first. ⇒ When you append a re-measure block, grep the body for what it refutes.

**2. A heading that TALLIES its own list.** `boss-design.md` §9 was headed *"Two
of §3's five rules are NOT implemented"* with Rule 5 struck through and marked
landed two bullets below. The count was correct when written and died in the
same edit that struck the bullet. ⇒ Any heading containing a number about the
section under it is a maintenance liability; prefer a heading that names the
subject and let the list carry the count.

**3. A decision that lives ONLY in a code comment.** `boss-system.md` listed
`BossAnim`→`CharacterAnim` as remaining residue; the fold had been decided
against two days later, and the sole record is the enum's own doc
(*"E6(b) policy: keep this boss-domain vocabulary…"*, `sprite_sheet/src/boss.rs:21`).
`E6(b)` appears nowhere in `docs/`. ⇒ Before implementing a plan row, read the
doc comment on the type it names. The code is the newer document more often than
the plan is.

**4. A dead cross-reference that no link checker can see.**
`decision-principles.md` sent readers to *"`vision.md` §8"* for a digest. In July
that section existed and was called "Principles digest"; vision.md was later
rewritten unnumbered and the digest deleted. ⛔ The LINK still resolves — only
the SECTION died — so `check_doc_links` reports green with it in place. ⇒ Prose
section references (`§8`, "see the table below") are unguarded by construction.

**5. A number whose instrument was never written down.** Covered in full above;
two more instances the same day. *"depended on by 6 manifests"* was a `grep -rl`
count including the crate's own manifest and the workspace root — 4 actually
consume it. And `docs/planning/README.md`'s own paragraph teaching *"quote the
line the gate PRINTS"* had itself retyped `45 crates linked, 18 …`; the contract
now prints 49/22.

⇒ **The through-line: four of the five are invisible to every gate in the
repository**, and the fifth is only visible if someone re-runs the instrument.
None of them is a broken link, a missing file, or a failing test. That is why
re-measurement is a person's job here and why the yield stays high — a page can
be green, well-written, internally cited, and two months wrong.

### ⛔⛔ SYMBOL PRESENCE FAILED IN BOTH DIRECTIONS ON ONE SWEEP — a present symbol can be a different symbol, and an absent one can be a rename

The strongest form of the section below, because the same instrument produced a
false absence AND a false presence within one afternoon, and each pointed the
opposite way from the truth.

**The question:** which of seven stale branches hold work that never landed?
The instrument: does main contain the symbols the branch adds?

* ⛔ **FALSE PRESENCE.** `web-gpu-wait` was called superseded because
  `gpu_prepared` appears in main **31 times**. The branch adds
  `report_gpu_prepared`, which was absent. A substring hit on a related name is
  not the symbol. ⇒ It was genuinely unlanded and merged at `6aa1f3d41`.
* ⛔ **FALSE ABSENCE.** `rescue/specials-are-real-moves-tail` was called
  unlanded because 22 of its 26 added symbols are missing from main —
  `SubmergedMarkerFact`, `SubmergedMarkerSprite`, `SubmergedMarkersView`,
  `MARKER_TEXTURE` and the rest. **They are missing because the feature landed
  under a RENAME**: `TrapdoorVisual`, `TrapdoorSprite`, `build_trapdoor_sprite`,
  `sync_trapdoor_visuals`, `place_door`, with the fact folded into
  `BodyPoseView.submerged` rather than carried as its own view — and main's
  `movement/tests/submerged.rs` is 284 lines to the branch's 130. ⇒ Merging it
  would have replaced newer code with older. The merge was started and aborted.

⭐ **AND THE RENAME WAS ONLY VISIBLE BY READING THE CONFLICT.** No grep finds it:
the old names are absent by construction and the new ones are unguessable from
the branch. A three-way merge is an instrument, and here it was the only one
that could answer.

⇒ **The ranking that survived the sweep:**
1. `git cherry origin/main <branch>` — patch-id equivalence. Proves LANDED
   cheaply and definitively. ⛔ Proves NOTHING on a miss: content reworked
   before landing shows as not-equivalent (three commits did exactly that the
   same day, and main held the richer revision of all three).
2. Symbol presence — **unreliable in both directions**, as above. Useful only
   to generate candidates, never to close one.
3. **Test the branch's stated PURPOSE against main.** Decisive, and cheap when
   the purpose is a signature: `cube-churn-focus-rows` claimed
   `focus_for_action` should take prebuilt rows, and main's already reads
   `rows: &[SystemRow]`. One command, settled.

### ⛔ The instrument error that reports ABSENCE, which is the dangerous direction

A wrong count invites a second look because a number looks like a claim. A wrong
ZERO looks like a finished check, and it ends the investigation.

Two produced false absences on 2026-09-03, an hour apart:

- **`grep -E "a\|b"`.** Under `-E`, alternation is `|`; `\|` is a LITERAL pipe.
  The pattern silently searched for the string `a|b`, matched nothing, and the
  three items it "proved missing" were all present. (`grep "a\|b"` — basic
  regex — is the form where `\|` alternates. The two spellings are exact
  opposites and both are silent.)
- **Searching the wrong tree.** `grep -r gated_by assets/` returned 0 and the
  authored levels live in `game/ambition_map_assets/`. The real answer was 2.

⭐ **FIVE in one session, which is why this has its own heading.** Beyond the two
above: grepping authored worlds for `"goblin_cantina"` WITH quotes returned 0
while the room exists; `grep -rl 'a\|b'` reported files as matching when only
the second name did, so a file was credited with a name it does not contain; and
searching `assets/` and `game/ambition_content/assets/` for character art
returned "no art" for all thirteen characters, because the sprites live under
`crates/ambition_platformer2d_actor_monolith/assets/sprites_potato`. ⇒ Every one
looked like a finding. None was. ⭐ **A sixth and seventh, both from a filename suffix:** searching
`scripts/tests/` for `check_retired_crate_names.py` finds nothing because the test
`import`s the MODULE, `check_retired_crate_names` — which briefly made four live
checks look unreachable; and `ls`'s ANSI colour codes leaked into a comparison and
reported all eleven tool directories as unindexed.

⇒ **Before believing a zero, prove the instrument can see a POSITIVE.** Run it
against something you already know is there — a room you just read, a character
you know ships. It costs one command and it is the only thing that separates
"absent" from "I asked wrong". Both errors above survive any
amount of re-reading and die instantly to one control search — and a zero is
exactly the result nobody re-reads, because it agrees with having finished.

⛔ **AN EIGHTH, 2026-09-03 — and it is the one that beat the control above.**
Re-measuring queue.md's "six dependency declarations are never named in their
crate's source", I swept every manifest's `ambition_*` deps against the crate's
own **`src/`**. It reported eleven. I ran the control this section demands and
the control PASSED: the same instrument saw 2 files for `platformer2d ->
encounter` and 8 for `sim_view -> combat`, so it was provably able to see a
positive. I published the eleven. Four of them were wrong — `platformer2d_host
-> characters` and `-> platformer2d_provider`, `content -> content_cli`,
`app -> demo_pocket` are all used in their crate's **`tests/`**, which `src/`
does not contain. A 67% false-positive rate on the plain edges, from an
instrument that had just passed its own control.

⇒ **A control proves your instrument can SEE; it does not prove it is AIMED at
the whole subject.** Those are two different failures and only one of them dies
to a positive control. The zero was not "I asked wrong" — the query was exactly
right — it was "I asked correctly about two-thirds of the thing". When the claim
is about a whole crate, the scan is the whole crate: `src/`, `tests/`, `benches/`,
`examples/`, `build.rs`. Cargo already names the correct scope and I did not
listen to it — `--all-targets` is that scope, spelled out, in a flag I use daily.
⭐ Ask of any control: *if my instrument were pointed at only part of the
subject, would this control still pass?* Here it would, and did. What caught the
error was not a control but a REVIEWER's methodological caveat — a peer saying
"removals are compiler-verified, never grep-only" — which is a slower and less
reliable net than getting the scope right the first time.

⭐⭐ **THE UNIFYING FORM, after the same day produced FOUR of these on one
question: THE ERROR IS NOT A BAD TOOL, IT IS A CLAIM WIDER THAN THE TOOL'S
SCOPE.** Asking "which dependencies are never named in their crate's source":

| the tool measured | I claimed |
|---|---|
| occurrences in `src/` | the whole crate — missed four uses in `tests/` |
| occurrences of TEXT | usage — an intra-doc link counted as a use |
| `ambition_*` names only | every dependency — missed `thiserror`, `ron` |
| a build with DEFAULT features | the crate's dependencies — `ron` is real code behind `#[cfg(feature = "content_pack")]` |

⇒ Every row is a tool that **succeeded**. None errored, none returned nothing,
none looked broken; each answered its own narrower question correctly and I
widened the answer on the way to the page. That is why this class survives
controls, re-reading, and a green gate: **there is no failure to notice.**
⇒ So the check is not "did it run?" but **"what exactly did it measure, and is
my sentence larger than that?"** Write the scope into the claim — *"never named
in `src/` on a default-feature build"* is a sentence you can defend and a
reviewer can attack; *"never used"* is neither.

⚠ The practical form for dependencies specifically: the default-feature lint is
a DETECTOR that over-reports, and the confirmation is a second run with
`--all-features`. **Only a dep unused under BOTH is unused.** Detector then
confirmer costs two runs on the hits rather than two runs on all 77 crates.

### ⛔ A GREEN TOOL IS NOT A GREEN CLAIM — three tools, three false claims

The table above is four tools too narrow for ONE question. This is the other
half, found the same day by the coordinator session and recorded here at their
request: three DIFFERENT tools, each green, each beside a claim that was false.

- **`check_planning_citations.py --strict` read 1,222 citations all-resolved
  BEFORE AND AFTER five flatly false sentences were fixed.** It resolves cited
  SYMBOLS; a claim about where code LIVES is prose. Six crates left the actor
  monolith that day, every citation the tooling could see was updated, and every
  sentence that merely *described* a location survived — *"…is
  `actor_monolith::items::pickup`, which stayed in the kernel"* reads as settled
  architecture rather than as a dated fact. ⇒ Sweep old paths after a carve and
  sort the hits into HISTORY vs STALE, then **re-tense rather than delete**: of
  eight hits on one path, five were correct history, so a purge would have
  broken five true sentences to fix three false ones.
- **A memorised rule applied to the wrong situation.** Five docs tell readers to
  run `check_absence_contracts.py | grep footprint`, and the rule *"a pipe voids
  the exit code"* nearly got all five "corrected". That checker prints `ok`/`FAIL` <!-- cite-ok: quoted TOOL OUTPUT, not a symbol — the point is where the verdict is printed -->
  **inside each line**, so the verdict survives the pipe and the rule did not
  apply. ⇒ A remembered rule is a hypothesis ABOUT A SITUATION, not a fact about
  a command: ask **where the tool puts its verdict** before correcting a pipe.
  (The rule itself is real — see the exit-status rows in
  [`checks-that-did-not-run.md`](checks-that-did-not-run.md) — which is exactly
  what makes it dangerous to apply without looking.)
- ⭐ **The sharpest one: a RETRACTION that was itself the mistake.** A
  `theme_loaded` gate in `sync_session_room_visuals` returns before the block
  loop; it was read, matched to that function's own comment about a room not
  being *"held hostage to a backdrop that is never coming"*, and published as the
  `painted_blocks` mechanism. `painted_blocks` then measured **4/4 green**, so
  the claim was retracted. Hours later two other sessions traced the Ultra hall's
  129-warning NPC placeholder burst to **that exact gate**. ⇒ **Right mechanism,
  wrong claim attached to it; retracted for the wrong reason; and only a third
  party's independent need revealed the reading had been correct all along.**

⭐ **What saved the third one is the whole method of this page.** It had been
written up as a SOURCE READING — what the gate does, quoted, with its file and
its comment — rather than as a finding. So the retraction cost one paragraph and
the rescue cost one paragraph, and the evidence was still sitting there to be
rescued BY. Had it been published as a conclusion ("the burst is caused by
X") the retraction would have deleted the observation along with the claim.

⇒ **A retraction can also be a mistake.** The defence is the same one this page
asks for everywhere else: **say what the evidence IS, not only what you
concluded from it** — a wrong conclusion over a correct observation is one edit
to fix, while a wrong conclusion that consumed its observation is a re-run.

⚠ ⇒ **The combined rule, and it is the day's finding:** an instrument's green is
a statement about the instrument's own question. **Before quoting a passing
check as support, say in one clause what that check actually resolves** — symbols
for the citation checker, default-feature compilation for the dependency lint,
`src/` for a source sweep. If that clause is narrower than your sentence, the
green is not evidence for your sentence.

⇒ The residue is worth keeping too: those four are not bugs in the code, they are
deps used only by tests while declared in `[dependencies]` rather than
`[dev-dependencies]`. The mis-scan found a real, smaller thing while claiming a
bigger one. **Do not let the smaller true finding launder the retraction of the
larger false one** — report the retraction first and the residue second, in that
order, or the correction reads as a discovery.

### The mirror: a frightening POSITIVE that the rule's own kind explains

The section above is about believing a zero. This is the same error inverted, and
it produced the most alarming number of the day.

Sweeping every source path literal in `tests/ambition_workspace_policy/policies/*.toml`
for files that do not exist returns **45 unresolved out of 365** — in the data
driving a gate, which reads like forty-five rules quietly governing nothing.

⇒ **All 45 belong to `kind = "forbidden-path"` rules, where a path that does NOT
exist is the PASSING state.** `engine.actor-portal-facade-gone` lists
`…/src/portal/{mod,plugin,transit,gun,presentation}.rs` precisely because the
portal facade was deleted; the day those paths resolve, the rule fails, which is
its whole job. Split by `kind`, the real count of unresolved paths in rules that
need their targets present is **zero**.

⚠ The check that dissolves it is one line — group the findings by the `kind`
field before reading them — and the temptation not to run it is strongest
exactly when the raw number is large enough to feel like a discovery.

⇒ **The general rule, and it is the same one both directions:** a
presence/absence sweep over rule DATA is meaningless until you know what each
rule ASSERTS. Existence is evidence only relative to a claim, and half the claims
in a policy engine are negative.

### Surfaces swept clean on 2026-09-03, so you do not sweep them again

A negative result is worth writing down once, with its date and its command, or
the next person pays for it a second time. All of these were run against
`fdb83d4e9` and found **nothing** — each took under a minute:

| surface | population | result |
|---|---|---|
| script paths cited anywhere in `docs/` (`scripts/`, `tools/`, `dev/`) | 372 citations | 0 genuinely missing |
| doc paths cited in `tests/ambition_workspace_policy/` | 262 citations, 13 documents | 0 unresolved, and only ONE cited doc is a distilled receipt |
| `paths = [...]` literals in the policy TOMLs | 365 literals | 0 unresolved in rules that need their target present |
| `related_docs:` frontmatter across `docs/` | 125 entries | 0 missing |
| path citations in `AGENTS.md` | 22 | 0 missing |
| pages under `docs/` that nothing links to | 278 documents | 3, of which 2 are closed receipts |
| tools missing from `docs/tools/index.md` | 11 tool directories | 0 — the index groups by guide, and all are covered |
| CamelCase types cited in `docs/concepts/` + `docs/architecture/` | 73 distinct | 0 real — every miss is an upstream type, an enum VARIANT, an LDtk entity id, or a name the page says must NOT exist |
| CamelCase types cited in `docs/systems/` | 65 distinct | **1 real** — `ResetRoomFeaturesEvent`, listed under "Current boundary" and deleted four days earlier | <!-- cite-ok: the sweep table records this deleted type as its one real finding -->
| every `scripts/check_*.py` — is it reachable from the gate, the pytest lane or CI? | 16 scripts | **0 unreachable**, confirming e7's CI-only fix closed completely |
| identifiers cited in `docs/tools/` + `docs/sdk/` | 165 distinct | 0 real — env vars, a user's own example type, a prototype's proposed names, and three functions named inside POSTMORTEM sections about their own removal |
| `snake_case` cited in `docs/concepts/` + `docs/architecture/` + `docs/recipes/` | 189 distinct | **1 real** — `player_body_tick`, doctrine describing a system a live test forbids |

⚠ **Three of those seven produced a scary intermediate number that dissolved on
inspection**, and each dissolved for a reason worth knowing: 45 "unresolved"
policy paths were all `forbidden-path` rules; 5 "unindexed" tools were all
covered by group guides the index points to; 3 "missing" scripts were a dated
patch record, an elided `.../` shape, and a row already carrying `cite-ok`.
⇒ **The population is never the finding.** Read what the rule asserts, what the
index claims to be, and what the citation is FOR, before counting anything as
wrong.

⭐ **The type sweep is the one to repeat, and it needs a filter to be usable.**
Raw, it reports ~45% of citations as missing and is worthless. Four categories
account for nearly all of that — upstream types (`Entity`, `Query`,
`SystemParam`), enum VARIANTS (a `struct|enum|trait|type` grep sees declarations
only), LDtk entity identifiers (`LockWall`, `OneWayPlatform`), and names a page
deliberately says must NOT exist (*"do not reintroduce a `PlayerAttackState` /
`ActorAttackState` split"*). ⇒ Subtract those four and the signal is one row in
138 citations — but that row was a message deleted four days earlier and still
documented as current, which is exactly the rot no link checker or test can see.

⛔ **And point it at `docs/systems/`, not `docs/planning/`.** Run over planning
the same sweep returns 47 names absent from all Rust, and nearly every one is
CORRECT: a plan proposing `EncounterScript` over triggers named `RopeCut` and
`MemberAtPosition` is doing its job, and `ParticipantUnlock` is absent because
its own page says so. ⇒ The sweep tests "does this exist", which is only a
question worth asking of a document that claims something DOES. A systems page
describing a current boundary must resolve; a planning page naming what to build
must not.

### ⭐ Before writing a grep, ask whether the COMPILER already answers it

The queue row "N dependency declarations are never named in their crate's
source" is a question about Rust, asked in text. I answered it in text: a sweep
of every manifest's `ambition_*` deps against the crate's own files. It took
three iterations to stop being wrong, and it was wrong in BOTH directions.

- **False positives, 4 of 6.** The sweep read `src/` only, so four deps used in
  the crate's `tests/` were reported stranded. See the eighth false-absence
  entry above.
- **A false negative, and it is the prettier failure.** Widened to the whole
  crate, the sweep still missed `ambition_abilities -> ambition_items`, because
  the string does occur in the crate — once, inside a `//!` doc comment, as the
  intra-doc link ``[`ambition_items::Item::Grapple`]``. A DOCUMENTATION
  CITATION made a dependency look live to a text search. No amount of widening
  the file set fixes this one; the fix is to stop matching text.

⇒ `rustc` has answered this question since 1.44: **`-W unused_crate_dependencies`**.
One command, no manifest edit, no heuristic:

    cargo rustc -p <crate> --lib -- -W unused_crate_dependencies

It named all three in `ambition_abilities` in 58 seconds, including the one I
could not have found by reading, and it cannot produce either error above
because it is reading the same resolved crate graph the build reads. Prefer
`cargo rustc` over a `RUSTFLAGS=` run: the flag then applies to that crate only,
so the shared target cache is not invalidated and every dependency stays warm.

⭐ This is the twin of *"check whether a test already drives it"* further down.
Both say: the repository and its toolchain already contain instruments better
than the one you are about to build, and the cost of not looking is not just the
build time — it is that a hand-rolled instrument fails SILENTLY and in
directions you did not enumerate. I published a wrong count from mine before the
compiler corrected it.

⚠ **What the lint does NOT settle.** `--lib` asks only whether the library
target names the crate. Three residues need a human:
  • a dep used only by `tests/` is not unused, it is MISFILED — it belongs in
    `[dev-dependencies]`, and the lint on `--lib` will call it unused;
  • an optional dep referenced from the manifest's own `[features]` table
    (`causal = ["dep:ambition_causal"]`) is public feature surface, and removing
    it changes what downstream crates can turn on;
  • a dep reached only by an intra-doc link is load-bearing FOR THE DOCS —
    delete the dependency and rustdoc's link breaks, so the removal is two edits,
    not one.
⇒ So the lint replaces the grep as the DETECTOR and does not replace the
judgement about what each hit means.

### ⭐ Before building an instrument, check whether a test already drives it

The most expensive re-measurement of 2026-09-03 was the one that needed no new
tooling at all.

A planning entry had stood for weeks saying a residency ceiling *"cannot be taken
on a software rasteriser"* and needed time on a 3090. Two tools were checked —
`capture_scene`, which cannot cross a door, and `profile_desktop.sh`, which
refuses without a display — and the conclusion was "no walk driver exists here".

⛔ **The walk driver was `game/ambition_app/tests/hall_transition_cover.rs`**,
which boots the full app with `build_visible_app(NoWindow)` and drives *"the REAL
transition, resolved through the room graph rather than synthesised: stand in the
Hall door and press interact"*. It is a module of `app_it`, so it had been
crossing that door **on every gate run**, printing the census with `eprintln!`,
and having it swallowed by libtest's output capture. Getting the number took a
test filter and `--nocapture`:

```bash
cargo test -p ambition_app --test app_it <module>::<test> -- --nocapture --test-threads=1
```

⇒ **The rule: when a claim says a measurement is impossible here, search the TEST
SUITE before believing it.** Acceptance tests boot real compositions and drive
real content; they are instruments that happen to assert. A test written to check
one thing usually measures ten, and nine of them are discarded every run.

⚠ **And the number is a property of the COMPOSITION, not just the subject.** The
same room reported 119.4 MB through `capture_scene` and 1452 MB through the test —
twelve times the pixels — because one composition seeds visual quality from the
Cpu adapter and loads `sprites_potato` while the other loads the base tree.
⇒ Neither is wrong. A residency figure without the composition named beside it is
not a measurement, it is a number.

### ⛔ When you CORRECT a count, re-test the members you inherited

The most expensive error of 2026-09-03 was not a stale number or a bad grep. It
was a correction that corrected only the part I was looking at.

The decomposition plan said eight kernel modules were "already islands". I
re-measured, found six more, and published **14** — with a rule stated in the
same paragraph: *no out-edges, and nothing pointing back except
`rollback_registration` and `snapshot_impls`*. The real answer by that rule is
**11**. `causal`, `body_mode` and `music` have zero out-edges but are read — by
`features` (8 refs), `avatar` (1) and `audio` (4).

⇒ **The arithmetic shows exactly where the rigour stopped.** Sixteen modules have
zero out-edges. I applied the in-edge half of the rule to the SIX I was adding,
and to the two I excluded by name — 16 − 2 = 14. I never applied it to the EIGHT
I had inherited from the sentence above, because they arrived already labelled
"islands" and my job felt like *extending* the list. The published number was
neither the loose rule (16) nor the stated rule (11): it was **the stated rule
applied only to the members I happened to be holding**.

⭐ **A correction inherits the credibility of the thing it corrects.** "8 → 14,
re-measured" reads as though all fourteen were measured, and the six new ones
were. That is what makes this worse than an ordinary stale figure: a stale
figure announces its age, while a half-re-measured list announces the opposite
of its age. ⇒ **Re-run the rule over the WHOLE population, including the members
you are keeping.** If a claim's rule is worth stating, it is worth applying to
the names you did not touch — and if that is too expensive, say which members
were re-tested and which were carried forward.

⭐ **THE GENERAL FORM, found the same day by auditing the rest of my own
figures: A TRUE COUNT LENDS ITS CREDIBILITY TO A MEMBERSHIP NOBODY CHECKED.**
The decomposition plan priced an inversion at *"1 production file, 16
references, 5 distinct recipes named"* and listed the five. The counts were
exactly right and re-verified a day later — one file, sixteen references,
`tests.rs` at 38. **Four of the five names do not occur in the file at all.**
Same failure as the islands count, from the other side: there the rule was
right and the members were not re-tested; here the count was right and the
names were never taken from the file.

⇒ The mechanism is that a reader spot-checks the CHECKABLE part. `wc -l` and a
`grep -c` are one command, so a sceptical reader tests those, they pass, and the
list of names rides along on the credibility they just earned. ⇒ **When a claim
is a count PLUS a list, they are two claims and only one of them is cheap.**
Re-take the names from the file — `grep -o 'features::[A-Za-z_:]*' … | sort -u`
is as cheap as the count was, and it is the half nobody runs.

⚠ Two cheap habits would each have caught it. **State the rule as a filter and
run it**, rather than stating it in prose and hand-checking; the eleven fell out
of a six-line script that no more trusted the old list than the new. And **run
the control against your own tree**: I only found this because a peer made me
date the number, so I measured `origin/main` (11) and then my own branch
expecting 14 — the control returned 11 with an identical module list, which
proved the gap was never drift. See [checks-that-did-not-run](checks-that-did-not-run.md).

### ⚠ A disagreeing re-measurement is only DRIFT if both sides measured the same thing

This page spends most of its length on numbers that went stale, so it needs the
opposite rule beside them or it teaches over-correction.

An entry raised 2026-09-02 says *"the shared sprite pack is 442.6 MB and one prop
reads it"*, over *"197 targets"*. Re-run on another machine the next day, the same
script reported **164 targets** and the directory measured **318 MB**. Two figures
disagreeing by a quarter, one day later, with the instrument named — the shape
this page has been correcting all day.

⛔ **It is not drift.** Sprite packs are generated and gitignored, and the sibling
script says so of its own subject: *"these are gitignored generated files, and
this is ONE machine's tree."* Two machines with different regeneration histories
produce different populations, and neither number is wrong or stale.

⇒ **The load-bearing half verified exactly** — *"1 target(s) opt into the pack —
`intro_cart`"* — which is what the entry actually argues from.

⭐ **AND THAT IS THE CONSTRUCTIVE LESSON: write the claim so it survives.** Three
measurements from `asset-preparation-and-residency.md` were re-run on a second
machine a day later. Every absolute count moved — 197 targets → 164, 442.6 MB →
318, 225 pages → 201, 662 MP → 580 — and **every conclusion held**: one prop
reads the pack, occupancy is ~90% and "not a lead". ⇒ The conclusions survived
because each argues from a RATIO or a SINGLE NAMED CONSUMER rather than a size.
⇒ So when a finding needs a number, prefer the form that cannot rot: *"one target
opts in"* outlives *"442.6 MB"*, and *"90% occupied"* outlives *"66.6 MP of
waste"* — on any machine, in any regeneration state.

✔ **AND THE RULE WAS CONFIRMED FROM BOTH SIDES, which is what makes it more than
a hunch.** A recorded defect — *"four sheets' reduced tiers are not reduced"* —
did not reproduce on the second machine: `measure_tier_variant_scaling.py`
reported **0** violating sheets there and the files were genuinely smaller
(`author_spritesheet.png` 4.3M full against 2.0M at `0_5x`). Rather than declare
it fixed, the ambiguity was reported. The originating box then re-ran it and
still saw **4** — its variants are stale, the second machine's are fresh.
⇒ **Both measurements were correct and the disagreement was the regeneration
history**, exactly as the rule predicts. ⇒ Note what would have happened
otherwise: "fixed, does not reproduce" would have closed a live defect on the
strength of a build directory.

⭐ **AND THE DISAMBIGUATING INSTRUMENT ALREADY EXISTED — run it FIRST next time.**
`scripts/check_quality_variants_are_fresh.py` answers "is this tree's generated
output current?" in one command, and it answered differently on the two machines
the same hour: *"quality tiers are current"*, exit 0, on the box that saw no
defect; **82** stale variants on the box that saw four. ⇒ So the general move,
whenever a re-measurement of generated content disagrees, is not to reason about
regeneration histories — it is to ask each tree whether its build output is
fresh, before comparing anything downstream of it. ⇒ **So before
correcting a number, ask whether the thing it counts is repository content or
build output.** Repository content that disagrees is drift. Build output that
disagrees is two machines, and rewriting one machine's figure with another's
manufactures a finding out of a build directory.

## A red commit-citation may be about YOUR CHECKOUT, not the citation

⛔⛔ **I WROTE THE OPPOSITE HERE AN HOUR AGO AND IT WAS FALSE.** The claim was
that `test_the_live_planning_tree_has_no_fabricated_commit` walks the
superproject only, so a submodule SHA reads as fabricated and the fix is to
name the repository in prose. **The checker asks the submodule** — its own
source says so (*"of the eight the survey flagged were submodule commits… the
checker asks the submodule, which is a fact"*), and my "form that passes"
example passed because the submodule HELD that commit, not because of anything
in the sentence.

⭐ **What the red actually means, measured:** `a5d9f3a` failed here and passed
on the machine that wrote it, at the same superproject commit — because **a
superproject `git fetch` does not fetch submodule branches**. The commit existed
there and not here. ⇒ One `git -C tools/ambition_music_renderer fetch --all`
and the arm went from 1 failed to **9 passed**, with nothing edited.

⇒ **So before treating a commit-citation red as a bad citation, fetch:**
`git submodule foreach git fetch`. The checker prints this advice itself. A
guard whose verdict differs between two machines at one commit is answering a
question about the reader, and that is worse than a wrong citation because
neither reader can reproduce the other's result.

⚠ **Qualify anyway, for the READER rather than the checker.** *"(`a5d9f3a` — a
commit in `tools/ambition_music_renderer`)"* does not change the check, and it
is still worth writing: a bare submodule SHA resolves for a human only while
that submodule happens to be checked out at a commit whose history contains it,
and a fresh clone at a different pointer gets nothing.

⛔ **AND A REWRITTEN SHA LOOKS IDENTICAL TO A FOREIGN ONE.** Same assertion,
same message, opposite remedies — the rewritten one wants REPLACING with the
surviving commit, the foreign one wants FETCHING (and, for the reader, naming).
⇒ `git cat-file -t <sha>` inside the submodule separates them in one command.
Both happened on 2026-09-04, hours apart, and the first taught the wrong lesson
for the second.
