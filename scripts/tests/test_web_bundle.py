"""A browser must receive the original code bytes after HTTP decompression."""

from functools import partial
import gzip
from http.server import ThreadingHTTPServer
import importlib.util
import json
from pathlib import Path
from threading import Thread
import urllib.request


spec = importlib.util.spec_from_file_location("web_bundle", Path(__file__).parents[1] / "web_bundle.py")
web = importlib.util.module_from_spec(spec)
spec.loader.exec_module(web)


def test_browser_receives_wasm_with_the_correct_mime_type_and_encoding(tmp_path):
    code = tmp_path / "pkg"
    code.mkdir()
    wasm = b"\0asm\x01\0\0\0" + b"payload" * 100
    (code / "game.wasm").write_bytes(wasm)
    borrowed = tmp_path / "borrowed.gz"
    borrowed.write_bytes(b"another checkout")
    (code / "game.wasm.gz").symlink_to(borrowed)
    (code / "game.js").write_text("export const value = 1;\n")
    audio = tmp_path / "assets" / "audio"
    audio.mkdir(parents=True)
    (audio / "sfx.bank").write_bytes(b"sound" * 100)
    report = web.prepare(tmp_path, code)
    assert borrowed.read_bytes() == b"another checkout"
    assert not (code / "game.wasm.gz").is_symlink()
    assert report["code_gzip_bytes"] < report["code_raw_bytes"]
    assert json.loads((tmp_path / "bundle-report.json").read_text()) == report
    with ThreadingHTTPServer(("127.0.0.1", 0), partial(web.Handler, directory=str(tmp_path))) as server:
        thread = Thread(target=server.serve_forever, daemon=True)
        thread.start()
        try:
            url = f"http://127.0.0.1:{server.server_port}/pkg/game.wasm?v=1"
            for encoding, expected in (("gzip", True), ("gzip;q=0", False), ("identity", False)):
                request = urllib.request.Request(url, headers={"Accept-Encoding": encoding})
                with urllib.request.urlopen(request) as response:
                    assert response.headers["Content-Type"] == "application/wasm"
                    body = response.read()
                    if expected:
                        assert response.headers["Content-Encoding"] == "gzip"
                        assert gzip.decompress(body) == wasm
                    else:
                        assert response.headers.get("Content-Encoding") is None
                        assert body == wasm
            with urllib.request.urlopen(urllib.request.Request(
                f"http://127.0.0.1:{server.server_port}/assets/audio/sfx.bank",
                headers={"Accept-Encoding": "gzip"},
            )) as response:
                assert response.headers["Content-Encoding"] == "gzip"
                assert gzip.decompress(response.read()) == b"sound" * 100
            assert sorted(p.name for p in audio.iterdir()) == ["sfx.bank"]
        finally:
            server.shutdown()
            thread.join()
