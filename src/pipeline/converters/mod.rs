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
use crate::structure::table_extractor::Table;

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
        tables: &[Table],
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

/// Returns `true` if `c` is a CJK character (Chinese, Japanese, or Korean).
fn is_cjk_char(c: char) -> bool {
    matches!(c,
        '\u{3040}'..='\u{309F}' |   // Hiragana
        '\u{30A0}'..='\u{30FF}' |   // Katakana
        '\u{4E00}'..='\u{9FFF}' |   // CJK Unified Ideographs
        '\u{AC00}'..='\u{D7AF}' |   // Hangul
        '\u{3400}'..='\u{4DBF}' |   // CJK Extension A
        '\u{20000}'..='\u{2A6DF}'   // CJK Extension B
    )
}

/// Returns `true` if `c` is a fullwidth or mathematical operator that is
/// commonly embedded inside CJK text without surrounding spaces.
///
/// These characters have slightly wider advances than typical ASCII characters,
/// which can trigger the gap heuristic and insert a spurious space when they
/// appear between CJK glyphs (e.g. `25000≤Q＜40000`).
fn is_fullwidth_or_math_op(c: char) -> bool {
    matches!(c,
        '\u{FF0B}' |                // ＋
        '\u{FF0D}' |                // －
        '\u{FF1A}' |                // ：
        '\u{FF1B}' |                // ；
        '\u{FF1C}'..='\u{FF1E}' |  // ＜ ＝ ＞
        '\u{2260}' |               // ≠
        '\u{2248}' |               // ≈
        '\u{2264}'..='\u{2265}' |  // ≤ ≥
        '\u{00B5}' |               // µ
        '\u{03BC}' |               // μ
        '\u{00B1}' |               // ±
        '\u{00D7}' |               // ×
        '\u{00F7}'                 // ÷
    )
}

/// Clause-boundary delimiters that always warrant a space even next to CJK text.
/// Parentheses, brackets, and quotation marks mark syntactic boundaries and must
/// not have their surrounding space suppressed by the CJK gap-suppression logic.
fn is_clause_delimiter(c: char) -> bool {
    matches!(c,
        '(' | ')' | '[' | ']' | '{' | '}' |
        '\u{FF08}' | '\u{FF09}' | // （ ）fullwidth parens
        '\u{FF3B}' | '\u{FF3D}' | // ［ ］fullwidth brackets
        '\u{300C}' | '\u{300D}' | // 「 」corner brackets
        '\u{300E}' | '\u{300F}' | // 『 』white corner brackets
        '\u{2018}' | '\u{2019}' | // ' ' single smart quotes
        '\u{201C}' | '\u{201D}'   // " " double smart quotes
    )
}

/// Check whether two horizontally adjacent spans have a visible gap between them.
///
/// Returns `true` when the horizontal distance between the end of `prev` and
/// the start of `current` exceeds a small fraction of the font size but is not
/// unreasonably large (which would indicate a column break rather than a word
/// gap).
///
/// CJK scripts do not use spaces between words.  When one side of the boundary
/// is a CJK character and the other side is CJK or a fullwidth/math operator
/// (e.g. `≤`, `＜`, `μ`), no space is inserted even if the geometric gap
/// exceeds the threshold.  This mirrors the CJK-pair suppression in the text
/// extraction path (`document.rs`).
pub(crate) fn has_horizontal_gap(prev: &TextSpan, current: &TextSpan) -> bool {
    let font_size = prev.font_size.max(current.font_size).max(1.0);
    let prev_end_x = prev.bbox.x + prev.bbox.width;
    let gap = current.bbox.x - prev_end_x;
    let threshold = font_size * 0.15;
    if !(gap > threshold && gap < font_size * 5.0) {
        return false;
    }

    // Suppress space insertion when one side is CJK and the other is CJK or a
    // fullwidth/math operator.  This mirrors the CJK-pair suppression in the
    // text extraction path (document.rs:5587-5605).
    let prev_last = prev.text.chars().next_back();
    let curr_first = current.text.chars().next();
    if let (Some(p), Some(c)) = (prev_last, curr_first) {
        let p_cjk = is_cjk_char(p);
        let c_cjk = is_cjk_char(c);
        if (p_cjk || is_fullwidth_or_math_op(p)) && (c_cjk || is_fullwidth_or_math_op(c)) {
            // At least one side must actually be CJK (not two pure math ops).
            if p_cjk || c_cjk {
                return false;
            }
        }
    }

    true
}

