//! Text block representation for layout analysis.
//!
//! This module defines structures for representing text elements in a PDF document
//! with their geometric and styling information.

use crate::geometry::{Point, Rect};
use crate::extractors::text::ArtifactType;
use std::collections::HashMap;

/// A text span (complete string from a Tj/TJ operator).
///
/// This represents text as the PDF specification provides it - complete strings
/// from text showing operators, not individual characters. This is the correct
/// approach per PDF spec ISO 32000-1:2008.
///
/// Extracting complete strings instead of individual characters:
/// - Avoids overlapping character issues
/// - Preserves PDF's text positioning intent
/// - Matches industry best practices (PyMuPDF, etc.)
/// - More robust for complex layouts
#[derive(Debug, Clone, serde::Serialize)]
pub struct TextSpan {
    /// The complete text string
    pub text: String,
    /// Bounding box of the entire span
    pub bbox: Rect,
    /// Font name/family
    pub font_name: String,
    /// Font size in points
    pub font_size: f32,
    /// Font weight (normal or bold)
    pub font_weight: FontWeight,
    /// Font style: italic or normal
    pub is_italic: bool,
    /// Text color
    pub color: Color,
    /// Marked Content ID (for Tagged PDFs)
    pub mcid: Option<u32>,
    /// Extraction sequence number (used as tie-breaker for Y-coordinate sorting)
    ///
    /// When PDFs use unusual coordinate systems where many spans have identical
    /// Y coordinates (e.g., EU GDPR PDF with Y=0.0 for multiple spans), this
    /// sequence number preserves the original extraction order from the content
    /// stream, which often reflects the intended reading order per PDF spec.
    pub sequence: usize,
    /// If true, this span was created by splitting fused words and should not be re-merged.
    ///
    /// When CamelCase splitting creates separate spans from a single fused word
    /// (e.g., "theGeneral" -> "the" + "General"), this flag prevents them from
    /// being re-merged during the span merging phase, even if the gap is 0pt.
    /// This preserves the split intent and prevents word fusion regressions.
    pub split_boundary_before: bool,
    /// If true, this span was created by the TJ processor as a space from a negative offset.
    ///
    /// Per PDF spec ISO 32000-1:2008 Section 9.4.4, negative offsets in TJ arrays
    /// indicate word boundaries where spaces should be inserted. This flag marks
    /// those automatically generated space spans so merge logic can avoid double-spacing.
    pub offset_semantic: bool,
    /// Character spacing (Tc parameter) per ISO 32000-1:2008 Section 9.3.1.
    ///
    /// Tc is added after each character during text positioning. Default value is 0.
    /// This value is used for text justification detection (Phase 3.5).
    pub char_spacing: f32,
    /// Word spacing (Tw parameter) per ISO 32000-1:2008 Section 9.3.1.
    ///
    /// Tw is added after space characters (U+0020) during text positioning. Default is 0.
    /// This value is critical for text justification detection - the variance in Tw
    /// values across a line indicates the degree of justification applied.
    pub word_spacing: f32,
    /// Horizontal scaling (Tz parameter) per ISO 32000-1:2008 Section 9.3.1.
    ///
    /// Tz scales all character widths and word spacing. Value is in percent (e.g., 100 = 100%).
    /// Default value is 100.0. Used for justification detection and layout analysis.
    pub horizontal_scaling: f32,
    /// If true, was created by WordBoundaryDetector primary detection.
    ///
    /// Used to mark spans created by primary detection mode so they
    /// are not re-merged during the span merging phase.
    /// Default is false for backward compatibility.
    pub primary_detected: bool,
    /// Artifact type classification for filtered content (PDF Spec Section 14.8.2.2)
    pub artifact_type: Option<ArtifactType>,
}

