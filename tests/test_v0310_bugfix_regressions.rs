//! Regression tests for v0.3.10 bug fixes.
//!
//! Guards against regressions for:
//! - Issue #170: XRef /Prev chain overflow on circular or deep incremental updates
//! - Issue #163: Circular Form XObject references causing infinite recursion
//! - Issue #154: Broken ligature characters not repaired in extracted text
//! - Issue #104: Long leader dot runs not normalized in TOC lines
//! - Issue #155: Panics from .unwrap() on invalid input instead of returning Err

fn write_temp_pdf(data: &[u8], name: &str) -> std::path::PathBuf {
    use std::io::Write;
    let dir = std::env::temp_dir().join("pdf_oxide_tests_v0310");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(name);
    let mut f = std::fs::File::create(&path).unwrap();
    f.write_all(data).unwrap();
    path
}

/// Appends a standard xref table + trailer + startxref + %%EOF to `pdf`.
/// `offsets` must include object 0 placeholder (pass 0 for the free entry).
fn finalize_pdf(pdf: &mut Vec<u8>, obj_offsets: &[usize]) {
    let xref_offset = pdf.len();
    let count = obj_offsets.len();
    pdf.extend_from_slice(format!("xref\n0 {}\n", count).as_bytes());
    pdf.extend_from_slice(b"0000000000 65535 f \r\n");
    for &off in &obj_offsets[1..] {
        pdf.extend_from_slice(format!("{:010} 00000 n \r\n", off).as_bytes());
    }
    let trailer = format!(
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
        count, xref_offset
    );
    pdf.extend_from_slice(trailer.as_bytes());
}

// ===========================================================================
// Issue #170 — XRef /Prev chain overflow
//
// Bug: Parser followed /Prev pointers without cycle detection, hanging on
// circular chains and failing on deep incremental saves (>100 sections).
// Fix: Iterative parsing with HashSet<u64> visited-set in src/xref.rs:340.
// ===========================================================================
mod issue_170_xref_prev_chain {
    use super::*;
    use pdf_oxide::document::PdfDocument;

    /// Build a PDF with `n` incremental xref sections linked via /Prev.
    ///
    /// startxref points to the last section; each section's trailer /Prev
    /// points to the previous one.
    fn build_incremental_xref_chain(section_count: usize) -> Vec<u8> {
        assert!(section_count >= 1);

        let objects = b"%PDF-1.4
1 0 obj
<< /Type /Catalog /Pages 2 0 R >>
endobj

2 0 obj
<< /Type /Pages /Kids [3 0 R] /Count 1 >>
endobj

3 0 obj
<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] >>
endobj

";
        let mut pdf = objects.to_vec();
        let mut xref_offsets: Vec<usize> = Vec::new();

        for i in 0..section_count {
            let xref_start = pdf.len();
            xref_offsets.push(xref_start);

            pdf.extend_from_slice(b"xref\n");
            if i == 0 {
                pdf.extend_from_slice(b"0 4\n");
                pdf.extend_from_slice(b"0000000000 65535 f \r\n");
                pdf.extend_from_slice(format!("{:010} 00000 n \r\n", 9).as_bytes());
                pdf.extend_from_slice(format!("{:010} 00000 n \r\n", 58).as_bytes());
                pdf.extend_from_slice(format!("{:010} 00000 n \r\n", 115).as_bytes());
            } else {
                pdf.extend_from_slice(b"0 1\n");
                pdf.extend_from_slice(b"0000000000 65535 f \r\n");
            }

            pdf.extend_from_slice(b"trailer\n<< /Size 4 /Root 1 0 R");
            if i > 0 {
                pdf.extend_from_slice(format!(" /Prev {}", xref_offsets[i - 1]).as_bytes());
            }
            pdf.extend_from_slice(b" >>\n");
        }

