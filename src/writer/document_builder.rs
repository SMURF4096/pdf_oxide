//! High-level document builder with fluent API.
//!
//! Provides a convenient interface for building PDF documents
//! using method chaining, wrapping the lower-level PdfWriter.
//!
//! # Annotations
//!
//! The fluent API supports adding annotations directly to text elements:
//!
//! ```ignore
//! use pdf_oxide::writer::{DocumentBuilder, PageSize};
//!
//! let mut builder = DocumentBuilder::new();
//! builder
//!     .page(PageSize::Letter)
//!     .at(72.0, 720.0)
//!     .text("Click here for more info")
//!     .link_url("https://example.com")  // Link the previous text
//!     .text("Important note")
//!     .highlight((1.0, 1.0, 0.0))       // Highlight in yellow
//!     .sticky_note("Review this section")
//!     .done();
//! ```

use super::annotation_builder::{Annotation, LinkAnnotation};
use super::font_manager::{EmbeddedFont, FontManager, TextLayout};
use super::freetext::FreeTextAnnotation;
use super::page_template::PageTemplate;
use super::pdf_writer::{PdfWriter, PdfWriterConfig};
use super::stamp::{StampAnnotation, StampType};
use super::table_renderer::{FontMetrics, Table};
use super::text_annotations::TextAnnotation;
use super::text_markup::TextMarkupAnnotation;
use super::watermark::WatermarkAnnotation;
use crate::annotation_types::{TextAnnotationIcon, TextMarkupType};
use crate::elements::{ContentElement, TextContent};
use crate::error::Result;
use crate::geometry::Rect;
use std::path::Path;

/// Metadata for a PDF document.
#[derive(Debug, Clone, Default)]
pub struct DocumentMetadata {
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
    /// PDF version (default: "1.7")
    pub version: Option<String>,
}

impl DocumentMetadata {
    /// Create new empty metadata.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set document title.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set document author.
    pub fn author(mut self, author: impl Into<String>) -> Self {
        self.author = Some(author.into());
        self
    }

    /// Set document subject.
    pub fn subject(mut self, subject: impl Into<String>) -> Self {
        self.subject = Some(subject.into());
        self
    }

    /// Set document keywords.
    pub fn keywords(mut self, keywords: impl Into<String>) -> Self {
        self.keywords = Some(keywords.into());
        self
    }

    /// Set creator application.
    pub fn creator(mut self, creator: impl Into<String>) -> Self {
        self.creator = Some(creator.into());
        self
    }
}

/// Standard page sizes.
#[derive(Debug, Clone, Copy)]
pub enum PageSize {
    /// US Letter (8.5" x 11")
    Letter,
    /// A4 (210mm x 297mm)
    A4,
    /// Legal (8.5" x 14")
    Legal,
    /// A3 (297mm x 420mm)
    A3,
    /// Custom dimensions in points
    Custom(f32, f32),
}

impl PageSize {
    /// Get dimensions in points (1 inch = 72 points).
    pub fn dimensions(&self) -> (f32, f32) {
        match self {
            PageSize::Letter => (612.0, 792.0),
            PageSize::A4 => (595.0, 842.0),
            PageSize::Legal => (612.0, 1008.0),
            PageSize::A3 => (842.0, 1190.0),
            PageSize::Custom(w, h) => (*w, *h),
        }
    }
}

/// Text alignment options.
#[derive(Debug, Clone, Copy, Default)]
pub enum TextAlign {
    /// Left-aligned text (default)
    #[default]
    Left,
    /// Center-aligned text
    Center,
    /// Right-aligned text
    Right,
}

/// Configuration for text rendering.
#[derive(Debug, Clone)]
pub struct TextConfig {
    /// Font name (default: Helvetica)
    pub font: String,
    /// Font size in points (default: 12)
    pub size: f32,
    /// Text alignment
    pub align: TextAlign,
    /// Line height multiplier (default: 1.2)
    pub line_height: f32,
}

impl Default for TextConfig {
    fn default() -> Self {
        Self {
            font: "Helvetica".to_string(),
            size: 12.0,
            align: TextAlign::Left,
            line_height: 1.2,
        }
    }
}

/// Stroke style for line-drawing primitives (`stroke_rect`, `stroke_line`).
///
/// Introduced alongside the buffered `Table` surface so cell borders and
/// row rules can have explicit thickness and colour without forcing users
/// through the lower-level `ContentElement::Path` builder.
///
/// **Dash patterns are not in scope for v0.3.39** — the content-stream
/// writer does not yet emit `d` (set-dash) ops. Track: issue #400 v0.3.40
/// follow-ups.
#[derive(Debug, Clone, Copy)]
pub struct LineStyle {
    /// Stroke width in points. Must be > 0.
    pub width: f32,
    /// RGB colour, each channel in `0.0..=1.0`.
    pub color: (f32, f32, f32),
}

impl Default for LineStyle {
    fn default() -> Self {
        Self {
            width: 1.0,
            color: (0.0, 0.0, 0.0),
        }
    }
}

impl LineStyle {
    /// Construct a `LineStyle` from a width (points) and RGB colour
    /// channels (each `0.0..=1.0`).
    pub fn new(width: f32, r: f32, g: f32, b: f32) -> Self {
        Self {
            width,
            color: (r, g, b),
        }
    }
}

/// Page builder for adding content to a page with fluent API.
pub struct FluentPageBuilder<'a> {
    builder: &'a mut DocumentBuilder,
    page_index: usize,
    cursor_x: f32,
    cursor_y: f32,
    text_config: TextConfig,
    text_layout: TextLayout,
    /// Track the last text element's bounding box for text markup annotations
    last_text_rect: Option<Rect>,
    /// Pending annotations for this page
    pending_annotations: Vec<Annotation>,
}

impl<'a> FluentPageBuilder<'a> {
    /// Set the text configuration for subsequent text operations.
    pub fn text_config(mut self, config: TextConfig) -> Self {
        self.text_config = config;
        self
    }

    /// Set font for subsequent text operations.
    pub fn font(mut self, name: &str, size: f32) -> Self {
        self.text_config.font = name.to_string();
        self.text_config.size = size;
        self
    }

    /// Set cursor position for text placement.
    pub fn at(mut self, x: f32, y: f32) -> Self {
        self.cursor_x = x;
        self.cursor_y = y;
        self
    }

    /// Vertical points remaining on the current page from the cursor down to
    /// the bottom margin (conventionally 72 pt / 1 inch from the bottom of
    /// the page). Pure query — no mutation, no emission.
    ///
    /// The streaming `Table` surface (research #393) uses this to decide
    /// whether the next row fits or whether to trigger a page break:
    ///
    /// ```no_run
    /// # use pdf_oxide::writer::DocumentBuilder;
    /// # let mut doc = DocumentBuilder::new();
    /// # let page = doc.letter_page();
    /// if page.remaining_space() < 40.0 {
    ///     // ... trigger new_page_same_size() and redraw header
    /// }
    /// ```
    ///
    /// Returns 0.0 (not negative) when the cursor has already passed the
    /// bottom margin. A return of > 0.0 does not *guarantee* the next row
    /// fits — that depends on the row's own measured height.
    pub fn remaining_space(&self) -> f32 {
        const BOTTOM_MARGIN: f32 = 72.0;
        (self.cursor_y - BOTTOM_MARGIN).max(0.0)
    }

