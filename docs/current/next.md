# Current next moves

**Review date:** 2026-05-30. Reviewed against source archive `ambition-source-2026-05-30T104014-5-e721ea65c578`.

1. Keep ADRs and concept pages modern enough that agents can trust them.
2. Continue shifting runtime integration toward data-driven Bevy ECS instead of parallel code-owned world state.
3. Replace obsolete migration docs with current systems/concepts or archive them.
4. Fix known wall-cling / collision / transition issues with trace-backed tests.
5. Expand platform smoke coverage for desktop, web, Android/mobile touch, controller, and Steam Deck.
6. Improve tool documentation so agents know which generator/validator to use.
7. Promote durable lessons from `dev/` into concepts, recipes, or ADRs when they stop being one-off postmortems.
8. Finish the remaining actor/brain cleanup now that the main seam is live: keep player movement on `ActorControl`, keep player melee/projectile and enemy/boss action consumers on `ActorActionMessage`, centralize enemy brain construction policy, and keep the remaining player-specific pogo start path from duplicating target-surface policy. The `ae::Player` aggregate has been deleted (2026-05-28) — the player entity carries 18 cluster components.
9. Add a canonical combat hit pipeline. Current damage works, but `DamageEvent`, hostile `Hitbox` entities, `PlayerDamageEvent`, boss damage outcomes, VFX/SFX, stagger, and knockback are still separate payload shapes. The next durable combat cleanup is a `HitSpec` -> `HitInstance` -> `HitResult` path.
10. Improve Silksong-style feel in focused slices: general-purpose input buffers for attack/pogo/projectile/tool/blink, apex hang / jump sustain, sprint-jump / long-jump momentum, and later grapple/harpoon traversal.