/// A single character with its position and styling.
///
/// NOTE: This is kept for backward compatibility and special use cases.
/// For normal text extraction, prefer TextSpan which represents complete
/// text strings as the PDF provides them.
///
/// ## Transformation Properties (v0.3.1+)
///
/// TextChar now includes transformation information for precise text positioning:
/// - `origin_x`, `origin_y`: Baseline position (where the character sits)
/// - `rotation_degrees`: Text rotation angle
/// - `advance_width`: Horizontal distance to next character
/// - `matrix`: Full 6-element transformation matrix for advanced use cases
///
/// These properties match industry standards (MuPDF, iText, PDFBox, pdfium-render).
#[derive(Debug, Clone, serde::Serialize)]
pub struct TextChar {
    /// The character itself
    pub char: char,
    /// Bounding box of the character
    pub bbox: Rect,
    /// Font name/family
    pub font_name: String,
    /// Font size in points
    pub font_size: f32,
    /// Font weight (normal or bold)
    pub font_weight: FontWeight,
    /// Font style: italic or normal
    pub is_italic: bool,
    /// Text color
    pub color: Color,
    /// Marked Content ID (for Tagged PDFs)
    ///
    /// This field stores the MCID if this character was extracted within
    /// a marked content sequence in a Tagged PDF.
    pub mcid: Option<u32>,

    // === Transformation properties (v0.3.1, Issue #27) ===
    /// Baseline origin X coordinate.
    ///
    /// This is the X position where the character's baseline starts,
    /// which is the standard reference point for text positioning in PDFs.
    /// Unlike bbox.x which is the left edge of the glyph, origin_x is
    /// the typographic origin point.
    pub origin_x: f32,

    /// Baseline origin Y coordinate.
    ///
    /// This is the Y position of the character's baseline. For horizontal
    /// text, this is where the bottom of letters like 'a', 'x' sit, while
    /// letters with descenders like 'g', 'y' extend below this line.
    pub origin_y: f32,

    /// Rotation angle in degrees (0-360, clockwise from horizontal).
    ///
    /// Calculated from the text transformation matrix using atan2(b, a).
    /// - 0° = normal horizontal text (left to right)
    /// - 90° = vertical text (top to bottom)
    /// - 180° = upside down text
    /// - 270° = vertical text (bottom to top)
    pub rotation_degrees: f32,

    /// Horizontal advance width (distance to next character position).
    ///
    /// This is the amount the text position advances after drawing this
    /// character, accounting for character width and any spacing. Used
    /// for precise text layout calculations.
    pub advance_width: f32,

    /// Full transformation matrix [a, b, c, d, e, f].
    ///
    /// The composed text matrix (CTM × Tm) that transforms this character
    /// from text space to device space. Provides complete transformation
    /// info for advanced use cases like re-rendering or precise positioning.
    ///
    /// Matrix layout:
    /// ```text
    /// [ a  b  0 ]
    /// [ c  d  0 ]
    /// [ e  f  1 ]
    /// ```
    /// Where (a,d) = scaling, (b,c) = rotation/skew, (e,f) = translation.
    pub matrix: Option<[f32; 6]>,
}

impl TextChar {
    /// Get the rotation angle in radians.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use pdf_oxide::layout::TextChar;
    ///
    /// let char = // ... create TextChar with rotation_degrees = 90.0
    /// assert!((char.rotation_radians() - std::f32::consts::FRAC_PI_2).abs() < 0.01);
    /// ```
    pub fn rotation_radians(&self) -> f32 {
        self.rotation_degrees.to_radians()
    }

    /// Check if this character is rotated (non-zero rotation).
    ///
    /// Returns true if the rotation angle is greater than 0.01 degrees,
    /// which accounts for floating-point precision while detecting
    /// intentional rotation.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use pdf_oxide::layout::TextChar;
    ///
    /// let normal_char = // ... rotation_degrees = 0.0
    /// assert!(!normal_char.is_rotated());
    ///
    /// let rotated_char = // ... rotation_degrees = 45.0
    /// assert!(rotated_char.is_rotated());
    /// ```
    pub fn is_rotated(&self) -> bool {
        self.rotation_degrees.abs() > 0.01
    }

