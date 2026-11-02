"""Read command descriptions from their owners. This catalog grants no authority."""
from __future__ import annotations

import json
import subprocess
import tomllib
from pathlib import Path


def commands(repo: Path) -> list[dict]:
    rows = []
    descriptors = [repo / 'scripts/commands.json', *sorted((repo / 'scripts').glob('*/commands.json')),
                   *sorted((repo / 'tools').glob('*/commands.json')),
                   repo / 'game/ambition_app_tools/commands.json']
    for descriptor in descriptors:
        if not descriptor.is_file():
            continue
        for entry in json.loads(descriptor.read_text())['commands']:
            row = dict(entry, descriptor=descriptor.relative_to(repo).as_posix())
            missing = [path for path in row['requires'] if not (repo / path).is_file()]
            row['availability'] = 'missing source: ' + ', '.join(missing) if missing else 'source present; runtime prerequisites not checked'
            if row.get('binary'):
                row['existing_binaries'] = [f'target/{profile}/{row["binary"]}' for profile in ('debug', 'release')
                                            if (repo / 'target' / profile / row['binary']).is_file()]
            rows.append(row)
    return rows


def inventory(repo: Path) -> dict:
    """List command candidates and path consumers before a physical move."""
    supported = commands(repo)
    scripts = []
    for path in sorted((repo / 'scripts').rglob('*')):
        if path.suffix not in {'.py', '.sh', '.cjs'} or any(part in {'tests', 'lib', '__pycache__'} for part in path.relative_to(repo / 'scripts').parts):
            continue
        relative = path.relative_to(repo).as_posix()
        text = path.read_text(errors='replace')
        scripts.append({'path': relative, 'status': 'active' if any(relative in row['requires'] for row in supported) else ('experimental' if 'experimental' in path.parts else 'internal'),
                        'path_risks': [token for token in ('__file__', 'sys.path', 'scripts/lib', 'Path.cwd()', 'os.getcwd') if token in text]})
    manifest = repo / 'game/ambition_app_tools/Cargo.toml'
    bins = tomllib.loads(manifest.read_text()).get('bin', []) if manifest.is_file() else []
    candidates = sorted({row['path'] for row in scripts} | {path for row in supported for path in row['requires']})
    result = subprocess.run(['git', 'grep', '-n', '-F', '-f', '-',
                             '--', '.', ':!docs/brainstorms', ':!dev/journals'], input='\n'.join(candidates),
                            cwd=repo, text=True, capture_output=True)
    return {'commands': supported, 'scripts': scripts,
            'cargo_bins': [dict(row, status='active' if any(command.get('binary') == row['name'] for command in supported) else 'unknown') for row in bins],
            'path_consumers': result.stdout.splitlines(), 'consumer_scan_available': result.returncode in (0, 1)}
