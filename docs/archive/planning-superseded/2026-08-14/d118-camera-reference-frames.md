# D118 — selectable per-view camera reference frames (DISCHARGED 2026-08-14)

⚠ **EVIDENCE, NOT AUTHORITY.** This is the closed case file for D118, moved out of
the live ledger under that ledger's own rule: *when a row closes, remove its
historical case file, preserve useful history in the archive, and continue.* ⛔ do
not reconstruct anything named here because this file names it. The live
statement of what remains is the D116 row in
[`../../../planning/queue-72h-2026-08-08.md`](../../../planning/queue-72h-2026-08-08.md).

- ▢ **D118 — Add selectable per-view camera reference frames.**

The decision is made: preserve the current world-fixed/external-observer camera
and add an optional subject-relative mode where a view follows the controlled or
designated body's resolved frame, so gravity changes can appear as world
rotation. Use
[`engine/camera-reference-frame-policy.md`](engine/camera-reference-frame-policy.md)
and [`engine/multiplayer-and-multiview.md`](engine/multiplayer-and-multiview.md).

This is a bounded presentation slice and must not become gravity-side special
logic. Keep existing framing/zoom/scroll modes, pair the Ambition product mode
with coherent body-relative movement/aim, make rotated viewport clamps correct,
and leave the representation ready to become per-view under D116. Do not preempt
the remaining D117 actor-kernel ownership work if that campaign is active.

**C1, C2, C3 and C4's portal half are done (2026-08-14).** The composed roll
reaches the resolver, `camera_follow` no longer overwrites it, and the clamp
measures the ROTATED footprint — which closes the live defect where a floor↔wall
transit could show outside the room. ⛔ the composition is not an addition of two
angles; the derivation and its counter-case are in the focused plan under C4.

✔ **C4 AND C3's residue CLOSED 2026-08-14, and this row is now a REST ROW.**
Observer-roll continuity landed (`live_observer_roll` on `CameraEaseState`, eased
at π/0.30s with shortest-path wrapping, portal seams exempt because a transit
ADOPTS its entry roll) — so the world no longer cuts a half turn in one frame
when gravity flips or possession moves. Safe-area framing was re-derived into
SCREEN axes (`apply_soft_subject_framing` takes `roll_radians`; world is y-down,
render y-up, screen y-down, so `flip ∘ R(-θ) ∘ flip = R(θ)` — a plain rotation by
+roll), which fixes a rolled view protecting the wrong screen edge and a quarter
turn swapping the constrained axis. Camera shake needed no change: it composes
with roll by isotropy. ✔ the SELECTION landed with D116 M1 —
`CameraReferenceFrame` is a component on the local view.

⛔ **DO NOT CONTINUE D118 AS A STANDALONE CAMPAIGN** (2026-08-14). Its core
camera-frame implementation is complete: subject-relative roll, rotated clamping,
safe-area framing, easing, portal composition and view-owned policy are all
present. What is left is not camera-frame work at all — **C5 (policy read off the
view index) is N-VIEW work and belongs to D116**, and the remaining feel
questions (shake units, C6 acceptance customers) wait on the maintainer. A row
whose remainder lives in another row is a rest row, not a campaign.

⚠ one MAINTAINER DECISION is parked in
[`awaiting-maintainer-decision.md`](awaiting-maintainer-decision.md): camera
shake's `amplitude_px` behaves as WORLD units, so its felt magnitude scales with
zoom. Three options are filed; none was taken, because picking one answers a feel
question by refactor.

