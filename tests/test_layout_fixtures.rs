//! Layout-fix fixtures (TDD-1 of v0.3.34 layout-fix plan).
//!
//! Hand-crafted PDFs that capture every text-layout failure class we
//! found in the v0.3.33 → release/v0.3.34 regression study, plus the
//! cases that should already work. Built deterministically via the
//! low-level `PdfWriter` so each text fragment is its own BT/ET block
//! and the extractor's run-grouper sees independent spans.
//!
//! On a clean tree the "bug" tests (L3YHC reversal, table cell
//! concatenation, end-of-line hyphenation, distinct-region collision)
//! must FAIL — that's the red bar that gates the four corresponding
//! fixes. The "should already work" tests (single column, two column,
//! narrow gutter, mixed-size columns, the #319 wins) must stay green
//! from day one and across every fix.
//!
//! See `docs/v0.3.34-layout-fix-plan.md` for the design context.
//!
//! Builder convention: PDF coordinate origin is bottom-left.
//! Letter page = 612 × 792 pt. We use 72 pt margins, 12 pt body,
//! 14.4 pt line height (= 1.2 × body) unless stated.
//!
//! BT/ET separation: between consecutive `add_text` calls we insert a
//! zero-size `draw_rect(0,0,0,0)` which forces an `ET` marker, so the
//! next `add_text` starts a fresh `BT`. Without this the writer keeps
//! one open BT block across all calls and the extractor merges
//! adjacent text into a single span — defeating multi-column layout.

use pdf_oxide::document::PdfDocument;
use pdf_oxide::writer::{PageBuilder, PdfWriter};

const PAGE_H: f32 = 792.0;
const MARGIN: f32 = 72.0;
const BODY: f32 = 12.0;
const LH: f32 = BODY * 1.2; // 14.4 pt

/// Add one text fragment and force a BT/ET boundary so the next text
/// fragment starts a fresh text object (otherwise the extractor merges
/// adjacent text on the same baseline into one span).
fn put(page: &mut PageBuilder<'_>, text: &str, x: f32, y: f32, font: &str, size: f32) {
    page.add_text(text, x, y, font, size);
    page.draw_rect(0.0, 0.0, 0.0, 0.0);
}

fn build_and_extract(build_fn: impl FnOnce(&mut PdfWriter)) -> String {
    let mut writer = PdfWriter::new();
    build_fn(&mut writer);
    let bytes = writer.finish().expect("build PDF");
    let mut doc = PdfDocument::from_bytes(bytes).expect("open PDF");
    doc.extract_text(0).expect("extract page 0")
}

/// Place body-text lines top-down at column x, starting from y_top.
fn place_column(page: &mut PageBuilder<'_>, x: f32, y_top: f32, lines: &[String]) {
    let mut y = y_top;
    for line in lines {
        put(page, line, x, y, "Helvetica", BODY);
        y -= LH;
    }
}

// =====================================================================
// Fixtures that MUST stay green from day one.
// =====================================================================

/// `body_single_column.pdf` — 30 lines of body text in a single column.
/// Should never trigger XY-cut. Output preserves input order.
#[test]
fn fixture_body_single_column_preserves_order() {
    let lines: Vec<String> = (1..=30)
        .map(|i| format!("Body line number {i:02} reads top to bottom in a single column."))
        .collect();
    let out = build_and_extract(|w| {
        let mut page = w.add_letter_page();
        place_column(&mut page, MARGIN, PAGE_H - MARGIN, &lines);
    });

    let mut last_pos = 0usize;
    for line in &lines {
        let pos = out[last_pos..]
            .find(line.as_str())
            .unwrap_or_else(|| {
                panic!("line not found in expected order: {line:?}\n--- output ---\n{out}")
            })
            + last_pos;
        last_pos = pos + line.len();
    }
}

