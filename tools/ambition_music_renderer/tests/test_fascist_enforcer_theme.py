from pathlib import Path

import yaml

from ambition_music_renderer.cli import find_score, radio_cues


def test_fascist_enforcer_theme_is_active_radio_cue():
    score_path = find_score('fascist_enforcer_theme')
    assert score_path is not None
    assert score_path.name == 'fascist_enforcer_theme.music.yaml'
    assert 'fascist_enforcer_theme' in radio_cues()

    spec = yaml.safe_load(Path(score_path).read_text())
    assert spec['schema'] == 'ambition.musicir.v1'
    assert spec['id'] == 'fascist_enforcer_theme'
    assert spec['render']['backend'] == 'fallback'
    assert spec['sections'][0]['id'] == 'enforcer_loop'
    assert 'hook_brass' in spec['sections'][0]['layers']
