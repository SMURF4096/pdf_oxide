# DocumentBuilder gap analysis — what tables need that we don't have yet

Audit target: `release/v0.3.39`. Prompted by issue #393 (programmatic table API).
Scope: Rust-core only (the Rust surface drives all 7 bindings per v0.3.38 pattern).

## Summary

| Capability                               | Status    |
|------------------------------------------|-----------|
| 1. Text measurement                      | sufficient (internal) |
| 2. Cell text layout with wrapping        | partial (internal exists, not on FluentPageBuilder) |
| 3. Cursor / relative positioning         | partial (cursor exists, only implicit vertical advance) |
| 4. Page-break signal                     | **missing** |
| 5. Header / footer repetition            | sufficient (PageTemplate) |
| 6. Borders / rules (per-side)            | partial (rect outline only, no per-side) |
| 7. Multi-column flow                     | **missing** on DocumentBuilder (HTML path has it) |
| 8. Alignment in a bounding box           | **missing** on FluentPageBuilder (TextAlign field is dead) |
| 9. Section / frame across pages          | **missing** |
| 10. Font subset integration              | sufficient — dynamic text registers glyphs automatically |

**Key discovery:** `src/writer/table_renderer.rs` (1 269 lines) already defines
`Table`, `TableCell`, `TableRow`, `TableStyle`, `CellAlign`, `ColumnWidth`,
`Borders`, and a full `Table::render(&mut ContentStreamBuilder, x, y, layout)`
with background/border/text drawing. It is re-exported from
`src/writer/mod.rs:167-170` but **never called** from `FluentPageBuilder` and is
not reachable through the existing fluent chain (DocumentBuilder accumulates
`ContentElement` enum values; table_renderer writes directly to a
`ContentStreamBuilder`, a different pipeline). Also — no page-break handling
and no registration with the subsetter.

**Recommended scope addition:** "tables + 4 supporting primitives", not
"tables only". The four: `cell(rect, text)` text-in-rect, `stroke_rect` with
per-side widths, a page-break / overflow signal, and alignment-in-bbox for
`.text()`. Without these, a `.table(...)` bolted onto the current fluent builder
will either (a) lack headers-repeat-on-page-break (#5 trivially covers headers
via PageTemplate — but *table-internal* header rows across splits is a separate
need), (b) silently overflow off-page, or (c) duplicate the shape-drawing logic
already in `table_renderer.rs`.

## Evaluated primitives

### 1. Text measurement

- **Current state:** `FontManager::text_width(text, font_name, size) -> f32`
  at `src/writer/font_manager.rs:150-153` (delegates to
  `FontInfo::text_width` at `font_manager.rs:302` / `:946`). `TextLayout`
  owns a `FontManager` and exposes `text_bounds(text, font, size, max_width)
  -> (f32, f32)` at `font_manager.rs:795-810`. Works on all 14 base fonts
  and any registered embedded font.
- **Gap vs required:** No *public* builder-surface method. A caller holding
  only a `FluentPageBuilder` can't query width. `FluentPageBuilder.text_layout`
  (`document_builder.rs:165`) is private.
- **Proposed change:** expose `FluentPageBuilder::measure(text: &str) -> f32`
  using the current `text_config.font`/`size` and delegating to
  `self.text_layout.font_manager().text_width(...)`. One-line adapter.
  No new engine code.

### 2. Cell text layout with wrapping

- **Current state:** Two half-answers.
  - (a) `FluentPageBuilder::paragraph(text)` at
    `document_builder.rs:241-274` wraps against `page.width - cursor_x - 72.0`
    (hard-coded 72 pt right margin). Wrapping engine is
    `TextLayout::wrap_text` at `font_manager.rs:750-792` — splits on
    whitespace, accurate metrics, no justification.
  - (b) `table_renderer.rs:1084` has a `wrap_text(text, max_width,
    font_size, metrics)` free function that takes a `FontMetrics` trait
    and re-does the same loop. Currently only consumed internally by
    `Table::calculate_row_heights` (`table_renderer.rs:817`).
- **Gap vs required:** No `cell(rect, text)` / `text_in_rect(rect, text)`
  on `FluentPageBuilder`. A caller has to compute the wrap themselves
  (which `paragraph` does — but only relative to page width, not an
  arbitrary rect).
