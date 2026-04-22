//! Validation for mozilla/pdf.js#19978 — a yellow rectangle with an
//! alpha channel that pdf.js rendered as solid (masking the text
//! beneath). Fixture came straight from the issue attachment
//! (user-attachments/files/20496364/538250-1.pdf).
//!
//! pdf_oxide's rendering path goes through tiny-skia + `qcms`, both of
//! which are alpha-aware. This test pins two properties:
//!
//! 1. Rendering the fixture doesn't error out (regression guard —
//!    several decoders historically mishandle the page's transparency
//!    group / /CA soft-mask state).
//! 2. The rendered page contains visible output (non-zero bytes after
//!    the PNG header), so the regression isn't "renders a blank page
//!    instead".
//!
//! The "text under the yellow bar should still be readable" claim
//! from the bug is ultimately an OCR-level assertion that tiny-skia's
//! alpha blending already satisfies; if we ever regress it, the text
//! becomes invisible and extract_text of the rendered region would go
//! empty. That stronger assertion is cheap to add here but requires
//! the OCR feature, which this fixture-level test keeps optional.

#![cfg(feature = "rendering")]

use pdf_oxide::rendering::{render_page, RenderOptions};
use pdf_oxide::PdfDocument;

const FIXTURE: &str = "tests/fixtures/issue_regressions/alpha_channel/538250-1.pdf";

#[test]
fn renders_without_error() {
    let mut doc = PdfDocument::open(FIXTURE).expect("open fixture");
    let opts = RenderOptions::with_dpi(96);
    let img = render_page(&mut doc, 0, &opts).expect("render page 0");
    assert!(img.width > 0);
    assert!(img.height > 0);
    // At least some pixels must have been written — a blank-page
    // regression would leave a small, near-empty buffer.
    assert!(
        img.data.len() > 1024,
        "rendered image only {} bytes — likely blank-page regression",
        img.data.len()
    );
}

/// pdf.js#19978 parity: the yellow bar uses a Multiply ExtGState, so
/// CCITT text underneath must show through as non-yellow intermediate
/// shades, not be flattened to solid yellow. Without image filtering
/// the bilevel CCITT source downsampled to the render resolution with
/// nearest-neighbour collapsed every intermediate tone; the renderer
/// now uses Bicubic filtering for image XObjects (see
/// `page_renderer.rs::render_image`), restoring the full shade range.
#[test]
fn alpha_preserved_has_non_yellow_pixels() {
    // If the yellow bar rendered as solid, the whole top band would be
    // ~pure yellow (#FFFF00). Blended against white page background,
    // a semi-transparent yellow produces a range of intermediate hues.
    // This test checks that the rendered image has at least 100
    // distinct colours in the top 20% of the page — a solid bar would
    // collapse that to a handful.
    let mut doc = PdfDocument::open(FIXTURE).expect("open fixture");
    // PNG is the default output format for RenderOptions.
    let opts = RenderOptions::with_dpi(96);
    let img = render_page(&mut doc, 0, &opts).expect("render page 0");

    // Decode the PNG bytes so we can inspect pixels.
    let cursor = std::io::Cursor::new(&img.data);
    let decoded =
        image::load(cursor, image::ImageFormat::Png).expect("decode rendered PNG");
    let rgba = decoded.to_rgba8();

    let top_band_height = rgba.height() / 5;
    let mut colours = std::collections::HashSet::new();
    for y in 0..top_band_height {
        for x in 0..rgba.width() {
            let p = rgba.get_pixel(x, y);
            colours.insert((p[0], p[1], p[2]));
            if colours.len() > 100 {
                return;
            }
        }
    }
    panic!(
        "top 20% of page only has {} distinct colours — looks like the yellow bar rendered solid",
        colours.len()
    );
}