    /// Finish the current page and start a new one with the **same page
    /// size**. The builder's `text_config` (font, size, alignment,
    /// line-height multiplier) carries over; cursor resets to the same
    /// top-left origin the current page started at.
    ///
    /// Pending annotations and form fields on the current page are
    /// committed before the new page is opened. Does not re-draw any
    /// header / footer — callers that want header-repeat-on-break (tables,
    /// long documents) must redraw explicitly.
    pub fn new_page_same_size(mut self) -> FluentPageBuilder<'a> {
        self.new_page_same_size_inplace();
        self
    }

    /// In-place page break: same semantics as `new_page_same_size` but
    /// mutates `self` rather than consuming it. Used by `StreamingTable`
    /// (which borrows its `FluentPageBuilder` and can't consume).
    pub(crate) fn new_page_same_size_inplace(&mut self) {
        let current = &self.builder.pages[self.page_index];
        let width = current.width;
        let height = current.height;

        // Commit pending annotations to the current page before switching.
        let annotations = std::mem::take(&mut self.pending_annotations);
        self.builder.pages[self.page_index]
            .annotations
            .extend(annotations);

        // Append a fresh PageData with matching dimensions.
        self.page_index = self.builder.pages.len();
        self.builder.pages.push(PageData {
            width,
            height,
            elements: Vec::new(),
            annotations: Vec::new(),
            form_fields: Vec::new(),
        });

        // Reset cursor to top-left (mirrors DocumentBuilder::page).
        self.cursor_x = 72.0;
        self.cursor_y = height - 72.0;
        self.last_text_rect = None;
        // text_config, text_layout, pending_annotations (now empty)
        // carry over automatically.
    }

    /// Current cursor X (points from left edge). Used by
    /// `StreamingTable` to anchor column offsets.
    pub(crate) fn cursor_x(&self) -> f32 {
        self.cursor_x
    }
    /// Current cursor Y (points from bottom edge, PDF convention).
    pub(crate) fn cursor_y(&self) -> f32 {
        self.cursor_y
    }
    /// Move the cursor down to `y`. Internal use — public callers should
    /// use `at()` which takes (x, y).
    pub(crate) fn set_cursor_y(&mut self, y: f32) {
        self.cursor_y = y;
    }
    /// Font name from the builder's current text_config.
    pub(crate) fn text_config_font_name(&self) -> &str {
        &self.text_config.font
    }
    /// Font size in points.
    pub(crate) fn text_config_font_size(&self) -> f32 {
        self.text_config.size
    }
    /// Line-height multiplier (multiplied with font size to get baseline
    /// step).
    pub(crate) fn text_config_line_height(&self) -> f32 {
        self.text_config.line_height
    }
    /// Wrap `text` to `max_width` using the builder's TextLayout engine.
    /// Returns one `(line, measured_width)` per visual line.
    pub(crate) fn wrap_cell_text(&self, text: &str, max_width: f32) -> Vec<(String, f32)> {
        self.text_layout.wrap_text(
            text,
            &self.text_config.font,
            self.text_config.size,
            max_width,
        )
    }
    /// Push a `ContentElement` into the current page's element list.
    pub(crate) fn push_element(&mut self, element: ContentElement) {
        self.builder.pages[self.page_index].elements.push(element);
    }
    /// Number of elements already on the current page — used to seed
    /// monotone reading_order values.
    pub(crate) fn page_element_count(&self) -> usize {
        self.builder.pages[self.page_index].elements.len()
    }

    /// Open a streaming table that consumes this page builder. See
    /// [`super::streaming_table::StreamingTable`] for the full API and
    /// [issue #393](https://github.com/yfedoseev/pdf_oxide/issues/393)
    /// for design rationale.
    pub fn streaming_table(
        self,
        config: super::streaming_table::StreamingTableConfig,
    ) -> super::streaming_table::StreamingTable<'a> {
        super::streaming_table::StreamingTable::open(self, config)
    }

    /// Measure the rendered width of `text` in the builder's current font
    /// and size, in PDF points. Pure query — does not advance the cursor or
    /// emit any content.
    ///
    /// Use this to pick explicit column widths before calling
    /// `streaming_table` (see #393) or to right-align custom labels. For
    /// embedded fonts, the measure honours the face's horizontal advances
    /// (HMTX). For base-14 fonts, it uses the AFM width tables.
    pub fn measure(&self, text: &str) -> f32 {
        self.text_layout.font_manager().text_width(
            text,
            &self.text_config.font,
            self.text_config.size,
        )
    }

    /// Add text at the current cursor position.
    pub fn text(mut self, text: &str) -> Self {
        let text_width = self.text_layout.font_manager().text_width(
            text,
            &self.text_config.font,
            self.text_config.size,
        );

        // Create the bounding box and track it for potential markup annotations
        let text_rect = Rect::new(self.cursor_x, self.cursor_y, text_width, self.text_config.size);
        self.last_text_rect = Some(text_rect);

        let page = &mut self.builder.pages[self.page_index];
        page.elements.push(ContentElement::Text(TextContent {
            text: text.to_string(),
            bbox: text_rect,
            font: crate::elements::FontSpec {
                name: self.text_config.font.clone(),
                size: self.text_config.size,
            },
            style: Default::default(),
            reading_order: Some(page.elements.len()),
            artifact_type: None,
            origin: None,
            rotation_degrees: None,
            matrix: None,
        }));
        // Move cursor down for next line
        self.cursor_y -= self.text_config.size * self.text_config.line_height;
        self
    }

    /// Place wrapped text inside a rectangle with horizontal alignment.
    ///
    /// Wraps `text` to `rect.width` using the builder's current font and
    /// size, emits one content-stream line per wrapped line, and positions
    /// each line within the rect per `align`:
    ///
    /// - `Left`:   line left edge at `rect.x`
    /// - `Center`: line centered within `rect.x .. rect.x + rect.width`
    /// - `Right`:  line right edge at `rect.x + rect.width`
    ///
    /// Vertical layout is top-anchored: the first line's top sits at
    /// `rect.y`, subsequent lines drop by `size * line_height`. The cursor
    /// is **not** advanced — the rect has its own geometry and the caller
    /// owns the cursor. Use `measure()` or `text_layout.text_bounds()` to
    /// pre-compute rect dimensions if needed.
    ///
    /// This is the cell-text primitive the buffered `Table` surface
    /// (research #393) consumes. It is also usable standalone for
    /// box-constrained captions, labels, and pull-quotes.
    pub fn text_in_rect(mut self, rect: Rect, text: &str, align: TextAlign) -> Self {
        let lines = self.text_layout.wrap_text(
            text,
            &self.text_config.font,
            self.text_config.size,
            rect.width,
        );

        let line_height = self.text_config.size * self.text_config.line_height;
        let mut line_top = rect.y;

        for (line_text, line_width) in lines {
            if line_text.is_empty() {
                line_top -= line_height;
                continue;
            }

            let line_x = match align {
                TextAlign::Left => rect.x,
                TextAlign::Center => rect.x + (rect.width - line_width) / 2.0,
                TextAlign::Right => rect.x + rect.width - line_width,
            };

            let bbox = Rect::new(line_x, line_top, line_width, self.text_config.size);
            let page = &mut self.builder.pages[self.page_index];
            page.elements.push(ContentElement::Text(TextContent {
                text: line_text,
                bbox,
                font: crate::elements::FontSpec {
                    name: self.text_config.font.clone(),
                    size: self.text_config.size,
                },
                style: Default::default(),
                reading_order: Some(page.elements.len()),
                artifact_type: None,
                origin: None,
                rotation_degrees: None,
                matrix: None,
            }));

            line_top -= line_height;
        }

        // Track only the last emitted line for potential markup annotations.
        // Table callers typically set markup at the whole-cell level, not
        // per-line; the default matches the behaviour of `.text()`.
        self.last_text_rect = Some(rect);
        self
    }

    /// Add a heading (larger, bold text).
    pub fn heading(self, level: u8, text: &str) -> Self {
        let size = match level {
            1 => 24.0,
            2 => 20.0,
            3 => 16.0,
            _ => 14.0,
        };
        let font = match level {
            1 | 2 => "Helvetica-Bold",
            _ => "Helvetica",
        };
        self.font(font, size).text(text)
    }

    /// Add a paragraph of text with automatic word wrapping.
    pub fn paragraph(mut self, text: &str) -> Self {
        // Use FontManager-based word wrapping for accurate metrics
        let page = &mut self.builder.pages[self.page_index];
        let max_width = page.width - self.cursor_x - 72.0; // 72pt right margin

        let lines = self.text_layout.wrap_text(
            text,
            &self.text_config.font,
            self.text_config.size,
            max_width,
        );

        for (line_text, line_width) in lines {
            let page = &mut self.builder.pages[self.page_index];
            page.elements.push(ContentElement::Text(TextContent {
                text: line_text,
                bbox: Rect::new(self.cursor_x, self.cursor_y, line_width, self.text_config.size),
                font: crate::elements::FontSpec {
                    name: self.text_config.font.clone(),
                    size: self.text_config.size,
                },
                style: Default::default(),
                reading_order: Some(page.elements.len()),
                artifact_type: None,
                origin: None,
                rotation_degrees: None,
                matrix: None,
            }));
            self.cursor_y -= self.text_config.size * self.text_config.line_height;
        }
        // Add extra space after paragraph
        self.cursor_y -= self.text_config.size * 0.5;
        self
    }

    /// Add vertical space.
    pub fn space(mut self, points: f32) -> Self {
        self.cursor_y -= points;
        self
    }

    /// Add a horizontal line.
    pub fn horizontal_rule(mut self) -> Self {
        let page = &mut self.builder.pages[self.page_index];
        let line_y = self.cursor_y + self.text_config.size * 0.5;
        page.elements
            .push(ContentElement::Path(crate::elements::PathContent {
                operations: vec![
                    crate::elements::PathOperation::MoveTo(self.cursor_x, line_y),
                    crate::elements::PathOperation::LineTo(page.width - 72.0, line_y),
                ],
                bbox: Rect::new(self.cursor_x, line_y, page.width - 72.0 - self.cursor_x, 1.0),
                stroke_color: Some(crate::layout::Color {
                    r: 0.5,
                    g: 0.5,
                    b: 0.5,
                }),
                fill_color: None,
                stroke_width: 0.5,
                line_cap: crate::elements::LineCap::Butt,
                line_join: crate::elements::LineJoin::Miter,
                reading_order: None,
            }));
        self.cursor_y -= self.text_config.size;
        self
    }

    /// Add a content element directly.
    pub fn element(self, element: ContentElement) -> Self {
        let page = &mut self.builder.pages[self.page_index];
        page.elements.push(element);
        self
    }

    /// Add multiple content elements.
    pub fn elements(self, elements: Vec<ContentElement>) -> Self {
        let page = &mut self.builder.pages[self.page_index];
        page.elements.extend(elements);
        self
    }

    // ==========================================================================
    // Annotation Methods
    // ==========================================================================

    /// Add a URL link annotation to the last text element.
    ///
    /// The link will cover the bounding box of the most recently added text.
    ///
    /// # Example
    ///
    /// ```ignore
    /// builder.page(PageSize::Letter)
    ///     .at(72.0, 720.0)
    ///     .text("Visit our website")
    ///     .link_url("https://example.com")
    ///     .done();
    /// ```
    pub fn link_url(mut self, url: &str) -> Self {
        if let Some(rect) = self.last_text_rect {
            let link = LinkAnnotation::uri(rect, url);
            self.pending_annotations.push(link.into());
        }
        self
    }

    /// Add an internal page link annotation to the last text element.
    ///
    /// # Arguments
    ///
    /// * `page` - The target page index (0-based)
    ///
    /// # Example
    ///
    /// ```ignore
    /// builder.page(PageSize::Letter)
    ///     .at(72.0, 720.0)
    ///     .text("Go to page 5")
    ///     .link_page(4)  // 0-indexed
    ///     .done();
    /// ```
    pub fn link_page(mut self, page: usize) -> Self {
        if let Some(rect) = self.last_text_rect {
            let link = LinkAnnotation::goto_page(rect, page);
            self.pending_annotations.push(link.into());
        }
        self
    }

    /// Add a named destination link to the last text element.
    ///
    /// # Arguments
    ///
    /// * `destination` - The named destination string
    pub fn link_named(mut self, destination: &str) -> Self {
        if let Some(rect) = self.last_text_rect {
            let link = LinkAnnotation::goto_named(rect, destination);
            self.pending_annotations.push(link.into());
        }
        self
    }

    /// Add a highlight annotation to the last text element.
    ///
    /// # Arguments
    ///
    /// * `color` - RGB color tuple (0.0-1.0 for each component)
    ///
    /// # Example
    ///
    /// ```ignore
    /// builder.page(PageSize::Letter)
    ///     .at(72.0, 720.0)
    ///     .text("Important text")
    ///     .highlight((1.0, 1.0, 0.0))  // Yellow highlight
    ///     .done();
    /// ```
    pub fn highlight(mut self, color: (f32, f32, f32)) -> Self {
        if let Some(rect) = self.last_text_rect {
            let markup = TextMarkupAnnotation::from_rect(TextMarkupType::Highlight, rect)
                .with_color(color.0, color.1, color.2);
            self.pending_annotations.push(markup.into());
        }
        self
    }

    /// Add an underline annotation to the last text element.
    ///
    /// # Arguments
    ///
    /// * `color` - RGB color tuple (0.0-1.0 for each component)
    pub fn underline(mut self, color: (f32, f32, f32)) -> Self {
        if let Some(rect) = self.last_text_rect {
            let markup = TextMarkupAnnotation::from_rect(TextMarkupType::Underline, rect)
                .with_color(color.0, color.1, color.2);
            self.pending_annotations.push(markup.into());
        }
        self
    }

    /// Add a strikeout annotation to the last text element.
    ///
    /// # Arguments
    ///
    /// * `color` - RGB color tuple (0.0-1.0 for each component)
    pub fn strikeout(mut self, color: (f32, f32, f32)) -> Self {
        if let Some(rect) = self.last_text_rect {
            let markup = TextMarkupAnnotation::from_rect(TextMarkupType::StrikeOut, rect)
                .with_color(color.0, color.1, color.2);
            self.pending_annotations.push(markup.into());
        }
        self
    }

    /// Add a squiggly underline annotation to the last text element.
    ///
    /// # Arguments
    ///
    /// * `color` - RGB color tuple (0.0-1.0 for each component)
    pub fn squiggly(mut self, color: (f32, f32, f32)) -> Self {
        if let Some(rect) = self.last_text_rect {
            let markup = TextMarkupAnnotation::from_rect(TextMarkupType::Squiggly, rect)
                .with_color(color.0, color.1, color.2);
            self.pending_annotations.push(markup.into());
        }
        self
    }

    /// Add a sticky note annotation at the current cursor position.
    ///
    /// # Arguments
    ///
    /// * `text` - The note content
    ///
    /// # Example
    ///
    /// ```ignore
    /// builder.page(PageSize::Letter)
    ///     .at(72.0, 720.0)
    ///     .sticky_note("Please review this section")
    ///     .done();
    /// ```
    pub fn sticky_note(mut self, text: &str) -> Self {
        // Place sticky note at current cursor position (small 24x24 icon)
        let rect = Rect::new(self.cursor_x, self.cursor_y, 24.0, 24.0);
        let note = TextAnnotation::new(rect, text);
        self.pending_annotations.push(note.into());
        self
    }

    /// Add a sticky note annotation with a specific icon at the current cursor position.
    ///
    /// # Arguments
    ///
    /// * `text` - The note content
    /// * `icon` - The icon to display
    pub fn sticky_note_with_icon(mut self, text: &str, icon: TextAnnotationIcon) -> Self {
        let rect = Rect::new(self.cursor_x, self.cursor_y, 24.0, 24.0);
        let note = TextAnnotation::new(rect, text).with_icon(icon);
        self.pending_annotations.push(note.into());
        self
    }

    /// Add a sticky note annotation at a specific position.
    ///
    /// # Arguments
    ///
    /// * `x` - X coordinate
    /// * `y` - Y coordinate
    /// * `text` - The note content
    pub fn sticky_note_at(mut self, x: f32, y: f32, text: &str) -> Self {
        let rect = Rect::new(x, y, 24.0, 24.0);
        let note = TextAnnotation::new(rect, text);
        self.pending_annotations.push(note.into());
        self
    }

    /// Add a stamp annotation at the current cursor position.
    ///
    /// # Arguments
    ///
    /// * `stamp_type` - The type of stamp (Approved, Draft, Confidential, etc.)
    ///
    /// # Example
    ///
    /// ```ignore
    /// use pdf_oxide::writer::StampType;
    ///
    /// builder.page(PageSize::Letter)
    ///     .at(72.0, 720.0)
    ///     .stamp(StampType::Approved)
    ///     .done();
    /// ```
    pub fn stamp(mut self, stamp_type: StampType) -> Self {
        // Default stamp size: 150x50 points
        let rect = Rect::new(self.cursor_x, self.cursor_y, 150.0, 50.0);
        let stamp = StampAnnotation::new(rect, stamp_type);
        self.pending_annotations.push(stamp.into());
        self
    }

    /// Add a stamp annotation at a specific position with custom size.
    ///
    /// # Arguments
    ///
    /// * `rect` - The bounding rectangle for the stamp
    /// * `stamp_type` - The type of stamp
    pub fn stamp_at(mut self, rect: Rect, stamp_type: StampType) -> Self {
        let stamp = StampAnnotation::new(rect, stamp_type);
        self.pending_annotations.push(stamp.into());
        self
    }

    /// Add a FreeText annotation (text displayed directly on page).
    ///
    /// # Arguments
    ///
    /// * `rect` - The bounding rectangle for the text box
    /// * `text` - The text content
    pub fn freetext(mut self, rect: Rect, text: &str) -> Self {
        let freetext = FreeTextAnnotation::new(rect, text);
        self.pending_annotations.push(freetext.into());
        self
    }

    /// Add a FreeText annotation with custom font settings.
    ///
    /// # Arguments
    ///
    /// * `rect` - The bounding rectangle for the text box
    /// * `text` - The text content
    /// * `font` - Font name
    /// * `size` - Font size in points
    pub fn freetext_styled(mut self, rect: Rect, text: &str, font: &str, size: f32) -> Self {
        let freetext = FreeTextAnnotation::new(rect, text).with_font(font, size);
        self.pending_annotations.push(freetext.into());
        self
    }

    /// Add a watermark annotation (appears behind content, optionally print-only).
    ///
    /// # Arguments
    ///
    /// * `text` - The watermark text
    ///
    /// # Example
    ///
    /// ```ignore
    /// builder.page(PageSize::Letter)
    ///     .watermark("DRAFT")
    ///     .done();
    /// ```
    pub fn watermark(mut self, text: &str) -> Self {
        let page = &self.builder.pages[self.page_index];
        // Center the watermark on the page with diagonal orientation
        let rect =
            Rect::new(page.width * 0.1, page.height * 0.3, page.width * 0.8, page.height * 0.4);
        let watermark = WatermarkAnnotation::new(text)
            .with_rect(rect)
            .with_rotation(45.0)
            .with_opacity(0.3)
            .with_font("Helvetica", 72.0);
        self.pending_annotations.push(watermark.into());
        self
    }

    /// Add a "CONFIDENTIAL" watermark with preset styling.
    pub fn watermark_confidential(mut self) -> Self {
        let page = &self.builder.pages[self.page_index];
        let rect =
            Rect::new(page.width * 0.1, page.height * 0.3, page.width * 0.8, page.height * 0.4);
        let watermark = WatermarkAnnotation::confidential().with_rect(rect);
        self.pending_annotations.push(watermark.into());
        self
    }

    /// Add a "DRAFT" watermark with preset styling.
    pub fn watermark_draft(mut self) -> Self {
        let page = &self.builder.pages[self.page_index];
        let rect =
            Rect::new(page.width * 0.1, page.height * 0.3, page.width * 0.8, page.height * 0.4);
        let watermark = WatermarkAnnotation::draft().with_rect(rect);
        self.pending_annotations.push(watermark.into());
        self
    }

    /// Add a custom watermark with full control over positioning and styling.
    pub fn watermark_custom(mut self, watermark: WatermarkAnnotation) -> Self {
        self.pending_annotations.push(watermark.into());
        self
    }

    /// Add a generic annotation.
    ///
    /// This is a low-level method that allows adding any annotation type.
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
    /// builder.page(PageSize::Letter)
    ///     .add_annotation(link)
    ///     .done();
    /// ```
    pub fn add_annotation<A: Into<Annotation>>(mut self, annotation: A) -> Self {
        self.pending_annotations.push(annotation.into());
        self
    }

    /// Add a single-line text form field to the page. `name` is the
    /// unique field identifier used for form submission;
    /// `default_value` is the initial text shown in the field (pass
    /// `None` or an empty string for a blank field).
    pub fn text_field(
        self,
        name: impl Into<String>,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        default_value: Option<String>,
    ) -> Self {
        let page = &mut self.builder.pages[self.page_index];
        page.form_fields.push(PendingFormField::TextField {
            name: name.into(),
            rect: Rect::new(x, y, w, h),
            default_value,
        });
        self
    }

    /// Add a checkbox form field to the page. `checked` sets whether
    /// the box is initially ticked.
    pub fn checkbox(
        self,
        name: impl Into<String>,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        checked: bool,
    ) -> Self {
        let page = &mut self.builder.pages[self.page_index];
        page.form_fields.push(PendingFormField::Checkbox {
            name: name.into(),
            rect: Rect::new(x, y, w, h),
            checked,
        });
        self
    }

    /// Add a dropdown combo-box form field. Each entry of `options` is
    /// a user-visible string that also serves as the submitted value.
    /// `selected` picks the initial choice by value; pass `None` to
    /// leave the field blank.
    pub fn combo_box(
        self,
        name: impl Into<String>,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        options: Vec<String>,
        selected: Option<String>,
    ) -> Self {
        let page = &mut self.builder.pages[self.page_index];
        page.form_fields.push(PendingFormField::ComboBox {
            name: name.into(),
            rect: Rect::new(x, y, w, h),
            options,
            selected,
        });
        self
    }

    /// Add a radio-button group. Each entry of `buttons` is an
    /// `(export_value, x, y, w, h)` tuple describing one option's
    /// submitted value and its visible bounding rectangle. `selected`
    /// picks the initial choice by export value.
    pub fn radio_group(
        self,
        name: impl Into<String>,
        buttons: Vec<(String, f32, f32, f32, f32)>,
        selected: Option<String>,
    ) -> Self {
        let page = &mut self.builder.pages[self.page_index];
        let buttons = buttons
            .into_iter()
            .map(|(v, x, y, w, h)| (v, Rect::new(x, y, w, h)))
            .collect();
        page.form_fields.push(PendingFormField::RadioGroup {
            name: name.into(),
            buttons,
            selected,
        });
        self
    }

    /// Add a clickable push button with a visible caption.
    pub fn push_button(
        self,
        name: impl Into<String>,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        caption: impl Into<String>,
    ) -> Self {
        let page = &mut self.builder.pages[self.page_index];
        page.form_fields.push(PendingFormField::PushButton {
            name: name.into(),
            rect: Rect::new(x, y, w, h),
            caption: caption.into(),
        });
        self
    }

    // ───────────────────────────────────────────────────────────────────
    // Low-level graphics primitives (PdfWriter exposure)
    //
    // These emit `ContentElement::Path` directly, the same backing
    // primitive DocumentBuilder already supports via `element()`. Kept
    // as first-class fluent methods because "I want a rectangle" is
    // common enough that forcing users through the lower-level
    // `ContentElement::Path` builder is ergonomically bad across 6
    // bindings.
    // ───────────────────────────────────────────────────────────────────

    /// Place a buffered `Table` at the current cursor position.
    ///
    /// This is the v0.3.39 buffered table surface — see research #393.
    /// The table layout (column widths, row heights, cell positions, wrapped
    /// cell text) is solved against the page's content width
    /// (`page.width - 2 × 72pt`), the result is emitted as a sequence of
    /// `ContentElement::Text` and `ContentElement::Path` via
    /// `Table::to_content_elements`, and the cursor is advanced by the
    /// table's total height.
    ///
    /// **Scope:** in-memory tables. Supports colspan / rowspan / rich cell
    /// styling. Does **not** page-break — if the layout overflows the
    /// current page the overflow is drawn past the bottom margin. For
    /// 1000+ rows that cross page boundaries, use `streaming_table`
    /// (lands in step 5/9).
    ///
    /// Font measurement uses the page-default font (`text_config.font`).
    /// Per-cell font overrides honour the font name string in the cell but
    /// are measured against the table default — good enough for v0.3.39.
    /// Track: #400 v0.3.40 (mixed-font precise metrics).
    pub fn table(mut self, table: Table) -> Self {
        let page_width = self.builder.pages[self.page_index].width;
        let content_width = page_width - 2.0 * 72.0; // match margin convention

        let metrics = FluentFontMetrics {
            manager: self.text_layout.font_manager(),
            font_name: self.text_config.font.clone(),
        };
        let layout = table.calculate_layout(content_width, &metrics);

        let elements = table.to_content_elements(self.cursor_x, self.cursor_y, &layout);
        let n = elements.len();

        let page = &mut self.builder.pages[self.page_index];
        let base_order = page.elements.len();
        for (i, mut elem) in elements.into_iter().enumerate() {
            // Rebase table-local reading_order onto the page's running
            // sequence so subsequent builder calls don't alias orders.
            match &mut elem {
                ContentElement::Text(t) => t.reading_order = Some(base_order + i),
                ContentElement::Path(p) => p.reading_order = Some(base_order + i),
                _ => {},
            }
            page.elements.push(elem);
        }
        let _ = n; // retained for future logging / subsetter-registration path

        self.cursor_y -= layout.total_height;
        self
    }

    /// Draw a stroked rectangle with a caller-supplied `LineStyle`.
    /// Unlike [`Self::rect`] (1pt black default), this exposes width and
    /// colour. Used by the upcoming buffered `Table` surface for per-side
    /// coloured / variable-thickness cell borders (#393 D-P1.3).
    pub fn stroke_rect(self, x: f32, y: f32, w: f32, h: f32, style: LineStyle) -> Self {
        use crate::elements::PathContent;
        use crate::elements::PathOperation;
        let mut path = PathContent::new(Rect::new(x, y, w, h));
        path.operations.push(PathOperation::Rectangle(x, y, w, h));
        path.stroke_color = Some(crate::layout::Color {
            r: style.color.0,
            g: style.color.1,
            b: style.color.2,
        });
        path.fill_color = None;
        path.stroke_width = style.width;
        let page = &mut self.builder.pages[self.page_index];
        page.elements.push(ContentElement::Path(path));
        self
    }

    /// Draw a straight line with a caller-supplied `LineStyle`. Variable-
    /// thickness / coloured rules — e.g. a 0.5pt grey rule between rows.
    pub fn stroke_line(
        self,
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        style: LineStyle,
    ) -> Self {
        use crate::elements::PathContent;
        use crate::elements::PathOperation;
        let min_x = x1.min(x2);
        let min_y = y1.min(y2);
        let w = (x2 - x1).abs().max(1.0);
        let h = (y2 - y1).abs().max(1.0);
        let mut path = PathContent::new(Rect::new(min_x, min_y, w, h));
        path.operations.push(PathOperation::MoveTo(x1, y1));
        path.operations.push(PathOperation::LineTo(x2, y2));
        path.stroke_color = Some(crate::layout::Color {
            r: style.color.0,
            g: style.color.1,
            b: style.color.2,
        });
        path.fill_color = None;
        path.stroke_width = style.width;
        let page = &mut self.builder.pages[self.page_index];
        page.elements.push(ContentElement::Path(path));
        self
    }

    // ───────────────────────────────────────────────────────────────────
    // Shape primitives (circle / ellipse / polygon / arc / bezier_curve)
    // ───────────────────────────────────────────────────────────────────
    // Each primitive accepts an optional LineStyle (stroke) and optional
    // fill colour. `None` for a style leaves that side of the paint
    // undrawn (e.g. fill-only vs stroke-only). Shared helper
    // `push_stroked_fill` applies both to a `PathContent`.

    /// Draw a circle centred at `(cx, cy)` with `radius`. Pass
    /// `stroke = Some(...)` for outlined, `fill = Some((r, g, b))` for
    /// filled. Both together draw a stroked + filled disc.
    pub fn circle(
        self,
        cx: f32,
        cy: f32,
        radius: f32,
        stroke: Option<LineStyle>,
        fill: Option<(f32, f32, f32)>,
    ) -> Self {
        let path = crate::elements::PathContent::circle(cx, cy, radius);
        self.push_shaped_path(path, stroke, fill)
    }

    /// Draw an ellipse centred at `(cx, cy)` with horizontal radius
    /// `rx` and vertical radius `ry`. Same stroke/fill semantics as
    /// [`Self::circle`].
    pub fn ellipse(
        self,
        cx: f32,
        cy: f32,
        rx: f32,
        ry: f32,
        stroke: Option<LineStyle>,
        fill: Option<(f32, f32, f32)>,
    ) -> Self {
        // Magic constant for approximating a quarter-ellipse with a cubic
        // Bezier: k = 4 * (sqrt(2) - 1) / 3.
        use crate::elements::{PathContent, PathOperation};
        const K: f32 = 0.552_284_8;
        let kx = rx * K;
        let ky = ry * K;
        let ops = vec![
            PathOperation::MoveTo(cx, cy + ry),
            PathOperation::CurveTo(cx + kx, cy + ry, cx + rx, cy + ky, cx + rx, cy),
            PathOperation::CurveTo(cx + rx, cy - ky, cx + kx, cy - ry, cx, cy - ry),
            PathOperation::CurveTo(cx - kx, cy - ry, cx - rx, cy - ky, cx - rx, cy),
            PathOperation::CurveTo(cx - rx, cy + ky, cx - kx, cy + ry, cx, cy + ry),
            PathOperation::ClosePath,
        ];
        self.push_shaped_path(PathContent::from_operations(ops), stroke, fill)
    }

    /// Draw a closed polygon through `points`. Requires at least 2
    /// points — fewer is a no-op. Same stroke/fill semantics as
    /// [`Self::circle`].
    pub fn polygon(
        self,
        points: &[(f32, f32)],
        stroke: Option<LineStyle>,
        fill: Option<(f32, f32, f32)>,
    ) -> Self {
        if points.len() < 2 {
            return self;
        }
        use crate::elements::{PathContent, PathOperation};
        let mut ops: Vec<PathOperation> = Vec::with_capacity(points.len() + 2);
        let (x0, y0) = points[0];
        ops.push(PathOperation::MoveTo(x0, y0));
        for &(x, y) in &points[1..] {
            ops.push(PathOperation::LineTo(x, y));
        }
        ops.push(PathOperation::ClosePath);
        self.push_shaped_path(PathContent::from_operations(ops), stroke, fill)
    }

    /// Draw a circular arc centred at `(cx, cy)` with `radius`, from
    /// `start_angle` to `end_angle` (radians, anticlockwise). Only
    /// stroke; arcs are not filled. Approximated by up to 4 cubic
    /// Beziers (one per quadrant), matching the accuracy of the
    /// [`Self::circle`] primitive.
    pub fn arc(
        self,
        cx: f32,
        cy: f32,
        radius: f32,
        start_angle: f32,
        end_angle: f32,
        stroke: LineStyle,
    ) -> Self {
        use crate::elements::{PathContent, PathOperation};
        // Subdivide into arcs of <= π/2 each so a single cubic Bezier
        // stays accurate. Magic ratio for quarter-arcs.
        const K_Q: f32 = 0.552_284_8;
        let (mut a, b) = (start_angle, end_angle);
        let step = std::f32::consts::FRAC_PI_2;
        let mut ops: Vec<PathOperation> =
            vec![PathOperation::MoveTo(cx + radius * a.cos(), cy + radius * a.sin())];
        while a < b {
            let seg_end = (a + step).min(b);
            let sweep = seg_end - a;
            // Bezier control-point length for `sweep` radians around the
            // origin: (4/3) * tan(sweep/4) × radius.
            let k = (4.0 / 3.0) * (sweep / 4.0).tan();
            let (sa, ca) = (a.sin(), a.cos());
            let (sb, cb) = (seg_end.sin(), seg_end.cos());
            let c1x = cx + radius * (ca - k * sa);
            let c1y = cy + radius * (sa + k * ca);
            let c2x = cx + radius * (cb + k * sb);
            let c2y = cy + radius * (sb - k * cb);
            let ex = cx + radius * cb;
            let ey = cy + radius * sb;
            ops.push(PathOperation::CurveTo(c1x, c1y, c2x, c2y, ex, ey));
            a = seg_end;
            // If sweep was full π/2, fall through; otherwise finish.
            if (seg_end - b).abs() < 1e-6 {
                break;
            }
        }
        let _ = K_Q; // quarter-circle shortcut retained for future use
        self.push_shaped_path(PathContent::from_operations(ops), Some(stroke), None)
    }

    /// Draw a single cubic Bezier curve from `(x0, y0)` to `(x3, y3)`
    /// with control points `(c1x, c1y)` and `(c2x, c2y)`. Stroke only
    /// by default; pass `Some((r, g, b))` for fill.
    pub fn bezier_curve(
        self,
        x0: f32,
        y0: f32,
        c1x: f32,
        c1y: f32,
        c2x: f32,
        c2y: f32,
        x3: f32,
        y3: f32,
        stroke: LineStyle,
        fill: Option<(f32, f32, f32)>,
    ) -> Self {
        use crate::elements::{PathContent, PathOperation};
        let ops = vec![
            PathOperation::MoveTo(x0, y0),
            PathOperation::CurveTo(c1x, c1y, c2x, c2y, x3, y3),
        ];
        self.push_shaped_path(PathContent::from_operations(ops), Some(stroke), fill)
    }

    /// Internal helper: apply optional stroke + fill to a `PathContent`
    /// and push it as a `ContentElement::Path` on the current page.
    /// Shared by all shape primitives so stroke/fill semantics stay
    /// consistent across `circle` / `ellipse` / `polygon` / `arc` /
    /// `bezier_curve`.
    fn push_shaped_path(
        self,
        mut path: crate::elements::PathContent,
        stroke: Option<LineStyle>,
        fill: Option<(f32, f32, f32)>,
    ) -> Self {
        if let Some(style) = stroke {
            path.stroke_color = Some(crate::layout::Color {
                r: style.color.0,
                g: style.color.1,
                b: style.color.2,
            });
            path.stroke_width = style.width;
        } else {
            path.stroke_color = None;
        }
        if let Some((r, g, b)) = fill {
            path.fill_color = Some(crate::layout::Color { r, g, b });
        } else {
            path.fill_color = None;
        }
        let page = &mut self.builder.pages[self.page_index];
        page.elements.push(ContentElement::Path(path));
        self
    }

    /// Draw a stroked rectangle outline at `(x, y)` with size `w × h`
    /// using the default 1pt black stroke. For a filled rectangle with
    /// a custom colour, see [`Self::filled_rect`].
    pub fn rect(self, x: f32, y: f32, w: f32, h: f32) -> Self {
        use crate::elements::PathContent;
        use crate::elements::PathOperation;
        let mut path = PathContent::new(Rect::new(x, y, w, h));
        path.operations.push(PathOperation::Rectangle(x, y, w, h));
        path.stroke_color = Some(crate::layout::Color {
            r: 0.0,
            g: 0.0,
            b: 0.0,
        });
        path.fill_color = None;
        let page = &mut self.builder.pages[self.page_index];
        page.elements.push(ContentElement::Path(path));
        self
    }

    /// Draw a filled rectangle at `(x, y)` with size `w × h` in the
    /// given RGB colour (channels in `0.0..=1.0`). No outline.
    pub fn filled_rect(self, x: f32, y: f32, w: f32, h: f32, r: f32, g: f32, b: f32) -> Self {
        use crate::elements::PathContent;
        use crate::elements::PathOperation;
        let mut path = PathContent::new(Rect::new(x, y, w, h));
        path.operations.push(PathOperation::Rectangle(x, y, w, h));
        path.fill_color = Some(crate::layout::Color { r, g, b });
        path.stroke_color = None;
        let page = &mut self.builder.pages[self.page_index];
        page.elements.push(ContentElement::Path(path));
        self
    }

    /// Draw a straight line from `(x1, y1)` to `(x2, y2)` with the
    /// default 1pt black stroke.
    pub fn line(self, x1: f32, y1: f32, x2: f32, y2: f32) -> Self {
        use crate::elements::PathContent;
        use crate::elements::PathOperation;
        let min_x = x1.min(x2);
        let min_y = y1.min(y2);
        let w = (x2 - x1).abs().max(1.0);
        let h = (y2 - y1).abs().max(1.0);
        let mut path = PathContent::new(Rect::new(min_x, min_y, w, h));
        path.operations.push(PathOperation::MoveTo(x1, y1));
        path.operations.push(PathOperation::LineTo(x2, y2));
        path.stroke_color = Some(crate::layout::Color {
            r: 0.0,
            g: 0.0,
            b: 0.0,
        });
        path.fill_color = None;
        let page = &mut self.builder.pages[self.page_index];
        page.elements.push(ContentElement::Path(path));
        self
    }

    /// Finish building this page and return to the document builder.
    pub fn done(mut self) -> &'a mut DocumentBuilder {
        // Move pending annotations to page data
        let page = &mut self.builder.pages[self.page_index];
        page.annotations.append(&mut self.pending_annotations);
        self.builder
    }
}

