//! Feature regression tests for v0.3.10.
//!
//! Tests for issues #167 (batch processing), #151 (open_from_bytes),
//! #158 (OCR page type detection), and #157 (table detection).

use pdf_oxide::document::PdfDocument;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

// ===========================================================================
// Issue #167 — Batch Processing
// ===========================================================================

use pdf_oxide::batch::{BatchProcessor, BatchSummary};

/// Process multiple valid PDFs → all succeed.
#[test]
fn test_batch_multiple_valid_files() {
    let processor = BatchProcessor::new();
    let results = processor.extract_text_from_files(&[
        Path::new("tests/fixtures/simple.pdf"),
        Path::new("tests/fixtures/outline.pdf"),
    ]);

    assert_eq!(results.len(), 2);
    for r in &results {
        assert!(
            r.text.is_ok(),
            "File {} should succeed: {:?}",
            r.path.display(),
            r.text.as_ref().err()
        );
    }
}

/// Mix of valid and non-existent file → one Ok, one Err, batch continues.
#[test]
fn test_batch_mixed_valid_and_invalid() {
    let processor = BatchProcessor::new();
    let results = processor.extract_text_from_files(&[
        Path::new("tests/fixtures/simple.pdf"),
        Path::new("/nonexistent/fake.pdf"),
    ]);

    assert_eq!(results.len(), 2, "Should have results for both files");

    let ok_count = results.iter().filter(|r| r.text.is_ok()).count();
    let err_count = results.iter().filter(|r| r.text.is_err()).count();

    assert_eq!(ok_count, 1, "One file should succeed");
    assert_eq!(err_count, 1, "One file should fail");
}

/// Progress callback is invoked the correct number of times.
#[test]
fn test_batch_progress_callback_invoked() {
    let call_count = Arc::new(AtomicUsize::new(0));
    let count_clone = Arc::clone(&call_count);

    let processor = BatchProcessor::new().with_progress(Box::new(move |completed, total| {
        count_clone.fetch_add(1, Ordering::Relaxed);
        assert!(completed <= total, "completed should not exceed total");
        assert_eq!(total, 2, "total should be 2");
    }));

    let _results = processor.extract_text_from_files(&[
        Path::new("tests/fixtures/simple.pdf"),
        Path::new("tests/fixtures/outline.pdf"),
    ]);

    let calls = call_count.load(Ordering::Relaxed);
    assert_eq!(calls, 2, "Progress callback should be called once per file");
}

/// BatchSummary totals match expectations.
#[test]
fn test_batch_summary_statistics() {
    let processor = BatchProcessor::new();
    let results = processor.extract_text_from_files(&[
        Path::new("tests/fixtures/simple.pdf"),
        Path::new("/nonexistent/fake.pdf"),
        Path::new("tests/fixtures/outline.pdf"),
    ]);

    let summary = BatchSummary::from_results(&results);

    assert_eq!(summary.total, 3);
    assert_eq!(summary.succeeded, 2);
    assert_eq!(summary.failed, 1);
    // total_chars may be 0 if fixtures are structure-only (no content streams)
    assert!(summary.total_pages > 0, "Should have counted some pages");
}

/// extract_text_from_directory finds PDF files in fixtures dir.
#[test]
fn test_batch_directory_extraction() {
    let processor = BatchProcessor::new();
    let results = processor
        .extract_text_from_directory(Path::new("tests/fixtures/"))
        .unwrap();

    // Should find at least simple.pdf and outline.pdf
    assert!(
        results.len() >= 2,
        "Should find at least 2 PDFs in fixtures dir, found {}",
        results.len()
    );

    let ok_count = results.iter().filter(|r| r.text.is_ok()).count();
    assert!(ok_count >= 2, "At least 2 PDFs should parse successfully");
}

// ===========================================================================
// Issue #151 — open_from_bytes
// ===========================================================================

/// Read simple.pdf to bytes → open_from_bytes → page_count > 0, extract_text Ok.
#[test]
fn test_open_from_bytes_valid_pdf() {
    let data = std::fs::read("tests/fixtures/simple.pdf").unwrap();
    let mut doc = PdfDocument::open_from_bytes(data).unwrap();
    let pages = doc.page_count().unwrap();
    assert!(pages > 0, "Should have at least 1 page");
    // simple.pdf is a blank page — extract_text should succeed (empty is OK)
    let _text = doc.extract_text(0).unwrap();
}

