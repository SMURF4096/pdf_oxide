# E-0 — Rich-text accumulator RFC

Target release: v0.3.40 (Bundle E implementation). Status: research, no code
changes. Issue: pdf_oxide #393 (Bundle E-0).

## Summary

- **Shape.** Add `FluentPageBuilder::paragraph() -> ParagraphBuilder<'_,'a>`
  that borrows the parent builder mutably, accumulates a `Vec<TextRun>`,
  and on `.done()` flushes through a reusable wrap path.
- **Span model.** Each `TextRun` carries `{ text, font, size, color,
  bold, italic, underline }`. Styles are locally scoped: `.bold("x")`
  flips the bit for that run only, restored on return. Matches pdfmake
  and iText semantics; avoids Typst's StyleChain machinery.
- **Lifetimes.** `ParagraphBuilder<'p, 'a>` holds `&'p mut
  FluentPageBuilder<'a>`; `.done()` returns the borrow back. Matches the
  existing `StreamingTable` borrow pattern
  (`document_builder.rs:322 new_page_same_size_inplace`).
- **Font-name munging.** `resolve_font(family, bold, italic) -> &str`
  maps `(Helvetica, true, true)` → `"Helvetica-BoldOblique"`, matching
  the convention already in `heading()` at `document_builder.rs:537`.
  Times and Courier covered symmetrically.
