using System;
using PdfOxide.Exceptions;
using PdfOxide.Internal;

namespace PdfOxide.Core
{
    /// <summary>
    /// Fluent per-page builder returned by <see cref="DocumentBuilder.A4Page"/>,
    /// <see cref="DocumentBuilder.LetterPage"/>, and
    /// <see cref="DocumentBuilder.Page(float, float)"/>.
    /// </summary>
    /// <remarks>
    /// All content methods return <c>this</c> for chaining. Call
    /// <see cref="Done"/> to commit the page back to its parent builder;
    /// after that the <see cref="PageBuilder"/> is invalid. Use
    /// <see cref="Dispose"/> only for error recovery when you want to
    /// drop the page without committing.
    /// </remarks>
    public sealed class PageBuilder : IDisposable
    {
        private readonly DocumentBuilder _parent;
        private IntPtr _handle;
        private bool _done;

        internal PageBuilder(DocumentBuilder parent, IntPtr handle)
        {
            _parent = parent;
            _handle = handle;
        }

        private IntPtr Handle
        {
            get
            {
                if (_done)
                    throw new ObjectDisposedException(nameof(PageBuilder));
                return _handle;
            }
        }

        // --- Content ops -----------------------------------------------------

        /// <summary>Set the font + size for subsequent text.</summary>
        public PageBuilder Font(string name, float size)
        {
            if (name == null) throw new ArgumentNullException(nameof(name));
            NativeMethods.PdfPageBuilderFont(Handle, name, size, out var ec);
            ExceptionMapper.ThrowIfError(ec);
            return this;
        }

        /// <summary>Move the cursor to absolute coordinates (points from lower-left).</summary>
        public PageBuilder At(float x, float y)
        {
            NativeMethods.PdfPageBuilderAt(Handle, x, y, out var ec);
            ExceptionMapper.ThrowIfError(ec);
            return this;
        }

        /// <summary>Emit a line of text at the current cursor position.</summary>
        public PageBuilder Text(string text)
        {
            if (text == null) throw new ArgumentNullException(nameof(text));
            NativeMethods.PdfPageBuilderText(Handle, text, out var ec);
            ExceptionMapper.ThrowIfError(ec);
            return this;
        }

        /// <summary>Emit a heading. <paramref name="level"/> is 1–6.</summary>
        public PageBuilder Heading(byte level, string text)
        {
            if (text == null) throw new ArgumentNullException(nameof(text));
            NativeMethods.PdfPageBuilderHeading(Handle, level, text, out var ec);
            ExceptionMapper.ThrowIfError(ec);
            return this;
        }

        /// <summary>Emit a paragraph with automatic line wrapping.</summary>
        public PageBuilder Paragraph(string text)
        {
            if (text == null) throw new ArgumentNullException(nameof(text));
            NativeMethods.PdfPageBuilderParagraph(Handle, text, out var ec);
            ExceptionMapper.ThrowIfError(ec);
            return this;
        }

        /// <summary>Advance the cursor by <paramref name="points"/>.</summary>
        public PageBuilder Space(float points)
        {
            NativeMethods.PdfPageBuilderSpace(Handle, points, out var ec);
            ExceptionMapper.ThrowIfError(ec);
            return this;
        }

        /// <summary>Draw a horizontal rule across the page.</summary>
        public PageBuilder HorizontalRule()
        {
            NativeMethods.PdfPageBuilderHorizontalRule(Handle, out var ec);
            ExceptionMapper.ThrowIfError(ec);
            return this;
        }

        // --- Annotations (Phase 3) ------------------------------------------

        /// <summary>Attach a URL link to the previously-emitted text element.</summary>
        public PageBuilder LinkUrl(string url)
        {
            if (url == null) throw new ArgumentNullException(nameof(url));
            NativeMethods.PdfPageBuilderLinkUrl(Handle, url, out var ec);
            ExceptionMapper.ThrowIfError(ec);
            return this;
        }

        /// <summary>Link the previous text to an internal page (0-based).</summary>
        public PageBuilder LinkPage(int pageIndex)
        {
            NativeMethods.PdfPageBuilderLinkPage(Handle, (nuint)pageIndex, out var ec);
            ExceptionMapper.ThrowIfError(ec);
            return this;
        }