    /// Set the transformation matrix and update derived values.
    ///
    /// This method sets the full transformation matrix and automatically
    /// calculates the rotation angle and origin from the matrix components.
    ///
    /// # Arguments
    ///
    /// * `matrix` - A 6-element transformation matrix [a, b, c, d, e, f]
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use pdf_oxide::layout::TextChar;
    ///
    /// let mut char = // ... create TextChar
    /// // Set a 45-degree rotation matrix
    /// let cos45 = 0.707;
    /// let sin45 = 0.707;
    /// char.with_matrix([cos45, sin45, -sin45, cos45, 100.0, 200.0]);
    ///
    /// assert!((char.rotation_degrees - 45.0).abs() < 0.5);
    /// assert_eq!(char.origin_x, 100.0);
    /// assert_eq!(char.origin_y, 200.0);
    /// ```
    pub fn with_matrix(mut self, matrix: [f32; 6]) -> Self {
        self.matrix = Some(matrix);
        // Extract origin from translation components
        self.origin_x = matrix[4];
        self.origin_y = matrix[5];
        // Calculate rotation from matrix: atan2(b, a)
        self.rotation_degrees = matrix[1].atan2(matrix[0]).to_degrees();
        self
    }

    /// Get the transformation matrix, computing from basic values if not stored.
    ///
    /// If the matrix was stored during extraction, returns it directly.
    /// Otherwise, reconstructs a basic matrix from origin and rotation.
    ///
    /// # Returns
    ///
    /// A 6-element transformation matrix [a, b, c, d, e, f]
    pub fn get_matrix(&self) -> [f32; 6] {
        if let Some(m) = self.matrix {
            m
        } else {
            // Reconstruct matrix from rotation and origin
            let rad = self.rotation_radians();
            let cos_r = rad.cos();
            let sin_r = rad.sin();
            [cos_r, sin_r, -sin_r, cos_r, self.origin_x, self.origin_y]
        }
    }

    /// Create a simple TextChar with default transformation values.
    ///
    /// This is a convenience constructor for creating TextChar instances
    /// when transformation data is not available (e.g., programmatic creation).
    /// The origin defaults to the bbox position, rotation to 0, and
    /// advance_width to the bbox width.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use pdf_oxide::layout::{TextChar, FontWeight, Color};
    /// use pdf_oxide::geometry::Rect;
    ///
    /// let char = TextChar::simple(
    ///     'A',
    ///     Rect::new(100.0, 200.0, 10.0, 12.0),
    ///     "Helvetica".to_string(),
    ///     12.0,
    /// );
    /// assert_eq!(char.origin_x, 100.0);
    /// assert_eq!(char.rotation_degrees, 0.0);
    /// ```
    pub fn simple(char: char, bbox: Rect, font_name: String, font_size: f32) -> Self {
        Self {
            char,
            bbox,
            font_name,
            font_size,
            font_weight: FontWeight::Normal,
            is_italic: false,
            color: Color::black(),
            mcid: None,
            origin_x: bbox.x,
            origin_y: bbox.y,
            rotation_degrees: 0.0,
            advance_width: bbox.width,
            matrix: None,
        }
    }
}

/// Font weight classification following PDF spec numeric scale.
///
/// PDF Spec: ISO 32000-1:2008, Table 122 - FontDescriptor
/// Values: 100-900 where 400 = normal, 700 = bold
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
#[repr(u16)]
#[derive(Default)]
pub enum FontWeight {
    /// Thin (100)
    Thin = 100,
    /// Extra Light (200)
    ExtraLight = 200,
    /// Light (300)
    Light = 300,
    /// Normal (400) - default weight
    #[default]
    Normal = 400,
    /// Medium (500)
    Medium = 500,
    /// Semi Bold (600)
    SemiBold = 600,
    /// Bold (700) - standard bold weight
    Bold = 700,
    /// Extra Bold (800)
    ExtraBold = 800,
    /// Black (900) - heaviest weight
    Black = 900,
}