        let last_xref = xref_offsets.last().unwrap();
        pdf.extend_from_slice(format!("startxref\n{}\n%%EOF\n", last_xref).as_bytes());
        pdf
    }

    /// Build a PDF whose single xref section has /Prev pointing to itself.
    fn build_self_referencing_xref() -> Vec<u8> {
        let objects = b"%PDF-1.4
1 0 obj
<< /Type /Catalog /Pages 2 0 R >>
endobj

2 0 obj
<< /Type /Pages /Kids [] /Count 0 >>
endobj

";
        let mut pdf = objects.to_vec();
        let xref_start = pdf.len();

        pdf.extend_from_slice(b"xref\n0 3\n");
        pdf.extend_from_slice(b"0000000000 65535 f \r\n");
        pdf.extend_from_slice(format!("{:010} 00000 n \r\n", 9).as_bytes());
        pdf.extend_from_slice(format!("{:010} 00000 n \r\n", 58).as_bytes());

        let trailer = format!(
            "trailer\n<< /Size 3 /Root 1 0 R /Prev {} >>\n",
            xref_start
        );
        pdf.extend_from_slice(trailer.as_bytes());
        pdf.extend_from_slice(format!("startxref\n{}\n%%EOF\n", xref_start).as_bytes());
        pdf
    }

    /// Build a PDF with two xref sections forming A → B → A cycle.
    fn build_two_node_xref_cycle() -> Vec<u8> {
        let objects = b"%PDF-1.4
1 0 obj
<< /Type /Catalog /Pages 2 0 R >>
endobj

2 0 obj
<< /Type /Pages /Kids [] /Count 0 >>
endobj

";
        let mut pdf = objects.to_vec();

        // Write xref A with a placeholder for /Prev (patched after we know B's offset)
        let offset_a = pdf.len();
        pdf.extend_from_slice(b"xref\n0 3\n");
        pdf.extend_from_slice(b"0000000000 65535 f \r\n");
        pdf.extend_from_slice(format!("{:010} 00000 n \r\n", 9).as_bytes());
        pdf.extend_from_slice(format!("{:010} 00000 n \r\n", 58).as_bytes());

        pdf.extend_from_slice(b"trailer\n<< /Size 3 /Root 1 0 R /Prev ");
        let prev_placeholder_pos = pdf.len();
        pdf.extend_from_slice(b"XXXXXXXXXX >>\n"); // 10-char placeholder

        // Write xref B pointing back to A
        let offset_b = pdf.len();
        pdf.extend_from_slice(b"xref\n0 1\n");
        pdf.extend_from_slice(b"0000000000 65535 f \r\n");
        let trailer_b = format!(
            "trailer\n<< /Size 3 /Root 1 0 R /Prev {} >>\n",
            offset_a
        );
        pdf.extend_from_slice(trailer_b.as_bytes());

        // Patch A's /Prev to point to B
        let offset_b_str = format!("{:<10}", offset_b);
        pdf[prev_placeholder_pos..prev_placeholder_pos + 10]
            .copy_from_slice(offset_b_str.as_bytes());

        pdf.extend_from_slice(format!("startxref\n{}\n%%EOF\n", offset_b).as_bytes());
        pdf
    }

    #[test]
    fn three_section_chain_opens_successfully() {
        let data = build_incremental_xref_chain(3);
        let path = write_temp_pdf(&data, "xref_3_sections.pdf");
        let result = PdfDocument::open(&path);
        assert!(
            result.is_ok(),
            "PDF with 3 incremental xref sections should open: {:?}",
            result.err()
        );
    }

    #[test]
    fn five_section_deep_chain_opens_successfully() {
        let data = build_incremental_xref_chain(5);
        let path = write_temp_pdf(&data, "xref_5_sections.pdf");
        let result = PdfDocument::open(&path);
        assert!(
            result.is_ok(),
            "PDF with 5 incremental xref sections should open: {:?}",
            result.err()
        );
    }

    #[test]
    fn self_referencing_prev_terminates_without_hang() {
        let data = build_self_referencing_xref();
        let path = write_temp_pdf(&data, "xref_self_loop.pdf");
        // Must terminate (no hang). Either Ok or Err is acceptable.
        let _result = PdfDocument::open(&path);
    }

    #[test]
    fn two_node_prev_cycle_terminates_without_hang() {
        let data = build_two_node_xref_cycle();
        let path = write_temp_pdf(&data, "xref_two_node_cycle.pdf");
        // Must terminate (no hang). Either Ok or Err is acceptable.
        let _result = PdfDocument::open(&path);
    }
}

// ===========================================================================
// Issue #163 — Circular Form XObject references
//
// Bug: A Form XObject referencing itself (or forming an indirect cycle via
// other XObjects) caused infinite recursion / stack overflow during text
// extraction.
// Fix: Cycle tracking via can_process_xobject/push_xobject/pop_xobject
// in src/document.rs:4922-4925.
// ===========================================================================
mod issue_163_circular_xobject {
    use super::*;
    use pdf_oxide::document::PdfDocument;

    /// Build a PDF where Form XObject /X0's content stream contains `/X0 Do`,
    /// referencing itself.
    fn build_self_referencing_form_xobject() -> Vec<u8> {
        let mut pdf = Vec::new();
        pdf.extend_from_slice(b"%PDF-1.4\n");

        let obj1 = pdf.len();
        pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\n");

        let obj2 = pdf.len();
        pdf.extend_from_slice(
            b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\n",
        );

        let obj3 = pdf.len();
        pdf.extend_from_slice(
            b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] \
              /Resources << /XObject << /X0 4 0 R >> >> /Contents 5 0 R >>\nendobj\n\n",
        );

