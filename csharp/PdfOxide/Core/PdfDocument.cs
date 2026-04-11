using System;
using System.IO;
using System.Threading;
using System.Threading.Tasks;
using PdfOxide.Exceptions;
using PdfOxide.Internal;

namespace PdfOxide.Core
{
    /// <summary>
    /// Represents a PDF document for reading and text extraction.
    /// Provides read-only access with automatic reading order detection.
    /// </summary>
    /// <remarks>
    /// <para>
    /// PdfDocument is the primary API for opening and reading existing PDF files.
    /// It supports:
    /// <list type="bullet">
    /// <item><description>Opening PDF files from disk or memory</description></item>
    /// <item><description>Extracting text with automatic reading order detection</description></item>
    /// <item><description>Converting pages to various formats (Markdown, HTML, PlainText)</description></item>
    /// <item><description>Accessing PDF metadata and structure information</description></item>
    /// </list>
    /// </para>
    /// <para>
    /// The document must be explicitly disposed to release native resources.
    /// Use 'using' statements for automatic cleanup.
    /// </para>
    /// </remarks>
    /// <example>
    /// <code>
    /// using (var doc = PdfDocument.Open("document.pdf"))
    /// {
    ///     // Get PDF version and page count
    ///     var version = doc.Version;
    ///     var pageCount = doc.PageCount;
    ///     Console.WriteLine($"PDF {version.Major}.{version.Minor}, {pageCount} pages");
    ///
    ///     // Extract text from first page
    ///     var text = doc.ExtractText(0);
    ///     Console.WriteLine(text);
    ///
    ///     // Convert to Markdown
    ///     var markdown = doc.ToMarkdown(0);
    ///     File.WriteAllText("output.md", markdown);
    /// }
    /// </code>
    /// </example>
    public sealed class PdfDocument : IDisposable
    {
        private NativeHandle _handle;
        private volatile bool _disposed;
        private readonly ReaderWriterLockSlim _lock = new ReaderWriterLockSlim();

        private PdfDocument(NativeHandle handle)
        {
            _handle = handle ?? throw new ArgumentNullException(nameof(handle));
        }

        /// <summary>
        /// Gets the native handle pointer for internal use by managers.
        /// Thread-safe: acquires a read lock.
        /// </summary>
        internal IntPtr Handle
        {
            get
            {
                _lock.EnterReadLock();
                try
                {
                    ThrowIfDisposed();
                    return _handle.Ptr;
                }
                finally
                {
                    _lock.ExitReadLock();
                }
            }
        }

        /// <summary>
        /// Gets the native handle for internal use by managers.
        /// Thread-safe: acquires a read lock.
        /// </summary>
        internal NativeHandle NativeHandle
        {
            get
            {
                _lock.EnterReadLock();
                try
                {
                    ThrowIfDisposed();
                    return _handle;
                }
                finally
                {
                    _lock.ExitReadLock();
                }
            }
        }

        /// <summary>
        /// Opens a PDF document from a file path.
        /// </summary>
        /// <param name="path">The file path to the PDF.</param>
        /// <returns>An opened PdfDocument.</returns>
        /// <exception cref="ArgumentNullException">Thrown if <paramref name="path"/> is null.</exception>
        /// <exception cref="PdfIoException">Thrown if the file cannot be opened.</exception>
        /// <exception cref="PdfParseException">Thrown if the PDF is invalid.</exception>
        public static PdfDocument Open(string path)
        {
            if (path == null)
                throw new ArgumentNullException(nameof(path));

            var handle = NativeMethods.PdfDocumentOpen(path, out var errorCode);
            if (handle.IsInvalid)
            {
                ExceptionMapper.ThrowIfError(errorCode);
            }

            return new PdfDocument(handle);
        }

        /// <summary>
        /// Opens a PDF document from a stream.
        /// </summary>
        /// <param name="stream">The stream containing PDF data.</param>
        /// <returns>An opened PdfDocument.</returns>
        /// <exception cref="ArgumentNullException">Thrown if <paramref name="stream"/> is null.</exception>
        /// <exception cref="PdfIoException">Thrown if the stream cannot be read.</exception>
        /// <exception cref="PdfParseException">Thrown if the PDF is invalid.</exception>
        public static PdfDocument Open(Stream stream)
        {
            if (stream == null)
                throw new ArgumentNullException(nameof(stream));

            byte[] data;
            using (var ms = new MemoryStream())
            {
                stream.CopyTo(ms);
                data = ms.ToArray();
            }

            var handle = NativeMethods.PdfDocumentOpenFromBytes(data, data.Length, out var errorCode);
            if (handle.IsInvalid)
            {
                ExceptionMapper.ThrowIfError(errorCode);
            }

            return new PdfDocument(handle);
        }

