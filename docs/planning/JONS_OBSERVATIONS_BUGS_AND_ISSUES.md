# Agents should only edit this file to mark something as potentially done. Jon
# will remove it if it is actually done, or mark it not actually solved if an
# attempt doesn't work.


* ◐ **When SANIC is hit** (all four parts addressed 2026-08-07 — your sentence
  had four requirements and they turned out to have four different answers).
  Your words: *"there it seems like he is given no iframes. He should also have
  some hitstun and be knocked back a bit, and then have a few second of recovery
  iframes. The rings don't splash out nearly large enough. He needs an
  opportunity to recollect some of them after his hitstun wears off and before
  they disappear."*
  * **i-frames — they were never missing, they were 0.75s.** That is the engine's
    `knockback_invulnerability_time`, and Sanic authored no feel at all so he was
    on the default. It is a fair "longest window in the game" for Ambition, where
    a hit is a hit; here the hit throws your purse across half a screen, so it is
    over before the rings land and the badnik you bounced off is still touching
    you. Now **2.0s** — the classic — set in Sanic's own ring-loss handler rather
    than in the engine, because Mary-O shares that default and her feel is pinned.
  * **hitstun — genuinely missing, and fixed.** The resolver's armor branch arms a
    beat (added after your Mary-O report, *"there needs to be a bit of hitstun
    when maryo gets hit"*); the wallet-shield branch six lines below it did not.
    The rings burst out of a body that never flinched.
  * **knocked back — already there.** A contact hit carries `HitMode::Knockback`
    and the wallet branch keeps the physical reaction deliberately, so this half
    was working.
  * **the splash — already landed**, with a test that drives the real integrator
    and requires the spray to reach eight tiles.
  * **and the recollection window, which is the part the i-frame number actually
    decides.** Rings become collectible at 0.6s and vanish at 4.2s. At 0.75s of
    i-frames you had **0.15s** to grab one without being re-hit; at 2.0s you have
    **1.4s**. Nine times the safe scramble, with the scatter numbers untouched.
  ⚠ **2.0s is the one number here that is taste rather than measurement** — say
  the word and it moves; it is one constant, `RING_LOSS_INVULN_S`.

* ◐ **The patent clerk: HE IS IN THE GAME** (2026-08-06). You said "patent clerk
  is new: note we want to use the svg clerk not the older one", so:
  `special_patent_clerk` has a catalog row spliced from the target's own
  `ACTOR_METADATA` — your coy-Einstein authoring note, the gameplay note, and
  all twelve authored lines — plus a Hall pedestal from
  `generate_hall_of_characters`. The waiver that excused him in
  `rendered_identities_are_registered.rs` is deleted, so the check now enforces
  him. ⚠ **the older clerk is untouched**: `npc_manifest_clerk` is a different
  character (an intro bit part still borrowing the architect sheet), and giving
  it this art would put the same distinctive face on two pedestals — say the
  word if you did mean to replace it.
  ▢ **his art cannot currently be re-rendered.** `./regen_sprites.sh --target
  patent_clerk` dies in your own live rig work: `SVG view 'Patent Clerk - Side
  Left' does not have one-to-one drawable ownership: multiply_assigned={'path1115':
  ['torso', 'neck']}`. The sheets in the tree came from your 13:58 render and were
  installed from `generated/`, so the game is fine today and a fresh clone is not.
  ⭐ that failure is now VISIBLE: `patent_clerk` was in the render batch and
  missing from the postcondition list, so a run that failed to draw him still
  reported every expected output present.
  ✔ **FIXED 2026-08-08** (submodule `6203ae9`). It was a binding-spec defect, not
  an art fact: `path1115` is a single path labelled "Neck" drawing only the neck.
  It gained a second owner because `part-neck` was authored **inside**
  `part-torso`, and `_descendant_ids` collects with `elem.iter()`, which recurses
  into nested part groups. Across your seven rigged SVGs — 170 part groups — this
  was the only nested one, and Carl Stargan has the identical neck/torso split as
  siblings.
  ⚠ **so `9521978` "new higher quality rigs" is what introduced it**: that commit
  added the one-to-one check and migrated `carl-stargan.svg` to satisfy it,
  without migrating `patent-clerk.svg`. This was the unfinished half of its own
  migration rather than an older latent bug. A second failure was queued behind
  it — `data-rig-z` still carried the pre-migration `5..20` numbering, so fixing
  only the nesting would have failed the next check.
  ⭐ **and the duplicate was not cosmetic.** Rasterising the torso with and
  without `path1115` shows the shipped sprite carried a stray skin-coloured neck
  slab at torso depth, **covering the top of the far arm's sleeve and cuff** —
  visible in the portraits as a sleeve ending in a blob. The regenerated art
  differs from the 13:58 render in exactly that.
  Carl re-renders byte-identical, the regen is idempotent, and all three `.ron`
  outputs are unchanged, so nothing on the Rust side sees a contract change.
  ⭐ **the five others in this gap are down to one, and it is a ruling of yours**:
  busy beaver, charley beagle, niels boar and vera ruin were cast on 2026-08-05,
  and `npc_pirate_heavy` is a family rather than a character by your own answer.

* ✔ **YOU HAVE BEEN COLLECTING MAGENTA BOXES, and never reported it** (found and
  fixed 2026-08-08, `014678998`). Not from a report of yours — a room-transition
  fixture measured it in passing, which is why it is worth telling you: **every
  Ambition drop since drops existed has been drawn as the floor's deliberately-ugly
  diagnostic stand-in instead of as a coin, a heart or an ability pickup.**
  ⭐ **cause**: `rebuild_dynamic_feature_views` is the only pass that can draw loot
  the simulation minted, and it selects by construction PROVENANCE — correctly, since
  *"this pickup was not in the room spec"* is exactly when the room-load pass could
  not have seen it. But its query REQUIRES a `SpawnOrigin`, and all three drop
  functions stamped none. Every drop fell out of the query, nothing claimed it, and
  `draw_unclaimed_feature_views` did its job.
  **Measured through the shipped app**: `proving_grounds` at 0 stand-ins clean and
  **8 after seven defeats**; through the real render stack the log carried
  ``no render family claimed `coin:EnemySpawn-5910` (Pickup)`` before and **nothing
  after**.
  ⚠ **`dd73a3087` did not fix this** — that one stopped a coin FOLLOWING you between
  rooms. This is the room the coin fell in.
  ⭐ **and a second bug was queued behind it**: once drops draw, they would have
  drawn a heart wearing a coin's sprite. The sprite now resolves from the pickup's
  live kind.
  ⚠ **the guard asserts against the ids the death path MINTED, not a count** — the
  stand-in population is re-spawned per room, so "it grew" is answerable by a room
  that merely has more in it. Probed red: 8 of 12 minted views undrawn while 4 split
  offspring in the same set were drawn, so it discriminates rather than passing on
  everything.

* ✔ **THE GRADIENT SENTINEL SUMMONS NOW — it never had** (found and fixed
  2026-08-08, `da1563ec1`). **A Puppy Slug actually appears**, measured in the real
  app in `sandbox:basement_boss` (LDtk `BossSpawn-0158`, brain
  `PhaseScript:clockwork_warden`), emitting the Minima Trap's own request:

  ```text
  before  counter=None      descendants: []                       'Puppy Slug': []
  after   counter=Some(0)   descendants: ["…BossSpawn-0158/0"]    'Puppy Slug': ["Puppy Slug"]
  ```

  ⭐ **the fix is a type property, not a patch.** `SimId` is now
  `#[require(SimIdCounter)]`. `SimId::spawned` is the only way to name a
  dynamically-spawned entity and it needs a counter **on the spawner** — so
  *"identified"* and *"able to be descended from"* are the same condition, and
  making it the type's business removes what **six** mint sites had to remember.
  Two of the six remembered; the executor that builds every authored boss did not.
  ⚠ **rollback cost, checked**: no registration moved, so the schema version and
  both baselines are untouched. Every `SimId` carrier snapshots 8 more bytes, and
  a required component is supplied only when ABSENT — so a restore putting back
  `SimIdCounter(7)` keeps 7 rather than re-minting.
  ⭐ **the guard is built THROUGH the real executor** — it authors a `boss_spawns`
  row, runs the construction plan, finds the boss by querying the world for its
  `SimId`, and only then summons. Nothing about the summoner is hand-built, which
  is the whole reason the old test was green over a dead feature.

* ~~⛔ **THE GRADIENT SENTINEL'S SUMMON DOES NOTHING** (found 2026-08-08, NOT fixed
  — queued as D20).~~ Its Minima Trap is authored to summon a *"Puppy Slug"*
  minion (`bosses/specials/gradient_sentinel.rs:604`, and a second site at `:860`);
  the boss ships with a sheet, a cutscene intro and a derived encounter
  (`clockwork_warden`). **The minion never appears.**
  ⭐ **why**: `apply_summon_effects` requires the summoner to carry BOTH a `SimId`
  and a `SimIdCounter`, and `warn!`s + skips otherwise. A body built by the
  construction executor gets a `SimId` and **no counter** —
  `ensure_sim_id` is filtered `Without<SimId>`, so it never backfills one, and
  `construction/mod.rs` inserts a counter **nowhere** (its 16 mentions are 15 in
  `tests.rs` plus a doc comment describing a counter nothing supplies).
  ⚠ **the unit tests cannot catch it**: they build the summoner by hand and so
  supply exactly the counter the real path omits.
  ⭐ **you can confirm it from play without any code**: the bail emits a `warn!`,
  so if you have fought either boss recently and no Puppy Slug appeared, it is in
  your log. If one DID appear, tell me — that would refute this and I want to know.
  ⚠ found while fixing the magenta-coin bug, and verified by a second route
  before being written down; the first search said *"only constructed in tests"*
  and that was `head -6` cutting off the two production sites.

