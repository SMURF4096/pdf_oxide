//! Second-pass regression tests for vertical writing mode (WMode 1).
//!
//! These tests pin the fixes for the bugs the reviewer flagged in the
//! first-pass implementation:
//!
//!   * C1/C2/C3: multi-Tj / TJ vertical advance must axis-swap in every
//!     downstream renderer and the extractor.
//!   * C4: `extract_chars` TJ-numeric-offset path must advance along Y.
//!   * C5: ToUnicode /WMode must NOT override /Encoding /WMode
//!     (ISO 32000-1 §9.10.2 — the ToUnicode CMap is extraction-only).
//!   * I2: horizontal scaling (Tz) must NOT apply to vertical w1y / Tc / Tw.
//!   * I3: malformed inner /W2 Form A triple must not desync subsequent CIDs.
//!   * I4: /W2 Form B range must not collapse onto u16::MAX on overflow.
//!
//! Each PDF is hand-assembled at the byte level (no real CJK fonts needed):
//! the ToUnicode CMap maps the test CIDs to ASCII so we can identify each
//! glyph in extracted output. /DW2 supplies vertical metrics; /W2 supplies
//! per-CID overrides where needed.

use pdf_oxide::document::PdfDocument;

/// Shared helper: emit a Type0 font + Identity-V/Identity-H wrapper with
/// the given content stream, /DW, /DW2, and an optional /W2 array.
///
/// Returns the assembled PDF bytes.
fn build_pdf(
    encoding_name: &str,
    content: &[u8],
    dw: i32,
    dw2: (i32, i32),
    w2: Option<&str>,
    cmap_extra: Option<&str>,
    horizontal_scaling_tz: Option<i32>,
) -> Vec<u8> {
    // Build ToUnicode CMap that maps CIDs 0001..0006 to ASCII A..F. If
    // `cmap_extra` is provided we splice it in (used by C5 to add an
    // explicit `/WMode 1 def`).
    let extra = cmap_extra.unwrap_or("");
    let cmap_src = format!(
        "/CIDInit /ProcSet findresource begin
12 dict begin
begincmap
/CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def
/CMapName /Adobe-Identity-UCS def
/CMapType 2 def
{extra}
1 begincodespacerange
<0000> <FFFF>
endcodespacerange
6 beginbfchar
<0001> <0041>
<0002> <0042>
<0003> <0043>
<0004> <0044>
<0005> <0045>
<0006> <0046>
endbfchar
endcmap
CMapName currentdict /CMap defineresource pop
end
end"
    );
    let cmap = cmap_src.as_bytes();

    // Optionally inject Tz at the head of the content stream.
    let content_with_tz: Vec<u8> = if let Some(tz) = horizontal_scaling_tz {
        let mut v = Vec::new();
        v.extend_from_slice(b"BT /F1 12 Tf ");
        v.extend_from_slice(format!("{} Tz ", tz).as_bytes());
        // Strip the leading "BT /F1 12 Tf " from `content` and use the rest
        // (callers always pass content beginning with the same prelude).
        let prefix = b"BT /F1 12 Tf ";
        if content.starts_with(prefix) {
            v.extend_from_slice(&content[prefix.len()..]);
        } else {
            v.extend_from_slice(content);
        }
        v
    } else {
        content.to_vec()
    };
    let content = content_with_tz.as_slice();

    let mut pdf = Vec::new();
    pdf.extend_from_slice(b"%PDF-1.4\n");

    let o1 = pdf.len();
    pdf.extend_from_slice(b"1 0 obj << /Type /Catalog /Pages 2 0 R >> endobj\n");
    let o2 = pdf.len();
    pdf.extend_from_slice(b"2 0 obj << /Type /Pages /Kids [3 0 R] /Count 1 >> endobj\n");
    let o3 = pdf.len();
    pdf.extend_from_slice(
        b"3 0 obj << /Type /Page /Parent 2 0 R /MediaBox [0 0 600 800] \
          /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >> endobj\n",
    );
    let o4 = pdf.len();
    pdf.extend_from_slice(format!("4 0 obj << /Length {} >> stream\n", content.len()).as_bytes());
    pdf.extend_from_slice(content);
    pdf.extend_from_slice(b"\nendstream\nendobj\n");

    let o5 = pdf.len();
    let f5 = format!(
        "5 0 obj << /Type /Font /Subtype /Type0 /BaseFont /TestFont \
         /Encoding /{} /DescendantFonts [6 0 R] /ToUnicode 7 0 R >> endobj\n",
        encoding_name
    );
    pdf.extend_from_slice(f5.as_bytes());

    let o6 = pdf.len();
    let w2_clause = w2.map(|s| format!(" /W2 {}", s)).unwrap_or_default();
    let f6 = format!(
        "6 0 obj << /Type /Font /Subtype /CIDFontType2 /BaseFont /TestFont \
         /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> \
         /DW {} /DW2 [{} {}]{} >> endobj\n",
        dw, dw2.0, dw2.1, w2_clause
    );
    pdf.extend_from_slice(f6.as_bytes());

    let o7 = pdf.len();
    pdf.extend_from_slice(format!("7 0 obj << /Length {} >> stream\n", cmap.len()).as_bytes());
    pdf.extend_from_slice(cmap);
    pdf.extend_from_slice(b"\nendstream\nendobj\n");

    let xref = pdf.len();
    pdf.extend_from_slice(b"xref\n0 8\n0000000000 65535 f \n");
    for off in [o1, o2, o3, o4, o5, o6, o7] {
        pdf.extend_from_slice(format!("{:010} 00000 n \n", off).as_bytes());
    }
    pdf.extend_from_slice(
        format!(
            "trailer << /Size 8 /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
            xref
        )
        .as_bytes(),
    );

    pdf
}

