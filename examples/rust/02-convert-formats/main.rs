// Convert PDF pages to Markdown, HTML, and plain text files.
// Run: cargo run --example convert_formats -- document.pdf

use pdf_oxide::PdfDocument;
use std::{env, fs, process};

fn main() {
    let path = env::args().nth(1).unwrap_or_else(|| {
        eprintln!("Usage: convert_formats <file.pdf>");
        process::exit(1);
    });

    let doc = PdfDocument::open(&path).unwrap_or_else(|e| {
        eprintln!("Failed to open {}: {}", path, e);
        process::exit(1);
    });

    fs::create_dir_all("output").expect("Failed to create output directory");
    let pages = doc.page_count();
    println!("Converting {} pages from {}...", pages, path);

    for i in 0..pages {
        let md = doc.to_markdown(i).unwrap();
        let html = doc.to_html(i).unwrap();
        let text = doc.extract_text(i).unwrap();

        let n = i + 1;
        fs::write(format!("output/page_{}.md", n), &md).unwrap();
        println!("Saved: output/page_{}.md", n);

        fs::write(format!("output/page_{}.html", n), &html).unwrap();
        println!("Saved: output/page_{}.html", n);

        fs::write(format!("output/page_{}.txt", n), &text).unwrap();
        println!("Saved: output/page_{}.txt", n);
    }

    println!("Done. Files written to output/");
}