/// `body_two_column.pdf` — clean two-column body. Left column must be
/// fully read before right column. No interleave like the legacy
/// `accompaally` / `correlaanonymous` artifacts.
///
/// 30 lines per column ensures the page passes `is_multi_column_page`'s
/// ≥15-spans-per-half threshold so XY-cut routing kicks in.
#[test]
fn fixture_body_two_column_no_interleave() {
    let left: Vec<String> = (1..=30)
        .map(|i| format!("Left{i:02} short body text"))
        .collect();
    let right: Vec<String> = (1..=30)
        .map(|i| format!("Right{i:02} other body text"))
        .collect();

    let out = build_and_extract(|w| {
        let mut page = w.add_letter_page();
        place_column(&mut page, MARGIN, PAGE_H - MARGIN, &left);
        place_column(&mut page, 360.0, PAGE_H - MARGIN, &right);
    });

    let last_left = out.find(left.last().unwrap().as_str()).expect("last-left missing");
    let first_right = out.find(right.first().unwrap().as_str()).expect("first-right missing");
    assert!(
        last_left < first_right,
        "two-column reading order broken: last-left at {last_left}, first-right at {first_right}\n--- output ---\n{out}"
    );

    // No interleave: between two consecutive left lines, no right line.
    for pair in left.windows(2) {
        let a = out.find(pair[0].as_str()).unwrap();
        let b = out.find(pair[1].as_str()).unwrap();
        let between = &out[a..b];
        for r in &right {
            assert!(
                !between.contains(r.as_str()),
                "interleave detected: right line {r:?} appears between left {:?} and left {:?}",
                pair[0],
                pair[1]
            );
        }
    }
}

/// `body_two_column_narrow_gutter.pdf` — 2 cols with only ~1.5 em gutter.
/// Edge case for valley detection in XY-cut. Should still split cleanly.
#[test]
fn fixture_body_two_column_narrow_gutter_still_splits() {
    // Short text so left column ends well before the right column starts.
    let left: Vec<String> = (1..=30).map(|i| format!("L{i:02} body")).collect();
    let right: Vec<String> = (1..=30).map(|i| format!("R{i:02} other")).collect();

    // Use realistic column widths (~220pt body, 18pt gutter ≈ 1.5em).
    // Each column has wider text so center spread of histogram still
    // shows two distinct peaks — the narrow-gutter case for XY-cut
    // valley detection.
    let left_wide: Vec<String> = (1..=30)
        .map(|i| format!("L{i:02} extended body text spanning whole column"))
        .collect();
    let right_wide: Vec<String> = (1..=30)
        .map(|i| format!("R{i:02} extended other text spanning whole column"))
        .collect();

    let out = build_and_extract(|w| {
        let mut page = w.add_letter_page();
        place_column(&mut page, MARGIN, PAGE_H - MARGIN, &left_wide);
        // Left text is ~250pt wide (42 chars × 6pt) → ends at ~322.
        // Right at 340 → 18pt gutter (~1.5 em at 12pt body).
        place_column(&mut page, 340.0, PAGE_H - MARGIN, &right_wide);
    });

    let last_left = out.find(left_wide.last().unwrap().as_str()).expect("last-left missing");
    let first_right = out.find(right_wide.first().unwrap().as_str()).expect("first-right missing");
    assert!(
        last_left < first_right,
        "narrow-gutter two-column reading order broken\n--- output ---\n{out}"
    );
    let _ = (left, right); // keep the original short Vecs alive for future debug tweaks
}

