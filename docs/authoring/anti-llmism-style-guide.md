---
id: anti-llmism-style-guide
status: current
aliases:
  - anti-LLMism
  - LLM prose patterns
  - dialogue style guide
related_docs:
  - docs/concepts/agent-native-authoring.md
last_verified: 2026-09-02
---

# Anti-LLMism Style Guide

This document defines writing patterns that should be rejected when authoring or reviewing Ambition dialogue and other player-facing prose.

The goal is not to make dialogue generically casual, fragmented, plain, or "human sounding." Those substitutions create another shared house voice.

The goal is to identify recurring LLM-generated rhetorical machinery so it can be recognized and rejected even when the exact wording changes.

A rewrite does not fix a violation if it preserves the same underlying formula with different words.

---

# Scope

These rules apply primarily to player-facing authored dialogue:

* Yarn Spinner dialogue;
* character barks;
* banter;
* cutscene dialogue;
* fallback dialogue stored in character definitions;
* other text intended to be spoken by a character.

For dialogue audits, review the actual dialogue text.

Comments, implementation notes, character-development notes, data descriptions, prompts, and other non-player-facing material do not need to follow the dialogue-specific rules merely because they are stored next to dialogue.

The general rhetorical rules may also be applied to documentation, papers, planning prose, and other authored text when appropriate.

---

# Intentional exceptions

Some characters may intentionally speak in a style that violates this guide.

Examples include characters whose characterization explicitly involves:

* LLM-generated language;
* startup jargon;
* synthetic motivational prose;
* grandiose AI rhetoric;
* generated corporate language;
* deliberate parody of these styles.

For those characters, violations may be retained or exaggerated when they serve the joke or characterization.

The exception is character-specific.

Do not allow the exempt character's voice to leak into unrelated characters.

A character such as Chadwick may deliberately use polished abstractions, slogans, false profundity, startup phrasing, or conspicuous LLMisms if that is part of the character.

Literal mathematical negation, necessary technical distinctions, direct quotations, and domain terminology are also allowed when the subject requires them.

---

# Hard banned rhetorical words

Do not use these words as rhetorical emphasis:

* `matters`
* `silently`
* `quietly`
* `unusually`

Do not replace them with a synonym that performs the same empty function.

State the concrete behavior or consequence instead.

For example, replacing:

> The timing matters.

with:

> The timing is important.

does not fix the problem.

State what changes when the timing is wrong.

---

# Hard banned phrases

Do not use:

* "The X is worth stating."
* "The X is worth saying out loud."
* "The X is worth clarifying."
* routine variants of "The X is worth..."
* "X bites."
* "That's <noun> with <adjective>."
* `keeps count`

The ban on `keeps count` includes figurative constructions such as:

* "the sea keeps count";
* "history keeps count";
* "the machine keeps count";
* "someone always keeps count."

Literal code or data structures that maintain a count are outside the dialogue rule.

---

# Banned contrast template

Do not use the rhetorical formula:

`X is not Y; it is Z.`

This includes routine variants:

* "This is X, not Y."
* "The point is not X but Y."
* "Our contribution is not X; rather, it is Y."
* "This should not be understood as X, but as Y."
* "I'm not X. I'm Y."
* "It isn't X. It's Y."
* "That's not X. That's Y."
* "We don't X. We Y."
* "You didn't X. You Y."
* "No X. Just Y."

Changing punctuation does not avoid the rule.

The prohibition concerns the rhetorical reversal.

Literal negation is allowed when the fact itself requires negation.

---

# Banned slogan style

Do not use:

* taglines;
* dramatic one-line morals;
* compact philosophical declarations added for weight;
* punchy oppositions written for cadence;
* sentences designed primarily to be quotable;
* metaphorical names for ordinary actions when literal language is available;
* self-congratulatory claims about intelligence, rigor, scale, precision, novelty, or sophistication unless characterization specifically calls for boasting;
* sentences announcing what something "really" means;
* final clauses added only to make a line feel profound or finished.

A deliberately pompous or synthetic character may violate this section as part of the voice.

---

# Banned note-to-self prose

Do not leave future-work or author-facing prose in player-facing dialogue.

Examples:

