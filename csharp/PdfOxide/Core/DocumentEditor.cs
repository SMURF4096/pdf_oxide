using System;
using System.IO;
using System.Threading;
using System.Threading.Tasks;
using PdfOxide.Exceptions;
using PdfOxide.Internal;

namespace PdfOxide.Core
{
    /// <summary>
    /// Represents a PDF document opened for editing.
    /// Provides capabilities to modify metadata, content, and save changes.
    /// </summary>
    /// <remarks>
    /// <para>
    /// DocumentEditor is the editing API that provides:
    /// <list type="bullet">
    /// <item><description>Opening existing PDFs for editing</description></item>
    /// <item><description>Modifying document metadata (title, author, subject)</description></item>
    /// <item><description>Managing pages (add, remove, reorder)</description></item>
    /// <item><description>Modifying page content (text, images, annotations)</description></item>
    /// <item><description>Saving changes with incremental updates or full rewrite</description></item>
    /// </list>
    /// </para>
    /// <para>
    /// The document must be explicitly disposed to release native resources.
    /// Use 'using' statements for automatic cleanup.
    /// </para>
    /// </remarks>
    /// <example>
    /// <code>
    /// // Open a PDF for editing
    /// using (var editor = DocumentEditor.Open("document.pdf"))
    /// {
    ///     // Modify metadata
    ///     editor.Title = "Updated Title";
    ///     editor.Author = "New Author";
    ///
    ///     // Save changes
    ///     editor.Save("output.pdf");
    /// }
    /// </code>
    /// </example>
    public sealed class DocumentEditor : IDisposable
    {
        private NativeHandle _handle;
        private bool _disposed;

        private DocumentEditor(NativeHandle handle)
        {
            _handle = handle ?? throw new ArgumentNullException(nameof(handle));
        }

        /// <summary>
        /// Opens a PDF document for editing.
        /// </summary>
        /// <param name="path">The file path to the PDF document.</param>
        /// <returns>A new DocumentEditor instance.</returns>
        /// <exception cref="ArgumentNullException">Thrown if <paramref name="path"/> is null.</exception>
        /// <exception cref="PdfException">Thrown if the document cannot be opened.</exception>
        /// <example>
        /// <code>
        /// using (var editor = DocumentEditor.Open("document.pdf"))
        /// {
        ///     Console.WriteLine($"Pages: {editor.PageCount}");
        /// }
        /// </code>
        /// </example>
        public static DocumentEditor Open(string path)
        {
            if (path == null)
                throw new ArgumentNullException(nameof(path));

            var handle = NativeMethods.DocumentEditorOpen(path, out var errorCode);
            if (handle.IsInvalid)
            {
                ExceptionMapper.ThrowIfError(errorCode);
            }

            return new DocumentEditor(handle);
        }

        /// <summary>
        /// Checks if the document has unsaved changes.
        /// </summary>
        /// <value>True if the document has been modified, false otherwise.</value>
        public bool IsModified
        {
            get
            {
                ThrowIfDisposed();
                return NativeMethods.DocumentEditorIsModified(_handle.DangerousGetHandle());
            }
        }

        /// <summary>
        /// Gets the source file path for this document.
        /// </summary>
        /// <value>The file path where the document was opened from.</value>
        /// <exception cref="ObjectDisposedException">Thrown if the document has been disposed.</exception>
        public string SourcePath
        {
            get
            {
                ThrowIfDisposed();
                var ptr = NativeMethods.DocumentEditorGetSourcePath(_handle.DangerousGetHandle(), out var errorCode);
                ExceptionMapper.ThrowIfError(errorCode);

                try
                {
                    return StringMarshaler.PtrToString(ptr);
                }
                finally
                {
                    NativeMethods.FreeString(ptr);
                }
            }
        }

        /// <summary>
        /// Gets the PDF version as (major, minor).
        /// </summary>
        /// <value>A tuple containing the major and minor version numbers.</value>
        /// <exception cref="ObjectDisposedException">Thrown if the document has been disposed.</exception>
        public (byte Major, byte Minor) Version
        {
            get
            {
                ThrowIfDisposed();
                NativeMethods.DocumentEditorGetVersion(_handle.DangerousGetHandle(),
                    out var major, out var minor);
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
                var count = NativeMethods.DocumentEditorGetPageCount(_handle.DangerousGetHandle(), out var errorCode);
                ExceptionMapper.ThrowIfError(errorCode);
                return count;
            }
        }

        /// <summary>
        /// Gets or sets the document title.
        /// </summary>
        /// <value>The document title, or <c>null</c> if not set.</value>
        /// <exception cref="ObjectDisposedException">Thrown if the document has been disposed.</exception>
        /// <exception cref="PdfException">Thrown if the title cannot be retrieved or set.</exception>
        public string? Title
        {
            get
            {
                ThrowIfDisposed();
                var ptr = NativeMethods.DocumentEditorGetTitle(_handle.DangerousGetHandle(), out var errorCode);
                ExceptionMapper.ThrowIfError(errorCode);

                if (ptr == IntPtr.Zero)
                    return null;

                try
                {
                    return StringMarshaler.PtrToString(ptr);
                }
                finally
                {
                    NativeMethods.FreeString(ptr);
                }
            }
            set
            {
                ThrowIfDisposed();
                NativeMethods.DocumentEditorSetTitle(_handle.DangerousGetHandle(), value, out var errorCode);
                ExceptionMapper.ThrowIfError(errorCode);
            }
        }

