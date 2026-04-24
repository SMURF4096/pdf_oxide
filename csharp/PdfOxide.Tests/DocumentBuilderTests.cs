using System;
using System.IO;
using System.Linq;
using PdfOxide.Core;
using PdfOxide.Exceptions;
using Xunit;

namespace PdfOxide.Tests
{
    /// <summary>
    /// Integration tests for the C# write-side API:
    /// <see cref="DocumentBuilder"/>, <see cref="PageBuilder"/>,
    /// <see cref="EmbeddedFont"/>, plus HTML+CSS hooks.
    /// </summary>
    /// <remarks>
    /// The DejaVuSans fixture ships at <c>tests/fixtures/fonts/DejaVuSans.ttf</c>
    /// — ~760 KB, covers Latin + Cyrillic + Greek. Each test that needs a
    /// font loads it via <see cref="FixtureFontPath"/> which walks up from
    /// the test working directory.
    /// </remarks>
    public class DocumentBuilderTests
    {
        private static string FixtureFontPath
        {
            get
            {
                // The test runner's working directory is something like
                // .../csharp/PdfOxide.Tests/bin/Release/net8.0/. Walk up
                // until we find the repo root (the one with the tests/
                // directory containing fonts/).
                var dir = AppContext.BaseDirectory;
                while (!string.IsNullOrEmpty(dir))
                {
                    var candidate = Path.Combine(dir, "tests", "fixtures", "fonts", "DejaVuSans.ttf");
                    if (File.Exists(candidate))
                        return candidate;
                    dir = Path.GetDirectoryName(dir);
                }
                throw new FileNotFoundException("Could not locate DejaVuSans.ttf fixture from "
                    + AppContext.BaseDirectory);
            }
        }

        [Fact]
        public void DocumentBuilder_Minimal_Ascii()
        {
            using var builder = DocumentBuilder.Create();
            builder.A4Page()
                .At(72, 720).Text("Hello, world.")
                .Done();
            var bytes = builder.Build();
            Assert.StartsWith("%PDF-", System.Text.Encoding.ASCII.GetString(bytes.Take(8).ToArray()));
            Assert.True(bytes.Length > 256);
        }

        [Fact]
        public void DocumentBuilder_Cjk_RoundTrip()
        {
            using var font = EmbeddedFont.FromFile(FixtureFontPath);
            using var builder = DocumentBuilder.Create()
                .RegisterEmbeddedFont("DejaVu", font);
            builder.A4Page()
                .Font("DejaVu", 12)
                .At(72, 720).Text("Привет, мир!")
                .At(72, 700).Text("Καλημέρα κόσμε")
                .Done();
            var bytes = builder.Build();

            using var tmp = WriteTemp(bytes);
            using var doc = PdfDocument.Open(tmp);
            var text = doc.ExtractText(0);
            Assert.Contains("Привет, мир!", text);
            Assert.Contains("Καλημέρα κόσμε", text);
        }

        [Fact]
        public void DocumentBuilder_Output_Is_Subsetted()
        {
            using var font = EmbeddedFont.FromFile(FixtureFontPath);
            using var builder = DocumentBuilder.Create()
                .RegisterEmbeddedFont("DejaVu", font);
            builder.A4Page()
                .Font("DejaVu", 12)
                .At(72, 700).Text("Hello world")
                .Done();
            var bytes = builder.Build();

            long faceSize = new FileInfo(FixtureFontPath).Length;
            Assert.True(bytes.Length * 10 < faceSize,
                $"Expected PDF ({bytes.Length}) to be >=10x smaller than the face ({faceSize})");
        }

        [Fact]
        public void DocumentBuilder_SaveEncrypted_Produces_Aes256_Dict()
        {
            var path = Path.Combine(Path.GetTempPath(), $"pdfoxide-enc-{Guid.NewGuid():N}.pdf");
            try
            {
                using (var builder = DocumentBuilder.Create())
                {
                    builder.A4Page()
                        .At(72, 720).Text("secret")
                        .Done();
                    builder.SaveEncrypted(path, "userpw", "ownerpw");
                }
                // PDFs are binary; decode as ASCII to scan for markers
                // without corrupting non-ASCII bytes via UTF-8 coercion.
                var raw = System.Text.Encoding.ASCII.GetString(File.ReadAllBytes(path));
                Assert.Contains("/Encrypt", raw);
                Assert.Contains("/V 5", raw);
            }
            finally { if (File.Exists(path)) File.Delete(path); }
        }

