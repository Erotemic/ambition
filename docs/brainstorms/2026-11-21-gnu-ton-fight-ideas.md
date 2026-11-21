# GNU-ton fight: ideas past the polish pass

Status: brainstorm, nothing here is planned. The 2026-11-21 polish pass
(GNU-ton fight polish, task 171) gave the fight a wider hall, an upright
GNU2 that escorts GNU-ton (unhit, no bar of its own, falls when he falls),
stacked boss bars that cannot overlap, giants with their arms on, and apple
trees on real floating islands whose apples ripen, fall and grow again.
These are the larger ideas that pass left alone.

## Jon's idea: grow to their size through a scaling pair

Jon, 2026-11-21: "maybe we could put the scaling portals to use in the
fight, so the player can grow large enough to fight the gnus themselves?"

The engine already has what this needs. A portal pair whose mouths have
different lengths scales what crosses it: `scale_through` in
`crates/ambition_portal2d/src/pieces.rs` is "what a length at `enter` is at
`exit`", and a body that crosses is mapped by the pair's similarity
(`Sim2::through`). A 64 px mouth paired with a 640 px mouth makes the player
ten times bigger on the way through, and ten times smaller on the way back.

How it could run as a fight:

- **The giants are the stage at first.** As now: you stand on the
  shoulders of a giant and strike the fists while they are stuck.
- **At his enrage he opens a big pair to bring GNU2 in.** A small mouth
  that pairs with it is somewhere you can reach (on Newton's island, say).
  Step through and you come out of the big mouth at giant size.
- **Giant-size, the gnus are fightable.** Then and only then the escort
  rule lifts: GNU2 takes hits from a body its size. The rule can be about
  scale, not about phases: an escort takes hits from a body at least half
  its height. That keeps "no hit lands on it" true for the normal-size
  player without a phase flag.
- **The scholar becomes the small one.** At giant size GNU-ton is a figure
  on a gnu's back the size of your hand. He can no longer be struck with a
  sword; the way to win is to knock him off his mount. This flips the
  first half of the fight.
- **Time limit by design, not by timer.** The big mouth closes when his
  phase ends, or when you are hit; you are pushed back through the small
  one (a shrink you can see).

What needs checking before it is planned: whether the movement kernel,
the camera and the room collision hold up at 10x body size (tile-sized
features become ankle-high bumps), whether the shared combat volumes scale
with the body (`ambition.boss.scale` does this for bosses; the player has
no such scale yet), and what the camera zone does when the subject is
640 px tall.

## Other ideas

1. **Newton's island, as a payoff.** The middle island is only reached
   from the giant's back ("standing on the shoulders of giants"). Put
   something on it worth the climb: an apple that is not like the others
   (a golden one, or one that falls up), or a short line from GNU-ton when
   you land there ("If I have seen further...").
2. **The apples are ammunition.** The trees' apples are real held items.
   A thrown apple could stun the scholar for a beat, or, thrown into one of
   his own catching pairs, come out over his head. The rain is his move;
   the trees could be yours.
3. **Gravity apples in the hall.** The gravity works already has a gravity
   apple enemy. One or two in the rain would turn the room's pull for a
   moment, and the islands' apples would fall the other way. It ties the
   fight back to the works it ends.
4. **A legible enrage beat.** When he calls GNU2 the hall could dim, the
   cell's wall glow, and the pair open with a sound you hear before you
   see it. The call is the fight's turn and is currently quiet.
5. **The giants react to each other.** GNU2 coming through the pair could
   stop GNU-ton's gnu for a beat: both heads turn. Two giants that do not
   see each other read as two props.
6. **A slower last phase.** At 20 percent he could dismount and fight from
   the ground beside two giants that stand still and guard him. The arena
   would read as his court, and the end of the fight would be his alone.
7. **A falling ending.** When he falls, both giants could kneel (the dead
   pose with arms on) instead of only dying. The escort falling with him is
   the mechanic; kneeling is how it reads.
