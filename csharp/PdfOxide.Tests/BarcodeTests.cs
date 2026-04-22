using System;
using PdfOxide.Core;
using PdfOxide.Exceptions;
using Xunit;

namespace PdfOxide.Tests
{
    /// <summary>
    /// Tests for the <see cref="Barcode"/> class — #384 gap H.
    /// Rust FFI at <c>src/ffi.rs:1974</c> has shipped since well before
    /// v0.3.38; this surface is the first public C# exposure.
    /// </summary>
    public class BarcodeTests
    {
        [Fact]
        public void Generate_Code128_RoundTripsData()
        {
            using var bc = Barcode.Generate("HELLO-CODE128", BarcodeFormat.Code128);
            Assert.Equal(BarcodeFormat.Code128, bc.Format);
            Assert.Equal("HELLO-CODE128", bc.Data);
            Assert.True(bc.Confidence > 0f);
        }

        [Fact]
        public void Generate_Ean13_DefaultsToConfidence1()
        {
            using var bc = Barcode.Generate("1234567890128", BarcodeFormat.Ean13);
            Assert.Equal(BarcodeFormat.Ean13, bc.Format);
            Assert.Equal(1.0f, bc.Confidence);
        }

        [Fact]
        public void ToPng_EmitsPngMagic()
        {
            using var bc = Barcode.Generate("HELLO", BarcodeFormat.Code128);
            var png = bc.ToPng();
            Assert.True(png.Length >= 8);
            Assert.Equal(0x89, png[0]);
            Assert.Equal(0x50, png[1]);
            Assert.Equal(0x4E, png[2]);
            Assert.Equal(0x47, png[3]);
        }

        [Fact]
        public void Generate_Null_Throws()
        {
            Assert.Throws<ArgumentNullException>(() => Barcode.Generate(null!));
        }

        [Fact]
        public void Generate_Empty_Throws()
        {
            Assert.Throws<ArgumentException>(() => Barcode.Generate(""));
        }

        [Fact]
        public void Generate_InvalidSize_Throws()
        {
            Assert.Throws<ArgumentException>(() => Barcode.Generate("x", BarcodeFormat.Code128, 0));
        }

        [Fact]
        public void Dispose_IsIdempotent()
        {
            var bc = Barcode.Generate("x", BarcodeFormat.Code128);
            bc.Dispose();
            bc.Dispose();  // double-dispose must not throw
            Assert.Throws<ObjectDisposedException>(() => _ = bc.Data);
        }
    }
}
