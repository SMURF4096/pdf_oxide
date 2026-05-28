//! Separation plate renderer.
//!
//! Renders individual ink separation plates as grayscale images where
//! pixel intensity represents the tint percentage of that ink at each point.
//! Used in prepress workflows, ink coverage analysis, and ML pipelines
//! that process packaging/label PDFs.
#![allow(
    clippy::field_reassign_with_default,
    clippy::ptr_arg,
    clippy::only_used_in_recursion
)]

use std::collections::HashMap;

use tiny_skia::{FillRule, Mask, PathBuilder, Pixmap, Transform};

use crate::content::graphics_state::{GraphicsState, GraphicsStateStack, Matrix};
use crate::content::operators::Operator;
use crate::content::parser::parse_content_stream;
use crate::document::PdfDocument;
use crate::error::{Error, Result};
use crate::object::Object;

/// A rendered separation plate for a single ink.
#[derive(Debug, Clone)]
pub struct SeparationPlate {
    /// Ink name (e.g., "Cyan", "PANTONE 185 C", "Dieline").
    pub ink_name: String,
    /// Grayscale pixel data, row-major, top-left origin.
    /// 0 = no ink, 255 = full tint. `data.len() == width * height`.
    pub data: Vec<u8>,
    /// Image width in pixels.
    pub width: u32,
    /// Image height in pixels.
    pub height: u32,
}

/// Render all separation plates for a page.
///
/// Returns one `SeparationPlate` per ink found on the page (process CMYK
/// inks are included when the page uses DeviceCMYK content). Each plate is
/// a grayscale image where pixel intensity = tint percentage of that ink.
pub fn render_separations(
    doc: &PdfDocument,
    page_num: usize,
    dpi: u32,
) -> Result<Vec<SeparationPlate>> {
    let inks = collect_page_inks(doc, page_num)?;
    if inks.is_empty() {
        return Ok(Vec::new());
    }

    let mut plates = Vec::with_capacity(inks.len());
    for ink in &inks {
        let plate = render_single_separation(doc, page_num, ink, dpi)?;
        plates.push(plate);
    }
    Ok(plates)
}

/// Render a single ink separation plate for a page.
///
/// Returns a grayscale image where pixel intensity = tint percentage
/// of the named ink. If the ink is not present on the page, the plate
/// is all zeros.
pub fn render_separation(
    doc: &PdfDocument,
    page_num: usize,
    ink_name: &str,
    dpi: u32,
) -> Result<SeparationPlate> {
    render_single_separation(doc, page_num, ink_name, dpi)
}

/// Collect all ink names present on a page, including process CMYK.
fn collect_page_inks(doc: &PdfDocument, page_num: usize) -> Result<Vec<String>> {
    let mut inks = Vec::new();

    // Check for DeviceCMYK content by scanning the content stream
    let content_data = doc.get_page_content_data(page_num)?;
    let operators = parse_content_stream(&content_data)?;
    let has_cmyk = has_cmyk_content(&operators);

    if has_cmyk {
        inks.extend_from_slice(&[
            "Cyan".to_string(),
            "Magenta".to_string(),
            "Yellow".to_string(),
            "Black".to_string(),
        ]);
    }

    // Get spot color inks from color space resources
    let spot_inks = doc.get_page_inks(page_num)?;
    for ink in spot_inks {
        if !inks.contains(&ink) {
            inks.push(ink);
        }
    }

    Ok(inks)
}

/// Check whether a content stream references DeviceCMYK content.
fn has_cmyk_content(operators: &[Operator]) -> bool {
    for op in operators {
        match op {
            Operator::SetFillCmyk { .. } | Operator::SetStrokeCmyk { .. } => return true,
            Operator::SetFillColorSpace { name } | Operator::SetStrokeColorSpace { name } => {
                if name == "DeviceCMYK" || name == "CMYK" {
                    return true;
                }
            },
            _ => {},
        }
    }
    false
}