        // X0 references itself: its stream contains "/X0 Do"
        let obj4 = pdf.len();
        let stream = b"/X0 Do";
        let header = format!(
            "4 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 100 100] \
             /Resources << /XObject << /X0 4 0 R >> >> /Length {} >>\nstream\n",
            stream.len()
        );
        pdf.extend_from_slice(header.as_bytes());
        pdf.extend_from_slice(stream);
        pdf.extend_from_slice(b"\nendstream\nendobj\n\n");

        // Page content invokes X0
        let obj5 = pdf.len();
        let content = b"/X0 Do";
        let content_header = format!("5 0 obj\n<< /Length {} >>\nstream\n", content.len());
        pdf.extend_from_slice(content_header.as_bytes());
        pdf.extend_from_slice(content);
        pdf.extend_from_slice(b"\nendstream\nendobj\n\n");

        finalize_pdf(&mut pdf, &[0, obj1, obj2, obj3, obj4, obj5]);
        pdf
    }

    /// Build a PDF with an indirect XObject cycle: X0 → X1 → X0.
    fn build_two_node_xobject_cycle() -> Vec<u8> {
        let mut pdf = Vec::new();
        pdf.extend_from_slice(b"%PDF-1.4\n");

        let obj1 = pdf.len();
        pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\n");

        let obj2 = pdf.len();
        pdf.extend_from_slice(
            b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\n",
        );

        let obj3 = pdf.len();
        pdf.extend_from_slice(
            b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] \
              /Resources << /XObject << /X0 4 0 R /X1 5 0 R >> >> /Contents 6 0 R >>\nendobj\n\n",
        );

        // X0 invokes X1
        let obj4 = pdf.len();
        let stream = b"/X1 Do";
        let header = format!(
            "4 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 100 100] \
             /Resources << /XObject << /X1 5 0 R >> >> /Length {} >>\nstream\n",
            stream.len()
        );
        pdf.extend_from_slice(header.as_bytes());
        pdf.extend_from_slice(stream);
        pdf.extend_from_slice(b"\nendstream\nendobj\n\n");

        // X1 invokes X0 (completes cycle)
        let obj5 = pdf.len();
        let stream = b"/X0 Do";
        let header = format!(
            "5 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 100 100] \
             /Resources << /XObject << /X0 4 0 R >> >> /Length {} >>\nstream\n",
            stream.len()
        );
        pdf.extend_from_slice(header.as_bytes());
        pdf.extend_from_slice(stream);
        pdf.extend_from_slice(b"\nendstream\nendobj\n\n");

        // Page content invokes X0
        let obj6 = pdf.len();
        let content = b"/X0 Do";
        let header = format!("6 0 obj\n<< /Length {} >>\nstream\n", content.len());
        pdf.extend_from_slice(header.as_bytes());
        pdf.extend_from_slice(content);
        pdf.extend_from_slice(b"\nendstream\nendobj\n\n");

        finalize_pdf(&mut pdf, &[0, obj1, obj2, obj3, obj4, obj5, obj6]);
        pdf
    }

    /// Build a PDF with a three-node XObject cycle: X0 → X1 → X2 → X0.
    fn build_three_node_xobject_cycle() -> Vec<u8> {
        let mut pdf = Vec::new();
        pdf.extend_from_slice(b"%PDF-1.4\n");

        let obj1 = pdf.len();
        pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\n");

        let obj2 = pdf.len();
        pdf.extend_from_slice(
            b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\n",
        );

        let obj3 = pdf.len();
        pdf.extend_from_slice(
            b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] \
              /Resources << /XObject << /X0 4 0 R /X1 5 0 R /X2 6 0 R >> >> \
              /Contents 7 0 R >>\nendobj\n\n",
        );

        let obj4 = pdf.len();
        let stream = b"/X1 Do";
        let header = format!(
            "4 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 100 100] \
             /Resources << /XObject << /X1 5 0 R >> >> /Length {} >>\nstream\n",
            stream.len()
        );
        pdf.extend_from_slice(header.as_bytes());
        pdf.extend_from_slice(stream);
        pdf.extend_from_slice(b"\nendstream\nendobj\n\n");

        let obj5 = pdf.len();
        let stream = b"/X2 Do";
        let header = format!(
            "5 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 100 100] \
             /Resources << /XObject << /X2 6 0 R >> >> /Length {} >>\nstream\n",
            stream.len()
        );
        pdf.extend_from_slice(header.as_bytes());
        pdf.extend_from_slice(stream);
        pdf.extend_from_slice(b"\nendstream\nendobj\n\n");

        // X2 → X0 (completes the cycle)
        let obj6 = pdf.len();
        let stream = b"/X0 Do";
        let header = format!(
            "6 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 100 100] \
             /Resources << /XObject << /X0 4 0 R >> >> /Length {} >>\nstream\n",
            stream.len()
        );
        pdf.extend_from_slice(header.as_bytes());
        pdf.extend_from_slice(stream);
        pdf.extend_from_slice(b"\nendstream\nendobj\n\n");

        // Page content invokes X0
        let obj7 = pdf.len();
        let content = b"/X0 Do";
        let header = format!("7 0 obj\n<< /Length {} >>\nstream\n", content.len());
        pdf.extend_from_slice(header.as_bytes());
        pdf.extend_from_slice(content);
        pdf.extend_from_slice(b"\nendstream\nendobj\n\n");

        finalize_pdf(&mut pdf, &[0, obj1, obj2, obj3, obj4, obj5, obj6, obj7]);
        pdf
    }

    /// Build a PDF where the same non-circular XObject is referenced twice via
    /// `q /X0 Do Q q /X0 Do Q` — verifying reuse works after cycle tracking.
    fn build_reused_form_xobject() -> Vec<u8> {
        let mut pdf = Vec::new();
        pdf.extend_from_slice(b"%PDF-1.4\n");

        let obj1 = pdf.len();
        pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\n");

        let obj2 = pdf.len();
        pdf.extend_from_slice(
            b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\n",
        );

        let obj3 = pdf.len();
        pdf.extend_from_slice(
            b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] \
              /Resources << /Font << /F1 5 0 R >> /XObject << /X0 4 0 R >> >> \
              /Contents 6 0 R >>\nendobj\n\n",
        );

        // X0: a Form XObject containing text
        let obj4 = pdf.len();
        let stream = b"BT /F1 12 Tf 10 10 Td (Reused) Tj ET";
        let header = format!(
            "4 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 100 100] \
             /Resources << /Font << /F1 5 0 R >> >> /Length {} >>\nstream\n",
            stream.len()
        );
        pdf.extend_from_slice(header.as_bytes());
        pdf.extend_from_slice(stream);
        pdf.extend_from_slice(b"\nendstream\nendobj\n\n");

        let obj5 = pdf.len();
        pdf.extend_from_slice(
            b"5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica \
              /Encoding /WinAnsiEncoding >>\nendobj\n\n",
        );

        // Page content invokes X0 twice
        let obj6 = pdf.len();
        let content = b"q /X0 Do Q q /X0 Do Q";
        let header = format!("6 0 obj\n<< /Length {} >>\nstream\n", content.len());
        pdf.extend_from_slice(header.as_bytes());
        pdf.extend_from_slice(content);
        pdf.extend_from_slice(b"\nendstream\nendobj\n\n");

        finalize_pdf(&mut pdf, &[0, obj1, obj2, obj3, obj4, obj5, obj6]);
        pdf
    }

    #[test]
    fn self_referencing_xobject_terminates_without_overflow() {
        let data = build_self_referencing_form_xobject();
        let path = write_temp_pdf(&data, "xobj_self_ref.pdf");
        let mut doc = PdfDocument::open(&path).expect("Should parse PDF structure");
        // Must not hang or stack overflow. Either Ok or Err is acceptable.
        let _result = doc.extract_text(0);
    }

    #[test]
    fn two_node_xobject_cycle_terminates_without_overflow() {
        let data = build_two_node_xobject_cycle();
        let path = write_temp_pdf(&data, "xobj_two_node_cycle.pdf");
        let mut doc = PdfDocument::open(&path).expect("Should parse PDF structure");
        let _result = doc.extract_text(0);
    }

    #[test]
    fn three_node_xobject_cycle_terminates_gracefully() {
        let data = build_three_node_xobject_cycle();
        let path = write_temp_pdf(&data, "xobj_three_node_cycle.pdf");
        let mut doc = PdfDocument::open(&path).expect("Should parse PDF structure");
        let _result = doc.extract_text(0);
    }

    #[test]
    fn non_circular_xobject_invoked_twice_produces_text() {
        let data = build_reused_form_xobject();
        let path = write_temp_pdf(&data, "xobj_reused_twice.pdf");
        let mut doc = PdfDocument::open(&path).expect("Should parse PDF structure");
        let text = doc.extract_text(0).unwrap();
        // The XObject renders "Reused" — invoked twice, but dedup may merge overlapping text
        assert!(
            text.contains("Reused"),
            "Reused XObject text should appear at least once: got '{}'",
            text
        );
    }
}

