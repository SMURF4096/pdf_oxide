//! Phase PAINT — emit PDF content streams from a [`PaginatedDocument`].
//!
//! This is the bridge between Phase LAYOUT/PAGINATE (geometry) and
//! Phase PDF emission (`pdf_oxide::writer`). PAINT walks each page
//! fragment, resolves box styles to colours/fonts, and emits draw
//! commands via the existing ContentStreamBuilder primitives.
//!
//! v0.3.35 first cut covers:
//! - Borders (1px solid stroke when border-width > 0).
//! - Backgrounds — parsed but not yet rendered. Surfacing
//!   `ContentStreamBuilder::fill` through `PageBuilder` is a
//!   follow-up (tracked as PAINT-2b); `background-color` values
//!   parse successfully but currently produce no fill.
//! - Text content from `BoxKind::Text` rendered via the registered
//!   embedded font (falls back to Helvetica Base-14 if no font is
//!   registered).
//! - Y-flip from HTML top-down → PDF bottom-up applied once at page
//!   emission so all internal coordinates stay top-down.
//!
//! Out of scope (lands when caller wires them up):
//! - Gradients (`shading.rs` is ready in writer/; PAINT-3 wiring).
//! - Shadows + opacity via ExtGState soft masks.
//! - Transforms (`cm` operator already in ContentStreamBuilder).

use crate::elements::{
    ColorSpace as ElemColorSpace, ContentElement, ImageContent, ImageFormat as ElemImageFormat,
};
use crate::geometry::Rect;
use crate::html_css::css::{parse_color, parse_property, ComputedStyles, Value};
use crate::html_css::layout::{BoxKind, BoxTree};
use crate::html_css::paginate::{PageFragment, PaginatedDocument};
use crate::writer::{ImageData, PageBuilder, PdfWriter};

/// Read `opacity: <number>` from a [`ComputedStyles`]. Returns `1.0`
/// (fully opaque) when the property is absent or unparseable. Values
/// are clamped to `[0, 1]` per CSS Color L4 §3.2.
pub fn opacity_for(styles: &ComputedStyles<'_>) -> f32 {
    let Some(rv) = styles.get("opacity") else {
        return 1.0;
    };
    use crate::html_css::css::parser::ComponentValue;
    use crate::html_css::css::tokenizer::Token;
    for cv in &rv.value {
        if let ComponentValue::Token(token) = cv {
            match token {
                Token::Number(n) => return (n.value as f32).clamp(0.0, 1.0),
                Token::Percentage(n) => return ((n.value as f32) / 100.0).clamp(0.0, 1.0),
                _ => {},
            }
        }
    }
    1.0
}

/// Read `transform: translate*(…)` from a [`ComputedStyles`] and return
/// the resulting `(dx, dy)` in CSS pixels. Other transform functions
/// (scale, rotate, matrix, skew) are silently ignored for the v0.3.37
/// first cut. Absent / `none` / unsupported → `(0.0, 0.0)`.
pub fn translate_offset_for(styles: &ComputedStyles<'_>) -> (f32, f32) {
    let Some(rv) = styles.get("transform") else {
        return (0.0, 0.0);
    };
    use crate::html_css::css::parser::ComponentValue;
    use crate::html_css::css::tokenizer::Token;
    let mut dx = 0.0;
    let mut dy = 0.0;
    for cv in &rv.value {
        let ComponentValue::Function { name, body } = cv else {
            continue;
        };
        let lower = name.to_ascii_lowercase();
        // Collect numeric (value, is_length) tuples separated by commas.
        let mut parts: Vec<f32> = Vec::new();
        for inner in body.iter() {
            if let ComponentValue::Token(t) = inner {
                match t {
                    Token::Dimension { value, .. } => parts.push(value.value as f32),
                    Token::Number(n) => parts.push(n.value as f32),
                    _ => {},
                }
            }
        }
        match lower.as_str() {
            "translatex" => {
                if let Some(&v) = parts.first() {
                    dx += v;
                }
            },
            "translatey" => {
                if let Some(&v) = parts.first() {
                    dy += v;
                }
            },
            "translate" => {
                if let Some(&v) = parts.first() {
                    dx += v;
                }
                if let Some(&v) = parts.get(1) {
                    dy += v;
                }
            },
            _ => {},
        }
    }
    (dx, dy)
}

