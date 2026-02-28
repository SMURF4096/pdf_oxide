//! Markdown output converter.
//!
//! Converts ordered text spans to Markdown format.

use crate::error::Result;
use crate::layout::FontWeight;
use crate::pipeline::{OrderedTextSpan, TextPipelineConfig};
use crate::structure::table_extractor::ExtractedTable;
use crate::text::HyphenationHandler;
use lazy_static::lazy_static;
use regex::Regex;

use super::OutputConverter;

lazy_static! {
    /// Regex for matching URLs in text
    static ref RE_URL: Regex = Regex::new(r"(https?://[^\s<>\[\]]*[^\s<>\[\].,!?;:])").unwrap();

    /// Regex for matching email addresses
    static ref RE_EMAIL: Regex = Regex::new(r"([a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,})").unwrap();
}

/// Markdown output converter.
///
/// Converts ordered text spans to Markdown format with optional formatting:
/// - Bold text using `**text**` markers
/// - Italic text using `*text*` markers
/// - Heading detection based on font size (when enabled)
/// - Paragraph separation based on vertical gaps
/// - Table detection and formatting
/// - Layout preservation with whitespace
/// - URL/Email linkification
/// - Whitespace normalization
pub struct MarkdownOutputConverter {
    /// Line spacing threshold ratio for paragraph detection.
    paragraph_gap_ratio: f32,
}

impl MarkdownOutputConverter {
    /// Create a new Markdown converter with default settings.
    pub fn new() -> Self {
        Self {
            paragraph_gap_ratio: 1.5,
        }
    }

    /// Create a Markdown converter with custom paragraph gap ratio.
    pub fn with_paragraph_gap(ratio: f32) -> Self {
        Self {
            paragraph_gap_ratio: ratio,
        }
    }

    /// Check if a span should be rendered as bold.
    fn is_bold(&self, span: &OrderedTextSpan, config: &TextPipelineConfig) -> bool {
        use crate::pipeline::config::BoldMarkerBehavior;

        match span.span.font_weight {
            FontWeight::Bold | FontWeight::Black | FontWeight::ExtraBold | FontWeight::SemiBold => {
                match config.output.bold_marker_behavior {
                    BoldMarkerBehavior::Aggressive => true,
                    BoldMarkerBehavior::Conservative => {
                        // Only apply bold to content-bearing text
                        span.span.text.chars().any(|c| !c.is_whitespace())
                    },
                }
            },
            _ => false,
        }
    }

    /// Check if a span should be rendered as italic.
    fn is_italic(&self, span: &OrderedTextSpan) -> bool {
        span.span.is_italic && span.span.text.chars().any(|c| !c.is_whitespace())
    }

    /// Format text with bold and/or italic markers.
    fn apply_formatting(&self, text: &str, is_bold: bool, is_italic: bool) -> String {
        if is_bold && is_italic {
            format!("***{}***", text)
        } else if is_bold {
            format!("**{}**", text)
        } else if is_italic {
            format!("*{}*", text)
        } else {
            text.to_string()
        }
    }

    /// Apply linkification to text (URLs and emails).
    fn linkify(&self, text: &str) -> String {
        // Quick pre-check: skip regex for spans that can't contain URLs or emails.
        // This avoids regex overhead for ~95% of regular text spans.
        let might_have_url = text.contains("://") || text.contains("www.");
        let might_have_email = text.contains('@');

        if !might_have_url && !might_have_email {
            return text.to_string();
        }

        let mut result = if might_have_url {
            RE_URL
                .replace_all(text, |caps: &regex::Captures| {
                    let url = &caps[0];
                    format!("[{}]({})", url, url)
                })
                .to_string()
        } else {
            text.to_string()
        };

        if might_have_email {
            result = RE_EMAIL
                .replace_all(&result, |caps: &regex::Captures| {
                    let email = &caps[0];
                    format!("[{}](mailto:{})", email, email)
                })
                .to_string();
        }

        result
    }

    /// Normalize whitespace in text.
    fn normalize_whitespace(&self, text: &str) -> String {
        // Replace multiple spaces with single space
        text.split_whitespace().collect::<Vec<_>>().join(" ")
    }