* "should be repeated before submission";
* "needs a final refresh";
* "final reporting should";
* "the appendix should eventually";
* "when the inventory adapter lands";
* "this will be updated once";
* "the final version should";
* "the camera override will land later."

If a diagnostic character needs to discuss an unavailable capability, state the current state.

Do not narrate the development plan.

---

# Banned vague methodology language

Do not use an abstract description when the concrete thing can be named.

Examples of suspect prose:

* "the missingness mechanism is structured";
* "coordination-tooling provenance";
* unexplained "interaction categories";
* vague "presentation choices";
* unspecified references to "the process";
* unspecified references to "the system";
* generic claims that some distinction `matters`.

Name the file, action, object, subsystem, observation, failure, cost, or consequence.

---

# `presentation`

Use `presentation` sparingly.

Keep it when it names an actual technical object, mathematical representation, presentation layer, or other concrete concept.

Do not use it merely to discuss how information is arranged for the reader.

---

# Dialogue-specific prohibited formulas

The following rules describe semantic patterns.

A line violates them even if none of the exact example words appear.

---

## Proposition -> interpretation

Do not state a fact and immediately explain what the listener should conclude from it.

Formula:

`[fact / event / observation]. [interpretation].`

Common versions:

* `[event]. That proves X.`
* `[fact]. That means X.`
* `[observation]. Which tells you X.`
* `[event]. That's why X.`
* `[fact]. The point is X.`
* `[fact]. The trick is X.`

Also reject implicit versions.

Example:

> The leak happened before I arrived. Getting in proved it.

The second sentence packages the first into a neat conclusion.

Characters may leave the inference unstated.

---

## Instruction -> principle

Do not turn practical advice into a miniature lesson.

Formula:

`[instruction]. [general principle explaining it].`

Example:

> Read the surface first. Its geometry tells you which routes are cheap.

If the player needs:

> Go left.

then let the character say:

> Go left.

Add an explanation only when the scene actually requires one.

---

## Miniature syllogism

Do not make dialogue read like a compressed proof.

Formula:

`[premise]. [premise]. [conclusion].`

or:

`[evidence]. [conclusion derived from evidence].`

Common markers include:

* `therefore`;
* `which means`;
* `that means`;
* `which proves`;
* `that proves`;
* `that tells me`;
* `that tells you`;
* `so obviously`;
* `if X, then Y`.

Characters can reason.

The failure occurs when their speech repeatedly packages reasoning into polished, self-contained logical demonstrations.

---

## Setup -> semantic closure

Do not automatically give an idea a perfectly fitted ending.

Formula:

`[setup]. [sentence or clause that completes the idea].`

Example:

> The manifest has precedence here. If reality disagrees, I file a correction.

The second sentence exists to finish the rhetorical object.

Related forms include:

* observation -> payoff;
* statement -> clever consequence;
* setup -> reversal;
* accusation -> distilled verdict;
* literal statement -> figurative conclusion.

A line may end before its idea has been rhetorically completed.

---

## Concrete statement -> abstract summary

Do not state something plainly and then restate it in cleaner conceptual language.

Formula:

`[concrete statement]. [abstract reformulation].`

Example shape:

> He took the east door twice. He prefers predictable exits.

If the first sentence already performs the conversational job, do not attach a summary merely to demonstrate insight.

---

## Local event -> general principle

Do not extract a philosophy, rule, moral, or universal statement from a specific event.

Formula:

`[specific event] -> [general truth]`

Common warning forms:

* "That's how power works."
* "Every empire does this."
* "Nothing perfect stays perfect."
* "A good hunter always..."
* "The real danger is..."
* "That's the nature of..."
* "Everything breaks when..."
* "There is no X without Y."

A character can respond to what happened without converting it into a lesson.

---

## Character premise -> themed metaphor

Do not generate characterization by translating ordinary situations into vocabulary associated with the character's concept.

Formula:

`[ordinary event] -> [figurative language from character theme]`

Common failures:

* clerk -> forms, manifests, filing, corrections;
* mathematician -> geometry, proofs, optimization;
* programmer -> bugs, exceptions, recursion;
* hacker -> vulnerabilities, permissions, exploits;
* chef -> recipes, ingredients;
* musician -> rhythm, harmony;
* scientist -> experiments, hypotheses;
* merchant -> prices and debts;
* pirate -> navigation metaphors for unrelated subjects.

