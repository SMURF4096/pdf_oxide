//! PDF document writer.
//!
//! Assembles complete PDF documents with proper structure:
//! header, body, xref table, and trailer.

use super::acroform::AcroFormBuilder;
use super::annotation_builder::{AnnotationBuilder, LinkAnnotation};
use super::content_stream::ContentStreamBuilder;
use super::form_fields::{
    CheckboxWidget, ComboBoxWidget, FormFieldEntry, ListBoxWidget, PushButtonWidget,
    RadioButtonGroup, TextFieldWidget,
};
use super::freetext::FreeTextAnnotation;
use super::ink::InkAnnotation;
use super::object_serializer::ObjectSerializer;
use super::shape_annotations::{LineAnnotation, PolygonAnnotation, ShapeAnnotation};
use super::special_annotations::{
    CaretAnnotation, FileAttachmentAnnotation, FileAttachmentIcon, PopupAnnotation,
    RedactAnnotation,
};
use super::stamp::{StampAnnotation, StampType};
use super::text_annotations::TextAnnotation;
use super::text_markup::TextMarkupAnnotation;
use crate::annotation_types::{LineEndingStyle, TextAlignment, TextAnnotationIcon, TextMarkupType};
use crate::elements::ContentElement;
use crate::error::Result;
use crate::geometry::Rect;
use crate::object::{Object, ObjectRef};
use std::collections::HashMap;
use std::io::Write;

/// Configuration for PDF generation.
#[derive(Debug, Clone)]
pub struct PdfWriterConfig {
    /// PDF version (e.g., "1.7")
    pub version: String,
    /// Document title
    pub title: Option<String>,
    /// Document author
    pub author: Option<String>,
    /// Document subject
    pub subject: Option<String>,
    /// Document keywords
    pub keywords: Option<String>,
    /// Creator application
    pub creator: Option<String>,
    /// Whether to compress streams
    pub compress: bool,
}

impl Default for PdfWriterConfig {
    fn default() -> Self {
        Self {
            version: "1.7".to_string(),
            title: None,
            author: None,
            subject: None,
            keywords: None,
            creator: Some("pdf_oxide".to_string()),
            compress: false, // Disable compression for now (requires flate2)
        }
    }
}

impl PdfWriterConfig {
    /// Set document title.
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set document author.
    pub fn with_author(mut self, author: impl Into<String>) -> Self {
        self.author = Some(author.into());
        self
    }

    /// Set document subject.
    pub fn with_subject(mut self, subject: impl Into<String>) -> Self {
        self.subject = Some(subject.into());
        self
    }

    /// Enable or disable stream compression.
    ///
    /// When enabled, content streams and embedded data will be compressed
    /// using FlateDecode (zlib/deflate) to reduce file size.
    pub fn with_compress(mut self, compress: bool) -> Self {
        self.compress = compress;
        self
    }
}

/// Compress data using Flate/Deflate compression.
///
/// Returns compressed bytes suitable for FlateDecode filter.
fn compress_data(data: &[u8]) -> std::io::Result<Vec<u8>> {
    use flate2::write::ZlibEncoder;
    use flate2::Compression;

    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(data)?;
    encoder.finish()
}

/// A page being built.
pub struct PageBuilder<'a> {
    writer: &'a mut PdfWriter,
    page_index: usize,
}

impl<'a> PageBuilder<'a> {
    /// Add text to the page.
    pub fn add_text(
        &mut self,
        text: &str,
        x: f32,
        y: f32,
        font_name: &str,
        font_size: f32,
    ) -> &mut Self {
        let page = &mut self.writer.pages[self.page_index];
        page.content_builder
            .begin_text()
            .set_font(font_name, font_size)
            .text(text, x, y);
        self
    }

    /// Add Unicode text on a page using a previously-registered embedded
    /// TrueType font. The font must have been registered with
    /// [`PdfWriter::register_embedded_font`] first; the returned resource
    /// name is what `font_resource_name` should be (e.g. `"EF1"`).
    ///
    /// Glyph IDs are looked up via the font's `cmap` and buffered into
    /// a structured [`crate::writer::content_stream::ContentStreamOp::ShowEmbeddedText`]
    /// op carrying the font resource name. Hex emission is deferred to
    /// [`PdfWriter::finish`], which runs
    /// [`crate::fonts::subset_font_bytes`] on each embedded font and
    /// uses the resulting [`crate::fonts::GlyphRemapper`] to renumber
    /// every original GID in the content stream into its subset-local
    /// index — so `FontFile2`, `/W`, `ToUnicode`, and the content stream
    /// all agree on the subset GID space.
    pub fn add_embedded_text(
        &mut self,
        text: &str,
        x: f32,
        y: f32,
        font_resource_name: &str,
        font_size: f32,
    ) -> &mut Self {
        // `embedded_fonts` is keyed by the `EFn` resource name directly,
        // so no indirection through a name map. An unknown resource name
        // is a silent no-op — missing-text is easier to debug than a
        // panic deep inside the writer, and HTML→PDF hits unknown fonts
        // often during early development.
        let glyph_ids = self
            .writer
            .embedded_fonts
            .get_mut(font_resource_name)
            .map(|font| font.encode_string(text));
        let Some(glyph_ids) = glyph_ids else {
            return self;
        };

        let page = &mut self.writer.pages[self.page_index];
        page.content_builder
            .begin_text()
            .set_font(font_resource_name, font_size)
            .embedded_text(font_resource_name, glyph_ids, x, y);
        self
    }

    /// Add Unicode text on a page using the rustybuzz shaper. Required
    /// for any complex script (Arabic, Hebrew, Devanagari) where
    /// `add_embedded_text`'s naive char→glyph cmap lookup produces
    /// the wrong glyphs (no contextual forms, no ligatures, no RTL
    /// reordering).
    ///
    /// `direction` controls visual reordering — pass
    /// [`crate::writer::font_shaping::Direction::Rtl`] for Arabic/
    /// Hebrew runs after BiDi segmentation.
    ///
    /// On any error (unknown resource, unparseable face) this is a
    /// silent no-op for the same reason as `add_embedded_text`: a
    /// missing-glyph symptom is easier to debug than a panic.
    #[cfg(feature = "system-fonts")]
    pub fn add_shaped_embedded_text(
        &mut self,
        text: &str,
        x: f32,
        y: f32,
        font_resource_name: &str,
        font_size: f32,
        direction: super::font_shaping::Direction,
    ) -> &mut Self {
        let Some(font) = self.writer.embedded_fonts.get_mut(font_resource_name) else {
            return self;
        };
        // Shape directly against the font's owned bytes — no clone.
        // `shape` returns owned ShapedRun, so the &[u8] borrow on `font`
        // is released before we call `encode_shaped_run` (&mut self).
        let Some(shaped) = super::font_shaping::shape(text, font.font_data(), direction) else {
            return self;
        };
        // encode_shaped_run records (codepoint, glyph) pairs via the
        // shaper's cluster field so the ToUnicode CMap round-trips.
        let glyph_ids = font.encode_shaped_run(&shaped, text);

        let page = &mut self.writer.pages[self.page_index];
        page.content_builder
            .begin_text()
            .set_font(font_resource_name, font_size)
            .embedded_text(font_resource_name, glyph_ids, x, y);
        self
    }

    /// Add a content element to the page.
    ///
    /// Text elements whose `FontSpec.name` matches a font registered
    /// via `PdfWriter::register_embedded_font_as` are routed through
    /// `add_embedded_text` (Type-0 hex emission) so that CJK / Cyrillic
    /// / Greek / etc. render with the embedded subset. All other
    /// elements fall through to the default base-14 content-stream
    /// path.
    pub fn add_element(&mut self, element: &ContentElement) -> &mut Self {
        if let ContentElement::Text(t) = element {
            // `embedded_resource_for_user_name` returns `Option<&str>`
            // borrowing into the writer's own map — we clone once here
            // because `add_embedded_text` takes `&mut self.writer`
            // immediately after and the borrow rules won't let the
            // immutable ref survive the mutable call.
            let resource_name = self
                .writer
                .embedded_resource_for_user_name(&t.font.name)
                .map(String::from);
            if let Some(resource_name) = resource_name {
                self.add_embedded_text(&t.text, t.bbox.x, t.bbox.y, &resource_name, t.font.size);
                return self;
            }
        }
        let page = &mut self.writer.pages[self.page_index];
        page.content_builder.add_element(element);
        self
    }

    /// Add multiple content elements. Each element is routed through
    /// `add_element` so the embedded-font dispatch applies per-element.
    pub fn add_elements(&mut self, elements: &[ContentElement]) -> &mut Self {
        for element in elements {
            self.add_element(element);
        }
        self
    }

    /// Draw a rectangle on the page.
    pub fn draw_rect(&mut self, x: f32, y: f32, width: f32, height: f32) -> &mut Self {
        let page = &mut self.writer.pages[self.page_index];
        page.content_builder.end_text();
        page.content_builder.rect(x, y, width, height).stroke();
        self
    }

    /// Add a link annotation to the page.
    ///
    /// # Arguments
    ///
    /// * `link` - The link annotation to add
    pub fn add_link(&mut self, link: LinkAnnotation) -> &mut Self {
        let page = &mut self.writer.pages[self.page_index];
        page.annotations.add_link(link);
        self
    }

    /// Add a URI link annotation to the page.
    ///
    /// # Arguments
    ///
    /// * `rect` - The clickable area in page coordinates
    /// * `uri` - The target URL
    pub fn link(&mut self, rect: Rect, uri: impl Into<String>) -> &mut Self {
        self.add_link(LinkAnnotation::uri(rect, uri))
    }

    /// Add an internal page link annotation.
    ///
    /// # Arguments
    ///
    /// * `rect` - The clickable area in page coordinates
    /// * `page` - The target page index (0-based)
    pub fn internal_link(&mut self, rect: Rect, page: usize) -> &mut Self {
        self.add_link(LinkAnnotation::goto_page(rect, page))
    }

    /// Add a text markup annotation.
    pub fn add_text_markup(&mut self, markup: TextMarkupAnnotation) -> &mut Self {
        let page = &mut self.writer.pages[self.page_index];
        page.annotations.add_text_markup(markup);
        self
    }

    /// Add a highlight annotation.
    ///
    /// # Arguments
    ///
    /// * `rect` - Bounding rectangle
    /// * `quad_points` - QuadPoints defining the text area (each is 8 f64 values)
    pub fn highlight(&mut self, rect: Rect, quad_points: Vec<[f64; 8]>) -> &mut Self {
        self.add_text_markup(TextMarkupAnnotation::highlight(rect, quad_points))
    }

    /// Add a highlight annotation from a simple rectangle.
    ///
    /// Generates QuadPoints automatically from the rectangle.
    pub fn highlight_rect(&mut self, rect: Rect) -> &mut Self {
        self.add_text_markup(TextMarkupAnnotation::from_rect(TextMarkupType::Highlight, rect))
    }

    /// Add an underline annotation.
    pub fn underline(&mut self, rect: Rect, quad_points: Vec<[f64; 8]>) -> &mut Self {
        self.add_text_markup(TextMarkupAnnotation::underline(rect, quad_points))
    }

    /// Add an underline annotation from a simple rectangle.
    pub fn underline_rect(&mut self, rect: Rect) -> &mut Self {
        self.add_text_markup(TextMarkupAnnotation::from_rect(TextMarkupType::Underline, rect))
    }

    /// Add a strikeout annotation.
    pub fn strikeout(&mut self, rect: Rect, quad_points: Vec<[f64; 8]>) -> &mut Self {
        self.add_text_markup(TextMarkupAnnotation::strikeout(rect, quad_points))
    }