        [Fact]
        public void DocumentBuilder_ToBytesEncrypted()
        {
            using var builder = DocumentBuilder.Create();
            builder.A4Page().At(72, 720).Text("x").Done();
            var bytes = builder.ToBytesEncrypted("u", "o");
            var raw = System.Text.Encoding.ASCII.GetString(bytes);
            Assert.Contains("/Encrypt", raw);
            Assert.Contains("/V 5", raw);
        }

        [Fact]
        public void DocumentBuilder_Build_Consumes_Handle()
        {
            using var builder = DocumentBuilder.Create();
            builder.A4Page().At(72, 720).Text("x").Done();
            _ = builder.Build();
            Assert.Throws<ObjectDisposedException>(() => builder.Build());
        }

        [Fact]
        public void DocumentBuilder_Double_Open_Page_Throws()
        {
            using var builder = DocumentBuilder.Create();
            _ = builder.A4Page();
            Assert.ThrowsAny<PdfException>(() => builder.A4Page());
        }

        [Fact]
        public void EmbeddedFont_Consumed_After_Register()
        {
            using var font = EmbeddedFont.FromFile(FixtureFontPath);
            using var builder = DocumentBuilder.Create()
                .RegisterEmbeddedFont("A", font);
            // Font handle is consumed; a second RegisterEmbeddedFont on the
            // same font should throw because the wrapper reports disposed.
            Assert.Throws<ObjectDisposedException>(() =>
                builder.RegisterEmbeddedFont("B", font));
        }

        [Fact]
        public void DocumentBuilder_Multiple_Pages()
        {
            using var builder = DocumentBuilder.Create();
            builder.A4Page().At(72, 720).Text("page 1").Done();
            builder.A4Page().At(72, 720).Text("page 2").Done();
            builder.A4Page().At(72, 720).Text("page 3").Done();
            var bytes = builder.Build();
            using var tmp = WriteTemp(bytes);
            using var doc = PdfDocument.Open(tmp);
            Assert.Equal(3, doc.PageCount);
        }

        [Fact]
        public void DocumentBuilder_Annotations_Do_Not_Break_Extraction()
        {
            using var builder = DocumentBuilder.Create();
            builder.A4Page()
                .At(72, 720).Text("click me").LinkUrl("https://example.com")
                .At(72, 700).Text("important").Highlight(1.0f, 1.0f, 0.0f)
                .At(72, 680).Text("revisit").StickyNote("review").WatermarkDraft()
                .Done();
            var bytes = builder.Build();
            using var tmp = WriteTemp(bytes);
            using var doc = PdfDocument.Open(tmp);
            var text = doc.ExtractText(0);
            Assert.Contains("click me", text);
            Assert.Contains("important", text);
            Assert.Contains("revisit", text);
        }

        // --- Phase 2 — HTML+CSS pipeline --------------------------------------

        [Fact]
        public void Pdf_FromHtmlCss_Single_Font()
        {
            var fontBytes = File.ReadAllBytes(FixtureFontPath);
            var pdf = Pdf.FromHtmlCss(
                "<h1>Hello</h1><p>World</p>",
                "h1 { color: blue; font-size: 24pt }",
                fontBytes);
            using (pdf)
            {
                var bytes = pdf.SaveToBytes();
                using var tmp = WriteTemp(bytes);
                using var doc = PdfDocument.Open(tmp);
                var text = doc.ExtractText(0);
                Assert.Contains("Hello", text);
                Assert.Contains("World", text);
            }
        }

        // --- Helpers ---------------------------------------------------------

        /// <summary>
        /// Scoped temp-file wrapper — deletes the underlying file on Dispose.
        /// Used via <c>using var tmp = WriteTemp(bytes);</c> so PdfDocument
        /// tests don't leak temp files into the user's tmpdir across runs.
        /// Implicitly converts to <see cref="string"/> so existing
        /// <c>PdfDocument.Open(tmp)</c> call sites keep working.
        /// </summary>
        private readonly struct TempFile : IDisposable
        {
            public string Path { get; }

            public TempFile(string path) => Path = path;

            public static implicit operator string(TempFile t) => t.Path;

            public void Dispose()
            {
                try { File.Delete(Path); }
                catch { /* best-effort cleanup; OS will reap on reboot */ }
            }
        }

