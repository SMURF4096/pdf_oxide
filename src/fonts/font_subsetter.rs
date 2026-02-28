//! Font subsetting for PDF embedding.
//!
//! Subsets TrueType fonts to include only glyphs that are actually used,
//! significantly reducing PDF file size. Per PDF spec Section 9.9,
//! subset fonts use a tag prefix (e.g., "ABCDEF+FontName").
//!
//! # Subsetting Strategy
//!
//! For maximum compatibility, we use a simple approach:
//! 1. Track which glyphs are used in the document
//! 2. Generate a ToUnicode CMap for those glyphs
//! 3. Embed the full font (or use proper subsetting crate in future)
//!
//! Full subsetting (removing unused glyph data) is complex due to:
//! - Composite glyphs referencing other glyphs
//! - OpenType feature tables (GSUB, GPOS)
//! - Hinting data dependencies
//!
//! For v0.3.0, we track used glyphs and generate proper CID mappings,
//! deferring binary font subsetting to a future version or external crate.

use std::collections::{BTreeSet, HashMap};

/// Font subsetter for tracking used glyphs and generating subset metadata.
///
/// This tracks which Unicode characters and glyphs are used in a document,
/// enabling efficient ToUnicode CMap generation and potential future subsetting.
#[derive(Debug, Default)]
pub struct FontSubsetter {
    /// Used Unicode codepoints mapped to their glyph IDs
    used_chars: HashMap<u32, u16>,
    /// Set of used glyph IDs (for width array generation)
    used_glyphs: BTreeSet<u16>,
    /// Subset tag (6 uppercase letters, e.g., "ABCDEF")
    subset_tag: Option<String>,
}

impl FontSubsetter {
    /// Create a new font subsetter.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a character as used.
    ///
    /// # Arguments
    /// * `codepoint` - Unicode codepoint
    /// * `glyph_id` - Corresponding glyph ID from the font
    pub fn use_char(&mut self, codepoint: u32, glyph_id: u16) {
        self.used_chars.insert(codepoint, glyph_id);
        self.used_glyphs.insert(glyph_id);
    }

    /// Record multiple characters as used.
    pub fn use_string(&mut self, text: &str, glyph_lookup: impl Fn(u32) -> Option<u16>) {
        for ch in text.chars() {
            let codepoint = ch as u32;
            if let Some(glyph_id) = glyph_lookup(codepoint) {
                self.use_char(codepoint, glyph_id);
            }
        }
    }

    /// Get the set of used glyph IDs.
    pub fn used_glyphs(&self) -> &BTreeSet<u16> {
        &self.used_glyphs
    }

    /// Get the used character to glyph mapping.
    pub fn used_chars(&self) -> &HashMap<u32, u16> {
        &self.used_chars
    }

    /// Get the number of used glyphs.
    pub fn glyph_count(&self) -> usize {
        self.used_glyphs.len()
    }

    /// Get the number of used characters.
    pub fn char_count(&self) -> usize {
        self.used_chars.len()
    }

    /// Check if any characters have been used.
    pub fn is_empty(&self) -> bool {
        self.used_chars.is_empty()
    }

    /// Generate a subset tag for the font name.
    ///
    /// Per PDF spec, subset fonts should be named "ABCDEF+FontName"
    /// where ABCDEF is a unique 6-letter tag.
    pub fn generate_subset_tag(&mut self) -> &str {
        if self.subset_tag.is_none() {
            // Generate a deterministic tag based on used glyphs
            // This ensures the same subset gets the same tag
            let hash = self.compute_subset_hash();
            let tag = Self::hash_to_tag(hash);
            self.subset_tag = Some(tag);
        }
        // Safety: subset_tag is set to Some on the line above
        self.subset_tag
            .as_ref()
            .expect("subset_tag set on prior line")
    }

    /// Get the subset tag if already generated.
    pub fn subset_tag(&self) -> Option<&str> {
        self.subset_tag.as_deref()
    }

    /// Compute a hash of the subset for tag generation.
    fn compute_subset_hash(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        for glyph in &self.used_glyphs {
            glyph.hash(&mut hasher);
        }
        hasher.finish()
    }

    /// Convert a hash to a 6-letter uppercase tag.
    fn hash_to_tag(hash: u64) -> String {
        let mut tag = String::with_capacity(6);
        let mut h = hash;
        for _ in 0..6 {
            let ch = (h % 26) as u8 + b'A';
            tag.push(ch as char);
            h /= 26;
        }
        tag
    }

    /// Create the subset font name.
    ///
    /// # Arguments
    /// * `base_name` - Original font name (e.g., "Arial")
    ///
    /// # Returns
    /// Subset name (e.g., "ABCDEF+Arial")
    pub fn subset_font_name(&mut self, base_name: &str) -> String {
        let tag = self.generate_subset_tag();
        format!("{}+{}", tag, base_name)
    }

    /// Clear the subsetter for reuse.
    pub fn clear(&mut self) {
        self.used_chars.clear();
        self.used_glyphs.clear();
        self.subset_tag = None;
    }

    /// Get statistics about the subset.
    pub fn stats(&self) -> SubsetStats {
        SubsetStats {
            unique_chars: self.used_chars.len(),
            unique_glyphs: self.used_glyphs.len(),
            min_glyph_id: self.used_glyphs.first().copied(),
            max_glyph_id: self.used_glyphs.last().copied(),
        }
    }
}

/// Statistics about a font subset.
#[derive(Debug, Clone)]
pub struct SubsetStats {
    /// Number of unique Unicode characters used
    pub unique_chars: usize,
    /// Number of unique glyphs used
    pub unique_glyphs: usize,
    /// Minimum glyph ID used
    pub min_glyph_id: Option<u16>,
    /// Maximum glyph ID used
    pub max_glyph_id: Option<u16>,
}

