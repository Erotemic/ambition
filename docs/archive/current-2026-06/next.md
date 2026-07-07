# Current next moves

**Review date:** 2026-06-13.
**⚠ STALE (flagged 2026-07-04):** the live work queue is
[`../reviews/fable-review-2026-07-04.md`](../reviews/fable-review-2026-07-04.md)
(R1–R6); the crate-decomposition target is
[`../planning/engine/architecture.md`](../planning/engine/architecture.md). The
"hard-won rule" below (measure OUTWARD deps) remains binding wisdom.

0. **Monolith breakup (active direction).** The Stage-20 bisection landed
   (foundations ← machinery `ambition_actors` ← content `ambition_content` ←
   app `ambition_app`); the work now is shrinking `ambition_actors` (~90k LOC).

   **Hard-won rule:** measure a module's *outward* dependency count, not its line
   size or content-guard status. "Names no content" (passes the guard) is NOT
   the same as "extractable" — a module can be content-free yet have dozens of
   inbound mechanic deps. Two move shapes: extract a reusable crate DOWN
   (foundations) vs. move named content UP (into `ambition_content` / `app`).

   Done this push: `menu` host → app; `ambition_characters` extracted; portal +
   portal-presentation crates; the **enemy roster** is now content-owned data
   installed into a lib holder (no `EnemyArchetype` enum — see
   [`state.md`](state.md), the content-installed-roster pattern); the **boss
   roster** is now fully content-owned the same way — both `boss_profiles.ron`
   AND `boss_encounters/*.ron` are embedded + installed by `install_boss_roster`
   (from `init_sandbox_resources`, the early-resolution choke point), the old
   runtime `std::fs` encounter-spec read is gone, and `roster.rs` collapsed to
   the one in-lib generic `gradient_sentinel` base.

   Remaining backlog, by readiness:
   - **Unified actor+brain crate (deferred carve).** Fold the boss runtime fully
     into `ambition_characters`, leaving only named boss data in content.
   - **`mechanics` crate extraction — verified HARD.** Needs ~15 dependency
     inversions (or pre-extracting half the lib first). Do NOT attempt as a
     quick crate; pre-invert its inbound deps incrementally instead.
   - **`dev` state/systems split (partial).** ~1.3k of presentation-only dev
     overlays already moved up; the dev STATE + systems split is deferred.
   - **Render-boundary polish / `world`+LDtk (~9k).** Presentation has moved to `ambition_render` / `ambition_portal_presentation`; the remaining work is policing residual sim→presentation leaks and measuring `world`+LDtk outward deps before promising any extraction.
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
