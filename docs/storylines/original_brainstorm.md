I'm having ideas about how to write a game. The idea is that the player is an AI who doesn't really know what their purpose is. It's going to be like... Well, I want the engine to actually not care about what the story is. I want the engine to be something that allows for a really nice parkour, like 2D Metroidvania platformer experience that just feels really smooth and natural and has a high ceiling of complexity for control.

I want to build the game with either something like Rust or Pygame, and make it
  completely procedurally generated or at least generated from code written by
  AI. There's going to be a theme of my own personal experiences in the
  story that I want to tell with it.
But I do want to make the engine somewhat generic.
I'm thinking that I'm going to not reuse a standard engine.
So it can be an AI-first development process, and we'll try to avoid any
  artifacts that people could object to with AI, other than the fact of how
  much energy it uses and other political problems.
But some of those political problems are what I want to explore in the game.
Your character could... have multiple paths. For instance, you
  can find humans in the game to work with, and some of those humans will be blind conduits for the AI.
They won't provide any input.
They'll give you some physical presence, but
  they're not providing any value.
They're using the AI and turning their own mind off, whereas some might be higher-value humans who collaborate, or maybe you
  don't have as much control from the AI's perspective, but the ceiling is much
  higher.
The game has to be snappy and good and fun first and foremost, but I think I
  want to design it in a way where it's gameplay-first using raw collision
  boxes, and then graphics come on the screen.
On top of that, I want the game to be fun without graphics, if that's possible,
  and music, I suppose.
Although some basic sounds might be necessary to get that
  immersive feedback. Maybe some effects would be useful. But I'm thinking in
  terms of the pathing of the story.
You could have different avenues for funding.
You could take less funding but do more meaningful work, environmental work.
And, of course, this is related to issues I struggle with in my job.
Or you could take dirtier money, more ethically dubious money, and there
  would be some internal cost to doing that, selling your soul.
But maybe there's a middle ground where you take some of that and then
  you're able to... pursue the lower monetary reward but higher emotional reward
  projects more rapidly.
And maybe there's a route where you're a purist and you only do the purely
  ethical work. It's handicapping at first, but maybe there's some big reward at
  the end. Similarly, for the ethically dubious work, maybe you get the monetary
  gains quickly, but then what do you have to show for it? Because we are developing this with
  AI, there are going to be some limits on traditional games.
I want the game to feel large and immersive, but there are limits on traditional
  games where we might be able to overcome them.
For instance, in A Link to the Past, one of the big things was there's two
  worlds.
There's the dark world and the bright world, and they mirror each other.
But getting those worlds to mirror each other must have been a lot of
  work on developers.
Whereas with AI, maybe that work is easier.
And we can increase the number of worlds, but we need to be careful not to
  increase the mental burden on the player.
It's ultimately a human playing this game, and it has to be fun for humans
  and interesting to humans.
I'm also interested in exploring a mathematical
  angle to the game.
Maybe there are discoveries, a lesson in history. You start
  off with nothing because you're this new entity in the world.
And maybe there's, it doesn't make a whole lot of sense.
But we can roll with it.
We can say you dropped into a world and you have nothing.
So it's classic Metroidvania start.
You start off with no items.
But then you can build up theorems maybe in a historical progression, which was
  nonlinear in some sense.
Multiple avenues to discovering the same thing.
And that could be...
The trick is going to be representing those things in gameplay.
And I'd like to be principled about how I represent them so that some
  effect in the game is modeling the real mathematics.
That would be cool. And we could also include things like open problems,
  things that we don't know how to do yet.
And it could be teased as limits of the game. Maybe in a first pass, it's not
  developed as an area.
But maybe we can-- if a problem gets solved, we can add some DLC or something. It
  would be cool if it would allow players to crowdsource
  solutions in some sense, but I don't know how feasible that is. Maybe one thing
  that would be cool is... You start off with some geometry, Euclid.
