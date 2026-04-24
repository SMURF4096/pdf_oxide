# Scalable table layout — algorithmic state of the art

Research brief supporting pdf_oxide issue #393 (MigraDoc crashes at ~30k rows).
Goal: decide an algorithm + API shape that keeps rendering **linear in row count
and constant in memory** beyond the current page.

---

## TL;DR

- **Winning approach for 30k+ rows streaming: fixed-width columns + sampling-based
  autofit + row-granularity page splits + zero retained row history.** This is the
  pattern used by ReportLab's `LongTable` (with `_longTableOptimize`) and CSS
  `table-layout: fixed`. Typst's grid layouter is the gold standard for correctness
  but it deliberately holds the full cell matrix, so it is *not* a fit for streaming.
- **What the pdf_oxide API must permit:**
  1. Column widths resolvable **without inspecting any row** (explicit widths, or
     widths derived from a user-supplied *sample* — e.g. header row or first N
     rows).
  2. `push_row(cells)` that measures, draws, advances `current_y`, and **drops the
     row** — no `Vec<Row>` accumulation in the writer.
  3. Page-break trigger = row overflow; library flushes the page, repeats the
     header band, and continues.
  4. `finish()` closes the last page. No whole-table finalisation pass.
- **What we must deny the user, or charge them for:** "autofit everything based on
  the longest cell anywhere in the table." That is the O(rows) scan that MigraDoc
  does and the reason auto CSS tables cannot stream. Offer it as `TableMode::AutoAll`
  with a documented memory footprint of O(rows × cols), gated behind an opt-in.

---

## Case studies

### ReportLab `LongTable` (Python)

**Source-code pointer:** `src/reportlab/platypus/tables.py` in the ReportLab OSS
mirror (https://hg.reportlab.com/hg-public/reportlab, also mirrored at
`github.com/MatthewWilkes/reportlab/blob/master/src/reportlab/platypus/tables.py`).
Relevant symbols: `class Table` (~line 400), `_calcPreliminaryWidths` (~line 1050),
`_calc_height` (~line 620), `_splitRows` (~line 1320), `class LongTable` (~line
1430).

**`LongTable` is a two-line subclass:**

```python
class LongTable(Table):
    '''Henning von Bargen's changes will be active'''
    _longTableOptimize = 1
```

Everything interesting happens inside `Table._calc_height`, where the flag is
checked:

```python
longTable = self._longTableOptimize
...
while None in H:
    ...
    if longTable:
        hmax = i
        height = sum(H[:i])
        if height > availHeight:
            if spanCons:
                msr = max([x[1] for x in spanCons.keys()])
                if hmax >= msr:
                    break   # <— early exit
```

**Memory model:** All rows are still held in `self._cellvalues` (a `[[cell]]`
matrix) — ReportLab is not truly streaming. What `LongTable` gives up is the
*recomputation* pass: once accumulated height exceeds the page height and all
active row-spans are closed, further row heights are not measured for this page.
The next page re-enters the loop at the split row.

**Autosize algorithm (`_calcPreliminaryWidths`):**

```python
for colNo in range(self._ncols):
    w = W[colNo]
    if w is None or w == '*' or w.endswith('%'):
        for rowNo in range(self._nrows):      # O(rows * cols) scan
            measure cell(rowNo, colNo)
```

So ReportLab's autofit is O(rows × cols). `LongTable` does NOT skip this. The
LongTable "speed" win is in the *height* pass, not the *width* pass. If the user
supplies explicit column widths, this nested loop is skipped entirely.

**Page-break handling:** `Table._splitRows(availHeight)` finds the first legal
split row (respects rowspans, nosplit markers) and returns *two new `Table`
instances* — `R0` (rows 0..k) and `R1` (`data[:repeatRows] + data[k:]` when
`repeatRows` is set). This is elegant but **allocates O(remaining rows) on every
split**. For 30k rows over 1000 pages that is 1000 × 29 000 = 2.9 × 10⁷ cell-refs
copied. Fine in Python, ruinous for our goal.

