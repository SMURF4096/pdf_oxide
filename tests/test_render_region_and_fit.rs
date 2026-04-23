//! Exercise the `rendering::render_page_region` and
//! `rendering::render_page_fit` entry points.
#![cfg(feature = "rendering")]

use pdf_oxide::api::Pdf;
use pdf_oxide::document::PdfDocument;
use pdf_oxide::rendering::{render_page, render_page_fit, render_page_region, RenderOptions};

fn setup() -> PdfDocument {
    let bytes = Pdf::from_text("region fit probe").unwrap().into_bytes();
    PdfDocument::from_bytes(bytes).unwrap()
}

fn is_png(b: &[u8]) -> bool {
    b.len() >= 8 && b.starts_with(&[0x89, 0x50, 0x4e, 0x47])
}

#[test]
fn render_page_region_returns_clipped_png() {
    let mut doc = setup();
    let full = render_page(&mut doc, 0, &RenderOptions::with_dpi(72)).unwrap();
    let region = render_page_region(
        &mut doc,
        0,
        (36.0, 36.0, 144.0, 144.0), // 2"×2" crop at (0.5", 0.5")
        &RenderOptions::with_dpi(72),
    )
    .unwrap();

    assert!(is_png(&region.data));
    assert!(region.width < full.width);
    assert!(region.height < full.height);
    assert!(region.width > 0 && region.height > 0);
}

#[test]
fn render_page_region_rejects_zero_rect() {
    let mut doc = setup();
    let err = render_page_region(&mut doc, 0, (0.0, 0.0, 0.0, 0.0), &RenderOptions::with_dpi(72));
    assert!(err.is_err(), "zero-area rect should fail");
}

#[test]
fn render_page_fit_respects_box() {
    let mut doc = setup();
    let img = render_page_fit(&mut doc, 0, 200, 100, &RenderOptions::with_dpi(72)).unwrap();
    assert!(is_png(&img.data));
    // Output must fit inside the box (plus rounding slack).
    assert!(img.width <= 200 + 5, "fit width {} > 200", img.width);
    assert!(img.height <= 100 + 5, "fit height {} > 100", img.height);
}

#[test]
fn render_page_fit_rejects_zero_box() {
    let mut doc = setup();
    assert!(render_page_fit(&mut doc, 0, 0, 100, &RenderOptions::with_dpi(72)).is_err());
    assert!(render_page_fit(&mut doc, 0, 100, 0, &RenderOptions::with_dpi(72)).is_err());
}
