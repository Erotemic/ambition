# Shell vocabulary: provider, experience, route

**Checked 2026-08-07.** Every citation below was fetched, not recalled; one claim
that failed its check is recorded at the bottom rather than quietly dropped.

## The Ambition concepts

Our shell (`crates/ambition_game_shell/`) has three nouns, and they are three
because they answer three different questions:

| our term | the question it answers | where |
|---|---|---|
| **provider** | who AUTHORED this content | audio fragments, characters, SFX are keyed by it |
| **experience** | what KIND of thing the shell runs | `ShellExperienceId`; `ExperienceRegistration` |
| **route** | how you GET there | `ShellRouteId`; `ShellRouteSpec::new(route, experience)` |

Measured 2026-08-07: `BASIC_LAUNCHER_EXPERIENCE` is ONE experience reached by
FIVE routes (Ambition's launcher, Sanic's, Mary-O's, the provider composition's,
and a test's). So route→experience is many-to-one and the split is load-bearing,
not decoration.

Jon's question that started this: *"I'm not sure I like the terms experience and
route. The idea is experience is the game itself, and route is a some screen or
menu or system in that game?"* That reading has two levels where the code has
three — which is a real complaint about the NAMES, and the reason to look outward
before renaming anything.

---

## Unreal Engine — has all three, and calls one of them the same thing we do

| our term | Unreal |
|---|---|
| provider | **Game Feature Plugin** |
| experience | **GameMode**, and in the Lyra sample literally **Experience** |
| route | **Map** + **URL travel options** |

### Game Features are Unreal's "provider"

> "Unreal Engine 5 introduces Modular Game Features, a system that lets you
> inject new content and experiences into your game via a plugin architecture.
> The Modular Game Feature is created in such a way that the core game is
> completely unaware of its existence, eliminating the need for creating
> dependencies from the game to the new content."

They can be "freely turned on and off at any time without breaking the game".
That is Ambition's provider: a self-contained bundle of content plus code that a
host composes without the host knowing what is in it.

Source: [Game Features and Modular Gameplay in Unreal Engine](https://dev.epicgames.com/documentation/en-us/unreal-engine/game-features-and-modular-gameplay-in-unreal-engine)
(Epic, official).

### The route can override which experience it runs

Unreal's game mode is chosen by an explicit priority ladder:

> "The class of this GameMode actor is determined by (in order) either the URL
> ?game=xxx, the GameMode Override value set in the World Settings, or the
> DefaultGameMode entry set in the game's Project Settings."

Syntax: `UE4Editor.exe /Game/Maps/MyMap?game=MyGameMode -game`.

⭐ **That is the strongest single data point for us.** Unreal treats "which map"
and "which rules" as separate facts, addressed together in one URL, with the
address allowed to override the map's own default. Our `ShellRouteSpec::new(route,
experience)` is the same pairing with none of the override machinery.

Source: [Game Mode and Game State in Unreal Engine](https://dev.epicgames.com/documentation/en-us/unreal-engine/game-mode-and-game-state-in-unreal-engine)
(Epic, official).

### Lyra uses the word "Experience" for our meaning

Epic's Lyra sample formalises the concept as a data asset:

> "**ULyraExperienceDefinition** … a Primary Data Asset that literally defines a
> given Experience, containing default Pawn Data, Game Feature Actions,
> Experience Action Sets, and GFP dependencies."

and the map names which one it wants:

> "Each level in a Lyra project can specify the `Default Lyra Experience` to load
> for that level via custom World Settings."

There is also a second, thinner type that pairs the two ids explicitly:

> "**ULyraUserFacingExperienceDefinition** … a lighter weight object which holds
> the IDs (of type `FPrimaryAssetId`) of maps and experiences together with
> references to UI widgets."

⭐⭐ **So Epic independently arrived at our three nouns and named two of them the
same way**: an Experience is a configuration of a runnable session, a Map is
where, and Game Feature Plugins are the content bundles an Experience switches
on. Notably, Lyra's Experience is NOT "the game" — it is closer to "the mode you
are about to play", which is exactly how Ambition uses it and exactly not how the
word reads to a first-time reader.

Sources: [X157 Dev Notes — Lyra Experience](https://x157.github.io/UE5/LyraStarterGame/Experience/)
and [Unrealcode.net — Looking into Lyra: Experiences](https://www.unrealcode.net/LyraPart1.html).
⚠ **both third-party.** Lyra's internals are documented far better by the
community than by Epic; treat the class names as accurate-as-of-writing and the
concepts as solid.

---

## Unity — collapses the middle term

| our term | Unity |
|---|---|
| provider | **Package** (UPM, `com.company.feature`) / Asset Bundle |
| experience | *no equivalent* — a Scene is both the kind and the instance |
| route | **Scene name / build index**, or an **Addressables** address |

Unity loads a scene by a key that IS its address:

> "To load a scene by address, you use `Addressables.LoadSceneAsync(key,
> LoadSceneMode.Additive)` where the key is an address string."

⚠ **the vocabulary lesson here is Unity's, not ours**: they named the addressing
layer *Addressables* — the address is explicitly a separate thing from the asset
addressed. That is our `route`, and it is the one term of our three that nobody
should rename.

What Unity does NOT have is a name for "what kind of session this is". A Scene is
its own rules; two screens sharing one implementation is a prefab-and-script
convention, not a first-class concept.

Source: [Load a scene | Addressables](https://docs.unity3d.com/Packages/com.unity.addressables@2.0/manual/LoadingScenes.html)
(Unity, official).

---

## Godot — collapses it further

| our term | Godot |
|---|---|
| provider | **Plugin / addon**, PCK |
| experience | *no equivalent* |
| route | **`res://` resource path** |

> `get_tree().change_scene_to_file("res://your_scene_path.tscn")` — "loads a new
> scene, removes the current scene from the tree immediately, and adds the new
> scene at the end of the frame".

The path IS the address and the scene IS the thing. Godot has no separate notion
of "which rules is this session running", and does not appear to want one.

Source: [SceneTree — Godot Engine (stable)](https://docs.godotengine.org/en/stable/classes/class_scenetree.html)
(Godot, official). ⚠ the method was `change_scene` in Godot 3 and
`change_scene_to_file` in Godot 4 — a small reminder that spellings age faster
than concepts.

---

## What this changed

**The rename is off** (Jon, 2026-08-07: *"We don't need to do the rename. Having
a finer grained vocabulary is probably the right way to do it."*), and the
related work is why the earlier recommendation was wrong.

The draft recommendation had been `experience` → `surface`, on the grounds that
"experience" reads like "the game". Related work says otherwise:

* **Two of three engines have no word for the middle concept at all**, because
  they do not separate it. Having the concept is the finer-grained position, and
  it is the one Unreal takes too.
* **The engine that DOES separate it calls it an Experience.** Renaming away from
  the only prior art's term, on taste, would have been churn in the wrong
  direction.

⚠ **the term that is genuinely weak is `provider`** — it is the one a player
would never say, it is the one carrying the meaning Jon expected "experience" to
have, and both engines that have the concept name it better (Unreal's *Game
Feature*, Unity's *Package*). Recorded, not acted on: no churn for now.

⚠ **and one real defect surfaced by looking**: Ambition uses the SAME STRING for a
provider id and an experience id at most call sites — smash registers its audio
fragment under `SMASH_EXPERIENCE`, and
`ambition_game_shell/src/session.rs:570` defaults the audio provider to
`activation.experience_id` when none is named. Unreal keeps Game Feature and
GameMode as separate types that cannot be confused. Two distinct concepts of ours
happen to agree today, which is how a distinction gets quietly lost.

## ⛔ A citation that failed its check

The first draft attributed Unreal's `?game=` URL option to
[Travelling in Multiplayer](https://dev.epicgames.com/documentation/en-us/unreal-engine/travelling-in-multiplayer-in-unreal-engine).
Fetched: that page does not document it. The claim was TRUE and the source was
WRONG — the option is documented on the Game Mode page, cited above.

Recorded because it is the failure mode this section is most exposed to: a
citation nobody follows is indistinguishable from a citation that does not say
what it is claimed to say, and the second kind is how a plausible wrong fact gets
laundered into a reference.
