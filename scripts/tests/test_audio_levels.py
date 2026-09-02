"""What `scripts/audio_levels.py` must not silently get wrong.

Only three things are pinned, and each one is a way the instrument could report
a confident number that is false:

* the bank reader agrees with the bank's own stored metadata;
* the procedural-spec extractor still finds every provider's specs — a regex
  over Rust is the fragile part, and its failure mode is an empty list, which
  makes every downstream "no outliers" conclusion vacuously true;
* the synthesizer port puts a cue's body at exactly the authored `volume` of
  the engine's procedural reference level, whatever the cue is made of — the
  analytic invariant the whole SFX ranking rests on, and the one the engine
  changed `volume` from a peak to a loudness in order to have.

Nothing here tests that the script prints, formats, or writes a file.
"""

from __future__ import annotations

import dataclasses
import io
import math
import re
import sys
from pathlib import Path

import pytest

REPO_ROOT = Path(__file__).resolve().parent.parent.parent
sys.path.insert(0, str(REPO_ROOT / 'scripts'))

import audio_levels as al  # noqa: E402


def _soundfile():
    """The audio tooling's decoder, reached through `importorskip`.

    ⛔ A MISSING OPTIONAL TOOL IS "NOT SET UP HERE", NOT A FAILING AUDIO BANK.
    `soundfile` is declared by `tools/ambition_music_renderer` and
    `tools/ambition_sfx_renderer`, so a machine without the audio tooling simply
    does not have it — and a bare `import soundfile` turned that into SEVEN red
    tests that read exactly like the shipped audio being wrong. This file already
    skips when the bank asset is absent; it guarded one precondition and not the
    other, and the ungarded one is the likelier of the two to be missing.

    ⚠ The point is not tidiness. A lane that reports seven failures for "you did
    not install a renderer" teaches its reader to skim past it, and the next real
    red goes with them.
    """
    return pytest.importorskip("soundfile")

def test_the_shipped_bank_agrees_with_the_metadata_it_carries():
    """A bank slice that is off by one byte still decodes; it decodes as garbage.

    The packer writes each clip's peak and RMS at pack time. Re-deriving them
    from the sliced payload is the cheapest proof that the slice boundaries are
    the ones the packer wrote.
    """
    bank = al.declared_asset_roots()['assets'] / 'audio' / 'sfx.bank'
    if not bank.exists():
        pytest.skip(f'{bank} is a git-ignored binary asset and is absent here')

    import numpy as np

    sf = _soundfile()

    entries = al.read_bank(bank)
    assert len(entries) > 100, 'the shipped bank is not nearly empty'

    # THE LEVEL COMPARISON ONLY MEANS ANYTHING FOR A LOSSLESS PAYLOAD. The bank carries both
    # WAV/PCM_16 and OGG/VORBIS clips, and Vorbis is not sample-exact: re-decoding it returns a
    # *similar* waveform, not the one the packer measured.
    #
    # The bank's contents shifted, one landed inside it, and a test that was always scoped wrong
    # started failing while nobody had touched it or the packer.
    #
    # so it now checks EVERY lossless entry rather than the first 24 — both
    # honest about what it can prove and about ten times stronger.
    lossless = []
    for entry in entries:
        if sf.info(io.BytesIO(entry.payload)).format != 'WAV':
            continue
        lossless.append(entry)
        frames, rate = sf.read(io.BytesIO(entry.payload), dtype='float64', always_2d=True)
        assert rate == entry.sample_rate
        assert frames.shape[1] == entry.channels
        peak_db = 20.0 * np.log10(np.abs(frames).max())
        rms_db = 20.0 * np.log10(np.sqrt(np.mean(frames**2)))
        assert peak_db == pytest.approx(entry.stored_peak_db, abs=0.05), entry.sfx_id
        assert rms_db == pytest.approx(entry.stored_rms_db, abs=0.05), entry.sfx_id

    # without this the whole test passes on a bank of pure Vorbis, having
    # proved nothing at all.
    assert len(lossless) > 100, (
        f'only {len(lossless)} lossless entries were examined — the slice-boundary '
        'proof needs sample-exact payloads, and a bank without them cannot supply it'
    )


def _provider_files_that_author_specs() -> list[Path]:
    """Game providers only.

    `crates/ambition_audio/src/spec.rs` also contains an `SfxSpec { .. }` — the
    type's own `Default` impl, which is not a sound anybody hears and carries
    neither a cue nor an id. Sounds are authored by providers.
    """
    found = []
    for rs in sorted((REPO_ROOT / 'game').rglob('src/**/*.rs')):
        if rs.name == 'tests.rs':
            continue
        text = al._strip_test_modules(al._strip_comments(rs.read_text(errors='replace')))
        if re.search(r'SfxSpec\s*\{', text):
            found.append(rs)
    return found


def test_every_source_that_authors_a_spec_yields_at_least_one_resolved_spec():
    """The extractor's failure mode is silence, so absence is what is asserted.

    ⛔ "zero unresolved fields" passes trivially on an empty list. The load-
    bearing half is that every file which *contains* an `SfxSpec` literal
    produces specs — that is what catches a provider refactoring its helper out
    from under the regex and vanishing from the report.
    """
    authoring = _provider_files_that_author_specs()
    assert authoring, 'no provider authors an SfxSpec — the search itself is broken'

    for path in authoring:
        specs = al.extract_rust_specs(path)
        named = [s for s in specs if s.sfx_id != '<unknown>']
        assert named, f'{path.relative_to(REPO_ROOT)} authors SfxSpec but yielded no spec'

    everything = al.discover_procedural_specs()
    assert len(everything) >= 40, f'only {len(everything)} procedural specs found'
    unresolved = [(s.owner, s.sfx_id, s.unresolved) for s in everything if s.unresolved]
    assert not unresolved, f'fields the extractor could not evaluate: {unresolved}'
    for spec in everything:
        assert spec.waveform in al.WAVEFORMS, (spec.owner, spec.sfx_id, spec.waveform)
        assert 0.0 < spec.duration < 10.0, (spec.owner, spec.sfx_id, spec.duration)
        assert 0.0 < spec.volume <= 1.0, (spec.owner, spec.sfx_id, spec.volume)
        assert spec.sample_rate >= 8000, (spec.owner, spec.sfx_id, spec.sample_rate)


