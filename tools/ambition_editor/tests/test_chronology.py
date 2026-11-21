"""Character dates keep catalog age separate from art generation."""
import os
import subprocess

from asset_index.chronology import attach, catalog_ids, first_added


def test_catalog_keys_ignore_profiles_comments_and_nested_data():
    assert catalog_ids('''(
        profiles: {"profile": ()},
        // characters: {"comment": ()}
        characters: {"old": (barks: {"nested": ["characters: {"]}),
                     /* "comment": () */ "new": ()},
    )''') == {'old', 'new'}


def test_added_dates_survive_catalog_move_edit_and_readdition(tmp_path):
    def git(*args, date=None):
        env = dict(os.environ)
        if date:
            env.update(GIT_AUTHOR_DATE=date, GIT_COMMITTER_DATE=date)
        return subprocess.run(['git', '-C', str(tmp_path), *args], env=env, check=True, capture_output=True)
    git('init')
    git('config', 'user.name', 'Inspector Test')
    git('config', 'user.email', 'inspector@example.com')
    path = tmp_path / 'old.ron'
    path.write_text('(characters: {"old": ()})')
    git('add', '.')
    git('commit', '-m', 'Add old', date='2020-01-01T00:00:00+00:00')
    git('mv', 'old.ron', 'catalog.ron')
    git('commit', '-m', 'Move catalog', date='2021-01-01T00:00:00+00:00')
    path = tmp_path / 'catalog.ron'
    path.write_text('(characters: {"new": ()})')
    git('add', '.')
    git('commit', '-m', 'Replace old', date='2022-01-01T00:00:00+00:00')
    path.write_text('(characters: {"old": (), "new": ()})')
    git('add', '.')
    git('commit', '-m', 'Restore old', date='2023-01-01T00:00:00+00:00')
    path.write_text('(characters: {"old": (), "new": (), "uncommitted": ()})')
    assert first_added(path) == {'old': '2020-01-01T00:00:00+00:00', 'new': '2022-01-01T00:00:00+00:00'}


def test_generation_uses_latest_render_receipt_and_leaves_missing_dates_unknown(tmp_path):
    report = {'identities': {'game:old': {'id': 'old', 'units': ['body', 'portrait', 'converted']},
                             'game:missing': {'id': 'missing', 'units': []}},
              'units': {name: {'receipt': {'basis': basis, 'written_at': date}} for name, basis, date in [
                  ('body', 'rendered', '2026-01-01T12:00:00+02:00'),
                  ('portrait', 'rendered', '2026-01-01T11:00:00+00:00'),
                  ('converted', 'converted', '2026-02-01T00:00:00+00:00')]}}
    attach(report, {'pack_root': str(tmp_path)}, tmp_path)
    assert report['identities']['game:old']['generated_at'] == '2026-01-01T11:00:00+00:00'
    assert report['identities']['game:old']['added_at'] is None
    assert report['identities']['game:missing']['generated_at'] is None


def test_published_image_date_is_an_explicit_fallback(tmp_path):
    image = tmp_path / 'sheet.webp'
    image.write_bytes(b'fixture')
    os.utime(image, (1609459200, 1609459200))
    report = {'identities': {'game:old': {'id': 'old', 'units': ['body']}},
              'units': {'body': {'required_paths': {'sheet.webp': 'sheet.webp', 'missing.png': 'missing.png'}}}}
    attach(report, {'pack_root': str(tmp_path)}, tmp_path)
    actor = report['identities']['game:old']
    assert actor['generated_at'] == '2021-01-01T00:00:00+00:00'
    assert actor['generated_at_evidence'] == 'file-mtime'