impl FontWeight {
    /// Check if this weight is considered bold (>= 600).
    ///
    /// Per PDF spec, weights 600+ are semi-bold or bolder.
    pub fn is_bold(&self) -> bool {
        *self as u16 >= 600
    }

    /// Create FontWeight from PDF numeric value.
    ///
    /// Rounds to nearest standard weight value.
    pub fn from_pdf_value(value: i32) -> Self {
        match value {
            ..=150 => FontWeight::Thin,
            151..=250 => FontWeight::ExtraLight,
            251..=350 => FontWeight::Light,
            351..=450 => FontWeight::Normal,
            451..=550 => FontWeight::Medium,
            551..=650 => FontWeight::SemiBold,
            651..=750 => FontWeight::Bold,
            751..=850 => FontWeight::ExtraBold,
            851.. => FontWeight::Black,
        }
    }

    /// Get the numeric PDF value for this weight.
    pub fn to_pdf_value(&self) -> u16 {
        *self as u16
    }
}

/// RGB color representation.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize)]
pub struct Color {
    /// Red channel (0.0 - 1.0)
    pub r: f32,
    /// Green channel (0.0 - 1.0)
    pub g: f32,
    /// Blue channel (0.0 - 1.0)
    pub b: f32,
}

impl Color {
    /// Create a new color.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use pdf_oxide::layout::Color;
    ///
    /// let black = Color::new(0.0, 0.0, 0.0);
    /// let white = Color::new(1.0, 1.0, 1.0);
    /// let red = Color::new(1.0, 0.0, 0.0);
    /// ```
    pub fn new(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b }
    }

    /// Create a black color.
    pub fn black() -> Self {
        Self::new(0.0, 0.0, 0.0)
    }

    /// Create a white color.
    pub fn white() -> Self {
        Self::new(1.0, 1.0, 1.0)
    }
}

/// A text block (word, line, or paragraph).
#[derive(Debug, Clone, serde::Serialize)]
pub struct TextBlock {
    /// Characters in this block
    pub chars: Vec<TextChar>,
    /// Bounding box of the entire block
    pub bbox: Rect,
    /// Text content
    pub text: String,
    /// Average font size
    pub avg_font_size: f32,
    /// Dominant font name
    pub dominant_font: String,
    /// Whether the block contains bold text
    pub is_bold: bool,
    /// Whether the block contains italic text
    pub is_italic: bool,
    /// Marked Content ID (for Tagged PDFs)
    ///
    /// This field stores the MCID (Marked Content ID) if this text block
    /// belongs to a marked content sequence in a Tagged PDF. The MCID can
    /// be used to determine reading order via the structure tree.
    pub mcid: Option<u32>,
}

