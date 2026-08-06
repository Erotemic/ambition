# Agents should only edit this file to mark something as potentially done. Jon
# will remove it if it is actually done, or mark it not actually solved if an
# attempt doesn't work.


* When SANIC is hit, there it seems like he is given no iframes. He should also have some hitstun and be knocked back a bit, and then have a few second of recovery iframes. The rings don't splash out nearly large enough. He needs an opportunity to recollect some of them after his hitstun wears off and before they disappear.

* ◐ **The patent clerk: you are making one right now.** During this run
  `patent_clerk.py` appeared in the sprite renderer and `patent_clerk` went into
  `regen_sprites.sh`'s batch; the target declares `special_patent_clerk` /
  "Patent Clerk". ⚠ **it has no catalog row yet**, so once it renders the sheet
  exists and nothing loads it — and no test says so, because the orphan check
  only asks the other direction (a catalog character with no art). Five other
  rendered identities are in the same gap: busy beaver, charley beagle, niels
  boar, pirate heavy, vera ruin. Tell me the tier and I will write the rows.

* For the web build we can't use kaledioscope because lunex doesn't support wasm

* ▢ **still open next door**: `maryo flashes when her fireball hits an enemy` (below) is a separate item and was not touched by this.

* ◐ **The snakes-on-a-plane art already EXISTS, and half the wiring is now in.**
  Both `snakes_on_a_cartesian_plane` and `snakes_on_a_paper_plane` were drawn,
  published, and named by nothing. They have IDENTITY rows now — id, display
  name, traits and barks all DERIVED from the renderer's own spec rather than
  invented — plus hall pedestals and conversations, so they exist as characters
  and their art resolves.
  ▢ **what is left is behaviour and placement**: a roster archetype the way
  `solid_snake` has `mary_o_snake`, and an LDtk placement in 1-2. ⭐ **the flying
  part is already handled** — the movement kernel has carried a flight limb all
  along, and `body_kind: Floating` in the catalog row is the whole switch (it
  turns gravity off and grants the fly ability at spawn). Both rows now say
  Floating; they said Standard for an hour, which would have had a snake on a
  paper airplane fall out of the sky.
  ⚠ **which one is the 1-2 enemy is yours to say.** The Cartesian one is the
  maths joke, the paper one is the aviation joke, and they are different
  creatures rather than two skins.

* Maryo world 1-2 needs moving platforms that move vertically down and up like an elevator. When they go OOB (far enough so they are off screen of the player in normal gameplay) they can teleport to the top / bottom of the screen to make an infinite elevator effect.

* The maryo world 1-2 needs to happen after she wins world 1-1, she doesn't just get to go there in the middle of 1-1 and come back. It should be a new level. When she beats world 1-2 we should cycle back to world 1-1 for the demo. World 1-2 also needs to be built out a lot more, its very plain, and there are no enemies. I would like to add the flying snakes on a plane as enemies in this world.

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

* In the hall of characters, the humanoid characters are all dramatically out of scale with each other. Alice and bob are great, but characters like the vikings, or jeff hinter render as tiny little characters and look out of place compared to the rest of the cast. The character art needs to be rescaled (probably at the generator level, not via some post-hoc fix) to balance the size of these characters so they make more sense appearing in the same game together. Note the player robot v3 is supposed to be chibi and short compared to other humanoids.

* ◐ **When maryo-dies the enemies seem to reset before the death animation is finished.** The level reset needs to happen all at once at a time that is easy to express in the game code. This might be a larger refactor if there is a structural problem here, and we need to avoid creating a monolith.
  * ⚠ **TRACED 2026-08-03, and the code does NOT currently do what the report describes — so this needs your re-check before it gets a refactor.** `ResetRoomFeaturesEvent` (what restores enemies, breakables and pickups) has exactly ONE production writer: `reset_sandbox`, reached only through `apply_room_replay_request_system` draining `RoomReplayRequested`. Mary-O emits that from `restart_level_after_death`, which returns early while `sequence.active()` — i.e. **after** the beat. So enemies should not reset early.
  * ⭐ **what DOES happen immediately is the PLAYER**: `death_respawn_player` resets her clusters, anim and combat on the fatal hit, and the death module's own doc says so — *"the engine respawns a dead player IMMEDIATELY, that is why her `Death` row was unreachable"*. The beat then holds her body and re-arms `death_anim_timer` every tick because that immediate respawn wipes it. **So the thing that resets mid-animation is her, not them** — and if what you saw was the world looking untouched while she died, that is the same root wearing different clothes.
  * ▢ so the "one reset at one time" question is still live and still worth the refactor, but the specific symptom wants confirming against HEAD first — the broken-brick report turned out to be already gone when you re-checked it.

