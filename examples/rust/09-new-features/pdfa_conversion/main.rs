// PDF/A conversion: validate → convert → validate — v0.3.41
//
// Demonstrates the full archival pipeline:
//   1. Build a PDF in memory
//   2. Validate PDF/A-2b conformance (expect errors before conversion)
//   3. Convert to PDF/A-2b in-place
//   4. Validate again (expect compliant or fewer errors)
//
// Run: cargo run --example showcase_pdfa_conversion

use pdf_oxide::{
    compliance::{convert_to_pdf_a, validate_pdf_a, PdfALevel},
    error::Result,
    writer::DocumentBuilder,
    PdfDocument,
};
use std::path::PathBuf;

fn main() -> Result<()> {
    let out_dir = PathBuf::from("target/examples_output/pdfa_conversion");
    std::fs::create_dir_all(&out_dir)?;

    let mut builder = DocumentBuilder::new();
    builder
        .letter_page()
        .font("Helvetica", 12.0)
        .at(72.0, 720.0)
        .heading(1, "PDF/A-2b Conversion Demo")
        .at(72.0, 690.0)
        .paragraph("This document will be converted to PDF/A-2b archival format.")
        .done();

    let pdf_bytes = builder.build()?;
    println!("Original PDF size: {} bytes", pdf_bytes.len());

    // Step 1: validate before conversion.
    let mut doc = PdfDocument::from_bytes(pdf_bytes.clone())?;
    let pre = validate_pdf_a(&mut doc, PdfALevel::A2b)?;
    println!("Before conversion — compliant: {}, errors: {}", pre.is_compliant, pre.errors.len());

    // Step 2: convert to PDF/A-2b.
    let result = convert_to_pdf_a(&mut doc, PdfALevel::A2b)?;
    println!("Conversion success: {}, actions: {}", result.success, result.actions.len());
    for action in &result.actions {
        println!("  - {:?}", action.action_type);
    }

    // Step 3: validate after conversion.
    let post = validate_pdf_a(&mut doc, PdfALevel::A2b)?;
    println!("After conversion  — compliant: {}, errors: {}", post.is_compliant, post.errors.len());

    let out_path = out_dir.join("pdfa_converted.pdf");
    std::fs::write(&out_path, &doc.source_bytes)?;
    println!("Written: {}", out_path.display());
    Ok(())
}
