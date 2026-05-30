//! Shared parser for PDF `ExtGState` dictionary entries.
//!
//! Both the page renderer and the separation-plate renderer need to apply
//! transparency / blend-mode overrides from `gs` operators. Keeping the
//! parser in a single module avoids drift between the two renderers and
//! removes the `pub(crate)` leak that previously crossed module boundaries.

use crate::content::graphics_state::GraphicsState;
use crate::document::PdfDocument;
use crate::error::Result;
use crate::object::Object;

/// Parsed effects of a PDF `ExtGState` dictionary. Only the fields actually
/// applied during rendering are captured (fill/stroke alpha and blend mode).
/// Anything else (TK / SMask / AIS) is intentionally ignored so the cached
/// entry stays tiny.
#[derive(Clone, Debug, Default)]
pub(crate) struct ParsedExtGState {
    pub(crate) fill_alpha: Option<f32>,
    pub(crate) stroke_alpha: Option<f32>,
    pub(crate) blend_mode: Option<String>,
}

impl ParsedExtGState {
    /// Apply this dictionary's fields to `gs`. Fields that were not present
    /// in the source dictionary are left untouched on `gs`.
    pub(crate) fn apply(&self, gs: &mut GraphicsState) {
        if let Some(a) = self.fill_alpha {
            gs.fill_alpha = a;
        }
        if let Some(a) = self.stroke_alpha {
            gs.stroke_alpha = a;
        }
        if let Some(ref m) = self.blend_mode {
            gs.blend_mode = m.clone();
        }
    }
}

/// Parse the fields we need from an `ExtGState` *entry* (the inner dict, not
/// the resource dict that holds it). Resolves `state_obj` once if it is a
/// reference.
pub(crate) fn parse_ext_g_state_inner(
    state_obj: &Object,
    doc: &PdfDocument,
) -> Result<ParsedExtGState> {
    let mut out = ParsedExtGState::default();
    let state_resolved = doc.resolve_object(state_obj)?;
    let state_dict = match state_resolved.as_dict() {
        Some(d) => d,
        None => return Ok(out),
    };

    if let Some(ca) = state_dict.get("ca") {
        out.fill_alpha = ca
            .as_real()
            .map(|v| v as f32)
            .or_else(|| ca.as_integer().map(|v| v as f32));
    }
    if let Some(ca_upper) = state_dict.get("CA") {
        out.stroke_alpha = ca_upper
            .as_real()
            .map(|v| v as f32)
            .or_else(|| ca_upper.as_integer().map(|v| v as f32));
    }
    if let Some(bm) = state_dict.get("BM") {
        let mode = match bm {
            Object::Name(n) => n.clone(),
            Object::Array(arr) => arr
                .first()
                .and_then(|o| o.as_name())
                .unwrap_or("Normal")
                .to_string(),
            _ => "Normal".to_string(),
        };
        out.blend_mode = Some(mode);
    }
    Ok(out)
}
