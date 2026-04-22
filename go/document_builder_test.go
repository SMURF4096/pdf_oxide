package pdfoxide

// Integration tests for the Go write-side API (#384 Phase 1-3). Mirrors the
// Python / C# / Rust-FFI test suites. Each test is self-contained; the
// DejaVuSans fixture ships at tests/fixtures/fonts/DejaVuSans.ttf relative
// to the repo root.

import (
	"bytes"
	"errors"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// fixtureFontPath walks up from the test binary's cwd until it finds the
// repo-root-relative fixture.
func fixtureFontPath(t *testing.T) string {
	t.Helper()
	dir, err := os.Getwd()
	if err != nil {
		t.Fatalf("getwd: %v", err)
	}
	for {
		candidate := filepath.Join(dir, "tests", "fixtures", "fonts", "DejaVuSans.ttf")
		if _, err := os.Stat(candidate); err == nil {
			return candidate
		}
		parent := filepath.Dir(dir)
		if parent == dir {
			t.Skipf("DejaVuSans.ttf fixture not found from %s", dir)
		}
		dir = parent
	}
}

func TestDocumentBuilderMinimalAscii(t *testing.T) {
	b, err := NewDocumentBuilder()
	if err != nil {
		t.Skipf("NewDocumentBuilder unavailable: %v", err)
	}
	defer b.Close()

	p, err := b.A4Page()
	if err != nil {
		t.Fatalf("A4Page: %v", err)
	}
	if _, err := p.At(72, 720).Text("Hello, world.").Done(); err != nil {
		t.Fatalf("chain: %v", err)
	}

	data, err := b.Build()
	if err != nil {
		t.Fatalf("Build: %v", err)
	}
	if !bytes.HasPrefix(data, []byte("%PDF-")) {
		t.Fatalf("output is not a PDF: % x", data[:8])
	}
	if len(data) < 256 {
		t.Fatalf("PDF suspiciously small: %d bytes", len(data))
	}
}

func TestDocumentBuilderCjkRoundTrip(t *testing.T) {
	fontPath := fixtureFontPath(t)
	font, err := EmbeddedFontFromFile(fontPath)
	if err != nil {
		t.Skipf("EmbeddedFontFromFile unavailable: %v", err)
	}

	b, err := NewDocumentBuilder()
	if err != nil {
		t.Fatalf("NewDocumentBuilder: %v", err)
	}
	defer b.Close()

	if err := b.RegisterEmbeddedFont("DejaVu", font); err != nil {
		t.Fatalf("RegisterEmbeddedFont: %v", err)
	}

	p, err := b.A4Page()
	if err != nil {
		t.Fatalf("A4Page: %v", err)
	}
	if _, err := p.
		Font("DejaVu", 12).
		At(72, 720).Text("Привет, мир!").
		At(72, 700).Text("Καλημέρα κόσμε").
		Done(); err != nil {
		t.Fatalf("chain: %v", err)
	}

	data, err := b.Build()
	if err != nil {
		t.Fatalf("Build: %v", err)
	}

	// Round-trip the output through PdfDocument.ExtractText.
	tmp := filepath.Join(t.TempDir(), "out.pdf")
	if err := os.WriteFile(tmp, data, 0o644); err != nil {
		t.Fatalf("write tmp: %v", err)
	}
	doc, err := Open(tmp)
	if err != nil {
		t.Fatalf("Open: %v", err)
	}
	defer doc.Close()
	text, err := doc.ExtractText(0)
	if err != nil {
		t.Fatalf("ExtractText: %v", err)
	}
	if !strings.Contains(text, "Привет, мир!") {
		t.Errorf("Cyrillic missing from extracted text: %q", text)
	}
	if !strings.Contains(text, "Καλημέρα κόσμε") {
		t.Errorf("Greek missing from extracted text: %q", text)
	}
}

func TestDocumentBuilderOutputIsSubsetted(t *testing.T) {
	fontPath := fixtureFontPath(t)
	faceBytes, err := os.ReadFile(fontPath)
	if err != nil {
		t.Fatalf("read font: %v", err)
	}
	font, err := EmbeddedFontFromFile(fontPath)
	if err != nil {
		t.Skipf("EmbeddedFontFromFile unavailable: %v", err)
	}
	b, err := NewDocumentBuilder()
	if err != nil {
		t.Fatalf("NewDocumentBuilder: %v", err)
	}
	defer b.Close()
	if err := b.RegisterEmbeddedFont("DejaVu", font); err != nil {
		t.Fatalf("RegisterEmbeddedFont: %v", err)
	}
	p, _ := b.A4Page()
	if _, err := p.Font("DejaVu", 12).At(72, 700).Text("Hello world").Done(); err != nil {
		t.Fatalf("chain: %v", err)
	}
	data, err := b.Build()
	if err != nil {
		t.Fatalf("Build: %v", err)
	}
	if len(data)*10 >= len(faceBytes) {
		t.Errorf("expected PDF (%d bytes) to be >= 10x smaller than face (%d bytes)", len(data), len(faceBytes))
	}
}

func TestDocumentBuilderSaveEncrypted(t *testing.T) {
	b, err := NewDocumentBuilder()
	if err != nil {
		t.Skipf("NewDocumentBuilder unavailable: %v", err)
	}
	defer b.Close()
	p, _ := b.A4Page()
	if _, err := p.At(72, 720).Text("secret").Done(); err != nil {
		t.Fatalf("chain: %v", err)
	}
	tmp := filepath.Join(t.TempDir(), "enc.pdf")
	if err := b.SaveEncrypted(tmp, "userpw", "ownerpw"); err != nil {
		t.Fatalf("SaveEncrypted: %v", err)
	}
	raw, err := os.ReadFile(tmp)
	if err != nil {
		t.Fatalf("read back: %v", err)
	}
	if !bytes.Contains(raw, []byte("/Encrypt")) {
		t.Errorf("missing /Encrypt dict")
	}
	if !bytes.Contains(raw, []byte("/V 5")) {
		t.Errorf("missing /V 5 (AES-256) marker")
	}
}

func TestDocumentBuilderToBytesEncrypted(t *testing.T) {
	b, err := NewDocumentBuilder()
	if err != nil {
		t.Skipf("NewDocumentBuilder unavailable: %v", err)
	}
	defer b.Close()
	p, _ := b.A4Page()
	if _, err := p.At(72, 720).Text("x").Done(); err != nil {
		t.Fatalf("chain: %v", err)
	}
	data, err := b.ToBytesEncrypted("u", "o")
	if err != nil {
		t.Fatalf("ToBytesEncrypted: %v", err)
	}
	if !bytes.Contains(data, []byte("/Encrypt")) {
		t.Errorf("missing /Encrypt")
	}
	if !bytes.Contains(data, []byte("/V 5")) {
		t.Errorf("missing /V 5")
	}
}

func TestDocumentBuilderConsumedAfterBuild(t *testing.T) {
	b, err := NewDocumentBuilder()
	if err != nil {
		t.Skipf("NewDocumentBuilder unavailable: %v", err)
	}
	p, _ := b.A4Page()
	if _, err := p.At(72, 720).Text("x").Done(); err != nil {
		t.Fatalf("chain: %v", err)
	}
	if _, err := b.Build(); err != nil {
		t.Fatalf("first build: %v", err)
	}
	if _, err := b.Build(); !errors.Is(err, ErrBuilderConsumed) {
		t.Errorf("expected ErrBuilderConsumed on second build, got %v", err)
	}
}

func TestDocumentBuilderDoubleOpenPage(t *testing.T) {
	b, err := NewDocumentBuilder()
	if err != nil {
		t.Skipf("NewDocumentBuilder unavailable: %v", err)
	}
	defer b.Close()
	if _, err := b.A4Page(); err != nil {
		t.Fatalf("A4Page: %v", err)
	}
	if _, err := b.A4Page(); !errors.Is(err, ErrBuilderHasOpenPage) {
		t.Errorf("expected ErrBuilderHasOpenPage, got %v", err)
	}
}

func TestEmbeddedFontConsumedAfterRegister(t *testing.T) {
	fontPath := fixtureFontPath(t)
	font, err := EmbeddedFontFromFile(fontPath)
	if err != nil {
		t.Skipf("EmbeddedFontFromFile unavailable: %v", err)
	}
	b, err := NewDocumentBuilder()
	if err != nil {
		t.Fatalf("NewDocumentBuilder: %v", err)
	}
	defer b.Close()
	if err := b.RegisterEmbeddedFont("A", font); err != nil {
		t.Fatalf("first register: %v", err)
	}
	if err := b.RegisterEmbeddedFont("B", font); !errors.Is(err, ErrFontConsumed) {
		t.Errorf("expected ErrFontConsumed on re-register, got %v", err)
	}
}

func TestDocumentBuilderMultiplePages(t *testing.T) {
	b, err := NewDocumentBuilder()
	if err != nil {
		t.Skipf("NewDocumentBuilder unavailable: %v", err)
	}
	defer b.Close()
	for _, s := range []string{"page one", "page two", "page three"} {
		p, err := b.A4Page()
		if err != nil {
			t.Fatalf("A4Page: %v", err)
		}
		if _, err := p.At(72, 720).Text(s).Done(); err != nil {
			t.Fatalf("chain: %v", err)
		}
	}
	data, err := b.Build()
	if err != nil {
		t.Fatalf("Build: %v", err)
	}
	tmp := filepath.Join(t.TempDir(), "multi.pdf")
	if err := os.WriteFile(tmp, data, 0o644); err != nil {
		t.Fatalf("write: %v", err)
	}
	doc, err := Open(tmp)
	if err != nil {
		t.Fatalf("Open: %v", err)
	}
	defer doc.Close()
	pages, err := doc.PageCount()
	if err != nil {
		t.Fatalf("PageCount: %v", err)
	}
	if pages != 3 {
		t.Errorf("expected 3 pages, got %d", pages)
	}
}

func TestDocumentBuilderAnnotationsDoNotBreakExtraction(t *testing.T) {
	b, err := NewDocumentBuilder()
	if err != nil {
		t.Skipf("NewDocumentBuilder unavailable: %v", err)
	}
	defer b.Close()
	p, _ := b.A4Page()
	if _, err := p.
		At(72, 720).Text("click me").LinkURL("https://example.com").
		At(72, 700).Text("important").Highlight(1.0, 1.0, 0.0).
		At(72, 680).Text("revisit").StickyNote("review").WatermarkDraft().
		Done(); err != nil {
		t.Fatalf("chain: %v", err)
	}
	data, err := b.Build()
	if err != nil {
		t.Fatalf("Build: %v", err)
	}
	tmp := filepath.Join(t.TempDir(), "annots.pdf")
	if err := os.WriteFile(tmp, data, 0o644); err != nil {
		t.Fatalf("write: %v", err)
	}
	doc, _ := Open(tmp)
	defer doc.Close()
	text, err := doc.ExtractText(0)
	if err != nil {
		t.Fatalf("ExtractText: %v", err)
	}
	for _, want := range []string{"click me", "important", "revisit"} {
		if !strings.Contains(text, want) {
			t.Errorf("missing %q in extracted text: %q", want, text)
		}
	}
}

func TestFromHTMLCSS(t *testing.T) {
	fontPath := fixtureFontPath(t)
	fontBytes, err := os.ReadFile(fontPath)
	if err != nil {
		t.Fatalf("read font: %v", err)
	}
	pdf, err := FromHTMLCSS(
		"<h1>Hello</h1><p>World</p>",
		"h1 { color: blue; font-size: 24pt }",
		fontBytes,
	)
	if err != nil {
		t.Skipf("FromHTMLCSS unavailable: %v", err)
	}
	defer pdf.Close()
	tmp := filepath.Join(t.TempDir(), "html.pdf")
	if err := pdf.Save(tmp); err != nil {
		t.Fatalf("Save: %v", err)
	}
	doc, _ := Open(tmp)
	defer doc.Close()
	text, _ := doc.ExtractText(0)
	if !strings.Contains(text, "Hello") {
		t.Errorf("missing 'Hello' in %q", text)
	}
	if !strings.Contains(text, "World") {
		t.Errorf("missing 'World' in %q", text)
	}
}