    /// Add a strikeout annotation from a simple rectangle.
    pub fn strikeout_rect(&mut self, rect: Rect) -> &mut Self {
        self.add_text_markup(TextMarkupAnnotation::from_rect(TextMarkupType::StrikeOut, rect))
    }

    /// Add a squiggly underline annotation.
    pub fn squiggly(&mut self, rect: Rect, quad_points: Vec<[f64; 8]>) -> &mut Self {
        self.add_text_markup(TextMarkupAnnotation::squiggly(rect, quad_points))
    }

    /// Add a squiggly underline annotation from a simple rectangle.
    pub fn squiggly_rect(&mut self, rect: Rect) -> &mut Self {
        self.add_text_markup(TextMarkupAnnotation::from_rect(TextMarkupType::Squiggly, rect))
    }

    /// Add a text annotation (sticky note).
    pub fn add_text_note(&mut self, note: TextAnnotation) -> &mut Self {
        let page = &mut self.writer.pages[self.page_index];
        page.annotations.add_text_note(note);
        self
    }

    /// Add a sticky note annotation with default Note icon.
    ///
    /// # Arguments
    ///
    /// * `rect` - The position and size for the icon (typically 24x24)
    /// * `contents` - The text content of the note
    pub fn sticky_note(&mut self, rect: Rect, contents: impl Into<String>) -> &mut Self {
        self.add_text_note(TextAnnotation::note(rect, contents))
    }

    /// Add a comment annotation (speech bubble icon).
    pub fn comment(&mut self, rect: Rect, contents: impl Into<String>) -> &mut Self {
        self.add_text_note(TextAnnotation::comment(rect, contents))
    }

    /// Add a text annotation with a specific icon.
    ///
    /// # Arguments
    ///
    /// * `rect` - The position and size for the icon
    /// * `contents` - The text content of the note
    /// * `icon` - The icon to display
    pub fn text_note_with_icon(
        &mut self,
        rect: Rect,
        contents: impl Into<String>,
        icon: TextAnnotationIcon,
    ) -> &mut Self {
        self.add_text_note(TextAnnotation::new(rect, contents).with_icon(icon))
    }

    // ===== FreeText Annotation Methods =====

    /// Add a FreeText annotation.
    pub fn add_freetext(&mut self, freetext: FreeTextAnnotation) -> &mut Self {
        let page = &mut self.writer.pages[self.page_index];
        page.annotations.add_freetext(freetext);
        self
    }

    /// Add a text box annotation.
    ///
    /// # Arguments
    ///
    /// * `rect` - The position and size of the text box
    /// * `contents` - The text content
    pub fn textbox(&mut self, rect: Rect, contents: impl Into<String>) -> &mut Self {
        self.add_freetext(FreeTextAnnotation::new(rect, contents))
    }

    /// Add a text box with specific font and size.
    ///
    /// # Arguments
    ///
    /// * `rect` - The position and size of the text box
    /// * `contents` - The text content
    /// * `font` - Font name (Helvetica, Times, Courier)
    /// * `size` - Font size in points
    pub fn textbox_styled(
        &mut self,
        rect: Rect,
        contents: impl Into<String>,
        font: &str,
        size: f32,
    ) -> &mut Self {
        self.add_freetext(FreeTextAnnotation::new(rect, contents).with_font(font, size))
    }

    /// Add a centered text box.
    pub fn textbox_centered(&mut self, rect: Rect, contents: impl Into<String>) -> &mut Self {
        self.add_freetext(
            FreeTextAnnotation::new(rect, contents).with_alignment(TextAlignment::Center),
        )
    }

    /// Add a callout annotation (text box with leader line).
    ///
    /// # Arguments
    ///
    /// * `rect` - The position and size of the text box
    /// * `contents` - The text content
    /// * `callout_points` - Leader line coordinates [x1,y1, x2,y2] or [x1,y1, x2,y2, x3,y3]
    pub fn callout(
        &mut self,
        rect: Rect,
        contents: impl Into<String>,
        callout_points: Vec<f64>,
    ) -> &mut Self {
        self.add_freetext(FreeTextAnnotation::callout(rect, contents, callout_points))
    }

    /// Add a typewriter annotation (plain text without border).
    pub fn typewriter(&mut self, rect: Rect, contents: impl Into<String>) -> &mut Self {
        self.add_freetext(FreeTextAnnotation::typewriter(rect, contents))
    }

    // ===== Line Annotation Methods =====

    /// Add a Line annotation.
    pub fn add_line(&mut self, line: LineAnnotation) -> &mut Self {
        let page = &mut self.writer.pages[self.page_index];
        page.annotations.add_line(line);
        self
    }

    /// Add a simple line from start to end.
    pub fn line(&mut self, start: (f64, f64), end: (f64, f64)) -> &mut Self {
        self.add_line(LineAnnotation::new(start, end))
    }

    /// Add a line with an arrow at the end.
    pub fn arrow(&mut self, start: (f64, f64), end: (f64, f64)) -> &mut Self {
        self.add_line(LineAnnotation::arrow(start, end))
    }

    /// Add a double-headed arrow line.
    pub fn double_arrow(&mut self, start: (f64, f64), end: (f64, f64)) -> &mut Self {
        self.add_line(LineAnnotation::double_arrow(start, end))
    }

    /// Add a line with custom line endings.
    pub fn line_with_endings(
        &mut self,
        start: (f64, f64),
        end: (f64, f64),
        start_ending: LineEndingStyle,
        end_ending: LineEndingStyle,
    ) -> &mut Self {
        self.add_line(LineAnnotation::new(start, end).with_line_endings(start_ending, end_ending))
    }

    // ===== Shape Annotation Methods =====

    /// Add a Shape annotation.
    pub fn add_shape(&mut self, shape: ShapeAnnotation) -> &mut Self {
        let page = &mut self.writer.pages[self.page_index];
        page.annotations.add_shape(shape);
        self
    }

    /// Add a rectangle annotation.
    pub fn rectangle(&mut self, rect: Rect) -> &mut Self {
        self.add_shape(ShapeAnnotation::square(rect))
    }

    /// Add a filled rectangle annotation.
    pub fn rectangle_filled(
        &mut self,
        rect: Rect,
        stroke: (f32, f32, f32),
        fill: (f32, f32, f32),
    ) -> &mut Self {
        self.add_shape(
            ShapeAnnotation::square(rect)
                .with_stroke_color(stroke.0, stroke.1, stroke.2)
                .with_fill_color(fill.0, fill.1, fill.2),
        )
    }

    /// Add a circle/ellipse annotation.
    pub fn circle(&mut self, rect: Rect) -> &mut Self {
        self.add_shape(ShapeAnnotation::circle(rect))
    }

    /// Add a filled circle/ellipse annotation.
    pub fn circle_filled(
        &mut self,
        rect: Rect,
        stroke: (f32, f32, f32),
        fill: (f32, f32, f32),
    ) -> &mut Self {
        self.add_shape(
            ShapeAnnotation::circle(rect)
                .with_stroke_color(stroke.0, stroke.1, stroke.2)
                .with_fill_color(fill.0, fill.1, fill.2),
        )
    }

    // ===== Polygon/PolyLine Annotation Methods =====

    /// Add a Polygon or PolyLine annotation.
    pub fn add_polygon(&mut self, polygon: PolygonAnnotation) -> &mut Self {
        let page = &mut self.writer.pages[self.page_index];
        page.annotations.add_polygon(polygon);
        self
    }

    /// Add a closed polygon annotation.
    pub fn polygon(&mut self, vertices: Vec<(f64, f64)>) -> &mut Self {
        self.add_polygon(PolygonAnnotation::polygon(vertices))
    }

    /// Add a filled polygon annotation.
    pub fn polygon_filled(
        &mut self,
        vertices: Vec<(f64, f64)>,
        stroke: (f32, f32, f32),
        fill: (f32, f32, f32),
    ) -> &mut Self {
        self.add_polygon(
            PolygonAnnotation::polygon(vertices)
                .with_stroke_color(stroke.0, stroke.1, stroke.2)
                .with_fill_color(fill.0, fill.1, fill.2),
        )
    }

    /// Add an open polyline annotation.
    pub fn polyline(&mut self, vertices: Vec<(f64, f64)>) -> &mut Self {
        self.add_polygon(PolygonAnnotation::polyline(vertices))
    }

    // ===== Ink Annotation Methods =====

    /// Add an Ink annotation (freehand drawing).
    pub fn add_ink(&mut self, ink: InkAnnotation) -> &mut Self {
        let page = &mut self.writer.pages[self.page_index];
        page.annotations.add_ink(ink);
        self
    }

    /// Add a freehand stroke annotation.
    ///
    /// # Arguments
    ///
    /// * `stroke` - List of (x, y) points forming the stroke path
    pub fn ink(&mut self, stroke: Vec<(f64, f64)>) -> &mut Self {
        self.add_ink(InkAnnotation::with_stroke(stroke))
    }

    /// Add a freehand drawing with multiple strokes.
    ///
    /// # Arguments
    ///
    /// * `strokes` - List of strokes, each being a list of (x, y) points
    pub fn freehand(&mut self, strokes: Vec<Vec<(f64, f64)>>) -> &mut Self {
        self.add_ink(InkAnnotation::with_strokes(strokes))
    }

    /// Add a styled ink annotation.
    ///
    /// # Arguments
    ///
    /// * `stroke` - List of (x, y) points
    /// * `color` - RGB color tuple
    /// * `line_width` - Line width in points
    pub fn ink_styled(
        &mut self,
        stroke: Vec<(f64, f64)>,
        color: (f32, f32, f32),
        line_width: f32,
    ) -> &mut Self {
        self.add_ink(
            InkAnnotation::with_stroke(stroke)
                .with_stroke_color(color.0, color.1, color.2)
                .with_line_width(line_width),
        )
    }

    // ===== Stamp Annotation Methods =====

    /// Add a Stamp annotation.
    pub fn add_stamp(&mut self, stamp: StampAnnotation) -> &mut Self {
        let page = &mut self.writer.pages[self.page_index];
        page.annotations.add_stamp(stamp);
        self
    }

    /// Add a stamp annotation with the given type.
    ///
    /// # Arguments
    ///
    /// * `rect` - Position and size of the stamp
    /// * `stamp_type` - The type of stamp to display
    pub fn stamp(&mut self, rect: Rect, stamp_type: StampType) -> &mut Self {
        self.add_stamp(StampAnnotation::new(rect, stamp_type))
    }

    /// Add an "Approved" stamp.
    pub fn stamp_approved(&mut self, rect: Rect) -> &mut Self {
        self.add_stamp(StampAnnotation::approved(rect))
    }

    /// Add a "Draft" stamp.
    pub fn stamp_draft(&mut self, rect: Rect) -> &mut Self {
        self.add_stamp(StampAnnotation::draft(rect))
    }

    /// Add a "Confidential" stamp.
    pub fn stamp_confidential(&mut self, rect: Rect) -> &mut Self {
        self.add_stamp(StampAnnotation::confidential(rect))
    }

    /// Add a "Final" stamp.
    pub fn stamp_final(&mut self, rect: Rect) -> &mut Self {
        self.add_stamp(StampAnnotation::final_stamp(rect))
    }

    /// Add a "Not Approved" stamp.
    pub fn stamp_not_approved(&mut self, rect: Rect) -> &mut Self {
        self.add_stamp(StampAnnotation::not_approved(rect))
    }

    /// Add a "For Comment" stamp.
    pub fn stamp_for_comment(&mut self, rect: Rect) -> &mut Self {
        self.add_stamp(StampAnnotation::for_comment(rect))
    }

    /// Add a custom stamp.
    ///
    /// # Arguments
    ///
    /// * `rect` - Position and size of the stamp
    /// * `name` - Custom stamp name
    pub fn stamp_custom(&mut self, rect: Rect, name: impl Into<String>) -> &mut Self {
        self.add_stamp(StampAnnotation::custom(rect, name))
    }