* ~~We probably need an engine concept that allows actors to be dormant.~~ **BUILT, and then found half-wired 2026-08-05.** `features::ecs::dormancy` declares it the way your last clause asks: an actor with no `DormancyPolicy` is always awake, so "not inherent" is the default rather than an opt-out, and `DormancyPolicy::Never` exists so a character that must keep simulating says so where a reader finds it. The wake test is *near any OBSERVER*, never near "the player" — one player, four on a sofa and a remote peer are the same rule, which is your split-screen and netplay point. It sleeps the BRAIN and CLEARS the control frame (a sleeping actor with a stale `ActorControl` keeps walking, which is the exact symptom), and `Dormant` is recomputed every tick from positions so a rollback reproduces it with no memo to get wrong.
  * ⛔ **but only the SLOP was wired to it for a day.** You named the slop, so the slop got the policy — and Solid Snake, the other patrolling enemy in the same level with the same job, thought for the whole course. Fixed `ad43b63ba`. ⚠ **the test was defending the gap**: it asserted that ONLY the slop declares dormancy, with a strays check that would have failed the moment anyone wired the snake. It asserts the property now — every authored enemy declares whether it sleeps — and was probed red.
  * ◐ **Sanic adopted it too (`df269eaec`), and the second game is what made the seam's assumption visible.** A wake radius is a LEAD TIME wearing distance's clothes: your 720px in front of a Mary-O at 300px/s is 2.4s of warning, and the same 720px in front of Sanic at his 2000px/s super top speed is **0.36s** — a badnik snapping into motion in full view, worse than one that had been walking all along. His radius is 4800, derived from his own top speed for the same 2.4s of lead. ⭐ that a fixed radius silently encodes an assumption about how fast the OBSERVER moves is the argument for the policy eventually taking SECONDS rather than pixels; recorded at the constant, not acted on, because two games is not enough to change an engine seam's units.
  * ▢ **the content bosses and the hall cast still declare nothing.** Both are stationary or scripted, so the cost of leaving them is low and the right answer may well be `DormancyPolicy::Never` stated explicitly rather than silence — but nobody has decided, and silence currently reads the same as "always awake" whether or not that was chosen.

  * *(your words, kept)* We probably need an engine concept that allows actors to be dormant. This is important for maryo because ai slop will just walk off the edge of the level before she even gets to that part of the level, so we need to wake or sleep their brain depending on how close she is to them. This sort of optimization will likely be generally important for any game using the engine, although it's not something that should be inherent. There might be characters that don't go dormant off screen, this matters a lot for split screen or network multiplayer games. It also might matter in other cases. Not 100% sure how its elegantly expressed though.

* ~~In mary-o blocks that are used need a new texture so they are visually distinguishable. They also need a small animation (probably an in-code position nudge up and back into place) when they are hit.~~ **BOTH DONE.** A spent block wears `EntitySprite::SpentBlockTile` — its OWN inert texture rather than falling back to plain masonry, which would have hidden its history — chosen in `dress_power_blocks` from `SpentPowerBlocks` every frame rather than from the bonk EVENT, because that set is rollback state and art driven by the event would keep the used look through a rewind that undid the strike. The nudge is `BlockStruck`, emitted by the bonk and consumed by the render layer (`rendering/world.rs`): the block's own position is never moved, exactly as you guessed it should not be — moving it would lift a body standing on it.
  * ⭐ **and a related asymmetry fixed 2026-08-05**: a BRICK drew as the generic dark slab while the level's own solid surfaces drew `SolidTile`, the seamless brick pattern. Same `BlockKind::Solid`, two textures, decided by whether the block came from the IntGrid or from an entity. Bricks wear the masonry now (`c6a7034a3`).

* **[agent-found, next long run] The couch-input smash test fails only in a full-binary run.** `app_it::smash_in_the_host::a_keyboard_player_and_a_pad_player_drive_different_fighters` passes in isolation and fails when the whole `app_it` binary runs (measured both ways, 2026-08-02, and confirmed identical on a clean worktree at the previous commit — it is not a regression from the Mary-O geometry or contact-harm work). So it is order- or parallelism-dependent: something earlier in the binary leaves seat/pad state that this test then reads. That is a real defect and not merely a flaky test — a keyboard participant and a pad participant driving the same fighter is exactly the couch bug class that has bitten repeatedly, and the isolated pass is what makes it invisible. Worth finding what the shared state is rather than adding a serial-test attribute over it.

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
  ▢ **what is left, stated as the choice it is:**
  * the SNAKE's quad/box disagreement is an art-pipeline crop or a quad sized
    from the body (`enemy_quad_matches_its_box` ratchets it at 2.47x);
  * the SIZES themselves — the snake at 41 x 18 world and the slop at 28 x 18
    against Mary-O's 48 tall — are one number each, and both now live where
    stating a different one is a one-line edit rather than a hunt.
  ⭐ **the fix Mary-O already demonstrates, in the same crate**: state the world
  size you want and divide by the measured sheet, so a regen that moves the crop
  by a pixel does not silently resize the creature. Her own comment says why —
  *"a scale pinned to today's pixel count silently changes her height the first
  time a crop moves"* — and both enemies are pinned to today's pixel count.
  ▢ **what this does NOT decide**: what the numbers should BE. The instrument
  asserts no ratio on purpose — what counts as too big is Jon's call, and a limit
  written by a test would be a taxonomy nobody chose. What it does say is that
  the snake should be stated as a WIDTH (or a height with the sheet's aspect
  respected) and the slop should stop being a square.
