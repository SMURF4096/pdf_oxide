using System;
using PdfOxide.Core;
using PdfOxide.Exceptions;
using Xunit;

namespace PdfOxide.Tests
{
    /// <summary>
    /// Cross-binding mirror of <c>tests/test_signature_enumeration.rs</c>.
    /// Closes #384 gap D / #51 — the inspection half. GetCertificate()
    /// and Verify() are now wired to the Rust-core CMS path; the
    /// cryptographic round-trip for Verify()/VerifyDetached() is
    /// covered by <c>tests/test_cms_verify_round_trip.rs</c> on the
    /// Rust side and does not yet have a C# integration fixture
    /// (a signed-PDF golden file is still to land as part of #77).
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
