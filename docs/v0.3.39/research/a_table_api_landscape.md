# Table APIs in OSS PDF libraries — landscape scan

Input for issue #393 (programmatic table-generation API). Catalog of API
*shape* and capability surface across 6 ecosystems; not a "which is best".

## Summary

- Two dominant shapes exist. (1) **Declarative 2-D data** (`data: [[...]]`)
  plus a separate style/layout object — ReportLab, jsPDF-AutoTable, pdfmake,
  PDFKit.js, fpdf2-style "take a matrix". (2) **Fluent/push-cell builder** —
  QuestPDF, iText7, UniPDF, genpdf, MigraDoc, easytable. Typst's
  `#table(columns: n, ..cells)` is a third, less common "variadic positional"
  shape that blurs the two.
- Column widths almost everywhere reduce to four primitives: **absolute pt**,
  **fraction/star** (`*`, `1fr`, relative units), **percent of table width**,
  and **auto/content-driven**. Every serious library supports at least two of
  the four; the best (QuestPDF, Typst, iText7 UnitValue, pdfmake) support three.
- Streaming (ability to emit rows before the whole table is built) is the
  single biggest differentiator and the main pdf_oxide lever. Only **iText7
  LargeElement** (`flushContent`/`isComplete`) and **UniPDF creator.Table**
  (draws when `Draw` called; rows can be added until then) give explicit
  multi-MB table support. MigraDoc and ReportLab's plain `Table` are the
  canonical "fall over above N rows" cases; ReportLab's `LongTable` and
  fpdf2's streaming `with pdf.table()` partially mitigate, but still hold
  all rows in memory.