        /// <summary>
        /// Gets the PDF version as a tuple of (major, minor).
        /// </summary>
        /// <value>A tuple containing the major and minor version numbers.</value>
        /// <exception cref="ObjectDisposedException">Thrown if the document has been disposed.</exception>
        public (byte Major, byte Minor) Version
        {
            get
            {
                ThrowIfDisposed();
                NativeMethods.PdfDocumentGetVersion(_handle, out var major, out var minor);
                return (major, minor);
            }
        }

        /// <summary>
        /// Gets the number of pages in the document.
        /// </summary>
        /// <value>The page count.</value>
        /// <exception cref="ObjectDisposedException">Thrown if the document has been disposed.</exception>
        /// <exception cref="PdfException">Thrown if page count cannot be determined.</exception>
        public int PageCount
        {
            get
            {
                ThrowIfDisposed();
                var count = NativeMethods.PdfDocumentGetPageCount(_handle, out var errorCode);
                ExceptionMapper.ThrowIfError(errorCode);
                return count;
            }
        }

        /// <summary>
        /// Gets a value indicating whether the document has a structure tree (Tagged PDF).
        /// </summary>
        /// <value>True if the document has a structure tree, false otherwise.</value>
        /// <exception cref="ObjectDisposedException">Thrown if the document has been disposed.</exception>
        public bool HasStructureTree
        {
            get
            {
                ThrowIfDisposed();
                return NativeMethods.PdfDocumentHasStructureTree(_handle);
            }
        }

        /// <summary>
        /// Extracts text from a page with automatic reading order detection.
        /// </summary>
        /// <param name="pageIndex">The page index (0-based).</param>
        /// <returns>
        /// The extracted text as a managed <see cref="string"/>. The native
        /// layer returns UTF-8, which is decoded to .NET's native UTF-16 here,
        /// so <see cref="string.Length"/> reports UTF-16 code units, not
        /// bytes. Use <c>System.Text.Encoding.UTF8.GetByteCount(text)</c> if
        /// you need the byte count (e.g. to compare against Go's
        /// <c>len(string)</c> or Rust's <c>String::len()</c>).
        /// </returns>
        /// <exception cref="ArgumentOutOfRangeException">Thrown if <paramref name="pageIndex"/> is out of range.</exception>
        /// <exception cref="ObjectDisposedException">Thrown if the document has been disposed.</exception>
        /// <exception cref="PdfException">Thrown if text extraction fails.</exception>
        public string ExtractText(int pageIndex)
        {
            ThrowIfDisposed();

            if (pageIndex < 0 || pageIndex >= PageCount)
                throw new ArgumentOutOfRangeException(nameof(pageIndex));

            var ptr = NativeMethods.PdfDocumentExtractText(_handle, pageIndex, out var errorCode);
            ExceptionMapper.ThrowIfError(errorCode);

            return StringMarshaler.PtrToStringAndFree(ptr);
        }

        /// <summary>
        /// Asynchronously extracts text from a page.
        /// </summary>
        /// <param name="pageIndex">The page index (0-based).</param>
        /// <param name="cancellationToken">A cancellation token.</param>
        /// <returns>A task that yields the extracted text.</returns>
        /// <exception cref="ArgumentOutOfRangeException">Thrown if <paramref name="pageIndex"/> is out of range.</exception>
        /// <exception cref="OperationCanceledException">Thrown if the operation is cancelled.</exception>
        public Task<string> ExtractTextAsync(int pageIndex, CancellationToken cancellationToken = default)
        {
            return Task.Run(() =>
            {
                cancellationToken.ThrowIfCancellationRequested();
                return ExtractText(pageIndex);
            }, cancellationToken);
        }

        /// <summary>
        /// Converts a page to Markdown format.
        /// </summary>
        /// <param name="pageIndex">The page index (0-based).</param>
        /// <returns>The page content as Markdown.</returns>
        /// <exception cref="ArgumentOutOfRangeException">Thrown if <paramref name="pageIndex"/> is out of range.</exception>
        /// <exception cref="ObjectDisposedException">Thrown if the document has been disposed.</exception>
        /// <exception cref="PdfException">Thrown if conversion fails.</exception>
        public string ToMarkdown(int pageIndex)
        {
            ThrowIfDisposed();

            if (pageIndex < 0 || pageIndex >= PageCount)
                throw new ArgumentOutOfRangeException(nameof(pageIndex));

            var ptr = NativeMethods.PdfDocumentToMarkdown(_handle, pageIndex, out var errorCode);
            ExceptionMapper.ThrowIfError(errorCode);

            return StringMarshaler.PtrToStringAndFree(ptr);
        }