def _probe(waveform: str, volume: float) -> al.ProceduralSpec:
    return al.ProceduralSpec(
        sfx_id='probe',
        owner='test',
        source='test',
        waveform=waveform,
        frequency=600.0,
        frequency_end=900.0,
        duration=0.2,
        volume=volume,
        attack=0.005,
        release=0.05,
        noise=0.2,
        sample_rate=44100,
    )


def _rendered(spec: al.ProceduralSpec):
    import numpy as np

    sf = _soundfile()

    frames, rate = sf.read(io.BytesIO(al.synthesize(spec)), dtype='float64', always_2d=True)
    assert rate == spec.sample_rate
    assert frames.shape[1] == 2, 'the Rust writes the same sample to both channels'
    return np.asarray(frames[:, 0])


def _rms_db(spec: al.ProceduralSpec) -> float:
    import numpy as np

    return float(20.0 * np.log10(np.sqrt(np.mean(_rendered(spec) ** 2))))


def _peak(spec: al.ProceduralSpec) -> float:
    import numpy as np

    return float(np.abs(_rendered(spec)).max())


@pytest.mark.parametrize('waveform', sorted(al.WAVEFORMS))
def test_a_synthesized_cue_lands_on_the_reference_level_whatever_its_waveform(waveform):
    """`volume` is a loudness trim, and loudness is RMS.

    This is the invariant every procedural row in the report is computed
    against, so it is pinned rather than assumed. ⚠ the *whole-clip* RMS is
    below the target by however much the envelope removes, which is why the
    probe is measured with the envelope off — the target is a property of the
    cue's body, not of the shape wrapped around it.
    """
    unenveloped = dataclasses.replace(_probe(waveform, 1.0), attack=0.0, release=0.0)
    assert _rms_db(unenveloped) == pytest.approx(al.PROCEDURAL_CUE_REFERENCE_RMS_DBFS, abs=0.05)

    for volume in (0.16, 0.25, 0.5):
        trimmed = dataclasses.replace(unenveloped, volume=volume)
        expected = al.PROCEDURAL_CUE_REFERENCE_RMS_DBFS + 20.0 * math.log10(volume)
        assert _rms_db(trimmed) == pytest.approx(expected, abs=0.05), waveform
        assert _peak(trimmed) <= 1.0, f'{waveform} clipped at volume {volume}'


def test_the_noise_mix_does_not_move_a_cue_off_its_target():
    """Noise lowers RMS while leaving the peak at 1, so a peak-domain `volume`
    made an airy cue quieter than a clean one at the same number. It no longer
    does, and the report's per-owner deltas depend on that being true."""
    levels = [
        _rms_db(dataclasses.replace(_probe('Saw', 0.4), attack=0.0, release=0.0, noise=noise))
        for noise in (0.0, 0.25, 0.7, 1.0)
    ]
    assert max(levels) - min(levels) < 0.1, levels


def test_a_quieter_authored_volume_measures_quieter():
    """Poison for the above: 'every cue hits the target' is also satisfied by a
    synthesizer that ignores `volume` entirely."""
    loud = _rms_db(_probe('Square', 0.5))
    quiet = _rms_db(_probe('Square', 0.16))
    assert loud - quiet == pytest.approx(20.0 * math.log10(0.5 / 0.16), abs=0.05)


def _sfx(name: str, rms: float, owner: str = 'someone') -> al.Item:
    """One packed SFX item with just the fields the finding reads."""
    return al.Item(
        key=name,
        cohort='sfx_packed',
        name=name,
        owner=owner,
        origin=f'<synthetic>/{name}',
        metrics={'rms_db': rms, 'dbtp': rms + 6.0, 'duration_s': 0.2},
    )


def test_the_loudest_sound_finding_can_actually_fire():
    """A planted loudness outlier must appear in the finding by name and owner."""
    quiet = [_sfx(f'quiet.{n}', -24.0 + (n % 5) * 0.5) for n in range(40)]
    screamer = _sfx('a.screamer', -6.0, owner='loud_provider')
    lines = al._loudest_sounds_finding({'sfx': quiet + [screamer]})
    body = '\n'.join(lines)
    assert 'a.screamer' in body, f'the planted outlier was not reported:\n{body}'
    assert 'loud_provider' in body, 'the report must name who owns it'
    assert 'quiet.0' not in body, 'an ordinary sound must not be flagged'


def test_the_loudest_sound_finding_stays_quiet_on_an_even_population():
    """The poison: a population with no outlier must say so rather than always
    naming its top row. A "loudest sound" section that always fires is a
    ranking, not a finding, and would train the reader to ignore it."""
    even = [_sfx(f'even.{n}', -24.0 + (n % 7) * 0.4) for n in range(40)]
    body = '\n'.join(al._loudest_sounds_finding({'sfx': even}))
    assert 'No single sound sits' in body, f'a flat population was flagged:\n{body}'