/// Return the index of the table whose bounding box contains the span's origin,
/// or `None` if the span does not fall inside any table region.
pub(crate) fn span_in_table(span: &OrderedTextSpan, tables: &[Table]) -> Option<usize> {
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

    // ========================================================================
    // has_horizontal_gap CJK suppression tests (#485)
    // ========================================================================

    /// Build a minimal TextSpan for gap tests.
    ///
    /// `x` is the left edge of the span, `w` is its width, `text` is the
    /// content.  Font size is set to 10 so that the 0.15em threshold = 1.5.
    fn make_span(x: f32, w: f32, text: &str) -> crate::layout::TextSpan {
        crate::layout::TextSpan {
            text: text.to_string(),
            bbox: crate::geometry::Rect::new(x, 0.0, w, 10.0),
            font_size: 10.0,
            ..Default::default()
        }
    }

    #[test]
    fn test_has_horizontal_gap_cjk_cjk_suppressed() {
        // CJK char followed by CJK char with a gap > 0.15em → no space.
        let prev = make_span(0.0, 10.0, "数");   // ends with CJK
        let curr = make_span(12.0, 10.0, "学");  // starts with CJK; gap = 2.0 > 1.5
        assert!(!has_horizontal_gap(&prev, &curr),
            "CJK→CJK should suppress space insertion");
    }

    #[test]
    fn test_has_horizontal_gap_cjk_fullwidth_suppressed() {
        // CJK char followed by fullwidth operator → no space.
        let prev = make_span(0.0, 10.0, "Q");    // ends with ASCII (not CJK alone)
        // override: use a CJK ending character
        let prev_cjk = make_span(0.0, 10.0, "量");
        let curr = make_span(12.0, 10.0, "＜");  // starts with fullwidth '<'; gap = 2.0
        assert!(!has_horizontal_gap(&prev_cjk, &curr),
            "CJK→fullwidth-op should suppress space insertion");
        let _ = prev; // silence unused warning
    }

    #[test]
    fn test_has_horizontal_gap_fullwidth_cjk_suppressed() {
        // Fullwidth operator followed by CJK char → no space.
        let prev = make_span(0.0, 10.0, "≤");   // ends with math op
        let curr = make_span(12.0, 10.0, "Q");  // pure ASCII start — not suppressed
        // For suppression we need curr to start with CJK
        let curr_cjk = make_span(12.0, 10.0, "量");
        assert!(!has_horizontal_gap(&prev, &curr_cjk),
            "fullwidth-op→CJK should suppress space insertion");
        let _ = curr; // silence unused warning
    }

    #[test]
    fn test_has_horizontal_gap_latin_latin_unchanged() {
        // Latin→Latin: gap-based logic unchanged — gap > threshold → true.
        let prev = make_span(0.0, 10.0, "hello");
        let curr = make_span(12.0, 10.0, "world"); // gap = 2.0 > 1.5
        assert!(has_horizontal_gap(&prev, &curr),
            "Latin→Latin with gap > threshold should still insert space");
    }

    #[test]
    fn test_has_horizontal_gap_latin_latin_no_gap() {
        // Latin→Latin: gap ≤ threshold → false (no change from CJK fix).
        let prev = make_span(0.0, 10.0, "hello");
        let curr = make_span(11.0, 10.0, "world"); // gap = 1.0 < 1.5
        assert!(!has_horizontal_gap(&prev, &curr),
            "Latin→Latin below threshold should not insert space");
    }

    #[test]
    fn test_has_horizontal_gap_two_pure_math_ops_unchanged() {
        // Two pure math operators (neither is CJK): gap-based logic unchanged.
        let prev = make_span(0.0, 10.0, "≤");
        let curr = make_span(12.0, 10.0, "≥"); // gap = 2.0 > 1.5; neither is CJK
        assert!(has_horizontal_gap(&prev, &curr),
            "math-op→math-op (no CJK) should still apply gap-based logic");
    }
}
