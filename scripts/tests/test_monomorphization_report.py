"""Pure tests for the monomorphization/artifact reporting tool."""

from __future__ import annotations

import importlib.util
from pathlib import Path
import sys

import pytest

pytestmark = pytest.mark.detached_tool

SCRIPT = Path(__file__).resolve().parents[1] / "monomorphization_report.py"
spec = importlib.util.spec_from_file_location("monomorphization_report", SCRIPT)
assert spec and spec.loader
report = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = report
spec.loader.exec_module(report)


def test_logical_artifact_name_groups_hash_variants_but_keeps_artifact_kind():
    assert report.logical_artifact_name("libfoo-0123456789abcdef.rlib") == "libfoo.rlib"
    assert report.logical_artifact_name("libfoo-fedcba9876543210.rmeta") == "libfoo.rmeta"
    assert report.logical_artifact_name("app_it-0123456789abcdef") == "app_it<link-product>"
    assert report.logical_artifact_name("unhashed") is None


def test_target_inventory_reports_variant_retention_without_calling_it_duplicate_bytes(tmp_path):
    deps = tmp_path / "debug" / "deps"
    deps.mkdir(parents=True)
    (deps / "libfoo-0123456789abcdef.rlib").write_bytes(b"x" * 100)
    (deps / "libfoo-fedcba9876543210.rlib").write_bytes(b"y" * 80)
    (deps / "libbar-1111111111111111.rlib").write_bytes(b"z" * 50)

    inv = report.target_inventory(tmp_path)

    assert inv["total_bytes"] == 230
    assert inv["variant_retained_excess_bytes"] == 80
    assert len(inv["variant_groups"]) == 1
    row = inv["variant_groups"][0]
    assert row["artifact"] == "libfoo.rlib"
    assert row["variants"] == 2
    assert row["total_bytes"] == 180
    assert row["retained_excess_bytes"] == 80


def test_generic_family_only_erases_turbofish_not_trait_implementation_identity():
    assert (
        report.generic_family("crate::do_work::<alloc::vec::Vec<u8>>::habcdef0123456789")
        == "crate::do_work::<...>"
    )
    # Conservative on purpose: these may be two different trait implementations,
    # not two monomorphizations of one generic function body.
    left = report.generic_family("<crate::A as dep::Trait>::run::habcdef0123456789")
    right = report.generic_family("<crate::B as dep::Trait>::run::h1111111111111111")
    assert left != right


def test_nested_turbofish_is_replaced_as_one_family():
    name = "crate::f::<alloc::vec::Vec<core::option::Option<u8>>>::h0123456789abcdef"
    assert report.generic_family(name) == "crate::f::<...>"


def test_nm_parser_keeps_text_symbols_and_groups_concrete_instances():
    text = """
0000000000000010 0000000000000020 T crate::work::<u32>::h0123456789abcdef
0000000000000040 0000000000000030 t crate::work::<u64>::h1111111111111111
0000000000000080 0000000000000080 D crate::DATA::h2222222222222222
"""
    symbols = report.parse_nm_output(text, Path("fake"))
    assert [row.size for row in symbols] == [0x20, 0x30]
    assert {row.family for row in symbols} == {"crate::work::<...>"}


def test_symbol_report_pressure_is_explicitly_all_but_largest(monkeypatch, tmp_path):
    artifact = tmp_path / "binary"
    artifact.write_bytes(b"binary")
    rows = [
        report.Symbol(artifact, 100, "T", "crate::work::<u32>", "crate::work::<...>", "crate"),
        report.Symbol(artifact, 80, "T", "crate::work::<u64>", "crate::work::<...>", "crate"),
        report.Symbol(artifact, 50, "T", "crate::other", "crate::other", "crate"),
    ]
    monkeypatch.setattr(report, "symbols_for_artifact", lambda path, nm=None: (rows, None))

    data = report.symbol_report([artifact])
    family = data["families"][0]
    assert family["instances"] == 2
    assert family["total_text_bytes"] == 180
    assert family["single_body_pressure_bytes"] == 80
    assert data["family_pressure_bytes"] == 80


def test_mono_item_parser_accepts_rustc_codegen_unit_suffixes():
    text = """
MONO_ITEM fn crate::work::<u32> @@ crate.abc[Internal]
MONO_ITEM fn crate::work::<u64> @@ crate.def[External]
noise
MONO_ITEM static crate::THING
"""
    items = report.parse_mono_items(text)
    assert items == ["fn crate::work::<u32>", "fn crate::work::<u64>", "static crate::THING"]
    data = report.mono_report(items)
    assert data["mono_items"] == 3
    assert data["multi_instance_families"] == 1
    assert data["families"][0]["instances"] == 2


def test_find_artifacts_chooses_newest_matching_variant(tmp_path):
    deps = tmp_path / "debug" / "deps"
    deps.mkdir(parents=True)
    old = deps / "app_it-0123456789abcdef"
    new = deps / "app_it-fedcba9876543210"
    old.write_bytes(b"old")
    new.write_bytes(b"new")
    old.touch()
    new.touch()
    # Force stable ordering independent of filesystem timestamp granularity.
    import os
    os.utime(old, ns=(1, 1))
    os.utime(new, ns=(2, 2))
    assert report.find_artifacts(tmp_path, ["app_it"]) == [new]


def test_nm_command_uses_gnu_rust_demangle_style():
    cmd = report._nm_command("/usr/bin/nm", Path("artifact"), flavor="gnu")
    assert "--demangle=rust" in cmd
    assert "--demangle" not in cmd


