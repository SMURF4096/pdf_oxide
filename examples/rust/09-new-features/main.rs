// v0.3.39 new-feature showcase.
//
// Exercises every major feature added in this release as a real user would:
//   1. StreamingTable with rowspan
//   2. PDF/UA accessible image (image_from_bytes_with_alt)
//   3. PDF/UA decorative image artifact (image_from_bytes_as_artifact)
//   4. save_to_bytes / open_from_bytes in-memory round-trip
//   5. CMS signing via PKCS#12 (requires --features signatures)
//
// Run:
//   cargo run --example showcase_new_features
//   cargo run --example showcase_new_features --features signatures

use pdf_oxide::{
    error::Result,
    geometry::Rect,
    writer::{CellAlign, DocumentBuilder, DocumentMetadata, StreamingColumn, StreamingTableConfig},
    PdfDocument,
};
use std::path::{Path, PathBuf};

#[cfg(feature = "signatures")]
use pdf_oxide::signatures::{sign_pdf_bytes, SignOptions, SigningCredentials};

fn main() -> Result<()> {
    let out_dir = PathBuf::from("target/new_features_demo");
    std::fs::create_dir_all(&out_dir)?;

    feature_streaming_table_rowspan(&out_dir)?;
    feature_pdf_ua_accessible_image(&out_dir)?;
    feature_save_to_bytes_roundtrip(&out_dir)?;

    #[cfg(feature = "signatures")]
    feature_pkcs12_signing(&out_dir)?;

    println!("All outputs written to {}", out_dir.display());
    Ok(())
}

// ── 1. StreamingTable with rowspan ───────────────────────────────────────────

fn feature_streaming_table_rowspan(out_dir: &Path) -> Result<()> {
    println!("Building streaming table with rowspan...");

    let cfg = StreamingTableConfig::new()
        .column(StreamingColumn::new("Category").width_pt(120.0))
        .column(StreamingColumn::new("Item").width_pt(160.0))
        .column(
            StreamingColumn::new("Notes")
                .width_pt(150.0)
                .align(CellAlign::Right),
        )
        .repeat_header(true)
        .max_rowspan(2);

    let mut builder =
        DocumentBuilder::new().metadata(DocumentMetadata::new().title("StreamingTable Demo"));
    {
        let mut tbl = builder
            .letter_page()
            .font("Helvetica", 10.0)
            .at(72.0, 700.0)
            .heading(1, "Product Catalogue")
            .at(72.0, 660.0)
            .streaming_table(cfg);
        tbl.push_row(|r| {
            r.span_cell("Fruits", 2); // spans row 1 + row 2
            r.cell("Apple");
            r.cell("crisp");
        })?;
        tbl.push_row(|r| {
            r.cell(""); // continuation cell for the span
            r.cell("Banana");
            r.cell("sweet");
        })?;
        tbl.push_row(|r| {
            r.cell("Vegetables");
            r.cell("Carrot");
            r.cell("earthy");
        })?;
        tbl.finish().done();
    }

    let bytes = builder.build()?;
    std::fs::write(out_dir.join("streaming_table_rowspan.pdf"), bytes)?;
    println!("  -> streaming_table_rowspan.pdf");
    Ok(())
}

// ── 2. PDF/UA accessible image ───────────────────────────────────────────────

fn feature_pdf_ua_accessible_image(out_dir: &Path) -> Result<()> {
    println!("Building PDF/UA document with accessible image...");

    // Minimal 1×1 white PNG (no external fixture needed).
    let white_png: &[u8] = &[
        0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00, 0x00, 0x90,
        0x77, 0x53, 0xde, 0x00, 0x00, 0x00, 0x0c, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9c, 0x63, 0xf8,
        0xff, 0xff, 0x3f, 0x00, 0x05, 0xfe, 0x02, 0xfe, 0x0d, 0xef, 0x46, 0xb8, 0x00, 0x00, 0x00,
        0x00, 0x49, 0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
    ];

    let mut builder = DocumentBuilder::new().metadata(
        DocumentMetadata::new()
            .title("Accessible PDF Demo")
            .tagged_pdf_ua1()
            .language("en-US"),
    );
    builder
        .a4_page()
        .font("Helvetica", 12.0)
        .at(72.0, 750.0)
        .heading(1, "Accessible document with images")
        .at(72.0, 720.0)
        .paragraph("The image below has descriptive alt text for screen readers.")
        .image_from_bytes_with_alt(
            white_png,
            Rect::new(72.0, 580.0, 100.0, 100.0),
            "A white placeholder image used for demonstration purposes",
        )?
        .at(72.0, 550.0)
        .paragraph("The logo below is purely decorative and marked as an artifact.")
        .image_from_bytes_as_artifact(white_png, Rect::new(72.0, 450.0, 60.0, 60.0))?
        .done();

    let bytes = builder.build()?;
    std::fs::write(out_dir.join("pdf_ua_accessible_images.pdf"), bytes)?;
    println!("  -> pdf_ua_accessible_images.pdf");
    Ok(())
}

// ── 3. save_to_bytes / open_from_bytes round-trip ────────────────────────────

fn feature_save_to_bytes_roundtrip(out_dir: &Path) -> Result<()> {
    println!("Demonstrating in-memory round-trip (build → bytes → open_from_bytes)...");

    let mut builder = DocumentBuilder::new();
    builder
        .letter_page()
        .font("Helvetica", 12.0)
        .at(72.0, 720.0)
        .heading(1, "In-Memory Round-Trip")
        .at(72.0, 690.0)
        .paragraph("This PDF was built in memory, never written to disk mid-way.")
        .done();

    let pdf_bytes = builder.build()?;

    // Re-open from bytes — no filesystem path involved.
    let doc = PdfDocument::from_bytes(pdf_bytes.clone())?;
    let text = doc.extract_all_text()?;
    println!("  Extracted {} chars from in-memory PDF", text.len());
    assert!(text.contains("In-Memory"), "round-trip text missing");

    std::fs::write(out_dir.join("save_to_bytes_roundtrip.pdf"), &pdf_bytes)?;
    println!("  -> save_to_bytes_roundtrip.pdf");
    Ok(())
}

// ── 4. PKCS#12 signing (requires --features signatures) ──────────────────────

#[cfg(feature = "signatures")]
fn feature_pkcs12_signing(out_dir: &Path) -> Result<()> {
    println!("Signing PDF with PKCS#12 certificate...");

    let p12_path = "tests/fixtures/test_signing.p12";
    if !std::path::Path::new(p12_path).exists() {
        println!("  SKIP: {} not found", p12_path);
        return Ok(());
    }

    let p12_data = std::fs::read(p12_path)?;
    let creds = SigningCredentials::from_pkcs12(&p12_data, "testpass")?;

    let mut builder = DocumentBuilder::new();
    builder
        .letter_page()
        .font("Helvetica", 12.0)
        .at(72.0, 720.0)
        .heading(1, "Signed Invoice")
        .at(72.0, 690.0)
        .paragraph("This document carries a CMS/PKCS#7 digital signature.")
        .done();
    let pdf_bytes = builder.build()?;

    let opts = SignOptions::default()
        .with_reason("Approved")
        .with_location("HQ");
    let signed = sign_pdf_bytes(&pdf_bytes, &creds, opts)?;

    std::fs::write(out_dir.join("signed_document.pdf"), &signed)?;
    println!("  -> signed_document.pdf ({} bytes)", signed.len());

    let content = String::from_utf8_lossy(&signed);
    assert!(content.contains("/ByteRange"), "signature ByteRange missing");
    println!("  Signature verified: /ByteRange present.");
    Ok(())
}