* ⛔ **`capture_scene` HAS BEEN READING AND WRITING YOUR REAL SAVE, and playing
  music at whoever ran it** (found and fixed 2026-08-08, `fb8755333`). Not from a
  report — found while checking whether the tool composes the game the way a
  player does. `build_visible_app` redirects audio off the speakers and
  persistence out of `~/.local/share/ambition/` for every windowless host; ROUTE
  mode is built by it and **room mode had neither**.
  ⭐ **the evidence is in the captures themselves**: the money readout reads
  **`$115` before the fix and `$0` after** — the before-run was reading your
  actual save. Now redirected unconditionally rather than by render mode, because
  `--show-window` is still a screenshot tool.

* ⛔⛔ **AND FOR TWO DAYS THE PHONE PROXY PHOTOGRAPHED A VOID WITH A HUD ON IT.**
  Same commit. `capture_scene` migrated to composing the shell on 2026-08-06 and
  **never installed `install_ambition_shell_visuals`** — the only registrar of the
  systems that spawn parallax, static room visuals, signage and the LDtk spine
  **on activation**. The Startup path that used to do it was deleted with the
  `direct_entry` gate.
  **Before**: a black frame with the player, an NPC, the HUD, the touch bezel and
  the menu buttons — and **no floor, walls, doors, signage or backdrop**.
  **After**: the room. Subject pose identical in both (`950.0000, 904.0000` at 12
  warmup ticks), so the fix added the world without moving anything.
  ⚠ **why nobody caught it**: everything hanging off the SESSION drew, and only
  what hangs off the ROOM was missing — so the image read as *"a dark corner"*
  rather than as a broken tool. The 08-06 commit message was true word for word
  (*"draws the robot, the HUD, the touch bezel and an NPC"*).
  ⭐ **this is the Bevy param-panic class arriving SILENTLY.** The same fork
  panicked loudly twice before (`VisualQualityPlugin` 07-31,
  `sync_portal_quality_budget` 08-04) and was caught in minutes each time. Here
  the missing piece was a SPAWN system rather than a `Res` reader, so nothing
  panicked. **A composition that half-runs beats one that refuses to — right up
  until you believe the photograph.**

* ✔ **`capture_scene --show-window` NEVER WORKED, and three other things you may
  have assumed about that tool** (2026-08-08, `8bbbfc273` — the two-builder fork
  closed). It opened a window and then `setup_capture_target` retargeted every
  camera to the offscreen image, so the window rendered a **blank rectangle**.
  Deleted rather than fixed: a screenshot tool does not need one.
  ⭐ **`--character` was accepted and ignored for route captures** — same commit.
  ⭐⭐ **room captures now run the GGRS host a player runs.** The hand-assembled
  app was missing `set_simulation_host(Ggrs)`, `serialize_frame_schedules` and the
  `"game"` asset source too — so the fork was **wider than the five drifts we
  knew about**. The visible consequence is tiny and worth stating: of 230,400
  pixels, **27 differ and all 27 are the robot's foot**, one animation step apart.
  Sim pose byte-identical.
  ⛔ **and route captures had been running BLIND since they were written.** The
  shared builder disables `LogPlugin` (correct for tests, which build several Apps
  per process; wrong for a binary that builds one), so composing through it
  silences every engine `INFO`/`WARN`. Put back for both modes — measured by an
  empty log on the first run after, not predicted.
  ⚠ **4 of the 5 known drifts are now structurally impossible** (display surface,
  shell visuals, audio/persistence redirect, asset root) because there is one
  builder to forget them in. **2 remain possible** — `--dev-overlays` and
  `--combat-overlay` are systems, and room vs route genuinely install different
  ones. That is the irreducible half.

* For the web build we can't use kaledioscope because lunex doesn't support wasm

* ✔ **`maryo flashes when her fireball hits an enemy` — FIXED, and now GUARDED (2026-08-07).**
  ⛔ this row said "still open next door … (below)" and there was no row below: the
  only mention of the fireball in this whole file was the pointer. The fix had in
  fact landed — `damage/mod.rs` wraps the attacker flash in
  `if !matches!(event.source, HitSource::PlayerProjectile)`, quoting Jon's words
  verbatim — so the row was a dangling pointer to work that was already done.
  ⚠ **but nothing pinned it.** A bare `if` with no test is one refactor away from
  handing the bug back. `a_projectile_hit_flashes_its_victim_but_never_its_thrower`
  pins it now, and was verified by POISONING the guard rather than by observing a
  pass: with the condition removed the thrower's `hit_flash` reads 0.1 against the
  expected 0.0, which is Jon's symptom exactly.
  ⭐ it asserts BOTH halves — a slash still flashes — because a projectile-only
  assertion would pass just as happily against `hit_flash` being deleted outright.
  ⚠ the HITSTOP is deliberately not asserted absent: the brief hold is what makes
  a shot feel like it connected, nobody reported it, and the fix kept it.

* ◐ **The snakes-on-a-plane art already EXISTS, and half the wiring is now in.**
  Both `snakes_on_a_cartesian_plane` and `snakes_on_a_paper_plane` were drawn,
  published, and named by nothing. They have IDENTITY rows now — id, display
  name, traits and barks all DERIVED from the renderer's own spec rather than
  invented — plus hall pedestals and conversations, so they exist as characters
  and their art resolves.
  ✔ **behaviour and placement are BOTH in now (2026-08-06).** The archetypes are
  `mary_o::plane`, and 1-2 places two of each — see the 1-2 row below for what
  placing them cost, which was not nothing: an archetype is a brain, and the
  demo publishes its own enemy ART, so they drew red placeholder rectangles
  until `register_snakes_on_a_plane_sheets` existed.
  ⭐ **the flying part was already handled** — the movement kernel has carried a flight limb all
  along, and `body_kind: Floating` in the catalog row is the whole switch (it
  turns gravity off and grants the fly ability at spawn). Both rows now say
  Floating; they said Standard for an hour, which would have had a snake on a
  paper airplane fall out of the sky.
  ⚠ **which one is the 1-2 enemy was yours to say, and 1-2 got BOTH** — two of
  each, because they are different creatures rather than two skins and the room
  is where a player can see the difference (1 HP / fast against 2 HP / steady).
  Say the word and either one comes out.