- **Composition with tables / columns.** Pure accumulator: the streaming
  `Table` (#393) can later accept a `Vec<TextRun>` in a cell via the
  existing `text_in_rect` wrapper (`document_builder.rs:478`);
  multi-column flow (E #11) consumes the same runs once the column
  engine lands.

## Prior art

**Typst — `TextElem` + markup.** Inline `*bold*` / `_italic_` desugars to
`strong[...]` / `emph[...]` around a `TextElem { font, weight, style,
size, fill, body: Content, ... }`; styles compose via a `StyleChain`
cascade at layout time rather than by mutating the run. Source:
`crates/typst-library/src/text/mod.rs`
(https://github.com/typst/typst/blob/main/crates/typst-library/src/text/mod.rs).
Too heavy for us — pdf_oxide has no style-chain engine and E-0 should
not introduce one.

**iText — `Paragraph.Add(Chunk)`.** A `Paragraph` is an ordered list of
`IElement` children (`Text` in v7+, `Chunk` in v5); each carries its own
font and colour and the engine wraps across them:

```csharp
var p = new Paragraph();
p.Add(new Text("Hello ").SetFont(regular));
p.Add(new Text("world").SetBold());
p.Add(new Text("!").SetFontColor(ColorConstants.RED));
```

Source: https://api.itextpdf.com/iText7/dotnet/latest/ —
`iText.Layout.Element.Paragraph`. Closest match to our target shape; our
`TextRun` is the same idea expressed as a fluent chain rather than `.Add`.

**pdfmake (JS) — array-shaped `text`.**

```js
{ text: [ { text: 'Hello ', bold: true }, 'World',
          { text: '!', color: 'red' } ] }
```

Source:
https://pdfmake.github.io/docs/0.1/document-definition-object/styling/.
Ergonomic for JS; a direct Rust port is a large enum or `Box<dyn>` noise.
We surface the array shape only in TS / Python bindings on top of the
chain.

**docx-js / pdfkit-table.** docx-js:
`new Paragraph({ children: [ new TextRun("Hello "), new TextRun({ text:
"world", bold: true }) ] })` (https://docx.js.org/#/usage/text) — same
children-array-of-runs shape as pdfmake. pdfkit-table wraps pdfkit's
stateful `doc.text("...", { continued: true }).fillColor('red').text(...)`
continuation style (https://pdfkit.org/docs/text.html); that shape is
hard to port cleanly to Rust borrows — we reject it.

**ReportLab — inline XML-ish markup.**
`Paragraph('Hello <b>world</b><font color="red">!</font>', style)` —
source: https://www.reportlab.com/docs/reportlab-userguide.pdf §6.
Clean for user input but needs a parser. pdf_oxide's `html_css/` already
does this job for full documents; we propose a `paragraph_md(…)` sugar in
Python as a follow-up that delegates to the same `TextRun` accumulator.

## Proposed API (Rust)

```rust
// new module: src/writer/rich_text.rs
#[derive(Debug, Clone)]
pub struct TextRun {
    pub text: String,
    pub font: String,         // resolved base-14 name
    pub size: f32,
    pub color: (f32, f32, f32),
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
}

// Internal: does NOT escape the crate — users touch ParagraphBuilder.
pub(crate) trait StyledRun {
    fn run(&self, text: &str) -> TextRun;
}
```

```rust
// new, in src/writer/document_builder.rs (or split file)
pub struct ParagraphBuilder<'p, 'a> {
    page: &'p mut FluentPageBuilder<'a>,
    runs: Vec<TextRun>,
    // current cascade — snapshot of the page's text_config at open time,
    // then mutated by .bold() / .italic() / .color() / .font() during build
    cur_family: String,      // "Helvetica" | "Times" | "Courier"
    cur_size: f32,
    cur_color: (f32, f32, f32),
    cur_bold: bool,
    cur_italic: bool,
    cur_underline: bool,
    // layout hints
    align: TextAlign,
    leading: f32,            // size * line_height
}

impl<'a> FluentPageBuilder<'a> {
    pub fn paragraph(&mut self) -> ParagraphBuilder<'_, 'a> { /* ... */ }
    // NB: consuming `fn paragraph(self, text: &str) -> Self` at
    // document_builder.rs:544 stays; add this new *borrowing* overload
    // under a different signature. See "Open questions" for the naming
    // conflict.
}

impl<'p, 'a> ParagraphBuilder<'p, 'a> {
    pub fn text(mut self, s: &str) -> Self { self.push(s); self }
    pub fn bold(mut self, s: &str) -> Self {
        let was = self.cur_bold; self.cur_bold = true;
        self.push(s); self.cur_bold = was; self
    }
    pub fn italic(mut self, s: &str) -> Self { /* symmetric */ self }
    pub fn color(mut self, rgb: (f32, f32, f32), s: &str) -> Self {
        let was = self.cur_color; self.cur_color = rgb;
        self.push(s); self.cur_color = was; self
    }
    pub fn font(mut self, family: &str, size: f32) -> Self {
        self.cur_family = family.into(); self.cur_size = size; self
    }
    pub fn align(mut self, a: TextAlign) -> Self { self.align = a; self }
    pub fn break_line(mut self) -> Self {
        // push a synthetic zero-width hard-break run
        self.runs.push(TextRun { text: "\n".into(), ..self.snapshot() });
        self
    }
    pub fn done(self) -> &'p mut FluentPageBuilder<'a> {
        self.flush();       // wrap + emit ContentElement::Text per line
        self.page
    }

    fn push(&mut self, s: &str) {
        let font = resolve_font(&self.cur_family, self.cur_bold, self.cur_italic);
        self.runs.push(TextRun {
            text: s.into(), font: font.into(), size: self.cur_size,
            color: self.cur_color, bold: self.cur_bold,
            italic: self.cur_italic, underline: self.cur_underline,
        });
    }

    fn flush(self) { /* see below */ }
}

fn resolve_font(family: &str, bold: bool, italic: bool) -> &'static str {
    match (family, bold, italic) {
        ("Helvetica", false, false) => "Helvetica",
        ("Helvetica", true,  false) => "Helvetica-Bold",
        ("Helvetica", false, true ) => "Helvetica-Oblique",
        ("Helvetica", true,  true ) => "Helvetica-BoldOblique",
        ("Times",     false, false) => "Times-Roman",
        ("Times",     true,  false) => "Times-Bold",
        ("Times",     false, true ) => "Times-Italic",
        ("Times",     true,  true ) => "Times-BoldItalic",
        ("Courier",   false, false) => "Courier",
        ("Courier",   true,  false) => "Courier-Bold",
        ("Courier",   false, true ) => "Courier-Oblique",
        ("Courier",   true,  true ) => "Courier-BoldOblique",
        // custom embedded family: caller-provided name wins; style bits
        // are ignored (future: look up /FontDescriptor flags)
        (name, _, _) => Box::leak(name.to_string().into_boxed_str()),
    }
}
```

**Flush algorithm.** Build a token stream `[(word, style), (" ", style),
...]`, measure each token with its run-local font via
`FontManager::text_width` (`font_manager.rs:750`), greedy-pack until the
next token would exceed column width, commit a line. Each line becomes
one `ContentElement::Text` per contiguous same-style span — reusing the
existing per-`Text` font/colour writer path, same flush shape as
`text_in_rect` (`document_builder.rs:478-526`). No new renderer.

## Per-binding shape

### Rust — explicit chain (authoritative)

```rust
page.paragraph()
    .text("Hello ")
    .bold("World")
    .color((1.0, 0.0, 0.0), "!")
    .italic(" inline")
    .done();
```

### Python — dual API: chain + markdown sugar

```python
(page.paragraph()
     .text("Hello ")
     .bold("World")
     .color((1.0, 0.0, 0.0), "!")
     .italic(" inline")
     .done())

# Sugar — delegates to the same accumulator
page.paragraph_md("Hello **World**<span color='red'>!</span>*inline*")
```

The markdown overload is opt-in; a `_md` suffix keeps typing obvious. We
do *not* auto-detect `f"**bold**"` raw strings (too magical, breaks
round-tripping with user content that legitimately contains `**`).

### WASM / Node TS — builder + template literal

```ts
page.paragraph()
    .text("Hello ")
    .bold("World")
    .color([1, 0, 0], "!")
    .italic(" inline")
    .done();

// Tagged template sugar (TS only) — deferred to v0.3.41
page.paragraphFmt`Hello ${b("World")}${rgb([1,0,0], "!")}${i(" inline")}`;
```

Tagged templates land in a follow-up. v0.3.40 ships the chain only.

### C# — fluent chain with named args for colour

```csharp
page.Paragraph()
    .Text("Hello ")
    .Bold("World")
    .Color(r: 1.0f, g: 0, b: 0, text: "!")
    .Italic(" inline")
    .Done();
```

### Go — chain with error return on `Done`

```go
p := page.Paragraph().
    Text("Hello ").
    Bold("World").
    Color(1.0, 0, 0, "!").
    Italic(" inline")
if err := p.Done(); err != nil { return err }
```

Go binding returns `error` from `Done()` to surface font-not-found /
measure failures — matches the existing `doc.Save(path) error` pattern in
`go/pdf_oxide.go`.

### C FFI — handle-based (one fn per method)

```c
PdfoxParagraph* p = pdfox_paragraph_new(page);
pdfox_paragraph_text  (p, "Hello ");
pdfox_paragraph_bold  (p, "World");
pdfox_paragraph_color (p, 1.0f, 0, 0, "!");
pdfox_paragraph_italic(p, " inline");
pdfox_paragraph_done  (p);  // frees p
```

Stringly-typed markup (ReportLab-style `<b>...</b>`) is **not** proposed
for any binding in E-0; it can arrive in E-2 via `paragraph_md` on the
languages that want it. Keeps the initial surface minimal and avoids a
new parser.

## Open questions

- **Hyphenation.** Not in E-0 scope. `wrap_text` is whitespace-only today
  (`font_manager.rs:762 split_whitespace`). A soft-hyphen pass would need
  either a dictionary (Knuth-Liang) or user-inserted U+00AD. Recommend
  deferring to v0.3.42 with a `.hyphenate(true)` flag on ParagraphBuilder;
  default off preserves byte-for-byte regression stability.
- **Justified alignment.** Current `TextAlign` has `Left | Center | Right`
  only (`document_builder.rs:495`). Justify needs inter-word-spacing
  injection (PDF `Tw` operator) and a last-line-handling policy. E-0 does
  not ship justify; reserves `.align(TextAlign::Justify)` as a symbol
  rejected at runtime until E-2. This is consistent with how iText
  `TextAlignment.JUSTIFIED` is a follow-on, not day-one.
- **Inline figures.** Bundle A lands `page.image(...)`. A `.inline_image(…)`
  on ParagraphBuilder is attractive but blows up the run model (an image
  is a line-breakable *box*, not a text span). Recommend: do not add to
  E-0. Instead, after Bundle A lands, introduce `ContentRun::Image` and a
  min-box-line algorithm in v0.3.41's multi-column flow work (#393
  Bundle E item 11), where this already needs to exist.
- **Naming conflict with existing `paragraph(text)`.** The current
  `FluentPageBuilder::paragraph(mut self, text: &str) -> Self` at
  `document_builder.rs:544` consumes `self` and takes a string. The new
  accumulator needs a different name **or** we overload: rename the
  existing one to `paragraph_simple` (keep a `#[deprecated]` alias) and
  claim `.paragraph()` for the builder. Recommend rename because the
  current signature is stateless wrapping — no user is chaining off its
  return value in a way that requires it to be named `paragraph`.
- **Borrow shape.** Returning `&mut FluentPageBuilder<'a>` from `.done()`
  breaks the existing consuming-fluent pattern elsewhere (e.g.
  `.text().font().text()` chains). Mitigation: `ParagraphBuilder::done`
  returns `&mut FluentPageBuilder<'a>` but the usual example in docs uses
  a rebinding: `let page = page.build(|p| p.paragraph().text(...).done());`
  — or we add `page.paragraph_owned()` that round-trips `self` by value
  for pure-chain users. TBD in E-1 review.

## Implementation plan

**Rust core (~4 commits, ~350 LOC)**

- E-1: add `src/writer/rich_text.rs` with `TextRun`, `resolve_font`,
  unit tests on font-name munging. ~80 LOC.
- E-2: add `ParagraphBuilder` in `src/writer/document_builder.rs` (or a
  sibling `paragraph_builder.rs`), implement `.text / .bold / .italic /
  .color / .font / .break_line / .align / .done`. ~180 LOC incl. tests.
- E-3: cross-run wrap — extract the greedy loop from
  `font_manager.rs::wrap_text` into a generic
  `wrap_runs(&[TextRun], max_width) -> Vec<Vec<TextRun>>` helper;
  `ParagraphBuilder::flush` calls it. ~90 LOC incl. tests + one golden
  pdf fixture in `tests/golden/paragraph_rich_text.pdf`.
- E-4: rename existing `FluentPageBuilder::paragraph(&str)` →
  `paragraph_simple(&str)` with `#[deprecated]` alias; wire new
  `paragraph() -> ParagraphBuilder`. ~30 LOC + doc updates.

**Bindings (~6 commits, ~60 LOC each)**

- FFI: 6 new `pdfox_paragraph_*` entry points in `src/ffi.rs` +
  `include/pdf_oxide.h`. Uses handle struct + opaque pointer like
  existing `PdfoxDocument`.
- Python (`src/python.rs`): new `PyParagraphBuilder` class with
  methods mirroring Rust. ~70 LOC.
- WASM (`src/wasm.rs`): `#[wasm_bindgen] struct ParagraphBuilder` with
  `#[wasm_bindgen(method)]` entry points, returns `&mut self` by
  `&mut JsValue`-wrapped proxy (same pattern as
  `WasmFluentPageBuilder`). ~80 LOC + TS type refresh in `js/`.
- C# (`csharp/PdfOxide/`): `ParagraphBuilder.cs` wrapping FFI. ~90 LOC.
- Go (`go/pdf_oxide.go`): `type Paragraph struct { ... }` methods. ~70 LOC.
- Node TS (`js/src/`): thin wrapper on WASM. ~50 LOC.

**Docs / CHANGELOG**

- `docs/PDF_CREATION_GUIDE.md`: add "Rich-text paragraphs" section.
- `CHANGELOG.md` v0.3.40 entry under "Added".
- `examples/rich_text_paragraph.rs` — hello-world identical to the
  "Per-binding shape / Rust" snippet above.

**Validation**

- Golden PDF: `tests/golden/paragraph_rich_text.pdf` — baseline renders
  of the "Hello World!" example across Helvetica / Times / Courier,
  checked via the existing golden-test harness.
- Regression: run v0.3.23 regression suite with new builder disabled
  (it is purely additive) — expect zero byte delta on the corpus.
- Docs build: `PDF_CREATION_GUIDE` snippet must compile via `cargo test
  --doc`.

**Total estimate.** ~350 Rust LOC + ~420 binding LOC + docs = ~770 LOC
across ~10 commits. Fits inside a single v0.3.40 release slot alongside
Bundle E items #9 (lists) and #10 (code blocks), both of which reuse the
same `TextRun` / `ParagraphBuilder` primitives.

---

## 10-line recommended-shape summary

1. Add `ParagraphBuilder<'p, 'a>` that borrows `&'p mut FluentPageBuilder<'a>`.
2. Accumulate a `Vec<TextRun>` where `TextRun` carries text + resolved
   font name + size + color + bold/italic/underline bits.
3. Fluent methods `.text / .bold / .italic / .color / .font /
   .break_line / .align` push one run each with current cascade state.
4. Bold/italic are **locally scoped** to the argument — restored on
   return, like iText's `SetBold()` applied per-chunk.
5. Font resolution via `resolve_font(family, bold, italic)` matching the
   existing `Helvetica-BoldOblique` convention at `document_builder.rs:537`.
6. `.done()` runs cross-run greedy word-wrap, then emits one
   `ContentElement::Text` per contiguous same-style span per wrapped line
   — reusing the existing writer, no new renderer.
7. Rust ships the chain API as authoritative; Python gets a `_md`
   markdown overload; TS gets a tagged template in v0.3.41.
8. Lifetimes: `.done()` returns `&'p mut FluentPageBuilder<'a>` so the
   parent chain survives, matching `StreamingTable`'s borrow pattern.
9. Deferred: hyphenation, justified alignment, inline images (the last
   goes with multi-column in v0.3.41 Bundle E #11).
10. Estimate: ~770 LOC / ~10 commits / 1 release slot (v0.3.40),
    additive-only — zero risk to the v0.3.23→HEAD regression corpus.