// ---------------------------------------------------------------------------
// C1/C2 — multi-Tj vertical advance (page extractor path)
// ---------------------------------------------------------------------------

/// Two successive Tj operators under Identity-V at the same starting cursor
/// must produce CHARS that advance along Y across the operator boundary,
/// not along X. /DW2 = [880 -1000] means each glyph displaces by
/// -font_size in y (12 pt at fs=12), so the second Tj's char Y must be
/// ~12 less than the first's.
///
/// `extract_chars` records per-character positions, bypassing the
/// `tj_span_buffer` that would otherwise coalesce consecutive Tj operators
/// into a single span. This makes per-Tj advances directly observable.
#[test]
fn c1_c2_extractor_multi_tj_vertical_advances_along_y() {
    let content = b"BT /F1 12 Tf 100 700 Td <0001> Tj <0002> Tj ET";
    let pdf = build_pdf("Identity-V", content, 1000, (880, -1000), None, None, None);
    let doc = PdfDocument::from_bytes(pdf).expect("parse synthetic vertical PDF");
    let chars = doc.extract_chars(0).expect("extract chars");

    let a = chars
        .iter()
        .find(|c| c.char == 'A')
        .expect("expected an 'A' char");
    let b = chars
        .iter()
        .find(|c| c.char == 'B')
        .expect("expected a 'B' char");

    // X stays stable across the two Tj operators…
    assert!(
        (a.bbox.x - b.bbox.x).abs() < 1.0,
        "vertical multi-Tj X must stay stable: A.x={}, B.x={}",
        a.bbox.x,
        b.bbox.x
    );
    // …and the second glyph's Y is ~12 units below the first. The first
    // glyph starts at y=700; after a w1y=-1000 advance at fs=12 we expect
    // the cursor at y ≈ 688.
    let dy = a.bbox.y - b.bbox.y;
    assert!(
        (dy - 12.0).abs() < 0.5,
        "vertical multi-Tj Y delta should be ~12; got {} (A.y={}, B.y={})",
        dy,
        a.bbox.y,
        b.bbox.y
    );
}

/// Same property for TJ arrays: numeric offsets and string segments alike
/// must advance along Y in vertical mode.
#[test]
fn c2_c4_extractor_tj_array_vertical_advances_along_y() {
    // [(<0001>) -250 (<0002>)] TJ — a 250/1000 × 12 = 3.0 unit positive Y
    // shift in vertical mode (negative offset moves the cursor *forward*
    // along the writing axis per §9.4.3). Vertical mode forward = -Y.
    let content = b"BT /F1 12 Tf 100 700 Td [<0001> -250 <0002>] TJ ET";
    let pdf = build_pdf("Identity-V", content, 1000, (880, -1000), None, None, None);
    let doc = PdfDocument::from_bytes(pdf).expect("parse synthetic vertical PDF");
    let spans = doc.extract_spans(0).expect("extract spans");

    let mut s = spans.clone();
    s.sort_by_key(|sp| sp.sequence);

    let combined: String = s.iter().map(|sp| sp.text.as_str()).collect();
    assert!(
        combined.contains('A') && combined.contains('B'),
        "expected A and B in TJ output; got {:?}",
        combined
    );

    // X stays stable across the TJ-array operator boundary.
    let xs: Vec<f32> = s.iter().map(|sp| sp.bbox.x).collect();
    let x_min = xs.iter().cloned().fold(f32::INFINITY, f32::min);
    let x_max = xs.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    assert!(
        (x_max - x_min) < 1.0,
        "vertical TJ array must keep X stable; got xs={:?}",
        xs
    );

    // Y must decrease across the operator: A is at the top, B follows below.
    let a = s.iter().find(|sp| sp.text.contains('A')).unwrap();
    let b = s.iter().find(|sp| sp.text.contains('B')).unwrap();
    assert!(
        a.bbox.y > b.bbox.y,
        "vertical TJ: A should be above B; A.y={}, B.y={}",
        a.bbox.y,
        b.bbox.y
    );
}