    // ===== Popup Annotation Methods =====

    /// Add a Popup annotation.
    pub fn add_popup(&mut self, popup: PopupAnnotation) -> &mut Self {
        let page = &mut self.writer.pages[self.page_index];
        page.annotations.add_popup(popup);
        self
    }

    /// Add a popup window for annotations.
    pub fn popup(&mut self, rect: Rect, open: bool) -> &mut Self {
        self.add_popup(PopupAnnotation::new(rect).with_open(open))
    }

    // ===== Caret Annotation Methods =====

    /// Add a Caret annotation.
    pub fn add_caret(&mut self, caret: CaretAnnotation) -> &mut Self {
        let page = &mut self.writer.pages[self.page_index];
        page.annotations.add_caret(caret);
        self
    }

    /// Add a caret (text insertion marker).
    pub fn caret(&mut self, rect: Rect) -> &mut Self {
        self.add_caret(CaretAnnotation::new(rect))
    }

    /// Add a caret with paragraph symbol.
    pub fn caret_paragraph(&mut self, rect: Rect) -> &mut Self {
        self.add_caret(CaretAnnotation::paragraph(rect))
    }

    /// Add a caret with a comment.
    pub fn caret_with_comment(&mut self, rect: Rect, comment: impl Into<String>) -> &mut Self {
        self.add_caret(CaretAnnotation::new(rect).with_contents(comment))
    }

    // ===== FileAttachment Annotation Methods =====

    /// Add a FileAttachment annotation.
    pub fn add_file_attachment(&mut self, file: FileAttachmentAnnotation) -> &mut Self {
        let page = &mut self.writer.pages[self.page_index];
        page.annotations.add_file_attachment(file);
        self
    }

    /// Add a file attachment annotation.
    pub fn file_attachment(&mut self, rect: Rect, file_name: impl Into<String>) -> &mut Self {
        self.add_file_attachment(FileAttachmentAnnotation::new(rect, file_name))
    }

    /// Add a file attachment with paperclip icon.
    pub fn file_attachment_paperclip(
        &mut self,
        rect: Rect,
        file_name: impl Into<String>,
    ) -> &mut Self {
        self.add_file_attachment(
            FileAttachmentAnnotation::new(rect, file_name).with_icon(FileAttachmentIcon::Paperclip),
        )
    }

    // ===== Redact Annotation Methods =====

    /// Add a Redact annotation.
    pub fn add_redact(&mut self, redact: RedactAnnotation) -> &mut Self {
        let page = &mut self.writer.pages[self.page_index];
        page.annotations.add_redact(redact);
        self
    }

    /// Add a redact annotation.
    pub fn redact(&mut self, rect: Rect) -> &mut Self {
        self.add_redact(RedactAnnotation::new(rect))
    }

    /// Add a redact annotation with overlay text.
    pub fn redact_with_text(&mut self, rect: Rect, overlay_text: impl Into<String>) -> &mut Self {
        self.add_redact(RedactAnnotation::new(rect).with_overlay_text(overlay_text))
    }

    // ===== Form Field Methods =====

    /// Add a text field to the page.
    ///
    /// # Arguments
    ///
    /// * `field` - The text field widget to add
    pub fn add_text_field(&mut self, field: TextFieldWidget) -> &mut Self {
        let page_ref = ObjectRef::new(0, 0); // Will be resolved during finish()
        let entry = field.build_entry(page_ref);
        let page = &mut self.writer.pages[self.page_index];
        page.form_fields.push(entry);
        self
    }

    /// Add a text field with builder pattern.
    pub fn text_field(&mut self, name: impl Into<String>, rect: Rect) -> &mut Self {
        self.add_text_field(TextFieldWidget::new(name, rect))
    }

    /// Add a checkbox to the page.
    ///
    /// # Arguments
    ///
    /// * `checkbox` - The checkbox widget to add
    pub fn add_checkbox(&mut self, checkbox: CheckboxWidget) -> &mut Self {
        let page_ref = ObjectRef::new(0, 0);
        let entry = checkbox.build_entry(page_ref);
        let page = &mut self.writer.pages[self.page_index];
        page.form_fields.push(entry);
        self
    }

    /// Add a checkbox with builder pattern.
    pub fn checkbox(&mut self, name: impl Into<String>, rect: Rect) -> &mut Self {
        self.add_checkbox(CheckboxWidget::new(name, rect))
    }

    /// Add a radio button group to the page.
    ///
    /// Note: All buttons in the group are added to this page.
    ///
    /// # Arguments
    ///
    /// * `group` - The radio button group to add
    pub fn add_radio_group(&mut self, group: RadioButtonGroup) -> &mut Self {
        let page_ref = ObjectRef::new(0, 0);
        let (parent_dict, entries) = group.build_entries(page_ref);
        let page = &mut self.writer.pages[self.page_index];

        // Add the parent field entry (contains group name, value, flags)
        // The parent is a non-widget field that groups all radio buttons
        let parent_entry = FormFieldEntry {
            widget_dict: HashMap::new(), // Parent has no widget (not visible)
            field_dict: parent_dict,
            name: group.name().to_string(),
            rect: Rect::new(0.0, 0.0, 0.0, 0.0), // No visual representation
            field_type: "Btn".to_string(),
        };
        page.form_fields.push(parent_entry);

        // Add child widget entries (the actual radio buttons)
        for entry in entries {
            page.form_fields.push(entry);
        }
        self
    }

    /// Add a combo box (dropdown) to the page.
    ///
    /// # Arguments
    ///
    /// * `combo` - The combo box widget to add
    pub fn add_combo_box(&mut self, combo: ComboBoxWidget) -> &mut Self {
        let page_ref = ObjectRef::new(0, 0);
        let entry = combo.build_entry(page_ref);
        let page = &mut self.writer.pages[self.page_index];
        page.form_fields.push(entry);
        self
    }

    /// Add a list box to the page.
    ///
    /// # Arguments
    ///
    /// * `list` - The list box widget to add
    pub fn add_list_box(&mut self, list: ListBoxWidget) -> &mut Self {
        let page_ref = ObjectRef::new(0, 0);
        let entry = list.build_entry(page_ref);
        let page = &mut self.writer.pages[self.page_index];
        page.form_fields.push(entry);
        self
    }

    /// Add a push button to the page.
    ///
    /// # Arguments
    ///
    /// * `button` - The push button widget to add
    pub fn add_push_button(&mut self, button: PushButtonWidget) -> &mut Self {
        let page_ref = ObjectRef::new(0, 0);
        let entry = button.build_entry(page_ref);
        let page = &mut self.writer.pages[self.page_index];
        page.form_fields.push(entry);
        self
    }

    // ===== Generic Annotation Method =====

    /// Add any annotation type to the page.
    ///
    /// This is a generic method that accepts any type that can be converted
    /// to an Annotation enum, including all the specific annotation types.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use pdf_oxide::writer::{LinkAnnotation, Annotation};
    /// use pdf_oxide::geometry::Rect;
    ///
    /// let link = LinkAnnotation::uri(
    ///     Rect::new(72.0, 720.0, 100.0, 12.0),
    ///     "https://example.com",
    /// );
    ///
    /// let mut writer = PdfWriter::new();
    /// let mut page = writer.add_page(612.0, 792.0);
    /// page.add_annotation(link);
    /// ```
    pub fn add_annotation<A: Into<super::annotation_builder::Annotation>>(
        &mut self,
        annotation: A,
    ) -> &mut Self {
        let page = &mut self.writer.pages[self.page_index];
        page.annotations.add_annotation(annotation);
        self
    }

    /// Finish building this page and return to the writer.
    pub fn finish(self) -> &'a mut PdfWriter {
        let page = &mut self.writer.pages[self.page_index];
        page.content_builder.end_text();
        self.writer
    }
}

/// Internal page data.
struct PageData {
    width: f32,
    height: f32,
    content_builder: ContentStreamBuilder,
    annotations: AnnotationBuilder,
    form_fields: Vec<FormFieldEntry>,
}

/// PDF document writer.
///
/// Builds a complete PDF document with pages, fonts, and content.
pub struct PdfWriter {
    config: PdfWriterConfig,
    pages: Vec<PageData>,
    /// Object ID counter
    next_obj_id: u32,
    /// Allocated objects (id -> object)
    objects: HashMap<u32, Object>,
    /// Font resources used (name -> object ref)
    fonts: HashMap<String, ObjectRef>,
    /// Registered embedded TrueType fonts, keyed by their `EFn`
    /// resource name. Keying by `EFn` (monotonic, guaranteed unique)
    /// instead of `EmbeddedFont::name` means two fonts with the same
    /// display name don't silently overwrite one another.
    ///
    /// Populated by [`PdfWriter::register_embedded_font`]; consumed in
    /// [`PdfWriter::finish`] where each font's five PDF objects are
    /// emitted and the Type-0 ref is added to every page's `/Font`
    /// resource dict.
    embedded_fonts: HashMap<String, super::font_manager::EmbeddedFont>,
    /// Insertion-order list of registered `EFn` resource names so
    /// [`PdfWriter::finish`] can iterate them in a stable, reproducible
    /// order (HashMap iteration would otherwise randomise the emitted
    /// font-object ordering across runs).
    embedded_font_order: Vec<String>,
    /// User-supplied font name (e.g. "NotoSansCJKtc") → `EFn` resource
    /// name. Lets the high-level `DocumentBuilder` / `PageBuilder`
    /// dispatch `ContentElement::Text` through `add_embedded_text`
    /// when the `FontSpec.name` matches a registered embedded font
    /// instead of silently falling back to Helvetica.
    user_font_to_resource: HashMap<String, String>,
    /// Counter for allocating `EFn` resource names.
    next_embedded_font_id: u32,
    /// AcroForm builder for interactive forms
    acroform: Option<AcroFormBuilder>,
}

impl PdfWriter {
    /// Create a new PDF writer with default config.
    pub fn new() -> Self {
        Self::with_config(PdfWriterConfig::default())
    }

    /// Create a PDF writer with custom config.
    pub fn with_config(config: PdfWriterConfig) -> Self {
        Self {
            config,
            pages: Vec::new(),
            next_obj_id: 1,
            objects: HashMap::new(),
            fonts: HashMap::new(),
            embedded_fonts: HashMap::new(),
            embedded_font_order: Vec::new(),
            user_font_to_resource: HashMap::new(),
            next_embedded_font_id: 1,
            acroform: None,
        }
    }

    /// Register an embedded TrueType font for use in content streams.
    ///
    /// Returns the resource name (e.g. `"EF1"`) that `add_embedded_text`
    /// should use. The font is consumed; `finish()` emits its five PDF
    /// objects (Type 0 / CIDFontType2 / FontDescriptor / FontFile2 stream
    /// / ToUnicode CMap stream — ISO 32000-1 §9.6.4 / §9.7.4 / §9.8 / §9.9
    /// / §9.10.2).
    ///
    /// The font's display name (used in PostScript/BaseFont fields) is
    /// taken from `EmbeddedFont::name`. Callers wanting a stable subset
    /// tag should track the resource name they get back and reuse it.
    pub fn register_embedded_font(&mut self, font: super::font_manager::EmbeddedFont) -> String {
        let resource_name = format!("EF{}", self.next_embedded_font_id);
        self.next_embedded_font_id += 1;
        self.embedded_fonts.insert(resource_name.clone(), font);
        self.embedded_font_order.push(resource_name.clone());
        resource_name
    }

