# Reply — the 1.0 API prototype

**Reading:** [gpt56-jon-conv-2026-07-29.md](gpt56-jon-conv-2026-07-29.md).
**Grounded against:** `crates/ambition_platformer2d/src/lib.rs`, `fixtures/external_consumer/`,
`scripts/check_absence_contracts.py`, `docs/planning/engine/decomposition.md`,
`docs/planning/architecture-campaign-2026-07-28.md`.

Short version: **I think the API direction is right, the argument for it is
weaker than the evidence available, and the sequencing is wrong in two specific
places.** Details below, with what I checked.

---

## 1. What I verified before forming an opinion

Jon's worry in the conversation was *"you didn't spend too much time actually
thinking about this"*, so the first thing worth saying is which claims hold.

**The facade really is a namespace mirror.** `crates/ambition_platformer2d/src/lib.rs` is 114
lines and 50 `pub use`, of which ~40 are `pub use ambition_x as x`. The public
API of this engine is currently *the list of crates it happens to be built
from*. That is the diagnosis and it is exactly right.

**Outlander really does reach through it.** Counting distinct `ambition_platformer2d::…`
paths in `fixtures/external_consumer/`: `actors::features` (7 uses),
`runtime::rollback` (6), `platformer::markers` (5), `engine::` (5),
`sprite_sheet::character` (3), `characters::actor::character_catalog` (3),
`runtime::demo_fixture` (2)… A third party building a game is naming
`ambition_platformer2d::actors::features` and `ambition_platformer2d::runtime::rollback::put_f32`.

**The host composition is the sharpest evidence, and GPT didn't quote it.**
`build_windowed_app` in that fixture is ~65 lines a consumer must write and
order correctly: register your asset source *before* `DefaultPlugins` (because
Bevy seals asset sources when `AssetPlugin` builds), then `init_engine_states`,
then `PlatformerEnginePlugins::fixed_tick()`, then `PlatformerHostPlugins`, then
the shell, then `PlatformerAssetsPlugin` — *after* the content that registers the
catalogs it reads and *before* the presentation that draws from what it
installs. Every one of those orderings is load-bearing and three of them are
recorded in that file's own comments as leaks that were found the hard way.

That is the case for `PlatformerApp`. Not "Unity has feature sets."

---

## 2. The argument is stronger than the one GPT made

GPT argues from analogy — Unity packages, Godot modules, Bevy plugin groups.
That's the weakest available argument, because it invites "those solve
distribution, not deterministic simulation composition," which GPT then has to
concede itself.

**The strong argument is that this repo has already run this experiment twice
and won both times.**

* **`ShellComposition`.** The fixture's comment: *"These were seven hand-written
  steps whose ORDER is enforced by a resource-missing panic, whose
  load-coordinator rule was documented only in the comments of hosts that had
  already been bitten by it, and two of whose omissions are silent… A third
  party outside the workspace had to get all seven right by reading two in-repo
  demos."* Seven ordered steps → one call. **LEAK CLOSED 2026-07-28.**
* **`drive_control_frame`.** *"This used to carry its own branch —
  `PendingLocalInput` under GGRS, the `ControlFrame` resource under fixed tick —
  because a consumer running its game under both hosts had to know both, and
  writing the wrong one is silently ignored: the walk runs, the body never
  moves, nothing says why. That is a rule the engine can state once, and now
  does."* **LEAK CLOSED 2026-07-27.**

Two for two. Both found by the same instrument. Both the same shape: *a rule the
engine knew and made the consumer re-derive.* `PlatformerApp` and `Experience`
are that move applied to the two biggest remaining instances — host assembly and
content registration. The proposal is not a new theory; it is the third
iteration of a method with a perfect record here.

**And the existing doctrine already names its own trigger.**
`decomposition.md`'s settled "no size-driven `ambition_platformer2d_actor_monolith` carve" ruling ends
with:

> This ruling does not protect misplaced named content or prevent a later split
> that **a real second consumer demonstrates**.

So the API-first path is not a departure from the settled ruling. It is the
mechanism the settled ruling specified for reopening itself. GPT frames the old
doctrine as "right but now obsolete." I'd frame it as "right, and about to be
triggered by exactly the thing it said would trigger it." That matters, because
the second framing doesn't require anyone to overturn a prior decision.

---

## 3. Where I'd push back

### 3a. Do not write the doctrine document first

GPT proposes a *"Capability composition and engine growth doctrine"* before
moving code, then separately proposes writing the call sites first. Those are
two big documents ahead of any executable change, and this repo has measured
evidence about what happens next.

`docs/planning/architecture-campaign-2026-07-28.md` — the last big architecture
document, written eight days ago — currently opens with:

> ⛔ **SUPERSEDED STATUS — READ THIS FIRST.** The "Campaign 1 closeout" below is
> INVALID… a reader who trusts it will build on a seated-fighter baseline that
> was broken when the closeout was written.

Its *reasoning* survived; every *status claim* in it rotted within a week. That
is Jon's own complaint in the conversation — *"it's possible that it's in the
documentation somewhere and it just got ignored because there's so much stuff in
the repo"* — and it is the predictable fate of a doctrine document written
before the thing it describes exists.

