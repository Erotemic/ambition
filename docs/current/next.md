# Current next moves

**Review date:** 2026-06-13.

0. **Monolith breakup (active direction).** The Stage-20 bisection landed
   (foundations ← machinery `ambition_sandbox` ← content `ambition_content` ←
   app `ambition_app`); the work now is shrinking `ambition_sandbox` (~90k LOC).

   **Hard-won rule:** measure a module's *outward* dependency count, not its line
   size or content-guard status. "Names no content" (passes the guard) is NOT
   the same as "extractable" — a module can be content-free yet have dozens of
   inbound mechanic deps. Two move shapes: extract a reusable crate DOWN
   (foundations) vs. move named content UP (into `ambition_content` / `app`).

   Done this push: `menu` host → app; `ambition_actor` extracted; portal +
   portal-presentation crates; the **enemy roster** is now content-owned data
   installed into a lib holder (no `EnemyArchetype` enum — see
   [`state.md`](state.md), the content-installed-roster pattern).

   Remaining backlog, by readiness:
   - **Boss roster (in progress).** Applying the enemy-roster pattern to the
     boss named data. DONE: `boss_profiles.ron` (per-boss behavior/attacks/
     rewards) is content-owned + installed into the lib's `BossProfileRegistry`
     holder via `install_boss_roster()`; production lib embeds no boss behavior
     data. KEY GOTCHA learned: bosses resolve EARLIER than enemies (at registry
     population + content validation, not just spawn), so the install choke
     point is `init_sandbox_resources` (every sim entry path flows through it),
     not the content plugin's build. REMAINING: (a) move `boss_encounters/*.ron`
     (the encounter specs — note this is a RUNTIME `std::fs` read via
     `load_boss_specs_from_disk(CARGO_MANIFEST_DIR)`, replay-sensitive, unlike
     the compile-time `boss_profiles.ron`); (b) de-dup the `roster.rs`
     `BossSpecRoster` named spec constructors — they hardcode the same values as
     `boss_encounters/*.ron` (test-pins, like the deleted `EnemyArchetype`).
     The generic boss-runtime + `BossEncounterSpec`/`BossBehaviorProfile` schema
     stay in the lib (or fold into `ambition_actor`). Bosses are already actors
     (ADR 0016).
   - **Unified actor+brain crate (deferred carve).** Fold the boss runtime fully
     into `ambition_actor`, leaving only named boss data in content.
   - **`mechanics` crate extraction — verified HARD.** Needs ~15 dependency
     inversions (or pre-extracting half the lib first). Do NOT attempt as a
     quick crate; pre-invert its inbound deps incrementally instead.
   - **`dev` state/systems split (partial).** ~1.3k of presentation-only dev
     overlays already moved up; the dev STATE + systems split is deferred.
   - **`presentation` (~10k) / `world`+LDtk (~9k).** Large; measure outward deps
     before promising any extraction.
1. Keep ADRs and concept pages modern enough that agents can trust them.
2. Continue shifting runtime integration toward data-driven Bevy ECS instead of parallel code-owned world state.
3. Replace obsolete migration docs with current systems/concepts or archive them.
4. Fix known wall-cling / collision / transition issues with trace-backed tests.
5. Expand platform smoke coverage for desktop, web, Android/mobile touch, controller, and Steam Deck.
6. Improve tool documentation so agents know which generator/validator to use.
7. Promote durable lessons from `dev/` into concepts, recipes, or ADRs when they stop being one-off postmortems.
8. Finish the remaining actor/brain cleanup now that the main seam is live: keep player movement on `ActorControl`, keep player melee/projectile and enemy/boss action consumers on `ActorActionMessage`, centralize enemy brain construction policy, and keep the remaining player-specific pogo start path from duplicating target-surface policy. The `ae::Player` aggregate has been deleted (2026-05-28) — the player entity carries 18 cluster components.
9. Extend the canonical combat hit object. _Largely landed (verified 2026-06-02):_ the old split damage events are deleted; player slash/pogo/projectile, hostile `Hitbox` entities, hazards, enemy, and boss hits all flow through one `HitEvent { volume, damage, source: HitSource, attacker, target: HitTarget, mode: HitMode, knockback }` consumed by `apply_feature_hit_events`. The remaining work is the *advanced* `HitResult` fields — stagger/poise, elements/status, hitstop, and explicit rejection reasons — which should land alongside the mechanics that need them (don't add the fields speculatively).
10. Improve Silksong-style feel in focused slices: general-purpose input buffers for attack/pogo/projectile/tool/blink, apex hang / jump sustain, sprint-jump / long-jump momentum, and later grapple/harpoon traversal.