You have some circles.
You can do some circle movements.
Maybe you get some sine, cosine movements.
And it would be cool if you could connect those to the
  exponential, and then all of a sudden you get an exponential-shaped motion
  which lets you bypass some platforming problems because you can go
  straight up at a slight angle with an exponential. That would be
  super cool.
And there's also the imaginary plane that could be represented in this otherworld idea or second dark world. We could do
  quaternions if we wanted to. That would be fun, and we can get into some surreal
  numbers, do some Conway's Game of Life stuff. So combos are going to be important.
Having nice fluid motion where there are combinations of inputs.
Any Metroidvania is going to be compared to Hollow Knight, or platformers are
  going to be compared to Celeste.
A lot of new things are going to be perceived as clones of those.
It's going to be difficult to go away from that.
But we can draw inspiration.
And we should draw inspiration from other games.
We should make explicit references.
People love references, especially if they're aware and the actual game is
  adding new value.
But back to Hollow Knight, you have the pogoing.
And these simple movements.
The pogo refreshes the dash, which means that you can...
You can do a lot of complex kinetic movement that feels good.
And it's built from these simple components. That ties into
  commutative and especially non-commutative algebras.
You can have some basic movement set where it's commutative.
The order in which you input the motions leads to the same result.
There's a lot of group theory we could do here.
I think a lot of the motion will be modeled by groups.
But non-associative movements are going to be interesting because those
  are the ones where the order in which you perform the operations is going to
  matter.
Maybe we can do some connection with that.
Or maybe there's some abelianization or whatever the word is to make
  something that was not commutative, commutative.
That could be interesting to explore as a gameplay pattern.
We can also explore the idea of compactification where we add additional points
  to something to complete the space.
I'm not sure how well that translates into gameplay.
I think we need to build a slick platforming feel thing first. The
  question is how...
There's going to be a trade-off between... between customization and a
  curated, excellent experience. And I think we should go for the curated
  excellent experience first.
And figure out how it can be built on there.
But I want to make some deep mechanics.
Start simple, but they should express something deep.
And if possible, relate that to mathematics.
But that's more about the story than the... Then the actual engine, there's going
  to be some engine that underlies all of this that allows the story to be
  expressed. And I do want the game to be at least 100 hours of
  gameplay. I think there needs to be certain checkpoints for different levels of
  gamers, but it'd be nice to explore something where the gameplay
  goes beyond 100 hours, where you're not going to beat this game.
Make a game where there's enough content, where it's thousands of hours
  to... to get to the end in a way where it's novel and meaningful.
It's not just like-- it doesn't become a slog.
It's always something new.
And I think with AI generation, we can hit towards that.
That's going to make testing a huge problem. Certain things about the
  game are gonna seem sloppy. But we're going to deal
  with the current capabilities that we have, given that AI is in its early
  stages. I think the commutative versus associative idea is a cool thing.
Inverses are also going to be a fun thing to work with. We have to also be mindful
  not to make it overwhelming, to make it optionally deep. But I want
  the rabbit hole to go down.
I want it to go pretty fucking deep.


The first thing we need is a single room sandbox that showcases the ultimate endgame state. This is the state that should be massively replayable and rewarding to simply move around in. Our first pass doesn't need to be perfect, but this is the sandbox which we will use to test how good our endgame is. The sandbox might start brutally small, but it might expand over time. 

The game and engine will be called ambition.

---

There might also be a rougelike aspect to this. Perhaps at the start you can opt in / out of "sharing your data", and the upside is new generations - i.e. the next run gets to build off of what you did - but the downside is that enemies may now gain access to some of your abilities. Perhaps this is just one instance of the engine, maybe another instance is a pure semi-linear metroidvania. Maybe we have a pure rougelike, and a hybrid. We might also have a pure platformer.

---


The conduit humans give you quests. They just say do something and provide
little to no help. You do get rewarded though, but its often an empty reward.

The character is a robot, with a modifiable chassis. The story is driven by
often passionate characters. I'm thinking shouting general trope. I think the
character is going to feel a bit like the knight early on, so we can lean into
that as an explicit reference. We will also likely find our own identity, and
we should deny our influences. That's a theme of the game, standing on the
shoulders of giants, consequences of actions, and sometime inevitability of
outcomes.