**Header repetition:** `repeatRows: int` on `Table`; `_splitRows` re-prepends
`data[:repeatRows]` plus the appropriate style-command subset to `R1`. Cheap in
concept, but combines with the above O(n) split to stay quadratic-ish.

### QuestPDF (C#)

**Source-code pointer:** `github.com/QuestPDF/QuestPDF`, `Source/QuestPDF/`. Key files:
- `Infrastructure/SpacePlan.cs` — four variants: `Empty`, `Wrap(reason)`,
  `PartialRender(size)`, `FullRender(size)`.
- `Elements/Table/Table.cs` — `internal sealed class Table : Element, IStateful`.
- `Elements/Table/TableLayoutPlanner.cs` — per-row layout planning.

**The pull-based lifecycle** (Element.cs / Table.cs):

```csharp
internal override SpacePlan Measure(Size availableSpace) { ... }
internal override void Draw(Size availableSpace) { ... }
```

Every element implements both phases. The page driver walks the tree top-down:

1. `Measure(space)` asks "given this much room, what can you render?"
2. The child returns `SpacePlan`:
   - `FullRender(w,h)` → you fit, draw me here.
   - `PartialRender(w,h)` → I drew what I could, call me again on the next page.
   - `Wrap(reason)` → nothing fits; flush page, give me a fresh region.
3. `Draw(space)` is called matching the Measure result. The element **mutates its
   `CurrentRow` state** via `IStateful`. Next page, `Measure` is called again and
   picks up where it left off.

**Table autofit:** QuestPDF requires explicit column definitions:

```csharp
public List<TableColumnDefinition> Columns { get; set; }   // ConstantSize + RelativeSize
```

`UpdateColumnsWidth(availableWidth)` distributes available space among columns
with `RelativeSize` after subtracting `ConstantSize`:

```csharp
var constantWidth   = Columns.Sum(x => x.ConstantSize);
var relativeWidth   = Columns.Sum(x => x.RelativeSize);
var perUnit         = (availableWidth - constantWidth) / relativeWidth;
foreach (var c in Columns) c.Width = c.ConstantSize + c.RelativeSize * perUnit;
```

**There is no content-driven column sizing.** The user declares the grid; the
library only resolves the relative units. O(cols), not O(rows).

**Memory model:** `CellsCache` is `TableCell[][]` grouped by *ending row*. All
cells are still held in memory — but `IsRendered => CurrentRow > LastRowIndex`
means Measure/Draw incrementally advance a single integer and the system can
technically be re-entered page after page without re-allocating. The engine
itself does **not** stream cells from an iterator; the API forces the full cell
list up front.

**Page-break handling (Table.cs):**

```csharp
return CalculateCurrentRow(renderingCommands) > LastRowIndex
    ? SpacePlan.FullRender(tableSize)
    : SpacePlan.PartialRender(tableSize);
```

`PartialRender` signals the page driver that more rows remain; the next
`Measure`/`Draw` cycle resumes at `CurrentRow`. Row-granularity is the default;
cells with large content can wrap through the same mechanism (their own
`PartialRender`).

**Header repetition:** implemented as a dedicated `RepeatContent` element
(`Elements/RepeatContent.cs`) that re-renders on every page. Cheap: the header
component is evaluated once per page, not once per row.

### Typst (Rust)

**Source-code pointer:** `crates/typst-layout/src/grid/` in `github.com/typst/typst`.
Key files: `layouter.rs` (the `GridLayouter` struct), `rowspans.rs`, `lines.rs`.

**Column width resolution** — `measure_columns()` is a three-phase algorithm:

1. **Fixed:** relative-sized columns resolved via
   `v.resolve(styles).relative_to(regions.base().x)`. Fractional (`Fr`) units
   accumulated but not yet assigned.
2. **Auto:** for each auto column, lay out **every cell in that column** to find
   the max required width. Colspans only affect the last auto column they span.
3. **Fractional:** distribute remaining space to `Fr` columns; if negative,
   iteratively shrink the largest auto columns via `shrink_auto_columns()`.

