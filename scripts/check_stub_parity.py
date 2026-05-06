#!/usr/bin/env python3
"""Verify that every public symbol in pdf_oxide's .pyi stub exists in the
installed module.

Exit 1 if any stub symbol is absent — this catches stubs generated with
wider Cargo features than the installed wheel (issue #464).

Usage:
    python scripts/check_stub_parity.py <path-to-pyi>
"""
from __future__ import annotations

import ast
import importlib
import sys


def pyi_top_level_names(pyi_path: str) -> set[str]:
    """Return all public names defined at the top level of a .pyi file."""
    with open(pyi_path, encoding="utf-8") as f:
        source = f.read()
    tree = ast.parse(source, filename=pyi_path)
    names: set[str] = set()
    for node in ast.iter_child_nodes(tree):
        match node:
            case ast.ClassDef(name=n) | ast.FunctionDef(name=n) | ast.AsyncFunctionDef(name=n):
                if not n.startswith("_"):
                    names.add(n)
            case ast.Assign(targets=targets):
                for t in targets:
                    if isinstance(t, ast.Name) and not t.id.startswith("_"):
                        names.add(t.id)
            case ast.AnnAssign(target=ast.Name(id=n)) if not n.startswith("_"):
                names.add(n)
            case ast.ImportFrom(names=aliases):
                for alias in aliases:
                    exported = alias.asname or alias.name
                    if not exported.startswith("_"):
                        names.add(exported)
    return names


def main() -> int:
    if len(sys.argv) != 2:
        print(f"Usage: {sys.argv[0]} <path-to-pyi>", file=sys.stderr)
        return 2

    pyi_path = sys.argv[1]
    stub_names = pyi_top_level_names(pyi_path)

    mod = importlib.import_module("pdf_oxide")
    mod_names = set(dir(mod))

    missing = stub_names - mod_names
    if missing:
        print("FAIL: stub symbols missing from installed wheel:")
        for name in sorted(missing):
            print(f"  {name}")
        print(
            "\nThe stub was likely generated with broader Cargo features than the"
            " installed wheel. Fix: regenerate the stub with --features matching"
            " the release wheel (see rylai.toml)."
        )
        return 1

    print(f"OK: all {len(stub_names)} stub symbols present in installed module.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