/// Core rendering logic for a single separation plate.
///
/// This uses the page renderer to rasterize the full page once via
/// the standard RGBA pipeline, but instead of using the composited result,
/// we walk the operator stream ourselves to track which ink is active
/// for each drawing operation, then render only the operations that
/// contribute to the target ink into a grayscale buffer.
///
/// We implement this by:
/// 1. Computing page dimensions and transform (same as the normal renderer)
/// 2. Walking the operator stream to track color state
/// 3. For each path painting operation, checking if the current color
///    contributes to the target ink
/// 4. If so, painting the path into a grayscale pixmap with the tint value
fn render_single_separation(
    doc: &PdfDocument,
    page_num: usize,
    ink_name: &str,
    dpi: u32,
) -> Result<SeparationPlate> {
    let page_info = doc.get_page_info(page_num)?;
    let media_box = page_info.media_box;

    let rotation = page_info.rotation % 360;
    let (page_w, page_h) = if rotation == 90 || rotation == 270 {
        (media_box.height, media_box.width)
    } else {
        (media_box.width, media_box.height)
    };
    let scale = dpi as f32 / 72.0;
    let width = (page_w * scale).ceil() as u32;
    let height = (page_h * scale).ceil() as u32;

    let base_transform = match rotation {
        90 => Transform::from_translate(-media_box.x, -media_box.y)
            .post_concat(Transform::from_row(0.0, scale, scale, 0.0, 0.0, 0.0)),
        180 => Transform::from_translate(-media_box.x, -media_box.y)
            .post_scale(-scale, scale)
            .post_translate(media_box.width * scale, 0.0),
        270 => Transform::from_translate(-media_box.x, -media_box.y).post_concat(
            Transform::from_row(0.0, scale, -scale, 0.0, media_box.height * scale, 0.0),
        ),
        _ => Transform::from_translate(-media_box.x, -media_box.y)
            .post_scale(scale, -scale)
            .post_translate(0.0, page_h * scale),
    };

    // Create an RGBA pixmap for rasterization via tiny-skia.
    // We render in white-on-black: tint -> white channel (255 = full tint).
    let mut pixmap = Pixmap::new(width, height)
        .ok_or_else(|| Error::InvalidPdf("Failed to create separation pixmap".to_string()))?;

    // Load page resources and color spaces
    let resources = doc.get_page_resources(page_num)?;
    let color_spaces = load_color_spaces(doc, &resources)?;

    // Parse content stream
    let content_data = doc.get_page_content_data(page_num)?;
    let operators = parse_content_stream(&content_data)?;

    // Execute operators, painting only the target ink
    execute_separation_operators(
        &mut pixmap,
        base_transform,
        &operators,
        doc,
        page_num,
        &resources,
        &color_spaces,
        ink_name,
    )?;

    // Extract the grayscale channel from the RGBA pixmap.
    // We used R channel to carry tint values.
    let pixel_count = (width * height) as usize;
    let mut data = vec![0u8; pixel_count];
    let rgba = pixmap.data();
    for i in 0..pixel_count {
        // Red channel holds the tint value
        data[i] = rgba[i * 4];
    }

    Ok(SeparationPlate {
        ink_name: ink_name.to_string(),
        data,
        width,
        height,
    })
}

/// Load color space definitions from page resources.
fn load_color_spaces(doc: &PdfDocument, resources: &Object) -> Result<HashMap<String, Object>> {
    let mut color_spaces = HashMap::new();
    if let Object::Dictionary(res_dict) = resources {
        if let Some(cs_obj) = res_dict.get("ColorSpace") {
            let cs_dict_obj = doc.resolve_object(cs_obj)?;
            if let Some(cs_dict) = cs_dict_obj.as_dict() {
                for (name, o) in cs_dict {
                    if let Ok(resolved_cs) = doc.resolve_object(o) {
                        color_spaces.insert(name.clone(), resolved_cs);
                    }
                }
            }
        }
    }
    Ok(color_spaces)
}