        /// <summary>
        /// Converts all pages to Markdown format.
        /// </summary>
        /// <returns>The document content as Markdown.</returns>
        /// <exception cref="ObjectDisposedException">Thrown if the document has been disposed.</exception>
        /// <exception cref="PdfException">Thrown if conversion fails.</exception>
        public string ToMarkdownAll()
        {
            ThrowIfDisposed();

            var ptr = NativeMethods.PdfDocumentToMarkdownAll(_handle, out var errorCode);
            ExceptionMapper.ThrowIfError(errorCode);

            return StringMarshaler.PtrToStringAndFree(ptr);
        }

        /// <summary>
        /// Converts a page to HTML format.
        /// </summary>
        /// <param name="pageIndex">The page index (0-based).</param>
        /// <returns>The page content as HTML.</returns>
        /// <exception cref="ArgumentOutOfRangeException">Thrown if <paramref name="pageIndex"/> is out of range.</exception>
        /// <exception cref="ObjectDisposedException">Thrown if the document has been disposed.</exception>
        /// <exception cref="PdfException">Thrown if conversion fails.</exception>
        public string ToHtml(int pageIndex)
        {
            ThrowIfDisposed();

            if (pageIndex < 0 || pageIndex >= PageCount)
                throw new ArgumentOutOfRangeException(nameof(pageIndex));

            var ptr = NativeMethods.PdfDocumentToHtml(_handle, pageIndex, out var errorCode);
            ExceptionMapper.ThrowIfError(errorCode);

            return StringMarshaler.PtrToStringAndFree(ptr);
        }

        /// <summary>
        /// Converts a page to plain text format.
        /// </summary>
        /// <param name="pageIndex">The page index (0-based).</param>
        /// <returns>The page content as plain text.</returns>
        /// <exception cref="ArgumentOutOfRangeException">Thrown if <paramref name="pageIndex"/> is out of range.</exception>
        /// <exception cref="ObjectDisposedException">Thrown if the document has been disposed.</exception>
        /// <exception cref="PdfException">Thrown if conversion fails.</exception>
        public string ToPlainText(int pageIndex)
        {
            ThrowIfDisposed();

            if (pageIndex < 0 || pageIndex >= PageCount)
                throw new ArgumentOutOfRangeException(nameof(pageIndex));

            var ptr = NativeMethods.PdfDocumentToPlainText(_handle, pageIndex, out var errorCode);
            ExceptionMapper.ThrowIfError(errorCode);

            return StringMarshaler.PtrToStringAndFree(ptr);
        }

        // ================================================================
        // v0.3.23 New Methods
        // ================================================================

        /// <summary>Extracts text from all pages.</summary>
        public string ExtractAllText()
        {
            ThrowIfDisposed();
            var ptr = NativeMethods.pdf_document_extract_all_text(_handle.Ptr, out var errorCode);
            ExceptionMapper.ThrowIfError(errorCode);
            return StringMarshaler.PtrToStringAndFree(ptr);
        }

        /// <summary>Converts all pages to HTML.</summary>
        public string ToHtmlAll()
        {
            ThrowIfDisposed();
            var ptr = NativeMethods.pdf_document_to_html_all(_handle.Ptr, out var errorCode);
            ExceptionMapper.ThrowIfError(errorCode);
            return StringMarshaler.PtrToStringAndFree(ptr);
        }

        /// <summary>Converts all pages to plain text.</summary>
        public string ToPlainTextAll()
        {
            ThrowIfDisposed();
            var ptr = NativeMethods.pdf_document_to_plain_text_all(_handle.Ptr, out var errorCode);
            ExceptionMapper.ThrowIfError(errorCode);
            return StringMarshaler.PtrToStringAndFree(ptr);
        }

        /// <summary>Checks if the document is encrypted.</summary>
        public bool IsEncrypted
        {
            get
            {
                ThrowIfDisposed();
                return NativeMethods.pdf_document_is_encrypted(_handle.Ptr);
            }
        }

        /// <summary>Authenticates with a password. Returns true if successful.</summary>
        public bool Authenticate(string password)
        {
            ThrowIfDisposed();
            if (password == null) throw new ArgumentNullException(nameof(password));
            return NativeMethods.pdf_document_authenticate(_handle.Ptr, password, out _);
        }

        /// <summary>Checks if the document has XFA forms.</summary>
        public bool HasXfa
        {
            get
            {
                ThrowIfDisposed();
                return NativeMethods.pdf_document_has_xfa(_handle.Ptr);
            }
        }

        /// <summary>Extracts text from a rectangular region on a page.</summary>
        public string ExtractTextInRect(int pageIndex, float x, float y, float width, float height)
        {
            ThrowIfDisposed();
            var ptr = NativeMethods.pdf_document_extract_text_in_rect(_handle.Ptr, pageIndex, x, y, width, height, out var errorCode);
            ExceptionMapper.ThrowIfError(errorCode);
            return StringMarshaler.PtrToStringAndFree(ptr);
        }

