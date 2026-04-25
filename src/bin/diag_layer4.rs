//! Diagnostic: extract pages start..end-1, then extract target page.
//! Usage: diag_layer4 <pdf> <target_page> [prior_end]
//! prior_end defaults to target_page (extract 0..target before target).
//! When prior_end=0, extracts target page in isolation.
use pdf_oxide::PdfDocument;
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    let path = &args[1];
    let target: usize = args[2].parse().unwrap();
    let prior_end: usize = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(target);

    let doc = PdfDocument::open(path).unwrap();

    // Extract all prior pages first
    for i in 0..prior_end {
        eprintln!("[PAGE_START] page={i}");
        let _ = doc.extract_text(i);
        eprintln!("[PAGE_END] page={i}");
    }

    eprintln!("[PAGE_START] page={target}");
    let text = doc.extract_text(target).unwrap_or_default();
    eprintln!("[PAGE_END] page={target}");
    let nonws: usize = text.chars().filter(|c| !c.is_whitespace()).count();
    println!("prior_end={prior_end} target={target} nonws={nonws}");
    println!("text_start: {}", &text[..text.len().min(300)]);
}