* Maryo world 1-2 needs moving platforms that move vertically down and up like an elevator. When they go OOB (far enough so they are off screen of the player in normal gameplay) they can teleport to the top / bottom of the screen to make an infinite elevator effect.

* The maryo world 1-2 needs to happen after she wins world 1-1, she doesn't just get to go there in the middle of 1-1 and come back. It should be a new level. When she beats world 1-2 we should cycle back to world 1-1 for the demo. World 1-2 also needs to be built out a lot more, its very plain, and there are no enemies. I would like to add the flying snakes on a plane as enemies in this world.

  ◐ **two of the four are done, and one was already done before you asked.**
  * ✔ *"cycle back to 1-1"* and *"after she wins 1-1"* — the flag-pole route
    landed on 08-05: `LevelDestination` + `cycle_level_on_flag_tally` send 1-1's
    pole to 1-2 and 1-2's pole back to 1-1, and `level_circuit` runs the loop on
    the real schedule.
  * ✔ *"built out a lot more, its very plain"* — 1-2 was a rectangular box with
    one shelf. It has ceiling teeth, a second shelf, a ledge that makes the
    hidden block reachable, and a staircase up to the pole (2026-08-06).
    ⚠ **I did NOT add a second pit**: `the_chasm_is_only_crossable_by_the_platform`
    states the room's design rule — one hole, crossed by the ferry — and that
    rule is yours, so the floor went back. Say the word and it comes out.
  * ✔ *"add the flying snakes on a plane"* — two of each creature, in the open
    band above the shelves. ⭐ **the archetype was NOT the whole enemy**: they
    drew red placeholder rectangles until `plane.rs` published their sheets, the
    way `snake.rs` and `ai_slop.rs` publish theirs. The 08-06 ledger row saying
    *"the whole enemy is a TABLE"* was true of behaviour and false of the enemy.
    ⚠ they ROAM and never dive at you — `aggro_radius: 0.0` on the archetype —
    which is a design choice you can reverse in one number if you want a flyer
    that hunts.
  * ✔ *"she doesn't just get to go there in the middle of 1-1 and come back"* —
    all four mouths of the round trip are deleted (`mary_o_1_1_descent`, its pad
    `mary_o_1_2_arrival`, the alcove `mary_o_1_2_exit`, its pad
    `mary_o_1_1_surface_return`). 1-2 is entered only by finishing 1-1 and left
    only by finishing 1-2. The vault stays — its pipes move you inside 1-1.
  ⭐ **and doing it found the bug you then reported.** *"when I end world 1-1, I
    get warped into 1-2 in what looks like the same place and then immediately
    die"*, and the same out of 1-2: `FlagSequence::driven` is a position in the
    room the pole was in, `run_flag_sequence` writes it onto the body every tick
    the phase is not `Idle`, and `cycle_level_on_flag_tally` deliberately STAYS
    `Tallied` while departing. So the transition put her at the new room's spawn
    and the very next driver tick put her back at the old room's pole
    coordinates — 1-1's x=3240 is 1300px past the end of 1-2 — where she fell out
    of the world and the death beat restarted the level.
    `follow_the_active_room` clears the sequence when the room changes now: it is
    already the authority that re-derives the pole, and it is adjacent to the
    driver in the same `.chain()`, so nothing can land between them. Two tests
    pin it (`two_rooms::she_walks_out_of_one_room_and_into_another` waits for the
    body rather than reading it on the flip frame, and
    `the_run_survives_the_crossing` catches the lost life).

* ◐ **MEASURED 2026-08-05 — the number is now available, the taxonomy is still yours.**
  `cargo test -p ambition_app --test app_it print_how_tall -- --ignored --nocapture`
  prints every character's BAKED BODY in sheet pixels, tallest first. Your
  reference pair reads Alice **206** and Bob **204**; the ones you called tiny
  read Jeff Hinter **149**, viking heavy warrior **149**, viking warrior **143**,
  viking heavy shieldmaiden **135** — roughly **70%** of Alice, which is exactly
  the "out of place" you saw. Player robot v3 is **91**, and you said that one is
  meant to be short. Across all 129 the spread is **14.6x** (a 452px cellular
  automaton to a 31px puppy slug), so a single global rule cannot be right.
  ⚠ **no ratchet is written**, deliberately: you named an exception in the same
  breath as the complaint, so which characters are supposed to match is a call
  only you can make. Say which ones and the limit becomes writable.

* ⚠ **MEASURED 2026-08-06, and the proposed fix would not have worked — read this before rescaling any art.** A character's rendered HEIGHT is `max(collision.x, collision.y) * collision_scale` (`sprite_sheet::character::sprite_render_size`); the frame's pixel size never enters it, only its ASPECT sets the width. So rescaling art at the generator changes `frame_width` and `frame_height` together, leaves the aspect alone, and moves nothing on screen.
  * ⭐ **what actually differs is ONE authored number.** Alice and Bob have no `collision_scale` and take the default **1.5**. Jeff Hinter declares **1.05** — so for the same collision box he renders at **70%** of their height. That is the whole "tiny little character" effect. 33 of 182 sheets declare the field; the other 149 inherit 1.5, and every one of the smallest declares something explicitly.
  * ⭐ the field's own doc says it "compensates for transparent/cropped frame space", which is the mechanism: Jeff's frame is 160×160 SQUARE with a humanoid in it, Alice's is 225×252 portrait-shaped. A figure surrounded by more empty frame reads smaller at the same rendered height, and the low scale then shrinks it again.
  * ⭐ **QUANTIFIED — `scripts/report_character_scale.py`.** Every sheet carries
    `body_pixel_bbox`, so what a viewer sees is measurable:
    `figure = collision_scale * (bbox.h / frame_height)` per unit of collision box.
    Across 179 sheets with a figure the spread is **10.9x**. The humanoids you
    named: **alice 1.22, bob 1.20, jeff_hinter 0.98, genghis_cant 1.56,
    data_lovelace 1.75, noether 1.90** — so Noether reads nearly **twice**
    Jeff's height for the same box.
  * ⛔ **and `collision_scale` is going the WRONG WAY.** Its doc says it
    "compensates for transparent/cropped frame space", so a well-cropped sheet
    should need LESS. Instead the 95%-fill characters carry the biggest values
    (noether 2.0, data_lovelace 1.85) while jeff_hinter at 93% fill carries 1.05.
    The values were hand-set per character and drifted; they do not compensate
    for anything.
  * ⭐ **your own reference makes the fix arithmetic rather than taste.** You said
    *"Alice and bob are great"* — their figure height is **1.21**, and
    `collision_scale = 1.21 / fill` REPRODUCES Alice's authored 1.5 from her 81%
    fill (1.21 / 0.81 = 1.49). A formula that predicts the values you already
    like is not a matter of opinion. `--suggest` prints it per row.
  * ⭐ **MEASURED 2026-08-07, and the number is the argument.** Two groups of
    HUMANS currently render **1.69× apart**: the 13 scientist-cast sheets carrying
    an explicit `collision_scale: 1.0` sit at figure height **0.84**, while the 33
    high-fill sheets left on the `1.5` default sit at **1.42**. Your reference is
    1.21. So this is not "the values drifted a bit" — it is two populations of
    people, in the same Hall, one of them two-thirds the height of the other.
  * ⭐ **and the judgement is smaller than the row implies.** The report lists 180
    sheets; only **116 are catalog characters**. The other 64 have no catalog row
    at all — `lasersword`, `portal_gun`, `cut_rope_anvil`, `bow_arrow`,
    `throwing_javelin`, `news_board`, `shrine`, the `goblin_*` weapon sheets — so
    a figure-height formula is meaningless for them and they are out of scope
    without any judgement being made. ⚠ a few in that 64 look like UNCAST
    CHARACTERS rather than props (`george_booul`, `goblin_brute_hammer`), which is
    the same orphan-sheet class the Hall identity scan turned up; noted, not acted
    on.
  * ▢ **so the work is: pick which sheets are HUMANOID and apply the suggestion.**
    The candidate list is the 116, and the ones where the formula is obviously
    WRONG are identifiable by shape rather than by taste — `puppy_slug` (0.49),
    `snakes_on_a_*` (0.59), `stochastic_parrot` (0.70), `trex_enemy`,
    `burning_flying_shark`, the `*_mite`s. Everything above ~80% fill in that list
    is a person.
    ⚠ the script cannot tell a viking from a puppy slug, and its suggestion is
    nonsense for a snake, an explosion, a pipe, or a deliberately chibi robot —
    that judgement is yours and is the only part left. ⛔ **edit the `.yaml`, not
    the `.ron`**: the RON says "Auto-emitted from …_spritesheet.yaml" on its first
    line and a regeneration discards a RON edit. ⚠ `collision_scale` is
    presentation-only, so none of this can change how anything plays.
  * ⚠ **the player robot v3 exception still holds** and is easy here: chibi is a low `collision_scale`, stated deliberately rather than inherited.
  * *(your words, kept)* In the hall of characters, the humanoid characters are all dramatically out of scale with each other. Alice and bob are great, but characters like the vikings, or jeff hinter render as tiny little characters and look out of place compared to the rest of the cast. The character art needs to be rescaled (probably at the generator level, not via some post-hoc fix) to balance the size of these characters so they make more sense appearing in the same game together. Note the player robot v3 is supposed to be chibi and short compared to other humanoids.

