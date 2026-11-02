"""The editor shows the same operations as command discovery."""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from asset_index.workbench import write_tools


def test_tools_page_links_owner_commands(tmp_path):
    repo = Path(__file__).resolve().parents[3]
    write_tools(repo, tmp_path)
    page = (tmp_path / 'tools.html').read_text()
    assert 'bash tools/ambition_moveset_inspector/serve_inspector.sh --no-export' in page
    assert 'http://127.0.0.1:8777/' in page
    assert 'tools/ecs_architecture_browser/ecs_browser.py' in page
    assert 'target/debug/capture_scene' in page
    assert 'Source presence does not prove runtime readiness' in page
