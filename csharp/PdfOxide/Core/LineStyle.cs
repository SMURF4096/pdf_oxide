using System;
using System.Collections.Generic;

namespace PdfOxide.Core
{
    /// <summary>
    /// Horizontal alignment options for text placement primitives
    /// (<see cref="PageBuilder.TextInRect"/>) and table columns
    /// (<see cref="Column"/>).
    /// </summary>
    /// <remarks>
    /// The underlying FFI encodes these as <c>int</c>:
    /// <c>0 = Left</c>, <c>1 = Center</c>, <c>2 = Right</c>. Keep the
    /// enum discriminant values stable with <c>src/ffi.rs</c>.
    /// </remarks>
    public enum Alignment
    {
        /// <summary>Left-align the text (default).</summary>
        Left = 0,

        /// <summary>Center the text within the available width.</summary>
        Center = 1,

        /// <summary>Right-align the text.</summary>
        Right = 2,
    }

    /// <summary>
    /// Declarative column spec for buffered and streaming tables
    /// (<see cref="PageBuilder.Table(TableSpec)"/>,
    /// <see cref="PageBuilder.StreamingTable(System.Collections.Generic.IReadOnlyList{Column}, bool)"/>).
    /// </summary>
    /// <remarks>
    /// <para>
    /// Each column carries a display <see cref="Header"/> (used when the
    /// owning table has <c>HasHeader = true</c>), a <see cref="Width"/>
    /// in PDF points, and an <see cref="Align"/> specifier for both the
    /// header cell and every body cell in that column.
    /// </para>
    /// <para>
    /// Column widths are absolute — percent or fractional widths are not
    /// currently supported at the FFI layer (tracked for a later
    /// release). To mimic percentage layout, compute
    /// <c>pageWidth * pct</c> in user code.
    /// </para>
    /// </remarks>
    public sealed class Column
    {
        /// <summary>Header label shown when the table has <c>HasHeader = true</c>.</summary>
        public string Header { get; set; }

        /// <summary>Column width in PDF points.</summary>
        public float Width { get; set; }

        /// <summary>Alignment for the header cell and every body cell.</summary>
        public Alignment Align { get; set; }

        /// <summary>
        /// Construct a column.
        /// </summary>
        /// <param name="header">Header label; pass an empty string if
        /// the owning table sets <c>HasHeader = false</c>.</param>
        /// <param name="width">Column width in PDF points. Must be positive.</param>
        /// <param name="align">Horizontal alignment (default
        /// <see cref="Alignment.Left"/>).</param>
        public Column(string header, float width, Alignment align = Alignment.Left)
        {
            ArgumentNullException.ThrowIfNull(header);
            if (width <= 0f)
                throw new ArgumentOutOfRangeException(nameof(width), "width must be positive");
            Header = header;
            Width = width;
            Align = align;
        }
    }

    /// <summary>
    /// Buffered table specification — all rows are materialised up
    /// front and passed to the FFI in a single call via
    /// <see cref="PageBuilder.Table(TableSpec)"/>.
    /// </summary>
    /// <remarks>
    /// Memory is <c>O(rows * columns)</c>. For large data sets prefer
    /// <see cref="PageBuilder.StreamingTable(System.Collections.Generic.IReadOnlyList{Column}, bool)"/>,
    /// which presents a streaming <c>AddRow</c> surface (the v0.3.39
    /// implementation still buffers internally — see
    /// <see cref="StreamingTable"/>).
    /// </remarks>
    public sealed class TableSpec
    {
        /// <summary>Ordered columns. Must be non-empty at <see cref="PageBuilder.Table(TableSpec)"/>.</summary>
        public List<Column> Columns { get; set; } = new();

        /// <summary>
        /// Row-major cell matrix. Each inner collection must have
        /// exactly <c>Columns.Count</c> entries. <see langword="null"/>
        /// cells are rendered as empty strings.
        /// </summary>
        public IList<IList<string>> Rows { get; set; } = new List<IList<string>>();

        /// <summary>
        /// When <see langword="true"/>, the first rendered row is drawn
        /// as a header (bold + default header background). When
        /// <see langword="false"/>, all rows are body rows and
        /// <see cref="Column.Header"/> labels are ignored.
        /// </summary>
        public bool HasHeader { get; set; }
    }