/// Buffered form-field widget added by `FluentPageBuilder::text_field`
/// etc. Applied to the underlying `pdf_writer::PageBuilder` inside
/// `DocumentBuilder::build`.
enum PendingFormField {
    /// A simple single-line text field.
    TextField {
        name: String,
        rect: Rect,
        default_value: Option<String>,
    },
    /// A checkbox, initially checked or not.
    Checkbox {
        name: String,
        rect: Rect,
        checked: bool,
    },
    /// A dropdown combo-box with a fixed list of string options and an
    /// optional initial selection.
    ComboBox {
        name: String,
        rect: Rect,
        options: Vec<String>,
        selected: Option<String>,
    },
    /// A radio-button group. Each entry in `buttons` has an export
    /// value (the PDF form's submitted value if that button is chosen)
    /// and its own rect.
    RadioGroup {
        name: String,
        buttons: Vec<(String, Rect)>,
        selected: Option<String>,
    },
    /// A clickable push button with a visible caption.
    PushButton {
        name: String,
        rect: Rect,
        caption: String,
    },
}

/// Internal page data for DocumentBuilder.
struct PageData {
    width: f32,
    height: f32,
    elements: Vec<ContentElement>,
    annotations: Vec<Annotation>,
    form_fields: Vec<PendingFormField>,
}