// ===========================================================================
// Issue #154 — Broken ligature character repair
//
// Bug: Some PDF producers encode ligatures as ASCII substitution characters
// (! for ff, " for ffi, # for fi, $ for fl, % for ffl). Extracted text
// contained raw punctuation instead of the intended letter sequences.
// Fix: repair_ligatures() in src/converters/text_post_processor.rs:322.
// ===========================================================================
mod issue_154_ligature_repair {
    use pdf_oxide::converters::text_post_processor::TextPostProcessor;

    #[test]
    fn all_five_ligature_substitutions_between_letters() {
        // !→ff  "→ffi  #→fi  $→fl  %→ffl
        assert_eq!(TextPostProcessor::repair_ligatures("di!erent"), "different");
        assert_eq!(TextPostProcessor::repair_ligatures("o\"ce"), "office");
        assert_eq!(TextPostProcessor::repair_ligatures("de#ne"), "define");
        assert_eq!(TextPostProcessor::repair_ligatures("re$ect"), "reflect");
        assert_eq!(TextPostProcessor::repair_ligatures("ba%e"), "baffle");
    }

    #[test]
    fn substitution_requires_preceding_letter() {
        // # at word start (no preceding letter) — must NOT be replaced
        assert_eq!(TextPostProcessor::repair_ligatures("#nancial"), "#nancial");
    }

