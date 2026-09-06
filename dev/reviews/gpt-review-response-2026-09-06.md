# Response to the GPT review of `9948acdf5409 → de5aba4f113b` (2026-09-06)

What was done, what was routed, and — the part worth reading — **what the review
asked for that could no longer be reproduced, and why.**

## HIGH — portal dependent visibility · FIXED (`82af28e25`)

The finding reproduced exactly, and the tell was an asymmetry inside one function:
the **body** loop inserts `PortalSourceHidden` and has an `else if we_hid_it`
release; the **dependent** loop had neither.

* **The latch.** `PortalDependantHidden` is now the claim, and the release
  **restores** — the opposite of the body branch, *for the reason that branch
  itself gives*. It asserts no value because "every population this reaches has a
  per-frame owner"; the dependent population does not (`hit_flash`'s update path
  says visibility "stays `Visible` permanently"), which is exactly why a silent
  release latched it hidden for the session. `Inherited` rather than `Visible`, so
  a parented dependent defers instead of overriding a legitimate hide.
  ⚠ A separate marker is forced, not stylistic: a dependent carrying a body marker
  would match both queries and Bevy refuses the system (B0001).
* **The geometry.** `publish_portal_compositing_candidates` already admits
  `PresentationOf` drawables, so an unparented **sprite** dependent classifies
  itself from its own bounds and now skips the fallback entirely.
  ⚠ `Without<ChildOf>` is load-bearing — the publisher substitutes the LOCAL
  transform for the world one, so a *parented* sprite is not a candidate and still
  needs the fallback.
* **The false green.** The existing test wrote `Visibility::Visible` onto the
  silhouette before its second update, on the stated premise that "the drawable's
  own writer sets its value every frame" — false for the population the seam
  serves. Nothing touches it now. Two poisons, each failing only its own arm.

◐ **NOT CLAIMED FIXED**: the hit-flash `Mesh2d` still takes a whole-drawable hide.
Route and cost recorded in `docs/planning/engine/render-animation-and-vfx.md`
(`0509e05b1`) — `clip_material.rs` says it was modelled on the hit-flash overlay,
so a clip half-plane uniform is the symmetric change; what makes it real work is
plumbing the pane plane to the overlay and giving a straddling body two flash
pieces.

## MEDIUM ×2 — riposte/tether body frame, tether first tick · ROUTED

`game/ambition_demo_smash/**` is the fighter lane's. Relayed with the diagnosis,
including that **every existing riposte test installs
`ResolvedMotionFrame::default()`**, so the 90° poison with a non-square hitbox and
two victims is what the acceptance needs.

## MEDIUM — v3 stateful constraints · FIXED (`d911d1f`, submodule)

One score-scoped `voicing_state` dict carried across generator clips; generic
two-clip parity test; poison-verified. `docs/musicir_v3.md` corrected: `intensity`
is a positive gain, not a bounded 0..1 intent, and the reviewed v1 values
(`0.88 … 1.24`) are *why* the rule is what it is.

⛔⛔ **THE REVIEW'S OWN WITNESS NO LONGER EXISTS, AND THAT IS THE MOST USEFUL THING
HERE.** It cited v1's `E4-G4-B5` against the bridge's `E4-G4-B4`. At HEAD,
`scores/active/standing_on_shoulders.music.yaml` is `schema: ambition.musicir.v1`:
`git show ff26b4f:` still carries the "Parity-first v3 source" header the review
quotes, and the **unpushed** `f2ae21f` "Update music" replaced it with a v1 score.
Measured: `_expand_generator_clips` runs **zero** times for that song now.
⇒ The defect was real and is fixed; its published evidence stopped meaning anything
and nothing announced that. ⚠ I nearly reported "fixed — 2,340 notes, multisets
identical" off that comparison **with my own fix poisoned out**.

## LOW · FIXED — `smash_riposte.rs` EOF blank line (`82af28e25`).

## Maintainer decision · UNTOUCHED — the map-assets keep/discard is Jon's (#62).

## ⚠ One thing the review could not see: the box was at its disk floor

`--rust` returned 5/6 on a test of mine that asserted `returncode == 0` from a
`--maintenance` run. The cause was **40 GB free** — the suite refuses a job under
its floor — so a disk condition printed itself as a nesting-guard failure.
Reclaimed 98G of stale incremental (40G → 136G) with the sanctioned script, and the
test now skips with the real reason. **A red test's message is the first thing its
reader believes**, and mine named the wrong subsystem.
