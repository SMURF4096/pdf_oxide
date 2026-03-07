# Changelog

All notable changes to PDFOxide are documented here.

## [0.3.16] - 2026-03-07
> Advanced Visual Table Detection and Automated Python Stubs

### Features

- **Smart Hybrid Table Extraction** (#206) — Introduced a robust, zero-config visual detection engine that handles both bordered and borderless tables.
    - **Localized Grid Detection:** Uses Union-Find clustering to group vector paths into discrete table regions, enabling multiple tables per page.
    - **Visual Line Analysis:** Detects cell boundaries from actual drawing primitives (lines and rectangles), significantly improving accuracy for untagged PDFs.
    - **Visual Spans:** Identifies colspans and rowspans by analyzing the absence of internal grid lines and text-overflow signals.
    - **Visual Headers:** Heuristically identifies hierarchical (multi-row) header rows.
- **Professional ASCII Tables:** Added high-quality ASCII table formatting for plain text output, featuring automatic multiline text wrapping and balanced column alignment.
- **Auto-generated Python type stubs** (#220) — Added `pyo3-stub-gen` to automatically generate `.pyi` stub files from Rust PyO3 bindings, ensuring Python IDEs always have up-to-date type information.
- **Enabled by Default:** Table extraction is now active by default in all Markdown, HTML, and Plain Text conversions.
- **Robust Geometry:** Updated `Rect` primitive to handle negative dimensions and coordinate normalization natively.

### Bug Fixes

- **Fixed Python Coordinate Scaling:** Corrected `erase_region` coordinate mapping in Python bindings to use the standard `[x1, y1, x2, y2]` format.
- **Improved ASCII Table Wrapping:** Reworked text wrapping to be UTF-8 safe, preventing panics on multi-byte characters.
- **Refined Rendering API:** Restored backward compatibility for the `render_page` method.

### 🏆 Community Contributors

🥇 **@monchin** — Thank you for implementing automated Python stub generation (#220)! This significantly improves the developer experience for Python users by providing consistent, IDE-friendly type hints automatically synced with our Rust core. Outstanding contribution! 🚀

## [0.3.15] - 2026-03-06
> Header & Footer Management, Multi-Column Stability, and Font Fixes

### Features

- **PDF Header/Footer Management API** (#207) — Added a dedicated API for managing page artifacts across Rust, Python, and WASM.
    - **Add:** Ability to insert custom headers and footers with styling and placeholders via `PageTemplate`.
    - **Remove:** Heuristic detection engine to automatically identify and strip repeating artifacts. Includes modular methods: `remove_headers()`, `remove_footers()`, and `remove_artifacts()`. Prioritizes ISO 32000 spec-compliant `/Artifact` tags when available.
    - **Edit:** Ability to mask or erase existing content on a per-page basis via `erase_header()`, `erase_footer()`, and `erase_artifacts()`.
- **Page Templates** — Introduced `PageTemplate`, `Artifact`, and `ArtifactStyle` classes for reusable page design. Supports dynamic placeholders like `{page}`, `{pages}`, `{title}`, and `{author}`.
- **Scoped Extraction Filtering** — Updated all extraction methods to respect `erase_regions`, enabling clean text extraction by excluding identified headers and footers.
- **Python `PdfDocument.from_bytes()`** — Open PDFs directly from in-memory bytes without requiring a file path. (Contributed by **@hoesler** in #216)
- **Future-Proofed Rust API** — Implemented `Default` trait for key extraction structs (`TextSpan`, `TextChar`, `TextContent`) to protect users from future field additions.

### Bug Fixes

- **Fixed Multi-Column Reading Order** (#211) — Refactored `extract_words()` and `extract_text_lines()` to use XY-Cut partitioning. This prevents text from adjacent columns from being interleaved and standardizes top-to-bottom extraction. (Reported by **@ankursri494**)
- **Resolved Font Identity Collisions** (#213) — Improved font identity hashing to include `ToUnicode` and `DescendantFonts` references. Fixes garbled text extraction in documents where multiple fonts share the same name but use different character mappings. (Reported by **@productdevbook**)
- **Fixed `Lines` table strategy false positives** (#215) — `extract_tables()` with `horizontal_strategy="lines"` now builds the grid purely from vector path geometry and returns empty when no lines are found, preventing spurious tables on plain-text pages. (Contributed by **@hoesler**)
- **Optimized CMap Parsing** — Standardized 2-byte consumption for Identity-H fonts and improved robust decoding for Turkish and other extended character sets.

### 🏆 Community Contributors

🥇 **@hoesler** — Huge thanks for PR #216 and #215! Your contribution of `from_bytes()` for Python unlocks new serverless and in-memory workflows for the entire community. Additionally, your fix for the `Lines` table strategy significantly improves the precision of our table extraction engine. Outstanding work! 🚀

🥈 **@ankursri494** (Ankur Srivastava) — Thank you for identifying the multi-column reading order issue (#211). Your detailed report and sample document were the catalyst for our new XY-Cut partitioning engine, which makes PDFOxide's reading order detection among the best in the ecosystem! 🎯

🥉 **@productdevbook** — Thanks for reporting the complex font identity collision issue (#213). This report led to a deep dive into PDF font internals and a significantly more robust font hashing system that fixes garbled text for thousands of professional documents! 🔍✨