def test_nm_command_uses_llvm_boolean_demangle_flag():
    cmd = report._nm_command("/usr/bin/llvm-nm", Path("artifact"), flavor="llvm")
    assert "--demangle" in cmd
    assert "--demangle=rust" not in cmd


def test_symbols_for_artifact_reads_llvm_nm_without_gnu_style_argument(monkeypatch, tmp_path):
    artifact = tmp_path / "binary"
    artifact.write_bytes(b"binary")
    calls = []

    class Result:
        returncode = 0
        stderr = ""
        stdout = "0000000000000010 0000000000000020 T crate::work::<u32>::h0123456789abcdef\n"

    def fake_run(cmd, **kwargs):
        calls.append(cmd)
        return Result()

    monkeypatch.setattr(report.subprocess, "run", fake_run)
    symbols, error = report.symbols_for_artifact(artifact, "/usr/bin/llvm-nm")

    assert error is None
    assert len(symbols) == 1
    assert "--demangle" in calls[0]
    assert "--demangle=rust" not in calls[0]


def test_symbols_for_artifact_retries_other_demangle_spelling_for_renamed_wrapper(monkeypatch, tmp_path):
    artifact = tmp_path / "binary"
    artifact.write_bytes(b"binary")
    calls = []

    class Result:
        def __init__(self, returncode, stdout="", stderr=""):
            self.returncode = returncode
            self.stdout = stdout
            self.stderr = stderr

    def fake_run(cmd, **kwargs):
        calls.append(cmd)
        if "--demangle=rust" in cmd:
            return Result(1, stderr="unknown argument --demangle=rust")
        return Result(
            0,
            stdout="0000000000000010 0000000000000020 T crate::work::<u32>::h0123456789abcdef\n",
        )

    monkeypatch.setattr(report.subprocess, "run", fake_run)
    symbols, error = report.symbols_for_artifact(artifact, "/opt/tools/custom-nm")

    assert error is None
    assert len(symbols) == 1
    assert len(calls) == 2
    assert "--demangle=rust" in calls[0]
    assert "--demangle" in calls[1]


def test_parse_readelf_sections_tracks_archive_members_and_nobits():
    text = """
File: libx.rlib(one.o)
  [ 1] .text             PROGBITS        0000000000000000 000040 000100 00  AX  0   0  1
  [ 2] .bss              NOBITS          0000000000000000 000140 000080 00  WA  0   0  8
  [ 3] .debug_line       PROGBITS        0000000000000000 000140 000200 00      0   0  1
  [ 4] .rela.text        RELA            0000000000000000 000340 000030 18   I  0   1  8
  [ 5] .symtab           SYMTAB          0000000000000000 000370 000060 18      0   0  8
  [ 6] .strtab           STRTAB          0000000000000000 0003d0 000040 00      0   0  1
File: libx.rlib(two.o)
  [ 1] .text.foo         PROGBITS        0000000000000000 000040 000020 00  AX  0   0  1
"""
    rows = report.parse_readelf_sections(text, Path("libx.rlib"))
    assert len(rows) == 7
    assert {row["member"] for row in rows} == {"libx.rlib(one.o)", "libx.rlib(two.o)"}
    bss = next(row for row in rows if row["name"] == ".bss")
    assert bss["declared_bytes"] == 0x80
    assert bss["disk_bytes"] == 0
    assert bss["category"] == "writable/TLS data"
    assert next(row for row in rows if row["name"] == ".debug_line")["category"] == "debug"
    assert next(row for row in rows if row["name"] == ".rela.text")["category"] == "relocations"
    assert next(row for row in rows if row["name"] == ".symtab")["category"] == "symbol tables"
    assert next(row for row in rows if row["name"] == ".strtab")["category"] == "string tables"


def test_section_report_separates_file_payload_from_residual_container_bytes(monkeypatch, tmp_path):
    artifact = tmp_path / "libfoo.rlib"
    artifact.write_bytes(b"x" * 1000)
    rows = [
        {"member": "one.o", "name": ".text", "type": "PROGBITS", "declared_bytes": 200, "disk_bytes": 200, "category": "code"},
        {"member": "one.o", "name": ".debug_line", "type": "PROGBITS", "declared_bytes": 300, "disk_bytes": 300, "category": "debug"},
        {"member": "one.o", "name": ".bss", "type": "NOBITS", "declared_bytes": 400, "disk_bytes": 0, "category": "writable/TLS data"},
    ]
    monkeypatch.setattr(report, "sections_for_artifact", lambda path, readelf=None: (rows, None))

    data = report.section_report([artifact])
    row = data["artifacts"][0]
    assert row["file_bytes"] == 1000
    assert row["accounted_section_disk_bytes"] == 500
    assert row["residual_container_bytes"] == 500
    assert row["virtual_nobits_bytes"] == 400
    assert row["elf_members"] == 1
    assert [category["category"] for category in row["categories"][:2]] == ["debug", "code"]


def test_sections_for_artifact_keeps_parsed_rlib_rows_even_when_readelf_warns(monkeypatch, tmp_path):
    artifact = tmp_path / "libfoo.rlib"
    artifact.write_bytes(b"archive")

    class Result:
        returncode = 1
        stderr = "readelf: Error: lib.rmeta: Failed to read file header"
        stdout = "  [ 1] .text PROGBITS 0000000000000000 000040 000010 00 AX 0 0 1\n"

    monkeypatch.setattr(report.subprocess, "run", lambda *args, **kwargs: Result())
    rows, warning = report.sections_for_artifact(artifact, "/usr/bin/readelf")
    assert len(rows) == 1
    assert rows[0]["disk_bytes"] == 0x10
    assert "lib.rmeta" in warning