* ◐ **When maryo-dies the enemies seem to reset before the death animation is finished.** The level reset needs to happen all at once at a time that is easy to express in the game code. This might be a larger refactor if there is a structural problem here, and we need to avoid creating a monolith.
  * ⚠ **TRACED 2026-08-03, and the code does NOT currently do what the report describes — so this needs your re-check before it gets a refactor.** `ResetRoomFeaturesEvent` (what restores enemies, breakables and pickups) has exactly ONE production writer: `reset_sandbox`, reached only through `apply_room_replay_request_system` draining `RoomReplayRequested`. Mary-O emits that from `restart_level_after_death`, which returns early while `sequence.active()` — i.e. **after** the beat. So enemies should not reset early.
  * ⭐ **what DOES happen immediately is the PLAYER**: `death_respawn_player` resets her clusters, anim and combat on the fatal hit, and the death module's own doc says so — *"the engine respawns a dead player IMMEDIATELY, that is why her `Death` row was unreachable"*. The beat then holds her body and re-arms `death_anim_timer` every tick because that immediate respawn wipes it. **So the thing that resets mid-animation is her, not them** — and if what you saw was the world looking untouched while she died, that is the same root wearing different clothes.
  * ✔ **RE-MEASURED at HEAD 2026-08-06, and the trace's CONCLUSION holds — but its count was wrong.** It said `ResetRoomFeaturesEvent` has "exactly ONE production writer". There are TWO: `apply_room_replay_request_system` (two write sites, one function — the replay path Mary-O's death reaches) and `apply_player_reset_input_system` (the manual Reset button). Two more writes are in tests. The second production writer is not the reported symptom — pressing Reset is not dying — so nothing about the finding changes, but a row that says "exactly one" is a row somebody will trust the next time they look for a stray writer. ⭐ the guard is intact: `restart_level_after_death` still returns early while `sequence.active()` (`remaining > 0.0`), so the replay cannot fire during the beat.
  * ⛔ **RETRACTED — a 2026-08-07 answer here was WRONG and is corrected below.**
    It claimed `death_respawn_player` teleporting on the fatal tick meant "no beat,
    nothing to animate over". That is true of the ENGINE and false of Mary-O:
    `death.rs`'s own module doc says *"The engine respawns a dead player
    IMMEDIATELY … So this holds the level for a beat after the death"*, and *"She
    dies WHERE SHE DIED, not at spawn"*. The demo OVERRIDES the engine's teleport
    and pins her at the death position. ⚠ the mistake was reading one call site
    and not asking whether a consumer supersedes it — the failure this repo keeps
    paying for.
  * ⭐ **what the code actually says, and it makes the row look STALE.** Mary-O has
    a complete death beat: `DEATH_DWELL = 3.2` seconds, body held at the death
    position, controls blanked, the `Death` animation row playing, level replay
    gated behind it. So on the current code neither half of the original report
    reproduces — the enemies are behind the beat and she does not blink to spawn.
  * ⭐⭐ **and Jon has already been shown a beat**: `DEATH_DWELL`'s own comment
    records a LATER report of his — *"death isn't long enough for the entire death
    music to play"* — and says the value was raised from **1.6s to 3.2s** because
    the sting was being cut off. A beat of 1.6s that ended mid-tumble is a very
    good candidate for *"the enemies seem to reset before the death animation is
    finished"*: the reset was correctly gated behind the beat, and the BEAT was
    ending too early.
  * ~~▢ **so the row's remaining question is whether it still reproduces at 3.2s.**
    That is one death, watched once, and it is the only part a code reading cannot
    settle.~~ ⛔ do NOT do the "one reset at one time" refactor first: the reset
    ordering is already correct and was never the symptom.
  * ✔ **MEASURED 2026-08-08 — no, and it never could have.** Three numbers, all
    found at their source:
    | | | |
    |---|---|---|
    | death **animation** | **0.12 s** | 1 frame × 120 ms, non-looping (`mary_o_v2_spritesheet.yaml`) |
    | death **beat** | **3.2 s** | `DEATH_DWELL`, `demo_mary_o/src/death.rs:45` |
    | death **music** | **3.200 s** | `ffprobe` on `mary_o_you_died/full.ogg`; 4 bars of 2/4 at 150bpm |

    The reset lands **3.08 s after the clip finishes — 26.7× its length.** Even
    at the old 1.6 s the margin was 1.48 s. Confirmed three independent ways: the
    deterministic sim log (`beat started f3, ended f195, room reset f195`), three
    real captured deaths (reset at 2.882 / 3.299 / 3.349 s), and screenshots
    inside the beat.
  * ⛔ **and this retracts the ⭐⭐ theory above: there is no tumble.** Mary-O's
    death animation is a **single static frame** in all three forms — authored
    that way in the generator (`SHORT_POSES["death"]` is one pose). A 1.6 s beat
    cannot cut a 0.12 s clip, so the beat's length was never the mechanism behind
    your original report. The only thing 1.6 s ever cut off was the **music**,
    which is exactly what the second report said.
  * ⭐ **so the live explanation is the one the freeze row already names**: the
    enemies walk for the full 3.2 s and then snap. Measured today at eight
    verified in-beat captures — the slop at screen x = 334, 330, 306, 248, 273,
    279, 275, 202 with the camera anchored. That is your *"reset before the
    animation is finished"*, and it is about the absence of a freeze rather than
    about any duration.
