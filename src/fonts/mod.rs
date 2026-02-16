//! Font handling and encoding.
//!
//! This module provides font dictionary parsing, encoding handling,
//! and ToUnicode CMap parsing for accurate text extraction.
//!
//! # PDF Creation (v0.3.0)
//!
//! For PDF creation with embedded fonts, this module also provides:
//! - `truetype_parser` - Parse TTF/OTF fonts for embedding
//! - `font_subsetter` - Subset fonts to reduce file size
//! - `encoding` - Unicode encoding support for CID fonts

mod adobe_glyph_list;
pub mod character_mapper;
/// CID to Unicode mappings for predefined Adobe CJK character collections.
pub mod cid_mappings;
pub mod cmap;
pub mod encoding;
pub mod encoding_normalizer;
pub mod font_dict; // Private module - only used internally by font_dict
pub mod font_subsetter;
pub mod non_text_detection;
/// TrueType font CMap parsing for glyph-to-character mapping.
pub mod truetype_cmap;
/// TrueType/OpenType font parser for PDF embedding (v0.3.0).
pub mod truetype_parser;

pub use character_mapper::CharacterMapper;
pub use cmap::{parse_tounicode_cmap, CMap, LazyCMap};
pub use encoding::UnicodeEncoder;
pub use encoding_normalizer::EncodingNormalizer;
pub use font_dict::{CIDSystemInfo, CIDToGIDMap, Encoding, FontInfo};
pub use font_subsetter::FontSubsetter;
pub use non_text_detection::{
    CharacterConfidence, ConfidenceReason, NonTextDetector, NonTextStats,
};
pub use truetype_cmap::TrueTypeCMap;
pub use truetype_parser::{FontMetrics, TrueTypeError, TrueTypeFont, TrueTypeResult};
