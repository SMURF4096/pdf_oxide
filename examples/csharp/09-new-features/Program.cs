// v0.3.39 new-feature showcase — C#
//
// Exercises every major feature added in this release as a real user would:
//   1. StreamingTable with rowspan
//   2. PDF/UA accessible image (ImageWithAlt)
//   3. PDF/UA decorative image artifact (ImageArtifact)
//   4. Build() / PdfDocument.Open(bytes) in-memory round-trip
//   5. CMS signing via PKCS#12 (Certificate.Load + SignPdfBytes)
//   6. RFC 3161 Timestamp parsing
//   7. TsaClient construction (offline — no network call)
//
// Run: dotnet run

using System;
using System.IO;
using PdfOxide.Core;
using PdfOxide.Exceptions;

const string OutDir = "output_new_features";
Directory.CreateDirectory(OutDir);

// Minimal 1×1 white PNG (no external file needed).
var whitePng = new byte[]
{
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A,
    0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01,
    0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53,
    0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41,
    0x54, 0x78, 0x9C, 0x63, 0xF8, 0xFF, 0xFF, 0x3F,
    0x00, 0x05, 0xFE, 0x02, 0xFE, 0x0D, 0xEF, 0x46,
    0xB8, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E,
    0x44, 0xAE, 0x42, 0x60, 0x82,
};

FeatureStreamingTableRowspan();
FeaturePdfUaAccessibleImage();
FeatureSaveToBytesRoundtrip();
FeatureTimestampParsing();
FeatureTsaClientConstruction();
FeaturePkcs12Signing();

Console.WriteLine($"\nAll outputs written to {OutDir}/");

// ── 1. StreamingTable with rowspan ────────────────────────────────────────────

void FeatureStreamingTableRowspan()
{
    Console.WriteLine("Building streaming table with rowspan...");

    var columns = new[]
    {
        new Column("Category", 120),
        new Column("Item",     160),
        new Column("Notes",    150, Alignment.Right),
    };

    using var builder = DocumentBuilder.Create();
    builder.Title("StreamingTable Demo");
    var page = builder.LetterPage();
    page.Font("Helvetica", 10).At(72, 700).Heading(1, "Product Catalogue").At(72, 660);

    using (var tbl = page.StreamingTable(columns, repeatHeader: true, maxRowspan: 2))
    {
        tbl.AddRowSpan(("Fruits", 2), ("Apple", 1),   ("crisp",  1));  // Fruits spans 2 rows
        tbl.AddRowSpan(("",       1), ("Banana", 1),  ("sweet",  1));  // continuation
        tbl.AddRowSpan(("Vegetables", 1), ("Carrot", 1), ("earthy", 1));
        tbl.Build().Done(); // finalise table then close the page
    }

    var path = Path.Combine(OutDir, "streaming_table_rowspan.pdf");
    builder.Save(path);
    Console.WriteLine($"  -> {path}");
}

// ── 2. PDF/UA accessible image ────────────────────────────────────────────────

void FeaturePdfUaAccessibleImage()
{
    Console.WriteLine("Building PDF/UA document with accessible image...");

    using var builder = DocumentBuilder.Create();
    builder
        .Title("Accessible PDF Demo")
        .TaggedPdfUa1()
        .Language("en-US");

    var page = builder.A4Page();
    page
        .Font("Helvetica", 12)
        .At(72, 750)
        .Heading(1, "Accessible document with images")
        .At(72, 720)
        .Paragraph("The image below has descriptive alt text for screen readers.")
        // PDF/UA accessible image: alt text for assistive technology
        .ImageWithAlt(whitePng, 72, 580, 100, 100,
                      "A white placeholder image used for demonstration purposes")
        .At(72, 545)
        .Paragraph("The logo below is purely decorative and marked as an artifact.")
        // Decorative image: marked as /Artifact, no alt text
        .ImageArtifact(whitePng, 72, 445, 60, 60)
        .Done();

    var path = Path.Combine(OutDir, "pdf_ua_accessible_images.pdf");
    builder.Save(path);
    Console.WriteLine($"  -> {path}");
}

// ── 3. Build() / PdfDocument.Open(bytes) round-trip ──────────────────────────