    /// Detect paragraph breaks between spans based on vertical spacing.
    fn is_paragraph_break(&self, current: &OrderedTextSpan, previous: &OrderedTextSpan) -> bool {
        let line_height = current.span.font_size.max(previous.span.font_size);
        let gap = (previous.span.bbox.y - current.span.bbox.y).abs();
        gap > line_height * self.paragraph_gap_ratio
    }

    /// Detect if span should be a heading based on font size.
    ///
    /// Uses absolute font sizes (only for clear heading cases):
    /// - H1: 24pt and above
    /// - H2: 18-23pt
    /// - H3: 14-17pt
    ///
    /// Note: Falls back to ratio-based detection for more nuanced cases.
    fn heading_level_absolute(&self, span: &OrderedTextSpan) -> Option<u8> {
        let size = span.span.font_size;
        if size >= 24.0 {
            Some(1)
        } else if size >= 18.0 {
            Some(2)
        } else if size >= 14.0 {
            Some(3)
        } else {
            None
        }
    }

    /// Detect heading level based on font size ratio to base size.
    fn heading_level_ratio(&self, span: &OrderedTextSpan, base_font_size: f32) -> Option<u8> {
        let size_ratio = span.span.font_size / base_font_size;
        if size_ratio >= 2.0 {
            Some(1)
        } else if size_ratio >= 1.5 {
            Some(2)
        } else if size_ratio >= 1.25 {
            Some(3)
        } else {
            None
        }
    }

    /// Check if a span's bbox overlaps with any table region.
    fn span_in_table(&self, span: &OrderedTextSpan, tables: &[ExtractedTable]) -> Option<usize> {
        let sx = span.span.bbox.x;
        let sy = span.span.bbox.y;

        for (i, table) in tables.iter().enumerate() {
            if let Some(ref bbox) = table.bbox {
                // Use generous tolerance for bbox overlap
                let tolerance = 2.0;
                if sx >= bbox.x - tolerance
                    && sx <= bbox.x + bbox.width + tolerance
                    && sy >= bbox.y - tolerance
                    && sy <= bbox.y + bbox.height + tolerance
                {
                    return Some(i);
                }
            }
        }
        None
    }

    /// Render an ExtractedTable as a markdown table string.
    fn render_table_markdown(table: &ExtractedTable) -> String {
        if table.rows.is_empty() {
            return String::new();
        }

        let mut output = String::new();

        // Determine header row index - use first row if has_header, or first is_header row
        let header_end = if table.has_header {
            table
                .rows
                .iter()
                .position(|r| !r.is_header)
                .unwrap_or(1)
        } else {
            // Treat first row as header for markdown (markdown requires a header row)
            1
        };

        for (row_idx, row) in table.rows.iter().enumerate() {
            output.push('|');
            for cell in &row.cells {
                output.push(' ');
                // Escape pipe characters in cell text
                let text = cell.text.replace('|', "\\|");
                let text = text.replace('\n', " ");
                output.push_str(text.trim());
                output.push(' ');
                // Handle colspan by adding extra | separators
                for _ in 1..cell.colspan {
                    output.push_str("| ");
                }
                output.push('|');
            }
            output.push('\n');

            // Add header separator after header rows
            if row_idx + 1 == header_end {
                output.push('|');
                for cell in &row.cells {
                    for _ in 0..cell.colspan {
                        output.push_str("---|");
                    }
                }
                output.push('\n');
            }
        }

        output
    }