/// High-level document builder with fluent API.
///
/// Provides a convenient way to build PDF documents using method chaining.
///
/// # Example
///
/// ```ignore
/// use pdf_oxide::writer::{DocumentBuilder, PageSize, DocumentMetadata};
///
/// let pdf_bytes = DocumentBuilder::new()
///     .metadata(DocumentMetadata::new().title("My Document"))
///     .page(PageSize::Letter)
///         .at(72.0, 720.0)
///         .heading(1, "Hello, World!")
///         .paragraph("This is a simple PDF document.")
///         .done()
///     .build()?;
/// ```
pub struct DocumentBuilder {
    metadata: DocumentMetadata,
    pages: Vec<PageData>,
    template: Option<PageTemplate>,
    /// Embedded TTF/OTF fonts registered by user-supplied name.
    /// Drained into the internal `PdfWriter` at `build()` time so that
    /// `FluentPageBuilder::font(name, size).text(...)` can emit
    /// CJK / Cyrillic / Greek text via Type-0 hex strings instead of
    /// silently falling back to Helvetica.
    embedded_fonts: Vec<(String, EmbeddedFont)>,
}

impl DocumentBuilder {
    /// Create a new document builder.
    pub fn new() -> Self {
        Self {
            metadata: DocumentMetadata::default(),
            pages: Vec::new(),
            template: None,
            embedded_fonts: Vec::new(),
        }
    }

