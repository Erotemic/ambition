"""What `scripts/audio_levels.py` must not silently get wrong.

Only three things are pinned, and each one is a way the instrument could report
a confident number that is false:

* the bank reader agrees with the bank's own stored metadata;
* the procedural-spec extractor still finds every provider's specs — a regex
  over Rust is the fragile part, and its failure mode is an empty list, which
  makes every downstream "no outliers" conclusion vacuously true;
* the synthesizer port peaks at exactly the authored `volume`, which is the
  analytic invariant the whole SFX ranking rests on.

Nothing here tests that the script prints, formats, or writes a file.
"""

from __future__ import annotations

import io
import re
import sys
from pathlib import Path

import pytest

REPO_ROOT = Path(__file__).resolve().parent.parent.parent
sys.path.insert(0, str(REPO_ROOT / 'scripts'))

import audio_levels as al  # noqa: E402


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
    import soundfile as sf

    entries = al.read_bank(bank)
    assert len(entries) > 100, 'the shipped bank is not nearly empty'

    for entry in entries[:24]:
        frames, rate = sf.read(io.BytesIO(entry.payload), dtype='float64', always_2d=True)
        assert rate == entry.sample_rate
        assert frames.shape[1] == entry.channels
        peak_db = 20.0 * np.log10(np.abs(frames).max())
        rms_db = 20.0 * np.log10(np.sqrt(np.mean(frames**2)))
        assert peak_db == pytest.approx(entry.stored_peak_db, abs=0.05), entry.sfx_id
        assert rms_db == pytest.approx(entry.stored_rms_db, abs=0.05), entry.sfx_id


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


def _peak(spec: al.ProceduralSpec) -> float:
    import numpy as np
    import soundfile as sf

    frames, rate = sf.read(io.BytesIO(al.synthesize(spec)), dtype='float64', always_2d=True)
    assert rate == spec.sample_rate
    assert frames.shape[1] == 2, 'the Rust writes the same sample to both channels'
    return float(np.abs(frames).max())


@pytest.mark.parametrize('waveform', sorted(al.WAVEFORMS))
def test_a_synthesized_cue_is_authored_volume_times_a_waveform_constant(waveform):
    """`volume` is the ceiling AND the only per-cue scale factor.

    Tone and noise are both bounded by ±1 and the envelope only attenuates, so
    the peak can never exceed `volume`. ⚠ it does not always REACH it either:
    a discretely sampled saw or triangle misses its apex by up to one sample
    step. What is exact is the linearity — doubling `volume` doubles the peak —
    and that is what "Sanic is N dB hotter than the cohort" reduces to, so it is
    pinned rather than assumed.
    """
    ratios = []
    for volume in (0.16, 0.25, 0.5):
        peak = _peak(_probe(waveform, volume))
        assert peak <= volume + 1e-9, f'{waveform} exceeded its authored volume'
        assert peak > 0.9 * volume, f'{waveform} produced near-silence at volume {volume}'
        ratios.append(peak / volume)
    assert max(ratios) - min(ratios) < 1e-6, f'{waveform} peak is not linear in volume: {ratios}'


def test_a_quieter_authored_volume_measures_quieter():
    """Poison for the above: linearity alone is satisfied by a constant."""
    loud = _peak(_probe('Square', 0.5))
    quiet = _peak(_probe('Square', 0.16))
    assert loud > quiet * 2.5
