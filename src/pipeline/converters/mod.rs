//! Output converters for the text extraction pipeline.
//!
//! This module provides the OutputConverter trait and implementations for
//! converting ordered text spans to various output formats.
//!
//! # Available Converters
//!
//! - [`MarkdownOutputConverter`]: Convert to Markdown format
//! - [`HtmlOutputConverter`]: Convert to HTML format
//! - [`PlainTextConverter`]: Convert to plain text
//!
//! # Example
//!
//! ```ignore
//! use pdf_oxide::pipeline::converters::{OutputConverter, MarkdownOutputConverter};
//! use pdf_oxide::pipeline::TextPipelineConfig;
//!
//! let converter = MarkdownOutputConverter::new();
//! let config = TextPipelineConfig::default();
//! let output = converter.convert(&ordered_spans, &config)?;
//! ```

mod html;
mod markdown;
mod plain_text;
pub mod toc_detector;

pub use html::HtmlOutputConverter;
pub use markdown::MarkdownOutputConverter;
pub use plain_text::PlainTextConverter;
pub use toc_detector::{TocDetector, TocEntry};

use crate::error::Result;
use crate::layout::TextSpan;
use crate::pipeline::{OrderedTextSpan, TextPipelineConfig};
use crate::structure::table_extractor::ExtractedTable;

/// Trait for converting ordered text spans to output formats.
///
/// Implementations transform a sequence of ordered text spans into a specific
/// output format (Markdown, HTML, plain text, etc.).
///
/// This trait provides a clean abstraction layer between the PDF extraction
/// pipeline and the output generation, following the PDF spec compliance goal
/// of separating PDF representation from output formatting.
pub trait OutputConverter: Send + Sync {
    /// Convert ordered spans to the target format.
    ///
    /// # Arguments
    ///
    /// * `spans` - Ordered text spans from the reading order strategy
    /// * `config` - Pipeline configuration affecting output formatting
    ///
    /// # Returns
    ///
    /// The formatted output string.
    fn convert(&self, spans: &[OrderedTextSpan], config: &TextPipelineConfig) -> Result<String>;

    /// Convert ordered spans to the target format, with pre-detected tables.
    ///
    /// Table regions are rendered using the converter's table formatting
    /// (markdown tables, HTML tables, or tab-delimited text). Spans that
    /// fall within table bounding boxes are excluded from normal rendering.
    ///
    /// Default implementation ignores tables and falls back to `convert()`.
    fn convert_with_tables(
        &self,
        spans: &[OrderedTextSpan],
        tables: &[ExtractedTable],
        config: &TextPipelineConfig,
    ) -> Result<String> {
        let _ = tables;
        self.convert(spans, config)
    }

    /// Return the name of this converter for debugging.
    fn name(&self) -> &'static str;

    /// Return the MIME type for the output format.
    fn mime_type(&self) -> &'static str;
}

/// Check whether two horizontally adjacent spans have a visible gap between them.
///
/// Returns `true` when the horizontal distance between the end of `prev` and
/// the start of `current` exceeds a small fraction of the font size but is not
/// unreasonably large (which would indicate a column break rather than a word
/// gap).
pub(crate) fn has_horizontal_gap(prev: &TextSpan, current: &TextSpan) -> bool {
    let font_size = prev.font_size.max(current.font_size).max(1.0);
    let prev_end_x = prev.bbox.x + prev.bbox.width;
    let gap = current.bbox.x - prev_end_x;
    let threshold = font_size * 0.15;
    gap > threshold && gap < font_size * 5.0
}