impl SubsetStats {
    /// Calculate potential file size reduction percentage.
    ///
    /// This is an estimate based on glyph count ratio.
    /// Actual reduction depends on glyph complexity.
    pub fn estimated_reduction(&self, total_glyphs: u16) -> f32 {
        if total_glyphs == 0 || self.unique_glyphs == 0 {
            return 0.0;
        }
        let used = self.unique_glyphs as f32;
        let total = total_glyphs as f32;
        (1.0 - used / total) * 100.0
    }
}

/// Builder for creating subsets with additional options.
#[derive(Debug)]
pub struct SubsetBuilder {
    subsetter: FontSubsetter,
    /// Always include certain glyphs (e.g., .notdef)
    always_include: BTreeSet<u16>,
}

impl SubsetBuilder {
    /// Create a new subset builder.
    pub fn new() -> Self {
        let mut always_include = BTreeSet::new();
        // Always include glyph 0 (.notdef) per PDF spec
        always_include.insert(0);

        Self {
            subsetter: FontSubsetter::new(),
            always_include,
        }
    }

    /// Add a glyph ID that should always be included.
    pub fn always_include_glyph(mut self, glyph_id: u16) -> Self {
        self.always_include.insert(glyph_id);
        self
    }

    /// Record a character as used.
    pub fn use_char(mut self, codepoint: u32, glyph_id: u16) -> Self {
        self.subsetter.use_char(codepoint, glyph_id);
        self
    }

    /// Record a string as used.
    pub fn use_string(mut self, text: &str, glyph_lookup: impl Fn(u32) -> Option<u16>) -> Self {
        self.subsetter.use_string(text, glyph_lookup);
        self
    }

    /// Build the final subsetter with always-included glyphs.
    pub fn build(mut self) -> FontSubsetter {
        for glyph in self.always_include {
            self.subsetter.used_glyphs.insert(glyph);
        }
        self.subsetter
    }
}

impl Default for SubsetBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_subsetter_creation() {
        let subsetter = FontSubsetter::new();
        assert!(subsetter.is_empty());
        assert_eq!(subsetter.glyph_count(), 0);
    }

    #[test]
    fn test_use_char() {
        let mut subsetter = FontSubsetter::new();
        subsetter.use_char(0x0041, 1); // 'A' -> GID 1
        subsetter.use_char(0x0042, 2); // 'B' -> GID 2

        assert!(!subsetter.is_empty());
        assert_eq!(subsetter.char_count(), 2);
        assert_eq!(subsetter.glyph_count(), 2);
        assert!(subsetter.used_glyphs().contains(&1));
        assert!(subsetter.used_glyphs().contains(&2));
    }

    #[test]
    fn test_use_string() {
        let mut subsetter = FontSubsetter::new();
        // Simple lookup: codepoint = glyph_id for testing
        subsetter.use_string("AB", |cp| Some(cp as u16));

        assert_eq!(subsetter.char_count(), 2);
        assert!(subsetter.used_chars().contains_key(&0x41));
        assert!(subsetter.used_chars().contains_key(&0x42));
    }

    #[test]
    fn test_subset_tag_generation() {
        let mut subsetter = FontSubsetter::new();
        subsetter.use_char(0x0041, 1);

        let tag = subsetter.generate_subset_tag().to_string();
        assert_eq!(tag.len(), 6);
        assert!(tag.chars().all(|c| c.is_ascii_uppercase()));

        // Same subset should generate same tag
        let tag2 = subsetter.generate_subset_tag().to_string();
        assert_eq!(tag, tag2);
    }

    #[test]
    fn test_subset_font_name() {
        let mut subsetter = FontSubsetter::new();
        subsetter.use_char(0x0041, 1);

        let name = subsetter.subset_font_name("Arial");
        assert!(name.contains('+'));
        assert!(name.ends_with("Arial"));
        assert_eq!(name.split('+').next().unwrap().len(), 6);
    }

    #[test]
    fn test_stats() {
        let mut subsetter = FontSubsetter::new();
        subsetter.use_char(0x0041, 5);
        subsetter.use_char(0x0042, 10);
        subsetter.use_char(0x0043, 15);

        let stats = subsetter.stats();
        assert_eq!(stats.unique_chars, 3);
        assert_eq!(stats.unique_glyphs, 3);
        assert_eq!(stats.min_glyph_id, Some(5));
        assert_eq!(stats.max_glyph_id, Some(15));
    }

    #[test]
    fn test_estimated_reduction() {
        let mut subsetter = FontSubsetter::new();
        for i in 0..10 {
            subsetter.use_char(0x0041 + i, i as u16 + 1);
        }

        let stats = subsetter.stats();
        // Using 10 out of 1000 glyphs = 99% reduction
        let reduction = stats.estimated_reduction(1000);
        assert!(reduction > 98.0);
        assert!(reduction < 100.0);
    }

    #[test]
    fn test_builder_always_includes_notdef() {
        let subsetter = SubsetBuilder::new().use_char(0x0041, 1).build();

        // Glyph 0 (.notdef) should be included automatically
        assert!(subsetter.used_glyphs().contains(&0));
        assert!(subsetter.used_glyphs().contains(&1));
    }

    #[test]
    fn test_clear() {
        let mut subsetter = FontSubsetter::new();
        subsetter.use_char(0x0041, 1);
        let _ = subsetter.generate_subset_tag();

        subsetter.clear();

        assert!(subsetter.is_empty());
        assert!(subsetter.subset_tag().is_none());
    }
}
