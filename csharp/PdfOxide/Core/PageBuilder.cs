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
                ObjectDisposedException.ThrowIf(_done, this);
                return _handle;
            }
        }

        // --- Content ops -----------------------------------------------------

        /// <summary>Set the font + size for subsequent text.</summary>
        public PageBuilder Font(string name, float size)
        {
            ArgumentNullException.ThrowIfNull(name);
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
            ArgumentNullException.ThrowIfNull(text);
            NativeMethods.PdfPageBuilderText(Handle, text, out var ec);
            ExceptionMapper.ThrowIfError(ec);
            return this;
        }

        /// <summary>Emit a heading. <paramref name="level"/> is 1–6.</summary>
        public PageBuilder Heading(byte level, string text)
        {
            ArgumentNullException.ThrowIfNull(text);
            NativeMethods.PdfPageBuilderHeading(Handle, level, text, out var ec);
            ExceptionMapper.ThrowIfError(ec);
            return this;
        }

        /// <summary>Emit a paragraph with automatic line wrapping.</summary>
        public PageBuilder Paragraph(string text)
        {
            ArgumentNullException.ThrowIfNull(text);
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
            ArgumentNullException.ThrowIfNull(url);
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
            ArgumentNullException.ThrowIfNull(destination);
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
            ArgumentNullException.ThrowIfNull(text);
            NativeMethods.PdfPageBuilderStickyNote(Handle, text, out var ec);
            ExceptionMapper.ThrowIfError(ec);
            return this;
        }

        /// <summary>Place a sticky-note at an absolute position on the page.</summary>
        public PageBuilder StickyNoteAt(float x, float y, string text)
        {
            ArgumentNullException.ThrowIfNull(text);
            NativeMethods.PdfPageBuilderStickyNoteAt(Handle, x, y, text, out var ec);
            ExceptionMapper.ThrowIfError(ec);
            return this;
        }

        /// <summary>Apply a text watermark to the page.</summary>
        public PageBuilder Watermark(string text)
        {
            ArgumentNullException.ThrowIfNull(text);
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
        /// Attach a standard stamp annotation at the cursor position
        /// (default 150×50 pt box). <paramref name="typeName"/> matches
        /// the PDF spec's standard stamps ("Approved", "NotApproved",
        /// "Draft", "Confidential", "Final", "Experimental", "Expired",
        /// "ForPublicRelease", "NotForPublicRelease", "AsIs", "Sold",
        /// "Departmental", "ForComment", "TopSecret"); any other name
        /// becomes a custom stamp.
        /// </summary>
        public PageBuilder Stamp(string typeName)
        {
            ArgumentNullException.ThrowIfNull(typeName);
            NativeMethods.PdfPageBuilderStamp(Handle, typeName, out var ec);
            ExceptionMapper.ThrowIfError(ec);
            return this;
        }

        /// <summary>
        /// Place a free-flowing text annotation inside the rectangle
        /// (x, y, w, h). Independent of the cursor flow.
        /// </summary>
        public PageBuilder FreeText(float x, float y, float w, float h, string text)
        {
            ArgumentNullException.ThrowIfNull(text);
            NativeMethods.PdfPageBuilderFreetext(Handle, x, y, w, h, text, out var ec);
            ExceptionMapper.ThrowIfError(ec);
            return this;
        }

        // --- Form-field widgets ---------------------------------------------

        /// <summary>
        /// Add a single-line text form field widget at the rectangle
        /// (x, y, w, h). <paramref name="name"/> is the unique field
        /// identifier used for form submission;
        /// <paramref name="defaultValue"/> is the initial text (pass
        /// null or empty for a blank field).
        /// </summary>
        public PageBuilder TextField(string name, float x, float y, float w, float h, string? defaultValue = null)
        {
            ArgumentNullException.ThrowIfNull(name);
            NativeMethods.PdfPageBuilderTextField(Handle, name, x, y, w, h, defaultValue, out var ec);
            ExceptionMapper.ThrowIfError(ec);
            return this;
        }

        /// <summary>
        /// Add a checkbox form field widget. <paramref name="checked"/>
        /// sets the initial state.
        /// </summary>
        public PageBuilder Checkbox(string name, float x, float y, float w, float h, bool @checked = false)
        {
            ArgumentNullException.ThrowIfNull(name);
            NativeMethods.PdfPageBuilderCheckbox(Handle, name, x, y, w, h, @checked, out var ec);
            ExceptionMapper.ThrowIfError(ec);
            return this;
        }

        /// <summary>
        /// Add a dropdown combo-box form field. Each entry of
        /// <paramref name="options"/> is a user-visible (and submitted)
        /// choice. <paramref name="selected"/> picks the initial value;
        /// pass null to leave blank.
        /// </summary>
        public unsafe PageBuilder ComboBox(string name, float x, float y, float w, float h,
            System.Collections.Generic.IReadOnlyList<string> options, string? selected = null)
        {
            ArgumentNullException.ThrowIfNull(name);
            if (options == null || options.Count == 0)
                throw new ArgumentException("options must be non-empty", nameof(options));
            int n = options.Count;
            var buffers = new byte[n][];
            var handles = new System.Runtime.InteropServices.GCHandle[n];
            var pointers = new IntPtr[n];
            try
            {
                for (int i = 0; i < n; i++)
                {
                    buffers[i] = System.Text.Encoding.UTF8.GetBytes(options[i] + "\0");
                    handles[i] = System.Runtime.InteropServices.GCHandle.Alloc(buffers[i], System.Runtime.InteropServices.GCHandleType.Pinned);
                    pointers[i] = handles[i].AddrOfPinnedObject();
                }
                var ptrHandle = System.Runtime.InteropServices.GCHandle.Alloc(pointers, System.Runtime.InteropServices.GCHandleType.Pinned);
                try
                {
                    NativeMethods.PdfPageBuilderComboBox(Handle, name, x, y, w, h,
                        (byte**)ptrHandle.AddrOfPinnedObject(), (nuint)n, selected, out var ec);
                    ExceptionMapper.ThrowIfError(ec);
                }
                finally { ptrHandle.Free(); }
            }
            finally
            {
                for (int i = 0; i < n; i++)
                    if (handles[i].IsAllocated) handles[i].Free();
            }
            return this;
        }

        /// <summary>
        /// Add a radio-button group. <paramref name="buttons"/> is a
        /// sequence of <c>(exportValue, x, y, w, h)</c> tuples — one
        /// entry per option. <paramref name="selected"/> picks the
        /// initial value.
        /// </summary>
        public unsafe PageBuilder RadioGroup(string name,
            System.Collections.Generic.IReadOnlyList<(string value, float x, float y, float w, float h)> buttons,
            string? selected = null)
        {
            ArgumentNullException.ThrowIfNull(name);
            if (buttons == null || buttons.Count == 0)
                throw new ArgumentException("buttons must be non-empty", nameof(buttons));
            int n = buttons.Count;
            var valueBuffers = new byte[n][];
            var handles = new System.Runtime.InteropServices.GCHandle[n];
            var pointers = new IntPtr[n];
            var xs = new float[n];
            var ys = new float[n];
            var ws = new float[n];
            var hs = new float[n];
            try
            {
                for (int i = 0; i < n; i++)
                {
                    valueBuffers[i] = System.Text.Encoding.UTF8.GetBytes(buttons[i].value + "\0");
                    handles[i] = System.Runtime.InteropServices.GCHandle.Alloc(valueBuffers[i], System.Runtime.InteropServices.GCHandleType.Pinned);
                    pointers[i] = handles[i].AddrOfPinnedObject();
                    xs[i] = buttons[i].x;
                    ys[i] = buttons[i].y;
                    ws[i] = buttons[i].w;
                    hs[i] = buttons[i].h;
                }
                var ptrHandle = System.Runtime.InteropServices.GCHandle.Alloc(pointers, System.Runtime.InteropServices.GCHandleType.Pinned);
                var xsH = System.Runtime.InteropServices.GCHandle.Alloc(xs, System.Runtime.InteropServices.GCHandleType.Pinned);
                var ysH = System.Runtime.InteropServices.GCHandle.Alloc(ys, System.Runtime.InteropServices.GCHandleType.Pinned);
                var wsH = System.Runtime.InteropServices.GCHandle.Alloc(ws, System.Runtime.InteropServices.GCHandleType.Pinned);
                var hsH = System.Runtime.InteropServices.GCHandle.Alloc(hs, System.Runtime.InteropServices.GCHandleType.Pinned);
                try
                {
                    NativeMethods.PdfPageBuilderRadioGroup(Handle, name,
                        (byte**)ptrHandle.AddrOfPinnedObject(),
                        (float*)xsH.AddrOfPinnedObject(),
                        (float*)ysH.AddrOfPinnedObject(),
                        (float*)wsH.AddrOfPinnedObject(),
                        (float*)hsH.AddrOfPinnedObject(),
                        (nuint)n, selected, out var ec);
                    ExceptionMapper.ThrowIfError(ec);
                }
                finally { ptrHandle.Free(); xsH.Free(); ysH.Free(); wsH.Free(); hsH.Free(); }
            }
            finally
            {
                for (int i = 0; i < n; i++)
                    if (handles[i].IsAllocated) handles[i].Free();
            }
            return this;
        }

        /// <summary>Add a clickable push button with a visible caption.</summary>
        public PageBuilder PushButton(string name, float x, float y, float w, float h, string caption)
        {
            ArgumentNullException.ThrowIfNull(name);
            ArgumentNullException.ThrowIfNull(caption);
            NativeMethods.PdfPageBuilderPushButton(Handle, name, x, y, w, h, caption, out var ec);
            ExceptionMapper.ThrowIfError(ec);
            return this;
        }

        // --- Low-level graphics primitives (PdfWriter exposure) -------------

        /// <summary>Draw a stroked rectangle outline (1pt black).</summary>
        public PageBuilder Rect(float x, float y, float w, float h)
        {
            NativeMethods.PdfPageBuilderRect(Handle, x, y, w, h, out var ec);
            ExceptionMapper.ThrowIfError(ec);
            return this;
        }

        /// <summary>Draw a filled rectangle in RGB colour (channels 0–1).</summary>
        public PageBuilder FilledRect(float x, float y, float w, float h, float r, float g, float b)
        {
            NativeMethods.PdfPageBuilderFilledRect(Handle, x, y, w, h, r, g, b, out var ec);
            ExceptionMapper.ThrowIfError(ec);
            return this;
        }

        /// <summary>Draw a line from (x1, y1) to (x2, y2) with 1pt black stroke.</summary>
        public PageBuilder Line(float x1, float y1, float x2, float y2)
        {
            NativeMethods.PdfPageBuilderLine(Handle, x1, y1, x2, y2, out var ec);
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
            ObjectDisposedException.ThrowIf(_done, this);
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
