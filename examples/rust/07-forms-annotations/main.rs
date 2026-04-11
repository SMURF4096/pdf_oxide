// Extract form fields and annotations from a PDF.
// Run: cargo run --example forms_annotations -- form.pdf

use pdf_oxide::PdfDocument;
use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = env::args().nth(1).expect("Usage: forms_annotations <file.pdf>");
    let doc = PdfDocument::open(&path)?;
    println!("Opened: {}", path);

    let page_count = doc.page_count();
    for page in 0..page_count {
        // Form fields
        let fields = doc.get_form_fields(page)?;
        if !fields.is_empty() {
            println!("\n--- Form Fields (page {}) ---", page + 1);
            for f in &fields {
                println!(
                    "  Name: {:<20} Type: {:<12} Value: {:<16} Required: {}",
                    format!("\"{}\"", f.name),
                    format!("{:?}", f.field_type),
                    format!("\"{}\"", f.value),
                    f.required
                );
            }
        }

        // Annotations
        let annotations = doc.get_annotations(page)?;
        if !annotations.is_empty() {
            println!("\n--- Annotations (page {}) ---", page + 1);
            for a in &annotations {
                println!(
                    "  Type: {:<14} Page: {}   Contents: \"{}\"",
                    format!("{:?}", a.annotation_type),
                    page + 1,
                    a.contents.as_deref().unwrap_or("")
                );
            }
        }
    }

    Ok(())
}