impl TextBlock {
    /// Create a text block from a collection of characters.
    ///
    /// This computes the bounding box, text content, average font size,
    /// and dominant font from the character data.
    ///
    /// # Panics
    ///
    /// Panics if the `chars` vector is empty.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use pdf_oxide::geometry::Rect;
    /// use pdf_oxide::layout::{TextChar, TextBlock, FontWeight, Color};
    ///
    /// let chars = vec![
    ///     TextChar {
    ///         char: 'H',
    ///         bbox: Rect::new(0.0, 0.0, 10.0, 12.0),
    ///         font_name: "Times".to_string(),
    ///         font_size: 12.0,
    ///         font_weight: FontWeight::Normal,
    ///         is_italic: false,
    ///         color: Color::black(),
    ///     },
    ///     TextChar {
    ///         char: 'i',
    ///         bbox: Rect::new(10.0, 0.0, 5.0, 12.0),
    ///         font_name: "Times".to_string(),
    ///         font_size: 12.0,
    ///         font_weight: FontWeight::Normal,
    ///         is_italic: false,
    ///         color: Color::black(),
    ///     },
    /// ];
    ///
    /// let block = TextBlock::from_chars(chars);
    /// assert_eq!(block.text, "Hi");
    /// assert_eq!(block.avg_font_size, 12.0);
    /// ```
    pub fn from_chars(chars: Vec<TextChar>) -> Self {
        assert!(!chars.is_empty(), "Cannot create TextBlock from empty chars");

        // Collect text directly (word spacing is handled at markdown level)
        let text: String = chars.iter().map(|c| c.char).collect();

        // Compute bounding box as union of all character bboxes
        let bbox = chars
            .iter()
            .map(|c| c.bbox)
            .fold(chars[0].bbox, |acc, r| acc.union(&r));

        let avg_font_size = chars.iter().map(|c| c.font_size).sum::<f32>() / chars.len() as f32;

        // Find dominant font (most common)
        let mut font_counts = HashMap::new();
        for c in &chars {
            *font_counts.entry(c.font_name.clone()).or_insert(0) += 1;
        }
        let dominant_font = font_counts
            .iter()
            .max_by_key(|(_, count)| *count)
            .map(|(font, _)| font.clone())
            .unwrap_or_default();

        let is_bold = chars.iter().any(|c| c.font_weight.is_bold());
        let is_italic = chars.iter().any(|c| c.is_italic);

        // Determine MCID for the block
        // Use the MCID of the first character if all chars have the same MCID
        let mcid = chars
            .first()
            .and_then(|c| c.mcid)
            .filter(|&first_mcid| chars.iter().all(|c| c.mcid == Some(first_mcid)));

        Self {
            chars,
            bbox,
            text,
            avg_font_size,
            dominant_font,
            is_bold,
            is_italic,
            mcid,
        }
    }

    /// Get the center point of the text block.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use pdf_oxide::geometry::Rect;
    /// use pdf_oxide::layout::{TextChar, TextBlock, FontWeight, Color};
    ///
    /// let chars = vec![
    ///     TextChar {
    ///         char: 'A',
    ///         bbox: Rect::new(0.0, 0.0, 100.0, 50.0),
    ///         font_name: "Times".to_string(),
    ///         font_size: 12.0,
    ///         font_weight: FontWeight::Normal,
    ///         is_italic: false,
    ///         color: Color::black(),
    ///     },
    /// ];
    ///
    /// let block = TextBlock::from_chars(chars);
    /// let center = block.center();
    /// assert_eq!(center.x, 50.0);
    /// assert_eq!(center.y, 25.0);
    /// ```
    pub fn center(&self) -> Point {
        self.bbox.center()
    }

    /// Check if this block is horizontally aligned with another block.
    ///
    /// Two blocks are considered horizontally aligned if their Y coordinates
    /// are within the specified tolerance.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use pdf_oxide::geometry::Rect;
    /// use pdf_oxide::layout::{TextChar, TextBlock, FontWeight, Color};
    ///
    /// let chars1 = vec![
    ///     TextChar {
    ///         char: 'A',
    ///         bbox: Rect::new(0.0, 0.0, 10.0, 10.0),
    ///         font_name: "Times".to_string(),
    ///         font_size: 12.0,
    ///         font_weight: FontWeight::Normal,
    ///         is_italic: false,
    ///         color: Color::black(),
    ///     },
    /// ];
    /// let chars2 = vec![
    ///     TextChar {
    ///         char: 'B',
    ///         bbox: Rect::new(50.0, 1.0, 10.0, 10.0),
    ///         font_name: "Times".to_string(),
    ///         font_size: 12.0,
    ///         font_weight: FontWeight::Normal,
    ///         is_italic: false,
    ///         color: Color::black(),
    ///     },
    /// ];
    ///
    /// let block1 = TextBlock::from_chars(chars1);
    /// let block2 = TextBlock::from_chars(chars2);
    ///
    /// assert!(block1.is_horizontally_aligned(&block2, 5.0));
    /// assert!(!block1.is_horizontally_aligned(&block2, 0.5));
    /// ```
    pub fn is_horizontally_aligned(&self, other: &TextBlock, tolerance: f32) -> bool {
        (self.bbox.y - other.bbox.y).abs() < tolerance
    }