        /// <summary>Removes repeated headers across pages. Returns count removed.</summary>
        public int RemoveHeaders(float threshold = 0.8f)
        {
            ThrowIfDisposed();
            var n = NativeMethods.pdf_document_remove_headers(_handle.Ptr, threshold, out var errorCode);
            ExceptionMapper.ThrowIfError(errorCode);
            return n;
        }

        /// <summary>Removes repeated footers across pages. Returns count removed.</summary>
        public int RemoveFooters(float threshold = 0.8f)
        {
            ThrowIfDisposed();
            var n = NativeMethods.pdf_document_remove_footers(_handle.Ptr, threshold, out var errorCode);
            ExceptionMapper.ThrowIfError(errorCode);
            return n;
        }

        /// <summary>Removes headers and footers. Returns count removed.</summary>
        public int RemoveArtifacts(float threshold = 0.8f)
        {
            ThrowIfDisposed();
            var n = NativeMethods.pdf_document_remove_artifacts(_handle.Ptr, threshold, out var errorCode);
            ExceptionMapper.ThrowIfError(errorCode);
            return n;
        }

        /// <summary>Opens a PDF with password.</summary>
        public static PdfDocument OpenWithPassword(string path, string password)
        {
            var doc = Open(path);
            NativeMethods.pdf_document_authenticate(doc._handle.Ptr, password, out _);
            return doc;
        }

        /// <summary>Extracts words from a page. Returns handle-based results (use NativeMethods directly for now).</summary>
        public (string Text, float X, float Y, float W, float H)[] ExtractWords(int pageIndex)
        {
            ThrowIfDisposed();
            var handle = NativeMethods.pdf_document_extract_words(_handle.Ptr, pageIndex, out var errorCode);
            ExceptionMapper.ThrowIfError(errorCode);
            if (handle == IntPtr.Zero) return Array.Empty<(string, float, float, float, float)>();
            try
            {
                var count = NativeMethods.pdf_oxide_word_count(handle);
                var results = new (string, float, float, float, float)[count];
                for (int i = 0; i < count; i++)
                {
                    var textPtr = NativeMethods.pdf_oxide_word_get_text(handle, i, out _);
                    var text = StringMarshaler.PtrToStringAndFree(textPtr);
                    NativeMethods.pdf_oxide_word_get_bbox(handle, i, out var x, out var y, out var w, out var h, out _);
                    results[i] = (text, x, y, w, h);
                }
                return results;
            }
            finally { NativeMethods.pdf_oxide_word_list_free(handle); }
        }

        /// <summary>Extracts text lines from a page.</summary>
        public (string Text, float X, float Y, float W, float H)[] ExtractTextLines(int pageIndex)
        {
            ThrowIfDisposed();
            var handle = NativeMethods.pdf_document_extract_text_lines(_handle.Ptr, pageIndex, out var errorCode);
            ExceptionMapper.ThrowIfError(errorCode);
            if (handle == IntPtr.Zero) return Array.Empty<(string, float, float, float, float)>();
            try
            {
                var count = NativeMethods.pdf_oxide_line_count(handle);
                var results = new (string, float, float, float, float)[count];
                for (int i = 0; i < count; i++)
                {
                    var textPtr = NativeMethods.pdf_oxide_line_get_text(handle, i, out _);
                    var text = StringMarshaler.PtrToStringAndFree(textPtr);
                    NativeMethods.pdf_oxide_line_get_bbox(handle, i, out var x, out var y, out var w, out var h, out _);
                    results[i] = (text, x, y, w, h);
                }
                return results;
            }
            finally { NativeMethods.pdf_oxide_line_list_free(handle); }
        }

        /// <summary>Extracts tables from a page. Returns row/col counts per table.</summary>
        public (int RowCount, int ColCount)[] ExtractTables(int pageIndex)
        {
            ThrowIfDisposed();
            var handle = NativeMethods.pdf_document_extract_tables(_handle.Ptr, pageIndex, out var errorCode);
            ExceptionMapper.ThrowIfError(errorCode);
            if (handle == IntPtr.Zero) return Array.Empty<(int, int)>();
            try
            {
                var count = NativeMethods.pdf_oxide_table_count(handle);
                var results = new (int, int)[count];
                for (int i = 0; i < count; i++)
                {
                    var rows = NativeMethods.pdf_oxide_table_get_row_count(handle, i, out _);
                    var cols = NativeMethods.pdf_oxide_table_get_col_count(handle, i, out _);
                    results[i] = (rows, cols);
                }
                return results;
            }
            finally { NativeMethods.pdf_oxide_table_list_free(handle); }
        }

