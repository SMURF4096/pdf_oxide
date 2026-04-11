// Process multiple PDFs concurrently using rayon.
// Run: cargo run --example batch_processing -- *.pdf

use pdf_oxide::PdfDocument;
use rayon::prelude::*;
use std::env;
use std::time::Instant;

fn main() {
    let paths: Vec<String> = env::args().skip(1).collect();
    if paths.is_empty() {
        eprintln!("Usage: batch_processing <file1.pdf> <file2.pdf> ...");
        std::process::exit(1);
    }

    println!("Processing {} PDFs concurrently...", paths.len());
    let start = Instant::now();

    let results: Vec<_> = paths
        .par_iter()
        .map(|path| {
            let doc = match PdfDocument::open(path) {
                Ok(d) => d,
                Err(e) => return format!("[{}] ERROR: {}", path, e),
            };
            let pages = doc.page_count();
            let mut total_words = 0;
            let mut total_tables = 0;
            for p in 0..pages {
                if let Ok(words) = doc.extract_words(p) {
                    total_words += words.len();
                }
                if let Ok(tables) = doc.extract_tables(p) {
                    total_tables += tables.len();
                }
            }
            format!(
                "[{}]\tpages={}\twords={}\ttables={}",
                path, pages, total_words, total_tables
            )
        })
        .collect();

    for r in &results {
        println!("{}", r);
    }
    println!("\nDone: {} files processed in {:.2}s", paths.len(), start.elapsed().as_secs_f64());
}
