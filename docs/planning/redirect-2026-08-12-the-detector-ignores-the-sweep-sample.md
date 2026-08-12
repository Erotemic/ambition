# Redirect, 2026-08-12 — the loading-zone detector is the last swept reader still reconstructing motion from velocity

**Jon, verbatim:**

> Note, in ambition I can't use "F" to go through doors anymore, and in smash, I
> see the new emmy sprite on the select screen, but her character is the old
> sprite in the match. In fact going through loading zones doesn't work anymore.

> When I change the video quality in ambition, my sprite went from the robot v3
> character to the robot v2 character.

> I moved into a loading zone and the room didn't change. that is not a key
> binding issue. at the very least maybe we need more logs.

⭐ **the second message is the correction.** My first pass answered the two sprite
reports (they were one fact — stale reduced-resolution tiers, ledger D91) and left
the loading-zone report unreproduced with a keyboard-preset theory attached. Jon's
automatic-zone observation disproves that theory outright: an `EdgeExit` needs no
key. The theory was reachable only because I had stopped at "every test is green",
and the tests were green about the wrong seam.

---

## GPT 5.6's review, verbatim (relayed by Jon, "Worked for 9m 23s")

> Yes. Your loading-zone observation rules out the key-binding explanation, and I
> found a much stronger engine-level problem.
>
> The room-transition detector is currently the odd swept-path reader out. After
> the controlled body has already gone through the movement kernel,
> `detect_room_transition_system` reconstructs the path like this:
>
> ```rust
> let delta = kin.vel * world_time.sim_dt();
> ```
>
> and feeds that to `transition_for_player`.
>
> That is inconsistent with the engine's own collision/CCD architecture.
> `SweepSample` exists specifically to record the body's actual movement segment
> `prev -> curr` inside the movement kernel, and its documentation says `vel * dt`
> is only the historical fallback for bodies that do not have a sample. The
> collision doctrine is even more explicit: reconstructing a path from velocity is
> forbidden when a canonical sweep sample exists.
>
> This matters because the collision solver changes velocity after the body has
> moved. On a collision it advances the body to time-of-impact and then can zero
> that velocity axis. So a frame can truthfully be:
>
> ```text
> actual movement:
>     prev ---------------------> curr at loading-zone boundary
>
> SweepSample.delta():
>     nonzero, correct
>
> kin.vel after collision:
>     0
>
> room-transition reconstruction:
>     delta = 0 * dt = 0
> ```
>
> If the body ended merely touching the loading-zone boundary rather than strictly
> inside it, the transition detector has just thrown away the exact segment proving
> that the body entered the zone. "I touched a loading zone and did not go through
> it" is almost a textbook manifestation of this mistake.
>
> And this explains why the existing tests falsely reassured the other agent. The
> test named `a_fast_body_cannot_tunnel_a_walk_loading_zone` does not run the body
> through the movement kernel at all. It manually chooses start, chooses end, and
> then manufactures `vel = (end - start) / dt`. In other words, the test supplies
> the detector with an artificially perfect final velocity, which is precisely the
> assumption that fails in the real game. The door tests similarly place the body
> directly into the zone. They prove the later machinery, not natural entry through
> movement.
>
> There is particularly strong sibling evidence. Hazard collision already does the
> right thing:
>
> ```rust
> let delta = sweep
>     .map(|s| s.delta())
>     .unwrap_or(kin.vel * dt);
> ```
>
> because hazards were migrated onto the canonical sweep model. Portal transit
> likewise consumes `SweepSample`, rather than reconstructing movement from final
> velocity. Room transitions appear simply to have missed that migration.
>
> The repair I would make is therefore not "make zones more forgiving." It is to
> make loading-zone detection consume the same authoritative motion fact as the
> rest of the engine. The query should include `Option<&SweepSample>`, use
> `sample.delta()` for ordinary kernel bodies, and retain `vel * dt` only as the
> legacy fallback for a body that genuinely lacks a sample. There is a small
> post-integration wrinkle: portal transit happens in `PlayerSimulation`, before
> `RoomTransition`, but the portal adapter correctly calls the movement authority's
> reconciliation, which collapses the sample to a zero-length segment at the
> arrival. So the architecture already supports this cleanly.
>
> I would replace/strengthen the existing loading-zone test with a production-motion
> test that proves all three facts together:
>
> 1. The movement kernel moves a body into/across a loading zone and produces a
>    nonzero `SweepSample`.
> 2. Collision resolution leaves `kin.vel * dt` unable to describe that movement.
> 3. The actual room-transition detector nevertheless fires.
>
> There is also a second real defect worth fixing even if it isn't this
> reproduction. On the rollback host, successful detection does not immediately
> change rooms. A pending transition waits for `commit_confirmed_lifecycle`, and
> that function silently returns if `session_health()` reports any GGRS checksum
> mismatch. So a live rollback desync can make all doors and loading zones look
> inert even though detection succeeded. I rank that second because current
> rollback tests are green, whereas the `SweepSample` mistake is visibly present in
> production code and directly contradicts an established engine invariant. But the
> commit veto should at least emit a diagnostic rather than silently turning room
> traversal off.
>
> So I would revise D92 substantially: the keyboard-preset theory is disproven by
> your automatic-zone observation, and the "tests are green" evidence was testing
> the wrong seam. The first thing to fix/probe is `detect_room_transition_system`
> ignoring `SweepSample`.

---

## What I verified before acting, and what landed

Every premise checked against the tree rather than taken on the review's word:

* `SweepSample`'s own doc (`body_clusters.rs`): *"Swept readers (hazard touch,
  CC6's relative portal sweep) consume `prev → curr`; bodies without the component
  … fall back to the historical `vel·dt` approximation at the read site."* The
  detector had no `Option<&SweepSample>` in its query at all.
* `movement/collision.rs` has `zero_axis_vel`, called from five arms after the
  body is advanced to time-of-impact. The mechanism is real.
* `crates/ambition_combat/src/hazards.rs:125` is the sibling, written exactly as
  quoted.
* ⭐ **and the fix reaches production bodies**: `SweepSample` is a field of the
  core body bundle (`actor.rs`), so every player and actor carries one. The
  `Option` covers scratch fixtures only.

**Landed:** the detector reads `sweep.map(|s| s.delta()).unwrap_or_else(|| kin.vel
* dt)`, and a new regression —
`a_body_stopped_at_the_boundary_still_crosses_the_zone_it_walked_into` — models
what collision leaves behind (velocity ZERO, sample carrying the travelled
segment) and asserts the transition fires. Its second half is the poison: the
identical body with no sample does NOT fire, which is the shipped bug, so the
fixture cannot quietly stop modelling it. The commit veto now says why it is
holding instead of returning silently.

⚠ **what this does NOT yet do**, and it is GPT's fact (1): drive a body through the
real movement kernel into a zone. The regression models the kernel's output rather
than producing it. That is an honest gap and it is the next thing to close — a
kernel-driven version would also catch a future change to WHERE the sample is
written, which this one cannot see.