        /// <summary>Link the previous text to a named destination.</summary>
        public PageBuilder LinkNamed(string destination)
        {
            if (destination == null) throw new ArgumentNullException(nameof(destination));
            NativeMethods.PdfPageBuilderLinkNamed(Handle, destination, out var ec);
            ExceptionMapper.ThrowIfError(ec);
            return this;
        }

        /// <summary>Highlight the previous text with an RGB colour (0–1 channels).</summary>
        public PageBuilder Highlight(float r, float g, float b)
        {
            NativeMethods.PdfPageBuilderHighlight(Handle, r, g, b, out var ec);
            ExceptionMapper.ThrowIfError(ec);
            return this;
        }

        /// <summary>Underline the previous text.</summary>
        public PageBuilder Underline(float r, float g, float b)
        {
            NativeMethods.PdfPageBuilderUnderline(Handle, r, g, b, out var ec);
            ExceptionMapper.ThrowIfError(ec);
            return this;
        }

        /// <summary>Strike through the previous text.</summary>
        public PageBuilder Strikeout(float r, float g, float b)
        {
            NativeMethods.PdfPageBuilderStrikeout(Handle, r, g, b, out var ec);
            ExceptionMapper.ThrowIfError(ec);
            return this;
        }

        /// <summary>Squiggly-underline the previous text.</summary>
        public PageBuilder Squiggly(float r, float g, float b)
        {
            NativeMethods.PdfPageBuilderSquiggly(Handle, r, g, b, out var ec);
            ExceptionMapper.ThrowIfError(ec);
            return this;
        }

        /// <summary>Attach a sticky-note annotation to the previous text.</summary>
        public PageBuilder StickyNote(string text)
        {
            if (text == null) throw new ArgumentNullException(nameof(text));
            NativeMethods.PdfPageBuilderStickyNote(Handle, text, out var ec);
            ExceptionMapper.ThrowIfError(ec);
            return this;
        }

        /// <summary>Place a sticky-note at an absolute position on the page.</summary>
        public PageBuilder StickyNoteAt(float x, float y, string text)
        {
            if (text == null) throw new ArgumentNullException(nameof(text));
            NativeMethods.PdfPageBuilderStickyNoteAt(Handle, x, y, text, out var ec);
            ExceptionMapper.ThrowIfError(ec);
            return this;
        }

        /// <summary>Apply a text watermark to the page.</summary>
        public PageBuilder Watermark(string text)
        {
            if (text == null) throw new ArgumentNullException(nameof(text));
            NativeMethods.PdfPageBuilderWatermark(Handle, text, out var ec);
            ExceptionMapper.ThrowIfError(ec);
            return this;
        }

        /// <summary>Apply the standard "CONFIDENTIAL" diagonal watermark.</summary>
        public PageBuilder WatermarkConfidential()
        {
            NativeMethods.PdfPageBuilderWatermarkConfidential(Handle, out var ec);
            ExceptionMapper.ThrowIfError(ec);
            return this;
        }

        /// <summary>Apply the standard "DRAFT" diagonal watermark.</summary>
        public PageBuilder WatermarkDraft()
        {
            NativeMethods.PdfPageBuilderWatermarkDraft(Handle, out var ec);
            ExceptionMapper.ThrowIfError(ec);
            return this;
        }

        /// <summary>
        /// Commit the page's buffered operations back to the parent
        /// builder. Returns the parent for continued chaining. After
        /// <see cref="Done"/> this <see cref="PageBuilder"/> is invalid.
        /// </summary>
        public DocumentBuilder Done()
        {
            if (_done)
                throw new ObjectDisposedException(nameof(PageBuilder));
            var rc = NativeMethods.PdfPageBuilderDone(_handle, out var ec);
            _done = true;
            _parent.ClearOpenPage();
            _handle = IntPtr.Zero;
            if (rc != 0)
                ExceptionMapper.ThrowIfError(ec);
            return _parent;
        }

        /// <summary>
        /// Drop the page without committing. Use for error recovery —
        /// the parent's open-page slot is released so the next
        /// <see cref="DocumentBuilder.A4Page"/> etc. succeeds.
        /// </summary>
        public void Dispose()
        {
            if (!_done && _handle != IntPtr.Zero)
            {
                NativeMethods.PdfPageBuilderFree(_handle);
                _parent.ClearOpenPage();
                _handle = IntPtr.Zero;
                _done = true;
            }
        }
    }
}