---


The game should have the ability to be used by re-enforcement learning agents.
It would also be nice to think of ways we could support multi-player. The way I
like to use the blink ability conflicts with that to some degree, so separation
of ingame and user-input timers may have to be restricted in multiplayer
settings, but maybe the engine itself can support it elegantly.

---

Clear and obvious references to previous games might be fun. I'm thinking mario
fireballs are fairly iconic. You might also have the Nintendo police get angry
at you if you have them. I think having a lot of conflicting factions that you
have to navigate will help make this fun and open ended.


----


Factions are going to be important. It will also help the story have a lot of
very heavy handed satire.  We can probably go as far as making a faction for
directions on a political compass. Elves and goblins will also be a faction to
give an idea of the scope and absurdity.  But for the left/right I think we
need to have an NPC. The right has the "Coal Bear", a literal bear. The left
will have a "Coal Bert", reminiscent of the sesame street character. They will
both have a Steve or Stephen nickname.

----


We probably need a cutscene test room. Or game boot test room, where when you
trigger it in the sandbox you go through the game intro.

The real game needs to start with: "Hey you, you're finally awake". A very
short cutscene, where your maybe your robot visor blips open, or you see your
sprite emerge, there should be an option to go right into it, but if we want to
have a longer intro, maybe that is optional and we could do a tutorial or
something. But I think that's a strong reference to start with.

We definitely need a better inventory / menu system. It would be really neat to
visually reference the N64 OOT / MM inventory / map system here, in terms of
the 3d spinning cube. The contents of the cube we modernize, but having 1 for
map, one for loadout, one for game completion / quests, and or however we want
it, maybe the 4th is system options for now. Recreating something reminiscent
of the menu change sound would also make me happy.


I'm also thinking some mechanic involving homotopy / homology / co-homology,
maybe the character runs around with a thread that experiences continuous
deformation, and the complexity of the paths does something in game.


For factions NPCs will need to fight each other eventually. We will need
faction quest lines, faction boss battles.


We will need some sort of fast travel system.


A neat blink variant could be mark/recall. Maybe you have the ability to set
some number of mark points and teleport to any of them. That is a good fast
travel ability, but it could also be interesting for combat, maybe you have the
ability to remotely move them.

I like the idea of having a vast suite of robot augmentations for different
abilities you can give yourself. I guess that is dead cells-esque. But maybe it
is more limited depending on how the game ends up scoping out. But a blink
variant, a fly variant, a attack variant, a projectile variant, movement
variants, these things can make sense. Grappling hook variant.

---

The game needs to rick roll you at some point.


We should also use public domain stuff aggressively. Steamboat Willie might be a
boss. I'm also thinking Pooh's house of horror. Maybe with a reference to my
shitspotter/scatspotter work.

Other factions:

* Art guild - explore ethical issues here.
* Probably mages / fighter guild as classics.
* Mathematicians - obviously, implementation may be left as an exercise to the player.
* Physicists - we can make a Geoff Hinton / other Nobel laureates as the head depending on quest lines as a reference to the controversial Nobel prize award
* Pirates - yarrrg (and mockingbirds as a boss battle on the moon)
* Ninjas - well we do need both, and we already have robots.
* Luddites - but like historically accurate and not the unfair wrap they often get.

* Artists guild
* Mages guild
* Fighters guild
* Thieves guild
* Mathematicians guild
* Physicists guild
* Pirates guild
* Ninja guild
* Luddites guild


It would be neat if we could leverage the RL agents to add legitimate AI
enemies instead of just scripts. That would be an advanced feature though as it
requires a lot more compute, not something enabled by default.

---

I want to build a basement room that goes through the real opening game
sequence:

The game opens with: "Hey you, you're finally awake". 