        /// <summary>Searches all pages for text. Returns results with page index and bounding box.</summary>
        public (int Page, string Text, float X, float Y, float W, float H)[] SearchAll(string text, bool caseSensitive = false)
        {
            ThrowIfDisposed();
            if (text == null) throw new ArgumentNullException(nameof(text));
            var resultsHandle = NativeMethods.pdf_document_search_all(_handle.Ptr, text, caseSensitive, out var errorCode);
            ExceptionMapper.ThrowIfError(errorCode);
            if (resultsHandle == IntPtr.Zero) return Array.Empty<(int, string, float, float, float, float)>();
            try
            {
                return DecodeSearchResults(resultsHandle);
            }
            finally { NativeMethods.pdf_oxide_search_result_free(resultsHandle); }
        }

        /// <summary>Searches a specific page for text.</summary>
        public (int Page, string Text, float X, float Y, float W, float H)[] SearchPage(int pageIndex, string text, bool caseSensitive = false)
        {
            ThrowIfDisposed();
            if (text == null) throw new ArgumentNullException(nameof(text));
            var resultsHandle = NativeMethods.pdf_document_search_page(_handle.Ptr, pageIndex, text, caseSensitive, out var errorCode);
            ExceptionMapper.ThrowIfError(errorCode);
            if (resultsHandle == IntPtr.Zero) return Array.Empty<(int, string, float, float, float, float)>();
            try
            {
                return DecodeSearchResults(resultsHandle);
            }
            finally { NativeMethods.pdf_oxide_search_result_free(resultsHandle); }
        }

        // One FFI crossing → Rust serializes the entire result list to JSON →
        // System.Text.Json decodes it. Matches the Go binding pattern and is
        // O(1) FFI calls instead of O(count × 4) per-field calls.
        private static (int Page, string Text, float X, float Y, float W, float H)[] DecodeSearchResults(IntPtr handle)
        {
            var jsonPtr = NativeMethods.PdfOxideSearchResultsToJson(handle, out var jsonErr);
            ExceptionMapper.ThrowIfError(jsonErr);
            if (jsonPtr == IntPtr.Zero) return Array.Empty<(int, string, float, float, float, float)>();

            string json;
            try
            {
                json = StringMarshaler.PtrToString(jsonPtr);
            }
            finally
            {
                NativeMethods.FreeString(jsonPtr);
            }

            using var doc = System.Text.Json.JsonDocument.Parse(json);
            var arr = doc.RootElement;
            var results = new (int, string, float, float, float, float)[arr.GetArrayLength()];
            int idx = 0;
            foreach (var el in arr.EnumerateArray())
            {
                results[idx++] = (
                    el.GetProperty("page").GetInt32(),
                    el.GetProperty("text").GetString() ?? string.Empty,
                    el.GetProperty("x").GetSingle(),
                    el.GetProperty("y").GetSingle(),
                    el.GetProperty("width").GetSingle(),
                    el.GetProperty("height").GetSingle());
            }
            return results;
        }

        /// <summary>Gets page labels as JSON.</summary>
        public string GetPageLabels()
        {
            ThrowIfDisposed();
            var ptr = NativeMethods.pdf_document_get_page_labels(_handle.Ptr, out var errorCode);
            ExceptionMapper.ThrowIfError(errorCode);
            return StringMarshaler.PtrToStringAndFree(ptr);
        }

        /// <summary>Gets XMP metadata as JSON.</summary>
        public string GetXmpMetadata()
        {
            ThrowIfDisposed();
            var ptr = NativeMethods.pdf_document_get_xmp_metadata(_handle.Ptr, out var errorCode);
            ExceptionMapper.ThrowIfError(errorCode);
            return StringMarshaler.PtrToStringAndFree(ptr);
        }

        /// <summary>Gets document outline/bookmarks as JSON.</summary>
        public string GetOutline()
        {
            ThrowIfDisposed();
            var ptr = NativeMethods.pdf_document_get_outline(_handle.Ptr, out var errorCode);
            ExceptionMapper.ThrowIfError(errorCode);
            return StringMarshaler.PtrToStringAndFree(ptr);
        }

        /// <summary>Extracts individual characters from a page.</summary>
        public (char Char, float X, float Y, float W, float H)[] ExtractChars(int pageIndex)
        {
            ThrowIfDisposed();
            var handle = NativeMethods.pdf_document_extract_chars(_handle.Ptr, pageIndex, out var errorCode);
            ExceptionMapper.ThrowIfError(errorCode);
            if (handle == IntPtr.Zero) return Array.Empty<(char, float, float, float, float)>();
            try
            {
                var count = NativeMethods.pdf_oxide_char_count(handle);
                var results = new (char, float, float, float, float)[count];
                for (int i = 0; i < count; i++)
                {
                    var ch = NativeMethods.pdf_oxide_char_get_char(handle, i, out _);
                    NativeMethods.pdf_oxide_char_get_bbox(handle, i, out var x, out var y, out var w, out var h, out _);
                    results[i] = ((char)ch, x, y, w, h);
                }
                return results;
            }
            finally { NativeMethods.pdf_oxide_char_list_free(handle); }
        }

