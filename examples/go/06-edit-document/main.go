// Open a PDF, modify metadata, delete a page, and save.
// Run: go run main.go input.pdf output.pdf

package main

import (
	"fmt"
	"os"

	"github.com/yfedoseev/pdfoxide"
)

func main() {
	if len(os.Args) < 3 {
		fmt.Fprintln(os.Stderr, "Usage: go run main.go <input.pdf> <output.pdf>")
		os.Exit(1)
	}
	input, output := os.Args[1], os.Args[2]

	editor, err := pdfoxide.EditorOpen(input)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error: %v\n", err)
		os.Exit(1)
	}
	fmt.Printf("Opened: %s\n", input)

	pdfoxide.EditorSetTitle(editor, "Edited Document")
	fmt.Println(`Set title: "Edited Document"`)

	pdfoxide.EditorSetAuthor(editor, "pdf_oxide")
	fmt.Println(`Set author: "pdf_oxide"`)

	if err := pdfoxide.EditorDeletePage(editor, 1); err != nil {
		fmt.Fprintf(os.Stderr, "DeletePage error: %v\n", err)
		os.Exit(1)
	}
	fmt.Println("Deleted page 2")

	if err := pdfoxide.EditorSave(editor, output); err != nil {
		fmt.Fprintf(os.Stderr, "Save error: %v\n", err)
		os.Exit(1)
	}
	fmt.Printf("Saved: %s\n", output)
}