        private static TempFile WriteTemp(byte[] bytes)
        {
            var path = System.IO.Path.Combine(
                System.IO.Path.GetTempPath(),
                $"pdfoxide-tmp-{Guid.NewGuid():N}.pdf");
            File.WriteAllBytes(path, bytes);
            return new TempFile(path);
        }

        // ── Issue #401 regression tests ───────────────────────────────────────

        /// <summary>
        /// Verifies that <see cref="DocumentBuilder.SaveEncrypted"/> writes all
        /// font sub-objects (DescendantFonts, FontFile2, ToUnicode, FontDescriptor)
        /// into the encrypted output when an embedded TrueType font is used.
        ///
        /// Strategy: the embedded DejaVu font program is several KB even after
        /// subsetting. Without the fix (issue #401) those sub-objects are silently
        /// dropped and the file barely differs from a base-14-font encrypted PDF.
        /// With the fix the embedded-font file must be ≥10 KB larger.
        /// </summary>
        [Fact]
        public void DocumentBuilder_SaveEncrypted_EmbeddedFont_ContentObjects_Preserved()
        {
            // Baseline: simple text (base-14 font), encrypted.
            int simpleSize;
            {
                var path = Path.Combine(Path.GetTempPath(), $"pdfoxide-simple-enc-{Guid.NewGuid():N}.pdf");
                try
                {
                    using var builder = DocumentBuilder.Create();
                    builder.A4Page().At(72, 720).Text("Hello simple").Done();
                    builder.SaveEncrypted(path, "userpw", "ownerpw");
                    simpleSize = (int)new FileInfo(path).Length;
                }
                finally { if (File.Exists(path)) File.Delete(path); }
            }

            // Embedded-font PDF, encrypted.
            int ttfSize;
            {
                var path = Path.Combine(Path.GetTempPath(), $"pdfoxide-ttf-enc-{Guid.NewGuid():N}.pdf");
                try
                {
                    using var font = EmbeddedFont.FromFile(FixtureFontPath);
                    using var builder = DocumentBuilder.Create()
                        .RegisterEmbeddedFont("DejaVu", font);
                    builder.A4Page()
                        .Font("DejaVu", 12).At(72, 720).Text("Hello from embedded font")
                        .Done();
                    builder.SaveEncrypted(path, "userpw", "ownerpw");
                    var raw = System.Text.Encoding.ASCII.GetString(File.ReadAllBytes(path));
                    Assert.Contains("/Encrypt", raw);
                    ttfSize = (int)new FileInfo(path).Length;
                }
                finally { if (File.Exists(path)) File.Delete(path); }
            }

            // The embedded font program (FontFile2) adds ≥10 KB even when subsetted.
            var diff = ttfSize - simpleSize;
            Assert.True(
                diff >= 10_000,
                $"issue #401: embedded-font encrypted PDF ({ttfSize} B) is not " +
                $"substantially larger than simple encrypted PDF ({simpleSize} B); " +
                $"diff={diff} B — font sub-objects likely missing from encrypted output");
        }

        /// <summary>
        /// Verifies <see cref="DocumentBuilder.ToBytesEncrypted"/> preserves
        /// embedded font sub-objects in the encrypted byte output.
        /// </summary>
        [Fact]
        public void DocumentBuilder_ToBytesEncrypted_EmbeddedFont_ContentObjects_Preserved()
        {
            using var font = EmbeddedFont.FromFile(FixtureFontPath);
            using var builder = DocumentBuilder.Create()
                .RegisterEmbeddedFont("DejaVu", font);
            builder.A4Page()
                .Font("DejaVu", 12).At(72, 720)
                .Text("bytes encrypted with embedded font")
                .Done();

            var bytes = builder.ToBytesEncrypted("u", "o");
            var raw = System.Text.Encoding.ASCII.GetString(bytes);
            Assert.Contains("/Encrypt", raw);

            // Font program must be present: PDF must be >15 KB.
            Assert.True(
                bytes.Length > 15_000,
                $"issue #401: ToBytesEncrypted embedded-font result ({bytes.Length} B) " +
                "is too small; font sub-objects likely missing from encrypted output");
        }
    }
}
