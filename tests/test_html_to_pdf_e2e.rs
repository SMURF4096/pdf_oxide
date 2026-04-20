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

/// B1 RED — three sibling `<p>` elements must each emit ALL their
/// words, not just the first one. The 10-doc cross-render corpus
/// demonstrated only ~20% of words survive; this test catches that
/// regression at unit-test granularity by counting words after
/// extraction.
#[test]
fn three_paragraphs_emit_all_words_in_order() {
    let extracted = build_and_extract(
        "<p>Alpha beta gamma delta epsilon zeta.</p>\
         <p>One two three four five six seven eight.</p>\
         <p>Red orange yellow green blue indigo violet.</p>",
        "",
    );
    let normalized: String = extracted.split_whitespace().collect::<Vec<_>>().join(" ");
    // Each paragraph's full content must survive.
    let p1_words = ["Alpha", "beta", "gamma", "delta", "epsilon", "zeta"];
    let p2_words = ["One", "two", "three", "four", "five", "six", "seven", "eight"];
    let p3_words = ["Red", "orange", "yellow", "green", "blue", "indigo", "violet"];
    for w in p1_words.iter().chain(p2_words.iter()).chain(p3_words.iter()) {
        assert!(
            normalized.contains(w),
            "missing word `{w}` from extracted text:\n{extracted}"
        );
    }
    // Order: the three paragraphs' anchor words must appear in order.
    let alpha = normalized.find("Alpha").unwrap_or(usize::MAX);
    let one = normalized.find("One").unwrap_or(usize::MAX);
    let red = normalized.find("Red").unwrap_or(usize::MAX);
    assert!(alpha < one, "Alpha must precede One in paragraph order");
    assert!(one < red, "One must precede Red in paragraph order");
}

/// B2 RED — a single `<p>` with ~50 words must round-trip ≥ 90 % of
/// its words. The cross-render harness showed ~20 % retention on
/// multi-line paragraphs (bodies after the first word lost their
/// position). This test exercises the inline formatter at moderate
/// length without involving sibling paragraphs.
#[test]
fn long_single_paragraph_keeps_all_words() {
    let words: Vec<String> = (1..=60).map(|i| format!("word{i}")).collect();
    let body = words.join(" ");
    let html = format!("<p>{body}</p>");
    let extracted = build_and_extract(&html, "");
    let normalized: String = extracted.split_whitespace().collect::<Vec<_>>().join(" ");
    let present = words.iter().filter(|w| normalized.contains(w.as_str())).count();
    let ratio = present as f32 / words.len() as f32;
    assert!(
        ratio >= 0.90,
        "long paragraph retained only {present}/{} words ({:.0}%): {extracted}",
        words.len(),
        ratio * 100.0
    );
}

/// B1 visual-positioning RED — beyond text content, the SPAN
/// Y-coordinates must reflect the document's logical paragraph order.
/// `extract_text` reads in stream order (so it can pass even when the
/// PDF is visually broken); a stronger test inspects bbox.y on
/// extracted spans and asserts each paragraph's first-word span sits
/// at a strictly lower y than the previous (PDF coordinates: y=0 is
/// page bottom, y grows up).
#[test]
fn three_paragraphs_have_decreasing_y_baselines() {
    let pdf = Pdf::from_html_css(
        "<p>Alpha first paragraph.</p>\
         <p>Beta second paragraph.</p>\
         <p>Gamma third paragraph.</p>",
        "",
        DEJAVU.to_vec(),
    )
    .expect("from_html_css");
    let mut doc = PdfDocument::from_bytes(pdf.into_bytes()).expect("re-open PDF");
    let spans = doc.extract_spans(0).expect("extract_spans");
    let pos = |needle: &str| -> Option<f32> {
        spans
            .iter()
            .find(|s| s.text.contains(needle))
            .map(|s| s.bbox.y)
    };
    let alpha_y = pos("Alpha").expect("Alpha span missing");
    let beta_y = pos("Beta").expect("Beta span missing");
    let gamma_y = pos("Gamma").expect("Gamma span missing");
    assert!(
        alpha_y > beta_y,
        "Alpha (y={alpha_y}) must sit ABOVE Beta (y={beta_y}) on the page (PDF y grows up)"
    );
    assert!(
        beta_y > gamma_y,
        "Beta (y={beta_y}) must sit ABOVE Gamma (y={gamma_y})"
    );
    // And: each paragraph's body words must share that paragraph's
    // baseline (within one line height), not be scattered across the
    // page at increasing x.
    let alpha_body_y = pos("first").expect("`first` span missing");
    assert!(
        (alpha_body_y - alpha_y).abs() < 30.0,
        "`first` (y={alpha_body_y}) must sit on Alpha's line (y={alpha_y})"
    );
}

#[test]
fn produces_valid_pdf_header() {
    let pdf = Pdf::from_html_css("<p>x</p>", "", DEJAVU.to_vec()).unwrap();
    let bytes = pdf.into_bytes();
    assert!(bytes.starts_with(b"%PDF-1.7"));
}