/// `mixed_size_columns.pdf` — left col 12pt, right col 10pt. Dominant
/// font (mode by char count) should resolve to 12pt and the resulting
/// thresholds should still split the page correctly.
#[test]
fn fixture_mixed_size_columns_dominant_em_picks_mode() {
    let left: Vec<String> = (1..=30)
        .map(|i| format!("Left12pt{i:02} body text content"))
        .collect();
    let right: Vec<String> = (1..=22)
        .map(|i| format!("Right10pt{i:02} smaller body text"))
        .collect();

    let out = build_and_extract(|w| {
        let mut page = w.add_letter_page();
        let mut y = PAGE_H - MARGIN;
        for line in &left {
            put(&mut page, line, MARGIN, y, "Helvetica", 12.0);
            y -= 14.4;
        }
        let mut y = PAGE_H - MARGIN;
        for line in &right {
            put(&mut page, line, 360.0, y, "Helvetica", 10.0);
            y -= 12.0;
        }
    });

    let last_left = out.find(left.last().unwrap().as_str()).expect("last-left missing");
    let first_right = out.find(right.first().unwrap().as_str()).expect("first-right missing");
    assert!(
        last_left < first_right,
        "mixed-size two-column reading order broken\n--- output ---\n{out}"
    );
}

// =====================================================================
// Fixtures that lock in the #319 multi-column-interleave fixes.
//
// These were broken on v0.3.33 (`accompaally` / `correlaanonymous`
// garbled tokens from row-aware re-sort interleaving left and right
// columns). They became correct on commit cb86499 (#319). They must
// stay green through every future layout change — that's the explicit
// reason we cannot revert cb86499 to fix L3YHC.
// =====================================================================

/// `two_column_accompa_nying_no_interleave.pdf` — the canonical #319
/// case: left column has the word `accompa-` at the end of its line,
/// continuing as `nying` on the next left-column line. The right
/// column at the same Y carries unrelated text. v0.3.33 produced
/// `accompaally`-style mash by row-interleaving the two columns;
/// the cb86499 fix reads each column to completion first.
#[test]
fn fixture_two_column_accompa_nying_no_interleave() {
    // 30 lines per column to trigger multi-column detection.
    let mut left: Vec<String> = (1..=12).map(|i| format!("LeftPad{i:02} body text")).collect();
    left.push("We refer to the accompa-".to_string());
    left.push("nying table for details.".to_string());
    left.extend((15..=30).map(|i| format!("LeftPad{i:02} body text")));

    let mut right: Vec<String> = (1..=12).map(|i| format!("RightPad{i:02} other text")).collect();
    right.push("This line reads really clearly.".to_string());
    right.push("This line is independent text.".to_string());
    right.extend((15..=30).map(|i| format!("RightPad{i:02} other text")));

    let out = build_and_extract(|w| {
        let mut page = w.add_letter_page();
        place_column(&mut page, MARGIN, PAGE_H - MARGIN, &left);
        place_column(&mut page, 360.0, PAGE_H - MARGIN, &right);
    });

    for bad in &[
        "accompaally",
        "accompareally",
        "accompathis",
        "accompanyingreally",
    ] {
        assert!(
            !out.contains(bad),
            "Multi-column row-interleave artifact `{bad}` re-appeared:\n--- output ---\n{out}"
        );
    }
    assert!(
        out.contains("accompanying") || out.contains("accompa-nying"),
        "Hyphenated word `accompa-`/`nying` not rejoined cleanly:\n--- output ---\n{out}"
    );
}

/// `two_column_correla_tion_no_interleave.pdf` — second canonical
/// #319 case (`correlaanonymous`). `correla-` / `tion` hyphenation
/// in the left column, `anonymous` on the right side at interleave Y.
#[test]
fn fixture_two_column_correla_tion_no_interleave() {
    let mut left: Vec<String> = (1..=10).map(|i| format!("LeftPad{i:02} body text")).collect();
    left.push("We compute pairwise correla-".to_string());
    left.push("tion across all sample pairs.".to_string());
    left.extend((13..=30).map(|i| format!("LeftPad{i:02} body text")));

    let mut right: Vec<String> = (1..=10).map(|i| format!("RightPad{i:02} other text")).collect();
    right.push("All datasets are anonymous and labelled.".to_string());
    right.push("This line is independent text.".to_string());
    right.extend((13..=30).map(|i| format!("RightPad{i:02} other text")));

    let out = build_and_extract(|w| {
        let mut page = w.add_letter_page();
        place_column(&mut page, MARGIN, PAGE_H - MARGIN, &left);
        place_column(&mut page, 360.0, PAGE_H - MARGIN, &right);
    });

    for bad in &["correlaanonymous", "correlathis", "correlaall"] {
        assert!(
            !out.contains(bad),
            "Multi-column row-interleave artifact `{bad}` re-appeared:\n--- output ---\n{out}"
        );
    }
    assert!(
        out.contains("correlation") || out.contains("correla-tion"),
        "Hyphenated word `correla-`/`tion` not rejoined cleanly:\n--- output ---\n{out}"
    );
}