    /// Core rendering logic shared between convert() and convert_with_tables().
    fn render_spans(
        &self,
        spans: &[OrderedTextSpan],
        tables: &[ExtractedTable],
        config: &TextPipelineConfig,
    ) -> Result<String> {
        if spans.is_empty() && tables.is_empty() {
            return Ok(String::new());
        }

        // Sort by reading order
        let mut sorted: Vec<_> = spans.iter().collect();
        sorted.sort_by_key(|s| s.reading_order);

        // Calculate base font size for heading detection
        let base_font_size = if config.output.detect_headings {
            let sizes: Vec<f32> = sorted.iter().map(|s| s.span.font_size).collect();
            let mut sizes_sorted = sizes.clone();
            sizes_sorted.sort_by(|a, b| crate::utils::safe_float_cmp(*a, *b));
            sizes_sorted
                .get(sizes_sorted.len() / 2)
                .copied()
                .unwrap_or(12.0)
        } else {
            12.0
        };

        // Track which tables have been rendered
        let mut tables_rendered = vec![false; tables.len()];

        let mut result = String::new();
        let mut prev_span: Option<&OrderedTextSpan> = None;
        let mut current_line = String::new();

        for span in sorted.iter() {
            // Check if this span belongs to a table region
            if !tables.is_empty() {
                if let Some(table_idx) = self.span_in_table(span, tables) {
                    if !tables_rendered[table_idx] {
                        // Flush current line
                        if !current_line.is_empty() {
                            result.push_str(current_line.trim());
                            result.push_str("\n\n");
                            current_line.clear();
                        }

                        // Render the table
                        let table_md = Self::render_table_markdown(&tables[table_idx]);
                        result.push_str(&table_md);
                        result.push('\n');
                        tables_rendered[table_idx] = true;
                        prev_span = None;
                    }
                    // Skip this span (it's part of a table)
                    continue;
                }
            }

            // Check for paragraph break
            if let Some(prev) = prev_span {
                if self.is_paragraph_break(span, prev) {
                    if !current_line.is_empty() {
                        result.push_str(current_line.trim());
                        result.push_str("\n\n");
                        current_line.clear();
                    }
                } else {
                    let same_line =
                        (span.span.bbox.y - prev.span.bbox.y).abs() < span.span.font_size * 0.5;
                    if !same_line {
                        if config.output.preserve_layout {
                            let spacing = (span.span.bbox.x - prev.span.bbox.x).max(0.0) as usize;
                            for _ in 0..spacing.min(20) {
                                current_line.push(' ');
                            }
                        } else {
                            current_line.push(' ');
                        }
                    }
                }
            }

            // Check for heading
            if config.output.detect_headings {
                let level = self
                    .heading_level_absolute(span)
                    .or_else(|| self.heading_level_ratio(span, base_font_size));

                if let Some(level) = level {
                    if !current_line.is_empty() {
                        result.push_str(current_line.trim());
                        result.push_str("\n\n");
                        current_line.clear();
                    }

                    let prefix = "#".repeat(level as usize);
                    result.push_str(&format!("{} {}\n\n", prefix, span.span.text.trim()));
                    prev_span = None;
                    continue;
                }
            }

            // Format text with bold/italic and apply linkification
            let mut text = span.span.text.as_str();

            let normalized;
            if !config.output.preserve_layout {
                normalized = self.normalize_whitespace(text);
                text = &normalized;
            }

            let linkified = self.linkify(text);

            let is_bold = self.is_bold(span, config);
            let is_italic = self.is_italic(span);
            let formatted = self.apply_formatting(&linkified, is_bold, is_italic);

            current_line.push_str(&formatted);

            prev_span = Some(span);
        }

        // Render any tables that weren't matched to spans (e.g., all spans were in tables)
        for (i, table) in tables.iter().enumerate() {
            if !tables_rendered[i] && !table.is_empty() {
                if !current_line.is_empty() {
                    result.push_str(current_line.trim());
                    result.push_str("\n\n");
                    current_line.clear();
                }
                let table_md = Self::render_table_markdown(table);
                result.push_str(&table_md);
                result.push('\n');
            }
        }

        // Flush remaining content
        if !current_line.is_empty() {
            result.push_str(current_line.trim());
            result.push('\n');
        }

        // Final whitespace normalization
        let mut final_result = if config.output.preserve_layout {
            result
        } else {
            let cleaned = result
                .split("\n\n")
                .map(|para| para.trim())
                .filter(|para| !para.is_empty())
                .collect::<Vec<_>>()
                .join("\n\n");

            if result.ends_with('\n') && !cleaned.ends_with('\n') {
                format!("{}\n", cleaned)
            } else {
                cleaned
            }
        };

        // Apply hyphenation reconstruction if enabled
        if config.enable_hyphenation_reconstruction {
            let handler = HyphenationHandler::new();
            final_result = handler.process_text(&final_result);
        }

        Ok(final_result)
    }
}

impl Default for MarkdownOutputConverter {
    fn default() -> Self {
        Self::new()
    }
}

impl OutputConverter for MarkdownOutputConverter {
    fn convert(&self, spans: &[OrderedTextSpan], config: &TextPipelineConfig) -> Result<String> {
        self.render_spans(spans, &[], config)
    }