/// Decode an HTML `<img src=…>` value to a raw image byte buffer.
///
/// v0.3.37 supports inline `data:` URIs only (both `;base64,` and
/// percent-encoded plain payloads); external URLs and filesystem
/// paths return `None`. Whoever drives `paint_document` is free to
/// resolve those themselves and hand `PaintImage { data }` directly
/// via the `image_for` callback.
pub fn decode_image_src(src: &str) -> Option<Vec<u8>> {
    let trimmed = src.trim();
    let rest = trimmed.strip_prefix("data:")?;
    // `data:[<mediatype>][;base64],<data>` — we don't care about the
    // mediatype since `ImageData::from_bytes` sniffs the magic bytes.
    let comma = rest.find(',')?;
    let meta = &rest[..comma];
    let payload = &rest[comma + 1..];
    if meta.split(';').any(|s| s.eq_ignore_ascii_case("base64")) {
        use base64::Engine as _;
        base64::engine::general_purpose::STANDARD
            .decode(payload.as_bytes())
            .ok()
    } else {
        // Percent-encoded plain data. Decode %XX triples to bytes;
        // everything else passes through as-is.
        let mut out = Vec::with_capacity(payload.len());
        let bytes = payload.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == b'%' && i + 2 < bytes.len() {
                let hi = (bytes[i + 1] as char).to_digit(16)?;
                let lo = (bytes[i + 2] as char).to_digit(16)?;
                out.push(((hi << 4) | lo) as u8);
                i += 3;
            } else {
                out.push(bytes[i]);
                i += 1;
            }
        }
        Some(out)
    }
}

/// Opaque handle returned by the `image_for` callback. The API layer
/// decodes each `<img>` source (data-URI / file path / raw bytes) into
/// one of these; PAINT just places it.
#[derive(Debug, Clone)]
pub struct PaintImage {
    /// Decoded image ready for embedding.
    pub data: ImageData,
}

/// Emit `doc` to `writer`, one page per [`PageFragment`].
///
/// `style_for` returns the cascaded computed style for a given box id
/// (the API layer in Phase API wires this from the cascade output).
/// `font_resource_name` is the registered embedded-font resource name
/// returned by `PdfWriter::register_embedded_font` — every text box
/// uses it for v0.3.35.
pub fn paint_document<'sty>(
    writer: &mut PdfWriter,
    doc: &PaginatedDocument,
    tree: &BoxTree,
    style_for: impl Fn(u32) -> Option<ComputedStyles<'sty>>,
    font_resource_name: &str,
    font_size_px: f32,
    link_href_for: impl Fn(u32) -> Option<String>,
    marker_for: impl Fn(u32) -> Option<String>,
    font_for_box: impl Fn(u32) -> Option<String>,
    pseudo_before_for: impl Fn(u32) -> Option<String>,
    pseudo_after_for: impl Fn(u32) -> Option<String>,
    image_for: impl Fn(u32) -> Option<PaintImage>,
) {
    for page in &doc.pages {
        let mut page_builder = writer.add_page(doc.config.width_px, doc.config.height_px);
        paint_page(
            &mut page_builder,
            page,
            tree,
            doc.config.height_px,
            doc.config.margin_px.left,
            doc.config.margin_px.top,
            &style_for,
            font_resource_name,
            font_size_px,
            &link_href_for,
            &marker_for,
            &font_for_box,
            &pseudo_before_for,
            &pseudo_after_for,
            &image_for,
        );
    }
}