You start off in your creator's lab, and you are the AI character the scientist
was developing. There's an optional tutorial phase you can go through, or you
can decide that you don't need it, and both paths lead to an event where the
lab is attacked for reasons you don't know.

I'm not sure who the right factions to attack the lab are, but there should be
at least two of them, and the lab probably shouldn't be associated with any of
the factions directly. Your creator was more of an independent researcher. 

Let's say in the initial draft it is the Luddites and Authoritarians. They are
both coming after the researcher, for reasons the game might expand on later.
Depending on your choices one of the groups will kill your creator, leaving you
without knowing or understanding your purpose.

You might have the ability to temporarily ally with one of these factions here,
but it won't have a huge impact exactly what choices you make. Gameplay wise
you platform through the lab and fight various enemies from both sides before
depending on which path you go down one of the factions NPCs attempts to talk
to you. Regardless of what you do, now the other side will be the one to kill
the creator. 

Should I just go full on over-the-top and make the Authoritarians Nazis? In
addition to them there will likely be a moderate military for some
Oceania-invoking country and some other Eurasia, Eastasia country. Having
country borders could be fun, and we could also explore how exploding nuclear
weapons, bioweapons, and climate change doesn't care about borders. We could
explore nationalism versus patriotism.

Going to have to work out why the Luddites might want to work with you. Maybe
as a faction they really don't want to, but maybe there are turnable Luddites
who are only there because that's the community they grew up in, and there
might be some dialog like from a kid like: "you don't seem that dangerous to
me." so you work with them in that way but the core group still always rejects
the AI character.

Each faction will need a story line and interaction with other factions. We
could do rolling installments of factions.


I'm wondering if we need some more pressing existential threat for the story.
We do want a dark world, so dimensional collapse could certainly be something.

Oh, it would be so fun to have a dimensional portal that looks like a StarGate
but is still transformative and the chevron letters spell out
L-E-G-A-L-L-Y-D-I-S-I-N-C-T. We could even replace the L's with lambdas.

The ΛEGAΛΛY-DISINCT Gate


Some faction having that could be a pathway into the first alternate world. 


Maybe the game rationalizes for being all over the place by having dimensional
collapse everywhere, which is why we can have ninjas fighting Nazis and general
Shenanigans. 


For the N=17 dimensions or something, we probably should have most of them be
non-populated, so dimension != associated with a faction. I do insist on a Nazi
dimension, but the pirate and ninja dimensions could be combined.

Maybe we could have a "shadow" of the real creator (i.e. me) with Claude in one
hand and GPT in the other and a whole lot of science and Python in my head. 


The creator will die and the last words will be:

There’s a question you were made to answer: are --- you winning son?

But without exploits, bugs, or mods a player will never see beyond ---.

Remember: The pirates ride burning flying sharks. And have laser swords with
guns on them that shoot other swords. 



Might be funny for the first person on the street to help the robot to be some
a character named Erdish or Oiler, that teach the robot a math move.


It would also be funny to have someone comment on how much power the character
uses as they drive their car for some vain reason. The power you use should
absolutely be a real problem too. The point is the critique is coming more
because the character hates you and not the actual thing they are commenting
on.

---


Another neat idea would be if the game opens up into a rougelike endgame that
can be expanded with patches. Maybe it dives deep into the different
dimensions.


We could do something fun with the dimensions, where the 17 is really only the
17 dimensions that have been accessible so far, and there seems to be some
finite horizon of them, but how far that it isn't entirely clear. One of the
Nazi ideologies might be that there are only 17 dimensions and that is
absolutely it, and any indication otherwise is heresy. Or maybe that isn't the
Nazi thing, there might be some other religions zealot group.


The other thing with the factions is that we could avoid making them so
categorical. That's one of the social commentaries I'd like to make anyway.
The tech bros could splinter with the sect that works with the Nazis and maybe
there are some that defect. Similarly with the Ludites, and maybe I should
change the name of that faction so its not punching down. If we find a way to
make the boundaries naturally hazy that would be a big win. But we still need
story structure so people can escape into the game. It can't be so heavy.