    /// Register an embedded TrueType font under a user-visible name
    /// (e.g. `"NotoSansCJKtc"`). The name is what callers pass to
    /// `FluentPageBuilder::font(name, size)` / `FontSpec::name`; when
    /// a `ContentElement::Text` is dispatched, the `PageBuilder` looks
    /// up this map and routes matching elements through
    /// `add_embedded_text` (hex-encoded Type-0 emission) instead of the
    /// base-14 `map_font_name` fallback that silently collapses unknown
    /// names to `Helvetica`.
    ///
    /// Returns the `EFn` resource name for callers that want to mix
    /// low-level `add_embedded_text` calls with the high-level path.
    pub fn register_embedded_font_as(
        &mut self,
        user_name: impl Into<String>,
        font: super::font_manager::EmbeddedFont,
    ) -> String {
        let user_name = user_name.into();
        let resource_name = self.register_embedded_font(font);
        self.user_font_to_resource
            .insert(user_name, resource_name.clone());
        resource_name
    }

    /// Resolve a user-supplied font name (as stored in `FontSpec.name`)
    /// to its `EFn` resource name, if it was registered via
    /// `register_embedded_font_as`. Used by `PageBuilder::add_element`
    /// to decide whether `ContentElement::Text` should take the
    /// embedded-font path.
    ///
    /// Returns a borrow into the writer's own map so the dispatch
    /// path in `PageBuilder::add_element` doesn't allocate per text
    /// element — matters when a page has thousands of text runs
    /// coming through the HTML+CSS painter.
    pub(super) fn embedded_resource_for_user_name(&self, user_name: &str) -> Option<&str> {
        self.user_font_to_resource
            .get(user_name)
            .map(|s| s.as_str())
    }

    /// Allocate a new object ID.
    fn alloc_obj_id(&mut self) -> u32 {
        let id = self.next_obj_id;
        self.next_obj_id += 1;
        id
    }

    /// Add a page with the given dimensions.
    pub fn add_page(&mut self, width: f32, height: f32) -> PageBuilder<'_> {
        let page_index = self.pages.len();
        self.pages.push(PageData {
            width,
            height,
            content_builder: ContentStreamBuilder::new(),
            annotations: AnnotationBuilder::new(),
            form_fields: Vec::new(),
        });
        PageBuilder {
            writer: self,
            page_index,
        }
    }

    /// Add a US Letter sized page (8.5" x 11").
    pub fn add_letter_page(&mut self) -> PageBuilder<'_> {
        self.add_page(612.0, 792.0)
    }

    /// Add an A4 sized page (210mm x 297mm).
    pub fn add_a4_page(&mut self) -> PageBuilder<'_> {
        self.add_page(595.0, 842.0)
    }

    /// Get a font reference, creating the font object if needed.
    fn get_font_ref(&mut self, font_name: &str) -> ObjectRef {
        if let Some(font_ref) = self.fonts.get(font_name) {
            return *font_ref;
        }

        let font_id = self.alloc_obj_id();
        let font_obj = ObjectSerializer::dict(vec![
            ("Type", ObjectSerializer::name("Font")),
            ("Subtype", ObjectSerializer::name("Type1")),
            ("BaseFont", ObjectSerializer::name(font_name)),
            ("Encoding", ObjectSerializer::name("WinAnsiEncoding")),
        ]);

        self.objects.insert(font_id, font_obj);
        let font_ref = ObjectRef::new(font_id, 0);
        self.fonts.insert(font_name.to_string(), font_ref);
        font_ref
    }

    /// Build the complete PDF document.
    pub fn finish(mut self) -> Result<Vec<u8>> {
        let serializer = ObjectSerializer::compact();
        let mut output = Vec::new();
        let mut xref_offsets: Vec<(u32, usize)> = Vec::new();

        // PDF Header
        writeln!(output, "%PDF-{}", self.config.version)?;
        // Binary marker (recommended for binary content)
        output.extend_from_slice(b"%\xE2\xE3\xCF\xD3\n");

        // Collect all fonts used across pages
        let font_names: Vec<String> = vec![
            "Helvetica".to_string(),
            "Helvetica-Bold".to_string(),
            "Times-Roman".to_string(),
            "Times-Bold".to_string(),
            "Courier".to_string(),
            "Courier-Bold".to_string(),
        ];

        for font_name in &font_names {
            self.get_font_ref(font_name);
        }

        // Build font resources dictionary — Base-14 first.
        //
        // Key the resource dict by the *exact* font name the content
        // stream uses in its `Tf` operator (e.g. `Helvetica-Bold`,
        // with the dash). Previous versions stripped dashes here
        // (`HelveticaBold`), which meant every `Tf /Helvetica-Bold …`
        // referenced a missing resource — PDF readers silently fell
        // back to the default non-bold font, so *bold base-14 text
        // rendered without bold*. `map_font_name` in
        // `ContentStreamBuilder` emits the dashed form; keep the key
        // identical so the reference resolves.
        let mut font_resources: HashMap<String, Object> = self
            .fonts
            .iter()
            .map(|(name, obj_ref)| (name.clone(), Object::Reference(*obj_ref)))
            .collect();

        // Emit each embedded font's five-object graph (FONT-3) and add
        // the Type 0 ref to the resource dict under its EFn name.
        // Iterate `embedded_font_order` (insertion order) rather than
        // the HashMap itself so output PDFs are byte-reproducible
        // regardless of HashMap randomisation. Drained because
        // `build_embedded_font_objects` takes `&mut EmbeddedFont`.
        //
        // Each font produces a `GlyphRemapper` (subset GID → new GID)
        // that the content-stream builder needs at serialisation time
        // to renumber every `ShowEmbeddedText` op into the subset's
        // dense 0..N GID space. We collect them keyed by resource name
        // (e.g. "EF1") and pass the whole map into every page's
        // `build_with_remappers` below. FONT-3b.
        let mut embedded = std::mem::take(&mut self.embedded_fonts);
        let order = std::mem::take(&mut self.embedded_font_order);
        let mut embedded_object_ids: Vec<u32> = Vec::new();
        let mut font_remappers: HashMap<String, crate::fonts::GlyphRemapper> = HashMap::new();
        for resource_name in order {
            let Some(mut font) = embedded.remove(&resource_name) else {
                continue;
            };
            // Allocate IDs upfront so we don't need to borrow `self` inside
            // the build closure.
            let mut allocated: Vec<u32> = (0..5)
                .map(|_| {
                    let id = self.next_obj_id;
                    self.next_obj_id += 1;
                    id
                })
                .collect();
            let (ids, objects, remapper) =
                super::font_pdf_objects::build_embedded_font_objects(&mut font, || {
                    allocated.remove(0)
                })?;
            font_resources.insert(resource_name.clone(), ObjectSerializer::reference(ids.type0, 0));
            for (id, obj) in objects {
                embedded_object_ids.push(id);
                self.objects.insert(id, obj);
            }
            font_remappers.insert(resource_name.clone(), remapper);
        }

        // Catalog object (object 1)
        let catalog_id = self.alloc_obj_id();
        let pages_id = self.alloc_obj_id();

        // Pre-allocate object IDs for all pages
        let page_count = self.pages.len();
        let mut page_ids: Vec<(u32, u32)> = Vec::with_capacity(page_count);
        for _ in 0..page_count {
            let page_id = self.alloc_obj_id();
            let content_id = self.alloc_obj_id();
            page_ids.push((page_id, content_id));
        }

        // Pre-allocate annotation IDs for all pages
        // First collect annotation counts to avoid borrow conflict
        let annot_counts: Vec<usize> = self.pages.iter().map(|p| p.annotations.len()).collect();
        let mut annot_ids: Vec<Vec<u32>> = Vec::with_capacity(page_count);
        for count in annot_counts {
            let mut page_annot_ids = Vec::with_capacity(count);
            for _ in 0..count {
                page_annot_ids.push(self.alloc_obj_id());
            }
            annot_ids.push(page_annot_ids);
        }

        // Pre-allocate form field IDs for all pages
        let form_field_counts: Vec<usize> =
            self.pages.iter().map(|p| p.form_fields.len()).collect();
        let mut form_field_ids: Vec<Vec<u32>> = Vec::with_capacity(page_count);
        for count in form_field_counts {
            let mut page_field_ids = Vec::with_capacity(count);
            for _ in 0..count {
                page_field_ids.push(self.alloc_obj_id());
            }
            form_field_ids.push(page_field_ids);
        }

        // Build page ObjectRefs for annotation destinations (internal links)
        let page_obj_refs: Vec<ObjectRef> = page_ids
            .iter()
            .map(|(page_id, _)| ObjectRef::new(*page_id, 0))
            .collect();

        // Create page objects
        let mut page_refs: Vec<Object> = Vec::new();
        let mut page_objects: Vec<(u32, Object, Vec<u8>)> = Vec::new();
        let mut annotation_objects: Vec<(u32, Object)> = Vec::new();
        let mut form_field_objects: Vec<(u32, Object)> = Vec::new();
        let mut all_field_refs: Vec<ObjectRef> = Vec::new();

        // Image XObjects — per page, capture the (resource_id, ImageData,
        // soft_mask_id?) tuples and pre-allocate object IDs so the main
        // page-build loop can weave them into Resources without needing
        // a second &mut borrow on `self`.
        let mut pending_per_page: Vec<Vec<super::content_stream::PendingImage>> =
            Vec::with_capacity(page_count);
        for page_data in self.pages.iter_mut() {
            pending_per_page.push(page_data.content_builder.take_pending_images());
        }
        let mut image_ids_per_page: Vec<Vec<(u32, Option<u32>)>> = Vec::with_capacity(page_count);
        let mut image_objects: Vec<(u32, Object, Vec<u8>)> = Vec::new();
        for pending in &pending_per_page {
            let mut per_page_ids: Vec<(u32, Option<u32>)> = Vec::with_capacity(pending.len());
            for p in pending {
                // Decode to writer::ImageData — the content stream
                // builder kept the elements::ImageContent verbatim; we
                // need the ColorSpace/Filter conversion from elements
                // to build the XObject dict.
                let (data, soft_mask) = image_content_to_xobject_stream(&p.image);
                let img_id = self.alloc_obj_id();
                let soft_mask_id = if soft_mask.is_some() {
                    Some(self.alloc_obj_id())
                } else {
                    None
                };
                // Build dictionaries.
                let mut dict: HashMap<String, Object> = data.build_xobject_dict();
                if let Some(sm_id) = soft_mask_id {
                    dict.insert("SMask".to_string(), Object::Reference(ObjectRef::new(sm_id, 0)));
                }
                image_objects.push((
                    img_id,
                    Object::Stream {
                        dict,
                        data: bytes::Bytes::from(data.data.clone()),
                    },
                    Vec::new(),
                ));
                if let (Some(sm_id), Some(sm_data)) = (soft_mask_id, &data.soft_mask) {
                    let sm_dict = data.build_soft_mask_dict().expect("soft mask present");
                    image_objects.push((
                        sm_id,
                        Object::Stream {
                            dict: sm_dict,
                            data: bytes::Bytes::from(sm_data.clone()),
                        },
                        Vec::new(),
                    ));
                }
                per_page_ids.push((img_id, soft_mask_id));
            }
            image_ids_per_page.push(per_page_ids);
        }

        for (i, page_data) in self.pages.iter().enumerate() {
            let (page_id, content_id) = page_ids[i];
            let page_ref = ObjectRef::new(page_id, 0);

            // Build content stream, threading the per-font remappers
            // through so every `ShowEmbeddedText` op is renumbered into
            // the subset's dense GID space (FONT-3b).
            let raw_content = page_data
                .content_builder
                .build_with_remappers(&font_remappers)?;

            // Optionally compress the content stream
            let (content_bytes, is_compressed) = if self.config.compress {
                match compress_data(&raw_content) {
                    Ok(compressed) => (compressed, true),
                    Err(_) => (raw_content, false), // Fall back to uncompressed on error
                }
            } else {
                (raw_content, false)
            };

            // Create content stream object
            let mut content_dict = HashMap::new();
            content_dict.insert("Length".to_string(), Object::Integer(content_bytes.len() as i64));
            if is_compressed {
                content_dict.insert("Filter".to_string(), Object::Name("FlateDecode".to_string()));
            }

            // Build annotation objects for this page
            let mut annot_refs: Vec<Object> = Vec::new();
            if !page_data.annotations.is_empty() {
                let annot_dicts = page_data.annotations.build(&page_obj_refs);
                for (j, annot_dict) in annot_dicts.into_iter().enumerate() {
                    let annot_id = annot_ids[i][j];
                    annotation_objects.push((annot_id, Object::Dictionary(annot_dict)));
                    annot_refs.push(Object::Reference(ObjectRef::new(annot_id, 0)));
                }
            }

            // Build form field objects for this page
            for (j, field_entry) in page_data.form_fields.iter().enumerate() {
                let field_id = form_field_ids[i][j];
                let field_ref = ObjectRef::new(field_id, 0);
                all_field_refs.push(field_ref);

                // Build merged field/widget dictionary
                let mut field_dict = field_entry.field_dict.clone();

                // Update widget dict with correct page reference
                let mut widget_dict = field_entry.widget_dict.clone();
                widget_dict.insert("P".to_string(), Object::Reference(page_ref));

                // Merge widget entries into field dict (merged field/widget)
                for (key, value) in widget_dict {
                    field_dict.insert(key, value);
                }

                form_field_objects.push((field_id, Object::Dictionary(field_dict)));
                annot_refs.push(Object::Reference(field_ref));
            }

            // Build Resources dict — Font always, XObject when this
            // page produced any image content during paint.
            let mut resource_entries: Vec<(&str, Object)> =
                vec![("Font", Object::Dictionary(font_resources.clone()))];
            let pending = &pending_per_page[i];
            let image_ids = &image_ids_per_page[i];
            if !pending.is_empty() {
                let mut xobject_dict: HashMap<String, Object> = HashMap::new();
                for (pi, (img_id, _)) in pending.iter().zip(image_ids.iter()) {
                    xobject_dict.insert(
                        pi.resource_id.clone(),
                        Object::Reference(ObjectRef::new(*img_id, 0)),
                    );
                }
                resource_entries.push(("XObject", Object::Dictionary(xobject_dict)));
            }

            // Page object
            let mut page_entries: Vec<(&str, Object)> = vec![
                ("Type", ObjectSerializer::name("Page")),
                ("Parent", ObjectSerializer::reference(pages_id, 0)),
                (
                    "MediaBox",
                    ObjectSerializer::rect(
                        0.0,
                        0.0,
                        page_data.width as f64,
                        page_data.height as f64,
                    ),
                ),
                ("Contents", ObjectSerializer::reference(content_id, 0)),
                ("Resources", ObjectSerializer::dict(resource_entries)),
            ];

            // Add Annots array if page has annotations
            if !annot_refs.is_empty() {
                page_entries.push(("Annots", Object::Array(annot_refs)));
            }

            let page_obj = ObjectSerializer::dict(page_entries);

            page_refs.push(Object::Reference(ObjectRef::new(page_id, 0)));
            page_objects.push((page_id, page_obj, Vec::new()));
            page_objects.push((
                content_id,
                Object::Stream {
                    dict: content_dict,
                    data: bytes::Bytes::from(content_bytes),
                },
                Vec::new(),
            ));
        }

        // Pages object
        let pages_obj = ObjectSerializer::dict(vec![
            ("Type", ObjectSerializer::name("Pages")),
            ("Kids", Object::Array(page_refs)),
            ("Count", ObjectSerializer::integer(self.pages.len() as i64)),
        ]);

        // Build AcroForm if there are form fields
        let acroform_id = if !all_field_refs.is_empty() {
            let id = self.alloc_obj_id();
            let mut acroform = self.acroform.take().unwrap_or_default();
            acroform.add_fields(all_field_refs);
            let acroform_dict = acroform.build_with_resources();
            self.objects.insert(id, Object::Dictionary(acroform_dict));
            Some(id)
        } else {
            None
        };

        // Catalog object
        let mut catalog_entries = vec![
            ("Type", ObjectSerializer::name("Catalog")),
            ("Pages", ObjectSerializer::reference(pages_id, 0)),
        ];
        if let Some(acroform_id) = acroform_id {
            catalog_entries.push(("AcroForm", ObjectSerializer::reference(acroform_id, 0)));
        }
        let catalog_obj = ObjectSerializer::dict(catalog_entries);

        // Info object (optional metadata)
        let info_id = self.alloc_obj_id();
        let mut info_entries = Vec::new();
        if let Some(title) = &self.config.title {
            info_entries.push(("Title", ObjectSerializer::string(title)));
        }
        if let Some(author) = &self.config.author {
            info_entries.push(("Author", ObjectSerializer::string(author)));
        }
        if let Some(subject) = &self.config.subject {
            info_entries.push(("Subject", ObjectSerializer::string(subject)));
        }
        if let Some(creator) = &self.config.creator {
            info_entries.push(("Creator", ObjectSerializer::string(creator)));
        }
        let info_obj = ObjectSerializer::dict(info_entries);

        // Write all objects
        // Catalog
        xref_offsets.push((catalog_id, output.len()));
        output.extend_from_slice(&serializer.serialize_indirect(catalog_id, 0, &catalog_obj));

        // Pages
        xref_offsets.push((pages_id, output.len()));
        output.extend_from_slice(&serializer.serialize_indirect(pages_id, 0, &pages_obj));

        // Font objects (Base-14)
        for font_ref in self.fonts.values() {
            if let Some(font_obj) = self.objects.get(&font_ref.id) {
                xref_offsets.push((font_ref.id, output.len()));
                output.extend_from_slice(&serializer.serialize_indirect(font_ref.id, 0, font_obj));
            }
        }

        // Embedded font objects (FONT-3): the five-object graph per font
        // (Type 0, CIDFontType2, FontDescriptor, FontFile2 stream,
        // ToUnicode stream).
        for &id in &embedded_object_ids {
            if let Some(obj) = self.objects.get(&id) {
                xref_offsets.push((id, output.len()));
                output.extend_from_slice(&serializer.serialize_indirect(id, 0, obj));
            }
        }

        // Page and content objects
        for (obj_id, obj, _) in &page_objects {
            xref_offsets.push((*obj_id, output.len()));
            output.extend_from_slice(&serializer.serialize_indirect(*obj_id, 0, obj));
        }

        // Image XObject streams (from HTML <img> / add_element Image).
        for (obj_id, obj, _) in &image_objects {
            xref_offsets.push((*obj_id, output.len()));
            output.extend_from_slice(&serializer.serialize_indirect(*obj_id, 0, obj));
        }

        // Annotation objects
        for (annot_id, annot_obj) in &annotation_objects {
            xref_offsets.push((*annot_id, output.len()));
            output.extend_from_slice(&serializer.serialize_indirect(*annot_id, 0, annot_obj));
        }

        // Form field objects
        for (field_id, field_obj) in &form_field_objects {
            xref_offsets.push((*field_id, output.len()));
            output.extend_from_slice(&serializer.serialize_indirect(*field_id, 0, field_obj));
        }

        // AcroForm object (if present)
        if let Some(acroform_id) = acroform_id {
            if let Some(acroform_obj) = self.objects.get(&acroform_id) {
                xref_offsets.push((acroform_id, output.len()));
                output.extend_from_slice(&serializer.serialize_indirect(
                    acroform_id,
                    0,
                    acroform_obj,
                ));
            }
        }

        // Info object
        xref_offsets.push((info_id, output.len()));
        output.extend_from_slice(&serializer.serialize_indirect(info_id, 0, &info_obj));

        // Write xref table
        let xref_start = output.len();
        writeln!(output, "xref")?;
        writeln!(output, "0 {}", self.next_obj_id)?;

        // Object 0 is always free
        writeln!(output, "0000000000 65535 f ")?;

        // Sort xref entries by object ID
        xref_offsets.sort_by_key(|(id, _)| *id);

        for (_, offset) in &xref_offsets {
            writeln!(output, "{:010} 00000 n ", offset)?;
        }

        // Write trailer
        let trailer = ObjectSerializer::dict(vec![
            ("Size", ObjectSerializer::integer(self.next_obj_id as i64)),
            ("Root", ObjectSerializer::reference(catalog_id, 0)),
            ("Info", ObjectSerializer::reference(info_id, 0)),
        ]);

        writeln!(output, "trailer")?;
        output.extend_from_slice(&serializer.serialize(&trailer));
        writeln!(output)?;
        writeln!(output, "startxref")?;
        writeln!(output, "{}", xref_start)?;
        write!(output, "%%EOF")?;

        Ok(output)
    }

    /// Save the PDF to a file.
    pub fn save(self, path: impl AsRef<std::path::Path>) -> Result<()> {
        let bytes = self.finish()?;
        std::fs::write(path, bytes)?;
        Ok(())
    }
}

