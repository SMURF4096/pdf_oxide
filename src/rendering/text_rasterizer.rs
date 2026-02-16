//! Text rasterizer - renders PDF text using tiny-skia.
//!
//! Text rendering in PDF is complex because:
//! - Fonts may be embedded or use standard PDF fonts
//! - Character encoding varies (identity-H, MacRoman, custom ToUnicode, etc.)
//! - Glyph positioning is explicit via TJ arrays
//!
//! This module provides a basic text rendering implementation that:
//! - Uses system fonts as fallback when embedded fonts aren't available
//! - Renders text as simple glyphs at calculated positions

use super::create_fill_paint;
use crate::content::operators::TextElement;
use crate::content::GraphicsState;
use crate::document::PdfDocument;
use crate::error::Result;
use crate::object::Object;

use tiny_skia::{Paint, PathBuilder, Pixmap, Transform};

/// Rasterizer for PDF text operations.
pub struct TextRasterizer {
    // Font database for system font fallback
    // fontdb: fontdb::Database,
}

impl TextRasterizer {
    /// Create a new text rasterizer.
    pub fn new() -> Self {
        Self {
            // fontdb: fontdb::Database::new(),
        }
    }

    /// Render a text string (Tj operator).
    pub fn render_text(
        &self,
        pixmap: &mut Pixmap,
        text: &[u8],
        base_transform: Transform,
        gs: &GraphicsState,
        _resources: &Object,
        _doc: &mut PdfDocument,
        clip_mask: Option<&tiny_skia::Mask>,
    ) -> Result<()> {
        // Get text position from text matrix combined with CTM
        let text_matrix = &gs.text_matrix;
        let font_size = gs.font_size;

        // Create paint from fill color (text uses fill color, not stroke)
        let paint = create_fill_paint(gs, "Normal");

        // Calculate position in user space
        let x = text_matrix.e;
        let y = text_matrix.f;

        // For now, we render text as simple shapes
        // Real implementation would use font glyph outlines
        self.render_text_simple(
            pixmap,
            text,
            x,
            y,
            font_size,
            &paint,
            base_transform,
            gs,
            clip_mask,
        )?;

        Ok(())
    }

    /// Render a TJ array (text with positioning adjustments).
    pub fn render_tj_array(
        &self,
        pixmap: &mut Pixmap,
        array: &[TextElement],
        base_transform: Transform,
        gs: &GraphicsState,
        _resources: &Object,
        _doc: &mut PdfDocument,
        clip_mask: Option<&tiny_skia::Mask>,
    ) -> Result<()> {
        let paint = create_fill_paint(gs, "Normal");

        let font_size = gs.font_size;
        let text_matrix = &gs.text_matrix;

        let mut current_x = text_matrix.e;
        let y = text_matrix.f;

        for element in array {
            match element {
                TextElement::String(text) => {
                    self.render_text_simple(
                        pixmap,
                        text,
                        current_x,
                        y,
                        font_size,
                        &paint,
                        base_transform,
                        gs,
                        clip_mask,
                    )?;
                    // Advance position based on text width (simplified)
                    let char_count = text.len() as f32;
                    current_x += char_count * font_size * 0.5; // Rough estimate
                },
                TextElement::Offset(offset) => {
                    // Offset is in thousandths of a unit of text space
                    // Negative offset moves right
                    let adjustment = -(*offset) / 1000.0 * font_size;
                    current_x += adjustment;
                },
            }
        }

        Ok(())
    }

    /// Simple text rendering using rectangles as placeholder.
    ///
    /// This is a basic implementation that draws rectangles where text would appear.
    /// A full implementation would use the actual font glyph outlines.
    fn render_text_simple(
        &self,
        pixmap: &mut Pixmap,
        text: &[u8],
        x: f32,
        y: f32,
        font_size: f32,
        paint: &Paint,
        base_transform: Transform,
        gs: &GraphicsState,
        clip_mask: Option<&tiny_skia::Mask>,
    ) -> Result<()> {
        // Calculate transform including text matrix
        let text_transform = Transform::from_row(
            gs.text_matrix.a,
            gs.text_matrix.b,
            gs.text_matrix.c,
            gs.text_matrix.d,
            0.0,
            0.0,
        );
        let transform = base_transform.pre_concat(text_transform);

        // Render each character as a simple glyph
        let char_width = font_size * 0.6; // Approximate character width
        let char_height = font_size;

        let mut current_x = x;

        for byte in text {
            // Skip control characters
            if *byte < 32 {
                continue;
            }

            // Draw a simple rectangle for each character
            // Real implementation would render actual glyph paths
            let mut path = PathBuilder::new();

            // Create a simple character shape (rectangle with slight indent)
            let char_left = current_x;
            let char_bottom = y;
            let char_right = current_x + char_width * 0.8;
            let char_top = y + char_height * 0.8;

            // Draw glyph shape based on character type
            if byte.is_ascii_uppercase() || byte.is_ascii_digit() {
                // Upper case and digits - full height
                if let Some(rect) =
                    tiny_skia::Rect::from_ltrb(char_left, char_bottom, char_right, char_top)
                {
                    path.push_rect(rect);
                }
            } else if byte.is_ascii_lowercase() {
                // Lower case - x-height
                let x_height = char_height * 0.6;
                if let Some(rect) = tiny_skia::Rect::from_ltrb(
                    char_left,
                    char_bottom,
                    char_right,
                    char_bottom + x_height,
                ) {
                    path.push_rect(rect);
                }
            } else if *byte == b' ' {
                // Space - just advance
            } else {
                // Other characters - moderate height
                if let Some(rect) = tiny_skia::Rect::from_ltrb(
                    char_left,
                    char_bottom,
                    char_right,
                    char_bottom + char_height * 0.7,
                ) {
                    path.push_rect(rect);
                }
            }

            if let Some(path) = path.finish() {
                pixmap.fill_path(&path, paint, tiny_skia::FillRule::Winding, transform, clip_mask);
            }

            // Advance position
            current_x += char_width + gs.char_space;
            if *byte == b' ' {
                current_x += gs.word_space;
            }
        }

        Ok(())
    }
}

impl Default for TextRasterizer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_rasterizer_new() {
        let _rasterizer = TextRasterizer::new();
        // Just verify it can be created
    }
}
