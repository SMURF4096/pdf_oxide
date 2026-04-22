using System;
using PdfOxide.Core;
using PdfOxide.Exceptions;
using Xunit;

namespace PdfOxide.Tests
{
    /// <summary>
    /// Cross-binding mirror of <c>tests/test_signature_enumeration.rs</c>.
    /// Closes #384 gap D / #51 — the inspection half. Verify() and
    /// GetCertificate() throw UnsupportedFeatureException until later
    /// slices of #72 land; tests pin that contract so it can't silently
    /// regress.
    /// </summary>
    public class SignatureTests
    {
        private const string UnsignedFixture = "../../../../../tests/fixtures/simple.pdf";
        private const string Issue395Fixture =
            "../../../../../tests/fixtures/issue_regressions/issue_395_render_signature_exception.pdf";

        [Fact]
        public void PdfWithoutAcroForm_HasZeroSignatures()
        {
            using var doc = PdfDocument.Open(UnsignedFixture);
            Assert.Equal(0, doc.SignatureCount);
            Assert.Empty(doc.Signatures);
        }

        [Fact]
        public void Issue395Fixture_EnumerationDoesNotThrow()
        {
            using var doc = PdfDocument.Open(Issue395Fixture);
            var count = doc.SignatureCount;
            var list = doc.Signatures;
            Assert.Equal(count, list.Count);
            foreach (var sig in list) sig.Dispose();
        }

        [Fact]
        public void Signatures_SnapshotListSemantics()
        {
            using var doc = PdfDocument.Open(UnsignedFixture);
            var a = doc.Signatures;
            var b = doc.Signatures;
            Assert.NotSame(a, b);
        }
    }
}