Phase 2 is **O(rows × auto-cols × measure-cost)**. Typst accepts this because
documents are typically small and the layout is memoised (`comemo`). For a 30k
document-table this is not acceptable — but the user can opt into `fixed` /
fractional columns and phase 2 becomes a no-op.

**Row pagination** — `layout_row()` per row, then:

```
if self.regions.is_full() {
    self.finish_region(engine, false)?;   // advance to next region
}
```

Row types: `auto` (measure then possibly split via `layout_multi_row`),
`relative` (forced break if it doesn't fit), `fractional` (sized after all
relatives at `finish_region()`). Row-granularity pagination is native; cell
content can split across regions for auto rows.

**Memory model** — the layouter keeps:
- `Current` (per-region state: `lrows`, header heights, orphan snapshots),
- `RowState` (per-row transient),
- `rowspans: Vec<...>` (incomplete rowspans across regions),
- `finished: Vec<Frame>`, `rrows: Vec<RowPiece>`, `finished_header_rows`,
- `cell_locators: HashMap<Axes<usize>, _>` for relayout of cells under
  different disambiguators (repeated headers).

Everything is held. Typst is a document compiler, not a streaming writer. The
algorithm is exemplary; the data structures are not usable as-is for our
constraint.

**Headers** — `repeating_headers`, `pending_headers`, `upcoming_headers` vectors.
`place_new_headers()` inserts them at region tops; `current.repeating_header_height`
is subtracted from auto-row target heights. This is the cleanest header model
surveyed; we can copy its *API shape* even if we don't copy its data structures.

### CSS `table-layout: fixed` vs `auto` (browsers)

