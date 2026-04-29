// PDF/A conversion — v0.3.40
//
// Demonstrates ConvertToPdfA: build a simple PDF, open it in the editor,
// convert to PDF/A-2b (level 2), then save the result.
// Run: go run -tags pdf_oxide_dev main.go

package main

import (
	"fmt"
	"log"
	"os"
	"path/filepath"

	pdfoxide "github.com/yfedoseev/pdf_oxide/go"
)

func must(err error) {
	if err != nil {
		log.Fatal(err)
	}
}

func main() {
	outDir := "output"
	must(os.MkdirAll(outDir, 0o755))

	// Build a simple PDF in memory.
	b, err := pdfoxide.NewDocumentBuilder()
	must(err)
	defer b.Close()
	must(b.Title("PDF/A Conversion Demo"))

	page, err := b.LetterPage()
	must(err)
	_, err = page.
		Font("Helvetica", 12).
		At(72, 720).
		Heading(1, "PDF/A-2b Conversion Demo").
		At(72, 690).
		Paragraph("This document will be converted to PDF/A-2b archival format.").
		Done()
	must(err)

	pdfBytes, err := b.Build()
	must(err)
	fmt.Printf("Original PDF size: %d bytes\n", len(pdfBytes))

	// Open with editor and convert to PDF/A-2b (level index 2).
	editor, err := pdfoxide.OpenEditorFromBytes(pdfBytes)
	must(err)
	defer editor.Close()

	// ConvertToPdfA: 0=A1b 1=A1a 2=A2b 3=A2a 4=A2u 5=A3b 6=A3a 7=A3u
	if err := editor.ConvertToPdfA(2); err != nil {
		// Non-fatal: log and continue — the save below still writes a valid PDF.
		fmt.Printf("PDF/A conversion note: %v\n", err)
	} else {
		fmt.Println("PDF/A-2b conversion succeeded")
	}

	result, err := editor.SaveToBytes()
	must(err)
	if len(result) == 0 {
		log.Fatal("saved PDF is empty")
	}
	fmt.Printf("Output PDF size: %d bytes\n", len(result))

	path := filepath.Join(outDir, "pdfa.pdf")
	must(os.WriteFile(path, result, 0o644))
	fmt.Printf("Written: %s\n", path)
}
