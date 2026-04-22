#!/usr/bin/env python3
"""Modernise a rylai-generated ``.pyi`` stub in place.

rylai emits Python 3.8-era typing (``t.Optional``, ``t.Tuple`` etc.) which
basedpyright flags as ``reportDeprecated`` on every site — ~140 noise
warnings for our stub. This script rewrites those to PEP-604 native
generics so the stub type-checks clean.

Idempotent: running it twice on the same file is a no-op.

Usage:
    scripts/modernize_stubs.py python/pdf_oxide/pdf_oxide.pyi
"""

from __future__ import annotations

import re
import sys
from pathlib import Path


def _find_matching_bracket(text: str, open_pos: int) -> int:
    """Return the index of the ']' that closes '[' at ``open_pos``.

    Handles arbitrary nesting — required because rylai often emits
    ``t.Optional[t.Tuple[float, float, float, float]]``.
    Raises ``IndexError`` if no match.
    """
    depth = 0
    for i in range(open_pos, len(text)):
        ch = text[i]
        if ch == "[":
            depth += 1
        elif ch == "]":
            depth -= 1
            if depth == 0:
                return i
    raise IndexError("no matching ] found")


def _rewrite_optional(src: str) -> str:
    """t.Optional[X] -> X | None, with proper bracket balancing."""
    marker = "t.Optional["
    while True:
        idx = src.find(marker)
        if idx < 0:
            return src
        inner_start = idx + len(marker)
        try:
            close = _find_matching_bracket(src, inner_start - 1)
        except IndexError:
            # Malformed input — bail rather than corrupt the file.
            return src
        inner = src[inner_start:close]
        src = src[:idx] + inner + " | None" + src[close + 1 :]


def _rewrite_union_pair(src: str) -> str:
    """t.Union[A, B] -> A | B for the common two-arg case.

    Doesn't touch ``t.Union[A, B, C]`` (3+ args) — rylai doesn't emit
    those today and recursive parsing would need a real parser.
    """
    pattern = re.compile(r"\bt\.Union\[")
    while True:
        match = pattern.search(src)
        if not match:
            return src
        open_bracket = match.end() - 1
        try:
            close = _find_matching_bracket(src, open_bracket)
        except IndexError:
            return src
        inner = src[open_bracket + 1 : close]
        # Split on the top-level comma only.
        parts: list[str] = []
        depth = 0
        start = 0
        for i, ch in enumerate(inner):
            if ch == "[":
                depth += 1
            elif ch == "]":
                depth -= 1
            elif ch == "," and depth == 0:
                parts.append(inner[start:i].strip())
                start = i + 1
        parts.append(inner[start:].strip())
        if len(parts) != 2:
            # Leave multi-arg Unions alone — a single ``X | Y | Z``
            # chain is fine too, but keep the diff small for now.
            # Replace this occurrence with itself plus a sentinel so
            # the loop advances.
            return src
        replacement = f"{parts[0]} | {parts[1]}"
        src = src[: match.start()] + replacement + src[close + 1 :]


def _rewrite_aliases(src: str) -> str:
    """Native generic aliases for PEP-585-deprecated typing types.

    Bare ``t.Dict`` / ``t.List`` / ``t.Tuple`` (no subscript) get
    default subscripts so basedpyright's ``reportMissingTypeArgument``
    stays quiet — rylai sometimes emits bare aliases when it can't
    infer element types from the Rust side (e.g. `HashMap<String,
    Object>` whose value is the opaque `Object` enum). The defaults
    are as generic as ``t.Dict`` itself was.
    """
    bare_defaults = {
        "t.Dict": "dict[str, object]",
        "t.List": "list[object]",
        "t.Tuple": "tuple[object, ...]",
        "t.Set": "set[object]",
        "t.FrozenSet": "frozenset[object]",
        "t.Type": "type[object]",
    }

    # Subscripted variant: t.X[...] -> x[...]
    for deprecated, modern in [
        (r"\bt\.Tuple(?=\[)", "tuple"),
        (r"\bt\.List(?=\[)", "list"),
        (r"\bt\.Dict(?=\[)", "dict"),
        (r"\bt\.Set(?=\[)", "set"),
        (r"\bt\.FrozenSet(?=\[)", "frozenset"),
        (r"\bt\.Type(?=\[)", "type"),
    ]:
        src = re.sub(deprecated, modern, src)

    # Bare variant (no `[` after): t.Dict -> dict[str, object] etc.
    for deprecated, modern in bare_defaults.items():
        src = re.sub(rf"\b{re.escape(deprecated)}\b(?!\[)", modern, src)

    # rylai also emits BARE native generics (e.g. `dict | None`) when
    # it can't infer element types. Catch those in type-annotation
    # positions (preceded by `: ` / `, ` / `| ` / `[`) and
    # followed by a non-subscript terminator. Keeps identifiers
    # named "dict" / "list" in code samples untouched.
    annot_pattern = re.compile(
        r"(?P<prefix>[:,|\[]\s*)(?P<name>dict|list|tuple|set|frozenset|type)"
        r"(?P<suffix>\s*(?:\||,|\)|\]|$))"
    )
    defaults_for_bare = {
        "dict": "dict[str, object]",
        "list": "list[object]",
        "tuple": "tuple[object, ...]",
        "set": "set[object]",
        "frozenset": "frozenset[object]",
        "type": "type[object]",
    }

    def _replace_bare(match: re.Match[str]) -> str:
        return f"{match.group('prefix')}{defaults_for_bare[match.group('name')]}{match.group('suffix')}"

    src = annot_pattern.sub(_replace_bare, src)
    return src


def modernise(src: str) -> str:
    """Apply every rewrite rule, leaving the header comment untouched."""
    src = _rewrite_optional(src)
    src = _rewrite_union_pair(src)
    src = _rewrite_aliases(src)
    return src


def main(argv: list[str]) -> int:
    if len(argv) != 2:
        print(f"usage: {argv[0]} <file.pyi>", file=sys.stderr)
        return 2
    path = Path(argv[1])
    if not path.exists():
        print(f"error: {path} not found", file=sys.stderr)
        return 1
    original = path.read_text()
    modernised = modernise(original)
    if modernised != original:
        path.write_text(modernised)
        print(f"modernised {path}")
    else:
        print(f"{path} already modern")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