/// `legal_style_two_column.pdf` — synthetic dense two-column layout
/// with hyphenated word breaks in both columns and an indented marker
/// (mimics CFR-style § section IDs without quoting any real regulation).
/// Validates that markers from the right column don't appear mid-sentence
/// in left-column text — the failure pattern that #319 fixed on real
/// CFR PDFs.
#[test]
fn fixture_legal_style_columns_no_section_marker_interleave() {
    let mut left: Vec<String> = vec![
        "Based upon the assessment by the certi-".to_string(),
        "fied reviewer, the Office shall re-".to_string(),
        "tain the right to request supplemen-".to_string(),
        "tary evaluations from external experts".to_string(),
        "selected by the Office. Such requests".to_string(),
        "must be approved in writing by the Re-".to_string(),
        "gional Director of Reviews before any".to_string(),
        "additional materials may be requested.".to_string(),
    ];
    left.extend((9..=32).map(|i| format!("LeftPad{i:02} body text")));

    let mut right: Vec<String> = vec![
        "A.1 Petition by an applicant to re-".to_string(),
        "move conditional restrictions on stand-".to_string(),
        "ing under this synthetic regulation.".to_string(),
        "(a) Filing rules. (1) General proce-".to_string(),
        "dures. A petition to remove the condi-".to_string(),
        "tional restriction shall be filed by".to_string(),
        "the applicant on the prescribed form".to_string(),
        "within the window described in (b).".to_string(),
    ];
    right.extend((9..=32).map(|i| format!("RightPad{i:02} body text")));

    let out = build_and_extract(|w| {
        let mut page = w.add_letter_page();
        place_column(&mut page, MARGIN, PAGE_H - MARGIN, &left);
        place_column(&mut page, 360.0, PAGE_H - MARGIN, &right);
    });

    for bad in &[
        "certi-A.1",
        "certified A.1",
        "Office shall re- move",
        "Officemove",
    ] {
        assert!(
            !out.contains(bad),
            "Right-column text leaked into left-column sentence (`{bad}`):\n--- output ---\n{out}"
        );
    }

    let certified_pos = out
        .find("certified")
        .or_else(|| out.find("certi-fied"))
        .expect("`certified` (or hyphenated variant) missing from left column");
    let office_pos = out.find("Office").expect("`Office` missing from left column");
    assert!(
        certified_pos < office_pos,
        "Left column reading order broken: certified at {certified_pos}, Office at {office_pos}\n--- output ---\n{out}"
    );
}

// =====================================================================
// Fixtures that MUST currently fail — these gate the four bug fixes.
// =====================================================================

