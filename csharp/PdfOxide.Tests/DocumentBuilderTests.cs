using System;
using System.IO;
using System.Linq;
using PdfOxide.Core;
using PdfOxide.Exceptions;
using Xunit;

namespace PdfOxide.Tests
{
    /// <summary>
    /// Integration tests for the #384 Phase 1-3 C# write-side API:
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

            using var doc = PdfDocument.Open(WriteTemp(bytes));
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
                var raw = File.ReadAllText(path);
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
            using var doc = PdfDocument.Open(WriteTemp(bytes));
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
            using var doc = PdfDocument.Open(WriteTemp(bytes));
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
                using var doc = PdfDocument.Open(WriteTemp(bytes));
                var text = doc.ExtractText(0);
                Assert.Contains("Hello", text);
                Assert.Contains("World", text);
            }
        }

        // --- Helpers ---------------------------------------------------------

        private static string WriteTemp(byte[] bytes)
        {
            var path = Path.Combine(Path.GetTempPath(), $"pdfoxide-tmp-{Guid.NewGuid():N}.pdf");
            File.WriteAllBytes(path, bytes);
            return path;
        }
    }
}
