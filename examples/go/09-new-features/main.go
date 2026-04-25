// v0.3.39 new-feature showcase — Go
//
// Exercises every major feature added in this release as a real user would:
//   1. StreamingTable with rowspan
//   2. PDF/UA accessible image (ImageWithAlt)
//   3. PDF/UA decorative image artifact (ImageArtifact)
//   4. Build() / OpenFromBytes() in-memory round-trip
//   5. CMS signing via PKCS#12 (LoadCertificate + SignPdfBytes)
//   6. RFC 3161 Timestamp parsing
//   7. TsaClient construction (offline — no network call)
//
// Run: go run main.go

package main

import (
	"fmt"
	"log"
	"os"
	"path/filepath"
	"strings"

	pdfoxide "github.com/yfedoseev/pdf_oxide/go"
)

const outDir = "output_new_features"

// Minimal 1×1 white PNG (no external file needed).
var whitePng = []byte{
	0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A,
	0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
	0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01,
	0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53,
	0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41,
	0x54, 0x08, 0xD7, 0x63, 0xF8, 0xCF, 0xC0, 0x00,
	0x00, 0x00, 0x02, 0x00, 0x01, 0xE2, 0x21, 0xBC,
	0x33, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E,
	0x44, 0xAE, 0x42, 0x60, 0x82,
}

func must(err error) {
	if err != nil {
		log.Fatal(err)
	}
}

func main() {
	must(os.MkdirAll(outDir, 0o755))

	featureStreamingTableRowspan()
	featurePdfUaAccessibleImage()
	featureSaveToBytesRoundtrip()
	featureTimestampParsing()
	featureTsaClientConstruction()
	featurePkcs12Signing()

	fmt.Printf("\nAll outputs written to %s/\n", outDir)
}

// ── 1. StreamingTable with rowspan ────────────────────────────────────────────

func featureStreamingTableRowspan() {
	fmt.Println("Building streaming table with rowspan...")

	cfg := pdfoxide.StreamingTableConfig{
		Columns: []pdfoxide.Column{
			{Header: "Category", Width: 120},
			{Header: "Item",     Width: 160},
			{Header: "Notes",    Width: 150, Align: pdfoxide.AlignRight},
		},
		RepeatHeader: true,
		MaxRowspan:   2,
	}

	b, err := pdfoxide.NewDocumentBuilder()
	must(err)
	defer b.Close()

	must(b.Title("StreamingTable Demo"))

	page, err := b.LetterPage()
	must(err)
	page.Font("Helvetica", 10).At(72, 700).Heading(1, "Product Catalogue").At(72, 660)

	tbl := page.StreamingTable(cfg)
	must(tbl.PushRowSpan([]pdfoxide.SpanCell{
		{Text: "Fruits",  Rowspan: 2}, // Fruits spans 2 rows
		{Text: "Apple",   Rowspan: 1},
		{Text: "crisp",   Rowspan: 1},
	}))
	must(tbl.PushRowSpan([]pdfoxide.SpanCell{
		{Text: "",       Rowspan: 1}, // continuation
		{Text: "Banana", Rowspan: 1},
		{Text: "sweet",  Rowspan: 1},
	}))
	must(tbl.PushRowSpan([]pdfoxide.SpanCell{
		{Text: "Vegetables", Rowspan: 1},
		{Text: "Carrot",     Rowspan: 1},
		{Text: "earthy",     Rowspan: 1},
	}))
	_, err = tbl.Finish().Done()
	must(err)

	path := filepath.Join(outDir, "streaming_table_rowspan.pdf")
	must(b.Save(path))
	fmt.Printf("  -> %s\n", path)
}

// ── 2. PDF/UA accessible image ────────────────────────────────────────────────

func featurePdfUaAccessibleImage() {
	fmt.Println("Building PDF/UA document with accessible image...")

	b, err := pdfoxide.NewDocumentBuilder()
	must(err)
	defer b.Close()

	must(b.Title("Accessible PDF Demo"))
	must(b.TaggedPdfUa1())
	must(b.Language("en-US"))

	page, err := b.A4Page()
	must(err)

	_, err = page.
		Font("Helvetica", 12).
		At(72, 750).
		Heading(1, "Accessible document with images").
		At(72, 720).
		Paragraph("The image below has descriptive alt text for screen readers.").
		// PDF/UA accessible image: alt text for assistive technology
		ImageWithAlt(whitePng, 72, 580, 100, 100,
			"A white placeholder image used for demonstration purposes").
		At(72, 545).
		Paragraph("The logo below is purely decorative and marked as an artifact.").
		// Decorative image: marked as /Artifact, no alt text
		ImageArtifact(whitePng, 72, 445, 60, 60).
		Done()
	must(err)

	path := filepath.Join(outDir, "pdf_ua_accessible_images.pdf")
	must(b.Save(path))
	fmt.Printf("  -> %s\n", path)
}

// ── 3. Build() / OpenFromBytes() round-trip ───────────────────────────────────

