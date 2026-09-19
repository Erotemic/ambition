# Triage — diagnosed, not yet scheduled

Each page here is one finding that was measured and written down, and that nobody
has scheduled. They are not proposals and they are not a queue: a page arrives
when the diagnosis is complete and the DECISION about whether to act is somebody
else's, or when the cost of acting is not yet known.

⛔⛤ **THIS INDEX EXISTS BECAUSE TWO OF THESE PAGES WERE REACHABLE FROM NOTHING
FOR SIX DAYS, AND BOTH TURNED OUT TO BE LIVE.** Swept 2026-09-16: of 305
documents under `docs/`, 21 were linked from no other document, and two of those
were open rows here with named next steps that had never been taken. One of them
recorded the SAME failure signature that hit `app_it` that night. ⇒ **A diagnosis
nobody can find is a diagnosis nobody has**, and the cost of the index is one
file.

⚠ A page here going stale is expected and is not a defect. Check its Status line
and its date before acting on it, and re-derive its measurement — see
[`../../recipes/re-measuring-a-planning-claim.md`](../../recipes/re-measuring-a-planning-claim.md).

## Open findings

⚠ One CLOSED page is kept in this section rather than moved: its value is now the
measurement it carries, and the index would lose that by demoting it to a link.

- [`a-composition-acceptance-that-only-fails-in-company.md`](a-composition-acceptance-that-only-fails-in-company.md)
  — an isolation test that FAILS in a full run and PASSES alone, intermittently.
  ⭐ **Live:** the same signature hit a different arm on 2026-09-16, so this is a
  CLASS with at least two instances. Its named next step is `--test-threads=1`,
  and [`../queue.md`](../queue.md)'s TEST-LANES row now links it.
- [`a-prose-path-inside-a-doc-comment-is-not-checked.md`](a-prose-path-inside-a-doc-comment-is-not-checked.md)
  — ✔ **CLOSED 2026-09-17, and kept here because the page is now the receipt.**
  Nothing read the paths a `.rs` comment names, so a carve broke them in silence;
  ten live citations were pointing at nothing when it was finally censused, two
  of them telling a reader the simulation phase order is *"configured by
  `app/schedule.rs`"* <!-- cite-ok: quoting the dead path the row is about -->, a
  file that does not exist. The objection that kept it parked — a checker buys a
  suppression list — was measured and did not survive: the legitimate cases
  announce themselves, the annotation cost was fourteen lines in a convention
  that already existed. ⚠ `--comment-paths` REPORTS in the default lane; it was
  registered as a gate and demoted the same day under `AGENTS.md`'s rule against
  permanent file-location machinery.
  ⭐ **AND THE PAGE GREW TWO MORE AXES ON 2026-09-19, so "CLOSED" names the
  path half only.** A guard's WAIVER can quote source verbatim and nothing
  resolves the quotation — found live, with the quoted comment deleted five
  days earlier. Censusing the same shape across `docs/planning`: 783
  quotations, 32 attributed to a `.rs` file, **4 genuine defects**, all
  repaired. And the gap underneath both: `--vanished` is the only pass that can
  judge a BARE name, and it read only documents, so a deleted method survived
  in five source comments while the QUALIFIED spelling of the same method was
  caught and repaired. It now reads source comments too when asked
  (`--vanished REF --comments`), which is opt-in because at the lane's baseline
  it reports 246 findings across 109 names — 223 across 104 once this page's
  own repairs landed.

## Design and scope pages awaiting a decision

- [`ambition-registry-core.md`](ambition-registry-core.md) — registry protocol,
  identity, and the deliberate-duplicate policy.
- [`ambition-test-support.md`](ambition-test-support.md) — sequester harness
  boilerplate so behavioural tests are cheap to write.
- [`bevy-system-parameter-architecture.md`](bevy-system-parameter-architecture.md)
  — system parameters and mutation boundaries.
- [`declared-id-resolution-checks.md`](declared-id-resolution-checks.md) —
  remaining authoring diagnostics for declared-id resolution.
- [`gameplay-presentation-profiles.md`](gameplay-presentation-profiles.md) —
  remaining presentation-profile work.
- [`stable-identifier-centralization.md`](stable-identifier-centralization.md) —
  semantic scope before shared syntax. ⚠ `tracks.md` gates this on *"concrete
  identity families that share actual operations"*.
- [`unused-dependency-census.md`](unused-dependency-census.md) — a
  compiler-verified census of unused dependency declarations.

## Deferred on an external or product condition

- [`character-dialogue-from-suggestions.md`](character-dialogue-from-suggestions.md)
  — SHELVED 2026-07-26 pending a design settle.
- [`leafwing-clash-scan-patch-2026-07-23.md`](leafwing-clash-scan-patch-2026-07-23.md)
  — a deferred UPSTREAM patch. `tracks.md` gates it on a dependency/version
  change or a measured CPU cost that matters to a declared profile.
