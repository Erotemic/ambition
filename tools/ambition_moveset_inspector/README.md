# Moveset inspection tools

Status: active. This tool owns move inspection and derived take reports.
The [inspector guide](docs/inspector.md) describes the server and review data.

Run commands from the repository root:

```bash
bash tools/ambition_moveset_inspector/serve_inspector.sh --no-export
python3 tools/ambition_moveset_inspector/moveset_report.py --help
python3 tools/ambition_moveset_inspector/render_take_diagnostic.py --help
python3 tools/ambition_moveset_inspector/moveset_visualizations.py --help
```

Reports read recorded simulation takes. They write JSON, Markdown, and diagnostic
SVG files to the requested output directory. They do not publish runtime assets.
The visualization coordinator can reuse `moveset_render`. It does not record new
takes or build Rust when those artifacts are missing. The server owns review
notes. Ambition Editor links to this tool and does not copy its data model.

Run the owner tests:

```bash
python3 -m pytest tools/ambition_moveset_inspector/tests -q
```