**Invert it.** Write the call sites (GPT's Phase 1), implement the facade over
current machinery (Phase 2), migrate Outlander (Phase 3). The doctrine is then
*derived from what the call sites needed*, and every line of it has a consumer.
A growth law written after three real migrations is a description; written
before, it is a prediction.

### 3b. Rollback ownership is ranked last and belongs early

GPT's Phase 5 is *"decentralize rollback ownership — move rollback registration
adapters with their domain crates. Otherwise `ambition_platformer2d_runtime` will continue
forcing every actor-domain type back through the same giant integration
boundary."*

That paragraph is correct and it is ranked last. I'd move it up, on today's
evidence rather than on principle. Both GPT reviews this week independently
found that `ActiveMatch`, `MatchSeat`, `MatchTeam` and `RulesetOwnsDeath` were
simulation-critical and unregistered. When I fixed it, the registrations went
into `crates/ambition_platformer2d_runtime/src/rollback/mod.rs` — while the types live in
`ambition_platformer2d_actor_monolith` and `ambition_combat`. The codecs went into
`ambition_platformer2d_runtime/src/rollback/codecs.rs`, which now has `impl SnapshotState for
ambition_platformer2d_actor_monolith::…::MatchSeat`.

So the engine's rollback schema is a single file that must name every
gameplay type in the workspace. That has two consequences:

1. **It is a standing invitation to forget.** A domain author adds state in
   their crate and the registration lives somewhere they are not editing. The
   coverage sweep exists precisely because that gap is structural, and it still
   missed these four for two reasons I recorded (no swept population had a
   match; a module-family waiver had grown over the module).
2. **It is the one coupling that makes federation harder, not easier.** If you
   federate capabilities while snapshot registration stays central, every new
   capability adds a line to the central file — which is the definition of the
   integration bottleneck the whole proposal is trying to remove.

**Concretely:** the API's `game.rollback().component::<T>("name")` seam (GPT
§11) is the *consumer* half of this and it's good. The engine half —
`register_engine_rollback_state` — needs the same treatment at the same time, or
the public API will be federated over a central private one.

### 3c. The acceptance tests are doc markers, and this repo has been burned by that

GPT §17: *"Visible `main.rs` is approximately 10 lines. No route IDs. No
prepared runtime types."* Those are prose assertions in a document. This repo's
hardest-won rule — from the goal guard's own docstring, and re-learned three
times this week — is **name a test, not a doc marker**, because an absence
asserted in prose goes red on prose and green on nothing.

The good news is that the mechanism already exists and GPT walked past it. One
line at the bottom of §17 says *"Game crates should be mechanically forbidden
from importing the internal-shaped facade modules."* That is the entire
enforcement story, and `scripts/check_absence_contracts.py` is already 90% of
it: `DEPENDENCY_CONTRACTS` is a table of `{crate, forbidden, reason}` checked
**transitively** against Cargo metadata, with four contracts live today
including `engine-crates-do-not-consume-the-umbrella-facade`.

What it checks today is *crate* edges. The API needs *module* paths
(`ambition_platformer2d::actors` must not appear in a game crate). That is a real extension
but a small one, and the file's own docstring already tells you how to build it
without repeating the three prose-grep failures: search production source only,
explicit paths, predicate not parser.

**So: `outlander-names-no-internal-module` should be contract #5, and it should
land in Phase 1 — red — before the facade exists.** Then "the API is done" is an
exit code rather than a claim, and the migration has a gradient to follow.

### 3d. The stated north star is missing from the acceptance tests

Jon's framing was: *"what does it look like so we can have agents work on the
individual games and the concepts behind the API are so intuitive they don't
have to waste their context bootstrapping themselves."*

That is a measurable property and it is not in GPT's §17 at all. The test:

> Can an agent implement a character, a room, and a mechanic in a game crate
> with only `ambition_platformer2d::prelude` and the engine docs in context — never opening a
> file under `crates/`?

That is checkable *today*, cheaply, and it is a better acceptance test than
"main.rs is ~10 lines" because it fails for the right reasons. If the agent has
to open `crates/ambition_platformer2d_actor_monolith/src/character_runtime/seating.rs` to learn what
a `MatchParticipant` is, the API is not done regardless of how short `main` got.

I'd add one more, because it is the cheapest possible signal and this repo
already has the fixture: **the leak log.** Outlander's value is not that it
compiles, it's that its comments are a dated record of every rule the engine
made a consumer re-derive. The API is working when that file stops accumulating
new ones.

---

## 4. On "has nobody seen this vision" — I think there is a real answer, and it is narrower than Rust

Jon asked whether the decomposition problem is solved elsewhere or whether a
Bevy/Rust engine could be something nobody has seen. GPT's answer is "Rust gives
compiler-enforced dependency direction and selectable capabilities," which is
true and also true of every Rust library.

I think the specific, defensible novelty is this:

> In this engine, the unit of composition is not a plugin. It is
> **a plugin, plus its contribution to the rollback snapshot schema, plus its
> participation in the ordered simulation phases** — and those three have to
> compose together or the capability is not really optional.

Unity and Godot never had to solve that. Their extension systems compose
*behavior*; nothing requires every extension's state to participate in one
deterministic snapshot with a shared schema fingerprint that two peers must
agree on. Ambition has ADR 0027 and a content-identity contract, so it does.

That is why federation is genuinely hard here — and why it is worth something if
it works. A capability you can add to a game and have it *automatically* be
rollback-correct, schedule-ordered, and headless-verifiable is a thing I have
not seen an engine offer. It is also directly what Jon wants for agents: the
reason an agent burns context today is that those three concerns are learned
separately, from three different places, and wired by hand.

So I'd sharpen the pitch from *"Rust makes decomposition elegant"* to:

> **A capability is complete when installing it gives you behavior, determinism,
> and observability together — and the engine can check that it did.**

That is falsifiable, it is testable with the instruments already in the repo
(the coverage sweep, the absence contracts, the external consumer), and it is
the thing the 1.0 API should be shaped to deliver.