        /// <summary>
        /// Gets or sets the document author.
        /// </summary>
        /// <value>The document author, or <c>null</c> if not set.</value>
        /// <exception cref="ObjectDisposedException">Thrown if the document has been disposed.</exception>
        /// <exception cref="PdfException">Thrown if the author cannot be retrieved or set.</exception>
        public string? Author
        {
            get
            {
                ThrowIfDisposed();
                var ptr = NativeMethods.DocumentEditorGetAuthor(_handle.DangerousGetHandle(), out var errorCode);
                ExceptionMapper.ThrowIfError(errorCode);

                if (ptr == IntPtr.Zero)
                    return null;

                try
                {
                    return StringMarshaler.PtrToString(ptr);
                }
                finally
                {
                    NativeMethods.FreeString(ptr);
                }
            }
            set
            {
                ThrowIfDisposed();
                NativeMethods.DocumentEditorSetAuthor(_handle.DangerousGetHandle(), value, out var errorCode);
                ExceptionMapper.ThrowIfError(errorCode);
            }
        }

        /// <summary>
        /// Gets or sets the document subject.
        /// </summary>
        /// <value>The document subject, or <c>null</c> if not set.</value>
        /// <exception cref="ObjectDisposedException">Thrown if the document has been disposed.</exception>
        /// <exception cref="PdfException">Thrown if the subject cannot be retrieved or set.</exception>
        public string? Subject
        {
            get
            {
                ThrowIfDisposed();
                var ptr = NativeMethods.DocumentEditorGetSubject(_handle.DangerousGetHandle(), out var errorCode);
                ExceptionMapper.ThrowIfError(errorCode);

                if (ptr == IntPtr.Zero)
                    return null;

                try
                {
                    return StringMarshaler.PtrToString(ptr);
                }
                finally
                {
                    NativeMethods.FreeString(ptr);
                }
            }
            set
            {
                ThrowIfDisposed();
                NativeMethods.DocumentEditorSetSubject(_handle.DangerousGetHandle(), value, out var errorCode);
                ExceptionMapper.ThrowIfError(errorCode);
            }
        }

        /// <summary>
        /// Saves the document to a file.
        /// </summary>
        /// <param name="path">The output file path.</param>
        /// <exception cref="ArgumentNullException">Thrown if <paramref name="path"/> is null.</exception>
        /// <exception cref="ObjectDisposedException">Thrown if the document has been disposed.</exception>
        /// <exception cref="PdfIoException">Thrown if the file cannot be written.</exception>
        /// <example>
        /// <code>
        /// using (var editor = DocumentEditor.Open("input.pdf"))
        /// {
        ///     editor.Title = "Modified";
        ///     editor.Save("output.pdf");
        /// }
        /// </code>
        /// </example>
        public void Save(string path)
        {
            if (path == null)
                throw new ArgumentNullException(nameof(path));

            ThrowIfDisposed();

            var result = NativeMethods.DocumentEditorSave(_handle.DangerousGetHandle(), path, out var errorCode);
            if (result != 0)
            {
                ExceptionMapper.ThrowIfError(errorCode);
            }
        }

        /// <summary>
        /// Sets the value of a form (AcroForm) field by its fully-qualified name.
        /// </summary>
        /// <param name="name">Fully-qualified field name (e.g. "employee.ssn").</param>
        /// <param name="value">New field value.</param>
        /// <exception cref="ArgumentNullException">Thrown if <paramref name="name"/> or <paramref name="value"/> is null.</exception>
        /// <exception cref="PdfException">Thrown if the native call fails.</exception>
        public void SetFormFieldValue(string name, string value)
        {
            if (name == null) throw new ArgumentNullException(nameof(name));
            if (value == null) throw new ArgumentNullException(nameof(value));
            ThrowIfDisposed();
            int rc = NativeMethods.document_editor_set_form_field_value(_handle, name, value, out int err);
            if (rc != 0)
                ExceptionMapper.ThrowIfError(err != 0 ? err : 11);
        }

        /// <summary>
        /// Flattens all form fields in the document, converting their rendered appearance into static content.
        /// After flattening, the fields are no longer editable.
        /// </summary>
        /// <exception cref="PdfException">Thrown if the native call fails.</exception>
        public void FlattenForms()
        {
            ThrowIfDisposed();
            int rc = NativeMethods.document_editor_flatten_forms(_handle, out int err);
            if (rc != 0)
                ExceptionMapper.ThrowIfError(err != 0 ? err : 11);
        }

        /// <summary>
        /// Asynchronously saves the document to a file.
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
        /// Disposes the DocumentEditor and releases native resources.
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
                throw new ObjectDisposedException(nameof(DocumentEditor));
        }
    }
}