* ~~We probably need an engine concept that allows actors to be dormant.~~ **BUILT, and then found half-wired 2026-08-05.** `features::ecs::dormancy` declares it the way your last clause asks: an actor with no `DormancyPolicy` is always awake, so "not inherent" is the default rather than an opt-out, and `DormancyPolicy::Never` exists so a character that must keep simulating says so where a reader finds it. The wake test is *near any OBSERVER*, never near "the player" — one player, four on a sofa and a remote peer are the same rule, which is your split-screen and netplay point. It sleeps the BRAIN and CLEARS the control frame (a sleeping actor with a stale `ActorControl` keeps walking, which is the exact symptom), and `Dormant` is recomputed every tick from positions so a rollback reproduces it with no memo to get wrong.
  * ⛔ **but only the SLOP was wired to it for a day.** You named the slop, so the slop got the policy — and Solid Snake, the other patrolling enemy in the same level with the same job, thought for the whole course. Fixed `ad43b63ba`. ⚠ **the test was defending the gap**: it asserted that ONLY the slop declares dormancy, with a strays check that would have failed the moment anyone wired the snake. It asserts the property now — every authored enemy declares whether it sleeps — and was probed red.
  * ◐ **Sanic adopted it too (`df269eaec`), and the second game is what made the seam's assumption visible.** A wake radius is a LEAD TIME wearing distance's clothes: your 720px in front of a Mary-O at 300px/s is 2.4s of warning, and the same 720px in front of Sanic at his 2000px/s super top speed is **0.36s** — a badnik snapping into motion in full view, worse than one that had been walking all along. His radius is 4800, derived from his own top speed for the same 2.4s of lead. ⭐ that a fixed radius silently encodes an assumption about how fast the OBSERVER moves is the argument for the policy eventually taking SECONDS rather than pixels; recorded at the constant, not acted on, because two games is not enough to change an engine seam's units.
  * ◐ **the content bosses and the hall cast still declare nothing — but the question is ANSWERABLE now** (`awaiting-maintainer-decision.md`, 2026-08-07): the Hall is 129 NpcSpawns all on `stand_still`, and `tick_npc_idle_barks` carries no dormancy filter, so sleeping a statue would not cost it its voice. Recommendation there: `Never` on the bosses (free, behaviour-preserving), leave the Hall until a frame profile says 129 no-op brain ticks matter. Both are stationary or scripted, so the cost of leaving them is low and the right answer may well be `DormancyPolicy::Never` stated explicitly rather than silence — but nobody has decided, and silence currently reads the same as "always awake" whether or not that was chosen.

  * *(your words, kept)* We probably need an engine concept that allows actors to be dormant. This is important for maryo because ai slop will just walk off the edge of the level before she even gets to that part of the level, so we need to wake or sleep their brain depending on how close she is to them. This sort of optimization will likely be generally important for any game using the engine, although it's not something that should be inherent. There might be characters that don't go dormant off screen, this matters a lot for split screen or network multiplayer games. It also might matter in other cases. Not 100% sure how its elegantly expressed though.

* ~~In mary-o blocks that are used need a new texture so they are visually distinguishable. They also need a small animation (probably an in-code position nudge up and back into place) when they are hit.~~ **BOTH DONE.** A spent block wears `EntitySprite::SpentBlockTile` — its OWN inert texture rather than falling back to plain masonry, which would have hidden its history — chosen in `dress_power_blocks` from `SpentPowerBlocks` every frame rather than from the bonk EVENT, because that set is rollback state and art driven by the event would keep the used look through a rewind that undid the strike. The nudge is `BlockStruck`, emitted by the bonk and consumed by the render layer (`rendering/world.rs`): the block's own position is never moved, exactly as you guessed it should not be — moving it would lift a body standing on it.
  * ⭐ **and a related asymmetry fixed 2026-08-05**: a BRICK drew as the generic dark slab while the level's own solid surfaces drew `SolidTile`, the seamless brick pattern. Same `BlockKind::Solid`, two textures, decided by whether the block came from the IntGrid or from an entity. Bricks wear the masonry now (`c6a7034a3`).

