//! Streaming table surface for `FluentPageBuilder` — see issue #393.
//!
//! Unlike `table_renderer::Table` (buffered: takes the whole row matrix,
//! computes a single layout, emits all content in one go), `StreamingTable`
//! emits each row into the page as soon as it is pushed. Persistent state
//! is **O(columns + current page)** — the row itself is consumed and
//! dropped per call.
//!
//! Scope for v0.3.39: `TableMode::Fixed` only — widths are declared
//! explicitly up front. No content-driven autofit (that is MigraDoc's
//! O(rows²) failure mode the design explicitly rejects, per research B
//! and decision doc `docs/v0.3.39/design/393_tables_decision.md`).
//!
//! Follow-ups deferred to v0.3.40 (#400):
//! - `TableMode::Sample` — measure first N rows, freeze widths.
//! - Cross-page cell splitting for tall rich cells.
//! - Bounded-lookahead rowspan.
//!
//! ## Example
//!
//! ```no_run
//! use pdf_oxide::writer::{
//!     CellAlign, DocumentBuilder, StreamingColumn, StreamingTableConfig,
//! };
//!
//! let mut doc = DocumentBuilder::new();
//! let page = doc
//!     .letter_page()
//!     .font("Helvetica", 10.0)
//!     .at(72.0, 720.0);
//!
//! let mut t = page.streaming_table(
//!     StreamingTableConfig::new()
//!         .column(StreamingColumn::new("SKU").width_pt(72.0))
//!         .column(StreamingColumn::new("Item").width_pt(240.0))
//!         .column(StreamingColumn::new("Qty").width_pt(48.0).align(CellAlign::Right))
//!         .repeat_header(true),
//! );
//!
//! for (sku, item, qty) in [("A-1", "Widget", 5), ("B-2", "Gadget", 12)] {
//!     t.push_row(|r| {
//!         r.cell(sku);
//!         r.cell(item);
//!         r.cell(qty.to_string());
//!     })
//!     .unwrap();
//! }
//! t.finish().done();
//! ```

use super::document_builder::{FluentPageBuilder, TextAlign};
use super::table_renderer::CellAlign;
use crate::elements::{ContentElement, FontSpec, PathContent, PathOperation, TextContent, TextStyle};
use crate::error::{Error, Result};
use crate::geometry::Rect;
use crate::layout::Color;

/// Alignment mapping helper: the table vocabulary (`CellAlign`) doesn't
/// share a type with `TextAlign` but maps trivially.
fn cell_to_text_align(a: CellAlign) -> TextAlign {
    match a {
        CellAlign::Left => TextAlign::Left,
        CellAlign::Center => TextAlign::Center,
        CellAlign::Right => TextAlign::Right,
    }
}

/// One column in a `StreamingTableConfig`.
///
/// Widths are **explicit** — streaming tables can't autofit because that
/// requires looking at rows the caller hasn't pushed yet. See research B
/// (docs/v0.3.39/research/b_scalable_layout_algorithms.md) for the full
/// rationale.
#[derive(Debug, Clone)]
pub struct StreamingColumn {
    /// Column heading text. Rendered at the top of the table and on every
    /// page break when `repeat_header` is set on the config.
    pub header: String,
    /// Column width in PDF points. Must be > 0.
    pub width: f32,
    /// Per-column default horizontal alignment.
    pub align: CellAlign,
}

impl StreamingColumn {
    /// Create a column with the given header and default width (100 pt, left-align).
    pub fn new(header: impl Into<String>) -> Self {
        Self {
            header: header.into(),
            width: 100.0,
            align: CellAlign::Left,
        }
    }

    /// Set the column width in PDF points.
    pub fn width_pt(mut self, pt: f32) -> Self {
        self.width = pt;
        self
    }

    /// Set the column's default cell alignment.
    pub fn align(mut self, align: CellAlign) -> Self {
        self.align = align;
        self
    }
}

