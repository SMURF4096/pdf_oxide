using System;
using System.IO;
using System.Threading;
using System.Threading.Tasks;
using PdfOxide.Exceptions;
using PdfOxide.Internal;

namespace PdfOxide.Core
{
    /// <summary>
    /// Represents a PDF document that can be created, edited, and saved.
    /// Universal API combining creation, reading, and editing capabilities.
    /// </summary>
    /// <remarks>
    /// <para>
    /// Pdf is the universal PDF API that provides:
    /// <list type="bullet">
    /// <item><description>Creating PDFs from Markdown, HTML, or plain text</description></item>
    /// <item><description>Saving to file or memory buffer</description></item>
    /// <item><description>Editing page content and metadata</description></item>
    /// <item><description>Extracting content and converting formats</description></item>
    /// </list>
    /// </para>
    /// <para>
    /// The document must be explicitly disposed to release native resources.
    /// Use 'using' statements for automatic cleanup.
    /// </para>
    /// </remarks>
    /// <example>
    /// <code>
    /// // Create PDF from Markdown
    /// using (var pdf = Pdf.FromMarkdown("# Hello\n\n**Bold** text"))
    /// {
    ///     pdf.Save("output.pdf");
    /// }
    ///
    /// // Create from HTML
    /// using (var pdf = Pdf.FromHtml("<h1>Title</h1><p>Content</p>"))
    /// {
    ///     byte[] bytes = pdf.SaveToBytes();
    ///     File.WriteAllBytes("output.pdf", bytes);
    /// }
    /// </code>
    /// </example>
    public sealed class Pdf : IDisposable
    {
        private NativeHandle _handle;
        private bool _disposed;

        private Pdf(NativeHandle handle)
        {
            _handle = handle ?? throw new ArgumentNullException(nameof(handle));
        }

        /// <summary>
        /// Creates a PDF from Markdown content.
        /// </summary>
        /// <param name="markdown">The Markdown content.</param>
        /// <returns>A new Pdf document.</returns>
        /// <exception cref="ArgumentNullException">Thrown if <paramref name="markdown"/> is null.</exception>
        /// <exception cref="PdfException">Thrown if PDF creation fails.</exception>
        /// <example>
        /// <code>
        /// using (var pdf = Pdf.FromMarkdown("# Title\n\nParagraph text"))
        /// {
        ///     pdf.Save("document.pdf");
        /// }
        /// </code>
        /// </example>
        public static Pdf FromMarkdown(string markdown)
        {
            if (markdown == null)
                throw new ArgumentNullException(nameof(markdown));

            var handle = NativeMethods.PdfFromMarkdown(markdown, out var errorCode);
            if (handle.IsInvalid)
            {
                ExceptionMapper.ThrowIfError(errorCode);
            }

            return new Pdf(handle);
        }

        /// <summary>
        /// Creates a PDF from HTML content.
        /// </summary>
        /// <param name="html">The HTML content.</param>
        /// <returns>A new Pdf document.</returns>
        /// <exception cref="ArgumentNullException">Thrown if <paramref name="html"/> is null.</exception>
        /// <exception cref="PdfException">Thrown if PDF creation fails.</exception>
        /// <example>
        /// <code>
        /// using (var pdf = Pdf.FromHtml("<h1>Title</h1><p>Content</p>"))
        /// {
        ///     pdf.Save("document.pdf");
        /// }
        /// </code>
        /// </example>
        public static Pdf FromHtml(string html)
        {
            if (html == null)
                throw new ArgumentNullException(nameof(html));

            var handle = NativeMethods.PdfFromHtml(html, out var errorCode);
            if (handle.IsInvalid)
            {
                ExceptionMapper.ThrowIfError(errorCode);
            }

            return new Pdf(handle);
        }

        /// <summary>
        /// Creates a PDF from plain text content.
        /// </summary>
        /// <param name="text">The text content.</param>
        /// <returns>A new Pdf document.</returns>
        /// <exception cref="ArgumentNullException">Thrown if <paramref name="text"/> is null.</exception>
        /// <exception cref="PdfException">Thrown if PDF creation fails.</exception>
        /// <example>
        /// <code>
        /// using (var pdf = Pdf.FromText("This is plain text"))
        /// {
        ///     pdf.Save("document.pdf");
        /// }
        /// </code>
        /// </example>
        public static Pdf FromText(string text)
        {
            if (text == null)
                throw new ArgumentNullException(nameof(text));

            var handle = NativeMethods.PdfFromText(text, out var errorCode);
            if (handle.IsInvalid)
            {
                ExceptionMapper.ThrowIfError(errorCode);
            }

            return new Pdf(handle);
        }

        /// <summary>
        /// Gets the number of pages in the PDF.
        /// </summary>
        /// <value>The page count.</value>
        /// <exception cref="ObjectDisposedException">Thrown if the document has been disposed.</exception>
        /// <exception cref="PdfException">Thrown if page count cannot be determined.</exception>
        public int PageCount
        {
            get
            {
                ThrowIfDisposed();
                var count = NativeMethods.PdfGetPageCount(_handle.DangerousGetHandle(), out var errorCode);
                ExceptionMapper.ThrowIfError(errorCode);
                return count;
            }
        }