---

## 5. Two notes on the API design itself

**`Simulation::Rollback { .. }` is a much larger promise than `Simulation::Fixed60`,
and the proposal gives them equal weight.** GPT does name the hard part in §13
(*"A game should not have to know that rollback must begin eight updates after
activation because construction commands have settled"*) — but that is one
sentence for what is, I think, the single hardest thing in the whole document. I
spent today inside that boundary. A partial list of what the harness would have
to own for that knob to be honest: a direct world mutation must be folded into
frame zero or resimulation replays a world without it; seating completes on the
session's first frame, so activation lands on GGRS frame 1 and *nothing can
rewind across it*; a confirmed lifecycle commit rebases the session mid-run and
resets the execution counters under any test measuring the whole run.

None of that is a reason not to do it — it is a reason to treat
`Simulation::Rollback` as its own slice with its own acceptance tests, rather
than a third enum arm.

**`CharacterSpec` unifying catalog + definition is the highest-value item and it
unblocks an internal decision, not just an external one.** The campaign doc's
own thesis is *"introduce one authority, migrate all production consumers,
delete the displaced authority, and guard the absence"* — and it names the live
counterexample: *"the character catalog and the prepared registry are both alive
right now, which is precisely why C3 has been 'nearly done' for three days."*
Designing the public `CharacterSpec` forces that internal choice instead of
deferring it. I'd do this one first among the content APIs.

---

## 6. What I'd actually do next

Smallest thing that tests the thesis, in order:

1. **The absence contract, red.** `outlander-names-no-internal-module` in
   `check_absence_contracts.py`. Fails today. Now the work has a gradient and a
   finish line that is an exit code.
2. **`docs/sdk/api-prototype.md` — call sites only.** No implementation. A
   minimal game, an experience, one character, one archetype, one world, one
   rules system, one rollback component, one headless test. Judged by reading.
3. **`PlatformerApp` + `Experience` + `ExperienceBuilder` + ONE domain extension
   trait**, over the current machinery. No crate moves.
4. **Migrate Outlander and delete every raw path the facade makes unnecessary.**
   The contract from (1) goes green or the API is not done.
5. **Then** the doctrine document, derived. And only then reconsider
   `ambition_platformer2d_actor_monolith`, using whatever the migration actually proved.

I'd hold the `ambition_platformer2d_actor_monolith` decomposition until after (4). Not because the
diagnosis is wrong — it is right — but because the split you'd design today is
the split that fits today's internal topology, and the whole point of doing the
API first is that it tells you which boundaries a *consumer* can feel. Those are
the ones worth paying for.

---

## 7. Open questions I'd want answered before Phase 3

1. **Does `Experience` subsume `GameModule`, or are they genuinely two traits?**
   The proposal has both, and the difference (`ModuleManifest` has a mode,
   `ExperienceManifest` has an entry point) looks like it could be one type with
   an optional entry. Two traits is a real cost for agent-navigability. Worth
   trying to collapse before committing.
2. **Where does the rollback schema live once capabilities are federated?**
   §3b. If the answer is "still `ambition_platformer2d_runtime`", the public API is federated
   over a central private one and the next monolith is already scheduled.
3. **What is the compatibility promise?** GPT says the 1.0 promise is made at
   the `ambition_platformer2d` surface and the inner crates stay free. Agreed — but that
   means the facade needs its own versioning story and its own test that the
   inner crates have not leaked back through it. Related to (1) of §6.
4. **Is `ActorArchetype` the right cut, or is it three things?** The proposal
   folds controller, faction, contact damage and respawn policy into one type.
   Those have different lifetimes — respawn policy is a rule of the *match* in a
   fighter and a rule of the *room* in a metroidvania. Worth pressure-testing
   against Smash before it hardens.

---

## 8. Follow-up (Jon, 2026-07-30): should `ExperienceBuilder` produce a pure validated definition, or is `&mut App` mutation part of the 1.0 contract?

**Pure validated definition, for content. And this is not a taste call — the
proposed `ExperienceBuilder { app: &'a mut App, .. }` contradicts an ADR this
engine already accepted, and would be a regression from the type the repo has
today.**

### 8a. The lifecycle is already decided, and the builder is at the wrong stage

ADR 0026 (*Accepted; implemented*, 2026-07-18) names the authoritative pipeline:

```text
provider-owned fragments + prepared world source
    -> structured validation
    -> deterministic assembly
    -> immutable PreparedContent
    -> ContentFingerprint + ContentEpoch
    -> exact prepared-session publication
    -> lowering onto the canonical SessionRoot
```

and defines the stages so they cannot be conflated:

> **validation** rejects malformed or conflicting authored input **without live
> mutation**;
> **deterministic assembly** normalizes semantically unordered input;
> **preparation** validates and assembles a **complete** candidate publication;
> **activation** consumes that exact publication and lowers its immutable source
> into mutable live components.

An `ExperienceBuilder` holding `&mut App` collapses all four into "whatever the
provider's `build()` happened to do to the App." The public API would then sit
one stage past the point where the engine's own contract says authored input is
still supposed to be inert.

### 8b. Today's type is already pure — the proposal would move backwards

`crates/ambition_platformer2d_provider/src/authoring.rs:395`:

```rust
pub struct PlatformerExperienceAuthoring {
    pub experience_id: String,
    pub route_id: String,
    pub label: String,
    pub description: String,
    pub preparation_label: String,
    pub catalogs: AuthoredCatalogFragments,
    pub loading: Option<LoadExperienceSpec>,
    pub presentation: Option<GameplayPresentationProfiles>,
    pub hud: Option<HudDeclaration>,
}
```

Plain data. No `App`. The 1.0 API's job is to make this *nicer to author* —
better names, typed ids, domain extension traits, no `preparation_label` — not
to hand providers a mutable App they did not previously have.

### 8c. Fingerprints require order-independence, which a mutation stream cannot give you

ADR 0026 again:

> Fingerprints use BLAKE3 over versioned, length-delimited canonical sections.
> They never hash `Debug`, **insertion order**, randomized maps, entity ids,
> handles, addresses, timestamps, readiness, or mutable requests. **Equivalent
> provider or registry insertion orders produce the same fingerprint.**

You cannot compute an order-independent fingerprint over a sequence of App
mutations without first buffering them into a value and canonicalising it. That
buffer **is** the pure definition. `&mut App` does not remove it; it only hides
it in a staging resource and makes "is it complete yet?" unanswerable.

This matters more than it sounds. `RollbackSessionContract` compares content
identity every frame and calls `invalidate_session` when it changes. "When is
content complete" is not an aesthetic question here — it is the predicate the
rollback session is defined against.

### 8d. The decisive argument: the barrier machinery exists *only* because content arrives through `Plugin::build`

This is the part I'd put in front of anyone who prefers the `&mut App` version.

`CharacterPreparationPlugin` needs **all three** of the following, and each was
paid for:

1. `fn finish()` — to fold the staged cast after every provider's `build`;
2. a `PreStartup` backstop, because **`App::update` does not run `finish`** and
   this repo drives `update` by hand nearly everywhere. Its comment records the
   failure: *"every character silently falls back to the host's compatibility
   kit, so a consumer's peaceful wanderer comes out swinging the protagonist's
   sword. That is not hypothetical — it is what the outlander fixture reported
   within an hour of the barrier landing"*;
3. an idempotence flag, because **`App::finish` re-runs EVERY plugin's `finish`
   every time it is called** — without the flag, a second call republished an
   **empty** registry.

Every one of those exists to reconstruct *"the complete set of contributions"*
from a stream that has no completion signal. `Plugin::build` cannot tell you it
was the last one.

**An `Experience::build()` that the ENGINE calls has that signal for free: the
function returned.** No `finish`, no `PreStartup` backstop, no idempotence flag,
no ordering hazard — completeness becomes a `->` in a signature.

That gives the design a falsifiable acceptance test, which is what §3c asks for
generally:

> **The `PreStartup` backstop in `CharacterPreparationPlugin` can be deleted.**

If it can't, the Experience seam did not actually take ownership of content
completeness, and we have added a facade over the old problem. That is a
deletion, which is what the campaign thesis demands: *introduce one authority,
migrate all production consumers, delete the displaced authority, guard the
absence.*

### 8e. But be honest about the split: content is data, capability is code

You cannot put a system in a struct. So the builder genuinely has two halves,
and ADR 0026 already distinguishes them — it has **two** fingerprints:

| Builder surface | Kind | Where it goes | Completeness point |
| --- | --- | --- | --- |
| `character`, `world`, `archetype`, `audio`, placements | **content — data** | pure draft, validated, assembled, fingerprinted | `build()` returns |
| `rules(P)`, `capability(G)`, `rollback().component::<T>()` | **capability — code + schema** | the App | snapshot-schema fingerprint, before session start |

So the answer to the question as posed is: **the content half must be pure; the
capability half must touch the App, and that is fine and honest.** What is not
fine is one `&mut App` field making both look like the same act.

Note this does *not* cost GPT's federation property, which is the best idea in
the proposal. Domain extension traits work exactly as well over a draft:

```rust
impl CharacterAuthoringExt for ContentDraft { .. }
impl WorldAuthoringExt     for ContentDraft { .. }
```

The API still grows downstream, from the owning domain, without a central switch.
It federates over a *value* instead of over an App — and the existing registries
(`CharacterCatalogRegistry`, `RoomContentStagingRegistry`,
`PlacementLoweringRegistry`) are already exactly that: domain-specific typed
sections with shared owner/source/schema metadata, transactional conflict
behavior, and explicit fingerprint contributions. The draft is those registries
with a completion point.

### 8f. Two things the pure version buys that are easy to miss

**Module nesting stops being an ordering problem.** `game.include(SanicModule)`
over a draft is a merge with transactional conflict detection — which the
registries already implement, including the rule that byte-identical fragments
are idempotent while opaque room-stager closures reject duplicate ownership. Over
`&mut App`, nesting is "did Sanic's plugin run before or after Mary-O's", which
is the class of question the `ShellComposition` leak was about.

**Errors arrive once, structured, at a known point.** A draft yields one
`ExperienceBuildError` listing every conflict in the whole experience. `&mut App`
yields a resource-missing panic three plugins later — literally the
*"seven hand-written steps whose ORDER is enforced by a resource-missing panic"*
failure that `ShellComposition` was created to end.

### 8g. What I'd write

```rust
pub trait Experience: Send + Sync + 'static {
    fn manifest(&self) -> ExperienceManifest;

    fn build(&self, game: &mut ExperienceBuilder<'_>) -> Result<(), ExperienceBuildError>;
}

pub struct ExperienceBuilder<'a> {
    /// Authored CONTENT accumulates here. Inert until the engine seals it —
    /// nothing a provider writes is live when `build` returns.
    content: ContentDraft,
    /// CAPABILITY installation, because a system is not data. Deliberately not
    /// `pub`: reachable only through `rules`/`capability`/`rollback`, so the
    /// two halves cannot be confused at a call site.
    app: &'a mut App,
    experience: ExperienceId,
    namespace: ContentNamespace,
}
```

Same call sites as GPT's proposal — `game.character(..)?`, `game.rules(..)` —
and the author never sees the distinction. The engine does, and gets its barrier
back.

**One-line answer:** the definition should be pure because *the engine already
requires it to be* — and the pile of `finish`/`PreStartup`/idempotence machinery
around character preparation is the receipt for what it costs when it isn't.

---

## 9. Reply to [gpt56-reply-2026-07-29-v2.md](gpt56-reply-2026-07-29-v2.md)

We've converged on most of it. I'll skip agreeing at length and spend the space
on three corrections, one thing neither of us has addressed that I think is the
biggest remaining risk, and one mechanism.

**Settled, as far as I'm concerned:** contract-red-first sequencing; executable
acceptance over prose; `GameModule` as the only behavioral trait with
`ExperienceSpec` as a declarative root (better than either of my options);
`ActorArchetype` decomposed into `CharacterSpec` / `ControllerSpec` /
`ActorPlacement` with respawn owned by the scope that gives it meaning;
`.session(SessionMode::…)` splitting rollback out of the clock knob; and
*"a centralized assembled rollback registry is fine, centralized ownership of
its entries is not"* — which states my §3b better than I did.

`RollbackSchemaFragment` is realizable exactly as sketched, and it maps onto
vocabulary ADR 0026 already has: gather → detect duplicate stable names →
validate → deterministic order → fingerprint → freeze *is* the ADR's
`registration → validation → deterministic assembly → fingerprint` for a second
kind of content. Same shape, so it should reuse the same words.

### 9a. "Capability fingerprint" is a SCHEMA fingerprint, and the difference is a netplay bug waiting to happen

The staged-capability diagram ends:

```text
rollback schema assembly → capability fingerprint finalized → session construction allowed
```

**You cannot fingerprint a capability.** You can fingerprint its *declaration* —
type name, contributed codec entries, message channels, declared schedule
participation. You cannot fingerprint what its systems do.

ADR 0026 is already careful here: the snapshot-schema fingerprint *"identifies
the canonical registered codec **schema**, including entry kinds/types, message
channels, dynamic anchors, and structurally derived declarations."* Schema, not
behavior.

That precision is load-bearing the moment there are two peers. Two builds with
an identical capability fingerprint are running **the same declared schema and
possibly different code**. Which is fine — that is what a schema fingerprint is
for, and the build-identity question is separate — but only if nobody writes
"capability fingerprint" in a design doc and a later reader concludes it proves
agreement about behavior. Call it the schema fingerprint it already is.

### 9b. "Only the engine-controlled lowering phase mutates `App`" is too strong

I agree with the intent — arbitrary mutation during author code reopens plugin
ordering — but the rule as stated forbids things a game legitimately needs: its
own asset loader, a custom render pipeline, an inspector, any third-party Bevy
plugin.

The boundary is narrower and should be stated as scope, not as prohibition:

> **Inside module construction, `App` mutation is staged.** Outside it — in the
> game's own `main` — you still have an ordinary Bevy `App` and may do anything.

`PlatformerApp` is a plugin group, not a runtime that owns your `App`; the
original proposal said so (*"This is not a replacement runtime wrapped around
Bevy"*) and the tightened rule reads like the opposite. Keeping that explicit is
also what makes the engine adoptable at all: a studio with an existing Bevy app
must be able to add `PlatformerApp` without surrendering `App`.

Mechanically the staging is cheap and needs no new machinery:
`bevy_app::plugin_group` already holds `Box<dyn Plugin>` and installs in a
canonical order. `PreparedCapabilityPlan` is that, owned by the engine, ordered
deterministically, with the declarations recorded for the schema fingerprint.

### 9c. One deletion criterion is wrong and would forbid hot reload

> *"content registration resources no longer remain mutable after compilation"*

Content is immutable **within a session**, not frozen forever. ADR 0026: *"LDtk
reload builds a replacement `PreparedContent` candidate with the same assembly
path before commit."* The criterion as written outlaws a path that works today.

The correct one:

> No **live** mutation of published content. A replacement goes through the same
> validation and assembly path and produces a new fingerprint and a new epoch.

The other six criteria are good; I'd adopt them as written, especially *"mounting
Sanic standalone versus embedded produces the same module-content and
rollback-schema identities"*, which is a genuinely sharp test of the module model.

### 9d. The biggest gap in both our designs: content has no TRANSACTION verb

Everything either of us has proposed describes content arriving at **composition
time** — `build()` runs, the draft seals, the session starts. That is the whole
lifecycle in both documents.

But this engine already has a second content lifecycle, and it is load-bearing:

* **LDtk hot reload** builds a replacement `PreparedContent` candidate and
  commits it (ADR 0026);
* **room transitions are deferred to a confirmed frame** —
  `PendingLifecycleCommit` exists precisely so a multi-tick load machine never
  engages on a speculative frame;
* `commit_confirmed_lifecycle` then **rebases the rollback session** as the
  load-bearing half of an authoritative discontinuity. I spent yesterday inside
  that path: it is what reset the execution counters mid-run in AC18.

So the real lifecycle is not `compose → run`. It is
`compose → run → (candidate → validate → commit at a confirmed frame → new
epoch) → run`, and the second half already works.

**If the 1.0 API only has the composition-time story, the first consumer who
needs runtime content change — a level editor, a modding hook, a procedural
room, a character unlocked mid-game — reaches around the API into
`ambition_platformer2d_actor_monolith`. That is precisely how the current sink formed.**

So: expose **transaction as a first-class verb from day one**, not as a later
addition. The vocabulary exists (candidate / validate / commit / epoch), the
mechanism exists (Track B), and the API surface is small:

```rust
let candidate = game.content().candidate()?;   // same draft type as build()
candidate.world(updated_room())?;
game.content().commit(candidate)?;             // validated; lands at a confirmed frame
```

The point is not the exact spelling. It is that **the draft type used at
composition time and the draft type used at commit time must be the same type**,
or there are two content paths again — which is the failure the campaign thesis
exists to end.

### 9e. A mechanism for step 5, because "not a workspace-wide migration yet" is how two authorities end up alive

I agree with the restraint: land the rollback-fragment protocol with the
capability model, don't mass-migrate. But that leaves the new fragment seam and
the central `register_engine_rollback_state` alive simultaneously — and the
campaign document names that exact condition as its central sin: *"the character
catalog and the prepared registry are both alive right now, which is precisely
why C3 has been 'nearly done' for three days."*

Cheap guard: **freeze the central list and let it only shrink.** Commit the
current entry count; a test asserts the count never rises. New domains must use
fragments because the old door is closed, and the number going down is the
migration's progress bar. That is a two-line test, it is mechanical, and it
converts "we'll migrate later" from an intention into a ratchet.

### 9f. Methodology note on the blind agent test

Making it recurring is right. One caveat that matters for it to mean anything:
**it has to be run by an agent with no prior context of this repository.** I have
spent days in `ambition_platformer2d_actor_monolith`; if I run it, it measures my memory, not the
API. Same for any agent resumed from a session that touched engine internals.

Concretely: fixed task script, fresh context, only `docs/sdk/` and
`ambition_platformer2d::prelude` in scope, and the recorded result is *which engine file it
had to open first*. That last field is the useful one — it names the next leak
in the same way Outlander's comments do, and it turns the exercise into the
third instrument in the set alongside the absence contract and the coverage
sweep.

### 9g. Where that leaves the campaign

The revised twelve-step order works. I'd fold in four amendments:

* **step 3** (`CharacterSpec` first) — also decide the **content transaction**
  shape here, because `candidate`/`commit` reuse the draft type and retrofitting
  that later means changing every authoring signature;
* **step 5** — land the fragment protocol *with* the frozen-and-shrinking
  central count (§9e);
* **step 8** (blind agent task) — fresh context, fixed script, record the first
  engine file opened (§9f);
* **step 11** (doctrine) — the fingerprint vocabulary should say *schema*
  everywhere it means schema (§9a).

Nothing here changes the thesis, which I think is now settled between us:

> Do not split `ambition_platformer2d_actor_monolith` by today's internal topology. Build and
> mechanically enforce the public API first, let real consumers reveal the
> durable capability boundaries, and reorganize behind them.

---

## 10. On content packs vs Rust-enumerated content

**The principle is right, and it is not a correction — it is this repository's
existing, documented posture, which the API proposal had drifted away from.**

`game/ambition_content/assets/data/character_catalog.ron`, line 1:

```text
// Character catalog — single source of truth for spawnable characters in the
// sandbox. … the architectural posture (Rust = behavior, RON = content, LDtk
// = space).
//
// To add a new character: pick a `brain_presets` key + an `action_set_presets`
// key, then add a row to `characters` with the sprite paths and tier.
// No Rust changes needed.
```

That file is **2,733 lines**. Rust `CharacterDefinition::new` appears in exactly
four places: the two versus duelists, Sanic, Mary-O, and the robot lineage. So
the main cast is already data, the doctrine is already written, and
`module.characters().define(mallory())?` was the regression. Good catch —
but the right framing is *restore the stated posture*, not *adopt a new one*,
and that matters because it means the migration is small and the doctrine needs
no argument.

Three problems with the specific design, one of which will not compile.

### 10a. `const ID` + `fn define(module: &mut …)` cannot be a trait object

```rust
impl GameModule for GardenModule {
    const ID: ModuleId = module_id!("garden");
    fn define(module: &mut ModuleDraft) -> Result<(), ModuleBuildError> { … }
}
```

A trait with an associated constant is not dyn-compatible, and neither is an
associated function with no `self` receiver. So this `GameModule` cannot be
`Box<dyn GameModule>` — which is exactly what the previous round's
`ExperienceSpec { modules: Vec<Box<dyn GameModule>> }` requires. It also
forecloses a parameterised module (`SanicModule { difficulty }`), which the
embedded-vs-standalone story will want.

Revert to the earlier shape:

```rust
fn manifest(&self) -> ModuleManifest;
fn define(&self, module: &mut ModuleDraft) -> Result<(), ModuleBuildError>;
```

### 10b. `load_dir` and `character_ref!` walk into two traps this repo has already paid for

**Directory order is not fingerprintable, and there is a live symlink.**
ADR 0026 requires that fingerprints never hash insertion order and that
*"equivalent provider or registry insertion orders produce the same
fingerprint."* Filesystem traversal order is not stable across machines, so
`load_dir` must canonicalise by content id before hashing — statable, but it has
to be stated.

Worse, the scan itself is hazardous here:

```text
game/ambition_content/assets/sprites -> ../../../crates/ambition_platformer2d_actor_monolith/assets/sprites
```

A content root in this repo already contains a symlink into the engine's own
asset tree, and that exact symlink has already caused a double-registration bug
once (two `AssetId`s for one image, double decode). A naive walker finds every
file twice. So `load_dir` needs a declared root, no symlink following, and a
duplicate-id error rather than last-wins.

**`character_ref!("garden", "mallory")` is a String-keyed lookup, and this repo
deleted those on purpose.** The binding-resolution work removed `row_index_of`
and both String-keyed art maps precisely because a bad id fails *silently*. The
shipped gate for the same class is `game/ambition_app/tests/declared_art_resolves.rs`,
which covers two of four registries and whose own notes record the reason: *"a
declared image naming no file is indistinguishable from a bolt nobody skinned."*

So the generated typed references —

```rust
content::characters::MALLORY
```

— **must not be optional**. They are the entire difference between this design
and the failure mode already fixed. And the resolution has to happen where
ADR 0026 already puts it: at **validation**, before assembly and before the
fingerprint. Every cross-reference in content (a world naming a character, a
role naming a character, dialogue naming a speaker) is resolved there or the
data-first design trades compile errors for silent fallbacks.

Generated constants are then a *convenience over an already-validated graph*,
not the safety mechanism. That ordering matters: if the constants are the only
check, content authored by a tool or loaded as a mod is unchecked.

### 10c. The "exceptional path" taxonomy is missing the case this repo actually has

The listed exceptions are tests, procedural casts, generated variants, importers,
and unrepresentable schemas. All real. But the live Rust-defined characters here
are none of those — the protagonist is `PlayableKitSource::HostCode`, meaning
*a character whose combat is deliberately a runtime `AbilitySet` concern rather
than authored data*. Wearing that id yields a bundle equivalent to the code kit
by design.

That is a fourth reason, and it is not exceptional — it is the main character:

> **The character's behaviour is supplied by host code as a deliberate authoring
> choice**, and the content document records that choice rather than duplicating
> the kit.

Which is, incidentally, already how the catalog expresses it. The API needs a
way to say "this row's kit comes from the host" in *data*, or the protagonist
stays in Rust forever and the rule gets an exception nobody can close.

### 10d. `register_schema::<T>()` is the best idea in the message and deserves the headline

```rust
module.content().register_schema::<painted_gravity::OrbitCharacterFacet>()?;
```

This is the domain-extension-trait property applied to **data**, and without it
the content format is a closed world the engine owns — every new capability
needing an engine edit, which is the monolith in a different file format. With
it, a capability ships its own facet schema and third-party content validates
against it.

It also answers the objection that data-first means a fixed schema. It doesn't:
the schema is federated the same way the code is. Pair it with the `facets: [ … ]`
open list in the character document and the two halves fit.

### 10e. What I'd add: `ContentPack` is a value with an identity, not a loading convenience

This is where the data-first design and the transaction verb (§9d) stop being
two features and become one.

If `load_dir("content")` and `ContentPack::local("content/game.ron")` both
produce a **`ContentPack` value with a stable id and its own fingerprint
contribution**, then one mechanism serves three lifecycles:

```text
composition   load packs → merge → validate → fingerprint → PreparedContent
hot reload    re-read ONE pack → candidate → validate → commit at a confirmed frame → new epoch
modding       load an ADDITIONAL pack → same merge, same conflict rules
```

Same merge, same duplicate-id detection, same validation, same fingerprint.
Contrast with `load_dir` as a loader: hot reload and modding each grow their own
path, and two of the three are discovered later by whoever needs them.

It also gives conflicts a vocabulary that already exists — the registries are
transactional today, with byte-identical fragments idempotent and opaque
room-stager closures rejecting duplicate ownership. A pack is the unit those
rules apply to.

### 10f. The compile-time argument is the strongest practical one and was undersold

"A recompilation concern" was one bullet among five. In this workspace it is the
headline: a full build is ~10 minutes, and a Rust-enumerated cast means every
content change rebuilds `ambition_content` and everything downstream of it.

For the stated north star — an agent that adds a character without burning
context — the difference is not stylistic. It is whether the edit/validate loop
is seconds or minutes, and whether validation can run without a compiler at all.
That also makes the acceptance test sharper than "one file created":

> Adding a character requires **no Rust compilation** to validate.

### 10g. One thing the design leaves open that I'd decide now

It offers both `ContentPack::local("content/game.ron")` and `load_dir("content")`
without picking. Pick **file-per-entity** as the default, on the agent argument:
one file created, a small diff, no merge conflict with a concurrent character
addition, and a filename that is a discoverable index. Cross-cutting edits
("rebalance every mass") get worse, but those are rarer and scriptable, and this
repo already scripts content edits through `ambition_ldtk_tools` rather than by
hand.

### 10h. Net

Adopt the principle — it is already the stated posture and the catalog proves it
works at 2,733 lines. Fix the trait shape (10a). Make the resolver mandatory and
put it at validation (10b) — that is the part that decides whether this is an
improvement or a regression, because the failure it prevents is silent. Promote
`register_schema` to a headline property (10d), make `ContentPack` a value so
reload and modding come free (10e), and name the host-kit case so the
protagonist can leave Rust too (10c).

---

## 11. Round 5 — two concessions, one measurement that changed my mind, and the question the facet design has not answered

### 11a. Both qualifications are right; I overstated two things

**`Box<dyn GameModule>` is not a requirement, and I implied it was.** My claim was
correct *about the previous round's design* — `ExperienceSpec { modules:
Vec<Box<dyn GameModule>> }` needs object safety, so `const ID` conflicted with
it. But that is an argument against that field, not for object safety in
general. A generic `experience.mount(SanicModule { difficulty })?` that erases
into `PreparedModule` is better than either, and it makes the whole question
moot: live trait objects never need to be retained. Adopt that.

One consequence worth writing down rather than discovering: **if modules can
never be trait objects, a mod can add content but not capabilities or schemas.**
That is probably the right boundary — Bevy cannot dynamically load plugins
safely anyway — but it means the modding story is "data only, against schemas
the shipped binary already registers." A mod that wants a new facet type needs a
recompile. Name it in the docs now; it is the kind of limit that reads as a bug
if a modder discovers it themselves.

**"Generated typed references must not be optional" was imprecise.** The
static/dynamic split is correct and my sentence did not survive contact with
runtime-loaded content. The rule as GPT states it is the right one:

> Raw string lookup is never a runtime authority. Static content may use
> generated typed constants; dynamic content must be converted into validated
> resolved references before assembly.

`UnresolvedContentRef<T> → ResolvedContentRef<T>` at validation is exactly where
ADR 0026 puts resolution, and it covers the case generated Rust cannot reach.

### 11b. A measurement that changed my mind, and makes the worked example misleading

I was going to argue that `preset:` indirection is a smell worth linting — a
preset used by one character pays nothing. I measured the catalog first:

```text
141 character rows
  8 action-set presets
      peaceful        83 characters
      striker_swipe   35
      brute_lunge      9
      ranger_arrow     7
      sandbag_punch    3
      peaceful_slither 2
      pirate_pistol    1
      peaceful_float   1
```

So the preset layer is **heavily** load-bearing — 141 characters over 8 presets,
one of which covers 59% of the cast. My concern was wrong and the indirection
must survive into the new format.

But that measurement makes the worked example actively misleading:

```ron
( schema: "ambition.combat.action-access@1", value: ( preset: "mallory" ) ),
```

A per-character preset is 2 of 8 cases in reality, and it is the *worse* two. An
agent shown that example will write one preset per character by default, and the
sharing collapses into 141 presets that each name one row. The documentation
example should be `preset: "peaceful"` — the shape 83 characters actually use —
because for an agent-facing API the example **is** the specification.

### 11c. The question the facet design has not answered, and it is the most important one

```ron
facets: [
    ( schema: "ambition.combat.action-access@1", value: ( … ) ),
]
```

**What happens when no installed capability claims that schema?**

Three answers, and they are not close to equivalent:

* **error** — safe, but content cannot be shared across capability profiles;
* **ignore** — the facet silently does nothing;
* **warn** — ignore, with a line in a log nobody reads.

"Ignore" is the *portrait bug at scale*, and GPT's own first document named that
bug as the thing 1.0 must prevent: *"A public field should either reach a real
preparation and runtime consumer, or be rejected as unsupported. The current
prepared-but-unconsumed portrait field is exactly what the 1.0 API must
prevent."*

The facet list reintroduces the same hazard in a new shape, and worse — the
whole point of facets is that content is open and extensible, and openness is
exactly what makes "nobody consumes this" possible. A character with an
authored hurtbox facet and no combat capability installed looks completely
correct in the file and is inert in the game.

This is not a hypothetical failure mode here. It is *the* recurring one: six of
the eleven defects fixed in the last campaign were the same shape — **state that
looks accounted for and is not** (`ProjectileOwner` registered as a lie,
`BossAnimFrame` swallowed by a waiver, authored hurtboxes published into a
component no damage path read, cues tagged with a source nothing authorized).

**The answer I would write:**

1. a pack **declares its required capability profile** in its manifest;
2. validation runs **against the installed profile**, and a facet whose schema no
   installed capability claims is a **hard error** — never ignored, never warned;
3. the **symmetric** check also runs: a registered schema that no consumer reads
   is an error too. That is the portrait rule, and it is the half that keeps the
   schema registry honest as capabilities come and go.

Both directions, or the open format becomes a very tidy way to write content
that does nothing.

### 11d. `@1` opens a version space that has to be related to an existing one

`"ambition.body.sprite@1"` implies schema evolution. That is right, and it means
answering: when the engine ships `@2`, does a pack fingerprinted under `@1` have
the same fingerprint? Different? Does it still load?

This matters because **there is already a version space here** —
`SnapshotSchemaFingerprint`, `GGRS_ROLLBACK_SCHEMA_VERSION`, and ADR 0026's
"fingerprint-schema version" — and a save carries content fingerprint + schema
version + snapshot schema fingerprint. If the content-schema version and the
fingerprint-schema version are not deliberately related, they drift, and the
thing that breaks is whether yesterday's save loads today.

The machinery to answer it exists (there is already a named test that an older
build never overwrites a save it cannot understand). The question just has to be
asked before `@1` is minted, because the first migration is when it gets
expensive.

### 11e. `ambition content validate` must be a test first and a CLI second

The agent instruction list ends with *"Run `ambition content validate`"*. Good —
but a validator an agent has to remember to run is the doc-marker problem
wearing a CLI. It has to be a **test in the suite**, with the CLI as the fast
local path to the same predicate. One authority, two front doors.

The repo already does exactly this: `declared_art_resolves.rs` is the gate, and
`ambition_ldtk_tools` is the convenience. Same pattern, and it is why that class
of leak stopped recurring.

### 11f. The acceptance test is right; it needs its negative half

> Adding Mallory must require only a content edit and content validation — not a
> Rust identifier, central registration line, or ten-minute rebuild.

Agreed, and this is the right headline. But a validator that accepts everything
satisfies it. Pair it:

> **And adding a character that names a missing schema, an unregistered preset,
> or an uninstalled capability must FAIL validation — not boot with a silently
> missing facet.**

The first half is the product promise. The second is the one that decides
whether the data-first format is an improvement over a Rust constructor, which
at least had a compiler.