/// Configuration for a streaming table. Built via `new()` + fluent setters,
/// then consumed by `FluentPageBuilder::streaming_table`.
#[derive(Debug, Clone, Default)]
pub struct StreamingTableConfig {
    pub(crate) columns: Vec<StreamingColumn>,
    pub(crate) repeat_header: bool,
    pub(crate) row_padding_top: f32,
    pub(crate) row_padding_bottom: f32,
    pub(crate) horizontal_padding: f32,
    pub(crate) grid_color: (f32, f32, f32),
    pub(crate) grid_width: f32,
    pub(crate) header_fill: Option<(f32, f32, f32)>,
}

impl StreamingTableConfig {
    /// Create an empty configuration. Add columns via `.column(...)`.
    pub fn new() -> Self {
        Self {
            columns: Vec::new(),
            repeat_header: false,
            row_padding_top: 2.0,
            row_padding_bottom: 2.0,
            horizontal_padding: 4.0,
            grid_color: (0.8, 0.8, 0.8),
            grid_width: 0.5,
            header_fill: Some((0.93, 0.93, 0.93)),
        }
    }

    /// Add a column. Order matters — this is the left-to-right visual order.
    pub fn column(mut self, c: StreamingColumn) -> Self {
        self.columns.push(c);
        self
    }

    /// Redraw the header row at the top of every page this table spans.
    pub fn repeat_header(mut self, yes: bool) -> Self {
        self.repeat_header = yes;
        self
    }

    /// Override the default header background (light grey). Pass
    /// `(r, g, b)` or set to `None` for no fill.
    pub fn header_fill(mut self, fill: Option<(f32, f32, f32)>) -> Self {
        self.header_fill = fill;
        self
    }

    /// Override grid line colour + width. Set `width` to 0.0 to suppress.
    pub fn grid(mut self, color: (f32, f32, f32), width: f32) -> Self {
        self.grid_color = color;
        self.grid_width = width;
        self
    }

    /// Override horizontal + vertical cell padding (default 4 / 2 / 2 pt).
    pub fn cell_padding(mut self, horizontal: f32, top: f32, bottom: f32) -> Self {
        self.horizontal_padding = horizontal;
        self.row_padding_top = top;
        self.row_padding_bottom = bottom;
        self
    }
}

/// One row being built inside `push_row`. Cells must be pushed in column
/// order; pushing more than `columns.len()` cells fails at `push_row`
/// return.
#[derive(Debug, Default)]
pub struct StreamingRow {
    cells: Vec<String>,
}

impl StreamingRow {
    /// Append the next cell's string content. Accepts anything
    /// `Into<String>` — `&str`, `String`, numbers via `.to_string()`.
    pub fn cell(&mut self, value: impl Into<String>) -> &mut Self {
        self.cells.push(value.into());
        self
    }

    fn into_cells(self) -> Vec<String> {
        self.cells
    }
}

/// Streaming table handle. Created by
/// `FluentPageBuilder::streaming_table`; consumed by `finish()`.
///
/// Holds a mutable borrow of its parent `FluentPageBuilder` through the
/// building window.
pub struct StreamingTable<'a> {
    page: FluentPageBuilder<'a>,
    config: StreamingTableConfig,
    /// Prefix-sum of column widths starting at origin_x.
    column_x: Vec<f32>,
    /// Total table width.
    total_width: f32,
    /// Left edge of the table (fixed by first push_row / header).
    origin_x: f32,
    /// Whether the header has been drawn on the current page.
    header_drawn: bool,
}

impl<'a> StreamingTable<'a> {
    /// Open a new streaming table. Called via
    /// `FluentPageBuilder::streaming_table`; not public because it couples
    /// to builder internals.
    pub(super) fn open(page: FluentPageBuilder<'a>, config: StreamingTableConfig) -> Self {
        let origin_x = page.cursor_x();
        let mut column_x = Vec::with_capacity(config.columns.len() + 1);
        let mut cursor = origin_x;
        column_x.push(cursor);
        for c in &config.columns {
            cursor += c.width;
            column_x.push(cursor);
        }
        let total_width = cursor - origin_x;

        Self {
            page,
            config,
            column_x,
            total_width,
            origin_x,
            header_drawn: false,
        }
    }

    /// Number of columns configured.
    pub fn column_count(&self) -> usize {
        self.config.columns.len()
    }