/// Determine the tint value for the target ink given the current color state.
///
/// Returns `Some(tint)` where tint is 0.0..=1.0 if the current color
/// contributes to the target ink, or `None` if it does not.
fn tint_for_ink(
    fill: bool,
    gs: &GraphicsState,
    color_spaces: &HashMap<String, Object>,
    target_ink: &str,
    fill_components: &[f32],
    stroke_components: &[f32],
) -> Option<f32> {
    let space_name = if fill {
        &gs.fill_color_space
    } else {
        &gs.stroke_color_space
    };
    let components = if fill {
        fill_components
    } else {
        stroke_components
    };

    match space_name.as_str() {
        "DeviceCMYK" | "CMYK" => {
            let cmyk = if fill {
                gs.fill_color_cmyk
            } else {
                gs.stroke_color_cmyk
            };
            if let Some((c, m, y, k)) = cmyk {
                match target_ink {
                    "Cyan" => Some(c),
                    "Magenta" => Some(m),
                    "Yellow" => Some(y),
                    "Black" => Some(k),
                    _ => None,
                }
            } else {
                None
            }
        },
        "DeviceRGB" | "RGB" | "DeviceGray" | "G" => None,
        _ => {
            // Check resolved color space
            if let Some(cs_obj) = color_spaces.get(space_name) {
                if let Some(arr) = cs_obj.as_array() {
                    if let Some(type_name) = arr.first().and_then(|o| o.as_name()) {
                        match type_name {
                            "Separation" => {
                                let sep_ink = arr.get(1).and_then(|o| o.as_name()).unwrap_or("");
                                if sep_ink == target_ink && !components.is_empty() {
                                    Some(components[0])
                                } else {
                                    None
                                }
                            },
                            "DeviceN" => {
                                if let Some(Object::Array(ink_names)) = arr.get(1) {
                                    for (i, ink_obj) in ink_names.iter().enumerate() {
                                        if let Object::Name(ink) = ink_obj {
                                            if ink == target_ink && i < components.len() {
                                                return Some(components[i]);
                                            }
                                        }
                                    }
                                }
                                None
                            },
                            "ICCBased" if arr.len() > 1 => {
                                // ICCBased with N=4 maps to CMYK
                                // We need the doc to resolve the ICC stream dict, but since
                                // this is a hot path, we use a heuristic: if 4 components
                                // are present, treat as CMYK.
                                if components.len() >= 4 {
                                    match target_ink {
                                        "Cyan" => Some(components[0]),
                                        "Magenta" => Some(components[1]),
                                        "Yellow" => Some(components[2]),
                                        "Black" => Some(components[3]),
                                        _ => None,
                                    }
                                } else {
                                    None
                                }
                            },
                            _ => None,
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            }
        },
    }
}

/// Color state tracked alongside the graphics state for separation rendering.
#[derive(Clone, Debug)]
struct SeparationColorState {
    fill_components: Vec<f32>,
    stroke_components: Vec<f32>,
}

impl SeparationColorState {
    fn new() -> Self {
        Self {
            fill_components: Vec::new(),
            stroke_components: Vec::new(),
        }
    }
}

/// Execute operators for separation plate rendering.
fn execute_separation_operators(
    pixmap: &mut Pixmap,
    base_transform: Transform,
    operators: &[Operator],
    doc: &PdfDocument,
    page_num: usize,
    resources: &Object,
    color_spaces: &HashMap<String, Object>,
    target_ink: &str,
) -> Result<()> {
    let mut gs_stack = GraphicsStateStack::new();
    {
        let gs = gs_stack.current_mut();
        gs.fill_color_space = "DeviceGray".to_string();
        gs.stroke_color_space = "DeviceGray".to_string();
        gs.fill_color_rgb = (0.0, 0.0, 0.0);
        gs.stroke_color_rgb = (0.0, 0.0, 0.0);
    }

    let mut color_state_stack: Vec<SeparationColorState> = vec![SeparationColorState::new()];
    let mut current_path = PathBuilder::new();
    let mut pending_clip: Option<(tiny_skia::Path, FillRule)> = None;
    let mut clip_stack: Vec<Option<Mask>> = vec![None];

    // Pre-resolve ExtGState
    let ext_g_state_resolved: Option<Object> = match resources {
        Object::Dictionary(rd) => rd.get("ExtGState").and_then(|o| doc.resolve_object(o).ok()),
        _ => None,
    };
    let ext_g_states: Option<&HashMap<String, Object>> =
        ext_g_state_resolved.as_ref().and_then(|o| o.as_dict());
    let mut ext_g_state_cache: HashMap<String, super::page_renderer::ParsedExtGState> =
        HashMap::new();

    // Pre-resolve XObject resources for Do operator
    let xobjects_resolved: Option<Object> = match resources {
        Object::Dictionary(rd) => rd.get("XObject").and_then(|o| doc.resolve_object(o).ok()),
        _ => None,
    };

    for op in operators {
        match op {
            // Graphics state save/restore
            Operator::SaveState => {
                gs_stack.save();
                let cs = color_state_stack
                    .last()
                    .cloned()
                    .unwrap_or_else(SeparationColorState::new);
                color_state_stack.push(cs);
                clip_stack.push(clip_stack.last().cloned().unwrap_or(None));
            },
            Operator::RestoreState => {
                gs_stack.restore();
                if color_state_stack.len() > 1 {
                    color_state_stack.pop();
                }
                if clip_stack.len() > 1 {
                    clip_stack.pop();
                }
            },

            // CTM operators
            Operator::Cm { a, b, c, d, e, f } => {
                let current = gs_stack.current_mut();
                let new_matrix = Matrix {
                    a: *a,
                    b: *b,
                    c: *c,
                    d: *d,
                    e: *e,
                    f: *f,
                };
                current.ctm = new_matrix.multiply(&current.ctm);
            },

            // Color operators
            Operator::SetFillRgb { r, g, b } => {
                let gs = gs_stack.current_mut();
                gs.fill_color_rgb = (*r, *g, *b);
                gs.fill_color_space = "DeviceRGB".to_string();
                gs.fill_color_cmyk = None;
                if let Some(cs) = color_state_stack.last_mut() {
                    cs.fill_components = vec![*r, *g, *b];
                }
            },
            Operator::SetStrokeRgb { r, g, b } => {
                let gs = gs_stack.current_mut();
                gs.stroke_color_rgb = (*r, *g, *b);
                gs.stroke_color_space = "DeviceRGB".to_string();
                gs.stroke_color_cmyk = None;
                if let Some(cs) = color_state_stack.last_mut() {
                    cs.stroke_components = vec![*r, *g, *b];
                }
            },
            Operator::SetFillGray { gray } => {
                let g = *gray;
                let gs = gs_stack.current_mut();
                gs.fill_color_rgb = (g, g, g);
                gs.fill_color_space = "DeviceGray".to_string();
                gs.fill_color_cmyk = None;
                if let Some(cs) = color_state_stack.last_mut() {
                    cs.fill_components = vec![g];
                }
            },
            Operator::SetStrokeGray { gray } => {
                let g = *gray;
                let gs = gs_stack.current_mut();
                gs.stroke_color_rgb = (g, g, g);
                gs.stroke_color_space = "DeviceGray".to_string();
                gs.stroke_color_cmyk = None;
                if let Some(cs) = color_state_stack.last_mut() {
                    cs.stroke_components = vec![g];
                }
            },
            Operator::SetFillCmyk { c, m, y, k } => {
                let gs = gs_stack.current_mut();
                gs.fill_color_cmyk = Some((*c, *m, *y, *k));
                gs.fill_color_space = "DeviceCMYK".to_string();
                if let Some(cs) = color_state_stack.last_mut() {
                    cs.fill_components = vec![*c, *m, *y, *k];
                }
            },
            Operator::SetStrokeCmyk { c, m, y, k } => {
                let gs = gs_stack.current_mut();
                gs.stroke_color_cmyk = Some((*c, *m, *y, *k));
                gs.stroke_color_space = "DeviceCMYK".to_string();
                if let Some(cs) = color_state_stack.last_mut() {
                    cs.stroke_components = vec![*c, *m, *y, *k];
                }
            },
            Operator::SetFillColorSpace { name } => {
                gs_stack.current_mut().fill_color_space = name.clone();
                gs_stack.current_mut().fill_color_cmyk = None;
            },
            Operator::SetStrokeColorSpace { name } => {
                gs_stack.current_mut().stroke_color_space = name.clone();
                gs_stack.current_mut().stroke_color_cmyk = None;
            },
            Operator::SetFillColor { components } | Operator::SetFillColorN { components, .. } => {
                let gs = gs_stack.current_mut();
                let space = gs.fill_color_space.clone();
                match space.as_str() {
                    "DeviceCMYK" | "CMYK" if components.len() >= 4 => {
                        gs.fill_color_cmyk =
                            Some((components[0], components[1], components[2], components[3]));
                    },
                    _ => {},
                }
                if let Some(cs) = color_state_stack.last_mut() {
                    cs.fill_components = components.clone();
                }
            },
            Operator::SetStrokeColor { components }
            | Operator::SetStrokeColorN { components, .. } => {
                let gs = gs_stack.current_mut();
                let space = gs.stroke_color_space.clone();
                match space.as_str() {
                    "DeviceCMYK" | "CMYK" if components.len() >= 4 => {
                        gs.stroke_color_cmyk =
                            Some((components[0], components[1], components[2], components[3]));
                    },
                    _ => {},
                }
                if let Some(cs) = color_state_stack.last_mut() {
                    cs.stroke_components = components.clone();
                }
            },

            // Line style operators (passed through to GS)
            Operator::SetLineWidth { width } => {
                gs_stack.current_mut().line_width = *width;
            },
            Operator::SetLineCap { cap_style } => {
                gs_stack.current_mut().line_cap = *cap_style;
            },
            Operator::SetLineJoin { join_style } => {
                gs_stack.current_mut().line_join = *join_style;
            },
            Operator::SetMiterLimit { limit } => {
                gs_stack.current_mut().miter_limit = *limit;
            },
            Operator::SetDash { array, phase } => {
                gs_stack.current_mut().dash_pattern = (array.clone(), *phase);
            },

            // Path construction
            Operator::MoveTo { x, y } => {
                current_path.move_to(*x, *y);
            },
            Operator::LineTo { x, y } => {
                current_path.line_to(*x, *y);
            },
            Operator::CurveTo {
                x1,
                y1,
                x2,
                y2,
                x3,
                y3,
            } => {
                current_path.cubic_to(*x1, *y1, *x2, *y2, *x3, *y3);
            },
            Operator::CurveToV { x2, y2, x3, y3 } => {
                if let Some(last) = current_path.last_point() {
                    current_path.cubic_to(last.x, last.y, *x2, *y2, *x3, *y3);
                }
            },
            Operator::CurveToY { x1, y1, x3, y3 } => {
                current_path.cubic_to(*x1, *y1, *x3, *y3, *x3, *y3);
            },
            Operator::Rectangle {
                x,
                y,
                width,
                height,
            } => {
                let (nx, nw) = if *width < 0.0 {
                    (x + width, -width)
                } else {
                    (*x, *width)
                };
                let (ny, nh) = if *height < 0.0 {
                    (y + height, -height)
                } else {
                    (*y, *height)
                };
                if let Some(rect) = tiny_skia::Rect::from_xywh(nx, ny, nw, nh) {
                    current_path.push_rect(rect);
                }
            },
            Operator::ClosePath => {
                current_path.close();
            },

            // Path painting
            Operator::Stroke => {
                apply_separation_clip(
                    &mut pending_clip,
                    &mut clip_stack,
                    pixmap,
                    base_transform,
                    &gs_stack,
                );
                if let Some(path) = current_path.finish() {
                    let gs = gs_stack.current();
                    let empty = SeparationColorState::new();
                    let cs = color_state_stack.last().unwrap_or(&empty);
                    if let Some(tint) = tint_for_ink(
                        false,
                        gs,
                        color_spaces,
                        target_ink,
                        &cs.fill_components,
                        &cs.stroke_components,
                    ) {
                        let transform = combine_transforms(base_transform, &gs.ctm);
                        let clip = clip_stack.last().and_then(|c| c.as_ref());
                        stroke_separation(pixmap, &path, transform, gs, tint, clip);
                    }
                }
                current_path = PathBuilder::new();
            },
            Operator::Fill => {
                apply_separation_clip(
                    &mut pending_clip,
                    &mut clip_stack,
                    pixmap,
                    base_transform,
                    &gs_stack,
                );
                if let Some(path) = current_path.finish() {
                    let gs = gs_stack.current();
                    let empty = SeparationColorState::new();
                    let cs = color_state_stack.last().unwrap_or(&empty);
                    if let Some(tint) = tint_for_ink(
                        true,
                        gs,
                        color_spaces,
                        target_ink,
                        &cs.fill_components,
                        &cs.stroke_components,
                    ) {
                        let transform = combine_transforms(base_transform, &gs.ctm);
                        let clip = clip_stack.last().and_then(|c| c.as_ref());
                        fill_separation(pixmap, &path, transform, tint, FillRule::Winding, clip);
                    }
                }
                current_path = PathBuilder::new();
            },
            Operator::FillEvenOdd => {
                apply_separation_clip(
                    &mut pending_clip,
                    &mut clip_stack,
                    pixmap,
                    base_transform,
                    &gs_stack,
                );
                if let Some(path) = current_path.finish() {
                    let gs = gs_stack.current();
                    let empty = SeparationColorState::new();
                    let cs = color_state_stack.last().unwrap_or(&empty);
                    if let Some(tint) = tint_for_ink(
                        true,
                        gs,
                        color_spaces,
                        target_ink,
                        &cs.fill_components,
                        &cs.stroke_components,
                    ) {
                        let transform = combine_transforms(base_transform, &gs.ctm);
                        let clip = clip_stack.last().and_then(|c| c.as_ref());
                        fill_separation(pixmap, &path, transform, tint, FillRule::EvenOdd, clip);
                    }
                }
                current_path = PathBuilder::new();
            },
            Operator::FillStroke | Operator::CloseFillStroke => {
                apply_separation_clip(
                    &mut pending_clip,
                    &mut clip_stack,
                    pixmap,
                    base_transform,
                    &gs_stack,
                );
                if let Some(path) = current_path.finish() {
                    let gs = gs_stack.current();
                    let empty = SeparationColorState::new();
                    let cs = color_state_stack.last().unwrap_or(&empty);
                    let transform = combine_transforms(base_transform, &gs.ctm);
                    let clip = clip_stack.last().and_then(|c| c.as_ref());
                    if let Some(tint) = tint_for_ink(
                        true,
                        gs,
                        color_spaces,
                        target_ink,
                        &cs.fill_components,
                        &cs.stroke_components,
                    ) {
                        fill_separation(pixmap, &path, transform, tint, FillRule::Winding, clip);
                    }
                    if let Some(tint) = tint_for_ink(
                        false,
                        gs,
                        color_spaces,
                        target_ink,
                        &cs.fill_components,
                        &cs.stroke_components,
                    ) {
                        stroke_separation(pixmap, &path, transform, gs, tint, clip);
                    }
                }
                current_path = PathBuilder::new();
            },
            Operator::FillStrokeEvenOdd | Operator::CloseFillStrokeEvenOdd => {
                apply_separation_clip(
                    &mut pending_clip,
                    &mut clip_stack,
                    pixmap,
                    base_transform,
                    &gs_stack,
                );
                if let Some(path) = current_path.finish() {
                    let gs = gs_stack.current();
                    let empty = SeparationColorState::new();
                    let cs = color_state_stack.last().unwrap_or(&empty);
                    let transform = combine_transforms(base_transform, &gs.ctm);
                    let clip = clip_stack.last().and_then(|c| c.as_ref());
                    if let Some(tint) = tint_for_ink(
                        true,
                        gs,
                        color_spaces,
                        target_ink,
                        &cs.fill_components,
                        &cs.stroke_components,
                    ) {
                        fill_separation(pixmap, &path, transform, tint, FillRule::EvenOdd, clip);
                    }
                    if let Some(tint) = tint_for_ink(
                        false,
                        gs,
                        color_spaces,
                        target_ink,
                        &cs.fill_components,
                        &cs.stroke_components,
                    ) {
                        stroke_separation(pixmap, &path, transform, gs, tint, clip);
                    }
                }
                current_path = PathBuilder::new();
            },
            Operator::EndPath => {
                apply_separation_clip(
                    &mut pending_clip,
                    &mut clip_stack,
                    pixmap,
                    base_transform,
                    &gs_stack,
                );
                current_path = PathBuilder::new();
            },

            // Clipping
            Operator::ClipNonZero => {
                if let Some(path) = current_path.clone().finish() {
                    pending_clip = Some((path, FillRule::Winding));
                }
            },
            Operator::ClipEvenOdd => {
                if let Some(path) = current_path.clone().finish() {
                    pending_clip = Some((path, FillRule::EvenOdd));
                }
            },

            // ExtGState
            Operator::SetExtGState { dict_name } => {
                let entry = ext_g_state_cache
                    .entry(dict_name.clone())
                    .or_insert_with(|| {
                        if let Some(states) = ext_g_states {
                            if let Some(state_obj) = states.get(dict_name) {
                                return super::page_renderer::parse_ext_g_state_inner(
                                    state_obj, doc,
                                )
                                .unwrap_or_default();
                            }
                        }
                        super::page_renderer::ParsedExtGState::default()
                    });
                entry.apply(gs_stack.current_mut());
            },

            // XObject (Form XObjects may contain ink-bearing content)
            Operator::Do { name } => {
                if let Some(xobjects) = xobjects_resolved.as_ref().and_then(|o| o.as_dict()) {
                    if let Some(xobj_ref_obj) = xobjects.get(name) {
                        if let Ok(xobj) = doc.resolve_object(xobj_ref_obj) {
                            if let Object::Stream { ref dict, .. } = xobj {
                                if let Some(subtype) = dict.get("Subtype").and_then(|o| o.as_name())
                                {
                                    if subtype == "Form" {
                                        let xobj_ref = xobj_ref_obj.as_reference();
                                        let stream_data = if let Some(r) = xobj_ref {
                                            doc.decode_stream_with_encryption(&xobj, r)?
                                        } else {
                                            xobj.decode_stream_data()?
                                        };

                                        let form_resources =
                                            if let Some(res) = dict.get("Resources") {
                                                doc.resolve_object(res)?
                                            } else {
                                                resources.clone()
                                            };

                                        let form_cs = load_color_spaces(doc, &form_resources)?;
                                        let mut merged_cs = color_spaces.clone();
                                        merged_cs.extend(form_cs);

                                        // Parse form matrix
                                        let form_matrix = parse_form_matrix(dict);
                                        let gs = gs_stack.current();
                                        let combined = combine_transforms(base_transform, &gs.ctm)
                                            .pre_concat(form_matrix);

                                        let form_ops = parse_content_stream(&stream_data)?;
                                        execute_separation_operators(
                                            pixmap,
                                            combined,
                                            &form_ops,
                                            doc,
                                            page_num,
                                            &form_resources,
                                            &merged_cs,
                                            target_ink,
                                        )?;
                                    }
                                    // Images are skipped for separation plates
                                }
                            }
                        }
                    }
                }
            },

            // Text and other operators are skipped for separation plates.
            // Text rendering with spot colors is rare in separation use cases;
            // we focus on path-based artwork which is the primary use.
            _ => {},
        }
    }
    Ok(())
}

/// Fill a path into the separation pixmap with the given tint value.
fn fill_separation(
    pixmap: &mut Pixmap,
    path: &tiny_skia::Path,
    transform: Transform,
    tint: f32,
    fill_rule: FillRule,
    clip: Option<&Mask>,
) {
    let gray = (tint.clamp(0.0, 1.0) * 255.0).round() as u8;
    let color = tiny_skia::Color::from_rgba8(gray, gray, gray, 255);
    let mut paint = tiny_skia::Paint::default();
    paint.set_color(color);
    paint.anti_alias = true;
    // Use SourceOver so overlapping shapes accumulate correctly
    paint.blend_mode = tiny_skia::BlendMode::SourceOver;

    pixmap.fill_path(path, &paint, fill_rule, transform, clip);
}

/// Stroke a path into the separation pixmap with the given tint value.
fn stroke_separation(
    pixmap: &mut Pixmap,
    path: &tiny_skia::Path,
    transform: Transform,
    gs: &GraphicsState,
    tint: f32,
    clip: Option<&Mask>,
) {
    let gray = (tint.clamp(0.0, 1.0) * 255.0).round() as u8;
    let color = tiny_skia::Color::from_rgba8(gray, gray, gray, 255);
    let mut paint = tiny_skia::Paint::default();
    paint.set_color(color);
    paint.anti_alias = true;

    let mut stroke = tiny_skia::Stroke::default();
    stroke.width = gs.line_width;
    stroke.line_cap = match gs.line_cap {
        1 => tiny_skia::LineCap::Round,
        2 => tiny_skia::LineCap::Square,
        _ => tiny_skia::LineCap::Butt,
    };
    stroke.line_join = match gs.line_join {
        1 => tiny_skia::LineJoin::Round,
        2 => tiny_skia::LineJoin::Bevel,
        _ => tiny_skia::LineJoin::Miter,
    };
    stroke.miter_limit = gs.miter_limit;

    if !gs.dash_pattern.0.is_empty() {
        stroke.dash = tiny_skia::StrokeDash::new(gs.dash_pattern.0.clone(), gs.dash_pattern.1);
    }

    pixmap.stroke_path(path, &paint, &stroke, transform, clip);
}

/// Apply a pending clip path to the clip stack.
fn apply_separation_clip(
    pending: &mut Option<(tiny_skia::Path, FillRule)>,
    clip_stack: &mut Vec<Option<Mask>>,
    pixmap: &Pixmap,
    base_transform: Transform,
    gs_stack: &GraphicsStateStack,
) {
    if let Some((path, fill_rule)) = pending.take() {
        let gs = gs_stack.current();
        let transform = combine_transforms(base_transform, &gs.ctm);

        if let Some(path_transformed) = path.transform(transform) {
            let mut new_mask = Mask::new(pixmap.width(), pixmap.height()).unwrap();
            new_mask.fill_path(&path_transformed, fill_rule, true, Transform::identity());

            if let Some(Some(current_mask)) = clip_stack.last() {
                let mut combined = current_mask.clone();
                let combined_data = combined.data_mut();
                let new_data = new_mask.data();
                for i in 0..combined_data.len() {
                    combined_data[i] = ((combined_data[i] as u32 * new_data[i] as u32) / 255) as u8;
                }
                *clip_stack.last_mut().unwrap() = Some(combined);
            } else {
                *clip_stack.last_mut().unwrap() = Some(new_mask);
            }
        }
    }
}

/// Parse a form XObject matrix from its dictionary.
fn parse_form_matrix(dict: &HashMap<String, Object>) -> Transform {
    if let Some(Object::Array(arr)) = dict.get("Matrix") {
        let get_f32 = |i: usize| -> f32 {
            match arr.get(i) {
                Some(Object::Real(v)) => *v as f32,
                Some(Object::Integer(v)) => *v as f32,
                _ => {
                    if i == 0 || i == 3 {
                        1.0
                    } else {
                        0.0
                    }
                },
            }
        };
        Transform::from_row(get_f32(0), get_f32(1), get_f32(2), get_f32(3), get_f32(4), get_f32(5))
    } else {
        Transform::identity()
    }
}

/// Combine two transformations (base + CTM).
fn combine_transforms(base: Transform, ctm: &Matrix) -> Transform {
    base.pre_concat(Transform::from_row(ctm.a, ctm.b, ctm.c, ctm.d, ctm.e, ctm.f))
}