    /// Register an embedded TrueType/OpenType font under a user-visible
    /// name. The `name` is what callers then pass to
    /// [`FluentPageBuilder::font`]; any `.text(...)` / element emitted
    /// with that font name is routed through the Type-0 / CIDFontType2
    /// path at build time, so Unicode scripts (CJK, Cyrillic, Greek,
    /// Hebrew, Arabic) render correctly.
    ///
    /// Unregistered font names continue to resolve against the
    /// standard base-14 set (Helvetica / Times / Courier families).
    ///
    /// ```ignore
    /// use pdf_oxide::writer::{DocumentBuilder, EmbeddedFont};
    ///
    /// let font = EmbeddedFont::from_file("fonts/NotoSansCJKtc-Regular.otf")?;
    /// let pdf = DocumentBuilder::new()
    ///     .register_embedded_font("NotoSansCJKtc", font)
    ///     .a4_page()
    ///         .font("NotoSansCJKtc", 10.5)
    ///         .at(72.0, 680.0)
    ///         .text("项目: Rust 特性")
    ///         .done()
    ///     .build()?;
    /// ```
    pub fn register_embedded_font(mut self, name: impl Into<String>, font: EmbeddedFont) -> Self {
        self.embedded_fonts.push((name.into(), font));
        self
    }