impl Default for PdfWriter {
    fn default() -> Self {
        Self::new()
    }
}

/// Convert an [`elements::ImageContent`] (what `PageBuilder::add_element`
/// accepts) into the matching [`super::image_handler::ImageData`] so
/// the XObject stream dictionary + soft mask dict can be emitted. The
/// two structs carry the same payload but are owned by different
/// layers — this keeps the paint pipeline plugged into the standard
/// writer-side serializer without reaching across module boundaries.
fn image_content_to_xobject_stream(
    image: &crate::elements::ImageContent,
) -> (super::image_handler::ImageData, Option<Vec<u8>>) {
    use super::image_handler::{ColorSpace as WColorSpace, ImageData, ImageFormat as WImageFormat};
    let color_space = match image.color_space {
        crate::elements::ColorSpace::Gray => WColorSpace::DeviceGray,
        crate::elements::ColorSpace::CMYK => WColorSpace::DeviceCMYK,
        crate::elements::ColorSpace::RGB => WColorSpace::DeviceRGB,
        // The writer's ImageData doesn't currently model Indexed or
        // Lab. The html_css paint pipeline only produces Gray / RGB /
        // CMYK ImageContents so this branch is latent — but if a
        // caller constructs an ImageContent with Indexed or Lab
        // directly (and routes it through `PageBuilder::add_element`),
        // silently coercing to RGB would produce wrong colours.
        // Fall back to RGB to keep the XObject emittable, and emit a
        // warning so the miscoloration is diagnosable.
        crate::elements::ColorSpace::Indexed | crate::elements::ColorSpace::Lab => {
            log::warn!(
                "image_content_to_xobject_stream: ColorSpace::{:?} is not yet supported by \
                 the writer pipeline; falling back to DeviceRGB (colours may be wrong)",
                image.color_space
            );
            WColorSpace::DeviceRGB
        },
    };
    let format = match image.format {
        crate::elements::ImageFormat::Jpeg => WImageFormat::Jpeg,
        crate::elements::ImageFormat::Png => WImageFormat::Png,
        _ => WImageFormat::Raw,
    };
    // Carry the alpha channel forward if the caller attached one
    // (PNG RGBA / LA). `ImageData::from_png` compresses alpha upstream,
    // so `soft_mask` here is already the FlateDecode payload ready to
    // stream straight into the /SMask XObject.
    let soft_mask = image.soft_mask.clone();
    let data = ImageData {
        width: image.width,
        height: image.height,
        bits_per_component: image.bits_per_component,
        color_space,
        format,
        data: image.data.clone(),
        soft_mask: soft_mask.clone(),
    };
    (data, soft_mask)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_empty_pdf() {
        let writer = PdfWriter::new();
        let mut writer = writer;
        writer.add_letter_page().finish();
        let bytes = writer.finish().unwrap();

        let content = String::from_utf8_lossy(&bytes);
        assert!(content.starts_with("%PDF-1.7"));
        assert!(content.contains("/Type /Catalog"));
        assert!(content.contains("/Type /Pages"));
        assert!(content.contains("/Type /Page"));
        assert!(content.contains("%%EOF"));
    }

    #[test]
    fn test_pdf_with_text() {
        let mut writer = PdfWriter::new();
        {
            let mut page = writer.add_letter_page();
            page.add_text("Hello, World!", 72.0, 720.0, "Helvetica", 12.0);
            page.finish();
        }

        let bytes = writer.finish().unwrap();
        let content = String::from_utf8_lossy(&bytes);

        assert!(content.contains("/Type /Font"));
        assert!(content.contains("/BaseFont /Helvetica"));
        assert!(content.contains("BT"));
        assert!(content.contains("(Hello, World!) Tj"));
        assert!(content.contains("ET"));
    }

    #[test]
    fn test_pdf_with_metadata() {
        let config = PdfWriterConfig::default()
            .with_title("Test Document")
            .with_author("Test Author");

        let mut writer = PdfWriter::with_config(config);
        writer.add_letter_page().finish();

        let bytes = writer.finish().unwrap();
        let content = String::from_utf8_lossy(&bytes);

        assert!(content.contains("/Title (Test Document)"));
        assert!(content.contains("/Author (Test Author)"));
    }

    #[test]
    fn test_multiple_pages() {
        let mut writer = PdfWriter::new();
        writer.add_letter_page().finish();
        writer.add_a4_page().finish();

        let bytes = writer.finish().unwrap();
        let content = String::from_utf8_lossy(&bytes);

        assert!(content.contains("/Count 2"));
        // Two MediaBox entries for different page sizes
        assert!(content.contains("[0 0 612 792]")); // Letter
        assert!(content.contains("[0 0 595 842]")); // A4
    }

    #[test]
    fn test_page_builder() {
        let mut writer = PdfWriter::new();
        {
            let mut page = writer.add_letter_page();
            page.add_text("Line 1", 72.0, 720.0, "Helvetica", 12.0);
            page.add_text("Line 2", 72.0, 700.0, "Helvetica", 12.0);
            page.finish();
        }

        let bytes = writer.finish().unwrap();
        assert!(!bytes.is_empty());
    }

    #[test]
    fn test_pdf_with_link_annotation() {
        let mut writer = PdfWriter::new();
        {
            let mut page = writer.add_letter_page();
            page.add_text("Click here to visit Rust", 72.0, 720.0, "Helvetica", 12.0);
            page.link(Rect::new(72.0, 720.0, 150.0, 12.0), "https://www.rust-lang.org");
            page.finish();
        }

        let bytes = writer.finish().unwrap();
        let content = String::from_utf8_lossy(&bytes);

        // Verify annotation structure
        assert!(content.contains("/Type /Annot"));
        assert!(content.contains("/Subtype /Link"));
        assert!(content.contains("/Annots"));
        assert!(content.contains("rust-lang.org"));
    }

    #[test]
    fn test_pdf_with_internal_link() {
        let mut writer = PdfWriter::new();

        // Page 1 with link to page 2
        {
            let mut page = writer.add_letter_page();
            page.add_text("Go to page 2", 72.0, 720.0, "Helvetica", 12.0);
            page.internal_link(Rect::new(72.0, 720.0, 100.0, 12.0), 1);
            page.finish();
        }

        // Page 2 (target)
        {
            let mut page = writer.add_letter_page();
            page.add_text("This is page 2", 72.0, 720.0, "Helvetica", 12.0);
            page.finish();
        }

        let bytes = writer.finish().unwrap();
        let content = String::from_utf8_lossy(&bytes);

        assert!(content.contains("/Type /Annot"));
        assert!(content.contains("/Subtype /Link"));
        assert!(content.contains("/Dest")); // Destination for internal link
        assert!(content.contains("/Fit")); // Fit mode
    }

    #[test]
    fn test_pdf_with_multiple_annotations() {
        let mut writer = PdfWriter::new();
        {
            let mut page = writer.add_letter_page();
            page.link(Rect::new(72.0, 720.0, 100.0, 12.0), "https://example1.com");
            page.link(Rect::new(72.0, 700.0, 100.0, 12.0), "https://example2.com");
            page.link(Rect::new(72.0, 680.0, 100.0, 12.0), "https://example3.com");
            page.finish();
        }

        let bytes = writer.finish().unwrap();
        let content = String::from_utf8_lossy(&bytes);

        // Count occurrences of /Type /Annot
        let annot_count = content.matches("/Type /Annot").count();
        assert_eq!(annot_count, 3, "Expected 3 annotations");
    }

    #[test]
    fn test_pdf_with_highlight() {
        let mut writer = PdfWriter::new();
        {
            let mut page = writer.add_letter_page();
            page.add_text("Important text to highlight", 72.0, 720.0, "Helvetica", 12.0);
            page.highlight_rect(Rect::new(72.0, 720.0, 150.0, 12.0));
            page.finish();
        }

        let bytes = writer.finish().unwrap();
        let content = String::from_utf8_lossy(&bytes);

        assert!(content.contains("/Type /Annot"));
        assert!(content.contains("/Subtype /Highlight"));
        assert!(content.contains("/QuadPoints"));
        assert!(content.contains("/Annots"));
    }

    #[test]
    fn test_pdf_with_all_text_markup_types() {
        let mut writer = PdfWriter::new();
        {
            let mut page = writer.add_letter_page();
            // Add all four text markup types
            page.highlight_rect(Rect::new(72.0, 720.0, 100.0, 12.0));
            page.underline_rect(Rect::new(72.0, 700.0, 100.0, 12.0));
            page.strikeout_rect(Rect::new(72.0, 680.0, 100.0, 12.0));
            page.squiggly_rect(Rect::new(72.0, 660.0, 100.0, 12.0));
            page.finish();
        }

        let bytes = writer.finish().unwrap();
        let content = String::from_utf8_lossy(&bytes);

        assert!(content.contains("/Subtype /Highlight"));
        assert!(content.contains("/Subtype /Underline"));
        assert!(content.contains("/Subtype /StrikeOut"));
        assert!(content.contains("/Subtype /Squiggly"));

        // Should have 4 annotations
        let annot_count = content.matches("/Type /Annot").count();
        assert_eq!(annot_count, 4, "Expected 4 text markup annotations");
    }

    #[test]
    fn test_pdf_with_mixed_annotations() {
        let mut writer = PdfWriter::new();
        {
            let mut page = writer.add_letter_page();
            // Mix link and text markup annotations
            page.link(Rect::new(72.0, 720.0, 100.0, 12.0), "https://example.com");
            page.highlight_rect(Rect::new(72.0, 700.0, 100.0, 12.0));
            page.underline_rect(Rect::new(72.0, 680.0, 100.0, 12.0));
            page.finish();
        }

        let bytes = writer.finish().unwrap();
        let content = String::from_utf8_lossy(&bytes);

        // Should have 3 annotations total
        let annot_count = content.matches("/Type /Annot").count();
        assert_eq!(annot_count, 3, "Expected 3 mixed annotations");

        assert!(content.contains("/Subtype /Link"));
        assert!(content.contains("/Subtype /Highlight"));
        assert!(content.contains("/Subtype /Underline"));
    }

    #[test]
    fn test_pdf_with_sticky_note() {
        let mut writer = PdfWriter::new();
        {
            let mut page = writer.add_letter_page();
            page.add_text("Document with a note", 72.0, 720.0, "Helvetica", 12.0);
            page.sticky_note(Rect::new(72.0, 700.0, 24.0, 24.0), "This is an important note!");
            page.finish();
        }

        let bytes = writer.finish().unwrap();
        let content = String::from_utf8_lossy(&bytes);

        assert!(content.contains("/Type /Annot"));
        assert!(content.contains("/Subtype /Text"));
        assert!(content.contains("/Name /Note"));
        assert!(content.contains("/Annots"));
        assert!(content.contains("important note"));
    }

    #[test]
    fn test_pdf_with_comment_annotation() {
        let mut writer = PdfWriter::new();
        {
            let mut page = writer.add_letter_page();
            page.comment(Rect::new(72.0, 720.0, 24.0, 24.0), "Review comment here");
            page.finish();
        }

        let bytes = writer.finish().unwrap();
        let content = String::from_utf8_lossy(&bytes);

        assert!(content.contains("/Type /Annot"));
        assert!(content.contains("/Subtype /Text"));
        assert!(content.contains("/Name /Comment"));
    }

    #[test]
    fn test_pdf_with_text_note_icons() {
        use crate::annotation_types::TextAnnotationIcon;

        let mut writer = PdfWriter::new();
        {
            let mut page = writer.add_letter_page();
            // Add notes with different icons
            page.text_note_with_icon(
                Rect::new(72.0, 720.0, 24.0, 24.0),
                "Help note",
                TextAnnotationIcon::Help,
            );
            page.text_note_with_icon(
                Rect::new(100.0, 720.0, 24.0, 24.0),
                "Key note",
                TextAnnotationIcon::Key,
            );
            page.text_note_with_icon(
                Rect::new(128.0, 720.0, 24.0, 24.0),
                "Insert note",
                TextAnnotationIcon::Insert,
            );
            page.finish();
        }

        let bytes = writer.finish().unwrap();
        let content = String::from_utf8_lossy(&bytes);

        assert!(content.contains("/Name /Help"));
        assert!(content.contains("/Name /Key"));
        assert!(content.contains("/Name /Insert"));

        // Should have 3 text annotations
        let annot_count = content.matches("/Subtype /Text").count();
        assert_eq!(annot_count, 3, "Expected 3 text annotations with different icons");
    }

    #[test]
    fn test_pdf_with_all_annotation_types() {
        use crate::annotation_types::TextAnnotationIcon;

        let mut writer = PdfWriter::new();

        // Page 1 with link to page 2
        {
            let mut page = writer.add_letter_page();
            page.add_text("Comprehensive annotation test", 72.0, 750.0, "Helvetica", 14.0);

            // Link annotation
            page.link(Rect::new(72.0, 720.0, 100.0, 12.0), "https://example.com");

            // Text markup annotations
            page.highlight_rect(Rect::new(72.0, 700.0, 100.0, 12.0));
            page.underline_rect(Rect::new(72.0, 680.0, 100.0, 12.0));
            page.strikeout_rect(Rect::new(72.0, 660.0, 100.0, 12.0));
            page.squiggly_rect(Rect::new(72.0, 640.0, 100.0, 12.0));

            // Text annotations (sticky notes)
            page.sticky_note(Rect::new(200.0, 720.0, 24.0, 24.0), "A sticky note");
            page.comment(Rect::new(200.0, 680.0, 24.0, 24.0), "A comment");
            page.text_note_with_icon(
                Rect::new(200.0, 640.0, 24.0, 24.0),
                "Help text",
                TextAnnotationIcon::Help,
            );

            // Internal link
            page.internal_link(Rect::new(72.0, 600.0, 100.0, 12.0), 1);

            page.finish();
        }

        // Page 2 (target)
        {
            let mut page = writer.add_letter_page();
            page.add_text("Page 2", 72.0, 720.0, "Helvetica", 12.0);
            page.finish();
        }

        let bytes = writer.finish().unwrap();
        let content = String::from_utf8_lossy(&bytes);

        // Verify all annotation types are present
        assert!(content.contains("/Subtype /Link"));
        assert!(content.contains("/Subtype /Highlight"));
        assert!(content.contains("/Subtype /Underline"));
        assert!(content.contains("/Subtype /StrikeOut"));
        assert!(content.contains("/Subtype /Squiggly"));
        assert!(content.contains("/Subtype /Text"));

        // Should have 9 annotations on page 1:
        // 2 links + 4 text markup + 3 sticky notes = 9
        let annot_count = content.matches("/Type /Annot").count();
        assert_eq!(annot_count, 9, "Expected 9 annotations total");
    }

    #[test]
    fn test_pdf_with_textbox() {
        let mut writer = PdfWriter::new();
        {
            let mut page = writer.add_letter_page();
            page.add_text("Document with text box", 72.0, 750.0, "Helvetica", 14.0);
            page.textbox(Rect::new(72.0, 650.0, 200.0, 80.0), "This is a text box annotation");
            page.finish();
        }

        let bytes = writer.finish().unwrap();
        let content = String::from_utf8_lossy(&bytes);

        assert!(content.contains("/Type /Annot"));
        assert!(content.contains("/Subtype /FreeText"));
        assert!(content.contains("/DA")); // Default Appearance
        assert!(content.contains("/Annots"));
    }

    #[test]
    fn test_pdf_with_styled_textbox() {
        let mut writer = PdfWriter::new();
        {
            let mut page = writer.add_letter_page();
            page.textbox_styled(
                Rect::new(72.0, 600.0, 250.0, 60.0),
                "Styled text content",
                "Courier",
                14.0,
            );
            page.finish();
        }

        let bytes = writer.finish().unwrap();
        let content = String::from_utf8_lossy(&bytes);

        assert!(content.contains("/Subtype /FreeText"));
        assert!(content.contains("/Cour")); // Courier font
        assert!(content.contains("14")); // Font size
    }

    #[test]
    fn test_pdf_with_centered_textbox() {
        let mut writer = PdfWriter::new();
        {
            let mut page = writer.add_letter_page();
            page.textbox_centered(Rect::new(100.0, 500.0, 200.0, 40.0), "Centered text");
            page.finish();
        }

        let bytes = writer.finish().unwrap();
        let content = String::from_utf8_lossy(&bytes);

        assert!(content.contains("/Subtype /FreeText"));
        assert!(content.contains("/Q 1")); // Center alignment
    }

    #[test]
    fn test_pdf_with_callout() {
        let mut writer = PdfWriter::new();
        {
            let mut page = writer.add_letter_page();
            // Callout with leader line from (50, 550) to (72, 600)
            page.callout(
                Rect::new(72.0, 600.0, 150.0, 50.0),
                "Callout annotation",
                vec![50.0, 550.0, 72.0, 600.0],
            );
            page.finish();
        }

        let bytes = writer.finish().unwrap();
        let content = String::from_utf8_lossy(&bytes);

        assert!(content.contains("/Subtype /FreeText"));
        assert!(content.contains("/IT /FreeTextCallout")); // Intent
        assert!(content.contains("/CL")); // Callout line
    }

    #[test]
    fn test_pdf_with_typewriter() {
        let mut writer = PdfWriter::new();
        {
            let mut page = writer.add_letter_page();
            page.typewriter(Rect::new(72.0, 500.0, 300.0, 20.0), "Typewriter text");
            page.finish();
        }

        let bytes = writer.finish().unwrap();
        let content = String::from_utf8_lossy(&bytes);

        assert!(content.contains("/Subtype /FreeText"));
        assert!(content.contains("/IT /FreeTextTypeWriter")); // Intent
    }

    #[test]
    fn test_pdf_with_multiple_freetext_types() {
        let mut writer = PdfWriter::new();
        {
            let mut page = writer.add_letter_page();
            page.textbox(Rect::new(72.0, 700.0, 150.0, 40.0), "Basic text box");
            page.textbox_centered(Rect::new(72.0, 640.0, 150.0, 40.0), "Centered box");
            page.typewriter(Rect::new(72.0, 580.0, 200.0, 20.0), "Typewriter");
            page.callout(
                Rect::new(300.0, 700.0, 150.0, 40.0),
                "Callout",
                vec![250.0, 680.0, 300.0, 720.0],
            );
            page.finish();
        }

        let bytes = writer.finish().unwrap();
        let content = String::from_utf8_lossy(&bytes);

        // Should have 4 FreeText annotations
        let freetext_count = content.matches("/Subtype /FreeText").count();
        assert_eq!(freetext_count, 4, "Expected 4 FreeText annotations");
    }

    #[test]
    fn test_pdf_with_line_annotation() {
        let mut writer = PdfWriter::new();
        {
            let mut page = writer.add_letter_page();
            page.line((100.0, 100.0), (300.0, 100.0));
            page.finish();
        }

        let bytes = writer.finish().unwrap();
        let content = String::from_utf8_lossy(&bytes);

        assert!(content.contains("/Subtype /Line"));
        assert!(content.contains("/L ")); // Line coordinates
    }

    #[test]
    fn test_pdf_with_arrow_annotation() {
        let mut writer = PdfWriter::new();
        {
            let mut page = writer.add_letter_page();
            page.arrow((100.0, 200.0), (300.0, 200.0));
            page.finish();
        }

        let bytes = writer.finish().unwrap();
        let content = String::from_utf8_lossy(&bytes);

        assert!(content.contains("/Subtype /Line"));
        assert!(content.contains("/LE")); // Line endings
        assert!(content.contains("/OpenArrow"));
    }

    #[test]
    fn test_pdf_with_rectangle_annotation() {
        let mut writer = PdfWriter::new();
        {
            let mut page = writer.add_letter_page();
            page.rectangle(Rect::new(100.0, 400.0, 150.0, 100.0));
            page.finish();
        }

        let bytes = writer.finish().unwrap();
        let content = String::from_utf8_lossy(&bytes);

        assert!(content.contains("/Subtype /Square"));
    }

    #[test]
    fn test_pdf_with_circle_annotation() {
        let mut writer = PdfWriter::new();
        {
            let mut page = writer.add_letter_page();
            page.circle(Rect::new(300.0, 400.0, 100.0, 100.0));
            page.finish();
        }

        let bytes = writer.finish().unwrap();
        let content = String::from_utf8_lossy(&bytes);

        assert!(content.contains("/Subtype /Circle"));
    }

    #[test]
    fn test_pdf_with_filled_shapes() {
        let mut writer = PdfWriter::new();
        {
            let mut page = writer.add_letter_page();
            page.rectangle_filled(
                Rect::new(100.0, 300.0, 100.0, 80.0),
                (0.0, 0.0, 1.0), // Blue stroke
                (0.8, 0.8, 1.0), // Light blue fill
            );
            page.circle_filled(
                Rect::new(250.0, 300.0, 80.0, 80.0),
                (1.0, 0.0, 0.0), // Red stroke
                (1.0, 0.8, 0.8), // Light red fill
            );
            page.finish();
        }

        let bytes = writer.finish().unwrap();
        let content = String::from_utf8_lossy(&bytes);

        assert!(content.contains("/Subtype /Square"));
        assert!(content.contains("/Subtype /Circle"));
        assert!(content.contains("/IC")); // Interior color
    }

    #[test]
    fn test_pdf_with_polygon() {
        let mut writer = PdfWriter::new();
        {
            let mut page = writer.add_letter_page();
            // Triangle
            page.polygon(vec![(100.0, 100.0), (150.0, 200.0), (50.0, 200.0)]);
            page.finish();
        }

        let bytes = writer.finish().unwrap();
        let content = String::from_utf8_lossy(&bytes);

        assert!(content.contains("/Subtype /Polygon"));
        assert!(content.contains("/Vertices"));
    }

    #[test]
    fn test_pdf_with_polyline() {
        let mut writer = PdfWriter::new();
        {
            let mut page = writer.add_letter_page();
            page.polyline(vec![
                (100.0, 500.0),
                (200.0, 550.0),
                (300.0, 500.0),
                (400.0, 550.0),
            ]);
            page.finish();
        }

        let bytes = writer.finish().unwrap();
        let content = String::from_utf8_lossy(&bytes);

        assert!(content.contains("/Subtype /PolyLine"));
        assert!(content.contains("/Vertices"));
    }

    #[test]
    fn test_pdf_with_all_shape_types() {
        let mut writer = PdfWriter::new();
        {
            let mut page = writer.add_letter_page();
            // Line
            page.line((72.0, 750.0), (200.0, 750.0));
            // Arrow
            page.arrow((72.0, 700.0), (200.0, 700.0));
            // Rectangle
            page.rectangle(Rect::new(72.0, 600.0, 100.0, 50.0));
            // Circle
            page.circle(Rect::new(200.0, 600.0, 50.0, 50.0));
            // Polygon
            page.polygon(vec![(300.0, 600.0), (350.0, 650.0), (250.0, 650.0)]);
            // Polyline
            page.polyline(vec![(72.0, 500.0), (150.0, 550.0), (250.0, 500.0)]);
            page.finish();
        }

        let bytes = writer.finish().unwrap();
        let content = String::from_utf8_lossy(&bytes);

        // Verify all shape types
        assert!(content.contains("/Subtype /Line"));
        assert!(content.contains("/Subtype /Square"));
        assert!(content.contains("/Subtype /Circle"));
        assert!(content.contains("/Subtype /Polygon"));
        assert!(content.contains("/Subtype /PolyLine"));

        // Should have 6 shape annotations
        let annot_count = content.matches("/Type /Annot").count();
        assert_eq!(annot_count, 6, "Expected 6 shape annotations");
    }

    #[test]
    fn test_pdf_with_ink_annotation() {
        let mut writer = PdfWriter::new();
        {
            let mut page = writer.add_letter_page();
            page.ink(vec![(100.0, 100.0), (150.0, 120.0), (200.0, 100.0)]);
            page.finish();
        }

        let bytes = writer.finish().unwrap();
        let content = String::from_utf8_lossy(&bytes);

        assert!(content.contains("/Type /Annot"));
        assert!(content.contains("/Subtype /Ink"));
        assert!(content.contains("/InkList"));
    }

    #[test]
    fn test_pdf_with_freehand_multiple_strokes() {
        let mut writer = PdfWriter::new();
        {
            let mut page = writer.add_letter_page();
            page.freehand(vec![
                vec![(100.0, 100.0), (150.0, 120.0), (200.0, 100.0)],
                vec![(100.0, 200.0), (200.0, 200.0)],
                vec![(150.0, 150.0), (150.0, 250.0)],
            ]);
            page.finish();
        }

        let bytes = writer.finish().unwrap();
        let content = String::from_utf8_lossy(&bytes);

        assert!(content.contains("/Subtype /Ink"));
        assert!(content.contains("/InkList"));
        // Should have 1 ink annotation
        let ink_count = content.matches("/Subtype /Ink").count();
        assert_eq!(ink_count, 1, "Expected 1 Ink annotation");
    }

    #[test]
    fn test_pdf_with_styled_ink() {
        let mut writer = PdfWriter::new();
        {
            let mut page = writer.add_letter_page();
            page.ink_styled(
                vec![(100.0, 300.0), (200.0, 350.0), (300.0, 300.0)],
                (1.0, 0.0, 0.0), // Red
                3.0,             // 3pt line width
            );
            page.finish();
        }

        let bytes = writer.finish().unwrap();
        let content = String::from_utf8_lossy(&bytes);

        assert!(content.contains("/Subtype /Ink"));
        assert!(content.contains("/C")); // Color
        assert!(content.contains("/BS")); // Border style
    }

    #[test]
    fn test_pdf_with_multiple_ink_annotations() {
        let mut writer = PdfWriter::new();
        {
            let mut page = writer.add_letter_page();
            // Add multiple separate ink annotations
            page.ink(vec![(100.0, 100.0), (150.0, 120.0)]);
            page.ink(vec![(200.0, 100.0), (250.0, 120.0)]);
            page.ink_styled(
                vec![(300.0, 100.0), (350.0, 120.0)],
                (0.0, 0.0, 1.0), // Blue
                2.0,
            );
            page.finish();
        }

        let bytes = writer.finish().unwrap();
        let content = String::from_utf8_lossy(&bytes);

        // Should have 3 ink annotations
        let ink_count = content.matches("/Subtype /Ink").count();
        assert_eq!(ink_count, 3, "Expected 3 Ink annotations");
    }

    #[test]
    fn test_pdf_with_ink_and_other_annotations() {
        let mut writer = PdfWriter::new();
        {
            let mut page = writer.add_letter_page();
            // Mix ink with other annotations
            page.ink(vec![(100.0, 100.0), (200.0, 150.0)]);
            page.highlight_rect(Rect::new(72.0, 700.0, 100.0, 12.0));
            page.sticky_note(Rect::new(300.0, 700.0, 24.0, 24.0), "Note");
            page.line((72.0, 600.0), (200.0, 600.0));
            page.finish();
        }

        let bytes = writer.finish().unwrap();
        let content = String::from_utf8_lossy(&bytes);

        assert!(content.contains("/Subtype /Ink"));
        assert!(content.contains("/Subtype /Highlight"));
        assert!(content.contains("/Subtype /Text"));
        assert!(content.contains("/Subtype /Line"));

        // Should have 4 annotations
        let annot_count = content.matches("/Type /Annot").count();
        assert_eq!(annot_count, 4, "Expected 4 mixed annotations");
    }

    #[test]
    fn test_pdf_with_approved_stamp() {
        let mut writer = PdfWriter::new();
        {
            let mut page = writer.add_letter_page();
            page.stamp_approved(Rect::new(400.0, 700.0, 150.0, 50.0));
            page.finish();
        }

        let bytes = writer.finish().unwrap();
        let content = String::from_utf8_lossy(&bytes);

        assert!(content.contains("/Type /Annot"));
        assert!(content.contains("/Subtype /Stamp"));
        assert!(content.contains("/Name /Approved"));
    }

    #[test]
    fn test_pdf_with_draft_stamp() {
        let mut writer = PdfWriter::new();
        {
            let mut page = writer.add_letter_page();
            page.stamp_draft(Rect::new(400.0, 650.0, 120.0, 40.0));
            page.finish();
        }

        let bytes = writer.finish().unwrap();
        let content = String::from_utf8_lossy(&bytes);

        assert!(content.contains("/Subtype /Stamp"));
        assert!(content.contains("/Name /Draft"));
    }

    #[test]
    fn test_pdf_with_confidential_stamp() {
        let mut writer = PdfWriter::new();
        {
            let mut page = writer.add_letter_page();
            page.stamp_confidential(Rect::new(400.0, 600.0, 150.0, 50.0));
            page.finish();
        }

        let bytes = writer.finish().unwrap();
        let content = String::from_utf8_lossy(&bytes);

        assert!(content.contains("/Subtype /Stamp"));
        assert!(content.contains("/Name /Confidential"));
    }

    #[test]
    fn test_pdf_with_custom_stamp() {
        let mut writer = PdfWriter::new();
        {
            let mut page = writer.add_letter_page();
            page.stamp_custom(Rect::new(400.0, 550.0, 150.0, 50.0), "ReviewPending");
            page.finish();
        }

        let bytes = writer.finish().unwrap();
        let content = String::from_utf8_lossy(&bytes);

        assert!(content.contains("/Subtype /Stamp"));
        assert!(content.contains("/Name /ReviewPending"));
    }

    #[test]
    fn test_pdf_with_multiple_stamps() {
        let mut writer = PdfWriter::new();
        {
            let mut page = writer.add_letter_page();
            page.stamp_approved(Rect::new(400.0, 700.0, 100.0, 40.0));
            page.stamp_draft(Rect::new(400.0, 650.0, 100.0, 40.0));
            page.stamp_final(Rect::new(400.0, 600.0, 100.0, 40.0));
            page.stamp_for_comment(Rect::new(400.0, 550.0, 100.0, 40.0));
            page.finish();
        }

        let bytes = writer.finish().unwrap();
        let content = String::from_utf8_lossy(&bytes);

        // Should have 4 stamp annotations
        let stamp_count = content.matches("/Subtype /Stamp").count();
        assert_eq!(stamp_count, 4, "Expected 4 Stamp annotations");

        assert!(content.contains("/Name /Approved"));
        assert!(content.contains("/Name /Draft"));
        assert!(content.contains("/Name /Final"));
        assert!(content.contains("/Name /ForComment"));
    }

    #[test]
    fn test_pdf_with_stamp_and_other_annotations() {
        let mut writer = PdfWriter::new();
        {
            let mut page = writer.add_letter_page();
            page.stamp_approved(Rect::new(400.0, 700.0, 150.0, 50.0));
            page.highlight_rect(Rect::new(72.0, 700.0, 100.0, 12.0));
            page.sticky_note(Rect::new(200.0, 700.0, 24.0, 24.0), "Note");
            page.line((72.0, 600.0), (200.0, 600.0));
            page.finish();
        }

        let bytes = writer.finish().unwrap();
        let content = String::from_utf8_lossy(&bytes);

        assert!(content.contains("/Subtype /Stamp"));
        assert!(content.contains("/Subtype /Highlight"));
        assert!(content.contains("/Subtype /Text"));
        assert!(content.contains("/Subtype /Line"));

        // Should have 4 annotations
        let annot_count = content.matches("/Type /Annot").count();
        assert_eq!(annot_count, 4, "Expected 4 mixed annotations");
    }

    // ============ Special Annotations Tests ============

    #[test]
    fn test_pdf_with_popup_annotation() {
        let mut writer = PdfWriter::new();
        {
            let mut page = writer.add_letter_page();
            page.popup(Rect::new(200.0, 600.0, 200.0, 100.0), true);
            page.finish();
        }

        let bytes = writer.finish().unwrap();
        let content = String::from_utf8_lossy(&bytes);

        assert!(content.contains("/Type /Annot"));
        assert!(content.contains("/Subtype /Popup"));
        assert!(content.contains("/Rect"));
        assert!(content.contains("/Open true"));
    }

    #[test]
    fn test_pdf_with_caret_annotation() {
        let mut writer = PdfWriter::new();
        {
            let mut page = writer.add_letter_page();
            page.caret(Rect::new(100.0, 700.0, 20.0, 20.0));
            page.caret_paragraph(Rect::new(100.0, 650.0, 20.0, 20.0));
            page.caret_with_comment(
                Rect::new(100.0, 600.0, 20.0, 20.0),
                "Insert new paragraph here",
            );
            page.finish();
        }

        let bytes = writer.finish().unwrap();
        let content = String::from_utf8_lossy(&bytes);

        let caret_count = content.matches("/Subtype /Caret").count();
        assert_eq!(caret_count, 3, "Expected 3 Caret annotations");

        assert!(content.contains("/Sy /None"));
        assert!(content.contains("/Sy /P"));
        assert!(content.contains("Insert new paragraph here"));
    }

    #[test]
    fn test_pdf_with_file_attachment_annotation() {
        let mut writer = PdfWriter::new();
        {
            let mut page = writer.add_letter_page();
            page.file_attachment(Rect::new(50.0, 700.0, 24.0, 24.0), "document.pdf");
            page.file_attachment_paperclip(Rect::new(50.0, 650.0, 24.0, 24.0), "notes.txt");
            page.finish();
        }

        let bytes = writer.finish().unwrap();
        let content = String::from_utf8_lossy(&bytes);

        let attach_count = content.matches("/Subtype /FileAttachment").count();
        assert_eq!(attach_count, 2, "Expected 2 FileAttachment annotations");

        assert!(content.contains("/Name /PushPin"));
        assert!(content.contains("/Name /Paperclip"));
        assert!(content.contains("document.pdf"));
        assert!(content.contains("notes.txt"));
    }

    #[test]
    fn test_pdf_with_redact_annotation() {
        let mut writer = PdfWriter::new();
        {
            let mut page = writer.add_letter_page();
            page.redact(Rect::new(100.0, 700.0, 200.0, 20.0));
            page.redact_with_text(Rect::new(100.0, 650.0, 200.0, 20.0), "REDACTED");
            page.finish();
        }

        let bytes = writer.finish().unwrap();
        let content = String::from_utf8_lossy(&bytes);

        let redact_count = content.matches("/Subtype /Redact").count();
        assert_eq!(redact_count, 2, "Expected 2 Redact annotations");

        assert!(content.contains("REDACTED"));
    }

    #[test]
    fn test_pdf_with_mixed_special_annotations() {
        let mut writer = PdfWriter::new();
        {
            let mut page = writer.add_letter_page();
            page.popup(Rect::new(200.0, 700.0, 150.0, 80.0), false);
            page.caret(Rect::new(100.0, 650.0, 20.0, 20.0));
            page.file_attachment(Rect::new(50.0, 600.0, 24.0, 24.0), "report.pdf");
            page.redact(Rect::new(100.0, 550.0, 200.0, 20.0));
            page.finish();
        }

        let bytes = writer.finish().unwrap();
        let content = String::from_utf8_lossy(&bytes);

        assert!(content.contains("/Subtype /Popup"));
        assert!(content.contains("/Subtype /Caret"));
        assert!(content.contains("/Subtype /FileAttachment"));
        assert!(content.contains("/Subtype /Redact"));

        let annot_count = content.matches("/Type /Annot").count();
        assert_eq!(annot_count, 4, "Expected 4 special annotations");
    }

    #[test]
    fn test_pdf_with_complete_annotation_coverage() {
        let mut writer = PdfWriter::new();
        {
            let mut page = writer.add_letter_page();
            // Link
            page.link(Rect::new(72.0, 750.0, 100.0, 20.0), "https://example.com");
            // Text markup
            page.highlight_rect(Rect::new(72.0, 720.0, 100.0, 12.0));
            page.underline_rect(Rect::new(72.0, 700.0, 100.0, 12.0));
            // Sticky note
            page.sticky_note(Rect::new(200.0, 720.0, 24.0, 24.0), "Note");
            // FreeText
            page.textbox(Rect::new(72.0, 660.0, 150.0, 30.0), "Comment here");
            // Shapes
            page.line((72.0, 620.0), (200.0, 620.0));
            page.rectangle(Rect::new(72.0, 570.0, 50.0, 50.0));
            page.circle(Rect::new(140.0, 570.0, 50.0, 50.0));
            // Ink
            page.ink(vec![(72.0, 520.0), (100.0, 540.0), (130.0, 520.0)]);
            // Stamp
            page.stamp_approved(Rect::new(400.0, 700.0, 100.0, 40.0));
            // Special
            page.popup(Rect::new(400.0, 600.0, 150.0, 80.0), false);
            page.caret(Rect::new(400.0, 550.0, 20.0, 20.0));
            page.file_attachment(Rect::new(400.0, 500.0, 24.0, 24.0), "data.xlsx");
            page.redact(Rect::new(400.0, 450.0, 150.0, 20.0));
            page.finish();
        }

        let bytes = writer.finish().unwrap();
        let content = String::from_utf8_lossy(&bytes);

        // Verify all annotation types are present
        assert!(content.contains("/Subtype /Link"));
        assert!(content.contains("/Subtype /Highlight"));
        assert!(content.contains("/Subtype /Underline"));
        assert!(content.contains("/Subtype /Text"));
        assert!(content.contains("/Subtype /FreeText"));
        assert!(content.contains("/Subtype /Line"));
        assert!(content.contains("/Subtype /Square"));
        assert!(content.contains("/Subtype /Circle"));
        assert!(content.contains("/Subtype /Ink"));
        assert!(content.contains("/Subtype /Stamp"));
        assert!(content.contains("/Subtype /Popup"));
        assert!(content.contains("/Subtype /Caret"));
        assert!(content.contains("/Subtype /FileAttachment"));
        assert!(content.contains("/Subtype /Redact"));

        let annot_count = content.matches("/Type /Annot").count();
        assert_eq!(annot_count, 14, "Expected 14 different annotation types");
    }
}