        /// <summary>Extracts paths from a page.</summary>
        public (float X, float Y, float W, float H, float StrokeWidth)[] ExtractPaths(int pageIndex)
        {
            ThrowIfDisposed();
            var handle = NativeMethods.pdf_document_extract_paths(_handle.Ptr, pageIndex, out var errorCode);
            ExceptionMapper.ThrowIfError(errorCode);
            if (handle == IntPtr.Zero) return Array.Empty<(float, float, float, float, float)>();
            try
            {
                var count = NativeMethods.pdf_oxide_path_count(handle);
                var results = new (float, float, float, float, float)[count];
                for (int i = 0; i < count; i++)
                {
                    NativeMethods.pdf_oxide_path_get_bbox(handle, i, out var x, out var y, out var w, out var h, out _);
                    var sw = NativeMethods.pdf_oxide_path_get_stroke_width(handle, i, out _);
                    results[i] = (x, y, w, h, sw);
                }
                return results;
            }
            finally { NativeMethods.pdf_oxide_path_list_free(handle); }
        }

        /// <summary>Extracts words from a rectangular region.</summary>
        public (string Text, float X, float Y, float W, float H)[] ExtractWordsInRect(int pageIndex, float x, float y, float width, float height)
        {
            ThrowIfDisposed();
            var handle = NativeMethods.pdf_document_extract_words_in_rect(_handle.Ptr, pageIndex, x, y, width, height, out var errorCode);
            ExceptionMapper.ThrowIfError(errorCode);
            if (handle == IntPtr.Zero) return Array.Empty<(string, float, float, float, float)>();
            try
            {
                var count = NativeMethods.pdf_oxide_word_count(handle);
                var results = new (string, float, float, float, float)[count];
                for (int i = 0; i < count; i++)
                {
                    var textPtr = NativeMethods.pdf_oxide_word_get_text(handle, i, out _);
                    var text = StringMarshaler.PtrToStringAndFree(textPtr);
                    NativeMethods.pdf_oxide_word_get_bbox(handle, i, out var wx, out var wy, out var ww, out var wh, out _);
                    results[i] = (text, wx, wy, ww, wh);
                }
                return results;
            }
            finally { NativeMethods.pdf_oxide_word_list_free(handle); }
        }

        /// <summary>Gets font names from a page.</summary>
        public string[] GetFonts(int pageIndex)
        {
            ThrowIfDisposed();
            var handle = NativeMethods.pdf_document_get_embedded_fonts(_handle.Ptr, pageIndex, out var errorCode);
            ExceptionMapper.ThrowIfError(errorCode);
            if (handle == IntPtr.Zero) return Array.Empty<string>();
            try
            {
                var count = NativeMethods.pdf_oxide_font_count(handle);
                var results = new string[count];
                for (int i = 0; i < count; i++)
                {
                    var namePtr = NativeMethods.pdf_oxide_font_get_name(handle, i, out _);
                    results[i] = StringMarshaler.PtrToStringAndFree(namePtr);
                }
                return results;
            }
            finally { NativeMethods.pdf_oxide_font_list_free(handle); }
        }

        /// <summary>Renders a page to PNG bytes. format: 0=PNG, 1=JPEG.</summary>
        public byte[] RenderPage(int pageIndex, int format = 0)
        {
            ThrowIfDisposed();
            var imgHandle = NativeMethods.pdf_render_page(_handle.Ptr, pageIndex, format, out var errorCode);
            ExceptionMapper.ThrowIfError(errorCode);
            if (imgHandle == IntPtr.Zero) return Array.Empty<byte>();
            try
            {
                var data = NativeMethods.pdf_get_rendered_image_data(imgHandle, out var dataLen, out _);
                if (data == IntPtr.Zero) return Array.Empty<byte>();
                var bytes = new byte[dataLen];
                System.Runtime.InteropServices.Marshal.Copy(data, bytes, 0, dataLen);
                NativeMethods.FreeBytes(data, dataLen);
                return bytes;
            }
            finally { NativeMethods.pdf_rendered_image_free(imgHandle); }
        }