/// `fax_scattered_fragments.pdf` — synthetic fixture mimicking the
/// shape of the L3YHC.pdf failure mode (a fax-style layout where each
/// "word" is a separately-positioned text fragment along the same
/// baseline). Content is fully fabricated; no text from the real
/// reporter PDF is used.
///
/// Bug on current HEAD: `is_multi_column_page` false-positives this
/// layout, the row-aware re-sort gets skipped, and the resulting
/// XY-cut order reverses fragments within a "column" — producing
/// concatenations like `WORDFIVEWORDFOURWORDTHREE` instead of
/// `WORDONE WORDTWO ... WORDFIVE`.
///
/// Fix: font-aware density check in `is_multi_column_page` rejects
/// sparse fax layouts; row-aware sort runs as before.
#[test]
fn fixture_fax_scattered_fragments_no_reversal() {
    // 7 rows of 5 separate fragments each at scattered X.
    let lines: &[&[(f32, &str)]] = &[
        &[(72.0, "WORDONE"), (140.0, "WORDTWO"), (190.0, "WORDTHREE"), (245.0, "WORDFOUR"), (300.0, "WORDFIVE")],
        &[(72.0, "ALPHA"), (115.0, "BETA"), (155.0, "GAMMA"), (210.0, "DELTA"), (260.0, "EPSILON")],
        &[(72.0, "First"), (105.0, "Second"), (150.0, "Third"), (195.0, "Fourth"), (245.0, "Fifth")],
        &[(72.0, "PartA"), (105.0, "PartB"), (140.0, "PartC"), (185.0, "PartD"), (235.0, "PartE")],
        &[(72.0, "ItemX"), (115.0, "ItemY"), (155.0, "ItemZ"), (195.0, "ItemW"), (245.0, "ItemV")],
        &[(72.0, "Quux"), (110.0, "Garply"), (160.0, "Waldo"), (210.0, "Fred"), (250.0, "Plugh")],
        &[(72.0, "Foo"), (95.0, "Bar"), (125.0, "Baz"), (160.0, "Qux"), (200.0, "Corge")],
    ];

    let out = build_and_extract(|w| {
        let mut page = w.add_letter_page();
        let mut y = PAGE_H - MARGIN;
        for line_frags in lines {
            for (x, word) in line_frags.iter() {
                put(&mut page, word, *x, y, "Helvetica", 8.0);
            }
            y -= 11.0;
        }
    });

    for bad in &[
        "WORDFIVEWORDFOURWORDTHREE",
        "WORDFOURWORDTHREEWORDTWOWORDONE",
        "EPSILONDELTAGAMMABETAALPHA",
        "FifthFourthThirdSecondFirst",
    ] {
        assert!(
            !out.contains(bad),
            "Reversal artifact `{bad}` detected in output:\n{out}"
        );
    }

    let one = out.find("WORDONE").expect("WORDONE missing");
    let five = out.find("WORDFIVE").expect("WORDFIVE missing");
    assert!(
        one < five,
        "Reading order reversed: WORDONE at {one}, WORDFIVE at {five}\n--- output ---\n{out}"
    );

    let alpha = out.find("ALPHA").expect("ALPHA missing");
    let epsilon = out.find("EPSILON").expect("EPSILON missing");
    assert!(
        alpha < epsilon,
        "Reading order reversed on Greek-letter line\n--- output ---\n{out}"
    );
}

/// `table_simple_3x3.pdf` — a 3-column table where the same word
/// appears stacked in three vertical cells. Vertical gap between cells
/// is ≈ 2 × line_height.
///
/// Bug on current HEAD: vertically-stacked cells get concatenated with
/// no separator, producing `instancesinstancesinstances` instead of
/// three separate occurrences.
///
/// Fix: span-emitter inserts `\n` when vertical gap > 0.7 × line_height.
#[test]
fn fixture_table_3x3_cells_not_concatenated() {
    let cell_text = "instances";
    let cols = [120.0_f32, 250.0, 380.0];
    let rows = [PAGE_H - MARGIN, PAGE_H - MARGIN - 30.0, PAGE_H - MARGIN - 60.0];

    let out = build_and_extract(|w| {
        let mut page = w.add_letter_page();
        for &x in &cols {
            for &y in &rows {
                put(&mut page, cell_text, x, y, "Helvetica", BODY);
            }
        }
    });

    assert!(
        !out.contains("instancesinstancesinstances"),
        "Adjacent cells concatenated without separator:\n{out}"
    );
    let occurrences = out.matches("instances").count();
    assert!(
        occurrences >= 9,
        "Expected ≥9 occurrences of cell text, found {occurrences}\n--- output ---\n{out}"
    );
}