        /// <summary>
        /// Saves the PDF to a file.
        /// </summary>
        /// <param name="path">The output file path.</param>
        /// <exception cref="ArgumentNullException">Thrown if <paramref name="path"/> is null.</exception>
        /// <exception cref="ObjectDisposedException">Thrown if the document has been disposed.</exception>
        /// <exception cref="PdfIoException">Thrown if the file cannot be written.</exception>
        /// <example>
        /// <code>
        /// using (var pdf = Pdf.FromMarkdown("# Hello"))
        /// {
        ///     pdf.Save("output.pdf");
        /// }
        /// </code>
        /// </example>
        public void Save(string path)
        {
            if (path == null)
                throw new ArgumentNullException(nameof(path));

            ThrowIfDisposed();

            var result = NativeMethods.PdfSave(_handle, path, out var errorCode);
            if (result != 0)
            {
                ExceptionMapper.ThrowIfError(errorCode);
            }
        }

        /// <summary>
        /// Saves the PDF to a byte array.
        /// </summary>
        /// <returns>The PDF content as bytes.</returns>
        /// <exception cref="ObjectDisposedException">Thrown if the document has been disposed.</exception>
        /// <exception cref="PdfException">Thrown if the PDF cannot be generated.</exception>
        /// <example>
        /// <code>
        /// using (var pdf = Pdf.FromMarkdown("# Hello"))
        /// {
        ///     byte[] pdfBytes = pdf.SaveToBytes();
        ///     File.WriteAllBytes("output.pdf", pdfBytes);
        /// }
        /// </code>
        /// </example>
        public byte[] SaveToBytes()
        {
            ThrowIfDisposed();

            var outputPtr = NativeMethods.PdfSaveToBytes(_handle, out var outputLen, out var errorCode);
            if (errorCode != 0)
            {
                ExceptionMapper.ThrowIfError(errorCode);
            }
            if (outputPtr == IntPtr.Zero || outputLen <= 0)
            {
                return System.Array.Empty<byte>();
            }

            try
            {
                var bytes = new byte[outputLen];
                System.Runtime.InteropServices.Marshal.Copy(outputPtr, bytes, 0, outputLen);
                return bytes;
            }
            finally
            {
                NativeMethods.FreeBytes(outputPtr, outputLen);
            }
        }

        /// <summary>
        /// Saves the PDF to a stream.
        /// </summary>
        /// <param name="stream">The output stream.</param>
        /// <exception cref="ArgumentNullException">Thrown if <paramref name="stream"/> is null.</exception>
        /// <exception cref="ObjectDisposedException">Thrown if the document has been disposed.</exception>
        /// <exception cref="PdfException">Thrown if the PDF cannot be generated.</exception>
        /// <example>
        /// <code>
        /// using (var pdf = Pdf.FromMarkdown("# Hello"))
        /// using (var file = File.Create("output.pdf"))
        /// {
        ///     pdf.SaveToStream(file);
        /// }
        /// </code>
        /// </example>
        public void SaveToStream(Stream stream)
        {
            if (stream == null)
                throw new ArgumentNullException(nameof(stream));

            byte[] bytes = SaveToBytes();
            stream.Write(bytes, 0, bytes.Length);
        }

        /// <summary>
        /// Asynchronously saves the PDF to a file.
        /// </summary>
        /// <param name="path">The output file path.</param>
        /// <param name="cancellationToken">A cancellation token.</param>
        /// <returns>A task that completes when the file is saved.</returns>
        /// <exception cref="ArgumentNullException">Thrown if <paramref name="path"/> is null.</exception>
        /// <exception cref="OperationCanceledException">Thrown if the operation is cancelled.</exception>
        public Task SaveAsync(string path, CancellationToken cancellationToken = default)
        {
            if (path == null)
                throw new ArgumentNullException(nameof(path));

            return Task.Run(() =>
            {
                cancellationToken.ThrowIfCancellationRequested();
                Save(path);
            }, cancellationToken);
        }

        /// <summary>
        /// Asynchronously saves the PDF to a stream.
        /// </summary>
        /// <param name="stream">The output stream.</param>
        /// <param name="cancellationToken">A cancellation token.</param>
        /// <returns>A task that completes when the PDF is saved.</returns>
        /// <exception cref="ArgumentNullException">Thrown if <paramref name="stream"/> is null.</exception>
        /// <exception cref="OperationCanceledException">Thrown if the operation is cancelled.</exception>
        public Task SaveToStreamAsync(Stream stream, CancellationToken cancellationToken = default)
        {
            if (stream == null)
                throw new ArgumentNullException(nameof(stream));

            return Task.Run(() =>
            {
                cancellationToken.ThrowIfCancellationRequested();
                SaveToStream(stream);
            }, cancellationToken);
        }

        /// <summary>
        /// Disposes the PDF and releases native resources.
        /// </summary>
        public void Dispose()
        {
            if (!_disposed)
            {
                _handle?.Dispose();
                _disposed = true;
            }
        }

        private void ThrowIfDisposed()
        {
            if (_disposed)
                throw new ObjectDisposedException(nameof(Pdf));
        }
    }
}