/// Return the index of the table whose bounding box contains the span's origin,
/// or `None` if the span does not fall inside any table region.
pub(crate) fn span_in_table(span: &OrderedTextSpan, tables: &[ExtractedTable]) -> Option<usize> {
    let sx = span.span.bbox.x;
    let sy = span.span.bbox.y;

    for (i, table) in tables.iter().enumerate() {
        if let Some(ref bbox) = table.bbox {
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

/// Post-process rendered text to merge key-value pairs that were split across
/// lines due to column-based reading order.
///
/// Detects the pattern where a text label (e.g. "Grand Total") appears on one
/// line and its corresponding value (e.g. "$750.00") appears alone on the next
/// line.  When detected, the two lines are merged into one with a separating
/// space (e.g. "Grand Total $750.00").
///
/// A line is considered a "value" if it is short (< 30 chars), starts with a
/// digit, currency symbol, or parenthesized number, and does not look like a
/// sentence continuation.  A line is considered a "label" if it ends with
/// alphabetic text (no trailing punctuation that would indicate a complete
/// sentence).
pub(crate) fn merge_key_value_pairs(text: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();
    if lines.len() < 2 {
        return text.to_string();
    }

    // Determine which lines are "value-only" lines that should merge upward.
    // A value line is short and starts with a digit, $, (, -, or similar
    // numeric indicator.
    fn is_value_line(line: &str) -> bool {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.len() > 30 {
            return false;
        }
        let first = trimmed.chars().next().unwrap();
        // Starts with digit, currency sign, open-paren (for negative numbers),
        // minus/dash (for negative), or period (for .50 style decimals)
        matches!(first, '0'..='9' | '$' | '€' | '£' | '¥' | '(' | '-' | '.')
    }

    // A label line: non-empty, ends with a word character (letter or digit),
    // does not end with sentence-terminal punctuation.  We also reject lines
    // that are themselves value-only (to avoid merging two values).
    fn is_label_line(line: &str) -> bool {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return false;
        }
        // Must not itself be a value-only line
        if is_value_line(line) {
            return false;
        }
        // Last non-whitespace character should be alphanumeric or ')' or ':'
        // (not sentence-ending like '.', '!', '?')
        let last = trimmed.chars().next_back().unwrap();
        last.is_alphanumeric() || last == ')' || last == ':'
    }

    let mut result = String::with_capacity(text.len());
    let mut i = 0;
    while i < lines.len() {
        // Pattern 1: label immediately followed by value (no blank line)
        if i + 1 < lines.len() && is_label_line(lines[i]) && is_value_line(lines[i + 1]) {
            result.push_str(lines[i].trim_end());
            result.push(' ');
            result.push_str(lines[i + 1].trim_start());
            result.push('\n');
            i += 2;
        }
        // Pattern 2: label, blank line, value (paragraph break between them)
        else if i + 2 < lines.len()
            && is_label_line(lines[i])
            && lines[i + 1].trim().is_empty()
            && is_value_line(lines[i + 2])
        {
            result.push_str(lines[i].trim_end());
            result.push(' ');
            result.push_str(lines[i + 2].trim_start());
            result.push('\n');
            i += 3;
        } else {
            result.push_str(lines[i]);
            result.push('\n');
            i += 1;
        }
    }

    // Restore the exact trailing-newline count of the original input.
    // `text.lines()` strips all trailing empty lines, so we count them here
    // and re-append them after processing.
    let orig_trailing_newlines = text.chars().rev().take_while(|&c| c == '\n').count();
    // Strip any trailing newlines we added, then re-append the original count.
    while result.ends_with('\n') {
        result.pop();
    }
    for _ in 0..orig_trailing_newlines {
        result.push('\n');
    }

    result
}

/// Create a converter based on the output format name.
pub fn create_converter(format: &str) -> Option<Box<dyn OutputConverter>> {
    match format.to_lowercase().as_str() {
        "markdown" | "md" => Some(Box::new(MarkdownOutputConverter::new())),
        "html" => Some(Box::new(HtmlOutputConverter::new())),
        "text" | "plain" | "txt" => Some(Box::new(PlainTextConverter::new())),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_converter_markdown() {
        let converter = create_converter("markdown").unwrap();
        assert_eq!(converter.name(), "MarkdownOutputConverter");
        assert_eq!(converter.mime_type(), "text/markdown");
    }

    #[test]
    fn test_create_converter_html() {
        let converter = create_converter("html").unwrap();
        assert_eq!(converter.name(), "HtmlOutputConverter");
        assert_eq!(converter.mime_type(), "text/html");
    }

    #[test]
    fn test_create_converter_text() {
        let converter = create_converter("text").unwrap();
        assert_eq!(converter.name(), "PlainTextConverter");
        assert_eq!(converter.mime_type(), "text/plain");
    }

    #[test]
    fn test_create_converter_unknown() {
        assert!(create_converter("unknown").is_none());
    }

    // ========================================================================
    // Key-value pair merging tests
    // ========================================================================

    #[test]
    fn test_key_value_pair_merging_basic() {
        let input = "Grand Total\n$750.00\nNet Amount\n$250.00\n";
        let expected = "Grand Total $750.00\nNet Amount $250.00\n";
        assert_eq!(merge_key_value_pairs(input), expected);
    }

    #[test]
    fn test_key_value_pair_merging_no_false_positive_on_sentences() {
        // Lines ending with period should not be treated as labels.
        let input = "This is a sentence.\n$100.00\n";
        assert_eq!(merge_key_value_pairs(input), input);
    }

    #[test]
    fn test_key_value_pair_merging_negative_numbers() {
        let input = "Balance Due\n-$42.50\n";
        let expected = "Balance Due -$42.50\n";
        assert_eq!(merge_key_value_pairs(input), expected);
    }

    #[test]
    fn test_key_value_pair_merging_plain_numbers() {
        let input = "Account Number\n434508032\n";
        let expected = "Account Number 434508032\n";
        assert_eq!(merge_key_value_pairs(input), expected);
    }

    #[test]
    fn test_key_value_pair_merging_skips_long_values() {
        // A long "value" line should not be merged (it is probably a paragraph).
        let input = "Introduction\nThis is a full paragraph of text that continues.\n";
        assert_eq!(merge_key_value_pairs(input), input);
    }

    #[test]
    fn test_key_value_pair_merging_preserves_blank_lines() {
        let input = "Section A\n\nTotal\n$100\n";
        let expected = "Section A\n\nTotal $100\n";
        assert_eq!(merge_key_value_pairs(input), expected);
    }

    #[test]
    fn test_key_value_pair_merging_consecutive_pairs() {
        let input = "Subtotal\n$200.00\nTax\n$18.00\nTotal\n$218.00\n";
        let expected = "Subtotal $200.00\nTax $18.00\nTotal $218.00\n";
        assert_eq!(merge_key_value_pairs(input), expected);
    }

    #[test]
    fn test_key_value_pair_merging_euro_and_pound() {
        let input = "Price\n€49.99\nShipping\n£5.00\n";
        let expected = "Price €49.99\nShipping £5.00\n";
        assert_eq!(merge_key_value_pairs(input), expected);
    }

    #[test]
    fn test_key_value_pair_merging_parenthesized_negative() {
        let input = "Net Loss\n(1,234.56)\n";
        let expected = "Net Loss (1,234.56)\n";
        assert_eq!(merge_key_value_pairs(input), expected);
    }

    #[test]
    fn test_key_value_pair_merging_no_merge_value_value() {
        // Two consecutive value-only lines should not merge.
        let input = "$100\n$200\n";
        assert_eq!(merge_key_value_pairs(input), input);
    }

    #[test]
    fn test_key_value_pair_merging_empty_input() {
        assert_eq!(merge_key_value_pairs(""), "");
        assert_eq!(merge_key_value_pairs("single line\n"), "single line\n");
    }
}