/// extract_text via open() == extract_text via open_from_bytes() (same PDF).
#[test]
fn test_open_from_bytes_matches_file() {
    let data = std::fs::read("tests/fixtures/simple.pdf").unwrap();

    let mut doc_file = PdfDocument::open("tests/fixtures/simple.pdf").unwrap();
    let mut doc_bytes = PdfDocument::open_from_bytes(data).unwrap();

    let pages = doc_file.page_count().unwrap();
    for p in 0..pages {
        let t1 = doc_file.extract_text(p).unwrap();
        let t2 = doc_bytes.extract_text(p).unwrap();
        assert_eq!(t1, t2, "Page {} text should match between open() and open_from_bytes()", p);
    }
}

/// page_count matches between file-based and bytes-based open.
#[test]
fn test_open_from_bytes_page_count() {
    let data = std::fs::read("tests/fixtures/outline.pdf").unwrap();

    let mut doc_file = PdfDocument::open("tests/fixtures/outline.pdf").unwrap();
    let mut doc_bytes = PdfDocument::open_from_bytes(data).unwrap();

    assert_eq!(
        doc_file.page_count().unwrap(),
        doc_bytes.page_count().unwrap(),
        "Page count should match"
    );
}

/// Invalid header → Err.
#[test]
fn test_open_from_bytes_invalid_header() {
    let result = PdfDocument::open_from_bytes(b"not a pdf".to_vec());
    assert!(result.is_err(), "Non-PDF data should return Err");
}

/// Severely truncated real PDF → Err.
#[test]
fn test_open_from_bytes_truncated() {
    // Take only the header — far too little to parse
    let truncated = b"%PDF-1.4\n1 0 obj\n".to_vec();
    let result = PdfDocument::open_from_bytes(truncated);
    assert!(result.is_err(), "Truncated PDF should return Err");
}

// ===========================================================================
// Issue #158 — OCR Page Type Detection
// ===========================================================================

/// simple.pdf page 0 → PageType::NativeText (it has real text, no scanned images).
#[cfg(feature = "ocr")]
#[test]
fn test_detect_page_type_text_page() {
    use pdf_oxide::ocr::{detect_page_type, PageType};
    let mut doc = PdfDocument::open("tests/fixtures/simple.pdf").unwrap();
    let page_type = detect_page_type(&mut doc, 0).unwrap();
    assert_eq!(
        page_type,
        PageType::NativeText,
        "simple.pdf should be detected as NativeText"
    );
}

/// simple.pdf page 0 → needs_ocr returns false.
#[cfg(feature = "ocr")]
#[test]
fn test_needs_ocr_text_page_false() {
    use pdf_oxide::ocr::needs_ocr;
    let mut doc = PdfDocument::open("tests/fixtures/simple.pdf").unwrap();
    let needs = needs_ocr(&mut doc, 0).unwrap();
    assert!(!needs, "Text-based PDF page should not need OCR");
}

// ===========================================================================
// Issue #157 — Table Detection
// ===========================================================================

use pdf_oxide::structure::spatial_table_detector::{SpatialTableDetector, TableDetectionConfig};

/// Extract spans from fixture PDFs and run table detection without panic.
#[test]
fn test_table_detection_on_fixtures() {
    for fixture in &["tests/fixtures/simple.pdf", "tests/fixtures/outline.pdf"] {
        let mut doc = PdfDocument::open(fixture).unwrap();
        let pages = doc.page_count().unwrap();
        let detector = SpatialTableDetector::with_config(TableDetectionConfig::default());

        for p in 0..pages {
            let spans = doc.extract_spans(p).unwrap();
            // Must not panic
            let _tables = detector.detect_tables(&spans);
        }
    }
}

/// Same PDF → same table results on two runs (deterministic).
#[test]
fn test_table_detection_deterministic() {
    let mut doc1 = PdfDocument::open("tests/fixtures/outline.pdf").unwrap();
    let mut doc2 = PdfDocument::open("tests/fixtures/outline.pdf").unwrap();

    let pages = doc1.page_count().unwrap();
    let detector = SpatialTableDetector::with_config(TableDetectionConfig::default());

    for p in 0..pages {
        let spans1 = doc1.extract_spans(p).unwrap();
        let spans2 = doc2.extract_spans(p).unwrap();

        let tables1 = detector.detect_tables(&spans1);
        let tables2 = detector.detect_tables(&spans2);

        assert_eq!(
            tables1.len(),
            tables2.len(),
            "Page {}: table count should be deterministic",
            p
        );
    }
}
