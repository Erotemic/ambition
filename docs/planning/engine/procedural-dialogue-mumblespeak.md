# Procedural dialogue voices (`ambition_mumblespeak` candidate)

**Priority:** Deferred; outside the execution queue.
**Scope:** Speech-like NPC dialogue sounds without recorded lines or spoken words. The effect is a presentation choice; it does not speak for the actor or decide dialogue content.
**Source baseline:** Ambition snapshot `102cbf1`, 2026-10-16. Recheck integration owners before implementation.

**Execution gate (maintainer ruling, 2026-10-16):** Do not implement, prototype, benchmark, add dependencies, add a crate, or schedule any work from this document unless one of these conditions is true:

1. The maintainer explicitly asks to implement **this specific proposal**.
2. All other work in the `docs/planning/` engineering queue is finished. There must be literally no other work left to select. A blocked, paused, or temporarily unassigned queue item does not count as finished.

Before case 2, check the current [`docs/planning/queue.md`](../queue.md) and its linked open work. Do not use a measurement, an adjacent task, or a promising experiment as permission to start. Research and planning edits alone do not authorize code, tool, content, or test changes. The work slices below are a record of possible future work, **not** an autonomous work order.

## Inspiration and attribution

**UnitOfTime (author), [*I Gave My NPCs Voices Without Recording Anything*](https://www.youtube.com/watch?v=NaW0d7Tghw0), a Mythfall development video.** The maintainer supplied the video transcript on 2026-10-16. This is the specific source that motivated the planned feature.

Credit UnitOfTime directly when discussing or presenting the design. The video supplies a clear demonstration of these ideas:

| Video time | Idea demonstrated |
| --- | --- |
| 1:10-1:52 | Drive sound from the same timed text-reveal events that the reader sees. |
| 2:19-4:35 | Build vowel-like sounds from a buzzing source and resonant formants; use a soft attack/hold/release envelope, breath noise, and small periodic variation. |
| 4:40-6:02 | Give speakers independent pitch, vocal-tract/formant scale, and controlled modulation. Do not treat pitch alone as voice identity. |
| 6:04-6:45 | Sound vowel-group starts rather than every character. Use the text to choose a repeatable vowel sound. |
| 6:48-7:43 | Shape each phrase with punctuation, emphasis, pitch fall/rise, and a fade for trailing text. |
| 7:45-8:45 | Overlap short vowel units slightly, place authored effects at exact reveal points, and stop voice output when a line is skipped or fully revealed. |

The video also describes earlier styles in *Undertale*, *Banjo-Kazooie*, and *Animal Crossing*. These are examples of the wider dialogue-sound tradition, not claims that UnitOfTime invented all mumble speech. This proposal extends the specific combination and design lessons presented in the Mythfall video.

**Provenance rule:** Implement new DSP, text analysis, and assets independently. Do not copy Mythfall code or audio, and do not assume any video code has a license for reuse. If a later implementation imports third-party code, samples, or data tables, check its license and record separate source attribution. Retain the UnitOfTime video credit in the feature's implementation notes and player-facing credits or release notes when the feature ships. Attribution is a credit, not an assertion of an exclusive claim to the general technique.

## Further prior art and what it contributes

The source above remains the feature's direct inspiration. The following sources are **separate** references for specific mechanisms, tools, or presentation behavior. Cite a source when using its specific work. None of these sources authorizes copying game audio, samples, art, or code without checking its license.

| Source | Useful idea for Ambition | Limit |
| --- | --- | --- |
| The *Undertale*, *Banjo-Kazooie*, and *Animal Crossing* examples in [UnitOfTime's video](https://www.youtube.com/watch?v=NaW0d7Tghw0) (0:30-1:06) | Three styles: fixed per-character tone, short grunt banks, and fast phonetic sprites. | Examples of prior game practice, not a license to copy sound assets or imitate a game's identifiable character voices. |
| [Neil Thapen's Pink Trombone](https://www.imaginary.org/program/pink-trombone) and the [Rust port by Victor Nova / lostmsu](https://github.com/lostmsu/pink-trombone) | Continuous vocal-tract shape changes instead of disconnected static vowels. | Treat a vocal-tract model as an optional synthesis backend. It may cost more CPU than a few filters. Attribute Neil Thapen and the Rust port if their code or model is used. |
| [Rhubarb Lip Sync, Daniel Wolf](https://github.com/DanielSWolf/rhubarb-lip-sync) | Small, timed 2D mouth-shape sets and export formats (TSV/JSON). | Rhubarb infers mouth shapes *from recorded audio*. Generated speech plans already know their sound units; do not add a recognizer to recreate known timing. |
| [Yarn Spinner: Typewriter and Effects](https://yarnspinner.dev/docs/godot/gdscript/04-showing-dialogue/04-typewriter-and-effects/) | One reveal clock, skip-to-end behavior, variable reveal speed, punctuation pauses, and cancellation. | Presentation reference, not a reason to replace Ambition's Yarn bridge or presenter choice. |
| [Dialogic 2: Text Effects](https://docs.dialogic.pro/text-effects.html) | Text-position-anchored cues for pause, speed, portraits, and sound mood. | Do not invent a second markup grammar if current Yarn markup and presentation facts can express the cue. |
| [W3C SSML 1.1](https://www.w3.org/TR/speech-synthesis/) | Vocabulary for `break`, `emphasis`, `prosody`, and pronunciation overrides. | Borrow concepts only. Full SSML parsing and spoken TTS are not requirements. |
| [Unicode Standard Annex #29](https://www.unicode.org/reports/tr29/) | Stable boundaries for visible grapheme clusters, words, and sentences. | Text boundary detection is not a language-specific pronunciation rule. |
| [CMU Pronouncing Dictionary](https://github.com/cmusphinx/cmudict) | English phonemes and stress for an optional phoneme-aware strategy. | English-only lexicon; keep the exact dictionary's copyright notice and attribution with redistributed data. |
| [Game Accessibility Guidelines: player-paced text](https://gameaccessibilityguidelines.com/allow-players-to-progress-through-text-prompts-at-their-own-pace/) | Let players read, advance, and disable/adjust the effect. | Presentation quality must remain good without audio. |

### Presentation direction beyond voice-only blips

A later dialogue presenter may consume one **shared line-performance plan** with speech onsets, visible text spans, optional mouth poses, expression cues, and gesture anchors. Start with 3-5 mouth poses if sprite art supports them. Derive their timing from the same events as the voice, not by running audio recognition on synthetic output. Optional brows, eyes, portrait changes, and modest body gestures belong to the **dialogue presentation adapter**, not to the pure audio crate. Do not make a full animation system a prerequisite for better dialogue sound.

This can also support character-specific non-human instruments, phrase motifs, a voice-preview editor, and precomputed line-performance plans. Those are experiments, not mandatory features. Keep author overrides small, stable, and local to presentation. Let the player change text speed, instantly reveal a line, mute procedural voice, or use reduced motion without affecting dialogue decisions.

## Rust reuse survey (investigate before implementing DSP or speech analysis)

The following are **candidates**, not approved dependencies. Versions and licenses were checked against docs.rs or upstream on 2026-10-16; check them again if the execution gate opens. Prefer existing Ambition audio and text dependencies. The key question is whether a candidate integrates with the current audio mixer and a shared reveal timeline without owning a new playback clock or device.

| Rust option | What might work with little code | Adoption decision / caveat |
| --- | --- | --- |
| **Already present:** [`bevy_kira_audio` 0.26](https://docs.rs/bevy_kira_audio/0.26.0/bevy_kira_audio/) and Kira | Reuse Ambition's current audio routing, mixer, cue lifetime, and gain controls. | **Keep this playback path.** The crate owns audio-device integration, not phonetic planning or synthesis. Do not add a second mixer or independent output device. |
| **Already in `Cargo.lock`:** [`unicode-segmentation`](https://docs.rs/unicode-segmentation/latest/unicode_segmentation/) | Extended grapheme and word indices for text event anchors. | **First choice** for boundaries; an indexed grapheme is not a phoneme or spoken syllable. |
| [`animalese` 0.2](https://docs.rs/animalese/latest/animalese/) (MIT) | Ready-to-play sample-based Animalese, voice profiles, intonation, and a command-line listening reference. | **Closest direct baseline**, but it bundles audio assets and uses an older Kira (`0.9`, versus Ambition's `0.12`). Review the actual assets' origin and redistribution rights. Do not use its independent playback path or bundled samples without review. Not the same as recorded-sample-free formant synthesis. |
| [`pink-trombone` 0.2.1](https://docs.rs/pink-trombone/latest/pink_trombone/) (MIT) | Rust vocal-tract waveguide generator with an `f32` audio output. | **Candidate if better articulation is audible.** Published in 2021, with little API documentation. Test CPU, control latency, stability, and web/mobile behavior before use. |
| [`fundsp` 0.23](https://docs.rs/fundsp/latest/fundsp/) (MIT OR Apache-2.0) | Oscillators, noise, filters, envelopes, and composable audio DSP nodes. | **Candidate general DSP toolkit**; may remove most custom signal code. Compare integration cost to a smaller filter library. Avoid an independent real-time graph scheduler. |
| [`biquad` 0.6](https://docs.rs/biquad/latest/biquad/) (MIT OR Apache-2.0) | Second-order filters and coefficient updates for formant bands. | **Small baseline** if the design only needs a voiced source and a few resonant bands. Verify smooth retuning and no audio-callback allocations. |
| [`dasp` 0.11](https://docs.rs/dasp/latest/dasp/) | PCM sample, frame, signal, and noise primitives. | Optional utility, not a finished speech synthesizer. Check whether `fundsp` or Ambition's mixer already covers the use. |
| [`hypher` 0.1](https://docs.rs/hypher/latest/hypher/) | Fast multilingual word hyphenation without runtime dictionary loading. | Potential rhythm/word-break heuristic. **Hyphenation is not phonetic syllabification**; test it against spoken stress rather than treating each break as a vocal onset. |
| [`cmudict` 0.3](https://docs.rs/cmudict/latest/cmudict/) (MIT/Apache-2.0 library) | Look up English phonemes and stress from a CMUdict file. | Its `download()` API uses the network; **never use that path in-game**. If chosen, bundle a pinned dictionary with its own BSD-style notice and provide fallback for names/OOV words. Review old dependencies. |
| [`loqui-g2p` 0.3](https://docs.rs/loqui-g2p/latest/loqui_g2p/) (MIT) | English G2P with embedded dictionaries and unknown-word fallback. | Larger model/lexicon payload (crates.io source package about 20 MB). Evaluate offline preparation only if phonemes improve listening enough to justify the cost. Preserve third-party notices. |
| [`hound` 3.5](https://docs.rs/hound/latest/hound/) | Simple WAV export and import for offline A/B listening reports. | Suitable for an authoring tool; not needed in the runtime audio graph. |
| [`espeak-ng` 0.2](https://docs.rs/espeak-ng/latest/espeak_ng/) (GPL-3.0-or-later) | Pure Rust phonemization and formant TTS, useful as an offline research baseline. | **Do not add as an Ambition runtime dependency** without an explicit maintainer license decision. Its GPL terms and more complex speech model need separate review. |

**Default reuse order, only after the execution gate:** (1) compare the existing blips with `animalese` and a formant baseline in an isolated listening tool; (2) use existing Kira plus `unicode-segmentation`, then prefer `biquad` *or* `fundsp` for the simplest good result; (3) evaluate `pink-trombone` if the small source-filter sound cannot meet the listening target; (4) add a phonetic dependency only if English spelling errors are audible enough to justify its cost. Do not add all candidates.

**Adoption checks:** Verify license and **asset/data** provenance separately, native/web/Android builds, real-time safety, worst-case CPU and allocations, sample-rate conversion, preallocation, headless behavior, startup time, incremental binary size, determinism of phrase events, and skip/cancel semantics. Measure the complete cost in Ambition, not just a crate's example. If a crate already meets the performance and presentation requirements, wrap it behind the planned pure interface rather than reimplementing its algorithms or adding a competing audio road.

## Desired player experience

Give each speaking character a stable sound identity. The sound should follow the displayed words without trying to pronounce them as TTS. A listener should hear when a character hesitates, asks a question, shouts, whispers, or shifts mood. The player must still be able to read or skip every line at their own pace.

A line should remain recognizable when it repeats. A character's identity should survive different lines, emotions, languages, and speaking rates. The sound should not become a stream of sharp per-letter clicks. There must be clear, quiet presentation settings and a fully silent path. Text reveal must have a player-controlled speed and an instant-reveal option.

**Non-goals:** Full spoken-language TTS, voice cloning, recorded actor replacement, online inference, lip-synchronized dialogue as a first requirement, and any change to authoritative NPC decisions or conversation rules. All platforms must work without a network service or microphone.

## Existing Ambition foundation: extend, do not replace

- `crates/ambition_dialog/src/runtime.rs`: `DialogState` and `LineRevealState` hold the active line and its visible prefix. Current reveal uses a fixed 112.5 characters/s and Unicode character boundaries; it does not yet schedule punctuation pauses or grapheme clusters.
- `crates/ambition_dialog/src/systems.rs`: `dialog_reveal_tick` advances the reveal using presentation time and emits a `SfxMessage::Play` when the old blip selector fires.
- `crates/ambition_dialog/src/speech_sfx.rs`: the present rule emits at most one blip per frame when the alphanumeric count crosses a multiple of five. It already avoids an unchecked one-beep-per-letter burst, but it does not select a phonetic sound from the text.
- `crates/ambition_sfx/src/voice.rs`: `DialogueVoiceCatalog` resolves speaker labels and dialogue IDs to SFX cues.
- `crates/ambition_audio/src/content_schema/voiceprints.rs` and `game/ambition_content/assets/data/dialogue_voiceprints.ron`: the `dialogue_voiceprints` content family is the existing provider-owned list of character voices. It is also covered by content reload. Extend this family rather than add a competing cast/voice table.
- `crates/ambition_sim_view/src/dialog_view.rs` and ADR [0028](../../adr/0028-dialogue-presentation-is-provider-selected.md): games select their own dialogue UI presenter. A voice effect must not force a specific dialogue panel or move presentation timing into gameplay simulation.
- `tools/ambition_sfx_renderer` is a separate offline authoring submodule. It may help with listening prototypes or generated previews. Do not make the game depend on its Python/audio production tools.

**Ownership:** Yarn/authoritative conversation state chooses *what* a character says. The dialogue reveal model chooses *which text becomes visible when*. The voice planner turns reveal facts and a voice profile into short, reproducible performance events. The audio backend generates or plays sound. Muting or failing to render sound cannot change dialogue progression, save data, collision, or AI decisions.

## One reveal timeline, two output consumers

```text
Yarn / conversation -> DialogState line, speaker, style
                            |
                 one timed reveal plan
             grapheme positions + pause spans
                     /                  \
          displayed text             voice events
                                      |
                          deterministic phrase plan
                                      |
                           synthesis / audio mixer
```

The text and audio must use **one** timing authority. Do not run a second clock that can drift. Do not let sound rendering determine when Yarn advances.

Design the pure planner around an indexed, immutable line. A candidate public API is:

- `MumbleVoiceProfile`: stable voice identity and optional timbre parameters (base pitch, formant scale, breath, roughness, articulation, register, vibrato, gain).
- `MumbleExpression`: temporary delivery intent (normal, soft, urgent, questioning, angry, tired, playful). The currently authored normal/whisper/shout styles map to this, without inventing character emotions from private simulation state.
- `DialogueRevealPlan`: grapheme spans and presentation timing for a chosen line and locale; punctuation and explicit markup may modify dwell and pauses.
- `MumblePhrasePlan`: a bounded series of sound events with reveal-span anchors, intended onset, vowel/formant target, duration, pitch/envelope shape, gain, and a stable variation seed.
- `MumbleRenderer`: a small DSP backend that renders the planned events to bounded PCM blocks or short cached samples. It never accesses the Bevy World or authoritative state.

These are design names, not existing types. The planner can be a pure Rust `ambition_mumblespeak` crate with no Bevy, game-cast, or audio-device dependency. A narrow adapter in `ambition_dialog`/`ambition_audio` can consume the plan. Add no new Bevy plugin unless a second integration makes it useful.

The current `SfxMessage::Play { id, pos }` does not carry timed formant or expression parameters. Do not smuggle dynamic voice parameters into a fake SFX ID. Either add a bounded typed presentation-audio event at the existing audio boundary or use a voice-specific adapter that writes through the same mixer. Keep one speaker/voice selection path in `dialogue_voiceprints`. When the new format is ready, migrate the existing cue entries with a deliberate data/schema change rather than keeping two unrelated registries.

## Sound design: baseline from the video

Start with a small, measured source-filter synthesizer:

1. Generate a stable voiced excitation with harmonics. Shape a handful of vowel sounds through three adjustable formant bands. Use independently tunable vocal-tract scale and fundamental pitch.
2. Apply an attack/hold/release envelope, a controlled breath component, and low-amplitude periodic variation. Avoid frame-to-frame random pitch leaps. Give voiced events a bounded overlap so the result can connect across syllables.
3. Segment the line into speech-like sound onsets. An English vowel-run heuristic is an acceptable **prototype only**. Do not hard-code English vowel spelling as the universal text model.
4. Let consonants and silent characters change timing or timbre only when that improves listening results. The video removed click-like consonant bursts for a reason; do not add them merely for physical detail.
5. Use word and punctuation boundaries to set a phrase contour. Questions may rise near the end, statements may fall, commas may reset, ellipses may fade, and authored emphasis may change gain or register. Apply effects through one bounded voice profile, not unlimited layers.
6. Select small, deterministic variation from a stable phrase/voice seed. The same speaker, line, locale, and authored delivery should yield the same event plan regardless of frame rate.

## Extensions worth testing beyond the video

### A. Text-to-sound units that work beyond English spelling

Separate Unicode grapheme segmentation from sound-unit selection. An optional language strategy can analyze vowel sequences, letter pairs, syllable breaks, and script conventions. It may produce approximate pseudo-syllables, not exact phonemes. Unknown scripts and emoji must have a clear silent or neutral fallback. Text itself remains unchanged; do not force localization or text normalization into content identity.

Start with English and a non-English fixture. Require each language strategy to explain its segmentation in a small text/audio preview. Do not pretend that an English vowel-run heuristic works in every language.

### B. Timbral transitions, not isolated vowels

Test short transitions between adjacent vowel targets, formant glides, and a mild nasal or aspirated component. Use transitions to keep a phrase connected rather than only layering longer tones. A voice profile may include controlled vocal fry, airy delivery, synthetic/robotic quantization, chirps, or creature timbres. Keep the number of simultaneous sources bounded. Prefer useful perceptual differences over a large knob catalog.

### C. Factorized identity, delivery, and context

A stable character voice is not the same thing as its current mood or sound location. Keep separate controls for:

- **Identity:** preferred pitch range, formant spacing, spectral color, rhythm tendency, and non-human traits.
- **Delivery:** whisper, urgency, question, interruption, fatigue, or laughter as authored or already-observed presentation facts.
- **Scene presentation:** player-controlled volume, spatial position, distance rolloff, room effects, and speech ducking through the existing audio mixer.

Do not infer identity from a simplistic age/gender preset. Give authors independent parameters and clear preview presets. An actor can be player-controlled, AI-controlled, or scripted without changing the voice path.

### D. Interaction at exact reveal points

Allow optional presentation cues on a line (purr, breath, laugh, cough, mechanical click) with stable grapheme anchors. These cues should use the existing provider SFX catalog. Skip, fast-forward, interruption, a new speaker, scene teardown, and reload must cancel the old line's pending events. Reopening the same text is a new presentation instance, not a replay of stale queued audio.

### E. Prepare speech plans, synthesize only what is heard

A tiny text-derived phrase plan can be compiled or cached by `(line content, locale, voice profile revision, expression, synthesis version, seed)`. Prepare plans at line admission or in authoring tools. Render small PCM blocks on demand or cache bounded fragments. Avoid generating large PCM files for every dynamic line. Generated binary previews may remain ignored outputs, as with other Ambition art/audio assets.

If many listeners or viewers need the same authoritative line, they may each render it according to local settings; audio is not replicated simulation state. If an offline speech-plan artifact is added, it must have versioned inputs and invalidation. Do not make it a dependency of the prevalidated-content work.

## Work slices (not scheduled)

### M0 — Listening prototype, outside gameplay

**This is not authorized until the execution gate opens.** Check the reuse survey and run the cheapest working candidate before writing a new sound engine. Build a pure Rust experiment or an offline audio prototype for three distinct voice profiles and a short set of lines. Include a statement, a question, a comma pause, ellipsis, emphasized word, proper name, and fast text. Render WAV previews and an event-timeline dump. Compare:

- existing five-character blip rhythm;
- a basic formant/vowel-onset design based on UnitOfTime's video;
- one variant with phrase prosody and controlled timbral transitions;
- a direct `animalese` listening baseline if its assets and tool dependencies are acceptable for an isolated evaluation.

Listen at ordinary SFX volume on desktop speakers, headphones, and a mobile speaker. Keep a small listening rubric: speaker identity, text rhythm, fatigue, intelligibility of intent, repeated-line consistency, and absence of clicks. Do not create a new crate from a weak listening result.

### M1 — One reveal plan and cancellation contract

If M0 is promising, define the shared line/reveal event interface. Make the current blip playback an adapter to the same reveal facts. Preserve existing user input and Yarn authority. Define line-generation identity, safe Unicode/grapheme indexing, clock behavior, punctuation waits, and exact cancellation on skip/close/interrupt. Handle a frame that reveals many graphemes without emitting a burst of stale sounds. Use bounded catch-up or drop policy that keeps the sound aligned with the *visible* text.

The player can skip the line and hear no trailing synthesized speech. A pause, slow-motion mode, or render-frame stutter must not leave the voice running after the visible line finishes. Keep the reveal clock in presentation time; game rollback does not own it.

### M2 — Pure planner and DSP crate

Create `crates/ambition_mumblespeak` **only if an existing Rust crate plus a small adapter cannot meet the result**, and only after the pure plan and renderer have a useful API and an independent fixture can use them without the Ambition game provider. The crate should expose deterministic phrase-event generation, voice-profile validation, bounded DSP rendering, and optional WAV preview export. It should not import Bevy, Yarn, Ambition character IDs, asset-server handles, or a network TTS dependency.

A host adapter converts planner events to the normal audio device/mixer. It must support a headless/no-audio configuration without affecting dialogue state.

### M3 — Provider content and a real dialogue customer

Extend the existing `dialogue_voiceprints` family with one authorable voice specification and a stable voice ID. Keep speaker resolution in `DialogueVoiceCatalog` or a typed successor owned by the same boundary. Support authored fallback to existing blip cues during the evaluation, then migrate the selected cast to one schema and remove redundant paths.

Attach the generator to one existing dialogue presenter and one independent demo/default presenter. The two consumers must not import each other's cast. Verify hot reload swaps voice profiles only for new or safely restarted line instances; it must not change a line already emitting samples by reading an unpinned mutable profile.

### M4 — Product tuning and optional features

Only after M3: add localization strategies, non-human timbres, per-speaker expression, spatial treatment, optional shared-timeline mouth/face/gesture cues, author-time preview, and quality budgets as measured needs. Keep these as independent knobs over the same planner and mixer. Do not add a full speech editor, mandatory renderer service, or offline model just because the feature allows it.

## Validation and performance contract

- **Repeatability:** Same line, locale, voice profile, expression, seed, and synthesis revision produce the same ordered phrase events at a fixed reveal policy. Do not require bit-identical output from every hardware audio backend.
- **Synchronization:** Revealed graphemes and sound events use the same timeline. A one-frame reveal jump does not queue a delayed wall of tones. Skip and interruption have bounded release and no stale callback.
- **Unicode:** Emoji, combining marks, multi-byte text, and mixed-script lines do not split visible graphemes or panic. Unsupported scripts degrade to silence or a documented neutral mode.
- **Character identity:** A voice is selected by the provider catalog, not player-versus-NPC branches. Two Apps and two providers may use different catalogs in one process.
- **Audio bounds:** Cap voice count, per-line pending events, rendered block size, peak/gain, and overlap. Measure allocations, CPU time, audio underruns, peak level, and cumulative fatigue on weak/mobile targets. Derive actual budgets from profiles, not assumed numbers.
- **Presentation controls:** Honor master/SFX level and provide a separate dialogue-voice level or disable switch if a shipping customer needs it. Full text remains readable without sound. Avoid sudden loud consonant-like transients.
- **Headless and reload:** Audio optionality does not alter simulation, rollback, save files, active conversations, or their outcomes. A profile reload never borrows an unpinned mutable voice definition.
- **Listening evidence:** Retain short A/B audio previews and a reproducible render command. Judge quality by listening as well as CPU traces; an attractive waveform is not evidence that the voice reads well.

## Open choices at promotion

- Should the reveal plan move from fixed character rate to punctuation-aware dwell now, or only after a controlled comparison with the current game feel?
- Should the runtime render PCM directly or reuse a small generated vowel-unit bank? Compare sound, web/mobile CPU, start latency, and packaging size.
- Where should line-level expression come from: Yarn markup, explicit provider presentation cues, or the already-selected read model? Avoid audio-derived simulation state.
- When a speaker changes mid-line, should one phrase stop or crossfade into a new voice? Choose an observable policy and test interruption.
- Is `ambition_mumblespeak` justified as a stand-alone crate? Require a pure tested API and a second consumer; crate creation is not the first milestone.

**Priority rule:** This entire proposal, including M0, stays deferred. Dialogue polish, an accessibility need, attractive prior art, or a low implementation cost **does not** authorize work. Only the explicit maintainer request or literal exhaustion of all other `docs/planning` queue work in the execution gate can authorize implementation.
