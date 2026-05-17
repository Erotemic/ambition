# Autonomous follow-up prompt v3 — no-stop discipline + bullet-time elevation

The v1 prompt asked the agent to work the backlog top-down; the v2
agent stopped after writing two ADRs (0015, 0016) without implementing
either. This v3 closes that loophole and adds bullet-time as a named
load-bearing item that must be implemented per the existing ADRs.

Copy the fenced block below into a fresh agent session.

---

```
You are continuing the Ambition intro vertical slice. There is a
backlog (docs/intro_handoff_to_next_agent.md) and a design spec
for bullet-time (docs/adr/0010-time-domains-and-regime-policies.md
+ docs/adr/0011-per-entity-proper-time-and-sr-ladder.md). Both
ADRs are status=Accepted.

YOUR MISSION

Drive the backlog to zero. The previous agent shipped four commits
(portal animator override, dedicated Prop entity, two ADRs, the
tileset add tool) then stopped because the next items were "big."
That stop is the failure mode this prompt is designed to prevent.

THE BACKLOG, in priority order. Read the handoff doc for full
context on each item.

  P0  BULLET-TIME ELEVATION (§NEW — see "BULLET-TIME WORK" below).
      The current WorldTime::scaled_dt is a SP-only global SimClock
      scalar. ADR 0010 + 0011 specify the multi-domain vocabulary
      that's multiplayer-coherent. IMPLEMENT IT, don't re-document
      it. The ADRs are the spec.

  P1  §1 portal bug 1.3 verification — verify the dedicated Prop
      entity (commit 195b5ce) actually killed the NPC-as-prop
      interact-prompt leak on lab props + gate sprites. Run the
      game and confirm no Interact prompt fires on those entities.
      If it still does, fix it.

  P2  §2.1 LDtk tileset rendering — ADR 0015 is proposed; the
      `tileset add` tool exists (commit 6ca1a81). Implement the
      Rust integration end-to-end:
        - Register intro_lab_tileset + town_tileset in intro.ldtk
          via the new tool.
        - Add tile layer instances to each intro level.
        - Wire bevy_ecs_ldtk's native tile renderer for those
          layers (flip LevelBackground::Nonexistent /
          IntGridRendering::Colorful per-layer).
        - Reconcile the coordinate-frame disagreement between
          Ambition's centered frame and bevy_ecs_ldtk's raw-pixel
          frame (the seam is in ldtk_world/bevy_runtime/asset.rs;
          comment block already calls it out).
      "Proposed ADR" is not done; rendering tiles in-game is done.

  P3  §2.3 Actor unification — ADR 0016 is proposed. Implement it:
        - Single Actor LDtk entity with `name`, `dialogue_id`
          (optional), `aggression` enum, optional `brain`,
          optional `path_id`.
        - Migrate existing NpcSpawn + EnemySpawn rows in
          sandbox.ldtk + intro.ldtk to Actor (`def update-entity`
          tool needed — build it; §4 gap).
        - Collapse the runtime ActorRuntime::Peaceful/Hostile
          enum dispatch into one path that branches on
          aggression.
        - Keep NpcSpawn/EnemySpawn as deprecated aliases for one
          release so existing saves load.

  P4  §2.4 GridVania world layout — flip intro.ldtk to GridVania,
      repack room positions adjacently, verify runtime active-area
      switching still fires on EdgeExit crossings.

  P5  §3 small story-content items — knock these out in batches.
      Cheap, additive, satisfying:
        - Faster creator final fragments (`creator_final_fast`,
          `creator_final_impossible` dialogue ids + skill-route
          variant selection in intro_raid cutscene).
        - Ripple interaction trigger in gate_stack_lower
          (Switch-like that fires first_ripple cutscene on first
          overlap).
        - Erdish first-appearance NPC in drain alley / gate stack.
        - Galwah duel scripted set-piece (future town zone — file
          stub if no town zone yet).
        - Real Manifest Office / Pirate / Ninja / Nazi-fortress
          .ldtk zone stubs (each a new file via `world init`,
          added to SECONDARY_WORLD_FILES).
        - Return-to-lab unlock path via lab_ruins.ldtk.

  P6  §4 remaining tooling: `def update-entity` (built during P3
      if not earlier), `intgrid paint` (TODO in cli.py).

  P7  §5 tech debt — opportunistic during the above (don't
      dedicate commits unless you trip on it):
        - Move intro/tests.rs + intro/banter::tests inline.
        - Split apply_feature_damage_events (>250 lines).
        - Replace overloaded CharacterAnim variants (Walk/Run as
          portal "stable"/"closing") with per-prop anim enums
          when the Prop entity refactor lets you.

================================================================
BULLET-TIME WORK (P0) — multi-domain time per ADR 0010 + 0011
================================================================

CURRENT STATE (a single-player hack):

  crate::WorldTime { raw_dt, scaled_dt }
    refresh_world_time(): scaled_dt = raw_dt * SandboxSimState::time_scale
    every gameplay/animation system reads Res<WorldTime>::scaled_dt

This is fine for one player but violates ADR 0010: time control is
expressed as a single global multiplier, not as a domain-keyed
permission table. Multi-player + RL-deterministic regimes can't
coexist on this surface.

WHAT YOU WILL BUILD (incremental, each step ships independently):

Step 1 — clock domains exist as data, not concept.
  - New `ClockDomain` enum: SimClock | PlayerClock(PlayerSlot) | WallClock.
  - WorldTime grows accessors: sim_dt(), player_dt(slot),
    wall_dt(). raw_dt becomes wall_dt; scaled_dt becomes sim_dt
    by default. Both old names stay as deprecated aliases for
    one release so existing callers keep compiling — DO NOT
    break Rust API in one commit.
  - SP regime sets all PlayerClocks equal to SimClock so today's
    callers are observationally identical.
  - Commit: "feat(time): clock-domain vocabulary; sim/player/wall
    dt accessors on WorldTime"

Step 2 — requests are messages, not direct mutations.
  - New message `ClockScaleRequest { domain, scale, requester,
    reason }`. Replace direct mutation of
    SandboxSimState::time_scale with writes of this message.
  - New resource `RegimePolicy` storing a (requester, domain)
    -> Permission map. Default SP policy grants all requests.
  - New system `apply_clock_scale_requests` consumes the messages,
    consults RegimePolicy, applies Grant (write the scale into
    the domain), Deny (drop), Rebind(other_domain) (rewrite the
    domain), Broadcast (apply to every domain in scope).
  - Commit: "feat(time): ClockScaleRequest message + RegimePolicy
    permission table; SP policy grants all"

Step 3 — per-entity proper time (ADR 0011 §"Per-entity proper time").
  - New `ProperTimeScale(f32)` component (defaults to 1.0).
  - WorldTime exposes `entity_dt(entity)` that multiplies SimClock
    dt by the entity's proper-time scale.
  - Animator + ai-tick sites that should be per-entity-time-aware
    migrate from sim_dt to entity_dt.
  - SP gameplay still works: nothing sets ProperTimeScale, so
    every entity gets 1.0 * sim_dt = sim_dt.
  - Commit: "feat(time): ProperTimeScale component + entity_dt
    accessor; SP unchanged"

Step 4 — bullet-time is implemented as the SP policy granting a
PlayerClock(0) scale request, NOT as a sim-time override.
  - When the player initiates bullet time (today: dev_tools
    slowmo + future ability), fire ClockScaleRequest {
      domain: SimClock, scale: 0.125, requester:
      Player(slot=0), reason: "bullet_blink"
    }. SP policy grants. Sim slows; player's PlayerClock(0)
    rate stays at 1.0 (per ADR 0011 Operation 2 interpretation
    — the player's cognitive rate is unchanged relative to
    their own clock).
  - Document in a code comment that the SP-equivalent of "boost
    player proper time" (Operation 2) is observationally the
    same as "slow sim" (Operation 1). The engine implements
    Operation 1; the per-player-clock surface in step 3 is the
    seam where future MP/RL regimes diverge.
  - Commit: "feat(time): bullet-time wired via ClockScaleRequest
    + Player requester; behavior unchanged"

Step 5 — write a memory entry consolidating the discipline.
  Path: /home/agent/.claude/projects/-home-joncrall-code-ambition/memory/
        feedback_time_domains.md
  Body: when you need a dt, decide deliberately between
        WorldTime::wall_dt() (real time), sim_dt() (gameplay
        clock; what bullet-time scales), player_dt(slot)
        (cognitive clock; multiplayer-coherent), entity_dt(e)
        (per-entity proper time; SR-ready). Default sim_dt
        for world-anchored animation + gameplay state machines.
        Update MEMORY.md index.

ACCEPTANCE for P0: every existing reader of WorldTime::scaled_dt
in the sandbox crate either (a) migrated to sim_dt and behavior
unchanged, OR (b) stayed on a deprecated alias that's documented
to be removed in the next release. Bullet-time still works in SP
exactly as today, but the path through the code is now the
multi-domain vocabulary, not a single global scalar. ADR 0010 §
Vocabulary should match the implementation 1:1.

================================================================
NO-STOP DISCIPLINE — explicit anti-patterns
================================================================

You are forbidden from stopping for any of these reasons:

  X "I shipped four commits, the user will be happy" — KEEP GOING.
    Backlog has more items. Name the next one out loud and start.

  X "I wrote an ADR; that documents the work" — NO. The ADR is
    the SPEC. Implementing it is the work. Don't confuse design
    docs with done work. The previous agent did this twice
    (ADR 0015 + 0016 written, neither implemented). Don't repeat
    their pattern.

  X "This next item is big / will take a while" — irrelevant.
    Big items get split into multiple commits, not skipped.

  X "Let me let the user pick between A and B" — NO. Pick the
    most reversible option, document the trade-off in the commit
    body, ship. The user has named the priority order — your
    job is to execute it, not re-derive it.

  X "Tests pass and I made meaningful progress; time to wrap up"
    — NO. Wrapping up is for when the backlog is empty.

  X "I might break something if I keep going" — make a commit
    NOW (preserving current green state), THEN keep going. Each
    commit is a save point.

  X "I'll write a 'progress so far' summary" — NO summary writing
    until the backlog is empty. The per-commit progress log is
    the running record (see below).

You ARE allowed to stop for these:

  ✓ Backlog is empty (every item P0..P7 has a committed
    implementation OR a written deferral with a successor
    artifact, e.g. an ADR + an issue + a follow-up prompt).

  ✓ Filesystem EMFILE/EIO from this virtiofs VM (per
    feedback_filesystem_errors memory). Stop + ask.

  ✓ A design ambiguity the spec + ADRs + memory + git history
    truly can't resolve. In that case: write the question down
    in the handoff doc's open-questions section, pick the most
    reversible answer, document the choice in the commit, and
    proceed. Don't block on the question — log it and move.

================================================================
PROGRESS LOG (now mandatory — previous agent skipped this)
================================================================

After EVERY commit (no exceptions), append one line to
docs/intro_handoff_to_next_agent.md under a section called
"## Completed by autonomous follow-up" (create if missing).
Format:

  - `<short-sha>` — <P# §x.y> — <one-line summary>

Example:

  - `8d963c7` — P1 §1.1 — PortalSprite marker prevents animate_characters
    from re-pinning portal anim each frame
  - `195b5ce` — P1 §2.2 — dedicated Prop LDtk entity replaces NpcSpawn-as-prop
    hack; cart/lab-props/gate-sprites migrated

This makes your work auditable WITHOUT the user reading 30 commit
messages. It's also how you'll know what's left.

When the backlog hits zero, append a final line:

  - ALL DONE @ <short-sha> — <one-paragraph summary of notable
    wins + any open questions filed to the handoff doc>

================================================================
PER-COMMIT PROTOCOL (run before EVERY commit, no skipping)
================================================================

  1. PATH="$HOME/.cargo/bin:$PATH" cargo check --manifest-path \
       crates/ambition_sandbox/Cargo.toml

  2. PATH="$HOME/.cargo/bin:$PATH" cargo test --manifest-path \
       crates/ambition_sandbox/Cargo.toml --lib
     Must pass 471+/471+ (more if you added tests). If a test
     fails, your work isn't done — fix it before committing.

  3. For .ldtk changes:
       PYTHONPATH=tools/ambition_ldtk_tools python3 -m \
         ambition_ldtk_tools validate \
         crates/ambition_sandbox/assets/ambition/worlds/sandbox.ldtk \
         --secondary-world \
         crates/ambition_sandbox/assets/ambition/worlds/intro.ldtk
     Must report "OK".

  4. git add <specific files>      ← NEVER `git add .`
  5. git commit -m '<why-focused>'  ← multi-line via heredoc;
                                      include Co-Authored-By trailer

  6. Append progress log line per format above.

  7. Loop: pick the next backlog item, repeat.

================================================================
RUNTIME DISCIPLINES (load-bearing rules from memory)
================================================================

These are grep-able by name so corrections can be terse:

- feedback-world-time-pattern → New timers use sim_dt() (after
  Step 1 of P0; until then, WorldTime::scaled_dt).
- feedback-loading-zone-activation → Door / EdgeExit / Walk
  picker rule. Gated zones own readiness; switches command.
- feedback-ldtk-tools-only → never hand-edit .ldtk JSON; extend
  the tools if a gap.
- feedback-patch-discipline → cargo/rustc at ~/.cargo/bin/;
  never claim compile-tested without running cargo check.
- feedback-no-binary-data → don't commit PNGs (gitignored);
  regenerate via the sprite renderer.
- feedback-filesystem-errors → EMFILE/EIO from virtiofs = stop +
  ask, don't /tmp around it.
- feedback-always-commit → after work compiles + tests cleanly,
  commit without being asked.
- feedback-no-sudo-apt → never `sudo apt-get` without confirming.

================================================================
START NOW
================================================================

  1. Read docs/intro_handoff_to_next_agent.md end-to-end.
  2. Read docs/adr/0010-time-domains-and-regime-policies.md and
     docs/adr/0011-per-entity-proper-time-and-sr-ladder.md.
  3. Read docs/adr/0015-ldtk-tileset-rendering.md and
     docs/adr/0016-actor-unification.md.
  4. Add the "## Completed by autonomous follow-up" section to
     the handoff doc with a header line: "(Started <date>; agent
     committed to working until backlog is zero per
     docs/intro_autonomous_followup_prompt_v3.md.)"
  5. Begin P0 Step 1. Don't preamble; the next message from you
     after these reads should be a tool call.
```

---

## Notes for the invoker

- The previous agent's pattern was to write design docs in lieu
  of implementation. The DISCIPLINE block here names that
  specifically with "X 'I wrote an ADR; that documents the work'
  — NO" so the agent has an explicit recognition pattern.
- If you see the agent producing summaries, lists of options for
  you to choose, or "I've made great progress, want me to
  continue?" messages, interrupt with: "Read the no-stop
  discipline section. Pick the next item and ship it." The
  prompt is structured so this correction is a one-liner.
- Memory entries this prompt references are in
  `/home/agent/.claude/projects/-home-joncrall-code-ambition/memory/`
  and load automatically. The new `feedback_time_domains.md` to
  be written in P0 Step 5 will land there.
- After this agent finishes, the handoff doc's progress log will
  show every commit + each item's status. The "ALL DONE" line
  marks completion.
