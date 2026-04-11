// Create PDFs from Markdown, HTML, and plain text.
// Run: go run main.go

package main

import (
	"fmt"
	"log"

	"github.com/yfedoseev/pdfoxide"
)

func main() {
	fmt.Println("Creating PDFs...")

	// From Markdown
	markdown := `# Project Report

## Summary

This document was generated from **Markdown** using pdf_oxide.

- Fast rendering
- Clean typography
- Cross-platform
`
	pdf, err := pdfoxide.FromMarkdown(markdown)
	if err != nil {
		log.Fatalf("Markdown: %v", err)
	}
	pdfoxide.PdfSave(pdf, "from_markdown.pdf")
	fmt.Println("Saved: from_markdown.pdf")

	// From HTML
	html := `<html><body>
<h1>Invoice #1234</h1>
<p>Generated from <em>HTML</em> using pdf_oxide.</p>
<table><tr><th>Item</th><th>Price</th></tr>
<tr><td>Widget</td><td>$9.99</td></tr></table>
</body></html>`
	pdf, err = pdfoxide.FromHtml(html)
	if err != nil {
		log.Fatalf("HTML: %v", err)
	}
	pdfoxide.PdfSave(pdf, "from_html.pdf")
	fmt.Println("Saved: from_html.pdf")

	// From plain text
	text := "Hello, World!\n\nThis PDF was created from plain text using pdf_oxide."
	pdf, err = pdfoxide.FromText(text)
	if err != nil {
		log.Fatalf("Text: %v", err)
	}
	pdfoxide.PdfSave(pdf, "from_text.pdf")
	fmt.Println("Saved: from_text.pdf")

	fmt.Println("Done. 3 PDFs created.")
}
