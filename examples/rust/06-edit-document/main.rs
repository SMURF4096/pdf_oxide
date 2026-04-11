// Open a PDF, modify metadata, delete a page, and save.
// Run: cargo run --example edit_document -- input.pdf output.pdf

use pdf_oxide::DocumentEditor;
use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: edit_document <input.pdf> <output.pdf>");
        std::process::exit(1);
    }
    let input = &args[1];
    let output = &args[2];

    let mut editor = DocumentEditor::open(input)?;
    println!("Opened: {}", input);

    editor.set_title("Edited Document");
    println!("Set title: \"Edited Document\"");

    editor.set_author("pdf_oxide");
    println!("Set author: \"pdf_oxide\"");

    // Delete page 2 (0-indexed = page index 1)
    editor.delete_page(1)?;
    println!("Deleted page 2");

    editor.save(output)?;
    println!("Saved: {}", output);

    Ok(())
}