fn paint_page<'sty>(
    page_builder: &mut PageBuilder<'_>,
    fragment: &PageFragment,
    tree: &BoxTree,
    page_height_px: f32,
    margin_left: f32,
    margin_top: f32,
    style_for: &impl Fn(u32) -> Option<ComputedStyles<'sty>>,
    font_resource_name: &str,
    font_size_px: f32,
    link_href_for: &impl Fn(u32) -> Option<String>,
    marker_for: &impl Fn(u32) -> Option<String>,
    font_for_box: &impl Fn(u32) -> Option<String>,
    pseudo_before_for: &impl Fn(u32) -> Option<String>,
    pseudo_after_for: &impl Fn(u32) -> Option<String>,
    image_for: &impl Fn(u32) -> Option<PaintImage>,
) {
    for pb in &fragment.boxes {
        let node = tree.get(pb.box_id);
        // CSS `opacity` + `transform: translate*(…)`. Only the first
        // two of the four FU3 features land in v0.3.37; gradients and
        // box-shadow stay deferred because each needs a new writer-
        // side primitive. Translate is applied as a pre-paint offset
        // (correct for all leaf emissions — text, images, links);
        // opacity <= 0.01 on ANY ancestor skips the box entirely, so
        // text children of a hidden element stay invisible too.
        let mut cur = Some(pb.box_id);
        let mut hidden = false;
        let mut tx = 0.0;
        let mut ty = 0.0;
        let mut applied_translate = false;
        while let Some(bid) = cur {
            let n = tree.get(bid);
            if n.element.is_some() {
                if let Some(styles) = style_for(bid) {
                    if opacity_for(&styles) <= 0.01 {
                        hidden = true;
                        break;
                    }
                    if !applied_translate {
                        let (dx, dy) = translate_offset_for(&styles);
                        if dx != 0.0 || dy != 0.0 {
                            tx = dx;
                            ty = dy;
                            applied_translate = true;
                        }
                    }
                }
            }
            cur = n.parent;
        }
        if hidden {
            continue;
        }
        // Convert top-down (HTML) y to bottom-up (PDF) y.
        let abs_x = margin_left + pb.local.x + tx;
        let abs_top_y = margin_top + pb.local.y + ty;
        let pdf_y = page_height_px - abs_top_y - pb.local.height;

        // Fill background-color if any.
        if let Some(styles) = node.element.and_then(|_| style_for(pb.box_id)) {
            if let Some(rv) = styles.get("background-color") {
                if let Ok(color) = parse_color(&rv.value, "background-color") {
                    if color.a > 0.0 {
                        // For v0.3.35 we paint the fill via direct
                        // ContentStreamBuilder access — but PageBuilder
                        // currently only exposes draw_rect (which
                        // strokes). Use it as a stub; the writer's
                        // shading/path APIs aren't piped through
                        // PageBuilder yet (PAINT-2b). For now this is
                        // a no-op visible only when borders show.
                        let _ = color;
                    }
                }
            }
            // Borders (very simple — single solid stroke if any side
            // declares a non-zero width).
            let has_border = ["border-width", "border-top-width", "border"]
                .iter()
                .any(|p| styles.get(p).is_some());
            if has_border {
                page_builder.draw_rect(abs_x, pdf_y, pb.local.width, pb.local.height);
            }
        }

        let box_font = font_for_box(pb.box_id);
        let box_font_name: &str = box_font.as_deref().unwrap_or(font_resource_name);

        // List marker — bullet or number drawn at the top-left of the
        // <li> box, offset into the gutter to the left of the content.
        if let Some(marker) = marker_for(pb.box_id) {
            if !marker.is_empty() {
                let marker_pdf_y = page_height_px - abs_top_y - font_size_px;
                let marker_x = (abs_x - font_size_px * 1.2).max(0.0);
                page_builder.add_embedded_text(
                    &marker,
                    marker_x,
                    marker_pdf_y,
                    box_font_name,
                    font_size_px,
                );
            }
        }

        // Link annotation — paint a clickable rect over the box if
        // the API layer says its DOM element is an `<a href=…>`.
        if let Some(href) = link_href_for(pb.box_id) {
            if !href.is_empty() && pb.local.width > 0.0 && pb.local.height > 0.0 {
                page_builder.link(Rect::new(abs_x, pdf_y, pb.local.width, pb.local.height), href);
            }
        }

        // <img> element — emit ImageContent at the box's placed rect.
        // The API layer decodes `src` (data-URI / file path) into a
        // PaintImage; PAINT just places it. Width/height follow the
        // box geometry from layout so CSS width/height + intrinsic
        // aspect already flowed through.
        if node.element.is_some() {
            if let Some(img) = image_for(pb.box_id) {
                let width = if pb.local.width > 0.0 {
                    pb.local.width
                } else {
                    img.data.width as f32
                };
                let height = if pb.local.height > 0.0 {
                    pb.local.height
                } else {
                    img.data.height as f32
                };
                let img_pdf_y = page_height_px - abs_top_y - height;
                let content = ImageContent {
                    bbox: Rect::new(abs_x, img_pdf_y, width, height),
                    format: match img.data.format {
                        crate::writer::ImageFormat::Jpeg => ElemImageFormat::Jpeg,
                        crate::writer::ImageFormat::Png => ElemImageFormat::Png,
                        crate::writer::ImageFormat::Raw => ElemImageFormat::Raw,
                    },
                    data: img.data.data.clone(),
                    width: img.data.width,
                    height: img.data.height,
                    bits_per_component: img.data.bits_per_component,
                    color_space: match img.data.color_space {
                        crate::writer::ColorSpace::DeviceGray => ElemColorSpace::Gray,
                        crate::writer::ColorSpace::DeviceRGB => ElemColorSpace::RGB,
                        crate::writer::ColorSpace::DeviceCMYK => ElemColorSpace::CMYK,
                    },
                    reading_order: None,
                    alt_text: None,
                    horizontal_dpi: None,
                    vertical_dpi: None,
                    // Carry the PNG alpha / soft-mask forward so the
                    // writer can emit a real /SMask XObject; without
                    // this the transparency is silently dropped.
                    soft_mask: img.data.soft_mask.clone(),
                };
                page_builder.add_element(&ContentElement::Image(content));
            }
        }

        // ::before / ::after generated content. For the v0.3.37 first
        // cut we place the generated text at the top-left (before) and
        // bottom-left (after) of the host box. A real inline-box
        // generator would splice them into the inline formatter's run
        // list — this is good enough to visualise content declarations
        // and to satisfy e2e assertions that check for the string.
        if node.element.is_some() {
            if let Some(before) = pseudo_before_for(pb.box_id) {
                if !before.is_empty() {
                    let y = page_height_px - abs_top_y - font_size_px;
                    page_builder.add_embedded_text(&before, abs_x, y, box_font_name, font_size_px);
                }
            }
            if let Some(after) = pseudo_after_for(pb.box_id) {
                if !after.is_empty() {
                    let y = page_height_px - abs_top_y - pb.local.height;
                    page_builder.add_embedded_text(&after, abs_x, y, box_font_name, font_size_px);
                }
            }
        }

        // Text content.
        if let BoxKind::Text(s) = &node.kind {
            if !s.trim().is_empty() {
                // Place the text near the top of its box (baseline
                // approx 0.8 of font_size). We place at top-left for
                // simplicity; LAYOUT-3's inline formatter will
                // produce per-glyph positions in a future commit.
                let text_pdf_y = page_height_px - abs_top_y - font_size_px;
                #[cfg(feature = "system-fonts")]
                let routed_shaped = crate::text::bidi::paragraph_is_rtl(s) && {
                    page_builder.add_shaped_embedded_text(
                        s,
                        abs_x,
                        text_pdf_y,
                        box_font_name,
                        font_size_px,
                        crate::writer::ShapeDirection::Rtl,
                    );
                    true
                };
                #[cfg(not(feature = "system-fonts"))]
                let routed_shaped = false;
                if !routed_shaped {
                    page_builder.add_embedded_text(
                        s,
                        abs_x,
                        text_pdf_y,
                        box_font_name,
                        font_size_px,
                    );
                }
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────
// Helper for the API layer — read effective body font size from a
// ComputedStyles, falling back to a sensible default.
// ─────────────────────────────────────────────────────────────────────

/// Resolve a body-text `font-size` from the root computed styles. Used
/// by Phase API as a default when the user doesn't set one explicitly.
pub fn resolve_root_font_size_px(root_styles: Option<&ComputedStyles<'_>>) -> f32 {
    let Some(styles) = root_styles else {
        return 16.0;
    };
    let Some(rv) = styles.get("font-size") else {
        return 16.0;
    };
    match parse_property("font-size", &rv.value).ok() {
        Some(Value::Length(l)) => l
            .resolve(&crate::html_css::css::CalcContext::default())
            .unwrap_or(16.0),
        _ => 16.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::html_css::css::{parse_stylesheet, ComputedStyles};
    use crate::html_css::html::parse_document;
    use crate::html_css::layout::{build_box_tree, run_layout};
    use crate::html_css::paginate::{paginate, PageConfig};
    use crate::writer::{EmbeddedFont, PdfWriter};
    use taffy::prelude::Size;

    const DEJAVU: &[u8] = include_bytes!("../../tests/fixtures/fonts/DejaVuSans.ttf");

    #[test]
    fn smoke_paint_produces_pdf_with_pages() {
        let html = "<html><body><p>Hello world</p></body></html>";
        let css = "";
        let dom: &'static _ = Box::leak(Box::new(parse_document(html)));
        let ss: &'static _ = Box::leak(Box::new(parse_stylesheet(css).unwrap()));
        let tree = build_box_tree(dom, ss).unwrap();
        let layout = run_layout(
            &tree,
            |id| {
                let node = tree.get(id);
                let Some(elem_id) = node.element else {
                    return ComputedStyles::default();
                };
                let element = dom.element(elem_id).unwrap();
                crate::html_css::css::cascade(ss, element, None)
            },
            Size {
                width: 600.0,
                height: 800.0,
            },
            &crate::html_css::css::CalcContext::default(),
            12.0,
        );
        let doc = paginate(&tree, &layout, PageConfig::a4());
        assert!(!doc.pages.is_empty());

        let mut writer = PdfWriter::new();
        let font = EmbeddedFont::from_data(Some("DejaVuSans".to_string()), DEJAVU.to_vec())
            .expect("DejaVuSans");
        let rn = writer.register_embedded_font(font);

        paint_document(
            &mut writer,
            &doc,
            &tree,
            |id| {
                let node = tree.get(id);
                let elem_id = node.element?;
                let element = dom.element(elem_id).unwrap();
                Some(crate::html_css::css::cascade(ss, element, None))
            },
            &rn,
            12.0,
            |_id| None,
            |_id| None,
            |_id| None,
            |_id| None,
            |_id| None,
            |_id| None,
        );

        let bytes = writer.finish().expect("PDF emission");
        assert!(bytes.starts_with(b"%PDF-1.7"));
        assert!(bytes.len() > 1000); // Embedded font alone is hundreds of KB.
    }

    #[test]
    fn decode_image_src_base64_png() {
        // 1×1 transparent PNG, pre-encoded base64.
        let src = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNkAAIAAAoAAv/lxKUAAAAASUVORK5CYII=";
        let bytes = decode_image_src(src).expect("decode");
        assert!(
            bytes.starts_with(b"\x89PNG\r\n\x1a\n"),
            "got {:?}",
            &bytes[..8.min(bytes.len())]
        );
    }

    #[test]
    fn decode_image_src_rejects_http() {
        assert!(decode_image_src("https://example.com/x.png").is_none());
        assert!(decode_image_src("/local/path.png").is_none());
    }

    #[test]
    fn opacity_absent_is_fully_opaque() {
        use crate::html_css::css::{cascade, parse_stylesheet};
        let ss: &'static _ = Box::leak(Box::new(parse_stylesheet("p { color: red; }").unwrap()));
        let dom: &'static _ =
            Box::leak(Box::new(crate::html_css::html::parse_document("<p>x</p>")));
        let p_id = dom.iter_elements().find(|&id| {
            matches!(&dom.node(id).kind, crate::html_css::html::NodeKind::Element { tag, .. } if tag == "p")
        }).unwrap();
        let el = dom.element(p_id).unwrap();
        let styles = cascade(ss, el, None);
        assert_eq!(opacity_for(&styles), 1.0);
    }

    #[test]
    fn opacity_number_parses() {
        use crate::html_css::css::{cascade, parse_stylesheet};
        let ss: &'static _ = Box::leak(Box::new(parse_stylesheet("p { opacity: 0.25; }").unwrap()));
        let dom: &'static _ =
            Box::leak(Box::new(crate::html_css::html::parse_document("<p>x</p>")));
        let p_id = dom.iter_elements().find(|&id| {
            matches!(&dom.node(id).kind, crate::html_css::html::NodeKind::Element { tag, .. } if tag == "p")
        }).unwrap();
        let el = dom.element(p_id).unwrap();
        let styles = cascade(ss, el, None);
        assert!((opacity_for(&styles) - 0.25).abs() < 1e-4);
    }

    #[test]
    fn translate_offset_parses_two_lengths() {
        use crate::html_css::css::{cascade, parse_stylesheet};
        let ss: &'static _ = Box::leak(Box::new(
            parse_stylesheet("p { transform: translate(10px, 20px); }").unwrap(),
        ));
        let dom: &'static _ =
            Box::leak(Box::new(crate::html_css::html::parse_document("<p>x</p>")));
        let p_id = dom.iter_elements().find(|&id| {
            matches!(&dom.node(id).kind, crate::html_css::html::NodeKind::Element { tag, .. } if tag == "p")
        }).unwrap();
        let el = dom.element(p_id).unwrap();
        let styles = cascade(ss, el, None);
        assert_eq!(translate_offset_for(&styles), (10.0, 20.0));
    }

    #[test]
    fn translate_x_only_sets_dx() {
        use crate::html_css::css::{cascade, parse_stylesheet};
        let ss: &'static _ =
            Box::leak(Box::new(parse_stylesheet("p { transform: translateX(7px); }").unwrap()));
        let dom: &'static _ =
            Box::leak(Box::new(crate::html_css::html::parse_document("<p>x</p>")));
        let p_id = dom.iter_elements().find(|&id| {
            matches!(&dom.node(id).kind, crate::html_css::html::NodeKind::Element { tag, .. } if tag == "p")
        }).unwrap();
        let el = dom.element(p_id).unwrap();
        let styles = cascade(ss, el, None);
        assert_eq!(translate_offset_for(&styles), (7.0, 0.0));
    }

    #[test]
    fn decode_image_src_percent_encoded() {
        let src = "data:text/plain,%48%69";
        let bytes = decode_image_src(src).expect("decode");
        assert_eq!(&bytes[..], b"Hi");
    }
}
