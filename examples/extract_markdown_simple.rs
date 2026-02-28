//! Simple markdown extraction tool for benchmarking

use pdf_oxide::converters::ConversionOptions;
use pdf_oxide::document::PdfDocument;
use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <pdf_file>", args[0]);
        std::process::exit(1);
    }

    let pdf_path = &args[1];
    let mut doc = PdfDocument::open(pdf_path)?;
    let page_count = doc.page_count()?;
    let options = ConversionOptions {
        include_images: false,
        ..ConversionOptions::default()
    };

    for page_idx in 0..page_count {
        let md = doc.to_markdown(page_idx, &options)?;
        println!("{}", md);
    }

    Ok(())
}
