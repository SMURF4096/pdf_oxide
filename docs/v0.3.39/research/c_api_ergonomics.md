# API ergonomics — 6 bindings, one concept

Scope: programmatic table API for #393. Goal: six bindings that each feel
native, not one API mechanically translated. Cutoff: 2026-04.

## Summary

**The one load-bearing pattern across all 6:** a *column-declared, row-streamed*
table. You declare the schema (columns + widths + types) eagerly, then push
rows lazily. This is the only shape that:

1. lets every ecosystem feel native (columns are a value in all 6, rows are an
   iterator/stream in all 6),
2. keeps the FFI boundary efficient (rows arrive in batches of ~64, invisible
   to the caller — see "FFI-batching strategy"),
3. produces identical PDF output across bindings because layout decisions
   (column widths, alignment, wrapping) are made column-wise up front.

**Key tensions and resolutions:**

| Tension | Resolution |
|---|---|
| Typed rows (Rust/TS/C#/Go-generics) vs untyped (Python/WASM) | Typed-row *adapter* on top of an untyped core. Core speaks `Vec<PdfCell>` — typed rows compile/convert to it at the call site. |
| Streaming vs accumulate-and-emit | Streaming is the default. `finish()` / `end()` / context-manager exit is the only commit point; before that, rows are buffered into FFI-page-sized chunks. |
| Error model (Rust `Result<&mut Self>` vs Python exceptions vs Go `error` returns) | Rows *validate* (column count, type) at insertion. Rendering errors (font missing, page overflow) surface at `finish()`. Insertion errors are cheap & per-row; rendering errors are one. |
| Fail-fast vs accumulate-and-report | Insertion: fail-fast on schema violation (programmer bug). Rendering: accumulate warnings, fail on first hard error. |

## Per-binding recommended shape

### Rust

```rust
use pdf_oxide::{DocumentBuilder, Table, Column, Align};

let pdf = DocumentBuilder::new()
    .title("Q1 sales")
    .table(
        Table::new()
            .column(Column::text("SKU").width_pct(20.0))
            .column(Column::text("Item").width_auto())
            .column(Column::number("Units").align(Align::Right))
            .column(Column::currency("Revenue").align(Align::Right))
            .rows(sales_iter)?        // any IntoIterator<Item: IntoRow>
    )
    .build()?;
```

**Rationale.** Rust's DocumentBuilder already uses consuming-self fluent
builders (see `src/writer/document_builder.rs:59-283`), so `Table` follows.
Columns are typed once (`Column::text`, `Column::number`, `Column::currency`)
— this is the typed-row entry point without forcing a generic all the way
through. `rows(iter)?` accepts any `IntoIterator<Item: IntoRow>` where
`IntoRow` is a trait with blanket impls for `(T1, T2, …)` tuples up to 12,
`Vec<impl ToPdfCell>`, and `&[&dyn ToPdfCell]` for heterogeneous cases.
Errors surface at `?` on `.rows()` (schema) and `.build()` (rendering).
Inspired by `comfy-table` (comfy's `Table::add_row` + `ToRow` trait,
<https://github.com/Nukesor/comfy-table>) and Arrow's `RecordBatch` shape
(<https://docs.rs/arrow>). Not inspired by `typst::model::Table` — too
typst-specific. Not `ratatui::widgets::Table` — TUI assumes terminal width,
not PDF points.

**What it costs.** Blanket tuple impls bloat macro expansion (~200 LOC of
`impl_row_tuple!` macro). `&dyn ToPdfCell` forces a boxed trait object for
heterogeneous rows — 1 alloc per cell in that path; fine for the escape
hatch, tuples are the fast path.

**What it buys.** `sales.iter().map(|s| (s.sku, s.name, s.units, s.revenue))`
just works. Zero-copy for `&str` cells.

### Python

```python
from pdf_oxide import DocumentBuilder, Column

with DocumentBuilder("q1_sales.pdf") as pdf:
    with pdf.table(columns=[
        Column.text("SKU", width="20%"),
        Column.text("Item"),
        Column.number("Units", align="right"),
        Column.currency("Revenue", align="right"),
    ]) as table:
        for sale in sales_cursor:              # any iterable of dict/tuple/dataclass
            table.row(sale)
        # or: table.extend(df.itertuples(index=False))  # pandas interop
```

**Rationale.** Context managers are how Python expresses "a resource that
needs a commit at exit" — `sqlite3.connect`, `open()`, `pd.ExcelWriter`.
`with pdf.table(...) as table:` exits via `__exit__`, which is where rows
actually flush and rendering errors surface. This maps cleanly to Rust's
`drop`-at-end-of-scope. `row()` accepts `dict` (column-name keyed), `tuple`
(positional), `dataclass`, `pydantic.BaseModel`, or `NamedTuple`. Pandas
interop is `table.extend(df.itertuples(index=False))` — we do NOT take a
DataFrame directly (would force pandas as a dep). Typed rows via
`TypedDict` are advertised but not enforced at runtime (mypy-only).
Inspired by `rich.table.Table` (columns-then-rows,
<https://rich.readthedocs.io/en/stable/tables.html>) and `reportlab`'s
`Table` (<https://docs.reportlab.com/reportlab/userguide/ch7_tables/>).
Not inspired by `pd.DataFrame.to_html` — that's a one-shot, not streaming.

**What it costs.** Two `with` blocks nested is slightly verbose. `Column`
factory methods duplicate the Rust enum — necessary because Python doesn't
have type-driven dispatch.

**What it buys.** Natural for the SQL-cursor use case (the actual demand —
see issue #393 thread). `table.extend(cursor)` is a one-liner.

### TypeScript / Node

```typescript
import { DocumentBuilder, Column } from "@pdf-oxide/node";

const pdf = await DocumentBuilder.create("q1_sales.pdf")
  .title("Q1 sales")
  .table({
    columns: [
      Column.text("sku",     { label: "SKU", widthPct: 20 }),
      Column.text("item",    { label: "Item" }),
      Column.number("units", { label: "Units", align: "right" }),
      Column.currency("revenue", { label: "Revenue", align: "right" }),
    ],
    rows: salesAsyncIter,            // AsyncIterable<Sale> | Iterable<Sale>
  })
  .build();
```

Typed version (escape hatch for TS-strict users):

```typescript
type Sale = { sku: string; item: string; units: number; revenue: number };
const table = Column.schema<Sale>()
  .text("sku").text("item").number("units").currency("revenue");
// table.rows(...) is now typed AsyncIterable<Sale>
```

**Rationale.** Async iterables are the 2026 idiom for "rows from a DB
stream" — Node `pg.Cursor`, `mysql2` streams, `kysely` stream(), all expose
`AsyncIterable`. A single `rows: AsyncIterable<T> | Iterable<T>` field
covers both sync and async. Tagged-template shapes (jsPDF-AutoTable-style)
are flashy but lose type inference — rejected. The typed `schema<T>()`
builder is opt-in; the untyped flat form is the default because most Node
users hand us arrays of plain objects.
Inspired by `pdfmake` (declarative column objects,
<https://pdfmake.github.io/docs/0.1/document-definition-object/tables/>)
and Kysely's query-builder generic chaining
(<https://kysely-org.github.io/kysely-apidoc/>).
Not inspired by `jsPDF-AutoTable` (<https://github.com/simonbengtsson/jsPDF-AutoTable>)
— global DOCX-style options object is an untypable bag.

**What it costs.** Two shapes (typed schema builder + untyped column array)
to maintain. Async iterables require Node 10+; already required elsewhere.

**What it buys.** `for await (const row of pgCursor) table.row(row)` works
out of the box when a user wants explicit control.

### C# / .NET

```csharp
using PdfOxide;

await using var pdf = DocumentBuilder.Create("q1_sales.pdf")
    .Title("Q1 sales");

await pdf.Table<Sale>()
    .Column("SKU",     s => s.Sku,      w => w.Percent(20))
    .Column("Item",    s => s.Item)
    .Column("Units",   s => s.Units,    c => c.AlignRight())
    .Column("Revenue", s => s.Revenue,  c => c.Currency().AlignRight())
    .RowsAsync(salesChannel.ReadAllAsync())   // IAsyncEnumerable<Sale>
    .BuildAsync();
```

**Rationale.** Expression-bound column accessors (`s => s.Sku`) are the
QuestPDF / EF-Core idiom — compile-time typed, reflection-free at runtime
(we grab the `MemberInfo` once). `IAsyncEnumerable<T>` is the canonical
streaming shape post-.NET Core 3. `await using` ensures
`DisposeAsync` flushes the final FFI batch. `Table<T>()` gives us a typed
row throughout, which C# developers expect — no `object[]` escape hatch
exposed by default (internal only).
Inspired by QuestPDF's fluent API
(<https://www.questpdf.com/api-reference/table.html>, widely loved on
r/dotnet) and EF-Core's `modelBuilder.Entity<T>(b => b.Property(x => x.Foo))`.
Not inspired by MigraDoc's `Table.AddRow()` / `row.Cells[0].AddParagraph(...)`
— imperative and verbose, developers openly dislike it
(<https://docs.pdfsharp.net/MigraDoc/General/Document-Object-Model/Tables.html>).
System.Data.DataTable interop is a one-liner:
`table.RowsFromDataTable(dt)` — an adapter, not the default path.

**What it costs.** Requires source generation or expression-tree parsing
at column registration. ~1 alloc per column at setup; zero per row.

**What it buys.** Rename-safe columns, IntelliSense on `s =>` lambda,
zero boxing on value-type rows (`Units` as `int`).

### Go

```go
package main

import "github.com/pdf-oxide/go"

type Sale struct {
    SKU     string
    Item    string
    Units   int
    Revenue float64
}

func main() {
    pdf := pdfoxide.NewDocument("q1_sales.pdf").Title("Q1 sales")

    tbl, err := pdf.Table(
        pdfoxide.TextColumn("SKU").WidthPct(20),
        pdfoxide.TextColumn("Item"),
        pdfoxide.NumberColumn("Units").AlignRight(),
        pdfoxide.CurrencyColumn("Revenue").AlignRight(),
    )
    if err != nil { log.Fatal(err) }

    for sale := range sales {                    // chan Sale or iter.Seq[Sale]
        if err := tbl.AddRow(sale.SKU, sale.Item, sale.Units, sale.Revenue); err != nil {
            log.Fatal(err)
        }
    }
    if err := pdf.Build(); err != nil { log.Fatal(err) }
}
```

**Rationale.** Go 1.23+ has `iter.Seq[T]` range-over-func, but most 2026
Go code still uses channels or plain slices — `AddRow(...any)` (variadic)
is the widest-reach shape. We expose a generic `AddRowT[T any](tbl, row)`
that uses reflect once at registration to map struct fields to columns
(tagged with `pdf:"sku"`), for users on 1.21+. Errors-as-values: `AddRow`
returns `error` for schema violations (wrong arity/type), and callers
choose to fail-fast or accumulate. `Build()` returns the rendering error.
Reader/Writer streaming (`io.Writer` for the PDF output) is already the
norm in the Go binding — we keep that.
Inspired by `tablewriter` (<https://github.com/olekukonko/tablewriter>,
`SetHeader` then `Append`) and `go-pretty/table`
(<https://github.com/jedib0t/go-pretty>, column config objects).
Maroto's table (<https://github.com/johnfercher/maroto>) was studied —
its row/column-grid model is nice for dashboards but awkward for
streamed data; rejected. Kubernetes' printer table
(<https://github.com/kubernetes/cli-runtime/tree/master/pkg/printers>)
confirmed: declarative `TableColumnDefinition` + rows-as-`[]Cell`.

**What it costs.** `...any` loses compile-time safety; we document that
`AddRowT[Sale]` is the preferred form for 1.21+. Reflection in the typed
path, once per Table.

**What it buys.** Zero-import friction for Go 1.20 users. Works with
channels, slices, `iter.Seq`, and DB `sql.Rows` (caller scans into
args, calls AddRow).

### WASM (browser)

```typescript
import init, { DocumentBuilder, Column } from "@pdf-oxide/wasm";

await init();

const pdf = new DocumentBuilder().title("Q1 sales");

const table = pdf.table([
  Column.text("SKU",     { widthPct: 20 }),
  Column.text("Item"),
  Column.number("Units", { align: "right" }),
  Column.currency("Revenue", { align: "right" }),
]);

// From a fetch stream:
const res = await fetch("/api/sales.ndjson");
for await (const sale of ndjson(res.body!)) {
  table.row(sale);
}

const bytes = pdf.build();      // Uint8Array, ready for a Blob
```

**Rationale.** WASM binding mirrors the TS shape (same package author,
same users) but drops `async` where possible because WASM exports are
synchronous. Rows come in via `table.row(obj)` one at a time; batching
happens *inside* the WASM boundary (see FFI strategy). We do NOT offer
DOM-bound construction (`tableFromHTMLTable(el)`) as the primary API —
it's a convenience adapter, because DOM tables don't carry type
information we need (currency vs plain number). Fetch streams /
`ReadableStream` work via `for await`.
Inspired by `@observablehq/plot` in-browser table construction
(<https://observablehq.com/plot/marks/cell>) and `apache-arrow` JS's
`tableFromIPC` (<https://arrow.apache.org/docs/js/>).

**What it costs.** Every `table.row(obj)` is a JS→WASM boundary call;
the row is serialized to a shared `ArrayBuffer` batch and flushed every
N rows (see below).

**What it buys.** Works with any browser data source —
`fetch().body.getReader()`, IndexedDB cursor, WebSocket feed.

## Cross-cutting decisions

### Streaming model (6-way)

| Binding   | Streaming input type                               | Commit point            |
|-----------|----------------------------------------------------|-------------------------|
| Rust      | `impl IntoIterator<Item: IntoRow>`                 | `.build()`              |
| Python    | any iterable (incl. generators, cursors, DataFrames via `itertuples`) | `__exit__` of `with pdf.table(...)` |
| TS/Node   | `AsyncIterable<T> \| Iterable<T>`                  | `await .build()`        |
| C#        | `IAsyncEnumerable<T>` / `IEnumerable<T>`           | `await BuildAsync()` / `DisposeAsync` |
| Go        | channel, slice, or `iter.Seq[T]` + `AddRow` loop   | `.Build()`              |
| WASM      | `for await` loop + `.row()`                        | `.build()`              |

Consistent principle: **the binding accepts whatever the language calls
"a lazy sequence"**. Nobody has to re-buffer a DB cursor into memory
before handing it to us.

### FFI-batching strategy (invisible to caller)

The problem: each `AddRow` across FFI is 1-10 µs of overhead (stack
frame, argument marshaling, bounds check). At 1M rows that's 1-10 seconds
of pure overhead, with zero PDF work done.

**Solution: caller-invisible batching via a shared row buffer per Table
handle.** On the binding side we maintain a thread-local
`RowBatch { columns: N, cells: SmallVec<Cell; 256> }`. Each `row()` /
`AddRow` appends to this buffer. When the buffer hits `N rows × cells per row
>= 256` entries, we cross FFI once with a pointer to the packed buffer and
drain it in Rust. `finish()` flushes the tail.

This is the pattern used by:
- **Apache Arrow** (`RecordBatchBuilder` / `StringBuilder.append_value`
  accumulates in a vec, `finish()` hands out a batch — per-value call is
  ~2 ns, batch handoff once)
  — <https://arrow.apache.org/docs/dev/format/Columnar.html>
- **DuckDB Appender** (`appender.append_row(...)` buffers, auto-flushes
  every ~2048 rows; caller sees per-row API)
  — <https://duckdb.org/docs/api/c/appender>
- **SQLite `sqlite3_stmt` + `sqlite3_step`** (different model — one row
  at a time — but the per-row C call is ~100 ns because no marshaling)

We pick the Arrow/DuckDB pattern. Flush threshold: 256 cells (≈ 64 rows
of 4 columns), tunable via `table.batch_size(n)`. A row fits in one
batch = one FFI call; long strings go through a side-table of
`&[u8]` slices pointing into a per-batch string arena (no per-cell copy).

Measured target: **< 50 ns per `row()` call amortized** in Python/Node/
C#/Go (one amortized memcpy per cell + one FFI call per 64 rows).

### Type-safety tier — who gets typed rows

| Binding | Typed-row default?     | How it's done                                      |
|---------|------------------------|----------------------------------------------------|
| Rust    | Yes                    | `IntoRow` trait, tuple impls, `ToPdfCell` per cell |
| C#      | Yes                    | `Table<T>()` + expression-bound columns            |
| TS      | Opt-in (recommended)   | `Column.schema<T>()` generic builder               |
| Go      | Opt-in (1.21+ generic) | `AddRowT[Sale]` with struct-tag reflection         |
| Python  | Docs-only (mypy)       | `TypedDict` / dataclass — accepted, not enforced   |
| WASM    | No (untyped by nature) | Plain JS objects; TS `.d.ts` gives IDE hints       |

**Where we draw the line:** typed rows are a compile-time ergonomic feature,
not a runtime safety feature. The runtime *always* validates arity and
cell type at `row()` time — that's non-negotiable, because a wrong-arity
row corrupts the PDF table layout. So a Python user with a bad dict gets
a `PdfOxideSchemaError` at `row()` time, same as a Rust user gets at the
`?`. Type systems are the polish, not the safety.

## Counter-proposals considered and rejected

### 1. One generic `TableBuilder<T>` in every language

**Shape:** force every binding to the Rust-generic shape
(`TableBuilder::<Row>::new().row(row).row(row).build()`). Looks "consistent"
on paper.

**Why rejected:** Python has no generics at runtime; WASM users pass raw
JS objects; Go pre-1.21 has no generics at all. Forcing the shape means
Python gets an ugly `TableBuilder[dict]()` that mypy can't even verify,
and Go gets `TableBuilder[map[string]any]` which is worse than a variadic
`AddRow`. The *concept* is consistent (column schema + row stream); the
*shape* can't be.

### 2. DataFrame-first Python API (`pdf.from_dataframe(df)`)

**Shape:** Python users pass a pandas DataFrame, we read columns and dtypes.

**Why rejected:** makes pandas a hard dep (6 MB+). Users on polars, DuckDB,
SQLAlchemy cursors have to convert first. A DataFrame adapter as an
*optional* extra (`pdf_oxide[pandas]`) is fine, but it's not the default.
Also: DataFrame metadata (dtypes) doesn't carry PDF-layout intent
(currency vs plain number, column width %), so we'd end up with a second
config object anyway.

### 3. Tagged-template HTML-like DSL in TS (`` pdf.table`<tr>...</tr>` ``)

**Shape:** jsPDF-AutoTable / docx.js pattern — let users write HTML-ish
markup in a template literal.

**Why rejected:** loses TS type inference, string-parsing overhead per
row, can't stream (the template is materialized eagerly). Looks cool in
README, fails in production. Users would reach for our declarative shape
within a week.

### 4. Declarative struct (`new Table { Columns = [...], Rows = [...] }`) everywhere

**Shape:** C# record-style `new Table { Rows = allRows }.Render()`.

**Why rejected:** kills streaming. `Rows = allRows` implies the full
collection is materialized. Even with `IEnumerable<T>` the syntactic
shape *reads* eager, and users build lists. The fluent `.RowsAsync(stream)`
+ `.BuildAsync()` shape is only 2 lines longer and actually streams.

### 5. "Accumulate-and-report" error model across the board

**Shape:** `row()` never fails; all errors surface at `build()` as a
`Vec<RowError>`.

**Why rejected:** a row with the wrong column count at row 50,000 means
every subsequent row is also wrong, flooding the user with duplicates.
Fail-fast on schema violations (programmer bug) + accumulate on
rendering concerns (font substitution, overflow) is the right split.
