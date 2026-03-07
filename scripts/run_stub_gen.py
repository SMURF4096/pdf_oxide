#!/usr/bin/env python3
"""Run stub_gen binary with env set so the Python shared library is found.

- Windows: PATH is prepended with Python dirs so stub_gen.exe finds python3xx.dll.
- Linux: LD_LIBRARY_PATH is set so stub_gen finds libpython3.x.so.
- macOS: DYLD_LIBRARY_PATH is set so stub_gen finds the Python dylib.
"""

from __future__ import annotations

import os
import subprocess
import sys
from pathlib import Path


def _lib_paths() -> list[str]:
    """Paths that might contain the Python shared library (DLL / .so / dylib)."""
    seen: set[str] = set()
    out: list[str] = []
    base_exe = getattr(sys, "base_executable", sys.executable)
    base = Path(sys.base_prefix).resolve()
    candidates = [
        Path(sys.executable).resolve().parent,
        Path(base_exe).resolve().parent,
        base,
        base / "lib",
        base / "lib64",
        base / "Scripts",
        base / "Library" / "bin",
    ]
    for p in candidates:
        if not p.is_dir():
            continue
        s = str(p)
        if s not in seen:
            seen.add(s)
            out.append(s)
    return out


def main() -> int:
    # Project root (directory with Cargo.toml / pyproject.toml). stub_gen expects CARGO_MANIFEST_DIR.
    project_root = Path(__file__).resolve().parent.parent
    os.chdir(project_root)

    extra = os.pathsep.join(_lib_paths())
    env = os.environ.copy()
    env["PATH"] = extra + os.pathsep + env.get("PATH", "")
    env["CARGO_MANIFEST_DIR"] = str(project_root)
    # Linux: so the stub_gen binary can load libpython3.x.so
    if sys.platform.startswith("linux"):
        env["LD_LIBRARY_PATH"] = extra + os.pathsep + env.get("LD_LIBRARY_PATH", "")
    # macOS: so the stub_gen binary can load the Python dylib
    if sys.platform == "darwin":
        env["DYLD_LIBRARY_PATH"] = extra + os.pathsep + env.get("DYLD_LIBRARY_PATH", "")

    # Build first (cargo run may not pass env to the exe on Windows).
    # On failure, do not run stub_gen; return the build exit code.
    r = subprocess.run(
        ["cargo", "build", "--bin", "stub_gen", "--features", "python,office"],
        env=env,
    )
    if r.returncode != 0:
        return r.returncode

    # Run the exe directly so it definitely gets our PATH and CARGO_MANIFEST_DIR.
    exe_name = "stub_gen.exe" if sys.platform == "win32" else "stub_gen"
    exe = project_root / "target" / "debug" / exe_name
    if not exe.is_file():
        exe = project_root / "target" / "release" / exe_name
    r = subprocess.run([str(exe)], env=env, cwd=project_root)
    if r.returncode != 0:
        return r.returncode

    # Remove top-level __init__.pyi if present; ty works correctly without it
    # (re-exports from __init__.py are enough; the stub lives in pdf_oxide/pdf_oxide/).
    top_level_stub = project_root / "python" / "pdf_oxide" / "__init__.pyi"
    if top_level_stub.is_file():
        top_level_stub.unlink()
    return 0


if __name__ == "__main__":
    sys.exit(main())