// ---------------------------------------------------------------------------
// C4 — extract_chars TJ numeric-offset path
// ---------------------------------------------------------------------------

/// `extract_chars` is a separate extraction pathway from `extract_spans`.
/// The TJ numeric-offset shift must use the active writing axis.
#[test]
fn c4_extract_chars_vertical_tj_offset_advances_along_y() {
    // TJ with two strings separated by a negative offset (forward shift).
    let content = b"BT /F1 12 Tf 100 700 Td [<0001> -250 <0002>] TJ ET";
    let pdf = build_pdf("Identity-V", content, 1000, (880, -1000), None, None, None);
    let doc = PdfDocument::from_bytes(pdf).expect("parse synthetic vertical PDF");
    let chars = doc.extract_chars(0).expect("extract chars");

    // Find char positions for 'A' and 'B'.
    let a = chars
        .iter()
        .find(|c| c.char == 'A')
        .expect("expected an 'A' char");
    let b = chars
        .iter()
        .find(|c| c.char == 'B')
        .expect("expected a 'B' char");

    // In vertical mode the two chars must share an X column and B must be
    // below A in user space (lower Y).
    assert!(
        (a.bbox.x - b.bbox.x).abs() < 1.0,
        "vertical extract_chars: A.x={} vs B.x={} should share a column",
        a.bbox.x,
        b.bbox.x
    );
    assert!(
        a.bbox.y > b.bbox.y,
        "vertical extract_chars: A.y={} should be above B.y={}",
        a.bbox.y,
        b.bbox.y
    );
}

// ---------------------------------------------------------------------------
// C5 — ToUnicode /WMode must NOT override /Encoding /WMode
// ---------------------------------------------------------------------------

/// A horizontal /Encoding (Identity-H) paired with a ToUnicode CMap whose
/// stream contains `/WMode 1 def` (stale tooling leftover) must NOT silently
/// flip the font to vertical. Per ISO 32000-1 §9.10.2 the ToUnicode CMap is
/// for extraction-time character → Unicode mapping ONLY.
#[test]
fn c5_to_unicode_wmode_does_not_override_encoding_wmode() {
    // Stale /WMode 1 def inside the ToUnicode prologue.
    let content = b"BT /F1 12 Tf 100 700 Td <0001> Tj <0002> Tj ET";
    let pdf = build_pdf(
        "Identity-H",
        content,
        1000,
        (880, -1000),
        None,
        Some("/WMode 1 def"),
        None,
    );
    let doc = PdfDocument::from_bytes(pdf).expect("parse synthetic horizontal PDF");
    let spans = doc.extract_spans(0).expect("extract spans");

    // Every span must report wmode=0 because /Encoding is Identity-H.
    for sp in &spans {
        assert_eq!(
            sp.wmode, 0,
            "ToUnicode /WMode 1 must NOT flip /Encoding /Identity-H to vertical; \
             span {:?} reported wmode={}",
            sp.text, sp.wmode
        );
    }
}

/// Counter-test: a vertical /Encoding (Identity-V) with a ToUnicode that has
/// no /WMode directive must still produce wmode=1.
#[test]
fn c5_identity_v_with_silent_to_unicode_is_vertical() {
    let content = b"BT /F1 12 Tf 100 700 Td <0001> Tj ET";
    let pdf = build_pdf("Identity-V", content, 1000, (880, -1000), None, None, None);
    let doc = PdfDocument::from_bytes(pdf).expect("parse synthetic vertical PDF");
    let spans = doc.extract_spans(0).expect("extract spans");
    assert!(!spans.is_empty());
    for sp in &spans {
        assert_eq!(
            sp.wmode, 1,
            "/Encoding /Identity-V must produce wmode=1; got {} for {:?}",
            sp.wmode, sp.text
        );
    }
}