void FeatureSaveToBytesRoundtrip()
{
    Console.WriteLine("Demonstrating in-memory round-trip (Build + PdfDocument.Open(bytes))...");

    using var builder = DocumentBuilder.Create();
    builder.Title("In-Memory Round-Trip Demo");
    builder.LetterPage()
        .Font("Helvetica", 12)
        .At(72, 720)
        .Heading(1, "In-Memory Round-Trip")
        .At(72, 690)
        .Paragraph("This PDF was built in memory, never written to disk mid-way.")
        .Done();

    byte[] pdfBytes = builder.Build();

    // Re-open from bytes — no filesystem path involved.
    using var doc = PdfDocument.Open(pdfBytes);
    string text = doc.ExtractText(0);
    Console.WriteLine($"  Extracted {text.Length} chars from in-memory PDF");
    if (!text.Contains("In-Memory"))
        throw new InvalidOperationException("round-trip text missing");

    var path = Path.Combine(OutDir, "save_to_bytes_roundtrip.pdf");
    File.WriteAllBytes(path, pdfBytes);
    Console.WriteLine($"  -> {path}");
}

// ── 4. RFC 3161 Timestamp parsing ─────────────────────────────────────────────

void FeatureTimestampParsing()
{
    Console.WriteLine("Parsing RFC 3161 timestamp...");

    var bareTstInfo = Convert.FromHexString(
        "3081B302010106042A0304013031300D060960864801650304020105000420" +
        "BA7816BF8F01CFEA414140DE5DAE2223B00361A396177A9CB410FF61F20015AD" +
        "020104180F32303233303630373131323632365A300A020101800201F4810164" +
        "0101FF0208314CFCE4E0651827A048A4463044310B30090603550406130255533113" +
        "301106035504080C0A536F6D652D5374617465310D300B060355040A0C04546573" +
        "743111300F06035504030C085465737420545341");

    try
    {
        using var ts = Timestamp.Parse(bareTstInfo);
        Console.WriteLine($"  Timestamp time (epoch): {ts.Time.ToUnixTimeSeconds()}");
        Console.WriteLine($"  Serial: {ts.Serial}  Policy OID: {ts.PolicyOid}");
        Console.WriteLine($"  TSA name: {ts.TsaName}");
        if (ts.Serial != "04")
            throw new InvalidOperationException($"unexpected serial: {ts.Serial}");
        if (ts.PolicyOid != "1.2.3.4.1")
            throw new InvalidOperationException($"unexpected policy OID: {ts.PolicyOid}");
        Console.WriteLine("  Timestamp fields verified.");
    }
    catch (UnsupportedFeatureException)
    {
        Console.WriteLine("  SKIP: signatures feature not compiled in.");
    }
}

// ── 5. TsaClient construction ─────────────────────────────────────────────────

void FeatureTsaClientConstruction()
{
    Console.WriteLine("Constructing TsaClient (offline, no network call)...");
    try
    {
        using var client = TsaClient.Create(new TsaClientOptions
        {
            Url            = "https://freetsa.org/tsr",
            TimeoutSeconds = 30,
            HashAlgorithm  = TimestampHashAlgorithm.Sha256,
            UseNonce       = true,
            CertReq        = true,
        });
        Console.WriteLine($"  TsaClient created: {client}");
    }
    catch (UnsupportedFeatureException)
    {
        Console.WriteLine("  SKIP: signatures feature not compiled in.");
    }
}

// ── 6. PKCS#12 signing ────────────────────────────────────────────────────────

void FeaturePkcs12Signing()
{
    Console.WriteLine("Signing PDF with PKCS#12 certificate...");

    var p12Path = Path.Combine(
        AppContext.BaseDirectory, "..", "..", "..", "..", "..",
        "tests", "fixtures", "test_signing.p12");
    if (!File.Exists(p12Path))
    {
        Console.WriteLine($"  SKIP: {p12Path} not found");
        return;
    }

    try
    {
        var p12Data = File.ReadAllBytes(p12Path);
        using var cert = Certificate.Load(p12Data, "testpass");
        Console.WriteLine($"  Certificate subject: {cert.Subject}");

        using var builder = DocumentBuilder.Create();
        builder.Title("Signed Invoice");
        builder.LetterPage()
            .Font("Helvetica", 12)
            .At(72, 720)
            .Heading(1, "Signed Invoice")
            .At(72, 690)
            .Paragraph("This document carries a CMS/PKCS#7 digital signature.")
            .Done();
        byte[] pdfBytes = builder.Build();

        byte[] signed = cert.SignPdfBytes(pdfBytes, reason: "Approved", location: "HQ");

        var path = Path.Combine(OutDir, "signed_document.pdf");
        File.WriteAllBytes(path, signed);
        Console.WriteLine($"  -> {path} ({signed.Length} bytes)");

        var content = System.Text.Encoding.Latin1.GetString(signed);
        if (!content.Contains("/ByteRange"))
            throw new InvalidOperationException("ByteRange missing from signed PDF");
        Console.WriteLine("  Signature verified: /ByteRange present.");
    }
    catch (UnsupportedFeatureException)
    {
        Console.WriteLine("  SKIP: signatures feature not compiled in.");
    }
}
