// Extract words with bounding boxes and tables from a PDF page.
// Run: cargo run --example extract_structured -- document.pdf

use pdf_oxide::PdfDocument;
use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = env::args().nth(1).expect("Usage: extract_structured <file.pdf>");
    let doc = PdfDocument::open(&path)?;
    println!("Opened: {}", path);

    let page = 0;

    // Extract words with position data
    let words = doc.extract_words(page)?;
    println!("\n--- Words (page {}) ---", page + 1);
    for w in words.iter().take(20) {
        println!(
            "{:20} x={:<7.1} y={:<7.1} w={:<7.1} h={:<7.1} font={} size={:.1}",
            format!("\"{}\"", w.text),
            w.x, w.y, w.width, w.height, w.font_name, w.font_size
        );
    }
    if words.len() > 20 {
        println!("... ({} more words)", words.len() - 20);
    }

    // Extract tables
    let tables = doc.extract_tables(page)?;
    println!("\n--- Tables (page {}) ---", page + 1);
    if tables.is_empty() {
        println!("(no tables found)");
    }
    for (i, table) in tables.iter().enumerate() {
        println!("Table {}: {} rows x {} cols", i + 1, table.rows, table.cols);
        for r in 0..table.rows.min(5) {
            let row_str: Vec<String> = (0..table.cols.min(6))
                .map(|c| format!("[{},{}] \"{}\"", r, c, table.cells[r][c]))
                .collect();
            println!("  {}", row_str.join("  "));
        }
    }

    Ok(())
}