    /// Set document title (convenience passthrough to
    /// `DocumentMetadata::title`).
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.metadata.title = Some(title.into());
        self
    }

    /// Set document author.
    pub fn author(mut self, author: impl Into<String>) -> Self {
        self.metadata.author = Some(author.into());
        self
    }

    /// Set document subject.
    pub fn subject(mut self, subject: impl Into<String>) -> Self {
        self.metadata.subject = Some(subject.into());
        self
    }

    /// Set document keywords (comma-separated per PDF convention).
    pub fn keywords(mut self, keywords: impl Into<String>) -> Self {
        self.metadata.keywords = Some(keywords.into());
        self
    }

    /// Set the creator application name.
    pub fn creator(mut self, creator: impl Into<String>) -> Self {
        self.metadata.creator = Some(creator.into());
        self
    }

    /// Set document metadata.
    pub fn metadata(mut self, metadata: DocumentMetadata) -> Self {
        self.metadata = metadata;
        self
    }

    /// Set the page template for headers and footers.
    pub fn template(mut self, template: PageTemplate) -> Self {
        self.template = Some(template);
        self
    }

    /// Number of pages in this document so far. Primarily for tests.
    #[allow(dead_code)]
    pub(crate) fn page_count(&self) -> usize {
        self.pages.len()
    }

    /// Elements already queued on page `idx`. Primarily for tests in
    /// sibling modules that can't see the private `pages` field.
    #[allow(dead_code)]
    pub(crate) fn page_elements(&self, idx: usize) -> &[ContentElement] {
        &self.pages[idx].elements
    }

    /// Add a page with the specified size and return a page builder.
    pub fn page(&mut self, size: PageSize) -> FluentPageBuilder<'_> {
        let (width, height) = size.dimensions();
        let page_index = self.pages.len();
        self.pages.push(PageData {
            width,
            height,
            elements: Vec::new(),
            annotations: Vec::new(),
            form_fields: Vec::new(),
        });
        FluentPageBuilder {
            builder: self,
            page_index,
            cursor_x: 72.0,          // 1 inch margin
            cursor_y: height - 72.0, // Start from top with 1 inch margin
            text_config: TextConfig::default(),
            text_layout: TextLayout::new(),
            last_text_rect: None,
            pending_annotations: Vec::new(),
        }
    }

    /// Add a Letter-sized page.
    pub fn letter_page(&mut self) -> FluentPageBuilder<'_> {
        self.page(PageSize::Letter)
    }

    /// Add an A4-sized page.
    pub fn a4_page(&mut self) -> FluentPageBuilder<'_> {
        self.page(PageSize::A4)
    }

    /// Build the PDF document and return the bytes.
    pub fn build(self) -> Result<Vec<u8>> {
        let mut config = PdfWriterConfig::default();
        if let Some(version) = self.metadata.version.clone() {
            config.version = version;
        }
        config.title = self.metadata.title.clone();
        config.author = self.metadata.author.clone();
        config.subject = self.metadata.subject.clone();
        config.keywords = self.metadata.keywords.clone();
        if self.metadata.creator.is_some() {
            config.creator = self.metadata.creator.clone();
        }

        let mut writer = PdfWriter::with_config(config);

        for (user_name, font) in self.embedded_fonts {
            writer.register_embedded_font_as(user_name, font);
        }

        let total_pages = self.pages.len();

        for (idx, page_data) in self.pages.iter().enumerate() {
            let mut page = writer.add_page(page_data.width, page_data.height);

            // 1. Add normal elements
            page.add_elements(&page_data.elements);

            // 2. Apply Template (Headers/Footers) - Draw on top of content
            if let Some(ref template) = self.template {
                let page_number = idx + 1;
                let context =
                    crate::writer::page_template::PlaceholderContext::new(page_number, total_pages)
                        .with_title(self.metadata.title.clone().unwrap_or_default())
                        .with_author(self.metadata.author.clone().unwrap_or_default());

                let layout_engine = TextLayout::new();

                // Apply Header
                if let Some(header) = template.get_header(page_number) {
                    for element in header.elements() {
                        let text = element.resolve(&context);
                        let style = element.style.as_ref().unwrap_or(&header.style);

                        let font_spec = crate::elements::FontSpec {
                            name: style.font_name.clone(),
                            size: style.font_size,
                        };

                        // Calculate width for alignment
                        let (text_width, _) = layout_engine.text_bounds(
                            &text,
                            &font_spec.name,
                            font_spec.size,
                            page_data.width,
                        );

                        let x = match element.alignment {
                            crate::writer::ArtifactAlignment::Left => template.margin_left,
                            crate::writer::ArtifactAlignment::Center => {
                                (page_data.width - text_width) / 2.0
                            },
                            crate::writer::ArtifactAlignment::Right => {
                                page_data.width - template.margin_right - text_width
                            },
                        };
                        let y = page_data.height - header.offset;

                        page.add_element(&ContentElement::Text(TextContent {
                            artifact_type: Some(crate::extractors::text::ArtifactType::Pagination(
                                crate::extractors::text::PaginationSubtype::Header,
                            )),
                            text,
                            bbox: Rect::new(x, y, text_width, style.font_size),
                            font: font_spec,
                            style: crate::elements::TextStyle {
                                color: crate::layout::Color {
                                    r: style.color.0,
                                    g: style.color.1,
                                    b: style.color.2,
                                },
                                weight: match style.font_weight {
                                    crate::writer::font_manager::FontWeight::Normal => {
                                        crate::layout::text_block::FontWeight::Normal
                                    },
                                    crate::writer::font_manager::FontWeight::Bold => {
                                        crate::layout::text_block::FontWeight::Bold
                                    },
                                },
                                ..Default::default()
                            },
                            reading_order: None,
                            origin: None,
                            rotation_degrees: None,
                            matrix: None,
                        }));
                    }
                }

                // Apply Footer
                if let Some(footer) = template.get_footer(page_number) {
                    for element in footer.elements() {
                        let text = element.resolve(&context);
                        let style = element.style.as_ref().unwrap_or(&footer.style);

                        let font_spec = crate::elements::FontSpec {
                            name: style.font_name.clone(),
                            size: style.font_size,
                        };

                        // Calculate width for alignment
                        let (text_width, _) = layout_engine.text_bounds(
                            &text,
                            &font_spec.name,
                            font_spec.size,
                            page_data.width,
                        );

                        let x = match element.alignment {
                            crate::writer::ArtifactAlignment::Left => template.margin_left,
                            crate::writer::ArtifactAlignment::Center => {
                                (page_data.width - text_width) / 2.0
                            },
                            crate::writer::ArtifactAlignment::Right => {
                                page_data.width - template.margin_right - text_width
                            },
                        };
                        let y = footer.offset;

                        page.add_element(&ContentElement::Text(TextContent {
                            artifact_type: Some(crate::extractors::text::ArtifactType::Pagination(
                                crate::extractors::text::PaginationSubtype::Footer,
                            )),
                            text,
                            bbox: Rect::new(x, y, text_width, style.font_size),
                            font: font_spec,
                            style: crate::elements::TextStyle {
                                color: crate::layout::Color {
                                    r: style.color.0,
                                    g: style.color.1,
                                    b: style.color.2,
                                },
                                weight: match style.font_weight {
                                    crate::writer::font_manager::FontWeight::Normal => {
                                        crate::layout::text_block::FontWeight::Normal
                                    },
                                    crate::writer::font_manager::FontWeight::Bold => {
                                        crate::layout::text_block::FontWeight::Bold
                                    },
                                },
                                ..Default::default()
                            },
                            reading_order: None,
                            origin: None,
                            rotation_degrees: None,
                            matrix: None,
                        }));
                    }
                }
            }

            // 3. Add annotations
            for annotation in &page_data.annotations {
                page.add_annotation(annotation.clone());
            }

            // 4. Emit form-field widgets. Each pending entry translates
            //    into the appropriate `pdf_writer::PageBuilder::add_*`
            //    call so the field lands in /AcroForm at finalize time.
            for field in &page_data.form_fields {
                use super::form_fields::{CheckboxWidget, TextFieldWidget};
                match field {
                    PendingFormField::TextField {
                        name,
                        rect,
                        default_value,
                    } => {
                        let widget = TextFieldWidget::new(name.clone(), *rect);
                        let widget = if let Some(default) = default_value {
                            widget.with_default_value(default.clone())
                        } else {
                            widget
                        };
                        page.add_text_field(widget);
                    },
                    PendingFormField::Checkbox {
                        name,
                        rect,
                        checked,
                    } => {
                        let widget = CheckboxWidget::new(name.clone(), *rect);
                        let widget = if *checked { widget.checked() } else { widget };
                        page.add_checkbox(widget);
                    },
                    PendingFormField::ComboBox {
                        name,
                        rect,
                        options,
                        selected,
                    } => {
                        use super::form_fields::ComboBoxWidget;
                        let widget =
                            ComboBoxWidget::new(name.clone(), *rect).with_options(options.clone());
                        let widget = if let Some(v) = selected {
                            widget.with_value(v.clone())
                        } else {
                            widget
                        };
                        page.add_combo_box(widget);
                    },
                    PendingFormField::RadioGroup {
                        name,
                        buttons,
                        selected,
                    } => {
                        use super::form_fields::RadioButtonGroup;
                        let mut group = RadioButtonGroup::new(name.clone());
                        for (value, rect) in buttons {
                            group = group.add_button(value.clone(), *rect, value.clone());
                        }
                        let group = if let Some(v) = selected {
                            group.selected(v.clone())
                        } else {
                            group
                        };
                        page.add_radio_group(group);
                    },
                    PendingFormField::PushButton {
                        name,
                        rect,
                        caption,
                    } => {
                        use super::form_fields::PushButtonWidget;
                        let widget = PushButtonWidget::new(name.clone(), *rect)
                            .with_caption(caption.clone());
                        page.add_push_button(widget);
                    },
                }
            }

            page.finish();
        }

        writer.finish()
    }

    /// Build and save the PDF to a file.
    pub fn save(self, path: impl AsRef<Path>) -> Result<()> {
        let bytes = self.build()?;
        std::fs::write(path, bytes)?;
        Ok(())
    }

    /// Build and save the PDF with AES-256 encryption using the given
    /// user and owner passwords. Grants all permissions — use
    /// [`DocumentBuilder::save_with_encryption`] for a custom
    /// [`crate::editor::EncryptionConfig`] (algorithm + permissions).
    ///
    /// Routes built bytes through the standard encryption pipeline
    /// (`DocumentEditor::save_with_options`), so the resulting PDF is
    /// byte-compatible with any PDF viewer that supports AES-256 (PDF
    /// 2.0 / `/V 5 /R 6`).
    ///
    /// # Example
    ///
    /// ```ignore
    /// use pdf_oxide::writer::{DocumentBuilder, PageSize};
    ///
    /// let mut builder = DocumentBuilder::new();
    /// builder.page(PageSize::A4).at(72.0, 700.0).text("secret").done();
    /// builder.save_encrypted("out.pdf", "user-pw", "owner-pw")?;
    /// # Ok::<(), pdf_oxide::error::Error>(())
    /// ```
    pub fn save_encrypted(
        self,
        path: impl AsRef<Path>,
        user_password: &str,
        owner_password: &str,
    ) -> Result<()> {
        use crate::editor::{EncryptionAlgorithm, EncryptionConfig, Permissions};
        let config = EncryptionConfig {
            user_password: user_password.to_string(),
            owner_password: owner_password.to_string(),
            algorithm: EncryptionAlgorithm::Aes256,
            permissions: Permissions::all(),
        };
        self.save_with_encryption(path, config)
    }

    /// Build and save the PDF with a custom encryption configuration.
    ///
    /// Use this when you need a specific algorithm (RC4-128, AES-128,
    /// AES-256) or restricted permissions. For the common AES-256
    /// all-permissions case, prefer [`DocumentBuilder::save_encrypted`].
    pub fn save_with_encryption(
        self,
        path: impl AsRef<Path>,
        config: crate::editor::EncryptionConfig,
    ) -> Result<()> {
        use crate::editor::{DocumentEditor, EditableDocument, SaveOptions};
        let bytes = self.build()?;
        let mut editor = DocumentEditor::from_bytes(bytes)?;
        editor.save_with_options(path, SaveOptions::with_encryption(config))
    }

    /// Build and return the encrypted PDF as bytes. Mirrors
    /// [`DocumentBuilder::save_encrypted`] but skips the filesystem —
    /// useful for WASM / server pipelines that stream bytes back to a
    /// caller.
    pub fn to_bytes_encrypted(self, user_password: &str, owner_password: &str) -> Result<Vec<u8>> {
        use crate::editor::{EncryptionAlgorithm, EncryptionConfig, Permissions};
        let config = EncryptionConfig {
            user_password: user_password.to_string(),
            owner_password: owner_password.to_string(),
            algorithm: EncryptionAlgorithm::Aes256,
            permissions: Permissions::all(),
        };
        self.to_bytes_with_encryption(config)
    }

    /// Build and return the PDF as encrypted bytes, using a custom
    /// configuration.
    pub fn to_bytes_with_encryption(
        self,
        config: crate::editor::EncryptionConfig,
    ) -> Result<Vec<u8>> {
        use crate::editor::{DocumentEditor, SaveOptions};
        let bytes = self.build()?;
        let mut editor = DocumentEditor::from_bytes(bytes)?;
        editor.save_to_bytes_with_options(SaveOptions::with_encryption(config))
    }
}

impl Default for DocumentBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Adapter that projects `FontManager` into the `table_renderer::FontMetrics`
/// trait against a fixed font name. Lives here (not on FontManager itself)
/// because FontMetrics is a table-renderer-owned abstraction the writer
/// layer doesn't know about.
struct FluentFontMetrics<'a> {
    manager: &'a FontManager,
    font_name: String,
}

impl FontMetrics for FluentFontMetrics<'_> {
    fn text_width(&self, text: &str, font_size: f32) -> f32 {
        self.manager.text_width(text, &self.font_name, font_size)
    }
}