Themed vocabulary is fine when it refers literally to the thing being discussed.

A pirate can talk about a mast.

A mathematician can talk about an actual proof.

The rule prohibits using a profession or gimmick as a metaphor generator.

---

## Character concept demonstration

Do not write lines whose primary purpose is to show the audience what the character's gimmick is.

Formula:

`[character premise] + [clever sentence demonstrating premise]`

A useful test:

Would this line work just as well if pasted into a character sheet under:

> Sample quote

If so, inspect it closely.

Dialogue should usually depend on the scene.

---

## Character explains own gimmick

Characters should not routinely explain:

* what kind of character they are;
* what their behavior represents;
* what their gimmick means;
* why their powers fit their personality;
* what narrative function they occupy.

Formula:

`[behavior] + [conceptual explanation of behavior]`

Characters may explain themselves when the scene gives them a reason: an introduction, confession, interrogation, argument, boast, lie, or direct question.

---

## Designer ontology in character speech

Do not make characters describe the world using the clean system architecture an author or developer would use.

Formula:

`[world-system term] + [high-level explanation of underlying mechanics]`

Example:

> The Kernel is integrating the world badly. I'm correcting its error.

This sounds like an implementation description spoken by a resident of the world.

Characters may know local terms such as `Kernel`.

They do not automatically know or verbalize the designer's conceptual model of what the Kernel is doing.

---

## Abstract noun as actor

Be suspicious when abstractions become agents.

Examples:

* reality disagrees;
* geometry tells you;
* precision rewards;
* chaos remembers;
* history demands;
* truth waits;
* order prefers;
* failure teaches;
* possibility closes;
* logic catches up;
* the sea keeps count.

Formula:

`[abstract concept] + [intentional or human-like action]`

This construction frequently turns a local statement into an aphorism.

Literal supernatural entities are exempt when the setting establishes them as entities.

---

## Epigram construction

Do not default to maxim-shaped dialogue.

High-risk formulas include:

* `A X is Y.`
* `X is what happens when Y.`
* `Every X eventually Y.`
* `Nothing X survives Y.`
* `There is no X without Y.`
* `The real X is Y.`
* `X always remembers Y.`
* `X rewards Y.`
* `X punishes Y.`
* `X is Y wearing Z.`
* `X is Y pretending to be Z.`
* `X is just Y with Z.`

These structures are suspect even when the sentence is competent prose.

---

## Definition as characterization

Do not make characters repeatedly define concepts.

Formula:

`X is [compact conceptual definition].`

Common targets:

* fear;
* power;
* victory;
* maps;
* proofs;
* trust;
* time;
* failure;
* hunting;
* war;
* friendship.

Definitions are allowed when someone is actually defining something.

Do not use the form as a shortcut to wisdom or personality.

---

## Automatic cleverness

Do not search for a:

* metaphor;
* inversion;
* paradox;
* pun;
* aphorism;
* conceptual twist;
* ironic definition;
* double meaning;
* elegant analogy;

merely because a literal response seems plain.

Plain dialogue is allowed.

---

## Automatic punchline

Do not assume an utterance needs a final clever clause.

Formula:

`[functional dialogue] + [extra clever ending]`

The extra clause may be:

* a metaphor;
* an inversion;
* a joke;
* a philosophical observation;
* a reference to the character gimmick;
* a polished final image.

Delete it when the conversational action is already complete.

---

## Automatic escalation

Do not enlarge the conceptual scale of a statement merely for rhetorical force.

Common transformations:

* problem in one room -> problem with reality;
* missed attack -> statement about failure;
* damaged machine -> statement about order;
* disagreement -> statement about truth;
* bad route -> statement about geometry;
* personal mistake -> statement about human nature.

Keep the scale of the language tied to what happened.

A grandiose character may intentionally overstate things, but the exaggeration should come from that character's behavior rather than a generic narrator's search for significance.

---

## Compressed multi-purpose line

Do not optimize every line to simultaneously provide:

* characterization;
* exposition;
* worldbuilding;
* a joke;
* thematic commentary;
* a plot fact;
* a gameplay hint;
* a memorable ending.

A line may accomplish one small thing.

---