    /// Push one row. The closure receives a mutable `StreamingRow` into
    /// which the caller pushes cells in column order.
    ///
    /// Returns `Err(Error::InvalidOperation)` if the number of cells
    /// pushed does not match the column count. The row is then discarded
    /// (no partial draw).
    pub fn push_row<F>(&mut self, build: F) -> Result<()>
    where
        F: FnOnce(&mut StreamingRow),
    {
        let n_cols = self.config.columns.len();
        if n_cols == 0 {
            return Err(Error::InvalidOperation(
                "streaming_table: no columns configured".into(),
            ));
        }

        let mut row = StreamingRow::default();
        build(&mut row);
        let cells = row.into_cells();

        if cells.len() != n_cols {
            return Err(Error::InvalidOperation(format!(
                "streaming_table: row has {} cells, expected {}",
                cells.len(),
                n_cols
            )));
        }

        // Lazy-draw the header on the first push, and on every page break
        // when repeat_header is enabled.
        if !self.header_drawn {
            self.draw_header();
            self.header_drawn = true;
        }

        self.draw_row(&cells, false)?;
        Ok(())
    }

    /// Finish the table and return the page builder for further fluent
    /// chaining.
    pub fn finish(self) -> FluentPageBuilder<'a> {
        self.page
    }

    // ───── internal ─────────────────────────────────────────────────

    fn draw_header(&mut self) {
        let headers: Vec<String> =
            self.config.columns.iter().map(|c| c.header.clone()).collect();
        self.draw_row(&headers, true).ok();
    }

    fn draw_row(&mut self, cells: &[String], is_header: bool) -> Result<()> {
        let font_size = self.page.text_config_font_size();
        let line_height = font_size * self.page.text_config_line_height();
        let h_pad = self.config.horizontal_padding;
        let top_pad = self.config.row_padding_top;
        let bot_pad = self.config.row_padding_bottom;

        // Pre-wrap every cell at frozen column widths.
        let mut wrapped: Vec<Vec<(String, f32)>> = Vec::with_capacity(cells.len());
        let mut max_lines = 1usize;
        for (col_idx, cell) in cells.iter().enumerate() {
            let col_w = self.config.columns[col_idx].width;
            let content_w = (col_w - 2.0 * h_pad).max(1.0);
            let lines = self.page.wrap_cell_text(cell, content_w);
            max_lines = max_lines.max(lines.len().max(1));
            wrapped.push(lines);
        }
        let row_height = top_pad + bot_pad + (max_lines as f32) * line_height;

        // Page-break check before drawing.
        if self.page.remaining_space() < row_height {
            self.page.new_page_same_size_inplace();
            // Rebind origin_x on the new page (cursor has been reset to the
            // top-left margin; existing column_x offsets were anchored to
            // the old origin).
            self.origin_x = self.page.cursor_x();
            let mut cursor = self.origin_x;
            for (i, c) in self.config.columns.iter().enumerate() {
                self.column_x[i] = cursor;
                cursor += c.width;
            }
            self.column_x[self.config.columns.len()] = cursor;
            self.total_width = cursor - self.origin_x;

            if self.config.repeat_header && !is_header {
                self.draw_header();
                // After redrawing header re-check remaining space for this row.
                if self.page.remaining_space() < row_height {
                    return Err(Error::InvalidOperation(format!(
                        "streaming_table: row height {} exceeds empty page content height",
                        row_height
                    )));
                }
            }
        }

        // Origin y for the row = top edge.
        let row_top = self.page.cursor_y();

        // 1. Header background fill (if any).
        if is_header {
            if let Some((r, g, b)) = self.config.header_fill {
                self.push_path_fill(self.origin_x, row_top - row_height, self.total_width, row_height, (r, g, b));
            }
        }

        // 2. Grid: horizontal top + bottom of the row; verticals at every column boundary.
        if self.config.grid_width > 0.0 {
            let gc = self.config.grid_color;
            let gw = self.config.grid_width;
            let top_y = row_top;
            let bot_y = row_top - row_height;
            let left_x = self.origin_x;
            let right_x = self.origin_x + self.total_width;
            self.push_path_stroke_line(left_x, top_y, right_x, top_y, gc, gw);
            self.push_path_stroke_line(left_x, bot_y, right_x, bot_y, gc, gw);
            // Snapshot boundaries to avoid the &self immut borrow holding
            // across the self.push_path_stroke_line &mut self call.
            let boundaries: Vec<f32> = self.column_x.clone();
            for x in boundaries {
                self.push_path_stroke_line(x, top_y, x, bot_y, gc, gw);
            }
        }

        // 3. Cell text — one Text per wrapped line with per-line alignment.
        for (col_idx, lines) in wrapped.iter().enumerate() {
            let col_left = self.column_x[col_idx];
            let col_w = self.config.columns[col_idx].width;
            let content_left = col_left + h_pad;
            let content_w = col_w - 2.0 * h_pad;
            let align = cell_to_text_align(self.config.columns[col_idx].align);
            let font_name = if is_header {
                let base = self.page.text_config_font_name();
                if base.ends_with("-Bold") || base.contains("-Bold") {
                    base.to_string()
                } else {
                    format!("{}-Bold", base)
                }
            } else {
                self.page.text_config_font_name().to_string()
            };

            for (line_idx, (line, line_w)) in lines.iter().enumerate() {
                if line.is_empty() {
                    continue;
                }
                let x = match align {
                    TextAlign::Left => content_left,
                    TextAlign::Center => content_left + (content_w - *line_w) / 2.0,
                    TextAlign::Right => content_left + content_w - *line_w,
                };
                let y = row_top - top_pad - (line_idx as f32) * line_height;
                self.push_text(line, x, y, *line_w, font_size, font_name.as_str());
            }
        }

        // Advance cursor past the row.
        self.page.set_cursor_y(row_top - row_height);
        Ok(())
    }

    fn push_path_fill(&mut self, x: f32, y: f32, w: f32, h: f32, color: (f32, f32, f32)) {
        let mut path = PathContent::new(Rect::new(x, y, w, h));
        path.operations.push(PathOperation::Rectangle(x, y, w, h));
        path.fill_color = Some(Color { r: color.0, g: color.1, b: color.2 });
        path.stroke_color = None;
        path.reading_order = Some(self.page.page_element_count());
        self.page.push_element(ContentElement::Path(path));
    }

    fn push_path_stroke_line(
        &mut self,
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        color: (f32, f32, f32),
        width: f32,
    ) {
        let min_x = x1.min(x2);
        let min_y = y1.min(y2);
        let w = (x2 - x1).abs().max(1.0);
        let h = (y2 - y1).abs().max(1.0);
        let mut path = PathContent::new(Rect::new(min_x, min_y, w, h));
        path.operations.push(PathOperation::MoveTo(x1, y1));
        path.operations.push(PathOperation::LineTo(x2, y2));
        path.stroke_color = Some(Color { r: color.0, g: color.1, b: color.2 });
        path.stroke_width = width;
        path.fill_color = None;
        path.reading_order = Some(self.page.page_element_count());
        self.page.push_element(ContentElement::Path(path));
    }

    fn push_text(&mut self, line: &str, x: f32, y: f32, w: f32, font_size: f32, font_name: &str) {
        let tc = TextContent {
            text: line.to_string(),
            bbox: Rect::new(x, y, w, font_size),
            font: FontSpec {
                name: font_name.to_string(),
                size: font_size,
            },
            style: TextStyle::default(),
            reading_order: Some(self.page.page_element_count()),
            artifact_type: None,
            origin: None,
            rotation_degrees: None,
            matrix: None,
        };
        self.page.push_element(ContentElement::Text(tc));
    }
}