    fn convert_with_tables(
        &self,
        spans: &[OrderedTextSpan],
        tables: &[ExtractedTable],
        config: &TextPipelineConfig,
    ) -> Result<String> {
        self.render_spans(spans, tables, config)
    }

    fn name(&self) -> &'static str {
        "MarkdownOutputConverter"
    }

    fn mime_type(&self) -> &'static str {
        "text/markdown"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Rect;
    use crate::layout::{Color, TextSpan};
    use crate::structure::table_extractor::{TableCell, TableRow};

    fn make_span(
        text: &str,
        x: f32,
        y: f32,
        font_size: f32,
        weight: FontWeight,
    ) -> OrderedTextSpan {
        OrderedTextSpan::new(
            TextSpan {
                text: text.to_string(),
                bbox: Rect::new(x, y, 50.0, font_size),
                font_name: "Test".to_string(),
                font_size,
                font_weight: weight,
                is_italic: false,
                color: Color::black(),
                mcid: None,
                sequence: 0,
                offset_semantic: false,
                split_boundary_before: false,
                char_spacing: 0.0,
                word_spacing: 0.0,
                horizontal_scaling: 100.0,
                primary_detected: false,
            },
            0,
        )
    }

    #[test]
    fn test_empty_spans() {
        let converter = MarkdownOutputConverter::new();
        let config = TextPipelineConfig::default();
        let result = converter.convert(&[], &config).unwrap();
        assert_eq!(result, "");
    }

    #[test]
    fn test_single_span() {
        let converter = MarkdownOutputConverter::new();
        let config = TextPipelineConfig::default();
        let spans = vec![make_span(
            "Hello world",
            0.0,
            100.0,
            12.0,
            FontWeight::Normal,
        )];
        let result = converter.convert(&spans, &config).unwrap();
        assert_eq!(result, "Hello world\n");
    }

    #[test]
    fn test_bold_text() {
        let converter = MarkdownOutputConverter::new();
        let config = TextPipelineConfig::default();
        let spans = vec![make_span("Bold text", 0.0, 100.0, 12.0, FontWeight::Bold)];
        let result = converter.convert(&spans, &config).unwrap();
        assert_eq!(result, "**Bold text**\n");
    }

    #[test]
    fn test_whitespace_bold_conservative() {
        let converter = MarkdownOutputConverter::new();
        let config = TextPipelineConfig::default();
        // Whitespace-only bold should not have markers in conservative mode
        let spans = vec![make_span("   ", 0.0, 100.0, 12.0, FontWeight::Bold)];
        let result = converter.convert(&spans, &config).unwrap();
        // Should not contain bold markers
        assert!(!result.contains("**"));
    }

    #[test]
    fn test_convert_with_tables_renders_markdown_table() {
        let converter = MarkdownOutputConverter::new();
        let config = TextPipelineConfig::default();

        let mut table = ExtractedTable::new();
        table.bbox = Some(Rect::new(10.0, 50.0, 200.0, 100.0));
        table.col_count = 2;
        table.has_header = true;

        let mut header = TableRow::new(true);
        header.add_cell(TableCell::new("Name".to_string(), true));
        header.add_cell(TableCell::new("Value".to_string(), true));
        table.add_row(header);

        let mut data = TableRow::new(false);
        data.add_cell(TableCell::new("A".to_string(), false));
        data.add_cell(TableCell::new("1".to_string(), false));
        table.add_row(data);

        let result = converter
            .convert_with_tables(&[], &[table], &config)
            .unwrap();

        assert!(result.contains("| Name |"));
        assert!(result.contains("| Value |"));
        assert!(result.contains("---|"));
        assert!(result.contains("| A |"));
        assert!(result.contains("| 1 |"));
    }

    // ============================================================================
    // render_table_markdown() tests
    // ============================================================================

    #[test]
    fn test_render_table_markdown_empty() {
        let table = ExtractedTable::new();
        let result = MarkdownOutputConverter::render_table_markdown(&table);
        assert_eq!(result, "");
    }

    #[test]
    fn test_render_table_markdown_single_row_no_header() {
        let mut table = ExtractedTable::new();
        let mut row = TableRow::new(false);
        row.add_cell(TableCell::new("A".to_string(), false));
        row.add_cell(TableCell::new("B".to_string(), false));
        table.add_row(row);

        let result = MarkdownOutputConverter::render_table_markdown(&table);
        assert!(result.contains("| A |"));
        assert!(result.contains("| B |"));
        // First row treated as header by default in markdown
        assert!(result.contains("---|"));
    }

    #[test]
    fn test_render_table_markdown_with_colspan() {
        let mut table = ExtractedTable::new();
        table.has_header = true;
        let mut header = TableRow::new(true);
        header.add_cell(TableCell::new("Wide".to_string(), true).with_colspan(2));
        table.add_row(header);

        let mut data = TableRow::new(false);
        data.add_cell(TableCell::new("Left".to_string(), false));
        data.add_cell(TableCell::new("Right".to_string(), false));
        table.add_row(data);

        let result = MarkdownOutputConverter::render_table_markdown(&table);
        // Colspan cell should produce extra | separators
        assert!(result.contains("| Wide |"));
        assert!(result.contains("---|---|"));
    }

    #[test]
    fn test_render_table_markdown_escapes_pipes() {
        let mut table = ExtractedTable::new();
        let mut row = TableRow::new(false);
        row.add_cell(TableCell::new("A|B".to_string(), false));
        table.add_row(row);

        let result = MarkdownOutputConverter::render_table_markdown(&table);
        assert!(result.contains("A\\|B"), "Pipes should be escaped: {}", result);
    }

    #[test]
    fn test_render_table_markdown_replaces_newlines() {
        let mut table = ExtractedTable::new();
        let mut row = TableRow::new(false);
        row.add_cell(TableCell::new("Line1\nLine2".to_string(), false));
        table.add_row(row);

        let result = MarkdownOutputConverter::render_table_markdown(&table);
        assert!(!result.contains("Line1\nLine2"), "Newlines in cells should be replaced");
        assert!(result.contains("Line1 Line2"));
    }

    #[test]
    fn test_render_table_markdown_trims_whitespace() {
        let mut table = ExtractedTable::new();
        let mut row = TableRow::new(false);
        row.add_cell(TableCell::new("  padded  ".to_string(), false));
        table.add_row(row);

        let result = MarkdownOutputConverter::render_table_markdown(&table);
        assert!(result.contains("| padded |"));
    }

    #[test]
    fn test_render_table_markdown_multiple_header_rows() {
        let mut table = ExtractedTable::new();
        table.has_header = true;

        let mut h1 = TableRow::new(true);
        h1.add_cell(TableCell::new("H1".to_string(), true));
        table.add_row(h1);

        let mut h2 = TableRow::new(true);
        h2.add_cell(TableCell::new("H2".to_string(), true));
        table.add_row(h2);

        let mut d1 = TableRow::new(false);
        d1.add_cell(TableCell::new("D1".to_string(), false));
        table.add_row(d1);

        let result = MarkdownOutputConverter::render_table_markdown(&table);
        // Separator should appear after last header row (row_idx == 1)
        let lines: Vec<&str> = result.lines().collect();
        assert_eq!(lines.len(), 4); // H1, H2, separator, D1
        assert!(lines[2].contains("---|"));
    }

    // ============================================================================
    // span_in_table() tests
    // ============================================================================

    #[test]
    fn test_span_in_table_match() {
        let converter = MarkdownOutputConverter::new();
        let span = make_span("text", 50.0, 70.0, 12.0, FontWeight::Normal);

        let mut table = ExtractedTable::new();
        table.bbox = Some(Rect::new(10.0, 50.0, 200.0, 100.0));

        assert_eq!(converter.span_in_table(&span, &[table]), Some(0));
    }

    #[test]
    fn test_span_in_table_no_match() {
        let converter = MarkdownOutputConverter::new();
        let span = make_span("text", 500.0, 500.0, 12.0, FontWeight::Normal);

        let mut table = ExtractedTable::new();
        table.bbox = Some(Rect::new(10.0, 50.0, 200.0, 100.0));

        assert_eq!(converter.span_in_table(&span, &[table]), None);
    }

    #[test]
    fn test_span_in_table_none_bbox() {
        let converter = MarkdownOutputConverter::new();
        let span = make_span("text", 50.0, 70.0, 12.0, FontWeight::Normal);

        let table = ExtractedTable::new(); // No bbox
        assert_eq!(converter.span_in_table(&span, &[table]), None);
    }

    #[test]
    fn test_span_in_table_tolerance() {
        let converter = MarkdownOutputConverter::new();
        // Span at bbox edge minus tolerance (2.0)
        let span = make_span("text", 8.5, 48.5, 12.0, FontWeight::Normal);

        let mut table = ExtractedTable::new();
        table.bbox = Some(Rect::new(10.0, 50.0, 200.0, 100.0));

        assert_eq!(
            converter.span_in_table(&span, &[table]),
            Some(0),
            "Should match within tolerance"
        );
    }

    #[test]
    fn test_span_in_table_multiple_tables() {
        let converter = MarkdownOutputConverter::new();
        let span = make_span("text", 350.0, 70.0, 12.0, FontWeight::Normal);

        let mut t1 = ExtractedTable::new();
        t1.bbox = Some(Rect::new(10.0, 50.0, 200.0, 100.0));

        let mut t2 = ExtractedTable::new();
        t2.bbox = Some(Rect::new(300.0, 50.0, 200.0, 100.0));

        assert_eq!(converter.span_in_table(&span, &[t1, t2]), Some(1));
    }

    // ============================================================================
    // convert_with_tables() integration tests
    // ============================================================================

    #[test]
    fn test_convert_with_tables_mixed_content() {
        let converter = MarkdownOutputConverter::new();
        let config = TextPipelineConfig::default();

        // Text before the table
        let mut span_before = make_span("Before table", 10.0, 200.0, 12.0, FontWeight::Normal);
        span_before.reading_order = 0;

        // Text after the table (lower Y = later in reading order)
        let mut span_after = make_span("After table", 10.0, 20.0, 12.0, FontWeight::Normal);
        span_after.reading_order = 2;

        // Text inside table region (should be excluded)
        let mut span_in_table = make_span("In table", 50.0, 70.0, 12.0, FontWeight::Normal);
        span_in_table.reading_order = 1;

        let mut table = ExtractedTable::new();
        table.bbox = Some(Rect::new(10.0, 50.0, 200.0, 100.0));
        table.has_header = true;
        let mut header = TableRow::new(true);
        header.add_cell(TableCell::new("Col".to_string(), true));
        table.add_row(header);
        let mut data = TableRow::new(false);
        data.add_cell(TableCell::new("Val".to_string(), false));
        table.add_row(data);

        let result = converter
            .convert_with_tables(
                &[span_before, span_in_table, span_after],
                &[table],
                &config,
            )
            .unwrap();

        assert!(result.contains("Before table"), "Should contain text before table");
        assert!(result.contains("| Col |"), "Should contain table");
        assert!(result.contains("After table"), "Should contain text after table");
        assert!(!result.contains("In table"), "Should exclude span inside table region");
    }

    #[test]
    fn test_convert_with_tables_no_tables_is_same_as_convert() {
        let converter = MarkdownOutputConverter::new();
        let config = TextPipelineConfig::default();
        let spans = vec![make_span("Hello", 0.0, 100.0, 12.0, FontWeight::Normal)];

        let result_convert = converter.convert(&spans, &config).unwrap();
        let result_with_tables = converter
            .convert_with_tables(&spans, &[], &config)
            .unwrap();

        assert_eq!(result_convert, result_with_tables);
    }

    #[test]
    fn test_convert_with_tables_multiple_tables() {
        let converter = MarkdownOutputConverter::new();
        let config = TextPipelineConfig::default();

        let make_table = |x: f32, text: &str| -> ExtractedTable {
            let mut t = ExtractedTable::new();
            t.bbox = Some(Rect::new(x, 50.0, 100.0, 50.0));
            let mut row = TableRow::new(false);
            row.add_cell(TableCell::new(text.to_string(), false));
            t.add_row(row);
            t
        };

        let result = converter
            .convert_with_tables(&[], &[make_table(10.0, "T1"), make_table(200.0, "T2")], &config)
            .unwrap();

        assert!(result.contains("| T1 |"), "Should contain first table");
        assert!(result.contains("| T2 |"), "Should contain second table");
    }
}