* **[agent-found] The couch-input smash test no longer fails ONLY in a full-binary run — ⚠ RE-MEASURED 2026-08-07 and the diagnosis below is STALE.** `app_it::smash_in_the_host::a_keyboard_player_and_a_pad_player_drive_different_fighters` passes in isolation and fails when the whole `app_it` binary runs (measured both ways, 2026-08-02, and confirmed identical on a clean worktree at the previous commit — it is not a regression from the Mary-O geometry or contact-harm work). So it is order- or parallelism-dependent: something earlier in the binary leaves seat/pad state that this test then reads. That is a real defect and not merely a flaky test — a keyboard participant and a pad participant driving the same fighter is exactly the couch bug class that has bitten repeatedly, and the isolated pass is what makes it invisible. Worth finding what the shared state is rather than adding a serial-test attribute over it.
  * ⛔ **RE-MEASURED 2026-08-07: it fails in ISOLATION too.**
    `cargo test -p ambition_app --test app_it -- --exact
    smash_in_the_host::a_keyboard_player_and_a_pad_player_drive_different_fighters`
    → `0 passed; 1 failed; 317 filtered out`. So the "passes alone, fails in the
    binary" characterisation no longer holds, and anyone acting on it would go
    hunting for cross-test shared state that is not what is breaking it now.
    ⚠ **still not a regression from any current campaign**: the same test fails at
    `b24ccb6b2` in a full-binary run, checked by detaching a worktree to that
    commit. Something between 08-02 and now turned an order-dependent failure
    into an unconditional one — which is arguably PROGRESS, because a
    deterministic failure is diagnosable and an order-dependent one is not.
  * ⭐ the current symptom is concrete: *"the keyboard moved the PAD player's
    fighter (40.34px against the keyboard player's -50.05px)"*, reproducible on
    demand.
  * ⛔ **but the SIGNS say it is probably not shared input, and the assertion's
    wording is what misleads.** The keyboard pressed RIGHT; seat one's fighter
    moved **-50** (left) and seat two's moved **+40** (right). Two seats reading
    one source move the SAME way — that is what the assertion is shaped to
    catch. Opposite directions with the presser going backwards is the signature
    of CONTACT: two fighters overlapping and pushing each other apart.
  * ⚠ **so the cheapest next measurement is their spawn separation**, not a hunt
    through device routing. If they spawn close enough to overlap, this test has
    been measuring push-apart rather than input ownership, and the 08-02
    "passes alone" behaviour was spawn-position luck rather than test order.
    `seat_input_participants_for_roster` already gives every non-primary seat
    `BindingRecipe::gamepad_only()`, so the keyboard has no route into seat two's
    map to begin with — which is evidence for the contact reading.
  * ⛔ **NOT picked up here on purpose**: smash seating is the parallel agent's
    active area (their in-flight work renames `ActiveMatch` / `seat_match_
    participants`). Two people editing that at once is how a merge gets
    expensive. Recorded with the evidence so whoever owns it starts from the
    measurement rather than from the 08-02 story.

* **[agent-found] Mary-O's gameplay box is her raw alpha silhouette, because her generator authors no `body_inset`.** Her three forms now hand their collision geometry to their sheets (`BodySource::SpriteAuthored`), so box and sprite derive from one authored scale and can no longer disagree — that part is done. But `body_pixel_bbox` is the measured alpha bbox, hat and outstretched arms included, so her tall form is ~36 px wide against a 32 px tile. The builder already has the right seam for this: `CharacterGenerator.body_inset()` takes per-edge fractions of the measured box, seven other characters override it, and its own docstring notes that being fractional is what makes it "survive art changes". Mary-O's generator overrides nothing. The fix belongs there — carve the gameplay body in from the silhouette — and NOT in a second box authority in the game. Per-pose `hurtbox_parts()` is the finer-grained version of the same seam if a pose needs its own rect.

  Related: the pixel→world scale has no representation in the builder at all. The game now derives it as `MARY_O_STANDING_HEIGHT / <measured bbox height>` so a regeneration that re-crops her keeps her exactly as tall as the level expects, but "how big is one sheet pixel in the world" is arguably a fact the sheet should carry.


* The current player V3 collision / hurt box  is larger than the player sprite. It needs to be slightly inset from the visible parts of the player. It should be under the main head, and well within the player arms. The player hitbox needs to be very forgiving to the player.

---

◐ **DONE 2026-08-05 except the two you marked iffy** — and per your follow-up
(*"only the ones I named in my observations should be removed"*), **nothing else
was cut**: every other uncast character was treated as belonging, and the four
that had art but no row were CAST rather than removed.

◐ — 21 rows removed from the
catalog, their hall pedestals and hall dialogue with them; `npc_creator_final`
folded into `npc_creator` (they already shared one spritesheet, so it really was
one character wearing two ids — the intro's raid corridor still plays the
`creator_final_normal` scene, because the SCENE is what differed); "Robot" is now
"Robot V1"; and the hall reserves the doorway column so no pedestal can stand in
the door. ⚠ **Exploding Mite and Dividing Mite are still here** — "iffy" is not a
decision and they are yours to make.

I want to remove some of the clone characters, that I don't find interesting, specificly:

Robot Caster, Robot Engineer, Robot Archivist, Robot Diver, Robot Guardian,
Robot Medic, Player Combat Review, Player Extended, Player Social Review,
Player Traversal Review, Robot Miner, Robot Runner, Sandbag Full Review, Sandbag Armored Review,

Goblin Frost Sword, Goblin Shaman Staff, Goblin Desert Bow, Goblin FOrest
Spear, Skirmisher, Goblin Cave Digger, Goblin Brute Hammer

Exploding Mite and Dividing Mite are iffy

Creator Final should not be a different character than creator.

"Robot" should be named to "Robot V1".

In the hall of characters we should prevent the characters from overlapping the
door. Currently robot v3 overlaps the door.


---

## ◐ HEADLINE FEATURE, 2026-08-05: the smash character select screen

*Jon's spec, kept verbatim at his instruction ("Copy this spec into my observations
file so we don't lose the verbatim text in case we compact"). Anything below this
line that is not in the blockquote is agent commentary and can be edited freely;
the blockquote cannot.*

> I want you to make a new headline feature for this next push, which is a better
> smash character select screen. It should be a new UI that lives in the smash demo
> game. Nothing in the engine needs to support it if it doesn't make sense. There
> might be minor engine helpers that do make sense (e.g. portraits). What it should
> have is a grid of portraits for each of the selectable characters on the top 65%
> of the screen. The bottom 35% of the screen should be 4 participant slot cards. In
> this UI the arrows or game stick or mouse should move a cursor that can click on
> elements. Each participant slot will have a button to toggle it between a
> controller player (which must have a corresponding attached controller), a CPU
> player, or not participating. Each participating card gets a corresponding sphere
> icon on the character grid that the cursor can pick up and drag to select a
> character. The selected portrait should appear on the participants bottom card.
> Effectively this should work just like how the real smash character select screen
> works, which should help you fill in details that I didn't write down explicitly
> here. The main out of scope features are, we will not have different character
> colorings or skins yet. There will be no match options, it will always be a 3stock
> round. The main purpose is to have a nice UI for selecting characeters to prove we
> can do it.

**Explicitly out of scope:** alternate costumes/colours; match options (always a
3-stock round).

**Read "just like the real one" as licence to fill in:** a slot that has picked
nothing is not ready; the match cannot start until every participating slot has a
character; a cursor hovering a portrait previews it; picking up a token and
dropping it on empty space returns it rather than clearing the slot; a controller
slot with no attached pad cannot be set to controller.


* ✅ **FIXED — You should not be able to stand on an invisible block.** (Jon,
  2026-08-05; landed 2026-08-06.) A `Hidden` block lowers to
  `BlockKind::BonkOnly`, the mirror of `OneWay`: solid ONLY against a head coming
  up into it, air to feet, never a side wall. The coin still pays, because the
  reward is the head contact.
  ⛔ **and it shipped half-done first, which is the part worth keeping.**
  `is_solid_for_axis(BonkOnly, gravity_axis)` is TRUE — a rising head must be
  stopped — so every caller that filtered on that alone and forgot
  `bonk_strike_from_head` still saw a floor. Two did: the controlled body's
  penetration REPAIR and the generic kinematic sweep enemies use, which meant one
  block kind meant different things depending on which movement engine drove the
  body. `blocks_only_a_rising_head` states the rule once and both paths ask it.
  ⭐ `an_invisible_block_is_not_a_floor_but_is_still_strikeable` is Jon's sentence
  as an assertion, next to the `OneWay` sibling it is the mirror of.
  The original report:
  A `MaryOBlockLook::Hidden` block is drawn transparent and is still `BlockKind::Solid`,
  so it is an invisible FLOOR — you can land on nothing. In SMB an invisible block
  is intangible until struck from below, and then it is solid.
  * ⚠ **the fix is a collision vocabulary the engine half has.** `BlockKind::OneWay`
    is *"only solid when the player crosses from above"*; a hidden block needs the
    MIRROR — solid only when struck from below — and `is_solid_for_axis` /
    `is_support_surface` in `collision_semantics.rs` are where both live. The
    matches there are exhaustive, so a new kind names every site that must decide.
  * ⛔ **not "just make it non-solid"**: the bonk is a `ContactKind::Head` contact
    from the collision system, so a block with no collision cannot be struck at
    all. Removing solidity removes the reward.

* ▢ **The snake and AI slop are still way too big visually, and the sprite might
  not match the box for the snake.** (Jon, 2026-08-05, second report)
  ⚠ the earlier attempt at this measured badly TWICE and is recorded as such — a
  colour filter ate the green warp pipes, and the snake has two body states so
  two captures compared different animals. The 0.35 that landed was arithmetic,
  not a derivation. Jon's "the sprite might not match the box" is the lead worth
  following: if the drawn quad and the collision box disagree, scaling one does
  not fix the other and the visual size will keep looking wrong.
  ⭐ **MEASURED 2026-08-06** — `enemy_body_scale`, in SHEET PIXELS (the unit the
  generator works in), asking `posed_body_geometry` rather than a capture:

  ```text
             target     collision        render   x_vs_p   y_vs_p
    player_robot_v3     57x91        224x224       1.00x    1.00x
        solid_snake    117x52        128x128       2.05x    0.57x
            ai_slop    257x167       271x232       4.51x    1.84x
  ```

  ⛔ **"way too big" is in the COLLISION BOX, not only the paint.** AI Slop's
  authored body is **four and a half times the player's width** and nearly twice
  its height. Scaling the art alone would leave a stomp that connects from half a
  screen away.
  ⭐ **and Jon's second clause is confirmed and quantified**: the snake's box is
  long and low (117x52) under a SQUARE 128x128 quad — 1.09x horizontally against
  2.46x vertically. A great deal of vertical empty space over a serpent that is
  barely half the player's height, which is exactly "the sprite might not match
  the box".
  ⭐⭐ **AND IN WORLD UNITS — which is what Jon is actually looking at — the two
  enemies are sized by two different mechanisms and NEITHER is a derivation:**

  | body | world size | how it is decided |
  |---|---|---|
  | Mary-O | 48 tall | **DERIVED**: `MARY_O_STANDING_HEIGHT / sheet_height`, re-measured every regen |
  | Solid Snake | **40.9 × 18.2** | `SNAKE_WORLD_PER_PIXEL = 0.35`, a constant whose own doc calls it a taste call |
  | AI Slop | **28 × 28** | `AI_SLOP_HALF = 14.0`, splatted — a forced SQUARE that ignores the sheet |

  ⚠ **the SNAKE half of this was already known** — `enemy_quad_matches_its_box`
  (2026-08-05) records 117x52 in a 128x128 frame, 41x18 world at 0.35, and
  ratchets the 2.47x overhang, with the two fixes named (*"an art-pipeline crop
  or sizing the quad from the body"*). Re-measuring it found the same numbers,
  which is a good sign about both instruments and NOT a new finding. What the
  re-measurement adds is the ruler: that test reads against a 32-unit TILE, and
  against the PLAYER the snake is **2.05x his width**.
  ⚠ **2026-08-08 — that "2.05x", and the "4.51x" below, are SHEET-PIXEL ratios
  across sheets with different pixel densities, so they do not describe in-game
  size.** In world units the snake is **1.00x** Mary-O's width and the slop
  **1.09x**. The conclusion drawn below is unaffected — it was about the box's
  SHAPE (a square over a 1.54:1 animal), which no ratio was needed to see — but
  the numbers should not be read as sizes. Annotated because I made the identical
  slip from the identical column three days later, which suggests the instrument's
  output invites it.
  ⛔⛔ **AI SLOP HAS NO SUCH TEST AND IS WORSE, and this part IS new.**
  `enemy_quad_matches_its_box` covers the snake only. The slop's sheet publishes
  **257x167** — a 1.54:1 animal, and **4.51x the player's width** — while
  `AI_SLOP_HALF: f32 = 14.0` is *splatted*, forcing a 28x28 **SQUARE** body that
  ignores the sheet entirely. So on the slop *"the sprite might not match the
  box"* is not a scale mismatch at all: the box has the wrong SHAPE, and no value
  of any scale knob can make a square describe a 1.54:1 creature.
  ✅ **FIXED 2026-08-06 (`fbc3689e7`) — the slop's box has its sheet's shape.**
  `AI_SLOP_BODY_WIDTH` is the one authored number (28, what it occupied before,
  so no room needs re-authoring) and the height is a measurement:
  **28 x 28 → 28 x 18.2**. Derived per call like `mary_o_world_per_pixel`, because
  sheets are regenerated and a scale pinned to today's pixel count silently
  resizes the creature the first time a crop moves.
  ⛔ **the first draft of its test recomputed the height from the sheet and was
  green against the very splat it prevents** — a guard passing through its own
  arithmetic rather than through the code. It asks `ai_slop_half_size()` now.
  ⚠ **AND THIS DOES NOT ADDRESS "TOO BIG VISUALLY."** It changed the COLLISION
  box, not the drawn sprite, so a capture would look identical. Jon's complaint
  has two halves and this is the one that was a defect; the other is a size, and
  a size is his call.
  ✅ **AND THE SNAKE IS SHRUNK — a BLIND DRAW, `1bff9c14d`, rejectable in one
  line.** Mary-O measures **25.6 x 48** world (height authored, width measured);
  the snake was **41.0** wide — 1.6x her width while barely a third of her
  height. `snake_body_width()` is now her width, so a snake occupies the corridor
  a Koopa does beside Mario. **41.0 → 25.6, a 38% reduction.**
  ⭐ **why the two previous answers did not settle it**: neither had a
  DENOMINATOR. A pixel scale cannot express "too big" — that is a comparison, and
  there was nothing to compare against. `mary_o_body_width()` is the denominator
  and it is a measurement, so it stays right through a regen.
  ⭐ looked at it: `capture_mary_o` of 1-1 shows the walking snake at about a
  tile, beside the tan crate that is a boxed one.
  ⭐ **AND THE TWO NAMED FIXES HAVE A THIRD, found 2026-08-06 while measuring the
  Hall cast — the same mechanism underlies both reports.** A quad's ASPECT comes
  from `frame_width / frame_height` (`sprite_render_size`), i.e. from the FRAME,
  which is mostly padding: the snake's 117x52 animal sits in a 128x128 square, and
  the Hall's "tiny" characters are figures adrift in frames. But every sheet
  already publishes `body_metrics.body_pixel_bbox` — the animal's real rectangle.
  Taking the aspect from the BBOX instead of the frame would make every drawn quad
  match its figure, on the whole cast, without touching the art pipeline (which is
  Jon's `tools/ambition_sprite2d_renderer` and off-limits to a code change anyway).
  ⛔ **but it is NOT a one-line aspect swap, and the trap is a known one.** Changing
  the quad's aspect while still drawing the FULL frame texture stretches the art —
  the "stretched sprite moves the collision off the picture" failure, again. It
  needs the drawn REGION cropped to the bbox in the same edit (a sprite sub-rect),
  and `body_metrics.feet_anchor_norm` exists precisely because the figure sits
  somewhere inside its frame, so the anchor is computed against the frame today and
  would have to move with the crop. Three coupled changes, not one.
  ▢ **what is left, stated as the choice it is:**
  * the SNAKE's quad/box disagreement is an art-pipeline crop, a quad sized from
    the body, or the bbox-aspect route above — which is the one that also fixes
    the Hall cast, and the one with the anchor coupling to get right
    (`enemy_quad_matches_its_box` ratchets it at 2.47x);
  * the SIZES themselves — the snake at 41 x 18 world and the slop at 28 x 18
    against Mary-O's 48 tall — are one number each, and both now live where
    stating a different one is a one-line edit rather than a hunt.
  ⭐ **the fix Mary-O already demonstrates, in the same crate**: state the world
  size you want and divide by the measured sheet, so a regen that moves the crop
  by a pixel does not silently resize the creature. Her own comment says why —
  *"a scale pinned to today's pixel count silently changes her height the first
  time a crop moves"* — and both enemies are pinned to today's pixel count.
  ⭐⭐ **AND THE TWO REPORTS COLLAPSE INTO ONE DECISION — the arithmetic says so.**
  The scale report's own formula is `figure_height = collision_scale x fill`,
  where `fill = body_pixel_bbox.h / frame_height`. So hitting a target figure
  height means `collision_scale = target / fill`: **`collision_scale` IS a
  reciprocal-of-frame-padding fudge and nothing else.** That is why the row above
  found the values "do not compensate for anything" — they are 116 hand-tuned
  approximations of a quantity the code can compute exactly.
  ⭐ **so the bbox-aspect route does not merely fix the snake — it deletes the
  per-character number.** If the quad is sized and cropped from
  `body_pixel_bbox`, fill is 1.0 BY CONSTRUCTION and every character's
  `collision_scale` becomes the same global constant (the target figure height).
  116 authored numbers collapse to one.
  ⚠ **which reorders the work.** Applying `--suggest` per character is a stopgap
  that the bbox route would then obsolete — the 116-row humanoid judgement would
  have been spent on values that stop existing. ⭐ **decide the bbox route
  first**; if it is taken, the only judgement left is the handful of creatures
  whose figure should NOT match a human's (slug, snakes, parrot, trex, shark,
  mites), which is a much smaller ask than 116 rows.
  ⚠ it remains three coupled changes (aspect + drawn sub-rect + `feet_anchor_norm`
  moving with the crop), and that has not changed.

  ⛔⛔ **[agent-found 2026-08-08 — THE ANNOTATION BELOW WAS WRONG, corrected the
  same day.]** I wrote that "the bbox route is already in the tree". It is not.
  `posed_body_geometry` returns `render: Vec2::new(frame_w, frame_h)` — **the
  quad is the FULL FRAME on both paths**, so none of the decision's three coupled
  sites has shipped:
  the quad's width still comes from the frame's aspect (site 1), nothing draws a
  bbox sub-rect (site 2), and `feet_anchor_norm` is still normalised against the
  frame (site 3). The measured overhang confirms it: the snake's quad is
  **2.46x** its body's height and the ratchet is green at that value.

  ⭐ **what IS in the tree is a different mechanism, and it is worth knowing
  about**: `character_sprites::posed_body` derives the COLLISION box from the
  bbox and emits a `sprite_offset` that puts the art's rectangle on that box.
  That solves the STRETCH the decision worries about — the quad keeps the
  frame's aspect, so nothing is squashed — without sizing the quad from the body.
  The overhang that remains is transparent padding, not a stretched creature.

  ⚠ **so the decision is still open and still needs the three sites**, and the
  honest correction to my own claim is that "already shipped" confused *"the art
  is aligned to the box"* with *"the quad is the box"*. Original annotation
  follows, kept because its measurement of the DATA is still true:

  ```text
  collision     = body bbox           x world_per_pixel
  render (quad) = the FULL FRAME      x world_per_pixel     ← not resized, so not stretched
  sprite_offset = frame centre - bbox centre                ← puts the art ON the box
  ```

  So there is **no drawn sub-rect to crop and no `feet_anchor_norm` to move**:
  the quad keeps the frame's own aspect and the OFFSET does the aligning.
  `sync_sprite_posed_bodies` re-derives all three every tick from the pose, and
  writes `ActorRenderSize` / `ActorSpriteOffset`, which the renderer already
  prefers over `collision x collision_scale`. ⛔ **"decide the bbox route first"
  is therefore already decided — it shipped.**

  ⚠ **and the real blocker is DATA, not design: 2 of 190 sheets.**
  `find -L …/assets/sprites -name '*_spritesheet.ron'` is 190 records; exactly
  **two** declare `authored_body: true` (`player_robot_v3`, `vera_ruin`). The
  other 188 fall back to `collision_scale`, and they cannot be switched by an
  engine edit: `authored_body_pixel_size` deliberately REFUSES a measured alpha
  bbox, because a measured box is the extent of the drawing — hat, antenna and
  outstretched arms — and using it as a body is how a collision box ends up 1.28x
  the character inside it. A sheet joins the route by AUTHORING its box
  (`body_metrics_fn` or a fractional `body_inset` in the renderer), which is a
  per-sheet content act.

  ⭐ **so the reordering worry dissolves.** `--suggest` and the bbox route are not
  competing routes to the same end — `report_character_scale.py --suggest`
  suggests a `collision_scale` (the legacy number), and the bbox route deletes
  that number for any sheet that authors a box. They are the stopgap and the fix,
  and the fix is available per character TODAY without waiting for anything.

  ⛔ **and "author a body box for the snake" would change NOTHING**, which the
  correction above is what reveals: the snake's sheet already publishes a
  117 x 52 body inside a 128 x 128 frame, and `render` is the frame regardless of
  whether that body is authored or measured. `authored_body` gates
  `authored_body_pixel_size` (which SIZES the collision box), not the quad. The
  data measurement below stands; the conclusion drawn from it did not.

  ⭐ **the smallest useful next step is therefore site 1 alone** — return
  `render` from the bbox in `posed_body_geometry` and see what the ratchet and a
  `capture_scene` say. If the art visibly stretches, sites 2 and 3 are required
  and the decision's "three coupled changes" was right; if it does not, the
  coupling is smaller than the decision assumes. Either answer is worth more than
  more reading.
  ✔ **DONE 2026-08-08, and the answer is: it stretches, badly.** The quad/box
  number goes perfect — `2.46x → 1.00x`, the ratchet fires its own lower bound —
  and the picture is destroyed getting there. The snake flattens to 41% height at
  90% width (a 9px green smear, the cardboard box on its back a flat plank);
  Mary-O squeezes to 41% width at 64% height, arms retracting into stubs. Both
  match `bbox/frame` arithmetic to measurement error, so this is the mechanism
  and not a rendering accident: `custom_size` scales the whole frame into the
  quad per axis, so shrinking the quad without cropping the source divides the
  padding into the body. **Your "three coupled changes" was right**, and the crop
  (site 2) is the load-bearing one.
  ⛔ **and it is FOUR sites, not three** — the decision named one render-size
  publisher and there are two. The snake and Mary-O go through
  `posed_body_geometry`; the `collision_scale` path at `geometry.rs:25` that the
  decision describes is a different one. Proof in the same captured frame: the AI
  Slop measured **0.99x/1.02x, completely unmoved**, because it never attaches
  `SpritePosedBody`. Fixing only the named site would fix the Hall's spread and
  leave the two characters you complained about untouched.
  ⚠ the code was reverted, not landed — this is evidence for your open decision,
  not a change made on your behalf.
  ⭐⭐ **THE ROUTE IS THE FIX FOR "way too big", and the body sizes are already
  right.** ⛔ this heading said the opposite for two hours — *"the route does not
  fix it, your two complaints are two different problems"* — while the correction
  sat in the paragraphs below it. **That is the same defect I fixed in a design
  doc this morning**, repeated by me in this file the same day: a heading is a
  claim, and it gets corrected in the same edit as the body.
  Ran `enemy_body_scale` for the first time since it was written:

  |          target | collision | render | x vs player | y vs player |
  |---|---|---|---|---|
  | player_robot_v3 | 57x91   | 224x224 | 1.00x | 1.00x |
  | solid_snake     | 117x52  | 128x128 | **2.05x** | 0.57x |
  | ai_slop         | 257x167 | 271x232 | **4.51x** | **1.84x** |
  | mary_o_v2       | 64x120  | 160x192 | 1.12x | 1.32x |

  ⛔ **I READ THAT TABLE WRONG FIRST — it is in SHEET PIXELS, and these sheets
  have different pixel densities, so cross-sheet ratios of it mean nothing.** In
  WORLD units, which is what you see:

  | body | world size | vs Mary-O's width |
  |---|---|---|
  | Mary-O | 25.6 x 48.0 | 1.00x |
  | AI slop | 28.0 x 18.2 | **1.09x** |
  | snake | 25.6 x 11.4 | **1.00x** |

  ⭐ **your enemies are already the right SIZE** — that landed 2026-08-06, and the
  snake's own comment names your report: *"before this it was 41.0 world units
  against her 25.6 — 1.6x her width … that is the 'way too big' Jon reported
  twice."* `AI_SLOP_BODY_WIDTH = 28.0` is the one authored number and the height
  follows the art.
  ⭐⭐ **so what is left of "way too big visually" IS the quad/box gap, and the
  bbox fork is exactly its fix** — the body is right, the picture is 2.46x the
  body. Not two problems as I said an hour ago; one, and it is the one you are
  being asked to decide.

  ▢ **what this does NOT decide**: what the numbers should BE. The instrument
  asserts no ratio on purpose — what counts as too big is Jon's call, and a limit
  written by a test would be a taxonomy nobody chose. What it does say is that
  the snake should be stated as a WIDTH (or a height with the sheet's aspect
  respected) and the slop should stop being a square.



----


✔ **FIXED — your call, 2026-08-08: *"the oni leader bug was fixed"*.** Recorded
here because this entry carried no triage mark of any kind, which is the only
reason it cost anything: it read as untouched work and a 2026-08-08 survey
re-opened it as a queue row before you said otherwise.

The fix is `game/ambition_demo_smash/src/lib.rs:1307`, and it quotes your
reproduction. The smash experience now releases the roster, `PreparedMatch`, the
select state, the cursor and the seating source — each **by OWNER**, never by
type, because another game stages its own cast into the same resources and a
type-level removal deletes their match. Your "some persistent state that is
likely a global resource that is not reset" was exactly right, and it was more
than one.

⚠ **one gap, recorded and not reopened**: the guard that defends this is the
scope declaration plus the smash tests. `the_full_multi_game_lifecycle_is_leak_free`
walks Sanic → Mary-O → Pocket → TwinTrack → Ambition → Sanic and never runs
**smash → ambition**, so it is not the regression test for this even though its
name sounds like it.

  *(your words, kept)* Interesting bug, that's a big smell. When you play a round of smash and choose
your character, that becomes your character if you quit to title and play
ambition itself. That is a big architecture problem. Playing one game should
have no impact on the others. A quit to title and restart a game should also
have no impact on the game, it should feel fresh every time unless the game has
a save mechanic. But it should behave like you quit to the desktop and
restarted, so there is some persistent state that is likely a global resource
that is not reset, and that is probably an architecture problem. I noticed that
when I picked the oni-leader and then quit to title and played ambition I was
the oni leader and I could talk to other characters, but not the oni leader
himself. Strange.