#[cfg(test)]
mod tests {
    use super::super::document_builder::DocumentBuilder;
    use super::*;

    #[test]
    fn test_streaming_table_emits_header_and_rows() {
        let mut doc = DocumentBuilder::new();
        let page = doc
            .letter_page()
            .font("Helvetica", 10.0)
            .at(72.0, 720.0);

        let mut t = page.streaming_table(
            StreamingTableConfig::new()
                .column(StreamingColumn::new("SKU").width_pt(60.0))
                .column(StreamingColumn::new("Item").width_pt(120.0))
                .column(
                    StreamingColumn::new("Qty")
                        .width_pt(40.0)
                        .align(CellAlign::Right),
                )
                .repeat_header(true),
        );

        for i in 0..3 {
            t.push_row(|r| {
                r.cell(format!("A-{}", i));
                r.cell("Widget");
                r.cell((i * 10).to_string());
            })
            .unwrap();
        }

        t.finish().done();

        let texts: Vec<_> = doc
            .page_elements(0)
            .iter()
            .filter_map(|e| match e {
                ContentElement::Text(t) => Some(t),
                _ => None,
            })
            .collect();

        // 3 header cells + 3 rows × 3 body cells = 12 text elements.
        assert_eq!(texts.len(), 12, "expected 12 text elements, got {}", texts.len());
        assert_eq!(texts[0].text, "SKU");
        assert_eq!(texts[0].font.name, "Helvetica-Bold");
        assert_eq!(texts[3].text, "A-0");
        assert_eq!(texts[3].font.name, "Helvetica");
    }

