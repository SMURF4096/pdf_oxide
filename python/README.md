# PDF Oxide - The Fastest Python PDF Library

The fastest Python PDF library for text extraction, image extraction, and document conversion. 0.8ms mean per document — 5× faster than PyMuPDF, 15× faster than pypdf. 100% pass rate on 3,830 real-world PDFs. MIT licensed.

## Why pdf_oxide?

- **Fastest** — 0.8ms mean text extraction, 5× faster than PyMuPDF, 15× faster than pypdf
- **100% reliable** — Zero failures, zero panics, zero timeouts on 3,830 test PDFs
- **MIT licensed** — Unlike PyMuPDF (AGPL-3.0), use freely in commercial and closed-source projects
- **Complete** — Extract text, images, forms. Create PDFs from Markdown, HTML, images. Edit existing PDFs.
- **Complex scripts** — RTL (Arabic/Hebrew), CJK (Japanese/Korean/Chinese), Devanagari, Thai
- **Format conversion** — PDF to Markdown, HTML, or plain text with automatic reading order

## Quick Start

```python
from pdf_oxide import PdfDocument

# Open a PDF (path can be str or pathlib.Path)
doc = PdfDocument("document.pdf")

# Or use as a context manager
with PdfDocument("document.pdf") as doc:
    text = doc.to_plain_text(0)
    print(text)

# Extract as plain text (with automatic reading order)
text = doc.to_plain_text(0)
print(text)

# Convert to Markdown
markdown = doc.to_markdown(0, detect_headings=True)
with open("output.md", "w") as f:
    f.write(markdown)

# Convert to HTML
html = doc.to_html(0, preserve_layout=False)
with open("output.html", "w") as f:
    f.write(html)
```

## Installation

```bash
pip install pdf_oxide
```

Wheels available for Linux, macOS, and Windows. Python 3.8–3.14. No system dependencies.

## Development
```bash
# Install uv
# refer to https://docs.astral.sh/uv/getting-started/installation/#standalone-installer

# Install necessary tools for python dev
uv tool install maturin
uv tool install pdm
uv tool install ruff
uv tool install ty

# Install python dependencies
uv sync --group test

# If you need to run scripts, please add the responsive group
# e.g., if you need to run "benchark_all_libraries.py" script, you should run
uv sync --group benchmark
# All the groups could be found in [dependency-groups] in "pyproject.toml"

# If you just need production code, please run
uv sync

# Build python bindings
maturin develop --uv

# format code (would format both python and rust code)
pdm fmt

# lint code (would lint both python and rust code)
pdm lint
```

## Type stubs (.pyi)

Type stubs are generated from the Rust PyO3 bindings with [pyo3-stub-gen](https://crates.io/crates/pyo3-stub-gen). After changing the Python API in `src/python.rs`, regenerate stubs so IDEs and type checkers see the correct signatures:

```bash
pdm run stub_gen
```

Output is written under `python/pdf_oxide/` (e.g. `pdf_oxide/pdf_oxide/__init__.pyi`) and is bundled into the wheel by maturin. The release workflow regenerates stubs automatically before building wheels.

## API Documentation

See the main README for full API documentation and examples.