- **Proposed change:** add `FluentPageBuilder::text_in_rect(rect, text) ->
  Self`. Internally wrap with `TextLayout::wrap_text` against `rect.width`,
  emit one `TextContent` per line positioned inside `rect`, clip the last
  line if `rect.height` is exceeded. This is what tables will call per
  cell; exposing it independently is ~30 LOC and closes #2 without a
  table-specific code path.

### 3. Per-page cursor / transforms / clip

- **Current state:** `FluentPageBuilder` holds `cursor_x` / `cursor_y`
  (`document_builder.rs:162-163`), advanced implicitly by `.text()` and
  `.paragraph()` (`:221`, `:269`). Initial cursor is `(72.0, height -
  72.0)` — hardcoded 1-inch margin (`document_builder.rs:996-997`).
  Absolute placement via `.at(x, y)` at `:187`. Clip operators exist at
  the `ContentStreamBuilder` layer (`content_stream.rs:496` `clip`,
  `:501` `clip_even_odd`, `:513` `clip_rect`) but are not surfaced on
  `FluentPageBuilder`. Same for `translate` / `scale` / `rotate` / matrix
  (`content_stream.rs:544-572`).
- **Gap vs required:** Tables only need cursor-read (for "start the table
  at the current y"). No clip is needed for v1 — but for "clip cell text
  that overflows" we'd want it. Graphics-state save/restore is missing
  from the fluent API too.
- **Proposed change:** expose `FluentPageBuilder::cursor() -> (f32, f32)`
  and `::advance_y(points)`. Defer clip / transforms — they belong to a
  follow-up graphics-state issue, not #393.

### 4. Page-break signal

- **Current state:** **No such signal exists.** `FluentPageBuilder` has
  no concept of page-overflow. Adding a new page requires the caller to
  call `.done()` to return to `&mut DocumentBuilder`, then
  `.page(PageSize::...)` again (`document_builder.rs:983`). No API tells
  the caller "current y is below the bottom margin". No API splits content
  automatically. `html_css/layout/tables.rs:8` mentions "`<thead>`
  repetition handled by the paginator (PAGINATE-2)" — that paginator is
  HTML-side, not fluent-side.
- **Gap vs required:** Critical for tables. A table whose rows exceed the
  remaining page space must paginate. Without a signal, either (a)
  overflow silently, (b) pre-compute total table height and force users
  to split manually, or (c) fail.
- **Proposed change:** add a `FluentPageBuilder::remaining_space(&self)
  -> f32` helper plus a `::new_page(&mut self) -> &mut Self` that
  internally does the `done() -> page()` transition while preserving
  `text_config` and `text_layout`. For table rendering, this is the
  minimum. A full "frame" abstraction (#9) is the more ambitious version.

### 5. Header / footer repetition on new page

- **Current state:** `PageTemplate` at
  `src/writer/page_template.rs:369-416` with `header(Artifact)`,
  `footer(Artifact)`, `first_page_header`, `first_page_footer`, and
  three-up alignment (`Artifact.left/center/right`). Rendered per page in
  `DocumentBuilder::build()` at `document_builder.rs:1044-1080` with
  `Placeholder` substitution (`{page}`, `{pages}`, `{title}` —
  `page_template.rs:20-47`).
- **Gap vs required:** Already sufficient for document-level chrome.
  **Not sufficient** for table-internal header-row repetition when a
  table splits across pages — that's a table-layout concern, not a
  template concern. The split-repeat logic would have to live in the
  table renderer and know when `new_page()` has happened.
- **Proposed change:** no change to `PageTemplate`. The table's own
  `Table::render_paginated(...)` will need to emit the header `TableRow`
  on each new page it produces — logic local to table_renderer.

### 6. Borders / rules

- **Current state:**
  - `FluentPageBuilder::rect(x,y,w,h)` — 1 pt black stroked outline,
    `document_builder.rs:759-773`.
  - `FluentPageBuilder::filled_rect(x,y,w,h,r,g,b)` — no outline,
    `:777-787`.
  - `FluentPageBuilder::line(x1,y1,x2,y2)` — 1 pt black stroke,
    `:791-810`.
  - Color and width are hard-coded in all three. No per-side thickness,
    no dash pattern, no user-chosen stroke color.
  - `ContentStreamBuilder` has full support:
    `set_stroke_color(r,g,b)` `:431`, `set_line_width(w)` `:436`,
    `set_dash_pattern(v, phase)` `:592`, `set_line_cap/join` `:573/:578`.
  - `table_renderer::Borders` at `table_renderer.rs:117-186` models
    per-side `top/bottom/left/right` with `TableBorderStyle{width,color}`
    — and `Table::draw_cell_borders` at `:992-1044` draws them by
    writing to a `ContentStreamBuilder` (not reachable from fluent).
- **Gap vs required:** The fluent API cannot draw a colored stroke at a
  user-chosen width. A caller who wants "1.5 pt red rule" has to drop
  through to content elements manually. For tables, the internal
  `Borders` machinery is fine if we reuse `table_renderer` — but if we
  *extend* the fluent API (for users drawing their own tables), we need
  `stroke_rect(rect, style)` with `LineStyle{width, color, dash,
  per_side}`.
- **Proposed change:** add `FluentPageBuilder::stroke_rect(rect, style:
  LineStyle)` and `::stroke_line(p1, p2, style)`. `LineStyle` carries
  `width`, `color`, optional `dash`. Per-side rect stroking is then
  expressible as four `stroke_line` calls — no separate API surface
  needed for v1.

### 7. Multi-column flow

- **Current state:** `src/html_css/layout/multicol.rs:36` `read_multicol`
  reads CSS `columns`, `column_count`, `column_gap`; `:73` `column_rects`
  produces per-column rects; `:97` `distribute_lines_into_columns`.
  **HTML-layout only** — consumed through the HTML→PDF pipeline, not
  accessible from `DocumentBuilder`.
- **Gap vs required:** Tables don't strictly need multi-column. A
  two-column report *containing* a table does, but that's a broader
  layout feature.
- **Proposed change:** **out of scope** for #393. Note in the follow-up
  list that `multicol.rs` is reusable the day we add `DocumentBuilder`
  columns — the engine already exists.

### 8. Alignment + typographic controls

- **Current state:** `TextConfig { font, size, align: TextAlign,
  line_height }` at `document_builder.rs:136-145`. `TextAlign` enum
  (`Left`/`Center`/`Right`) defined at `:124-132`. **`text_config.align`
  is never read by `.text()` or `.paragraph()`** — confirmed by grep:
  the only user-site read of `TextAlign::Center` is in
  `document_builder.rs:1492` (a test). Every text element positions
  baseline-left at `(cursor_x, cursor_y)`.
- **Gap vs required:** Tables absolutely need per-cell alignment. The
  internal `table_renderer::CellAlign` (`table_renderer.rs:25`) is
  honored inside `Table::render` (`:959-963`) by computing `text_x`
  based on align. But any table-rendering code that goes via
  `ContentElement::Text` (the fluent path) inherits the "baseline-left
  only" bug.
- **Proposed change:** either (a) make `.text()` honor `text_config.align`
  by offsetting `cursor_x` against the measured width and the right
  margin, or (b) add a new `text_in_rect(rect, text, align)` primitive
  (see #2) that does it explicitly. Option (b) is cleaner — `.text()`
  keeps its "at the cursor" semantics, alignment lives in the rect-
  taking variant.

### 9. State tracked across pages (frame / section)

- **Current state:** Nothing. `FluentPageBuilder` lives inside a single
  page (`document_builder.rs:159-170`), and `.done()` consumes it back
  to `&mut DocumentBuilder`. There is no "render this block, continuing
  on the next page if needed" abstraction. `PageTemplate` is per-page,
  not a span-multiple-pages construct.
- **Gap vs required:** Tables are the canonical example of content that
  spans. The other obvious one is long paragraphs. Adding a frame /
  section now means tables don't invent their own pagination
  mini-framework.
- **Proposed change:** introduce `FluentPageBuilder::frame(rect, F)`
  where `F: FnOnce(&mut FrameBuilder)`, and `FrameBuilder` holds a
  cursor scoped to `rect` plus a `page-break-needed` flag. Table
  rendering then becomes "for each row, if row won't fit, end frame on
  this page, start on next". This is the more ambitious version of #4.
  **Risk:** likely a multi-PR effort on its own; if we need tables to
  ship in v0.3.39, do #4 (page-break signal) instead and defer frames.

### 10. Font subset integration

- **Current state:** Every user-supplied text string flows through
  `FontInfo::encode_string` (`font_manager.rs:971-976`), which calls
  `use_string` (`:952-959`), which records `(codepoint, glyph_id)` into
  the per-font `FontSubsetter` (`:956`). The dispatch happens in
  `PdfWriter::PageBuilder::add_element` at
  `pdf_writer.rs:226-245` — any `ContentElement::Text` whose font name
  matches a registered embedded font is routed to `add_embedded_text`
  (`:142-172`) which does the `encode_string` call that populates
  `subsetter.used_glyphs()`. At `build()` finalization,
  `font_pdf_objects.rs:114` calls `crate::fonts::subset_font_bytes` with
  the final glyph set and emits the subset stream.
- **Gap vs required:** None for the fluent path — tables added via
  `ContentElement::Text` will correctly accumulate glyphs. **But** the
  orphaned `table_renderer::Table::render` writes directly to a
  `ContentStreamBuilder` (`table_renderer.rs:873-990`), bypassing
  `PageBuilder::add_element`, so if we revive it without also wiring
  subsetter calls, CJK/embedded-font cell text will render as `.notdef`
  in the subset.
- **Proposed change:** any new table primitive must go through
  `ContentElement::Text` (the existing fluent path), OR must call
  `FontManager::use_string` / `FontInfo::encode_string` explicitly
  before emitting raw content-stream ops. Prefer the first — less
  new code, more consistent with the rest of the builder. This means
  rewriting `Table::render` to emit `ContentElement`s, not direct
  content-stream ops. Small, mechanical change.

## Recommended additions alongside tables

Each item is a separate, focused change; all four together are
< 500 LOC and unblock a credible table API.

- [ ] **P1 — `FluentPageBuilder::text_in_rect(rect, text, align)`**
      — `src/writer/document_builder.rs`. Wraps via existing
      `TextLayout::wrap_text`, positions per `align`. Closes gaps
      #2 and #8 together. ~40 LOC + tests.
- [ ] **P1 — `FluentPageBuilder::remaining_space()` +
      `::new_page_same_size()`** — `src/writer/document_builder.rs`.
      The minimum page-break signal (#4). Returns `cursor_y - 72.0`
      (bottom margin). `new_page_same_size` does the
      `done() → page(size)` transition while preserving
      `text_config`. ~25 LOC.
- [ ] **P1 — `FluentPageBuilder::stroke_rect(rect, LineStyle)` +
      `::stroke_line(p1, p2, LineStyle)`** — `src/writer/document_builder.rs`.
      Closes #6. New `LineStyle { width, color, dash: Option<Vec<f32>> }`
      type. ~50 LOC.
- [ ] **P1 — `FluentPageBuilder::measure(&str) -> f32`** —
      `src/writer/document_builder.rs`. One-line adapter over the
      private `text_layout`. Closes #1 for callers who want autosize
      / custom alignment. ~5 LOC.
- [ ] **P2 — `FluentPageBuilder::table(table) -> Self`** —
      `src/writer/document_builder.rs` + rewrite of
      `src/writer/table_renderer.rs::Table::render` to emit
      `ContentElement`s instead of `ContentStreamBuilder` ops.
      Uses #2, #3, #4 above. The actual table surface.
- [ ] **P3 (defer) — frame / section abstraction (#9).**
      Wait until a second content type (long paragraphs that span
      pages) asks for it. Building it now for tables alone risks
      over-design.

## Recommended reuse

**Reuse `src/writer/table_renderer.rs`, not `src/html_css/layout/tables.rs`.**

### `table_renderer.rs` — pros
- Full domain model: `Table`, `TableCell`, `TableRow`, `TableStyle`,
  `Borders`, `CellPadding`, `ColumnWidth{Auto|Fixed|Percent|Weight}`,
  `CellAlign`, `CellVAlign` — all publicly re-exported
  (`mod.rs:167-170`).
- Layout solver already written: `calculate_column_widths`
  (`table_renderer.rs:701-788`, handles `Auto`/`Fixed`/`Percent`/`Weight`
  distribution), `calculate_row_heights` (`:790-835`, computes wrapped
  text height per cell), `calculate_cell_positions` (`:837-870`,
  colspan/rowspan-aware).
- Renderer (`Table::render`, `:872-990`) already handles backgrounds,
  per-row stripe, header-row detection, outer border, per-side cell
  borders, alignment.
- Test coverage is present: `test_table_layout_calculation`,
  `test_text_wrapping`, `test_cell_alignments`, `test_striped_table`,
  etc. (`table_renderer.rs:1213-1265`).

### `table_renderer.rs` — cons
- Renders to `ContentStreamBuilder` directly, bypassing the subsetter
  (see #10). **Must be rewritten to emit `ContentElement`s** before
  use from the fluent builder.
- Text rendering is single-line only (`:968-969`: "Simple single-line
  rendering for now, builder.text(&cell.content, text_x, text_y);").
  Row-height calc already accounts for wrapping (uses
  `wrap_text(..., content_width, ...)` at `:817`), so heights are
  right but rendering draws all content on one line — **bug**, needs
  a fix whether we reuse or not.
- No vertical alignment in `render` (`CellVAlign` field exists at
  `:37-45` but is never read in `render`).
- No pagination — `Table::calculate_layout` returns a single
  `total_height` with no split logic. Issue #4 has to be solved
  *and* `render` extended to take a vertical range.

### `src/html_css/layout/tables.rs` — why not this?
- Only produces `Vec<f32>` column widths and `Vec<f32>` row heights
  from abstract `CellHint` (min/max px) + `RowHint`. No concept of
  `TableCell` with content, font, color, border. No renderer. No
  header/body/footer grouping beyond a `RowGroupKind` enum
  (`tables.rs:18-27`) that the CSS layout consumer uses.
- It's a **width-and-height solver for a box-tree integration**, not a
  drawing API. The width algorithm (auto / fixed,
  `tables.rs:84-165`) is clean and could inspire a refactor of
  `table_renderer::calculate_column_widths`, but the two modules have
  different problem shapes.

### Hybrid option
`table_renderer.rs` keeps its public model types (user-facing); its
layout solver could internally delegate to
`html_css::layout::tables::compute_column_widths` for consistency
between HTML-sourced and builder-sourced tables. Low priority — a
v0.3.40 cleanup, not a #393 blocker.

## Risks / unknowns

1. **Dynamic glyph registration across `new_page_same_size()`.** When
   `done() → page()` rebuilds a `FluentPageBuilder`, a fresh
   `TextLayout::new()` is constructed (`document_builder.rs:999`) —
   its `FontManager` is **independent** of the one inside the
   `PdfWriter::embedded_fonts` that actually drives subsetting
   (`pdf_writer.rs:152-172`). Prototype: register an embedded font,
   draw a CJK string on page 1, force a new page, draw another CJK
   string on page 2; confirm both glyph sets are in the final subset.
   If they aren't, #4's `new_page_same_size` must thread the same
   `TextLayout`.

2. **Header-row repetition vs `PageTemplate.header`.** Two different
   "headers" collide: the `PageTemplate` document header (#5) and a
   `Table`'s thead that should repeat when the table splits across
   pages. Prototype a two-page table with a `PageTemplate` header
   active and verify draw order + no overlap. Concrete risk:
   `template.get_header(page_number)` (`document_builder.rs:1054`)
   always draws; the table would need to lay out with reduced top
   margin.

3. **`table_renderer::Table::render`'s single-line bug
   (`:968-969`).** Row heights are calculated for wrapped text but
   rendering draws one line. Fixing this requires a wrap-loop in
   `render` — the same loop `paragraph` uses. Before committing to
   reuse, prototype `render` with actual multi-line wrapping and
   per-cell `valign` to confirm the ~1 200-line module is still the
   right base vs. a rewrite inside `document_builder.rs`.
