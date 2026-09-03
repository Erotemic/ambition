# Overnight agent 3 — completed receipt

**State:** CLOSED 2026-08-27. This is no longer a task list.

All six requested items landed and are covered by current product/tooling docs:

- moveset balance inspector/export/take tooling;
- Pirate Admiral gun-sword side-B;
- held/banked Power Ball charge;
- Projectile Polygon returning/boomerang projectile;
- reusable Smash teleport recovery used by multiple fighters;
- Projectile Polygon bomb down-B built on ordinary ground-item machinery.

> **RE-VERIFIED 2026-09-03: all six landed, and one of them is not findable by
> the name this list gives it.** The receipt says *"held/banked Power Ball
> charge"*, and `power_ball` matches **nothing** in the tree — "Power Ball" is
> the move's flavour name, never an identifier. What landed is a general banked
> charge: *"The banked charge. It OUTLIVES the move that made it"*
> (`combat/src/rollback_registration.rs:229`), rollback-registered and guarded by
> `a_held_charge_has_no_live_strike_and_releases_into_one`. ⇒ **A closed receipt
> written in the language of the REQUEST rather than the code is unverifiable by
> anyone who was not there** — which is most readers, three weeks on.
>
> The other five resolve directly: `projectile_polygon_moveset.rs` carries both
> the boomerang and the bomb, `pirate_admiral` spans 40 files, the teleport
> recovery is shared vocabulary, and the inspector tooling has its own page.

Follow-up defects discovered by the tools (including the cast-wide back-air issue
and Robot v3's blocked specials) were fixed separately. Do not infer current
moveset status from this dated brief.

Current Smash mechanics live in
[`demos/smash-parity-inventory.md`](demos/smash-parity-inventory.md); current tool
behavior lives with the moveset-inspector/tool documentation and source.

The detailed implementation/schema chronology is intentionally left to git
history. Keep this receipt until Phase 2 removes the queue's historical link,
then delete it.
