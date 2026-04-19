//! End-to-end tests for `Pdf::from_html_css` — the v0.3.35
//! HTML+CSS→PDF pipeline (issue #248).
//!
//! Walks the full path: HTML parse → cascade → box tree → Taffy
//! layout → paginate → paint → PdfWriter → re-open via PdfDocument →
//! extract_text round-trip.

use pdf_oxide::api::Pdf;
use pdf_oxide::PdfDocument;

const DEJAVU: &[u8] = include_bytes!("fixtures/fonts/DejaVuSans.ttf");

fn build_and_extract(html: &str, css: &str) -> String {
    let pdf = Pdf::from_html_css(html, css, DEJAVU.to_vec()).expect("from_html_css");
    let bytes = pdf.into_bytes();
    let mut doc = PdfDocument::from_bytes(bytes).expect("re-open PDF");
    let pages = doc.page_count().expect("page count");
    let mut out = String::new();
    for i in 0..pages {
        out.push_str(&doc.extract_text(i).expect("extract_text"));
        out.push('\n');
    }
    out
}

#[test]
fn simple_paragraph_round_trips() {
    let extracted = build_and_extract("<p>Hello, world!</p>", "");
    assert!(
        extracted.contains("Hello, world!"),
        "expected 'Hello, world!' in: {extracted:?}"
    );
}

#[test]
fn multi_paragraph_round_trips() {
    let extracted = build_and_extract(
        "<p>First paragraph.</p><p>Second paragraph.</p>",
        "",
    );
    assert!(extracted.contains("First paragraph."));
    assert!(extracted.contains("Second paragraph."));
}

#[test]
fn nested_html_round_trips() {
    let extracted = build_and_extract(
        "<div><h1>Title</h1><p>Body text here.</p></div>",
        "",
    );
    assert!(extracted.contains("Title"));
    assert!(extracted.contains("Body text here."));
}

#[test]
fn css_styling_does_not_lose_text() {
    let extracted = build_and_extract(
        "<h1>Header</h1><p>Body.</p>",
        "h1 { color: blue; font-size: 24pt } p { color: gray }",
    );
    assert!(extracted.contains("Header"));
    assert!(extracted.contains("Body"));
}

#[test]
fn unicode_round_trips() {
    let extracted = build_and_extract(
        "<p>café Привет ❤</p>",
        "",
    );
    assert!(extracted.contains("café"));
    assert!(extracted.contains("Привет"));
}

#[test]
fn produces_valid_pdf_header() {
    let pdf = Pdf::from_html_css("<p>x</p>", "", DEJAVU.to_vec()).unwrap();
    let bytes = pdf.into_bytes();
    assert!(bytes.starts_with(b"%PDF-1.7"));
}

