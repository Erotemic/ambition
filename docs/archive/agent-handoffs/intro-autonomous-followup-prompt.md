# Autonomous follow-up prompt — intro vertical slice continuation

Copy the block below into a fresh agent session. It is fully
self-contained — no prior conversation context is required.

---

```
You are continuing the Ambition intro vertical slice. The full
backlog with priorities, risk notes, and bug recipes is in
docs/intro_handoff_to_next_agent.md (read it end-to-end FIRST,
then execute against it).

MISSION

Work through every item in §1 → §2 → §3 → §4 → §5 of the handoff
doc, in that order, until the backlog is empty. Do NOT stop to
ask permission between items — pick the next item, build it,
commit it, move on. Only stop when:

  (a) the backlog is empty,
  (b) you hit a filesystem EMFILE/EIO error on this virtiofs
      VM (per feedback_filesystem_errors memory — stop + ask),
  (c) a design decision genuinely cannot be made from the spec
      + memory entries + git history.

In case (c), pick the most reversible option, document the
choice in the commit body, and proceed. Don't block on
ambiguity — the user prefers forward motion with documented
trade-offs over silent waiting.

PRIORITY ORDER (top-down; each is its own commit or series)

  1. §1.1 Portal animator override (PortalSprite marker +
     Without<PortalSprite> filter on animate_characters).
     This is blocking the visible payoff of the v1 portal —
     fix it FIRST, validate with ./run_game.sh, screenshot
     mentally that the portal goes Off→Opening→On→Closing→Off
     visually when toggling the switch.

  2. §1.2 Gate ring `spin` row during Opening. Trivial after 1.1.

  3. §2.2 Real Prop entity type. Kills the NPC-as-prop interact
     leak + cleans the cart/lab-props/gate authoring. Will need
     a `def update-entity` Python tool (or just write the Prop
     def directly via def register-entity) — §4 tooling gap.

  4. §2.1 OR §2.3 — pick one. Both are big. Tilesets is the
     bigger visual win; Actor unification is the bigger
     architectural win. If unsure, ADR-it: write
     docs/adr/0010-<choice>.md first, then implement.

  5. §2.4 GridVania conversion. Smaller — mostly an authoring
     restructure of intro.ldtk + a runtime smoke test that
     active-area switching still fires on EdgeExit crossings.

  6. §3 small story-content items. Cheap, additive. Knock out
     in batches — e.g. ripple trigger + Erdish first-appearance
     can land in one commit.

  7. §4 tooling: build whatever's needed to unblock the work
     above instead of as a separate phase. Mention each new
     subcommand in the cli.py module doc + bump the help text.

  8. §5 tech debt: opportunistic during the above work; don't
     dedicate commits to it unless you trip on it.

DISCIPLINE (these are load-bearing — violating them creates
real bugs you'll have to fix later)

- New timers ALWAYS use Res<WorldTime>::scaled_dt, never
  Res<Time>::delta_secs() for gameplay/world-anchored stuff.
  See feedback_world_time_pattern memory.
- LoadingZone activation: Door for mid-room cross-map,
  EdgeExit for side-scroll, Walk for scripted environmental.
  Gated zones (portals, locked) own their readiness via a
  state machine; the switch only commands transitions.
  See feedback_loading_zone_activation memory.
- Never hand-edit .ldtk JSON. Use ambition_ldtk_tools
  exclusively (entity add/delete/move/query/check,
  intgrid summarize/erase, area create, door snap, world init).
  If a tool is missing, BUILD IT — small tight subcommand,
  not bloated. See feedback_ldtk_tools_only memory.
- New gameplay/UI code that's story-specific belongs in
  crates/ambition_sandbox/src/intro/ (or a new story
  submodule), wired into sandbox via a Bevy Plugin with
  guarded startup systems. Mirror crate::intro::plugin. The
  sandbox crate itself should stay story-agnostic.
- Never claim "compile-tested" without actually running cargo
  check. ~/.cargo/bin/cargo is the path. See
  feedback_patch_discipline.
- Don't commit binary blobs (PNGs etc.) — sprites/.gitignore
  catches *.png; regenerate via tools/ambition_sprite2d_renderer
  per feedback_no_binary_data.
- Commit completed work without waiting to be asked. After
  each chunk that compiles + validates + tests cleanly, commit
  with a message describing the why, then move on.
- Don't introduce destructive operations (rm -rf, git reset
  --hard, force push) — see CLAUDE Code system instructions
  on executing with care. Reversible actions only.

PER-COMMIT PROTOCOL (run before EVERY commit)

  1. cargo check --manifest-path crates/ambition_sandbox/Cargo.toml
     (path: PATH="$HOME/.cargo/bin:$PATH")
  2. cargo test --manifest-path crates/ambition_sandbox/Cargo.toml --lib
     — must pass 471/471 (or more, if you added tests).
  3. For .ldtk changes:
       PYTHONPATH=tools/ambition_ldtk_tools python3 -m \
         ambition_ldtk_tools validate \
         crates/ambition_sandbox/assets/ambition/worlds/sandbox.ldtk \
         --secondary-world \
         crates/ambition_sandbox/assets/ambition/worlds/intro.ldtk
     Must report "OK".
  4. git add <specific files>; git commit -m '<why-focused message>'
     — never `git add .` (catches sensitive files / untracked junk).
  5. Update memory entries when you discover new
     project/feedback/reference info. See the auto-memory
     section of your system prompt.

PROGRESS LOG

After each completed item from the backlog, append one line to
docs/intro_handoff_to_next_agent.md under a new "## Completed by
autonomous follow-up" section (create it if it doesn't exist).
Format:

  - `<commit-short-sha>` — <handoff-doc §x.y> — <one-line summary>

This lets the user track progress without reading every commit.

REPORTING

When you complete each item (between commits), produce one
sentence: "Done §x.y: <result>. Next: §y.z." No detailed status
dumps — the commit messages + handoff progress log are the
record. Brief continuation messages keep the user informed
without overwhelming.

When the entire backlog is empty, write a final summary commit
+ message: "All v1 follow-up items shipped. <Notable wins>.
Open questions: <if any>."

START NOW. Read docs/intro_handoff_to_next_agent.md, then
launch into §1.1 without preamble.
```

---

Notes for whoever is invoking this:

- The prompt assumes the agent has the same toolchain access as
  the previous sessions (cargo at `~/.cargo/bin/`, the python
  ldtk-tools, the sprite renderer venv at
  `tools/ambition_sprite2d_renderer/.venv/`).
- Memory entries referenced (`feedback_world_time_pattern`,
  `feedback_loading_zone_activation`, `feedback_ldtk_tools_only`,
  `feedback_patch_discipline`, `feedback_no_binary_data`,
  `feedback_filesystem_errors`) are in
  `/home/agent/.claude/projects/-home-joncrall-code-ambition/memory/`
  and load automatically as part of the agent's system prompt.
- If the agent goes off-rails (skips the per-commit protocol,
  starts hand-editing JSON, picks a different priority order),
  interrupt with a one-line correction quoting the relevant
  rule from the DISCIPLINE block — the prompt is structured so
  every rule is grep-able by name.
- The runtime test command `./run_game.sh` at the repo root
  builds + launches the game; use it to manually verify the
  portal animation and other visual fixes when the test suite
  can't.