/// `body_two_column_hyphenated.pdf` — two columns where some lines
/// end with `-` to indicate the word continues on the next line.
///
/// Bug on current HEAD: `cross-` + newline + `collateralized` collapses
/// into `crosscollateralized` (hyphen dropped, no space, no proper rejoin).
///
/// Fix: end-of-line hyphen rejoin — strip the trailing `-` and join the
/// next-line continuation.
#[test]
fn fixture_two_column_hyphen_rejoin() {
    let mut left: Vec<String> = (1..=10).map(|i| format!("LeftPad{i:02} body text")).collect();
    left.push("Continuation requires compre-".to_string());
    left.push("hensive understanding here.".to_string());
    left.extend((13..=30).map(|i| format!("LeftPad{i:02} body text")));

    let mut right: Vec<String> = (1..=10).map(|i| format!("RightPad{i:02} other text")).collect();
    right.push("Some financial terms cross-".to_string());
    right.push("collateralized in detail.".to_string());
    right.extend((13..=30).map(|i| format!("RightPad{i:02} other text")));

    let out = build_and_extract(|w| {
        let mut page = w.add_letter_page();
        place_column(&mut page, MARGIN, PAGE_H - MARGIN, &left);
        place_column(&mut page, 360.0, PAGE_H - MARGIN, &right);
    });

    let comp_joined = out.contains("comprehensive") || out.contains("compre-hensive");
    let cross_joined = out.contains("crosscollateralized") || out.contains("cross-collateralized");
    assert!(
        comp_joined,
        "End-of-line hyphen for `compre-`/`hensive` not rejoined\n--- output ---\n{out}"
    );
    assert!(
        cross_joined,
        "End-of-line hyphen for `cross-`/`collateralized` not rejoined\n--- output ---\n{out}"
    );
}

/// `body_with_figure_caption.pdf` — body text in 12pt with a figure
/// caption in 9pt embedded mid-column. The caption is a distinct
/// region and must not concatenate into the body sentence.
///
/// Bug on current HEAD: caption's first word gets sucked into the body,
/// producing tokens like `conceptFigure` where body word `concept` glues
/// to caption word `Figure`.
///
/// Fix: distinct-region detection — a span whose font_size differs from
/// its neighbours by > 25% AND has a different font_id is treated as a
/// region break (`\n\n` separator).
#[test]
fn fixture_figure_caption_not_merged_into_body() {
    let body_lines_pre = [
        "We describe the two dimensions through the concept",
        "outlined in the diagram presented further below,",
    ];
    let caption_line = "Figure 1: Synthetic illustration accompanying this fixture only.";
    let body_lines_post = [
        "which captures the relevant scaling properties and",
        "demonstrates the expected behaviour in this setting.",
    ];

    let out = build_and_extract(|w| {
        let mut page = w.add_letter_page();
        let mut y = PAGE_H - MARGIN;
        for line in &body_lines_pre {
            put(&mut page, line, MARGIN, y, "Helvetica", 12.0);
            y -= 14.4;
        }
        y -= 8.0;
        put(&mut page, caption_line, MARGIN, y, "Helvetica-Oblique", 9.0);
        y -= 14.0;
        for line in &body_lines_post {
            put(&mut page, line, MARGIN, y, "Helvetica", 12.0);
            y -= 14.4;
        }
    });

    for bad in &[
        "conceptFigure",
        "conceptSynthetic",
        "belowFigure",
        "belowSynthetic",
        "conceptIllustration",
    ] {
        assert!(
            !out.contains(bad),
            "Body word collided with caption word (`{bad}`):\n{out}"
        );
    }

    let concept = out.find("concept").expect("concept missing");
    let scaling = out.find("scaling").expect("scaling missing");
    assert!(
        concept < scaling,
        "Body sentence broken: concept at {concept}, scaling at {scaling}\n--- output ---\n{out}"
    );
}
