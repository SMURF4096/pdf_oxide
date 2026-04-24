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
            ArgumentNullException.ThrowIfNull(path);

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
            ArgumentNullException.ThrowIfNull(path);

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
            ArgumentNullException.ThrowIfNull(name);
            ArgumentNullException.ThrowIfNull(value);
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
            ArgumentNullException.ThrowIfNull(path);

            return Task.Run(() =>
            {
                cancellationToken.ThrowIfCancellationRequested();
                Save(path);
            }, cancellationToken);
        }

        // ================================================================
        // Mutations. All 12 methods below are thin wrappers over the
        // `document_editor_*` P/Invoke declarations in NativeMethods.cs.
        // ================================================================

        /// <summary>
        /// Append all pages from another PDF file to the end of this document.
        /// </summary>
        /// <param name="sourcePath">Path of the PDF whose pages should be appended.</param>
        public void MergeFrom(string sourcePath)
        {
            ArgumentNullException.ThrowIfNull(sourcePath);
            ThrowIfDisposed();
            NativeMethods.document_editor_merge_from(_handle, sourcePath, out int err);
            ExceptionMapper.ThrowIfError(err);
        }

        /// <summary>
        /// Delete the page at <paramref name="pageIndex"/> (zero-based).
        /// </summary>
        public void DeletePage(int pageIndex)
        {
            ThrowIfDisposed();
            NativeMethods.document_editor_delete_page(_handle, pageIndex, out int err);
            ExceptionMapper.ThrowIfError(err);
        }

        /// <summary>
        /// Move a page from one position to another. Indices are zero-based
        /// and refer to positions <em>before</em> the move, matching the
        /// Rust / Python / Go contract.
        /// </summary>
        public void MovePage(int fromIndex, int toIndex)
        {
            ThrowIfDisposed();
            NativeMethods.document_editor_move_page(_handle, fromIndex, toIndex, out int err);
            ExceptionMapper.ThrowIfError(err);
        }

        /// <summary>
        /// Get the rotation of a page in degrees (0, 90, 180, 270).
        /// </summary>
        public int GetPageRotation(int pageIndex)
        {
            ThrowIfDisposed();
            int degrees = NativeMethods.document_editor_get_page_rotation(_handle, pageIndex, out int err);
            ExceptionMapper.ThrowIfError(err);
            return degrees;
        }

        /// <summary>
        /// Set the rotation of a page. Valid values are 0, 90, 180, 270.
        /// </summary>
        public void SetPageRotation(int pageIndex, int degrees)
        {
            ThrowIfDisposed();
            NativeMethods.document_editor_set_page_rotation(_handle, pageIndex, degrees, out int err);
            ExceptionMapper.ThrowIfError(err);
        }

        /// <summary>
        /// Crop the visible area of every page by subtracting margins (points).
        /// </summary>
        public void CropMargins(float left, float right, float top, float bottom)
        {
            ThrowIfDisposed();
            NativeMethods.document_editor_crop_margins(_handle, left, right, top, bottom, out int err);
            ExceptionMapper.ThrowIfError(err);
        }

        /// <summary>
        /// Erase a rectangular region on <paramref name="pageIndex"/>.
        /// Origin is the page's bottom-left in PDF user space.
        /// </summary>
        public void EraseRegion(int pageIndex, float x, float y, float width, float height)
        {
            ThrowIfDisposed();
            NativeMethods.document_editor_erase_region(_handle, pageIndex, x, y, width, height, out int err);
            ExceptionMapper.ThrowIfError(err);
        }

        /// <summary>
        /// Flatten annotations on a single page — bakes their visual
        /// representation into page content and removes the interactive
        /// annotation objects.
        /// </summary>
        public void FlattenAnnotations(int pageIndex)
        {
            ThrowIfDisposed();
            NativeMethods.document_editor_flatten_annotations(_handle, pageIndex, out int err);
            ExceptionMapper.ThrowIfError(err);
        }

        /// <summary>
        /// Flatten annotations across every page in the document.
        /// </summary>
        public void FlattenAllAnnotations()
        {
            ThrowIfDisposed();
            NativeMethods.document_editor_flatten_all_annotations(_handle, out int err);
            ExceptionMapper.ThrowIfError(err);
        }

        /// <summary>
        /// Flatten form fields on a single page without touching the rest
        /// of the document.
        /// </summary>
        public void FlattenFormsOnPage(int pageIndex)
        {
            ThrowIfDisposed();
            NativeMethods.document_editor_flatten_forms_on_page(_handle, pageIndex, out int err);
            ExceptionMapper.ThrowIfError(err);
        }

        /// <summary>
        /// Returns warnings collected during the last form-flattening save.
        /// Each entry names a widget field that had no <c>/AP</c> appearance
        /// stream; flattening it produces a blank rectangle.
        /// </summary>
        public string[] FlattenWarnings
        {
            get
            {
                ThrowIfDisposed();
                int count = NativeMethods.document_editor_flatten_warnings_count(_handle);
                if (count <= 0) return Array.Empty<string>();
                var result = new string[count];
                for (int i = 0; i < count; i++)
                {
                    IntPtr ptr = NativeMethods.document_editor_flatten_warning(_handle, i, out int _);
                    if (ptr != IntPtr.Zero)
                    {
                        result[i] = System.Runtime.InteropServices.Marshal.PtrToStringUTF8(ptr) ?? string.Empty;
                        NativeMethods.FreeString(ptr);
                    }
                }
                return result;
            }
        }

        /// <summary>
        /// Save the document with AES-256 encryption using the supplied
        /// user and owner passwords.
        /// </summary>
        public void SaveEncrypted(string path, string userPassword, string ownerPassword)
        {
            ArgumentNullException.ThrowIfNull(path);
            ArgumentNullException.ThrowIfNull(userPassword);
            ArgumentNullException.ThrowIfNull(ownerPassword);
            ThrowIfDisposed();
            NativeMethods.document_editor_save_encrypted(_handle, path, userPassword, ownerPassword, out int err);
            ExceptionMapper.ThrowIfError(err);
        }

        /// <summary>
        /// Gets or sets the document producer string (the tool that
        /// produced the PDF). Round-trips through the
        /// <c>/Info.Producer</c> metadata entry.
        /// </summary>
        public string? Producer
        {
            get
            {
                ThrowIfDisposed();
                var ptr = NativeMethods.document_editor_get_producer(_handle, out int err);
                ExceptionMapper.ThrowIfError(err);
                if (ptr == IntPtr.Zero) return null;
                try { return StringMarshaler.PtrToString(ptr); }
                finally { NativeMethods.FreeString(ptr); }
            }
            set
            {
                ThrowIfDisposed();
                NativeMethods.document_editor_set_producer(_handle, value ?? string.Empty, out int err);
                ExceptionMapper.ThrowIfError(err);
            }
        }

        /// <summary>
        /// Gets or sets the raw PDF creation-date string
        /// (e.g. <c>D:20260421120000Z</c>).
        /// </summary>
        public string? CreationDate
        {
            get
            {
                ThrowIfDisposed();
                var ptr = NativeMethods.document_editor_get_creation_date(_handle, out int err);
                ExceptionMapper.ThrowIfError(err);
                if (ptr == IntPtr.Zero) return null;
                try { return StringMarshaler.PtrToString(ptr); }
                finally { NativeMethods.FreeString(ptr); }
            }
            set
            {
                ThrowIfDisposed();
                NativeMethods.document_editor_set_creation_date(_handle, value ?? string.Empty, out int err);
                ExceptionMapper.ThrowIfError(err);
            }
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
            ObjectDisposedException.ThrowIf(_disposed, this);
        }
    }
}