        /// <summary>Renders a page with zoom factor. Returns PNG bytes.</summary>
        public byte[] RenderPageZoom(int pageIndex, float zoom, int format = 0)
        {
            ThrowIfDisposed();
            var imgHandle = NativeMethods.pdf_render_page_zoom(_handle.Ptr, pageIndex, zoom, format, out var errorCode);
            ExceptionMapper.ThrowIfError(errorCode);
            if (imgHandle == IntPtr.Zero) return Array.Empty<byte>();
            try
            {
                var data = NativeMethods.pdf_get_rendered_image_data(imgHandle, out var dataLen, out _);
                if (data == IntPtr.Zero) return Array.Empty<byte>();
                var bytes = new byte[dataLen];
                System.Runtime.InteropServices.Marshal.Copy(data, bytes, 0, dataLen);
                NativeMethods.FreeBytes(data, dataLen);
                return bytes;
            }
            finally { NativeMethods.pdf_rendered_image_free(imgHandle); }
        }

        /// <summary>Renders a page thumbnail (72 DPI). Returns PNG bytes.</summary>
        public byte[] RenderThumbnail(int pageIndex, int format = 0)
        {
            ThrowIfDisposed();
            var imgHandle = NativeMethods.pdf_render_page_thumbnail(_handle.Ptr, pageIndex, 72, format, out var errorCode);
            ExceptionMapper.ThrowIfError(errorCode);
            if (imgHandle == IntPtr.Zero) return Array.Empty<byte>();
            try
            {
                var data = NativeMethods.pdf_get_rendered_image_data(imgHandle, out var dataLen, out _);
                if (data == IntPtr.Zero) return Array.Empty<byte>();
                var bytes = new byte[dataLen];
                System.Runtime.InteropServices.Marshal.Copy(data, bytes, 0, dataLen);
                NativeMethods.FreeBytes(data, dataLen);
                return bytes;
            }
            finally { NativeMethods.pdf_rendered_image_free(imgHandle); }
        }

        /// <summary>Saves a rendered page to a file.</summary>
        public void SaveRenderedImage(int pageIndex, string filePath, int format = 0)
        {
            ThrowIfDisposed();
            var imgHandle = NativeMethods.pdf_render_page(_handle.Ptr, pageIndex, format, out var errorCode);
            ExceptionMapper.ThrowIfError(errorCode);
            if (imgHandle == IntPtr.Zero) return;
            try { NativeMethods.pdf_save_rendered_image(imgHandle, filePath, out _); }
            finally { NativeMethods.pdf_rendered_image_free(imgHandle); }
        }

        /// <summary>
        /// Disposes the document and releases native resources.
        /// Thread-safe: acquires a write lock to prevent concurrent access during disposal.
        /// </summary>
        public void Dispose()
        {
            _lock.EnterWriteLock();
            try
            {
                if (!_disposed)
                {
                    _handle?.Dispose();
                    _disposed = true;
                }
            }
            finally
            {
                _lock.ExitWriteLock();
            }
        }

        /// <summary>
        /// Extracts embedded images from a page. Returns an empty array when the page has no images.
        /// </summary>
        /// <param name="pageIndex">Zero-based page index.</param>
        /// <returns>Array of embedded images with their pixel data and metadata.</returns>
        /// <exception cref="PdfException">Thrown if the native call fails.</exception>
        public ExtractedImage[] ExtractImages(int pageIndex)
        {
            _lock.EnterReadLock();
            try
            {
                ThrowIfDisposed();
                var list = NativeMethods.pdf_document_get_embedded_images(_handle.Ptr, pageIndex, out int err);
                if (err != 0)
                    throw new PdfException($"Failed to extract images: {PdfException.GetErrorMessage(err)}");
                if (list == IntPtr.Zero)
                    return Array.Empty<ExtractedImage>();
                try
                {
                    int count = NativeMethods.pdf_oxide_image_count_ptr(list);
                    var images = new ExtractedImage[count];
                    for (int i = 0; i < count; i++)
                    {
                        int w = NativeMethods.pdf_oxide_image_get_width_ptr(list, i, out int e1);
                        int h = NativeMethods.pdf_oxide_image_get_height_ptr(list, i, out int e2);
                        int bpc = NativeMethods.pdf_oxide_image_get_bits_per_component_ptr(list, i, out int e3);
                        string format = PtrToStringAndFree(NativeMethods.pdf_oxide_image_get_format_ptr(list, i, out int e4));
                        string colorspace = PtrToStringAndFree(NativeMethods.pdf_oxide_image_get_colorspace_ptr(list, i, out int e5));
                        var dataPtr = NativeMethods.pdf_oxide_image_get_data_ptr(list, i, out int dataLen, out int e6);
                        byte[] data = dataPtr != IntPtr.Zero && dataLen > 0
                            ? new byte[dataLen]
                            : Array.Empty<byte>();
                        if (dataPtr != IntPtr.Zero && dataLen > 0)
                        {
                            System.Runtime.InteropServices.Marshal.Copy(dataPtr, data, 0, dataLen);
                        }
                        images[i] = new ExtractedImage(w, h, format, colorspace, bpc, data);
                    }
                    return images;
                }
                finally
                {
                    NativeMethods.pdf_oxide_image_list_free_ptr(list);
                }
            }
            finally
            {
                _lock.ExitReadLock();
            }
        }