    /// Check if this block is vertically aligned with another block.
    ///
    /// Two blocks are considered vertically aligned if their X coordinates
    /// are within the specified tolerance.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use pdf_oxide::geometry::Rect;
    /// use pdf_oxide::layout::{TextChar, TextBlock, FontWeight, Color};
    ///
    /// let chars1 = vec![
    ///     TextChar {
    ///         char: 'A',
    ///         bbox: Rect::new(0.0, 0.0, 10.0, 10.0),
    ///         font_name: "Times".to_string(),
    ///         font_size: 12.0,
    ///         font_weight: FontWeight::Normal,
    ///         is_italic: false,
    ///         color: Color::black(),
    ///     },
    /// ];
    /// let chars2 = vec![
    ///     TextChar {
    ///         char: 'B',
    ///         bbox: Rect::new(1.0, 50.0, 10.0, 10.0),
    ///         font_name: "Times".to_string(),
    ///         font_size: 12.0,
    ///         font_weight: FontWeight::Normal,
    ///         is_italic: false,
    ///         color: Color::black(),
    ///     },
    /// ];
    ///
    /// let block1 = TextBlock::from_chars(chars1);
    /// let block2 = TextBlock::from_chars(chars2);
    ///
    /// assert!(block1.is_vertically_aligned(&block2, 5.0));
    /// assert!(!block1.is_vertically_aligned(&block2, 0.5));
    /// ```
    pub fn is_vertically_aligned(&self, other: &TextBlock, tolerance: f32) -> bool {
        (self.bbox.x - other.bbox.x).abs() < tolerance
    }
}

/// A word is a semantic unit of text (alias for TextBlock).
///
/// In this library, a "Word" is a sequence of characters grouped by spatial proximity.
/// It typically corresponds to a linguistic word, but may also be a number, symbol,
/// or other discrete text element.
pub type Word = TextBlock;

/// A line of text containing multiple words.
///
/// Represents a visually distinct line of text, composed of one or more words
/// sorted in reading order (left-to-right for LTR languages).
#[derive(Debug, Clone, serde::Serialize)]
pub struct TextLine {
    /// Words in this line
    pub words: Vec<Word>,
    /// Bounding box of the entire line
    pub bbox: Rect,
    /// Complete text content of the line (words joined by spaces)
    pub text: String,
}