## Conversational over-explanation

Do not state an observable fact and then explain an implication everyone present can already infer.

Formula:

`[observable thing]. [obvious implication].`

Characters can assume shared context.

---

## Reader-facing exposition

Characters should speak to each other, not to an outside reader who needs the lore unpacked.

Warning signs:

* explaining an event both speakers witnessed;
* naming a relationship both speakers know;
* explaining a local custom to another local;
* summarizing prior events to someone who was present;
* describing what an object does while both people are using it;
* completing obvious causal connections for the audience.

Exposition needs an in-scene reason.

---

## Perfect self-knowledge

Do not give characters clean analytical language for their own:

* motives;
* flaws;
* coping mechanisms;
* relationships;
* psychological state;
* narrative arc.

Characters may misunderstand themselves, dodge the issue, lie, answer only part of a question, or have no concise explanation.

Do not turn author notes into dialogue.

---

## Theme announcement

Do not make characters state the theme of their scene.

Formula:

`[event] -> [speaker states what event means thematically]`

The scene can communicate an idea without a character explaining the idea.

---

## Context-independent quotability

Treat a line as suspicious when it loses little or nothing outside its scene.

Ask whether it could appear unchanged on:

* a character poster;
* a quote card;
* a loading screen;
* a promotional image;
* a character profile.

This is a heuristic rather than an absolute ban.

Good dialogue can be memorable.

A script where unrelated characters constantly emit standalone quotes has a shared-style problem.

---

# Voice and register constraints

Removing LLMisms must not flatten the cast.

---

## Banned register convergence

Do not make unrelated characters converge on the same:

* formality;
* sentence length;
* vocabulary;
* grammatical completeness;
* amount of slang;
* conversational confidence;
* explanatory ability;
* abstraction level.

A cast should not all speak in the same competent middle register.

---

## Banned default conversationalization

Do not repair polished LLM prose by mechanically adding:

* "yeah";
* "nope";
* "hey";
* "come on";
* "seriously";
* contractions;
* fragments;
* slang;
* dropped subjects.

That produces a different artificial uniformity.

Casual speech belongs to some characters.

Formal speech belongs to others.

---

## Banned uniform formality

Do not assume formal speech is itself an LLMism.

Some characters should be:

* pompous;
* ceremonial;
* academic;
* archaic;
* bureaucratic;
* excessively polite;
* theatrical;
* grandiose.

PCA, for example, can be formal, arrogant, self-aggrandizing, and absurdly confident in its perfection.

The failure would be making PCA formal in the same explanatory voice as every scientist, clerk, or narrator.

---

## Banned uniform informality

Likewise, do not make every repair:

* shorter;
* slangier;
* choppier;
* more conversational;
* less grammatical.

That erases characters whose voice should be elaborate, formal, officious, theatrical, or pompous.

---

## Banned uniform communicative competence

Do not make every character equally good at explaining themselves.

Characters may:

* give poor directions;
* assume the listener knows something;
* ramble;
* repeat themselves;
* get distracted;
* answer the wrong part of a question;
* refuse to explain;
* overstate;
* understate;
* use unfamiliar terminology;
* leave implications hanging;
* become fixated on some small detail.

Do not optimize every speaker for information transfer.

---

## Character traits should change behavior, not just vocabulary

A character trait should affect what the character does in conversation.

Examples:

A pompous character may:

* boast;
* condescend;
* exaggerate achievements;
* refuse to admit uncertainty;
* treat ordinary success as confirmation of greatness;
* blame mistakes on others;
* use unnecessary ceremony.

A patient predator may:

* wait for long periods;
* repeat an observation;
* fixate on a route;
* show no urgency;
* remember a victim's habits.

A bureaucrat may:

* refuse to proceed without a form;
* care about a wrong field;
* send someone back to another desk;
* enforce a procedure at an inconvenient time.

A pirate may:

* boast;
* complain about rations;
* threaten someone;
* refer to an actual ship, cove, storm, haul, wound, crew member, or fight;
* exaggerate personal exploits;
* use dialect associated with that specific pirate voice.

Do not implement these traits primarily by converting unrelated events into themed metaphors.

---

## Preserve expressive character behavior

An anti-LLM rewrite should not strip away:

* repetition;
* strange noises;
* odd phrasing;
* emotional overreaction;
* dialect;
* boasting;
* petty grievances;
* physical business;
* fixation;
* childish enthusiasm;
* panic;
* theatricality;
* deliberate verbosity;
* deliberate terseness.

These can distinguish a speaker.

Do not confuse polish reduction with personality reduction.

Puppy Slug, for example, may remain highly expressive. Removing strange or enthusiastic behavior merely because it is elaborate can damage the character.

---

## Repetition can be characterization

Do not automatically remove repetition.

Compare a deliberately patient or obsessive voice:

> He'll come around that corner again. Just watching and waiting waiting waiting...

The repetition conveys behavior.

It does not exist to summarize a general truth.

Repetition should belong to the speaker rather than serving as a generic dramatic device.

---

## Preserve pirate flavor

Pirate characters should not be normalized into neutral modern conversational prose.

Where appropriate, preserve:

* maritime vocabulary;
* pirate dialect;
* boasts;
* threats;
* insults;
* complaints about food, drink, ships, treasure, weather, crew, wounds, or rivals;
* colorful exaggeration;
* rough social behavior.

Do not turn pirate flavor into a rule that every pirate line requires a nautical metaphor.

A pirate can say:

> Get off my chair.

without converting the chair into a ship.

---

# Structural cadence failures

---

## Symmetric sentence engineering

Be suspicious of sentences built for elegant balance.

Common forms:

`X does A; Y does B.`

`Some X. Others Y.`

`You bring X. I bring Y.`

`First X. Then Y.`

`More X, less Y.`

`No X. No Y. Just Z.`

Parallel structure is valid language.

Repeated use for dramatic cadence across unrelated characters is the failure.

---

## Three-beat rhetoric

Do not default to three short escalating units.

Examples:

`X. Y. Z.`

`No X. No Y. No Z.`

`First X. Then Y. Finally Z.`

Lists of three real objects are fine.

Do not manufacture three beats because they sound complete.

---

## Fragment simulation

Do not imitate human dialogue by mechanically inserting:

* fragments;
* ellipses;
* fake stammers;
* dashes;
* filler;
* arbitrary interruptions;
* broken grammar;
* random slang.

Fragments and interruptions are allowed when they belong to the character or scene.

They are not a generic antidote to LLM prose.

---

## Replacement-template behavior

When a line violates a rule, remove the rhetorical operation.

Do not merely transform:

`X is not Y. It is Z.`

into:

`X does Y. That's the difference.`

or:

`X does Y. That's the trick.`

or:

`X does Y. That's the point.`

or:

`X does Y. That's the job.`

or:

`X happened. Which tells you Y.`

A syntactic rewrite that preserves the same rhetorical mechanism is still a violation.

---

# High-risk phrases

These are not all hard bans, but their presence should trigger review:

* "that's the point";
* "that's the trick";
* "that's the difference";
* "that's the job";
* "that's the whole point";
* "that's how";
* "that's why";
* "the real X";
* "the true X";
* "the thing about X";
* "the nature of X";
* "what X really means";
* "which means";
* "which tells you";
* "which proves";
* "that proves";
* "the lesson";
* "in the end";
* "at the end of the day";
* "here's the thing";
* "precisely";
* "exactly";
* "always";
* "never";
* "everything";
* "nothing";
* "reality";
* "truth";
* "chaos";
* "order";
* "precision";
* "possibility";
* "purpose";
* "meaning";
* "structure";
* "system";
* "process";
* "geometry";
* "proof";
* "logic".

The word itself may be necessary.

Inspect the rhetorical function.

---

# Review procedure

For every proposed or existing line, perform a rejection pass.

Do not ask only:

> Does this sound good?

---

## 1. Identify what the speaker is doing

Examples:

* answering;
* asking;
* refusing;
* threatening;
* warning;
* requesting;
* lying;
* bargaining;
* complaining;
* boasting;
* reacting;
* giving directions;
* reporting;
* mocking;
* evading;
* changing the subject.

If the primary function instead appears to be:

* expressing the character concept;
* stating a theme;
* summarizing lore;
* sounding clever;
* delivering a maxim;

inspect the line closely.

---

## 2. Identify the semantic formula

Check for:

* contrast / reversal;
* proposition -> interpretation;
* instruction -> principle;
* evidence -> conclusion;
* event -> general truth;
* setup -> semantic closure;
* concrete statement -> abstract summary;
* character premise -> themed metaphor;
* behavior -> explanation of gimmick;
* local event -> ontology exposition.

Name the formula explicitly during review.

Do not accept a rewrite merely because its syntax changed.

---

## 3. Delete the final sentence as a test

For a multi-sentence utterance, temporarily remove the final sentence.

If the earlier text already performs the necessary conversational action and the final sentence mainly:

* explains;
* summarizes;
* sharpens;
* generalizes;
* moralizes;
* makes the line quotable;
* produces a punchline;
* demonstrates the character concept;

consider deleting it.

Apply the same test to final clauses after dashes, semicolons, or commas.

---

## 4. Remove the character gimmick as a test

Ask whether the generation process appears to have been:

`mathematician -> mathematics metaphor`

`clerk -> paperwork metaphor`

`pirate -> nautical metaphor`

`programmer -> software metaphor`

If so, reject the line unless the terminology is literal in the scene.

---

## 5. Check shared knowledge

Ask what both characters already know.

Remove explanations added for the player's benefit when the speaker has no reason to say them.

---

## 6. Check conceptual scale

Ask whether the line unnecessarily moves from:

* this room -> reality;
* this fight -> philosophy;
* this mistake -> failure in general;
* this person -> human nature;
* this route -> geometry;
* this broken object -> order and chaos.

If so, keep the statement local.

Intentional grandiosity remains available to characters whose voice calls for it.

---

## 7. Check completeness

A line does not need to contain:

* fact;
* explanation;
* interpretation;
* emotional response;
* characterization;
* joke;
* final flourish.

Allow incomplete conversational work.

---

## 8. Check neighboring characters

Review several characters together.

Look for shared:

* sentence rhythm;
* abstractions;
* logical structures;
* metaphor habits;
* levels of eloquence;
* definitions;
* punchline structures;
* formality;
* casual markers.

A line may be acceptable alone while contributing to voice convergence across the cast.

---

## 9. Check whether the rewrite erased the character

A rewrite can pass every anti-LLM rule and still be worse.

Ask whether it removed:

* dialect;
* temperament;
* exaggeration;
* fixation;
* odd syntax;
* repetition;
* emotional intensity;
* arrogance;
* nervousness;
* theatricality;
* cultural or faction flavor.

Do not solve LLMism by making everyone neutral.

---

# Audit classifications

When reviewing dialogue, classify findings.

## Hard violation

Direct use of:

* banned rhetorical word;
* banned phrase;
* `keeps count`;
* banned contrast;
* future-work prose.

## Structural LLMism

A semantic formula from this guide is present.

Name the formula.

Examples:

* `proposition -> interpretation`;
* `instruction -> principle`;
* `event -> general truth`;
* `character premise -> themed metaphor`;
* `setup -> semantic closure`.

Do not report only:

> sounds AI-written

Explain the construction.

## Voice convergence

Several unrelated characters use the same:

* rhetorical structure;
* formality;
* abstraction level;
* explanatory style;
* metaphor process;
* cadence.

Identify the cluster.

## Voice loss

A rewrite removed useful character-specific expression while removing an LLMism.

This should be treated as a defect.

## Intentional violation

The line matches a prohibited pattern, but the character explicitly uses LLM/startup/synthetic rhetoric as part of the characterization.

Leave it alone unless another problem remains.

---

# Core rejection test

Reject a candidate line when it appears to have been generated by solving:

> What is a clever, polished, self-contained sentence that expresses this character concept?

That objective produces many of the failures in this guide.

Also reject the opposite mechanical objective:

> How can I make this sound more human by making it shorter, slangier, or less grammatical?

Instead, constrain the line by the character and scene:

* Who is speaking?
* Who are they speaking to?
* What just happened?
* What do they want right now?
* What do they already know?
* What do they refuse to explain?
* How articulate are they?
* How formal are they?
* What do they exaggerate?
* What do they notice?
* What do they ignore?
* What do they repeat?
* What do they get wrong?

Character traits should alter conversational behavior.

They should not merely substitute themed vocabulary into a shared rhetorical template.