        /// <summary>
        /// Reads all form (AcroForm) fields from the document.
        /// </summary>
        /// <returns>Array of form fields. Empty if the document has no form fields.</returns>
        /// <exception cref="PdfException">Thrown if the native call fails.</exception>
        public FormField[] GetFormFields()
        {
            _lock.EnterReadLock();
            try
            {
                ThrowIfDisposed();
                var list = NativeMethods.pdf_document_get_form_fields(_handle.Ptr, out int err);
                if (err != 0)
                    throw new PdfException($"Failed to get form fields: {PdfException.GetErrorMessage(err)}");
                if (list == IntPtr.Zero)
                    return Array.Empty<FormField>();
                try
                {
                    int count = NativeMethods.pdf_oxide_form_field_count(list);
                    var fields = new FormField[count];
                    for (int i = 0; i < count; i++)
                    {
                        string name = PtrToStringAndFree(NativeMethods.pdf_oxide_form_field_get_name(list, i, out _));
                        string type = PtrToStringAndFree(NativeMethods.pdf_oxide_form_field_get_type(list, i, out _));
                        string value = PtrToStringAndFree(NativeMethods.pdf_oxide_form_field_get_value(list, i, out _));
                        fields[i] = new FormField(name, type, value);
                    }
                    return fields;
                }
                finally
                {
                    NativeMethods.pdf_oxide_form_field_list_free(list);
                }
            }
            finally
            {
                _lock.ExitReadLock();
            }
        }

        private static string PtrToStringAndFree(IntPtr ptr)
        {
            if (ptr == IntPtr.Zero)
                return string.Empty;
            try
            {
                return StringMarshaler.PtrToStringUtf8(ptr) ?? string.Empty;
            }
            finally
            {
                NativeMethods.FreeString(ptr);
            }
        }

        private void ThrowIfDisposed()
        {
            if (_disposed)
                throw new ObjectDisposedException(nameof(PdfDocument));
        }

        /// <summary>
        /// Sets the global log level for the native pdf_oxide library.
        /// </summary>
        /// <param name="level">Log level: 0=Off, 1=Error, 2=Warn, 3=Info, 4=Debug, 5=Trace</param>
        /// <exception cref="ArgumentOutOfRangeException">Thrown if level is not between 0 and 5.</exception>
        public static void SetLogLevel(int level)
        {
            if (level < 0 || level > 5)
                throw new ArgumentOutOfRangeException(nameof(level), "Log level must be between 0 (Off) and 5 (Trace).");
            NativeMethods.PdfOxideSetLogLevel(level);
        }

        /// <summary>
        /// Gets the current log level of the native pdf_oxide library.
        /// </summary>
        /// <returns>Current log level (0=Off, 1=Error, 2=Warn, 3=Info, 4=Debug, 5=Trace).</returns>
        public static int GetLogLevel()
        {
            return NativeMethods.PdfOxideGetLogLevel();
        }
    }

    /// <summary>
    /// An embedded image extracted from a PDF page.
    /// </summary>
    public sealed class ExtractedImage
    {
        /// <summary>Image width in pixels.</summary>
        public int Width { get; }

        /// <summary>Image height in pixels.</summary>
        public int Height { get; }

        /// <summary>Container format (e.g. "Jpeg", "Png", "Raw").</summary>
        public string Format { get; }

        /// <summary>Color space string (e.g. "DeviceRGB", "DeviceGray", "DeviceCMYK").</summary>
        public string Colorspace { get; }

        /// <summary>Bits per component (typically 1, 8, or 16).</summary>
        public int BitsPerComponent { get; }

        /// <summary>Raw image bytes. Interpretation depends on <see cref="Format"/>.</summary>
        public byte[] Data { get; }

        internal ExtractedImage(int width, int height, string format, string colorspace, int bitsPerComponent, byte[] data)
        {
            Width = width;
            Height = height;
            Format = format;
            Colorspace = colorspace;
            BitsPerComponent = bitsPerComponent;
            Data = data;
        }
    }

    /// <summary>
    /// An AcroForm field read from a PDF document.
    /// </summary>
    public sealed class FormField
    {
        /// <summary>Fully-qualified field name (e.g. "employee.name").</summary>
        public string Name { get; }

        /// <summary>Field type string (e.g. "Text", "Button", "Choice", "Signature").</summary>
        public string FieldType { get; }

        /// <summary>Current value of the field as a string (empty for unset fields).</summary>
        public string Value { get; }

        internal FormField(string name, string fieldType, string value)
        {
            Name = name;
            FieldType = fieldType;
            Value = value;
        }
    }
}
