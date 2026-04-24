# #393 — DocumentBuilder tables: decision doc

**Status:** DRAFT (2026-04-23) — synthesized from
[research A](../research/a_table_api_landscape.md),
[B](../research/b_scalable_layout_algorithms.md),
[C](../research/c_api_ergonomics.md),
[D](../research/d_builder_gap_analysis.md).

Target release: **v0.3.39** (Phase 1) + **v0.3.40** (Phase 2).

---

## TL;DR

Ship **two** table APIs in the same release, sharing one type system:

1. **`Table` (buffered)** — accepts full `TableCell` matrix, supports
   colspan / rowspan / rich cells, page-splits row-granularity.
   Reuses the existing `src/writer/table_renderer.rs` engine (1,269 LOC
   already written — D's key discovery). Target: 0 → ~1,000 rows. One
   fluent call: `.table(Table::new().rows(data))`.

2. **`StreamingTable` (row-at-a-time)** — `TableMode::Fixed` only in
   v0.3.39 (explicit widths, zero look-ahead), flushes each row directly
   into the content stream, `O(cols)` persistent memory, no rowspan.
   Target: 1,000 → ∞ rows. Two fluent calls: `.streaming_table(...)` to
   start + `.push_row(|r| r.cell(...))?` per row. Solves MigraDoc's
   30k-row failure directly.

Both surfaces share `Column`, `ColumnWidth`, `CellAlign`, `TableStyle`,
`Borders`, `CellPadding` — the type vocabulary defined once in Rust and
mirrored idiomatically per binding per [research C](../research/c_api_ergonomics.md).

**Shipping in v0.3.39 (every binding):**

- 4 supporting `FluentPageBuilder` primitives from
  [research D](../research/d_builder_gap_analysis.md).
- `Table` buffered API (reuses + fixes `table_renderer.rs`).
- `StreamingTable` + `TableMode::Fixed`.
- FFI-batching (C's Arrow/DuckDB Appender pattern) for the streaming
  path across Python / Node / C# / Go.
- All 6 bindings: Rust, Python, WASM, C#, Go, Node/TS.

**Deferred to v0.3.40:**

- `TableMode::Sample` (measure first N rows → freeze widths).
- `TableMode::AutoAll` (O(rows × cols) memory opt-in).
- Cross-page cell splitting.
- Bounded-lookahead rowspan in streaming mode.

---

## Decision axes

### 1. Buffered vs streaming: both, sharing types

[Research B](../research/b_scalable_layout_algorithms.md) established
that **nobody actually streams** — not ReportLab `LongTable`, not
QuestPDF, not Typst. The only mainstream library that does is iText7's
`LargeElement` (per [research A](../research/a_table_api_landscape.md)),
and it does it via a `flushContent()` + `isComplete` boolean rather
than a row iterator.

Meanwhile, D revealed that we already have a full **buffered** table
engine in `src/writer/table_renderer.rs` (1,269 LOC:
`Table`, `TableCell`, `TableStyle`, `Borders`, `CellAlign`,
`ColumnWidth`, column-width solver, row-height solver, positioner,
renderer — all re-exported at `src/writer/mod.rs:167-170`). It is
**unreachable from `FluentPageBuilder`** because it writes to
`ContentStreamBuilder` directly instead of emitting `ContentElement`s,
and it has a latent bug at `:968-969` (wraps for height calc, renders
single-line).

Both research paths converge on the same shape decision: **offer both.
Buffered is the dominant use case (small-to-medium tables with
colspan). Streaming is the motivating 30k-row case the issue was
filed for.** Shipping one without the other fails half the users; we
can afford both because the buffered engine already exists and the
streaming engine is less code than the buffered one (no colspan, no
rowspan, no full-matrix layout solver).

### 2. API shape: column-declared + row-streamed (C's load-bearing pattern)

[Research C](../research/c_api_ergonomics.md) identified a single
pattern that feels native in all 6 languages: **declare columns
eagerly (schema), feed rows lazily (data)**. This holds whether rows
arrive as a `Vec`, an `Iterator`, an `AsyncIterable`, an
`IAsyncEnumerable`, a Go channel, or a Python generator.

We adopt this for both `Table` and `StreamingTable`. The Rust surface:

```rust
// Buffered — reasonable-size table, full feature set
page.table(
    Table::new()
        .column(Column::text("SKU").width_pct(20.0))
        .column(Column::text("Item").width_auto())
        .column(Column::number("Units").align(Align::Right))
        .column(Column::currency("Revenue").align(Align::Right))
        .rows(sales_iter)?            // materialised here
        .style(TableStyle::striped())
);

// Streaming — unbounded rows, Fixed widths
let mut t = page.streaming_table(
    Table::new()
        .column(Column::text("SKU").width_pt(72.0))
        .column(Column::text("Item").width_pt(240.0))
        .column(Column::number("Units").width_pt(48.0).align(Align::Right))
        .column(Column::currency("Revenue").width_pt(80.0).align(Align::Right))
        .repeat_header(true)
        .begin()
)?;
for record in huge_dataset {          // never materialised
    t.push_row(|r| {
        r.cell(record.sku);
        r.cell(&record.name);
        r.cell(record.units);
        r.cell(record.revenue);
    })?;
}
t.finish()?;
```

Note: `Table::new().rows(iter)?` materialises (buffered). Calling
`.begin()` instead transitions to streaming mode — the same fluent
chain forks at the last call.

### 3. Width vocabulary: four primitives, three supported in streaming

From [research A's matrix](../research/a_table_api_landscape.md) — the
four widths every serious library speaks are **absolute pt**, **percent
of table width**, **fraction/star** (relative), and **auto / content-driven**.

- **Buffered `Table`:** all four. `ColumnWidth::Auto | Fixed(pt) |
  Percent(pct) | Weight(w)` — exactly what `table_renderer.rs::ColumnWidth`
  already implements.
- **Streaming `StreamingTable`:** three. `Fixed | Percent | Weight`.
  `Auto` is forbidden (it requires lookahead) — the compiler enforces
  this via a separate `StreamingColumnWidth` type. Callers who pass
  `Auto` get a compile error with a message pointing at `TableMode::Sample`
  (v0.3.40).

### 4. Pagination: row-granularity splits, header closure

Both modes split on row boundaries — **no mid-cell splitting** in v0.3.39
(the v0.3.40 candidate per B's open question #4).

Header repetition uses the QuestPDF `RepeatContent` closure pattern
(B, with citations): `.repeat_header(true)` stores the header-row cells
and re-draws them on every page break. This is cheaper than trying to
re-invoke an arbitrary header closure — we have the header cells in
hand at `.begin()` time.

### 5. Rowspan / colspan

- **Buffered `Table`:** colspan + rowspan supported. `table_renderer.rs`
  already has colspan/rowspan-aware `calculate_cell_positions`
  (`:837-870`). No new work beyond the ContentElement migration.
- **Streaming `StreamingTable`:** neither. Rowspan needs future-row
  knowledge the writer doesn't have; colspan is implementable but
  deferred for v0.3.39 budget. Both return a typed error if attempted.

### 6. FFI-batching (streaming only)

Per [research C's cross-cutting decision](../research/c_api_ergonomics.md#ffi-batching-strategy-invisible-to-caller):
a per-binding `RowBatch { cells: SmallVec<Cell; 256> }` lives on the
managed side, each `AddRow` / `push_row` / `row()` appends into it,
and every ~64 rows crosses FFI once with a pointer to the packed batch.
Target: **< 50 ns amortized per row() call** in non-Rust bindings.

Arrow/DuckDB Appender pattern; both are widely understood by users and
have high-quality reference implementations. Buffered `Table` doesn't
need this because it crosses FFI once at `.build()`.

### 7. Reuse vs rewrite

From [D's recommendation](../research/d_builder_gap_analysis.md#recommended-reuse):

- **Reuse** `table_renderer.rs` for buffered. Rewrite `render()` to
  emit `ContentElement`s instead of `ContentStreamBuilder` ops (so the
  v0.3.38 subsetter keeps working for CJK cell text).
- **Fix** the latent single-line rendering bug at `table_renderer.rs:968-969`.
- **Reuse** its public types (`Table`, `TableCell`, `Column`, `TableStyle`,
  `Borders`, `CellPadding`, `CellAlign`, `ColumnWidth`) as the shared
  vocabulary across buffered and streaming.
- **Do not** reuse `src/html_css/layout/tables.rs` as the drawing engine
  — it's a width-and-height solver for HTML integration, not a
  drawing surface. Might delegate width solving to it in v0.3.40 for
  consistency between HTML-sourced and builder-sourced tables; not a
  v0.3.39 blocker.
- **Streaming engine is new code**, ~400 LOC, sits alongside
  `table_renderer.rs` rather than extending it.

---

## The four supporting primitives (v0.3.39, all bindings)

Per [D](../research/d_builder_gap_analysis.md#recommended-additions-alongside-tables),
shipped in this order:

### P1.1 — `FluentPageBuilder::measure(&str) -> f32`
- **Location:** `src/writer/document_builder.rs`.
- **Size:** ~5 LOC + tests.
- **Why it blocks tables:** streaming `TableMode::Fixed` users need to
  declare widths; they want to measure their header/sample data first
  to pick sane widths. Without a public `measure`, callers can't
  pick good widths, and we ship a worse-than-MigraDoc UX.

### P1.2 — `FluentPageBuilder::text_in_rect(rect, text, align)`
- **Location:** `src/writer/document_builder.rs`. Delegates to existing
  `TextLayout::wrap_text`.
- **Size:** ~40 LOC + tests.
- **Why it blocks tables:** cells ARE rects with wrapped text. Without
  this primitive, the table engine has to reimplement wrapping (which
  is what `table_renderer.rs:968-969` tried and got wrong). Fixing the
  primitive fixes the renderer bug for free.
- **Side benefit:** closes the dead `TextConfig.align` field
  (`document_builder.rs:1492`) — it'll finally be honoured.

### P1.3 — `FluentPageBuilder::remaining_space() -> f32` +
### `FluentPageBuilder::new_page_same_size() -> Self`
- **Location:** `src/writer/document_builder.rs`.
- **Size:** ~25 LOC + tests.
- **Why it blocks tables:** streaming mode needs to know when to flush
  the page and start another. Without `remaining_space()` the table
  engine has to re-implement cursor tracking, and without
  `new_page_same_size()` the page-size + text_config + font registrations
  reset awkwardly on break.

### P1.4 — `FluentPageBuilder::stroke_rect(rect, LineStyle)` +
### `FluentPageBuilder::stroke_line(p1, p2, LineStyle)`
- **Location:** `src/writer/document_builder.rs`. New `LineStyle { width,
  color, dash: Option<Vec<f32>> }` type.
- **Size:** ~50 LOC + tests.
- **Why it blocks tables:** cell borders with per-side thickness / dashed
  rules / coloured lines. `rect` draws a filled quad and `line` draws a
  path; neither takes a `LineStyle`. Table rendering today works around
  this by duplicating PDF content-stream ops — that hurts subsetter
  parity.

**Budget for all four primitives: ~120 LOC of Rust core + ~120 LOC of
per-binding adapter per of 6 = ~840 LOC total, spread over all bindings.
Each primitive lands as one commit.**

---

## Scope split: v0.3.39 vs v0.3.40

### v0.3.39 — "tables land"

Rust core + all 6 bindings in one release (v0.3.38 discipline):

1. Fix `table_renderer.rs:968-969` multi-line rendering bug (1 commit).
2. Land the 4 supporting primitives, one commit each (4 commits).
3. Migrate `Table::render` to emit `ContentElement`s (1 commit).
4. Wire `FluentPageBuilder::table(Table)` — buffered path (1 commit).
5. Implement `StreamingTable` + `TableMode::Fixed` in Rust core
   (1-2 commits).
6. Per-binding DocumentBuilder `table()` / `streaming_table()` surface
   — Python / WASM / C# / Go / Node-TS (5 commits, one per binding).
7. Per-binding FFI-batching for streaming rows, Arrow/DuckDB style
   (3 commits — Python + Node + C#/Go share a pattern).
8. 30k-row benchmark in `tools/benchmark-harness/` with linear-scaling
   assertion (1 commit).
9. CHANGELOG + README + examples + CJK integration test (1 commit).

**Total commits: ~17–20.** All reversible. Benchmark is the release-gate.

### v0.3.40 — "tables grow up"

- `TableMode::Sample { sample_rows: N }` — measure first N rows, freeze
  widths. (Addresses research B open question #1.)
- Bounded-lookahead rowspan in streaming mode (research B open question #3).
- Cross-page cell splitting for rich cells (research B open question #4).
- `TableMode::AutoAll` — opt-in, O(rows × cols) memory, documented as
  a trade-off.
- Hybrid option from [D](../research/d_builder_gap_analysis.md#hybrid-option):
  delegate width-solving to `html_css::layout::tables` for HTML/builder
  consistency.
- Pandas DataFrame adapter in Python (research C's unresolved tension).

---

## Acceptance criteria (v0.3.39)

- [ ] Rust core: `Table`, `StreamingTable`, `Column`, `ColumnWidth`,
      `CellAlign`, `TableStyle`, `Borders`, `CellPadding`, `LineStyle`
      publicly exposed. Idiomatic fluent API: `page.table(...)`,
      `page.streaming_table(...)`.
- [ ] FFI in `include/pdf_oxide_c/pdf_oxide.h` for all of the above.
- [ ] Public idiomatic API in **all 6 bindings** (Rust, Python, WASM,
      C#, Go, Node/TS).
- [ ] Scale benchmark: **30,000-row × 5-column `StreamingTable`**, Latin
      content, completes on a 1 GB-RAM runner with linear scaling from
      1k → 30k rows. Benchmark lives under `tools/benchmark-harness/`
      and runs in CI at 5k rows.
- [ ] Per-binding regression tests for: 3×3 buffered table; header-repeats-
      on-page-break; 1k-row `StreamingTable`; empty cells; long single-cell
      wrap (buffered); colspan/rowspan (buffered only).
- [ ] CJK test case proves subsetter still works for dynamically-added
      cell text (closes the regression risk called out in
      [D](../research/d_builder_gap_analysis.md#font-subset-integration)).
- [ ] `text_in_rect`, `stroke_rect`, `stroke_line`, `remaining_space`,
      `new_page_same_size`, `measure` are **individually** tested on
      their own before tables use them.
- [ ] CHANGELOG entry under v0.3.38's write-side-addition pattern.
- [ ] Examples in `csharp/README.md` + `python/README.md` + `js/GUIDE.md`
      + `go/QUICK_REFERENCE.md` + main README.

---

## Risks / unknowns

Three things we should prototype before freezing the API:

1. **ContentElement migration of `Table::render`** — does emitting 100s
   of `ContentElement::Text` + `ContentElement::Rect` per table hurt
   the existing PageBuilder perf? Prototype: render a 50×5 table via
   the migrated path, time it vs. current direct-write path. Accept
   up to 2× slowdown (still in-memory; negligible at 50-row scale).

2. **FFI-batching for PyO3 specifically** — PyO3's `#[pyclass]` + GIL
   model means the "thread-local row buffer" has to be an instance
   attribute, not a `thread_local!`. Verify that per-`PdfDocument`-instance
   batching is cheap enough.

3. **Typst's column-resolver as a reference** — research B flagged it
   as gold-standard correctness. Before shipping our width solver,
   walk through Typst's `crates/typst-layout/src/grid/layouter.rs` for
   edge cases (fractional overflow, %-exceeds-100%, nested fractions).
   30 min of reading should cover it.

---

## Confidence + next action

Confidence in this plan: **high**. All four research inputs converge
cleanly; the most dangerous assumption (that the existing
`table_renderer.rs` can be adapted rather than replaced) is
de-risked by item (1) under "Risks" above — ~30 min of prototyping
settles it.

**Next action:** update issue #393 with this decision and link the four
research docs + this synthesis. Then start implementation: P1.1
`measure()` first (5 LOC, unblocks column-width sizing for every caller).
