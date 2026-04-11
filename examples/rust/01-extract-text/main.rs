// Extract text from every page of a PDF and print it.
// Run: cargo run --example extract_text -- document.pdf

use pdf_oxide::PdfDocument;
use std::env;
use std::process;

fn main() {
    let path = env::args().nth(1).unwrap_or_else(|| {
        eprintln!("Usage: extract_text <file.pdf>");
        process::exit(1);
    });

    let doc = PdfDocument::open(&path).unwrap_or_else(|e| {
        eprintln!("Failed to open {}: {}", path, e);
        process::exit(1);
    });

    let pages = doc.page_count();
    println!("Opened: {}", path);
    println!("Pages: {}\n", pages);

    for i in 0..pages {
        let text = doc.extract_text(i).unwrap_or_else(|e| {
            eprintln!("Error on page {}: {}", i + 1, e);
            String::new()
        });
        println!("--- Page {} ---", i + 1);
        println!("{}\n", text);
    }
}