/// Simple word wrapping utility.
#[allow(dead_code)]
fn wrap_text(text: &str, max_chars: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current_line = String::new();

    for word in text.split_whitespace() {
        if current_line.is_empty() {
            current_line = word.to_string();
        } else if current_line.len() + 1 + word.len() <= max_chars {
            current_line.push(' ');
            current_line.push_str(word);
        } else {
            lines.push(current_line);
            current_line = word.to_string();
        }
    }

    if !current_line.is_empty() {
        lines.push(current_line);
    }

    if lines.is_empty() {
        lines.push(String::new());
    }

    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_page_size_dimensions() {
        assert_eq!(PageSize::Letter.dimensions(), (612.0, 792.0));
        assert_eq!(PageSize::A4.dimensions(), (595.0, 842.0));
        assert_eq!(PageSize::Legal.dimensions(), (612.0, 1008.0));
        assert_eq!(PageSize::Custom(100.0, 200.0).dimensions(), (100.0, 200.0));
    }

    #[test]
    fn test_document_metadata() {
        let meta = DocumentMetadata::new()
            .title("Test Title")
            .author("Test Author")
            .subject("Test Subject");

        assert_eq!(meta.title, Some("Test Title".to_string()));
        assert_eq!(meta.author, Some("Test Author".to_string()));
        assert_eq!(meta.subject, Some("Test Subject".to_string()));
    }

    #[test]
    fn test_wrap_text() {
        let text = "This is a test of the word wrapping function";
        let wrapped = wrap_text(text, 20);
        assert!(wrapped.len() > 1);
        for line in &wrapped {
            assert!(line.len() <= 20 || line.split_whitespace().count() == 1);
        }
    }

    #[test]
    fn test_wrap_text_empty() {
        let wrapped = wrap_text("", 20);
        assert_eq!(wrapped.len(), 1);
        assert_eq!(wrapped[0], "");
    }

    #[test]
    fn test_document_builder_basic() {
        let mut builder = DocumentBuilder::new();
        builder
            .letter_page()
            .at(72.0, 720.0)
            .text("Hello, World!")
            .done();

        let bytes = builder.build().unwrap();
        let content = String::from_utf8_lossy(&bytes);

        assert!(content.starts_with("%PDF-1.7"));
        assert!(content.contains("%%EOF"));
    }

    #[test]
    fn test_document_builder_with_metadata() {
        let mut builder = DocumentBuilder::new().metadata(
            DocumentMetadata::new()
                .title("Test Document")
                .author("Test Author"),
        );

        builder.letter_page().text("Content").done();

        let bytes = builder.build().unwrap();
        let content = String::from_utf8_lossy(&bytes);

        assert!(content.contains("/Title (Test Document)"));
        assert!(content.contains("/Author (Test Author)"));
    }

    #[test]
    fn test_document_builder_multiple_pages() {
        let mut builder = DocumentBuilder::new();
        builder.letter_page().text("Page 1").done();
        builder.a4_page().text("Page 2").done();

        let bytes = builder.build().unwrap();
        let content = String::from_utf8_lossy(&bytes);

        assert!(content.contains("/Count 2"));
    }

    #[test]
    fn test_fluent_page_builder() {
        let mut builder = DocumentBuilder::new();
        builder
            .letter_page()
            .at(72.0, 720.0)
            .font("Helvetica-Bold", 18.0)
            .text("Title")
            .font("Helvetica", 12.0)
            .text("Body text")
            .space(12.0)
            .text("More text")
            .done();

        let bytes = builder.build().unwrap();
        assert!(!bytes.is_empty());
    }

    #[test]
    fn test_fluent_page_builder_measure() {
        // `measure` is a pure query: no cursor advance, no content emission.
        // Width must scale with font size and character count.
        let mut doc = DocumentBuilder::new();
        let page = doc.letter_page().at(72.0, 720.0);

        let short = page.measure("AB");
        let long = page.measure("ABCDEFGH");
        assert!(long > short, "longer string must measure wider");
        assert!(short > 0.0, "non-empty string must have positive width");

        // Empty string is zero width.
        assert_eq!(page.measure(""), 0.0);

        // Switching font size scales the measure.
        let small_page = page.font("Helvetica", 10.0);
        let small = small_page.measure("ABC");
        let big = small_page.font("Helvetica", 20.0).measure("ABC");
        assert!(
            (big - 2.0 * small).abs() < 0.5,
            "doubling font size should ~double measured width: {} vs 2*{}",
            big,
            small
        );
    }

    #[test]
    fn test_table_fluent_emits_elements_and_advances_cursor() {
        use super::super::table_renderer::{
            CellAlign, ColumnWidth, Table as RenderTable, TableCell,
        };

        let mut doc = DocumentBuilder::new();
        let page = doc.letter_page().font("Helvetica", 12.0);
        let cursor_before = page.cursor_y;

        let table = RenderTable::new(vec![
            vec![
                TableCell::text("Name"),
                TableCell::text("Value").align(CellAlign::Right),
            ],
            vec![TableCell::text("Alice"), TableCell::text("42")],
            vec![TableCell::text("Bob"), TableCell::text("7")],
        ])
        .with_header_row()
        .with_column_widths(vec![ColumnWidth::Fixed(200.0), ColumnWidth::Fixed(200.0)]);

        let page = page.table(table);
        let cursor_after = page.cursor_y;
        page.done();

        // Cursor advanced downward by at least one row's worth of height.
        assert!(
            cursor_after < cursor_before,
            "cursor must move down after .table(): before={} after={}",
            cursor_before,
            cursor_after
        );

        // At least one Text element per non-empty cell — 6 cells here.
        let texts: Vec<_> = doc.pages[0]
            .elements
            .iter()
            .filter_map(|e| match e {
                ContentElement::Text(t) => Some(t),
                _ => None,
            })
            .collect();
        assert_eq!(
            texts.len(),
            6,
            "expected one Text per cell; got {}: {:?}",
            texts.len(),
            texts.iter().map(|t| &t.text).collect::<Vec<_>>()
        );

        // Header row (first 2 Text elements) must use Helvetica-Bold because
        // TableCell::header and is_header promote the font name.
        assert_eq!(
            texts[0].font.name, "Helvetica-Bold",
            "header cell must use bold font"
        );
        assert_eq!(texts[1].font.name, "Helvetica-Bold");
        // Body rows stay on the default Helvetica.
        assert_eq!(texts[2].font.name, "Helvetica");
    }

    #[test]
    fn test_table_fluent_reading_order_is_page_relative() {
        // If there's already stuff on the page before .table(), the table's
        // reading_order must start after the existing elements — not from 0.
        use super::super::table_renderer::{ColumnWidth, Table as RenderTable, TableCell};

        let mut doc = DocumentBuilder::new();
        doc.letter_page()
            .at(72.0, 720.0)
            .font("Helvetica", 12.0)
            .text("Before the table") // becomes reading_order=0
            .table(
                RenderTable::new(vec![vec![TableCell::text("a"), TableCell::text("b")]])
                    .with_column_widths(vec![
                        ColumnWidth::Fixed(100.0),
                        ColumnWidth::Fixed(100.0),
                    ]),
            )
            .text("After the table")
            .done();

        let orders: Vec<usize> = doc.pages[0]
            .elements
            .iter()
            .filter_map(|e| e.reading_order())
            .collect();

        // Orders must be monotone and start from 0.
        for pair in orders.windows(2) {
            assert!(
                pair[1] > pair[0],
                "reading_order must be strictly monotone: {:?}",
                orders
            );
        }
    }

    #[test]
    fn test_shape_primitives_emit_path_elements() {
        // Every shape primitive must push exactly one ContentElement::Path
        // with stroke and/or fill honoured.
        let mut doc = DocumentBuilder::new();
        doc.letter_page()
            .circle(100.0, 100.0, 20.0, Some(LineStyle::new(1.5, 0.1, 0.2, 0.3)), None)
            .ellipse(200.0, 100.0, 30.0, 15.0, None, Some((0.9, 0.1, 0.1)))
            .polygon(
                &[(300.0, 100.0), (320.0, 120.0), (340.0, 100.0), (320.0, 80.0)],
                Some(LineStyle::default()),
                Some((0.5, 0.5, 0.9)),
            )
            .arc(400.0, 100.0, 25.0, 0.0, std::f32::consts::PI, LineStyle::default())
            .bezier_curve(
                500.0, 100.0, 510.0, 140.0, 540.0, 140.0, 550.0, 100.0,
                LineStyle::default(),
                None,
            )
            .done();

        let paths: Vec<_> = doc.pages[0]
            .elements
            .iter()
            .filter_map(|e| match e {
                ContentElement::Path(p) => Some(p),
                _ => None,
            })
            .collect();
        // Five primitives, five Path elements.
        assert_eq!(paths.len(), 5);

        // Circle: stroke set, no fill.
        assert!(paths[0].stroke_color.is_some() && paths[0].fill_color.is_none());
        assert!((paths[0].stroke_width - 1.5).abs() < 1e-6);

        // Ellipse: fill set, no stroke.
        assert!(paths[1].fill_color.is_some() && paths[1].stroke_color.is_none());

        // Polygon: both stroke (default 1pt black) and fill.
        assert!(paths[2].stroke_color.is_some() && paths[2].fill_color.is_some());

        // Arc: stroke only.
        assert!(paths[3].stroke_color.is_some() && paths[3].fill_color.is_none());

        // Bezier: stroke only (fill None).
        assert!(paths[4].stroke_color.is_some() && paths[4].fill_color.is_none());
    }

    #[test]
    fn test_polygon_requires_two_points() {
        // Fewer than 2 points must be a no-op, not a panic.
        let mut doc = DocumentBuilder::new();
        doc.letter_page()
            .polygon(&[], Some(LineStyle::default()), None)
            .polygon(&[(100.0, 100.0)], Some(LineStyle::default()), None)
            .done();
        // No paths emitted from either degenerate call.
        let paths_n = doc.pages[0]
            .elements
            .iter()
            .filter(|e| matches!(e, ContentElement::Path(_)))
            .count();
        assert_eq!(paths_n, 0);
    }

    #[test]
    fn test_remaining_space_matches_cursor_vs_bottom_margin() {
        // Letter page is 612 × 792. Default cursor_y at page start = 792 - 72 = 720.
        // Bottom margin convention = 72. So initial remaining = 720 - 72 = 648.
        let mut doc = DocumentBuilder::new();
        let page = doc.letter_page();
        assert!((page.remaining_space() - 648.0).abs() < 0.01, "initial: {}", page.remaining_space());

        // After .text(), cursor drops by size * line_height = 12 * 1.2 = 14.4.
        // Remaining should drop by the same.
        let page = page.font("Helvetica", 12.0).text("row 1");
        let expected = 648.0 - 12.0 * 1.2;
        assert!(
            (page.remaining_space() - expected).abs() < 0.01,
            "after one text line: {} vs expected {}",
            page.remaining_space(),
            expected
        );

        // Moving cursor below the bottom margin clamps remaining_space to 0.0.
        let page = page.at(72.0, 10.0);
        assert_eq!(page.remaining_space(), 0.0);
    }

    #[test]
    fn test_new_page_same_size_preserves_dimensions_and_config() {
        let mut doc = DocumentBuilder::new();
        doc.page(PageSize::A3)
            .font("Times-Roman", 14.0)
            .text("page 1")
            .new_page_same_size()
            .text("page 2") // uses carried font/size
            .done();

        assert_eq!(doc.pages.len(), 2);
        let (w0, h0) = (doc.pages[0].width, doc.pages[0].height);
        let (w1, h1) = (doc.pages[1].width, doc.pages[1].height);
        assert_eq!(w0, w1, "width preserved");
        assert_eq!(h0, h1, "height preserved");

        // The second page's "page 2" text element must be in Times-Roman
        // 14pt, proving text_config carried over.
        let texts_p2: Vec<_> = doc.pages[1]
            .elements
            .iter()
            .filter_map(|e| match e {
                ContentElement::Text(t) => Some(t),
                _ => None,
            })
            .collect();
        assert_eq!(texts_p2.len(), 1);
        assert_eq!(texts_p2[0].font.name, "Times-Roman");
        assert_eq!(texts_p2[0].font.size, 14.0);
    }

    #[test]
    fn test_new_page_same_size_resets_cursor_to_top() {
        // After a bunch of .text() calls the cursor is well down the first
        // page. A fresh new_page_same_size must start at the top-left
        // again (cursor_y = height - 72 for a fresh page).
        let mut doc = DocumentBuilder::new();
        let page = doc
            .letter_page()
            .font("Helvetica", 12.0)
            .text("l1")
            .text("l2")
            .text("l3");

        let first_remaining = page.remaining_space();
        let new_page = page.new_page_same_size();
        assert!(
            new_page.remaining_space() > first_remaining,
            "new page must have more headroom: new={} vs old={}",
            new_page.remaining_space(),
            first_remaining
        );
        assert!(
            (new_page.remaining_space() - 648.0).abs() < 0.01,
            "fresh letter page expected 648pt remaining, got {}",
            new_page.remaining_space()
        );
    }

    #[test]
    fn test_stroke_rect_emits_path_with_style() {
        // stroke_rect must push a Path element with the supplied width and
        // colour, fill unset, so downstream PDF emission does `S` (stroke
        // only) not `B` (stroke + fill).
        let mut doc = DocumentBuilder::new();
        doc.letter_page()
            .stroke_rect(50.0, 50.0, 200.0, 100.0, LineStyle::new(2.5, 0.8, 0.2, 0.1))
            .done();

        let page = &doc.pages[0];
        let paths: Vec<_> = page
            .elements
            .iter()
            .filter_map(|e| match e {
                ContentElement::Path(p) => Some(p),
                _ => None,
            })
            .collect();
        assert_eq!(paths.len(), 1);
        let p = paths[0];
        assert_eq!(p.stroke_width, 2.5);
        let c = p.stroke_color.expect("stroke color must be set");
        assert!((c.r - 0.8).abs() < 1e-6 && (c.g - 0.2).abs() < 1e-6 && (c.b - 0.1).abs() < 1e-6);
        assert!(p.fill_color.is_none(), "stroke_rect must not fill");
    }

    #[test]
    fn test_stroke_line_emits_path_with_style() {
        let mut doc = DocumentBuilder::new();
        doc.letter_page()
            .stroke_line(10.0, 100.0, 500.0, 100.0, LineStyle::new(0.5, 0.5, 0.5, 0.5))
            .done();

        let page = &doc.pages[0];
        let paths: Vec<_> = page
            .elements
            .iter()
            .filter_map(|e| match e {
                ContentElement::Path(p) => Some(p),
                _ => None,
            })
            .collect();
        assert_eq!(paths.len(), 1);
        let p = paths[0];
        assert_eq!(p.stroke_width, 0.5);
        let c = p.stroke_color.expect("stroke color must be set");
        assert!(
            (c.r - 0.5).abs() < 1e-6 && (c.g - 0.5).abs() < 1e-6 && (c.b - 0.5).abs() < 1e-6
        );
        assert!(p.fill_color.is_none());
    }

    #[test]
    fn test_line_style_default() {
        let s = LineStyle::default();
        assert_eq!(s.width, 1.0);
        assert_eq!(s.color, (0.0, 0.0, 0.0));
    }

    #[test]
    fn test_text_in_rect_wraps_and_aligns() {
        // Feed long-enough text into a narrow rect and assert N emitted
        // TextContent elements with correct per-line placement for each
        // alignment mode.
        for (align, anchor) in [
            (TextAlign::Left, "left"),
            (TextAlign::Center, "center"),
            (TextAlign::Right, "right"),
        ] {
            let mut doc = DocumentBuilder::new();
            let rect = Rect::new(100.0, 600.0, 60.0, 200.0);
            doc.letter_page()
                .font("Helvetica", 10.0)
                .text_in_rect(rect, "alpha beta gamma delta epsilon", align)
                .done();

            let page = &doc.pages[0];
            let elements: Vec<_> = page
                .elements
                .iter()
                .filter_map(|e| match e {
                    ContentElement::Text(t) => Some(t),
                    _ => None,
                })
                .collect();

            assert!(
                elements.len() >= 2,
                "{} align: expected wrap to >= 2 lines, got {}",
                anchor,
                elements.len()
            );

            // Every emitted line must fit inside the rect horizontally.
            for (idx, tc) in elements.iter().enumerate() {
                assert!(
                    tc.bbox.x >= rect.x - 0.01,
                    "{} line {} starts outside left edge: x={} rect.x={}",
                    anchor,
                    idx,
                    tc.bbox.x,
                    rect.x
                );
                assert!(
                    tc.bbox.x + tc.bbox.width <= rect.x + rect.width + 0.01,
                    "{} line {} extends past right edge: end={} rect_end={}",
                    anchor,
                    idx,
                    tc.bbox.x + tc.bbox.width,
                    rect.x + rect.width
                );
            }

            // Per-alignment placement: check the first line specifically.
            let first = elements[0];
            match align {
                TextAlign::Left => {
                    assert!(
                        (first.bbox.x - rect.x).abs() < 0.01,
                        "left align: line x must equal rect.x, got {} vs {}",
                        first.bbox.x,
                        rect.x
                    );
                },
                TextAlign::Center => {
                    let expected = rect.x + (rect.width - first.bbox.width) / 2.0;
                    assert!(
                        (first.bbox.x - expected).abs() < 0.01,
                        "center align: expected x={}, got {}",
                        expected,
                        first.bbox.x
                    );
                },
                TextAlign::Right => {
                    let expected = rect.x + rect.width - first.bbox.width;
                    assert!(
                        (first.bbox.x - expected).abs() < 0.01,
                        "right align: expected x={}, got {}",
                        expected,
                        first.bbox.x
                    );
                },
            }

            // Lines are stacked top-down: y decreases monotonically.
            for pair in elements.windows(2) {
                assert!(
                    pair[1].bbox.y < pair[0].bbox.y,
                    "{} lines must move down (y decreases): {} then {}",
                    anchor,
                    pair[0].bbox.y,
                    pair[1].bbox.y
                );
            }
        }
    }

    #[test]
    fn test_text_in_rect_does_not_advance_cursor() {
        // Callers track the cursor themselves for text_in_rect; unlike
        // `.text()` and `.paragraph()`, this primitive must leave cursor_y
        // untouched so tables can advance their own geometry.
        let mut doc = DocumentBuilder::new();
        let rect = Rect::new(100.0, 600.0, 80.0, 100.0);
        doc.letter_page()
            .at(200.0, 750.0)
            .font("Helvetica", 12.0)
            .text_in_rect(rect, "test", TextAlign::Left)
            .text("after") // this should land at the untouched cursor
            .done();

        let page = &doc.pages[0];
        let texts: Vec<_> = page
            .elements
            .iter()
            .filter_map(|e| match e {
                ContentElement::Text(t) => Some(t),
                _ => None,
            })
            .collect();

        // Two text elements: the rect'd one and the "after" one.
        assert_eq!(texts.len(), 2);
        // The "after" text sits at y=750 (untouched cursor), not at some
        // y derived from rect.y - line_height.
        let after = texts.iter().find(|t| t.text == "after").unwrap();
        assert!(
            (after.bbox.y - 750.0).abs() < 0.01,
            "cursor must be untouched by text_in_rect; got y={}",
            after.bbox.y
        );
    }

    #[test]
    fn test_text_config() {
        let config = TextConfig {
            font: "Times-Roman".to_string(),
            size: 14.0,
            align: TextAlign::Center,
            line_height: 1.5,
        };

        assert_eq!(config.font, "Times-Roman");
        assert_eq!(config.size, 14.0);
    }

    // ==========================================================================
    // Annotation Tests
    // ==========================================================================

    #[test]
    fn test_link_url_annotation() {
        let mut builder = DocumentBuilder::new();
        builder
            .letter_page()
            .at(72.0, 720.0)
            .text("Click here")
            .link_url("https://example.com")
            .done();

        let bytes = builder.build().unwrap();
        let content = String::from_utf8_lossy(&bytes);

        assert!(content.contains("/Subtype /Link"));
        assert!(content.contains("/S /URI"));
        assert!(content.contains("example.com"));
    }

    #[test]
    fn test_link_page_annotation() {
        let mut builder = DocumentBuilder::new();
        builder.letter_page().text("Page 1").done();
        builder
            .letter_page()
            .at(72.0, 720.0)
            .text("Go to page 1")
            .link_page(0)
            .done();

        let bytes = builder.build().unwrap();
        let content = String::from_utf8_lossy(&bytes);

        assert!(content.contains("/Subtype /Link"));
        assert!(content.contains("/Dest"));
    }

    #[test]
    fn test_highlight_annotation() {
        let mut builder = DocumentBuilder::new();
        builder
            .letter_page()
            .at(72.0, 720.0)
            .text("Important text")
            .highlight((1.0, 1.0, 0.0))
            .done();

        let bytes = builder.build().unwrap();
        let content = String::from_utf8_lossy(&bytes);

        assert!(content.contains("/Subtype /Highlight"));
        assert!(content.contains("/QuadPoints"));
    }

    #[test]
    fn test_underline_annotation() {
        let mut builder = DocumentBuilder::new();
        builder
            .letter_page()
            .at(72.0, 720.0)
            .text("Underlined text")
            .underline((1.0, 0.0, 0.0))
            .done();

        let bytes = builder.build().unwrap();
        let content = String::from_utf8_lossy(&bytes);

        assert!(content.contains("/Subtype /Underline"));
    }

    #[test]
    fn test_strikeout_annotation() {
        let mut builder = DocumentBuilder::new();
        builder
            .letter_page()
            .at(72.0, 720.0)
            .text("Deleted text")
            .strikeout((1.0, 0.0, 0.0))
            .done();

        let bytes = builder.build().unwrap();
        let content = String::from_utf8_lossy(&bytes);

        assert!(content.contains("/Subtype /StrikeOut"));
    }

    #[test]
    fn test_sticky_note_annotation() {
        let mut builder = DocumentBuilder::new();
        builder
            .letter_page()
            .at(72.0, 720.0)
            .sticky_note("This is a comment")
            .done();

        let bytes = builder.build().unwrap();
        let content = String::from_utf8_lossy(&bytes);

        assert!(content.contains("/Subtype /Text"));
        assert!(content.contains("This is a comment"));
    }

    #[test]
    fn test_stamp_annotation() {
        let mut builder = DocumentBuilder::new();
        builder
            .letter_page()
            .at(72.0, 720.0)
            .stamp(StampType::Approved)
            .done();

        let bytes = builder.build().unwrap();
        let content = String::from_utf8_lossy(&bytes);

        assert!(content.contains("/Subtype /Stamp"));
        assert!(content.contains("/Name /Approved"));
    }

    #[test]
    fn test_freetext_annotation() {
        let mut builder = DocumentBuilder::new();
        builder
            .letter_page()
            .freetext(Rect::new(100.0, 500.0, 200.0, 50.0), "Free text content")
            .done();

        let bytes = builder.build().unwrap();
        let content = String::from_utf8_lossy(&bytes);

        assert!(content.contains("/Subtype /FreeText"));
        assert!(content.contains("Free text content"));
    }

    #[test]
    fn test_watermark_annotation() {
        let mut builder = DocumentBuilder::new();
        builder.letter_page().watermark("DRAFT").done();

        let bytes = builder.build().unwrap();
        let content = String::from_utf8_lossy(&bytes);

        assert!(content.contains("/Subtype /Watermark"));
    }

    #[test]
    fn test_watermark_presets() {
        let mut builder = DocumentBuilder::new();
        builder.letter_page().watermark_confidential().done();

        let bytes = builder.build().unwrap();
        let content = String::from_utf8_lossy(&bytes);

        assert!(content.contains("/Subtype /Watermark"));
    }

    #[test]
    fn test_multiple_annotations() {
        let mut builder = DocumentBuilder::new();
        builder
            .letter_page()
            .at(72.0, 720.0)
            .text("Linked and highlighted text")
            .link_url("https://example.com")
            .highlight((1.0, 1.0, 0.0))
            .sticky_note("Review this")
            .done();

        let bytes = builder.build().unwrap();
        let content = String::from_utf8_lossy(&bytes);

        // Should have all three annotation types
        assert!(content.contains("/Subtype /Link"));
        assert!(content.contains("/Subtype /Highlight"));
        assert!(content.contains("/Subtype /Text"));
    }

    #[test]
    fn test_add_generic_annotation() {
        let mut builder = DocumentBuilder::new();
        let link =
            LinkAnnotation::uri(Rect::new(100.0, 700.0, 100.0, 20.0), "https://rust-lang.org");
        builder.letter_page().add_annotation(link).done();

        let bytes = builder.build().unwrap();
        let content = String::from_utf8_lossy(&bytes);

        assert!(content.contains("/Subtype /Link"));
        assert!(content.contains("rust-lang.org"));
    }

    #[test]
    fn test_no_annotation_when_no_text() {
        let mut builder = DocumentBuilder::new();
        // Try to add link without any text - should be a no-op
        builder
            .letter_page()
            .at(72.0, 720.0)
            .link_url("https://example.com") // No preceding text
            .done();

        let bytes = builder.build().unwrap();
        let content = String::from_utf8_lossy(&bytes);

        // Should NOT contain a link annotation since there was no text to link
        assert!(!content.contains("/Subtype /Link"));
    }
}
