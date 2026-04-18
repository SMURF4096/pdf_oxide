# Issue #371 — Idiomatic Page API

## What's being added

A `PdfPage` object returned when iterating or indexing a `PdfDocument`:

```python
with PdfDocument("paper.pdf") as doc:
    print(len(doc))

    for page in doc:
        text = page.text
        chars = page.chars
        md = page.markdown(detect_headings=True)

    # or by index
    page = doc[0]
```

## Python

**Missing (all small):**
- `__len__` on `PdfDocument` → delegates to `page_count()`
- `__iter__` / `__getitem__` on `PdfDocument` → returns `PdfPage`
- New `PdfPage` struct: holds `(Py<PyPdfDocument>, page_index)`
- `text`, `chars`, `words`, `lines`, `tables`, `images`, `paths`, `annotations`, `spans` as lazy `#[getter]` properties
- `markdown()`, `plain_text()`, `html()`, `render()`, `search()` as methods

No changes to existing API. `PdfPageRegion` stays as-is for sub-region use.

## Node.js

Two separate tasks:

**1. Wire missing extraction methods into `PdfDocumentImpl` (index.ts)**

The native `binding.cc` already exports everything — these are just missing
one-liner wrappers in the TypeScript class:

| Native export | Status |
|---|---|
| `extractWords` | ✅ native, ❌ TS |
| `extractTextLines` | ✅ native, ❌ TS |
| `extractTables` | ✅ native, ❌ TS |
| `getEmbeddedImages` | ✅ native, ❌ TS |
| `extractPaths` | ✅ native, ❌ TS |
| `ocrExtractText` | ✅ native, ❌ TS |

No C++ work needed — purely TypeScript.

**2. Add `PdfPage` class and iteration:**
- `[Symbol.iterator]` on `PdfDocument`
- `page(index)` method on `PdfDocument`
- `PdfPage` class: holds `(handle, pageIndex)`, dispatches to native calls
- `width`, `height`, `rotation` as cached properties (WeakMap pattern already exists)
- `text()`, `chars()`, `words()`, `lines()`, `tables()`, `images()`, `paths()`, `annotations()` as methods
- `markdown(opts?)`, `plainText()`, `html()`, `render()`, `search(query)` as methods

## C#

**Missing:**
- `PdfPage` class: holds `(PdfDocument doc, int pageIndex)`
- `Pages` property on `PdfDocument` returning `IReadOnlyList<PdfPage>`
- Indexer `doc[i]` returning `PdfPage`
- Full method surface on `PdfPage`: `ExtractText()`, `ExtractTextAsync()`, `ToMarkdown()`,
  `ToMarkdownAsync()`, `ExtractChars()`, `ExtractWords()`, `ExtractLines()`,
  `ExtractTables()`, `ExtractImages()`, `ExtractPaths()`, `GetAnnotations()`,
  `Search()`, `Render()`, `RenderThumbnail()`

## Go

**Missing:**
- `Page` struct: `type Page struct { doc *PdfDocument; index int }`
- `Pages()` on `PdfDocument` returning `[]Page`
- Full method surface on `Page`: `Text()`, `Markdown()`, `Html()`, `PlainText()`,
  `Chars()`, `Words()`, `Lines()`, `Tables()`, `Images()`, `Paths()`,
  `Annotations()`, `Search()`, `Info()`, `NeedsOcr()`, `TextWithOcr()`

---

No breaking changes in any language. All implementations dispatch to existing
extraction methods. No C++ / Rust core changes required.