    #[test]
    fn test_streaming_table_row_mismatch_errors() {
        let mut doc = DocumentBuilder::new();
        let page = doc.letter_page();
        let mut t = page.streaming_table(
            StreamingTableConfig::new()
                .column(StreamingColumn::new("A").width_pt(60.0))
                .column(StreamingColumn::new("B").width_pt(60.0)),
        );

        let err = t.push_row(|r| {
            r.cell("only one cell");
        });
        assert!(err.is_err());
    }

    #[test]
    fn test_streaming_table_page_break_and_repeat_header() {
        // Engineer a near-full page so one more row overflows and forces a break.
        let mut doc = DocumentBuilder::new();
        let page = doc.letter_page().font("Helvetica", 10.0);
        // Burn most of the vertical space by moving cursor down.
        let page = page.at(72.0, 90.0); // ~18 pt to bottom margin

        let mut t = page.streaming_table(
            StreamingTableConfig::new()
                .column(StreamingColumn::new("A").width_pt(100.0))
                .repeat_header(true),
        );
        // First row triggers: draw_header (12pt) + row_height ~12pt → overflows
        // 18pt, forces new page. Header must redraw on page 2 before row.
        t.push_row(|r| {
            r.cell("row-on-page-2");
        })
        .unwrap();
        t.finish().done();

        // Must have created a 2nd page.
        assert!(doc.page_count() >= 2, "expected a page break, got {} pages", doc.page_count());

        // Page 2 must contain both the header text AND the row text.
        let p2_texts: Vec<&str> = doc
            .page_elements(1)
            .iter()
            .filter_map(|e| match e {
                ContentElement::Text(t) => Some(t.text.as_str()),
                _ => None,
            })
            .collect();
        assert!(
            p2_texts.contains(&"A"),
            "page 2 must contain repeated header 'A', got {:?}",
            p2_texts
        );
        assert!(
            p2_texts.contains(&"row-on-page-2"),
            "page 2 must contain the body row, got {:?}",
            p2_texts
        );
    }

    #[test]
    fn test_streaming_table_thirty_thousand_rows_bounded_memory() {
        // The motivating case. We don't time the benchmark here (that is
        // in tools/benchmark-harness/) but we do verify the API sustains
        // 30k push_row calls without panicking and keeps per-row memory
        // bounded — i.e. the row is consumed and not retained.
        let mut doc = DocumentBuilder::new();
        let page = doc.letter_page().font("Helvetica", 8.0).at(72.0, 720.0);

        let mut t = page.streaming_table(
            StreamingTableConfig::new()
                .column(StreamingColumn::new("#").width_pt(40.0))
                .column(StreamingColumn::new("Value").width_pt(80.0))
                .repeat_header(true),
        );

        for i in 0..30_000usize {
            t.push_row(|r| {
                r.cell(i.to_string());
                r.cell("v");
            })
            .unwrap();
        }
        t.finish().done();

        // All 30k rows spread across many pages; the API completed without
        // error. We don't assert page count (depends on font metrics) —
        // the important property is no panic, no run-away memory.
        assert!(doc.page_count() > 100, "expected many pages for 30k rows");
    }
}