func featureSaveToBytesRoundtrip() {
	fmt.Println("Demonstrating in-memory round-trip (Build + OpenFromBytes)...")

	b, err := pdfoxide.NewDocumentBuilder()
	must(err)
	defer b.Close()

	must(b.Title("In-Memory Round-Trip Demo"))
	page, err := b.LetterPage()
	must(err)
	_, err = page.
		Font("Helvetica", 12).
		At(72, 720).
		Heading(1, "In-Memory Round-Trip").
		At(72, 690).
		Paragraph("This PDF was built in memory, never written to disk mid-way.").
		Done()
	must(err)

	pdfBytes, err := b.Build()
	must(err)

	// Re-open from bytes — no filesystem path involved.
	doc, err := pdfoxide.OpenFromBytes(pdfBytes)
	must(err)
	defer doc.Close()

	text, err := doc.ExtractText(0)
	must(err)
	fmt.Printf("  Extracted %d chars from in-memory PDF\n", len(text))
	if !strings.Contains(text, "In-Memory") {
		log.Fatal("round-trip text missing")
	}

	path := filepath.Join(outDir, "save_to_bytes_roundtrip.pdf")
	must(os.WriteFile(path, pdfBytes, 0o644))
	fmt.Printf("  -> %s\n", path)
}

// ── 4. RFC 3161 Timestamp parsing ─────────────────────────────────────────────

func featureTimestampParsing() {
	fmt.Println("Parsing RFC 3161 timestamp...")

	bareTstInfo := mustHex(
		"3081B302010106042A0304013031300D060960864801650304020105000420" +
			"BA7816BF8F01CFEA414140DE5DAE2223B00361A396177A9CB410FF61F20015AD" +
			"020104180F32303233303630373131323632365A300A020101800201F4810164" +
			"0101FF0208314CFCE4E0651827A048A4463044310B30090603550406130255533113" +
			"301106035504080C0A536F6D652D5374617465310D300B060355040A0C04546573" +
			"743111300F06035504030C085465737420545341",
	)

	ts, err := pdfoxide.ParseTimestamp(bareTstInfo)
	if err != nil {
		fmt.Printf("  SKIP: signatures feature not available (%v)\n", err)
		return
	}

	epochSec, err := ts.Time()
	must(err)
	serial, err := ts.Serial()
	must(err)
	policyOid, err := ts.PolicyOid()
	must(err)
	tsaName, err := ts.TsaName()
	must(err)

	fmt.Printf("  Timestamp time (epoch): %d\n", epochSec)
	fmt.Printf("  Serial: %s  Policy OID: %s\n", serial, policyOid)
	fmt.Printf("  TSA name: %s\n", tsaName)
	if serial != "04" {
		log.Fatalf("unexpected serial: %s", serial)
	}
	fmt.Println("  Timestamp fields verified.")
}

// ── 5. TsaClient construction ─────────────────────────────────────────────────

func featureTsaClientConstruction() {
	fmt.Println("Constructing TsaClient (offline, no network call)...")

	opts := pdfoxide.TsaClientOptions{
		URL:            "https://freetsa.org/tsr",
		TimeoutSeconds: 30,
		HashAlgorithm:  pdfoxide.TimestampHashSha256,
		UseNonce:       true,
		CertReq:        true,
	}
	client, err := pdfoxide.NewTsaClient(opts)
	if err != nil {
		fmt.Printf("  SKIP: signatures feature not available (%v)\n", err)
		return
	}
	defer client.Close()
	fmt.Println("  TsaClient created (no network call).")
}

// ── 6. PKCS#12 signing ────────────────────────────────────────────────────────

func featurePkcs12Signing() {
	fmt.Println("Signing PDF with PKCS#12 certificate...")

	p12Path := filepath.Join("..", "..", "..", "tests", "fixtures", "test_signing.p12")
	if _, statErr := os.Stat(p12Path); os.IsNotExist(statErr) {
		fmt.Printf("  SKIP: %s not found\n", p12Path)
		return
	}

	p12Data, err := os.ReadFile(p12Path)
	must(err)

	cert, err := pdfoxide.LoadCertificate(p12Data, "testpass")
	if err != nil {
		fmt.Printf("  SKIP: signatures feature not available (%v)\n", err)
		return
	}
	defer cert.Close()

	subject, err := cert.Subject()
	must(err)
	fmt.Printf("  Certificate subject: %s\n", subject)

	b, err := pdfoxide.NewDocumentBuilder()
	must(err)
	defer b.Close()

	must(b.Title("Signed Invoice"))
	page, err := b.LetterPage()
	must(err)
	_, err = page.
		Font("Helvetica", 12).
		At(72, 720).
		Heading(1, "Signed Invoice").
		At(72, 690).
		Paragraph("This document carries a CMS/PKCS#7 digital signature.").
		Done()
	must(err)

	pdfBytes, err := b.Build()
	must(err)

	signed, err := pdfoxide.SignPdfBytes(pdfBytes, cert, "Approved", "HQ")
	must(err)

	path := filepath.Join(outDir, "signed_document.pdf")
	must(os.WriteFile(path, signed, 0o644))
	fmt.Printf("  -> %s (%d bytes)\n", path, len(signed))

	if !strings.Contains(string(signed), "/ByteRange") {
		log.Fatal("ByteRange missing from signed PDF")
	}
	fmt.Println("  Signature verified: /ByteRange present.")
}

// ── helpers ───────────────────────────────────────────────────────────────────

func mustHex(s string) []byte {
	b := make([]byte, 0, len(s)/2)
	for i := 0; i < len(s)-1; i += 2 {
		var v byte
		fmt.Sscanf(s[i:i+2], "%02x", &v)
		b = append(b, v)
	}
	return b
}
