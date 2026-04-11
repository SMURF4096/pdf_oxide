// Search for a term across all pages of a PDF and print matches.
// Run: cargo run --example search_text -- document.pdf "query"

use pdf_oxide::{PdfDocument, TextSearcher};
use std::env;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: search_text <file.pdf> <query>");
        process::exit(1);
    }

    let path = &args[1];
    let query = &args[2];

    let doc = PdfDocument::open(path).unwrap_or_else(|e| {
        eprintln!("Failed to open {}: {}", path, e);
        process::exit(1);
    });

    let pages = doc.page_count();
    println!("Searching for {:?} in {} ({} pages)...\n", query, path, pages);

    let searcher = TextSearcher::new(&doc);
    let mut total = 0;
    let mut pages_with_hits = 0;

    for i in 0..pages {
        let results = searcher.search_page(i, query).unwrap_or_default();
        if results.is_empty() {
            continue;
        }
        pages_with_hits += 1;
        println!("Page {}: {} match(es)", i + 1, results.len());
        for r in &results {
            println!("  - \"...{}...\"", r.context);
            total += 1;
        }
        println!();
    }

    println!("Found {} total matches across {} pages.", total, pages_with_hits);
}