    #[test]
    fn punctuation_preserved_at_word_boundaries() {
        assert_eq!(TextPostProcessor::repair_ligatures("Hello!"), "Hello!");
        assert_eq!(TextPostProcessor::repair_ligatures("$100"), "$100");
        assert_eq!(TextPostProcessor::repair_ligatures("50%"), "50%");
        assert_eq!(
            TextPostProcessor::repair_ligatures("\"hello\""),
            "\"hello\""
        );
    }

    #[test]
    fn mixed_broken_ligatures_and_real_punctuation() {
        // "di!erent" (ff between letters) + "o\"ces" (ffi between letters)
        // + "#nancial" (# at word start — NOT replaced)
        assert_eq!(
            TextPostProcessor::repair_ligatures("di!erent o\"ces #nancial"),
            "different offices #nancial"
        );
    }
}

// ===========================================================================
// Issue #104 — Leader dot normalization
//
// Bug: TOC lines like "Section .................. 5" produced long dot runs
// in extracted text, making output noisy.
// Fix: normalize_leader_dots() in src/converters/text_post_processor.rs:383
// collapses runs of 4+ dots into "...".
// ===========================================================================
mod issue_104_leader_dot_normalization {
    use pdf_oxide::converters::text_post_processor::TextPostProcessor;

    #[test]
    fn four_or_more_dots_collapsed_to_ellipsis() {
        assert_eq!(
            TextPostProcessor::normalize_leader_dots("Section ............. 5"),
            "Section ... 5"
        );
    }

    #[test]
    fn three_or_fewer_dots_preserved() {
        assert_eq!(TextPostProcessor::normalize_leader_dots("wait..."), "wait...");
        assert_eq!(TextPostProcessor::normalize_leader_dots("hmm.."), "hmm..");
        assert_eq!(TextPostProcessor::normalize_leader_dots("one."), "one.");
    }
}

// ===========================================================================
// Issue #155 — No-panic safety (.unwrap removal)
//
// Bug: Internal .unwrap() calls panicked on malformed or out-of-range input
// instead of returning Err.
// Fix: Replaced .unwrap() with proper error propagation across the codebase.
// ===========================================================================
mod issue_155_no_panic_safety {
    use pdf_oxide::document::PdfDocument;

    #[test]
    fn out_of_range_page_index_returns_err() {
        let mut doc = PdfDocument::open("tests/fixtures/simple.pdf").unwrap();
        let result = doc.extract_text(99999);
        assert!(result.is_err(), "Out-of-range page index should return Err");
    }

    #[test]
    fn empty_bytes_returns_err() {
        let result = PdfDocument::open_from_bytes(vec![]);
        assert!(result.is_err(), "Empty bytes should return Err");
    }

    #[test]
    fn garbage_bytes_returns_err() {
        let result = PdfDocument::open_from_bytes(vec![0xFF; 100]);
        assert!(result.is_err(), "Garbage bytes should return Err");
    }
}