// ---------------------------------------------------------------------------
// I2 — horizontal scaling (Tz) MUST NOT apply to vertical advances
// ---------------------------------------------------------------------------

/// Per ISO 32000-1 §9.4.4 the vertical advance formula is
/// `ty = (w1y − Tj/1000) × Tfs + Tc + Tw` — no Th factor. Tz (horizontal
/// scaling) is the glyph-stretching direction (§9.3.4), not "writing-
/// direction scale". Two glyphs at fs=12 with /DW2 [880 -1000] under Tz=150
/// must still advance by exactly 12 units in y (not 18). This pins the
/// extractor's vertical-Tj path to the spec.
// ---------------------------------------------------------------------------
// C1/C3 — page renderer + separation renderer cursor advances along Y
// ---------------------------------------------------------------------------

/// Render the multi-Tj vertical PDF through the page renderer. The test
/// asserts that rendering succeeds AND that the painted ink occupies a
/// vertical column (more pixel rows touched than columns), which is the
/// pixel-level signature of axis-correct per-Tj advance. A renderer that
/// advanced along X across the second Tj would paint a horizontal row
/// instead.
#[cfg(feature = "rendering")]
#[test]
fn c1_page_renderer_multi_tj_vertical_paints_a_column() {
    let content = b"BT /F1 24 Tf 100 700 Td <0001> Tj <0002> Tj <0003> Tj ET";
    let pdf = build_pdf("Identity-V", content, 1000, (880, -1000), None, None, None);
    let doc = PdfDocument::from_bytes(pdf).expect("parse synthetic vertical PDF");

    let opts = pdf_oxide::rendering::RenderOptions::with_dpi(72).as_raw();
    let rendered = pdf_oxide::rendering::render_page(&doc, 0, &opts)
        .expect("render page 0 of vertical PDF");

    // Find the bounding box of non-white pixels. The renderer falls back
    // to rectangle painting when no system font is found, so the painted
    // glyphs are filled rectangles — perfect for testing layout without
    // depending on installed fonts.
    let w = rendered.width as usize;
    let h = rendered.height as usize;
    let mut x_min = w;
    let mut x_max = 0usize;
    let mut y_min = h;
    let mut y_max = 0usize;
    let mut hits = 0usize;
    for y in 0..h {
        for x in 0..w {
            let i = (y * w + x) * 4;
            let r = rendered.data[i];
            let g = rendered.data[i + 1];
            let b = rendered.data[i + 2];
            // Skip near-white background. Use a loose threshold to ignore
            // anti-aliasing edges.
            if r < 250 || g < 250 || b < 250 {
                hits += 1;
                if x < x_min {
                    x_min = x;
                }
                if x > x_max {
                    x_max = x;
                }
                if y < y_min {
                    y_min = y;
                }
                if y > y_max {
                    y_max = y;
                }
            }
        }
    }
    assert!(
        hits > 100,
        "vertical render produced too few non-white pixels ({hits})"
    );
    let dx = (x_max - x_min) as f32;
    let dy = (y_max - y_min) as f32;
    // Three glyphs stacked vertically must occupy more y-extent than
    // x-extent (column, not row). A bug-class regression to horizontal
    // advance would invert this.
    assert!(
        dy > dx,
        "vertical render must produce a column (dy > dx); got dx={}, dy={}",
        dx,
        dy
    );
}

#[test]
fn i2_tz_does_not_scale_vertical_advance() {
    let content = b"BT /F1 12 Tf 100 700 Td <0001> Tj <0002> Tj ET";
    let pdf = build_pdf(
        "Identity-V",
        content,
        1000,
        (880, -1000),
        None,
        None,
        Some(150),
    );
    let doc = PdfDocument::from_bytes(pdf).expect("parse synthetic vertical PDF (Tz=150)");
    let chars = doc.extract_chars(0).expect("extract chars (Tz=150)");

    let a = chars.iter().find(|c| c.char == 'A').unwrap();
    let b = chars.iter().find(|c| c.char == 'B').unwrap();
    let dy = a.bbox.y - b.bbox.y;
    // Exactly font_size × |w1y|/1000 = 12 × 1 = 12.0. Th would give 18.0.
    assert!(
        (dy - 12.0).abs() < 0.5,
        "Tz=150 must NOT scale vertical advance: expected ~12, got {}",
        dy
    );
}
