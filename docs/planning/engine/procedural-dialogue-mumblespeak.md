# Procedural dialogue voices (`ambition_mumblespeak` candidate)

**Priority:** Low. Explore with a small audio prototype when dialogue presentation becomes a product focus. Do not create a crate or move active queue work yet.
**Scope:** Speech-like NPC dialogue sounds without recorded lines or spoken words. The effect is a presentation choice; it does not speak for the actor or decide dialogue content.
**Source baseline:** Ambition snapshot `102cbf1`, 2026-10-16. Recheck integration owners before implementation.

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

## Desired player experience

Give each speaking character a stable sound identity. The sound should follow the displayed words without trying to pronounce them as TTS. A listener should hear when a character hesitates, asks a question, shouts, whispers, or shifts mood. The player must still be able to read or skip every line at their own pace.

A line should remain recognizable when it repeats. A character's identity should survive different lines, emotions, languages, and speaking rates. The sound should not become a stream of sharp per-letter clicks. There must be clear, quiet presentation settings and a fully silent path.

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

Build a pure Rust experiment or an offline audio prototype for three distinct voice profiles and a short set of lines. Include a statement, a question, a comma pause, ellipsis, emphasized word, proper name, and fast text. Render WAV previews and an event-timeline dump. Compare:

- existing five-character blip rhythm;
- a basic formant/vowel-onset design based on UnitOfTime's video;
- one variant with phrase prosody and controlled timbral transitions.

Listen at ordinary SFX volume on desktop speakers, headphones, and a mobile speaker. Keep a small listening rubric: speaker identity, text rhythm, fatigue, intelligibility of intent, repeated-line consistency, and absence of clicks. Do not create a new crate from a weak listening result.

### M1 — One reveal plan and cancellation contract

If M0 is promising, define the shared line/reveal event interface. Make the current blip playback an adapter to the same reveal facts. Preserve existing user input and Yarn authority. Define line-generation identity, safe Unicode/grapheme indexing, clock behavior, punctuation waits, and exact cancellation on skip/close/interrupt. Handle a frame that reveals many graphemes without emitting a burst of stale sounds. Use bounded catch-up or drop policy that keeps the sound aligned with the *visible* text.

The player can skip the line and hear no trailing synthesized speech. A pause, slow-motion mode, or render-frame stutter must not leave the voice running after the visible line finishes. Keep the reveal clock in presentation time; game rollback does not own it.

### M2 — Pure planner and DSP crate

Create `crates/ambition_mumblespeak` only after the pure plan and renderer have a useful API and an independent fixture can use them without the Ambition game provider. The crate should expose deterministic phrase-event generation, voice-profile validation, bounded DSP rendering, and optional WAV preview export. It should not import Bevy, Yarn, Ambition character IDs, asset-server handles, or a network TTS dependency.

A host adapter converts planner events to the normal audio device/mixer. It must support a headless/no-audio configuration without affecting dialogue state.

### M3 — Provider content and a real dialogue customer

Extend the existing `dialogue_voiceprints` family with one authorable voice specification and a stable voice ID. Keep speaker resolution in `DialogueVoiceCatalog` or a typed successor owned by the same boundary. Support authored fallback to existing blip cues during the evaluation, then migrate the selected cast to one schema and remove redundant paths.

Attach the generator to one existing dialogue presenter and one independent demo/default presenter. The two consumers must not import each other's cast. Verify hot reload swaps voice profiles only for new or safely restarted line instances; it must not change a line already emitting samples by reading an unpinned mutable profile.

### M4 — Product tuning and optional features

Only after M3: add localization strategies, non-human timbres, per-speaker expression, spatial treatment, author-time preview, and quality budgets as measured needs. Keep these as independent knobs over the same planner and mixer. Do not add a full speech editor, mandatory renderer service, or offline model just because the feature allows it.

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

**Priority rule:** Keep this out of the active queue until a dialogue polish or accessibility/customer review calls for it. The first investment is a bounded listening prototype, not a wholesale dialogue system rewrite.