**Spec pointer:** W3C CSS 2.1 §17.5 "Visual layout of table contents"
(https://www.w3.org/TR/CSS21/tables.html), CSSWG drafts
https://drafts.csswg.org/css-tables-3/.

**`auto` algorithm** (what Chrome/Firefox/Safari implement):

```
for each cell:
    compute min-content-width, max-content-width
for each column:
    col.min = max(cell.min for cell in column)
    col.pref = max(cell.pref for cell in column)
distribute remaining table width proportionally across columns
```

Cost: **O(rows × cols × cell-measure)** *and* requires the whole table to be
parsed before first paint. The CSSWG spec acknowledges the auto algorithm is
intentionally under-specified because no one can make it fast in general.

**`fixed` algorithm** (CSS 2.1 §17.5.2.1, ~10 lines of spec):

```
for each column in the first row:
    if column element has explicit width: col.width = that
    elif first-row cell has explicit width: col.width = that
    else: col.width = remainder_of_table_width / n_unsized_columns
render every row using the column widths just computed
```

Cost: **O(cols) for width computation, O(rows) for rendering**. The browser can
emit the first row as soon as it parses the first `<tr>`. Content that doesn't
fit is clipped via `overflow`.

**What `fixed` gives up:** content-driven column sizing. Requires either:
- explicit `table { width: X }` + `<col width=...>` entries, or
- a sentinel first row whose cell widths describe the grid.

This is exactly the compromise we want to expose.

### MigraDoc — why it is O(rows²)

**Source-code pointer:** `MigraDoc/code/MigraDoc.Rendering/MigraDoc.Rendering/TableRenderer.cs`
in `github.com/empira/MigraDoc-1.5` (also mirror `github.com/DavidS/MigraDoc`).
The bug is tracked in `empira/MigraDoc-1.5#13` ("O(n) instead of O(n²) for
TableRenderer.CreateConnectedRows()").

**The hot path:**

```csharp
void CreateConnectedRows()
{
    foreach (Cell cell in this.mergedCells)                         // O(n)
    {
        if (!this.connectedRowsMap.ContainsKey(cell.Row.Index))
        {
            int lastConnectedRow = CalcLastConnectedRow(cell.Row.Index);
            ...
        }
    }
}

int CalcLastConnectedRow(int row)
{
    foreach (Cell cell in this.mergedCells) { ... }                 // O(n) again
}
```

Nested iteration over every merged cell → **O(rows²)** once "merged cells"
scales with row count (which it does for any table with per-row borders or
rowspans). The issue reporter demonstrated 644 rows = several seconds; scaling
to 30k → hours-to-OOM. Secondary O(n²) surfaces: `bottomBorderMap` rebuilt
every probe, `Format()` called for the whole table before first page flushes.

**The invariant MigraDoc violates:** *per-row work must be local to the current
row*. `CalcLastConnectedRow` walks the **entire** merged-cells list to find the
last row of a span that the caller already has in its hand. The O(n) fix builds
a single map keyed by row index and probes it in O(1).

But even the O(n) fix does not solve the deeper issue: MigraDoc formats the
whole table before emitting any page. **No row is drawn until every row is
measured.** The architecture is wrong for streaming; the merge map fix only
makes the wrong architecture linear instead of quadratic.

---

## Proposed algorithm for pdf_oxide

### Shape

Two table modes, declared at construction:

- **`TableMode::Fixed { widths: &[Length] }`** — widths fully specified.
  Zero measurement pass. O(cols) setup, O(rows × cols) rendering, O(cols + 1 row)
  memory.
- **`TableMode::Sample { widths: ColumnWidths, sample_rows: usize }`**
  (default `sample_rows = 1`, i.e. just the header) — measure the sample, fix
  widths, stream the rest. O(sample_rows × cols) setup + O(rows × cols) render.

A third `TableMode::AutoAll` may be offered later, explicitly documented as
O(rows × cols) memory and O(rows × cols) time with no streaming. **Not**
enabled by default.

### Algorithm sketch

```text
// construction
let mut table = pdf.table()
    .columns(&[Length::Pct(40), Length::Pct(60)])   // Fixed mode
    .header(|row| { row.cell("Name"); row.cell("Email"); })
    .repeat_header(true)
    .build(&mut page);

// widths are resolved once, now:
resolve_widths(table.columns, page.content_width)      // O(cols)
draw_header(table, page)                               // O(cols)
page.current_y -= header_height

// streaming loop (user code)
for record in huge_dataset {                           // never materialised
    table.push_row(|row| {
        row.cell(record.name);
        row.cell(record.email);
    })?;
}
table.finish()?;
```

Inside `push_row`:

```text
fn push_row<F>(&mut self, build: F) -> Result<()> {
    // 1. Build the row into O(cols) transient cell descriptors.
    let cells = build_cells(build, self.columns);

    // 2. Measure row height at frozen column widths.
    //    This is O(cols * content_measure_cost) — bounded per row.
    let h = max(measure(cell, col.width) for (cell, col) in zip(cells, cols));

    // 3. Page-break check.
    if self.page.current_y - h < self.page.bottom_margin {
        self.page.finish();                    // flushes content stream
        self.page = self.pdf.new_page();
        if self.repeat_header { draw_header(self, self.page); }
    }

    // 4. Emit cell content streams at (current_x, current_y).
    for (cell, col) in zip(cells, cols) {
        draw_cell(self.page.content_stream, cell, col.x, self.page.current_y, col.width, h);
    }

    // 5. Advance y. Drop the row.
    self.page.current_y -= h;
    Ok(())
}
```

### Memory bound

- `table.columns`: `O(cols)`.
- `table.header_cells`: `O(cols)` (retained for repeat-header re-draw).
- Per-call transient `cells`: `O(cols)`, dropped at end of call.
- Content stream buffer: `O(current-page bytes)` — already the case for any
  pdf_oxide page.

Total persistent state: **O(cols)**. No row index, no row heights, no
rowspan bookkeeping retained beyond the current row (if we forbid rowspans in
streaming mode — see open question 3).

### Time bound

- Width resolve: O(cols) per page (redo on page break because page width may
  change, but typically no-op).
- Per row: O(cols × cell-measure). Cell-measure is bounded by the cell's own
  text length; independent of row count.
- Total: **O(rows × cols × avg-cell-measure)**. No quadratic term.

### API constraints this imposes on the caller

1. **Column widths must not depend on rows the caller hasn't pushed yet.**
   Either `Fixed` (explicit) or `Sample` (first N rows seen become canonical).
   Mode is fixed at `build()` time.
2. **No across-table rowspan in streaming mode.** A rowspan needs future-row
   knowledge the writer doesn't have. Allow rowspan only in `AutoAll`, or
   only within a small bounded look-ahead window.
3. **The header is a closure/block, not a row index**, so it can be re-invoked
   on each page without replay of any data row.

### API freedoms this preserves

- Fluent `push_row(|r| r.cell(...))` builder — matches our other fluent APIs.
- Content inside a cell can still be rich (text runs, images) — the measure
  call just becomes richer.
- `finish()` returns normally; no table-level finalisation that touches all
  rows.
- Works on top of our existing incremental content-stream writer — the
  per-row draw is just another call to the same `BT/Tj/ET` emitter. No new
  buffering layer.
- Parity across all 7 bindings: the API is row-at-a-time, which maps cleanly
  to Python generators, WASM callbacks, Go iterators, Node async iterators,
  etc.

---

## Open questions

1. **Sample-mode width policy.** If `sample_rows = 1` (header-only), what
   happens to data rows whose cell content exceeds the header-derived width?
   Options: clip, wrap inside cell, or grow the column (which forces re-flow
   of prior pages and breaks streaming). CSS `fixed` picks clip/wrap; QuestPDF
   picks wrap. Proposal: **wrap by default, expose `Overflow::Clip`**.

2. **Retroactive page re-flow.** If cell N on page 5 doesn't fit the header
   width, do we force-widen the column? In streaming mode we **cannot** —
   pages 1–4 are already serialised to the output. Accept this limitation and
   document it: "in Sample mode, late-discovered content wider than the
   sampled width is wrapped, not re-flowed". Prototype to verify the wrap
   path produces sensible output on the Kreuzberg fixtures.

3. **Rowspan support.** Pure streaming is incompatible with unbounded
   rowspans. Three candidate policies: (a) reject rowspans in streaming mode,
   (b) allow rowspans bounded by N rows with an N-row look-ahead buffer,
   (c) require the caller to declare rowspan length up front so the writer
   can reserve and back-fill. Decide before freezing the API surface.

4. **Multi-page cell splitting.** When a single cell's content exceeds the
   remaining page height, do we page-break inside the cell (Typst-style
   `layout_multi_row`) or push the whole row to the next page (QuestPDF
   default)? The latter is O(1) to implement; the former needs a
   partial-render return path similar to QuestPDF's `SpacePlan::PartialRender`.
   Proposal: whole-row next-page for v0.3.39; revisit cell splitting in
   v0.3.40 if users ask.

5. **Header component cost.** QuestPDF's `RepeatContent` re-evaluates the
   header closure per page. For an expensive header (e.g. embedded image),
   we should cache the measured/rendered content-stream snippet after the
   first page and blit it on subsequent pages. Does our content-stream writer
   support splicing a cached byte range? (Confirm vs. the current writer
   layer referenced in `ci: disable Semver check pre-1.0` churn.)

---

## Source references

- ReportLab: `src/reportlab/platypus/tables.py`
  (github.com/MatthewWilkes/reportlab, hg.reportlab.com), classes `Table` and
  `LongTable`; methods `_calc_height`, `_calcPreliminaryWidths`, `_splitRows`.
- QuestPDF: `Source/QuestPDF/Elements/Table/Table.cs`,
  `Source/QuestPDF/Drawing/SpacePlan.cs`,
  `Source/QuestPDF/Elements/RepeatContent.cs`
  (github.com/QuestPDF/QuestPDF).
- Typst: `crates/typst-layout/src/grid/layouter.rs`,
  `crates/typst-layout/src/grid/rowspans.rs`
  (github.com/typst/typst).
- CSS: W3C CSS 2.1 §17.5
  (https://www.w3.org/TR/CSS21/tables.html); CSSWG drafts
  https://drafts.csswg.org/css-tables-3/.
- MigraDoc: `MigraDoc/code/MigraDoc.Rendering/MigraDoc.Rendering/TableRenderer.cs`
  (github.com/empira/MigraDoc-1.5, issue #13); mirror
  github.com/DavidS/MigraDoc.
- Our issue: pdf_oxide #393.
