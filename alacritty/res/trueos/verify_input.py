#!/usr/bin/env python3
"""Verify the exact GLSL bytes and links expected by the TRUEOS Bakery."""

import hashlib
import json
from pathlib import Path


ROOT = Path(__file__).resolve().parent


def main() -> None:
    manifest = json.loads((ROOT / "bakery-input.json").read_text())
    assert manifest["schema"] == "trueos-alacritty-gles2pure-aot-v1"
    header = manifest["shader_header"]
    assert header == "#version 100\n#define GLES2_RENDERER\n"

    modules = {}
    for module in manifest["modules"]:
        module_id = module["id"]
        assert module_id not in modules, f"duplicate module: {module_id}"
        source = (ROOT / module["source"]).resolve()
        assert source.is_relative_to(ROOT.parent), f"source outside res: {source}"
        prefix = module.get("prefix", "")
        assert not prefix or prefix.endswith("\n"), f"unterminated define: {module_id}"
        actual = hashlib.sha256((header + prefix).encode() + source.read_bytes()).hexdigest()
        assert actual == module["sha256"], f"source hash mismatch: {module_id}"
        modules[module_id] = module["stage"]

    assert len(modules) == 7
    programs = set()
    for program in manifest["programs"]:
        name = program["id"]
        assert name not in programs, f"duplicate program: {name}"
        assert modules[program["vertex"]] == "vertex", name
        assert modules[program["fragment"]] == "fragment", name
        programs.add(name)
    assert programs == {
        "TextPure", "RectNormal", "RectUndercurl", "RectDotted", "RectDashed"
    }
    assert set(manifest["reflection"]) == {"TextPure", "Rect*"}
    print(f"verified {len(modules)} GLSL modules and {len(programs)} linked programs")


if __name__ == "__main__":
    main()
