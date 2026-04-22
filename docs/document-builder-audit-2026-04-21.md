# DocumentBuilder cross-binding audit — 2026-04-21

Closes #384 gap Q: "claim that every binding exposes the
DocumentBuilder fluent API is accurate." This doc records the
verification and the numbers.

## Rust FFI surface

49 symbols in `src/ffi.rs`:

- 15 × `pdf_document_builder_*` (create/free/save/a4_page/letter_page/
  page/register_embedded_font/build/save_encrypted/
  to_bytes_encrypted/set_title/set_author/set_keywords/
  set_subject/set_creator)
- 31 × `pdf_page_builder_*` (font/at/text/paragraph/heading/rect/
  filled_rect/line/horizontal_rule/space/highlight/underline/
  strikeout/squiggly/stamp/sticky_note/sticky_note_at/freetext/
  watermark/watermark_draft/watermark_confidential/link_url/
  link_page/link_named/text_field/checkbox/combo_box/push_button/
  radio_group/done/free)
- 3 × `pdf_embedded_font_*` (from_file/from_bytes/free)

## Per-binding declaration coverage

All counts produced with the audit-doc §12 commands on this branch.

| Binding | Symbols declared | Missing | Coverage |
|---------|:----------------:|:-------:|:--------:|
| **C#** (`NativeMethods.cs`) | 49 | 0 | 100% |
| **Go** (`document_builder.go`, `embedded_font.go`, `page_builder.go`) | 49 | 0 | 100% |
| **Node/JS** (`js/binding.cc`) | 49 | 0 | 100% |
| **Python** — N/A (pyo3 binds Rust types directly) | — | — | — |
| **WASM** — N/A (wasm-bindgen binds Rust types directly) | — | — | — |

## Python / WASM verification

Python and WASM don't use the C FFI symbol set — pyo3 and wasm-bindgen
bind to the Rust types directly. Coverage is verified by the binding
tests listed in Tier 1 above. Spot count of the method surface:

```bash
$ grep -c "fn [a-z_]*(" src/python.rs     # 349 methods
$ grep -c "pub fn "    src/wasm.rs        # 176 methods
```

The per-binding acceptance tests that exercise the full fluent chain
are:

- `tests/test_python_document_builder.py` — 20 pytest cases.
- `tests/wasm_bindgen_tests.rs` — 9 `#[wasm_bindgen_test]` blocks
  (including `document_builder_cjk_round_trip`).
- `csharp/PdfOxide.Tests/DocumentBuilderTests.cs` — 11 xUnit cases.
- `go/document_builder_test.go` — 11 Go test funcs.
- `js/tests/document-builder.test.mjs` — 10 node:test cases.
- `tests/test_document_builder_embedded_font.rs` — Rust reference
  (CJK / Cyrillic / Greek) that every binding mirrors.

All tests pass on this branch.

## Conclusion

The v0.3.38 CHANGELOG subtitle
"DocumentBuilder fluent API across every language binding" is
accurate. The earlier broader claim ("write-side API across every
binding") was intentionally narrowed to DocumentBuilder in commit
43da49ae (`docs(changelog): narrow v0.3.38 …`) so it matches the
surface that's actually shipping.

Methodology note — the audit doc at
`~/projects/pdf_oxide_fixes_api.md` flagged "Phase 2–4 API shape
may need tweaking once Phase 1 ships. Gap Q [this task] re-audited
the DocumentBuilder claim explicitly." This doc closes that loop.