impl TextLine {
    /// Create a new TextLine from a list of words.
    ///
    /// # Panics
    ///
    /// Panics if the `words` vector is empty.
    pub fn new(words: Vec<Word>) -> Self {
        assert!(!words.is_empty(), "Cannot create TextLine from empty words");

        // Compute bounding box as union of all word bboxes
        let bbox = words
            .iter()
            .map(|w| w.bbox)
            .fold(words[0].bbox, |acc, r| acc.union(&r));

        // Join word text with spaces
        let text = words
            .iter()
            .map(|w| w.text.as_str())
            .collect::<Vec<_>>()
            .join(" ");

        Self { words, bbox, text }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mock_char(c: char, x: f32, y: f32) -> TextChar {
        let bbox = Rect::new(x, y, 10.0, 12.0);
        TextChar {
            char: c,
            bbox,
            font_name: "Times".to_string(),
            font_size: 12.0,
            font_weight: FontWeight::Normal,
            is_italic: false,
            color: Color::black(),
            mcid: None,
            // Transformation fields (v0.3.1)
            origin_x: bbox.x,
            origin_y: bbox.y,
            rotation_degrees: 0.0,
            advance_width: bbox.width,
            matrix: None,
        }
    }

    #[test]
    fn test_color_creation() {
        let black = Color::black();
        assert_eq!(black.r, 0.0);
        assert_eq!(black.g, 0.0);
        assert_eq!(black.b, 0.0);

        let white = Color::white();
        assert_eq!(white.r, 1.0);
        assert_eq!(white.g, 1.0);
        assert_eq!(white.b, 1.0);

        let red = Color::new(1.0, 0.0, 0.0);
        assert_eq!(red.r, 1.0);
        assert_eq!(red.g, 0.0);
        assert_eq!(red.b, 0.0);
    }

    #[test]
    fn test_text_block_from_chars() {
        let chars = vec![
            mock_char('H', 0.0, 0.0),
            mock_char('e', 10.0, 0.0),
            mock_char('l', 20.0, 0.0),
            mock_char('l', 30.0, 0.0),
            mock_char('o', 40.0, 0.0),
        ];

        let block = TextBlock::from_chars(chars);
        assert_eq!(block.text, "Hello");
        assert_eq!(block.avg_font_size, 12.0);
        assert_eq!(block.dominant_font, "Times");
        assert!(!block.is_bold);
    }

    #[test]
    fn test_text_block_bold_detection() {
        let bbox = Rect::new(0.0, 0.0, 10.0, 12.0);
        let chars = vec![
            TextChar {
                char: 'B',
                bbox,
                font_name: "Times".to_string(),
                font_size: 12.0,
                font_weight: FontWeight::Bold,
                is_italic: false,
                color: Color::black(),
                mcid: None,
                origin_x: bbox.x,
                origin_y: bbox.y,
                rotation_degrees: 0.0,
                advance_width: bbox.width,
                matrix: None,
            },
            mock_char('o', 10.0, 0.0),
            mock_char('l', 20.0, 0.0),
            mock_char('d', 30.0, 0.0),
        ];

        let block = TextBlock::from_chars(chars);
        assert_eq!(block.text, "Bold");
        assert!(block.is_bold);
    }

    #[test]
    fn test_text_block_center() {
        let bbox = Rect::new(0.0, 0.0, 100.0, 50.0);
        let chars = vec![TextChar {
            char: 'A',
            bbox,
            font_name: "Times".to_string(),
            font_size: 12.0,
            font_weight: FontWeight::Normal,
            is_italic: false,
            color: Color::black(),
            mcid: None,
            origin_x: bbox.x,
            origin_y: bbox.y,
            rotation_degrees: 0.0,
            advance_width: bbox.width,
            matrix: None,
        }];

        let block = TextBlock::from_chars(chars);
        let center = block.center();
        assert_eq!(center.x, 50.0);
        assert_eq!(center.y, 25.0);
    }

    #[test]
    fn test_horizontal_alignment() {
        let chars1 = vec![mock_char('A', 0.0, 0.0)];
        let chars2 = vec![mock_char('B', 50.0, 2.0)];
        let chars3 = vec![mock_char('C', 100.0, 20.0)];

        let block1 = TextBlock::from_chars(chars1);
        let block2 = TextBlock::from_chars(chars2);
        let block3 = TextBlock::from_chars(chars3);

        assert!(block1.is_horizontally_aligned(&block2, 5.0));
        assert!(!block1.is_horizontally_aligned(&block3, 5.0));
    }

    #[test]
    fn test_vertical_alignment() {
        let chars1 = vec![mock_char('A', 0.0, 0.0)];
        let chars2 = vec![mock_char('B', 2.0, 50.0)];
        let chars3 = vec![mock_char('C', 20.0, 100.0)];

        let block1 = TextBlock::from_chars(chars1);
        let block2 = TextBlock::from_chars(chars2);
        let block3 = TextBlock::from_chars(chars3);

        assert!(block1.is_vertically_aligned(&block2, 5.0));
        assert!(!block1.is_vertically_aligned(&block3, 5.0));
    }
}