    /// <summary>
    /// Streaming-table handle returned by
    /// <see cref="PageBuilder.StreamingTable(System.Collections.Generic.IReadOnlyList{Column}, bool)"/>.
    /// Accepts rows one at a time via <see cref="AddRow(string[])"/>
    /// and flushes them to the page on <see cref="Build"/>.
    /// </summary>
    /// <remarks>
    /// <para>
    /// <strong>v0.3.39 limitation:</strong> the managed side buffers
    /// rows internally and forwards them to the buffered FFI
    /// <c>pdf_page_builder_table</c> call at <see cref="Build"/> time.
    /// Memory usage is therefore still <c>O(rows * columns)</c>. A
    /// true streaming FFI (<c>O(columns)</c>) is tracked as issue
    /// #393 step 6.5 and will land in a subsequent release — the
    /// managed-side <see cref="AddRow(string[])"/> / <see cref="Build"/>
    /// surface is forward-compatible and will transparently become
    /// page-at-a-time once the FFI is ready.
    /// </para>
    /// <para>
    /// Always wrap in a <c>using</c>. <see cref="Dispose"/> on an
    /// un-<see cref="Build"/>t handle drops the buffered rows without
    /// drawing anything — useful for error recovery mid-stream.
    /// </para>
    /// </remarks>
    public sealed class StreamingTable : IDisposable
    {
        private readonly PageBuilder _page;
        private readonly List<Column> _columns;
        private readonly bool _repeatHeader;
        private readonly List<IList<string>> _buffered = new();
        private bool _built;
        private bool _disposed;

        internal StreamingTable(PageBuilder page, IReadOnlyList<Column> columns, bool repeatHeader)
        {
            _page = page;
            _columns = new List<Column>(columns);
            _repeatHeader = repeatHeader;
        }

        /// <summary>
        /// Append a row. The <paramref name="cells"/> array must have
        /// exactly <c>Columns.Count</c> entries; <see langword="null"/>
        /// entries render as empty strings.
        /// </summary>
        /// <remarks>
        /// In v0.3.39 this appends to an in-memory buffer; actual
        /// drawing happens at <see cref="Build"/>. Once the streaming
        /// FFI lands, this will flush rows page-at-a-time.
        /// </remarks>
        public StreamingTable AddRow(params string[] cells)
        {
            ObjectDisposedException.ThrowIf(_disposed, this);
            if (_built)
                throw new InvalidOperationException("StreamingTable already built; construct a new one for additional rows.");
            ArgumentNullException.ThrowIfNull(cells);
            if (cells.Length != _columns.Count)
                throw new ArgumentException(
                    $"row width mismatch: got {cells.Length} cells, expected {_columns.Count}",
                    nameof(cells));
            var row = new string[cells.Length];
            for (int i = 0; i < cells.Length; i++)
                row[i] = cells[i] ?? string.Empty;
            _buffered.Add(row);
            return this;
        }

        /// <summary>
        /// Flush every buffered row to the underlying
        /// <see cref="PageBuilder"/> and return the page for continued
        /// chaining. Calling <see cref="AddRow(string[])"/> after
        /// <see cref="Build"/> throws.
        /// </summary>
        /// <remarks>
        /// The <c>repeatHeader</c> flag from the factory call is
        /// currently advisory — the v0.3.39 FFI always repeats the
        /// header on page breaks when <c>HasHeader = true</c>. The
        /// field is kept on the managed API so callers don't need to
        /// rewrite code once a separate "header once only" mode is
        /// added to the FFI.
        /// </remarks>
        public PageBuilder Build()
        {
            ObjectDisposedException.ThrowIf(_disposed, this);
            if (_built)
                throw new InvalidOperationException("StreamingTable already built.");
            _built = true;

            var spec = new TableSpec
            {
                Columns = new List<Column>(_columns),
                Rows = new List<IList<string>>(_buffered.Count),
                HasHeader = _repeatHeader,
            };
            foreach (var row in _buffered)
                spec.Rows.Add(row);
            _page.Table(spec);
            return _page;
        }

        /// <summary>
        /// Drop the handle. If <see cref="Build"/> has not been called
        /// the buffered rows are discarded silently — use this path
        /// for error recovery when the caller decides mid-stream not
        /// to emit the table.
        /// </summary>
        public void Dispose()
        {
            if (_disposed) return;
            _disposed = true;
            _buffered.Clear();
        }
    }
}
