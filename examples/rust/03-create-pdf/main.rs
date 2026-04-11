// Create PDFs from Markdown, HTML, and plain text.
// Run: cargo run --example create_pdf

use pdf_oxide::Pdf;

fn main() {
    println!("Creating PDFs...");

    // From Markdown
    let markdown = r#"# Project Report

## Summary

This document was generated from **Markdown** using pdf_oxide.

- Fast rendering
- Clean typography
- Cross-platform
"#;
    let pdf = Pdf::from_markdown(markdown).expect("Failed to create from Markdown");
    pdf.save("from_markdown.pdf").expect("Failed to save");
    println!("Saved: from_markdown.pdf");

    // From HTML
    let html = r#"<html><body>
<h1>Invoice #1234</h1>
<p>Generated from <em>HTML</em> using pdf_oxide.</p>
<table><tr><th>Item</th><th>Price</th></tr>
<tr><td>Widget</td><td>$9.99</td></tr></table>
</body></html>"#;
    let pdf = Pdf::from_html(html).expect("Failed to create from HTML");
    pdf.save("from_html.pdf").expect("Failed to save");
    println!("Saved: from_html.pdf");

    // From plain text
    let text = "Hello, World!\n\nThis PDF was created from plain text using pdf_oxide.";
    let pdf = Pdf::from_text(text).expect("Failed to create from text");
    pdf.save("from_text.pdf").expect("Failed to save");
    println!("Saved: from_text.pdf");

    println!("Done. 3 PDFs created.");
}
