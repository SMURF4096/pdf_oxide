using System;
using System.IO;
using PdfOxide.Core;
using Xunit;

namespace PdfOxide.Tests
{
    /// <summary>
    /// Tests for <see cref="PdfDocument.RenderPage(int, RenderOptions)"/>,
    /// the full RenderOptions surface (DPI, background, annotations, JPEG
    /// quality) for the C# layer. Mirrors the Python surface.
    /// </summary>
    public class RenderOptionsTests
    {
        private static PdfDocument CreateTestDoc()
        {
            using var pdf = Pdf.FromMarkdown("# Render me\n\nBody.");
            var bytes = pdf.SaveToBytes();
            return PdfDocument.Open(bytes);
        }

        private static bool IsPng(byte[] b) =>
            b.Length >= 8 && b[0] == 0x89 && b[1] == 0x50 && b[2] == 0x4E && b[3] == 0x47;

        private static bool IsJpeg(byte[] b) =>
            b.Length >= 3 && b[0] == 0xFF && b[1] == 0xD8 && b[2] == 0xFF;

        [Fact]
        public void RenderPage_WithOptions_DefaultIsPng()
        {
            using var doc = CreateTestDoc();
            var bytes = doc.RenderPage(0, new RenderOptions());
            Assert.True(IsPng(bytes));
        }

        [Fact]
        public void RenderPage_WithOptions_JpegFormat_EmitsJpegMagic()
        {
            using var doc = CreateTestDoc();
            var bytes = doc.RenderPage(0, new RenderOptions { Format = RenderImageFormat.Jpeg });
            Assert.True(IsJpeg(bytes));
        }

        [Fact]
        public void RenderPage_WithOptions_HigherDpiProducesBiggerOutput()
        {
            using var doc = CreateTestDoc();
            var small = doc.RenderPage(0, new RenderOptions { Dpi = 72 });
            var large = doc.RenderPage(0, new RenderOptions { Dpi = 300 });
            Assert.True(IsPng(small) && IsPng(large));
            Assert.True(large.Length > small.Length);
        }

        [Fact]
        public void RenderPage_WithOptions_LowerJpegQualityIsSmaller()
        {
            using var doc = CreateTestDoc();
            var low = doc.RenderPage(0, new RenderOptions
            {
                Format = RenderImageFormat.Jpeg,
                JpegQuality = 20,
            });
            var high = doc.RenderPage(0, new RenderOptions
            {
                Format = RenderImageFormat.Jpeg,
                JpegQuality = 95,
            });
            Assert.True(IsJpeg(low) && IsJpeg(high));
            Assert.True(low.Length <= high.Length);
        }

        [Fact]
        public void RenderPage_WithOptions_TransparentBackground_OK()
        {
            using var doc = CreateTestDoc();
            var bytes = doc.RenderPage(0, new RenderOptions
            {
                TransparentBackground = true,
            });
            Assert.True(IsPng(bytes));
        }

        [Fact]
        public void RenderPage_WithOptions_CustomBackground_OK()
        {
            using var doc = CreateTestDoc();
            var bytes = doc.RenderPage(0, new RenderOptions
            {
                Background = (0f, 0f, 0f, 1f),
            });
            Assert.True(IsPng(bytes));
        }

        [Fact]
        public void RenderPage_WithOptions_RenderAnnotationsFalse_OK()
        {
            using var doc = CreateTestDoc();
            var bytes = doc.RenderPage(0, new RenderOptions
            {
                RenderAnnotations = false,
            });
            Assert.True(IsPng(bytes));
        }

        [Fact]
        public void RenderPage_WithOptions_Null_Throws()
        {
            using var doc = CreateTestDoc();
            Assert.Throws<ArgumentNullException>(() => doc.RenderPage(0, (RenderOptions)null!));
        }

        [Fact]
        public void RenderPage_WithOptions_InvalidDpi_Throws()
        {
            using var doc = CreateTestDoc();
            Assert.Throws<ArgumentException>(() =>
                doc.RenderPage(0, new RenderOptions { Dpi = 0 }));
        }

        [Fact]
        public void RenderPage_WithOptions_InvalidJpegQuality_Throws()
        {
            using var doc = CreateTestDoc();
            Assert.Throws<ArgumentException>(() =>
                doc.RenderPage(0, new RenderOptions
                {
                    Format = RenderImageFormat.Jpeg,
                    JpegQuality = 0,
                }));
        }
    }
}