- Header repeat on page break is universal where pagination is handled at all
  (ReportLab `repeatRows`, fpdf2 `repeat_headings`, QuestPDF `table.Header`,
  iText7 `setHeaderRows`, Typst `table.header(repeat: true)`, pdfmake
  `headerRows`, jsPDF-AutoTable `showHead: 'everyPage'`, UniPDF
  `SetHeaderRows`). Row-splits-mid-cell is rare and often buggy (QuestPDF
  issue #591, WeasyPrint issue #333).
- Row/colspan is mostly supported but uneven: ReportLab `SPAN`,
  fpdf2 `colspan`/`rowspan`, Typst `table.cell(colspan:, rowspan:)`, QuestPDF
  `.ColumnSpan`/`.RowSpan`, pdfmake, PDFKit.js, jsPDF-AutoTable, easytable,
  borb `TableCell(col_span=, row_span=)`. genpdf and pure printpdf/pdf-writer
  have no notion of spans.
- **Worth studying deeply in tasks B and C:** (1) **QuestPDF** (cleanest
  fluent builder with explicit header/footer/Row/Column grid — the model
  most likely to feel idiomatic in Rust), (2) **iText7 LargeElement**
  (the only mainstream API that solves streaming correctly for 100k+ row
  reports), (3) **Typst `table`** (compact declarative syntax already
  proven inside a Rust codebase, closest to how pdf_oxide HTML/CSS pipeline
  models a table internally).

---

## Comparison matrix

| Library               | Lang     | Entry shape                    | Widths                   | Streaming                        | Pagination (header repeat / row split) | Col/Rowspan       | Scale known-good                  |
|-----------------------|----------|--------------------------------|--------------------------|----------------------------------|----------------------------------------|-------------------|-----------------------------------|
| ReportLab Table       | Py       | Declarative 2-D + TableStyle   | fixed pt, auto           | materialised                     | `repeatRows` yes / row-split via `splitLate` | SPAN              | Slow past ~few thousand rows      |
| ReportLab LongTable   | Py       | Same, greedy col-width         | fixed pt, auto (greedy)  | materialised                     | `repeatRows` yes / splits greedily     | SPAN              | Designed for "long" tables        |
| borb                  | Py       | Push cells into FixedCol/Flex  | fixed decimals, flexible | materialised                     | multi-page yes (issue #7 history)      | TableCell span    | Unknown; memory bound             |
| fpdf2 `pdf.table()`   | Py       | Context-manager builder        | pt or fractional         | **streaming rows** in-block      | `repeat_headings` yes / auto           | colspan/rowspan   | No doc limit                      |
| xhtml2pdf / WeasyPrint| Py       | HTML `<table>` + CSS           | CSS widths               | materialised                     | thead/tfoot repeat / limited in-cell   | HTML colspan/rowspan | WeasyPrint doc-large-perf issues |
| QuestPDF              | .NET     | Fluent builder                 | Constant pt, Relative    | materialised (buffered page)     | Header/Footer sections / known issues  | RowSpan/ColumnSpan| Issue #1124 endless loop on huge  |
| MigraDoc              | .NET     | AddRow/AddColumn on doc model  | fixed, percent-of-page   | materialised (whole doc)         | `HeadingFormat` repeats / KeepWith     | MergeRight/Down   | **Historically very slow >N pages**|
| iText7 Table          | .NET/Java| Fluent `addCell` / addHeaderCell | UnitValue: point, percent | **LargeElement flushContent**    | `setHeaderRows`/`setFooterRows` / yes  | rowspan/colspan   | Designed for 100k+ rows           |
| OpenPDF PdfPTable     | Java     | addCell builder (iText 4 fork) | relative + total width   | materialised                     | setHeaderRows / setSplitLate           | rowspan/colspan   | iText4-era; no streaming          |
| PDFBox + easytable    | Java     | Table/Row/Cell builder         | fixed or weighted        | per-page drawer                  | repeating headers / yes                | row/col span      | Per-page draw; OK multi-page      |
| jsPDF-AutoTable       | JS       | `autoTable(doc, {head, body})` | auto/wrap/fixed + colStyles | materialised                  | showHead variants / auto               | rowSpan/colSpan   | Browser memory bound              |
| pdfmake               | JS       | Declarative docDefinition      | `*`, auto, fixed         | materialised                     | `headerRows` repeat / yes              | rowSpan/colSpan   | Browser memory bound              |
| PDFKit.js             | JS       | `doc.table({data})`            | `*`, fixed               | materialised                     | header repeat / rowHeights per row     | rowSpan/colSpan   | Unknown                           |
| pdf-lib               | JS       | **no built-in table**          | —                        | —                                | —                                      | —                 | Use pdf-lib-draw-table wrapper    |
| genpdf                | Rust     | `TableLayout::new(weights)`    | integer weights          | materialised                     | unknown                                | **none**          | Unknown                           |
| printpdf / pdf-writer | Rust     | **no table API — primitives**  | —                        | —                                | —                                      | —                 | n/a                               |
| typst `#table`        | Rust lib | Variadic positional + table.cell | auto, 1fr, abs, relative | materialised (doc model)         | `header(repeat: true)` / yes           | colspan, rowspan  | Unknown                           |
| oxidize-pdf           | Rust     | **no generation table API** (extract only) | —              | —                                | —                                      | —                 | n/a                               |
| gofpdf                | Go       | **no dedicated table** — Cell() loop | manual           | streaming by construction        | manual                                 | manual            | n/a                               |
| maroto (v2)           | Go       | 12-col Bootstrap grid          | 1..12 col-span units     | auto-pages when overflow         | header can be set on every new page    | via col sizing    | Unknown                           |
| unipdf creator.Table  | Go       | NewTable(n), NewCell push      | fractional 0..1 per col  | materialised until `Draw`        | SetHeaderRows repeats / yes            | CellColspan/Rowspan | Production-oriented             |

---

## Per-library notes

### ReportLab Platypus `Table` / `LongTable` (Python)

```python
from reportlab.platypus import Table, TableStyle
from reportlab.lib import colors

data = [['00','01','02','03','04'],
        ['10','11','12','13','14'],
        ['20','21','22','23','24'],
        ['30','31','32','33','34']]
t = Table(data)
t.setStyle(TableStyle([
    ('BACKGROUND', (1,1), (-2,-2), colors.green),
    ('TEXTCOLOR',  (0,0), (1,-1),  colors.red),
    ('GRID',       (0,0), (-1,-1), 0.5, colors.black),
    ('SPAN',       (0,0), (1,1)),
]))
```

Entry shape: declarative 2-D list-of-lists + separate `TableStyle` command
tuples addressed by (col,row) coords (negative indices allowed). Widths:
`colWidths`/`rowHeights` as absolute pt or `None` for auto. Streaming:
materialised — all rows passed to constructor. Pagination: `repeatRows=N`
repeats first N rows as header on page break; `splitByRow=1` and `splitLate`
control splitting; rows normally split across page boundary if they don't fit.
Colspan/rowspan: `('SPAN', (sc,sr), (ec,er))` — other cells must be empty.
Styling: rich TableStyle commands — BACKGROUND, TEXTCOLOR, GRID/BOX/LINEABOVE,
ALIGN, VALIGN, FONTNAME, LEFTPADDING etc. Scale: `Table` is documented as
slow on long inputs; `LongTable` uses a greedy column-width algorithm
"intended for long tables where speed counts." Docs:
https://docs.reportlab.com/reportlab/userguide/ch7_tables/

### borb `FlexibleColumnWidthTable` / `FixedColumnWidthTable` (Python)

```python
FixedColumnWidthTable(number_of_rows=2, number_of_columns=2,
                      column_widths=[Decimal(0.3), Decimal(0.7)]
).add(TableCell(Image(...), row_span=2))
```

Entry shape: imperative `.add()` per cell; dimensions declared up-front.
Widths: `column_widths` as `Decimal` fractions; `FlexibleColumnWidthTable`
adapts to content. Streaming: materialised. Pagination: multi-page support
added later (tracked in borb issue #7). Colspan/rowspan: wrap in
`TableCell(..., col_span=N, row_span=M)`. Styling: per-cell background
colour, borders, padding; layout via `PageLayout`. Scale: not documented.
Docs: borb-examples README, https://github.com/jorisschellekens/borb-examples

### fpdf2 `pdf.table()` (Python)

```python
with pdf.table() as table:
    for data_row in TABLE_DATA:
        row = table.row()
        for datum in data_row:
            row.cell(datum)
```

Entry shape: **context-manager streaming builder** — cells added as they
are emitted. Widths: `col_widths` fixed pt *or* fractional weights
(`(1,1,2)` → 25/25/50). Streaming: yes, rows emitted inside `with` block.
Pagination: `repeat_headings=1` by default, controlled via
`TableHeadingsDisplay`. Colspan/rowspan: `.cell(colspan=, rowspan=)` or
`TableSpan.COL` / `TableSpan.ROW` placeholders. Styling: CSS-like padding
(1-4 values), alignment, borders_layout, per-cell `FontFace`. Scale: no
documented limit. Docs: https://py-pdf.github.io/fpdf2/Tables.html

### xhtml2pdf / WeasyPrint (Python, HTML-to-PDF)

No programmatic API — consumer writes `<table>` in HTML/CSS. WeasyPrint
treats `<thead>`/`<tfoot>` as `TableRowGroupBox` and repeats them on page
breaks (even with `border-collapse: collapse` per issue #76). Row/colspan
inherited from HTML. Page-break-* inside tables is documented as ignored
(issue #209). Scale: multiple open issues on large-table perf (#333, #413,
#905). Docs: https://doc.courtbouillon.org/weasyprint/stable/

### QuestPDF `Table` (.NET)

```csharp
.Table(table => {
    table.ColumnsDefinition(columns => {
        columns.ConstantColumn(50);
        columns.RelativeColumn();
    });
    table.Header(header => { header.Cell().Text("Column 1"); });
    table.Cell().Text("Data");
});
```

Entry shape: **fluent builder** with explicit `ColumnsDefinition`, `Header`,
and `Cell`/`Row`/`Column` placement. Widths: `ConstantColumn(pt)` and
`RelativeColumn(weight)`. Streaming: materialised; rows buffered until
draw. Pagination: `table.Header(...)`/`table.Footer(...)` repeat on page
breaks; `ExtendLastCellsToTableBottom()` helper. Colspan/rowspan: `.RowSpan(n)`,
`.ColumnSpan(n)` on cells; manual `.Row()`/`.Column()` positioning. Styling:
`.Border()`, `.Background()`, `.Padding()`, `.AlignRight()`,
`.DefaultTextStyle()`. Scale: issue #1124 ("Large tables cause endless
generation") and #591 (row-duplication on page-break wrap) show edges;
otherwise well-suited to multi-page reports. Docs:
https://www.questpdf.com/api-reference/table/basics.html

### MigraDoc `Table` (.NET)

Entry shape: `doc.LastSection.AddTable()` → `AddColumn(width)` →
`AddRow()` → `row.Cells[i].AddParagraph(...)`. Widths: absolute Unit,
`KeepWith`, `HeadingFormat`. Streaming: none — entire MigraDoc document
is materialised before rendering. Pagination: `row.HeadingFormat = true`
repeats header rows; `KeepWith` keeps rows together. Colspan/rowspan:
`cell.MergeRight`/`MergeDown`. Styling: `Format`, `Shading`, `Borders`.
**Scale ceiling**: canonical worst case — 4000-page reports historically
take >1h to render, partly fixed in MigraDoc 1.5.x (PDFsharp forum t=679,
ststeiger/PdfSharpCore #150); still the reference "bad for large tables"
library. Docs: MigraDoc wiki / forum.pdfsharp.net.

### iText7 `Table` (.NET & Java) — LargeElement mode

```java
Table table = new Table(UnitValue.createPercentArray(new float[]{3,3,3}))
        .useAllAvailableWidth();
table.addHeaderCell("H1"); table.addHeaderCell("H2"); table.addHeaderCell("H3");
for (Row r : bigDataset) {
    table.addCell(r.a); table.addCell(r.b); table.addCell(r.c);
    table.flush();                      // LargeElement.flushContent
}
table.complete();                       // isComplete() -> true
document.add(table);
```

Entry shape: fluent `addCell` / `addHeaderCell` / `addFooterCell`. Widths:
`UnitValue.createPointArray`, `createPercentArray`, `useAllAvailableWidth`.
Streaming: **`LargeElement` interface** — `flushContent`, `isComplete`,
`setComplete` — the only mainstream API that explicitly supports
add-some-rows, flush, GC, add more. Note: large tables "do not support
auto layout" and the width cannot be removed. Pagination:
`setHeaderRows(n)`, `setFooterRows(n)`, `setSkipFirstHeader`,
`setSkipLastFooter`. Colspan/rowspan: `new Cell(rowspan, colspan)`.
Styling: rich `setBackgroundColor`, `setBorder`, `setPadding`, `setTextAlignment`.
Scale: designed for 100k+ row reports. Docs:
https://kb.itextpdf.com/itext/large-tables and
https://api.itextpdf.com/iText/java/7.2.2/com/itextpdf/layout/element/Table.html

### OpenPDF `PdfPTable` (Java) — iText 4 fork

Entry shape: `PdfPTable table = new PdfPTable(3); table.addCell(...)`.
Widths: `setWidths(float[])` relative, `setTotalWidth`, `setWidthPercentage`.
Streaming: none (iText 4 pre-LargeElement). Pagination: `setHeaderRows(n)`,
`setSkipFirstHeader(true)`, `setSplitLate`, `setSplitRows`. Colspan/rowspan:
`PdfPCell.setColspan`, `setRowspan`. Styling: cell-level borders, padding,
alignment, backgrounds. Scale: inherits iText 4 limits — OK to low tens of
thousands of rows; no streaming. Docs: https://github.com/LibrePDF/OpenPDF

### Apache PDFBox + easytable (Java)

```java
Table table = Table.builder()
        .addColumnsOfWidth(100, 100, 100)
        .addRow(Row.builder().add(TextCell.builder().text("A").build())
                             .add(TextCell.builder().text("B").build())
                             .add(TextCell.builder().text("C").build()).build())
        .build();
TableDrawer.builder().table(table).startX(20f).startY(750f).build().draw();
```

PDFBox itself has no table API; easytable
(https://github.com/vandeseer/easytable) is the dominant helper (also
Boxable and pdfbox-layout). Entry: nested builders. Widths: fixed pt,
weighted columns. Streaming: `TableDrawer` draws per page; you can chain
across pages but the full `Table` is still materialised. Pagination:
repeating headers via `TableDrawer` over multiple pages. Colspan/rowspan:
supported. Styling: per-cell fonts, padding, borders, backgrounds,
alignment. Scale: unknown. Docs:
https://github.com/vandeseer/easytable

### jsPDF-AutoTable (JS)

```javascript
import jsPDF from 'jspdf'
import autoTable from 'jspdf-autotable'
const doc = new jsPDF()
autoTable(doc, {
  head: [['Name', 'Email']],
  body: [['David', 'david@example.com']]
})
doc.save('table.pdf')
```

Entry shape: **declarative options** object (`head`, `body`, `foot`, plus
columnStyles/headStyles/bodyStyles). Widths: `cellWidth: 'auto' | 'wrap' |
<number>`, plus `columnStyles[key].cellWidth`. Streaming: materialised;
one `autoTable` call per table. Pagination: `showHead: 'everyPage' |
'firstPage' | 'never'`, `startY`, `didDrawPage` hook for page footers.
Colspan/rowspan: `rowSpan`, `colSpan` on cell objects. Styling: theme
(`striped` / `grid` / `plain`) + headStyles/bodyStyles/alternateRowStyles.
Scale: browser memory bound; no specific doc limit. Docs:
https://github.com/simonbengtsson/jsPDF-AutoTable

### pdfmake (JS)

```javascript
var docDefinition = { content: [{ table: {
    headerRows: 1,
    widths: [ '*', 'auto', 100, '*' ],
    body: [
      [ 'First', 'Second', 'Third', 'The last one' ],
      [ 'Value 1', 'Value 2', 'Value 3', 'Value 4' ],
      [ { text: 'Bold value', bold: true }, 'Val 2', 'Val 3', 'Val 4' ]
]}}]};
```

Entry shape: **declarative JSON docDefinition**. Widths: `'*'` (star),
`'auto'`, absolute number. Streaming: materialised. Pagination: `headerRows`
repeats on every page, `dontBreakRows` opt-in. Colspan/rowspan: `rowSpan`
+ `colSpan` on cell objects; empty placeholders must fill spanned slots.
Styling: built-in `noBorders`, `headerLineOnly`, `lightHorizontalLines`
layouts, plus custom layout functions controlling per-cell border/width/fill.
Scale: unknown. Docs:
https://pdfmake.github.io/docs/0.1/document-definition-object/tables/

### PDFKit.js `doc.table()` (JS)

```javascript
doc.table({
    data: [
        ['Column 1','Column 2','Column 3'],
        ['Value 1',  'Value 2','Value 3']
    ]
})
```

Entry shape: config object or method chaining. Widths: `'*'` or fixed pt.
Row heights: scalar, array, or `(i)=>h`. Streaming: materialised.
Pagination: tables flow with text; automatic page continuation; header
repeat supported. Colspan/rowspan: `rowSpan`/`colSpan` HTML-like. Styling:
border configs, backgrounds, text styles, zebra via row-based conditional
styles; style precedence default → column → row → cell. Scale: unknown.
Docs: https://pdfkit.org/docs/table.html

### pdf-lib (JS)

No native table API — `pdf-lib` exposes only draw primitives (`drawText`,
`drawRectangle`). The community workaround is
[pdf-lib-draw-table](https://github.com/MP70/pdf-lib-draw-table), a thin
`drawTable(pdfDoc, page, data, {startX, startY})` helper. Not
production-grade, no streaming, no pagination. Upstream feature request:
Hopding/pdf-lib#382. Docs: https://pdf-lib.js.org/

### genpdf `TableLayout` (Rust)

```rust
let table = elements::TableLayout::new(vec![1, 1])
    .row()
    .element(elements::Paragraph::new("Cell 1"))
    .element(elements::Paragraph::new("Cell 2"))
    .push()
    .expect("Invalid table row");
```

Entry shape: fluent row builder on a pre-declared column-weight vector.
Widths: integer weights only. Streaming: rows added one at a time via
`row().push()` or `push_row(Vec<Box<dyn Element>>)`; but whole doc still
materialised before render. Pagination: undocumented. Colspan/rowspan:
**not supported**. Styling: `set_cell_decorator(FrameCellDecorator::new(...))`
for borders; `.styled()`, `.padded()`, `.framed()` via Element trait.
Scale: unknown. Docs:
https://docs.rs/genpdf/latest/genpdf/elements/struct.TableLayout.html

### printpdf / pdf-writer (Rust)

No dedicated table API — primitives only (line/rect/text).
`pdf-writer` explicitly states "this crate is rather low-level".
Table rendering would have to be built in userland. Relevant as a
baseline for pdf_oxide: if pdf_oxide is to compete it must layer a
real table model on top of a primitive writer.

### typst `#table` (Rust library, used via typst-library crate)

```typst
#table(
  columns: 2,
  [Hello], [World],
)
```

Entry shape: **variadic positional** — `table(columns: n, ...cells)` —
with companion elements `table.cell`, `table.hline`, `table.vline`,
`table.header`, `table.footer`. Widths: `auto`, `1fr`, `relative`,
absolute. Streaming: materialised in doc model. Pagination:
`table.header(repeat: true)` (default `true`) repeats headers across
pages. Colspan/rowspan: `table.cell(colspan: 2, rowspan: 3)`. Styling:
`stroke` (default `1pt + black`), `fill` (value / array / function),
`inset` (default `0% + 5pt`), `align`. Scale: undocumented, but
typst-library lives in the same Rust compilation unit as the typst
compiler so perf is acceptable for arbitrary inputs. Docs:
https://typst.app/docs/reference/model/table/

### oxidize-pdf (Rust, unrelated project despite name)

Element *extraction* library: recognises Table as one element type in
its partitioning pipeline. **No generation table API.** Not a model
for pdf_oxide's generation side. Docs:
https://docs.rs/crate/oxidize-pdf/latest

### gofpdf (Go)

No dedicated table type. Tables are built with `pdf.Cell(w, h, txt)` /
`pdf.MultiCell(...)` loops; test file `fpdf_test.go` has the canonical
"tuto7" multi-cell table pattern. Streaming by construction (each cell
emits output). No header repeat, no spans, no auto widths — all manual.
Docs: https://github.com/jung-kurt/gofpdf

### maroto v2 (Go)

```go
m := maroto.New()
m.AddRow(10,
    col.New(4).Add(text.New("A")),
    col.New(4).Add(text.New("B")),
    col.New(4).Add(text.New("C")),
)
```

Not strictly a table — a **12-column Bootstrap-style grid**. Widths:
`col.New(1..12)` integer units. Streaming: rows consumed incrementally,
pages auto-generated on overflow. Pagination: per-page header via
`RegisterHeader`. Colspan: via `col.New(span)` sizing. Rowspan: no.
Scale: no doc limit. Docs: https://github.com/johnfercher/maroto

### unipdf `creator.Table` (Go)

```go
c := creator.New()
table := c.NewTable(3)
table.SetColumnWidths(0.25, 0.5, 0.25)
for _, h := range []string{"A","B","C"} {
    cell := table.NewCell()
    p := c.NewStyledParagraph(); p.Append(h); cell.SetContent(p)
    cell.SetBorder(creator.CellBorderSideAll, creator.CellBorderStyleSingle, 1)
}
c.Draw(table)
c.WriteToFile("table.pdf")
```

Entry shape: `NewTable(n)` + `NewCell()` push. Widths: fractions 0..1
summing to 1 across columns. Streaming: materialised in-memory until
`Draw`. Pagination: `SetHeaderRows(startRow, endRow)` repeats header
rows on every page the table spans. Colspan/rowspan: `CellColspan`,
`CellRowspan` on NewCell. Styling: per-cell border, padding,
background, alignment. Scale: production-oriented; commercial license
required. Docs:
https://pkg.go.dev/github.com/unidoc/unipdf/creator and
https://docs.unidoc.io/docs/unipdf/guides/tables/headers/

---

## Ecosystem coverage notes

Rust is the thinnest — genpdf is the only idiomatic Rust crate with a table
abstraction; printpdf / pdf-writer / oxidize-pdf stop at primitives or
extraction, leaving pdf_oxide an open lane. Go is similarly thin outside
unipdf (commercial). JS/Py ecosystems offer the widest pattern menu; .NET
QuestPDF and Java iText7 offer the most mature treatments of the hard
problems (streaming, pagination, spans).
